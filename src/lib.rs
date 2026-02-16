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
pub mod bo; // File read/write

pub mod broc;
pub mod bs; // Filesystem check/set/read

pub mod bait;
pub mod bother;
pub mod bum;
pub mod macros;

#[cfg(feature = "text")]
pub mod text;

#[cfg(feature = "serde")]
pub mod serde;

pub mod bog;
pub use bog::BOGGER;

use std::fmt;
#[derive(Debug, PartialEq, Eq)]
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
