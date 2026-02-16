/// Splits a string on whitespace, respecting nested parentheses and escaped parentheses (`\(` or `\)`).
///
/// - Whitespace outside parentheses is a delimiter.
/// - Nesting starts at the first unescaped `(` and ends at the matching `)`.
/// - Outermost parentheses are omitted in the result.
/// - Escaped parentheses are included literally in tokens.
///
/// Returns `Ok(Vec<String>)` on success, or `Err(i32)` on unbalanced parentheses:
/// - Positive value: number of unmatched opening parentheses remaining at end of input.
/// - Negative value: index of the extra closing parenthesis encountered.
///
/// # Examples
///
/// ```rust
/// use cli_boilerplate_automation::text::split::split_nesting;
///
/// let input3 = "foo (bar";
/// match split_nesting(input3) {
///     Ok(_) => unreachable!(),
///     Err(n) if n > 0 => println!("Encountered {} unclosed parentheses", n),
///     Err(n) if n < 0 => println!("Extra closing parenthesis at index {}", -n),
///     _ => unreachable!(),
/// }
///
/// ```
pub fn split_nesting(input: &str, left: char, right: char) -> Result<Vec<String>, i32> {
    let mut result = Vec::new();
    let mut nesting: i32 = 0;
    let mut token = String::new();

    let mut chars = input.chars().enumerate().peekable();

    while let Some((i, ch)) = chars.next() {
        match ch {
            '\\' => {
                if let Some(&(_, next)) = chars.peek() {
                    if next == left || next == right {
                        token.push(next);
                        chars.next(); // consume escaped char
                        continue;
                    }
                }
                token.push(ch);
            }

            c if c == left => {
                if nesting > 0 {
                    token.push(ch); // nested '(' included
                }
                nesting += 1;
            }

            c if c == right => {
                nesting -= 1;
                if nesting < 0 {
                    return Err(-(i as i32));
                }

                if nesting == 0 {
                    // end of outermost parentheses: push token
                    if !token.is_empty() {
                        result.push(token.clone());
                        token.clear();
                    }
                    // omit outer ')'
                } else {
                    token.push(ch); // nested ')' included
                }
            }

            c if c.is_whitespace() && nesting == 0 => {
                if !token.is_empty() {
                    result.push(token.clone());
                    token.clear();
                }
            }

            c => {
                token.push(c);
            }
        }
    }

    if nesting != 0 {
        return Err(nesting);
    }

    if !token.is_empty() {
        result.push(token);
    }

    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_and_nested() {
        let input = "foo (bar baz (qux)) quux";
        let parsed = split_nesting(input, '(', ')').unwrap();
        assert_eq!(parsed, vec!["foo", "bar baz (qux)", "quux"]);
    }

    #[test]
    fn test_escaped_parentheses() {
        let input = r"foo \(bar baz\) qux";
        let parsed = split_nesting(input, '(', ')').unwrap();
        assert_eq!(parsed, vec!["foo", "(bar", "baz)", "qux"]);
    }

    #[test]
    fn test_outer_parentheses_omitted() {
        let input = "(foo bar)";
        let parsed = split_nesting(input, '(', ')').unwrap();
        assert_eq!(parsed, vec!["foo bar"]);
    }

    #[test]
    fn test_outer_with_nested_and_escapes() {
        let input = r"(foo \(bar) baz (qux\))";
        let parsed = split_nesting(input, '(', ')').unwrap();
        assert_eq!(parsed, vec!["foo (bar", "baz", "qux)"]);
    }

    #[test]
    fn test_unbalanced() {
        assert_eq!(split_nesting("foo (bar", '(', ')').unwrap_err(), 1);
        assert_eq!(split_nesting("foo )", '(', ')').unwrap_err(), -1);
    }
}
