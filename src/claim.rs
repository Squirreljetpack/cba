//! Leaky abstraction for exclusive ownership of filesystem resources addressed by path.
//!
//! Because path-based resources in a filesystem can be concurrently modified by other
//! processes or threads, exclusive ownership abstractions are inherently leaky. This module
//! provides atomic placeholders and operations to that perform some rudimentary checks against races:
//! - **Requesting a free resource**: Reserves a name by creating an atomic placeholder
//!   (`O_CREAT|O_EXCL` for files, `create_dir` for directories), creating any missing ancestor
//!   directories and holding open file descriptors. Supports [`ClaimPolicy::Strict`],
//!   [`ClaimPolicy::Suffix`], and [`ClaimPolicy::Ignore`].
//! - **Safe replacement ([`FileClaim::try_replace`], [`DirClaim::try_replace`])**: Replaces the
//!   claimed placeholder with a source path. When replacing directories under `ClaimPolicy::Ignore`,
//!   existing contents are cleared beforehand; when replacing empty directories, emptiness is
//!   verified before renaming.
//! - **Rollback ([`FileClaim::rollback`], [`DirClaim::rollback`])**: Safely undoes the reservation,
//!   removing untouched placeholders and unwinding any ancestor directory levels invented during claiming.
//! - **Guarded consumption ([`FileClaim::into_parts`], [`DirClaim::into_path`])**: Validates that
//!   the reservation is still intact upon consumption. If an empty directory is no longer empty,
//!   returns None (without rolling back).

use std::fs;
use std::io;
use std::os::unix::fs::MetadataExt;
use std::path::{Path, PathBuf};

/// How many suffixed alternatives to try before giving up
/// (`name`, `name_1`, `name_2`, ...).
const MAX_ALTERNATIVES: u32 = 9999;

/// Policy for handling name conflicts during reservation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClaimPolicy<'a> {
    /// Fails if the exact path already exists.
    Strict,
    /// Finds the first free `<stem><left>N<right>[.<ext>]` sibling.
    Suffix { left: &'a str, right: &'a str },
    /// Reuses/overwrites the target path even if it already exists.
    Ignore,
}

impl<'a> Default for ClaimPolicy<'a> {
    fn default() -> Self {
        Self::Suffix {
            left: "_",
            right: "",
        }
    }
}

/// A reserved file name, held open: the reservation and the write target
/// are the same object, so nothing can slip between claiming and writing.
/// Dropping it closes the file (contents persist).
///
/// Records how many ancestor directories the reservation had to invent
/// below the desired path's parent ([`reserve_file_all`]); zero when every
/// ancestor already existed.
#[derive(Debug)]
pub struct FileClaim {
    path: PathBuf,
    file: fs::File,
    levels: usize,
}

/// A reserved directory name (`create_dir` is atomic by itself; there is
/// no handle to hold). Invented ancestor levels as on [`FileClaim`].
#[derive(Debug)]
pub struct DirClaim {
    path: PathBuf,
    levels: usize,
    empty: bool,
}

impl FileClaim {
    /// Mutable access to the reserved file (write, `set_len`, `set_permissions`, sync).
    pub fn file(&mut self) -> &mut fs::File {
        &mut self.file
    }

    /// Consumes the claim, returning the won path and the held handle.
    ///
    /// No validation actually happens.
    pub fn into_parts(self) -> Option<(PathBuf, fs::File)> {
        Some((self.path, self.file))
    }

    /// Validates whether the open file handle still matches the file at `self.path`.
    fn is_still_valid(&self) -> io::Result<bool> {
        let handle_meta = self.file.metadata()?;
        let path_meta = fs::symlink_metadata(&self.path)?;

        #[cfg(unix)]
        {
            Ok(handle_meta.dev() == path_meta.dev() && handle_meta.ino() == path_meta.ino())
        }
        #[cfg(windows)]
        {
            // On Windows, compare file size, attributes, and creation/write timestamps
            // as standard MetadataExt fields, or use volume/index attributes via winapi if preferred.
            Ok(handle_meta.file_size() == path_meta.file_size()
                && handle_meta.creation_time() == path_meta.creation_time()
                && handle_meta.last_write_time() == path_meta.last_write_time())
        }

        #[cfg(not(any(unix, windows)))]
        {
            // Fallback: check file length and modification time
            Ok(handle_meta.len() == path_meta.len()
                && handle_meta.modified().ok() == path_meta.modified().ok())
        }
    }
    
    /// Moves `source` onto the claimed name, replacing the placeholder.
    /// See [`DirClaim::try_replace`] for the shared semantics.
    pub fn try_replace(self, source: impl AsRef<Path>) -> Replaced<Self> {
        let source = source.as_ref();

        // 1. Verify the handle still matches the target path before replacing
        match self.is_still_valid() {
            Ok(true) => {}
            Ok(false) => {
                return Replaced::Failed(
                    io::Error::new(
                        io::ErrorKind::NotFound,
                        "claimed file handle no longer matches target path",
                    ),
                    self,
                );
            }
            Err(e) => return Replaced::Failed(e, self),
        }

        // 2. Windows requires closing or releasing open handles before overwriting
        #[cfg(windows)]
        let (file, path) = (self.file, self.path);
        #[cfg(windows)]
        drop(file);

        #[cfg(not(windows))]
        let path = &self.path;

        // 3. Attempt the rename
        match fs::rename(source, &path) {
            Ok(()) => Replaced::Success,
            Err(e) => {
                #[cfg(windows)]
                {
                    // Rollback: recreate the placeholder file if the rename failed
                    let file = fs::OpenOptions::new()
                        .read(true)
                        .write(true)
                        .create(true)
                        .truncate(false)
                        .open(&path)
                        .unwrap_or_else(|_| fs::File::create(&path).unwrap());

                    Replaced::Failed(
                        e,
                        Self {
                            path,
                            file,
                            levels: self.levels,
                        },
                    )
                }

                #[cfg(not(windows))]
                Replaced::Failed(e, self)
            }
        }
    }


