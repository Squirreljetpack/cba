//! Path manipulation

use std::path::{Component, Path, PathBuf};

use crate::bog::BogUnwrapExt;

/// Get the (lossy) basename of a valid path
/// Exits if path terminates in ..
pub fn basename(path: &Path) -> Cow<'_, str> {
    let err_prefix = format!("Failed to determine filename of {path:?}");
    path.file_name().or_err(&err_prefix).to_string_lossy()
}

/// Split path around last '.'
pub fn split_ext(p: &str) -> [&str; 2] {
    match p.rfind('.') {
        Some(0) | None => [p, ""],
        Some(idx) if idx + 1 < p.len() => [&p[..idx], &p[idx + 1..]],
        Some(idx) => [&p[..idx], ""], // dot is last character
    }
}

pub fn root_dir() -> PathBuf {
    PathBuf::from(std::path::MAIN_SEPARATOR_STR)
}

#[easy_ext::ext(PathExt)]
pub impl<T: AsRef<Path>> T {
    /// Get the owned (lossy) basename of a valid path
    /// Exits if path terminates in ..
    fn basename(&self) -> String {
        let path = self.as_ref();
        basename(path).to_string()
    }

    fn len(&self) -> usize {
        self.as_ref().normalize().iter().count()
    }

    fn is_hidden(&self) -> bool {
        let path = self.as_ref();
        path.normalize().file_name()
        .map(|os_str| !os_str.to_string_lossy().starts_with('.'))
        .unwrap_or(false)
    }

    /// Prepend base to current path then normalize.
    ///
    /// # Example
    /// ```rust
    /// use std::path::Path;
    /// use cli_boilerplate_automation::{bog::{BogOkExt, BogUnwrapExt},bath::PathExt};
    ///
    /// let path = Path::new("");
    /// path.abs(std::env::current_dir().or_err().or_exit());
    /// ```
    fn abs(&self, base: impl AsRef<Path>) -> PathBuf {
        let path = self.as_ref();
        let base = base.as_ref();

        if path.is_absolute() {
            path.to_path_buf()
        } else {
            base.join(path)
        }
        .normalize()
    }

    fn is_empty(&self) -> bool {
        let path = self.as_ref();
        path.components().next().is_none()
    }

    /// clean path logically (so that all components are [`Component::Normal`])
    fn normalize(&self) -> PathBuf {
        let path = self.as_ref();
        let mut components = path.components().peekable();
        // keep the prefix
        let mut ret = if let Some(c @ Component::Prefix(..)) = components.peek().cloned() {
            components.next();
            PathBuf::from(c.as_os_str())
        } else {
            PathBuf::new()
        };

        for component in components {
            match component {
                Component::Prefix(..) => unreachable!(),
                Component::RootDir => {
                    ret.push(component.as_os_str());
                }
                Component::CurDir => {}
                Component::ParentDir => {
                    ret.pop();
                }
                Component::Normal(c) => {
                    ret.push(c);
                }
            }
        }
        ret
    }
}

/// Cache the expression into a fn() -> &'static Path
#[macro_export]
macro_rules! expr_as_path_fn {
    ($fn_name:ident, $expr:expr) => {
        paste::paste! {
            pub fn [<$fn_name>]() -> &'static std::path::Path {
                static VALUE: std::sync::LazyLock<std::path::PathBuf> = std::sync::LazyLock::new(|| {
                    $expr.into()
                });
                &VALUE
            }
        }
    };
}

use std::borrow::Cow;
use std::ffi::{OsStr, OsString};

#[cfg(unix)]
pub fn os_str_to_bytes(string: &OsStr) -> Cow<'_, [u8]> {
    use std::os::unix::ffi::OsStrExt;
    Cow::Borrowed(string.as_bytes())
}

#[cfg(windows)]
pub fn os_str_to_bytes(string: &OsStr) -> Cow<'_, [u8]> {
    use std::os::windows::ffi::OsStrExt;
    let bytes = string.encode_wide().flat_map(u16::to_le_bytes).collect();
    Cow::Owned(bytes)
}

#[cfg(unix)]
pub fn bytes_to_os_string(bytes: Vec<u8>) -> OsString {
    use std::os::unix::ffi::OsStringExt;
    OsString::from_vec(bytes)
}

#[cfg(windows)]
pub fn bytes_to_os_string(bytes: Vec<u8>) -> OsString {
    use std::os::windows::ffi::OsStringExt;

    debug_assert!(bytes.len() % 2 == 0, "invalid UTF-16 byte length");

    let wide: Vec<u16> = bytes
    .chunks_exact(2)
    .map(|c| u16::from_le_bytes([c[0], c[1]]))
    .collect();

    OsString::from_wide(&wide)
}
