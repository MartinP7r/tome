//! Per-machine path override application for [`Config`] (PORT-01..05).
//!
//! Hosts:
//! - `Config::apply_machine_overrides` — mutates a load-time-only copy of `Config`
//!   so override paths from `machine.toml` are seen by all downstream consumers,
//!   but never leak back into `tome.toml` on save (PORT-02 invariant).
//! - `Config::warn_unknown_overrides` — typo-target stderr warning helper (PORT-03).
//! - `format_override_validation_error` — wraps a `validate()` failure caused by
//!   an override, naming `machine.toml` as the file to edit (PORT-04).
//!
//! The single canonical caller for these methods is `Config::load_with_overrides`
//! in `super` (mod.rs), which threads the I2 invariant: expand → warn → snapshot →
//! apply → validate.

use anyhow::Result;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use super::types::Config;
use crate::paths::expand_tilde;

impl Config {
    /// Apply per-machine path overrides from `[directory_overrides.<name>]` entries
    /// in `machine.toml`. Mutates `self.directories[name].path` and sets
    /// `override_applied = true` on each matched entry.
    ///
    /// **Order constraint (I2 invariant):** Call this AFTER `expand_tildes()` and
    /// BEFORE `validate()`. The single canonical caller is `Config::load_with_overrides`.
    ///
    /// **Override path expansion:** the override's own `path` is tilde-expanded here
    /// (mirrors what `expand_tildes` did to the original path), so `~/...` works in
    /// `machine.toml` exactly as it does in `tome.toml`.
    ///
    /// **Unknown override targets:** silently ignored at this layer. The Plan 02
    /// follow-up (PORT-03 warn_unknown_overrides) emits stderr warnings; we keep
    /// them separate so this method stays infallible apart from tilde-expansion
    /// errors and side-effect-free apart from mutating `self`.
    ///
    /// **Idempotent:** safe to call multiple times — the override path is read
    /// from `prefs`, not from `self`, and tilde expansion is itself idempotent
    /// (already-absolute paths pass through unchanged).
    pub(crate) fn apply_machine_overrides(
        &mut self,
        prefs: &crate::machine::MachinePrefs,
    ) -> Result<()> {
        for (name, override_) in &prefs.directory_overrides {
            if let Some(dir) = self.directories.get_mut(name.as_str()) {
                // HARD-10: reject hostile path shapes BEFORE applying
                // them to the live config. The override is per-machine
                // user input; coercing or silently passing through a
                // path that escapes via `..` traversal would let an
                // imported `machine.toml` redirect tome sync at any
                // location on the filesystem.
                reject_hostile_override_path(name.as_str(), &override_.path)?;
                dir.path = expand_tilde(&override_.path)?;
                dir.override_applied = true;
            }
            // Unknown override targets: no-op here. PORT-03 (Plan 09-02) handles warnings.
        }

        // HARD-10: reject hostile `[directory_overrides.<name>]` shapes
        // that would silently corrupt downstream sync state. Two
        // overridden directories pointing at the same path would
        // distribute conflicting symlinks into one target dir, with
        // arbitrary "last write wins" ordering driven by BTreeMap
        // iteration. Refuse rather than coerce.
        let mut seen: BTreeMap<PathBuf, Vec<String>> = BTreeMap::new();
        for (name, dir) in &self.directories {
            if !dir.override_applied {
                continue;
            }
            seen.entry(dir.path.clone())
                .or_default()
                .push(name.as_str().to_string());
        }
        for (path, names) in &seen {
            if names.len() > 1 {
                anyhow::bail!(
                    "override-induced config error from machine.toml\n\
                     \n\
                     Conflict: two `[directory_overrides.<name>]` entries point at the same path:\n\
                       {}\n\
                     \n\
                     Affected directories: {}\n\
                     \n\
                     Why: distributing two different directories into one shared target path \
                     would clash on each `tome sync` run; ordering would be driven by \
                     alphabetical iteration, not user intent.\n\
                     \n\
                     hint: edit `machine.toml` and give each `[directory_overrides.<name>]` \
                     a distinct `path`.",
                    path.display(),
                    names.join(", "),
                );
            }
        }
        Ok(())
    }

