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
pub mod bog; // log
pub mod broc;
pub mod bs; // Filesystem check/set/read
pub mod macros;
pub mod misc;
