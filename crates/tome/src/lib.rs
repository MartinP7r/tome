//! Tome — sync AI coding skills across tools.
//!
//! This crate provides both a CLI binary (`tome`) and a library for managing
//! AI coding skills across multiple tools.
//!
//! # Core pipeline
//!
//! The `sync` function drives the main workflow:
//!
//! 1. **Discover** — scan configured sources for `*/SKILL.md` directories
//! 2. **Consolidate** — copy or symlink discovered skills into the library (managed skills are symlinked; local skills are copied)
//! 3. **Distribute** — push library skills to target tools via symlinks
//! 4. **Cleanup** — remove stale entries no longer in any source
//!
//! # Public API
//!
//! - [`config`] — TOML configuration loading and validation
//! - [`cli`] — command-line argument parsing (clap)
//! - [`run()`] — entry point that dispatches CLI commands

pub(crate) mod browse;
pub(crate) mod cleanup;
pub mod cli;
pub mod config;
pub(crate) mod discover;
pub(crate) mod distribute;
pub(crate) mod doctor;
pub(crate) mod library;
pub(crate) mod lockfile;
pub(crate) mod machine;
pub(crate) mod manifest;
pub(crate) mod paths;
pub(crate) mod status;
pub(crate) mod update;
pub(crate) mod wizard;

use std::collections::HashSet;
use std::io::IsTerminal;
use std::path::Path;
use std::process::Command as GitCommand;

use anyhow::{Context, Result};
use console::style;
use indicatif::{ProgressBar, ProgressStyle};

use cleanup::CleanupResult;
use cli::{Cli, Command};
use config::Config;
use distribute::DistributeResult;
use library::ConsolidateResult;
pub use paths::TomePaths;

/// Summary of a complete sync operation.
pub struct SyncReport {
    pub consolidate: ConsolidateResult,
    pub distributions: Vec<DistributeResult>,
    pub cleanup: CleanupResult,
    pub removed_from_targets: usize,
    pub warnings: Vec<String>,
}

/// Create a spinner with a consistent style.
fn spinner(msg: &str) -> ProgressBar {
    let sp = ProgressBar::new_spinner();
    sp.set_style(
        ProgressStyle::default_spinner()
            .template("{spinner:.cyan} {msg}")
            .expect("valid template"),
    );
    sp.set_message(msg.to_string());
    sp.enable_steady_tick(std::time::Duration::from_millis(80));
    sp
}

/// Resolve the machine preferences path from an optional override,
/// falling back to the default `~/.config/tome/machine.toml`.
fn resolve_machine_path(machine_override: Option<&Path>) -> Result<std::path::PathBuf> {
    match machine_override {
        Some(p) => Ok(p.to_path_buf()),
        None => machine::default_machine_path(),
    }
}

/// Derive the tome home directory from the config file path.
///
/// If an explicit `--config` path is given, tome home is its parent directory.
/// Otherwise, use the default `~/.tome/`.
fn resolve_tome_home(cli_config: Option<&Path>) -> Result<std::path::PathBuf> {
    match cli_config {
        Some(p) => {
            anyhow::ensure!(
                p.is_absolute(),
                "config path '{}' must be an absolute path",
                p.display()
            );
            let parent = p.parent().context("config path has no parent directory")?;
            Ok(parent.to_path_buf())
        }
        None => config::default_tome_home(),
    }
}