    /// Emit a warning for each `[directory_overrides.<name>]` entry whose `<name>`
    /// does not match any key in `self.directories`. Caller-supplied `warn` closure
    /// receives the formatted message body (without the `warning:` prefix), so the
    /// caller decides whether to `eprintln!`, push to a Vec, or do something else.
    ///
    /// Used by `Config::load_with_overrides` to surface PORT-03 typo guards.
    /// Mirrors `lib.rs::warn_unknown_disabled_directories` (which handles the same
    /// typo case for `disabled_directories`).
    ///
    /// **Order:** call this BEFORE `apply_machine_overrides` so the user sees
    /// warnings about typos even if the apply step never touches them. (Apply is
    /// silent for unknown targets — see Plan 09-01.)
    pub(crate) fn warn_unknown_overrides(
        &self,
        prefs: &crate::machine::MachinePrefs,
        mut warn: impl FnMut(String),
    ) {
        for name in prefs.directory_overrides.keys() {
            if !self.directories.contains_key(name.as_str()) {
                warn(format!(
                    "directory_overrides target '{name}' in machine.toml does not match any configured directory"
                ));
            }
        }
    }
}

/// HARD-10: reject hostile `[directory_overrides.<name>]` paths before
/// they reach `Config`. The override is per-machine user input; coercing
/// or silently accepting paths that escape via `..`, contain NUL bytes,
/// or break path-component invariants would let an imported
/// `machine.toml` redirect `tome sync` at arbitrary filesystem locations.
///
/// Rejected shapes:
/// - empty path
/// - path containing a NUL byte (interior `\0`)
/// - any `..` component (parent-traversal, including `../foo` and
///   `/abs/foo/../bar`)
fn reject_hostile_override_path(name: &str, path: &Path) -> Result<()> {
    use std::path::Component;

    let display = path.display().to_string();

    if path.as_os_str().is_empty() {
        anyhow::bail!(
            "override-induced config error from machine.toml\n\
             Conflict: `[directory_overrides.{name}]` has an empty path.\n\
             Why: an empty override path is not a valid filesystem location.\n\
             hint: edit `machine.toml` and give `[directory_overrides.{name}]` a real path."
        );
    }

    if display.as_bytes().contains(&0) {
        anyhow::bail!(
            "override-induced config error from machine.toml\n\
             Conflict: `[directory_overrides.{name}]` path contains a NUL byte.\n\
             Why: paths with NUL bytes are invalid on POSIX filesystems and \
             usually indicate a copy/paste error or hostile input.\n\
             hint: edit `machine.toml` and give `[directory_overrides.{name}]` a clean path."
        );
    }

    for component in path.components() {
        if matches!(component, Component::ParentDir) {
            anyhow::bail!(
                "override-induced config error from machine.toml\n\
                 Conflict: `[directory_overrides.{name}]` path contains a `..` traversal: {display}\n\
                 Why: tome refuses overrides that escape the user's intent via parent-directory \
                 traversal — an absolute path is required for portability and audit-ability.\n\
                 hint: edit `machine.toml` and rewrite the path as an absolute location \
                 (e.g. `/Users/you/skills` or `~/skills`)."
            );
        }
    }

    // Symlink-loop detection: if the override target IS a symlink and
    // canonicalize fails (ELOOP / "Too many levels of symbolic links"),
    // refuse rather than letting the failure bubble up later as a
    // confusing warning during discover. We don't run canonicalize on
    // non-symlinks because legitimate overrides may point at locations
    // that don't exist yet (validated downstream).
    let expanded = expand_tilde(path)?;
    if expanded.is_symlink()
        && let Err(e) = std::fs::canonicalize(&expanded)
    {
        anyhow::bail!(
            "override-induced config error from machine.toml\n\
             Conflict: `[directory_overrides.{name}]` path cannot be resolved: {display}\n\
             Why: the path resolves through a broken or looping symlink chain ({e}).\n\
             hint: edit `machine.toml` and point `[directory_overrides.{name}]` at a real directory."
        );
    }

    Ok(())
}

