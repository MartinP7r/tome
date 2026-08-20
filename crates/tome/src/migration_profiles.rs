//! Interactive, recoverable migration from the legacy portable configuration.

use std::io::{IsTerminal, Write};
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};

use crate::config::{Config, DirectoryName};
use crate::machine::MachinePrefs;
use crate::profiles;

const JOURNAL_FILE: &str = ".tome-profiles-migration.toml";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum FailurePoint {
    BeforeJournal,
    AfterJournal,
    BeforePool,
    AfterPool,
    BeforeProfile,
    AfterProfile,
    BeforeSettings,
    AfterSettings,
    BeforeValidate,
    AfterValidate,
    BeforeRetireLegacy,
    AfterRetireLegacy,
    BeforeJournalCompletion,
    AfterJournalCompletion,
}

#[derive(Debug)]
pub(crate) struct MigrationPlan {
    profile: DirectoryName,
    legacy_config: PathBuf,
    legacy_machine: PathBuf,
    pool_path: PathBuf,
    profile_path: PathBuf,
    settings_path: PathBuf,
    journal_path: PathBuf,
    pool: String,
    profile_toml: String,
    settings: String,
    legacy_config_bytes: String,
    legacy_machine_bytes: String,
    previous_settings_bytes: Option<String>,
}

/// A legacy layout is identified by portable directory topology in tome.toml
/// plus the old local machine preferences file. New pool policies never carry
/// a `directories` key.
pub(crate) fn legacy_layout(config_path: &Path, machine_path: &Path) -> Result<bool> {
    if !config_path.is_file() || !machine_path.is_file() {
        return Ok(false);
    }
    let config = std::fs::read_to_string(config_path)
        .with_context(|| format!("failed to read {}", config_path.display()))?;
    let value: toml::Value = toml::from_str(&config)
        .with_context(|| format!("failed to parse {}", config_path.display()))?;
    Ok(value.get("directories").is_some())
}

pub(crate) fn plan(
    config_path: &Path,
    machine_path: &Path,
    settings_path: &Path,
    name: &str,
) -> Result<MigrationPlan> {
    let profile = DirectoryName::new(name.to_owned())?;
    anyhow::ensure!(
        legacy_layout(config_path, machine_path)?,
        "no legacy configuration was found to migrate"
    );
    let config_dir = config_path
        .parent()
        .context("config path has no parent directory")?;
    let profile_path = config_dir.join("machines").join(format!("{profile}.toml"));
    anyhow::ensure!(
        !profile_path.exists(),
        "profile '{profile}' already exists at {}",
        profile_path.display()
    );
    let legacy_config_bytes = std::fs::read_to_string(config_path)
        .with_context(|| format!("failed to read {}", config_path.display()))?;
    let legacy_machine_bytes = std::fs::read_to_string(machine_path)
        .with_context(|| format!("failed to read {}", machine_path.display()))?;
    let previous_settings_bytes = if settings_path.exists() {
        Some(
            std::fs::read_to_string(settings_path)
                .with_context(|| format!("failed to read {}", settings_path.display()))?,
        )
    } else {
        None
    };
    let mut legacy: Config = toml::from_str(&legacy_config_bytes)
        .with_context(|| format!("failed to parse {}", config_path.display()))?;
    let prefs: MachinePrefs = toml::from_str(&legacy_machine_bytes)
        .with_context(|| format!("failed to parse {}", machine_path.display()))?;
    prefs.validate()?;
    // Legacy overrides are part of the effective topology. Apply them before
    // serializing the profile so their syntax remains only in the backup.
    legacy.expand_tildes()?;
    legacy.apply_machine_overrides(&prefs)?;
    legacy.validate()?;
    let (pool, profile_toml, _) = profiles::migration_layers(&legacy, &prefs)?;
    let settings = profiles::migration_settings(&profile, &prefs)?;
    Ok(MigrationPlan {
        profile,
        legacy_config: config_path.to_path_buf(),
        legacy_machine: machine_path.to_path_buf(),
        pool_path: config_path.to_path_buf(),
        profile_path,
        settings_path: settings_path.to_path_buf(),
        journal_path: config_dir.join(JOURNAL_FILE),
        pool,
        profile_toml,
        settings,
        legacy_config_bytes,
        legacy_machine_bytes,
        previous_settings_bytes,
    })
}