/// Run the CLI with parsed arguments.
pub fn run(cli: Cli) -> Result<()> {
    if matches!(cli.command, Command::Init) {
        if let Err(e) = Config::load_or_default(cli.config.as_deref()) {
            eprintln!(
                "warning: existing config is malformed ({}), the wizard will create a new one",
                e
            );
        }
        let tome_home = resolve_tome_home(cli.config.as_deref())?;
        let config = wizard::run(cli.dry_run)?;
        config.validate()?;
        if !cli.dry_run {
            let paths = TomePaths::new(tome_home, config.library_dir.clone())?;
            sync(
                &config,
                &paths,
                cli.dry_run,
                false,
                cli.verbose,
                cli.quiet,
                cli.machine.as_deref(),
            )?;
        }
        return Ok(());
    }

    let config = Config::load_or_default(cli.config.as_deref())?;
    config.validate()?;
    let tome_home = resolve_tome_home(cli.config.as_deref())?;
    let paths = TomePaths::new(tome_home, config.library_dir.clone())?;

    match cli.command {
        Command::Init => unreachable!(),
        Command::Sync { force } => sync(
            &config,
            &paths,
            cli.dry_run,
            force,
            cli.verbose,
            cli.quiet,
            cli.machine.as_deref(),
        )?,
        Command::Update => update_cmd(
            &config,
            &paths,
            cli.dry_run,
            cli.verbose,
            cli.quiet,
            cli.machine.as_deref(),
        )?,
        Command::Status => status::show(&config, &paths)?,
        Command::Doctor => doctor::diagnose(&config, &paths, cli.dry_run)?,
        Command::Browse => {
            let mut warnings = Vec::new();
            let skills = discover::discover_all(&config, &mut warnings)?;
            if !cli.quiet {
                for w in &warnings {
                    eprintln!("warning: {}", w);
                }
            }
            if skills.is_empty() {
                println!("No skills found. Run `tome init` to configure sources.");
                return Ok(());
            }
            browse::browse(skills)?;
        }
        Command::List { json } => list(&config, cli.quiet, json)?,
        Command::Config { path } => show_config(&config, path)?,
    }

    Ok(())
}

/// Warn about `disabled_targets` entries in machine.toml that don't match any
/// configured target name. Helps catch typos and stale entries.
fn warn_unknown_disabled_targets(machine_prefs: &machine::MachinePrefs, config: &Config) {
    for name in &machine_prefs.disabled_targets {
        if !config.targets.contains_key(name.as_str()) {
            eprintln!(
                "warning: disabled target '{}' in machine.toml does not match any configured target",
                name
            );
        }
    }
}

/// The core sync pipeline: discover → consolidate → distribute → cleanup.
fn sync(
    config: &Config,
    paths: &TomePaths,
    dry_run: bool,
    force: bool,
    verbose: bool,
    quiet: bool,
    machine_override: Option<&Path>,
) -> Result<()> {
    if dry_run && !quiet {
        eprintln!(
            "{}",
            style("[dry-run] No changes will be made").yellow().bold()
        );
    }

    let show_progress = !quiet && !verbose;

    // 1. Discover
    let sp = show_progress.then(|| spinner("Discovering skills..."));
    if verbose {
        eprintln!("{}", style("Discovering skills...").dim());
    }
    let mut warnings = Vec::new();
    let skills = discover::discover_all(config, &mut warnings)?;
    if let Some(sp) = sp {
        sp.finish_and_clear();
    }

    if !quiet {
        for w in &warnings {
            eprintln!("warning: {}", w);
        }
    }

    if skills.is_empty() {
        if !quiet {
            println!("No skills found. Run `tome init` to configure sources.");
        }
        return Ok(());
    }

    if verbose {
        eprintln!("  Found {} skills", skills.len());
    }

    // 2. Consolidate into library (copy)
    let sp = show_progress.then(|| spinner("Consolidating to library..."));
    if verbose {
        eprintln!("{}", style("Consolidating to library...").dim());
    }
    let (consolidate_result, mut manifest) = library::consolidate(&skills, paths, dry_run, force)?;
    if let Some(sp) = sp {
        sp.finish_and_clear();
    }

    let discovered_names: HashSet<String> =
        skills.iter().map(|s| s.name.as_str().to_string()).collect();

    // Load per-machine preferences (disabled skills and targets)
    let machine_path = resolve_machine_path(machine_override)?;
    let machine_prefs = machine::load(&machine_path)?;

    // Warn about disabled_targets that don't match any configured target
    if !quiet {
        warn_unknown_disabled_targets(&machine_prefs, config);
    }

    // 3. Distribute to targets
    let mut distribute_results = Vec::new();
    for (name, target) in config.targets.iter() {
        if machine_prefs.is_target_disabled(name.as_str()) {
            if verbose {
                eprintln!(
                    "{}",
                    style(format!(
                        "Skipping target '{}' (disabled in machine preferences)",
                        name
                    ))
                    .dim()
                );
            }
            continue;
        }
        let sp = show_progress.then(|| spinner(&format!("Distributing to {}...", name)));
        if verbose {
            eprintln!("{}", style(format!("Distributing to {}...", name)).dim());
        }
        let result = distribute::distribute_to_target(
            paths.library_dir(),
            name.as_str(),
            target,
            &manifest,
            &machine_prefs,
            dry_run,
            force,
        )?;
        distribute_results.push(result);
        if let Some(sp) = sp {
            sp.finish_and_clear();
        }
    }

    // 4. Cleanup stale entries
    let sp = show_progress.then(|| spinner("Cleaning up stale entries..."));
    if verbose {
        eprintln!("{}", style("Cleaning up stale entries...").dim());
    }
    let cleanup_result = cleanup::cleanup_library(
        paths.library_dir(),
        &discovered_names,
        &mut manifest,
        dry_run,
        quiet,
    )?;

    let mut removed_from_targets = 0usize;
    for (_name, target) in config.targets.iter() {
        let skills_dir = target.skills_dir();
        removed_from_targets += cleanup::cleanup_target(skills_dir, paths.library_dir(), dry_run)?;
        // Also clean up symlinks for disabled skills
        removed_from_targets +=
            cleanup_disabled_from_target(skills_dir, paths.library_dir(), &machine_prefs, dry_run)?;
    }
    // Save manifest after cleanup (may have removed entries)
    if !dry_run && paths.tome_home().is_dir() {
        manifest::save(&manifest, paths.tome_home())?;
    }

    // Generate .gitignore after cleanup so stale entries are excluded
    if !dry_run && paths.library_dir().is_dir() {
        library::generate_gitignore(paths.library_dir(), &manifest)?;
    }

    // Generate lockfile for reproducibility
    if !dry_run && paths.tome_home().is_dir() {
        let lf = lockfile::generate(&manifest, &skills);
        lockfile::save(&lf, paths.tome_home())?;
    }

    if let Some(sp) = sp {
        sp.finish_and_clear();
    }

    let report = SyncReport {
        consolidate: consolidate_result,
        distributions: distribute_results,
        cleanup: cleanup_result,
        removed_from_targets,
        warnings,
    };

    if !quiet {
        render_sync_report(&report);
    }

    // Offer git commit if the library dir is a git repo with changes
    if !dry_run && !quiet {
        offer_git_commit(
            paths.library_dir(),
            &manifest,
            report.consolidate.created,
            report.consolidate.updated,
            report.cleanup.removed_from_library,
        )?;
    }

    Ok(())
}

