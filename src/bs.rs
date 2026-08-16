//! Filesystem set, check, read
use crate::bait::ResultExt as _;
use crate::bog::BogOkExt;
use crate::{ebog, ibog, unwrap};
use std::io;
use std::path::{Component, PathBuf};
use std::{
    fs::{self, DirEntry},
    path::Path,
};

// --------------- EXECUTABLE ---------------
/// Check if executable
///
/// Prints error.
pub fn is_executable(path: impl AsRef<Path>) -> bool {
    let path = path.as_ref();
    let error_prefix = format!("Failed to check executability of {path:?}");

    #[cfg(unix)]
    {
        let metadata = unwrap!(std::fs::metadata(path).prefix(&error_prefix)._ebog());
        use std::os::unix::fs::PermissionsExt;
        metadata.permissions().mode() & 0o111 != 0
    }

    #[cfg(windows)]
    {
        let ext = path
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or_default()
            .to_ascii_lowercase();
        matches!(ext.as_str(), "exe" | "bat" | "cmd" | "com")
    }

    #[cfg(not(any(unix, windows)))]
    {
        ebog!("{error_prefix}: unsupported platform.");
        false
    }
}

/// Set executable.
/// # Example
/// ```rust,ignore
///     let error_prefix = format!("Failed set executability of {path:?}");
///     if symlink(src, dst)
///         .prefix_err(&error_prefix)
///         ._ebog()
///         .is_some() {
///     // success
///     }
///
/// ```
pub fn set_executable(path: &Path) -> Result<(), std::io::Error> {
    #[cfg(windows)]
    {
        // determined by ext
        // todo: improve
        Ok(())
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let metadata = std::fs::metadata(path)?;

        let mut perms = metadata.permissions();
        perms.set_mode(perms.mode() | 0o111); // add executable bits
        fs::set_permissions(path, perms)
    }
    #[cfg(not(any(unix, windows)))]
    {
        Ok(())
    }
}

/// Get permissions: [read, write, exec]
pub fn permissions(path: &Path) -> [bool; 3] {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let metadata = match std::fs::metadata(path) {
            Ok(m) => m,
            Err(_) => return [false; 3],
        };
        let mode = metadata.permissions().mode();
        [
            mode & 0o400 != 0, // read
            mode & 0o200 != 0, // write
            mode & 0o100 != 0, // exec
        ]
    }
    #[cfg(windows)]
    {
        let metadata = match std::fs::metadata(path) {
            Ok(m) => m,
            Err(_) => return [false; 3],
        };
        let readonly = metadata.permissions().readonly();
        let ext = path
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or_default()
            .to_ascii_lowercase();
        let executable = matches!(ext.as_str(), "exe" | "bat" | "cmd" | "com");
        [true, !readonly, executable]
    }
    #[cfg(not(any(unix, windows)))]
    {
        [false; 3]
    }
}

/// False if could not determine
///
/// Prints error.
pub fn is_symlink(path: impl AsRef<Path>) -> bool {
    let path = path.as_ref();
    let error_prefix = format!("Failed to check metadata of {path:?}");

    let meta = unwrap!(fs::symlink_metadata(path).prefix(&error_prefix)._ebog());
    meta.file_type().is_symlink()
}

/// Cross platform symlink creation (+ ancestors if needed).
/// # Example
/// ```rust,ignore
///     let error_prefix = format!("Failed to symlink {src:?} to {dst:?}");
///     if symlink(src, dst)
///         .prefix_err(&error_prefix)
///         ._ebog()
///         .is_some() {
///     // success
///     }
///
/// ```
pub fn symlink(
    src: impl AsRef<Path>,
    dst: impl AsRef<Path>,
    relative: bool,
) -> Result<(), std::io::Error> {
    let src = src.as_ref();
    let dst = dst.as_ref();

    if let Some(parent) = dst.parent() {
        std::fs::create_dir_all(parent)?;
    }

    let target = if relative {
        let base = dst.parent().unwrap_or(Path::new("."));
        unwrap!(
            diff_paths(src, base),
            std::io::Error::other("Unable to determine relative path")
        )
    } else {
        src.to_path_buf()
    };

    #[cfg(unix)]
    {
        std::os::unix::fs::symlink(&target, dst)?;
    }

    #[cfg(windows)]
    {
        use std::os::windows::fs::{symlink_dir, symlink_file};

        let meta = src.metadata()?;
        if meta.is_dir() {
            symlink_dir(&target, dst)?;
        } else {
            symlink_file(&target, dst)?;
        }
    }

    Ok(())
}

