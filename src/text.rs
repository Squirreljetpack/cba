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