//! Display colored log-style messages for CLI tools
//! Performance is no concern, (hence `bog`), only convenience and style.

use std::{
    borrow::Cow,
    fmt::Display,
    io::{Write, stderr, stdout},
    sync::Mutex,
    u8,
};

#[derive(Clone, Copy, Debug)]
pub enum BogLevel {
    NOTE,
    ERROR,
    WARN,
    INFO,
    DEBUG,
    DNOTE,
    ALL, // this is never shown due to having priority 0
    CUSTOM(&'static str),
}

pub trait BogFmter {
    fn begin_tag(&self, level: BogLevel) -> String;
    fn end_tag(&self) -> &'static str {
        "\x1b[0m"
    }

    fn push_tag(&self, s: &mut String, tag: &str) {
        if !tag.is_empty() {
            s.push_str(": ");
            s.push_str(tag);
        }
    }

    fn format(&self, level: BogLevel, tag: &str, msg: &str) -> String {
        let mut s = self.begin_tag(level);
        self.push_tag(&mut s, tag);
        s.push_str(self.end_tag());

        if !msg.is_empty() {
            s.push(' ');
            s.push_str(msg);
        }

        s
    }

    fn priority(&self, level: &BogLevel) -> u8 {
        match level {
            BogLevel::NOTE => 120,
            BogLevel::ERROR => 100,
            BogLevel::WARN => 80,
            BogLevel::INFO => 60,
            BogLevel::DEBUG => 40,
            BogLevel::DNOTE => 20,
            BogLevel::ALL => 0, // don't change
            BogLevel::CUSTOM(_) => 120,
        }
    }
}

// --------  GLOBAL  ----------

#[allow(non_camel_case_types)]
pub struct GLOBAL_BOGGER_STRUCT {
    formatter: Box<dyn BogFmter + Send + Sync>,
    writer: Box<dyn Write + Send + Sync>,
    min_level: (u8, BogLevel),
    downcast_to: (u8, BogLevel),
    pub prefix: String,
    pub suffix: String,
    pub tag_override: Option<String>
}

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

    fn pause(&mut self) {
        self.min_level.0 = u8::MAX;
    }

    fn resume(&mut self) {
        self.min_level.0 = self.formatter.priority(&self.min_level.1)
    }

    fn filter_below(&mut self, lvl: BogLevel) {
        self.min_level = (self.formatter.priority(&lvl), lvl);
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

    fn init_global(logger: Box<dyn BogFmter + Send + Sync>, write: Box<dyn Write + Send + Sync>) {
        let bogger = GLOBAL_BOGGER_STRUCT {
            formatter: logger,
            writer: write,
            downcast_to: (255, BogLevel::ERROR),
            min_level: (0, BogLevel::DEBUG),
            prefix: String::new(),
            suffix: String::new(),
            tag_override: None
        };
        *GLOBAL_BOGGER.lock().unwrap() = Some(bogger);
    }
}

// since stderr has an internal lock i guess this isn't a huge deal anyways
static GLOBAL_BOGGER: Mutex<Option<GLOBAL_BOGGER_STRUCT>> = Mutex::new(None);

// ------- REEXPORT --------

#[inline]
pub fn bog(level: BogLevel, tag: &str, msg: &str) {
    Bogger::bog(level, tag, msg);
}

pub struct Bogger {}

pub struct BogContext {
    bounds: [Option<BogLevel>; 2],
    pause: bool,
    prefix: Option<String>,
    suffix: Option<String>,
    tag_override: Option<String>
}

