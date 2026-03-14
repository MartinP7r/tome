use nucleo_matcher::pattern::{Atom, AtomKind, CaseMatching, Normalization};
use nucleo_matcher::{Config, Matcher, Utf32Str};

use super::app::SkillRow;

/// Filter rows by fuzzy matching against `"{name} {source} {path}"`.
///
/// Returns indices into `rows` sorted by match score (highest first).
/// An empty query returns all indices in original order.
pub fn filter_rows(query: &str, rows: &[SkillRow]) -> Vec<usize> {
    if query.is_empty() {
        return (0..rows.len()).collect();
    }

    let pattern = Atom::new(
        query,
        CaseMatching::Smart,
        Normalization::Smart,
        AtomKind::Fuzzy,
        false,
    );
    let mut matcher = Matcher::new(Config::DEFAULT);
    let mut buf = Vec::new();

    let mut scored: Vec<(usize, u16)> = rows
        .iter()
        .enumerate()
        .filter_map(|(i, row)| {
            let haystack = format!("{} {} {}", row.name, row.source, row.path);
            let haystack_utf32 = Utf32Str::new(&haystack, &mut buf);
            pattern
                .score(haystack_utf32, &mut matcher)
                .map(|score| (i, score))
        })
        .collect();

    scored.sort_by(|a, b| b.1.cmp(&a.1));
    scored.into_iter().map(|(i, _)| i).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_rows() -> Vec<SkillRow> {
        vec![
            SkillRow {
                name: "pdf-extract".into(),
                source: "claude-plugins".into(),
                path: "~/.claude/plugins/pdf-extract".into(),
            },
            SkillRow {
                name: "git-commit".into(),
                source: "claude-skills".into(),
                path: "~/.claude/skills/git-commit".into(),
            },
            SkillRow {
                name: "rust-clippy".into(),
                source: "agents-skills".into(),
                path: "~/.agents/skills/rust-clippy".into(),
            },
        ]
    }

    #[test]
    fn empty_query_returns_all() {
        let rows = make_rows();
        let result = filter_rows("", &rows);
        assert_eq!(result, vec![0, 1, 2]);
    }

    #[test]
    fn exact_match_ranks_first() {
        let rows = make_rows();
        let result = filter_rows("git-commit", &rows);
        assert!(!result.is_empty());
        assert_eq!(result[0], 1);
    }

    #[test]
    fn no_match_returns_empty() {
        let rows = make_rows();
        let result = filter_rows("zzzzzzzzz", &rows);
        assert!(result.is_empty());
    }

    #[test]
    fn fuzzy_partial_match() {
        let rows = make_rows();
        let result = filter_rows("clip", &rows);
        assert!(!result.is_empty());
        assert!(result.contains(&2));
    }
}
