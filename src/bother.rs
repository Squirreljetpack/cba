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