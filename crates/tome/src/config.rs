//! TOML configuration loading, saving, and validation. Handles tilde expansion and default paths.
//!
//! v0.6: Unified directory model — replaces separate `[[sources]]` and `[targets.*]`
//! with a single `[directories.*]` config.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use crate::discover::SkillName;

/// A validated directory name.
///
/// Rejects empty names and path separators, matching the `SkillName` validation pattern.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, serde::Serialize)]
#[serde(transparent)]
pub struct DirectoryName(String);

impl DirectoryName {
    /// Create a new directory name from any string-like value.
    ///
    /// Rejects empty names and names containing path separators (`/` or `\`).
    pub fn new(name: impl Into<String>) -> Result<Self> {
        let name = name.into();
        crate::validation::validate_identifier(&name, "directory name")?;
        Ok(Self(name))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for DirectoryName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl AsRef<str> for DirectoryName {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl AsRef<Path> for DirectoryName {
    fn as_ref(&self) -> &Path {
        Path::new(&self.0)
    }
}

impl PartialEq<str> for DirectoryName {
    fn eq(&self, other: &str) -> bool {
        self.0 == other
    }
}

impl PartialEq<&str> for DirectoryName {
    fn eq(&self, other: &&str) -> bool {
        self.0 == *other
    }
}

impl std::borrow::Borrow<str> for DirectoryName {
    fn borrow(&self) -> &str {
        &self.0
    }
}

impl TryFrom<String> for DirectoryName {
    type Error = anyhow::Error;

    fn try_from(s: String) -> Result<Self> {
        Self::new(s)
    }
}

impl<'de> serde::Deserialize<'de> for DirectoryName {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        DirectoryName::new(s).map_err(serde::de::Error::custom)
    }
}

/// The type of a configured directory — determines discovery strategy and default role.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum DirectoryType {
    /// Reads installed_plugins.json for plugin-based discovery
    ClaudePlugins,
    /// Scans for */SKILL.md directly
    #[default]
    Directory,
    /// Clones/pulls a remote git repository
    Git,
}

impl std::fmt::Display for DirectoryType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DirectoryType::ClaudePlugins => write!(f, "claude-plugins"),
            DirectoryType::Directory => write!(f, "directory"),
            DirectoryType::Git => write!(f, "git"),
        }
    }
}

impl DirectoryType {
    /// Returns the default role for this directory type.
    pub fn default_role(&self) -> DirectoryRole {
        match self {
            DirectoryType::ClaudePlugins => DirectoryRole::Managed,
            DirectoryType::Directory => DirectoryRole::Synced,
            DirectoryType::Git => DirectoryRole::Source,
        }
    }

    /// Returns the set of valid roles for this directory type.
    /// Used by the wizard to filter the role picker.
    pub fn valid_roles(&self) -> Vec<DirectoryRole> {
        match self {
            DirectoryType::ClaudePlugins => vec![DirectoryRole::Managed],
            DirectoryType::Directory => {
                vec![
                    DirectoryRole::Synced,
                    DirectoryRole::Source,
                    DirectoryRole::Target,
                ]
            }
            DirectoryType::Git => vec![DirectoryRole::Source],
        }
    }
}

/// The role a directory plays in the sync pipeline.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum DirectoryRole {
    /// Read-only, owned by package manager (e.g. Claude plugins cache)
    Managed,
    /// Skills discovered here AND distributed here (bidirectional)
    Synced,
    /// Skills discovered here, not distributed here
    Source,
    /// Skills distributed here, not discovered here
    Target,
}

impl DirectoryRole {
    /// Human-readable description with plain-english explanation.
    /// Per D-04/D-05: every user-facing display includes a parenthetical.
    pub fn description(&self) -> &'static str {
        match self {
            DirectoryRole::Managed => "Managed (read-only, owned by package manager)",
            DirectoryRole::Synced => "Synced (skills discovered here AND distributed here)",
            DirectoryRole::Source => "Source (skills discovered here, not distributed here)",
            DirectoryRole::Target => "Target (skills distributed here, not discovered here)",
        }
    }

    /// Whether this role participates in discovery (skills are read from it).
    pub fn is_discovery(&self) -> bool {
        matches!(
            self,
            DirectoryRole::Managed | DirectoryRole::Synced | DirectoryRole::Source
        )
    }

    /// Whether this role participates in distribution (skills are pushed to it).
    pub fn is_distribution(&self) -> bool {
        matches!(self, DirectoryRole::Synced | DirectoryRole::Target)
    }
}

