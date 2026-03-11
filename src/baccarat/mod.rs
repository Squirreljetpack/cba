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
    ($($expr:expr),+ $(,)?) => {{
        $(
            #[cfg(debug_assertions)]
            let __val = ::std::dbg!($expr);
            #[cfg(not(debug_assertions))]
            let __val = $expr;
        )+
        __val
    }};

    ($prefix:expr; $s:expr) => {{
        let val = $s;
        ::log::trace!(concat!("{}: {:?}"), $prefix, &val);
        val
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

#[macro_export]
/// Info log in debug
macro_rules! _info {
    ($($arg:tt)*) => {
        #[cfg(debug_assertions)]
        {
            log::info!($($arg)*);
        }
    };
}
