//! Lockfile for reproducible skill libraries.
//!
//! The `tome.lock` file captures provenance metadata (directory name, content hash,
//! registry ID + version for managed plugins) for every skill in the library.
//! It is regenerated after every `tome sync` and is meant to be committed to version control.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

use crate::config::{Config, DirectoryName, DirectoryType};
use crate::discover::{DiscoveredSkill, SkillName};
use crate::manifest::Manifest;
use crate::paths::TomePaths;
use crate::validation::ContentHash;

pub(crate) const LOCKFILE_NAME: &str = "tome.lock";

/// Top-level lockfile structure.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Lockfile {
    /// Schema version (currently 1).
    pub(crate) version: u32,
    /// One entry per skill, keyed by skill name.
    pub(crate) skills: BTreeMap<SkillName, LockEntry>,
}

impl Lockfile {
    /// Lockfile schema version.
    ///
    /// Mirrors the accessor surface of `Manifest::skills()` for consistency
    /// across the two file-backed registries. Per HARD-06, the underlying
    /// fields are `pub(crate)` so external crates (including the v1.0 GUI
    /// Tauri IPC) interact with `Lockfile` only via these methods.
    #[allow(dead_code)] // External-facing accessor for v1.0 GUI consumers
    pub fn version(&self) -> u32 {
        self.version
    }

    /// Per-skill entries indexed by skill name (alphabetical).
    #[allow(dead_code)] // External-facing accessor for v1.0 GUI consumers
    pub fn skills(&self) -> &BTreeMap<SkillName, LockEntry> {
        &self.skills
    }
}

/// A single skill entry in the lockfile.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct LockEntry {
    /// Directory name (maps to a `[directories.*]` entry in `tome.toml`), or
    /// `None` if the skill is **Unowned** (source removed from `tome.toml`,
    /// library copy preserved per LIB-04).
    ///
    /// Mirrors `SkillEntry.source_name` (D-12/D-14): old lockfiles with
    /// `"source_name": "foo"` parse as `Some(DirectoryName::new("foo")?)`;
    /// Unowned entries omit the key on serialize.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_name: Option<DirectoryName>,
    /// Last directory that owned this skill before transition to Unowned.
    /// Mirrors `SkillEntry.previous_source` (D-C1) for cross-machine
    /// surfacing in `tome status` / `tome doctor` Unowned section.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub previous_source: Option<DirectoryName>,
    /// SHA-256 content hash of the skill directory.
    pub content_hash: ContentHash,
    /// Registry identifier (e.g. "my-plugin@npm"). Present for managed plugins.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub registry_id: Option<String>,
    /// Version string (e.g. "1.2.0"). Present for managed plugins.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
    /// Git commit SHA for exact version pinning. Present for managed plugins.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub git_commit_sha: Option<String>,
}

/// Generate a **prospective** lockfile by re-hashing every discovered skill
/// on disk.
///
/// Unlike [`generate`], which copies `content_hash` from the manifest (correct
/// after a completed sync when the manifest hashes are current), this function
/// re-hashes each skill's source directory fresh — so the result reflects the
/// *current* on-disk state, not the last-synced state.
///
/// Used by `get_lockfile_diff` in the Tauri GUI to compute a true before/after
/// diff against the on-disk lockfile without requiring a full sync first.
/// Hashing errors for individual skills are propagated immediately (no silent
/// skipping), so a partially-hashed prospective lockfile is never returned.
pub fn generate_prospective(skills: &[DiscoveredSkill]) -> anyhow::Result<Lockfile> {
    let mut entries = BTreeMap::new();

    for skill in skills {
        let content_hash = crate::manifest::hash_directory(&skill.path).with_context(|| {
            format!(
                "failed to hash skill '{}' at {}",
                skill.name,
                skill.path.display()
            )
        })?;

        let (registry_id, version, git_commit_sha) = skill
            .origin
            .provenance()
            .map(|p| {
                (
                    Some(p.registry_id.clone()),
                    p.version.clone(),
                    p.git_commit_sha.clone(),
                )
            })
            .unwrap_or((None, None, None));

        entries.insert(
            skill.name.clone(),
            LockEntry {
                source_name: Some(skill.source_name.clone()),
                previous_source: None,
                content_hash,
                registry_id,
                version,
                git_commit_sha,
            },
        );
    }

    Ok(Lockfile {
        version: 1,
        skills: entries,
    })
}

