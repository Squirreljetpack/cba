//! Display colored log-style messages for CLI tools
//! Performance is no concern (hence `bog`), only convenience and style.

mod fmt;
mod global;
mod macros;

pub use fmt::*;
pub use global::*;
#[allow(unused)]
pub use macros::*;
