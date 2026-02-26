use std::{
    borrow::Cow,
    fmt::{self, Display},
    io::{Write, stderr, stdout},
    sync::Mutex,
    u8,
};

use crate::bait::{MutexExt, OptionExt};

use super::{Bg, BogFmter, BogLevel, Fg};

/// ---------- Global bogger instance --------------
#[allow(non_camel_case_types)]
struct GLOBAL_BOGGER_STRUCT {
    formatter: Box<dyn BogFmter + Send + Sync>,
    writer: Box<dyn Write + Send + Sync>,
    min_level: (u8, BogLevel),
    downcast_to: (u8, BogLevel),
    pub prefix: String,
    pub suffix: String,
    pub tag_override: Option<String>,
    /// Some(true) => only log, Some(false) => Only bog, None => Both
    pub log: Option<bool>,
}

// since stderr has an internal lock i guess this isn't a huge deal anyways
static GLOBAL_BOGGER: Mutex<Option<GLOBAL_BOGGER_STRUCT>> = Mutex::new(None);

fn init_(logger: Box<dyn BogFmter + Send + Sync>, write: Box<dyn Write + Send + Sync>) {
    let bogger = GLOBAL_BOGGER_STRUCT {
        formatter: logger,
        writer: write,
        downcast_to: (255, BogLevel::ERROR),
        min_level: (0, BogLevel::DEBUG),
        prefix: String::new(),
        suffix: String::new(),
        tag_override: None,
        log: None,
    };

    *GLOBAL_BOGGER.lock().unwrap() = Some(bogger);
}
// -------- (Internal) methods on global  ----------
impl GLOBAL_BOGGER_STRUCT {
    fn bog(&mut self, mut level: BogLevel, tag: &str, msg: &str) {
        // Determine priority
        let pri = self.formatter.priority(&level);
        if pri < self.min_level.0 {
            return;
        }
        if pri > self.downcast_to.0 {
            level = self.downcast_to.1;
        }

        if self.log != Some(false) {
            let level = match level {
                BogLevel::ERROR => Some(log::Level::Error),
                BogLevel::WARN => Some(log::Level::Warn),
                BogLevel::INFO => Some(log::Level::Info),
                BogLevel::DEBUG => Some(log::Level::Debug),
                BogLevel::___ => Some(log::Level::Trace),
                _ => None,
            };
            if let Some(lvl) = level {
                log::log!(lvl, "{}{}{}", self.prefix, msg, self.suffix);
            }
        }
        if self.log != Some(true) {
            // Determine effective tag
            let effective_tag = self.tag_override.as_deref().unwrap_or(tag);

            // Format message with prefix and suffix
            let mut formatted = if !self.prefix.is_empty() {
                let mut prefixed_msg = self.prefix.clone();
                prefixed_msg.push_str(&msg);
                self.formatter.format(level, effective_tag, &prefixed_msg)
            } else {
                self.formatter.format(level, effective_tag, msg)
            };

            if !self.suffix.is_empty() {
                formatted.push_str(&self.suffix);
            }
            formatted.push('\n');

            // Write to writer
            let _ = self.writer.write_all(formatted.as_bytes());
        }
    }

    fn pause(&mut self) {
        self.min_level.0 = u8::MAX;
    }

    fn resume(&mut self) {
        self.min_level.0 = self.formatter.priority(&self.min_level.1)
    }

    fn filter_below(&mut self, lvl: BogLevel) {
        self.min_level = (self.formatter.priority(&lvl), lvl);
    }

    /// Show only messages with this priority and higher.
    /// On resume, displays all messages.
    fn filter_below_priority(&mut self, v: u8) {
        self.min_level = (v, BogLevel::___);
    }

    fn downcast_above(&mut self, lvl: BogLevel) {
        self.downcast_to = (self.formatter.priority(&lvl), lvl);
    }

    fn bounds(&self) -> ((u8, BogLevel), (u8, BogLevel)) {
        (self.min_level, self.downcast_to)
    }

    pub fn set_bounds(&mut self, bounds: ((u8, BogLevel), (u8, BogLevel))) {
        self.min_level = bounds.0;
        self.downcast_to = bounds.1;
    }
}

// ------- CONTEXT --------
/// Context for temporary bogger settings. Use with [`Bogger::with`].
pub struct BogContext {
    /// [lower, upper]. Filter out messages below lower and downcast messages above upper.
    bounds: [Option<BogLevel>; 2],
    /// Whether to pause logging.
    pause: bool,
    /// Prefix to prepend to all messages.
    prefix: Option<String>,
    /// Suffix to append to all messages.
    suffix: Option<String>,
    /// Override tag for all messages.
    tag_override: Option<String>,
    /// Whether to only log (true), [`bog`] only (false), or both (None).
    log: Option<bool>,
}

