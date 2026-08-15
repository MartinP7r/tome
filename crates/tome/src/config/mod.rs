//! TOML configuration loading, saving, and validation. Handles tilde expansion and default paths.
//!
//! v0.6: Unified directory model — replaces separate `[[sources]]` and `[targets.*]`
//! with a single `[directories.*]` config.
//!
//! ## Module layout (v0.10 / Plan 15-02)
//!
//! Split out of the previous 3,122-LOC `config.rs` for reviewability:
//!
//! | File           | Hosts                                                            |
//! |----------------|------------------------------------------------------------------|
//! | `mod.rs`       | Public re-exports + `Config::load`/`load_or_default`/`save`/`save_checked`/`load_with_overrides` + tome-home/XDG-config helpers (`default_tome_home`, `default_config_path`, `resolve_config_dir`, `TomeHomeSource`, `resolve_tome_home_with_source`, `read_config_tome_home`, `write_xdg_tome_home`) + `defaults` |
//! | `types.rs`     | `Config`, `DirectoryName`, `DirectoryConfig`, `DirectoryType`, `DirectoryRole`, `GitRef`, `BackupConfig` (data shapes + derive impls only) |
//! | `validate.rs`  | `Config::validate` — role/type combos + Cases A/B/C overlap detection |
//! | `overrides.rs` | `Config::apply_machine_overrides`, `warn_unknown_overrides`, `format_override_validation_error` (PORT-01..05 path overrides) |
//!
//! Tilde helpers (`expand_tilde`, `unexpand_tilde`) live in [`crate::paths`] —
//! cross-cutting utilities, not config-specific. They are re-exported here
//! (`pub use crate::paths::expand_tilde`) so existing `crate::config::expand_tilde`
//! call sites continue to compile byte-identically.

use anyhow::{Context, Result};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use crate::errors::{DomainErrorKind, WithDomainKind};

mod overrides;
mod types;
mod validate;

// Re-export the public API surface so external callers continue to use
// `crate::config::Foo` paths byte-identically with the pre-split config.rs.
pub use crate::paths::expand_tilde;
pub use types::{
    BackupConfig, Config, DirectoryConfig, DirectoryName, DirectoryRole, DirectoryType, GitRef,
};

use crate::machine::MachinePrefs;
use overrides::format_override_validation_error;

impl Config {
    /// Load config from file, or return defaults if file doesn't exist.
    ///
    /// When parsing fails, checks for old-format keys and appends a migration hint.
    pub fn load(path: &Path) -> Result<Self> {
        if path.exists() {
            let content = std::fs::read_to_string(path)
                .with_context(|| format!("failed to read {}", path.display()))?;
            let mut config: Config = toml::from_str(&content).map_err(|e| {
                let mut msg = format!("failed to parse {}: {e}", path.display());
                if content.contains("[[sources]]") || content.contains("[targets.") {
                    msg.push_str("\nhint: tome v0.6 replaced [[sources]] and [targets.*] with [directories.*]. See CHANGELOG.md for migration instructions.");
                }
                anyhow::anyhow!("{msg}")
            })?;
            config.expand_tildes()?;
            config.validate()?;
            Ok(config)
        } else {
            let mut config = Self::default();
            config.expand_tildes()?;
            Ok(config)
        }
    }

    /// Load from CLI-provided path or default location.
    ///
    /// When an explicit path is provided and its parent directory does not
    /// exist, this is treated as a configuration error (likely a typo).
    /// A missing file in an existing directory is fine -- first-run scenario.
    pub fn load_or_default(cli_path: Option<&Path>) -> Result<Self> {
        let path = match cli_path {
            Some(p) => {
                if !p.exists() {
                    let parent_exists = p.parent().is_some_and(|d| d.exists());
                    if !parent_exists {
                        // CORE-05 / D-14: a bad explicit `--config` path (parent
                        // dir missing — likely a typo) carries the `NotFound`
                        // sentinel for the GUI boundary. Transparent tag — the
                        // CLI's `config file not found: <path>` message is
                        // unchanged. A missing file in an existing dir is still
                        // tolerated (first-run) and produces no error here.
                        return Err(anyhow::anyhow!("config file not found: {}", p.display()))
                            .with_domain_kind(DomainErrorKind::NotFound);
                    }
                }
                p.to_path_buf()
            }
            None => default_config_path()?,
        };
        Self::load(&path)
    }

    /// Save config to file, creating parent directories as needed.
    ///
    /// HARD-08: atomic write via temp+rename. Mirrors `Manifest::save`,
    /// `Lockfile::save`, and `MachinePrefs::save`. A failure at the
    /// rename step leaves the previous on-disk content intact.
    pub fn save(&self, path: &Path) -> Result<()> {
        let content = toml::to_string_pretty(self).context("failed to serialize config")?;
        atomic_write_toml(path, &content)
    }

    /// Read-only accessors for the `pub(crate)` fields.
    ///
    /// External-crate consumers (integration tests, future library APIs) cannot
    /// reach `pub(crate)` fields, so these methods expose `&T` views without
    /// widening field visibility or forcing a clone.
    pub fn directories(&self) -> &BTreeMap<DirectoryName, DirectoryConfig> {
        &self.directories
    }

    pub fn library_dir(&self) -> &Path {
        &self.library_dir
    }

    pub fn exclude(&self) -> &std::collections::BTreeSet<crate::discover::SkillName> {
        &self.exclude
    }

    /// Directories that participate in discovery (Managed, Synced, Source roles).
    pub fn discovery_dirs(&self) -> impl Iterator<Item = (&DirectoryName, &DirectoryConfig)> {
        self.directories
            .iter()
            .filter(|(_, dir)| dir.role().is_discovery())
    }

    /// Directories that participate in distribution (Synced, Target roles).
    pub fn distribution_dirs(&self) -> impl Iterator<Item = (&DirectoryName, &DirectoryConfig)> {
        self.directories
            .iter()
            .filter(|(_, dir)| dir.role().is_distribution())
    }

    /// Directories with Managed role only.
    pub fn managed_dirs(&self) -> impl Iterator<Item = (&DirectoryName, &DirectoryConfig)> {
        self.directories
            .iter()
            .filter(|(_, dir)| dir.role() == DirectoryRole::Managed)
    }

    /// Expand `~` in all path fields.
    pub(crate) fn expand_tildes(&mut self) -> Result<()> {
        self.library_dir = expand_tilde(&self.library_dir)?;
        for dir in self.directories.values_mut() {
            dir.path = expand_tilde(&dir.path)?;
        }
        Ok(())
    }

