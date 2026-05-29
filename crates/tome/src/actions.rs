//! Cross-surface skill actions (TUI + GUI).
//!
//! Pure-Rust helpers shared between the browse TUI (`browse::app`) and the
//! Tauri command surface (`tome-desktop::commands`). These functions own the
//! "what" of an action — compute the path, mutate `machine.toml` — but not the
//! "how" of presenting the result (clipboard, opener, focus management).
//!
//! # Scope
//!
//! - [`resolve_source_path`] — look up a skill's on-disk source path via the
//!   library manifest. Owned skills return their original directory location;
//!   Unowned skills fall back to the library-canonical copy.
//! - [`set_skill_disabled`] — toggle the global `disabled` set in
//!   `machine.toml` via the existing [`crate::machine::save`] atomic
//!   temp+rename pattern. Used by the GUI's `set_skill_disabled` Tauri command
//!   (D-06) and the TUI's `apply_toggle` Global scope arm.
//!
//! # Non-scope
//!
//! Per-directory blocklist / allowlist toggles (HARD-21 `ToggleScope::PerDir*`)
//! are NOT covered here — they are TUI-only semantics. The TUI's `apply_toggle`
//! continues to call `MachinePrefs::toggle_per_dir_*` directly for those arms.
//! The GUI's D-06 mutation is global-scope only.

use std::path::{Path, PathBuf};

use anyhow::{Context, Result};

use crate::TomePaths;
use crate::config::Config;
use crate::discover::SkillName;
use crate::machine;
use crate::manifest;

/// Resolve a skill's on-disk source path via the library manifest.
///
/// For Owned skills the manifest carries the original `source_path` (the
/// directory the skill was discovered from). For Unowned skills — whose
/// source directory has been removed from `tome.toml` but the library copy
/// is preserved per LIB-04 — we fall back to the library-canonical location
/// `<library_dir>/<skill-name>/`.
///
/// # Errors
///
/// Returns an error if no manifest entry exists for `name`. The
/// `anyhow::Context` attached is callable-friendly ("skill 'foo' not found
/// in manifest") so the IPC boundary or TUI status line surfaces an
/// actionable message.
pub fn resolve_source_path(
    name: &SkillName,
    _config: &Config,
    paths: &TomePaths,
) -> Result<PathBuf> {
    let manifest = manifest::load(paths.config_dir())
        .with_context(|| format!("failed to load manifest while resolving '{name}'"))?;
    match manifest.get(name.as_str()) {
        Some(entry) => Ok(entry.source_path.clone()),
        None => {
            // Unowned-or-missing fallback: if the library has a directory at
            // `<library_dir>/<name>` we treat that as the canonical location.
            // If neither manifest nor library has a record, bail with a clear
            // error so the caller can surface "skill not found".
            let library_copy = paths.library_dir().join(name.as_str());
            if library_copy.exists() {
                Ok(library_copy)
            } else {
                anyhow::bail!(
                    "skill '{name}' not found in manifest (and no library-canonical copy at {})",
                    library_copy.display()
                )
            }
        }
    }
}

