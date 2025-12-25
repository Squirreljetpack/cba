// BOG

#[macro_export]
macro_rules! get_or_err {
    ($expr:expr, $bog_prefix:expr) => {
        match $expr {
            Ok(v) => v,
            Err(e) => {
                $crate::ebog!("{}: {e}", $bog_prefix);
                return Default::default();
            }
        }
    };

    ($expr:expr, $bog_prefix:expr, ?) => {
        match $expr {
            Ok(v) => v,
            Err(e) => {
                $crate::ebog!("{}: {e}", $bog_prefix);
                return Err(Default::default());
            }
        }
    };

    // Err($return) because this is only intended to be used in a fn returning Result/Option/bool
    ($expr:expr, $bog_prefix:expr, $return:expr) => {
        match $expr {
            Ok(v) => v,
            Err(e) => {
                $crate::ebog!("{}: {e}", $bog_prefix);
                return Err($return);
            }
        }
    };
}

#[macro_export]
macro_rules! get_or_warn {
    ($expr:expr, $bog_prefix:expr) => {
        match $expr {
            Ok(v) => v,
            Err(e) => {
                $crate::wbog!("{}: {e}", $bog_prefix);
                return Default::default();
            }
        }
    };

    ($expr:expr, $bog_prefix:expr, ?) => {
        match $expr {
            Ok(v) => v,
            Err(e) => {
                $crate::wbog!("{}: {e}", $bog_prefix);
                return Err(Default::default());
            }
        }
    };

    ($expr:expr, $bog_prefix:expr, $return:expr) => {
        match $expr {
            Ok(v) => v,
            Err(e) => {
                $crate::wbog!("{}: {e}", $bog_prefix);
                return Err($return);
            }
        }
    };
}

#[macro_export]
macro_rules! unwrap_or_err {
    ($expr:expr, $bog_err:expr) => {
        match $expr {
            Some(v) => v,
            None => {
                $crate::ebog!("{}", $bog_err);
                return Default::default();
            }
        }
    };

    ($expr:expr, $bog_err:expr, ?) => {
        match $expr {
            Some(v) => v,
            None => {
                $crate::ebog!("{}", $bog_err);
                return Err(Default::default());
            }
        }
    };

    ($expr:expr, $bog_err:expr, $return:expr) => {
        match $expr {
            Some(v) => v,
            None => {
                $crate::ebog!("{}", $bog_err);
                return Err($return);
            }
        }
    };
}

#[macro_export]
macro_rules! unwrap_or_warn {
    ($expr:expr, $bog_err:expr) => {
        match $expr {
            Some(v) => v,
            None => {
                $crate::wbog!("{}", $bog_err);
                return Default::default();
            }
        }
    };

    ($expr:expr, $bog_err:expr, ?) => {
        match $expr {
            Some(v) => v,
            None => {
                $crate::wbog!("{}", $bog_err);
                return Err(Default::default());
            }
        }
    };

    ($expr:expr, $bog_err:expr, $return:expr) => {
        match $expr {
            Some(v) => v,
            None => {
                $crate::wbog!("{}", $bog_err);
                return Err($return);
            }
        }
    };
}

// #[macro_export]
// macro_rules! err_if_false {
//     ($expr:expr, $err:expr) => {
//         if !$expr {
//             return Err($err);
//         }
//     };

//     ($expr:expr) => {
//         if !$expr {
//             return Err(Default::default());
//         }
//     };
// }

// ------------- DEBUG

/// dbg but only in debug builds
#[macro_export]
macro_rules! _dbg {
    ($($val:expr),+ $(,)?) => {{
        #[cfg(debug_assertions)]
        {
            $(dbg!(&$val);)+
        }
    }};
    ($($args:tt)*) => {{
        #[cfg(debug_assertions)]
        {
            dbg!($($args)*)
        }
    }};
}

/// Prints to stderr like `eprintln!` but only in debug builds
#[macro_export]
macro_rules! _eprint {
    ($($args:tt)*) => {
        #[cfg(debug_assertions)]
        {
            eprintln!($($args)*);
        }
    };
}
