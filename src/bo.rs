//! IO

use std::{
    error::Error,
    fs, io,
    path::{Path, PathBuf},
};

use crate::{StringError, bait::ResultExt, bog::BogOkExt};

// ------------ File read/write (bile) -------------

/// Saves type to file.
///
/// Returns a formatted error.
pub fn dump_type<'a, T, E: Error>(
    path: impl AsRef<Path>,
    input: &'a T,
    string_maker: impl FnOnce(&'a T) -> Result<String, E>,
) -> Result<(), StringError> {
    let path = path.as_ref();
    let type_name = std::any::type_name::<T>().rsplit("::").next().unwrap();
    let error_prefix = format!("Failed to save {type_name} to {}", path.to_string_lossy());

    let content = string_maker(input).prefix(&error_prefix)?;
    fs::write(path, content).prefix(&error_prefix)
}

/// Serializes a value to a temporary file beside `path`, then atomically replaces `path`.
///
/// Symlinks are followed, so a symlinked destination is updated through its target rather than
/// replaced by a regular file; a symlink whose target does not exist is replaced itself. An
/// existing destination keeps its permissions. The destination's parent directory must already
/// exist. The temporary file is synchronized before it is persisted, so readers see either the
/// previous complete file or the new one.
pub fn dump_type_atomic<'a, T, E: Error>(
    path: impl AsRef<Path>,
    input: &'a T,
    string_maker: impl FnOnce(&'a T) -> Result<String, E>,
) -> Result<(), StringError> {
    let path = path.as_ref();
    let type_name = std::any::type_name::<T>().rsplit("::").next().unwrap();
    let error_prefix = format!("Failed to save {type_name} to {}", path.to_string_lossy());
    let content = string_maker(input).prefix(&error_prefix)?;
    let destination = symlink_destination(path);
    let parent = destination
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));

    let mut temporary = tempfile::NamedTempFile::new_in(parent).prefix(&error_prefix)?;
    io::Write::write_all(temporary.as_file_mut(), content.as_bytes()).prefix(&error_prefix)?;
    if let Ok(metadata) = fs::metadata(&destination) {
        temporary
            .as_file_mut()
            .set_permissions(metadata.permissions())
            .prefix(&error_prefix)?;
    }
    temporary.as_file_mut().sync_all().prefix(&error_prefix)?;
    temporary
        .persist(&destination)
        .map(|_| ())
        .map_err(|error| error.error)
        .prefix(&error_prefix)
}

