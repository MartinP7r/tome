//! Per-machine preferences for skill management.
//!
//! Each machine can opt out of specific skills or directories via `machine.toml`
//! at `~/.config/tome/machine.toml`. This is intentionally separate from `tome.toml`
//! at `~/.tome/tome.toml` — machine-specific preferences should not live in the portable
//! tome home directory. The library stays complete across machines; disabled skills are
//! simply skipped during distribution.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

use crate::config::DirectoryName;
use crate::discover::SkillName;

/// Per-machine path override for a specific directory.
///
/// Allows a single `tome.toml` checked into dotfiles to be applied across
/// machines with different filesystem layouts. The override is applied at
/// config load time (between `Config::expand_tildes()` and `Config::validate()`)
/// so every downstream command operates on the merged result.
///
/// Schema (v0.9): only `path` is supported. Future versions may add
/// `role`/`type`/`subdir` overrides — track via #458 follow-ups.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DirectoryOverride {
    /// Replaces `directories.<name>.path` on this machine. Tilde-expansion
    /// happens in `Config::apply_machine_overrides`, not here.
    pub path: PathBuf,
}

/// Per-directory skill filtering preferences.
///
/// A directory can have either a `disabled` blocklist OR an `enabled` allowlist,
/// but not both (MACH-04). When `enabled` is set, only those skills are distributed
/// to this directory — it acts as an exclusive allowlist.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DirectoryPrefs {
    /// Skills to exclude from this directory (blocklist).
    #[serde(default)]
    pub(crate) disabled: BTreeSet<SkillName>,

    /// If set, ONLY these skills are distributed to this directory (allowlist).
    /// Mutually exclusive with `disabled` (MACH-04).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) enabled: Option<BTreeSet<SkillName>>,
}

/// Per-machine preferences — disabled skills and directories for this machine.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct MachinePrefs {
    /// Skills that should not be distributed to directories on this machine.
    #[serde(default)]
    pub(crate) disabled: BTreeSet<SkillName>,

    /// Directories to skip on this machine (e.g. machine A doesn't have a certain tool installed).
    #[serde(default)]
    pub(crate) disabled_directories: BTreeSet<DirectoryName>,

    /// Per-directory skill filtering. Keys are directory names from config.
    #[serde(default)]
    pub(crate) directory: BTreeMap<DirectoryName, DirectoryPrefs>,

    /// Per-machine path overrides for entries in `tome.toml::directories`.
    /// Keyed by directory name; only the `path` field is currently supported (PORT-01).
    /// See `Config::apply_machine_overrides` for the apply step.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub(crate) directory_overrides: BTreeMap<DirectoryName, DirectoryOverride>,
}

impl MachinePrefs {
    /// Returns true if the given skill is disabled on this machine.
    pub fn is_disabled(&self, name: &str) -> bool {
        self.disabled.contains(name)
    }

    /// Mark a skill as disabled on this machine.
    pub fn disable(&mut self, name: SkillName) {
        self.disabled.insert(name);
    }

    /// Returns true if the given directory is disabled on this machine.
    pub fn is_directory_disabled(&self, name: &str) -> bool {
        self.disabled_directories.contains(name)
    }

    /// Mark a directory as disabled on this machine.
    #[allow(dead_code)]
    pub fn disable_directory(&mut self, name: DirectoryName) {
        self.disabled_directories.insert(name);
    }

    /// Validate machine preferences.
    ///
    /// Returns an error if any directory has both `disabled` and `enabled` set (MACH-04).
    pub fn validate(&self) -> Result<()> {
        for (name, prefs) in &self.directory {
            if !prefs.disabled.is_empty() && prefs.enabled.is_some() {
                anyhow::bail!(
                    "directory '{}' in machine.toml has both 'disabled' and 'enabled' — use one or the other",
                    name
                );
            }
        }
        Ok(())
    }