impl BogContext {
    pub fn new() -> Self {
        Self {
            bounds: [None, None],
            pause: false,
            prefix: None,
            suffix: None,
            tag_override: None,
            log: None,
        }
    }

    pub fn upper(mut self, level: BogLevel) -> Self {
        self.bounds[1] = Some(level);
        self
    }

    pub fn lower(mut self, level: BogLevel) -> Self {
        self.bounds[0] = Some(level);
        self
    }

    pub fn pause(mut self, pause: bool) -> Self {
        self.pause = pause;
        self
    }

    pub fn prefix<S: Into<String>>(mut self, prefix: S) -> Self {
        self.prefix = Some(prefix.into());
        self
    }

    pub fn suffix<S: Into<String>>(mut self, suffix: S) -> Self {
        self.suffix = Some(suffix.into());
        self
    }

    pub fn tag<S: Into<String>>(mut self, tag: S) -> Self {
        self.tag_override = Some(tag.into());
        self
    }

    pub fn log(mut self, log: Option<bool>) -> Self {
        self.log = log;
        self
    }
}

// --------- EXPORTS/MAIN API ---------
// convenience reexport

pub struct BOGGER {}
// organize under namespace
impl BOGGER {
    // don't panic
    /// Log a message at the given level with optional tag.
    #[inline]
    pub fn bog(level: BogLevel, tag: &str, msg: &str) {
        if let Some(b) = GLOBAL_BOGGER._lock().as_mut() {
            b.bog(level, tag, msg);
        }
    }

    /// Set the minimum level to log.
    #[inline]
    pub fn filter_below(lvl: BogLevel) {
        if let Some(b) = GLOBAL_BOGGER._lock().as_mut() {
            b.filter_below(lvl);
        }
    }

    /// Downcast messages above the given level to this level.
    #[inline]
    pub fn downcast_above(lvl: BogLevel) {
        if let Some(b) = GLOBAL_BOGGER._lock().as_mut() {
            b.downcast_above(lvl);
        }
    }

    /// Temporarily apply a BogContext while executing a closure.
    #[inline]
    pub fn with<T>(context: BogContext, f: impl FnOnce() -> T) -> T {
        let (prev_bounds, prev_paused, prev_prefix, prev_suffix, prev_tag) = {
            if let Some(b) = GLOBAL_BOGGER._lock().as_mut() {
                // Save previous state
                let prev_bounds = b.bounds();
                let prev_paused = prev_bounds.0.0 == u8::MAX;
                let prev_prefix = b.prefix.clone();
                let prev_suffix = b.suffix.clone();
                let prev_tag = b.tag_override.clone();

                // Apply new context
                if let Some(level) = context.bounds[0] {
                    b.filter_below(level);
                }
                if let Some(level) = context.bounds[1] {
                    b.downcast_above(level);
                }
                if let Some(ref prefix) = context.prefix {
                    b.prefix = prefix.clone();
                }
                if let Some(ref suffix) = context.suffix {
                    b.suffix = suffix.clone();
                }
                if let Some(ref tag) = context.tag_override {
                    b.tag_override = Some(tag.clone());
                }
                if context.pause {
                    b.pause();
                }

                (
                    Some(prev_bounds),
                    Some(prev_paused),
                    Some(prev_prefix),
                    Some(prev_suffix),
                    prev_tag,
                )
            } else {
                (None, None, None, None, None)
            }
        };

        // Execute the closure
        let result = f();

        // Restore previous state
        if let Some(b) = GLOBAL_BOGGER._lock().as_mut() {
            if let Some(bounds) = prev_bounds {
                b.set_bounds(bounds);
            }
            if let Some(paused) = prev_paused {
                if paused {
                    b.pause();
                } else {
                    b.resume();
                }
            }
            if let Some(prefix) = prev_prefix {
                b.prefix = prefix;
            }
            if let Some(suffix) = prev_suffix {
                b.suffix = suffix;
            }
            if let Some(tag) = prev_tag {
                b.tag_override = Some(tag);
            } else if context.tag_override.is_some() {
                b.tag_override = None
            }
        }

        result
    }

    /// Execute a closure while pausing logging.
    #[inline]
    pub fn paused<T>(f: impl FnOnce() -> T) -> T {
        BOGGER::pause();
        let ret = f();
        BOGGER::resume();
        ret
    }

