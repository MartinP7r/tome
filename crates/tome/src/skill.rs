//! SKILL.md frontmatter parsing.
//!
//! Extracts and parses YAML frontmatter from SKILL.md files.

use serde::Deserialize;
use std::collections::BTreeMap;

/// Parsed SKILL.md frontmatter fields.
#[derive(Debug, Clone, Deserialize, Default)]
#[serde(rename_all = "kebab-case")]
#[allow(dead_code)]
pub struct SkillFrontmatter {
    pub name: Option<String>,
    pub description: Option<String>,
    pub license: Option<String>,
    pub compatibility: Option<String>,
    pub metadata: Option<BTreeMap<String, serde_yaml::Value>>,
    pub allowed_tools: Option<String>,
    // Claude Code extensions
    pub user_invocable: Option<bool>,
    pub argument_hint: Option<String>,
    pub context: Option<String>,
    pub agent: Option<String>,
    // Capture unknown/non-standard fields
    #[serde(flatten)]
    pub extra: BTreeMap<String, serde_yaml::Value>,
}

/// Extract frontmatter YAML block from SKILL.md content.
/// Returns (yaml_content, body) or None if no valid frontmatter delimiters.
pub fn extract_frontmatter(content: &str) -> Option<(&str, &str)> {
    let content = content.trim_start();
    if !content.starts_with("---") {
        return None;
    }
    let after_first = &content[3..];
    // Skip the newline after opening ---
    let after_first = after_first.strip_prefix('\n').unwrap_or(after_first);

    // Handle empty frontmatter: closing --- immediately follows opening
    if let Some(rest) = after_first.strip_prefix("---") {
        let body = rest.strip_prefix('\n').unwrap_or(rest);
        return Some(("", body));
    }

    let end = after_first.find("\n---")?;
    let yaml = &after_first[..end];
    let body = &after_first[end + 4..];
    // Strip optional newline after closing ---
    let body = body.strip_prefix('\n').unwrap_or(body);
    Some((yaml.trim(), body))
}

/// Parse SKILL.md content into frontmatter + body.
pub fn parse(content: &str) -> Result<(SkillFrontmatter, String), String> {
    match extract_frontmatter(content) {
        Some((yaml, body)) => {
            let fm: SkillFrontmatter =
                serde_yaml::from_str(yaml).map_err(|e| format!("invalid YAML frontmatter: {e}"))?;
            Ok((fm, body.to_string()))
        }
        None => Err("no frontmatter found (expected --- delimiters)".to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_valid_frontmatter() {
        let content = "---\nname: my-skill\ndescription: A test skill\n---\n# Body";
        let (fm, body) = parse(content).unwrap();
        assert_eq!(fm.name.as_deref(), Some("my-skill"));
        assert_eq!(fm.description.as_deref(), Some("A test skill"));
        assert_eq!(body, "# Body");
    }

    #[test]
    fn parse_with_extra_fields() {
        let content = "---\nname: test\nversion: 1.0\ncategory: tools\n---\nbody";
        let (fm, _) = parse(content).unwrap();
        assert!(fm.extra.contains_key("version"));
        assert!(fm.extra.contains_key("category"));
    }

    #[test]
    fn parse_missing_frontmatter() {
        let content = "# Just a heading\nNo frontmatter here";
        assert!(parse(content).is_err());
    }

    #[test]
    fn parse_malformed_yaml() {
        let content = "---\n: invalid yaml [[\n---\nbody";
        assert!(parse(content).is_err());
    }

    #[test]
    fn parse_empty_frontmatter() {
        let content = "---\n---\nbody";
        let (fm, body) = parse(content).unwrap();
        assert!(fm.name.is_none());
        assert_eq!(body, "body");
    }

    #[test]
    fn parse_claude_code_extensions() {
        let content = "---\nname: test\nuser-invocable: false\ncontext: fork\n---\nbody";
        let (fm, _) = parse(content).unwrap();
        assert_eq!(fm.user_invocable, Some(false));
        assert_eq!(fm.context.as_deref(), Some("fork"));
    }

    #[test]
    fn extract_no_frontmatter_returns_none() {
        assert!(extract_frontmatter("just text").is_none());
    }

    #[test]
    fn extract_unclosed_frontmatter_returns_none() {
        assert!(extract_frontmatter("---\nname: test\n").is_none());
    }
}
