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