pub(crate) fn render_plan_to(plan: &MigrationPlan, writer: &mut impl Write) -> std::io::Result<()> {
    writeln!(writer, "Profile migration plan for '{}':", plan.profile)?;
    writeln!(writer, "  pool policy: {}", plan.pool_path.display())?;
    writeln!(writer, "  machine profile: {}", plan.profile_path.display())?;
    writeln!(writer, "  local settings: {}", plan.settings_path.display())?;
    writeln!(
        writer,
        "  legacy preferences retired last: {}",
        plan.legacy_machine.display()
    )?;
    writeln!(writer, "\n--- pool policy ---\n{}", plan.pool)?;
    writeln!(writer, "--- profile ---\n{}", plan.profile_toml)?;
    writeln!(writer, "--- settings ---\n{}", plan.settings)?;
    writeln!(
        writer,
        "Backups are timestamped siblings. Restore them manually after a completed migration."
    )
}

pub(crate) fn execute(plan: &MigrationPlan, fail_at: Option<FailurePoint>) -> Result<()> {
    backup(&plan.legacy_config, &plan.legacy_config_bytes)?;
    backup(&plan.legacy_machine, &plan.legacy_machine_bytes)?;
    if let Some(settings) = &plan.previous_settings_bytes {
        backup(&plan.settings_path, settings)?;
    }
    let journal = Journal::from_plan(plan);
    fail(fail_at, FailurePoint::BeforeJournal)?;
    write_journal(&plan.journal_path, &journal)?;
    fail(fail_at, FailurePoint::AfterJournal)?;
    fail(fail_at, FailurePoint::BeforePool)?;
    profiles::atomic_write_bytes(&plan.pool_path, &plan.pool)?;
    fail(fail_at, FailurePoint::AfterPool)?;
    fail(fail_at, FailurePoint::BeforeProfile)?;
    profiles::atomic_write_bytes(&plan.profile_path, &plan.profile_toml)?;
    fail(fail_at, FailurePoint::AfterProfile)?;
    fail(fail_at, FailurePoint::BeforeSettings)?;
    profiles::atomic_write_bytes(&plan.settings_path, &plan.settings)?;
    fail(fail_at, FailurePoint::AfterSettings)?;
    fail(fail_at, FailurePoint::BeforeValidate)?;
    profiles::load_effective_context(&plan.pool_path, &plan.settings_path)?;
    fail(fail_at, FailurePoint::AfterValidate)?;
    fail(fail_at, FailurePoint::BeforeRetireLegacy)?;
    std::fs::remove_file(&plan.legacy_machine)
        .with_context(|| format!("failed to retire {}", plan.legacy_machine.display()))?;
    fail(fail_at, FailurePoint::AfterRetireLegacy)?;
    fail(fail_at, FailurePoint::BeforeJournalCompletion)?;
    std::fs::remove_file(&plan.journal_path).with_context(|| {
        format!(
            "failed to complete migration journal {}",
            plan.journal_path.display()
        )
    })?;
    fail(fail_at, FailurePoint::AfterJournalCompletion)?;
    Ok(())
}

pub(crate) fn recover(config_path: &Path) -> Result<()> {
    let journal_path = config_path
        .parent()
        .context("config path has no parent directory")?
        .join(JOURNAL_FILE);
    if !journal_path.exists() {
        return Ok(());
    }
    let journal: Journal =
        toml::from_str(&std::fs::read_to_string(&journal_path)?).with_context(|| {
            format!(
                "failed to parse migration journal {}",
                journal_path.display()
            )
        })?;
    let complete_new = matches_bytes(&journal.pool_path, &journal.pool)
        && matches_bytes(&journal.profile_path, &journal.profile)
        && matches_bytes(&journal.settings_path, &journal.settings);
    if complete_new {
        // Validate the complete new state before retiring the legacy input.
        profiles::load_effective_context(&journal.pool_path, &journal.settings_path)?;
        if journal.legacy_machine.exists() {
            std::fs::remove_file(&journal.legacy_machine)?;
        }
    } else {
        profiles::atomic_write_bytes(&journal.legacy_config, &journal.legacy_config_bytes)?;
        profiles::atomic_write_bytes(&journal.legacy_machine, &journal.legacy_machine_bytes)?;
        let _ = std::fs::remove_file(&journal.profile_path);
        if let Some(settings) = &journal.previous_settings_bytes {
            profiles::atomic_write_bytes(&journal.settings_path, settings)?;
        } else {
            let _ = std::fs::remove_file(&journal.settings_path);
        }
    }
    std::fs::remove_file(&journal_path).with_context(|| {
        format!(
            "failed to complete migration journal {}",
            journal_path.display()
        )
    })?;
    Ok(())
}