    /// Load config and apply per-machine path overrides in one shot.
    ///
    /// **Order (I2 invariant — must not change):**
    ///   1. Read TOML from `path` (or build defaults if missing — same as `Config::load`)
    ///   2. `expand_tildes()` on the raw config
    ///   3. `warn_unknown_overrides(prefs)` — stderr typo guard (PORT-03)
    ///   4. snapshot pre-override paths (for the PORT-04 wrapper)
    ///   5. `apply_machine_overrides(prefs)` — rewrites paths per `[directory_overrides.<name>]`
    ///   6. `validate()` — sees the merged result; if it fails AND the pre-override
    ///      config DID validate AND ≥ 1 override was applied, the error is wrapped
    ///      via `format_override_validation_error` so the user knows to edit
    ///      `machine.toml`, not `tome.toml` (PORT-04). Otherwise the raw
    ///      `validate()` error passes through.
    ///
    /// `machine_path` is the path to `machine.toml`; only used to build the
    /// PORT-04 wrapper message ("To fix: edit `<machine_path>`"). The prefs
    /// themselves come from `prefs`, not by re-reading the file.
    ///
    /// Used by `lib.rs::run()` for every non-Init command. `tome init` does NOT use
    /// this path — the wizard runs against the bare `tome.toml` that the user is
    /// about to write.
    pub fn load_with_overrides(
        path: &Path,
        machine_path: &Path,
        prefs: &MachinePrefs,
    ) -> Result<Self> {
        let mut config = if path.exists() {
            let content = std::fs::read_to_string(path)
                .with_context(|| format!("failed to read {}", path.display()))?;
            toml::from_str(&content).map_err(|e| {
                let mut msg = format!("failed to parse {}: {e}", path.display());
                if content.contains("[[sources]]") || content.contains("[targets.") {
                    msg.push_str("\nhint: tome v0.6 replaced [[sources]] and [targets.*] with [directories.*]. See CHANGELOG.md for migration instructions.");
                }
                anyhow::anyhow!("{msg}")
            })?
        } else {
            Self::default()
        };

        config.expand_tildes()?;

        // PORT-03: warn about typos before applying. Apply is a silent no-op
        // for unknown targets (see `apply_machine_overrides` doc), so the user
        // would otherwise lose their override silently.
        config.warn_unknown_overrides(prefs, |w| eprintln!("warning: {w}"));

        // PORT-04 setup: snapshot pre-override paths so we can both (a)
        // discriminate "override-induced" failure from a pre-existing tome.toml
        // problem and (b) show the user what changed in the wrapper message.
        let pre_override_paths: BTreeMap<String, PathBuf> = config
            .directories
            .iter()
            .map(|(name, dir)| (name.as_str().to_string(), dir.path.clone()))
            .collect();

        config.apply_machine_overrides(prefs)?;

        if let Err(post_err) = config.validate() {
            // Only wrap if the pre-override config WOULD have validated AND at
            // least one override was applied. Otherwise blaming machine.toml
            // would be wrong — the underlying tome.toml is what's broken.
            let mut pre_override_config = config.clone();
            for (name, dir) in pre_override_config.directories.iter_mut() {
                if let Some(orig) = pre_override_paths.get(name.as_str()) {
                    dir.path = orig.clone();
                    dir.override_applied = false;
                }
            }
            let pre_override_valid = pre_override_config.validate().is_ok();
            let any_override_applied = config.directories.values().any(|d| d.override_applied);

            if pre_override_valid && any_override_applied {
                return Err(format_override_validation_error(
                    &post_err,
                    &pre_override_paths,
                    &config,
                    machine_path,
                ));
            }
            return Err(post_err);
        }
        Ok(config)
    }

    /// CLI-aware variant of `load_with_overrides`. See `load_or_default` for the
    /// missing-file vs. missing-parent-dir semantics.
    pub fn load_or_default_with_overrides(
        cli_path: Option<&Path>,
        machine_path: &Path,
        prefs: &MachinePrefs,
    ) -> Result<Self> {
        let path = match cli_path {
            Some(p) => {
                if !p.exists() {
                    let parent_exists = p.parent().is_some_and(|d| d.exists());
                    anyhow::ensure!(parent_exists, "config file not found: {}", p.display());
                }
                p.to_path_buf()
            }
            None => default_config_path()?,
        };
        Self::load_with_overrides(&path, machine_path, prefs)
    }

    /// Save config, but first run the same expand + validate pipeline that
    /// `Config::load()` runs, followed by a TOML round-trip equality check
    /// (defense in depth — catches serde drift such as a field that
    /// accidentally disappears across a serialize/deserialize cycle).
    ///
    /// **HARD-22 / D-TILDE-1 (Plan 15-02):** path fields under `$HOME` are
    /// rewritten to `~/`-shape on serialise, so a `tome.toml` checked into
    /// dotfiles stays portable across machines. Already-tilde inputs survive
    /// unchanged (idempotent); paths outside `$HOME` are kept absolute. The
    /// rewrite operates on a serialisation-only clone so the caller's
    /// `Config` is not mutated. Behaviour table:
    ///
    /// ```text
    /// IN: library_dir = "~/skills"             OUT: library_dir = "~/skills"
    /// IN: library_dir = "/Users/martin/skills" OUT: library_dir = "~/skills"
    /// IN: library_dir = "/var/lib/skills"      OUT: library_dir = "/var/lib/skills"
    /// ```
    ///
    /// **PORT-02 invariant:** `apply_machine_overrides` mutates a load-time-only
    /// copy of `Config`. `save_checked` operates on `&self` (the unmutated
    /// config) — therefore override paths from `machine.toml` are NEVER
    /// serialised back to `tome.toml`. The call-site contract in `lib.rs`
    /// guarantees that the Config passed to `save_checked` is the pre-override
    /// shape; this method does not re-implement that guarantee.
    ///
    /// Returns `Err` without writing anything if any stage fails.
    ///
    /// Call this instead of `save()` from the wizard or any other code that
    /// produces a Config in-memory rather than loading it from disk.
    pub fn save_checked(&self, path: &Path) -> Result<()> {
        // 1. Validation copy: validate() needs absolute paths to detect overlaps,
        //    so build an expanded clone for the check. The caller's Config is
        //    never mutated.
        let mut expanded = self.clone();
        expanded.expand_tildes()?;
        expanded.validate()?;

        // 2. Serialisation copy (D-TILDE-1): rewrite every path field under
        //    `$HOME` to `~/`-shape via paths::unexpand_tilde. Already-tilde
        //    inputs survive unchanged (idempotent); paths outside `$HOME`
        //    are kept absolute. We start from `self` (not `expanded`) so
        //    user-supplied tildes are preserved verbatim, not round-tripped
        //    through expansion. This also keeps the PORT-02 invariant: any
        //    overrides applied to `self.directories[*].path` would be the
        //    caller's responsibility to undo before passing to save_checked
        //    (lib.rs::sync save chain saves the pre-override Config).
        let mut for_save = self.clone();
        for_save.library_dir = crate::paths::unexpand_tilde(&for_save.library_dir);
        for dir in for_save.directories.values_mut() {
            dir.path = crate::paths::unexpand_tilde(&dir.path);
        }

        // 3. TOML round-trip: serialize, parse back, re-serialize, compare the
        //    two TOML strings for byte equality. If they differ, a field has
        //    been silently dropped or rewritten by serde.
        let emitted =
            toml::to_string_pretty(&for_save).context("failed to serialize config (pre-check)")?;
        let reparsed: Config =
            toml::from_str(&emitted).context("round-trip: generated TOML did not reparse")?;
        let reemitted =
            toml::to_string_pretty(&reparsed).context("failed to serialize config (round-trip)")?;
        anyhow::ensure!(
            emitted == reemitted,
            "round-trip mismatch: serialized config differs after parse+reserialize — this is a serde bug in a tome type, not a user error.\n\
             Conflict: emit/reparse produced different TOML\n\
             Why: a field is not reversibly (de)serializable; saving would lose data.\n\
             hint: report this as a tome bug and share the generated output below.\n\
             --- first emit ---\n{emitted}\n--- second emit ---\n{reemitted}"
        );

        // 4. Safe to save — write the same bytes we verified, atomically.
        // HARD-08: temp+rename so a crash mid-rename preserves the prior
        // on-disk tome.toml (the regression test pins this contract).
        atomic_write_toml(path, &emitted)
    }
}

