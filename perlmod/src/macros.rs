/// Convenience macros.

/// Create a standard destructor for a boxed type.
///
/// Due to compiler restrictions, the function itself needs to be written manually, only the
/// contents can be generated using this macro. This also means that the `this` parameter needs to
/// be passed to the macro.
///
/// For safety it is recommended to pass the package name to the macro in order for the generated
/// code to also guard against values not blessed into the package.
///
/// Custom code for when an invalid value is passed to the function can be provided as follows:
///
/// Usage:
/// ```ignore
/// #[export(name = "DESTROY")]
/// fn destroy(#[raw] this: Value) {
///     // complete:
///     destructor!(this, MyType : "My::RS::Package" => {
///         Err(err) => { eprintln!("DESTROY called with invalid pointer: {}", err); }
///     });
/// }
///
/// #[export(name = "DESTROY")]
/// fn destroy(#[raw] this: Value) {
///     // error case only
///     destructor!(this, MyType {
///         Err(err) => { eprintln!("DESTROY called with invalid pointer: {}", err); }
///     });
/// }
///
/// #[export(name = "DESTROY")]
/// fn destroy(#[raw] this: Value) {
///     // simple case with default error case (which is the above example case)
///     // the class name can also reference a constant.
///     destructor!(this, MyType : CLASSNAME);
/// }
///
/// #[export(name = "DESTROY")]
/// fn destroy(#[raw] this: Value) {
///     // simple less-safe case without checking the reference type.
///     destructor!(this, MyType);
/// }
/// ```
///
/// The generated code looks like this:
///
/// ```ignore
/// #[export(name = "DESTROY")]
/// fn destroy(#[raw] this: Value) {
///     match this.from_blessed_box::<MyType>("My::RS::Package") {
///         Ok(ptr) => {
///             let _ = unsafe { Box::<MyType>::from_raw(ptr) };
///         }
///         Err(err) => {
///             // this is the default error handler:
///             eprintln!("DESTROY called with invalid pointer: {}", err);
///         }
///     }
/// }
/// ```
#[macro_export]
macro_rules! destructor {
    ($this:expr, $ty:ty : $package:expr) => {
        $crate::destructor! {
            $this, $ty : $package => {
                Err(err) => {
                    eprintln!("DESTROY called with invalid pointer: {}", err);
                }
            }
        }
    };

    ($this:expr, $ty:ty : $package:expr => {
        Err($errname:ident) => $on_err:expr
    }) => {
        match unsafe { $this.from_blessed_box::<$ty>($package) } {
            Ok(ptr) => {
                let _ = unsafe { Box::<$ty>::from_raw(ptr as *const $ty as *mut $ty) };
            }
            Err($errname) => $on_err,
        }
    };

    ($this:expr, $ty:ty) => {
        $crate::destructor! {
            $this,
            $ty {
                Err(err) => {
                    eprintln!("DESTROY called with invalid pointer: {}", err);
                }
            }
        }
    };

    ($this:expr, $ty:ty {
        Err($name:ident) => $on_err:expr
    }) => {
        match unsafe { $this.from_ref_box::<$ty>() } {
            Ok(ptr) => {
                let _ = unsafe { Box::<Bless>::from_raw(ptr) };
            }
            Err($name) => $on_err,
        }
    };
}

