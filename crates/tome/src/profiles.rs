//! Layered pool, profile, and local runtime configuration.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

use crate::config::{BackupConfig, Config, DirectoryConfig, DirectoryName};
use crate::discover::SkillName;
use crate::machine::{AutoInstall, DirectoryPrefs, MachinePrefs};

/// Local consent for synchronising the shared pool repository.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum GitSyncPolicy {
    #[default]
    Ask,
    Always,
    Never,
}

/// Runtime preference for backup commands. Kept local rather than in profiles.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum BackupRuntimePolicy {
    #[default]
    Ask,
    Always,
    Never,
}

/// Shared policy persisted in `tome.toml`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct PoolPolicy {
    #[serde(default = "crate::config::defaults::library_dir")]
    library_dir: PathBuf,
    #[serde(default)]
    exclude: BTreeSet<SkillName>,
    #[serde(default)]
    backup: BackupConfig,
    #[serde(default)]
    source_pins: BTreeMap<SkillName, String>,
}

/// Shared choices used by pool reconciliation. Persisted only in pool policy.
#[derive(Debug, Clone, Default)]
pub(crate) struct PoolSettings {
    pub exclude: BTreeSet<SkillName>,
    pub source_pins: BTreeMap<SkillName, String>,
}

pub(crate) fn load_pool_settings(config_path: &Path) -> Result<PoolSettings> {
    let text = std::fs::read_to_string(config_path)
        .with_context(|| format!("failed to read {}", config_path.display()))?;
    let policy: PoolPolicy = toml::from_str(&text)
        .with_context(|| format!("failed to parse {}", config_path.display()))?;
    Ok(PoolSettings { exclude: policy.exclude, source_pins: policy.source_pins })
}

pub(crate) fn save_pool_settings(
    config_path: &Path,
    settings: &PoolSettings,
) -> Result<()> {
    let text = std::fs::read_to_string(config_path)
        .with_context(|| format!("failed to read {}", config_path.display()))?;
    let mut policy: PoolPolicy = toml::from_str(&text)
        .with_context(|| format!("failed to parse {}", config_path.display()))?;
    policy.exclude = settings.exclude.clone();
    policy.source_pins = settings.source_pins.clone();
    let content = checked_toml(&policy, "pool policy")?;
    atomic_write(config_path, &content)
}

/// Complete, committed topology and distribution preferences for one profile.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MachineProfile {
    #[serde(default)]
    directories: BTreeMap<DirectoryName, DirectoryConfig>,
    #[serde(default)]
    disabled: BTreeSet<SkillName>,
    #[serde(default)]
    disabled_directories: BTreeSet<DirectoryName>,
    #[serde(default)]
    directory: BTreeMap<DirectoryName, DirectoryPrefs>,
}

/// Local, uncommitted settings selecting a profile and runtime policies.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LocalSettings {
    #[serde(default)]
    pub profile: Option<String>,
    #[serde(default)]
    pub git_sync: GitSyncPolicy,
    #[serde(default)]
    pub managed_plugin_install: Option<AutoInstall>,
    #[serde(default)]
    pub backup_runtime: BackupRuntimePolicy,
}

/// Validated operational input for all normal commands.
#[derive(Debug, Clone)]
pub struct EffectiveContext {
    pub config: Config,
    pub machine_prefs: MachinePrefs,
    pub profile: DirectoryName,
    pub git_sync: GitSyncPolicy,
    pub managed_plugin_install: Option<AutoInstall>,
    pub backup_runtime: BackupRuntimePolicy,
}

/// Load the selected local profile and project it into the existing effective models.
pub fn load_effective_context(
    config_path: &Path,
    settings_path: &Path,
) -> Result<EffectiveContext> {
    let settings = load_settings(settings_path)?;
    let profile_name = settings
        .profile
        .as_deref()
        .context(profile_recovery_message())
        .and_then(|name| DirectoryName::new(name.to_owned()).context(profile_recovery_message()))?;
    let config_dir = config_path
        .parent()
        .context("config path has no parent directory")?;
    let profile_path = config_dir
        .join("machines")
        .join(format!("{}.toml", profile_name.as_str()));

    let pool_text = std::fs::read_to_string(config_path)
        .with_context(|| format!("failed to read {}", config_path.display()))?;
    let pool: PoolPolicy = toml::from_str(&pool_text)
        .with_context(|| format!("failed to parse {}", config_path.display()))?;
    anyhow::ensure!(
        profile_path.is_file(),
        "selected profile '{}' does not exist at {}. {}",
        profile_name,
        profile_path.display(),
        profile_recovery_message()
    );
    let profile_text = std::fs::read_to_string(&profile_path)
        .with_context(|| format!("failed to read {}", profile_path.display()))?;
    let profile: MachineProfile = toml::from_str(&profile_text)
        .with_context(|| format!("failed to parse {}", profile_path.display()))?;

    let mut config = Config::from_layers(
        pool.library_dir,
        pool.exclude,
        pool.backup,
        profile.directories,
    );
    config.expand_tildes()?;
    config.validate()?;
    let prefs = MachinePrefs::from_profile(
        profile.disabled,
        profile.disabled_directories,
        profile.directory,
        settings.managed_plugin_install,
    )?;
    Ok(EffectiveContext {
        config,
        machine_prefs: prefs,
        profile: profile_name,
        git_sync: settings.git_sync,
        managed_plugin_install: settings.managed_plugin_install,
        backup_runtime: settings.backup_runtime,
    })
}

