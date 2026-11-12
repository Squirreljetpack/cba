use std::path::PathBuf;

pub use dirs::home_dir;
pub use crate::bath::expand_tilde;

fn env_override(
    name: &str,
    suffix: &str,
) -> Option<PathBuf> {
    let var = format!("{}_{}", name.to_uppercase().replace('-', "_"), suffix);
    let path = PathBuf::from(std::env::var_os(&var)?);
    if path.exists() {
        Some(path)
    } else {
        log::warn!(
            "Environment variable {} points to non-existent directory {}, ignoring",
            var,
            path.display()
        );
        None
    }
}

/// `<NAME>_CONFIG_DIR` (when it exists), else `~/.config/<name>` (when it
/// exists), else the platform config directory joined with `name`.
pub fn config_dir(name: &str) -> Option<PathBuf> {
    if let Some(path) = env_override(name, "CONFIG_DIR") {
        return Some(path);
    }

    if let Some(home) = dirs::home_dir() {
        let config = home.join(".config").join(name);
        if config.exists() {
            return Some(config);
        }
    }

    dirs::config_dir().map(|p| p.join(name))
}

/// `<NAME>_CACHE_DIR` (when it exists), else the platform cache directory
/// joined with `name`.
pub fn cache_dir(name: &str) -> Option<PathBuf> {
    if let Some(path) = env_override(name, "CACHE_DIR") {
        return Some(path);
    }

    dirs::cache_dir().map(|p| p.join(name))
}

/// `<NAME>_STATE_DIR` (when it exists), else the platform state directory
/// joined with `name`, falling back to `~/.local/state/<name>` where no state
/// directory is defined.
pub fn state_dir(name: &str) -> Option<PathBuf> {
    if let Some(path) = env_override(name, "STATE_DIR") {
        return Some(path);
    }

    dirs::state_dir()
        .or_else(|| dirs::home_dir().map(|home| home.join(".local").join("state")))
        .map(|p| p.join(name))
}