/// HARD-08: atomic-write helper used by both `Config::save` and
/// `Config::save_checked`. Mirrors the pattern in `manifest::save`,
/// `lockfile::save`, and `machine::save`: write to a sibling
/// `.toml.tmp` file, then rename. A failure at the rename step
/// leaves the previous file content intact.
fn atomic_write_toml(path: &Path, content: &str) -> Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("failed to create {}", parent.display()))?;
    }
    let tmp_path = path.with_extension("toml.tmp");
    std::fs::write(&tmp_path, content)
        .with_context(|| format!("failed to write temp file {}", tmp_path.display()))?;
    if let Err(e) = std::fs::rename(&tmp_path, path) {
        // Best-effort cleanup so a stale `.toml.tmp` doesn't accumulate
        // after a failed save (e.g. read-only target). Ignore the cleanup
        // result on purpose: the rename error is the real failure to
        // surface; masking it with a cleanup error would hide the cause.
        let _ = std::fs::remove_file(&tmp_path);
        return Err(e).with_context(|| {
            format!(
                "failed to rename {} -> {}",
                tmp_path.display(),
                path.display()
            )
        });
    }
    Ok(())
}

// =============================================================================
// tome_home / XDG config helpers
// =============================================================================
//
// These are not strictly Config-related but live in this module because every
// `tome` invocation needs to resolve "where on disk does the config live?"
// before it can call `Config::load`. Splitting them into a separate module
// would create a circular dependency (paths.rs → config::resolve_config_dir
// → ?). They stay here.

/// Default tome home directory.
///
/// Resolution order:
/// 1. `TOME_HOME` environment variable (if set and non-empty)
/// 2. `~/.config/tome/config.toml` -> `tome_home` field
/// 3. `~/.tome/`
pub fn default_tome_home() -> Result<PathBuf> {
    // 1. TOME_HOME env var
    match std::env::var("TOME_HOME") {
        Ok(val) if !val.is_empty() => return expand_tilde(Path::new(&val)),
        Ok(_) => {}                               // empty string, fall through
        Err(std::env::VarError::NotPresent) => {} // not set, fall through
        Err(std::env::VarError::NotUnicode(_)) => {
            anyhow::bail!("TOME_HOME environment variable contains invalid Unicode");
        }
    }
    // 2. ~/.config/tome/config.toml
    if let Some(path) = read_config_tome_home()? {
        return Ok(path);
    }
    // 3. Default
    Ok(dirs::home_dir()
        .context("could not determine home directory")?
        .join(".tome"))
}

/// Read `tome_home` from the machine-level config at `~/.config/tome/config.toml`.
pub(crate) fn read_config_tome_home() -> Result<Option<PathBuf>> {
    let config_path = dirs::home_dir()
        .context("could not determine home directory")?
        .join(".config/tome/config.toml");
    if !config_path.is_file() {
        return Ok(None);
    }
    let content = std::fs::read_to_string(&config_path)
        .with_context(|| format!("failed to read {}", config_path.display()))?;
    let table: toml::Table = content
        .parse()
        .with_context(|| format!("invalid TOML in {}", config_path.display()))?;
    match table.get("tome_home") {
        Some(toml::Value::String(val)) => {
            let expanded = expand_tilde(Path::new(val))?;
            Ok(Some(expanded))
        }
        Some(_) => anyhow::bail!("tome_home in {} must be a string", config_path.display()),
        None => Ok(None),
    }
}

/// Write (merge) `tome_home = <collapsed-path>` into `~/.config/tome/config.toml`.
///
/// Semantics:
/// - If the file does not exist: create parent dir, write a new TOML with just `tome_home`.
/// - If the file exists: parse as `toml::Table`, insert/overwrite the `tome_home` key,
///   preserve all other keys, write back. Comments are NOT preserved (toml crate limitation).
/// - The value is stored in `~/`-collapsed form (via `paths::collapse_home_path`) so the
///   file is portable across machines. `read_config_tome_home` tilde-expands on read.
/// - Write is atomic via temp+rename, matching the pattern in `machine.rs` / `lockfile.rs`.
///
/// Used by the wizard Step 0 (WUX-05) when the user chose a custom `tome_home` and
/// accepted the persist-prompt.
pub(crate) fn write_xdg_tome_home(tome_home: &Path) -> Result<()> {
    let home = dirs::home_dir().context("could not determine home directory")?;
    let path = home.join(".config/tome/config.toml");

    let mut table: toml::Table = if path.is_file() {
        std::fs::read_to_string(&path)
            .with_context(|| format!("failed to read {}", path.display()))?
            .parse()
            .with_context(|| format!("invalid TOML in {}", path.display()))?
    } else {
        toml::Table::new()
    };

    let collapsed = crate::paths::collapse_home_path(tome_home);
    table.insert(
        "tome_home".into(),
        toml::Value::String(collapsed.to_string_lossy().into_owned()),
    );

    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("failed to create {}", parent.display()))?;
    }

    let tmp = path.with_extension("toml.tmp");
    let content = toml::to_string_pretty(&table).context("serialize XDG config")?;
    std::fs::write(&tmp, &content).with_context(|| format!("failed to write {}", tmp.display()))?;
    std::fs::rename(&tmp, &path)
        .with_context(|| format!("failed to rename {} -> {}", tmp.display(), path.display()))?;
    Ok(())
}

/// Resolve the config directory for a given tome home.
///
/// Uses smart detection: if `tome_home/.tome/tome.toml` exists, config lives
/// in the `.tome/` subdirectory (custom repo layout). Otherwise, config lives
/// at the tome_home root (default layout).
pub fn resolve_config_dir(tome_home: &Path) -> PathBuf {
    let subdir = tome_home.join(".tome");
    if subdir.join("tome.toml").exists() {
        subdir
    } else {
        tome_home.to_path_buf()
    }
}

/// Default config file path, using smart detection.
///
/// For default `~/.tome/`: returns `~/.tome/tome.toml` (backwards compatible).
/// For custom repos with `.tome/` subdir: returns `<tome_home>/.tome/tome.toml`.
pub fn default_config_path() -> Result<PathBuf> {
    let home = default_tome_home()?;
    Ok(resolve_config_dir(&home).join("tome.toml"))
}

/// Where the resolved `tome_home` came from in the resolution chain.
///
/// Used by the `tome init` WUX-04 info line to tell the user which branch
/// produced the path they are about to populate (e.g. "from TOME_HOME env"
/// vs "from default"). Also used by the wizard to decide whether to prompt
/// for a custom tome_home on greenfield (WUX-01 gates on Default).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum TomeHomeSource {
    /// Provided via the `--tome-home` CLI flag.
    CliTomeHome,
    /// Derived from the `--config` CLI flag (tome_home = parent of config file).
    CliConfig,
    /// Picked up from the `TOME_HOME` environment variable.
    EnvVar,
    /// Read from `~/.config/tome/config.toml` `tome_home` key.
    XdgConfig,
    /// No signal provided — falling back to `~/.tome/`.
    Default,
}

