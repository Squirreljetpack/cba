#[macro_export]
/// Implement a transparent wrapper around an inner type:
///
/// Implements Deref, DerefMut, FromStr, Display, Debug, PartialEq, Serialize, Deserialize.
///
/// # Example
/// ```rust
/// use cli_boilerplate_automation::define_transparent_wrapper;
///
/// #[cfg(feature = "serde")]
/// define_transparent_wrapper!(
///     #[derive(Copy)]
///     Count: u16 = 1
/// );
/// ```
macro_rules! define_transparent_wrapper {
    ($(#[$meta:meta])* $name:ident: $(#[$inner_meta:meta])* $inner:path = $default:expr) => {
        $(#[$meta])*
        #[derive(Debug, PartialEq, serde::Serialize, serde::Deserialize)]
        #[serde(transparent)]
        pub struct $name($(#[$inner_meta])* pub $inner);

        impl Default for $name {
            fn default() -> Self {
                $name($default)
            }
        }

        // Conversions
        impl From<$name> for $inner {
            fn from(c: $name) -> Self {
                c.0
            }
        }
        impl From<$inner> for $name {
            fn from(c: $inner) -> Self {
                Self(c)
            }
        }

        // string
        impl std::str::FromStr for $name {
            type Err = $crate::StringError;

            fn from_str(s: &str) -> Result<Self, Self::Err> {
                let inner = s.parse::<$inner>().map_err(|e| e.to_string())?;
                Ok($name(inner))
            }
        }
        impl std::fmt::Display for $name {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                write!(f, "{}", self.0)
            }
        }

        impl std::ops::Deref for $name {
            type Target = $inner;
            fn deref(&self) -> &Self::Target { &self.0 }
        }
        impl std::ops::DerefMut for $name {
            fn deref_mut(&mut self) -> &mut Self::Target { &mut self.0 }
        }
    };
}

#[macro_export]
/// Implement a restricted wrapper around an inner type:
///
/// Implements Deref, PartialEq, Clone, Debug, Serialize.
///
/// # Example
/// ```rust
/// use cli_boilerplate_automation::define_restricted_wrapper;
///
/// #[cfg(feature = "serde")] {
///     define_restricted_wrapper!(Percentage: u16 = 100);
///     impl Percentage {
///         pub fn new(value: u16) -> Self {
///             if value <= 100 { Self(value) } else { Self(100) }
///         }
///     }
/// }
///
/// ```
macro_rules! define_restricted_wrapper {
    ($(#[$meta:meta])* $name:ident: $(#[$inner_meta:meta])* $inner:path = $default:expr) => {
        $(#[$meta])*
        #[derive(Debug, PartialEq, serde::Serialize)]
        #[serde(transparent)]
        pub struct $name($(#[$inner_meta])* $inner);

        impl $name {
            pub fn inner(&self) -> $inner {
                self.0.clone()
            }
        }

        impl Default for $name {
            fn default() -> Self {
                $name($default)
            }
        }

        impl From<$name> for $inner {
            fn from(c: $name) -> Self {
                c.0
            }
        }

        impl std::ops::Deref for $name {
            type Target = $inner;
            fn deref(&self) -> &Self::Target { &self.0 }
        }
    };
}

#[macro_export]
/// Implement a wrapper around a container type (i.e. HashMap).
/// Implements the Deref, DerefMut, Default and IntoIterator/FromIterator traits and the new function.
///
/// ```rust
/// use cli_boilerplate_automation::define_collection_wrapper;
/// pub struct Module {};
/// define_collection_wrapper!(
///     #[cfg_attr(feature = "serde", derive(Debug, serde::Serialize, serde::Deserialize))]
///     Modules: std::collections::HashMap<String, Module>
/// );
/// ```
macro_rules! define_collection_wrapper {
    ($(#[$meta:meta])* $name:ident: $(#[$inner_meta:meta])* $inner:path) => {
        $(#[$meta])*
        pub struct $name($(#[$inner_meta])* $inner);

        impl $name {
            pub fn new() -> Self {
                Self(<$inner>::new())
            }
        }

        impl std::ops::Deref for $name {
            type Target = $inner;

            fn deref(&self) -> &Self::Target {
                &self.0
            }
        }

        impl std::ops::DerefMut for $name {
            fn deref_mut(&mut self) -> &mut Self::Target {
                &mut self.0
            }
        }

        impl Default for $name {
            fn default() -> Self {
                Self(<$inner>::new())
            }
        }

        impl From<$name> for $inner {
            fn from(c: $name) -> Self {
                c.0
            }
        }
        impl From<$inner> for $name {
            fn from(c: $inner) -> Self {
                Self(c)
            }
        }

        impl IntoIterator for $name {
            type Item = <$inner as IntoIterator>::Item;
            type IntoIter = <$inner as IntoIterator>::IntoIter;

            fn into_iter(self) -> Self::IntoIter {
                self.0.into_iter()
            }
        }

        impl<'a> IntoIterator for &'a $name {
            type Item = <&'a $inner as IntoIterator>::Item;
            type IntoIter = <&'a $inner as IntoIterator>::IntoIter;

            fn into_iter(self) -> Self::IntoIter {
                (&self.0).into_iter()
            }
        }

        // impl<'a> IntoIterator for &'a mut $name {
        //     type Item = <&'a mut $inner as IntoIterator>::Item;
        //     type IntoIter = <&'a mut $inner as IntoIterator>::IntoIter;

        //     fn into_iter(self) -> Self::IntoIter {
        //         (&mut self.0).into_iter()
        //     }
        // }

        impl FromIterator<<$inner as IntoIterator>::Item> for $name {
            fn from_iter<I: IntoIterator<Item = <$inner as IntoIterator>::Item>>(iter: I) -> Self {
                Self(iter.into_iter().collect())
            }
        }
    };
}

// ------------- DEBUG -------------

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

#[macro_export]
/// Info log in debug
macro_rules! _log {
    ($($arg:tt)*) => {
        #[cfg(debug_assertions)]
        {
            log::info!($($arg)*);
        }
    };
}

// -------------------------------

#[macro_export]
/// Map a function over the elements of a vec![].
/// By default, .into() is applied.
/// To specify the mapping, a function or type followed by `|` can precede the elements.
/// The pure prefix | is shorthand for ToString::to_string.
macro_rules! vec_ {
    ($($elem:expr),* $(,)?) => {
        vec![$($elem.into()),*]
    };
    (| $($elem:expr),*) => {
        vec![$($elem.to_string()),*]
    };
    ($t:ty | $($elem:expr),*) => {
        vec![$($t::from($elem)),*]
    };
    ($f:ident | $($elem:expr),*) => {
        vec![$($f($elem)),*]
    };
}

#[macro_export]
/// Map a function over elements of [] and collect.
/// By default, .into() is applied.
/// To specify the mapping, a function or type followed by `|` can precede the elements.
/// The pure prefix | is shorthand for ToString::to_string.
macro_rules! collect_ {
    ($($elem:expr),* $(,)?) => {
        [$($elem.into()),*].into_iter().collect()
    };
    (| $($elem:expr),*) => {
        [$($elem.to_string()),*].into_iter().collect()
    };
    ($t:ty | $($elem:expr),*) => {
        [$($t::from($elem)),*].into_iter().collect()
    };
    ($f:ident | $($elem:expr),*) => {
        [$($f($elem)),*].into_iter().collect()
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