    /// Check if a skill should be distributed to a specific directory.
    ///
    /// Resolution follows the locality principle (most specific wins) per D-08:
    /// 1. Per-directory `enabled` (allowlist) — if set, only listed skills pass
    /// 2. Per-directory `disabled` (blocklist) — if skill is listed, it's blocked
    /// 3. Global `disabled` — broad default blocklist
    #[allow(dead_code)] // Wired in Plan 02-03 (distribute.rs integration)
    pub fn is_skill_allowed(&self, skill_name: &str, dir_name: &str) -> bool {
        // Check per-directory preferences first (most specific)
        if let Some(dir_prefs) = self.directory.get(dir_name) {
            // Allowlist is strongest — if set, only listed skills pass
            if let Some(enabled) = &dir_prefs.enabled {
                return enabled.contains(skill_name);
            }
            // Blocklist — skill explicitly blocked for this directory
            if dir_prefs.disabled.contains(skill_name) {
                return false;
            }
        }
        // Fall back to global disabled
        !self.disabled.contains(skill_name)
    }
}

/// Default path for the machine preferences file: `~/.config/tome/machine.toml`.
pub fn default_machine_path() -> Result<PathBuf> {
    Ok(dirs::home_dir()
        .context("could not determine home directory")?
        .join(".config")
        .join("tome")
        .join("machine.toml"))
}

/// Load machine preferences from a TOML file.
///
/// Returns default (empty) prefs if the file doesn't exist. Errors on malformed TOML.
pub fn load(path: &Path) -> Result<MachinePrefs> {
    if !path.exists() {
        return Ok(MachinePrefs::default());
    }
    let content = std::fs::read_to_string(path)
        .with_context(|| format!("failed to read {}", path.display()))?;
    let prefs: MachinePrefs =
        toml::from_str(&content).with_context(|| format!("failed to parse {}", path.display()))?;
    prefs.validate()?;
    Ok(prefs)
}