    /// Undoes the reservation: removes the claimed leaf — only while it
    /// is still zero-length — and then unwinds the ancestor levels it
    /// invented. Callers roll back like this when a step after the
    /// reservation fails.
    pub fn rollback(self) {
        let untouched = fs::symlink_metadata(&self.path)
            .map(|md| md.len() == 0)
            .unwrap_or(false);
        if untouched {
            let _ = fs::remove_file(&self.path);
        }
        rollback(&self.path, self.levels);
    }

    fn with_levels(mut self, levels: usize) -> Self {
        self.levels = levels;
        self
    }
}

impl DirClaim {
    /// Whether this claim created a fresh empty directory placeholder.
    pub fn empty(&self) -> bool {
        self.empty
    }

    /// Clears the contents of the directory if it was pre-existing; no-op
    /// if it was freshly created empty.
    pub fn clear_contents(&self) -> io::Result<()> {
        if !self.empty {
            crate::bs::clear_dir(&self.path, |_| true)?;
        }
        Ok(())
    }

    /// Consumes the claim and returns the reserved path.
    ///
    /// If the directory was claimed empty (`self.empty == true`), verifies that the directory
    /// remains empty. If it is no longer empty, returns `None` without rolling back.
    pub fn into_path(self) -> Option<PathBuf> {
        if self.empty {
            match fs::read_dir(&self.path) {
                Ok(mut entries) => {
                    if entries.next().is_some() {
                        return None;
                    }
                }
                Err(_) => {
                    return None;
                }
            }
        }
        Some(self.path)
    }

    /// Moves `source` onto the claimed name, replacing the placeholder.
    /// If the directory was claimed over an existing one (`!self.empty`),
    /// its contents are cleared before renaming.
    pub fn try_replace(self, source: impl AsRef<Path>) -> Replaced<Self> {
        if self.empty {
            match fs::read_dir(&self.path) {
                Ok(mut entries) => {
                    if entries.next().is_some() {
                        return Replaced::Failed(
                            io::Error::new(
                                io::ErrorKind::DirectoryNotEmpty,
                                "directory is not empty",
                            ),
                            self,
                        );
                    }
                }
                Err(e) => return Replaced::Failed(e, self),
            }
        } else if let Err(e) = self.clear_contents() {
            return Replaced::Failed(e, self);
        }

        #[cfg(windows)]
        if let Err(e) = fs::remove_dir(&self.path) {
            return Replaced::Failed(e, self);
        }
        match fs::rename(source.as_ref(), &self.path) {
            Ok(()) => Replaced::Success,
            Err(e) => {
                #[cfg(windows)]
                let _ = fs::create_dir(&self.path);
                Replaced::Failed(e, self)
            }
        }
    }

    /// Undoes the reservation: removes the claimed leaf while it is still
    /// empty (`remove_dir` refuses otherwise) and then unwinds the
    /// ancestor levels it invented.
    pub fn rollback(self) {
        if self.empty {
            let _ = fs::remove_dir(&self.path);
        }
        rollback(&self.path, self.levels);
    }

    fn with_levels(mut self, levels: usize) -> Self {
        self.levels = levels;
        self
    }
}

/// Unwinds `levels` invented ancestors of `path`, deepest-first, stopping
/// at the first level that is no longer empty.
fn rollback(path: &Path, levels: usize) {
    let mut cur = match path.parent() {
        Some(p) if !p.as_os_str().is_empty() => p.to_path_buf(),
        _ => return,
    };
    for _ in 0..levels {
        // a level that gained content stays put, and everything above it
        // does too
        if fs::remove_dir(&cur).is_err() {
            break;
        }
        match cur.parent() {
            Some(p) if !p.as_os_str().is_empty() => cur = p.to_path_buf(),
            _ => break,
        }
    }
}

/// Why a reservation did not happen.
#[derive(Debug)]
pub enum ClaimError {
    /// Blocked, and no alternative was permitted (or all were taken).
    Taken,
    Io(io::Error),
}

impl std::fmt::Display for ClaimError {
    fn fmt(
        &self,
        f: &mut std::fmt::Formatter<'_>,
    ) -> std::fmt::Result {
        match self {
            ClaimError::Taken => write!(f, "destination is already taken"),
            ClaimError::Io(e) => write!(f, "{e}"),
        }
    }
}

impl std::error::Error for ClaimError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            ClaimError::Taken => None,
            ClaimError::Io(e) => Some(e),
        }
    }
}

/// The ways [`FileClaim::try_replace`] / [`DirClaim::try_replace`] can end.
#[derive(Debug)]
pub enum Replaced<C> {
    /// The source now lives at the claimed name; the placeholder is gone
    /// with it and the claim is dissolved.
    Success,
    /// The rename did not happen: the reservation is handed back intact
    /// for rollback or fallback.
    Failed(io::Error, C),
}

impl From<io::Error> for ClaimError {
    fn from(e: io::Error) -> Self {
        ClaimError::Io(e)
    }
}