/// Wrap a `Config::validate()` error that was caused by `[directory_overrides.*]`
/// rewriting paths into something invalid. Names `machine.toml` as the file to
/// edit (NOT `tome.toml`) and shows the pre-override vs post-override paths so
/// the user can see what changed.
///
/// Only called from `Config::load_with_overrides` when:
///   - pre-override config validates,
///   - at least one override was applied,
///   - post-override config fails validation.
///
/// PORT-04: the "distinct error class" is achieved by message-content
/// conventions (`override-induced config error from machine.toml`,
/// `directory_overrides`) rather than a typed error variant — matches the
/// existing tome anyhow conventions. If a future caller needs to detect this
/// error programmatically, consider migrating to a typed `OverrideValidationError`
/// enum (tracked as a v1.0 follow-up).
pub(super) fn format_override_validation_error(
    post_err: &anyhow::Error,
    pre_override_paths: &BTreeMap<String, PathBuf>,
    config: &Config,
    machine_path: &Path,
) -> anyhow::Error {
    let mut diff_lines = Vec::new();
    for (name, dir) in &config.directories {
        if dir.override_applied {
            let was = pre_override_paths
                .get(name.as_str())
                .map(|p| p.display().to_string())
                .unwrap_or_else(|| "<unknown>".to_string());
            diff_lines.push(format!(
                "  - {}: {} (was: {}, in tome.toml)",
                name,
                dir.path.display(),
                was,
            ));
        }
    }

    // Indent the original validate() error by 2 spaces so it visually nests
    // under our wrapper text. Use {:#} so anyhow's chained context shows up
    // even for multi-line errors.
    let indented = format!("{post_err:#}")
        .lines()
        .map(|l| format!("  {l}"))
        .collect::<Vec<_>>()
        .join("\n");

    anyhow::anyhow!(
        "override-induced config error from machine.toml\n\
         \n\
         The following directory paths come from `[directory_overrides.<name>]` overrides:\n\
         {}\n\
         \n\
         These overrides made an otherwise-valid `tome.toml` fail validation:\n\
         \n\
         {}\n\
         \n\
         To fix: edit `{}` (NOT tome.toml). Either remove the override(s) above \
         or change them to paths that don't conflict.",
        diff_lines.join("\n"),
        indented,
        machine_path.display(),
    )
}

#[cfg(test)]
mod tests {
    use super::super::types::{
        Config, DirectoryConfig, DirectoryName, DirectoryRole, DirectoryType,
    };
    use std::collections::BTreeMap;
    use std::path::PathBuf;

    /// Build a Config with one Synced directory at the given path. The path
    /// does not need to exist on disk — `validate()` only checks
    /// library_dir vs distribution-dirs overlap, not existence.
    fn config_with_one_dir(name: &str, path: &str) -> Config {
        Config {
            library_dir: PathBuf::from("/tmp/tome-test-lib"),
            directories: BTreeMap::from([(
                DirectoryName::new(name).unwrap(),
                DirectoryConfig {
                    path: PathBuf::from(path),
                    directory_type: DirectoryType::Directory,
                    role: Some(DirectoryRole::Source),
                    git_ref: None,
                    subdir: None,
                    override_applied: false,
                },
            )]),
            ..Default::default()
        }
    }

    fn prefs_with_override(name: &str, path: &str) -> crate::machine::MachinePrefs {
        let mut prefs = crate::machine::MachinePrefs::default();
        prefs.directory_overrides.insert(
            DirectoryName::new(name).unwrap(),
            crate::machine::DirectoryOverride {
                path: PathBuf::from(path),
            },
        );
        prefs
    }

    #[test]
    fn apply_machine_overrides_no_overrides_is_noop() {
        let mut config = config_with_one_dir("x", "/old/path");
        let prefs = crate::machine::MachinePrefs::default();
        config.apply_machine_overrides(&prefs).unwrap();

        let dir = config.directories.get("x").unwrap();
        assert_eq!(dir.path, PathBuf::from("/old/path"));
        assert!(!dir.override_applied);
    }