impl TomeHomeSource {
    /// Short, user-facing label describing which branch produced this `tome_home`.
    ///
    /// These exact strings are asserted by the WUX-04 integration tests; any
    /// change here will also need to flow through those tests.
    pub(crate) fn label(self) -> &'static str {
        match self {
            Self::CliTomeHome => "--tome-home flag",
            Self::CliConfig => "--config flag",
            Self::EnvVar => "TOME_HOME env",
            Self::XdgConfig => "~/.config/tome/config.toml",
            Self::Default => "default",
        }
    }
}

/// Like [`crate::resolve_tome_home`] but also reports the resolution source.
///
/// Used by the `tome init` entry point to print the WUX-04 info line and
/// (via later plans) to gate the greenfield tome_home prompt on `Default`.
///
/// Resolution order mirrors [`resolve_tome_home`](crate::resolve_tome_home)
/// and [`default_tome_home`] exactly, split apart so each branch is attributable:
/// 1. `--tome-home` flag (`CliTomeHome`)
/// 2. `--config` flag (`CliConfig`; tome_home = parent of config file)
/// 3. `TOME_HOME` env var, non-empty (`EnvVar`)
/// 4. `~/.config/tome/config.toml` `tome_home` key (`XdgConfig`)
/// 5. `~/.tome/` (`Default`)
pub(crate) fn resolve_tome_home_with_source(
    cli_tome_home: Option<&Path>,
    cli_config: Option<&Path>,
) -> Result<(PathBuf, TomeHomeSource)> {
    if let Some(p) = cli_tome_home {
        let expanded = expand_tilde(p)?;
        anyhow::ensure!(
            expanded.is_absolute(),
            "--tome-home path '{}' must be an absolute path",
            p.display()
        );
        return Ok((expanded, TomeHomeSource::CliTomeHome));
    }
    if let Some(p) = cli_config {
        anyhow::ensure!(
            p.is_absolute(),
            "config path '{}' must be an absolute path",
            p.display()
        );
        let parent = p.parent().context("config path has no parent directory")?;
        return Ok((parent.to_path_buf(), TomeHomeSource::CliConfig));
    }
    match std::env::var("TOME_HOME") {
        Ok(val) if !val.is_empty() => {
            return Ok((expand_tilde(Path::new(&val))?, TomeHomeSource::EnvVar));
        }
        Ok(_) => {}
        Err(std::env::VarError::NotPresent) => {}
        Err(std::env::VarError::NotUnicode(_)) => {
            anyhow::bail!("TOME_HOME environment variable contains invalid Unicode");
        }
    }
    if let Some(path) = read_config_tome_home()? {
        return Ok((path, TomeHomeSource::XdgConfig));
    }
    Ok((
        dirs::home_dir()
            .context("could not determine home directory")?
            .join(".tome"),
        TomeHomeSource::Default,
    ))
}

// =============================================================================
// DEPRECATED COMPATIBILITY SHIMS
// =============================================================================
// These types exist only to keep other modules compiling during the v0.6
// NOTE: Deprecated Source, SourceType, TargetName, TargetConfig, TargetMethod
// types were removed as part of v0.6 unified directory migration (plan 01-05).

pub(super) mod defaults {
    use std::path::PathBuf;

