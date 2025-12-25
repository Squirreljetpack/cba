#[easy_ext::ext(MaybeExt)]
pub impl<T> T
where
    T: Sized,
{
    fn maybe_take(&mut self, maybe: Option<T>) {
        if let Some(v) = maybe {
            *self = v;
        }
    }

    fn maybe_clone(&mut self, maybe: &Option<T>)
    where
        T: Clone,
    {
        if let Some(v) = maybe {
            *self = v.clone();
        }
    }
}

// this would be more useful if try blocks exposed their "other" type
#[easy_ext::ext(ResultExt)]
pub impl<T, E> Result<T, E> {
    fn cast_err<Q>(self) -> Result<T, Q>
    where
        Q: From<E>,
    {
        self.map_err(|e| e.into())
    }

    /// Convert Err(e) to the string '{prefix}: {e}'
    fn prefix_err(self, prefix: &str) -> Result<T, String>
    where
        E: std::fmt::Display,
    {
        match self {
            Ok(val) => Ok(val),
            Err(e) => Err(format!("{prefix}: {e}")),
        }
    }
}

// -----------------------------------------
use log::LevelFilter;
pub fn level_filter_from_env() -> LevelFilter {
    match std::env::var("RUST_LOG")
        .ok()
        .map(|s| s.to_lowercase())
        .as_deref()
    {
        Some("trace") => LevelFilter::Trace,
        Some("debug") => LevelFilter::Debug,
        Some("info") => LevelFilter::Info,
        Some("warn") => LevelFilter::Warn,
        Some("error") => LevelFilter::Error,
        _ => LevelFilter::Info,
    }
}