impl std::fmt::Display for DirectoryRole {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DirectoryRole::Managed => write!(f, "managed"),
            DirectoryRole::Synced => write!(f, "synced"),
            DirectoryRole::Source => write!(f, "source"),
            DirectoryRole::Target => write!(f, "target"),
        }
    }
}

/// Configuration for a single directory in the unified model.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DirectoryConfig {
    /// Path to the directory
    pub path: PathBuf,

    /// How to discover skills in this directory
    #[serde(rename = "type", default)]
    pub directory_type: DirectoryType,

    /// Role in the sync pipeline (defaults based on directory_type)
    #[serde(default)]
    pub(crate) role: Option<DirectoryRole>,

    /// Git branch to track (git type only)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub branch: Option<String>,

    /// Git tag to pin (git type only)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tag: Option<String>,

    /// Git revision to pin (git type only)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rev: Option<String>,

    /// Subdirectory within the repo to scan for skills (git type only).
    /// When set, discovery scans `<clone_path>/<subdir>/` instead of the repo root.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub subdir: Option<String>,
}

impl DirectoryConfig {
    /// Returns the effective role, defaulting from directory_type if not explicitly set.
    pub fn role(&self) -> DirectoryRole {
        self.role
            .clone()
            .unwrap_or_else(|| self.directory_type.default_role())
    }
}

/// Backup configuration -- controls git-backed snapshots of the skill library.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct BackupConfig {
    pub(crate) enabled: bool,
    pub(crate) auto_snapshot: bool,
}

impl Default for BackupConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            auto_snapshot: false,
        }
    }
}