/// The update command: diff-then-distribute with interactive triage.
fn update_cmd(
    config: &Config,
    paths: &TomePaths,
    dry_run: bool,
    verbose: bool,
    quiet: bool,
    machine_override: Option<&Path>,
) -> Result<()> {
    if dry_run && !quiet {
        eprintln!(
            "{}",
            style("[dry-run] No changes will be made").yellow().bold()
        );
    }

    let show_progress = !quiet && !verbose;

    // Load per-machine preferences
    let machine_path = resolve_machine_path(machine_override)?;
    let mut machine_prefs = machine::load(&machine_path)?;

    // Warn about disabled_targets that don't match any configured target
    if !quiet {
        warn_unknown_disabled_targets(&machine_prefs, config);
    }

    // 1. Load existing lockfile (may be committed by another machine)
    let old_lockfile = lockfile::load(paths.tome_home())?;

    // 2. Discover
    let sp = show_progress.then(|| spinner("Discovering skills..."));
    if verbose {
        eprintln!("{}", style("Discovering skills...").dim());
    }
    let mut warnings = Vec::new();
    let skills = discover::discover_all(config, &mut warnings)?;
    if let Some(sp) = sp {
        sp.finish_and_clear();
    }
    if !quiet {
        for w in &warnings {
            eprintln!("warning: {}", w);
        }
    }

    if skills.is_empty() {
        if !quiet {
            println!("No skills found. Run `tome init` to configure sources.");
        }
        return Ok(());
    }

    // 3. Consolidate into library
    let sp = show_progress.then(|| spinner("Consolidating to library..."));
    if verbose {
        eprintln!("{}", style("Consolidating to library...").dim());
    }
    let (consolidate_result, mut manifest) = library::consolidate(&skills, paths, dry_run, false)?;
    if let Some(sp) = sp {
        sp.finish_and_clear();
    }

    // 4. Generate new lockfile and diff against old
    let new_lockfile = lockfile::generate(&manifest, &skills);
    if let Some(ref old) = old_lockfile {
        let d = update::diff(old, &new_lockfile);
        if !d.is_empty() {
            if !quiet {
                println!("{}", style("Library changes detected:").bold());
            }
            let newly_disabled = update::present_changes(&d, &mut machine_prefs, quiet)?;
            if !newly_disabled.is_empty() && !dry_run {
                machine::save(&machine_prefs, &machine_path)?;
                if !quiet {
                    println!(
                        "  {} skill(s) disabled in {}",
                        newly_disabled.len(),
                        machine_path.display()
                    );
                }
            }
        } else if !quiet {
            println!("{}", style("No library changes since last sync.").dim());
        }
    } else if !quiet {
        println!(
            "{}",
            style("No previous lockfile found — performing initial sync.").dim()
        );
    }

    let discovered_names: HashSet<String> =
        skills.iter().map(|s| s.name.as_str().to_string()).collect();

    // 5. Distribute (respects machine_prefs including just-disabled skills)
    let mut distribute_results = Vec::new();
    for (name, target) in config.targets.iter() {
        if machine_prefs.is_target_disabled(name.as_str()) {
            if verbose {
                eprintln!(
                    "{}",
                    style(format!(
                        "Skipping target '{}' (disabled in machine preferences)",
                        name
                    ))
                    .dim()
                );
            }
            continue;
        }
        let sp = show_progress.then(|| spinner(&format!("Distributing to {}...", name)));
        if verbose {
            eprintln!("{}", style(format!("Distributing to {}...", name)).dim());
        }
        let result = distribute::distribute_to_target(
            paths.library_dir(),
            name.as_str(),
            target,
            &manifest,
            &machine_prefs,
            dry_run,
            false,
        )?;
        distribute_results.push(result);
        if let Some(sp) = sp {
            sp.finish_and_clear();
        }
    }

    // 6. Cleanup stale entries + disabled skill symlinks
    let sp = show_progress.then(|| spinner("Cleaning up stale entries..."));
    if verbose {
        eprintln!("{}", style("Cleaning up stale entries...").dim());
    }
    let cleanup_result = cleanup::cleanup_library(
        paths.library_dir(),
        &discovered_names,
        &mut manifest,
        dry_run,
        quiet,
    )?;

    let mut removed_from_targets = 0usize;
    for (_name, target) in config.targets.iter() {
        let skills_dir = target.skills_dir();
        removed_from_targets += cleanup::cleanup_target(skills_dir, paths.library_dir(), dry_run)?;
        // Also clean up symlinks for disabled skills
        removed_from_targets +=
            cleanup_disabled_from_target(skills_dir, paths.library_dir(), &machine_prefs, dry_run)?;
    }

    if let Some(sp) = sp {
        sp.finish_and_clear();
    }

    // 7. Save lockfile + manifest
    if !dry_run && paths.tome_home().is_dir() {
        manifest::save(&manifest, paths.tome_home())?;
        if paths.library_dir().is_dir() {
            library::generate_gitignore(paths.library_dir(), &manifest)?;
        }
        lockfile::save(&new_lockfile, paths.tome_home())?;
    }

    let report = SyncReport {
        consolidate: consolidate_result,
        distributions: distribute_results,
        cleanup: cleanup_result,
        removed_from_targets,
        warnings,
    };

    if !quiet {
        render_sync_report(&report);
    }

    // Offer git commit if the library dir is a git repo with changes
    if !dry_run && !quiet {
        offer_git_commit(
            paths.library_dir(),
            &manifest,
            report.consolidate.created,
            report.consolidate.updated,
            report.cleanup.removed_from_library,
        )?;
    }

    Ok(())
}