/// Reserves `desired`, or under policy finds a free sibling or opens existing.
pub fn reserve_file(desired: &Path, policy: ClaimPolicy) -> Result<FileClaim, ClaimError> {
    if policy == ClaimPolicy::Ignore {
        let file = fs::OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(false)
            .open(desired)?;
        return Ok(FileClaim {
            path: desired.to_path_buf(),
            file,
            levels: 0,
        });
    }

    let reserved = |p: &Path| -> io::Result<Option<fs::File>> {
        match fs::OpenOptions::new().write(true).create_new(true).open(p) {
            Ok(file) => Ok(Some(file)),
            Err(e) if e.kind() == io::ErrorKind::AlreadyExists => Ok(None),
            Err(e) => Err(e),
        }
    };

    if let Some(file) = reserved(desired)? {
        return Ok(FileClaim {
            path: desired.to_path_buf(),
            file,
            levels: 0,
        });
    }

    let ClaimPolicy::Suffix { left, right } = policy else {
        return Err(ClaimError::Taken);
    };

    let parent = desired.parent().unwrap_or(Path::new("."));
    let stem_ext = desired
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();
    let [stem, ext] = crate::bath::split_ext(&stem_ext);
    for i in 1..=MAX_ALTERNATIVES {
        let candidate = if ext.is_empty() {
            parent.join(format!("{stem}{left}{i}{right}"))
        } else {
            parent.join(format!("{stem}{left}{i}{right}.{ext}"))
        };
        if let Some(file) = reserved(&candidate)? {
            return Ok(FileClaim {
                path: candidate,
                file,
                levels: 0,
            });
        }
    }
    Err(ClaimError::Taken)
}

/// Reserves a directory via `create_dir` (atomic per component).
/// Parent directories must already exist — use [`reserve_dir_all`] when
/// they may not, or [`std::fs::create_dir_all`] on the parent yourself.
pub fn reserve_dir(desired: &Path, policy: ClaimPolicy) -> Result<DirClaim, ClaimError> {
    if policy == ClaimPolicy::Ignore {
        if let Ok(md) = fs::symlink_metadata(desired) {
            if md.is_dir() {
                return Ok(DirClaim {
                    path: desired.to_path_buf(),
                    levels: 0,
                    empty: false,
                });
            } else {
                return Err(ClaimError::Io(io::Error::new(
                    io::ErrorKind::AlreadyExists,
                    "destination is not a directory",
                )));
            }
        }
    }

    if let Err(e) = fs::create_dir(desired) {
        if e.kind() != io::ErrorKind::AlreadyExists {
            return Err(ClaimError::Io(e));
        }
    } else {
        return Ok(DirClaim {
            path: desired.to_path_buf(),
            levels: 0,
            empty: true,
        });
    }

    let ClaimPolicy::Suffix { left, right } = policy else {
        return Err(ClaimError::Taken);
    };

    let parent = desired.parent().unwrap_or(Path::new("."));
    let stem = desired
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();
    for i in 1..=MAX_ALTERNATIVES {
        let candidate = parent.join(format!("{stem}{left}{i}{right}"));
        match fs::create_dir(&candidate) {
            Ok(()) => {
                return Ok(DirClaim {
                    path: candidate,
                    levels: 0,
                    empty: true,
                });
            }
            Err(e) if e.kind() == io::ErrorKind::AlreadyExists => continue,
            Err(e) => return Err(ClaimError::Io(e)),
        }
    }
    Err(ClaimError::Taken)
}

/// Counts the ancestor directories of `desired` that do not exist yet:
/// climbs until the first existing ancestor. An `Err` means existence of
/// some ancestor could not be determined.
fn missing_levels(desired: &Path) -> io::Result<usize> {
    let mut levels = 0;
    let mut cur = desired.parent();
    while let Some(p) = cur {
        if p.as_os_str().is_empty() {
            break;
        }
        match fs::symlink_metadata(p) {
            Ok(_) => break,
            Err(e) if e.kind() == io::ErrorKind::NotFound => {}
            Err(e) => return Err(e),
        }
        levels += 1;
        cur = p.parent();
    }
    Ok(levels)
}

/// Reserves `desired` via [`reserve_file`], first inventing any missing
/// ancestor directories with one `create_dir_all` on the parent. A failed
/// reservation repays the levels it invented; a successful one carries
/// them as [`FileClaim::levels`] for [`FileClaim::rollback`] after later
/// failures.
///
/// The count is taken before creating: a concurrent creator racing the
/// same chain can leave this claim owing more levels than it made —
/// rollback only ever removes directories that are still empty.
pub fn reserve_file_all(desired: &Path, policy: ClaimPolicy) -> Result<FileClaim, ClaimError> {
    let levels = missing_levels(desired)?;
    if levels > 0 {
        let parent = desired.parent().unwrap_or(Path::new("."));
        if let Err(e) = fs::create_dir_all(parent) {
            rollback(desired, levels);
            return Err(ClaimError::Io(e));
        }
    }
    match reserve_file(desired, policy) {
        Ok(res) => Ok(res.with_levels(levels)),
        Err(e) => {
            rollback(desired, levels);
            Err(e)
        }
    }
}

/// Reserves a directory via [`reserve_dir`], first inventing any missing
/// ancestor directories with one `create_dir_all` on the parent. See
/// [`reserve_file_all`] for the invented-levels accounting.
pub fn reserve_dir_all(desired: &Path, policy: ClaimPolicy) -> Result<DirClaim, ClaimError> {
    let levels = missing_levels(desired)?;
    if levels > 0 {
        let parent = desired.parent().unwrap_or(Path::new("."));
        if let Err(e) = fs::create_dir_all(parent) {
            rollback(desired, levels);
            return Err(ClaimError::Io(e));
        }
    }
    match reserve_dir(desired, policy) {
        Ok(res) => Ok(res.with_levels(levels)),
        Err(e) => {
            rollback(desired, levels);
            Err(e)
        }
    }
}

