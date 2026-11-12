//! A collection of utilities which wrap common tasks needed in cli utilities
//! Results/Options are downgraded to Options/bools by handling errors within the wrappers using [`bog`].
//!
//! # Error handling strategies:
//! ### Macros
//! Unwrap errors from Result/Option with (get/unwrap_or) and immediately return
//! ### BogOkExt
//! Downgrade errors to options by bogging the error
//! ### BogUnwrapExt
//! Unwrap infallible errors or bog and exit process
//! ### Misc
//! A prefix can be added to the error with prefix_err
//!
//!
//! # Additional
//! These functions are mostly not composable

pub mod bath; // Path manipulation
#[cfg(feature = "blaim")]
pub mod blaim; // Atomic path reservation
pub mod bo; // File read/write

pub mod broc;
pub mod bs; // Filesystem check/set/read

/// Macros for building values (`vec_!`, `prints!`) and defining wrapper types.
pub mod baccarat;
/// Extension traits that log or discard errors, exit, and chain transforms.
pub mod bait;
/// Log level filters picked from `RUST_LOG` or CLI verbosity flags.
pub mod bother;
/// Numeric helpers on `f32` and on `usize` counters.
pub mod bum;

#[cfg(feature = "bring")]
/// Feature `bring`: string padding, splitting and table printing.
pub mod bring;

#[cfg(feature = "serde")]
/// Feature `serde`: helpers for `#[serde(with = ...)]`.
pub mod bird;
#[cfg(feature = "serde")]
pub use crate::bird as serde;

pub mod bog;
pub use bog::BOGGER;

pub mod vecmap;

use std::fmt;
#[derive(Debug, PartialEq, Eq)]
/// An error carrying an already formatted message.
pub struct StringError(pub String);

impl fmt::Display for StringError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for StringError {}

// cannot use T: Display bound unless specialization ig
impl<T: Into<String>> From<T> for StringError {
    fn from(s: T) -> Self {
        StringError(s.into())
    }
}