pub fn load_settings(path: &Path) -> Result<LocalSettings> {
    let text = std::fs::read_to_string(path).with_context(|| {
        format!(
            "local settings are required at {}. {}",
            path.display(),
            profile_recovery_message()
        )
    })?;
    toml::from_str(&text).with_context(|| format!("failed to parse {}", path.display()))
}

pub fn save_settings(settings: &LocalSettings, path: &Path) -> Result<()> {
    let content = toml::to_string_pretty(settings).context("failed to serialize local settings")?;
    let reparsed: LocalSettings =
        toml::from_str(&content).context("round-trip: generated local settings did not reparse")?;
    anyhow::ensure!(
        content
            == toml::to_string_pretty(&reparsed).context("failed to serialize local settings")?,
        "round-trip mismatch while serializing local settings"
    );
    atomic_write(path, &content)
}

/// Create an empty committed machine profile without local runtime policy.
pub fn create_profile(config_path: &Path, name: &str) -> Result<()> {
    let name = DirectoryName::new(name.to_owned())?;
    let path = profile_path(config_path, &name)?;
    anyhow::ensure!(
        !path.exists(),
        "profile '{}' already exists at {}",
        name,
        path.display()
    );
    save_profile(&MachineProfile::default(), &path)
}

/// List valid committed machine profile names in deterministic order.
pub fn list_profiles(config_path: &Path) -> Result<Vec<DirectoryName>> {
    let config_dir = config_path
        .parent()
        .context("config path has no parent directory")?;
    let machines_dir = config_dir.join("machines");
    if !machines_dir.exists() {
        return Ok(Vec::new());
    }

    let mut profiles = Vec::new();
    for entry in std::fs::read_dir(&machines_dir)
        .with_context(|| format!("failed to read {}", machines_dir.display()))?
    {
        let path = entry
            .with_context(|| format!("failed to read {}", machines_dir.display()))?
            .path();
        if path
            .extension()
            .is_some_and(|extension| extension == "toml")
            && let Some(name) = path.file_stem().and_then(|name| name.to_str())
        {
            profiles.push(DirectoryName::new(name.to_owned())?);
        }
    }
    profiles.sort();
    Ok(profiles)
}

pub fn select_profile(settings_path: &Path, config_path: &Path, name: &str) -> Result<()> {
    let name = DirectoryName::new(name.to_owned())?;
    let profile_path = profile_path(config_path, &name)?;
    anyhow::ensure!(
        profile_path.is_file(),
        "profile '{}' does not exist at {}",
        name,
        profile_path.display()
    );
    let mut settings = if settings_path.exists() {
        load_settings(settings_path)?
    } else {
        LocalSettings::default()
    };
    settings.profile = Some(name.to_string());
    save_settings(&settings, settings_path)
}

pub fn profile_recovery_message() -> &'static str {
    "Available profiles are in <config-dir>/machines. Run `tome profile select <name>` or `tome init`."
}

fn atomic_write(path: &Path, content: &str) -> Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("failed to create {}", parent.display()))?;
    }
    let tmp = path.with_extension("toml.tmp");
    std::fs::write(&tmp, content).with_context(|| format!("failed to write {}", tmp.display()))?;
    if let Err(error) = std::fs::rename(&tmp, path) {
        let _ = std::fs::remove_file(&tmp);
        return Err(error).with_context(|| format!("failed to rename {}", path.display()));
    }
    Ok(())
}

fn profile_path(config_path: &Path, name: &DirectoryName) -> Result<PathBuf> {
    let config_dir = config_path
        .parent()
        .context("config path has no parent directory")?;
    Ok(config_dir.join("machines").join(format!("{name}.toml")))
}

