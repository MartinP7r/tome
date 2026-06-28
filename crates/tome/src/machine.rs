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

/// 3-state auto-install consent persisted in `machine.toml` (RECON-02 D-07).
///
/// `None` (field absent / unset) means "first-time prompt" — distinguished
/// from `Some(Ask)` which means "user picked 'n' last time, ask again."
///
/// Per CONTEXT.md D-08:
/// - `Always` — auto-apply drift on every sync (default Y).
/// - `Ask` — re-prompt every sync that detects drift (n).
/// - `Never` — surface drift as warnings; no install/update (never).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AutoInstall {
    Always,
    Ask,
    Never,
}

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

impl DirectoryPrefs {
    /// Read-only view of the blocklist for HARD-21 D-BROWSE-1 scope
    /// resolution. Stays `pub(crate)` because the underlying field is
    /// per-machine implementation detail; the v1.0 GUI Tauri IPC will
    /// surface a different read-shape.
    pub(crate) fn disabled_set(&self) -> &BTreeSet<SkillName> {
        &self.disabled
    }

    /// Read-only view of the allowlist for HARD-21 D-BROWSE-1 scope
    /// resolution. Returns `None` when no allowlist is configured (the
    /// directory falls through to blocklist-or-global semantics).
    pub(crate) fn enabled_set(&self) -> Option<&BTreeSet<SkillName>> {
        self.enabled.as_ref()
    }
}