fn is_cross_device(err: &io::Error) -> bool {
    #[cfg(unix)]
    {
        err.raw_os_error() == Some(libc::EXDEV)
    }
    #[cfg(windows)]
    {
        err.raw_os_error() == Some(17)
    }
    #[cfg(not(any(unix, windows)))]
    {
        let _ = err;
        false
    }
}

#[cfg(target_os = "linux")]
fn rename_noreplace(from: &Path, to: &Path) -> io::Result<()> {
    use std::ffi::CString;
    use std::os::unix::ffi::OsStrExt;

    let from_c = CString::new(from.as_os_str().as_bytes())
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidInput, e))?;
    let to_c = CString::new(to.as_os_str().as_bytes())
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidInput, e))?;

    let res = unsafe {
        libc::syscall(
            libc::SYS_renameat2,
            libc::AT_FDCWD,
            from_c.as_ptr(),
            libc::AT_FDCWD,
            to_c.as_ptr(),
            libc::RENAME_NOREPLACE,
        )
    };
    if res == 0 {
        Ok(())
    } else {
        Err(io::Error::last_os_error())
    }
}

fn try_move_exclusive(source: &Path, dest: &Path) -> io::Result<bool> {
    #[cfg(target_os = "linux")]
    {
        match rename_noreplace(source, dest) {
            Ok(()) => return Ok(true),
            Err(e) if e.raw_os_error() == Some(libc::EEXIST) => return Ok(false),
            Err(e) if is_cross_device(&e) => return Err(e),
            Err(e)
                if e.raw_os_error() == Some(libc::EINVAL)
                    || e.raw_os_error() == Some(libc::ENOSYS) => {}
            Err(e) => return Err(e),
        }
    }

    match fs::hard_link(source, dest) {
        Ok(()) => {
            let _ = fs::remove_file(source);
            Ok(true)
        }
        #[cfg(unix)]
        Err(e)
            if e.kind() == io::ErrorKind::AlreadyExists
                || e.raw_os_error() == Some(libc::EEXIST) =>
        {
            Ok(false)
        }
        #[cfg(not(unix))]
        Err(e) if e.kind() == io::ErrorKind::AlreadyExists => Ok(false),
        Err(e) if is_cross_device(&e) => Err(e),
        Err(_e) => {
            if fs::symlink_metadata(dest).is_ok() {
                Ok(false)
            } else {
                match fs::rename(source, dest) {
                    Ok(()) => Ok(true),
                    #[cfg(unix)]
                    Err(e)
                        if e.kind() == io::ErrorKind::AlreadyExists
                            || e.raw_os_error() == Some(libc::EEXIST) =>
                    {
                        Ok(false)
                    }
                    #[cfg(not(unix))]
                    Err(e) if e.kind() == io::ErrorKind::AlreadyExists => Ok(false),
                    Err(e) => Err(e),
                }
            }
        }
    }
}

fn try_move_dir_exclusive(source: &Path, dest: &Path) -> io::Result<bool> {
    #[cfg(target_os = "linux")]
    {
        match rename_noreplace(source, dest) {
            Ok(()) => return Ok(true),
            Err(e) if e.raw_os_error() == Some(libc::EEXIST) => return Ok(false),
            Err(e) if is_cross_device(&e) => return Err(e),
            Err(e)
                if e.raw_os_error() == Some(libc::EINVAL)
                    || e.raw_os_error() == Some(libc::ENOSYS) => {}
            Err(e) => return Err(e),
        }
    }

    if fs::symlink_metadata(dest).is_ok() {
        return Ok(false);
    }
    match fs::rename(source, dest) {
        Ok(()) => Ok(true),
        #[cfg(unix)]
        Err(e)
            if e.kind() == io::ErrorKind::AlreadyExists
                || e.raw_os_error() == Some(libc::EEXIST) =>
        {
            Ok(false)
        }
        #[cfg(not(unix))]
        Err(e) if e.kind() == io::ErrorKind::AlreadyExists => Ok(false),
        Err(e) => Err(e),
    }
}

/// Swaps directory `source` onto `dest` by moving `dest` aside to a temporary sibling,
/// renaming `source` to `dest`, and removing the temporary sibling on success.
/// If renaming fails, restores `dest` from the temporary sibling.
fn swap_dir_via_tmp(source: &Path, dest: &Path) -> io::Result<()> {
    let parent = dest.parent().unwrap_or(Path::new("."));
    let stem = dest
        .file_name()
        .map(|n| n.to_string_lossy())
        .unwrap_or_default();

    let tmp_path = parent.join(format!(
        ".{stem}.cba-tmp-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0)
    ));

    fs::rename(dest, &tmp_path)?;

    match fs::rename(source, dest) {
        Ok(()) => {
            let _ = fs::remove_dir_all(&tmp_path);
            Ok(())
        }
        Err(e) => {
            let _ = fs::rename(&tmp_path, dest);
            Err(e)
        }
    }
}