/// Remove symlinks from a target directory that point to disabled skills.
///
/// Unlike `cleanup::cleanup_target` (which only removes *broken* symlinks),
/// this removes symlinks even if their target still exists on disk — because
/// the skill has been disabled in machine preferences.
///
/// Only removes symlinks that point into the library directory, matching the
/// origin check in `cleanup::cleanup_target`.
fn cleanup_disabled_from_target(
    target_dir: &Path,
    library_dir: &Path,
    machine_prefs: &machine::MachinePrefs,
    dry_run: bool,
) -> Result<usize> {
    if !target_dir.is_dir() {
        return Ok(0);
    }

    let canonical_library = std::fs::canonicalize(library_dir).unwrap_or_else(|e| {
        eprintln!(
            "warning: could not canonicalize library path {}: {}",
            library_dir.display(),
            e
        );
        library_dir.to_path_buf()
    });

    let mut removed = 0;
    let entries = std::fs::read_dir(target_dir)
        .with_context(|| format!("failed to read target dir {}", target_dir.display()))?;

    for entry in entries {
        let entry =
            entry.with_context(|| format!("failed to read entry in {}", target_dir.display()))?;
        let path = entry.path();
        if path.is_symlink() {
            let name = entry.file_name();
            if machine_prefs.is_disabled(&name.to_string_lossy()) {
                // Only remove if symlink points into the tome library
                let raw_target = std::fs::read_link(&path)
                    .with_context(|| format!("failed to read symlink {}", path.display()))?;
                let target = paths::resolve_symlink_target(&path, &raw_target);
                let points_into_library =
                    target.starts_with(library_dir) || target.starts_with(&canonical_library);
                if points_into_library {
                    if !dry_run {
                        std::fs::remove_file(&path).with_context(|| {
                            format!("failed to remove disabled symlink {}", path.display())
                        })?;
                    }
                    removed += 1;
                }
            }
        }
    }

    Ok(removed)
}