/// Resolves a chain of symlinks to the path that a write should target, or returns `path` when
/// the chain ends at something that does not exist.
fn symlink_destination(path: &Path) -> PathBuf {
    const MAX_DEPTH: usize = 32;

    let mut current = path.to_path_buf();
    for _ in 0..MAX_DEPTH {
        let is_symlink =
            fs::symlink_metadata(&current).is_ok_and(|metadata| metadata.file_type().is_symlink());
        if !is_symlink {
            break;
        }
        let Ok(target) = fs::read_link(&current) else {
            break;
        };
        current = if target.is_absolute() {
            target
        } else {
            current
                .parent()
                .unwrap_or_else(|| Path::new("."))
                .join(target)
        };
    }
    if current.exists() {
        current
    } else {
        path.to_path_buf()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn atomically_replaces_the_destination() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("value.txt");
        fs::write(&path, "old").unwrap();

        dump_type_atomic(&path, &"new", |value| {
            Ok::<_, io::Error>((*value).to_owned())
        })
        .unwrap();

        assert_eq!(fs::read_to_string(path).unwrap(), "new");
    }

    #[cfg(unix)]
    #[test]
    fn symlinked_destination_keeps_the_link() {
        let dir = tempfile::tempdir().unwrap();
        let target = dir.path().join("target.txt");
        let link = dir.path().join("link.txt");
        fs::write(&target, "old").unwrap();
        std::os::unix::fs::symlink(&target, &link).unwrap();

        dump_type_atomic(&link, &"new", |value| {
            Ok::<_, io::Error>((*value).to_owned())
        })
        .unwrap();

        assert!(
            fs::symlink_metadata(&link)
                .unwrap()
                .file_type()
                .is_symlink()
        );
        assert_eq!(fs::read_to_string(&target).unwrap(), "new");
    }

    #[cfg(unix)]
    #[test]
    fn broken_symlink_is_replaced() {
        let dir = tempfile::tempdir().unwrap();
        let target = dir.path().join("missing.txt");
        let link = dir.path().join("link.txt");
        std::os::unix::fs::symlink(&target, &link).unwrap();

        dump_type_atomic(&link, &"new", |value| {
            Ok::<_, io::Error>((*value).to_owned())
        })
        .unwrap();

        assert!(
            !fs::symlink_metadata(&link)
                .unwrap()
                .file_type()
                .is_symlink()
        );
        assert_eq!(fs::read_to_string(&link).unwrap(), "new");
        assert!(!target.exists());
    }

    #[cfg(unix)]
    #[test]
    fn existing_permissions_are_kept() {
        use std::os::unix::fs::PermissionsExt;

        let dir = tempfile::tempdir().unwrap();
        let target = dir.path().join("target.txt");
        let link = dir.path().join("link.txt");
        fs::write(&target, "old").unwrap();
        fs::set_permissions(&target, fs::Permissions::from_mode(0o644)).unwrap();
        std::os::unix::fs::symlink(&target, &link).unwrap();

        dump_type_atomic(&link, &"new", |value| {
            Ok::<_, io::Error>((*value).to_owned())
        })
        .unwrap();

        let mode = fs::metadata(&target).unwrap().permissions().mode() & 0o777;
        assert_eq!(mode, 0o644);
    }

    #[test]
    fn serialization_failure_keeps_the_destination() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("value.txt");
        fs::write(&path, "old").unwrap();

        let result = dump_type_atomic(&path, &"new", |_| {
            Err::<String, _>(io::Error::other("cannot serialize"))
        });

        assert!(result.is_err());
        assert_eq!(fs::read_to_string(path).unwrap(), "old");
    }
}
/// Returns error string if file could not be found/read/parsed.
pub fn load_type<T, E: std::fmt::Display>(
    path: impl AsRef<Path>,
    str_loader: impl FnOnce(&str) -> Result<T, E>, // pass a closure here if u need to satisfy hrtb
) -> Result<T, StringError> {
    let path = path.as_ref();
    let type_name = std::any::type_name::<T>().rsplit("::").next().unwrap();
    let error_prefix = format!("Failed to load {type_name} from {}", path.to_string_lossy());

    let mut file = fs::File::open(path).prefix(&error_prefix)?;

    let mut contents = String::new();
    io::Read::read_to_string(&mut file, &mut contents).prefix(&error_prefix)?;

    str_loader(&contents).prefix(&error_prefix)
}

/// If the path exists, load from it, otherwise load from the provided default.
///
/// If `target` is provided, logs a warning to the given target on failure;
/// otherwise logs via `bog`.
///
/// # Example
/// ```rust, ignore
/// #[derive(Debug, serde::Deserialize)]
/// pub struct LessfilterConfig {
///     #[serde(flatten, default)]
///     pub test: TestSettings,
///     #[serde(default)]
///     pub rules: RulesConfig,
///     #[serde(default)]
///     pub actions: CustomActions,
/// }
///
/// impl Default for LessfilterConfig {
///     fn default() -> Self {
///         let ret = toml::from_str(include_str!("../../assets/config/lessfilter.toml"));
///         ret.unwrap()
///     }
/// }
///
/// let cfg: LessfilterConfig = load_type_or_default(lessfilter_cfg_path(), None, |s| toml::from_str(s));
/// ```
pub fn load_type_or_default<T: Default, E: std::fmt::Display>(
    path: impl AsRef<Path>,
    target: Option<&str>,
    str_loader: impl Fn(&str) -> Result<T, E>,
) -> T {
    let path = path.as_ref();
    if path.is_file() {
        let res = load_type(path, &str_loader).prefix("Using default config due to errors");
        match target {
            Some(target) => match res {
                Ok(val) => val,
                Err(err) => {
                    log::warn!(target: target, "{err}");
                    T::default()
                }
            },
            None => res._wbog().unwrap_or_else(T::default),
        }
    } else {
        T::default()
    }
}

