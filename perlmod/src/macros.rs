/// Convenience macros.

/// Create a standard destructor for a boxed type.
///
/// For safety it is recommended to pass the package name to the macro in order for the generated
/// code to also guard against values not blessed into the package.
///
/// Custom code for when an invalid value is passed to the function can be provided as follows:
///
/// Usage:
/// ```ignore
/// // complete:
/// destructor!(MyType : "My::RS::Package" {
///     Err(err) => { eprintln!("DESTROY called with invalid pointer: {}", err); }
/// });
///
/// // error case only
/// destructor!(MyType {
///     Err(err) => { eprintln!("DESTROY called with invalid pointer: {}", err); }
/// });
///
/// // simple case with default error case (which is the above example case)
/// destructor!(MyType : "My::RS::Package");
///
/// // simple less-safe case without checking the reference type.
/// destructor!(MyType);
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
    ($ty:ty : $package:literal) => {
        $crate::destructor! {
            $ty : $package {
                Err(err) => {
                    eprintln!("DESTROY called with invalid pointer: {}", err);
                }
            }
        }
    };

    ($ty:ty : $package:literal {
        Err($errname:ident) => $on_err:expr
    }) => {
        #[perlmod::export(name = "DESTROY")]
        fn destroy(#[raw] this: Value) {
            match unsafe { this.from_blessed_box::<$ty>($package) } {
                Ok(ptr) => {
                    let _ = unsafe { Box::<$ty>::from_raw(ptr as *const $ty as *mut $ty) };
                }
                Err($errname) => $on_err,
            }
        }
    };

    ($ty:ty) => {
        $crate::destructor! {
            $ty {
                Err(err) => {
                    eprintln!("DESTROY called with invalid pointer: {}", err);
                }
            }
        }
    };

    ($ty:ty {
        Err($name:ident) => $on_err:expr
    }) => {
        #[perlmod::export(name = "DESTROY")]
        fn destroy(#[raw] this: Value) {
            match unsafe { this.from_ref_box::<$ty>() } {
                Ok(ptr) => {
                    let _ = unsafe { Box::<Bless>::from_raw(ptr) };
                }
                Err($name) => $on_err,
            }
        }
    };
}