    #[test]
    fn apply_machine_overrides_replaces_path() {
        let mut config = config_with_one_dir("x", "/old/path");
        let prefs = prefs_with_override("x", "/new/path");
        config.apply_machine_overrides(&prefs).unwrap();

        let dir = config.directories.get("x").unwrap();
        assert_eq!(dir.path, PathBuf::from("/new/path"));
        assert!(dir.override_applied);
    }

    #[test]
    fn apply_machine_overrides_expands_tilde_in_override_path() {
        let mut config = config_with_one_dir("x", "/old/path");
        let prefs = prefs_with_override("x", "~/work");
        config.apply_machine_overrides(&prefs).unwrap();

        let dir = config.directories.get("x").unwrap();
        let path_str = dir.path.to_string_lossy();
        assert!(
            !path_str.starts_with('~'),
            "tilde should be expanded, got: {path_str}"
        );
        assert!(
            path_str.ends_with("/work"),
            "expected path to end with /work, got: {path_str}"
        );
        assert!(dir.override_applied);
    }

    #[test]
    fn apply_machine_overrides_unknown_target_does_not_panic() {
        // PORT-03 (Plan 09-02) will add the warning emission. In Plan 09-01,
        // unknown override targets are a silent no-op — the existing dir
        // is unchanged and override_applied stays false.
        let mut config = config_with_one_dir("x", "/old/path");
        let prefs = prefs_with_override("bogus", "/p");
        config.apply_machine_overrides(&prefs).unwrap();

        let dir = config.directories.get("x").unwrap();
        assert_eq!(dir.path, PathBuf::from("/old/path"));
        assert!(!dir.override_applied);
    }

    #[test]
    fn apply_machine_overrides_idempotent() {
        let mut config = config_with_one_dir("x", "/old/path");
        let prefs = prefs_with_override("x", "/new/path");

        config.apply_machine_overrides(&prefs).unwrap();
        let path_after_first = config.directories.get("x").unwrap().path.clone();

        config.apply_machine_overrides(&prefs).unwrap();
        let path_after_second = config.directories.get("x").unwrap().path.clone();

        assert_eq!(path_after_first, path_after_second);
        assert_eq!(path_after_second, PathBuf::from("/new/path"));
        assert!(config.directories.get("x").unwrap().override_applied);
    }

    #[test]
    fn load_with_overrides_runs_in_order_expand_apply_validate() {
        // Verifies the I2 invariant: override happens AFTER expand_tildes
        // (so `~` in the override path is expanded) and BEFORE validate.
        let tmp = tempfile::TempDir::new().unwrap();
        let cfg_path = tmp.path().join("tome.toml");
        let lib_dir = tmp.path().join("library");
        std::fs::write(
            &cfg_path,
            format!(
                "library_dir = \"{}\"\n\
                 \n\
                 [directories.x]\n\
                 path = \"~/old\"\n\
                 type = \"directory\"\n\
                 role = \"source\"\n",
                lib_dir.display(),
            ),
        )
        .unwrap();

        let prefs = prefs_with_override("x", "~/new");
        let machine_path = tmp.path().join("machine.toml");
        let config = Config::load_with_overrides(&cfg_path, &machine_path, &prefs).unwrap();

        let dir = config.directories.get("x").unwrap();
        let path_str = dir.path.to_string_lossy();
        assert!(
            !path_str.starts_with('~'),
            "tilde in override path should be expanded, got: {path_str}"
        );
        assert!(
            path_str.ends_with("/new"),
            "expected path resolved to <home>/new, got: {path_str}"
        );
        assert!(dir.override_applied);
    }

