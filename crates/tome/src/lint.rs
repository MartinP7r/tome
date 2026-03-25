//! Skill validation and linting.
//!
//! Implements tiered validation (error/warning/info) for SKILL.md frontmatter
//! based on the agentskills.io standard and platform compatibility requirements.

use console::style;
use std::path::Path;

use crate::skill;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Severity {
    Error,
    Warning,
    Info,
}

#[derive(Debug, Clone)]
pub struct LintIssue {
    pub severity: Severity,
    pub message: String,
}

pub struct LintReport {
    pub results: Vec<(String, Vec<LintIssue>)>, // (skill_name, issues)
    pub skills_checked: usize,
}

impl LintReport {
    pub fn error_count(&self) -> usize {
        self.results
            .iter()
            .flat_map(|(_, issues)| issues)
            .filter(|i| i.severity == Severity::Error)
            .count()
    }
    pub fn warning_count(&self) -> usize {
        self.results
            .iter()
            .flat_map(|(_, issues)| issues)
            .filter(|i| i.severity == Severity::Warning)
            .count()
    }
    pub fn info_count(&self) -> usize {
        self.results
            .iter()
            .flat_map(|(_, issues)| issues)
            .filter(|i| i.severity == Severity::Info)
            .count()
    }
    pub fn has_errors(&self) -> bool {
        self.error_count() > 0
    }
}

/// Lint a single skill directory.
pub fn lint_skill(dir_name: &str, skill_dir: &Path) -> Vec<LintIssue> {
    let mut issues = Vec::new();
    let skill_md = skill_dir.join("SKILL.md");

    let content = match std::fs::read_to_string(&skill_md) {
        Ok(c) => c,
        Err(e) => {
            issues.push(LintIssue {
                severity: Severity::Error,
                message: format!("could not read SKILL.md: {e}"),
            });
            return issues;
        }
    };

    let (fm, body) = match skill::parse(&content) {
        Ok(result) => result,
        Err(e) => {
            issues.push(LintIssue {
                severity: Severity::Error,
                message: e,
            });
            return issues;
        }
    };

    // --- Errors ---

    // name present but doesn't match directory
    if let Some(ref name) = fm.name {
        if name != dir_name {
            issues.push(LintIssue {
                severity: Severity::Error,
                message: format!(
                    "name '{}' does not match directory name '{}'",
                    name, dir_name
                ),
            });
        }
        if name.len() > 64 {
            issues.push(LintIssue {
                severity: Severity::Error,
                message: format!("name exceeds 64 characters ({} chars)", name.len()),
            });
        }
        if !name
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
        {
            issues.push(LintIssue {
                severity: Severity::Error,
                message: format!(
                    "name '{}' uses invalid characters (expected [a-z0-9-])",
                    name
                ),
            });
        }
    }

    // Missing description
    if fm.description.is_none() {
        issues.push(LintIssue {
            severity: Severity::Error,
            message: "missing required 'description' field".to_string(),
        });
    }

    // --- Warnings ---

    // Missing name (not portable)
    if fm.name.is_none() {
        issues.push(LintIssue {
            severity: Severity::Warning,
            message:
                "missing 'name' field -- Claude Code infers from directory, but other tools may not"
                    .to_string(),
        });
    }

    // Description length warnings
    if let Some(ref desc) = fm.description {
        if desc.is_empty() {
            issues.push(LintIssue {
                severity: Severity::Info,
                message: "description is empty".to_string(),
            });
        } else if desc.len() > 1024 {
            issues.push(LintIssue {
                severity: Severity::Warning,
                message: format!("description exceeds 1024 characters ({} chars)", desc.len()),
            });
        } else if desc.len() > 500 {
            issues.push(LintIssue {
                severity: Severity::Warning,
                message: format!(
                    "description exceeds 500 characters ({} chars) -- may be truncated by VS Code Copilot",
                    desc.len()
                ),
            });
        }
    }

    // Non-standard fields
    let known_non_standard = ["version", "category", "tags", "last-updated", "author"];
    for key in fm.extra.keys() {
        if known_non_standard.contains(&key.as_str()) {
            issues.push(LintIssue {
                severity: Severity::Warning,
                message: format!(
                    "non-standard field '{}' -- consider moving to 'metadata' section",
                    key
                ),
            });
        }
    }

    // Body length
    if body.len() > 6000 {
        issues.push(LintIssue {
            severity: Severity::Warning,
            message: format!(
                "body exceeds 6000 characters ({} chars) -- may be truncated by Windsurf",
                body.len()
            ),
        });
    }

    // Unicode Tag codepoints (U+E0001-U+E007F) -- security risk
    if content
        .chars()
        .any(|c| ('\u{E0001}'..='\u{E007F}').contains(&c))
    {
        issues.push(LintIssue {
            severity: Severity::Warning,
            message: "contains hidden Unicode Tag codepoints (U+E0001-U+E007F) -- potential prompt injection risk".to_string(),
        });
    }

    // --- Info ---

    if fm.allowed_tools.is_some() {
        issues.push(LintIssue {
            severity: Severity::Info,
            message: "'allowed-tools' field is experimental".to_string(),
        });
    }

    issues
}

