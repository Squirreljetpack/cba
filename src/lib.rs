//! A collection of utilities which wrap common tasks needed in cli utilities
//!
//! # Logging & Bogging Quick Reference
//!
//! ### Standalone Macros
//! - `wbog!("msg")` / `wbog!("tag"; "msg")`: log at `WARN` to stderr and `log::warn!`.
//! - `ebog!("msg")` / `ebog!("tag"; "msg")`: log at `ERROR` to stderr and `log::error!`.
//! - `ibog!`, `dbog!`: `INFO` and `DEBUG`.
//!
//! ### Fallible Chaining on `Result<T, E: Display>` (`use cba::bog::BogOkExt;`)
//! - `res._wbog()`: logs `Err` as `WARN`, returns `Option<T>` (`Some(val)` or `None`).
//! - `res._ebog()`: logs `Err` as `ERROR`, returns `Option<T>`.
//! - `res.__ebog()`: logs `Err` as `ERROR`, exits process with code 1, returns `T`.
//!
//! ### Fallible Chaining on `Option<T>` (`use cba::bog::BogUnwrapExt;`)
//! - `opt._ebog("msg")`: if `None`, logs `"msg"` as `ERROR` and exits process with code 1, returns `T`.
//! - `opt.ebog("msg")`: if `None`, logs `"msg"` as `ERROR`, returns `Option<T>` (`None`).

pub mod bath; // Path manipulation
#[cfg(feature = "dirs")]
pub mod birs; // Platform application directories
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