fn render_sync_report(report: &SyncReport) {
    println!("{}", style("Sync complete").green().bold());
    println!(
        "  Library: {} created, {} unchanged, {} updated{}",
        style(report.consolidate.created).cyan(),
        report.consolidate.unchanged,
        report.consolidate.updated,
        skipped_note(report.consolidate.skipped)
    );

    for dr in &report.distributions {
        println!(
            "  {}: {} linked, {} unchanged{}{}",
            style(&dr.target_name).bold(),
            style(dr.changed).cyan(),
            dr.unchanged,
            skipped_note(dr.skipped),
            disabled_note(dr.disabled)
        );
    }

    if report.cleanup.removed_from_library > 0 {
        println!(
            "  Cleaned {} stale entry/entries",
            style(report.cleanup.removed_from_library).yellow()
        );
    }

    if report.removed_from_targets > 0 {
        println!(
            "  Cleaned {} stale target link(s)",
            style(report.removed_from_targets).yellow()
        );
    }
}

/// List all discovered skills.
fn list(config: &Config, quiet: bool, json: bool) -> Result<()> {
    let mut warnings = Vec::new();
    let skills = discover::discover_all(config, &mut warnings)?;
    if !quiet {
        for w in &warnings {
            eprintln!("warning: {}", w);
        }
    }

    if json {
        let rows: Vec<serde_json::Value> = skills
            .iter()
            .map(|s| {
                serde_json::json!({
                    "name": s.name,
                    "source": s.source_name,
                    "path": s.path,
                })
            })
            .collect();
        println!("{}", serde_json::to_string_pretty(&rows)?);
        return Ok(());
    }

    if quiet {
        return Ok(());
    }

    if skills.is_empty() {
        println!("No skills found. Run `tome init` to configure sources.");
        return Ok(());
    }

    use tabled::settings::{Modify, Style, object::Rows};

    let mut rows: Vec<[String; 3]> = Vec::with_capacity(skills.len() + 1);
    rows.push([
        "SKILL".to_string(),
        "SOURCE".to_string(),
        "PATH".to_string(),
    ]);
    for s in &skills {
        rows.push([
            s.name.to_string(),
            s.source_name.clone(),
            s.path.display().to_string(),
        ]);
    }

    let table = tabled::Table::from_iter(rows)
        .with(Style::blank())
        .with(
            Modify::new(Rows::first()).with(tabled::settings::Format::content(|s| {
                style(s).bold().to_string()
            })),
        )
        .to_string();

    println!("{table}");
    println!();
    println!("{} skill(s) total", skills.len());

    Ok(())
}

/// Format a "skipped (path conflict)" suffix, or an empty string if count is zero.
fn skipped_note(count: usize) -> String {
    if count > 0 {
        format!(", {} skipped (path conflict)", style(count).yellow())
    } else {
        String::new()
    }
}

fn disabled_note(count: usize) -> String {
    if count == 0 {
        String::new()
    } else {
        format!(", {} disabled (machine prefs)", style(count).dim())
    }
}