/// Lint all skills in a library directory.
pub fn lint_library(library_dir: &Path) -> LintReport {
    let mut results = Vec::new();
    let mut skills_checked = 0;

    if let Ok(entries) = std::fs::read_dir(library_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if !path.is_dir() && !path.is_symlink() {
                continue;
            }
            // For symlinks (managed skills), resolve to check if directory
            if path.is_symlink() {
                match std::fs::metadata(&path) {
                    Ok(m) if m.is_dir() => {}
                    _ => continue,
                }
            }

            let dir_name = match path.file_name().and_then(|n| n.to_str()) {
                Some(n) => n.to_string(),
                None => continue,
            };

            // Skip non-skill entries (manifest, gitignore, etc.)
            if dir_name.starts_with('.') {
                continue;
            }

            let skill_md = path.join("SKILL.md");
            if !skill_md.exists() {
                if let Ok(true) = skill_md.try_exists() {
                    // exists through symlink, proceed
                } else {
                    continue;
                }
            }

            let issues = lint_skill(&dir_name, &path);
            skills_checked += 1;
            results.push((dir_name, issues));
        }
    }

    results.sort_by(|a, b| a.0.cmp(&b.0));
    LintReport {
        results,
        skills_checked,
    }
}

/// Render the lint report to stdout.
pub fn render_text(report: &LintReport) {
    for (name, issues) in &report.results {
        if issues.is_empty() {
            continue; // Skip clean skills in text output
        }
        println!("{}:", style(name).bold());
        for issue in issues {
            let (icon, styled_msg) = match issue.severity {
                Severity::Error => (style("x").red(), style(&issue.message).red()),
                Severity::Warning => (style("!").yellow(), style(&issue.message).yellow()),
                Severity::Info => (style("i").cyan(), style(&issue.message).cyan()),
            };
            println!("  {} {}", icon, styled_msg);
        }
        println!();
    }

    let errors = report.error_count();
    let warnings = report.warning_count();
    let info = report.info_count();
    println!(
        "Checked {} skill(s): {} error(s), {} warning(s), {} info",
        report.skills_checked, errors, warnings, info
    );
}

