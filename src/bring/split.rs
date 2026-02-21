/// Splits a string on whitespace, respecting nested delimiters and escapes.
///
/// - Whitespace outside delimiters is a separator.
/// - `non_consuming` specifies a delimiter pair whose outer delimiters are preserved.
/// - `consuming` specifies a delimiter pair whose outer delimiters are removed.
/// - At least one of `non_consuming` or `consuming` must be `Some`.
/// - Nested delimiters of the same active type are always preserved.
/// - Escaped whitespace outside delimiters and escaped delimiters (`\(`, `\]`, etc.) and are included literally.
///
/// Returns `Ok(Vec<String>)` on success, or `Err(i32)` on unbalanced delimiters:
/// - Positive value: number of unmatched opening delimiters remaining at end.
/// - Negative value: index of the extra closing delimiter encountered.
pub fn split_nesting(
    input: &str,
    non_consuming: impl Into<Option<[char; 2]>>,
    consuming: impl Into<Option<[char; 2]>>,
) -> Result<Vec<String>, i32> {
    let mut result = Vec::new();
    let mut nesting: i32 = 0;
    let mut token = String::new();
    let consuming = consuming.into();
    let [left, right] = non_consuming.into().unwrap_or_else(|| consuming.unwrap());

    let mut chars = input.chars().enumerate().peekable();
    let mut in_consuming_delimiter = false;

    while let Some((i, c)) = chars.next() {
        let is_consuming_delimiter =
            |right: bool| consuming.is_some_and(|x| c == x[right as usize]);

        match c {
            '\\' => {
                if let Some(&(_, next)) = chars.peek() {
                    if nesting == 0
                        && ([left, right].contains(&next)
                            || consuming.is_some_and(|x| x.contains(&next))
                            || next.is_whitespace())
                    {
                        token.push(next);
                        chars.next();
                        continue;
                    } else if (in_consuming_delimiter
                        && consuming.is_some_and(|x| x.contains(&next)))
                        || (!in_consuming_delimiter && [left, right].contains(&next))
                    {
                        token.push(next);
                        chars.next();
                        continue;
                    }
                }
                token.push(c);
            }

            // opening
            c if c == left || is_consuming_delimiter(false) => {
                if nesting > 0 {
                    if in_consuming_delimiter ^ is_consuming_delimiter(false) {
                        continue;
                    }
                    token.push(c);
                } else if is_consuming_delimiter(false) {
                    in_consuming_delimiter = true;
                } else {
                    token.push(c);
                }
                nesting += 1;
            }

            // closing
            c if (c == right && !in_consuming_delimiter)
                || (is_consuming_delimiter(true) && in_consuming_delimiter) =>
            {
                nesting -= 1;
                if nesting < 0 {
                    return Err(-(i as i32));
                }

                if nesting == 0 {
                    if !is_consuming_delimiter(true) {
                        token.push(c);
                    }

                    if !token.is_empty() {
                        result.push(token.clone());
                        token.clear();
                    }
                    in_consuming_delimiter = false;
                } else {
                    token.push(c);
                }
            }

            c if c.is_whitespace() && nesting == 0 => {
                if !token.is_empty() {
                    result.push(token.clone());
                    token.clear();
                }
            }

            c => token.push(c),
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
    fn test_basic_and_nested_non_consuming_default() {
        let input = "foo (bar baz (qux)) quux";
        let parsed = split_nesting(input, Some(['(', ')']), None).unwrap();
        assert_eq!(parsed, vec!["foo", "(bar baz (qux))", "quux"]);
    }

    #[test]
    fn test_escaped_parentheses() {
        let input = r"foo \(bar baz\) qux";
        let parsed = split_nesting(input, Some(['(', ')']), None).unwrap();
        assert_eq!(parsed, vec!["foo", "(bar", "baz)", "qux"]);
    }

    #[test]
    fn test_outer_parentheses_consuming() {
        let input = "(foo bar)";
        let parsed = split_nesting(input, None, Some(['(', ')'])).unwrap();
        assert_eq!(parsed, vec!["foo bar"]);
    }

    #[test]
    fn test_outer_with_nested_and_escapes_consuming() {
        let input = r"(foo \(bar) baz (qux\))";
        let parsed = split_nesting(input, None, Some(['(', ')'])).unwrap();
        assert_eq!(parsed, vec!["foo (bar", "baz", "qux)"]);
    }

    #[test]
    fn test_unbalanced() {
        assert_eq!(
            split_nesting("foo (bar", Some(['(', ')']), None).unwrap_err(),
            1
        );
        assert_eq!(
            split_nesting("foo )", Some(['(', ')']), None).unwrap_err(),
            -4
        );
    }

    #[test]
    fn test_mixed_brackets_and_escapes() {
        let input = r"(( )) [one word] [\[]";
        let parsed = split_nesting(input, Some(['(', ')']), Some(['[', ']'])).unwrap();
        assert_eq!(parsed, vec!["(( ))", "one word", "["]);
    }
}
