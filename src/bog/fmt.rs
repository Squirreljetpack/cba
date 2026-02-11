// ----- MESSAGE FORMATTING --------

#[derive(Clone, Copy, Debug)]
pub enum BogLevel {
    ///
    NOTE,
    ERROR,
    WARN,
    INFO,
    /// Low priority warn
    _WRN,
    /// Low priority info
    _NFO,
    DEBUG,
    EMPTY,
    /// A "never" level.
    ///
    /// # Note
    /// This should be kept at priority 0 in [`BogFmter::priority`].
    ___,

    CUSTOM(&'static str), // e.g. trace
}

/// Trait for formatting bog messages, passed to [`init_with`]
///
/// Also see: [`Fg`] and [`Bg`].
pub trait BogFmter {
    fn tag(&self, level: BogLevel, tag: &str) -> String;

    fn format(&self, level: BogLevel, tag: &str, msg: &str) -> String {
        let mut s = self.tag(level, tag);

        if !msg.is_empty() {
            s.push(' ');
            s.push_str(msg);
        }

        s
    }

    /// Order the levels with numeric values.
    /// The values are used for [`downcasting`](super::BOGGER::downcast_above) and [`filtering`](super::BOGGER::filter_below).
    ///
    /// Note that the value of [`BogLevel::___`] is expected to be 0.
    fn priority(&self, level: &BogLevel) -> u8 {
        match level {
            BogLevel::NOTE => 120,
            BogLevel::ERROR => 100,
            BogLevel::WARN => 80,
            BogLevel::INFO => 60,
            BogLevel::DEBUG => 20,
            BogLevel::___ => 0, // don't change

            // usually you'll want to change these
            BogLevel::_WRN | BogLevel::_NFO => 40,
            BogLevel::EMPTY => 120,
            BogLevel::CUSTOM(_) => 120,
        }
    }
}

// -------- IMPL ---------
pub struct Fg {}
impl BogFmter for Fg {
    fn tag(&self, level: BogLevel, tag: &str) -> String {
        let (code, lvl) = match level {
            BogLevel::NOTE => ("34", "NOTE"),                  // blue
            BogLevel::ERROR => ("31", "ERRO"),                 // red
            BogLevel::WARN | BogLevel::_WRN => ("33", "WARN"), // yellow
            BogLevel::INFO | BogLevel::_NFO => ("32", "INFO"), // green
            BogLevel::DEBUG => ("35", "DBUG"),                 // magenta
            BogLevel::EMPTY => ("30", ""),                     // black
            BogLevel::___ => ("", ""),                         // unreachable
            BogLevel::CUSTOM(s) => ("34", s),                  // blue
        };
        let mut s = format!("\x1b[{code}m[{lvl}");
        if !tag.is_empty() {
            s.push_str(": ");
            s.push_str(tag);
        } else if matches!(level, BogLevel::EMPTY) {
            s.push_str(tag);
        };

        s.push_str("]\x1b[0m");
        s
    }
}

pub struct Bg {}
impl BogFmter for Bg {
    fn tag(&self, level: BogLevel, tag: &str) -> String {
        let (code, lvl) = match level {
            BogLevel::NOTE => ("44", "NOTE "),                  // blue
            BogLevel::ERROR => ("41", "ERROR"),                 // red
            BogLevel::WARN | BogLevel::_WRN => ("43", "WARN "), // yellow
            BogLevel::INFO | BogLevel::_NFO => ("42", "INFO "), // green
            BogLevel::DEBUG => ("45", "DEBUG"),                 // purple
            BogLevel::EMPTY => ("47", ""),                      // white
            BogLevel::___ => ("", ""),                          // unreachable
            BogLevel::CUSTOM(s) => ("44", s),                   // blue
        };

        let mut start = format!("\x1b[30;{code}m{lvl}"); // colored bg with black text (white also looks bad/worse)
        if !tag.is_empty() {
            start.push_str("| ");
            start.push_str(tag);
        } else if matches!(level, BogLevel::EMPTY) {
            start.push_str(tag);
        };
        start.push_str(" \x1b[0m");

        start
    }
}
