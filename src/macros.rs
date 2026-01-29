#[macro_export]
/// Implement a transparent wrapper around an inner type:
///  (name, type, default (; meta)).
///
/// Implements Deref, DerefMut, FromStr, Display, PartialEq, Clone, Debug, Serialize, Deserialize.
macro_rules! impl_transparent_wrapper {
        ($(#[$meta:meta])* $name:ident, $inner:ty, $default:expr) => {
                $(#[$meta])*
                #[derive(Debug, Clone, Eq, serde::Serialize, serde::Deserialize)]
                #[serde(transparent)]
                pub struct $name(pub $inner);

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
                        type Err = std::num::ParseIntError;

                        fn from_str(s: &str) -> Result<Self, Self::Err> {
                                Ok($name(s.parse()?))
                        }
                }
                impl std::fmt::Display for $name {
                        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                                write!(f, "{}", self.0)
                        }
                }

                // standard
                impl PartialEq for $name {
                        fn eq(&self, other: &Self) -> bool {
                                self.0 == other.0
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
///  (name, type, default (; meta)).
///
/// Implements Deref, PartialEq, Clone, Debug, Serialize.
macro_rules! impl_restricted_wrapper {
        ($(#[$meta:meta])* $name:ident, $inner:ty, $default:expr) => {
                $(#[$meta])*
                #[derive(Debug, Clone, Eq, serde::Serialize)]
                #[serde(transparent)]
                pub struct $name($inner);

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

                // standard
                impl PartialEq for $name {
                        fn eq(&self, other: &Self) -> bool {
                                self.0 == other.0
                        }
                }
                impl std::ops::Deref for $name {
                        type Target = $inner;
                        fn deref(&self) -> &Self::Target { &self.0 }
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
/// The prefix ; is shorthand for ToString::to_string.
macro_rules! vec_ {
    ($($elem:expr),* $(,)?) => {
        vec![$($elem.into()),*]
    };
    (; $($elem:expr),*) => {
        vec![$($elem.to_string()),*]
    };
    ($f:expr; $($elem:expr),*) => {
        vec![$($type::from($elem)),*]
    };
}

#[macro_export]
/// Write newline-delimited strings directly to stdout
macro_rules! prints {
    ($($s:expr),+ $(,)?) => {{
        use std::io::{self, Write};
        let mut out = io::stdout().lock();
        $(
            let _ = out.write_all($s.as_bytes());
            let _ = out.write_all(b"\n");
        )+
    }};
}

#[macro_export]
// Easier than format! to concatenate strings
macro_rules! cats {
    ( $( $x:expr ),* $(,)? ) => {{
        let mut s = String::new();
        $(
            use std::fmt::Write;
            write!(&mut s, "{}", $x).unwrap();
        )*
        s
    }};
}

