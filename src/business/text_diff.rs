//! Exact line comparison driven by an explicit stack, without recursive calls.
use std::collections::{HashMap, HashSet};
use std::ops::Range;

pub const MAX_TEXT: usize = 2 * 1024 * 1024;
pub const MAX_COMPARISONS: usize = 50_000_000;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DiffLine {
    /// One-based line number in this side's source.
    pub number: usize,
    /// Line content without its LF or CRLF terminator. A lone CR is retained.
    pub text: String,
    pub has_newline: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DiffRow {
    /// The new/compared text, displayed on the left.
    pub compare: Option<DiffLine>,
    /// The original text, displayed on the right.
    pub original: Option<DiffLine>,
    /// Changed rows color compare lines green and original lines red.
    pub changed: bool,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct DiffResult {
    pub rows: Vec<DiffRow>,
    pub added: usize,
    pub deleted: usize,
    pub unchanged: usize,
}

#[derive(Clone, Copy, Hash, PartialEq, Eq)]
struct Line<'a> {
    text: &'a str,
    has_newline: bool,
}

fn lines(text: &str) -> Vec<Line<'_>> {
    text.split_inclusive('\n')
        .map(|line| {
            let has_newline = line.ends_with('\n');
            let text = if has_newline {
                let content = &line[..line.len() - 1];
                content.strip_suffix('\r').unwrap_or(content)
            } else {
                line
            };
            Line { text, has_newline }
        })
        .collect()
}

enum Task {
    Solve(Range<usize>, Range<usize>),
    Equal {
        compare: usize,
        original: usize,
        len: usize,
    },
}

fn line_ids<'a>(lines: &[Line<'a>], interned: &mut HashMap<Line<'a>, usize>) -> Vec<usize> {
    lines
        .iter()
        .map(|line| {
            let next = interned.len();
            *interned.entry(*line).or_insert(next)
        })
        .collect()
}

// A single DP row suffices for each half of a Hirschberg split.
fn lcs_scores(compare: &[usize], original: &[usize], reversed: bool) -> Vec<usize> {
    let mut scores = vec![0; original.len() + 1];
    for a in 0..compare.len() {
        let a = if reversed { compare.len() - 1 - a } else { a };
        let mut diagonal = 0;
        for b in 0..original.len() {
            let source_b = if reversed { original.len() - 1 - b } else { b };
            let previous = scores[b + 1];
            scores[b + 1] = if compare[a] == original[source_b] {
                diagonal + 1
            } else {
                scores[b].max(previous)
            };
            diagonal = previous;
        }
    }
    scores
}

fn have_common_lines(compare: &[usize], original: &[usize]) -> bool {
    let (shorter, longer) = if compare.len() <= original.len() {
        (compare, original)
    } else {
        (original, compare)
    };
    let ids: HashSet<_> = shorter.iter().copied().collect();
    longer.iter().any(|id| ids.contains(id))
}

/// Compare the left/new text against the right/original text.
///
/// Iterative Hirschberg LCS uses a LIFO `Task` stack to solve line ranges and emit
/// matches in order. Its working memory is linear in the input line counts;
/// unmatched ranges become aligned additions/deletions rather than shifting all
/// subsequent lines. LF and CRLF are equivalent, but a final newline is significant.
pub fn compare(compare: &str, original: &str) -> Result<DiffResult, String> {
    if compare.len() > MAX_TEXT || original.len() > MAX_TEXT {
        return Err("Each text must be at most 2 MiB.".into());
    }
    let compare = lines(compare);
    let original = lines(original);
    let mut interned = HashMap::new();
    let compare_ids = line_ids(&compare, &mut interned);
    let original_ids = line_ids(&original, &mut interned);
    let mut tasks = vec![Task::Solve(0..compare.len(), 0..original.len())];
    let mut matches = Vec::new();
    let mut budget = MAX_COMPARISONS;
    while let Some(task) = tasks.pop() {
        match task {
            Task::Equal {
                compare,
                original,
                len,
            } => {
                matches.extend((0..len).map(|offset| (compare + offset, original + offset)));
            }
            Task::Solve(mut a, mut b) => {
                while !a.is_empty()
                    && !b.is_empty()
                    && compare_ids[a.start] == original_ids[b.start]
                {
                    matches.push((a.start, b.start));
                    a.start += 1;
                    b.start += 1;
                }
                let mut suffix = 0;
                while !a.is_empty()
                    && !b.is_empty()
                    && compare_ids[a.end - 1] == original_ids[b.end - 1]
                {
                    a.end -= 1;
                    b.end -= 1;
                    suffix += 1;
                }
                if suffix > 0 {
                    // LIFO ordering emits the suffix after both smaller ranges.
                    tasks.push(Task::Equal {
                        compare: a.end,
                        original: b.end,
                        len: suffix,
                    });
                }
                if a.is_empty() || b.is_empty() {
                    continue;
                }
                if a.len() == 1 {
                    if let Some(offset) = original_ids[b.clone()]
                        .iter()
                        .position(|id| *id == compare_ids[a.start])
                    {
                        matches.push((a.start, b.start + offset));
                    }
                    continue;
                }
                if b.len() == 1 {
                    if let Some(offset) = compare_ids[a.clone()]
                        .iter()
                        .position(|id| *id == original_ids[b.start])
                    {
                        matches.push((a.start + offset, b.start));
                    }
                    continue;
                }
                if !have_common_lines(&compare_ids[a.clone()], &original_ids[b.clone()]) {
                    continue;
                }
                let cost = a
                    .len()
                    .checked_mul(b.len())
                    .ok_or("Comparison is too complex. Try smaller sections.")?;
                budget = budget.checked_sub(cost).ok_or(
                    "Comparison exceeds 50 million line comparisons. Try smaller sections.",
                )?;
                let middle = a.start + a.len() / 2;
                let forward = lcs_scores(
                    &compare_ids[a.start..middle],
                    &original_ids[b.clone()],
                    false,
                );
                let backward =
                    lcs_scores(&compare_ids[middle..a.end], &original_ids[b.clone()], true);
                let mut best = 0;
                let mut split = 0;
                for offset in 0..=b.len() {
                    let score = forward[offset] + backward[b.len() - offset];
                    if score > best {
                        best = score;
                        split = offset;
                    }
                }
                let split = b.start + split;
                tasks.push(Task::Solve(middle..a.end, split..b.end));
                tasks.push(Task::Solve(a.start..middle, b.start..split));
            }
        }
    }
    let source_line = |source: &[Line<'_>], index: usize| DiffLine {
        number: index + 1,
        text: source[index].text.to_owned(),
        has_newline: source[index].has_newline,
    };
    let mut result = DiffResult::default();
    let (mut a, mut b) = (0, 0);
    // The sentinel flushes the last changed block, including entirely different texts.
    for (next_a, next_b) in matches.into_iter().chain([(compare.len(), original.len())]) {
        result.added += next_a - a;
        result.deleted += next_b - b;
        for offset in 0..(next_a - a).max(next_b - b) {
            result.rows.push(DiffRow {
                compare: (a + offset < next_a).then(|| source_line(&compare, a + offset)),
                original: (b + offset < next_b).then(|| source_line(&original, b + offset)),
                changed: true,
            });
        }
        if next_a < compare.len() {
            result.rows.push(DiffRow {
                compare: Some(source_line(&compare, next_a)),
                original: Some(source_line(&original, next_b)),
                changed: false,
            });
            result.unchanged += 1;
        }
        a = next_a + 1;
        b = next_b + 1;
    }
    Ok(result)
}
