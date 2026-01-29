//! IO

use std::{error::Error, fs, io, path::Path};

use crate::{bait::ResultExt, bog::BogOkExt, ebog, else_default};

// ------------ File read/write (bile) -------------

/// Saves type to file.
///
/// Prints error.
pub fn dump_type<'a, T, E: Error>(
    path: impl AsRef<Path>,
    input: &'a T,
    string_maker: impl FnOnce(&'a T) -> Result<String, E>,
) -> bool {
    let path = path.as_ref().with_extension("toml");
    let type_name = std::any::type_name::<T>().rsplit("::").next().unwrap();
    let error_prefix = format!("Failed to save {type_name} to {}", path.to_string_lossy());

    let content = else_default!(string_maker(input).prefix(&error_prefix)._ebog());
    match fs::write(path, content) {
        Ok(_) => true,
        Err(e) => {
            ebog!("{error_prefix}: {e}");
            false
        }
    }
}

/// Returns error string if file could not be found/read/parsed.
pub fn load_type<T, E: std::fmt::Display>(
    path: impl AsRef<Path>,
    str_loader: impl FnOnce(&str) -> Result<T, E>, // pass a closure here if u need to satisfy hrtb
) -> anyhow::Result<T> {
    let path = path.as_ref().with_extension("toml");
    let type_name = std::any::type_name::<T>().rsplit("::").next().unwrap();
    let error_prefix = format!("Failed to load {type_name} from {}", path.to_string_lossy());

    let mut file = fs::File::open(path).context(&error_prefix)?;

    let mut contents = String::new();
    io::Read::read_to_string(&mut file, &mut contents).context(&error_prefix)?;

    str_loader(&contents).context(&error_prefix)
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
use log::{error, warn};
use std::io::{BufRead, Read};

#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum MapReaderError<E> {
    #[error("Failed to read chunk: {0}")]
    ChunkError(usize),
    #[error("Aborted: {0}")]
    Custom(E),
}

/// Adapt a reader, splitting on the delim character.
pub fn read_to_chunks<R: Read>(reader: R, delim: char) -> std::io::Split<std::io::BufReader<R>> {
    io::BufReader::new(reader).split(delim as u8)
}

// do not use for newlines as it doesn't handle \r!
// todo: warn about this in config
// note: stream means wrapping with closure passed stream::unfold and returning f() inside

/// Map each chunk read from reader to a string, passing to f.
pub fn map_chunks<const INVALID_FAIL: bool, E>(
    iter: impl Iterator<Item = std::io::Result<Vec<u8>>>,
    mut f: impl FnMut(String) -> Result<(), E>,
) -> Result<usize, MapReaderError<E>> {
    let mut count = 0;
    for (i, chunk_result) in iter.enumerate() {
        if i == u32::MAX as usize {
            warn!("Reached maximum segment limit, stopping input read");
            return Err(MapReaderError::ChunkError(i));
        }

        let bytes = match chunk_result {
            Ok(bytes) => bytes,
            Err(e) => {
                error!("Error reading chunk: {e}");
                return Err(MapReaderError::ChunkError(i));
            }
        };

        match String::from_utf8(bytes) {
            Ok(s) => {
                if let Err(e) = f(s) {
                    return Err(MapReaderError::Custom(e));
                } else {
                    count += 1;
                }
            }
            Err(e) => {
                error!(
                    "Invalid UTF-8 in stdin at byte {}: {}",
                    e.utf8_error().valid_up_to(),
                    e
                );
                // Skip but continue reading
                if INVALID_FAIL {
                    return Err(MapReaderError::ChunkError(i));
                } else {
                    continue;
                }
            }
        }
    }
    Ok(count)
}

/// Map each line read from reader to a string, passing to f.
pub fn map_reader_lines<const INVALID_FAIL: bool, E>(
    reader: impl Read,
    mut f: impl FnMut(String) -> Result<(), E>,
) -> Result<usize, MapReaderError<E>> {
    let buf_reader = io::BufReader::new(reader);
    let mut count = 0;

    for (i, line) in buf_reader.lines().enumerate() {
        if i == u32::MAX as usize {
            eprintln!("Reached maximum line limit, stopping input read");
            return Err(MapReaderError::ChunkError(i));
        }
        match line {
            Ok(l) => {
                if let Err(e) = f(l) {
                    return Err(MapReaderError::Custom(e));
                } else {
                    count += 1;
                }
            }
            Err(e) => {
                eprintln!("Error reading line: {}", e);
                if INVALID_FAIL {
                    return Err(MapReaderError::ChunkError(i));
                } else {
                    continue;
                }
            }
        }
    }
    Ok(count)
}