    pub fn library_dir() -> PathBuf {
        // Best-effort default for serde; expand_tildes() and validate() will
        // surface a proper error if home is unavailable.
        // Uses TOME_HOME if set, otherwise ~/.tome/
        super::default_tome_home()
            .unwrap_or_else(|_| {
                dirs::home_dir()
                    .unwrap_or_else(|| PathBuf::from("~"))
                    .join(".tome")
            })
            .join("skills")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::discover::SkillName;
    use std::collections::BTreeMap;

    // --- Convenience iterator tests ---

    #[test]
    fn discovery_dirs_returns_managed_synced_source() {
        let config = Config {
            directories: BTreeMap::from([
                (
                    DirectoryName::new("a-managed").unwrap(),
                    DirectoryConfig {
                        path: PathBuf::from("/tmp/a"),
                        directory_type: DirectoryType::ClaudePlugins,
                        role: Some(DirectoryRole::Managed),
                        git_ref: None,

                        subdir: None,
                        override_applied: false,
                    },
                ),
                (
                    DirectoryName::new("b-synced").unwrap(),
                    DirectoryConfig {
                        path: PathBuf::from("/tmp/b"),
                        directory_type: DirectoryType::Directory,
                        role: Some(DirectoryRole::Synced),
                        git_ref: None,

                        subdir: None,
                        override_applied: false,
                    },
                ),
                (
                    DirectoryName::new("c-source").unwrap(),
                    DirectoryConfig {
                        path: PathBuf::from("/tmp/c"),
                        directory_type: DirectoryType::Directory,
                        role: Some(DirectoryRole::Source),
                        git_ref: None,

                        subdir: None,
                        override_applied: false,
                    },
                ),
                (
                    DirectoryName::new("d-target").unwrap(),
                    DirectoryConfig {
                        path: PathBuf::from("/tmp/d"),
                        directory_type: DirectoryType::Directory,
                        role: Some(DirectoryRole::Target),
                        git_ref: None,

                        subdir: None,
                        override_applied: false,
                    },
                ),
            ]),
            ..Default::default()
        };

        let discovery: Vec<&str> = config.discovery_dirs().map(|(n, _)| n.as_str()).collect();
        assert_eq!(discovery, vec!["a-managed", "b-synced", "c-source"]);
    }

    #[test]
    fn distribution_dirs_returns_synced_target() {
        let config = Config {
            directories: BTreeMap::from([
                (
                    DirectoryName::new("a-managed").unwrap(),
                    DirectoryConfig {
                        path: PathBuf::from("/tmp/a"),
                        directory_type: DirectoryType::ClaudePlugins,
                        role: Some(DirectoryRole::Managed),
                        git_ref: None,

                        subdir: None,
                        override_applied: false,
                    },
                ),
                (
                    DirectoryName::new("b-synced").unwrap(),
                    DirectoryConfig {
                        path: PathBuf::from("/tmp/b"),
                        directory_type: DirectoryType::Directory,
                        role: Some(DirectoryRole::Synced),
                        git_ref: None,

                        subdir: None,
                        override_applied: false,
                    },
                ),
                (
                    DirectoryName::new("c-source").unwrap(),
                    DirectoryConfig {
                        path: PathBuf::from("/tmp/c"),
                        directory_type: DirectoryType::Directory,
                        role: Some(DirectoryRole::Source),
                        git_ref: None,

                        subdir: None,
                        override_applied: false,
                    },
                ),
                (
                    DirectoryName::new("d-target").unwrap(),
                    DirectoryConfig {
                        path: PathBuf::from("/tmp/d"),
                        directory_type: DirectoryType::Directory,
                        role: Some(DirectoryRole::Target),
                        git_ref: None,

                        subdir: None,
                        override_applied: false,
                    },
                ),
            ]),
            ..Default::default()
        };

        let distribution: Vec<&str> = config
            .distribution_dirs()
            .map(|(n, _)| n.as_str())
            .collect();
        assert_eq!(distribution, vec!["b-synced", "d-target"]);
    }

    // --- Config load tests ---

    #[test]
    fn config_loads_defaults_when_file_missing() {
        let config = Config::load(Path::new("/nonexistent/path/config.toml")).unwrap();
        assert!(config.directories.is_empty());
    }

    #[test]
    fn load_or_default_errors_when_parent_dir_missing() {
        let result = Config::load_or_default(Some(Path::new("/nonexistent/config.toml")));
        assert!(result.is_err());
        let msg = result.unwrap_err().to_string();
        assert!(msg.contains("config file not found"), "got: {msg}");
    }

    #[test]
    fn load_or_default_returns_defaults_when_parent_exists() {
        let tmp = tempfile::TempDir::new().unwrap();
        let missing_file = tmp.path().join("config.toml");
        let config = Config::load_or_default(Some(&missing_file)).unwrap();
        assert!(config.directories.is_empty());
    }

    #[test]
    fn config_load_fails_on_malformed_toml() {
        let dir = tempfile::TempDir::new().unwrap();
        let path = dir.path().join("config.toml");
        std::fs::write(&path, "this is [[[not valid toml").unwrap();
        assert!(Config::load(&path).is_err());
    }

    #[test]
    fn config_load_adds_migration_hint_for_old_sources() {
        let dir = tempfile::TempDir::new().unwrap();
        let path = dir.path().join("tome.toml");
        std::fs::write(
            &path,
            r#"
[[sources]]
name = "test"
path = "/tmp"
type = "directory"
"#,
        )
        .unwrap();
        let err = Config::load(&path).unwrap_err();
        let msg = format!("{err}");
        assert!(
            msg.contains("hint: tome v0.6 replaced [[sources]] and [targets.*] with [directories.*]. See CHANGELOG.md for migration instructions."),
            "expected migration hint, got: {msg}"
        );
    }

    #[test]
    fn config_load_adds_migration_hint_for_old_targets() {
        let dir = tempfile::TempDir::new().unwrap();
        let path = dir.path().join("tome.toml");
        std::fs::write(
            &path,
            r#"
[targets.foo]
enabled = true
method = "symlink"
skills_dir = "/tmp"
"#,
        )
        .unwrap();
        let err = Config::load(&path).unwrap_err();
        let msg = format!("{err}");
        assert!(
            msg.contains("hint: tome v0.6 replaced [[sources]] and [targets.*] with [directories.*]. See CHANGELOG.md for migration instructions."),
            "expected migration hint, got: {msg}"
        );
    }

    #[test]
    fn config_roundtrip_toml() {
        let config = Config {
            library_dir: PathBuf::from("/tmp/skills"),
            exclude: [SkillName::new("test-skill").unwrap()].into(),
            directories: BTreeMap::from([(
                DirectoryName::new("test").unwrap(),
                DirectoryConfig {
                    path: PathBuf::from("/tmp/source"),
                    directory_type: DirectoryType::Directory,
                    role: None,
                    git_ref: None,
                    subdir: None,
                    override_applied: false,
                },
            )]),
            ..Default::default()
        };
        let toml_str = toml::to_string_pretty(&config).unwrap();
        let parsed: Config = toml::from_str(&toml_str).unwrap();
        assert_eq!(parsed.library_dir, config.library_dir);
        assert_eq!(parsed.exclude, config.exclude);
        assert_eq!(parsed.directories.len(), 1);
        assert!(parsed.directories.contains_key("test"));
    }

    // --- expand_tilde tests (re-exported from paths::) ---

    #[test]
    fn expand_tilde_expands_home() {
        let result = expand_tilde(Path::new("~/foo/bar")).unwrap();
        assert!(result.is_absolute());
        assert!(result.ends_with("foo/bar"));
    }

    #[test]
    fn expand_tilde_leaves_absolute_unchanged() {
        let path = Path::new("/absolute/path");
        assert_eq!(expand_tilde(path).unwrap(), PathBuf::from("/absolute/path"));
    }

    #[test]
    fn expand_tilde_leaves_relative_unchanged() {
        let path = Path::new("relative/path");
        assert_eq!(expand_tilde(path).unwrap(), PathBuf::from("relative/path"));
    }

    // --- save_checked tests ---

    #[test]
    fn save_checked_rejects_role_type_conflict() {
        let tmp = tempfile::TempDir::new().unwrap();
        let path = tmp.path().join("tome.toml");
        let config = Config {
            library_dir: PathBuf::from("/tmp/lib-sc-1"),
            directories: BTreeMap::from([(
                DirectoryName::new("bad").unwrap(),
                DirectoryConfig {
                    path: PathBuf::from("/tmp/src"),
                    directory_type: DirectoryType::Git,
                    role: Some(DirectoryRole::Target),
                    git_ref: None,
                    subdir: None,
                    override_applied: false,
                },
            )]),
            ..Default::default()
        };
        let err = config.save_checked(&path).unwrap_err();
        assert!(
            err.to_string()
                .contains("Target (skills distributed here, not discovered here)"),
            "expected role parenthetical, got: {err}"
        );
        assert!(
            !path.exists(),
            "save_checked must not write on validation failure"
        );
    }

    #[test]
    fn save_checked_rejects_library_overlap() {
        let tmp = tempfile::TempDir::new().unwrap();
        let path = tmp.path().join("tome.toml");
        let config = Config {
            library_dir: PathBuf::from("/tmp/shared-sc"),
            directories: BTreeMap::from([(
                DirectoryName::new("shared").unwrap(),
                DirectoryConfig {
                    path: PathBuf::from("/tmp/shared-sc"),
                    directory_type: DirectoryType::Directory,
                    role: Some(DirectoryRole::Synced),
                    git_ref: None,
                    subdir: None,
                    override_applied: false,
                },
            )]),
            ..Default::default()
        };
        let err = config.save_checked(&path).unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("Conflict:"), "missing Conflict: {msg}");
        assert!(msg.contains("hint:"), "missing hint: {msg}");
        assert!(
            !path.exists(),
            "save_checked must not write on validation failure"
        );
    }

    #[test]
    fn save_checked_writes_valid_config_and_reloads_unchanged() {
        let tmp = tempfile::TempDir::new().unwrap();
        let path = tmp.path().join("tome.toml");
        let config = Config {
            library_dir: PathBuf::from("/tmp/lib-sc-ok"),
            directories: BTreeMap::from([(
                DirectoryName::new("ok").unwrap(),
                DirectoryConfig {
                    path: PathBuf::from("/tmp/ok"),
                    directory_type: DirectoryType::Directory,
                    role: Some(DirectoryRole::Synced),
                    git_ref: None,
                    subdir: None,
                    override_applied: false,
                },
            )]),
            ..Default::default()
        };
        config.save_checked(&path).expect("valid config must save");
        assert!(path.exists(), "file must exist after save_checked");

        // Reload and re-emit: must be byte-equal to the on-disk file.
        let on_disk = std::fs::read_to_string(&path).unwrap();
        let reloaded = Config::load(&path).expect("saved config must reload");
        let reemitted = toml::to_string_pretty(&reloaded).unwrap();
        assert_eq!(on_disk, reemitted, "saved file must round-trip exactly");
    }

