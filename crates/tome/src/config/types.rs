//! Type definitions for the unified directory model.
//!
//! Lifecycle methods (`load`, `save_checked`) live in [`super`](crate::config) (mod.rs);
//! validation lives in [`super::validate`]; per-machine override application lives in
//! [`super::overrides`]. This file holds the data shapes (and their derive impls) only.

use anyhow::Result;
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
                // Phase 22 / v0.15: Managed is now valid for `directory`
                // type, opening up flat-directory package managers
                // (pfw, etc.) as first-class. Discovery + consolidate
                // already key on `role() == Managed` end-to-end (the
                // `is_managed: bool` flag flows through to manifest
                // entries), so allowing the combo here lets the rest
                // of the pipeline do the right thing.
                vec![
                    DirectoryRole::Managed,
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
///
/// The `clap::ValueEnum` derive lets `tome add --role <ROLE>` accept these
/// variants in their kebab-case form (matching the `tome.toml` wire format).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, clap::ValueEnum)]
#[serde(rename_all = "kebab-case")]
#[clap(rename_all = "kebab-case")]
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
    /// Kebab-case string matching the serde wire format (also the form clap
    /// uses to parse `--role <ROLE>` and the form used in `tome.toml`).
    pub fn kebab_case(&self) -> &'static str {
        match self {
            DirectoryRole::Managed => "managed",
            DirectoryRole::Synced => "synced",
            DirectoryRole::Source => "source",
            DirectoryRole::Target => "target",
        }
    }

    /// Human-readable description used in validation error messages.
    ///
    /// The parenthetical ("Managed (read-only, owned by package manager)") is
    /// load-bearing: validator errors assert on this substring, so both the
    /// kind and the hint render identically.
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

/// A git reference pin — exactly one of branch / tag / rev.
///
/// Closes #490: replaces three mutually-exclusive `Option<String>` fields
/// (`branch`, `tag`, `rev`) on `DirectoryConfig`. The TOML schema is
/// preserved (custom serde shim reads the flat `branch = "..."` form),
/// but the in-memory representation makes illegal states (e.g. all three
/// fields set) unrepresentable.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GitRef {
    /// Track a branch (e.g. `"main"`). Implies pull-on-update semantics.
    Branch(String),
    /// Pin to an immutable tag (e.g. `"v1.2.0"`).
    Tag(String),
    /// Pin to an exact commit SHA.
    Rev(String),
}

impl GitRef {
    /// The branch name, if this is a `Branch` variant.
    pub fn branch(&self) -> Option<&str> {
        match self {
            GitRef::Branch(s) => Some(s),
            _ => None,
        }
    }
    /// The tag name, if this is a `Tag` variant.
    pub fn tag(&self) -> Option<&str> {
        match self {
            GitRef::Tag(s) => Some(s),
            _ => None,
        }
    }
    /// The commit SHA, if this is a `Rev` variant.
    pub fn rev(&self) -> Option<&str> {
        match self {
            GitRef::Rev(s) => Some(s),
            _ => None,
        }
    }
}

/// Configuration for a single directory in the unified model.
///
/// Custom serde shim (`DirectoryConfigRaw`) keeps the TOML schema flat:
/// users still write `branch = "main"` (or `tag = ...` / `rev = ...`) and
/// the shim collapses these into `Option<GitRef>` at deserialize time,
/// rejecting combinations that set more than one. Eliminates ~20 lines of
/// runtime exclusivity validation that previously lived in
/// `Config::validate`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(try_from = "DirectoryConfigRaw", into = "DirectoryConfigRaw")]
pub struct DirectoryConfig {
    /// Path to the directory
    pub path: PathBuf,

    /// How to discover skills in this directory
    pub directory_type: DirectoryType,

    /// Role in the sync pipeline (defaults based on directory_type)
    pub(crate) role: Option<DirectoryRole>,

    /// Git ref pin (git type only). `None` means "track HEAD of default branch".
    pub git_ref: Option<GitRef>,

    /// Subdirectory within the repo to scan for skills (git type only).
    /// When set, discovery scans `<clone_path>/<subdir>/` instead of the repo root.
    pub subdir: Option<String>,

