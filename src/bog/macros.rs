// Sometimes easier to read than let else
#[macro_export]

/// # Examples
///
/// Unwrap Option or return Default::default():
/// ```rust
/// use cli_boilerplate_automation::unwrap;
/// use std::fs::File;
///
/// pub fn check_should_template(path: &std::path::Path) -> bool {
/// 	let err_prefix = format!("Failed to read {path:?} for templating");
/// 	let file = unwrap!(File::open(path).ok());
///
/// 	// process file
///
/// 	true
/// }
/// ```
///
/// A closure in the second argument unwraps Result:
/// ```rust,ignore
/// for binname in list {
/// 	let ActionBin {
///             name,
///             alias,
///             ..
/// 	} = unwrap!(
///     	binname.parse();
///     	|e| { err_count += 1; ebog!("Scan"; "Failed to parse filename of {}: {e}", path.to_string_lossy()) };
///     	continue
/// 	);
///
///	    println!("{name}");
/// }
/// ```
macro_rules! unwrap {
    ($expr:expr) => {
        match $expr {
            Some(v) => v,
            None => {
                return Default::default();
            }
        }
    };
    ($expr:expr; continue) => {
        match $expr {
            Some(v) => v,
            None => continue,
        }
    };

    ($expr:expr; $err:expr) => {
        match $expr {
            Some(v) => v,
            None => {
                return Err($err);
            }
        }
    };

    ($expr:expr; |$i:ident| $body:expr) => {
        match $expr {
            Ok(v) => v,
            Err(e) => {
                return (|$i| $body)(e);
            }
        }
    };

    ($expr:expr; |$i:ident| $body:expr; continue) => {
        match $expr {
            Ok(v) => v,
            Err(e) => {
                (|$i| $body)(e);
                continue;
            }
        }
    };
}
