//! IO

use std::{error::Error, fs, io, path::Path};

use crate::{
    bait::ResultExt,
    bog::{BOGGER, BogContext, BogLevel, BogOkExt},
    ebog, else_default,
};

// ------------ File read/write (bile) -------------

/// Saves type to file.
///
/// Prints error.
pub fn dump_type<T, E: Error>(
    path: impl AsRef<Path>,
    input: &T,
    string_maker: impl FnOnce(&T) -> Result<String, E>,
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

/// Returns none if file could not be found/read/parsed.
///
/// Prints error.
pub fn load_type<T, E: Error>(
    path: impl AsRef<Path>,
    str_loader: impl FnOnce(&str) -> Result<T, E>, // pass a closure here if u need to satisfy hrtb
) -> Option<T> {
    let path = path.as_ref().with_extension("toml");
    let type_name = std::any::type_name::<T>().rsplit("::").next().unwrap();
    let error_prefix = format!("Failed to load {type_name} from {}", path.to_string_lossy());

    let mut file = else_default!(fs::File::open(path).prefix(&error_prefix)._ebog());

    let mut contents = String::new();
    else_default!(
        io::Read::read_to_string(&mut file, &mut contents)
            .prefix(&error_prefix)
            ._ebog()
    );

    Some(else_default!(
        str_loader(&contents).prefix(&error_prefix)._ebog()
    ))
}

/// If the path exists, load from it, otherwise load from the provided default.
///
/// Prints error.
pub fn load_type_or_default<T: Default, E: Error>(
    path: impl AsRef<Path>,
    str_loader: impl FnOnce(&str) -> Result<T, E>,
    default_str: &str,
) -> T {
    let path = path.as_ref();
    if path.is_file() {
        BOGGER::with(
            BogContext::new()
                .upper(BogLevel::WARN)
                .prefix("Using default config: "),
            || load_type(path, str_loader).unwrap_or_default(),
        )
    } else {
        str_loader(default_str).unwrap()
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
) -> Result<(), MapReaderError<E>> {
    for (i, chunk_result) in iter.enumerate() {
        if i == u32::MAX as usize {
            warn!("Reached maximum segment limit, stopping input read");
            return Err(MapReaderError::ChunkError(i));
        }

        let chunk = match chunk_result {
            Ok(bytes) => bytes,
            Err(e) => {
                error!("Error reading chunk: {e}");
                return Err(MapReaderError::ChunkError(i));
            }
        };

        match String::from_utf8(chunk) {
            Ok(s) => {
                if let Err(e) = f(s) {
                    return Err(MapReaderError::Custom(e));
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
    Ok(())
}

/// Map each line read from reader to a string, passing to f.
pub fn map_reader_lines<const INVALID_FAIL: bool, E>(
    reader: impl Read,
    mut f: impl FnMut(String) -> Result<(), E>,
) -> Result<(), MapReaderError<E>> {
    let buf_reader = io::BufReader::new(reader);

    for (i, line) in buf_reader.lines().enumerate() {
        if i == u32::MAX as usize {
            eprintln!("Reached maximum line limit, stopping input read");
            return Err(MapReaderError::ChunkError(i));
        }
        match line {
            Ok(l) => {
                if let Err(e) = f(l) {
                    return Err(MapReaderError::Custom(e));
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
    Ok(())
}