/// Generate a lockfile from the manifest and discovered skills.
///
/// For each manifest entry, looks up the matching `DiscoveredSkill` to extract
/// provenance metadata (registry_id, version) when available.
///
/// NOTE: this function copies `content_hash` from the manifest, making it
/// suitable for *regenerating* a lockfile after a completed sync (where the
/// manifest already reflects current on-disk state). For computing a
/// pre-sync diff, use [`generate_prospective`] instead.
pub fn generate(manifest: &Manifest, skills: &[DiscoveredSkill]) -> Lockfile {
    let skill_map: BTreeMap<&str, &DiscoveredSkill> =
        skills.iter().map(|s| (s.name.as_str(), s)).collect();

    let mut entries = BTreeMap::new();

    for (name, entry) in manifest.iter() {
        let (registry_id, version, git_commit_sha) = skill_map
            .get(name.as_str())
            .and_then(|s| s.origin.provenance())
            .map(|p| {
                (
                    Some(p.registry_id.clone()),
                    p.version.clone(),
                    p.git_commit_sha.clone(),
                )
            })
            .unwrap_or((None, None, None));

        entries.insert(
            name.clone(),
            LockEntry {
                source_name: entry.source_name().cloned(),
                previous_source: entry.previous_source().cloned(),
                content_hash: entry.content_hash.clone(),
                registry_id,
                version,
                git_commit_sha,
            },
        );
    }

    Lockfile {
        version: 1,
        skills: entries,
    }
}

/// Build a `resolved_paths` map for [`crate::discover::discover_all`] from the
/// current lockfile and on-disk git cache, with **no network calls**.
///
/// Used by the destructive commands (`tome remove` / `reassign` / `fork`) when
/// they regenerate the lockfile after in-memory mutation. Unlike `sync()`'s
/// online `resolve_git_directories`, this helper:
///
/// - reads the previous lockfile to recover `git_commit_sha` per directory,
/// - checks the on-disk cache dir for each git-type directory,
/// - emits a stderr-bound warning string when the lockfile is missing OR
///   the cache dir is gone (so the caller can `eprintln!("warning: {}", w)`),
/// - never clones, fetches, or talks to a remote.
///
/// Returns `(map, warnings)`. Map values are `(effective_path, Option<git_commit_sha>)`
/// in the shape `discover::discover_all` requires.
#[allow(clippy::type_complexity)] // (BTreeMap, Vec<String>) — matches discover_all's input shape
pub(crate) fn resolved_paths_from_lockfile_cache(
    config: &Config,
    paths: &TomePaths,
) -> (
    BTreeMap<DirectoryName, (PathBuf, Option<String>)>,
    Vec<String>,
) {
    let mut resolved = BTreeMap::new();
    let mut warnings = Vec::new();

    // Bail early if config has no git directories — avoid loading the lockfile.
    let has_git = config
        .directories()
        .values()
        .any(|d| d.directory_type == DirectoryType::Git);
    if !has_git {
        return (resolved, warnings);
    }

    // Load the previous lockfile (offline). On read error, surface it and
    // proceed as if it were missing.
    let previous = match load(paths.config_dir()) {
        Ok(opt) => opt,
        Err(e) => {
            warnings.push(format!(
                "could not read lockfile for git-directory cache lookup: {e}"
            ));
            None
        }
    };

    // Build a quick "directory_name -> first git_commit_sha seen" index from
    // the lockfile. All skills sourced from one directory share a SHA, so any
    // one is correct.
    let mut sha_by_dir: std::collections::BTreeMap<&str, Option<String>> =
        std::collections::BTreeMap::new();
    if let Some(ref lf) = previous {
        for entry in lf.skills.values() {
            if let Some(source) = &entry.source_name {
                sha_by_dir
                    .entry(source.as_str())
                    .or_insert_with(|| entry.git_commit_sha.clone());
            }
            // Unowned entries (source_name == None) are skipped — they have no
            // directory in the current config to resolve against.
        }
    }

    let repos_dir = paths.repos_dir();

    for (name, dir_config) in config.directories() {
        if dir_config.directory_type != DirectoryType::Git {
            continue;
        }

        // The lockfile is our only offline source of `git_commit_sha`. If it's
        // missing entirely, surface a per-directory warning so the user is
        // not left in the dark — replaces the silent drop in #461 H1.
        if previous.is_none() {
            warnings.push(format!(
                "cannot resolve git directory '{name}' from lockfile (lockfile missing) — \
                 git-sourced skills may be omitted from the regenerated lockfile",
            ));
            continue;
        }

        let url = dir_config.path.to_string_lossy();
        let cache_dir = crate::git::repo_cache_dir(&repos_dir, &url);
        if !cache_dir.is_dir() {
            warnings.push(format!(
                "cannot resolve git directory '{name}' — cache dir {} not found; \
                 git-sourced skills will be omitted from the regenerated lockfile",
                cache_dir.display(),
            ));
            continue;
        }

        let effective = crate::git::effective_path(&cache_dir, dir_config.subdir.as_deref());
        // sha may legitimately be None when the directory has no skills yet.
        let sha = sha_by_dir.get(name.as_str()).and_then(|opt| opt.clone());
        resolved.insert(name.clone(), (effective, sha));
    }

    (resolved, warnings)
}

