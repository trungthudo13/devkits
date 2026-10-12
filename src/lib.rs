pub mod regex_tools;
use rand::{rngs::OsRng, seq::SliceRandom, Rng};
use sqlparser::{
    dialect::{Dialect, GenericDialect, MySqlDialect, PostgreSqlDialect, SQLiteDialect},
    parser::Parser,
    tokenizer::{Token, Tokenizer},
};

pub const MAX_INPUT: usize = 2 * 1024 * 1024;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Language {
    Json,
    Xml,
    Sql,
}
impl Language {
    pub fn extension(self) -> &'static str {
        match self {
            Self::Json => "json",
            Self::Xml => "xml",
            Self::Sql => "sql",
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Action {
    Format,
    Minify,
    Lint,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SqlDialect {
    Generic,
    PostgreSql,
    MySql,
    Sqlite,
}
impl SqlDialect {
    fn parser(self) -> Box<dyn Dialect> {
        match self {
            Self::Generic => Box::new(GenericDialect {}),
            Self::PostgreSql => Box::new(PostgreSqlDialect {}),
            Self::MySql => Box::new(MySqlDialect {}),
            Self::Sqlite => Box::new(SQLiteDialect {}),
        }
    }
}

#[derive(Clone, Copy, Default)]
pub struct SqlFormatOptions {
    pub keyword_uppercase: bool,
    pub alias_align: bool,
}

pub fn process(
    input: &str,
    lang: Language,
    action: Action,
    indent: usize,
    dialect: SqlDialect,
) -> Result<String, String> {
    process_with_sql_options(
        input,
        lang,
        action,
        indent,
        dialect,
        SqlFormatOptions::default(),
    )
}

pub fn process_with_sql_options(
    input: &str,
    lang: Language,
    action: Action,
    indent: usize,
    dialect: SqlDialect,
    options: SqlFormatOptions,
) -> Result<String, String> {
    if input.len() > MAX_INPUT {
        return Err("Input exceeds the 2 MiB limit.".into());
    }
    if input.trim().is_empty() {
        return Err("Enter some data first.".into());
    }
    match lang {
        Language::Json => {
            let _: serde_json::Value = serde_json::from_str(input).map_err(|e| e.to_string())?;
            if action == Action::Lint {
                Ok("Valid JSON. Syntax check passed.".into())
            } else {
                Ok(json_layout(
                    input,
                    action == Action::Format,
                    indent.clamp(1, 8),
                ))
            }
        }
        Language::Xml => xml(input, action, indent),
        Language::Sql => {
            let dialect = dialect.parser();
            let statements =
                Parser::parse_sql(dialect.as_ref(), input).map_err(|e| e.to_string())?;
            if statements.is_empty() {
                return Err("No SQL statements found.".into());
            }
            let result = match action {
                Action::Lint => Ok::<String, String>(format!("Valid SQL. {} statement(s); syntax check passed. Schema and database semantics are not checked.", statements.len())),
                Action::Format => {
                    let tokens = Tokenizer::new(dialect.as_ref(), input).with_unescape(false).tokenize().map_err(|e| e.to_string())?;
                    let significant: Vec<_> = tokens.iter().filter(|t| !matches!(t, Token::Whitespace(_))).collect();
                    let function_names: Vec<String> = significant.windows(2).filter_map(|pair| {
                        if let (Token::Word(word), Token::LParen) = (pair[0], pair[1]) {
                            if !["IN", "EXISTS", "OVER", "VALUES", "SELECT", "WITH", "CAST", "EXTRACT", "FILTER"].contains(&word.value.to_uppercase().as_str()) { return Some(word.value.clone()); }
                        }
                        None
                    }).collect();
                    let mut prefix = "__devkits_literal_".to_owned();
                    while input.contains(&prefix) { prefix.push('_'); }
                    let mut literals = Vec::new();
                    let protected = tokens.into_iter().map(|t| {
                        if matches!(t, Token::DollarQuotedString(_)) {
                            let marker = format!("'{prefix}{}'", literals.len());
                            literals.push((marker.clone(), t.to_string()));
                            marker
                        } else { t.to_string() }
                    }).collect::<String>();
                    let mut formatted = sqlformat::format(&protected, &sqlformat::QueryParams::None, &sqlformat::FormatOptions { indent: sqlformat::Indent::Spaces(indent.clamp(1,8) as u8), uppercase: options.keyword_uppercase.then_some(true), ignore_case_convert: Some(function_names.iter().map(String::as_str).collect()), ..Default::default() });
                    for (marker, literal) in literals { formatted = formatted.replace(&marker, &literal); }
                    if options.alias_align { formatted = align_sql_aliases(&formatted, dialect.as_ref())?; }
                    Ok(formatted)
                },
                Action::Minify => {
                    let tokens = Tokenizer::new(dialect.as_ref(), input).with_unescape(false).tokenize().map_err(|e| e.to_string())?;
                    Ok(tokens.into_iter().filter(|t| !matches!(t, Token::Whitespace(_))).map(|t| t.to_string()).collect::<Vec<_>>().join(" "))
                }
            }?;
            if action != Action::Lint {
                let reparsed = Parser::parse_sql(dialect.as_ref(), &result)
                    .map_err(|e| format!("Unable to safely rewrite SQL: {e}"))?;
                if reparsed.iter().map(ToString::to_string).collect::<Vec<_>>()
                    != statements
                        .iter()
                        .map(ToString::to_string)
                        .collect::<Vec<_>>()
                {
                    return Err(
                        "Rewrite would change the SQL syntax tree; original input preserved."
                            .into(),
                    );
                }
            }
            Ok(result)
        }
    }
}

// Align consecutive projection aliases using lexer positions, never text inside literals.
fn align_sql_aliases(input: &str, dialect: &dyn Dialect) -> Result<String, String> {
    use sqlparser::keywords::Keyword;
    let tokens = Tokenizer::new(dialect, input)
        .tokenize_with_location()
        .map_err(|e| e.to_string())?;
    let mut aliases = Vec::new();
    let mut parens = 0usize;
    for token in tokens {
        match token.token {
            Token::LParen => parens += 1,
            Token::RParen => parens = parens.saturating_sub(1),
            Token::Word(w) if w.keyword == Keyword::AS && parens == 0 => aliases.push((
                token.span.start.line as usize - 1,
                token.span.start.column as usize - 1,
            )),
            _ => {}
        }
    }
    let mut lines: Vec<String> = input.split('\n').map(str::to_owned).collect();
    let mut start = 0;
    while start < aliases.len() {
        let mut end = start + 1;
        while end < aliases.len() && aliases[end].0 == aliases[end - 1].0 + 1 {
            end += 1;
        }
        let width = aliases[start..end].iter().map(|(_, c)| *c).max().unwrap();
        for &(row, column) in &aliases[start..end] {
            let byte = lines[row]
                .char_indices()
                .nth(column)
                .map(|(i, _)| i)
                .unwrap_or(lines[row].len());
            lines[row].insert_str(byte, &" ".repeat(width - column));
        }
        start = end;
    }
    Ok(lines.join("\n"))
}

// Transform only whitespace outside strings, retaining duplicate keys and exact numbers.
fn json_layout(input: &str, pretty: bool, indent: usize) -> String {
    let mut output = String::new();
    let mut quoted = false;
    let mut escaped = false;
    let mut depth: usize = 0;
    let mut chars = input.chars().peekable();
    while let Some(c) = chars.next() {
        if quoted {
            output.push(c);
            if escaped {
                escaped = false;
            } else if c == '\\' {
                escaped = true;
            } else if c == '"' {
                quoted = false;
            }
            continue;
        }
        if c.is_ascii_whitespace() {
            continue;
        }
        if c == '"' {
            quoted = true;
            output.push(c);
            continue;
        }
        match c {
            '{' | '[' if pretty => {
                output.push(c);
                depth += 1;
                while chars.peek().is_some_and(|c| c.is_ascii_whitespace()) {
                    chars.next();
                }
                if !matches!(chars.peek(), Some('}') | Some(']')) {
                    output.push('\n');
                    output.push_str(&" ".repeat(depth * indent));
                }
            }
            '}' | ']' if pretty => {
                depth = depth.saturating_sub(1);
                if !output.ends_with('{') && !output.ends_with('[') {
                    output.push('\n');
                    output.push_str(&" ".repeat(depth * indent));
                }
                output.push(c);
            }
            ',' if pretty => {
                output.push_str(",\n");
                output.push_str(&" ".repeat(depth * indent));
            }
            ':' if pretty => output.push_str(": "),
            _ => output.push(c),
        }
    }
    output
}

fn xml(input: &str, action: Action, indent: usize) -> Result<String, String> {
    let doc = roxmltree::Document::parse(input).map_err(|e| e.to_string())?;
    if action == Action::Lint {
        return Ok(
            "Valid XML. Well-formed document; DTD/schema validation is not performed.".into(),
        );
    }
    // Whitespace can be content in mixed XML or xml:space documents. Preserve it.
    let preserve = doc.descendants().any(|n| {
        n.attribute(("http://www.w3.org/XML/1998/namespace", "space")) == Some("preserve")
            || (n.children().any(|c| c.is_element())
                && n.children()
                    .any(|c| c.is_text() && !c.text().unwrap_or("").trim().is_empty()))
    });
    if preserve {
        return Ok(input.to_owned());
    }
    let mut reader = quick_xml::Reader::from_str(input);
    let mut writer = if action == Action::Format {
        quick_xml::Writer::new_with_indent(Vec::new(), b' ', indent.clamp(1, 8))
    } else {
        quick_xml::Writer::new(Vec::new())
    };
    // Preserve whitespace-only leaf values; remove only formatting between elements.
    let mut element_only = Vec::new();
    for n in doc.descendants().filter(|n| n.is_element()) {
        element_only.push(n.children().any(|c| c.is_element()));
    }
    let mut next = 0;
    let mut stack = Vec::new();
    loop {
        use quick_xml::events::Event;
        let event = reader.read_event().map_err(|e| e.to_string())?;
        match &event {
            Event::Eof => break,
            Event::Start(_) => {
                stack.push(element_only[next]);
                next += 1;
            }
            Event::Empty(_) => {
                next += 1;
            }
            Event::Text(t)
                if t.iter().all(u8::is_ascii_whitespace)
                    && stack.last().copied().unwrap_or(true) =>
            {
                continue
            }
            _ => {}
        }
        writer
            .write_event(event.borrow())
            .map_err(|e| e.to_string())?;
        if matches!(event, Event::End(_)) {
            stack.pop();
        }
    }
    String::from_utf8(writer.into_inner()).map_err(|e| e.to_string())
}

#[derive(Clone, Debug)]
pub struct PasswordOptions {
    pub length: usize,
    pub lower: bool,
    pub upper: bool,
    pub digits: bool,
    pub symbols: bool,
    pub exclude_ambiguous: bool,
}
impl Default for PasswordOptions {
    fn default() -> Self {
        Self {
            length: 24,
            lower: true,
            upper: true,
            digits: true,
            symbols: true,
            exclude_ambiguous: true,
        }
    }
}
/// Generate a bounded batch using the same constraints for every password.
pub fn passwords(options: &PasswordOptions, count: usize) -> Result<Vec<String>, String> {
    if !(1..=1000).contains(&count) {
        return Err("Password quantity must be 1–1000.".into());
    }
    (0..count).map(|_| password(options)).collect()
}

pub fn password(options: &PasswordOptions) -> Result<String, String> {
    if !(8..=128).contains(&options.length) {
        return Err("Password length must be 8–128.".into());
    }
    let groups: Vec<Vec<u8>> = [
        (options.lower, "abcdefghijklmnopqrstuvwxyz"),
        (options.upper, "ABCDEFGHIJKLMNOPQRSTUVWXYZ"),
        (options.digits, "0123456789"),
        (options.symbols, "!@#$%^&*()-_=+[]{}:,.?"),
    ]
    .into_iter()
    .filter(|(enabled, _)| *enabled)
    .map(|(_, chars)| {
        chars
            .bytes()
            .filter(|c| !options.exclude_ambiguous || !b"Il1O0o".contains(c))
            .collect()
    })
    .collect();
    if groups.is_empty() {
        return Err("Select at least one character group.".into());
    }
    let pool: Vec<u8> = groups.iter().flatten().copied().collect();
    let mut rng = OsRng;
    let mut bytes: Vec<u8> = groups
        .iter()
        .map(|g| g[rng.gen_range(0..g.len())])
        .collect();
    while bytes.len() < options.length {
        bytes.push(pool[rng.gen_range(0..pool.len())]);
    }
    bytes.shuffle(&mut rng);
    Ok(String::from_utf8(bytes).expect("ASCII password alphabet"))
}

pub fn qr_png(data: &str, level: qrcode::EcLevel) -> Result<image::GrayImage, String> {
    if data.is_empty() {
        return Err("Enter text or a URL first.".into());
    }
    let code = qrcode::QrCode::with_error_correction_level(data.as_bytes(), level)
        .map_err(|e| e.to_string())?;
    Ok(code
        .render::<image::Luma<u8>>()
        .quiet_zone(true)
        .module_dimensions(8, 8)
        .build())
}
