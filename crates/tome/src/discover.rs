//! Skill discovery from configured sources. Supports `ClaudePlugins` and `Directory` source types,
//! with deduplication (first source wins) and exclusion filtering.

use anyhow::{Context, Result};
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

use crate::config::{Config, Source, SourceType};

/// A validated skill name.
///
/// Lenient validation: rejects empty names and path separators.
/// Warns on names that don't match the strict `[a-z0-9-]+` pattern
/// (which may become a hard requirement in a future version).
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, serde::Serialize)]
#[serde(transparent)]
pub struct SkillName(String);

impl SkillName {
    /// Create a new skill name from any string-like value.
    ///
    /// Rejects empty names and names containing path separators (`/` or `\`).
    ///
    /// # Examples
    ///
    /// ```text
    /// let name = SkillName::new("my-skill").unwrap();
    /// assert_eq!(name.as_str(), "my-skill");
    ///
    /// // Empty names and path separators are rejected
    /// assert!(SkillName::new("").is_err());
    /// assert!(SkillName::new("foo/bar").is_err());
    /// ```
    pub fn new(name: impl Into<String>) -> Result<Self> {
        let name = name.into();
        crate::validation::validate_identifier(&name, "skill name")?;
        Ok(Self(name))
    }

    /// Whether this name follows the strict `[a-z0-9-]+` convention
    /// (which may become a hard requirement in a future version).
    pub fn is_conventional(&self) -> bool {
        self.0
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
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

impl std::borrow::Borrow<str> for SkillName {
    fn borrow(&self) -> &str {
        &self.0
    }
}

impl TryFrom<String> for SkillName {
    type Error = anyhow::Error;

    fn try_from(s: String) -> Result<Self> {
        Self::new(s)
    }
}

impl<'de> serde::Deserialize<'de> for SkillName {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        SkillName::new(s).map_err(serde::de::Error::custom)
    }
}

/// Provenance metadata from package manager sources.
#[derive(Debug, Clone)]
pub struct SkillProvenance {
    /// Registry identifier (e.g. "my-plugin@npm")
    pub registry_id: String,
    /// Version string (e.g. "1.2.0"). `None` when not available.
    pub version: Option<String>,
    /// Git commit SHA for exact version pinning. `None` when not available.
    pub git_commit_sha: Option<String>,
}

/// How a skill was sourced — determines consolidation strategy.
#[derive(Debug, Clone)]
pub enum SkillOrigin {
    /// Managed by a package manager; library entry is a symlink to source dir.
    Managed { provenance: Option<SkillProvenance> },
    /// Local skill; library entry is a copy of the source.
    Local,
}

impl SkillOrigin {
    pub fn is_managed(&self) -> bool {
        matches!(self, Self::Managed { .. })
    }