    #[test]
    fn load_with_overrides_validate_failure_propagates() {
        // Build a config with an invalid role/type combo (Target on a Git
        // type — git directories can only be Source per valid_roles()).
        // load_with_overrides must run validate() and surface the error.
        //
        // Phase 22 / v0.15 note: this test previously used
        // `Directory + Managed` as the invalid combo, but that pairing is
        // now valid (generalized for pfw-style flat-directory package
        // managers). Switched to `Git + Target` which remains
        // structurally invalid (git is a remote-clone source — tome
        // cannot write distribution symlinks into a working tree).
        let tmp = tempfile::TempDir::new().unwrap();
        let cfg_path = tmp.path().join("tome.toml");
        let lib_dir = tmp.path().join("library");
        std::fs::write(
            &cfg_path,
            format!(
                "library_dir = \"{}\"\n\
                 \n\
                 [directories.x]\n\
                 path = \"https://github.com/owner/repo\"\n\
                 type = \"git\"\n\
                 role = \"target\"\n",
                lib_dir.display(),
            ),
        )
        .unwrap();

        let prefs = crate::machine::MachinePrefs::default();
        let machine_path = tmp.path().join("machine.toml");
        let result = Config::load_with_overrides(&cfg_path, &machine_path, &prefs);
        let err = result.unwrap_err().to_string();
        assert!(
            err.contains("role/type conflict") || err.contains("Conflict:"),
            "expected role/type conflict error, got: {err}"
        );
    }

    #[test]
    fn override_applied_field_starts_false_after_load() {
        // No overrides declared → override_applied stays false (default-initialized
        // via #[serde(skip)] + bool::default).
        let tmp = tempfile::TempDir::new().unwrap();
        let cfg_path = tmp.path().join("tome.toml");
        let lib_dir = tmp.path().join("library");
        std::fs::write(
            &cfg_path,
            format!(
                "library_dir = \"{}\"\n\
                 \n\
                 [directories.x]\n\
                 path = \"/some/path\"\n\
                 type = \"directory\"\n\
                 role = \"source\"\n",
                lib_dir.display(),
            ),
        )
        .unwrap();

        let prefs = crate::machine::MachinePrefs::default();
        let machine_path = tmp.path().join("machine.toml");
        let config = Config::load_with_overrides(&cfg_path, &machine_path, &prefs).unwrap();

        let dir = config.directories.get("x").unwrap();
        assert!(
            !dir.override_applied,
            "override_applied should default to false when no overrides declared"
        );
    }

    // === warn_unknown_overrides tests (PORT-03) ===

    #[test]
    fn warn_unknown_overrides_no_overrides_emits_nothing() {
        let config = config_with_one_dir("real", "/old/path");
        let prefs = crate::machine::MachinePrefs::default();

        let mut warnings: Vec<String> = Vec::new();
        config.warn_unknown_overrides(&prefs, |w| warnings.push(w));

        assert!(
            warnings.is_empty(),
            "expected no warnings when directory_overrides is empty, got: {warnings:?}"
        );
    }

    #[test]
    fn warn_unknown_overrides_known_target_emits_nothing() {
        let config = config_with_one_dir("real", "/old/path");
        let prefs = prefs_with_override("real", "/new/path");

        let mut warnings: Vec<String> = Vec::new();
        config.warn_unknown_overrides(&prefs, |w| warnings.push(w));

        assert!(
            warnings.is_empty(),
            "expected no warnings when override target matches a configured directory, got: {warnings:?}"
        );
    }

    #[test]
    fn warn_unknown_overrides_unknown_target_emits_one_warning() {
        let config = config_with_one_dir("real", "/old/path");
        let prefs = prefs_with_override("claud", "/p");

        let mut warnings: Vec<String> = Vec::new();
        config.warn_unknown_overrides(&prefs, |w| warnings.push(w));

        assert_eq!(
            warnings.len(),
            1,
            "expected exactly one warning for the unknown target, got: {warnings:?}"
        );
        assert!(
            warnings[0].contains("claud"),
            "warning should name the typo target 'claud', got: {}",
            warnings[0]
        );
        assert!(
            warnings[0].contains("machine.toml"),
            "warning should reference machine.toml as the source file, got: {}",
            warnings[0]
        );
        assert!(
            warnings[0].contains("directory_overrides") || warnings[0].contains("override"),
            "warning should mention 'directory_overrides' or 'override', got: {}",
            warnings[0]
        );
    }