/// If the library directory is a git repo with uncommitted changes, prompt the user to commit.
fn offer_git_commit(
    library_dir: &Path,
    manifest: &manifest::Manifest,
    created: usize,
    updated: usize,
    removed: usize,
) -> Result<()> {
    if !library_dir.join(".git").exists() || !std::io::stdin().is_terminal() {
        return Ok(());
    }

    let output = match GitCommand::new("git")
        .args(["status", "--porcelain"])
        .current_dir(library_dir)
        .output()
    {
        Ok(o) => o,
        Err(e) => {
            eprintln!("warning: could not run git status: {e}");
            return Ok(());
        }
    };

    if !output.status.success() {
        eprintln!(
            "warning: git status returned non-zero exit code {:?}",
            output.status.code()
        );
        return Ok(());
    }
    if output.stdout.is_empty() {
        return Ok(());
    }

    let msg = sync_commit_message(created, updated, removed);

    let confirm = dialoguer::Confirm::new()
        .with_prompt(format!("Commit library changes? ({})", msg))
        .default(true)
        .interact_opt()?;

    if confirm != Some(true) {
        return Ok(());
    }

    // Stage specific paths instead of `git add .` to avoid accidentally
    // committing unrelated files. Manifest and lockfile live at tome home
    // (outside the library), so only stage .gitignore and skill dirs.
    let mut paths: Vec<String> = vec![".gitignore".into()];
    for (name, entry) in manifest.iter() {
        if !entry.managed {
            paths.push(name.as_str().to_string());
        }
    }

    let add_status = GitCommand::new("git")
        .arg("add")
        .arg("--")
        .args(&paths)
        .current_dir(library_dir)
        .status()?;
    if !add_status.success() {
        eprintln!(
            "warning: git add failed (exit code {:?})",
            add_status.code()
        );
        return Ok(());
    }

    // Also stage deletions for the same set of paths
    let stage_deleted = GitCommand::new("git")
        .args(["add", "--update", "--"])
        .args(&paths)
        .current_dir(library_dir)
        .status()?;
    if !stage_deleted.success() {
        eprintln!(
            "warning: git add --update failed (exit code {:?})",
            stage_deleted.code()
        );
        return Ok(());
    }

    let commit_status = GitCommand::new("git")
        .args(["commit", "-m", &msg])
        .current_dir(library_dir)
        .status()?;
    if !commit_status.success() {
        eprintln!(
            "warning: git commit failed (exit code {:?})",
            commit_status.code()
        );
        return Ok(());
    }

    Ok(())
}

/// Build a commit message summarizing sync changes.
fn sync_commit_message(created: usize, updated: usize, removed: usize) -> String {
    let mut parts = Vec::new();
    if created > 0 {
        parts.push(format!("{created} created"));
    }
    if updated > 0 {
        parts.push(format!("{updated} updated"));
    }
    if removed > 0 {
        parts.push(format!("{removed} removed"));
    }
    if parts.is_empty() {
        return "tome sync".to_string();
    }
    format!("tome sync: {}", parts.join(", "))
}

