//! Lockfile for reproducible skill libraries.
//!
//! The `tome.lock` file captures provenance metadata (source name, content hash,
//! registry ID + version for managed plugins) for every skill in the library.
//! It is regenerated after every `tome sync` and is meant to be committed to version control.

use std::collections::BTreeMap;
use std::path::Path;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

use crate::discover::DiscoveredSkill;
use crate::manifest::Manifest;

const LOCKFILE_NAME: &str = "tome.lock";

/// Top-level lockfile structure.
#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct Lockfile {
    /// Schema version (currently 1).
    pub version: u32,
    /// One entry per skill, keyed by skill name.
    pub skills: BTreeMap<String, LockEntry>,
}

/// A single skill entry in the lockfile.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct LockEntry {
    /// Config source name (maps to a `[[sources]]` entry in `config.toml`).
    pub source_name: String,
    /// SHA-256 content hash of the skill directory.
    pub content_hash: String,
    /// Registry identifier (e.g. "my-plugin@npm"). Present for managed plugins.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub registry_id: Option<String>,
    /// Version string (e.g. "1.2.0"). Present for managed plugins.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
}

/// Generate a lockfile from the manifest and discovered skills.
///
/// For each manifest entry, looks up the matching `DiscoveredSkill` to extract
/// provenance metadata (registry_id, version) when available.
pub fn generate(manifest: &Manifest, skills: &[DiscoveredSkill]) -> Lockfile {
    let skill_map: BTreeMap<&str, &DiscoveredSkill> =
        skills.iter().map(|s| (s.name.as_str(), s)).collect();

    let mut entries = BTreeMap::new();

    for (name, entry) in manifest.iter() {
        let (registry_id, version) = skill_map
            .get(name.as_str())
            .and_then(|s| s.provenance.as_ref())
            .map(|p| {
                let version = if p.version.is_empty() {
                    None
                } else {
                    Some(p.version.clone())
                };
                (Some(p.registry_id.clone()), version)
            })
            .unwrap_or((None, None));

        entries.insert(
            name.to_string(),
            LockEntry {
                source_name: entry.source_name.clone(),
                content_hash: entry.content_hash.clone(),
                registry_id,
                version,
            },
        );
    }

    Lockfile {
        version: 1,
        skills: entries,
    }
}

/// Load an existing lockfile from the library directory.
///
/// Returns `None` if the file doesn't exist (first run). Errors on corrupt JSON.
pub fn load(library_dir: &Path) -> Result<Option<Lockfile>> {
    let path = library_dir.join(LOCKFILE_NAME);
    if !path.exists() {
        return Ok(None);
    }
    let content = std::fs::read_to_string(&path)
        .with_context(|| format!("failed to read lockfile {}", path.display()))?;
    let lockfile: Lockfile = serde_json::from_str(&content)
        .with_context(|| format!("failed to parse lockfile {}", path.display()))?;
    Ok(Some(lockfile))
}