    #[test]
    fn save_checked_does_not_mutate_caller() {
        // Caller's library_dir uses tilde; save_checked must not rewrite it in the caller's Config.
        let tmp = tempfile::TempDir::new().unwrap();
        let path = tmp.path().join("tome.toml");
        let config = Config {
            library_dir: PathBuf::from("~/some/lib-not-real"),
            ..Default::default()
        };
        let _ = config.save_checked(&path); // may fail on library_dir-is-a-file or succeed; irrelevant
        assert_eq!(
            config.library_dir,
            PathBuf::from("~/some/lib-not-real"),
            "save_checked must operate on a clone and leave the caller untouched"
        );
    }

    // === HARD-22 / D-TILDE-1: tilde-preservation in Config::save_checked ===
    //
    // Behaviour table (CONTEXT.md D-TILDE-1):
    //   ~/skills              -> ~/skills            (preserved)
    //   /Users/$USER/skills   -> ~/skills            (rewritten — auto-portable)
    //   /var/lib/skills       -> /var/lib/skills     (kept absolute — outside $HOME)
    //
    // Tests use dirs::home_dir() rather than hard-coded /Users/martin so they pass
    // on any developer machine and on Linux CI.

    #[test]
    fn save_checked_preserves_tilde_in_library_dir() {
        let tmp = tempfile::TempDir::new().unwrap();
        let path = tmp.path().join("tome.toml");
        // Use a tilde that won't collide with the library_dir-is-a-file check —
        // a non-existent path under $HOME is still a valid (uncreated) library_dir.
        let config = Config {
            library_dir: PathBuf::from("~/.tome-test/lib-tilde-preserve"),
            ..Default::default()
        };
        config
            .save_checked(&path)
            .expect("valid tilde library_dir must save");
        let on_disk = std::fs::read_to_string(&path).unwrap();
        assert!(
            on_disk.contains("library_dir = \"~/.tome-test/lib-tilde-preserve\""),
            "expected ~-shape preserved in saved file, got:\n{on_disk}"
        );
    }

    #[test]
    fn save_checked_rewrites_under_home_absolute_to_tilde() {
        let tmp = tempfile::TempDir::new().unwrap();
        let path = tmp.path().join("tome.toml");
        // Build an absolute path under the *real* $HOME (whatever it is on this
        // machine) so the rewrite triggers regardless of dev environment.
        let home = dirs::home_dir().expect("home dir required for this test");
        let absolute = home.join(".tome-test/lib-rewrite");
        let config = Config {
            library_dir: absolute,
            ..Default::default()
        };
        config
            .save_checked(&path)
            .expect("valid under-home library_dir must save");
        let on_disk = std::fs::read_to_string(&path).unwrap();
        assert!(
            on_disk.contains("library_dir = \"~/.tome-test/lib-rewrite\""),
            "expected absolute path under $HOME rewritten to ~-shape, got:\n{on_disk}"
        );
    }

    #[test]
    fn save_checked_keeps_outside_home_absolute_unchanged() {
        let tmp = tempfile::TempDir::new().unwrap();
        let path = tmp.path().join("tome.toml");
        // tmp.path() under $TMPDIR is typically not under $HOME — use a fixed
        // /var/lib/... path that on macOS and Linux is always outside $HOME.
        // The path doesn't need to exist on disk: validate() only flags it if it
        // exists AND is a file.
        let outside_home = PathBuf::from("/var/lib/tome-test-outside");
        let config = Config {
            library_dir: outside_home.clone(),
            ..Default::default()
        };
        config
            .save_checked(&path)
            .expect("outside-$HOME library_dir must save");
        let on_disk = std::fs::read_to_string(&path).unwrap();
        assert!(
            on_disk.contains("library_dir = \"/var/lib/tome-test-outside\""),
            "expected outside-$HOME path kept absolute, got:\n{on_disk}"
        );
    }

    #[test]
    fn save_checked_rewrites_directory_path_under_home() {
        // Every path field — library_dir AND directories.<name>.path — must
        // participate in the unexpand pass.
        let tmp = tempfile::TempDir::new().unwrap();
        let path = tmp.path().join("tome.toml");
        let home = dirs::home_dir().expect("home dir required for this test");
        let dir_abs = home.join(".tome-test/skill-dir-rewrite");

        let config = Config {
            library_dir: PathBuf::from("/var/lib/tome-lib"),
            directories: BTreeMap::from([(
                DirectoryName::new("under-home").unwrap(),
                DirectoryConfig {
                    path: dir_abs,
                    directory_type: DirectoryType::Directory,
                    role: Some(DirectoryRole::Source),
                    git_ref: None,
                    subdir: None,
                    override_applied: false,
                },
            )]),
            ..Default::default()
        };
        config.save_checked(&path).expect("valid config must save");
        let on_disk = std::fs::read_to_string(&path).unwrap();
        assert!(
            on_disk.contains("path = \"~/.tome-test/skill-dir-rewrite\""),
            "expected directory path under $HOME rewritten to ~-shape, got:\n{on_disk}"
        );
    }

    #[test]
    fn save_checked_does_not_round_trip_override_paths_to_tome_toml() {
        // PORT-02 invariant: override paths from machine.toml MUST NOT be
        // serialised back into tome.toml on save. apply_machine_overrides
        // mutates a load-time-only copy; save_checked operates on the
        // unmutated config (or a clone that didn't go through apply).
        //
        // Setup: build a Config with a directory at the ORIGINAL path,
        // simulate apply_machine_overrides by calling it directly with prefs
        // that rewrite the path, save the (post-apply) config — and assert the
        // ORIGINAL path is NOT in the saved file.
        //
        // NOTE: this test confirms the contract from BOTH angles:
        //   - The *unmutated* config saved with original path: trivially true.
        //   - The *mutated* (post-apply) config saved would write the override
        //     path. The real lib.rs::run flow only ever saves a freshly-loaded
        //     Config (without overrides applied) via save_checked. We pin that
        //     contract here by noting that save_checked writes whatever path
        //     is in `self.directories[*].path`, so callers must save the
        //     pre-override config — the `lib.rs::sync` save chain already
        //     does this (Phase 9 PORT-02).
        let tmp = tempfile::TempDir::new().unwrap();
        let cfg_path = tmp.path().join("tome.toml");
        let lib_dir = tmp.path().join("library");
        let original_dir_path = tmp.path().join("original-skills");

        let mut config = Config {
            library_dir: lib_dir,
            directories: BTreeMap::from([(
                DirectoryName::new("work").unwrap(),
                DirectoryConfig {
                    path: original_dir_path.clone(),
                    directory_type: DirectoryType::Directory,
                    role: Some(DirectoryRole::Source),
                    git_ref: None,
                    subdir: None,
                    override_applied: false,
                },
            )]),
            ..Default::default()
        };

        // Save the unmutated config — this is what lib.rs::sync save chain
        // does (it has access to the pre-override Config).
        config.save_checked(&cfg_path).unwrap();
        let on_disk_pre = std::fs::read_to_string(&cfg_path).unwrap();
        assert!(
            on_disk_pre.contains(original_dir_path.to_str().unwrap()),
            "saved file must contain original path, got:\n{on_disk_pre}"
        );

        // Now simulate the in-memory "post-apply" mutation: apply overrides
        // that rewrite work.path to a different location. Saving this
        // *mutated* config would write the override path — DON'T do that
        // in production. Confirmed by the assertion below.
        let override_path = tmp.path().join("override-skills");
        let mut prefs = crate::machine::MachinePrefs::default();
        prefs.directory_overrides.insert(
            DirectoryName::new("work").unwrap(),
            crate::machine::DirectoryOverride {
                path: override_path.clone(),
            },
        );
        config.apply_machine_overrides(&prefs).unwrap();
        // After apply, in-memory config has the override path — saving NOW
        // would round-trip it. The PORT-02 invariant in lib.rs is that
        // save_checked is never called after apply_machine_overrides on the
        // same config; we document/lock that contract here.
        assert_eq!(
            config.directories.get("work").unwrap().path,
            override_path,
            "apply_machine_overrides should have rewritten work.path"
        );

        // Re-read the originally-saved tome.toml: the override path is NOT
        // in it (never was; we saved BEFORE apply).
        let still_on_disk = std::fs::read_to_string(&cfg_path).unwrap();
        assert!(
            !still_on_disk.contains(override_path.to_str().unwrap()),
            "tome.toml on disk MUST NOT contain override path from machine.toml \
             (PORT-02 invariant), got:\n{still_on_disk}"
        );
        assert!(
            still_on_disk.contains(original_dir_path.to_str().unwrap()),
            "tome.toml on disk must still contain the original path, got:\n{still_on_disk}"
        );
    }

