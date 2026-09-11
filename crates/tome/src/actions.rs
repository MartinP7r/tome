//! Cross-surface skill actions (TUI + GUI).
//!
//! Pure-Rust helpers shared between the browse TUI (`browse::app`) and the
//! Tauri command surface (`tome-desktop::commands`). These functions own the
//! "what" of an action — compute the path or reject a legacy mutation that
//! lacks a route destination — but not the "how" of presenting the result
//! (clipboard, opener, focus management).
//!
//! # Scope
//!
//! - [`resolve_source_path`] — look up a skill's on-disk source path via the
//!   library manifest. Owned skills return their original directory location;
//!   Unowned skills fall back to the library-canonical copy.
//! - [`set_skill_disabled`] — retain the legacy global-toggle API while
//!   directing callers to route exclusion, which requires a destination.
//!
//! # Non-scope
//!
//! Legacy per-directory blocklist / allowlist toggles are rejected by the TUI
//! for the same reason: neither control can choose a route destination.

use std::path::{Path, PathBuf};

use anyhow::{Context, Result};

use crate::TomePaths;
use crate::config::Config;
use crate::discover::SkillName;
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

/// Reject the legacy global disabled control because it cannot select a route
/// destination. The parameters remain for paused Desktop API compatibility.
pub fn set_skill_disabled(_name: &SkillName, _disabled: bool, _machine_path: &Path) -> Result<()> {
    anyhow::bail!(
        "legacy skill exclusion controls are unsupported because they cannot choose a route destination; use `tome route exclude`"
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::DirectoryName;
    use crate::manifest::{Manifest, SkillEntry};
    use crate::validation::test_hash;
    use std::fs;
    use tempfile::TempDir;

    // ---- set_skill_disabled --------------------------------------------------

    #[test]
    fn set_skill_disabled_rejects_without_writing_machine_toml() {
        let tmp = TempDir::new().unwrap();
        let machine_path = tmp.path().join("machine.toml");
        let skill = SkillName::new("focus-me").unwrap();

        let error = set_skill_disabled(&skill, true, &machine_path).unwrap_err();
        assert!(
            error.to_string().contains("tome route exclude"),
            "legacy controls must direct users to the routed exclusion command: {error:#}"
        );
        assert!(
            !machine_path.exists(),
            "legacy controls must not create machine.toml without a route destination"
        );
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
