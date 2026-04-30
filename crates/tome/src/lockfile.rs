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
#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct Lockfile {
    /// Schema version (currently 1).
    pub version: u32,
    /// One entry per skill, keyed by skill name.
    pub skills: BTreeMap<SkillName, LockEntry>,
}

/// A single skill entry in the lockfile.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct LockEntry {
    /// Directory name (maps to a `[directories.*]` entry in `tome.toml`).
    /// On-disk JSON shape is unchanged (`DirectoryName` is `#[serde(transparent)]`); the
    /// type lift to `DirectoryName` (closes #489) tightens validation at deserialize time.
    pub source_name: DirectoryName,
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

/// Generate a lockfile from the manifest and discovered skills.
///
/// For each manifest entry, looks up the matching `DiscoveredSkill` to extract
/// provenance metadata (registry_id, version) when available.
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
                source_name: entry.source_name.clone(),
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
            sha_by_dir
                .entry(entry.source_name.as_str())
                .or_insert_with(|| entry.git_commit_sha.clone());
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
        assert_eq!(entry.source_name, "standalone");
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
                    source_name: DirectoryName::new("test").unwrap(),
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
        assert_eq!(a.source_name, "src");
        assert_eq!(a.content_hash, test_hash("hash_a"));

        let b_key = SkillName::new("b-skill").unwrap();
        let b = &lockfile.skills[&b_key];
        assert_eq!(b.source_name, "src");
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
                source_name: DirectoryName::new(source).unwrap(),
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
}
