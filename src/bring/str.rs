use easy_ext::ext;
use std::fmt::Alignment;
pub use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};

#[ext(StrExt)]
impl str {
    /// Pad a string with spaces a specified number of times on the left and right.
    /// Not unicode aware
    pub fn pad(&self, left_count: usize, right_count: usize) -> String {
        let total_len = left_count + self.len() + right_count;
        let mut result = String::with_capacity(total_len);

        if left_count > 0 {
            result.push_str(&" ".repeat(left_count));
        }
        result.push_str(self);
        if right_count > 0 {
            result.push_str(&" ".repeat(right_count));
        }

        result
    }

    /// Pad a string to at least a maximum unicode width.
    /// Center align prioritizes left side padding when odd parity.
    pub fn pad_to(&self, max: usize, align: Alignment) -> String {
        let pad = max.saturating_sub(self.width());
        let mut s = self.to_string();

        match align {
            Alignment::Left => {
                s.extend(std::iter::repeat(' ').take(pad));
                s
            }
            Alignment::Right => {
                let mut out = String::new();
                out.extend(std::iter::repeat(' ').take(pad));
                out.push_str(&s);
                out
            }
            Alignment::Center => {
                let right = pad / 2;
                let left = pad - right;
                let mut out = String::new();
                out.extend(std::iter::repeat(' ').take(left));
                out.push_str(&s);
                out.extend(std::iter::repeat(' ').take(right));
                out
            }
        }
    }

    pub fn ellipsize(&self, max: usize, align: Alignment) -> String {
        let mut used = 0;
        let mut out = Vec::new();

        match align {
            Alignment::Right => {
                let mut iter = self.chars().rev();

                while let Some(ch) = iter.next() {
                    let cw = ch.width().unwrap_or(0);

                    if used + cw == max {
                        for c in iter.by_ref() {
                            if c.width().unwrap_or(0) != 0 {
                                out.push('…');
                                return out.into_iter().rev().collect();
                            }
                        }
                        return self.to_string();
                    } else if used + cw > max {
                        out.push('…');
                        return out.into_iter().rev().collect();
                    }

                    out.push(ch);
                    used += cw;
                }

                self.into()
            }

            Alignment::Left => {
                // Same as above, but without rev here.
                let mut iter = self.chars();

                while let Some(ch) = iter.next() {
                    let cw = ch.width().unwrap_or(0);

                    if used + cw == max {
                        for c in iter.by_ref() {
                            if c.width().unwrap_or(0) != 0 {
                                out.push('…');
                                return out.into_iter().collect();
                            }
                        }
                        return self.to_string();
                    } else if used + cw > max {
                        out.push('…');
                        return out.into_iter().collect();
                    }

                    out.push(ch);
                    used += cw;
                }

                self.into()
            }

            Alignment::Center => {
                unimplemented!("unimplemented");
            }
        }
    }

    /// Works like split_whitespace, but \ keeps tokens together.
    /// # Notes
    /// '\' escapes any character.
    pub fn split_escaped_by<F>(&self, is_sep: F) -> impl Iterator<Item = String>
    where
        F: FnMut(char) -> bool,
    {
        SplitEscapedBy::new(&self, is_sep)
    }
}

pub struct SplitEscapedBy<'a, F> {
    iter: std::str::Chars<'a>,
    is_sep: F,
    cur: String,
    escaped: bool,
    done: bool,
}

impl<'a, F> SplitEscapedBy<'a, F>
where
    F: FnMut(char) -> bool,
{
    pub fn new(s: &'a str, is_sep: F) -> Self {
        Self {
            iter: s.chars(),
            is_sep,
            cur: String::new(),
            escaped: false,
            done: false,
        }
    }
}

impl<'a, F> Iterator for SplitEscapedBy<'a, F>
where
    F: FnMut(char) -> bool,
{
    // escape removes the backslash so we allocate
    type Item = String;

    fn next(&mut self) -> Option<Self::Item> {
        if self.done {
            return None;
        }

        while let Some(ch) = self.iter.next() {
            if self.escaped {
                self.cur.push(ch);
                self.escaped = false;
            } else if ch == '\\' {
                self.escaped = true;
            } else if (self.is_sep)(ch) {
                if !self.cur.is_empty() {
                    return Some(std::mem::take(&mut self.cur));
                }
            } else {
                self.cur.push(ch);
            }
        }

        self.done = true;

        if self.cur.is_empty() {
            None
        } else {
            Some(std::mem::take(&mut self.cur))
        }
    }
}