    /// Pause logging.
    #[inline]
    pub fn pause() {
        if let Some(b) = GLOBAL_BOGGER._lock().as_mut() {
            b.pause();
        }
    }

    /// Resume logging.
    #[inline]
    pub fn resume() {
        if let Some(b) = GLOBAL_BOGGER._lock().as_mut() {
            b.resume();
        }
    }
}

// ----------- INITIALIZATION -------------
pub fn init_bogger(fg: bool, output_stderr: bool) {
    let writer: Box<dyn Write + Send + Sync> = if output_stderr {
        Box::new(stderr())
    } else {
        Box::new(stdout())
    };

    if fg {
        init_(Box::new(Fg {}), writer);
    } else {
        init_(Box::new(Bg {}), writer);
    }
}

/// Initialize the global log filter based on a numeric verbosity level. [`init_bogger`] must be called beforehand.
///
/// The verbosity value maps to a minimum [`BogLevel`] that will be emitted:
///
/// - `0` → silence all
/// - `1` → show `NOTE`, `EMPTY` and `CUSTOM` messages only
/// - `2` → show `ERROR` and above
/// - `3` → show `WARN` and above
/// - `4` → show `INFO` and above
/// - `5` → show `_WRN`/`_NFO` and above
/// - `6` → show `DEBUG` and above
/// - `7` → show all messages
///
/// Note that the ordering is dependant on the default implementation of [`BogFmter::priority`]. If overridden in a non-compatible way, we recommended calling [`BOGGER::filter_below`] directly to avoid confusion.
pub fn init_filter(verbosity: u8) {
    let level = match verbosity {
        0 => {
            GLOBAL_BOGGER
                ._lock()
                .as_mut()
                .unwrap()
                .filter_below_priority(u8::MAX);
            return;
        }
        1 => BogLevel::NOTE,
        2 => BogLevel::ERROR,
        3 => BogLevel::WARN,
        4 => BogLevel::INFO,
        5 => BogLevel::_WRN,
        6 => BogLevel::DEBUG,
        _ => BogLevel::___,
    };
    log::debug!("Bogging level initialized at {level:?}");
    BOGGER::filter_below(level);
}

// ----------- MACROS ------------------
#[macro_export]
macro_rules! ibog {
    // With tag expressions
    ($($harg:expr),* ; $($arg:expr),*) => {{
        $crate::BOGGER::bog(
            $crate::bog::BogLevel::INFO,
            &format!($($harg),*),
            &format!($($arg),*),
        );
    }};
    // Without tag
    ($($arg:expr),*) => {{
        $crate::BOGGER::bog(
            $crate::bog::BogLevel::INFO,
            "",
            &format!($($arg),*),
        );
    }};
}

#[macro_export]
macro_rules! dbog {
    ($($harg:expr),* ; $($arg:expr),*) => {{
        $crate::BOGGER::bog(
            $crate::bog::BogLevel::DEBUG,
            &format!($($harg),*),
            &format!($($arg),*),
        );
    }};
    ($($arg:expr),*) => {{
        $crate::BOGGER::bog(
            $crate::bog::BogLevel::DEBUG,
            "",
            &format!($($arg),*),
        );
    }};
}

#[macro_export]
macro_rules! ebog {
    ($($harg:expr),* ; $($arg:expr),*) => {{
        $crate::BOGGER::bog(
            $crate::bog::BogLevel::ERROR,
            &format!($($harg),*),
            &format!($($arg),*),
        );
    }};
    ($($arg:expr),*) => {{
        $crate::BOGGER::bog(
            $crate::bog::BogLevel::ERROR,
            "",
            &format!($($arg),*),
        );
    }};
}

#[macro_export]
macro_rules! wbog {
    ($($harg:expr),* ; $($arg:expr),*) => {{
        $crate::BOGGER::bog(
            $crate::bog::BogLevel::WARN,
            &format!($($harg),*),
            &format!($($arg),*),
        );
    }};
    ($($arg:expr),*) => {{
        $crate::BOGGER::bog(
            $crate::bog::BogLevel::WARN,
            "",
            &format!($($arg),*),
        );
    }};
}

#[macro_export]
macro_rules! nbog {
    ($($harg:expr),* ; $($arg:expr),*) => {{
        $crate::BOGGER::bog(
            $crate::bog::BogLevel::NOTE,
            &format!($($harg),*),
            &format!($($arg),*),
        );
    }};
    ($($arg:expr),*) => {{
        $crate::BOGGER::bog(
            $crate::bog::BogLevel::NOTE,
            "",
            &format!($($arg),*),
        );
    }};
}