impl BogContext {
    pub fn new() -> Self {
        Self {
            bounds: [None, None],
            pause: false,
            prefix: None,
            suffix: None,
            tag_override: None,
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
}

// organize under namespace
impl Bogger {
    // don't panic
    #[inline]
    pub fn bog(level: BogLevel, tag: &str, msg: &str) {
        if let Ok(mut guard) = GLOBAL_BOGGER.lock() {
            if let Some(b) = guard.as_mut() {
                b.bog(level, tag, msg);
            }
        }
    }

    #[inline]
    pub fn filter_below(lvl: BogLevel) {
        if let Ok(mut guard) = GLOBAL_BOGGER.lock() {
            if let Some(b) = guard.as_mut() {
                b.filter_below(lvl);
            }
        }
    }

    #[inline]
    pub fn downcast_above(lvl: BogLevel) {
        if let Ok(mut guard) = GLOBAL_BOGGER.lock() {
            if let Some(b) = guard.as_mut() {
                b.downcast_above(lvl);
            }
        }
    }

    #[inline]
    pub fn with<T>(context: BogContext, f: impl FnOnce() -> T) -> T {
        let (prev_bounds, prev_paused, prev_prefix, prev_suffix, prev_tag) = if let Ok(mut guard) = GLOBAL_BOGGER.lock() {
            if let Some(b) = guard.as_mut() {
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

                (Some(prev_bounds), Some(prev_paused), Some(prev_prefix), Some(prev_suffix), prev_tag)
            } else {
                (None, None, None, None, None)
            }
        } else {
            Default::default()
        };

        // Execute the closure
        let result = f();

        // Restore previous state
        if let Ok(mut guard) = GLOBAL_BOGGER.lock() {
            if let Some(b) = guard.as_mut() {
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
        }

        result
    }

    #[inline]
    pub fn paused<T>(f: impl FnOnce() -> T) -> T {
        Bogger::pause();
        let ret = f();
        Bogger::resume();
        ret
    }

    #[inline]
    pub fn pause() {
        if let Ok(mut guard) = GLOBAL_BOGGER.lock() {
            if let Some(b) = guard.as_mut() {
                b.pause();
            }
        }
    }

    #[inline]
    pub fn resume() {
        if let Ok(mut guard) = GLOBAL_BOGGER.lock() {
            if let Some(b) = guard.as_mut() {
                b.resume();
            }
        }
    }
}
// -------- IMPL ---------
pub struct Fg {}
impl BogFmter for Fg {
    fn begin_tag(&self, level: BogLevel) -> String {
        let (code, level) = match level {
            BogLevel::NOTE => ("34", "NOTE"),  // blue foreground
            BogLevel::ERROR => ("31", "ERRO"), // red foreground
            BogLevel::WARN => ("33", "WARN"),  // yellow foreground
            BogLevel::INFO => ("32", "INFO"),  // green foreground
            BogLevel::DEBUG => ("35", "DBUG"), // purple/magenta foreground
            BogLevel::DNOTE => ("30", "DNTE"), // black foreground
            BogLevel::ALL => ("", ""),         // unreachable
            BogLevel::CUSTOM(s) => ("34", s),  // blue foreground
        };
        format!("\x1b[{code}m[{level}")
    }
    fn end_tag(&self) -> &'static str {
        "]\x1b[0m"
    }
}

pub struct Bg {}
impl BogFmter for Bg {
    fn begin_tag(&self, level: BogLevel) -> String {
        let (code, level) = match level {
            BogLevel::NOTE => ("44", "NOTE "),  // blue bg
            BogLevel::ERROR => ("41", "ERROR"), // red bg
            BogLevel::WARN => ("43", "WARN "),  // yellow bg
            BogLevel::INFO => ("42", "INFO "),  // green bg
            BogLevel::DEBUG => ("45", "DEBUG"), // purple bg
            BogLevel::DNOTE => ("47", "DNOTE"), // white bg
            BogLevel::ALL => ("", ""),          // unreachable
            BogLevel::CUSTOM(s) => ("44", s),   // blue bg
        };
        format!("\x1b[30;{code}m{level}") // colored bg with black text (white also looks (worse))
    }
    fn push_tag(&self, s: &mut String, tag: &str) {
        if !tag.is_empty() {
            s.push_str("| ");
            s.push_str(tag);
        }
    }
    fn end_tag(&self) -> &'static str {
        " \x1b[0m"
    }
}

// ----------- PUBLIC -------------
pub fn init_bogger(fg: bool, output_stderr: bool) {
    let writer: Box<dyn Write + Send + Sync> = if output_stderr {
        Box::new(stderr())
    } else {
        Box::new(stdout())
    };

    if fg {
        GLOBAL_BOGGER_STRUCT::init_global(Box::new(Fg {}), writer);
    } else {
        GLOBAL_BOGGER_STRUCT::init_global(Box::new(Bg {}), writer);
    }
}

/// Initialize the global log filter based on a numeric verbosity level.
///
/// The verbosity value maps to a minimum [`BogLevel`] that will be emitted:
///
/// - `0` → show `ERROR` messages only
/// - `1` → show `WARN` and above
/// - `2` → show `INFO` and above
/// - `3` → show `DEBUG` and above
/// - `4` → show `DEBUGNOTE` and above
/// - `> 4` → show all messages
pub fn init_filter(verbosity: u8) {
    match verbosity {
        0 => Bogger::filter_below(BogLevel::ERROR),
        1 => Bogger::filter_below(BogLevel::WARN),
        2 => Bogger::filter_below(BogLevel::INFO),
        3 => Bogger::filter_below(BogLevel::DEBUG),
        4 => Bogger::filter_below(BogLevel::DNOTE),
        _ => Bogger::filter_below(BogLevel::ALL),
    }
}

// ----------- MACROS ------------------
#[macro_export]
macro_rules! ibog {
    // With tag expressions
    ($($harg:expr),* ; $($arg:expr),*) => {{
        $crate::bog::bog(
            $crate::bog::BogLevel::INFO,
            &format!($($harg),*),
            &format!($($arg),*),
        );
    }};
    // Without tag
    ($($arg:expr),*) => {{
        $crate::bog::bog(
            $crate::bog::BogLevel::INFO,
            "",
            &format!($($arg),*),
        );
    }};
}

#[macro_export]
macro_rules! dbog {
    ($($harg:expr),* ; $($arg:expr),*) => {{
        $crate::bog::bog(
            $crate::bog::BogLevel::DEBUG,
            &format!($($harg),*),
            &format!($($arg),*),
        );
    }};
    ($($arg:expr),*) => {{
        $crate::bog::bog(
            $crate::bog::BogLevel::DEBUG,
            "",
            &format!($($arg),*),
        );
    }};
}

#[macro_export]
macro_rules! ebog {
    ($($harg:expr),* ; $($arg:expr),*) => {{
        $crate::bog::bog(
            $crate::bog::BogLevel::ERROR,
            &format!($($harg),*),
            &format!($($arg),*),
        );
    }};
    ($($arg:expr),*) => {{
        $crate::bog::bog(
            $crate::bog::BogLevel::ERROR,
            "",
            &format!($($arg),*),
        );
    }};
}

#[macro_export]
macro_rules! wbog {
    ($($harg:expr),* ; $($arg:expr),*) => {{
        $crate::bog::bog(
            $crate::bog::BogLevel::WARN,
            &format!($($harg),*),
            &format!($($arg),*),
        );
    }};
    ($($arg:expr),*) => {{
        $crate::bog::bog(
            $crate::bog::BogLevel::WARN,
            "",
            &format!($($arg),*),
        );
    }};
}

#[macro_export]
macro_rules! nbog {
    ($($harg:expr),* ; $($arg:expr),*) => {{
        $crate::bog::bog(
            $crate::bog::BogLevel::NOTE,
            &format!($($harg),*),
            &format!($($arg),*),
        );
    }};
    ($($arg:expr),*) => {{
        $crate::bog::bog(
            $crate::bog::BogLevel::NOTE,
            "",
            &format!($($arg),*),
        );
    }};
}

#[macro_export]
macro_rules! dnbog {
    ($($harg:expr),* ; $($arg:expr),*) => {{
        $crate::bog::bog(
            $crate::bog::BogLevel::DNOTE,
            &format!($($harg),*),
            &format!($($arg),*),
        );
    }};
    ($($arg:expr),*) => {{
        $crate::bog::bog(
            $crate::bog::BogLevel::DNOTE,
            "",
            &format!($($arg),*),
        );
    }};
}

#[macro_export]
macro_rules! cbog {
    ($discriminant:literal ; $($harg:expr),* ; $($arg:expr),*) => {{
        $crate::bog::bog(
            $crate::bog::BogLevel::CUSTOM($discriminant),
            &format!($($harg),*),
            &format!($($arg),*),
        );
    }};
    ($discriminant:literal ; $($arg:expr),*) => {{
        $crate::bog::bog(
            $crate::bog::BogLevel::CUSTOM($discriminant),
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
/// if let Some(x) = fallible_result().or_err() {
///     process(x);
/// }
/// ```

#[easy_ext::ext(BogOkExt)]
pub impl<T, E: Display> Result<T, E> {
    fn or_bog_tagged<'a>(self, level: BogLevel, tag: impl Into<Cow<'a, str>>) -> Option<T> {
        match self {
            Ok(val) => Some(val),
            Err(e) => {
                bog(level, &tag.into(), &e.to_string());
                None
            }
        }
    }

    fn or_err_tagged<'a>(self, tag: impl Into<Cow<'a, str>>) -> Option<T> {
        self.or_bog_tagged(BogLevel::ERROR, tag)
    }

    fn or_warn_tagged<'a>(self, tag: impl Into<Cow<'a, str>>) -> Option<T> {
        self.or_bog_tagged(BogLevel::WARN, tag)
    }

    fn or_bog(self, level: BogLevel) -> Option<T> {
        self.or_bog_tagged(level, "")
    }

    fn or_err(self) -> Option<T> {
        self.or_err_tagged("")
    }

    fn or_warn(self) -> Option<T> {
        self.or_warn_tagged("")
    }
}

#[easy_ext::ext(BogUnwrapExt)]
pub impl<T> Option<T> {
    /// Unwrap or bog and exit
    fn or_bog_tagged<'a>(
        self,
        level: BogLevel,
        tag: impl Into<Cow<'a, str>>,
        msg: impl Into<Cow<'a, str>>,
    ) -> T {
        match self {
            Some(val) => val,
            None => {
                bog(level, &tag.into(), &msg.into());
                std::process::exit(1);
            }
        }
    }

    /// Unwrap or exit
    fn or_exit(self) -> T {
        match self {
            Some(val) => val,
            None => {
                std::process::exit(1);
            }
        }
    }

    /// Unwrap or bog and exit
    fn or_bog<'a>(self, level: BogLevel, msg: impl Into<Cow<'a, str>>) -> T {
        self.or_bog_tagged(level, "", msg)
    }