/// Toggle a skill's membership in the **global** `disabled` set in
/// `machine.toml` (D-06).
///
/// Loads the current `MachinePrefs`, calls
/// [`MachinePrefs::toggle_global_disabled`](crate::machine::MachinePrefs::toggle_global_disabled),
/// and saves via the existing [`crate::machine::save`] atomic temp+rename
/// pattern. Per-directory blocklist / allowlist toggles are NOT covered here —
/// they are TUI-only semantics (see module docs).
///
/// # Behaviour
///
/// - `disabled = true` adds the skill to the set (idempotent).
/// - `disabled = false` removes it (idempotent).
///
/// # File-watcher contract
///
/// The atomic temp+rename write fires a `MachinePrefsChanged` event on the
/// Phase 26 file watcher (plan 26-06), including for own-process writes —
/// callers do NOT need to emit a manual refresh signal. The GUI's React
/// hooks (`useSkills`, `useSkillDetail`) refetch on that event.
pub fn set_skill_disabled(name: &SkillName, disabled: bool, machine_path: &Path) -> Result<()> {
    let mut prefs =
        machine::load(machine_path).context("failed to load machine prefs before toggle")?;
    prefs.toggle_global_disabled(name.clone(), disabled);
    machine::save(&prefs, machine_path).context("failed to save machine prefs after toggle")?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::DirectoryName;
    use crate::machine::MachinePrefs;
    use crate::manifest::{Manifest, SkillEntry};
    use crate::validation::test_hash;
    use std::fs;
    use tempfile::TempDir;

    // ---- set_skill_disabled --------------------------------------------------

    #[test]
    fn set_skill_disabled_writes_machine_toml_atomically() {
        let tmp = TempDir::new().unwrap();
        let machine_path = tmp.path().join("machine.toml");
        let skill = SkillName::new("focus-me").unwrap();

        // Disable a skill from a clean slate — must create the file with the
        // entry present.
        set_skill_disabled(&skill, true, &machine_path).unwrap();
        let prefs = machine::load(&machine_path).unwrap();
        assert!(
            prefs.is_disabled("focus-me"),
            "set_skill_disabled(true) must persist into machine.toml"
        );

        // Re-enable — the entry must be removed.
        set_skill_disabled(&skill, false, &machine_path).unwrap();
        let prefs = machine::load(&machine_path).unwrap();
        assert!(
            !prefs.is_disabled("focus-me"),
            "set_skill_disabled(false) must remove the entry from machine.toml"
        );
    }

    #[test]
    fn set_skill_disabled_is_idempotent_on_repeat() {
        let tmp = TempDir::new().unwrap();
        let machine_path = tmp.path().join("machine.toml");
        let skill = SkillName::new("idem").unwrap();

        set_skill_disabled(&skill, true, &machine_path).unwrap();
        set_skill_disabled(&skill, true, &machine_path).unwrap();
        let prefs = machine::load(&machine_path).unwrap();
        assert!(prefs.is_disabled("idem"));

        set_skill_disabled(&skill, false, &machine_path).unwrap();
        set_skill_disabled(&skill, false, &machine_path).unwrap();
        let prefs = machine::load(&machine_path).unwrap();
        assert!(!prefs.is_disabled("idem"));
    }

    #[test]
    fn set_skill_disabled_preserves_existing_entries() {
        // Pre-existing `disabled` entries (and other prefs fields) must survive
        // a toggle of an unrelated skill.
        let tmp = TempDir::new().unwrap();
        let machine_path = tmp.path().join("machine.toml");

        let mut prefs = MachinePrefs::default();
        prefs.disable(SkillName::new("keep-me").unwrap());
        machine::save(&prefs, &machine_path).unwrap();

        let other = SkillName::new("toggle-me").unwrap();
        set_skill_disabled(&other, true, &machine_path).unwrap();

        let loaded = machine::load(&machine_path).unwrap();
        assert!(
            loaded.is_disabled("keep-me"),
            "unrelated pre-existing disabled entry must survive the toggle"
        );
        assert!(loaded.is_disabled("toggle-me"));
    }

    // ---- resolve_source_path -------------------------------------------------

    /// Build a minimal `TomePaths` + `Config` pointing at a tempdir-backed
    /// fake tome_home with the canonical layout (`<root>/skills/` library,
    /// `<root>/` config dir).
    fn temp_paths() -> (TempDir, Config, TomePaths) {
        let tmp = TempDir::new().unwrap();
        let tome_home = tmp.path().to_path_buf();
        let library_dir = tome_home.join("skills");
        fs::create_dir_all(&library_dir).unwrap();
        let paths = TomePaths::new(tome_home, library_dir).unwrap();
        // Default Config — actions::resolve_source_path doesn't consume any
        // directories on the manifest-hit path; the no-manifest fallback only
        // looks at `paths.library_dir()`.
        let config = Config::default();
        (tmp, config, paths)
    }

    #[test]
    fn resolve_source_path_returns_manifest_source_for_owned() {
        let (_tmp, config, paths) = temp_paths();

        let skill = SkillName::new("axiom-build").unwrap();
        let source_path = PathBuf::from("/work/dotfiles/skills/axiom-build");

        let mut manifest = Manifest::default();
        manifest.insert(
            skill.clone(),
            SkillEntry::new(
                source_path.clone(),
                DirectoryName::new("dotfiles").unwrap(),
                test_hash("axiom-build"),
                false,
            ),
        );
        manifest::save(&manifest, paths.config_dir()).unwrap();

        let resolved = resolve_source_path(&skill, &config, &paths).unwrap();
        assert_eq!(
            resolved, source_path,
            "Owned skill must resolve to the manifest's source_path"
        );
    }

    #[test]
    fn resolve_source_path_falls_back_to_library_for_unowned() {
        let (_tmp, config, paths) = temp_paths();

        let skill = SkillName::new("orphaned").unwrap();
        // No manifest entry, but the library has a copy at the canonical
        // location — Unowned skills live there per LIB-04.
        let library_copy = paths.library_dir().join("orphaned");
        fs::create_dir_all(&library_copy).unwrap();

        let resolved = resolve_source_path(&skill, &config, &paths).unwrap();
        assert_eq!(
            resolved, library_copy,
            "Skill with no manifest entry but a library-canonical copy must \
             resolve to the library path (Unowned fallback per LIB-04)"
        );
    }

    #[test]
    fn resolve_source_path_errors_when_skill_not_found_anywhere() {
        let (_tmp, config, paths) = temp_paths();

        let skill = SkillName::new("nope").unwrap();
        let err = resolve_source_path(&skill, &config, &paths).unwrap_err();
        let msg = format!("{err:#}");
        assert!(
            msg.contains("nope"),
            "error message must name the missing skill; got: {msg}"
        );
    }

    #[test]
    fn resolve_source_path_returns_unowned_manifest_entry_path() {
        // A manifest can carry an Unowned entry (source removed from
        // `tome.toml` but library copy preserved). `resolve_source_path`
        // should still return the `source_path` field from the manifest —
        // the library-fallback branch is only for the missing-entry case.
        let (_tmp, config, paths) = temp_paths();

        let skill = SkillName::new("legacy").unwrap();
        let preserved_source_path = PathBuf::from("/no/longer/configured/legacy");

        let mut manifest = Manifest::default();
        manifest.insert(
            skill.clone(),
            SkillEntry::new_unowned(
                preserved_source_path.clone(),
                test_hash("legacy"),
                false,
                Some(DirectoryName::new("old-dir").unwrap()),
            ),
        );
        manifest::save(&manifest, paths.config_dir()).unwrap();

        let resolved = resolve_source_path(&skill, &config, &paths).unwrap();
        assert_eq!(
            resolved, preserved_source_path,
            "Unowned manifest entry must surface its preserved source_path"
        );
    }
}
