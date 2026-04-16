use nucleo_matcher::pattern::{Atom, AtomKind, CaseMatching, Normalization};
use nucleo_matcher::{Config, Matcher, Utf32Str};

use super::app::SkillRow;

/// A fuzzy match result with the row index and matched character positions.
#[derive(Debug)]
pub struct FuzzyMatch {
    /// Index into the original `rows` slice.
    pub row_index: usize,
    /// Character positions in the skill *name* that matched the query.
    pub name_indices: Vec<u32>,
}

/// Filter rows by fuzzy matching against `"{name} {source}"`.
///
/// Returns indices into `rows` sorted by match score (highest first).
/// An empty query returns all indices in original order.
#[cfg(test)]
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
            let haystack = format!("{} {}", row.name, row.source);
            let haystack_utf32 = Utf32Str::new(&haystack, &mut buf);
            pattern
                .score(haystack_utf32, &mut matcher)
                .map(|score| (i, score))
        })
        .collect();

    scored.sort_by_key(|s| std::cmp::Reverse(s.1));
    scored.into_iter().map(|(i, _)| i).collect()
}

/// Filter rows and return match indices for highlighting.
///
/// Matches against the skill name only (not composite "{name} {source}") to
/// ensure indices map directly to character positions in the name string.
/// This avoids the off-by-one pitfall when indices span across a composite haystack.
pub fn filter_rows_with_indices(query: &str, rows: &[SkillRow]) -> Vec<FuzzyMatch> {
    if query.is_empty() {
        return rows
            .iter()
            .enumerate()
            .map(|(i, _)| FuzzyMatch {
                row_index: i,
                name_indices: Vec::new(),
            })
            .collect();
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
    let mut indices_buf = Vec::new();

    let mut scored: Vec<(FuzzyMatch, u16)> = rows
        .iter()
        .enumerate()
        .filter_map(|(i, row)| {
            // Score against composite for filtering (same as filter_rows)
            let composite = format!("{} {}", row.name, row.source);
            let composite_utf32 = Utf32Str::new(&composite, &mut buf);
            let score = pattern.score(composite_utf32, &mut matcher)?;

            // Get indices against name only for highlighting
            indices_buf.clear();
            let name_utf32 = Utf32Str::new(&row.name, &mut buf);
            pattern.indices(name_utf32, &mut matcher, &mut indices_buf);
            indices_buf.sort_unstable();
            indices_buf.dedup();

            Some((
                FuzzyMatch {
                    row_index: i,
                    name_indices: indices_buf.clone(),
                },
                score,
            ))
        })
        .collect();

    scored.sort_by_key(|s| std::cmp::Reverse(s.1));
    scored.into_iter().map(|(m, _)| m).collect()
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
                managed: false,
                synced_at: String::new(),
            },
            SkillRow {
                name: "git-commit".into(),
                source: "claude-skills".into(),
                path: "~/.claude/skills/git-commit".into(),
                managed: false,
                synced_at: String::new(),
            },
            SkillRow {
                name: "rust-clippy".into(),
                source: "agents-skills".into(),
                path: "~/.agents/skills/rust-clippy".into(),
                managed: false,
                synced_at: String::new(),
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

    #[test]
    fn filter_with_indices_returns_positions() {
        let rows = vec![SkillRow {
            name: "a-b-c-skill".into(),
            source: "test".into(),
            path: "/test".into(),
            managed: false,
            synced_at: String::new(),
        }];
        let results = filter_rows_with_indices("abc", &rows);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].row_index, 0);
        // Indices should point to 'a', 'b', 'c' positions in the name
        assert!(!results[0].name_indices.is_empty());
        // 'a' is at position 0, 'b' at 2, 'c' at 4
        assert!(results[0].name_indices.contains(&0));
        assert!(results[0].name_indices.contains(&2));
        assert!(results[0].name_indices.contains(&4));
    }

    #[test]
    fn filter_with_indices_empty_query() {
        let rows = make_rows();
        let results = filter_rows_with_indices("", &rows);
        assert_eq!(results.len(), 3);
        // All name_indices should be empty for empty query
        for m in &results {
            assert!(m.name_indices.is_empty());
        }
    }
}
