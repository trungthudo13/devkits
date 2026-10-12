use regex::RegexBuilder;

pub const INPUT_LIMIT: usize = 128 * 1024;
pub const MATCH_LIMIT: usize = 1000;
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Mode {
    Match,
    Substitution,
    Extraction,
}
#[derive(Clone, Debug)]
pub struct Capture {
    pub name: String,
    pub text: Option<String>,
    pub range: Option<(usize, usize)>,
}
#[derive(Clone, Debug)]
pub struct Found {
    pub bytes: (usize, usize),
    pub characters: (usize, usize),
    pub captures: Vec<Capture>,
}
#[derive(Default, Clone, Debug)]
pub struct ResultData {
    pub matches: Vec<Found>,
    pub output: String,
}

pub fn flags_valid(flags: &str) -> Result<(), String> {
    let mut seen = String::new();
    for c in flags.chars() {
        if !"gimsxUu".contains(c) {
            return Err(format!("Unsupported flag: {c}"));
        }
        if seen.contains(c) {
            return Err(format!("Duplicate flag: {c}"));
        }
        seen.push(c);
    }
    Ok(())
}

// Delimiters are presentation syntax, not part of the matching expression.
pub fn parse_expression(
    expression: &str,
    delimiter: Option<char>,
) -> Result<(String, String), String> {
    let Some(d) = delimiter else {
        return Ok((expression.into(), String::new()));
    };
    if !expression.starts_with(d) {
        return Err(format!("Expected opening delimiter {d}"));
    }
    let mut escaped = false;
    let mut end = None;
    for (i, c) in expression.char_indices().skip(1) {
        if escaped {
            escaped = false;
            continue;
        }
        if c == '\\' {
            escaped = true;
            continue;
        }
        if c == d {
            end = Some(i);
        }
    }
    let end = end.ok_or_else(|| format!("Missing closing delimiter {d}"))?;
    let flags = &expression[end + d.len_utf8()..];
    flags_valid(flags)?;
    Ok((expression[d.len_utf8()..end].into(), flags.into()))
}
pub fn expression(pattern: &str, delimiter: Option<char>, flags: &str) -> String {
    let Some(d) = delimiter else {
        return pattern.into();
    };
    let mut result = String::new();
    result.push(d);
    let mut escaped = false;
    for c in pattern.chars() {
        if c == d && !escaped {
            result.push('\\');
        }
        result.push(c);
        if escaped {
            escaped = false;
        } else if c == '\\' {
            escaped = true;
        }
    }
    result.push(d);
    result.push_str(flags);
    result
}