/// Show or print config information.
fn show_config(config: &Config, path_only: bool) -> Result<()> {
    if path_only {
        println!("{}", config::default_config_path()?.display());
    } else {
        let toml_str = toml::to_string_pretty(config)?;
        println!("{}", toml_str);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::discover::SkillName;
    use std::os::unix::fs as unix_fs;
    use tempfile::TempDir;

    #[test]
    fn commit_message_all_changes() {
        assert_eq!(
            sync_commit_message(3, 1, 2),
            "tome sync: 3 created, 1 updated, 2 removed"
        );
    }

    #[test]
    fn commit_message_created_only() {
        assert_eq!(sync_commit_message(5, 0, 0), "tome sync: 5 created");
    }

    #[test]
    fn commit_message_no_changes() {
        assert_eq!(sync_commit_message(0, 0, 0), "tome sync");
    }

    // -- cleanup_disabled_from_target tests --

    #[test]
    fn cleanup_disabled_removes_library_symlink() {
        let library = TempDir::new().unwrap();
        let target = TempDir::new().unwrap();

        // Create a skill dir in the library and symlink it in the target
        let skill_dir = library.path().join("disabled-skill");
        std::fs::create_dir_all(&skill_dir).unwrap();
        unix_fs::symlink(&skill_dir, target.path().join("disabled-skill")).unwrap();

        let mut prefs = machine::MachinePrefs::default();
        prefs.disable(SkillName::new("disabled-skill").unwrap());

        let removed =
            cleanup_disabled_from_target(target.path(), library.path(), &prefs, false).unwrap();
        assert_eq!(removed, 1);
        assert!(!target.path().join("disabled-skill").exists());
    }

    #[test]
    fn cleanup_disabled_preserves_external_symlink() {
        let library = TempDir::new().unwrap();
        let target = TempDir::new().unwrap();
        let external = TempDir::new().unwrap();

        // Symlink in target with a disabled name but pointing outside the library
        let ext_dir = external.path().join("disabled-skill");
        std::fs::create_dir_all(&ext_dir).unwrap();
        unix_fs::symlink(&ext_dir, target.path().join("disabled-skill")).unwrap();

        let mut prefs = machine::MachinePrefs::default();
        prefs.disable(SkillName::new("disabled-skill").unwrap());

        let removed =
            cleanup_disabled_from_target(target.path(), library.path(), &prefs, false).unwrap();
        assert_eq!(
            removed, 0,
            "should not remove symlink pointing outside library"
        );
        assert!(target.path().join("disabled-skill").is_symlink());
    }

    #[test]
    fn cleanup_disabled_skips_non_symlink() {
        let library = TempDir::new().unwrap();
        let target = TempDir::new().unwrap();

        // Regular directory (not a symlink) with a disabled skill name
        std::fs::create_dir_all(target.path().join("disabled-skill")).unwrap();

        let mut prefs = machine::MachinePrefs::default();
        prefs.disable(SkillName::new("disabled-skill").unwrap());

        let removed =
            cleanup_disabled_from_target(target.path(), library.path(), &prefs, false).unwrap();
        assert_eq!(removed, 0);
        assert!(target.path().join("disabled-skill").is_dir());
    }

    #[test]
    fn cleanup_disabled_nonexistent_dir_returns_zero() {
        let prefs = machine::MachinePrefs::default();
        let removed = cleanup_disabled_from_target(
            std::path::Path::new("/nonexistent/target"),
            std::path::Path::new("/nonexistent/library"),
            &prefs,
            false,
        )
        .unwrap();
        assert_eq!(removed, 0);
    }

    #[test]
    fn cleanup_disabled_dry_run_preserves_symlink() {
        let library = TempDir::new().unwrap();
        let target = TempDir::new().unwrap();

        let skill_dir = library.path().join("disabled-skill");
        std::fs::create_dir_all(&skill_dir).unwrap();
        unix_fs::symlink(&skill_dir, target.path().join("disabled-skill")).unwrap();

        let mut prefs = machine::MachinePrefs::default();
        prefs.disable(SkillName::new("disabled-skill").unwrap());

        let removed =
            cleanup_disabled_from_target(target.path(), library.path(), &prefs, true).unwrap();
        assert_eq!(removed, 1, "should count the would-be removal");
        assert!(
            target.path().join("disabled-skill").is_symlink(),
            "dry-run should not actually remove"
        );
    }

    #[test]
    fn resolve_tome_home_absolute_path_returns_parent() {
        let result = resolve_tome_home(Some(Path::new("/home/user/.tome/tome.toml"))).unwrap();
        assert_eq!(result, Path::new("/home/user/.tome"));
    }

    #[test]
    fn resolve_tome_home_bare_filename_returns_error() {
        let result = resolve_tome_home(Some(Path::new("tome.toml")));
        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(
            err_msg.contains("must be an absolute path"),
            "unexpected error: {err_msg}"
        );
    }

    #[test]
    fn resolve_tome_home_relative_path_returns_error() {
        for path in &["./tome.toml", "../tome.toml", "subdir/tome.toml"] {
            let result = resolve_tome_home(Some(Path::new(path)));
            assert!(result.is_err(), "expected error for relative path: {path}");
            let err_msg = result.unwrap_err().to_string();
            assert!(
                err_msg.contains("must be an absolute path"),
                "unexpected error for '{path}': {err_msg}"
            );
        }
    }

    #[test]
    fn resolve_tome_home_none_returns_default() {
        let result = resolve_tome_home(None).unwrap();
        let expected = config::default_tome_home().unwrap();
        assert_eq!(result, expected);
    }
}