/// Replaces `desired` with file `source` atomically where possible (handling intermediate ancestors).
///
/// Returns:
/// - `Ok(None)`: Successfully moved `source` onto `desired` (or an alternative sibling under
///   [`ClaimPolicy::Suffix`]) in a single atomic step.
/// - `Ok(Some(claim))`: The atomic rename could not be completed in one step (e.g. cross-device transfer),
///   but `desired` has been reserved and intermediate ancestors created. The caller can proceed with chunked
///   copy and use [`FileClaim::rollback`] if it fails.
/// - `Err(ClaimError)`: Reservation/replacement was refused (e.g. target exists under [`ClaimPolicy::Strict`])
///   or an I/O error occurred (any invented ancestor directories are rolled back).
pub fn replace_file(
    source: &Path,
    desired: &Path,
    policy: ClaimPolicy,
) -> Result<Option<FileClaim>, ClaimError> {
    let levels = missing_levels(desired)?;
    if levels > 0 {
        let parent = desired.parent().unwrap_or(Path::new("."));
        if let Err(e) = fs::create_dir_all(parent) {
            rollback(desired, levels);
            return Err(ClaimError::Io(e));
        }
    }

    match policy {
        ClaimPolicy::Ignore => match fs::rename(source, desired) {
            Ok(()) => Ok(None),
            Err(e) if is_cross_device(&e) => match reserve_file(desired, ClaimPolicy::Ignore) {
                Ok(claim) => Ok(Some(claim.with_levels(levels))),
                Err(err) => {
                    rollback(desired, levels);
                    Err(err)
                }
            },
            Err(e) => {
                rollback(desired, levels);
                Err(ClaimError::Io(e))
            }
        },
        ClaimPolicy::Strict => match try_move_exclusive(source, desired) {
            Ok(true) => Ok(None),
            Ok(false) => {
                rollback(desired, levels);
                Err(ClaimError::Taken)
            }
            Err(e) if is_cross_device(&e) => match reserve_file(desired, ClaimPolicy::Strict) {
                Ok(claim) => Ok(Some(claim.with_levels(levels))),
                Err(err) => {
                    rollback(desired, levels);
                    Err(err)
                }
            },
            Err(e) => {
                rollback(desired, levels);
                Err(ClaimError::Io(e))
            }
        },
        ClaimPolicy::Suffix { left, right } => {
            match try_move_exclusive(source, desired) {
                Ok(true) => return Ok(None),
                Ok(false) => {}
                Err(e) if is_cross_device(&e) => match reserve_file(desired, policy) {
                    Ok(claim) => return Ok(Some(claim.with_levels(levels))),
                    Err(err) => {
                        rollback(desired, levels);
                        return Err(err);
                    }
                },
                Err(e) => {
                    rollback(desired, levels);
                    return Err(ClaimError::Io(e));
                }
            }

            let parent = desired.parent().unwrap_or(Path::new("."));
            let stem_ext = desired
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_default();
            let [stem, ext] = crate::bath::split_ext(&stem_ext);

            for i in 1..=MAX_ALTERNATIVES {
                let candidate = if ext.is_empty() {
                    parent.join(format!("{stem}{left}{i}{right}"))
                } else {
                    parent.join(format!("{stem}{left}{i}{right}.{ext}"))
                };
                match try_move_exclusive(source, &candidate) {
                    Ok(true) => return Ok(None),
                    Ok(false) => continue,
                    Err(e) if is_cross_device(&e) => {
                        match reserve_file(&candidate, ClaimPolicy::Strict) {
                            Ok(claim) => return Ok(Some(claim.with_levels(levels))),
                            Err(ClaimError::Taken) => continue,
                            Err(err) => {
                                rollback(desired, levels);
                                return Err(err);
                            }
                        }
                    }
                    Err(e) => {
                        rollback(desired, levels);
                        return Err(ClaimError::Io(e));
                    }
                }
            }

            rollback(desired, levels);
            Err(ClaimError::Taken)
        }
    }
}