    /// True iff this directory's `path` was rewritten by a `[directory_overrides.<name>]`
    /// entry in `machine.toml` during config load. Set in `Config::apply_machine_overrides`.
    /// Never appears in `tome.toml` (it's machine-local state, not portable config) — see
    /// `From<DirectoryConfig> for DirectoryConfigRaw` which drops it during serialization.
    /// Default = `false`.
    ///
    /// Wired by Plan 09-03 (status/doctor surfacing — PORT-05): consumed by
    /// `status::gather` and `doctor::check` to render an `(override)` annotation
    /// in text output and an `override_applied: true|false` field in JSON output.
    pub(crate) override_applied: bool,
}

impl DirectoryConfig {
    /// Returns the effective role, defaulting from directory_type if not explicitly set.
    pub fn role(&self) -> DirectoryRole {
        self.role
            .unwrap_or_else(|| self.directory_type.default_role())
    }
}

/// On-disk shape for `DirectoryConfig` — preserves the v0.6 TOML schema
/// (flat `branch` / `tag` / `rev` fields). Converted to/from
/// `DirectoryConfig` via `TryFrom`/`From` so the public type can use the
/// `Option<GitRef>` enum (closes #490).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct DirectoryConfigRaw {
    path: PathBuf,
    #[serde(rename = "type", default)]
    directory_type: DirectoryType,
    #[serde(default)]
    role: Option<DirectoryRole>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    branch: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    tag: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    rev: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    subdir: Option<String>,
}

impl TryFrom<DirectoryConfigRaw> for DirectoryConfig {
    type Error = anyhow::Error;

    fn try_from(raw: DirectoryConfigRaw) -> Result<Self> {
        // Reject combinations that would have all three flat fields set.
        // This validation used to live in `Config::validate`; lifting it
        // here means an invalid config is rejected at deserialize time
        // with line/column context, before any other code runs.
        let git_ref = match (raw.branch, raw.tag, raw.rev) {
            (None, None, None) => None,
            (Some(b), None, None) => Some(GitRef::Branch(b)),
            (None, Some(t), None) => Some(GitRef::Tag(t)),
            (None, None, Some(r)) => Some(GitRef::Rev(r)),
            (b, t, r) => {
                let mut set = Vec::with_capacity(3);
                if b.is_some() {
                    set.push("branch");
                }
                if t.is_some() {
                    set.push("tag");
                }
                if r.is_some() {
                    set.push("rev");
                }
                anyhow::bail!(
                    "directory: branch, tag, and rev are mutually exclusive — \
                     {} are set; pick one",
                    set.join(" and ")
                );
            }
        };
        Ok(Self {
            path: raw.path,
            directory_type: raw.directory_type,
            role: raw.role,
            git_ref,
            subdir: raw.subdir,
            override_applied: false,
        })
    }
}

impl From<DirectoryConfig> for DirectoryConfigRaw {
    fn from(d: DirectoryConfig) -> Self {
        let (branch, tag, rev) = match d.git_ref {
            None => (None, None, None),
            Some(GitRef::Branch(b)) => (Some(b), None, None),
            Some(GitRef::Tag(t)) => (None, Some(t), None),
            Some(GitRef::Rev(r)) => (None, None, Some(r)),
        };
        // `override_applied` is intentionally dropped: it's machine-local
        // state, never written to portable `tome.toml`.
        Self {
            path: d.path,
            directory_type: d.directory_type,
            role: d.role,
            branch,
            tag,
            rev,
            subdir: d.subdir,
        }
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
    #[serde(default = "super::defaults::library_dir")]
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

impl Default for Config {
    fn default() -> Self {
        Self {
            library_dir: super::defaults::library_dir(),
            exclude: BTreeSet::new(),
            directories: BTreeMap::new(),
            backup: BackupConfig::default(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
                // Phase 22 / v0.15 added Managed (pfw and other flat-
                // directory package managers — generalized from
                // ClaudePlugins-only).
                DirectoryRole::Managed,
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
        assert_eq!(dir.git_ref.as_ref().and_then(|r| r.branch()), Some("main"));
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

    #[test]
    fn empty_directories_is_detectable() {
        let config = Config::default();
        assert!(config.directories.is_empty());
    }

    #[test]
    fn default_config_has_empty_directories() {
        let config = Config::default();
        assert!(config.directories.is_empty());
        assert!(config.exclude.is_empty());
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
}
