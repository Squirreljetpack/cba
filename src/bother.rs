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
    ///    /// Reduce the verbosity level (min: -qq).
    ///    #[clap(short, conflicts_with("verbose"), action = ArgAction::Count)]
    ///    quiet: u8,
    ///
    ///    /// Increase the verbosity level (max: -vvv).
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

    /// Converts a numeric verbosity level into a [`LevelFilter`].
    ///
    /// The mapping is:
    ///
    /// - `0` or `1` → disables logging entirely
    /// - `2` → logs only errors
    /// - `3` (default) → logs warnings and errors
    /// - `4` → logs info, warnings, and errors
    /// - `5` → logs debug and above
    /// - `6` or higher → logs everything (trace and above)
    pub fn from_verbosity(verbosity: u8) -> LevelFilter {
        match verbosity {
            0 | 1 => LevelFilter::Off,
            2 => LevelFilter::Error,
            3 => LevelFilter::Warn,
            4 => LevelFilter::Info,
            5 => LevelFilter::Debug,
            _ => LevelFilter::Trace,
        }
    }
}