/// # Notes
// \ is always consumed to aid with escaping custom parsing tokens.
///
/// # Example
///
/// ```rust
/// let s = r"hello\tworld";
/// let mut out = String::with_capacity(s.len());
/// let mut chars = s.chars();
/// while let Some(c) = chars.next() {
///    if c == '\\' {
///        consume_escaped(&mut chars, &mut out);
///        continue;
///    }
///    out.push(c);
/// }
/// assert_eq!(out, "hello\tworld");
/// ```
pub fn consume_escaped(chars: &mut impl Iterator<Item = char>, out: &mut String) {
    match parse_next_escape(chars) {
        Ok(e) => out.push(e),
        Err(orig) => {
            out.push(orig);
        }
    }
}

pub fn parse_next_escape<I>(chars: &mut I) -> Result<char, char>
where
    I: Iterator<Item = char>,
{
    let next = match chars.next() {
        Some(c) => c,
        None => return Err('\\'), // nothing after backslash
    };

    match next {
        'n' => Ok('\n'),
        'r' => Ok('\r'),
        't' => Ok('\t'),
        '\\' => Ok('\\'),
        '"' => Ok('"'),
        '\'' => Ok('\''),
        '0' => Ok('\0'),
        'x' => {
            let hi = chars.next();
            let lo = chars.next();
            if let (Some(hi), Some(lo)) = (hi, lo) {
                if let Ok(v) = u8::from_str_radix(&format!("{hi}{lo}"), 16) {
                    return Ok(v as char);
                }
            }
            Err('x')
        }
        'u' => {
            if chars.next() == Some('{') {
                let mut hex = String::new();
                for c in chars.by_ref() {
                    if c == '}' {
                        break;
                    }
                    hex.push(c);
                }
                if let Ok(v) = u32::from_str_radix(&hex, 16) {
                    if let Some(c) = char::from_u32(v) {
                        return Ok(c);
                    }
                }
                return Err('u');
            }
            Err('u')
        }
        other => Err(other),
    }
}

#[cfg(test)]
mod tests {
    use crate::vec_;

    use super::*;

    #[test]
    fn truncate_mixed_widths() {
        // widths: 你(2) a(1) 好(2) b(1) 世(2) c(1) 界(2)
        let s = "你a好b世c界";

        // total width = 11

        // keep left
        assert_eq!(s.ellipsize(5, Alignment::Left), "你a…"); // 2 + 1 + …
        assert_eq!(s.ellipsize(6, Alignment::Left), "你a好…"); // 2 + 1 + 2 + …

        // keep right
        assert_eq!(s.ellipsize(5, Alignment::Right), "…c界"); // … + 1 + 2
        assert_eq!(s.ellipsize(6, Alignment::Right), "…世c界"); // … + 2 + 1 + 2

        // widths: A(1) 你(2) B(1) 好(2) C(1)
        let s = "A你B好C";

        // exact fit on visible width, but more visible chars remain
        assert_eq!(s.ellipsize(3, Alignment::Left), "A…"); // A(1) + …, not "A你"
        assert_eq!(s.ellipsize(3, Alignment::Right), "…C"); // … + C(1)

        assert_eq!(s.ellipsize(4, Alignment::Left), "A你…"); // 1 + 2 + …
        assert_eq!(s.ellipsize(4, Alignment::Right), "…好C"); // … + 2 + 1
    }

    #[test]
    fn truncate_zero_width_chars() {
        // widths: 你(2) ZW(0) 好(2) ZW(0) 世(2)
        let s = "\u{200B}你好\u{200B}";

        assert_eq!(s.ellipsize(5, Alignment::Left), "\u{200B}你好\u{200B}");
        assert_eq!(s.ellipsize(5, Alignment::Right), "\u{200B}你好\u{200B}");

        // truncation past visible chars → ellipsis
        assert_eq!(s.ellipsize(3, Alignment::Left), "\u{200B}你…");
        assert_eq!(s.ellipsize(3, Alignment::Right), "…好\u{200B}");
    }

    #[test]
    fn split_escaped() {
        let s = r"a\,b,c\\,d,e\,f\,\,g\\,,,h";
        let parts: Vec<_> = SplitEscapedBy::new(s, |c| c == ',').collect();

        let expected: Vec<String> = vec_!["a,b", "c\\", "d", "e,f,,g\\", "h",];

        assert_eq!(parts, expected);
    }
}
