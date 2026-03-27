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
//! - [`TomePaths`] — bundled home/library paths
//! - [`SyncReport`] — sync operation results

pub(crate) mod backup;
pub(crate) mod browse;
pub(crate) mod cleanup;
pub mod cli;
pub mod config;
pub(crate) mod discover;
pub(crate) mod distribute;
pub(crate) mod doctor;
pub(crate) mod eject;
pub(crate) mod library;
pub(crate) mod lint;
pub(crate) mod lockfile;
pub(crate) mod machine;
pub(crate) mod manifest;
pub(crate) mod paths;
pub(crate) mod relocate;
pub(crate) mod skill;
pub(crate) mod status;
pub(crate) mod update;
pub(crate) mod validation;
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
    if matches!(cli.command, Command::Version) {
        println!("tome {}", env!("CARGO_PKG_VERSION"));
        return Ok(());
    }

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
                SyncOptions {
                    dry_run: cli.dry_run,
                    force: false,
                    no_triage: true, // skip on initial sync after init
                    verbose: cli.verbose,
                    quiet: cli.quiet,
                    machine_override: cli.machine.as_deref(),
                },
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
        Command::Sync { force, no_triage } => sync(
            &config,
            &paths,
            SyncOptions {
                dry_run: cli.dry_run,
                force,
                no_triage,
                verbose: cli.verbose,
                quiet: cli.quiet,
                machine_override: cli.machine.as_deref(),
            },
        )?,
        Command::Status => status::show(&config, &paths)?,
        Command::Doctor => doctor::diagnose(&config, &paths, cli.dry_run)?,
        Command::Lint { path, format } => {
            let report = match path {
                Some(p) => {
                    let dir_name = p.file_name().and_then(|n| n.to_str()).unwrap_or("unknown");
                    let issues = lint::lint_skill(dir_name, &p);
                    lint::LintReport {
                        results: vec![(dir_name.to_string(), issues)],
                        skills_checked: 1,
                    }
                }
                None => lint::lint_library(paths.library_dir()),
            };
            match format {
                cli::LintFormat::Text => lint::render_text(&report),
                cli::LintFormat::Json => lint::render_json(&report),
            }
            if report.has_errors() {
                std::process::exit(1);
            }
        }
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
            let manifest = manifest::load(paths.tome_home())?;
            browse::browse(skills, &manifest)?;
        }
        Command::Eject => {
            let plan = eject::plan(&config, &paths)?;
            eject::render_plan(&plan);

            if plan.total_symlinks == 0 {
                return Ok(());
            }

            if cli.dry_run {
                println!("\n{}", style("Dry run — no changes made.").yellow());
                return Ok(());
            }

            if std::io::stdin().is_terminal() {
                let confirmed = dialoguer::Confirm::new()
                    .with_prompt("Remove these symlinks?")
                    .default(true)
                    .interact()?;
                if !confirmed {
                    println!("Aborted.");
                    return Ok(());
                }
            }

            let removed = eject::execute(&plan, false)?;
            println!(
                "\n{} Removed {} symlink(s). Run {} to re-distribute.",
                style("✓").green(),
                removed,
                style("tome sync").cyan()
            );
        }
        Command::Relocate { new_path } => {
            let config_path = cli
                .config
                .clone()
                .unwrap_or_else(|| paths.tome_home().join("tome.toml"));

            let plan = relocate::plan(&config, &paths, &new_path, &config_path)?;
            relocate::render_plan(&plan);

            if cli.dry_run {
                println!("\n{}", style("Dry run -- no changes made.").yellow());
                return Ok(());
            }

            if std::io::stdin().is_terminal() {
                let confirmed = dialoguer::Confirm::new()
                    .with_prompt("Proceed with relocation?")
                    .default(false)
                    .interact()?;
                if !confirmed {
                    println!("Aborted.");
                    return Ok(());
                }
            } else {
                anyhow::bail!(
                    "tome relocate requires interactive confirmation -- refusing in non-interactive mode"
                );
            }

            relocate::execute(&plan, false)?;

            let new_config = Config::load(&config_path)?;
            relocate::verify(&new_config, &plan.new_library_dir, paths.tome_home())?;
        }
        Command::Version => unreachable!(),
        Command::Completions { shell } => {
            let mut cmd = <cli::Cli as clap::CommandFactory>::command();
            clap_complete::generate(shell, &mut cmd, "tome", &mut std::io::stdout());
        }
        Command::List { json } => list(&config, cli.quiet, json)?,
        Command::Config { path } => show_config(&config, path)?,
        Command::Backup { sub } => match sub {
            cli::BackupCommand::Init => {
                backup::init(paths.tome_home(), cli.dry_run)?;
            }
            cli::BackupCommand::Snapshot { message } => {
                backup::snapshot(paths.tome_home(), message.as_deref(), cli.dry_run)?;
            }
            cli::BackupCommand::List { count } => {
                let entries = backup::list(paths.tome_home(), count)?;
                backup::render_list(&entries);
            }
            cli::BackupCommand::Restore { target, force } => {
                if !force {
                    if std::io::stdin().is_terminal() {
                        let confirmed = dialoguer::Confirm::new()
                            .with_prompt(format!(
                                "Restore to {}? This will overwrite current state",
                                target
                            ))
                            .default(false)
                            .interact()?;
                        if !confirmed {
                            println!("Aborted.");
                            return Ok(());
                        }
                    } else {
                        anyhow::bail!(
                            "tome backup restore requires confirmation — use --force in non-interactive mode"
                        );
                    }
                }
                backup::restore(paths.tome_home(), &target, cli.dry_run)?;
            }
            cli::BackupCommand::Diff { target } => {
                let diff = backup::diff(paths.tome_home(), &target)?;
                if diff.is_empty() {
                    println!("No changes since {}", target);
                } else {
                    println!("{}", diff);
                }
            }
        },
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

