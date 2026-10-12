use self_devkits::*;
fn run(s: &str, lang: Language, action: Action) -> Result<String, String> {
    process(s, lang, action, 2, SqlDialect::Generic)
}
#[test]
fn json_roundtrip_and_errors() {
    let input = r#"{"z":12345678901234567890123456789,"a":[true,"héllo",null]}"#;
    let pretty = run(input, Language::Json, Action::Format).unwrap();
    assert!(pretty.contains("\n  \"z\""));
    assert_eq!(run(&pretty, Language::Json, Action::Minify).unwrap(), input);
    assert!(run("{\"x\":}", Language::Json, Action::Lint)
        .unwrap_err()
        .contains("line"));
}
#[test]
fn xml_preserves_content_and_rejects_invalid() {
    for input in [
        "<p>Hello <b>world</b> !</p>",
        "<x xml:space=\"preserve\">  <y/> </x>",
    ] {
        assert_eq!(run(input, Language::Xml, Action::Format).unwrap(), input);
        assert_eq!(run(input, Language::Xml, Action::Minify).unwrap(), input);
    }
    assert_eq!(
        run(
            "<root>\n  <a> </a>\n  <b/>\n</root>",
            Language::Xml,
            Action::Minify
        )
        .unwrap(),
        "<root><a> </a><b/></root>"
    );
    for bad in [
        "<a><b></a>",
        "<a/><b/>",
        "<x>&unknown;</x>",
        "<!DOCTYPE x [<!ENTITY a 'x'>]><x>&a;</x>",
    ] {
        assert!(run(bad, Language::Xml, Action::Lint).is_err(), "{bad}");
    }
}
#[test]
fn sql_minify_preserves_literals_and_statements() {
    let input = "SELECT 'it''s  ok', \"a b\" FROM users; -- comment\n SELECT 1 - -2;";
    let output = run(input, Language::Sql, Action::Minify).unwrap();
    assert!(output.contains("'it''s  ok'"));
    assert!(!output.contains("comment"));
    assert!(run(&output, Language::Sql, Action::Lint).is_ok());
    assert!(run("SELECT FROM", Language::Sql, Action::Lint).is_err());
}
#[test]
fn password_constraints() {
    for _ in 0..100 {
        let p = password(&PasswordOptions::default()).unwrap();
        assert_eq!(p.len(), 24);
        assert!(p.bytes().any(|b| b.is_ascii_lowercase()));
        assert!(p.bytes().any(|b| b.is_ascii_uppercase()));
        assert!(p.bytes().any(|b| b.is_ascii_digit()));
        assert!(p.bytes().any(|b| !b.is_ascii_alphanumeric()));
        assert!(!p.bytes().any(|b| b"Il1O0o".contains(&b)));
    }
    assert!(password(&PasswordOptions {
        lower: false,
        upper: false,
        digits: false,
        symbols: false,
        ..Default::default()
    })
    .is_err());
}
#[test]
fn qr_roundtrip_unicode() {
    let text = "Xin chào! https://example.com/?a=1&b=2";
    let img = qr_png(text, qrcode::EcLevel::M).unwrap();
    let mut prepared = rqrr::PreparedImage::prepare(img);
    let grids = prepared.detect_grids();
    assert_eq!(grids.len(), 1);
    assert_eq!(grids[0].decode().unwrap().1, text);
    assert!(qr_png("", qrcode::EcLevel::M).is_err());
    assert!(qr_png(&"a".repeat(5000), qrcode::EcLevel::H).is_err());
}
#[test]
fn rejects_empty_and_oversized() {
    assert!(run("", Language::Sql, Action::Lint).is_err());
    assert!(run(&" ".repeat(MAX_INPUT + 1), Language::Json, Action::Format).is_err());
}

#[test]
fn json_keeps_duplicate_keys_and_escapes() {
    let input = r#"{"x":1,"x":2,"escaped":"a\\b\"c","empty":{},"array":[]}"#;
    let pretty = run(input, Language::Json, Action::Format).unwrap();
    assert_eq!(run(&pretty, Language::Json, Action::Minify).unwrap(), input);
}
#[test]
fn sql_dialects_roundtrip() {
    for (dialect, input) in [
        (
            SqlDialect::PostgreSql,
            "SELECT $$hello  world$$, 'it''s ok', x::text FROM users WHERE x >= 1;",
        ),
        (
            SqlDialect::MySql,
            "SELECT `a b` FROM `table` WHERE id != 4 LIMIT 10;",
        ),
        (
            SqlDialect::Sqlite,
            "SELECT [name] FROM [users] WHERE id IN (1, 2);",
        ),
    ] {
        for action in [Action::Format, Action::Minify] {
            process(input, Language::Sql, action, 2, dialect)
                .unwrap_or_else(|e| panic!("{dialect:?} {action:?}: {e}"));
        }
    }
}

#[test]
fn sql_toolbar_format_options() {
    let input = "select a as first_alias, longer_column as second_alias from table1 where value = 'do not change AS text'";
    let output = process_with_sql_options(
        input,
        Language::Sql,
        Action::Format,
        2,
        SqlDialect::MySql,
        SqlFormatOptions {
            keyword_uppercase: true,
            alias_align: true,
        },
    )
    .unwrap();
    assert!(output.starts_with("SELECT"));
    assert!(output.contains("'do not change AS text'"));
    let aliases: Vec<_> = output
        .lines()
        .filter(|l| l.contains(" AS ") && l.contains("_alias"))
        .map(|l| l.find(" AS ").unwrap())
        .collect();
    assert_eq!(aliases.len(), 2);
    assert_eq!(aliases[0], aliases[1]);
    let unchanged = process_with_sql_options(
        input,
        Language::Sql,
        Action::Format,
        2,
        SqlDialect::MySql,
        SqlFormatOptions::default(),
    )
    .unwrap();
    assert!(unchanged.starts_with("select"));
}

#[test]
fn sql_uppercase_keeps_function_names_and_literals() {
    let input =
        "select avg(amount) as average, custom_fn(id) as result from data where id in (1, 2)";
    let output = process_with_sql_options(
        input,
        Language::Sql,
        Action::Format,
        2,
        SqlDialect::PostgreSql,
        SqlFormatOptions {
            keyword_uppercase: true,
            alias_align: true,
        },
    )
    .unwrap();
    assert!(output.starts_with("SELECT"));
    assert!(output.contains("avg(amount)"));
    assert!(output.contains("custom_fn(id)"));
    assert!(output.contains(" IN "));
}

#[test]
fn password_batch_limits_and_constraints() {
    let options = PasswordOptions {
        length: 32,
        ..Default::default()
    };
    for count in [1, 20, 1000] {
        let batch = passwords(&options, count).unwrap();
        assert_eq!(batch.len(), count);
        for value in batch {
            assert_eq!(value.len(), 32);
            assert!(value.bytes().any(|b| b.is_ascii_lowercase()));
            assert!(value.bytes().any(|b| b.is_ascii_uppercase()));
            assert!(value.bytes().any(|b| b.is_ascii_digit()));
            assert!(value.bytes().any(|b| !b.is_ascii_alphanumeric()));
            assert!(!value.bytes().any(|b| b"Il1O0o".contains(&b)));
        }
    }
    assert!(passwords(&options, 0).is_err());
    assert!(passwords(&options, 1001).is_err());
    assert!(passwords(
        &PasswordOptions {
            length: 0,
            ..options
        },
        5
    )
    .is_err());
}