    /// Unwrap or err and exit
    fn or_err<'a>(self, msg: impl Into<Cow<'a, str>>) -> T {
        self.or_bog(BogLevel::ERROR, msg)
    }

    /// Unwrap or err and exit
    fn or_err_tagged<'a>(self, tag: impl Into<Cow<'a, str>>, msg: impl Into<Cow<'a, str>>) -> T {
        self.or_bog_tagged(BogLevel::ERROR, tag, msg)
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
        dnbog!("justification");
        dnbog!("DNOTE"; "ancillary");

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
        dnbog!("justification");
        dnbog!("DNOTE"; "ancillary");

        // CUSTOM
        cbog!("NOTE"; "Custom note message: {}", "all good");
        cbog!("NOTE"; ""; "Custom note with tag: {}", 123);
        cbog!("CUSTOM"; "Custom discriminant"; "Message with both tag and content");
    }

    #[test]
    fn min_level_and_downcast_combined() {
        init_bogger(true, false);

        // drop DEBUG/INFO entirely
        Bogger::filter_below(/* WARN priority */ BogLevel::INFO);
        // downcast ERROR to WARN
        Bogger::downcast_above(BogLevel::WARN);

        dbog!("debug filtered");
        ibog!("info normal");
        ebog!("error shown as warn");
    }
}