/// Top-level configuration for tome.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Config {
    /// Where the consolidated skill library lives
    #[serde(default = "defaults::library_dir")]
    pub(crate) library_dir: PathBuf,

    /// Skills to exclude by name
    #[serde(default)]
    pub(crate) exclude: BTreeSet<SkillName>,

    /// Unified directory entries -- replaces separate sources and targets
    #[serde(default)]
    pub(crate) directories: BTreeMap<DirectoryName, DirectoryConfig>,

    /// Backup settings
    #[serde(default)]
    pub(crate) backup: BackupConfig,
}

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
                    anyhow::ensure!(parent_exists, "config file not found: {}", p.display());
                }
                p.to_path_buf()
            }
            None => default_config_path()?,
        };
        Self::load(&path)
    }

    /// Save config to file, creating parent directories as needed.
    pub fn save(&self, path: &Path) -> Result<()> {
        let content = toml::to_string_pretty(self).context("failed to serialize config")?;
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("failed to create {}", parent.display()))?;
        }
        std::fs::write(path, &content)
            .with_context(|| format!("failed to write {}", path.display()))
    }

    /// Validate config for common misconfigurations.
    ///
    /// Checks:
    /// - library_dir is not a file
    /// - Role/type combos are valid (Managed only for ClaudePlugins, Target not for Git)
    /// - Git fields (branch/tag/rev) only on Git type directories
    pub fn validate(&self) -> Result<()> {
        // library_dir exists but is a file, not a directory
        if self.library_dir.exists() && !self.library_dir.is_dir() {
            anyhow::bail!(
                "library_dir exists but is not a directory: {}",
                self.library_dir.display()
            );
        }

        for (name, dir) in &self.directories {
            let role = dir.role();

            // Managed role only valid with ClaudePlugins type
            if role == DirectoryRole::Managed && dir.directory_type != DirectoryType::ClaudePlugins
            {
                anyhow::bail!(
                    "directory '{name}': role/type conflict\n\
                     Conflict: role is {} but type is '{}'\n\
                     Why: the Managed role means skills are owned by a package manager; only the claude-plugins type is known to behave this way, so any other type with Managed would be sync'd incorrectly.\n\
                     hint: either change type to 'claude-plugins', or change role to {} or {}.",
                    DirectoryRole::Managed.description(),
                    dir.directory_type,
                    DirectoryRole::Synced.description(),
                    DirectoryRole::Source.description(),
                );
            }

            // Target role invalid with Git type
            if role == DirectoryRole::Target && dir.directory_type == DirectoryType::Git {
                anyhow::bail!(
                    "directory '{name}': role/type conflict\n\
                     Conflict: role is {} but type is 'git'\n\
                     Why: the Target role means skills are distributed into this directory, but git-type directories are remote clones that tome must not write skills into — pushing symlinks into a git clone would clash with the working tree.\n\
                     hint: change role to {} (git repos are read-only skill sources).",
                    DirectoryRole::Target.description(),
                    DirectoryRole::Source.description(),
                );
            }

            // Git fields only valid with Git type
            let has_git_fields = dir.branch.is_some() || dir.tag.is_some() || dir.rev.is_some();
            if has_git_fields && dir.directory_type != DirectoryType::Git {
                anyhow::bail!(
                    "directory '{name}': git ref fields on non-git directory\n\
                     Conflict: branch/tag/rev is set but type is '{}'\n\
                     Why: branch, tag, and rev pin a remote git clone to a specific commit; they have no meaning for a local directory or a claude-plugins cache.\n\
                     hint: either change type to 'git', or remove the branch/tag/rev fields from this directory.",
                    dir.directory_type,
                );
            }

            // subdir only valid with Git type
            if dir.subdir.is_some() && dir.directory_type != DirectoryType::Git {
                anyhow::bail!(
                    "directory '{name}': subdir on non-git directory\n\
                     Conflict: subdir is set but type is '{}'\n\
                     Why: subdir scopes skill discovery to a sub-path within a remote git clone; for a plain directory you can just point 'path' at the sub-path directly.\n\
                     hint: either change type to 'git', or remove 'subdir' and adjust 'path' to point where skills actually live.",
                    dir.directory_type,
                );
            }
        }

        // --- Path overlap between library_dir and distribution directories ---
        // D-01/D-02/D-04/D-06/D-07: lexical, tilde-aware, trailing-separator-normalized.
        // Scope (D-05): library_dir vs each distribution directory (Synced or Target).
        let lib = expand_tilde(&self.library_dir)?;
        for (name, dir) in self.distribution_dirs() {
            let dist = expand_tilde(&dir.path)?;
            let role_desc = dir.role().description();

            // Case A: exact equality (also tolerates a trailing '/' on either side)
            if lib == dist
                || lib.to_string_lossy().trim_end_matches('/')
                    == dist.to_string_lossy().trim_end_matches('/')
            {
                anyhow::bail!(
                    "library_dir overlaps distribution directory '{name}'\n\
                     Conflict: library_dir ({}) is the same path as directory '{name}' ({})\n\
                     Why: this directory has role {role_desc}; tome would try to distribute the library into itself, creating a self-loop at sync time.\n\
                     hint: choose a library_dir outside any distribution directory, such as '~/.tome/skills'.",
                    lib.display(),
                    dist.display(),
                );
            }

            // Case B: library_dir is inside the distribution directory (WHARD-03 circular case)
            if path_contains(&dist, &lib) {
                anyhow::bail!(
                    "library_dir is inside distribution directory '{name}' (circular symlink risk)\n\
                     Conflict: library_dir ({}) is a subdirectory of directory '{name}' ({})\n\
                     Why: directory '{name}' has role {role_desc}; tome would distribute the library back into a directory that contains it, producing circular symlinks at distribute time.\n\
                     hint: move library_dir outside '{}' — for example, '~/.tome/skills'.",
                    lib.display(),
                    dist.display(),
                    dist.display(),
                );
            }

            // Case C: the distribution directory is inside library_dir
            if path_contains(&lib, &dist) {
                anyhow::bail!(
                    "distribution directory '{name}' is inside library_dir\n\
                     Conflict: directory '{name}' ({}) is a subdirectory of library_dir ({})\n\
                     Why: directory '{name}' has role {role_desc}; tome would distribute library contents into a directory that already lives inside the library, producing a self-loop at sync time.\n\
                     hint: move library_dir to a location outside '{name}' — for example, '~/.tome/skills'.",
                    dist.display(),
                    lib.display(),
                );
            }
        }

        Ok(())
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
    fn expand_tildes(&mut self) -> Result<()> {
        self.library_dir = expand_tilde(&self.library_dir)?;
        for dir in self.directories.values_mut() {
            dir.path = expand_tilde(&dir.path)?;
        }
        Ok(())
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            library_dir: defaults::library_dir(),
            exclude: BTreeSet::new(),
            directories: BTreeMap::new(),
            backup: BackupConfig::default(),
        }
    }
}

/// Expand `~` prefix to the user's home directory.
pub fn expand_tilde(path: &Path) -> Result<PathBuf> {
    if let Ok(stripped) = path.strip_prefix("~") {
        Ok(dirs::home_dir()
            .context("could not determine home directory")?
            .join(stripped))
    } else {
        Ok(path.to_path_buf())
    }
}

