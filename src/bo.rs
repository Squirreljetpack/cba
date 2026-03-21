//! IO

use std::{error::Error, fs, io, path::Path};

use crate::{StringError, bait::ResultExt, bog::BogOkExt};

// ------------ File read/write (bile) -------------

/// Saves type to file.
///
/// Prints error.
pub fn dump_type<'a, T, E: Error>(
    path: impl AsRef<Path>,
    input: &'a T,
    string_maker: impl FnOnce(&'a T) -> Result<String, E>,
) -> Result<(), StringError> {
    let path = path.as_ref().with_extension("toml");
    let type_name = std::any::type_name::<T>().rsplit("::").next().unwrap();
    let error_prefix = format!("Failed to save {type_name} to {}", path.to_string_lossy());

    let content = string_maker(input).prefix(&error_prefix)?;
    fs::write(path, content).prefix(&error_prefix)
}

/// Returns error string if file could not be found/read/parsed.
pub fn load_type<T, E: std::fmt::Display>(
    path: impl AsRef<Path>,
    str_loader: impl FnOnce(&str) -> Result<T, E>, // pass a closure here if u need to satisfy hrtb
) -> Result<T, StringError> {
    let path = path.as_ref().with_extension("toml");
    let type_name = std::any::type_name::<T>().rsplit("::").next().unwrap();
    let error_prefix = format!("Failed to load {type_name} from {}", path.to_string_lossy());

    let mut file = fs::File::open(path).prefix(&error_prefix)?;

    let mut contents = String::new();
    io::Read::read_to_string(&mut file, &mut contents).prefix(&error_prefix)?;

    str_loader(&contents).prefix(&error_prefix)
}

/// If the path exists, load from it, otherwise load from the provided default.
///
/// Prints error.
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
/// let cfg: LessfilterConfig = load_type_or_default(lessfilter_cfg_path(), |s| toml::from_str(s));
/// ```
pub fn load_type_or_default<T: Default, E: std::fmt::Display>(
    path: impl AsRef<Path>,
    str_loader: impl Fn(&str) -> Result<T, E>,
) -> T {
    let path = path.as_ref();
    if path.is_file() {
        load_type(path, &str_loader)
            .prefix("Using default config due to errors")
            ._wbog()
            .unwrap_or_else(T::default)
    } else {
        T::default()
    }
}

/// [`load_type_or_default`] but log instead of bog
pub fn load_type_or_default_log<T: Default, E: std::fmt::Display>(
    path: impl AsRef<Path>,
    str_loader: impl Fn(&str) -> Result<T, E>,
) -> T {
    let path = path.as_ref();
    if path.is_file() {
        load_type(path, &str_loader)
            .prefix("Using default config due to errors")
            ._wlog()
            .unwrap_or_else(T::default)
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
pub enum MapReaderError<E> {
    #[error("Failed to read chunk {0}: {1}")]
    ChunkError(usize, std::io::Error),
    #[error("Aborted: {0}")]
    Custom(#[from] E),
}

/// Adapt a reader, splitting on the delim character.
pub fn read_to_chunks<R: Read>(reader: R, delim: char) -> std::io::Split<std::io::BufReader<R>> {
    io::BufReader::new(reader).split(delim as u8)
}

// note: stream means wrapping with closure passed stream::unfold and returning f() inside

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
/// ) -> tokio::task::JoinHandle<Result<usize, MapReaderError<E>>> {
///     tokio::task::spawn_blocking(move || {
///         let ret = if let Some(delim) = input_separator {
///             map_chunks::<true, E>(read_to_chunks(reader, delim), f).elog()
///         } else {
///             map_reader_lines::<true, E>(reader, f).elog()
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
pub fn map_chunks<const INVALID_FAIL: bool, E>(
    iter: impl Iterator<Item = std::io::Result<Vec<u8>>>,
    mut f: impl FnMut(String) -> Result<(), E>,
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
                if INVALID_FAIL {
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
pub fn map_reader_lines<const INVALID_FAIL: bool, E>(
    reader: impl Read,
    mut f: impl FnMut(String) -> Result<(), E>,
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
                if INVALID_FAIL {
                    return Err(MapReaderError::ChunkError(i, e));
                } else {
                    continue;
                }
            }
        }
    }
    Ok(count)
}