    pub fn provenance(&self) -> Option<&SkillProvenance> {
        match self {
            Self::Managed { provenance } => provenance.as_ref(),
            Self::Local => None,
        }
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
    /// How this skill was sourced (managed vs local), with optional provenance metadata.
    pub origin: SkillOrigin,
    /// Parsed frontmatter from SKILL.md (None if parsing failed).
    #[allow(dead_code)]
    pub frontmatter: Option<crate::skill::SkillFrontmatter>,
}

/// Discover all skills from configured sources.
///
/// Returns deduplicated skills — first source wins on name conflicts.
/// Applies exclusion list from config.
/// Warnings about naming conventions and deduplication are collected in `warnings`.
pub fn discover_all(config: &Config, warnings: &mut Vec<String>) -> Result<Vec<DiscoveredSkill>> {
    let mut seen: HashMap<String, usize> = HashMap::new();
    let mut skills: Vec<DiscoveredSkill> = Vec::new();
    let mut conflicts: Vec<(String, String, String)> = Vec::new();

    for source in &config.sources {
        let source_skills = discover_source(source, warnings)?;

        for skill in source_skills {
            if config.exclude.contains(&skill.name) {
                continue;
            }

            if !skill.name.is_conventional() {
                warnings.push(format!(
                    "skill name '{}' should be lowercase letters, digits, or hyphens",
                    skill.name
                ));
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
        warnings.push(format!(
            "skill '{}' found in both '{}' and '{}', using '{}'",
            name, winner, loser, winner
        ));
    }

    Ok(skills)
}

/// Discover skills from a single source.
pub fn discover_source(
    source: &Source,
    warnings: &mut Vec<String>,
) -> Result<Vec<DiscoveredSkill>> {
    match source.source_type {
        SourceType::ClaudePlugins => discover_claude_plugins(source, warnings),
        SourceType::Directory => discover_directory(source, warnings),
    }
}

/// Discover skills from a Claude plugins cache directory.
///
/// Reads `installed_plugins.json` from the parent of `source.path`,
/// then scans each plugin's `skills/*/SKILL.md`.
fn discover_claude_plugins(
    source: &Source,
    warnings: &mut Vec<String>,
) -> Result<Vec<DiscoveredSkill>> {
    // Look for installed_plugins.json in multiple locations:
    // 1. Directly in source.path (e.g. ~/.claude/plugins/)
    // 2. Parent directory (when source.path points to cache subdir, e.g. ~/.claude/plugins/cache/)
    let mut candidates = vec![source.path.join("installed_plugins.json")];
    if let Some(parent) = source.path.parent() {
        candidates.push(parent.join("installed_plugins.json"));
    }

    for candidate in &candidates {
        if candidate.exists() {
            return discover_claude_plugins_from_json(candidate, &source.name, warnings);
        }
    }

    warnings.push(format!(
        "no installed_plugins.json found for source '{}'",
        source.name
    ));
    Ok(Vec::new())
}

fn discover_claude_plugins_from_json(
    json_path: &Path,
    source_name: &str,
    warnings: &mut Vec<String>,
) -> Result<Vec<DiscoveredSkill>> {
    let content = std::fs::read_to_string(json_path)
        .with_context(|| format!("failed to read {}", json_path.display()))?;

    let plugins: serde_json::Value = serde_json::from_str(&content)
        .with_context(|| format!("failed to parse {}", json_path.display()))?;

    let mut raw_skills = Vec::new();

    if let Some(arr) = plugins.as_array() {
        // v1 format: flat array of plugin objects with "installPath"
        scan_install_records(arr, source_name, None, &mut raw_skills, warnings)?;
    } else if let Some(obj) = plugins.get("plugins").and_then(|v| v.as_object()) {
        // v2 format: { "version": 2, "plugins": { "name@registry": [records...] } }
        for (plugin_name, records) in obj {
            if let Some(arr) = records.as_array() {
                scan_install_records(
                    arr,
                    source_name,
                    Some(plugin_name),
                    &mut raw_skills,
                    warnings,
                )?;
            } else {
                warnings.push(format!(
                    "unexpected format for plugin '{}' in {} — expected array, skipping",
                    plugin_name,
                    json_path.display()
                ));
            }
        }
    } else {
        warnings.push(format!(
            "unrecognized installed_plugins.json format in {}",
            json_path.display()
        ));
    }

    // Deduplicate within a single source — multiple install records can point to the
    // same installPath, which would otherwise surface as spurious same-source conflicts.
    let mut seen: HashSet<String> = HashSet::new();
    let skills = raw_skills
        .into_iter()
        .filter(|s| seen.insert(s.name.as_str().to_string()))
        .collect();

    Ok(skills)
}

/// Scan an array of plugin install records for skills at each `installPath`.
///
/// When `registry_id` is provided (v2 format), provenance metadata (registry ID + version)
/// is attached to each discovered skill for lockfile generation.
fn scan_install_records(
    records: &[serde_json::Value],
    source_name: &str,
    registry_id: Option<&str>,
    skills: &mut Vec<DiscoveredSkill>,
    warnings: &mut Vec<String>,
) -> Result<()> {
    for record in records {
        if let Some(install_path) = record.get("installPath").and_then(|v| v.as_str()) {
            let skills_dir = PathBuf::from(install_path).join("skills");
            if skills_dir.is_dir() {
                let provenance = registry_id.map(|reg_id| {
                    let version = record
                        .get("version")
                        .and_then(|v| v.as_str())
                        .filter(|v| !v.is_empty())
                        .map(|v| v.to_string());
                    let git_commit_sha = record
                        .get("gitCommitSha")
                        .and_then(|v| v.as_str())
                        .filter(|v| !v.is_empty())
                        .map(|v| v.to_string());
                    SkillProvenance {
                        registry_id: reg_id.to_string(),
                        version,
                        git_commit_sha,
                    }
                });
                let mut found =
                    scan_for_skills(&skills_dir, source_name, Some(provenance), warnings)?;
                skills.append(&mut found);
            }
        }
    }
    Ok(())
}

/// Discover skills from a flat directory (scan for */SKILL.md).
fn discover_directory(source: &Source, warnings: &mut Vec<String>) -> Result<Vec<DiscoveredSkill>> {
    if !source.path.exists() {
        warnings.push(format!(
            "source '{}' path does not exist: {}",
            source.name,
            source.path.display()
        ));
        return Ok(Vec::new());
    }

    if !source.path.is_dir() {
        warnings.push(format!(
            "source '{}' path exists but is not a directory: {} — skipping",
            source.name,
            source.path.display()
        ));
        return Ok(Vec::new());
    }

    scan_for_skills(&source.path, &source.name, None, warnings)
}

/// Scan a directory for skill subdirectories containing SKILL.md.
///
/// When `managed_provenance` is `Some`, skills are marked as `Managed` with the
/// given provenance (which itself may be `None` for v1 plugins). When
/// `managed_provenance` is `None`, skills are marked as `Local`.
fn scan_for_skills(
    dir: &Path,
    source_name: &str,
    managed_provenance: Option<Option<SkillProvenance>>,
    warnings: &mut Vec<String>,
) -> Result<Vec<DiscoveredSkill>> {
    let mut skills = Vec::new();

    // Collect walkdir results into entries and walk errors separately,
    // so that the mutable borrow of `warnings` isn't held across the loop body.
    let (entries, walk_errors): (Vec<_>, Vec<_>) = WalkDir::new(dir)
        .follow_links(false)
        .min_depth(1)
        .max_depth(2)
        .into_iter()
        .partition(|e| e.is_ok());

    // Push walk errors as warnings.
    for err in walk_errors {
        let err = err.unwrap_err();
        // Distinguish root errors (whole source unreadable) from sub-entry errors.
        // A root error means the walk immediately failed — this is likely a config
        // or permissions issue that warrants a more visible warning.
        let path = err.path().unwrap_or(dir);
        if path == dir {
            warnings.push(format!(
                "cannot read source directory {}: {}",
                dir.display(),
                err
            ));
        } else {
            warnings.push(format!("skipping entry in {}: {}", dir.display(), err));
        }
    }

    for entry in entries {
        let entry = entry.expect("BUG: entries vec was filtered to Ok variants by partition above");
        if entry.file_name() == "SKILL.md"
            && entry.file_type().is_file()
            && let Some(skill_dir) = entry.path().parent()
            && skill_dir != dir // skip SKILL.md at source root
            && let Some(name_str) = skill_dir.file_name().and_then(|n| n.to_str())
        {
            match SkillName::new(name_str) {
                Ok(name) => {
                    let origin = match &managed_provenance {
                        Some(prov) => SkillOrigin::Managed {
                            provenance: prov.clone(),
                        },
                        None => SkillOrigin::Local,
                    };
                    // Parse frontmatter if SKILL.md exists and is readable
                    let frontmatter = match std::fs::read_to_string(skill_dir.join("SKILL.md")) {
                        Ok(content) => match crate::skill::parse(&content) {
                            Ok((fm, _body)) => Some(fm),
                            Err(e) => {
                                warnings.push(format!(
                                    "could not parse frontmatter in {}/SKILL.md: {}",
                                    skill_dir.display(),
                                    e
                                ));
                                None
                            }
                        },
                        Err(_) => None, // File not readable — already validated by scan
                    };

                    skills.push(DiscoveredSkill {
                        name,
                        path: skill_dir.to_path_buf(),
                        source_name: source_name.to_string(),
                        origin,
                        frontmatter,
                    });
                }
                Err(e) => {
                    warnings.push(format!("skipping skill in {}: {}", skill_dir.display(), e));
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
        let skills = discover_directory(&source, &mut Vec::new()).unwrap();
        assert_eq!(skills.len(), 2);
    }

    #[test]
    fn discover_directory_warns_on_missing_path() {
        let source = Source {
            name: "missing".into(),
            path: PathBuf::from("/nonexistent/path"),
            source_type: SourceType::Directory,
        };
        let mut warnings = Vec::new();
        let skills = discover_directory(&source, &mut warnings).unwrap();
        assert!(skills.is_empty());
        assert_eq!(warnings.len(), 1);
        assert!(warnings[0].contains("does not exist"));
    }

    #[test]
    fn discover_directory_skips_skill_md_at_source_root() {
        let tmp = TempDir::new().unwrap();
        // SKILL.md directly at the source root (not inside a subdirectory)
        std::fs::write(
            tmp.path().join("SKILL.md"),
            "---\nname: root-skill\n---\n# Root",
        )
        .unwrap();
        // A legitimate skill in a subdirectory
        create_skill(tmp.path(), "real-skill");

        let source = Source {
            name: "test".into(),
            path: tmp.path().to_path_buf(),
            source_type: SourceType::Directory,
        };
        let skills = discover_directory(&source, &mut Vec::new()).unwrap();
        assert_eq!(skills.len(), 1);
        assert_eq!(skills[0].name, "real-skill");
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

        let skills = discover_all(&config, &mut Vec::new()).unwrap();
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
            exclude: [SkillName::new("exclude-me").unwrap()].into(),
            sources: vec![Source {
                name: "test".into(),
                path: tmp.path().to_path_buf(),
                source_type: SourceType::Directory,
            }],
            ..Config::default()
        };

        let skills = discover_all(&config, &mut Vec::new()).unwrap();
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
        let skills = discover_claude_plugins(&source, &mut Vec::new()).unwrap();
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
                        "installedAt": "2025-12-15T02:47:14.944Z",
                        "gitCommitSha": "eb872450105745e9489c6f6d73fa2fb4ff5a1e9a"
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
        let skills = discover_claude_plugins(&source, &mut Vec::new()).unwrap();
        assert_eq!(skills.len(), 2);

        let names: Vec<&str> = skills.iter().map(|s| s.name.as_str()).collect();
        assert!(names.contains(&"swift-skill"));
        assert!(names.contains(&"rust-skill"));

        // v2 format should capture provenance metadata
        let swift = skills.iter().find(|s| s.name == "swift-skill").unwrap();
        let prov = swift
            .origin
            .provenance()
            .expect("v2 should have provenance");
        assert_eq!(prov.registry_id, "swift-skill@swift-registry");
        assert_eq!(prov.version.as_deref(), Some("1.0.0"));
        assert_eq!(
            prov.git_commit_sha.as_deref(),
            Some("eb872450105745e9489c6f6d73fa2fb4ff5a1e9a")
        );

        let rust_s = skills.iter().find(|s| s.name == "rust-skill").unwrap();
        let prov = rust_s
            .origin
            .provenance()
            .expect("v2 should have provenance");
        assert_eq!(prov.registry_id, "rust-skill@rust-registry");
        assert_eq!(prov.version.as_deref(), Some("2.0.0"));
        // rust-skill has no gitCommitSha in the fixture
        assert!(prov.git_commit_sha.is_none());
    }

    #[test]
    fn discover_claude_plugins_v1_no_provenance() {
        let tmp = TempDir::new().unwrap();
        let plugin_dir = tmp.path().join("my-plugin");
        create_skill(&plugin_dir.join("skills"), "v1-skill");

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
        let skills = discover_claude_plugins(&source, &mut Vec::new()).unwrap();
        assert_eq!(skills.len(), 1);
        assert!(
            skills[0].origin.provenance().is_none(),
            "v1 format should not have provenance"
        );
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

        let mut warnings = Vec::new();
        let skills = discover_claude_plugins_from_json(
            &tmp.path().join("installed_plugins.json"),
            "test",
            &mut warnings,
        )
        .unwrap();
        assert!(skills.is_empty());
        assert_eq!(warnings.len(), 1);
        assert!(warnings[0].contains("unrecognized"));
    }

    #[test]
    fn discover_claude_plugins_deduplicates_within_source() {
        let tmp = TempDir::new().unwrap();

        // Two install records pointing to the same plugin dir → same skill
        let plugin_dir = tmp.path().join("my-plugin");
        create_skill(&plugin_dir.join("skills"), "shared-skill");

        let json = serde_json::json!([
            { "installPath": plugin_dir.to_str().unwrap() },
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
        let skills = discover_claude_plugins(&source, &mut Vec::new()).unwrap();
        // Should deduplicate to 1, not produce a spurious conflict
        assert_eq!(skills.len(), 1);
        assert_eq!(skills[0].name, "shared-skill");
    }

    #[test]
    fn discover_all_with_partial_config_returns_skills() {
        // Simulates what the wizard does: build a partial Config from selected
        // sources and run discover_all to populate the exclusion picker.
        let tmp = TempDir::new().unwrap();
        create_skill(tmp.path(), "skill-alpha");
        create_skill(tmp.path(), "skill-beta");

        let config = Config {
            sources: vec![Source {
                name: "wizard-test".into(),
                path: tmp.path().to_path_buf(),
                source_type: SourceType::Directory,
            }],
            ..Config::default()
        };

        let skills = discover_all(&config, &mut Vec::new()).unwrap();
        assert_eq!(skills.len(), 2);
        let names: Vec<&str> = skills.iter().map(|s| s.name.as_str()).collect();
        assert!(names.contains(&"skill-alpha"));
        assert!(names.contains(&"skill-beta"));
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

    #[test]
    fn skill_name_rejects_dot_special() {
        assert!(SkillName::new(".").is_err());
        assert!(SkillName::new("..").is_err());
    }

    #[test]
    fn skill_name_rejects_whitespace() {
        assert!(SkillName::new("  ").is_err());
        assert!(SkillName::new(" leading").is_err());
        assert!(SkillName::new("trailing ").is_err());
    }

    #[test]
    fn skill_name_conventional_check() {
        assert!(SkillName::new("my-skill-123").unwrap().is_conventional());
        assert!(!SkillName::new("My_Skill").unwrap().is_conventional());
        assert!(!SkillName::new("UPPER").unwrap().is_conventional());
    }

    #[test]
    fn discover_claude_plugins_parent_path_json() {
        let tmp = TempDir::new().unwrap();

        // source.path points to a cache/ subdirectory
        let cache_dir = tmp.path().join("cache");
        std::fs::create_dir_all(&cache_dir).unwrap();

        // installed_plugins.json is in the PARENT directory (not in cache/)
        let plugin_dir = tmp.path().join("my-plugin");
        create_skill(&plugin_dir.join("skills"), "parent-skill");

        let json = serde_json::json!([
            { "installPath": plugin_dir.to_str().unwrap() }
        ]);
        std::fs::write(
            tmp.path().join("installed_plugins.json"),
            serde_json::to_string(&json).unwrap(),
        )
        .unwrap();

        // Verify the JSON is NOT in the cache dir itself
        assert!(!cache_dir.join("installed_plugins.json").exists());

        let source = Source {
            name: "plugins".into(),
            path: cache_dir,
            source_type: SourceType::ClaudePlugins,
        };
        let skills = discover_claude_plugins(&source, &mut Vec::new()).unwrap();
        assert_eq!(skills.len(), 1);
        assert_eq!(skills[0].name, "parent-skill");
    }

    #[test]
    fn discover_all_collects_naming_warnings() {
        let tmp = TempDir::new().unwrap();
        create_skill(tmp.path(), "My_Unconventional");

        let config = Config {
            sources: vec![Source {
                name: "test".into(),
                path: tmp.path().to_path_buf(),
                source_type: SourceType::Directory,
            }],
            ..Config::default()
        };

        let mut warnings = Vec::new();
        let skills = discover_all(&config, &mut warnings).unwrap();
        assert_eq!(skills.len(), 1);
        assert_eq!(warnings.len(), 1);
        assert!(warnings[0].contains("lowercase"));
    }

    #[test]
    fn discover_all_collects_dedup_warnings() {
        let tmp1 = TempDir::new().unwrap();
        let tmp2 = TempDir::new().unwrap();
        create_skill(tmp1.path(), "shared");
        create_skill(tmp2.path(), "shared");

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

        let mut warnings = Vec::new();
        let skills = discover_all(&config, &mut warnings).unwrap();
        assert_eq!(skills.len(), 1);
        assert_eq!(warnings.len(), 1);
        assert!(warnings[0].contains("first"));
        assert!(warnings[0].contains("second"));
    }
}