/// Write string to file, creating parent directories as needed.
pub fn write_str(path: &Path, contents: &str) -> io::Result<()> {
    if let Some(p) = path.parent() {
        std::fs::create_dir_all(p)?; // normalize should ensure parent always works
    }
    std::fs::write(path, contents)?;

    Ok(())
}

// --------- READER ------------
// todo: decide on how to handle max chunks
use std::io::{BufRead, Read};

#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
/// Failure while mapping chunks read from a reader.
pub enum MapReaderError<E> {
    #[error("Failed to read chunk {0}: {1}")]
    /// The chunk could not be read.
    ChunkError(usize, std::io::Error),
    #[error("Aborted: {0}")]
    /// The mapping closure aborted with this error.
    Custom(#[from] E),
}

/// Adapt a reader, splitting on the delim character.
pub fn read_to_chunks<R: Read>(reader: R, delim: char) -> std::io::Split<std::io::BufReader<R>> {
    io::BufReader::new(reader).split(delim as u8)
}

// note: stream means wrapping with closure passed stream::unfold and returning f() inside
// skip_invalid could be mut skip_invalid: Option<impl FnMut(Vec<u8>)>, if we use str::from_utf8 but that makes skip_invalid into handle_invalid Some(|| {}) which is a bit more annoying

/// Map each chunk read from reader to a string, passing to f.
/// Logs chunk reading errors.
/// Use [`map_reader_lines`] instead for reading newlines.
///
///
/// # Example
/// ```rust,ignore
/// pub fn map_reader<E: SSS + std::fmt::Display>(
///     reader: impl Read + SSS,
///     f: impl FnMut(String) -> Result<(), E> + SSS,
///     input_separator: Option<char>,
///     abort_empty: Option<RenderSender<NullActionExt>>,
///     skip_invalid: bool
/// ) -> tokio::task::JoinHandle<Result<usize, MapReaderError<E>>> {
///     tokio::task::spawn_blocking(move || {
///         let ret = if let Some(delim) = input_separator {
///             map_chunks::<E>(read_to_chunks(reader, delim, skip_invalid), f).elog()
///         } else {
///             map_reader_lines::<E>(reader, f, skip_invalid).elog()
///         };
///
///         if let Some(render_tx) = abort_empty
///             && matches!(ret, Ok(0))
///         {
///             let _ = render_tx.send(matchmaker::message::RenderCommand::QuitEmpty);
///         }
///         ret
///     })
/// }
/// ```
pub fn map_chunks<E>(
    iter: impl Iterator<Item = std::io::Result<Vec<u8>>>,
    mut f: impl FnMut(String) -> Result<(), E>,
    skip_invalid: bool,
) -> Result<usize, MapReaderError<E>> {
    let mut count = 0;
    for (i, chunk_result) in iter.enumerate() {
        let bytes = chunk_result.map_err(|e| MapReaderError::ChunkError(i, e))?;

        match String::from_utf8(bytes) {
            Ok(s) => {
                if let Err(e) = f(s) {
                    return Err(MapReaderError::Custom(e));
                } else {
                    count += 1;
                }
            }
            Err(e) => {
                let err = format!(
                    "Invalid UTF-8 in stdin at byte {}: {}",
                    e.utf8_error().valid_up_to(),
                    e
                );
                // Skip but continue reading
                if !skip_invalid {
                    return Err(MapReaderError::ChunkError(i, std::io::Error::other(err)));
                } else {
                    continue;
                }
            }
        }
    }
    Ok(count)
}

/// Map each line read from reader to a string, passing to f.
/// Logs read errors.
pub fn map_reader_lines<E>(
    reader: impl Read,
    mut f: impl FnMut(String) -> Result<(), E>,
    skip_invalid: bool,
) -> Result<usize, MapReaderError<E>> {
    let buf_reader = io::BufReader::new(reader);
    let mut count = 0;

    for (i, line) in buf_reader.lines().enumerate() {
        match line {
            Ok(l) => {
                if let Err(e) = f(l) {
                    return Err(MapReaderError::Custom(e));
                } else {
                    count += 1;
                }
            }
            Err(e) => {
                if !skip_invalid {
                    return Err(MapReaderError::ChunkError(i, e));
                } else {
                    continue;
                }
            }
        }
    }
    Ok(count)
}
