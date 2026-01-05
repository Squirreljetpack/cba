// ----- MESSAGE FORMATTING --------

#[derive(Clone, Copy, Debug)]
pub enum BogLevel {
    NOTE,
    ERROR,
    WARN,
    INFO,
    DEBUG,
    EMPTY,
    ALL, // this is never shown due to having priority 0
    CUSTOM(&'static str),
}

/// Trait for formatting bog messages, passed to [`init_with`]
///
/// Also see: [`Fg`] and [`Bg`].
pub trait BogFmter {
    fn begin_tag(&self, level: BogLevel, tag: &str) -> String;
    fn end_tag(&self) -> &'static str {
        "\x1b[0m"
    }

    fn format(&self, level: BogLevel, tag: &str, msg: &str) -> String {
        let mut s = self.begin_tag(level, tag);
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
            BogLevel::ALL => 0, // don't change
            BogLevel::EMPTY => 120,
            BogLevel::CUSTOM(_) => 120,
        }
    }
}

// -------- IMPL ---------
pub struct Fg {}
impl BogFmter for Fg {
    fn begin_tag(&self, level: BogLevel, tag: &str) -> String {
        let (code, lvl) = match level {
            BogLevel::NOTE => ("34", "NOTE"),  // blue foreground
            BogLevel::ERROR => ("31", "ERRO"), // red foreground
            BogLevel::WARN => ("33", "WARN"),  // yellow foreground
            BogLevel::INFO => ("32", "INFO"),  // green foreground
            BogLevel::DEBUG => ("35", "DBUG"), // magenta foreground
            BogLevel::EMPTY => ("30", ""),     // black foreground
            BogLevel::ALL => ("", ""),         // unreachable
            BogLevel::CUSTOM(s) => ("34", s),  // blue foreground
        };
        let mut s = format!("\x1b[{code}m[{lvl}");
        if !tag.is_empty() {
            s.push_str(": ");
            s.push_str(tag);
        } else if matches!(level, BogLevel::EMPTY) {
            s.push_str(tag);
        };
        s
    }
    fn end_tag(&self) -> &'static str {
        "]\x1b[0m"
    }
}

pub struct Bg {}
impl BogFmter for Bg {
    fn begin_tag(&self, level: BogLevel, tag: &str) -> String {
        let (code, lvl) = match level {
            BogLevel::NOTE => ("44", "NOTE "),  // blue bg
            BogLevel::ERROR => ("41", "ERROR"), // red bg
            BogLevel::WARN => ("43", "WARN "),  // yellow bg
            BogLevel::INFO => ("42", "INFO "),  // green bg
            BogLevel::DEBUG => ("45", "DEBUG"), // purple bg
            BogLevel::EMPTY => ("47", ""),      // white bg
            BogLevel::ALL => ("", ""),          // unreachable
            BogLevel::CUSTOM(s) => ("44", s),   // blue bg
        };

        let mut start = format!("\x1b[30;{code}m{lvl}"); // colored bg with black text (white also looks (worse))
        if !tag.is_empty() {
            start.push_str("| ");
            start.push_str(tag);
        } else if matches!(level, BogLevel::EMPTY) {
            start.push_str(tag);
        };
        start
    }
    fn end_tag(&self) -> &'static str {
        " \x1b[0m"
    }
}
