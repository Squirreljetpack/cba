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

pub mod types {
    #[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Default)]
    #[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]

    // Deserializable from true/false/always/never
    pub enum When {
        #[cfg_attr(feature = "serde", serde(alias = "false", alias = "never"))]
        Never,
        #[default]
        #[cfg_attr(feature = "serde", serde(alias = "auto"))]
        Auto,
        #[cfg_attr(feature = "serde", serde(alias = "true", alias = "always"))]
        Always,
    }

    impl From<When> for Option<bool> {
        fn from(w: When) -> Self {
            match w {
                When::Never => Some(false),
                When::Always => Some(true),
                When::Auto => None,
            }
        }
    }

    impl From<Option<bool>> for When {
        fn from(opt: Option<bool>) -> Self {
            match opt {
                Some(true) => When::Always,
                Some(false) => When::Never,
                None => When::Auto,
            }
        }
    }

    impl From<bool> for When {
        fn from(b: bool) -> Self {
            if b { When::Always } else { When::Never }
        }
    }

    impl When {
        /// Returns the inner boolean, or `default` if Auto
        pub fn unwrap_or(self, default: bool) -> bool {
            match self {
                When::Never => false,
                When::Always => true,
                When::Auto => default,
            }
        }

        /// Returns the inner boolean, or computes it via `f` if Auto
        pub fn unwrap_or_else<F>(self, f: F) -> bool
        where
            F: FnOnce() -> bool,
        {
            match self {
                When::Never => false,
                When::Always => true,
                When::Auto => f(),
            }
        }

        pub fn is_default(&self) -> bool {
            matches!(self, When::Auto)
        }

        pub fn is_always(&self) -> bool {
            matches!(self, When::Always)
        }

        /// For compatibility with Option<bool>
        pub fn is_none(&self) -> bool {
            matches!(self, When::Auto)
        }
    }

    #[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
    #[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
    pub enum Either<L, R = L> {
        Left(L),
        Right(R),
    }

    impl<L, R> Either<L, R> {
        /// Convert `Either<L, R>` into an `Option<R>`, discarding `Left`.
        pub fn right(self) -> Option<R> {
            match self {
                Either::Right(r) => Some(r),
                Either::Left(_) => None,
            }
        }

        /// Convert `Either<L, R>` into an `Option<L>`, discarding `Right`.
        pub fn left(self) -> Option<L> {
            match self {
                Either::Left(l) => Some(l),
                Either::Right(_) => None,
            }
        }

        pub fn _left(self) -> L {
            self.left().unwrap()
        }

        pub fn _right(self) -> R {
            self.right().unwrap()
        }

        pub fn as_ref(&self) -> Either<&L, &R> {
            use Either::*;
            match self {
                Left(x) => Either::Left(x),
                Right(x) => Either::Right(x),
            }
        }

        pub fn as_mut(&mut self) -> Either<&mut L, &mut R> {
            use Either::*;
            match self {
                Left(x) => Either::Left(x),
                Right(x) => Either::Right(x),
            }
        }

        /// Map `Left` value while leaving `Right` untouched.
        pub fn map_left<F, LL>(self, f: F) -> Either<LL, R>
        where
            F: FnOnce(L) -> LL,
        {
            match self {
                Either::Left(l) => Either::Left(f(l)),
                Either::Right(r) => Either::Right(r),
            }
        }

        /// Map `Right` value while leaving `Left` untouched.
        pub fn map_right<F, RR>(self, f: F) -> Either<L, RR>
        where
            F: FnOnce(R) -> RR,
        {
            match self {
                Either::Left(l) => Either::Left(l),
                Either::Right(r) => Either::Right(f(r)),
            }
        }

        /// Whether enum is a `Left` value.
        pub fn is_left(&self) -> bool {
            matches!(self, Either::Left(_))
        }

        /// Convert `Either<L, R>` into a `Either<R, L>`.
        pub fn swap(self) -> Either<R, L> {
            match self {
                Either::Left(x) => Either::Right(x),
                Either::Right(x) => Either::Left(x),
            }
        }

        /// Convert `Either<L, R>` into a `Result<L, R>`.
        pub fn into_result(self) -> Result<L, R> {
            match self {
                Either::Left(l) => Ok(l),
                Either::Right(r) => Err(r),
            }
        }
    }
}