/// Load an existing lockfile from the tome home directory.
///
/// Returns `None` if the file doesn't exist (first run). Errors on corrupt JSON.
pub fn load(tome_home: &Path) -> Result<Option<Lockfile>> {
    let path = tome_home.join(LOCKFILE_NAME);
    if !path.exists() {
        return Ok(None);
    }
    let content = std::fs::read_to_string(&path)
        .with_context(|| format!("failed to read lockfile {}", path.display()))?;
    let lockfile: Lockfile = serde_json::from_str(&content)
        .with_context(|| format!("failed to parse lockfile {}", path.display()))?;
    Ok(Some(lockfile))
}

/// Write the lockfile to the tome home directory using atomic temp+rename.
pub fn save(lockfile: &Lockfile, tome_home: &Path) -> Result<()> {
    let path = tome_home.join(LOCKFILE_NAME);
    let tmp_path = tome_home.join("tome.lock.tmp");
    let content = serde_json::to_string_pretty(lockfile).context("failed to serialize lockfile")?;
    // Add trailing newline for POSIX compliance
    let content = format!("{content}\n");
    std::fs::write(&tmp_path, &content)
        .with_context(|| format!("failed to write temp lockfile {}", tmp_path.display()))?;
    if let Err(e) = std::fs::rename(&tmp_path, &path) {
        // Best-effort cleanup so a stale `tome.lock.tmp` doesn't accumulate
        // after a failed save. Ignore the cleanup result on purpose: the
        // rename error is the real failure; masking it with a cleanup
        // error would hide the actual cause.
        let _ = std::fs::remove_file(&tmp_path);
        return Err(e).with_context(|| format!("failed to rename lockfile {}", path.display()));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::discover::SkillName;
    use crate::manifest::SkillEntry;
    use crate::validation::test_hash;
    use std::path::PathBuf;
    use tempfile::TempDir;

    fn make_manifest(entries: &[(&str, &str, &str, bool)]) -> Manifest {
        let mut manifest = Manifest::default();
        for &(name, source, hash_seed, managed) in entries {
            manifest.insert(
                SkillName::new(name).unwrap(),
                SkillEntry::new(
                    PathBuf::from(format!("/tmp/{name}")),
                    DirectoryName::new(source).unwrap(),
                    test_hash(hash_seed),
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
        use crate::discover::{SkillOrigin, SkillProvenance};
        let origin = match provenance {
            Some((reg, ver)) => SkillOrigin::Managed {
                provenance: Some(SkillProvenance {
                    registry_id: reg.to_string(),
                    version: if ver.is_empty() {
                        None
                    } else {
                        Some(ver.to_string())
                    },
                    git_commit_sha: None,
                }),
            },
            None => SkillOrigin::Local,
        };
        DiscoveredSkill {
            name: SkillName::new(name).unwrap(),
            path: PathBuf::from(format!("/tmp/{name}")),
            source_name: DirectoryName::new(source).unwrap(),
            origin,
            frontmatter: None,
            synced_at: None,
        }
    }

    #[test]
    fn generate_local_skill_no_provenance() {
        let manifest = make_manifest(&[("my-skill", "standalone", "abc123", false)]);
        let skills = vec![make_discovered("my-skill", "standalone", None)];

        let lockfile = generate(&manifest, &skills);
        assert_eq!(lockfile.version, 1);
        assert_eq!(lockfile.skills.len(), 1);

        let key = SkillName::new("my-skill").unwrap();
        let entry = &lockfile.skills[&key];
        assert_eq!(
            entry.source_name.as_ref().map(|d| d.as_str()),
            Some("standalone")
        );
        assert_eq!(entry.content_hash, test_hash("abc123"));
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
        let key = SkillName::new("swift-format").unwrap();
        let entry = &lockfile.skills[&key];
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
        let local_key = SkillName::new("local-skill").unwrap();
        let managed_key = SkillName::new("managed-skill").unwrap();
        assert!(lockfile.skills[&local_key].registry_id.is_none());
        assert_eq!(
            lockfile.skills[&managed_key].registry_id.as_deref(),
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
    fn save_does_not_leave_tmp_file() {
        let tmp = TempDir::new().unwrap();
        let lockfile = Lockfile {
            version: 1,
            skills: BTreeMap::new(),
        };

        save(&lockfile, tmp.path()).unwrap();
        assert!(tmp.path().join("tome.lock").exists());
        assert!(
            !tmp.path().join("tome.lock.tmp").exists(),
            "atomic save should not leave tmp file behind"
        );
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
                SkillName::new("my-skill").unwrap(),
                LockEntry {
                    source_name: Some(DirectoryName::new("test").unwrap()),
                    previous_source: None,
                    content_hash: test_hash("abc123"),
                    registry_id: None,
                    version: None,
                    git_commit_sha: None,
                },
            )]),
        };
        save(&lockfile, tmp.path()).unwrap();

        let loaded = load(tmp.path()).unwrap().expect("should be Some");
        assert_eq!(loaded, lockfile);
    }

    #[test]
    fn load_accepts_unknown_version() {
        // Documents current behavior: Lockfile::load() silently accepts
        // a version number it doesn't know about. The `version` field is
        // deserialized but not validated, so version 999 loads without error.
        let tmp = TempDir::new().unwrap();
        let valid_hash = "a".repeat(64);
        let json = serde_json::json!({
            "version": 999,
            "skills": {
                "some-skill": {
                    "source_name": "test",
                    "content_hash": valid_hash
                }
            }
        });
        std::fs::write(
            tmp.path().join("tome.lock"),
            serde_json::to_string_pretty(&json).unwrap(),
        )
        .unwrap();

        let result = load(tmp.path()).unwrap();
        let lockfile = result.expect("should load successfully despite unknown version");
        assert_eq!(lockfile.version, 999);
        assert_eq!(lockfile.skills.len(), 1);
        let key = SkillName::new("some-skill").unwrap();
        assert!(lockfile.skills.contains_key(&key));
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
        let keys: Vec<&str> = lockfile.skills.keys().map(|k| k.as_str()).collect();
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

        let a_key = SkillName::new("a-skill").unwrap();
        let a = &lockfile.skills[&a_key];
        assert_eq!(a.source_name.as_ref().map(|d| d.as_str()), Some("src"));
        assert_eq!(a.content_hash, test_hash("hash_a"));

        let b_key = SkillName::new("b-skill").unwrap();
        let b = &lockfile.skills[&b_key];
        assert_eq!(b.source_name.as_ref().map(|d| d.as_str()), Some("src"));
        assert_eq!(b.content_hash, test_hash("hash_b"));
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
        let a_key = SkillName::new("a-skill").unwrap();
        let extra_key = SkillName::new("extra-skill").unwrap();
        assert!(lockfile.skills.contains_key(&a_key));
        assert!(
            !lockfile.skills.contains_key(&extra_key),
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
        let key = SkillName::new("my-plugin").unwrap();
        let entry = &lockfile.skills[&key];
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

    // -- resolved_paths_from_lockfile_cache tests (HOTFIX-01 / #461 H1) --

    use crate::config::{Config, DirectoryConfig, DirectoryName, DirectoryRole, DirectoryType};
    use crate::paths::TomePaths;

    /// Build a minimal `DirectoryConfig` for a git-type directory.
    fn git_dir_config(url: &str, subdir: Option<&str>) -> DirectoryConfig {
        // Round-trip through TOML so we set the `pub(crate) role` field via
        // serde — avoids depending on field visibility from this module.
        let toml_src = format!(
            "path = \"{}\"\ntype = \"git\"\nrole = \"source\"\n{}",
            url,
            match subdir {
                Some(s) => format!("subdir = \"{}\"\n", s),
                None => String::new(),
            },
        );
        toml::from_str(&toml_src).expect("valid DirectoryConfig TOML")
    }

    /// Build a minimal `DirectoryConfig` for a directory-type directory.
    fn dir_dir_config(path: &Path) -> DirectoryConfig {
        let toml_src = format!(
            "path = \"{}\"\ntype = \"directory\"\nrole = \"source\"\n",
            path.display(),
        );
        toml::from_str(&toml_src).expect("valid DirectoryConfig TOML")
    }

    /// Build a `Config` from a list of `(name, DirectoryConfig)` pairs and a library_dir.
    fn config_with_dirs(
        library_dir: &std::path::Path,
        dirs: Vec<(&str, DirectoryConfig)>,
    ) -> Config {
        let mut config_toml = format!("library_dir = \"{}\"\n", library_dir.display());
        for (name, dc) in &dirs {
            config_toml.push_str(&format!(
                "\n[directories.{name}]\npath = \"{}\"\ntype = \"{}\"\nrole = \"{}\"\n",
                dc.path.display(),
                match dc.directory_type {
                    DirectoryType::Git => "git",
                    DirectoryType::Directory => "directory",
                    DirectoryType::ClaudePlugins => "claude-plugins",
                },
                match dc.role() {
                    DirectoryRole::Source => "source",
                    DirectoryRole::Synced => "synced",
                    DirectoryRole::Managed => "managed",
                    DirectoryRole::Target => "target",
                },
            ));
            if let Some(s) = dc.subdir.as_deref() {
                config_toml.push_str(&format!("subdir = \"{s}\"\n"));
            }
        }
        toml::from_str(&config_toml).expect("valid Config TOML")
    }

    /// Build `TomePaths` rooted at `tmp` with a `skills` library dir.
    fn paths_for(tmp: &Path) -> TomePaths {
        let library_dir = tmp.join("skills");
        std::fs::create_dir_all(&library_dir).unwrap();
        TomePaths::new(tmp.to_path_buf(), library_dir).unwrap()
    }

    /// Write a minimal lockfile to `tome_home` containing a single skill
    /// whose `source_name = source` and `git_commit_sha = sha`.
    fn write_lockfile(tome_home: &Path, source: &str, sha: Option<&str>) {
        let mut skills = BTreeMap::new();
        skills.insert(
            SkillName::new("seed-skill").unwrap(),
            LockEntry {
                source_name: Some(DirectoryName::new(source).unwrap()),
                previous_source: None,
                content_hash: test_hash("seed"),
                registry_id: None,
                version: None,
                git_commit_sha: sha.map(|s| s.to_string()),
            },
        );
        let lf = Lockfile { version: 1, skills };
        save(&lf, tome_home).unwrap();
    }

    /// Write an empty (no skills) lockfile so `load()` returns `Some` but
    /// `sha_by_dir` is empty — used to test the no-skills-for-dir fallback.
    fn write_empty_lockfile(tome_home: &Path) {
        let lf = Lockfile {
            version: 1,
            skills: BTreeMap::new(),
        };
        save(&lf, tome_home).unwrap();
    }

    #[test]
    fn resolved_paths_from_lockfile_cache_returns_empty_when_no_git_dirs() {
        let tmp = TempDir::new().unwrap();
        let paths = paths_for(tmp.path());

        let local_path = tmp.path().join("local-skills");
        std::fs::create_dir_all(&local_path).unwrap();
        let config = config_with_dirs(
            paths.library_dir(),
            vec![("local", dir_dir_config(&local_path))],
        );

        let (map, warnings) = resolved_paths_from_lockfile_cache(&config, &paths);
        assert!(map.is_empty(), "no git dirs → empty map, got {map:?}");
        assert!(
            warnings.is_empty(),
            "no git dirs → no warnings, got {warnings:?}"
        );
    }

    #[test]
    fn resolved_paths_from_lockfile_cache_warns_when_lockfile_missing() {
        let tmp = TempDir::new().unwrap();
        let paths = paths_for(tmp.path());

        // No tome.lock written. One git directory in config.
        let config = config_with_dirs(
            paths.library_dir(),
            vec![(
                "myrepo",
                git_dir_config("https://example.invalid/foo.git", None),
            )],
        );

        let (map, warnings) = resolved_paths_from_lockfile_cache(&config, &paths);
        assert!(
            map.is_empty(),
            "lockfile missing → entry omitted, got {map:?}"
        );
        assert_eq!(warnings.len(), 1, "expected one warning, got {warnings:?}");
        let w = &warnings[0];
        assert!(
            w.contains("cannot resolve git directory 'myrepo'"),
            "warning should name the directory: {w}"
        );
        assert!(
            w.contains("lockfile"),
            "warning should mention 'lockfile': {w}"
        );
    }

    #[test]
    fn resolved_paths_from_lockfile_cache_warns_when_cache_dir_missing() {
        let tmp = TempDir::new().unwrap();
        let paths = paths_for(tmp.path());

        // Lockfile present (with a sha for "myrepo") but no cache dir on disk.
        write_lockfile(paths.config_dir(), "myrepo", Some("deadbeef"));

        let config = config_with_dirs(
            paths.library_dir(),
            vec![(
                "myrepo",
                git_dir_config("https://example.invalid/foo.git", None),
            )],
        );

        let (map, warnings) = resolved_paths_from_lockfile_cache(&config, &paths);
        assert!(
            map.is_empty(),
            "cache dir missing → entry omitted, got {map:?}"
        );
        assert_eq!(warnings.len(), 1, "expected one warning, got {warnings:?}");
        let w = &warnings[0];
        assert!(
            w.contains("cannot resolve git directory 'myrepo'"),
            "warning should name the directory: {w}"
        );
        assert!(w.contains("cache"), "warning should mention 'cache': {w}");
    }

    #[test]
    fn resolved_paths_from_lockfile_cache_populates_when_cache_exists() {
        let tmp = TempDir::new().unwrap();
        let paths = paths_for(tmp.path());

        let url = "https://example.invalid/foo.git";
        write_lockfile(paths.config_dir(), "myrepo", Some("deadbeef00"));

        // Create the cache dir at repos_dir/<sha256(url)>/.
        let cache_dir = crate::git::repo_cache_dir(&paths.repos_dir(), url);
        std::fs::create_dir_all(&cache_dir).unwrap();

        let config = config_with_dirs(
            paths.library_dir(),
            vec![("myrepo", git_dir_config(url, None))],
        );

        let (map, warnings) = resolved_paths_from_lockfile_cache(&config, &paths);
        assert!(
            warnings.is_empty(),
            "cache present → no warnings, got {warnings:?}"
        );
        assert_eq!(map.len(), 1, "expected one entry, got {map:?}");

        let key = DirectoryName::new("myrepo").unwrap();
        let (path, sha) = map.get(&key).expect("myrepo entry");
        assert_eq!(path, &cache_dir);
        assert_eq!(sha.as_deref(), Some("deadbeef00"));
    }

    #[test]
    fn resolved_paths_from_lockfile_cache_respects_subdir() {
        let tmp = TempDir::new().unwrap();
        let paths = paths_for(tmp.path());

        let url = "https://example.invalid/foo.git";
        write_lockfile(paths.config_dir(), "myrepo", Some("cafebabe"));

        let cache_dir = crate::git::repo_cache_dir(&paths.repos_dir(), url);
        std::fs::create_dir_all(&cache_dir).unwrap();

        let config = config_with_dirs(
            paths.library_dir(),
            vec![("myrepo", git_dir_config(url, Some("skills")))],
        );

        let (map, warnings) = resolved_paths_from_lockfile_cache(&config, &paths);
        assert!(
            warnings.is_empty(),
            "no warnings expected, got {warnings:?}"
        );
        let key = DirectoryName::new("myrepo").unwrap();
        let (path, _) = map.get(&key).expect("myrepo entry");
        assert_eq!(path, &cache_dir.join("skills"));
    }

    #[test]
    fn resolved_paths_from_lockfile_cache_falls_back_when_no_lockfile_sha() {
        let tmp = TempDir::new().unwrap();
        let paths = paths_for(tmp.path());

        let url = "https://example.invalid/foo.git";
        // Lockfile present but no entry references "myrepo" — the directory
        // has no skills yet. Helper should still resolve via cache (sha = None)
        // so the regen does not silently drop the directory.
        write_empty_lockfile(paths.config_dir());

        let cache_dir = crate::git::repo_cache_dir(&paths.repos_dir(), url);
        std::fs::create_dir_all(&cache_dir).unwrap();

        let config = config_with_dirs(
            paths.library_dir(),
            vec![("myrepo", git_dir_config(url, None))],
        );

        let (map, warnings) = resolved_paths_from_lockfile_cache(&config, &paths);
        assert!(
            warnings.is_empty(),
            "no warnings expected, got {warnings:?}"
        );
        let key = DirectoryName::new("myrepo").unwrap();
        let (path, sha) = map.get(&key).expect("myrepo entry");
        assert_eq!(path, &cache_dir);
        assert!(
            sha.is_none(),
            "sha should be None when lockfile has no entry, got {sha:?}"
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
        let my_skill = &parsed["skills"]["my-skill"];
        assert!(
            my_skill.get("registry_id").is_none(),
            "should omit null registry_id in JSON"
        );
        assert!(
            my_skill.get("version").is_none(),
            "should omit null version in JSON"
        );
    }

    #[test]
    fn deserialize_old_shape_lockfile_source_name_string() {
        let valid_hash = "a".repeat(64);
        let json = format!(r#"{{"source_name":"foo","content_hash":"{valid_hash}"}}"#);
        let entry: LockEntry = serde_json::from_str(&json).unwrap();
        assert_eq!(entry.source_name, Some(DirectoryName::new("foo").unwrap()));
    }

    #[test]
    fn deserialize_new_shape_lockfile_null_source_name() {
        let valid_hash = "a".repeat(64);
        let json = format!(r#"{{"source_name":null,"content_hash":"{valid_hash}"}}"#);
        let entry: LockEntry = serde_json::from_str(&json).unwrap();
        assert_eq!(entry.source_name, None);
    }

    #[test]
    fn deserialize_new_shape_lockfile_missing_source_name() {
        let valid_hash = "a".repeat(64);
        let json = format!(r#"{{"content_hash":"{valid_hash}"}}"#);
        let entry: LockEntry = serde_json::from_str(&json).unwrap();
        assert_eq!(entry.source_name, None);
    }

    #[test]
    fn unowned_skill_omits_source_name_in_lockfile_json() {
        use crate::manifest::SkillEntry;
        let mut manifest = Manifest::default();
        manifest.insert(
            SkillName::new("orphan").unwrap(),
            SkillEntry::new_unowned(PathBuf::from("/tmp/orphan"), test_hash("h"), false, None),
        );
        let lockfile = generate(&manifest, &[]);
        let json = serde_json::to_string_pretty(&lockfile).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
        let orphan = &parsed["skills"]["orphan"];
        assert!(
            orphan.get("source_name").is_none(),
            "Unowned skill must omit source_name in lockfile JSON, got: {json}"
        );
    }

    #[test]
    fn lockentry_round_trip_with_previous_source() {
        use crate::manifest::SkillEntry;
        let mut manifest = Manifest::default();
        manifest.insert(
            SkillName::new("orphan").unwrap(),
            SkillEntry::new_unowned(
                PathBuf::from("/tmp/orphan"),
                test_hash("h"),
                false,
                Some(DirectoryName::new("old-source").unwrap()),
            ),
        );
        let lf = generate(&manifest, &[]);
        let key = SkillName::new("orphan").unwrap();
        assert_eq!(
            lf.skills[&key].previous_source,
            Some(DirectoryName::new("old-source").unwrap()),
            "generate() must copy previous_source from manifest entry"
        );

        // Round-trip through JSON.
        let json = serde_json::to_string_pretty(&lf).unwrap();
        let parsed: Lockfile = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, lf);
    }

    #[test]
    fn deserialize_old_shape_lockfile_without_previous_source() {
        let valid_hash = "a".repeat(64);
        let json = format!(r#"{{"source_name":"foo","content_hash":"{valid_hash}"}}"#);
        let entry: LockEntry = serde_json::from_str(&json).unwrap();
        assert_eq!(entry.previous_source, None);
    }

    // -- HARD-06: Lockfile accessor parity tests --
    //
    // Lockfile.version and Lockfile.skills are pub(crate); external callers
    // (and the v1.0 GUI Tauri IPC) must use Lockfile::version() and
    // Lockfile::skills() accessors. These tests pin parity between accessor
    // output and the underlying field shape.

    #[test]
    fn lockfile_version_accessor_returns_field() {
        let lf = Lockfile {
            version: 1,
            skills: BTreeMap::new(),
        };
        assert_eq!(lf.version(), 1);
    }

    #[test]
    fn lockfile_skills_accessor_returns_full_map() {
        let mut skills = BTreeMap::new();
        skills.insert(
            SkillName::new("alpha").unwrap(),
            LockEntry {
                source_name: Some(DirectoryName::new("src").unwrap()),
                previous_source: None,
                content_hash: test_hash("a"),
                registry_id: None,
                version: None,
                git_commit_sha: None,
            },
        );
        skills.insert(
            SkillName::new("bravo").unwrap(),
            LockEntry {
                source_name: Some(DirectoryName::new("src").unwrap()),
                previous_source: None,
                content_hash: test_hash("b"),
                registry_id: None,
                version: None,
                git_commit_sha: None,
            },
        );
        let lf = Lockfile { version: 1, skills };
        let via_accessor = lf.skills();
        assert_eq!(via_accessor.len(), 2);
        assert!(via_accessor.contains_key(&SkillName::new("alpha").unwrap()));
        assert!(via_accessor.contains_key(&SkillName::new("bravo").unwrap()));
    }

    /// HARD-08: rename failure during atomic save must leave the previous
    /// `tome.lock` content untouched. Phase 13 D-22 makes the lockfile the
    /// authoritative cross-machine state — corrupting it on a partial save
    /// would break reconciliation.
    ///
    /// Mechanism: chmod 0o500 on the parent dir → fs::rename returns
    /// EACCES → save() returns Err → original file content is unchanged.
    #[cfg(unix)]
    #[test]
    fn save_preserves_previous_on_rename_failure() {
        use crate::manifest::SkillEntry;
        use std::os::unix::fs::PermissionsExt;

        let tmp = TempDir::new().unwrap();

        // Build lockfile A with one entry, save through the canonical
        // happy path so we know the on-disk shape.
        let mut manifest_a = Manifest::default();
        manifest_a.insert(
            SkillName::new("alpha").unwrap(),
            SkillEntry::new(
                PathBuf::from("/tmp/alpha"),
                DirectoryName::new("src-a").unwrap(),
                test_hash("a"),
                false,
            ),
        );
        let lockfile_a = generate(&manifest_a, &[]);
        save(&lockfile_a, tmp.path()).unwrap();
        let lock_path = tmp.path().join(LOCKFILE_NAME);
        let bytes_a = std::fs::read(&lock_path).unwrap();

        // Lock the parent dir so rename will EACCES.
        let original_mode = std::fs::metadata(tmp.path()).unwrap().permissions().mode();
        std::fs::set_permissions(tmp.path(), std::fs::Permissions::from_mode(0o500)).unwrap();

        // Build a different lockfile B and try to save it.
        let mut manifest_b = Manifest::default();
        manifest_b.insert(
            SkillName::new("beta").unwrap(),
            SkillEntry::new(
                PathBuf::from("/tmp/beta"),
                DirectoryName::new("src-b").unwrap(),
                test_hash("b"),
                false,
            ),
        );
        let lockfile_b = generate(&manifest_b, &[]);
        let result = save(&lockfile_b, tmp.path());

        // Restore permissions before any assertion to keep TempDir cleanup
        // working even on assertion panic.
        std::fs::set_permissions(tmp.path(), std::fs::Permissions::from_mode(original_mode))
            .unwrap();

        assert!(
            result.is_err(),
            "save() must fail when the rename target dir is not writable"
        );

        let bytes_after = std::fs::read(&lock_path).unwrap();
        assert_eq!(
            bytes_after, bytes_a,
            "atomic-save invariant violated: lockfile content corrupted by \
             a failed save"
        );

        // Re-load and confirm the surviving entry is the original alpha,
        // not the would-be-saved beta.
        let reloaded = load(tmp.path()).unwrap().expect("lockfile must exist");
        assert!(
            reloaded
                .skills
                .contains_key(&SkillName::new("alpha").unwrap())
        );
        assert!(
            !reloaded
                .skills
                .contains_key(&SkillName::new("beta").unwrap())
        );
    }

    /// HARD-08 round-trip pin: previous_source survives a save -> load
    /// cycle when the entry is Owned (companion to the Unowned-state
    /// `lockentry_round_trip_with_previous_source` above).
    #[test]
    fn previous_source_round_trip_through_lockfile_save_load_owned() {
        use crate::manifest::SkillEntry;

        let tmp = TempDir::new().unwrap();
        let mut manifest = Manifest::default();
        manifest.insert(
            SkillName::new("kept").unwrap(),
            SkillEntry::new(
                PathBuf::from("/tmp/kept"),
                DirectoryName::new("active").unwrap(),
                test_hash("k"),
                false,
            ),
        );
        let lf = generate(&manifest, &[]);
        save(&lf, tmp.path()).unwrap();

        let reloaded = load(tmp.path()).unwrap().expect("lockfile must exist");
        let entry = &reloaded.skills[&SkillName::new("kept").unwrap()];
        // Owned entries serialise without previous_source — the field is
        // None in memory and the JSON omits it via skip_serializing_if.
        assert_eq!(entry.previous_source, None);
        assert_eq!(
            entry.source_name,
            Some(DirectoryName::new("active").unwrap())
        );
    }
}
