//! Shared validation logic for identifier types (skill names, target names).

use anyhow::Result;

/// Validate an identifier string used for skill or target names.
///
/// Rejects empty names, `.`/`..`, whitespace-only or leading/trailing whitespace,
/// and names containing path separators (`/` or `\`).
///
/// The `kind` parameter is used in error messages (e.g. "skill name", "target name").
pub(crate) fn validate_identifier(name: &str, kind: &str) -> Result<()> {
    anyhow::ensure!(!name.is_empty(), "{kind} cannot be empty");
    anyhow::ensure!(name != "." && name != "..", "{kind} cannot be '.' or '..'");
    anyhow::ensure!(
        !name.chars().all(|c| c.is_whitespace()) && name.trim() == name,
        "{kind} cannot be whitespace-only or have leading/trailing whitespace: '{name}'"
    );
    anyhow::ensure!(
        !name.contains('/') && !name.contains('\\'),
        "{kind} contains path separator: '{name}'"
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_empty() {
        assert!(validate_identifier("", "test name").is_err());
    }

    #[test]
    fn rejects_dots() {
        assert!(validate_identifier(".", "test name").is_err());
        assert!(validate_identifier("..", "test name").is_err());
    }

    #[test]
    fn rejects_whitespace() {
        assert!(validate_identifier("  ", "test name").is_err());
        assert!(validate_identifier(" leading", "test name").is_err());
        assert!(validate_identifier("trailing ", "test name").is_err());
    }

    #[test]
    fn rejects_path_separators() {
        assert!(validate_identifier("foo/bar", "test name").is_err());
        assert!(validate_identifier("foo\\bar", "test name").is_err());
    }

    #[test]
    fn accepts_valid() {
        assert!(validate_identifier("my-skill-123", "test name").is_ok());
        assert!(validate_identifier("My_Skill", "test name").is_ok());
    }
}