/// Create a standard destructor for a value where a rust value has been attached via a
/// [`MagicSpec`]
///
/// This assumes the type is a reference and calls [`dereference`](crate::Value::dereference()) on
/// it.
///
/// Note that this only makes sense if the used [`MagicSpec`] does not include a `free` method.
/// This method *is* includded when using its `DEFAULT` or the [`crate::declare_magic!`] macro, so
/// this macro is only required when using custom magic with a custom `DESTROY` sub.
///
/// Due to compiler restrictions, the function itself needs to be written manually, only the
/// contents can be generated using this macro. This also means that the `this` parameter needs to
/// be passed to the macro.
///
/// Usage:
/// ```ignore
/// #[export(name = "DESTROY")]
/// fn destroy(#[raw] this: Value) {
///     // complete:
///     magic_destructor!(this: MyMagic => {
///         Err(err) => { eprintln!("DESTROY called with an invalid pointer: {}", err); }
///     });
/// }
///
/// #[export(name = "DESTROY")]
/// fn destroy(#[raw] this: Value) {
///     // simplest case with the default error message as shown above
///     destructor!(this: MyMagic);
/// }
/// ```
///
/// The generated code looks like this:
///
/// ```ignore
/// #[export(name = "DESTROY")]
/// fn destroy(#[raw] this: Value) {
///     match this.remove_magic(&MyMagic) {
///         Ok(_drpo) => (),
///         Err(err) => {
///             eprintln!("DESTROY called with an invalid pointer: {}", err);
///         }
///     }
/// }
/// ```
/// [`MagicSpec`]: crate::magic::MagicSpec
#[macro_export]
macro_rules! magic_destructor {
    ($this:ident: $spec:expr) => {
        $crate::magic_destructor!($this: $spec => {
            ref => eprintln!("DESTROY called with a non-reference"),
            type => eprintln!("DESTROY called on a value with no magic"),
        });
    };

    ($this:ident: $spec:expr => { ref => $on_ref_err:expr, type => $on_type_err:expr, }) => {
        match Value::dereference(&$this) {
            None => $on_ref_err,
            Some(value) => match $crate::ScalarRef::remove_magic(&value, $spec) {
                Ok(Some(_drop)) => (),
                Ok(None) => (),
                Err(_) => $on_type_err,
            }
        }
    };
}

/// Helper to create the data required for blessed references to values containing a magic pointer.
///
/// This is a simple shortcut to avoid repetitive tasks and adds the following:
/// * `const CLASSNAME: &'static str`: The perl package name.
/// * `const MAGIC: perlmod::MagicSpec<Container>`: The magic specification used for
///   [`add_magic`](crate::ScalarRef::add_magic()).
/// * `impl TryFrom<&Value> for &Inner`: assuming the value is a reference (calling
///   [`dereference`](crate::Value::dereference()) on it) and then looking for the `MAGIC` pointer.
/// * Binds the `Drop` handler for to the magic value, so that a custom destructor for perl is not
///   necessary.
///
/// ```
/// struct MyThing {} // anything
///
/// perlmod::declare_magic!(Box<MyThing> : &MyThing as "RSPM::MagicMacroClass");
/// ```
///
/// For a usage example see the [`magic`](crate::magic) module documentation.
#[macro_export]
macro_rules! declare_magic {
    ($ty:ty : &$inner:ty as $class:literal) => {
        const CLASSNAME: &str = $class;
        static MAGIC: $crate::MagicSpec<$ty> = unsafe {
            static TAG: $crate::MagicTag<$ty> = $crate::MagicTag::<$ty>::DEFAULT;
            perlmod::MagicSpec::new_static(&TAG)
        };

        impl<'a> ::std::convert::TryFrom<&'a $crate::Value> for &'a $inner {
            type Error = $crate::error::MagicError;

            fn try_from(value: &'a $crate::Value) -> Result<Self, $crate::error::MagicError> {
                use $crate::error::MagicError;
                value
                    .dereference()
                    .ok_or_else(|| MagicError::NotAReference(CLASSNAME))?
                    .find_magic(&MAGIC)
                    .ok_or_else(|| MagicError::NotFound(CLASSNAME))
            }
        }
    };
}

/// This is a version of `instantiate_magic` without the implicit return when an error happens.
/// Instead, this yields a `Result<Value, Error>`.
///
/// For a usage example see the [`magic`](crate::magic) module documentation.
#[macro_export]
macro_rules! instantiate_magic_result {
    ($class:expr, $magic:expr => $value:expr) => {{
        let value = $crate::Value::new_hash();
        let this = $crate::Value::new_ref(&value);
        match this.bless_sv($class) {
            Err(err) => Err(err),
            Ok(_) => {
                value.add_magic($magic.with_value($value));
                Ok(this)
            }
        }
    }};
}

/// Create an empty hash ref with magic data. This is a convenience helper for `sub new`
/// implementations.
///
/// For a usage example see the [`magic`](crate::magic) module documentation.
#[macro_export]
macro_rules! instantiate_magic {
    ($class:expr, $magic:expr => $value:expr) => {{
        $crate::instantiate_magic_result!($class, $magic => $value)?
    }};
}