pub(crate) fn require_interactive(no_input: bool, dry_run: bool) -> Result<()> {
    anyhow::ensure!(
        !no_input && !dry_run,
        "tome migrate profiles is interactive-only; remove --no-input and --dry-run"
    );
    anyhow::ensure!(
        std::io::stdin().is_terminal() && std::io::stderr().is_terminal(),
        "tome migrate profiles requires a terminal"
    );
    Ok(())
}

pub(crate) fn confirm() -> Result<bool> {
    Ok(dialoguer::Confirm::new()
        .with_prompt("Apply this migration?")
        .default(false)
        .interact_opt()?
        .unwrap_or(false))
}

fn fail(at: Option<FailurePoint>, point: FailurePoint) -> Result<()> {
    anyhow::ensure!(at != Some(point), "injected migration failure at {point:?}");
    Ok(())
}

fn backup(path: &Path, bytes: &str) -> Result<()> {
    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)?
        .as_nanos();
    let backup = path.with_file_name(format!(
        "{}.pre-profiles-{stamp}",
        path.file_name().unwrap().to_string_lossy()
    ));
    profiles::atomic_write_bytes(&backup, bytes)
}

fn matches_bytes(path: &Path, expected: &str) -> bool {
    std::fs::read_to_string(path).is_ok_and(|actual| actual == expected)
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
struct Journal {
    legacy_config: PathBuf,
    legacy_machine: PathBuf,
    pool_path: PathBuf,
    profile_path: PathBuf,
    settings_path: PathBuf,
    pool: String,
    profile: String,
    settings: String,
    legacy_config_bytes: String,
    legacy_machine_bytes: String,
    previous_settings_bytes: Option<String>,
}
impl Journal {
    fn from_plan(plan: &MigrationPlan) -> Self {
        Self {
            legacy_config: plan.legacy_config.clone(),
            legacy_machine: plan.legacy_machine.clone(),
            pool_path: plan.pool_path.clone(),
            profile_path: plan.profile_path.clone(),
            settings_path: plan.settings_path.clone(),
            pool: plan.pool.clone(),
            profile: plan.profile_toml.clone(),
            settings: plan.settings.clone(),
            legacy_config_bytes: plan.legacy_config_bytes.clone(),
            legacy_machine_bytes: plan.legacy_machine_bytes.clone(),
            previous_settings_bytes: plan.previous_settings_bytes.clone(),
        }
    }
}
fn write_journal(path: &Path, journal: &Journal) -> Result<()> {
    profiles::atomic_write_bytes(path, &toml::to_string_pretty(journal)?)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn fixture() -> (tempfile::TempDir, PathBuf, PathBuf, PathBuf) {
        let tmp = tempfile::TempDir::new().unwrap();
        let config = tmp.path().join("tome.toml");
        let machine = tmp.path().join("machine.toml");
        let settings = tmp.path().join("settings.toml");
        std::fs::write(&config, format!("library_dir = \"{}\"\n\n[directories.source]\npath = \"{}\"\ntype = \"directory\"\nrole = \"source\"\n", tmp.path().join("library").display(), tmp.path().join("source").display())).unwrap();
        std::fs::write(&machine, "disabled = []\n").unwrap();
        (tmp, config, machine, settings)
    }
    #[test]
    fn migration_recovers_each_interruption_to_one_complete_layout() {
        for point in [
            FailurePoint::BeforeJournal,
            FailurePoint::AfterJournal,
            FailurePoint::BeforePool,
            FailurePoint::AfterPool,
            FailurePoint::BeforeProfile,
            FailurePoint::AfterProfile,
            FailurePoint::BeforeSettings,
            FailurePoint::AfterSettings,
            FailurePoint::BeforeValidate,
            FailurePoint::AfterValidate,
            FailurePoint::BeforeRetireLegacy,
            FailurePoint::AfterRetireLegacy,
            FailurePoint::BeforeJournalCompletion,
            FailurePoint::AfterJournalCompletion,
        ] {
            let (_tmp, config, machine, settings) = fixture();
            let legacy_config = std::fs::read_to_string(&config).unwrap();
            let legacy_machine = std::fs::read_to_string(&machine).unwrap();
            let plan = plan(&config, &machine, &settings, "work").unwrap();
            assert!(execute(&plan, Some(point)).is_err());
            recover(&config).unwrap();

            let restored_legacy = std::fs::read_to_string(&config).unwrap() == legacy_config
                && std::fs::read_to_string(&machine).unwrap() == legacy_machine
                && !plan.profile_path.exists()
                && !settings.exists();
            let completed_new = std::fs::read_to_string(&config).unwrap() == plan.pool
                && std::fs::read_to_string(&plan.profile_path).unwrap() == plan.profile_toml
                && std::fs::read_to_string(&settings).unwrap() == plan.settings
                && !machine.exists()
                && profiles::load_effective_context(&config, &settings).is_ok();
            assert!(
                restored_legacy || completed_new,
                "{point:?} left neither complete layout"
            );
            assert!(!plan.journal_path.exists());
        }
    }
    #[test]
    fn completed_migration_is_idempotent() {
        let (_tmp, config, machine, settings) = fixture();
        let migration = plan(&config, &machine, &settings, "work").unwrap();
        execute(&migration, None).unwrap();
        let before = (
            std::fs::read_to_string(&config).unwrap(),
            std::fs::read_to_string(&migration.profile_path).unwrap(),
            std::fs::read_to_string(&settings).unwrap(),
        );
        assert!(!legacy_layout(&config, &machine).unwrap());
        assert!(plan(&config, &machine, &settings, "work").is_err());
        assert_eq!(
            before,
            (
                std::fs::read_to_string(&config).unwrap(),
                std::fs::read_to_string(&migration.profile_path).unwrap(),
                std::fs::read_to_string(&settings).unwrap(),
            )
        );
    }

    #[test]
    fn migration_bakes_legacy_overrides_into_the_profile() {
        let (tmp, config, machine, settings) = fixture();
        let overridden_source = tmp.path().join("overridden-source");
        std::fs::write(
            &machine,
            format!(
                "[directory_overrides.source]\npath = \"{}\"\n",
                overridden_source.display()
            ),
        )
        .unwrap();

        let plan = plan(&config, &machine, &settings, "work").unwrap();

        assert!(
            plan.profile_toml
                .contains(&format!("path = \"{}\"", overridden_source.display()))
        );
        assert!(!plan.profile_toml.contains("directory_overrides"));
    }

    #[test]
    fn migration_refusals_do_not_convert_any_files() {
        let (tmp, config, machine, settings) = fixture();
        let legacy_config = std::fs::read_to_string(&config).unwrap();
        let legacy_machine = std::fs::read_to_string(&machine).unwrap();
        let existing_profile = tmp.path().join("machines/work.toml");
        std::fs::create_dir_all(existing_profile.parent().unwrap()).unwrap();
        std::fs::write(&existing_profile, "").unwrap();

        assert!(plan(&config, &machine, &settings, "work").is_err());
        assert_eq!(legacy_config, std::fs::read_to_string(&config).unwrap());
        assert_eq!(legacy_machine, std::fs::read_to_string(&machine).unwrap());
        assert!(!settings.exists());

        std::fs::remove_file(existing_profile).unwrap();
        std::fs::write(&config, "not valid TOML = [").unwrap();
        let malformed = std::fs::read_to_string(&config).unwrap();

        assert!(plan(&config, &machine, &settings, "other").is_err());
        assert_eq!(malformed, std::fs::read_to_string(&config).unwrap());
        assert_eq!(legacy_machine, std::fs::read_to_string(&machine).unwrap());
        assert!(!settings.exists());
    }
}