pub fn evaluate(
    pattern: &str,
    input: &str,
    flags: &str,
    mode: Mode,
    template: &str,
    separator: &str,
) -> Result<ResultData, String> {
    flags_valid(flags)?;
    if pattern.len() > 4096 {
        return Err("Pattern exceeds 4 KiB.".into());
    }
    if input.len() > INPUT_LIMIT {
        return Err("Input exceeds 128 KiB.".into());
    }
    if template.len() > 8192 {
        return Err("Template exceeds 8 KiB.".into());
    }
    let regex = RegexBuilder::new(pattern)
        .case_insensitive(flags.contains('i'))
        .multi_line(flags.contains('m'))
        .dot_matches_new_line(flags.contains('s'))
        .ignore_whitespace(flags.contains('x'))
        .swap_greed(flags.contains('U'))
        .unicode(flags.contains('u'))
        .size_limit(2 * 1024 * 1024)
        .build()
        .map_err(|e| e.to_string())?;
    if regex.captures_len() > 65 {
        return Err("At most 64 capture groups are supported.".into());
    }
    let mut positions = vec![0; input.len() + 1];
    let mut char_count = 0;
    for (i, _) in input.char_indices() {
        positions[i] = char_count;
        char_count += 1;
    }
    positions[input.len()] = char_count;
    let mut result = ResultData::default();
    let mut previous = 0;
    for captures in regex.captures_iter(input) {
        if result.matches.len() == MATCH_LIMIT {
            return Err(
                "More than 1,000 matches. Narrow the expression or shorten the input.".into(),
            );
        }
        let whole = captures.get(0).unwrap();
        let groups = regex
            .capture_names()
            .enumerate()
            .map(|(i, name)| {
                let m = captures.get(i);
                Capture {
                    name: name
                        .map(|n| format!("{i}: {n}"))
                        .unwrap_or_else(|| i.to_string()),
                    text: m.map(|m| m.as_str().to_owned()),
                    range: m.map(|m| (positions[m.start()], positions[m.end()])),
                }
            })
            .collect();
        result.matches.push(Found {
            bytes: (whole.start(), whole.end()),
            characters: (positions[whole.start()], positions[whole.end()]),
            captures: groups,
        });
        match mode {
            Mode::Match => {}
            Mode::Substitution => {
                result.output.push_str(&input[previous..whole.start()]);
                captures.expand(template, &mut result.output);
                previous = whole.end();
            }
            Mode::Extraction => {
                if result.matches.len() > 1 {
                    result.output.push_str(separator);
                }
                captures.expand(template, &mut result.output);
            }
        }
        if result.output.len() > crate::MAX_INPUT {
            return Err("Output exceeds 2 MiB.".into());
        }
        if !flags.contains('g') {
            break;
        }
    }
    if mode == Mode::Substitution {
        result.output.push_str(&input[previous..]);
    }
    if result.output.len() > crate::MAX_INPUT {
        return Err("Output exceeds 2 MiB.".into());
    }
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn modes_captures_and_unicode() {
        let pattern = r"(?P<name>\w+)=(\d+)";
        let input = "Việt=12 Bob=34";
        let found = evaluate(pattern, input, "gu", Mode::Match, "", "").unwrap();
        assert_eq!(found.matches.len(), 2);
        assert_eq!(found.matches[0].characters, (0, 7));
        assert_eq!(found.matches[0].captures[1].text.as_deref(), Some("Việt"));
        assert_eq!(
            evaluate(pattern, input, "gu", Mode::Substitution, "${name}:$2", "")
                .unwrap()
                .output,
            "Việt:12 Bob:34"
        );
        assert_eq!(
            evaluate(pattern, input, "gu", Mode::Extraction, "$2", ",")
                .unwrap()
                .output,
            "12,34"
        );
        assert_eq!(
            evaluate(pattern, input, "u", Mode::Substitution, "X", "")
                .unwrap()
                .output,
            "X Bob=34"
        );
    }
    #[test]
    fn flags_and_edge_cases() {
        assert_eq!(
            evaluate("^a$", "A\na", "gimu", Mode::Match, "", "")
                .unwrap()
                .matches
                .len(),
            2
        );
        assert_eq!(
            evaluate("a.b", "a\nb", "su", Mode::Match, "", "")
                .unwrap()
                .matches
                .len(),
            1
        );
        assert_eq!(
            evaluate(
                "a .+ b # comment",
                "aXXbYYb",
                "xUu",
                Mode::Extraction,
                "$0",
                ""
            )
            .unwrap()
            .output,
            "aXXb"
        );
        assert_eq!(
            evaluate("z", "abc", "gu", Mode::Substitution, "X", "")
                .unwrap()
                .output,
            "abc"
        );
        assert_eq!(
            evaluate("", "é", "gu", Mode::Match, "", "")
                .unwrap()
                .matches
                .len(),
            2
        );
        assert!(evaluate("[", "abc", "gu", Mode::Match, "", "").is_err());
        assert!(evaluate("a", "a", "j", Mode::Match, "", "").is_err());
        assert!(evaluate("", "a".repeat(1001).as_str(), "gu", Mode::Match, "", "").is_err());
        assert_eq!(
            evaluate("(a)?b", "b", "gu", Mode::Match, "", "")
                .unwrap()
                .matches[0]
                .captures[1]
                .text,
            None
        );
    }
    #[test]
    fn delimiter_roundtrip() {
        for delimiter in ['/', '#', '~', '|'] {
            let expression = expression("a/b", Some(delimiter), "gi");
            let (p, f) = parse_expression(&expression, Some(delimiter)).unwrap();
            assert_eq!(
                evaluate(&p, "A/B", &f, Mode::Match, "", "")
                    .unwrap()
                    .matches
                    .len(),
                1
            );
        }
        assert!(parse_expression("/abc", Some('/')).is_err());
        assert!(parse_expression("/abc/gg", Some('/')).is_err());
    }
}