/// Render the lint report as JSON.
pub fn render_json(report: &LintReport) {
    let issues: Vec<serde_json::Value> = report
        .results
        .iter()
        .flat_map(|(name, issues)| {
            issues.iter().map(move |issue| {
                serde_json::json!({
                    "skill": name,
                    "severity": match issue.severity {
                        Severity::Error => "error",
                        Severity::Warning => "warning",
                        Severity::Info => "info",
                    },
                    "message": issue.message,
                })
            })
        })
        .collect();

    let output = serde_json::json!({
        "skills_checked": report.skills_checked,
        "errors": report.error_count(),
        "warnings": report.warning_count(),
        "info": report.info_count(),
        "issues": issues,
    });

    println!("{}", serde_json::to_string_pretty(&output).unwrap());
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn create_skill_dir(dir: &Path, name: &str, content: &str) {
        let skill_dir = dir.join(name);
        std::fs::create_dir_all(&skill_dir).unwrap();
        std::fs::write(skill_dir.join("SKILL.md"), content).unwrap();
    }

    #[test]
    fn lint_missing_frontmatter() {
        let tmp = TempDir::new().unwrap();
        create_skill_dir(tmp.path(), "my-skill", "# No frontmatter here");
        let issues = lint_skill("my-skill", &tmp.path().join("my-skill"));
        assert!(
            issues
                .iter()
                .any(|i| i.severity == Severity::Error && i.message.contains("no frontmatter"))
        );
    }

    #[test]
    fn lint_invalid_yaml() {
        let tmp = TempDir::new().unwrap();
        create_skill_dir(tmp.path(), "my-skill", "---\n: invalid [[\n---\nbody");
        let issues = lint_skill("my-skill", &tmp.path().join("my-skill"));
        assert!(
            issues
                .iter()
                .any(|i| i.severity == Severity::Error && i.message.contains("invalid YAML"))
        );
    }

    #[test]
    fn lint_name_mismatch() {
        let tmp = TempDir::new().unwrap();
        create_skill_dir(
            tmp.path(),
            "my-skill",
            "---\nname: wrong-name\ndescription: test\n---\nbody",
        );
        let issues = lint_skill("my-skill", &tmp.path().join("my-skill"));
        assert!(
            issues
                .iter()
                .any(|i| i.severity == Severity::Error && i.message.contains("does not match"))
        );
    }

    #[test]
    fn lint_name_too_long() {
        let tmp = TempDir::new().unwrap();
        let long_name = "a".repeat(65);
        let dir_name = long_name.clone();
        create_skill_dir(
            tmp.path(),
            &dir_name,
            &format!("---\nname: {}\ndescription: test\n---\nbody", long_name),
        );
        let issues = lint_skill(&dir_name, &tmp.path().join(&dir_name));
        assert!(
            issues
                .iter()
                .any(|i| i.severity == Severity::Error && i.message.contains("exceeds 64"))
        );
    }

    #[test]
    fn lint_name_invalid_chars() {
        let tmp = TempDir::new().unwrap();
        create_skill_dir(
            tmp.path(),
            "My_Skill",
            "---\nname: My_Skill\ndescription: test\n---\nbody",
        );
        let issues = lint_skill("My_Skill", &tmp.path().join("My_Skill"));
        assert!(
            issues
                .iter()
                .any(|i| i.severity == Severity::Error && i.message.contains("invalid characters"))
        );
    }

    #[test]
    fn lint_missing_description() {
        let tmp = TempDir::new().unwrap();
        create_skill_dir(tmp.path(), "my-skill", "---\nname: my-skill\n---\nbody");
        let issues = lint_skill("my-skill", &tmp.path().join("my-skill"));
        assert!(
            issues
                .iter()
                .any(|i| i.severity == Severity::Error && i.message.contains("missing required"))
        );
    }

    #[test]
    fn lint_missing_name_warning() {
        let tmp = TempDir::new().unwrap();
        create_skill_dir(
            tmp.path(),
            "my-skill",
            "---\ndescription: A test skill\n---\nbody",
        );
        let issues = lint_skill("my-skill", &tmp.path().join("my-skill"));
        assert!(
            issues
                .iter()
                .any(|i| i.severity == Severity::Warning && i.message.contains("missing 'name'"))
        );
    }

    #[test]
    fn lint_description_too_long() {
        let tmp = TempDir::new().unwrap();
        let long_desc = "x".repeat(1025);
        create_skill_dir(
            tmp.path(),
            "my-skill",
            &format!(
                "---\nname: my-skill\ndescription: \"{}\"\n---\nbody",
                long_desc
            ),
        );
        let issues = lint_skill("my-skill", &tmp.path().join("my-skill"));
        assert!(
            issues
                .iter()
                .any(|i| i.severity == Severity::Warning && i.message.contains("exceeds 1024"))
        );
    }

    #[test]
    fn lint_non_standard_fields() {
        let tmp = TempDir::new().unwrap();
        create_skill_dir(
            tmp.path(),
            "my-skill",
            "---\nname: my-skill\ndescription: test\nversion: 1.0\n---\nbody",
        );
        let issues = lint_skill("my-skill", &tmp.path().join("my-skill"));
        assert!(
            issues.iter().any(
                |i| i.severity == Severity::Warning && i.message.contains("non-standard field")
            )
        );
    }

    #[test]
    fn lint_clean_skill() {
        let tmp = TempDir::new().unwrap();
        create_skill_dir(
            tmp.path(),
            "my-skill",
            "---\nname: my-skill\ndescription: A valid skill\n---\n# Body",
        );
        let issues = lint_skill("my-skill", &tmp.path().join("my-skill"));
        assert!(issues.is_empty(), "expected no issues, got: {:?}", issues);
    }

    #[test]
    fn lint_report_counts() {
        let tmp = TempDir::new().unwrap();
        // Clean skill
        create_skill_dir(
            tmp.path(),
            "good-skill",
            "---\nname: good-skill\ndescription: Valid\n---\nbody",
        );
        // Skill with errors (missing description + name mismatch)
        create_skill_dir(tmp.path(), "bad-skill", "---\nname: wrong-name\n---\nbody");

        let report = lint_library(tmp.path());
        assert_eq!(report.skills_checked, 2);
        assert!(report.has_errors());
        assert!(report.error_count() >= 2); // name mismatch + missing description
    }

    #[test]
    fn lint_skips_dotfiles() {
        let tmp = TempDir::new().unwrap();
        // Create a dotfile directory that should be skipped
        let dotdir = tmp.path().join(".hidden");
        std::fs::create_dir_all(&dotdir).unwrap();
        std::fs::write(dotdir.join("SKILL.md"), "---\nname: hidden\n---\nbody").unwrap();

        let report = lint_library(tmp.path());
        assert_eq!(report.skills_checked, 0);
    }

    #[test]
    fn lint_missing_skill_md_file() {
        let tmp = TempDir::new().unwrap();
        // Skill directory without SKILL.md — should not be counted
        std::fs::create_dir_all(tmp.path().join("empty-skill")).unwrap();

        let report = lint_library(tmp.path());
        assert_eq!(report.skills_checked, 0);
    }
}
