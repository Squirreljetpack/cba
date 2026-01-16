use easy_ext::ext;
pub use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};

#[ext(StrExt)]
impl str {
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

    pub fn truncate_left(&self, max: usize) -> String {
        let mut used = 0;
        let mut out = Vec::new();

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
        return self.into();
    }
}

/// # Notes
// \ is always consumed to aid with escaping custom parsing tokens.
///
/// # Example
///
/// ```rust
///
/// let mut out = String::with_capacity(s.len());
/// let mut chars = s.chars();
/// while let Some(c) = chars.next() {
///    if c == '\\' {
///        consume_escape(&mut chars, &mut out);
///        continue;
///    }
///    out.push(c);
/// }
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
