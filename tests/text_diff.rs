use self_devkits::business::text_diff::{compare, DiffResult, MAX_TEXT};

fn reconstruct(result: &DiffResult, left: bool) -> String {
    let mut output = String::new();
    let mut number = 1;
    for row in &result.rows {
        let line = if left { &row.compare } else { &row.original };
        if let Some(line) = line {
            assert_eq!(line.number, number);
            number += 1;
            output.push_str(&line.text);
            if line.has_newline {
                output.push('\n');
            }
        }
        if !row.changed {
            let (a, b) = (
                row.compare.as_ref().unwrap(),
                row.original.as_ref().unwrap(),
            );
            assert_eq!(a.text, b.text);
            assert_eq!(a.has_newline, b.has_newline);
        }
        assert!(row.compare.is_some() || row.original.is_some());
    }
    output
}

fn check(left: &str, right: &str, added: usize, deleted: usize, unchanged: usize) -> DiffResult {
    let result = compare(left, right).unwrap();
    assert_eq!(
        (result.added, result.deleted, result.unchanged),
        (added, deleted, unchanged),
        "{left:?} against {right:?}"
    );
    assert_eq!(reconstruct(&result, true), left.replace("\r\n", "\n"));
    assert_eq!(reconstruct(&result, false), right.replace("\r\n", "\n"));
    result
}

#[test]
fn empty_identical_insert_delete_replace_and_alignment() {
    assert!(check("", "", 0, 0, 0).rows.is_empty());
    check("a\nb\n", "a\nb\n", 0, 0, 2);
    let added = check("new\n", "", 1, 0, 0);
    assert!(added.rows[0].original.is_none());
    let deleted = check("", "old\n", 0, 1, 0);
    assert!(deleted.rows[0].compare.is_none());
    for (left, right, add, del) in [
        ("new\na\nb\n", "a\nb\n", 1, 0),
        ("a\nnew\nb\n", "a\nb\n", 1, 0),
        ("a\nb\nnew\n", "a\nb\n", 1, 0),
        ("a\nb\n", "old\na\nb\n", 0, 1),
        ("a\nb\n", "a\nold\nb\n", 0, 1),
        ("a\nb\n", "a\nb\nold\n", 0, 1),
    ] {
        check(left, right, add, del, 2);
    }
    let replaced = check("a\nnew\nextra\nz\n", "a\nold\nz\n", 2, 1, 2);
    assert_eq!(replaced.rows.len(), 4);
    assert_eq!(replaced.rows[1].compare.as_ref().unwrap().text, "new");
    assert_eq!(replaced.rows[1].original.as_ref().unwrap().text, "old");
    assert!(replaced.rows[2].original.is_none());
    assert!(!replaced.rows[3].changed);
}

#[test]
fn unicode_whitespace_line_endings_and_final_newline_are_explicit() {
    check(
        "Tiếng Việt 🙂\n日本語\n",
        "Tiếng Việt 🙂\r\n日本語\r\n",
        0,
        0,
        2,
    );
    check(" a\n\tb\n", "a\nb\n", 2, 2, 0);
    check("Hello\n", "hello\n", 1, 1, 0);
    check("a\rb\n", "ab\n", 1, 1, 0);
    check("\n", "", 1, 0, 0);
    check("a\n\n", "a\n", 1, 0, 1);
    let ending = check("a", "a\n", 1, 1, 0);
    assert!(!ending.rows[0].compare.as_ref().unwrap().has_newline);
    assert!(ending.rows[0].original.as_ref().unwrap().has_newline);
    let ending = check("a\r", "a\r\n", 1, 1, 0);
    assert_eq!(ending.rows[0].compare.as_ref().unwrap().text, "a\r");
    check("a\n\nlast", "a\r\n\r\nlast", 0, 0, 3);
}

fn oracle_lcs(a: &[usize], b: &[usize]) -> usize {
    let mut table = vec![vec![0; b.len() + 1]; a.len() + 1];
    for i in 0..a.len() {
        for (j, right) in b.iter().enumerate() {
            table[i + 1][j + 1] = if a[i] == *right {
                table[i][j] + 1
            } else {
                table[i][j + 1].max(table[i + 1][j])
            };
        }
    }
    table[a.len()][b.len()]
}

#[test]
fn stack_lcs_matches_an_independent_oracle_for_repeated_lines() {
    let mut samples = vec![vec![]];
    for len in 1..=4 {
        for mut value in 0..3usize.pow(len) {
            let mut sequence = Vec::new();
            for _ in 0..len {
                sequence.push(value % 3);
                value /= 3;
            }
            samples.push(sequence);
        }
    }
    for a in &samples {
        for b in &samples {
            let text = |s: &[usize]| s.iter().map(|n| format!("line {n}\n")).collect::<String>();
            let (left, right) = (text(a), text(b));
            let lcs = oracle_lcs(a, b);
            check(&left, &right, a.len() - lcs, b.len() - lcs, lcs);
        }
    }
}

#[test]
fn large_similar_and_disjoint_files_take_fast_paths_without_recursion() {
    let same = "same\n".repeat(40_000);
    let result = compare(&same, &same).unwrap();
    assert_eq!(
        (result.added, result.deleted, result.unchanged),
        (0, 0, 40_000)
    );
    let left = (0..10_000)
        .map(|n| format!("new {n}\n"))
        .collect::<String>();
    let right = (0..10_000)
        .map(|n| format!("old {n}\n"))
        .collect::<String>();
    check(&left, &right, 10_000, 10_000, 0);
}

#[test]
fn resource_limits_return_errors_instead_of_partial_results() {
    let at_limit = "x".repeat(MAX_TEXT);
    assert_eq!(compare(&at_limit, &at_limit).unwrap().unchanged, 1);
    let too_large = "x".repeat(MAX_TEXT + 1);
    assert!(compare(&too_large, "").unwrap_err().contains("2 MiB"));
    assert!(compare("", &too_large).unwrap_err().contains("2 MiB"));
    let left = (0..8_000).map(|n| format!("{n}\n")).collect::<String>();
    let right = (0..8_000)
        .rev()
        .map(|n| format!("{n}\n"))
        .collect::<String>();
    assert!(compare(&left, &right).unwrap_err().contains("50 million"));
}
