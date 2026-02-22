//! TOML configuration loading, saving, and validation. Handles tilde expansion and default paths.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// Top-level configuration for tome.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    /// Where the consolidated skill library lives
    #[serde(default = "defaults::library_dir")]
    pub library_dir: PathBuf,

    /// Skills to exclude by name
    #[serde(default)]
    pub exclude: Vec<String>,

    /// Skill sources — order determines priority for duplicates
    #[serde(default)]
    pub sources: Vec<Source>,

    /// Distribution targets
    #[serde(default)]
    pub targets: Targets,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Source {
    /// Display name for this source
    pub name: String,

    /// Path to the source directory
    pub path: PathBuf,

    /// How to discover skills in this source
    #[serde(rename = "type")]
    pub source_type: SourceType,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum SourceType {
    /// Reads installed_plugins.json for plugin-based discovery
    ClaudePlugins,
    /// Scans for */SKILL.md directly
    Directory,
}

impl std::fmt::Display for SourceType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SourceType::ClaudePlugins => write!(f, "claude-plugins"),
            SourceType::Directory => write!(f, "directory"),
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Targets {
    #[serde(default)]
    pub antigravity: Option<TargetConfig>,
    #[serde(default)]
    pub codex: Option<TargetConfig>,
    #[serde(default)]
    pub openclaw: Option<TargetConfig>,
}

impl Targets {
    /// Iterate over all configured targets as (name, config) pairs.
    pub fn iter(&self) -> impl Iterator<Item = (&str, &TargetConfig)> {
        [
            ("antigravity", self.antigravity.as_ref()),
            ("codex", self.codex.as_ref()),
            ("openclaw", self.openclaw.as_ref()),
        ]
        .into_iter()
        .filter_map(|(name, config)| config.map(|c| (name, c)))
    }
}

/// How a target receives skills — each variant carries its required path.
#[derive(Debug, Clone)]
pub enum TargetMethod {
    Symlink { skills_dir: PathBuf },
    Mcp { mcp_config: PathBuf },
}

/// Configuration for a single distribution target.
#[derive(Debug, Clone)]
pub struct TargetConfig {
    pub enabled: bool,
    pub method: TargetMethod,
}

impl TargetConfig {
    /// Returns the skills directory if this is a symlink target.
    pub fn skills_dir(&self) -> Option<&Path> {
        match &self.method {
            TargetMethod::Symlink { skills_dir } => Some(skills_dir),
            TargetMethod::Mcp { .. } => None,
        }
    }

    /// Returns the MCP config path if this is an MCP target.
    pub fn mcp_config(&self) -> Option<&Path> {
        match &self.method {
            TargetMethod::Mcp { mcp_config } => Some(mcp_config),
            TargetMethod::Symlink { .. } => None,
        }
    }
}

// --- Serde layer: flat TOML format ↔ TargetConfig ---

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
enum DistributionMethod {
    Symlink,
    Mcp,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct RawTargetConfig {
    enabled: bool,
    method: DistributionMethod,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    skills_dir: Option<PathBuf>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    mcp_config: Option<PathBuf>,
}

impl TryFrom<RawTargetConfig> for TargetConfig {
    type Error = anyhow::Error;

    fn try_from(raw: RawTargetConfig) -> Result<Self> {
        let method = match raw.method {
            DistributionMethod::Symlink => {
                let skills_dir = raw
                    .skills_dir
                    .ok_or_else(|| anyhow::anyhow!("symlink target requires skills_dir"))?;
                TargetMethod::Symlink { skills_dir }
            }
            DistributionMethod::Mcp => {
                let mcp_config = raw
                    .mcp_config
                    .ok_or_else(|| anyhow::anyhow!("mcp target requires mcp_config"))?;
                TargetMethod::Mcp { mcp_config }
            }
        };
        Ok(TargetConfig {
            enabled: raw.enabled,
            method,
        })
    }
}

impl From<&TargetConfig> for RawTargetConfig {
    fn from(tc: &TargetConfig) -> Self {
        match &tc.method {
            TargetMethod::Symlink { skills_dir } => RawTargetConfig {
                enabled: tc.enabled,
                method: DistributionMethod::Symlink,
                skills_dir: Some(skills_dir.clone()),
                mcp_config: None,
            },
            TargetMethod::Mcp { mcp_config } => RawTargetConfig {
                enabled: tc.enabled,
                method: DistributionMethod::Mcp,
                skills_dir: None,
                mcp_config: Some(mcp_config.clone()),
            },
        }
    }
}

impl Serialize for TargetConfig {
    fn serialize<S: serde::Serializer>(
        &self,
        serializer: S,
    ) -> std::result::Result<S::Ok, S::Error> {
        RawTargetConfig::from(self).serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for TargetConfig {
    fn deserialize<D: serde::Deserializer<'de>>(
        deserializer: D,
    ) -> std::result::Result<Self, D::Error> {
        let raw = RawTargetConfig::deserialize(deserializer)?;
        TargetConfig::try_from(raw).map_err(serde::de::Error::custom)
    }
}

impl Config {
    /// Load config from file, or return defaults if file doesn't exist.
    pub fn load(path: &Path) -> Result<Self> {
        if path.exists() {
            let content = std::fs::read_to_string(path)
                .with_context(|| format!("failed to read {}", path.display()))?;
            let mut config: Config = toml::from_str(&content)
                .with_context(|| format!("failed to parse {}", path.display()))?;
            config.expand_tildes()?;
            Ok(config)
        } else {
            Ok(Self::default())
        }
    }

    /// Load from CLI-provided path or default location.
    pub fn load_or_default(cli_path: Option<&Path>) -> Result<Self> {
        let path = match cli_path {
            Some(p) => p.to_path_buf(),
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
    pub fn validate(&self) -> Result<()> {
        // library_dir exists but is a file, not a directory
        if self.library_dir.exists() && !self.library_dir.is_dir() {
            anyhow::bail!(
                "library_dir exists but is not a directory: {}",
                self.library_dir.display()
            );
        }

        // Empty source names
        for source in &self.sources {
            anyhow::ensure!(!source.name.is_empty(), "source name cannot be empty");
        }

        // Duplicate source names
        let mut seen = std::collections::HashSet::new();
        for source in &self.sources {
            anyhow::ensure!(
                seen.insert(&source.name),
                "duplicate source name: '{}'",
                source.name
            );
        }

        Ok(())
    }

    /// Expand `~` in all path fields.
    fn expand_tildes(&mut self) -> Result<()> {
        self.library_dir = expand_tilde(&self.library_dir)?;
        for source in &mut self.sources {
            source.path = expand_tilde(&source.path)?;
        }
        if let Some(ref mut t) = self.targets.antigravity {
            expand_target_tildes(t)?;
        }
        if let Some(ref mut t) = self.targets.codex {
            expand_target_tildes(t)?;
        }
        if let Some(ref mut t) = self.targets.openclaw {
            expand_target_tildes(t)?;
        }
        Ok(())
    }
}

fn expand_target_tildes(t: &mut TargetConfig) -> Result<()> {
    match &mut t.method {
        TargetMethod::Symlink { skills_dir } => {
            *skills_dir = expand_tilde(skills_dir)?;
        }
        TargetMethod::Mcp { mcp_config } => {
            *mcp_config = expand_tilde(mcp_config)?;
        }
    }
    Ok(())
}

impl Default for Config {
    fn default() -> Self {
        Self {
            library_dir: defaults::library_dir(),
            exclude: Vec::new(),
            sources: Vec::new(),
            targets: Targets::default(),
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

/// Default config file path: ~/.config/tome/config.toml
pub fn default_config_path() -> Result<PathBuf> {
    Ok(dirs::home_dir()
        .context("could not determine home directory")?
        .join(".config")
        .join("tome")
        .join("config.toml"))
}

mod defaults {
    use std::path::PathBuf;

    pub fn library_dir() -> PathBuf {
        dirs::home_dir()
            .expect("could not determine home directory — is $HOME set?")
            .join(".local")
            .join("share")
            .join("tome")
            .join("skills")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

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
    fn default_config_has_empty_sources() {
        let config = Config::default();
        assert!(config.sources.is_empty());
        assert!(config.exclude.is_empty());
    }

    #[test]
    fn config_loads_defaults_when_file_missing() {
        let config = Config::load(Path::new("/nonexistent/path/config.toml")).unwrap();
        assert!(config.sources.is_empty());
    }

    #[test]
    fn config_roundtrip_toml() {
        let config = Config {
            library_dir: PathBuf::from("/tmp/skills"),
            exclude: vec!["test-skill".into()],
            sources: vec![Source {
                name: "test".into(),
                path: PathBuf::from("/tmp/source"),
                source_type: SourceType::Directory,
            }],
            targets: Targets::default(),
        };
        let toml_str = toml::to_string_pretty(&config).unwrap();
        let parsed: Config = toml::from_str(&toml_str).unwrap();
        assert_eq!(parsed.library_dir, config.library_dir);
        assert_eq!(parsed.exclude, config.exclude);
        assert_eq!(parsed.sources.len(), 1);
        assert_eq!(parsed.sources[0].name, "test");
    }

    #[test]
    fn config_load_fails_on_malformed_toml() {
        let dir = tempfile::TempDir::new().unwrap();
        let path = dir.path().join("config.toml");
        std::fs::write(&path, "this is [[[not valid toml").unwrap();
        assert!(Config::load(&path).is_err());
    }

    #[test]
    fn validate_passes_for_valid_config() {
        let config = Config {
            library_dir: PathBuf::from("/tmp/nonexistent-lib"),
            exclude: Vec::new(),
            sources: vec![Source {
                name: "test".into(),
                path: PathBuf::from("/tmp/source"),
                source_type: SourceType::Directory,
            }],
            targets: Targets {
                antigravity: Some(TargetConfig {
                    enabled: true,
                    method: TargetMethod::Symlink {
                        skills_dir: PathBuf::from("/tmp/target"),
                    },
                }),
                ..Default::default()
            },
        };
        config.validate().unwrap();
    }

    #[test]
    fn validate_rejects_empty_source_name() {
        let config = Config {
            sources: vec![Source {
                name: "".into(),
                path: PathBuf::from("/tmp"),
                source_type: SourceType::Directory,
            }],
            ..Default::default()
        };
        let err = config.validate().unwrap_err();
        assert!(
            err.to_string().contains("cannot be empty"),
            "unexpected error: {err}"
        );
    }

    #[test]
    fn validate_rejects_duplicate_source_names() {
        let config = Config {
            sources: vec![
                Source {
                    name: "dupe".into(),
                    path: PathBuf::from("/tmp/a"),
                    source_type: SourceType::Directory,
                },
                Source {
                    name: "dupe".into(),
                    path: PathBuf::from("/tmp/b"),
                    source_type: SourceType::Directory,
                },
            ],
            ..Default::default()
        };
        let err = config.validate().unwrap_err();
        assert!(
            err.to_string().contains("duplicate source name"),
            "unexpected error: {err}"
        );
    }

    #[test]
    fn try_from_raw_rejects_symlink_without_skills_dir() {
        let raw = RawTargetConfig {
            enabled: true,
            method: DistributionMethod::Symlink,
            skills_dir: None,
            mcp_config: None,
        };
        let err = TargetConfig::try_from(raw).unwrap_err();
        assert!(
            err.to_string().contains("requires skills_dir"),
            "unexpected error: {err}"
        );
    }

    #[test]
    fn try_from_raw_rejects_mcp_without_mcp_config() {
        let raw = RawTargetConfig {
            enabled: true,
            method: DistributionMethod::Mcp,
            skills_dir: None,
            mcp_config: None,
        };
        let err = TargetConfig::try_from(raw).unwrap_err();
        assert!(
            err.to_string().contains("requires mcp_config"),
            "unexpected error: {err}"
        );
    }

    #[test]
    fn target_config_roundtrip_symlink() {
        let tc = TargetConfig {
            enabled: true,
            method: TargetMethod::Symlink {
                skills_dir: PathBuf::from("/tmp/skills"),
            },
        };
        let toml_str = toml::to_string_pretty(&tc).unwrap();
        let parsed: TargetConfig = toml::from_str(&toml_str).unwrap();
        assert!(parsed.enabled);
        assert_eq!(parsed.skills_dir(), Some(Path::new("/tmp/skills")));
        assert!(parsed.mcp_config().is_none());
    }

    #[test]
    fn target_config_roundtrip_mcp() {
        let tc = TargetConfig {
            enabled: true,
            method: TargetMethod::Mcp {
                mcp_config: PathBuf::from("/tmp/.mcp.json"),
            },
        };
        let toml_str = toml::to_string_pretty(&tc).unwrap();
        let parsed: TargetConfig = toml::from_str(&toml_str).unwrap();
        assert!(parsed.enabled);
        assert_eq!(parsed.mcp_config(), Some(Path::new("/tmp/.mcp.json")));
        assert!(parsed.skills_dir().is_none());
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
library_dir = "~/.local/share/tome/skills"
exclude = ["deprecated-skill"]

[[sources]]
name = "claude-plugins"
path = "~/.claude/plugins/cache"
type = "claude-plugins"

[[sources]]
name = "standalone"
path = "~/.claude/skills"
type = "directory"

[targets.antigravity]
enabled = true
method = "symlink"
skills_dir = "~/.gemini/antigravity/skills"

[targets.codex]
enabled = true
method = "mcp"
mcp_config = "~/.codex/.mcp.json"
"#;
        let config: Config = toml::from_str(toml_str).unwrap();
        assert_eq!(config.sources.len(), 2);
        assert!(config.targets.antigravity.is_some());
        assert!(config.targets.codex.is_some());
        assert!(config.targets.openclaw.is_none());
    }
}
