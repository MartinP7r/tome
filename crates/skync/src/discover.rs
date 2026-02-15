use anyhow::{Context, Result};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

use crate::config::{Config, Source, SourceType};

/// A discovered skill with its metadata.
#[derive(Debug, Clone)]
pub struct DiscoveredSkill {
    /// Skill name (directory name)
    pub name: String,
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
            if config.exclude.contains(&skill.name) {
                continue;
            }

            if let Some(&existing_idx) = seen.get(&skill.name) {
                let existing = &skills[existing_idx];
                conflicts.push((
                    skill.name.clone(),
                    existing.source_name.clone(),
                    skill.source_name.clone(),
                ));
            } else {
                seen.insert(skill.name.clone(), skills.len());
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
fn discover_source(source: &Source) -> Result<Vec<DiscoveredSkill>> {
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

        if let Some(ref path) = parent_json {
            if path.exists() {
                return discover_claude_plugins_from_json(path, &source.name);
            }
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
        for plugin in arr {
            if let Some(install_path) = plugin.get("installPath").and_then(|v| v.as_str()) {
                let skills_dir = PathBuf::from(install_path).join("skills");
                if skills_dir.is_dir() {
                    let mut dir_skills = scan_for_skills(&skills_dir, source_name)?;
                    skills.append(&mut dir_skills);
                }
            }
        }
    }

    Ok(skills)
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
        .filter_map(|e| e.ok())
    {
        if entry.file_name() == "SKILL.md" && entry.file_type().is_file() {
            if let Some(skill_dir) = entry.path().parent() {
                if let Some(name) = skill_dir.file_name().and_then(|n| n.to_str()) {
                    skills.push(DiscoveredSkill {
                        name: name.to_string(),
                        path: skill_dir.to_path_buf(),
                        source_name: source_name.to_string(),
                    });
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
}