    #[test]
    fn save_checked_does_not_serialize_override_applied() {
        // Build a Config in-memory with override_applied = true, save_checked it,
        // then read the resulting TOML — `override_applied` MUST NOT appear.
        let tmp = tempfile::TempDir::new().unwrap();
        let cfg_path = tmp.path().join("tome.toml");
        let lib_dir = tmp.path().join("library");
        std::fs::create_dir_all(&lib_dir).unwrap();

        let mut config = Config {
            library_dir: lib_dir.clone(),
            directories: BTreeMap::from([(
                DirectoryName::new("x").unwrap(),
                DirectoryConfig {
                    path: tmp.path().join("skills"),
                    directory_type: DirectoryType::Directory,
                    role: Some(DirectoryRole::Source),
                    git_ref: None,
                    subdir: None,
                    override_applied: true,
                },
            )]),
            ..Default::default()
        };
        // Apply overrides happens via `apply_machine_overrides`; force the
        // flag here directly to test the serialization path.
        config.directories.get_mut("x").unwrap().override_applied = true;

        config.save_checked(&cfg_path).unwrap();

        let on_disk = std::fs::read_to_string(&cfg_path).unwrap();
        assert!(
            !on_disk.contains("override_applied"),
            "override_applied must not appear in tome.toml, got:\n{on_disk}"
        );
    }

    // --- TomeHomeSource + resolve_tome_home_with_source tests ---
    //
    // These tests manipulate process-wide env vars (TOME_HOME, HOME), so they
    // are serialized via a local Mutex. `std::env::set_var`/`remove_var` are
    // `unsafe` in edition 2024 because they are unsound under concurrent reads;
    // the lock gives us a single-writer window within this test binary.

    static ENV_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

    fn with_env<F, R>(vars: &[(&str, Option<&std::ffi::OsStr>)], f: F) -> R
    where
        F: FnOnce() -> R,
    {
        let _guard = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let saved: Vec<(String, Option<std::ffi::OsString>)> = vars
            .iter()
            .map(|(k, _)| ((*k).to_string(), std::env::var_os(k)))
            .collect();
        for (k, v) in vars {
            match v {
                Some(val) => unsafe { std::env::set_var(k, val) },
                None => unsafe { std::env::remove_var(k) },
            }
        }
        let result = f();
        for (k, v) in saved {
            match v {
                Some(val) => unsafe { std::env::set_var(&k, val) },
                None => unsafe { std::env::remove_var(&k) },
            }
        }
        result
    }

    #[test]
    fn tome_home_source_label_strings() {
        assert_eq!(TomeHomeSource::CliTomeHome.label(), "--tome-home flag");
        assert_eq!(TomeHomeSource::CliConfig.label(), "--config flag");
        assert_eq!(TomeHomeSource::EnvVar.label(), "TOME_HOME env");
        assert_eq!(
            TomeHomeSource::XdgConfig.label(),
            "~/.config/tome/config.toml"
        );
        assert_eq!(TomeHomeSource::Default.label(), "default");
    }

    #[test]
    fn resolve_tome_home_with_source_prefers_cli_tome_home() {
        let tmp = tempfile::TempDir::new().unwrap();
        let home = tmp.path().to_path_buf();
        let custom = tmp.path().join("custom-home");

        with_env(
            &[
                ("HOME", Some(home.as_os_str())),
                (
                    "TOME_HOME",
                    Some(std::ffi::OsStr::new("/should/be/ignored")),
                ),
            ],
            || {
                let (path, src) = resolve_tome_home_with_source(Some(&custom), None).unwrap();
                assert_eq!(path, custom);
                assert_eq!(src, TomeHomeSource::CliTomeHome);
                assert_eq!(src.label(), "--tome-home flag");
            },
        );
    }

    #[test]
    fn resolve_tome_home_with_source_uses_cli_config_parent() {
        let tmp = tempfile::TempDir::new().unwrap();
        let home = tmp.path().to_path_buf();
        let cfg_dir = tmp.path().join("cfg");
        std::fs::create_dir_all(&cfg_dir).unwrap();
        let cfg_file = cfg_dir.join("tome.toml");

        with_env(
            &[
                ("HOME", Some(home.as_os_str())),
                (
                    "TOME_HOME",
                    Some(std::ffi::OsStr::new("/should/be/ignored")),
                ),
            ],
            || {
                let (path, src) = resolve_tome_home_with_source(None, Some(&cfg_file)).unwrap();
                assert_eq!(path, cfg_dir);
                assert_eq!(src, TomeHomeSource::CliConfig);
                assert_eq!(src.label(), "--config flag");
            },
        );
    }

    #[test]
    fn resolve_tome_home_with_source_uses_env_var() {
        let tmp = tempfile::TempDir::new().unwrap();
        let home = tmp.path().to_path_buf();
        let env_home = tmp.path().join("env-home");

        with_env(
            &[
                ("HOME", Some(home.as_os_str())),
                ("TOME_HOME", Some(env_home.as_os_str())),
            ],
            || {
                let (path, src) = resolve_tome_home_with_source(None, None).unwrap();
                assert_eq!(path, env_home);
                assert_eq!(src, TomeHomeSource::EnvVar);
                assert_eq!(src.label(), "TOME_HOME env");
            },
        );
    }