/// Check whether `ancestor` is a path-prefix of `descendant` (or equal),
/// with trailing-separator normalization so that `/foo/bar` does NOT contain
/// `/foo/barbaz`.
///
/// Lexical only — no canonicalization. Both inputs must already be
/// tilde-expanded by the caller (D-07).
fn path_contains(ancestor: &Path, descendant: &Path) -> bool {
    // Strip trailing separator so component-wise comparison is correct
    // even when the user writes "/foo/bar/" in config.
    let a: &Path = ancestor
        .to_str()
        .map(|s| Path::new(s.trim_end_matches('/')))
        .unwrap_or(ancestor);
    let d: &Path = descendant
        .to_str()
        .map(|s| Path::new(s.trim_end_matches('/')))
        .unwrap_or(descendant);
    d == a || d.starts_with(a)
}

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
fn read_config_tome_home() -> Result<Option<PathBuf>> {
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

// =============================================================================
// DEPRECATED COMPATIBILITY SHIMS
// =============================================================================
// These types exist only to keep other modules compiling during the v0.6
// NOTE: Deprecated Source, SourceType, TargetName, TargetConfig, TargetMethod
// types were removed as part of v0.6 unified directory migration (plan 01-05).

mod defaults {
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
    use std::path::Path;

    // --- DirectoryName tests ---

    #[test]
    fn directory_name_accepts_valid() {
        let name = DirectoryName::new("my-dir-123").unwrap();
        assert_eq!(name.as_str(), "my-dir-123");
        assert_eq!(name.to_string(), "my-dir-123");
        assert_eq!(name, *"my-dir-123");
    }

    #[test]
    fn directory_name_rejects_empty() {
        assert!(DirectoryName::new("").is_err());
    }

    #[test]
    fn directory_name_rejects_path_separator() {
        assert!(DirectoryName::new("foo/bar").is_err());
        assert!(DirectoryName::new("foo\\bar").is_err());
    }

    #[test]
    fn directory_name_rejects_dot_special() {
        assert!(DirectoryName::new(".").is_err());
        assert!(DirectoryName::new("..").is_err());
    }

    #[test]
    fn directory_name_rejects_whitespace() {
        assert!(DirectoryName::new("  ").is_err());
        assert!(DirectoryName::new(" leading").is_err());
        assert!(DirectoryName::new("trailing ").is_err());
    }

    #[test]
    fn directory_name_deserialize_rejects_empty() {
        let result: std::result::Result<DirectoryName, _> = serde_json::from_str(r#""""#);
        assert!(result.is_err());
    }

    // --- DirectoryType tests ---

    #[test]
    fn directory_type_default_is_directory() {
        assert_eq!(DirectoryType::default(), DirectoryType::Directory);
    }

    #[test]
    fn directory_type_default_roles() {
        assert_eq!(
            DirectoryType::ClaudePlugins.default_role(),
            DirectoryRole::Managed
        );
        assert_eq!(
            DirectoryType::Directory.default_role(),
            DirectoryRole::Synced
        );
        assert_eq!(DirectoryType::Git.default_role(), DirectoryRole::Source);
    }

    #[test]
    fn directory_type_valid_roles() {
        assert_eq!(
            DirectoryType::ClaudePlugins.valid_roles(),
            vec![DirectoryRole::Managed]
        );
        assert_eq!(
            DirectoryType::Directory.valid_roles(),
            vec![
                DirectoryRole::Synced,
                DirectoryRole::Source,
                DirectoryRole::Target,
            ]
        );
        assert_eq!(
            DirectoryType::Git.valid_roles(),
            vec![DirectoryRole::Source]
        );
    }

    #[test]
    fn directory_type_display() {
        assert_eq!(DirectoryType::ClaudePlugins.to_string(), "claude-plugins");
        assert_eq!(DirectoryType::Directory.to_string(), "directory");
        assert_eq!(DirectoryType::Git.to_string(), "git");
    }

    // --- DirectoryRole tests ---

    #[test]
    fn directory_role_descriptions() {
        assert_eq!(
            DirectoryRole::Managed.description(),
            "Managed (read-only, owned by package manager)"
        );
        assert_eq!(
            DirectoryRole::Synced.description(),
            "Synced (skills discovered here AND distributed here)"
        );
        assert_eq!(
            DirectoryRole::Source.description(),
            "Source (skills discovered here, not distributed here)"
        );
        assert_eq!(
            DirectoryRole::Target.description(),
            "Target (skills distributed here, not discovered here)"
        );
    }

    #[test]
    fn directory_role_is_discovery() {
        assert!(DirectoryRole::Managed.is_discovery());
        assert!(DirectoryRole::Synced.is_discovery());
        assert!(DirectoryRole::Source.is_discovery());
        assert!(!DirectoryRole::Target.is_discovery());
    }

    #[test]
    fn directory_role_is_distribution() {
        assert!(!DirectoryRole::Managed.is_distribution());
        assert!(DirectoryRole::Synced.is_distribution());
        assert!(!DirectoryRole::Source.is_distribution());
        assert!(DirectoryRole::Target.is_distribution());
    }

    // --- Config parsing tests ---

    #[test]
    fn config_parses_minimal_directory() {
        let toml_str = r#"
[directories.foo]
path = "/tmp/foo"
"#;
        let config: Config = toml::from_str(toml_str).unwrap();
        let foo = config.directories.get("foo").expect("foo missing");
        assert_eq!(foo.path, PathBuf::from("/tmp/foo"));
        assert_eq!(foo.directory_type, DirectoryType::Directory);
        assert_eq!(foo.role(), DirectoryRole::Synced);
    }

    #[test]
    fn config_parses_explicit_directory() {
        let toml_str = r#"
[directories.foo]
path = "/tmp"
type = "claude-plugins"
role = "managed"
"#;
        let config: Config = toml::from_str(toml_str).unwrap();
        let foo = config.directories.get("foo").expect("foo missing");
        assert_eq!(foo.directory_type, DirectoryType::ClaudePlugins);
        assert_eq!(foo.role(), DirectoryRole::Managed);
    }

    #[test]
    fn config_parses_git_directory_with_branch() {
        let toml_str = r#"
[directories.remote-skills]
path = "/tmp/remote"
type = "git"
branch = "main"
"#;
        let config: Config = toml::from_str(toml_str).unwrap();
        let dir = config
            .directories
            .get("remote-skills")
            .expect("remote-skills missing");
        assert_eq!(dir.directory_type, DirectoryType::Git);
        assert_eq!(dir.role(), DirectoryRole::Source);
        assert_eq!(dir.branch.as_deref(), Some("main"));
    }

    #[test]
    fn config_rejects_old_format_sources() {
        let toml_str = r#"
[[sources]]
name = "claude-plugins"
path = "~/.claude/plugins/cache"
type = "claude-plugins"
"#;
        let err = toml::from_str::<Config>(toml_str).unwrap_err();
        // Config::load would add the migration hint; here we verify deny_unknown_fields catches it
        assert!(
            err.to_string().contains("unknown field"),
            "expected 'unknown field' error, got: {err}"
        );
    }

    #[test]
    fn config_rejects_old_format_targets() {
        let toml_str = r#"
[targets.antigravity]
enabled = true
method = "symlink"
skills_dir = "~/.gemini/antigravity/skills"
"#;
        let err = toml::from_str::<Config>(toml_str).unwrap_err();
        assert!(
            err.to_string().contains("unknown field"),
            "expected 'unknown field' error, got: {err}"
        );
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
    fn config_rejects_unknown_field_on_directory() {
        let toml_str = r#"
[directories.foo]
path = "/tmp"
bogus = true
"#;
        let err = toml::from_str::<Config>(toml_str).unwrap_err();
        assert!(
            err.to_string().contains("unknown field"),
            "expected 'unknown field' error, got: {err}"
        );
    }

    // --- Config validation tests ---

    #[test]
    fn validate_rejects_managed_with_directory_type() {
        let config = Config {
            directories: BTreeMap::from([(
                DirectoryName::new("bad").unwrap(),
                DirectoryConfig {
                    path: PathBuf::from("/tmp"),
                    directory_type: DirectoryType::Directory,
                    role: Some(DirectoryRole::Managed),
                    branch: None,
                    tag: None,
                    rev: None,
                    subdir: None,
                },
            )]),
            ..Default::default()
        };
        let err = config.validate().unwrap_err();
        let msg = err.to_string();
        assert!(
            msg.contains("Managed (read-only, owned by package manager)"),
            "missing role description: {msg}"
        );
        assert!(msg.contains("directory"), "missing type name: {msg}");
        assert!(msg.contains("hint:"), "missing hint line: {msg}");
    }

    #[test]
    fn validate_rejects_target_with_git_type() {
        let config = Config {
            directories: BTreeMap::from([(
                DirectoryName::new("bad").unwrap(),
                DirectoryConfig {
                    path: PathBuf::from("/tmp"),
                    directory_type: DirectoryType::Git,
                    role: Some(DirectoryRole::Target),
                    branch: None,
                    tag: None,
                    rev: None,
                    subdir: None,
                },
            )]),
            ..Default::default()
        };
        let err = config.validate().unwrap_err();
        let msg = err.to_string();
        assert!(
            msg.contains("Target (skills distributed here, not discovered here)"),
            "missing role description: {msg}"
        );
        assert!(msg.contains("git"), "missing type name: {msg}");
        assert!(msg.contains("hint:"), "missing hint line: {msg}");
    }

    #[test]
    fn validate_rejects_git_fields_with_non_git_type() {
        let config = Config {
            directories: BTreeMap::from([(
                DirectoryName::new("bad").unwrap(),
                DirectoryConfig {
                    path: PathBuf::from("/tmp"),
                    directory_type: DirectoryType::Directory,
                    role: None,
                    branch: Some("main".to_string()),
                    tag: None,
                    rev: None,
                    subdir: None,
                },
            )]),
            ..Default::default()
        };
        let err = config.validate().unwrap_err();
        let msg = err.to_string();
        assert!(
            msg.contains("branch") || msg.contains("tag") || msg.contains("rev"),
            "missing git-field mention: {msg}"
        );
        assert!(msg.contains("git"), "missing type name: {msg}");
        assert!(msg.contains("hint:"), "missing hint line: {msg}");
    }

    #[test]
    fn validate_rejects_subdir_with_non_git_type() {
        let config = Config {
            directories: BTreeMap::from([(
                DirectoryName::new("bad").unwrap(),
                DirectoryConfig {
                    path: PathBuf::from("/tmp"),
                    directory_type: DirectoryType::Directory,
                    role: None,
                    branch: None,
                    tag: None,
                    rev: None,
                    subdir: Some("nested".to_string()),
                },
            )]),
            ..Default::default()
        };
        let err = config.validate().unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("subdir"), "missing 'subdir': {msg}");
        assert!(msg.contains("git"), "missing type name: {msg}");
        assert!(msg.contains("hint:"), "missing hint line: {msg}");
    }

    #[test]
    fn validate_passes_for_valid_config() {
        let config = Config {
            library_dir: PathBuf::from("/tmp/nonexistent-lib"),
            directories: BTreeMap::from([
                (
                    DirectoryName::new("claude-plugins").unwrap(),
                    DirectoryConfig {
                        path: PathBuf::from("/tmp/plugins"),
                        directory_type: DirectoryType::ClaudePlugins,
                        role: Some(DirectoryRole::Managed),
                        branch: None,
                        tag: None,
                        rev: None,

                        subdir: None,
                    },
                ),
                (
                    DirectoryName::new("my-skills").unwrap(),
                    DirectoryConfig {
                        path: PathBuf::from("/tmp/skills"),
                        directory_type: DirectoryType::Directory,
                        role: None, // defaults to Synced
                        branch: None,
                        tag: None,
                        rev: None,

                        subdir: None,
                    },
                ),
            ]),
            ..Default::default()
        };
        config.validate().unwrap();
    }

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
                        branch: None,
                        tag: None,
                        rev: None,

                        subdir: None,
                    },
                ),
                (
                    DirectoryName::new("b-synced").unwrap(),
                    DirectoryConfig {
                        path: PathBuf::from("/tmp/b"),
                        directory_type: DirectoryType::Directory,
                        role: Some(DirectoryRole::Synced),
                        branch: None,
                        tag: None,
                        rev: None,

                        subdir: None,
                    },
                ),
                (
                    DirectoryName::new("c-source").unwrap(),
                    DirectoryConfig {
                        path: PathBuf::from("/tmp/c"),
                        directory_type: DirectoryType::Directory,
                        role: Some(DirectoryRole::Source),
                        branch: None,
                        tag: None,
                        rev: None,

                        subdir: None,
                    },
                ),
                (
                    DirectoryName::new("d-target").unwrap(),
                    DirectoryConfig {
                        path: PathBuf::from("/tmp/d"),
                        directory_type: DirectoryType::Directory,
                        role: Some(DirectoryRole::Target),
                        branch: None,
                        tag: None,
                        rev: None,

                        subdir: None,
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
                        branch: None,
                        tag: None,
                        rev: None,

                        subdir: None,
                    },
                ),
                (
                    DirectoryName::new("b-synced").unwrap(),
                    DirectoryConfig {
                        path: PathBuf::from("/tmp/b"),
                        directory_type: DirectoryType::Directory,
                        role: Some(DirectoryRole::Synced),
                        branch: None,
                        tag: None,
                        rev: None,

                        subdir: None,
                    },
                ),
                (
                    DirectoryName::new("c-source").unwrap(),
                    DirectoryConfig {
                        path: PathBuf::from("/tmp/c"),
                        directory_type: DirectoryType::Directory,
                        role: Some(DirectoryRole::Source),
                        branch: None,
                        tag: None,
                        rev: None,

                        subdir: None,
                    },
                ),
                (
                    DirectoryName::new("d-target").unwrap(),
                    DirectoryConfig {
                        path: PathBuf::from("/tmp/d"),
                        directory_type: DirectoryType::Directory,
                        role: Some(DirectoryRole::Target),
                        branch: None,
                        tag: None,
                        rev: None,

                        subdir: None,
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

    #[test]
    fn empty_directories_is_detectable() {
        let config = Config::default();
        assert!(config.directories.is_empty());
    }

    // --- Existing tests that remain valid ---

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

    #[test]
    fn default_config_has_empty_directories() {
        let config = Config::default();
        assert!(config.directories.is_empty());
        assert!(config.exclude.is_empty());
    }

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
                    branch: None,
                    tag: None,
                    rev: None,
                    subdir: None,
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

    #[test]
    fn config_load_fails_on_malformed_toml() {
        let dir = tempfile::TempDir::new().unwrap();
        let path = dir.path().join("config.toml");
        std::fs::write(&path, "this is [[[not valid toml").unwrap();
        assert!(Config::load(&path).is_err());
    }

    #[test]
    fn validate_rejects_library_dir_that_is_a_file() {
        let dir = tempfile::TempDir::new().unwrap();
        let file_path = dir.path().join("not-a-dir");
        std::fs::write(&file_path, "I'm a file").unwrap();

        let config = Config {
            library_dir: file_path,
            ..Default::default()
        };
        let err = config.validate().unwrap_err();
        assert!(
            err.to_string().contains("not a directory"),
            "unexpected error: {err}"
        );
    }

    #[test]
    fn config_parses_full_toml() {
        let toml_str = r#"
library_dir = "~/.tome/skills"
exclude = ["deprecated-skill"]

[directories.claude-plugins]
path = "~/.claude/plugins/cache"
type = "claude-plugins"
role = "managed"

[directories.standalone]
path = "~/.claude/skills"

[directories.antigravity]
path = "~/.gemini/antigravity/skills"
role = "target"
"#;
        let config: Config = toml::from_str(toml_str).unwrap();
        assert_eq!(config.directories.len(), 3);
        assert!(config.directories.contains_key("claude-plugins"));
        assert!(config.directories.contains_key("standalone"));
        assert!(config.directories.contains_key("antigravity"));
    }

    // TOME_HOME env var tests are covered by integration tests in cli.rs,
    // since set_var/remove_var are unsafe in Rust 2024 edition and env var
    // mutation in unit tests causes data races with parallel test execution.

    // --- Overlap tests (WHARD-02 / WHARD-03) ---

    fn dir_cfg(path: &str, dt: DirectoryType, role: Option<DirectoryRole>) -> DirectoryConfig {
        DirectoryConfig {
            path: PathBuf::from(path),
            directory_type: dt,
            role,
            branch: None,
            tag: None,
            rev: None,
            subdir: None,
        }
    }

    #[test]
    fn validate_rejects_library_equals_distribution() {
        let config = Config {
            library_dir: PathBuf::from("/tmp/shared"),
            directories: BTreeMap::from([(
                DirectoryName::new("shared").unwrap(),
                dir_cfg(
                    "/tmp/shared",
                    DirectoryType::Directory,
                    Some(DirectoryRole::Synced),
                ),
            )]),
            ..Default::default()
        };
        let msg = config.validate().unwrap_err().to_string();
        assert!(msg.contains("Conflict:"), "missing Conflict line: {msg}");
        assert!(msg.contains("shared"), "missing directory name: {msg}");
        assert!(
            msg.contains("Synced (skills discovered here AND distributed here)"),
            "missing role parenthetical: {msg}"
        );
        assert!(msg.contains("hint:"), "missing hint: {msg}");
    }

    #[test]
    fn validate_rejects_library_inside_synced_dir() {
        let config = Config {
            library_dir: PathBuf::from("/tmp/outer/inner"),
            directories: BTreeMap::from([(
                DirectoryName::new("outer").unwrap(),
                dir_cfg(
                    "/tmp/outer",
                    DirectoryType::Directory,
                    Some(DirectoryRole::Synced),
                ),
            )]),
            ..Default::default()
        };
        let msg = config.validate().unwrap_err().to_string();
        assert!(msg.contains("circular"), "missing 'circular': {msg}");
        assert!(msg.contains("symlink"), "missing 'symlink': {msg}");
        assert!(
            msg.contains("Synced (skills discovered here AND distributed here)"),
            "missing role parenthetical: {msg}"
        );
        assert!(msg.contains("hint:"), "missing hint: {msg}");
    }

    #[test]
    fn validate_rejects_target_inside_library() {
        let config = Config {
            library_dir: PathBuf::from("/tmp/outer"),
            directories: BTreeMap::from([(
                DirectoryName::new("inner-target").unwrap(),
                dir_cfg(
                    "/tmp/outer/inner",
                    DirectoryType::Directory,
                    Some(DirectoryRole::Target),
                ),
            )]),
            ..Default::default()
        };
        let msg = config.validate().unwrap_err().to_string();
        assert!(msg.contains("Conflict:"), "missing Conflict line: {msg}");
        assert!(
            msg.contains("Target (skills distributed here, not discovered here)"),
            "missing role parenthetical: {msg}"
        );
        assert!(msg.contains("hint:"), "missing hint: {msg}");
    }

    #[test]
    fn validate_accepts_sibling_paths_not_false_positive() {
        // /tmp/foo and /tmp/foobar are siblings, not nested.
        let config = Config {
            library_dir: PathBuf::from("/tmp/foo"),
            directories: BTreeMap::from([(
                DirectoryName::new("foobar").unwrap(),
                dir_cfg(
                    "/tmp/foobar",
                    DirectoryType::Directory,
                    Some(DirectoryRole::Synced),
                ),
            )]),
            ..Default::default()
        };
        config
            .validate()
            .expect("sibling paths must not trigger overlap");
    }

    #[test]
    fn validate_rejects_equality_despite_trailing_separator() {
        let config = Config {
            library_dir: PathBuf::from("/tmp/lib/"),
            directories: BTreeMap::from([(
                DirectoryName::new("lib").unwrap(),
                dir_cfg(
                    "/tmp/lib",
                    DirectoryType::Directory,
                    Some(DirectoryRole::Synced),
                ),
            )]),
            ..Default::default()
        };
        let msg = config.validate().unwrap_err().to_string();
        assert!(msg.contains("Conflict:"), "missing Conflict line: {msg}");
    }

    #[test]
    fn validate_accepts_source_role_inside_library() {
        // Source dirs don't participate in distribution — no self-loop risk (D-05).
        let config = Config {
            library_dir: PathBuf::from("/tmp/outer"),
            directories: BTreeMap::from([(
                DirectoryName::new("inner-source").unwrap(),
                dir_cfg(
                    "/tmp/outer/inner",
                    DirectoryType::Directory,
                    Some(DirectoryRole::Source),
                ),
            )]),
            ..Default::default()
        };
        config
            .validate()
            .expect("Source-role nesting must not trigger overlap");
    }

    #[test]
    fn validate_rejects_tilde_equal_paths() {
        // Both library_dir and directory path use tilde; must expand before compare.
        let config = Config {
            library_dir: PathBuf::from("~/.tome/skills"),
            directories: BTreeMap::from([(
                DirectoryName::new("same").unwrap(),
                dir_cfg(
                    "~/.tome/skills",
                    DirectoryType::Directory,
                    Some(DirectoryRole::Synced),
                ),
            )]),
            ..Default::default()
        };
        let msg = config.validate().unwrap_err().to_string();
        assert!(msg.contains("Conflict:"), "missing Conflict line: {msg}");
        assert!(
            msg.contains("Synced (skills discovered here AND distributed here)"),
            "missing role parenthetical: {msg}"
        );
    }

    // --- save_checked tests (WHARD-01) ---

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
                    branch: None,
                    tag: None,
                    rev: None,
                    subdir: None,
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
                    branch: None,
                    tag: None,
                    rev: None,
                    subdir: None,
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
                    branch: None,
                    tag: None,
                    rev: None,
                    subdir: None,
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
}