/// Options for the sync pipeline.
struct SyncOptions<'a> {
    dry_run: bool,
    force: bool,
    no_triage: bool,
    verbose: bool,
    quiet: bool,
    machine_override: Option<&'a Path>,
}

/// The core sync pipeline: discover → consolidate → distribute → cleanup.
fn sync(config: &Config, paths: &TomePaths, opts: SyncOptions<'_>) -> Result<()> {
    let SyncOptions {
        dry_run,
        force,
        no_triage,
        verbose,
        quiet,
        machine_override,
    } = opts;
    if dry_run && !quiet {
        eprintln!(
            "{}",
            style("[dry-run] No changes will be made").yellow().bold()
        );
    }

    let show_progress = !quiet && !verbose;

    // Pre-sync auto-snapshot if configured
    if !dry_run
        && config.backup.enabled
        && config.backup.auto_snapshot
        && backup::has_repo(paths.tome_home())
    {
        match backup::snapshot(paths.tome_home(), Some("pre-sync auto-snapshot"), false) {
            Ok(true) => {
                if !quiet {
                    eprintln!("info: pre-sync snapshot created");
                }
            }
            Ok(false) => {} // nothing to snapshot
            Err(e) => eprintln!("warning: auto-snapshot failed: {e}"),
        }
    }

    // Load per-machine preferences (disabled skills and targets)
    let machine_path = resolve_machine_path(machine_override)?;
    let mut machine_prefs = machine::load(&machine_path)?;

    // Load existing lockfile for diffing
    let old_lockfile = lockfile::load(paths.tome_home())?;

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

    // 3. Diff lockfile and triage changes
    let new_lockfile = lockfile::generate(&manifest, &skills);
    if !no_triage && !quiet {
        if let Some(ref old) = old_lockfile {
            let d = update::diff(old, &new_lockfile);
            if !d.is_empty() {
                println!("{}", style("Library changes detected:").bold());
                let newly_disabled = update::present_changes(&d, &mut machine_prefs, quiet)?;
                if !newly_disabled.is_empty() && !dry_run {
                    machine::save(&machine_prefs, &machine_path)?;
                    println!(
                        "  {} skill(s) disabled in {}",
                        newly_disabled.len(),
                        machine_path.display()
                    );
                }
            } else {
                println!("{}", style("No changes since last sync.").dim());
            }
        } else {
            println!(
                "{}",
                style("No previous lockfile — performing initial sync.").dim()
            );
        }
    }

    let discovered_names: HashSet<String> =
        skills.iter().map(|s| s.name.as_str().to_string()).collect();

    // Warn about disabled_targets that don't match any configured target
    if !quiet {
        warn_unknown_disabled_targets(&machine_prefs, config);
    }

    // 4. Distribute to targets
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

    // 5. Cleanup stale entries
    let sp = show_progress.then(|| spinner("Cleaning up stale entries..."));
    if verbose {
        eprintln!("{}", style("Cleaning up stale entries...").dim());
    }
    // Clear the spinner before cleanup_library runs: cleanup may show interactive
    // dialoguer prompts, and a live spinner overwrites them, causing an apparent hang.
    if let Some(sp) = sp {
        sp.finish_and_clear();
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

    // 6. Save manifest, gitignore, and lockfile
    if !dry_run && paths.tome_home().is_dir() {
        manifest::save(&manifest, paths.tome_home())?;
    }
    if !dry_run && paths.library_dir().is_dir() {
        library::generate_gitignore(paths.library_dir(), &manifest)?;
    }
    if !dry_run && paths.tome_home().is_dir() {
        generate_tome_home_gitignore(paths.tome_home())?;
        if let Err(e) = lockfile::save(&new_lockfile, paths.tome_home()) {
            eprintln!("warning: could not save lockfile: {e}");
        }
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

    // Post-sync health check
    if !dry_run && !quiet {
        let doctor_report = doctor::check(config, paths)?;
        if doctor_report.total_issues() > 0 {
            eprintln!(
                "warning: {} issue(s) detected after sync — run `tome doctor` for details",
                doctor_report.total_issues()
            );
        }
    }

    // Offer git commit if tome home is a git repo with changes
    if !dry_run && !quiet {
        offer_git_commit(
            paths.tome_home(),
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
            "warning: could not canonicalize library path {}: {} — symlinks using canonical paths may not be cleaned up",
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
    let mut skills = discover::discover_all(config, &mut warnings)?;
    skills.sort_by(|a, b| a.name.as_str().cmp(b.name.as_str()));
    if !quiet {
        for w in &warnings {
            eprintln!("warning: {}", w);
        }
    }

    if json {
        let rows: Vec<serde_json::Value> = skills
            .iter()
            .map(|s| {
                let mut row = serde_json::json!({
                    "name": s.name,
                    "source": s.source_name,
                    "path": s.path,
                    "managed": s.origin.is_managed(),
                });
                if let Some(p) = s.origin.provenance() {
                    row["registry_id"] = serde_json::json!(p.registry_id);
                    if let Some(v) = &p.version {
                        row["version"] = serde_json::json!(v);
                    }
                    if let Some(sha) = &p.git_commit_sha {
                        row["git_commit_sha"] = serde_json::json!(sha);
                    }
                }
                row
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

    let mut rows: Vec<[String; 4]> = Vec::with_capacity(skills.len() + 1);
    rows.push([
        "SKILL".to_string(),
        "SOURCE".to_string(),
        "VERSION".to_string(),
        "PATH".to_string(),
    ]);
    for s in &skills {
        let version = s
            .origin
            .provenance()
            .and_then(|p| p.version.as_deref())
            .unwrap_or("")
            .to_string();
        rows.push([
            s.name.to_string(),
            s.source_name.clone(),
            version,
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

/// Generate a top-level `.gitignore` at `~/.tome/` to exclude internal files.
///
/// The manifest is internal bookkeeping and should not be version-controlled.
/// Everything else (skills/, tome.toml, tome.lock) is tracked.
fn generate_tome_home_gitignore(tome_home: &Path) -> Result<()> {
    let content = "# Auto-generated by tome — do not edit\n\
                   # Internal manifest (recreated by tome sync)\n\
                   .tome-manifest.json\n";
    let gitignore_path = tome_home.join(".gitignore");

    // Only write if content would change
    if gitignore_path.exists() {
        let existing = std::fs::read_to_string(&gitignore_path)
            .with_context(|| format!("failed to read {}", gitignore_path.display()))?;
        if existing == content {
            return Ok(());
        }
    }

    std::fs::write(&gitignore_path, content)
        .with_context(|| format!("failed to write {}", gitignore_path.display()))?;

    Ok(())
}

/// If tome home is a git repo with uncommitted changes, prompt the user to commit.
fn offer_git_commit(
    tome_home: &Path,
    created: usize,
    updated: usize,
    removed: usize,
) -> Result<()> {
    if !tome_home.join(".git").exists() || !std::io::stdin().is_terminal() {
        return Ok(());
    }

    let output = match GitCommand::new("git")
        .args(["status", "--porcelain"])
        .current_dir(tome_home)
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
        .with_prompt(format!("Commit changes? ({})", msg))
        .default(true)
        .interact_opt()?;

    if confirm != Some(true) {
        return Ok(());
    }

    // Stage all tracked files — .gitignore handles exclusions.
    // The repo is at tome_home (~/.tome/) and covers skills, config, and lockfile.
    let add_output = GitCommand::new("git")
        .args(["add", "-A"])
        .current_dir(tome_home)
        .output()?;
    if !add_output.status.success() {
        eprintln!(
            "warning: git add failed (exit code {:?})",
            add_output.status.code()
        );
        let stderr = String::from_utf8_lossy(&add_output.stderr);
        if !stderr.trim().is_empty() {
            eprintln!("  git said: {}", stderr.trim());
        }
        return Ok(());
    }

    let commit_output = GitCommand::new("git")
        .args(["commit", "-m", &msg])
        .current_dir(tome_home)
        .output()?;
    if !commit_output.status.success() {
        eprintln!(
            "warning: git commit failed (exit code {:?})",
            commit_output.status.code()
        );
        let stderr = String::from_utf8_lossy(&commit_output.stderr);
        if !stderr.trim().is_empty() {
            eprintln!("  git said: {}", stderr.trim());
        }
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