/// Read-only accessor for `MachinePrefs::directory[<name>]` so the
/// `browse` module (HARD-21 D-BROWSE-1) can query scope without
/// widening the underlying field's visibility past `pub(crate)`.
pub(crate) fn directory_prefs<'a>(
    prefs: &'a MachinePrefs,
    name: &DirectoryName,
) -> Option<&'a DirectoryPrefs> {
    prefs.directory.get(name)
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

    /// Per-machine auto-install consent for missing/drifted managed plugins
    /// (RECON-02 D-07). `None` = first-time prompt; `Some(Always|Ask|Never)`
    /// is the persisted user choice. Reconcile reads/writes this field.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) auto_install_plugins: Option<AutoInstall>,
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

    /// Mutate or insert the per-directory `disabled` blocklist for `dir`.
    ///
    /// Used by `browse::App::apply_toggle` (HARD-21 D-BROWSE-1, scope =
    /// `PerDirBlocklist`). Returns true if the set changed (insert on
    /// `Disable`, remove on `Enable`). Honors MACH-04 by construction:
    /// only the `disabled` field is touched; `enabled` is never set here.
    pub(crate) fn toggle_per_dir_blocklist(
        &mut self,
        dir: &DirectoryName,
        skill: SkillName,
        disable: bool,
    ) -> bool {
        let entry = self.directory.entry(dir.clone()).or_default();
        if disable {
            entry.disabled.insert(skill)
        } else {
            entry.disabled.remove(skill.as_str())
        }
    }

    /// Mutate the per-directory `enabled` allowlist for `dir` with
    /// inverted polarity: `Disable` REMOVES the skill from the allowlist
    /// (membership = "include"), `Enable` INSERTS it.
    ///
    /// Used by `browse::App::apply_toggle` (HARD-21 D-BROWSE-1, scope =
    /// `PerDirAllowlist`). Honors MACH-04 by construction.
    pub(crate) fn toggle_per_dir_allowlist(
        &mut self,
        dir: &DirectoryName,
        skill: SkillName,
        disable: bool,
    ) -> bool {
        let entry = self.directory.entry(dir.clone()).or_default();
        let allowlist = entry.enabled.get_or_insert_with(BTreeSet::new);
        if disable {
            allowlist.remove(skill.as_str())
        } else {
            allowlist.insert(skill)
        }
    }

    /// Mutate the global `disabled` blocklist. `disable=true` adds,
    /// `disable=false` removes.
    pub(crate) fn toggle_global_disabled(&mut self, skill: SkillName, disable: bool) -> bool {
        if disable {
            self.disabled.insert(skill)
        } else {
            self.disabled.remove(skill.as_str())
        }
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

/// Single line in a `MachineTomlPreview`. Surfaces the side it lives on as a
/// 1-indexed line number (against the OLD side for `Removed`, against the NEW
/// side for `Added` + `Unchanged`) plus the literal content of the line
/// (trailing `\n` stripped — the diff renderer reintroduces the newline visually).
///
/// Produced by [`preview_save`] and consumed by the Desktop GUI's
/// `MachineTomlDiff` component via the `preview_machine_toml` Tauri command.
#[derive(Debug, Clone, Serialize)]
#[cfg_attr(feature = "bindings", derive(specta::Type))]
pub struct DiffLine {
    /// 1-indexed line number on the side this line lives on.
    pub line_number: u32,
    /// Whether this line was removed, added, or unchanged in the diff.
    pub kind: DiffLineKind,
    /// Literal text of the line, without the trailing newline character.
    pub content: String,
}

/// Kind of change for a [`DiffLine`]. Serializes as the lowercase tag names
/// (`"unchanged"`, `"removed"`, `"added"`) for direct consumption by the React
/// `MachineTomlDiff` component (see `27-UI-SPEC.md` §MachineTomlDiff).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
#[cfg_attr(feature = "bindings", derive(specta::Type))]
pub enum DiffLineKind {
    Unchanged,
    Removed,
    Added,
}

/// Structured Myers line-diff between the on-disk `machine.toml` and the
/// canonical `toml::to_string_pretty(proposed)` serialization.
///
/// Returned by [`preview_save`]; consumed by the Desktop GUI's
/// `preview_machine_toml` Tauri command. The companion `apply_machine_toml`
/// command commits the proposed prefs via the existing atomic [`save`] —
/// `preview_save` itself is a pure (no-write) helper.
#[derive(Debug, Clone, Serialize)]
#[cfg_attr(feature = "bindings", derive(specta::Type))]
pub struct MachineTomlPreview {
    /// Every line of the diff in display order: Equal/Removed/Added.
    pub lines: Vec<DiffLine>,
    /// Number of [`DiffLineKind::Added`] entries in `lines`.
    pub added_count: usize,
    /// Number of [`DiffLineKind::Removed`] entries in `lines`.
    pub removed_count: usize,
}

/// Compute a Myers line-diff between the current on-disk `machine.toml`
/// content and the canonical serialization of `proposed`. A missing
/// `current_path` is treated as empty current text — the very first Apply
/// shows every proposed line as Added.
///
/// This is the "preview" half of the Desktop GUI's preview-then-apply flow
/// (SYNC-03 / SC#3). It performs **no filesystem writes** — the apply step
/// uses the existing atomic [`save`] when the user explicitly confirms in
/// the PreviewPopover. The diff is computed via `similar::TextDiff::from_lines`
/// (MIT, mitsuhiko — see workspace `Cargo.toml` for the audit reference).
pub fn preview_save(proposed: &MachinePrefs, current_path: &Path) -> Result<MachineTomlPreview> {
    let current_text = std::fs::read_to_string(current_path).unwrap_or_default();
    let proposed_text =
        toml::to_string_pretty(proposed).context("failed to serialize proposed machine prefs")?;

    let diff = similar::TextDiff::from_lines(&current_text, &proposed_text);
    let mut lines = Vec::new();
    let mut added_count = 0usize;
    let mut removed_count = 0usize;
    let mut current_line: u32 = 1;
    let mut proposed_line: u32 = 1;

    for change in diff.iter_all_changes() {
        let (kind, line_number) = match change.tag() {
            similar::ChangeTag::Equal => {
                let n = proposed_line;
                proposed_line += 1;
                current_line += 1;
                (DiffLineKind::Unchanged, n)
            }
            similar::ChangeTag::Delete => {
                removed_count += 1;
                let n = current_line;
                current_line += 1;
                (DiffLineKind::Removed, n)
            }
            similar::ChangeTag::Insert => {
                added_count += 1;
                let n = proposed_line;
                proposed_line += 1;
                (DiffLineKind::Added, n)
            }
        };
        // `Change<&str>::to_string_lossy()` yields the line's literal text
        // (with the trailing newline preserved) as a Cow<str>. We strip the
        // trailing `\n` because the renderer reintroduces newlines visually
        // — keeping it in `content` would double up on display.
        let content = change.to_string_lossy().trim_end_matches('\n').to_string();
        lines.push(DiffLine {
            line_number,
            kind,
            content,
        });
    }

    Ok(MachineTomlPreview {
        lines,
        added_count,
        removed_count,
    })
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

    // === HARD-22 / D-TILDE-2: machine.toml override paths preserved verbatim ===
    //
    // Plan 15-02 explicitly fences `paths::unexpand_tilde` to `Config::save_checked`
    // only — `MachinePrefs::save` MUST NOT rewrite path fields. Per-machine
    // preferences are by definition machine-local; rewriting `/Volumes/External/...`
    // to `~/...` here would be wrong (Volumes paths don't live under $HOME on
    // any sane setup).
    //
    // Verified by save+load round-trips that compare the on-disk path bytes
    // against the input bytes for three representative cases.

    #[test]
    fn save_preserves_override_path_outside_home_verbatim() {
        let tmp = tempfile::TempDir::new().unwrap();
        let path = tmp.path().join("machine.toml");

        // Pick an absolute path that on every machine is OUTSIDE $HOME.
        let original = "/Volumes/External/skills";

        let mut prefs = MachinePrefs::default();
        prefs.directory_overrides.insert(
            crate::config::DirectoryName::new("foo").unwrap(),
            DirectoryOverride {
                path: PathBuf::from(original),
            },
        );

        save(&prefs, &path).unwrap();
        let on_disk = std::fs::read_to_string(&path).unwrap();
        assert!(
            on_disk.contains(&format!("path = \"{original}\"")),
            "machine.toml MUST preserve override path verbatim (D-TILDE-2), got:\n{on_disk}"
        );
        assert!(
            !on_disk.contains("~/"),
            "machine.toml MUST NOT contain ~/ rewrites (D-TILDE-2), got:\n{on_disk}"
        );
    }

    #[test]
    fn save_preserves_override_tilde_path_verbatim() {
        let tmp = tempfile::TempDir::new().unwrap();
        let path = tmp.path().join("machine.toml");

        // User-supplied tilde path: must survive byte-for-byte.
        let original = "~/skills";

        let mut prefs = MachinePrefs::default();
        prefs.directory_overrides.insert(
            crate::config::DirectoryName::new("foo").unwrap(),
            DirectoryOverride {
                path: PathBuf::from(original),
            },
        );

        save(&prefs, &path).unwrap();
        let on_disk = std::fs::read_to_string(&path).unwrap();
        assert!(
            on_disk.contains(&format!("path = \"{original}\"")),
            "machine.toml MUST preserve user-supplied tilde verbatim (D-TILDE-2), got:\n{on_disk}"
        );
    }

    #[test]
    fn save_preserves_override_absolute_under_home_verbatim() {
        // Even an absolute path under $HOME must NOT be rewritten in machine.toml
        // — D-TILDE-2 fences the unexpand pass to Config::save_checked only.
        let tmp = tempfile::TempDir::new().unwrap();
        let path = tmp.path().join("machine.toml");

        let home = dirs::home_dir().expect("home dir required for this test");
        let original = home.join("dotfiles/external");
        let original_str = original.to_str().unwrap();

        let mut prefs = MachinePrefs::default();
        prefs.directory_overrides.insert(
            crate::config::DirectoryName::new("foo").unwrap(),
            DirectoryOverride {
                path: original.clone(),
            },
        );

        save(&prefs, &path).unwrap();
        let on_disk = std::fs::read_to_string(&path).unwrap();
        assert!(
            on_disk.contains(&format!("path = \"{original_str}\"")),
            "machine.toml MUST preserve absolute under-$HOME path verbatim (D-TILDE-2), got:\n{on_disk}"
        );
        assert!(
            !on_disk.contains("path = \"~/"),
            "machine.toml MUST NOT rewrite under-$HOME paths to ~/ (D-TILDE-2 — fenced to tome.toml only), got:\n{on_disk}"
        );
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

    // === auto_install_plugins schema tests (RECON-02 D-07) ===

    #[test]
    fn auto_install_default_is_none() {
        let prefs = MachinePrefs::default();
        assert!(prefs.auto_install_plugins.is_none());
    }

    #[test]
    fn auto_install_round_trip() {
        // Always
        let tmp = tempfile::TempDir::new().unwrap();
        let path = tmp.path().join("machine.toml");
        let prefs = MachinePrefs {
            auto_install_plugins: Some(AutoInstall::Always),
            ..Default::default()
        };
        save(&prefs, &path).unwrap();
        let loaded = load(&path).unwrap();
        assert_eq!(loaded.auto_install_plugins, Some(AutoInstall::Always));

        // Ask
        let tmp = tempfile::TempDir::new().unwrap();
        let path = tmp.path().join("machine.toml");
        let prefs = MachinePrefs {
            auto_install_plugins: Some(AutoInstall::Ask),
            ..Default::default()
        };
        save(&prefs, &path).unwrap();
        let loaded = load(&path).unwrap();
        assert_eq!(loaded.auto_install_plugins, Some(AutoInstall::Ask));

        // Never
        let tmp = tempfile::TempDir::new().unwrap();
        let path = tmp.path().join("machine.toml");
        let prefs = MachinePrefs {
            auto_install_plugins: Some(AutoInstall::Never),
            ..Default::default()
        };
        save(&prefs, &path).unwrap();
        let loaded = load(&path).unwrap();
        assert_eq!(loaded.auto_install_plugins, Some(AutoInstall::Never));
    }

    #[test]
    fn auto_install_unset_omitted_on_save() {
        // Default MachinePrefs (auto_install_plugins = None) must NOT serialize
        // an `auto_install_plugins` key — skip_serializing_if = "Option::is_none".
        let prefs = MachinePrefs::default();
        let toml_str = toml::to_string_pretty(&prefs).unwrap();
        assert!(
            !toml_str.contains("auto_install_plugins"),
            "empty auto_install_plugins should not be serialized, got:\n{toml_str}"
        );
    }

    #[test]
    fn auto_install_lowercase_serde() {
        // Always
        let prefs = MachinePrefs {
            auto_install_plugins: Some(AutoInstall::Always),
            ..Default::default()
        };
        let toml_str = toml::to_string(&prefs).unwrap();
        assert!(
            toml_str.contains("auto_install_plugins = \"always\""),
            "expected lowercase 'always', got:\n{toml_str}"
        );

        // Ask
        let prefs = MachinePrefs {
            auto_install_plugins: Some(AutoInstall::Ask),
            ..Default::default()
        };
        let toml_str = toml::to_string(&prefs).unwrap();
        assert!(
            toml_str.contains("auto_install_plugins = \"ask\""),
            "expected lowercase 'ask', got:\n{toml_str}"
        );

        // Never
        let prefs = MachinePrefs {
            auto_install_plugins: Some(AutoInstall::Never),
            ..Default::default()
        };
        let toml_str = toml::to_string(&prefs).unwrap();
        assert!(
            toml_str.contains("auto_install_plugins = \"never\""),
            "expected lowercase 'never', got:\n{toml_str}"
        );
    }

    #[test]
    fn auto_install_existing_machine_toml_without_field_parses() {
        // Backward compat: existing machine.toml without auto_install_plugins
        // must still parse, with the field defaulting to None.
        let toml_str = "disabled = [\"x\"]\n";
        let prefs: MachinePrefs = toml::from_str(toml_str).unwrap();
        assert!(prefs.auto_install_plugins.is_none());
        assert!(prefs.is_disabled("x"));
    }

    #[test]
    fn auto_install_unknown_value_rejected() {
        // serde rename_all = "lowercase" only accepts the 3 known variants.
        let toml_str = "auto_install_plugins = \"sometimes\"\n";
        let result: Result<MachinePrefs, _> = toml::from_str(toml_str);
        assert!(
            result.is_err(),
            "expected parse failure for unknown auto_install_plugins value, got: {result:?}"
        );
    }

    // === preview_save (SYNC-03 27-03 Task 2) ===
    //
    // `preview_save(proposed, current_path) -> Result<MachineTomlPreview>` returns
    // a line-by-line Myers diff (via the `similar` crate) between the current
    // on-disk machine.toml text and the canonical `toml::to_string_pretty` of the
    // proposed prefs. The diff is the load-bearing piece that powers the
    // PreviewPopover in the Desktop GUI — the user MUST see the diff and click
    // [Apply] before any write occurs (SC#3 "no silent writes").

    #[test]
    fn preview_save_diffs_added_disabled_skill() {
        let tmp = tempfile::TempDir::new().unwrap();
        let path = tmp.path().join("machine.toml");

        // Step 1: write current state with one disabled skill.
        let mut current = MachinePrefs::default();
        current.disable(SkillName::new("foo").unwrap());
        save(&current, &path).unwrap();

        // Step 2: build proposed state with two disabled skills.
        let mut proposed = MachinePrefs::default();
        proposed.disable(SkillName::new("foo").unwrap());
        proposed.disable(SkillName::new("bar").unwrap());

        // Step 3: compute preview.
        let preview = preview_save(&proposed, &path).unwrap();

        // Should have at least one added line (the new `bar` membership) and at
        // least one removed line (the old `disabled = ["foo"]` shape).
        assert!(
            preview.added_count >= 1,
            "expected at least one added line, got preview={preview:?}"
        );
        assert!(
            preview.removed_count >= 1,
            "expected at least one removed line, got preview={preview:?}"
        );
        // And the `bar` token must appear in an Added line's content.
        let added_contains_bar = preview
            .lines
            .iter()
            .any(|l| matches!(l.kind, DiffLineKind::Added) && l.content.contains("bar"));
        assert!(
            added_contains_bar,
            "expected an Added line containing `bar`, got preview={preview:?}"
        );
    }

    #[test]
    fn preview_save_noop_when_proposed_matches_disk() {
        let tmp = tempfile::TempDir::new().unwrap();
        let path = tmp.path().join("machine.toml");

        let mut proposed = MachinePrefs::default();
        proposed.disable(SkillName::new("only-skill").unwrap());

        // Write the canonical serialization to disk; proposed matches byte-for-byte.
        let canonical = toml::to_string_pretty(&proposed).unwrap();
        std::fs::write(&path, &canonical).unwrap();

        let preview = preview_save(&proposed, &path).unwrap();
        assert_eq!(
            preview.added_count, 0,
            "expected no additions, got: {preview:?}"
        );
        assert_eq!(
            preview.removed_count, 0,
            "expected no removals, got: {preview:?}"
        );
        assert!(
            preview
                .lines
                .iter()
                .all(|l| matches!(l.kind, DiffLineKind::Unchanged)),
            "expected all lines Unchanged, got: {preview:?}"
        );
    }

    #[test]
    fn preview_save_missing_current_file_treats_as_empty() {
        let tmp = tempfile::TempDir::new().unwrap();
        let path = tmp.path().join("machine.toml"); // intentionally absent

        let mut proposed = MachinePrefs::default();
        proposed.disable(SkillName::new("new-skill").unwrap());

        let preview = preview_save(&proposed, &path).unwrap();
        // Every non-empty proposed line should show as Added; nothing Removed.
        assert!(preview.added_count >= 1);
        assert_eq!(preview.removed_count, 0);
        // Line numbers on Added entries are 1-indexed against the new side.
        let first_added = preview
            .lines
            .iter()
            .find(|l| matches!(l.kind, DiffLineKind::Added))
            .expect("expected at least one Added line");
        assert!(
            first_added.line_number >= 1,
            "line numbers are 1-indexed, got {}",
            first_added.line_number
        );
    }

    /// HARD-08: rename failure during atomic save must leave the previous
    /// `machine.toml` content untouched. machine.toml carries per-machine
    /// disable/override state — corrupting it would silently desync user
    /// preferences across the next sync.
    ///
    /// Mechanism: chmod 0o500 on the parent directory so fs::rename
    /// returns EACCES. Verify the on-disk bytes are byte-identical to
    /// the pre-fail state.
    #[cfg(unix)]
    #[test]
    fn save_preserves_previous_on_rename_failure() {
        use std::os::unix::fs::PermissionsExt;

        let tmp = tempfile::TempDir::new().unwrap();
        let path = tmp.path().join("machine.toml");

        // Step 1: write A via the canonical happy path.
        let mut prefs_a = MachinePrefs::default();
        prefs_a.disable(SkillName::new("alpha").unwrap());
        save(&prefs_a, &path).unwrap();
        let bytes_a = std::fs::read(&path).unwrap();

        // Step 2: lock the parent dir.
        let original_mode = std::fs::metadata(tmp.path()).unwrap().permissions().mode();
        std::fs::set_permissions(tmp.path(), std::fs::Permissions::from_mode(0o500)).unwrap();

        // Step 3: attempt to save B; must fail.
        let mut prefs_b = MachinePrefs::default();
        prefs_b.disable(SkillName::new("beta").unwrap());
        prefs_b.disable(SkillName::new("gamma").unwrap());
        let result = save(&prefs_b, &path);

        // Restore permissions BEFORE asserting so TempDir cleanup works.
        std::fs::set_permissions(tmp.path(), std::fs::Permissions::from_mode(original_mode))
            .unwrap();

        assert!(
            result.is_err(),
            "save() must fail when the parent directory is not writable"
        );

        // Step 4: re-read the file. It must still match A.
        let bytes_after = std::fs::read(&path).unwrap();
        assert_eq!(
            bytes_after, bytes_a,
            "atomic-save invariant violated: machine.toml content was \
             corrupted by a failed save"
        );
        let reloaded = load(&path).unwrap();
        assert!(reloaded.is_disabled("alpha"));
        assert!(!reloaded.is_disabled("beta"));
        assert!(!reloaded.is_disabled("gamma"));
    }
}
