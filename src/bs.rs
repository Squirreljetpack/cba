//! Filesystem set, check, read

use crate::bog::BogOkExt;
use crate::{ebog, get_or_err, ibog};
use std::cmp::Ordering;
use std::path::PathBuf;
use std::{
    fs::{self, DirEntry},
    path::Path,
};

// --------------- EXECUTABLE ---------------
/// Check if executable
pub fn is_executable(path: impl AsRef<Path>) -> bool {
    let path = path.as_ref();
    let error_prefix = format!("Failed to check executability of {path:?}");

    let metadata = get_or_err!(std::fs::metadata(path), error_prefix);

    #[cfg(unix)]
    {
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

pub fn set_executable(path: impl AsRef<Path>) -> bool {
    let path = path.as_ref();
    let error_prefix = format!("Failed set executability of {path:?}");

    #[cfg(windows)]
    {
        // determined by ext
        true
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let metadata = get_or_err!(std::fs::metadata(path), error_prefix);

        let mut perms = metadata.permissions();
        perms.set_mode(perms.mode() | 0o111); // add executable bits
        get_or_err!(fs::set_permissions(path, perms), error_prefix);
        true
    }
    #[cfg(not(any(unix, windows)))]
    {
        ebog!("{error_prefix}: unsupported platform.");
        false
    }
}

pub fn is_symlink(path: impl AsRef<Path>) -> bool {
    let path = path.as_ref();
    let error_prefix = format!("Failed to check metadata of {path:?}");

    let meta = get_or_err!(fs::symlink_metadata(path), error_prefix);
    meta.file_type().is_symlink()
}

pub fn symlink(src: impl AsRef<Path>, dst: impl AsRef<Path>) -> bool {
    let src = src.as_ref();
    let dst = dst.as_ref();
    let error_prefix = format!("Failed to check symlink {src:?} to {dst:?}");

    #[cfg(unix)]
    {
        use crate::misc::ResultExt;

        std::os::unix::fs::symlink(src, dst)
            .prefix_err(&error_prefix)
            .or_err()
            .is_some()
    }

    #[cfg(windows)]
    {
        let metadata = get_or_err!(std::fs::metadata(path), error_prefix);
        if metadata.is_dir() {
            windows_fs::symlink_dir(src, dst)
        } else {
            windows_fs::symlink_file(src, dst)
        }
        .prefix_err(&error_prefix)
        .or_err()
        .is_some()
    }
}

// ---------- DIRECTORIES -----------------
/// Use case: initialize configuration directories
pub fn create_dir(dir: impl AsRef<Path>) -> bool {
    let dir = dir.as_ref();
    if dir.as_os_str().is_empty() {
        ebog!("Failed to determine directory"); // i.e. state_dir().unwrap_or_default()
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

pub fn clear_directory(dir: impl AsRef<Path>, filter: impl Fn(&DirEntry) -> bool) -> bool {
    let path = dir.as_ref();
    let error_prefix = format!("Failed to clear directory at {path:?}");

    if !path.exists() {
        return true;
    }

    let entries = get_or_err!(fs::read_dir(path), error_prefix);

    for entry in entries {
        let entry = get_or_err!(entry, error_prefix);
        if !filter(&entry) {
            continue;
        }
        let path = entry.path();

        if path.is_dir() {
            get_or_err!(fs::remove_dir(&path), error_prefix)
        } else {
            get_or_err!(fs::remove_file(&path), error_prefix)
        }
    }
    true
}

#[easy_ext::ext(FsPathExt)]
pub impl<T: AsRef<Path>> T {
    fn is_empty_dir(&self) -> bool {
        let path = self.as_ref();
        fs::read_dir(path)
            .map(|mut entries| entries.next().is_none())
            .unwrap_or(false)
    }
}

//
pub fn sort_by_mtime(paths: &mut Vec<PathBuf>) {
    paths.sort_by(|a, b| {
        let ma = fs::metadata(a).and_then(|m| m.modified());
        let mb = fs::metadata(b).and_then(|m| m.modified());
        match (ma, mb) {
            (Ok(a), Ok(b)) => a.cmp(&b),
            (Ok(_), Err(_)) => Ordering::Less,
            (Err(_), Ok(_)) => Ordering::Greater,
            (Err(_), Err(_)) => Ordering::Equal,
        }
    });
}
