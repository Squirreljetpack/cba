mod table;
pub use table::TableBuilder;
pub mod split;
mod str;
pub use self::str::*;

/// Space + underscore as delimiters
pub fn camel_case(s: String) -> String {
    s.split(|c: char| c == '_' || c.is_whitespace())
        .filter(|p| !p.is_empty())
        .map(|part| {
            let mut chars = part.chars();
            match chars.next() {
                None => String::new(),
                Some(c) => c.to_ascii_uppercase().to_string() + chars.as_str(),
            }
        })
        .collect::<Vec<_>>()
        .join("")
}
