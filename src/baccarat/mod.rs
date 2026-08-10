mod define;
// pub use define::*;
mod types;
// pub use types::*;

// -------------------------------

#[macro_export]
/// Map a function over the elements of a vec![].
/// By default, .to_string() is applied.
/// To specify the mapping, a function or type followed by `|` can precede the elements.
/// The pure prefix `:` is shorthand for calling .into().
macro_rules! vec_ {
    ($($elem:expr),* $(,)?) => {
        vec![$($elem.to_string()),*]
    };
    (: $($elem:expr),*) => {
        vec![$($elem.into()),*]
    };
    ($t:ty: $($elem:expr),*) => {
        vec![$(
            < $t as ::std::convert::From<_> >::from($elem)
        ),*]
    };
    ($f:ident : $($elem:expr),*) => {
        vec![$($f($elem)),*]
    };
}

#[macro_export]
/// Map a function over the elements of a [].
/// By default, .to_string() is applied.
/// To specify the mapping, a function or type followed by `|` can precede the elements.
/// The pure prefix `:` is shorthand for calling .into().
macro_rules! slice_ {
    ($($elem:expr),* $(,)?) => {
        [$($elem.to_string()),*]
    };
    (: $($elem:expr),*) => {
        [$($elem.into()),*]
    };
    ($t:ty : $($elem:expr),*) => {
        [$(
            < $t as ::std::convert::From<_> >::from($elem)
        ),*]
    };
    ($f:ident : $($elem:expr),*) => {
        [$($f($elem)),*]
    };
}

#[macro_export]
/// Write newline-delimited strings directly to stdout without dynamic dispatch.
///
/// # Note
/// The `;` delimiter inserts new-lines, while `,` does not.
/// A trailing newline is always added.
macro_rules! prints {
    // write to custom buffer
    ($( $( $s:expr ),+ );* => $buf:expr) => {{
        use std::io::Write;
        $(
            $(
                let _ = $buf.write_all($s.as_bytes());
            )+
            let _ = $buf.write_all(b"\n");
        )*
    }};

    // default: stdout
    ( $( $( $s:expr ),+ );* ) => {{
        use std::io::{self, Write};
        let mut out = io::stdout().lock();

        $(
            $(
                let _ = out.write_all($s.as_bytes());
            )+
            let _ = out.write_all(b"\n");
        )*
    }};
}

#[macro_export]
// Easier than format! for concatenating strings
macro_rules! concat_ {
    ( $( $x:expr ),* $(,)? ) => {{
        use std::fmt::Write;
        let mut s = String::new();
        $(
            write!(&mut s, "{}", $x).unwrap();
        )*
        s
    }};
}

#[cfg(test)]
mod tests {
    #[test]
    fn writes_to_buffer() {
        let mut buf = Vec::new();

        prints!(
            "hello", " world";
            "line 2";
            "a", "b", "c" => &mut buf
        );

        assert_eq!(
            std::str::from_utf8(&buf).unwrap(),
            "hello world\nline 2\nabc\n"
        );

        let mut buf = Vec::new();

        prints!(
            "hello", " world" => &mut buf
        );

        assert_eq!(std::str::from_utf8(&buf).unwrap(), "hello world\n");
    }
}

// ------------- DEBUG -------------

/// dbg!/log::debug! and return the value.
/// - expr: dbg, but only in debug builds.
/// - prefix, expr: log::trace!("{prefix}: {:?}")
#[macro_export]
macro_rules! _dbg {
    ($($expr:expr);+ $(;)?) => {{
        $(
            #[cfg(debug_assertions)]
            let __val = ::std::dbg!($expr);
            #[cfg(not(debug_assertions))]
            let __val = $expr;
        )+
        __val
    }};

    ($prefix:expr, $s:expr) => {{
        let val = $s;
        ::log::trace!(concat!("{}: {:?}"), $prefix, &val);
        val
    }};
}

/// if cfg(debug_assertions), log::info expressions, "key": v and "literals".
/// One per line, separated by `;`.
#[macro_export]
macro_rules! _info {
    (@munch ($($format:expr),*) ($($values:expr),*) $label:literal , $expr:expr ; $($tail:tt)*) => {
        $crate::_info!(@munch ($($format,)* "\n", $label, " = {:?}") ($($values,)* $expr) $($tail)*)
    };

    (@munch ($($format:expr),*) ($($values:expr),*) $label:literal : $expr:expr ; $($tail:tt)*) => {
        $crate::_info!(@munch ($($format,)* "\n", $label, " = {:?}") ($($values,)* $expr) $($tail)*)
    };

    (@munch ($($format:expr),*) ($($values:expr),*) $msg:literal ; $($tail:tt)*) => {
        $crate::_info!(@munch ($($format,)* "\n", $msg) ($($values),*) $($tail)*)
    };

    (@munch ($($format:expr),*) ($($values:expr),*) $expr:expr ; $($tail:tt)*) => {
        $crate::_info!(@munch ($($format,)* "\n", stringify!($expr), " = {:?}") ($($values,)* $expr) $($tail)*)
    };

    (@munch ($($format:expr),*) ($($values:expr),*) $(;)?) => {
        log::info!(concat!($($format),*), $($values),*)
    };

    ($($args:tt)*) => {{
        #[cfg(debug_assertions)]
        {
            $crate::_info!(@munch () () $($args)* ;);
        }
    }};
}

/// log::trace expressions, "key": v and "literals".
/// One per line, separated by `;`.
#[macro_export]
macro_rules! _trace {
    (@munch ($($format:expr),*) ($($values:expr),*) $label:literal , $expr:expr ; $($tail:tt)*) => {
        $crate::_trace!(@munch ($($format,)* "\n", $label, " = {:?}") ($($values,)* $expr) $($tail)*)
    };

    (@munch ($($format:expr),*) ($($values:expr),*) $label:literal : $expr:expr ; $($tail:tt)*) => {
        $crate::_trace!(@munch ($($format,)* "\n", $label, " = {:?}") ($($values,)* $expr) $($tail)*)
    };

    (@munch ($($format:expr),*) ($($values:expr),*) $msg:literal ; $($tail:tt)*) => {
        $crate::_trace!(@munch ($($format,)* "\n", $msg) ($($values),*) $($tail)*)
    };

    (@munch ($($format:expr),*) ($($values:expr),*) $expr:expr ; $($tail:tt)*) => {
        $crate::_trace!(@munch ($($format,)* "\n", stringify!($expr), " = {:?}") ($($values,)* $expr) $($tail)*)
    };

    (@munch ($($format:expr),*) ($($values:expr),*) $(;)?) => {
        log::trace!(concat!($($format),*), $($values),*)
    };

    ($($args:tt)*) => {{
        {
            $crate::_trace!(@munch () () $($args)* ;);
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

// ----------------------

// Sometimes easier to read than let else

/// # Examples
///
/// Unwrap Option or return Default::default():
/// ```rust
/// use cba::unwrap;
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
#[macro_export]
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

    // this is a special case, so we use comma
    ($expr:expr, $err:expr) => {
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
                return (|$i: String| $body)(e.to_string());
            }
        }
    };

    ($expr:expr; |$i:ident: $ty:ty| $body:expr) => {
        match $expr {
            Ok(v) => v,
            Err(e) => {
                return (|$i: $ty| $body)(e);
            }
        }
    };

    ($expr:expr; $v:expr) => {
        match $expr {
            Some(v) => v,
            None => {
                return $v;
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