/// Save machine preferences to a TOML file using atomic temp+rename,
/// creating parent directories as needed.
pub fn save(prefs: &MachinePrefs, path: &Path) -> Result<()> {
    let content = toml::to_string_pretty(prefs).context("failed to serialize machine prefs")?;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("failed to create {}", parent.display()))?;
    }
    let tmp_path = path.with_extension("toml.tmp");
    std::fs::write(&tmp_path, &content)
        .with_context(|| format!("failed to write temp file {}", tmp_path.display()))?;
    if let Err(e) = std::fs::rename(&tmp_path, path) {
        // Best-effort cleanup so a stale `machine.toml.tmp` doesn't
        // accumulate after a failed save. Ignore the cleanup result on
        // purpose: the rename error is the real failure; masking it with
        // a cleanup error would hide the actual cause.
        let _ = std::fs::remove_file(&tmp_path);
        return Err(e).with_context(|| format!("failed to rename to {}", path.display()));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_prefs_has_empty_disabled() {
        let prefs = MachinePrefs::default();
        assert!(prefs.disabled.is_empty());
        assert!(!prefs.is_disabled("anything"));
    }

    #[test]
    fn is_disabled_checks_set() {
        let mut prefs = MachinePrefs::default();
        prefs.disable(SkillName::new("blocked").unwrap());
        assert!(prefs.is_disabled("blocked"));
        assert!(!prefs.is_disabled("allowed"));
    }

    #[test]
    fn load_missing_file_returns_defaults() {
        let tmp = tempfile::TempDir::new().unwrap();
        let path = tmp.path().join("machine.toml");
        let prefs = load(&path).unwrap();
        assert!(prefs.disabled.is_empty());
    }

    #[test]
    fn save_load_roundtrip() {
        let tmp = tempfile::TempDir::new().unwrap();
        let path = tmp.path().join("machine.toml");

        let mut prefs = MachinePrefs::default();
        prefs.disable(SkillName::new("skill-a").unwrap());
        prefs.disable(SkillName::new("skill-b").unwrap());

        save(&prefs, &path).unwrap();
        let loaded = load(&path).unwrap();

        assert_eq!(loaded.disabled.len(), 2);
        assert!(loaded.is_disabled("skill-a"));
        assert!(loaded.is_disabled("skill-b"));
    }

    #[test]
    fn toml_format_is_readable() {
        let mut prefs = MachinePrefs::default();
        prefs.disable(SkillName::new("unwanted-skill").unwrap());

        let toml_str = toml::to_string_pretty(&prefs).unwrap();
        assert!(toml_str.contains("unwanted-skill"));

        // Should be parseable
        let parsed: MachinePrefs = toml::from_str(&toml_str).unwrap();
        assert!(parsed.is_disabled("unwanted-skill"));
    }

    #[test]
    fn load_malformed_toml_returns_error() {
        let tmp = tempfile::TempDir::new().unwrap();
        let path = tmp.path().join("machine.toml");
        std::fs::write(&path, "disabled = not-a-list").unwrap();
        assert!(load(&path).is_err());
    }

    #[test]
    fn save_does_not_leave_tmp_file() {
        let tmp = tempfile::TempDir::new().unwrap();
        let path = tmp.path().join("machine.toml");

        let mut prefs = MachinePrefs::default();
        prefs.disable(SkillName::new("test-skill").unwrap());
        save(&prefs, &path).unwrap();

        assert!(path.exists());
        assert!(
            !tmp.path().join("machine.toml.tmp").exists(),
            "atomic save should not leave tmp file behind"
        );
    }

    #[test]
    fn save_creates_parent_directories() {
        let tmp = tempfile::TempDir::new().unwrap();
        let path = tmp.path().join("nested").join("dir").join("machine.toml");

        let prefs = MachinePrefs::default();
        save(&prefs, &path).unwrap();
        assert!(path.exists());
    }

    #[test]
    fn is_directory_disabled_checks_set() {
        let mut prefs = MachinePrefs::default();
        prefs.disable_directory(DirectoryName::new("claude").unwrap());
        prefs.disable_directory(DirectoryName::new("codex").unwrap());

        assert!(prefs.is_directory_disabled("claude"));
        assert!(prefs.is_directory_disabled("codex"));
        assert!(!prefs.is_directory_disabled("cursor"));
    }

    #[test]
    fn disabled_directories_roundtrip() {
        let tmp = tempfile::TempDir::new().unwrap();
        let path = tmp.path().join("machine.toml");

        let mut prefs = MachinePrefs::default();
        prefs.disable(SkillName::new("skill-a").unwrap());
        prefs.disable_directory(DirectoryName::new("claude").unwrap());
        prefs.disable_directory(DirectoryName::new("codex").unwrap());

        save(&prefs, &path).unwrap();
        let loaded = load(&path).unwrap();

        assert_eq!(loaded.disabled_directories.len(), 2);
        assert!(loaded.is_directory_disabled("claude"));
        assert!(loaded.is_directory_disabled("codex"));
        // Verify skills survived too
        assert!(loaded.is_disabled("skill-a"));
    }

    #[test]
    fn disabled_directories_defaults_empty() {
        // TOML with only the disabled field — disabled_directories should default to empty
        let toml_str = "disabled = [\"some-skill\"]\n";
        let prefs: MachinePrefs = toml::from_str(toml_str).unwrap();

        assert!(prefs.disabled_directories.is_empty());
        assert!(!prefs.is_directory_disabled("anything"));
        assert!(prefs.is_disabled("some-skill"));
    }

    // === Per-directory skill filtering tests (02-02) ===

    #[test]
    fn is_skill_allowed_empty_prefs_returns_true() {
        let prefs = MachinePrefs::default();
        assert!(prefs.is_skill_allowed("my-skill", "my-dir"));
    }

    #[test]
    fn is_skill_allowed_global_disabled_blocks() {
        let mut prefs = MachinePrefs::default();
        prefs.disable(SkillName::new("blocked").unwrap());
        assert!(!prefs.is_skill_allowed("blocked", "my-dir"));
    }

    #[test]
    fn is_skill_allowed_per_dir_enabled_overrides_global_disabled() {
        let mut prefs = MachinePrefs::default();
        prefs.disable(SkillName::new("blocked").unwrap());

        let dir_prefs = DirectoryPrefs {
            enabled: Some([SkillName::new("blocked").unwrap()].into_iter().collect()),
            ..Default::default()
        };
        prefs
            .directory
            .insert(DirectoryName::new("my-dir").unwrap(), dir_prefs);

        // Per-directory enabled overrides global disabled (locality principle D-08)
        assert!(prefs.is_skill_allowed("blocked", "my-dir"));
    }

    #[test]
    fn is_skill_allowed_per_dir_disabled_blocks() {
        let mut prefs = MachinePrefs::default();

        let dir_prefs = DirectoryPrefs {
            disabled: [SkillName::new("local-block").unwrap()]
                .into_iter()
                .collect(),
            ..Default::default()
        };
        prefs
            .directory
            .insert(DirectoryName::new("my-dir").unwrap(), dir_prefs);

        assert!(!prefs.is_skill_allowed("local-block", "my-dir"));
    }

    #[test]
    fn is_skill_allowed_per_dir_enabled_is_exclusive_allowlist() {
        let mut prefs = MachinePrefs::default();

        let dir_prefs = DirectoryPrefs {
            enabled: Some([SkillName::new("allowed").unwrap()].into_iter().collect()),
            ..Default::default()
        };
        prefs
            .directory
            .insert(DirectoryName::new("strict-dir").unwrap(), dir_prefs);

        assert!(prefs.is_skill_allowed("allowed", "strict-dir"));
        assert!(!prefs.is_skill_allowed("not-in-list", "strict-dir"));
    }

    #[test]
    fn is_skill_allowed_global_disabled_applies_to_unconfigured_dirs() {
        let mut prefs = MachinePrefs::default();
        prefs.disable(SkillName::new("global-block").unwrap());

        // Add per-directory prefs only for "my-dir"
        prefs.directory.insert(
            DirectoryName::new("my-dir").unwrap(),
            DirectoryPrefs::default(),
        );

        // "other-dir" has no per-directory prefs — global applies
        assert!(!prefs.is_skill_allowed("global-block", "other-dir"));
    }

    #[test]
    fn validate_rejects_both_disabled_and_enabled() {
        let mut prefs = MachinePrefs::default();
        let dir_prefs = DirectoryPrefs {
            disabled: [SkillName::new("a").unwrap()].into_iter().collect(),
            enabled: Some([SkillName::new("b").unwrap()].into_iter().collect()),
        };
        prefs
            .directory
            .insert(DirectoryName::new("bad-dir").unwrap(), dir_prefs);

        let err = prefs.validate().unwrap_err();
        assert!(
            err.to_string().contains("both 'disabled' and 'enabled'"),
            "expected validation error, got: {err}"
        );
    }

    #[test]
    fn validate_accepts_only_disabled() {
        let mut prefs = MachinePrefs::default();
        let dir_prefs = DirectoryPrefs {
            disabled: [SkillName::new("a").unwrap()].into_iter().collect(),
            ..Default::default()
        };
        prefs
            .directory
            .insert(DirectoryName::new("ok-dir").unwrap(), dir_prefs);

        prefs.validate().unwrap();
    }

    #[test]
    fn validate_accepts_only_enabled() {
        let mut prefs = MachinePrefs::default();
        let dir_prefs = DirectoryPrefs {
            enabled: Some([SkillName::new("a").unwrap()].into_iter().collect()),
            ..Default::default()
        };
        prefs
            .directory
            .insert(DirectoryName::new("ok-dir").unwrap(), dir_prefs);

        prefs.validate().unwrap();
    }

    #[test]
    fn validate_accepts_neither() {
        let mut prefs = MachinePrefs::default();
        prefs.directory.insert(
            DirectoryName::new("empty-dir").unwrap(),
            DirectoryPrefs::default(),
        );

        prefs.validate().unwrap();
    }

    #[test]
    fn toml_roundtrip_per_dir_disabled() {
        let toml_str = r#"
[directory.my-source]
disabled = ["unwanted"]
"#;
        let prefs: MachinePrefs = toml::from_str(toml_str).unwrap();
        assert!(prefs.directory.contains_key("my-source"));
        let dir_prefs = &prefs.directory["my-source"];
        assert!(dir_prefs.disabled.contains("unwanted"));
        assert!(dir_prefs.enabled.is_none());
    }

    #[test]
    fn toml_roundtrip_per_dir_enabled() {
        let toml_str = r#"
[directory.strict-dir]
enabled = ["only-this"]
"#;
        let prefs: MachinePrefs = toml::from_str(toml_str).unwrap();
        assert!(prefs.directory.contains_key("strict-dir"));
        let dir_prefs = &prefs.directory["strict-dir"];
        assert!(dir_prefs.disabled.is_empty());
        assert!(dir_prefs.enabled.as_ref().unwrap().contains("only-this"));
    }

    #[test]
    fn existing_machine_toml_without_directory_section_still_parses() {
        // Backward compat: existing machine.toml files only have global disabled
        let toml_str = r#"disabled = ["old-skill"]
disabled_directories = ["old-dir"]
"#;
        let prefs: MachinePrefs = toml::from_str(toml_str).unwrap();
        assert!(prefs.is_disabled("old-skill"));
        assert!(prefs.is_directory_disabled("old-dir"));
        assert!(prefs.directory.is_empty());
    }

    #[test]
    fn disabled_directories_toml_format() {
        let mut prefs = MachinePrefs::default();
        prefs.disable_directory(DirectoryName::new("claude").unwrap());
        prefs.disable_directory(DirectoryName::new("windsurf").unwrap());

        let toml_str = toml::to_string_pretty(&prefs).unwrap();
        assert!(toml_str.contains("disabled_directories"));
        assert!(toml_str.contains("claude"));
        assert!(toml_str.contains("windsurf"));

        // Should be parseable back
        let parsed: MachinePrefs = toml::from_str(&toml_str).unwrap();
        assert!(parsed.is_directory_disabled("claude"));
        assert!(parsed.is_directory_disabled("windsurf"));
    }

    // === [directory_overrides.<name>] schema tests (PORT-01) ===

    #[test]
    fn directory_overrides_default_empty() {
        let prefs = MachinePrefs::default();
        assert!(prefs.directory_overrides.is_empty());
    }

    #[test]
    fn directory_overrides_parses_from_toml() {
        let toml_str = r#"
[directory_overrides.claude-skills]
path = "/work/skills"
"#;
        let prefs: MachinePrefs = toml::from_str(toml_str).unwrap();
        let entry = prefs
            .directory_overrides
            .get("claude-skills")
            .expect("override missing");
        assert_eq!(entry.path, PathBuf::from("/work/skills"));
    }

    #[test]
    fn directory_overrides_with_tilde_path_is_preserved_unexpanded() {
        // serde::Deserialize for PathBuf treats `~` as a literal char; tilde
        // expansion is delayed to Config::apply_machine_overrides so override
        // paths follow the same expansion semantics as paths in tome.toml.
        let toml_str = r#"
[directory_overrides.x]
path = "~/work/skills"
"#;
        let prefs: MachinePrefs = toml::from_str(toml_str).unwrap();
        let entry = prefs.directory_overrides.get("x").unwrap();
        assert_eq!(entry.path, PathBuf::from("~/work/skills"));
    }

    #[test]
    fn directory_overrides_roundtrip() {
        let mut prefs = MachinePrefs::default();
        prefs.directory_overrides.insert(
            DirectoryName::new("work").unwrap(),
            DirectoryOverride {
                path: PathBuf::from("/work/skills"),
            },
        );

        let toml_str = toml::to_string_pretty(&prefs).unwrap();
        let parsed: MachinePrefs = toml::from_str(&toml_str).unwrap();

        assert_eq!(parsed.directory_overrides.len(), 1);
        let entry = parsed.directory_overrides.get("work").unwrap();
        assert_eq!(entry.path, PathBuf::from("/work/skills"));
    }

    #[test]
    fn existing_machine_toml_without_overrides_still_parses() {
        // Backward compat: an existing machine.toml without [directory_overrides.*]
        // must still parse, with directory_overrides defaulting to empty.
        let toml_str = "disabled = [\"x\"]\n";
        let prefs: MachinePrefs = toml::from_str(toml_str).unwrap();

        assert!(prefs.directory_overrides.is_empty());
        assert!(prefs.is_disabled("x"));
    }

    #[test]
    fn directory_overrides_save_skips_when_empty() {
        // With no overrides set, the on-disk TOML must not emit a
        // `[directory_overrides]` table heading — the empty map is invisible.
        let prefs = MachinePrefs::default();
        let toml_str = toml::to_string_pretty(&prefs).unwrap();
        assert!(
            !toml_str.contains("[directory_overrides"),
            "empty directory_overrides should not be serialized, got:\n{toml_str}"
        );
    }

    #[test]
    fn directory_overrides_unknown_extra_field_rejected() {
        // `deny_unknown_fields` on DirectoryOverride catches typos in a
        // future-renamed field before they silently no-op.
        let toml_str = r#"
[directory_overrides.x]
path = "/p"
bogus = "y"
"#;
        let result: Result<MachinePrefs, _> = toml::from_str(toml_str);
        assert!(
            result.is_err(),
            "expected parse failure for unknown field, got: {result:?}"
        );
    }
}