fn save_profile(profile: &MachineProfile, path: &Path) -> Result<()> {
    let content = toml::to_string_pretty(profile).context("failed to serialize machine profile")?;
    let reparsed: MachineProfile = toml::from_str(&content)
        .context("round-trip: generated machine profile did not reparse")?;
    anyhow::ensure!(
        content
            == toml::to_string_pretty(&reparsed).context("failed to serialize machine profile")?,
        "round-trip mismatch while serializing machine profile"
    );
    atomic_write(path, &content)
}

/// Produce checked TOML for a legacy-layout migration without writing it.
///
/// Kept here because the persisted layer fields are deliberately private to this
/// module; callers must not be able to accidentally put local consent in a
/// committed profile.
pub(crate) fn migration_layers(
    legacy: &Config,
    prefs: &MachinePrefs,
) -> Result<(String, String, String)> {
    let pool = PoolPolicy {
        library_dir: legacy.library_dir.clone(),
        exclude: legacy.exclude.clone(),
        backup: legacy.backup.clone(),
        source_pins: BTreeMap::new(),
    };
    let profile = MachineProfile {
        directories: legacy.directories.clone(),
        disabled: prefs.disabled.clone(),
        disabled_directories: prefs.disabled_directories.clone(),
        directory: prefs.directory.clone(),
    };
    let settings = LocalSettings {
        profile: None,
        git_sync: GitSyncPolicy::default(),
        managed_plugin_install: prefs.auto_install_plugins,
        backup_runtime: BackupRuntimePolicy::default(),
    };
    checked_toml(&pool, "pool policy")
        .and_then(|pool| checked_toml(&profile, "machine profile").map(|profile| (pool, profile)))
        .and_then(|(pool, profile)| {
            checked_toml(&settings, "local settings").map(|settings| (pool, profile, settings))
        })
}

pub(crate) fn migration_settings(profile: &DirectoryName, prefs: &MachinePrefs) -> Result<String> {
    checked_toml(
        &LocalSettings {
            profile: Some(profile.to_string()),
            git_sync: GitSyncPolicy::default(),
            managed_plugin_install: prefs.auto_install_plugins,
            backup_runtime: BackupRuntimePolicy::default(),
        },
        "local settings",
    )
}

pub(crate) fn atomic_write_bytes(path: &Path, content: &str) -> Result<()> {
    atomic_write(path, content)
}

fn checked_toml<T>(value: &T, label: &str) -> Result<String>
where
    T: Serialize + for<'de> Deserialize<'de>,
{
    let content =
        toml::to_string_pretty(value).with_context(|| format!("failed to serialize {label}"))?;
    let reparsed: T = toml::from_str(&content)
        .with_context(|| format!("round-trip: generated {label} did not reparse"))?;
    anyhow::ensure!(
        content
            == toml::to_string_pretty(&reparsed)
                .with_context(|| format!("failed to serialize {label}"))?,
        "round-trip mismatch while serializing {label}"
    );
    Ok(content)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn profile_name_is_not_inferred() {
        let tmp = tempfile::TempDir::new().unwrap();
        let err = load_effective_context(
            &tmp.path().join("tome.toml"),
            &tmp.path().join("settings.toml"),
        )
        .unwrap_err();
        assert!(err.to_string().contains("profile select"));
    }

    #[test]
    fn profile_serialization_excludes_local_runtime_policy() {
        let serialized = toml::to_string_pretty(&MachineProfile::default()).unwrap();
        assert!(!serialized.contains("git_sync"));
        assert!(!serialized.contains("managed_plugin_install"));
        assert!(!serialized.contains("backup_runtime"));
    }

    #[test]
    fn unchanged_layers_project_deterministically() {
        let tmp = tempfile::TempDir::new().unwrap();
        let config_path = tmp.path().join("tome.toml");
        let settings_path = tmp.path().join("settings.toml");
        std::fs::create_dir_all(tmp.path().join("machines")).unwrap();
        std::fs::write(
            &config_path,
            format!(
                "library_dir = \"{}\"\n",
                tmp.path().join("library").display()
            ),
        )
        .unwrap();
        std::fs::write(tmp.path().join("machines/work.toml"), "").unwrap();
        std::fs::write(
            &settings_path,
            "profile = \"work\"\ngit_sync = \"never\"\nmanaged_plugin_install = \"never\"\nbackup_runtime = \"always\"\n",
        )
        .unwrap();

        let first = load_effective_context(&config_path, &settings_path).unwrap();
        let second = load_effective_context(&config_path, &settings_path).unwrap();
        assert_eq!(format!("{first:?}"), format!("{second:?}"));
    }
}