/// From https://docs.rs/pathdiff/latest/src/pathdiff/lib.rs.html#43-86
/// Construct a relative path from a provided base directory path to the provided path.
///
/// ```rust
/// use cba::bs::diff_paths;
/// use std::path::*;
///
/// assert_eq!(diff_paths("/foo/bar",      "/foo/bar/baz"),  Some("../".into()));
/// assert_eq!(diff_paths("/foo/bar/baz",  "/foo/bar"),      Some("baz".into()));
/// assert_eq!(diff_paths("/foo/bar/quux", "/foo/bar/baz"),  Some("../quux".into()));
/// assert_eq!(diff_paths("/foo/bar/baz",  "/foo/bar/quux"), Some("../baz".into()));
/// assert_eq!(diff_paths("/foo/bar",      "/foo/bar/quux"), Some("../".into()));
///
/// assert_eq!(diff_paths("/foo/bar",      "baz"),           Some("/foo/bar".into()));
/// assert_eq!(diff_paths("/foo/bar",      "/baz"),          Some("../foo/bar".into()));
/// assert_eq!(diff_paths("foo",           "bar"),           Some("../foo".into()));
///
/// assert_eq!(
///     diff_paths(&"/foo/bar/baz", "/foo/bar".to_string()),
///     Some("baz".into())
/// );
/// assert_eq!(
///     diff_paths(Path::new("/foo/bar/baz"), Path::new("/foo/bar").to_path_buf()),
///     Some("baz".into())
/// );
/// ```
pub fn diff_paths<P, B>(path: P, base: B) -> Option<PathBuf>
where
    P: AsRef<Path>,
    B: AsRef<Path>,
{
    let path = path.as_ref();
    let base = base.as_ref();

    if path.is_absolute() != base.is_absolute() {
        if path.is_absolute() {
            Some(PathBuf::from(path))
        } else {
            None
        }
    } else {
        let mut ita = path.components();
        let mut itb = base.components();
        let mut comps: Vec<Component> = vec![];
        loop {
            match (ita.next(), itb.next()) {
                (None, None) => break,
                (Some(a), None) => {
                    comps.push(a);
                    comps.extend(ita.by_ref());
                    break;
                }
                (None, _) => comps.push(Component::ParentDir),
                (Some(a), Some(b)) if comps.is_empty() && a == b => (),
                (Some(a), Some(b)) if b == Component::CurDir => comps.push(a),
                (Some(_), Some(b)) if b == Component::ParentDir => return None,
                (Some(a), Some(_)) => {
                    comps.push(Component::ParentDir);
                    for _ in itb {
                        comps.push(Component::ParentDir);
                    }
                    comps.push(a);
                    comps.extend(ita.by_ref());
                    break;
                }
            }
        }
        Some(comps.iter().map(|c| c.as_os_str()).collect())
    }
}

// ---------- DIRECTORIES -----------------
/// Create a directory if it doesn't exist.
///
/// Use case: initialize configuration directories.
///
/// Prints error and success.
pub fn create_dir(dir: impl AsRef<Path>) -> bool {
    let dir = dir.as_ref();
    if dir.as_os_str().is_empty() {
        ebog!("Failed to determine directory to create."); // i.e. state_dir().unwrap_or_default()
        return false;
    }

    if !dir.exists() {
        match std::fs::create_dir_all(dir) {
            Ok(_) => {
                ibog!("Created directory: {}", dir.display());
                true
            }
            Err(e) => {
                ebog!("Failed to create {:?}: {e}", dir);
                false
            }
        }
    } else {
        true
    }
}

/// Clear directory contents matching filter.
/// Retusn Ok if dir has no contents or does not exist.
///
/// # Example
/// ```rust,ignore
/// let path = "/path/to/dir";
/// let err_prefix = format!("Failed to clear directory at {path:?}");
/// clear_dir(&path, |entry| {
///    // filter condition
///   true
/// }).prefix_err(&err_prefix)._ebog();
/// ```
pub fn clear_dir(
    dir: impl AsRef<Path>,
    filter: impl Fn(&DirEntry) -> bool,
) -> Result<(), io::Error> {
    let path = dir.as_ref();

    if !path.exists() {
        return Ok(());
    }

    let entries = fs::read_dir(path)?;

    for entry in entries {
        let entry = entry?;
        if !filter(&entry) {
            continue;
        }
        let path = entry.path();

        if path.is_dir() {
            fs::remove_dir_all(&path)?;
        } else {
            fs::remove_file(&path)?;
        }
    }
    Ok(())
}

#[easy_ext::ext(FsPathExt)]
pub impl<T: AsRef<Path>> T {
    /// Check if directory is empty
    fn is_empty_dir(&self) -> bool {
        let path = self.as_ref();
        fs::read_dir(path)
            .map(|mut entries| entries.next().is_none())
            .unwrap_or(false)
    }
}