    #[test]
    fn warn_unknown_overrides_multiple_unknowns_emit_one_each() {
        let config = config_with_one_dir("real", "/old/path");
        let mut prefs = crate::machine::MachinePrefs::default();
        // Insert in reverse alphabetical order to verify BTreeMap iteration is alphabetical.
        prefs.directory_overrides.insert(
            DirectoryName::new("b").unwrap(),
            crate::machine::DirectoryOverride {
                path: PathBuf::from("/b"),
            },
        );
        prefs.directory_overrides.insert(
            DirectoryName::new("a").unwrap(),
            crate::machine::DirectoryOverride {
                path: PathBuf::from("/a"),
            },
        );

        let mut warnings: Vec<String> = Vec::new();
        config.warn_unknown_overrides(&prefs, |w| warnings.push(w));

        assert_eq!(
            warnings.len(),
            2,
            "expected one warning per unknown target, got: {warnings:?}"
        );
        // BTreeMap iteration is alphabetical, so warnings should be in 'a', 'b' order.
        assert!(
            warnings[0].contains("'a'"),
            "first warning should name 'a' (alphabetical order), got: {}",
            warnings[0]
        );
        assert!(
            warnings[1].contains("'b'"),
            "second warning should name 'b' (alphabetical order), got: {}",
            warnings[1]
        );
    }

    #[test]
    fn warn_unknown_overrides_does_not_mutate_config() {
        // The helper takes &self (not &mut self) — verifying via a snapshot that
        // calling it leaves the config unchanged. Compile-time signature check is
        // the primary guard; this test is a defense-in-depth runtime check.
        let config = config_with_one_dir("real", "/old/path");
        let snapshot = config.clone();
        let prefs = prefs_with_override("claud", "/p");

        let mut warnings: Vec<String> = Vec::new();
        config.warn_unknown_overrides(&prefs, |w| warnings.push(w));

        // Compare path of the only directory to confirm no mutation.
        let original_path = snapshot.directories.get("real").unwrap().path.clone();
        let after_path = config.directories.get("real").unwrap().path.clone();
        assert_eq!(
            original_path, after_path,
            "warn_unknown_overrides must not mutate config paths"
        );
        assert_eq!(
            snapshot.directories.len(),
            config.directories.len(),
            "warn_unknown_overrides must not mutate directory count"
        );
    }

    // === load_with_overrides PORT-04 wrapping tests ===

    #[test]
    fn load_with_overrides_pre_override_invalid_returns_raw_error() {
        // tome.toml ALREADY invalid (library_dir == directories.work.path);
        // no overrides applied. The raw `validate()` error must surface as-is —
        // no machine.toml wrapper, since removing the (empty) override set
        // would not have fixed anything.
        let tmp = tempfile::TempDir::new().unwrap();
        let cfg_path = tmp.path().join("tome.toml");
        let shared = tmp.path().join("shared");
        std::fs::write(
            &cfg_path,
            format!(
                "library_dir = \"{shared}\"\n\
                 \n\
                 [directories.work]\n\
                 path = \"{shared}\"\n\
                 type = \"directory\"\n\
                 role = \"synced\"\n",
                shared = shared.display(),
            ),
        )
        .unwrap();

        let prefs = crate::machine::MachinePrefs::default();
        let machine_path = tmp.path().join("machine.toml");
        let err = Config::load_with_overrides(&cfg_path, &machine_path, &prefs)
            .unwrap_err()
            .to_string();
        assert!(
            err.contains("library_dir") && err.contains("overlaps"),
            "expected raw library_dir overlap error, got: {err}"
        );
        assert!(
            !err.contains("override-induced"),
            "raw error must not be wrapped — pre-override config was already invalid: {err}"
        );
        assert!(
            !err.contains("machine.toml"),
            "pre-existing tome.toml errors should not name machine.toml: {err}"
        );
    }

