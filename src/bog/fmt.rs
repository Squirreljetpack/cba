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
        use anstyle::{AnsiColor, Style};

        let (style, lvl) = match level {
            BogLevel::NOTE => (Style::new().fg_color(Some(AnsiColor::Blue.into())), "NOTE"),
            BogLevel::ERROR => (Style::new().fg_color(Some(AnsiColor::Red.into())), "ERRO"),
            BogLevel::WARN | BogLevel::_WRN => {
                (Style::new().fg_color(Some(AnsiColor::Yellow.into())), "WARN")
            }
            BogLevel::INFO | BogLevel::_NFO => {
                (Style::new().fg_color(Some(AnsiColor::Green.into())), "INFO")
            }
            BogLevel::DEBUG => (Style::new().fg_color(Some(AnsiColor::Magenta.into())), "DBUG"),
            BogLevel::EMPTY => (Style::new().fg_color(Some(AnsiColor::Black.into())), ""),
            BogLevel::___ => (Style::new(), ""),
            BogLevel::CUSTOM(s) => (Style::new().fg_color(Some(AnsiColor::Blue.into())), s),
        };

        let mut s = format!("{style}[{lvl}");
        if !tag.is_empty() {
            s.push_str(": ");
            s.push_str(tag);
        } else if matches!(level, BogLevel::EMPTY) {
            s.push_str(tag);
        };

        s.push_str("]");
        s.push_str(&style.render_reset().to_string());
        s
    }
}

pub struct Bg {}
impl BogFmter for Bg {
    fn tag(&self, level: BogLevel, tag: &str) -> String {
        use anstyle::{AnsiColor, Style};

        let (style, lvl) = match level {
            BogLevel::NOTE => (
                Style::new()
                    .fg_color(Some(AnsiColor::Black.into()))
                    .bg_color(Some(AnsiColor::Blue.into())),
                "NOTE ",
            ),
            BogLevel::ERROR => (
                Style::new()
                    .fg_color(Some(AnsiColor::Black.into()))
                    .bg_color(Some(AnsiColor::Red.into())),
                "ERROR",
            ),
            BogLevel::WARN | BogLevel::_WRN => (
                Style::new()
                    .fg_color(Some(AnsiColor::Black.into()))
                    .bg_color(Some(AnsiColor::Yellow.into())),
                "WARN ",
            ),
            BogLevel::INFO | BogLevel::_NFO => (
                Style::new()
                    .fg_color(Some(AnsiColor::Black.into()))
                    .bg_color(Some(AnsiColor::Green.into())),
                "INFO ",
            ),
            BogLevel::DEBUG => (
                Style::new()
                    .fg_color(Some(AnsiColor::Black.into()))
                    .bg_color(Some(AnsiColor::Magenta.into())),
                "DEBUG",
            ),
            BogLevel::EMPTY => (
                Style::new()
                    .fg_color(Some(AnsiColor::Black.into()))
                    .bg_color(Some(AnsiColor::White.into())),
                "",
            ),
            BogLevel::___ => (Style::new(), ""),
            BogLevel::CUSTOM(s) => (
                Style::new()
                    .fg_color(Some(AnsiColor::Black.into()))
                    .bg_color(Some(AnsiColor::Blue.into())),
                s,
            ),
        };

        let mut start = format!("{style}{lvl}");
        if !tag.is_empty() {
            start.push_str("| ");
            start.push_str(tag);
        } else if matches!(level, BogLevel::EMPTY) {
            start.push_str(tag);
        };
        start.push_str(" ");
        start.push_str(&style.render_reset().to_string());

        start
    }
}
