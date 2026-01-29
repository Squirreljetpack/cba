pub mod level_filter {
    use log::LevelFilter;
    pub fn from_env() -> LevelFilter {
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

    /// Logging filter level.
    /// # Example
    /// ```rust,ignore
    ///    /// Reduces the level of verbosity (the min level is -qq).
    ///    #[clap(short, conflicts_with("verbose"), action = ArgAction::Count)]
    ///    quiet: u8,
    ///
    ///    /// Increases the level of verbosity (the max level is -vvv).
    ///    #[clap(short, conflicts_with("quiet"), action = ArgAction::Count)]
    ///    verbose: u8,
    /// ```
    pub fn from_qv(quiet: u8, verbose: u8) -> LevelFilter {
        match (quiet, verbose) {
            // Default.
            (0, 0) => LevelFilter::Warn,

            // Verbose.
            (_, 1) => LevelFilter::Info,
            (_, 2) => LevelFilter::Debug,
            (0, _) => LevelFilter::Trace,

            // Quiet.
            (1, _) => LevelFilter::Error,
            (..) => LevelFilter::Off,
        }
    }
}