/// Replaces `desired` with directory `source` atomically where possible (handling intermediate ancestors).
///
/// Under [`ClaimPolicy::Ignore`], if `desired` exists and is non-empty, `desired` is moved aside
/// to a temporary sibling, `source` is renamed into place, and the temporary copy is deleted.
///
/// Returns:
/// - `Ok(None)`: Successfully moved directory `source` onto `desired` (or an alternative sibling) in a single atomic step.
/// - `Ok(Some(claim))`: The atomic rename could not be completed (e.g. cross-device transfer), but `desired`
///   has been reserved and intermediate ancestors created. The caller can proceed with chunked copy and use
///   [`DirClaim::rollback`] if it fails.
/// - `Err(ClaimError)`: Reservation/replacement was refused or an I/O error occurred.
pub fn replace_dir(
    source: &Path,
    desired: &Path,
    policy: ClaimPolicy,
) -> Result<Option<DirClaim>, ClaimError> {
    let levels = missing_levels(desired)?;
    if levels > 0 {
        let parent = desired.parent().unwrap_or(Path::new("."));
        if let Err(e) = fs::create_dir_all(parent) {
            rollback(desired, levels);
            return Err(ClaimError::Io(e));
        }
    }

    match policy {
        ClaimPolicy::Ignore => match fs::rename(source, desired) {
            Ok(()) => Ok(None),
            #[cfg(unix)]
            Err(e)
                if e.raw_os_error() == Some(libc::ENOTEMPTY)
                    || e.raw_os_error() == Some(libc::EEXIST) =>
            {
                match swap_dir_via_tmp(source, desired) {
                    Ok(()) => Ok(None),
                    Err(e) if is_cross_device(&e) => {
                        match reserve_dir(desired, ClaimPolicy::Ignore) {
                            Ok(claim) => Ok(Some(claim.with_levels(levels))),
                            Err(err) => {
                                rollback(desired, levels);
                                Err(err)
                            }
                        }
                    }
                    Err(e) => {
                        rollback(desired, levels);
                        Err(ClaimError::Io(e))
                    }
                }
            }
            #[cfg(not(unix))]
            Err(e)
                if e.kind() == io::ErrorKind::DirectoryNotEmpty
                    || e.kind() == io::ErrorKind::AlreadyExists =>
            {
                match swap_dir_via_tmp(source, desired) {
                    Ok(()) => Ok(None),
                    Err(e) if is_cross_device(&e) => {
                        match reserve_dir(desired, ClaimPolicy::Ignore) {
                            Ok(claim) => Ok(Some(claim.with_levels(levels))),
                            Err(err) => {
                                rollback(desired, levels);
                                Err(err)
                            }
                        }
                    }
                    Err(e) => {
                        rollback(desired, levels);
                        Err(ClaimError::Io(e))
                    }
                }
            }
            Err(e) if is_cross_device(&e) => match reserve_dir(desired, ClaimPolicy::Ignore) {
                Ok(claim) => Ok(Some(claim.with_levels(levels))),
                Err(err) => {
                    rollback(desired, levels);
                    Err(err)
                }
            },
            Err(e) => {
                rollback(desired, levels);
                Err(ClaimError::Io(e))
            }
        },
        ClaimPolicy::Strict => match try_move_dir_exclusive(source, desired) {
            Ok(true) => Ok(None),
            Ok(false) => {
                rollback(desired, levels);
                Err(ClaimError::Taken)
            }
            Err(e) if is_cross_device(&e) => match reserve_dir(desired, ClaimPolicy::Strict) {
                Ok(claim) => Ok(Some(claim.with_levels(levels))),
                Err(err) => {
                    rollback(desired, levels);
                    Err(err)
                }
            },
            Err(e) => {
                rollback(desired, levels);
                Err(ClaimError::Io(e))
            }
        },
        ClaimPolicy::Suffix { left, right } => {
            match try_move_dir_exclusive(source, desired) {
                Ok(true) => return Ok(None),
                Ok(false) => {}
                Err(e) if is_cross_device(&e) => match reserve_dir(desired, policy) {
                    Ok(claim) => return Ok(Some(claim.with_levels(levels))),
                    Err(err) => {
                        rollback(desired, levels);
                        return Err(err);
                    }
                },
                Err(e) => {
                    rollback(desired, levels);
                    return Err(ClaimError::Io(e));
                }
            }

            let parent = desired.parent().unwrap_or(Path::new("."));
            let stem = desired
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_default();

            for i in 1..=MAX_ALTERNATIVES {
                let candidate = parent.join(format!("{stem}{left}{i}{right}"));
                match try_move_dir_exclusive(source, &candidate) {
                    Ok(true) => return Ok(None),
                    Ok(false) => continue,
                    Err(e) if is_cross_device(&e) => {
                        match reserve_dir(&candidate, ClaimPolicy::Strict) {
                            Ok(claim) => return Ok(Some(claim.with_levels(levels))),
                            Err(ClaimError::Taken) => continue,
                            Err(err) => {
                                rollback(desired, levels);
                                return Err(err);
                            }
                        }
                    }
                    Err(e) => {
                        rollback(desired, levels);
                        return Err(ClaimError::Io(e));
                    }
                }
            }

            rollback(desired, levels);
            Err(ClaimError::Taken)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tmp(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "cba-claim-{name}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .subsec_nanos()
        ));
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn exact_reservation_blocks_second_claim() {
        let dir = tmp("exact");
        let want = dir.join("f.txt");
        let _first = reserve_file(&want, ClaimPolicy::Strict).unwrap();
        match reserve_file(&want, ClaimPolicy::Strict) {
            Err(ClaimError::Taken) => {}
            other => panic!("expected Taken, got {other:?}"),
        }
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn fallback_naming_skips_taken_names() {
        let dir = tmp("fallback");
        fs::write(dir.join("report.txt"), b"old").unwrap();
        fs::write(dir.join("report_1.txt"), b"old").unwrap();

        let reserved = reserve_file(&dir.join("report.txt"), ClaimPolicy::default()).unwrap();
        assert_eq!(reserved.into_parts().unwrap().0, dir.join("report_2.txt"));
        // untouched originals
        assert_eq!(fs::read(dir.join("report.txt")).unwrap(), b"old");
        assert_eq!(fs::read(dir.join("report_1.txt")).unwrap(), b"old");
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn extension_is_preserved_by_fallback() {
        let dir = tmp("ext");
        fs::write(dir.join("a.tar.gz"), b"x").unwrap();
        let reserved = reserve_file(&dir.join("a.tar.gz"), ClaimPolicy::default()).unwrap();
        // split_ext takes the last segment as the extension
        assert_eq!(reserved.into_parts().unwrap().0, dir.join("a.tar_1.gz"));
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn dir_reservation_and_exhaustion() {
        let dir = tmp("dirs");
        let _a = reserve_dir(&dir.join("d"), ClaimPolicy::default()).unwrap();
        let b = reserve_dir(&dir.join("d"), ClaimPolicy::default()).unwrap();
        assert_eq!(b.into_path().unwrap(), dir.join("d_1"));

        fs::write(dir.join("x"), b"").unwrap();
        let exhausted = reserve_file(&dir.join("x"), ClaimPolicy::Strict);
        assert!(matches!(exhausted, Err(ClaimError::Taken)));
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn held_handle_writes_land_in_reserved_file() {
        let dir = tmp("handle");
        let mut res = reserve_file(&dir.join("out.bin"), ClaimPolicy::Strict).unwrap();
        use std::io::Write;
        res.file().write_all(b"payload").unwrap();
        drop(res);
        assert_eq!(fs::read(dir.join("out.bin")).unwrap(), b"payload");
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn all_variants_invent_missing_ancestors() {
        let dir = tmp("all");
        let want = dir.join("a").join("b").join("f.txt");
        let res = reserve_file_all(&want, ClaimPolicy::Strict).unwrap();
        assert_eq!(res.into_parts().unwrap().0, want);

        let d = dir.join("x").join("y").join("z");
        let res = reserve_dir_all(&d, ClaimPolicy::Strict).unwrap();
        assert_eq!(res.into_path().unwrap(), d);
        assert!(d.is_dir());
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn all_levels_are_zero_when_ancestors_exist() {
        let dir = tmp("zero-levels");
        let res = reserve_file_all(&dir.join("f"), ClaimPolicy::Strict).unwrap();
        assert_eq!(res.into_parts().unwrap().0, dir.join("f"));
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn ancestor_unwind_is_shallowest_stopping_and_content_guarded() {
        let dir = tmp("unwind");
        // three simulated invented levels below an existing root
        let deep = dir.join("a").join("b").join("c");
        fs::create_dir_all(&deep).unwrap();
        rollback(&deep.join("leaf"), 3);
        assert!(!deep.exists());
        assert!(!deep.parent().unwrap().exists());
        assert!(!dir.join("a").exists());

        // content anywhere stops the unwind above it
        let deep = dir.join("x").join("y").join("z");
        fs::create_dir_all(&deep).unwrap();
        fs::write(deep.join("held"), b"").unwrap();
        rollback(&deep.join("leaf"), 3);
        assert!(deep.exists());
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn rollback_removes_leaf_and_invented_ancestors_while_empty() {
        let dir = tmp("rollback");
        // keep predates the claim and holds content: it must survive
        fs::create_dir(dir.join("keep")).unwrap();
        fs::write(dir.join("keep").join("marker"), b"").unwrap();

        let want = dir.join("keep").join("sub").join("d");
        let res = reserve_dir_all(&want, ClaimPolicy::Strict).unwrap();
        assert!(res.empty());
        res.rollback();
        assert!(!want.exists());
        assert!(!want.parent().unwrap().exists());
        assert!(dir.join("keep").join("marker").exists());

        // a file leaf is only rolled back while zero-length
        let fwant = dir.join("k2").join("f.bin");
        let res = reserve_file_all(&fwant, ClaimPolicy::Strict).unwrap();
        res.rollback();
        assert!(!fwant.exists());
        assert!(!fwant.parent().unwrap().exists());

        let fwant = dir.join("k3").join("f.bin");
        let mut res = reserve_file_all(&fwant, ClaimPolicy::Strict).unwrap();
        use std::io::Write;
        res.file().write_all(b"data").unwrap();
        res.rollback();
        // content protects the leaf, and with it the ancestor holding it
        assert!(fwant.exists());
        assert!(fwant.parent().unwrap().exists());
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn claim_policy_ignore_succeeds_on_existing() {
        let dir = tmp("ignore");
        let f = dir.join("file.txt");
        fs::write(&f, b"existing").unwrap();
        let mut res = reserve_file_all(&f, ClaimPolicy::Ignore).unwrap();
        use std::io::Write;
        res.file().write_all(b"new").unwrap();
        let (path, _file) = res.into_parts().unwrap();
        assert_eq!(path, f);

        let d = dir.join("existing_dir");
        fs::create_dir(&d).unwrap();
        fs::write(d.join("inside.txt"), b"data").unwrap();
        let res = reserve_dir_all(&d, ClaimPolicy::Ignore).unwrap();
        assert!(!res.empty());
        res.clear_contents().unwrap();
        assert!(!d.join("inside.txt").exists());
        res.rollback();
        // pre-existing dir not deleted by rollback
        assert!(d.is_dir());
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn try_replace_moves_source_and_dissolves_the_claim() {
        let dir = tmp("replace");
        fs::write(dir.join("src"), b"payload").unwrap();
        // a nested destination proves invented ancestors survive success
        let dest = dir.join("a").join("dst");
        let res = reserve_file_all(&dest, ClaimPolicy::Strict).unwrap();
        match res.try_replace(dir.join("src")) {
            Replaced::Success => {}
            other => panic!("expected Success, got {other:?}"),
        }
        assert_eq!(fs::read(&dest).unwrap(), b"payload");
        assert!(!dir.join("src").exists());
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn failed_replace_hands_the_claim_back_for_rollback() {
        let dir = tmp("replace-failed");
        // missing source: rename fails with ENOENT, claim stays valid
        let res = reserve_file_all(&dir.join("dst"), ClaimPolicy::Strict).unwrap();
        let (e, back) = match res.try_replace(dir.join("no-such-src")) {
            Replaced::Failed(e, back) => (e, back),
            other => panic!("expected Failed, got {other:?}"),
        };
        assert_eq!(e.kind(), io::ErrorKind::NotFound);
        back.rollback();
        assert!(!dir.join("dst").exists());

        // the dir variant round-trips through the same arm
        let res = reserve_dir_all(&dir.join("d"), ClaimPolicy::Strict).unwrap();
        match res.try_replace(dir.join("no-such-src")) {
            Replaced::Failed(_, back) => back.rollback(),
            other => panic!("expected Failed, got {other:?}"),
        }
        assert!(!dir.join("d").exists());
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn try_replace_on_occupied_dir_clears_and_replaces() {
        let dir = tmp("occupied-dir-replace");
        let src = dir.join("src");
        fs::create_dir(&src).unwrap();
        fs::write(src.join("new.txt"), b"NEW").unwrap();

        let dst = dir.join("dst");
        fs::create_dir(&dst).unwrap();
        fs::write(dst.join("old.txt"), b"OLD").unwrap();

        let res = reserve_dir_all(&dst, ClaimPolicy::Ignore).unwrap();
        assert!(!res.empty());
        match res.try_replace(&src) {
            Replaced::Success => {}
            Replaced::Failed(e, _) => panic!("replace failed: {e}"),
        }
        assert_eq!(fs::read(dst.join("new.txt")).unwrap(), b"NEW");
        assert!(!dst.join("old.txt").exists());
        assert!(!src.exists());
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn try_replace_on_empty_dir_refuses_if_populated() {
        let dir = tmp("empty-dir-populated-refuse");
        let src = dir.join("src");
        fs::create_dir(&src).unwrap();

        let dst = dir.join("dst");
        let res = reserve_dir_all(&dst, ClaimPolicy::Strict).unwrap();
        assert!(res.empty());

        // Place a stray file into the placeholder before replace
        fs::write(dst.join("sneaky.txt"), b"DATA").unwrap();

        match res.try_replace(&src) {
            Replaced::Failed(e, back) => {
                assert_eq!(e.kind(), io::ErrorKind::DirectoryNotEmpty);
                back.rollback(); // rollback shouldn't delete non-empty dir
                assert!(dst.exists());
            }
            Replaced::Success => panic!("expected Failure on non-empty placeholder"),
        }
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn into_path_returns_none_if_polluted() {
        let dir = tmp("into-path-polluted");
        let dst = dir.join("d").join("sub");
        let res = reserve_dir_all(&dst, ClaimPolicy::Strict).unwrap();
        assert!(res.empty());

        // Sneak a file into the empty claimed directory
        fs::write(dst.join("pollute.txt"), b"DATA").unwrap();

        assert!(res.into_path().is_none());
        // Non-empty does not rollback: directory and ancestors remain untouched
        assert!(dst.exists());
        assert!(dst.parent().unwrap().exists());
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn replace_file_atomic_overwrite() {
        let dir = tmp("replace-file-overwrite");
        let src = dir.join("src.txt");
        let dst = dir.join("dst.txt");
        fs::write(&src, b"new content").unwrap();
        fs::write(&dst, b"old content").unwrap();

        let outcome = replace_file(&src, &dst, ClaimPolicy::Ignore).unwrap();
        assert!(outcome.is_none());
        assert_eq!(fs::read(&dst).unwrap(), b"new content");
        assert!(!src.exists());
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn replace_file_strict_blocks_existing() {
        let dir = tmp("replace-file-strict");
        let src = dir.join("src.txt");
        let dst = dir.join("dst.txt");
        fs::write(&src, b"new content").unwrap();
        fs::write(&dst, b"old content").unwrap();

        let outcome = replace_file(&src, &dst, ClaimPolicy::Strict);
        assert!(matches!(outcome, Err(ClaimError::Taken)));
        assert_eq!(fs::read(&dst).unwrap(), b"old content");
        assert!(src.exists());
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn replace_file_suffix_lands_in_sibling() {
        let dir = tmp("replace-file-suffix");
        let src = dir.join("src.txt");
        let dst = dir.join("file.txt");
        fs::write(&src, b"payload").unwrap();
        fs::write(&dst, b"orig").unwrap();

        let outcome = replace_file(&src, &dst, ClaimPolicy::default()).unwrap();
        assert!(outcome.is_none());
        assert_eq!(fs::read(&dst).unwrap(), b"orig");
        assert_eq!(fs::read(dir.join("file_1.txt")).unwrap(), b"payload");
        assert!(!src.exists());
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn replace_dir_atomic_empty_and_nonempty_overwrite() {
        let dir = tmp("replace-dir-overwrite");
        let src = dir.join("src");
        fs::create_dir(&src).unwrap();
        fs::write(src.join("hello.txt"), b"world").unwrap();

        let dst = dir.join("dst");
        fs::create_dir(&dst).unwrap();
        fs::write(dst.join("obsolete.txt"), b"junk").unwrap();

        let outcome = replace_dir(&src, &dst, ClaimPolicy::Ignore).unwrap();
        assert!(outcome.is_none());
        assert_eq!(fs::read(dst.join("hello.txt")).unwrap(), b"world");
        assert!(!dst.join("obsolete.txt").exists());
        assert!(!src.exists());
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn replace_dir_strict_blocks_existing() {
        let dir = tmp("replace-dir-strict");
        let src = dir.join("src");
        fs::create_dir(&src).unwrap();

        let dst = dir.join("dst");
        fs::create_dir(&dst).unwrap();

        let outcome = replace_dir(&src, &dst, ClaimPolicy::Strict);
        assert!(matches!(outcome, Err(ClaimError::Taken)));
        assert!(src.exists());
        assert!(dst.exists());
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn replace_dir_suffix_lands_in_sibling() {
        let dir = tmp("replace-dir-suffix");
        let src = dir.join("src");
        fs::create_dir(&src).unwrap();
        fs::write(src.join("data.txt"), b"123").unwrap();

        let dst = dir.join("dir");
        fs::create_dir(&dst).unwrap();

        let outcome = replace_dir(&src, &dst, ClaimPolicy::default()).unwrap();
        assert!(outcome.is_none());
        assert!(dst.exists());
        assert_eq!(fs::read(dir.join("dir_1").join("data.txt")).unwrap(), b"123");
        assert!(!src.exists());
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn replace_creates_ancestors() {
        let dir = tmp("replace-ancestors");
        let src = dir.join("src.txt");
        fs::write(&src, b"data").unwrap();

        let dst = dir.join("a").join("b").join("c.txt");
        let outcome = replace_file(&src, &dst, ClaimPolicy::Strict).unwrap();
        assert!(outcome.is_none());
        assert_eq!(fs::read(&dst).unwrap(), b"data");
        assert!(!src.exists());
        fs::remove_dir_all(&dir).unwrap();
    }
}