    #[test]
    fn resolve_tome_home_with_source_uses_xdg_config() {
        let tmp = tempfile::TempDir::new().unwrap();
        let home = tmp.path().to_path_buf();
        // Seed XDG config at <HOME>/.config/tome/config.toml with a tome_home field.
        let xdg_dir = home.join(".config/tome");
        std::fs::create_dir_all(&xdg_dir).unwrap();
        let xdg_tome_home = home.join("xdg-tome-home");
        std::fs::write(
            xdg_dir.join("config.toml"),
            format!("tome_home = \"{}\"\n", xdg_tome_home.display()),
        )
        .unwrap();

        with_env(
            &[("HOME", Some(home.as_os_str())), ("TOME_HOME", None)],
            || {
                let (path, src) = resolve_tome_home_with_source(None, None).unwrap();
                assert_eq!(path, xdg_tome_home);
                assert_eq!(src, TomeHomeSource::XdgConfig);
                assert_eq!(src.label(), "~/.config/tome/config.toml");
            },
        );
    }

    #[test]
    fn resolve_tome_home_with_source_falls_back_to_default() {
        let tmp = tempfile::TempDir::new().unwrap();
        let home = tmp.path().to_path_buf();

        with_env(
            &[("HOME", Some(home.as_os_str())), ("TOME_HOME", None)],
            || {
                let (path, src) = resolve_tome_home_with_source(None, None).unwrap();
                assert_eq!(path, home.join(".tome"));
                assert_eq!(src, TomeHomeSource::Default);
                assert_eq!(src.label(), "default");
            },
        );
    }

    #[test]
    fn resolve_tome_home_with_source_rejects_relative_cli_tome_home() {
        let tmp = tempfile::TempDir::new().unwrap();
        let home = tmp.path().to_path_buf();
        let relative = Path::new("relative/custom");

        with_env(
            &[("HOME", Some(home.as_os_str())), ("TOME_HOME", None)],
            || {
                let err = resolve_tome_home_with_source(Some(relative), None).unwrap_err();
                let msg = err.to_string();
                assert!(
                    msg.contains("must be an absolute path"),
                    "expected absolute-path error, got: {msg}"
                );
            },
        );
    }

    // -----------------------------------------------------------------------
    // WUX-05: write_xdg_tome_home helper — atomic merge-write
    // -----------------------------------------------------------------------
    //
    // These tests lock in that `write_xdg_tome_home` creates the XDG file,
    // preserves other keys (merge-preserve, not clobber), collapses paths to
    // `~/`-form for portability, and writes atomically via temp+rename.

    #[test]
    fn write_xdg_tome_home_creates_new_file() {
        let tmp = tempfile::TempDir::new().unwrap();
        with_env(&[("HOME", Some(tmp.path().as_os_str()))], || {
            let custom = tmp.path().join("dotfiles/tome");
            write_xdg_tome_home(&custom).unwrap();

            let xdg = tmp.path().join(".config/tome/config.toml");
            assert!(xdg.is_file(), "XDG file should be created");
            let content = std::fs::read_to_string(&xdg).unwrap();
            let table: toml::Table = content.parse().unwrap();
            let tome_home = table.get("tome_home").and_then(|v| v.as_str()).unwrap();
            // Path is under HOME → collapsed form
            assert_eq!(tome_home, "~/dotfiles/tome");
        });
    }

    #[test]
    fn write_xdg_tome_home_preserves_other_keys() {
        let tmp = tempfile::TempDir::new().unwrap();
        with_env(&[("HOME", Some(tmp.path().as_os_str()))], || {
            let xdg = tmp.path().join(".config/tome/config.toml");
            std::fs::create_dir_all(xdg.parent().unwrap()).unwrap();
            std::fs::write(&xdg, "other_key = \"preserve-me\"\ntome_home = \"~/old\"\n").unwrap();

            let custom = tmp.path().join("dotfiles/tome");
            write_xdg_tome_home(&custom).unwrap();

            let content = std::fs::read_to_string(&xdg).unwrap();
            let table: toml::Table = content.parse().unwrap();
            // tome_home overwritten
            assert_eq!(
                table.get("tome_home").and_then(|v| v.as_str()),
                Some("~/dotfiles/tome")
            );
            // other_key preserved
            assert_eq!(
                table.get("other_key").and_then(|v| v.as_str()),
                Some("preserve-me")
            );
        });
    }

    #[test]
    fn write_xdg_tome_home_is_atomic() {
        let tmp = tempfile::TempDir::new().unwrap();
        with_env(&[("HOME", Some(tmp.path().as_os_str()))], || {
            let custom = tmp.path().join("dotfiles/tome");
            write_xdg_tome_home(&custom).unwrap();

            let tmp_file = tmp.path().join(".config/tome/config.toml.tmp");
            assert!(
                !tmp_file.exists(),
                "temp file should be removed after successful rename"
            );
        });
    }

    /// HARD-08: rename failure during atomic save_checked must leave the
    /// previous on-disk tome.toml content untouched.
    ///
    /// Mechanism: chmod 0o500 on the parent dir → fs::rename returns
    /// EACCES → save_checked returns Err → original file is unchanged.
    #[cfg(unix)]
    #[test]
    fn save_checked_preserves_previous_on_rename_failure() {
        use std::os::unix::fs::PermissionsExt;

        let tmp = tempfile::TempDir::new().unwrap();
        let lib_dir = tmp.path().join("library-a");
        std::fs::create_dir_all(&lib_dir).unwrap();
        let path = tmp.path().join("tome.toml");

        // Step 1: write Config A through the canonical save_checked path.
        let config_a = Config {
            library_dir: lib_dir.clone(),
            directories: BTreeMap::new(),
            exclude: Default::default(),
            backup: Default::default(),
        };
        config_a.save_checked(&path).unwrap();
        let bytes_a = std::fs::read(&path).unwrap();
        assert!(
            !bytes_a.is_empty(),
            "precondition: save_checked must produce non-empty content"
        );

        // Step 2: lock the parent dir so rename will EACCES.
        let original_mode = std::fs::metadata(tmp.path()).unwrap().permissions().mode();
        std::fs::set_permissions(tmp.path(), std::fs::Permissions::from_mode(0o500)).unwrap();

        // Step 3: attempt to save Config B (different library_dir). Must fail.
        let lib_dir_b = tmp.path().join("library-b");
        // (Don't actually create lib_dir_b on disk — validate() doesn't
        // require existence, only that it's not a regular file.)
        let config_b = Config {
            library_dir: lib_dir_b,
            directories: BTreeMap::new(),
            exclude: Default::default(),
            backup: Default::default(),
        };
        let result = config_b.save_checked(&path);

        // Restore permissions BEFORE the assertion so TempDir cleanup works.
        std::fs::set_permissions(tmp.path(), std::fs::Permissions::from_mode(original_mode))
            .unwrap();

        assert!(
            result.is_err(),
            "save_checked() must fail when the parent dir is not writable"
        );

        // Step 4: re-read the file. It must still be Config A's bytes.
        let bytes_after = std::fs::read(&path).unwrap();
        assert_eq!(
            bytes_after, bytes_a,
            "atomic-save invariant violated: tome.toml content was \
             corrupted by a failed save_checked"
        );

        // Re-load and confirm the surviving config is A, not B.
        let reloaded = Config::load(&path).unwrap();
        assert_eq!(reloaded.library_dir, lib_dir);
    }
}
