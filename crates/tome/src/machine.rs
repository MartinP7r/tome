//! Per-machine preferences for skill management.
//!
//! Each machine can opt out of specific skills via `machine.toml`, which lives alongside
//! the main `config.toml` at `~/.config/tome/machine.toml`. The library stays complete
//! across machines; disabled skills are simply skipped during distribution.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

use crate::discover::SkillName;

/// Per-machine preferences — currently just a set of disabled skill names.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct MachinePrefs {
    /// Skills that should not be distributed to targets on this machine.
    #[serde(default)]
    pub disabled: BTreeSet<SkillName>,
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
    std::fs::rename(&tmp_path, path)
        .with_context(|| format!("failed to rename to {}", path.display()))
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
}