#[macro_export]
macro_rules! mbog {
    ($($harg:expr),* ; $($arg:expr),*) => {{
        $crate::BOGGER::bog(
            $crate::bog::BogLevel::EMPTY,
            &format!($($harg),*),
            &format!($($arg),*),
        );
    }};
    ($($arg:expr),*) => {{
        $crate::BOGGER::bog(
            $crate::bog::BogLevel::EMPTY,
            "",
            &format!($($arg),*),
        );
    }};
}

#[macro_export]
macro_rules! cbog {
    ($discriminant:literal ; $($harg:expr),* ; $($arg:expr),*) => {{
        $crate::BOGGER::bog(
            $crate::bog::BogLevel::CUSTOM($discriminant),
            &format!($($harg),*),
            &format!($($arg),*),
        );
    }};
    ($discriminant:literal ; $($arg:expr),*) => {{
        $crate::BOGGER::bog(
            $crate::bog::BogLevel::CUSTOM($discriminant),
            "",
            &format!($($arg),*),
        );
    }};
}

#[macro_export]
macro_rules! _wbog {
    ($($harg:expr),* ; $($arg:expr),*) => {{
        $crate::BOGGER::bog(
            $crate::bog::BogLevel::_WRN,
            &format!($($harg),*),
            &format!($($arg),*),
        );
    }};
    ($($arg:expr),*) => {{
        $crate::BOGGER::bog(
            $crate::bog::BogLevel::_WRN,
            "",
            &format!($($arg),*),
        );
    }};
}

#[macro_export]
macro_rules! _ibog {
    ($($harg:expr),* ; $($arg:expr),*) => {{
        $crate::BOGGER::bog(
            $crate::bog::BogLevel::_NFO,
            &format!($($harg),*),
            &format!($($arg),*),
        );
    }};
    ($($arg:expr),*) => {{
        $crate::BOGGER::bog(
            $crate::bog::BogLevel::_NFO,
            "",
            &format!($($arg),*),
        );
    }};
}

// ----------- RESULT -----------------

/// # Example
/// ```rust
/// use cli_boilerplate_automation::bog::{BogOkExt, BogUnwrapExt};
///
/// fn fallible_result() -> Result<i32, Box<dyn std::error::Error>> {
///     Ok(42)
/// }
///
/// fn process(x: i32) {
///     println!("Processing {}", x);
/// }
///
/// if let Some(x) = fallible_result()._ebog() {
///     process(x);
/// }
/// ```

#[easy_ext::ext(BogOkExt)]
pub impl<T, E: Display> Result<T, E> {
    fn _bog_<'a>(self, level: BogLevel, tag: impl Into<Cow<'a, str>>) -> Option<T> {
        match self {
            Ok(val) => Some(val),
            Err(e) => {
                BOGGER::bog(level, &tag.into(), &e.to_string());
                None
            }
        }
    }

    fn _ebog_<'a>(self, tag: impl Into<Cow<'a, str>>) -> Option<T> {
        self._bog_(BogLevel::ERROR, tag)
    }

    fn _ibog_<'a>(self, tag: impl Into<Cow<'a, str>>) -> Option<T> {
        self._bog_(BogLevel::INFO, tag)
    }

    fn _dbog_<'a>(self, tag: impl Into<Cow<'a, str>>) -> Option<T> {
        self._bog_(BogLevel::DEBUG, tag)
    }

    fn _wbog_<'a>(self, tag: impl Into<Cow<'a, str>>) -> Option<T> {
        self._bog_(BogLevel::WARN, tag)
    }

    fn _bog(self, level: BogLevel) -> Option<T> {
        self._bog_(level, "")
    }

    fn _ebog(self) -> Option<T> {
        self._ebog_("")
    }

    fn __ebog(self) -> T {
        self._ebog_("").or_exit()
    }

    fn _wbog(self) -> Option<T> {
        self._wbog_("")
    }

    fn _dbog(self) -> Option<T> {
        self._dbog_("")
    }
    fn _ibog(self) -> Option<T> {
        self._ibog_("")
    }
}

