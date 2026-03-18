//! Path manipulation

use crate::StringError;
use crate::bait::ResultExt;
use std::path::{Component, Path, PathBuf};

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
    /// Convert to str, or provide useful error.
    fn filename(&self) -> Result<&str, StringError> {
        let path = self.as_ref();

        let err_prefix = format!("Failed to determine basename of {path:?}");
        path.file_name()
            .ok_or(StringError(err_prefix.clone()))?
            .to_str()
            .ok_or(StringError(err_prefix))
    }

    /// Get the owned (lossy) basename of a valid path (for display purposes).
    /// Returns the original if path has no filename.
    fn basename(&self) -> String {
        let path = self.as_ref();
        match path.file_name() {
            Some(s) => s.to_string_lossy(),
            None => path.to_string_lossy(),
        }
        .to_string()
    }

    fn display_short(&self, home_dir: &Path) -> String {
        let path = self.as_ref();
        if let Ok(stripped) = path.strip_prefix(home_dir) {
            PathBuf::from("~").join(stripped).to_string_lossy().into()
        } else {
            path.to_string_lossy().into()
        }
    }

    fn len(&self) -> usize {
        self.as_ref().normalize().iter().count()
    }

    /// Robustly determine whether a file is hidden
    fn is_hidden(&self) -> bool {
        let mut skip = 0;

        for c in self.as_ref().components().rev() {
            match c {
                Component::ParentDir => {
                    skip += 1;
                }
                Component::CurDir => {}
                Component::Normal(name) => {
                    if skip > 0 {
                        skip -= 1;
                        continue;
                    }
                    return name.as_encoded_bytes().first() == Some(&b'.');
                }
                _ => {}
            }
        }

        false
    }

    /// Prepend base to current path then normalize.
    ///
    /// # Example
    /// ```rust
    /// use std::path::Path;
    /// use cba::{bog::{BogOkExt, BogUnwrapExt}, bath::PathExt, bait::OptionExt};
    ///
    /// let path = Path::new("");
    /// path.abs(std::env::current_dir()._ebog().or_exit());
    /// ```
    fn abs(&self, base: impl AsRef<Path>) -> PathBuf {
        let path = self.as_ref();
        let base = base.as_ref();

        base.join(path).normalize()
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

    /// Quotes the path.
    /// Returns None if not Windows or Unix or not UTF-8.
    fn shell_quote(&self) -> Option<String> {
        let Some(s) = self.as_ref().to_str() else {
            return None;
        };

        if cfg!(windows) {
            // Windows CMD: wrap in double quotes, escape internal quotes by doubling them
            // e.g., C:\Path "With" Quotes -> "C:\Path ""With"" Quotes"
            let escaped = s.replace('"', "\"\"");
            Some(format!("\"{}\"", escaped))
        } else if cfg!(unix) {
            // Unix shells: wrap in single quotes, escape internal single quotes
            // e.g., /path/it's/here -> '/path/it'\''s/here'
            let escaped = s.replace('\'', r"'\''");
            Some(format!("'{}'", escaped))
        } else {
            None
        }
    }
}

pub fn __root() -> Option<PathBuf> {
    let cwd = std::env::current_dir().ok()?;

    let mut root = PathBuf::new();

    for c in cwd.components() {
        match c {
            Component::Prefix(p) => root.push(p.as_os_str()),
            Component::RootDir => {
                root.push(c.as_os_str());
                return Some(root);
            }
            _ => break,
        }
    }

    if root.as_os_str().is_empty() {
        None
    } else {
        Some(root)
    }
}

pub fn shell_quote_impl(s: &str) -> String {
    if cfg!(windows) {
        // Windows CMD: wrap in double quotes, escape internal quotes by doubling them
        // e.g., C:\Path "With" Quotes -> "C:\Path ""With"" Quotes"
        let escaped = s.replace('"', "\"\"");
        format!("\"{}\"", escaped)
    } else if cfg!(unix) {
        // Unix shells: wrap in single quotes, escape internal single quotes
        // e.g., /path/it's/here -> '/path/it'\''s/here'
        let escaped = s.replace('\'', r"'\''");
        format!("'{}'", escaped)
    } else {
        s.to_string()
    }
}

// ----------------------

/// Cache the expression into a fn() -> &'static Path
#[macro_export]
macro_rules! expr_as_path_fn {
    ($fn_name:ident, $expr:expr) => {
        pub fn $fn_name() -> &'static std::path::Path {
            static VALUE: std::sync::LazyLock<std::path::PathBuf> =
                std::sync::LazyLock::new(|| $expr.into());
            &VALUE
        }
    };
}

// ----------------------

use std::borrow::Cow;
use std::ffi::{OsStr, OsString};

/// not sure if as_encoded_bytes is better
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

/// to_string_lossy for any type AsRef<OsStr>.
/// Note: (Intent is that it's possibly useful for macros).
pub fn to_string_lossy(s: &impl AsRef<std::ffi::OsStr>) -> std::borrow::Cow<'_, str> {
    s.as_ref().to_string_lossy()
}

// ----------------------
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum RenamePolicy {
    WrappedInc(String, String),
    // WrappedSuffix(&'static str, &'static str),
    // RepeatedPrefix,
    Replace, // don't check
}

// impl RenamePolicy {
//     pub const DEFAULT: Self = Self::WrappedInc("_", "");
// }

impl Default for RenamePolicy {
    fn default() -> Self {
        Self::WrappedInc("_".into(), "".into())
    }
}

// Requires: src is a normalized path with a filename
// If dest ends with a slash, target becomes dest/src_name
pub fn auto_dest_for_src(
    src: impl AsRef<Path>,
    dest: impl AsRef<OsStr>,
    method: &RenamePolicy,
) -> PathBuf {
    let src = src.as_ref();
    let dest = dest.as_ref();

    let put_into_dest =
        dest.is_empty() || dest.to_string_lossy().ends_with(std::path::MAIN_SEPARATOR);
    let dest_path = Path::new(dest).normalize();

    let initial_dest = if put_into_dest || dest_path.file_name().is_none() {
        let name = src
            .file_name()
            .expect("Could not determine a valid destination: missing file_name.");
        dest_path.join(name)
    } else {
        dest_path
    };

    match method {
        RenamePolicy::Replace => {
            return initial_dest;
        }
        RenamePolicy::WrappedInc(prefix, suffix) => {
            if !initial_dest.exists() {
                return initial_dest;
            }

            let parent = initial_dest.parent().unwrap_or(&initial_dest);
            let s = initial_dest.filename()._elog().unwrap_or_default();
            let [stem, ext] = split_ext(&s);

            for i in 1usize.. {
                let candidate: PathBuf = parent.join(if ext.is_empty() {
                    format!("{stem}{prefix}{i}{suffix}")
                } else {
                    format!("{stem}{prefix}{i}{suffix}.{ext}")
                });

                if !candidate.exists() {
                    return candidate;
                }
            }
            unreachable!()
        }
    }
}
