//! Skill discovery from configured sources. Supports `ClaudePlugins` and `Directory` source types,
//! with deduplication (first source wins) and exclusion filtering.

use anyhow::{Context, Result};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

use crate::config::{Config, Source, SourceType};

/// A validated skill name.
///
/// Lenient validation: rejects empty names and path separators.
/// Warns on names that don't match the strict `[a-z0-9-]+` pattern
/// (which will become a hard requirement in v0.3).
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct SkillName(String);

impl SkillName {
    pub fn new(name: impl Into<String>) -> Result<Self> {
        let name = name.into();
        anyhow::ensure!(!name.is_empty(), "skill name cannot be empty");
        anyhow::ensure!(
            !name.contains('/') && !name.contains('\\'),
            "skill name contains path separator: '{name}'"
        );
        if !name
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
        {
            eprintln!(
                "warning: skill name '{}' should be lowercase letters, digits, or hyphens",
                name
            );
        }
        Ok(Self(name))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for SkillName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl AsRef<str> for SkillName {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl AsRef<Path> for SkillName {
    fn as_ref(&self) -> &Path {
        Path::new(&self.0)
    }
}

impl PartialEq<str> for SkillName {
    fn eq(&self, other: &str) -> bool {
        self.0 == other
    }
}

impl PartialEq<&str> for SkillName {
    fn eq(&self, other: &&str) -> bool {
        self.0 == *other
    }
}

/// A discovered skill with its metadata.
#[derive(Debug, Clone)]
pub struct DiscoveredSkill {
    /// Skill name (directory name)
    pub name: SkillName,
    /// Path to the skill directory (contains SKILL.md)
    pub path: PathBuf,
    /// Which source this skill came from
    pub source_name: String,
}

/// Discover all skills from configured sources.
///
/// Returns deduplicated skills — first source wins on name conflicts.
/// Applies exclusion list from config.
pub fn discover_all(config: &Config) -> Result<Vec<DiscoveredSkill>> {
    let mut seen: HashMap<String, usize> = HashMap::new();
    let mut skills: Vec<DiscoveredSkill> = Vec::new();
    let mut conflicts: Vec<(String, String, String)> = Vec::new();

    for source in &config.sources {
        let source_skills = discover_source(source)?;

        for skill in source_skills {
            if config.exclude.iter().any(|e| e == skill.name.as_str()) {
                continue;
            }

            let name_str = skill.name.as_str().to_string();
            if let Some(&existing_idx) = seen.get(&name_str) {
                let existing = &skills[existing_idx];
                conflicts.push((
                    name_str,
                    existing.source_name.clone(),
                    skill.source_name.clone(),
                ));
            } else {
                seen.insert(name_str, skills.len());
                skills.push(skill);
            }
        }
    }

    for (name, winner, loser) in &conflicts {
        eprintln!(
            "warning: skill '{}' found in both '{}' and '{}', using '{}'",
            name, winner, loser, winner
        );
    }

    Ok(skills)
}

/// Discover skills from a single source.
pub fn discover_source(source: &Source) -> Result<Vec<DiscoveredSkill>> {
    match source.source_type {
        SourceType::ClaudePlugins => discover_claude_plugins(source),
        SourceType::Directory => discover_directory(source),
    }
}

/// Discover skills from a Claude plugins cache directory.
///
/// Reads `installed_plugins.json` from the parent of `source.path`,
/// then scans each plugin's `skills/*/SKILL.md`.
fn discover_claude_plugins(source: &Source) -> Result<Vec<DiscoveredSkill>> {
    let plugins_json_path = source.path.join("installed_plugins.json");

    if !plugins_json_path.exists() {
        // Try parent directory (path might point to cache dir)
        let parent_json = source
            .path
            .parent()
            .map(|p| p.join("installed_plugins.json"));

        if let Some(ref path) = parent_json
            && path.exists()
        {
            eprintln!(
                "warning: installed_plugins.json not found at '{}', trying parent directory",
                plugins_json_path.display()
            );
            return discover_claude_plugins_from_json(path, &source.name);
        }

        eprintln!(
            "warning: no installed_plugins.json found for source '{}'",
            source.name
        );
        return Ok(Vec::new());
    }

    discover_claude_plugins_from_json(&plugins_json_path, &source.name)
}

fn discover_claude_plugins_from_json(
    json_path: &Path,
    source_name: &str,
) -> Result<Vec<DiscoveredSkill>> {
    let content = std::fs::read_to_string(json_path)
        .with_context(|| format!("failed to read {}", json_path.display()))?;

    let plugins: serde_json::Value = serde_json::from_str(&content)
        .with_context(|| format!("failed to parse {}", json_path.display()))?;

    let mut skills = Vec::new();

    if let Some(arr) = plugins.as_array() {
        // v1 format: flat array of plugin objects with "installPath"
        scan_install_records(arr, source_name, &mut skills)?;
    } else if let Some(obj) = plugins.get("plugins").and_then(|v| v.as_object()) {
        // v2 format: { "version": 2, "plugins": { "name@registry": [records...] } }
        for records in obj.values() {
            if let Some(arr) = records.as_array() {
                scan_install_records(arr, source_name, &mut skills)?;
            }
        }
    } else {
        eprintln!(
            "warning: unrecognized installed_plugins.json format in {}",
            json_path.display()
        );
    }

    Ok(skills)
}

/// Scan an array of plugin install records for skills at each `installPath`.
fn scan_install_records(
    records: &[serde_json::Value],
    source_name: &str,
    skills: &mut Vec<DiscoveredSkill>,
) -> Result<()> {
    for record in records {
        if let Some(install_path) = record.get("installPath").and_then(|v| v.as_str()) {
            let skills_dir = PathBuf::from(install_path).join("skills");
            if skills_dir.is_dir() {
                skills.append(&mut scan_for_skills(&skills_dir, source_name)?);
            }
        }
    }
    Ok(())
}

/// Discover skills from a flat directory (scan for */SKILL.md).
fn discover_directory(source: &Source) -> Result<Vec<DiscoveredSkill>> {
    if !source.path.is_dir() {
        eprintln!(
            "warning: source '{}' path does not exist: {}",
            source.name,
            source.path.display()
        );
        return Ok(Vec::new());
    }

    scan_for_skills(&source.path, &source.name)
}

/// Scan a directory for skill subdirectories containing SKILL.md.
fn scan_for_skills(dir: &Path, source_name: &str) -> Result<Vec<DiscoveredSkill>> {
    let mut skills = Vec::new();

    for entry in WalkDir::new(dir)
        .follow_links(false)
        .min_depth(1)
        .max_depth(2)
        .into_iter()
        .filter_map(|e| match e {
            Ok(entry) => Some(entry),
            Err(err) => {
                eprintln!("warning: skipping entry in {}: {}", dir.display(), err);
                None
            }
        })
    {
        if entry.file_name() == "SKILL.md"
            && entry.file_type().is_file()
            && let Some(skill_dir) = entry.path().parent()
            && let Some(name_str) = skill_dir.file_name().and_then(|n| n.to_str())
        {
            match SkillName::new(name_str) {
                Ok(name) => {
                    skills.push(DiscoveredSkill {
                        name,
                        path: skill_dir.to_path_buf(),
                        source_name: source_name.to_string(),
                    });
                }
                Err(e) => {
                    eprintln!("warning: skipping skill in {}: {}", skill_dir.display(), e);
                }
            }
        }
    }

    Ok(skills)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Source;
    use tempfile::TempDir;

    fn create_skill(dir: &Path, name: &str) {
        let skill_dir = dir.join(name);
        std::fs::create_dir_all(&skill_dir).unwrap();
        std::fs::write(
            skill_dir.join("SKILL.md"),
            format!("---\nname: {name}\n---\n# {name}"),
        )
        .unwrap();
    }

    #[test]
    fn discover_directory_finds_skills() {
        let tmp = TempDir::new().unwrap();
        create_skill(tmp.path(), "skill-a");
        create_skill(tmp.path(), "skill-b");
        // Not a skill — no SKILL.md
        std::fs::create_dir_all(tmp.path().join("not-a-skill")).unwrap();
        std::fs::write(tmp.path().join("not-a-skill/README.md"), "hi").unwrap();

        let source = Source {
            name: "test".into(),
            path: tmp.path().to_path_buf(),
            source_type: SourceType::Directory,
        };
        let skills = discover_directory(&source).unwrap();
        assert_eq!(skills.len(), 2);
    }

    #[test]
    fn discover_directory_warns_on_missing_path() {
        let source = Source {
            name: "missing".into(),
            path: PathBuf::from("/nonexistent/path"),
            source_type: SourceType::Directory,
        };
        let skills = discover_directory(&source).unwrap();
        assert!(skills.is_empty());
    }

    #[test]
    fn discover_all_deduplicates_first_wins() {
        let tmp1 = TempDir::new().unwrap();
        let tmp2 = TempDir::new().unwrap();
        create_skill(tmp1.path(), "shared-skill");
        create_skill(tmp2.path(), "shared-skill");
        create_skill(tmp2.path(), "unique-skill");

        let config = Config {
            sources: vec![
                Source {
                    name: "first".into(),
                    path: tmp1.path().to_path_buf(),
                    source_type: SourceType::Directory,
                },
                Source {
                    name: "second".into(),
                    path: tmp2.path().to_path_buf(),
                    source_type: SourceType::Directory,
                },
            ],
            ..Config::default()
        };

        let skills = discover_all(&config).unwrap();
        assert_eq!(skills.len(), 2);

        let shared = skills.iter().find(|s| s.name == "shared-skill").unwrap();
        assert_eq!(shared.source_name, "first");
    }

    #[test]
    fn discover_all_applies_exclusions() {
        let tmp = TempDir::new().unwrap();
        create_skill(tmp.path(), "keep-me");
        create_skill(tmp.path(), "exclude-me");

        let config = Config {
            exclude: vec!["exclude-me".into()],
            sources: vec![Source {
                name: "test".into(),
                path: tmp.path().to_path_buf(),
                source_type: SourceType::Directory,
            }],
            ..Config::default()
        };

        let skills = discover_all(&config).unwrap();
        assert_eq!(skills.len(), 1);
        assert_eq!(skills[0].name, "keep-me");
    }

    #[test]
    fn discover_claude_plugins_reads_json() {
        let tmp = TempDir::new().unwrap();
        let plugin_dir = tmp.path().join("my-plugin");
        create_skill(&plugin_dir.join("skills"), "plugin-skill");

        let json = serde_json::json!([
            { "installPath": plugin_dir.to_str().unwrap() }
        ]);
        std::fs::write(
            tmp.path().join("installed_plugins.json"),
            serde_json::to_string(&json).unwrap(),
        )
        .unwrap();

        let source = Source {
            name: "plugins".into(),
            path: tmp.path().to_path_buf(),
            source_type: SourceType::ClaudePlugins,
        };
        let skills = discover_claude_plugins(&source).unwrap();
        assert_eq!(skills.len(), 1);
        assert_eq!(skills[0].name, "plugin-skill");
    }

    #[test]
    fn discover_claude_plugins_reads_v2_json() {
        let tmp = TempDir::new().unwrap();

        // Create two plugins with skills at their install paths
        let plugin_a_dir = tmp.path().join("plugin-a-install");
        create_skill(&plugin_a_dir.join("skills"), "swift-skill");

        let plugin_b_dir = tmp.path().join("plugin-b-install");
        create_skill(&plugin_b_dir.join("skills"), "rust-skill");

        // v2 format: { "version": 2, "plugins": { "name@registry": [ { "installPath": ... } ] } }
        let json = serde_json::json!({
            "version": 2,
            "plugins": {
                "swift-skill@swift-registry": [
                    {
                        "scope": "user",
                        "installPath": plugin_a_dir.to_str().unwrap(),
                        "version": "1.0.0",
                        "installedAt": "2025-12-15T02:47:14.944Z"
                    }
                ],
                "rust-skill@rust-registry": [
                    {
                        "scope": "user",
                        "installPath": plugin_b_dir.to_str().unwrap(),
                        "version": "2.0.0",
                        "installedAt": "2026-01-05T04:13:51.923Z"
                    }
                ]
            }
        });
        std::fs::write(
            tmp.path().join("installed_plugins.json"),
            serde_json::to_string(&json).unwrap(),
        )
        .unwrap();

        let source = Source {
            name: "plugins".into(),
            path: tmp.path().to_path_buf(),
            source_type: SourceType::ClaudePlugins,
        };
        let skills = discover_claude_plugins(&source).unwrap();
        assert_eq!(skills.len(), 2);

        let names: Vec<&str> = skills.iter().map(|s| s.name.as_str()).collect();
        assert!(names.contains(&"swift-skill"));
        assert!(names.contains(&"rust-skill"));
    }

    #[test]
    fn discover_claude_plugins_unknown_format() {
        let tmp = TempDir::new().unwrap();

        // Unknown format: an object with no "plugins" key and not an array
        let json = serde_json::json!({
            "version": 99,
            "something_else": "unexpected"
        });
        std::fs::write(
            tmp.path().join("installed_plugins.json"),
            serde_json::to_string(&json).unwrap(),
        )
        .unwrap();

        let skills =
            discover_claude_plugins_from_json(&tmp.path().join("installed_plugins.json"), "test")
                .unwrap();
        assert!(skills.is_empty());
    }

    #[test]
    fn skill_name_rejects_empty() {
        assert!(SkillName::new("").is_err());
    }

    #[test]
    fn skill_name_rejects_path_separator() {
        assert!(SkillName::new("foo/bar").is_err());
        assert!(SkillName::new("foo\\bar").is_err());
    }

    #[test]
    fn skill_name_accepts_valid() {
        let name = SkillName::new("my-skill-123").unwrap();
        assert_eq!(name.as_str(), "my-skill-123");
        assert_eq!(name.to_string(), "my-skill-123");
        assert_eq!(name, *"my-skill-123");
    }
}