#[easy_ext::ext(BogUnwrapExt)]
pub impl<T> Option<T> {
    /// Unwrap or bog and exit
    fn _bog_<'a>(
        self,
        level: BogLevel,
        tag: impl Into<Cow<'a, str>>,
        msg: impl Into<Cow<'a, str>>,
    ) -> T {
        match self {
            Some(val) => val,
            None => {
                BOGGER::bog(level, &tag.into(), &msg.into());
                std::process::exit(1);
            }
        }
    }

    /// Unwrap or bog and exit
    fn _bog<'a>(self, level: BogLevel, msg: impl Into<Cow<'a, str>>) -> T {
        self._bog_(level, "", msg)
    }

    /// Unwrap or err and exit
    fn _ebog<'a>(self, msg: impl Into<Cow<'a, str>>) -> T {
        self._bog(BogLevel::ERROR, msg)
    }

    /// Unwrap or err and exit
    fn _ebog_<'a>(self, tag: impl Into<Cow<'a, str>>, msg: impl Into<Cow<'a, str>>) -> T {
        self._bog_(BogLevel::ERROR, tag, msg)
    }

    fn bog_<'a>(
        self,
        level: BogLevel,
        tag: impl Into<Cow<'a, str>>,
        msg: impl Into<Cow<'a, str>>,
    ) -> Option<T> {
        match self {
            Some(val) => Some(val),
            None => {
                BOGGER::bog(level, &tag.into(), &msg.into());
                None
            }
        }
    }
    fn bog<'a>(self, level: BogLevel, msg: impl Into<Cow<'a, str>>) -> Option<T> {
        self.bog_(level, "", msg)
    }
    fn ebog<'a>(self, msg: impl Into<Cow<'a, str>>) -> Option<T> {
        self.bog(BogLevel::ERROR, msg)
    }
    fn ebog_<'a>(self, msg: impl Into<Cow<'a, str>>) -> Option<T> {
        self.bog(BogLevel::ERROR, msg)
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn show_fg_bogger() {
        init_bogger(true, false);
        // DEBUG messages
        dbog!("DEBUG message: {}", 3.14159);
        dbog!("val"; "DEBUG values: x={}, y={}", 10, 20);

        // INFO messages
        ibog!("INFO message: {}", 42);
        ibog!("Created Directory"; "~/archr/Desktop");

        // WARN messages
        wbog!("WARN message: {}", "disk almost full");
        wbog!("NoSpace"; "WARN message: {} attempts left", 3);

        // ERROR messages
        ebog!("ERROR message: {}", "file not found");
        ebog!("404"; "Not found");

        // NOTE messages
        nbog!("justification");
        nbog!("NOTE"; "ancillary");
        mbog!("justification");
        mbog!("FULL TAG"; "ancillary");

        // CUSTOM / NOTE-like messages using cbog
        cbog!("NOTE"; "Custom note message: {}", "all good");
        cbog!("NOTE"; ""; "Custom note with tag: {}", 123);
        cbog!("CUSTOM"; "Custom discriminant"; "Message with both tag and content");
    }

    #[test]
    fn show_bg_bogger() {
        init_bogger(false, true);
        // DEBUG messages
        dbog!("DEBUG message: {}", 3.14159);
        dbog!("val"; "DEBUG values: x={}, y={}", 10, 20);

        // INFO messages
        ibog!("INFO message: {}", 42);
        ibog!("Urgent"; "INFO message number {}", 7);

        // WARN messages
        wbog!("WARN message: {}", "disk almost full");
        wbog!("NoSpace"; "WARN message: {} attempts left", 3);

        // ERROR messages
        ebog!("ERROR message: {}", "file not found");
        ebog!("404"; "Not found");

        // NOTE messages
        nbog!("justification");
        nbog!("NOTE"; "ancillary");
        mbog!("justification");
        mbog!("FULL"; "ancillary");

        // CUSTOM
        cbog!("NOTE"; "Custom note message: {}", "all good");
        cbog!("NOTE"; ""; "Custom note with tag: {}", 123);
        cbog!("CUSTOM"; "Custom discriminant"; "Message with both tag and content");
    }

    #[test]
    fn min_level_and_downcast_combined() {
        init_bogger(true, false);

        // drop DEBUG/INFO entirely
        BOGGER::filter_below(/* WARN priority */ BogLevel::INFO);
        // downcast ERROR to WARN
        BOGGER::downcast_above(BogLevel::WARN);

        dbog!("debug filtered");
        ibog!("info normal");
        ebog!("error shown as warn");
    }
}

// ----------------------------------------------------------
impl fmt::Debug for GLOBAL_BOGGER_STRUCT {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut ds = f.debug_struct("GLOBAL_BOGGER_STRUCT");

        ds.field("min_level", &self.min_level)
            .field("downcast_to", &self.downcast_to)
            .field("prefix", &self.prefix)
            .field("suffix", &self.suffix)
            .field("tag_override", &self.tag_override)
            .field("log", &self.log);

        // Opaque fields (trait objects / non-debuggable)
        ds.field("formatter", &"dyn BogFmter")
            .field("writer", &"dyn Write");

        ds.finish()
    }
}
