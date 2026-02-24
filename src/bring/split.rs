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
pub fn split_whitespace_preserving_nesting(
    input: &str,
    non_consuming: Option<[char; 2]>,
    consuming: Option<[char; 2]>,
) -> Result<Vec<String>, i32> {
    let mut result = Vec::new();
    let mut nesting: i32 = 0;
    let mut token = String::new();
    let [left, right] = non_consuming.unwrap_or_else(|| consuming.unwrap());

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
        let parsed = split_whitespace_preserving_nesting(input, Some(['(', ')']), None).unwrap();
        assert_eq!(parsed, vec!["foo", "(bar baz (qux))", "quux"]);
    }

    #[test]
    fn test_escaped_parentheses() {
        let input = r"foo \(bar baz\) qux";
        let parsed = split_whitespace_preserving_nesting(input, Some(['(', ')']), None).unwrap();
        assert_eq!(parsed, vec!["foo", "(bar", "baz)", "qux"]);
    }

    #[test]
    fn test_outer_parentheses_consuming() {
        let input = "(foo bar)";
        let parsed = split_whitespace_preserving_nesting(input, None, Some(['(', ')'])).unwrap();
        assert_eq!(parsed, vec!["foo bar"]);
    }

    #[test]
    fn test_outer_with_nested_and_escapes_consuming() {
        let input = r"(foo \(bar) baz (qux\))";
        let parsed = split_whitespace_preserving_nesting(input, None, Some(['(', ')'])).unwrap();
        assert_eq!(parsed, vec!["foo (bar", "baz", "qux)"]);
    }

    #[test]
    fn test_unbalanced() {
        assert_eq!(
            split_whitespace_preserving_nesting("foo (bar", Some(['(', ')']), None).unwrap_err(),
            1
        );
        assert_eq!(
            split_whitespace_preserving_nesting("foo )", Some(['(', ')']), None).unwrap_err(),
            -4
        );
    }

    #[test]
    fn test_mixed_brackets_and_escapes() {
        let input = r"(( )) [one word] [\[]";
        let parsed =
            split_whitespace_preserving_nesting(input, Some(['(', ')']), Some(['[', ']'])).unwrap();
        assert_eq!(parsed, vec!["(( ))", "one word", "["]);
    }
}

pub fn split_on_nesting(input: &str, brackets: [char; 2]) -> Result<Vec<String>, i32> {
    let [open, close] = brackets;
    let mut results = Vec::new();
    let mut current_chunk = String::new();
    let mut level = 0;
    let mut escaped = false;

    for (i, c) in input.chars().enumerate() {
        if escaped {
            current_chunk.push(c);
            escaped = false;
            continue;
        }

        if c == '\\' {
            // Check lookahead for brackets
            if input[i + c.len_utf8()..].starts_with(open)
                || input[i + c.len_utf8()..].starts_with(close)
            {
                escaped = true;
                current_chunk.push(c);
                continue;
            }
        }

        if c == open {
            if level == 0 && !current_chunk.is_empty() {
                results.push(current_chunk);
                current_chunk = String::new();
            }
            level += 1;
        } else if c == close {
            level -= 1;
            if level < 0 {
                return Err(-(i as i32));
            }
        }

        current_chunk.push(c);

        // Split after pushing the closing bracket if we are back at ground level
        if c == close && level == 0 && !current_chunk.is_empty() {
            results.push(current_chunk);
            current_chunk = String::new();
        }
    }

    if level > 0 {
        return Err(level);
    }

    if !current_chunk.is_empty() {
        results.push(current_chunk);
    }

    Ok(results)
}

#[cfg(test)]
mod nesting_tests {
    use super::*;

    #[test]
    fn test_basic_nesting() {
        let input = "a{b}c{d}";
        let expected = Ok(vec![
            "a".to_string(),
            "{b}".to_string(),
            "c".to_string(),
            "{d}".to_string(),
        ]);
        assert_eq!(split_on_nesting(input, ['{', '}']), expected);
    }

    #[test]
    fn test_deep_nesting() {
        // Should not split inside the nesting
        let input = "outside{level1{level2}}also_outside";
        let expected = Ok(vec![
            "outside".to_string(),
            "{level1{level2}}".to_string(),
            "also_outside".to_string(),
        ]);
        assert_eq!(split_on_nesting(input, ['{', '}']), expected);
    }

    #[test]
    fn test_escaped_brackets() {
        let input = "a\\{b{c}d";
        let expected = Ok(vec![
            "a\\{b".to_string(),
            "{c}".to_string(),
            "d".to_string(),
        ]);
        assert_eq!(split_on_nesting(input, ['{', '}']), expected);
    }

    #[test]
    fn test_unclosed_error() {
        let input = "a{b{c}";
        assert_eq!(split_on_nesting(input, ['{', '}']), Err(1));
    }

    #[test]
    fn test_negative_index_error() {
        let input = "a{b}}c";
        // The second '}' is at index 4
        assert_eq!(split_on_nesting(input, ['{', '}']), Err(-4));
    }

    #[test]
    fn test_start_with_bracket() {
        let input = "{a}b";
        let expected = Ok(vec!["{a}".to_string(), "b".to_string()]);
        assert_eq!(split_on_nesting(input, ['{', '}']), expected);
    }
}