/// Write the lockfile to the library directory.
pub fn save(lockfile: &Lockfile, library_dir: &Path) -> Result<()> {
    let path = library_dir.join(LOCKFILE_NAME);
    let content = serde_json::to_string_pretty(lockfile).context("failed to serialize lockfile")?;
    // Add trailing newline for POSIX compliance
    let content = format!("{content}\n");
    std::fs::write(&path, &content)
        .with_context(|| format!("failed to write lockfile {}", path.display()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::discover::{SkillName, SkillProvenance};
    use crate::manifest::SkillEntry;
    use std::path::PathBuf;
    use tempfile::TempDir;

    fn make_manifest(entries: &[(&str, &str, &str, bool)]) -> Manifest {
        let mut manifest = Manifest::default();
        for &(name, source, hash, managed) in entries {
            manifest.insert(
                SkillName::new(name).unwrap(),
                SkillEntry::new(
                    PathBuf::from(format!("/tmp/{name}")),
                    source.to_string(),
                    hash.to_string(),
                    managed,
                ),
            );
        }
        manifest
    }

    fn make_discovered(
        name: &str,
        source: &str,
        provenance: Option<(&str, &str)>,
    ) -> DiscoveredSkill {
        DiscoveredSkill {
            name: SkillName::new(name).unwrap(),
            path: PathBuf::from(format!("/tmp/{name}")),
            source_name: source.to_string(),
            managed: provenance.is_some(),
            provenance: provenance.map(|(reg, ver)| SkillProvenance {
                registry_id: reg.to_string(),
                version: ver.to_string(),
            }),
        }
    }

    #[test]
    fn generate_local_skill_no_provenance() {
        let manifest = make_manifest(&[("my-skill", "standalone", "abc123", false)]);
        let skills = vec![make_discovered("my-skill", "standalone", None)];

        let lockfile = generate(&manifest, &skills);
        assert_eq!(lockfile.version, 1);
        assert_eq!(lockfile.skills.len(), 1);

        let entry = &lockfile.skills["my-skill"];
        assert_eq!(entry.source_name, "standalone");
        assert_eq!(entry.content_hash, "abc123");
        assert!(entry.registry_id.is_none());
        assert!(entry.version.is_none());
    }

    #[test]
    fn generate_managed_skill_with_provenance() {
        let manifest = make_manifest(&[("swift-format", "claude-plugins", "def456", true)]);
        let skills = vec![make_discovered(
            "swift-format",
            "claude-plugins",
            Some(("swift-format@npm", "1.2.0")),
        )];

        let lockfile = generate(&manifest, &skills);
        let entry = &lockfile.skills["swift-format"];
        assert_eq!(entry.registry_id.as_deref(), Some("swift-format@npm"));
        assert_eq!(entry.version.as_deref(), Some("1.2.0"));
    }

    #[test]
    fn generate_mixed_skills() {
        let manifest = make_manifest(&[
            ("local-skill", "standalone", "aaa", false),
            ("managed-skill", "plugins", "bbb", true),
        ]);
        let skills = vec![
            make_discovered("local-skill", "standalone", None),
            make_discovered("managed-skill", "plugins", Some(("pkg@npm", "2.0.0"))),
        ];

        let lockfile = generate(&manifest, &skills);
        assert_eq!(lockfile.skills.len(), 2);
        assert!(lockfile.skills["local-skill"].registry_id.is_none());
        assert_eq!(
            lockfile.skills["managed-skill"].registry_id.as_deref(),
            Some("pkg@npm")
        );
    }

    #[test]
    fn roundtrip_serialization() {
        let manifest = make_manifest(&[
            ("local", "src", "hash1", false),
            ("managed", "plugins", "hash2", true),
        ]);
        let skills = vec![
            make_discovered("local", "src", None),
            make_discovered("managed", "plugins", Some(("pkg@npm", "3.0.0"))),
        ];

        let lockfile = generate(&manifest, &skills);
        let json = serde_json::to_string_pretty(&lockfile).unwrap();
        let parsed: Lockfile = serde_json::from_str(&json).unwrap();
        assert_eq!(lockfile, parsed);
    }

    #[test]
    fn save_creates_file() {
        let tmp = TempDir::new().unwrap();
        let lockfile = Lockfile {
            version: 1,
            skills: BTreeMap::new(),
        };

        save(&lockfile, tmp.path()).unwrap();
        assert!(tmp.path().join("tome.lock").exists());

        let content = std::fs::read_to_string(tmp.path().join("tome.lock")).unwrap();
        assert!(content.contains("\"version\": 1"));
        assert!(content.ends_with('\n'));
    }

    #[test]
    fn load_missing_file_returns_none() {
        let tmp = TempDir::new().unwrap();
        let result = load(tmp.path()).unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn load_valid_file_returns_some() {
        let tmp = TempDir::new().unwrap();
        let lockfile = Lockfile {
            version: 1,
            skills: BTreeMap::from([(
                "my-skill".to_string(),
                LockEntry {
                    source_name: "test".to_string(),
                    content_hash: "abc123".to_string(),
                    registry_id: None,
                    version: None,
                },
            )]),
        };
        save(&lockfile, tmp.path()).unwrap();

        let loaded = load(tmp.path()).unwrap().expect("should be Some");
        assert_eq!(loaded, lockfile);
    }

    #[test]
    fn load_corrupt_file_returns_error() {
        let tmp = TempDir::new().unwrap();
        std::fs::write(tmp.path().join("tome.lock"), "not valid json {{{").unwrap();
        let result = load(tmp.path());
        assert!(result.is_err());
    }

    #[test]
    fn deterministic_output() {
        let manifest = make_manifest(&[
            ("z-skill", "src", "hash_z", false),
            ("a-skill", "src", "hash_a", false),
            ("m-skill", "plugins", "hash_m", true),
        ]);
        let skills = vec![
            make_discovered("z-skill", "src", None),
            make_discovered("a-skill", "src", None),
            make_discovered("m-skill", "plugins", Some(("m@npm", "1.0.0"))),
        ];

        let json1 = serde_json::to_string_pretty(&generate(&manifest, &skills)).unwrap();
        let json2 = serde_json::to_string_pretty(&generate(&manifest, &skills)).unwrap();
        assert_eq!(json1, json2);

        // BTreeMap guarantees alphabetical order
        let lockfile = generate(&manifest, &skills);
        let keys: Vec<&String> = lockfile.skills.keys().collect();
        assert_eq!(keys, vec!["a-skill", "m-skill", "z-skill"]);
    }

    #[test]
    fn generate_manifest_entry_without_discovered_skill() {
        let manifest = make_manifest(&[
            ("a-skill", "src", "hash_a", false),
            ("b-skill", "src", "hash_b", false),
        ]);
        let skills = vec![make_discovered("a-skill", "src", None)];

        let lockfile = generate(&manifest, &skills);
        assert_eq!(lockfile.skills.len(), 2);

        let a = &lockfile.skills["a-skill"];
        assert_eq!(a.source_name, "src");
        assert_eq!(a.content_hash, "hash_a");

        let b = &lockfile.skills["b-skill"];
        assert_eq!(b.source_name, "src");
        assert_eq!(b.content_hash, "hash_b");
        assert!(b.registry_id.is_none());
        assert!(b.version.is_none());
    }

    #[test]
    fn generate_empty_manifest() {
        let manifest = Manifest::default();
        let skills: Vec<DiscoveredSkill> = vec![];

        let lockfile = generate(&manifest, &skills);
        assert_eq!(lockfile.version, 1);
        assert!(lockfile.skills.is_empty());
    }

    #[test]
    fn generate_discovered_skill_not_in_manifest() {
        let manifest = make_manifest(&[("a-skill", "src", "hash_a", false)]);
        let skills = vec![
            make_discovered("a-skill", "src", None),
            make_discovered("extra-skill", "src", None),
        ];

        let lockfile = generate(&manifest, &skills);
        assert_eq!(lockfile.skills.len(), 1);
        assert!(lockfile.skills.contains_key("a-skill"));
        assert!(
            !lockfile.skills.contains_key("extra-skill"),
            "skills not in manifest should not appear in lockfile"
        );
    }

    #[test]
    fn empty_version_string_becomes_none() {
        let manifest = make_manifest(&[("my-plugin", "claude-plugins", "abc123", true)]);
        let skills = vec![make_discovered(
            "my-plugin",
            "claude-plugins",
            Some(("my-plugin@npm", "")),
        )];

        let lockfile = generate(&manifest, &skills);
        let entry = &lockfile.skills["my-plugin"];
        assert_eq!(entry.registry_id.as_deref(), Some("my-plugin@npm"));
        assert!(
            entry.version.is_none(),
            "empty version string should become None, got: {:?}",
            entry.version
        );

        // Verify the version field is omitted from serialized JSON
        let json = serde_json::to_string_pretty(&lockfile).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
        let skill = &parsed["skills"]["my-plugin"];
        assert!(
            skill.get("version").is_none(),
            "empty version should be omitted from JSON"
        );
    }

    #[test]
    fn local_skill_omits_registry_fields_in_json() {
        let manifest = make_manifest(&[("my-skill", "src", "hash1", false)]);
        let skills = vec![make_discovered("my-skill", "src", None)];

        let lockfile = generate(&manifest, &skills);
        let json = serde_json::to_string_pretty(&lockfile).unwrap();
        assert!(
            !json.contains("registry_id"),
            "should omit null registry_id"
        );
        // Check the skill entry doesn't contain a "version" key.
        // The top-level "version": 1 is expected, so we check within the skill object.
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
        let skill = &parsed["skills"]["my-skill"];
        assert!(
            skill.get("registry_id").is_none(),
            "should omit null registry_id in JSON"
        );
        assert!(
            skill.get("version").is_none(),
            "should omit null version in JSON"
        );
    }
}