    #[test]
    fn load_with_overrides_override_induces_invalid_returns_wrapped_error() {
        // tome.toml is VALID (library_dir != directories.work.path); the
        // override rewrites work.path to equal library_dir, breaking
        // validate(). The error must be wrapped with the machine.toml class.
        let tmp = tempfile::TempDir::new().unwrap();
        let cfg_path = tmp.path().join("tome.toml");
        let lib_dir = tmp.path().join("library");
        let work_dir = tmp.path().join("work");
        std::fs::write(
            &cfg_path,
            format!(
                "library_dir = \"{}\"\n\
                 \n\
                 [directories.work]\n\
                 path = \"{}\"\n\
                 type = \"directory\"\n\
                 role = \"synced\"\n",
                lib_dir.display(),
                work_dir.display(),
            ),
        )
        .unwrap();

        let prefs = prefs_with_override("work", lib_dir.to_str().unwrap());
        let machine_path = tmp.path().join("machine.toml");
        let err = Config::load_with_overrides(&cfg_path, &machine_path, &prefs)
            .unwrap_err()
            .to_string();

        assert!(
            err.contains("machine.toml"),
            "wrapped error must name machine.toml, got: {err}"
        );
        assert!(
            err.contains("override-induced") || err.contains("directory_overrides"),
            "wrapped error must identify override origin, got: {err}"
        );
        assert!(
            err.contains("library_dir") && err.contains("overlaps"),
            "wrapped error must preserve the original validate() text, got: {err}"
        );
        // Negative assertion: the wrapper must NOT direct the user to edit tome.toml.
        assert!(
            !err.contains("edit tome.toml") && !err.contains("Edit tome.toml"),
            "wrapped error must NOT direct user to edit tome.toml, got: {err}"
        );
    }

    #[test]
    fn load_with_overrides_override_unrelated_to_failure_returns_raw_error() {
        // tome.toml is invalid (library_dir == work.path) AND machine.toml has
        // an override targeting an UNRELATED directory name (typo).
        // Discriminator: removing the override would not fix tome.toml, so
        // the raw error passes through (unwrapped).
        let tmp = tempfile::TempDir::new().unwrap();
        let cfg_path = tmp.path().join("tome.toml");
        let shared = tmp.path().join("shared");
        std::fs::write(
            &cfg_path,
            format!(
                "library_dir = \"{shared}\"\n\
                 \n\
                 [directories.work]\n\
                 path = \"{shared}\"\n\
                 type = \"directory\"\n\
                 role = \"synced\"\n",
                shared = shared.display(),
            ),
        )
        .unwrap();

        // Override 'unrelated' is a typo — does not match any configured directory.
        // It will produce a typo warning AND not affect any path, so post-override
        // failure has the same root cause as pre-override.
        let prefs = prefs_with_override("unrelated", "/elsewhere");
        let machine_path = tmp.path().join("machine.toml");
        let err = Config::load_with_overrides(&cfg_path, &machine_path, &prefs)
            .unwrap_err()
            .to_string();

        assert!(
            err.contains("library_dir") && err.contains("overlaps"),
            "expected raw library_dir overlap error, got: {err}"
        );
        assert!(
            !err.contains("override-induced"),
            "raw error must not be wrapped when override is unrelated to failure: {err}"
        );
    }

    #[test]
    fn load_with_overrides_path_appears_in_wrapper_message() {
        // Wrapped message must include the override target name, the new path,
        // AND the old (pre-override) path so the user can see the diff.
        let tmp = tempfile::TempDir::new().unwrap();
        let cfg_path = tmp.path().join("tome.toml");
        let lib_dir = tmp.path().join("library");
        let work_dir = tmp.path().join("work-original");
        std::fs::write(
            &cfg_path,
            format!(
                "library_dir = \"{}\"\n\
                 \n\
                 [directories.work]\n\
                 path = \"{}\"\n\
                 type = \"directory\"\n\
                 role = \"synced\"\n",
                lib_dir.display(),
                work_dir.display(),
            ),
        )
        .unwrap();

        let prefs = prefs_with_override("work", lib_dir.to_str().unwrap());
        let machine_path = tmp.path().join("machine.toml");
        let err = Config::load_with_overrides(&cfg_path, &machine_path, &prefs)
            .unwrap_err()
            .to_string();

        assert!(
            err.contains("work"),
            "wrapper must name the override target 'work', got: {err}"
        );
        assert!(
            err.contains(lib_dir.to_str().unwrap()),
            "wrapper must include the new (override) path, got: {err}"
        );
        assert!(
            err.contains("work-original"),
            "wrapper must include the old (pre-override) path, got: {err}"
        );
    }
}
