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
//! 2. **Consolidate** — copy or symlink discovered skills into the library
//! 3. **Triage** — diff lockfile, surface changes, let user disable new skills
//! 4. **Distribute** — push library skills to target tools via symlinks
//! 5. **Cleanup** — remove stale entries no longer in any source
//! 6. **Save** — persist manifest, lockfile, and `.gitignore`
//!
//! # Public API
//!
//! - [`config`] — TOML configuration loading and validation
//! - [`cli`] — command-line argument parsing (clap)
//! - [`run()`] — entry point that dispatches CLI commands
//! - [`TomePaths`] — bundled home/library paths
//! - [`SyncReport`] — sync operation results

pub(crate) mod add;
pub(crate) mod backup;
pub(crate) mod browse;
pub(crate) mod cleanup;
pub mod cli;
pub mod config;
pub(crate) mod discover;
pub(crate) mod distribute;
pub(crate) mod doctor;
pub(crate) mod eject;
pub(crate) mod git;
pub(crate) mod install;
pub(crate) mod library;
pub(crate) mod lint;
pub(crate) mod lockfile;
pub(crate) mod machine;
pub(crate) mod manifest;
pub(crate) mod paths;
pub(crate) mod reassign;
pub(crate) mod relocate;
pub(crate) mod remove;
pub(crate) mod skill;
pub(crate) mod status;
pub(crate) mod update;
pub(crate) mod validation;
pub(crate) mod wizard;

use std::collections::{BTreeMap, HashSet};
use std::io::IsTerminal;
use std::path::{Path, PathBuf};
use std::process::Command as GitCommand;

use anyhow::{Context, Result};
use console::style;
use indicatif::{ProgressBar, ProgressStyle};

use cleanup::CleanupResult;
use cli::{Cli, Command};
use config::{Config, DirectoryName, DirectoryType};
use distribute::DistributeResult;
use library::ConsolidateResult;
pub use paths::TomePaths;

/// Summary of a complete sync operation.
pub struct SyncReport {
    pub consolidate: ConsolidateResult,
    pub distributions: Vec<DistributeResult>,
    pub cleanup: CleanupResult,
    pub removed_from_targets: usize,
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

/// Derive the tome home directory.
///
/// Resolution order:
/// 1. `--tome-home` CLI flag (highest priority)
/// 2. `--config` CLI flag (tome home = parent directory of config file)
/// 3. `TOME_HOME` env var (checked inside `default_tome_home()`)
/// 4. `~/.tome/` (default)
fn resolve_tome_home(
    cli_tome_home: Option<&Path>,
    cli_config: Option<&Path>,
) -> Result<std::path::PathBuf> {
    if let Some(p) = cli_tome_home {
        let expanded = config::expand_tilde(p)?;
        anyhow::ensure!(
            expanded.is_absolute(),
            "--tome-home path '{}' must be an absolute path",
            p.display()
        );
        return Ok(expanded);
    }
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

/// Derive the effective config file path.
///
/// If `--config` is given, use that directly.
/// If `--tome-home` is given, use smart detection (`.tome/tome.toml` if exists, else `tome.toml`).
/// Otherwise, fall back to `default_config_path()` (which also reads TOME_HOME + smart detection).
fn resolve_config_path(
    cli_tome_home: Option<&Path>,
    cli_config: Option<&Path>,
) -> Result<Option<std::path::PathBuf>> {
    if cli_config.is_some() {
        return Ok(cli_config.map(|p| p.to_path_buf()));
    }
    if let Some(th) = cli_tome_home {
        let expanded = config::expand_tilde(th)?;
        return Ok(Some(
            config::resolve_config_dir(&expanded).join("tome.toml"),
        ));
    }
    Ok(None)
}

/// Run the CLI with parsed arguments.
pub fn run(cli: Cli) -> Result<()> {
    if matches!(cli.command, Command::Version) {
        println!("tome {}", env!("CARGO_PKG_VERSION"));
        return Ok(());
    }

    let effective_config = resolve_config_path(cli.tome_home.as_deref(), cli.config.as_deref())?;

    if matches!(cli.command, Command::Init) {
        if let Err(e) = Config::load_or_default(effective_config.as_deref()) {
            eprintln!(
                "warning: existing config is malformed ({}), the wizard will create a new one",
                e
            );
        }
        // WUX-04: surface the resolved tome_home + its source BEFORE any
        // wizard prompts so the user can Ctrl-C if the wrong path is about
        // to be populated (e.g. a stray `TOME_HOME=/wrong/path` in their
        // shell rc). Printed in both interactive and --no-input modes.
        //
        // `tome_home_source` is intentionally bound here; later plans in
        // this phase will consume it to gate greenfield prompts (WUX-01).
        let (tome_home, tome_home_source) =
            config::resolve_tome_home_with_source(cli.tome_home.as_deref(), cli.config.as_deref())?;
        println!();
        println!(
            "resolved tome_home: {} (from {})",
            style(tome_home.display()).cyan(),
            tome_home_source.label()
        );

        // WUX-03: Detect and handle legacy pre-v0.6 ~/.config/tome/config.toml.
        // The legacy file is silently ignored by v0.6+ (only its `tome_home`
        // key is read); this warns the user and offers cleanup.
        let home = dirs::home_dir().context("could not determine home directory")?;
        let machine_state = wizard::detect_machine_state(&home, &tome_home)?;
        if let wizard::MachineState::Legacy { legacy_path }
        | wizard::MachineState::BrownfieldWithLegacy { legacy_path, .. } = &machine_state
        {
            wizard::handle_legacy_cleanup(legacy_path, cli.no_input)?;
        }

        // WUX-02: Brownfield decision. When a tome.toml already exists at the
        // resolved tome_home, present a summary and a 4-way choice:
        //   UseExisting  → exit wizard, skip post-init sync
        //   Edit         → launch wizard with existing values pre-filled
        //   Reinit       → backup existing file and launch fresh wizard
        //   Cancel       → exit cleanly (exit 0), no changes
        // Non-brownfield states (Greenfield, Legacy-only) skip this block and
        // proceed to the wizard with no prefill.
        let prefill: Option<Config> = match &machine_state {
            wizard::MachineState::Brownfield {
                existing_config_path,
                existing_config,
            }
            | wizard::MachineState::BrownfieldWithLegacy {
                existing_config_path,
                existing_config,
                ..
            } => {
                let action = wizard::brownfield_decision(
                    existing_config_path,
                    existing_config,
                    cli.no_input,
                )?;
                match action {
                    wizard::BrownfieldAction::UseExisting => {
                        println!("  Config unchanged. Run `tome sync` to apply.");
                        return Ok(());
                    }
                    wizard::BrownfieldAction::Cancel => {
                        println!("Wizard cancelled. Existing config left unchanged.");
                        return Ok(());
                    }
                    wizard::BrownfieldAction::Reinit => {
                        let backup = wizard::backup_brownfield_config(existing_config_path)?;
                        println!(
                            "  Backed up existing config to: {}",
                            style(backup.display()).cyan()
                        );
                        None // proceed as greenfield
                    }
                    wizard::BrownfieldAction::Edit => match existing_config {
                        Ok(c) => Some(c.clone()),
                        Err(_) => unreachable!(
                            "brownfield_decision does not offer Edit for unparsable configs"
                        ),
                    },
                }
            }
            _ => None,
        };

        let config = wizard::run(
            cli.dry_run,
            cli.no_input,
            &tome_home,
            tome_home_source,
            prefill.as_ref(),
        )?;
        config.validate()?;
        if !cli.dry_run {
            // Expand `~` in library_dir before passing to TomePaths, which
            // requires absolute paths. The wizard preserves tilde-shaped paths
            // so the on-disk TOML stays portable; here we resolve them for the
            // post-init sync call.
            let mut expanded = config.clone();
            expanded
                .expand_tildes()
                .context("failed to expand ~ in wizard-produced config")?;
            let paths = TomePaths::new(tome_home, expanded.library_dir.clone())?;
            sync(
                &expanded,
                &paths,
                SyncOptions {
                    dry_run: cli.dry_run,
                    force: false,
                    no_triage: true, // skip on initial sync after init
                    no_input: cli.no_input,
                    verbose: cli.verbose,
                    quiet: cli.quiet,
                    machine_override: cli.machine.as_deref(),
                },
            )?;
        }
        return Ok(());
    }

    let config = Config::load_or_default(effective_config.as_deref())?;
    config.validate()?;
    let tome_home = resolve_tome_home(cli.tome_home.as_deref(), cli.config.as_deref())?;
    let paths = TomePaths::new(tome_home, config.library_dir.clone())?;

    match cli.command {
        Command::Init => unreachable!(),
        Command::Add {
            url,
            name,
            branch,
            tag,
            rev,
        } => {
            let mut config = config;
            add::add(
                &mut config,
                add::AddOptions {
                    url: &url,
                    name: name.as_deref(),
                    branch: branch.as_deref(),
                    tag: tag.as_deref(),
                    rev: rev.as_deref(),
                    dry_run: cli.dry_run,
                    config_path: &paths.config_path(),
                },
            )?;
        }
        Command::Sync { force, no_triage } => sync(
            &config,
            &paths,
            SyncOptions {
                dry_run: cli.dry_run,
                force,
                no_triage: no_triage || cli.no_input,
                no_input: cli.no_input,
                verbose: cli.verbose,
                quiet: cli.quiet,
                machine_override: cli.machine.as_deref(),
            },
        )?,
        Command::Status { json } => status::show(&config, &paths, json)?,
        Command::Doctor { json } => {
            doctor::diagnose(&config, &paths, cli.dry_run, cli.no_input, json)?;
        }
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
            let skills = discover::discover_all(&config, &BTreeMap::new(), &mut warnings)?;
            if !cli.quiet {
                for w in &warnings {
                    eprintln!("warning: {}", w);
                }
            }
            if skills.is_empty() {
                println!("No skills found. Run `tome init` to configure sources.");
                return Ok(());
            }
            let manifest = manifest::load(paths.config_dir())?;
            browse::browse(skills, &manifest)?;
        }
        Command::Remove { name, force } => {
            let manifest = manifest::load(paths.config_dir())?;
            let plan = remove::plan(&name, &config, &paths, &manifest)?;
            remove::render_plan(&plan);

            if cli.dry_run {
                println!("\n{}", style("Dry run — no changes made.").yellow());
                return Ok(());
            }

            if !force {
                if !cli.no_input && std::io::stdin().is_terminal() {
                    let confirmed = dialoguer::Confirm::new()
                        .with_prompt(format!("Remove directory '{}'?", name))
                        .default(false)
                        .interact()?;
                    if !confirmed {
                        println!("Aborted.");
                        return Ok(());
                    }
                } else {
                    anyhow::bail!(
                        "tome remove requires confirmation — use --force in non-interactive mode"
                    );
                }
            }

            let mut config = config;
            let mut manifest = manifest;
            let result = remove::execute(&plan, &mut config, &mut manifest, false)?;

            // Save updated config
            config.save(&paths.config_path())?;
            // Save updated manifest
            manifest::save(&manifest, paths.config_dir())?;
            // Regenerate lockfile. Recover git-skill provenance offline from
            // the previous lockfile + on-disk cache so git-type directories
            // are not silently dropped during regen (#461 H1).
            let (resolved_paths, mut regen_warnings) =
                lockfile::resolved_paths_from_lockfile_cache(&config, &paths);
            let skills = discover::discover_all(&config, &resolved_paths, &mut regen_warnings)?;
            for w in &regen_warnings {
                eprintln!("warning: {}", w);
            }
            let lockfile = lockfile::generate(&manifest, &skills);
            lockfile::save(&lockfile, paths.config_dir())?;

            // Surface partial-cleanup failures FIRST so they can't be
            // hidden by a ✓ success banner above them (scripted callers
            // keying on stdout miss stderr signals; humans dismiss ⚠
            // after reading ✓). On full success the success banner below
            // prints; on partial failure the ⚠ block prints and we
            // return Err (no success banner).
            if !result.failures.is_empty() {
                let k = result.failures.len();
                eprintln!(
                    "{} {} operations failed during remove of '{}' — config entry and \
                     manifest retained so you can retry after addressing these. Run {}:",
                    style("⚠").yellow(),
                    k,
                    name,
                    style("`tome doctor`").bold(),
                );

                for kind in crate::remove::FailureKind::ALL {
                    let group: Vec<&crate::remove::RemoveFailure> =
                        result.failures.iter().filter(|f| f.kind == kind).collect();
                    if group.is_empty() {
                        continue;
                    }
                    eprintln!("  {} ({}):", kind.label(), group.len());
                    for f in group {
                        eprintln!("    {}: {}", paths::collapse_home(&f.path), f.error);
                    }
                }

                return Err(anyhow::anyhow!("remove completed with {k} failures"));
            }

            // Success path — full cleanup completed with no failures.
            println!(
                "\n{} Removed directory '{}': {} library entries, {} symlinks{}",
                style("✓").green(),
                name,
                result.library_entries_removed,
                result.symlinks_removed,
                if result.git_cache_removed {
                    ", git cache"
                } else {
                    ""
                },
            );
        }
        Command::Reassign { skill, to } => {
            let mut manifest = manifest::load(paths.config_dir())?;
            let plan = reassign::plan(&skill, &to, &config, &paths, &manifest, false)?;
            reassign::render_plan(&plan);

            let target_dir_path = config
                .directories
                .get(&config::DirectoryName::new(&to)?)
                .map(|d| config::expand_tilde(&d.path))
                .transpose()?
                .ok_or_else(|| anyhow::anyhow!("directory '{}' not found in config", to))?;

            reassign::execute(&plan, &mut manifest, &target_dir_path, cli.dry_run)?;
            if !cli.dry_run {
                manifest::save(&manifest, paths.config_dir())?;
                // Regenerate lockfile to keep it in sync. Recover git-skill
                // provenance offline from the previous lockfile + on-disk
                // cache so git-type directories are not silently dropped
                // during regen (#461 H1).
                let (resolved_paths, mut regen_warnings) =
                    lockfile::resolved_paths_from_lockfile_cache(&config, &paths);
                let skills = discover::discover_all(&config, &resolved_paths, &mut regen_warnings)?;
                for w in &regen_warnings {
                    eprintln!("warning: {}", w);
                }
                let lockfile_data = lockfile::generate(&manifest, &skills);
                lockfile::save(&lockfile_data, paths.config_dir())?;
                println!(
                    "{} '{}' from '{}' to '{}'",
                    style("Reassigned").green(),
                    style(&skill).cyan(),
                    style(&plan.from_directory).cyan(),
                    style(&to).cyan(),
                );
            }
        }
        Command::Fork { skill, to, force } => {
            let mut manifest = manifest::load(paths.config_dir())?;
            let plan = reassign::plan(&skill, &to, &config, &paths, &manifest, true)?;
            reassign::render_plan(&plan);

            if !force {
                if !cli.no_input && std::io::stdin().is_terminal() {
                    let confirmed = dialoguer::Confirm::new()
                        .with_prompt(format!(
                            "Fork '{}' to '{}'? This copies skill files to the target directory.",
                            skill, to
                        ))
                        .default(false)
                        .interact()?;
                    if !confirmed {
                        println!("Aborted.");
                        return Ok(());
                    }
                } else {
                    anyhow::bail!(
                        "tome fork requires confirmation — use --force in non-interactive mode"
                    );
                }
            }

            let target_dir_path = config
                .directories
                .get(&config::DirectoryName::new(&to)?)
                .map(|d| config::expand_tilde(&d.path))
                .transpose()?
                .ok_or_else(|| anyhow::anyhow!("directory '{}' not found in config", to))?;

            reassign::execute(&plan, &mut manifest, &target_dir_path, cli.dry_run)?;
            if !cli.dry_run {
                manifest::save(&manifest, paths.config_dir())?;
                // Regenerate lockfile to keep it in sync. Recover git-skill
                // provenance offline from the previous lockfile + on-disk
                // cache so git-type directories are not silently dropped
                // during regen (#461 H1).
                let (resolved_paths, mut regen_warnings) =
                    lockfile::resolved_paths_from_lockfile_cache(&config, &paths);
                let skills = discover::discover_all(&config, &resolved_paths, &mut regen_warnings)?;
                for w in &regen_warnings {
                    eprintln!("warning: {}", w);
                }
                let lockfile_data = lockfile::generate(&manifest, &skills);
                lockfile::save(&lockfile_data, paths.config_dir())?;
                println!(
                    "{} '{}' to '{}' (local copy created)",
                    style("Forked").green(),
                    style(&skill).cyan(),
                    style(&to).cyan(),
                );
            }
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
            let config_path = cli.config.clone().unwrap_or_else(|| paths.config_path());

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
        Command::Completions { shell, print } => {
            if print {
                print_completions(shell);
            } else {
                install_completions(shell)?;
            }
        }
        Command::List { json } => list(&config, cli.quiet, json)?,
        Command::Config { path } => show_config(&config, path, &paths.config_path())?,
        Command::Backup { sub } => match sub {
            cli::BackupCommand::Init => {
                backup::init(paths.tome_home(), cli.dry_run)?;
                // Offer remote setup after successful init (interactive only)
                if !cli.dry_run
                    && std::io::stdin().is_terminal()
                    && !backup::has_remote(paths.tome_home())
                {
                    offer_remote_setup(paths.tome_home())?;
                }
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

/// Warn about `disabled_directories` entries in machine.toml that don't match any
/// configured directory name. Helps catch typos and stale entries.
fn warn_unknown_disabled_directories(machine_prefs: &machine::MachinePrefs, config: &Config) {
    for name in &machine_prefs.disabled_directories {
        if !config.directories.contains_key(name.as_str()) {
            eprintln!(
                "warning: disabled directory '{}' in machine.toml does not match any configured directory",
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
    no_input: bool,
    verbose: bool,
    quiet: bool,
    machine_override: Option<&'a Path>,
}

/// Pre-discovery step: clone or update git-type directories.
///
/// Returns a map of directory name -> (resolved local path, optional HEAD SHA).
/// Failed git operations produce warnings and are skipped (GIT-08).
fn resolve_git_directories(
    config: &Config,
    paths: &TomePaths,
    dry_run: bool,
    quiet: bool,
    verbose: bool,
) -> BTreeMap<DirectoryName, (PathBuf, Option<String>)> {
    let mut resolved = BTreeMap::new();
    let repos_dir = paths.repos_dir();

    // Check git availability once
    if !config
        .directories
        .values()
        .any(|d| d.directory_type == DirectoryType::Git)
    {
        return resolved;
    }

    if !git::is_git_available() {
        if !quiet {
            eprintln!("warning: git is not available — skipping all git-type directories");
        }
        return resolved;
    }

    // Read HEAD sha and warn (not silently swallow) when the cache is
    // unreadable — without the warning the lockfile would record
    // git_commit_sha: null, falsely claiming "no provenance".
    let read_sha_or_warn = |cache_dir: &Path, name: &DirectoryName| -> Option<String> {
        match git::read_head_sha(cache_dir) {
            Ok(sha) => Some(sha),
            Err(e) => {
                if !quiet {
                    eprintln!(
                        "warning: could not read HEAD sha for '{}' cache at {}: {e}",
                        name,
                        cache_dir.display()
                    );
                }
                None
            }
        }
    };

    for (name, dir_config) in &config.directories {
        if dir_config.directory_type != DirectoryType::Git {
            continue;
        }

        let url = dir_config.path.to_string_lossy();
        let cache_dir = git::repo_cache_dir(&repos_dir, &url);
        let already_cloned = cache_dir.is_dir();

        if dry_run {
            // In dry-run, use cached path if it exists, skip otherwise
            if already_cloned {
                let effective = git::effective_path(&cache_dir, dir_config.subdir.as_deref());
                let sha = read_sha_or_warn(&cache_dir, name);
                resolved.insert(name.clone(), (effective, sha));
            }
            continue;
        }

        // Create repos dir if needed
        if let Err(e) = std::fs::create_dir_all(&repos_dir) {
            if !quiet {
                eprintln!(
                    "warning: failed to create repos directory {}: {e}",
                    repos_dir.display()
                );
            }
            continue;
        }

        let result = if already_cloned {
            // Update existing clone (GIT-03)
            if verbose {
                eprintln!("  Updating git directory '{}'...", name);
            }
            git::update_repo(
                &cache_dir,
                dir_config.branch.as_deref(),
                dir_config.tag.as_deref(),
                dir_config.rev.as_deref(),
            )
        } else {
            // Fresh clone (GIT-02)
            if verbose {
                eprintln!("  Cloning git directory '{}'...", name);
            }
            git::clone_repo(
                &url,
                &cache_dir,
                dir_config.branch.as_deref(),
                dir_config.tag.as_deref(),
                dir_config.rev.as_deref(),
            )
        };

        match result {
            Ok(()) => {
                let effective = git::effective_path(&cache_dir, dir_config.subdir.as_deref());
                let sha = read_sha_or_warn(&cache_dir, name);
                resolved.insert(name.clone(), (effective, sha));
            }
            Err(e) => {
                // D-09, D-10: Distinct messages for never-cloned vs update-failed
                if already_cloned {
                    if !quiet {
                        eprintln!(
                            "warning: could not update '{}' — using cached state: {e}",
                            name
                        );
                    }
                    let effective = git::effective_path(&cache_dir, dir_config.subdir.as_deref());
                    let sha = read_sha_or_warn(&cache_dir, name);
                    resolved.insert(name.clone(), (effective, sha));
                } else if !quiet {
                    eprintln!(
                        "warning: could not clone '{}' — skipping (no cached state): {e}",
                        name
                    );
                }
            }
        }
    }
    resolved
}

/// The core sync pipeline: discover → consolidate → distribute → cleanup.
fn sync(config: &Config, paths: &TomePaths, opts: SyncOptions<'_>) -> Result<()> {
    let SyncOptions {
        dry_run,
        force,
        no_triage,
        no_input,
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

    // Cache git state to avoid repeated subprocess calls
    let has_backup_repo = backup::has_repo(paths.tome_home());
    let has_remote = has_backup_repo && backup::has_remote(paths.tome_home());

    // Pull from remote before anything else (if configured)
    if !dry_run && has_remote {
        match backup::pull(paths.tome_home()) {
            Ok(true) => {
                if !quiet {
                    println!(
                        "  {} Pulled changes from remote",
                        console::style("↓").cyan()
                    );
                }
            }
            Ok(false) => {} // up to date
            Err(e) => eprintln!("warning: remote pull failed: {e}"),
        }
    }

    // Pre-sync auto-snapshot if configured
    if !dry_run && config.backup.enabled && config.backup.auto_snapshot && has_backup_repo {
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

    // Load existing lockfile for diffing and auto-install
    let old_lockfile = lockfile::load(paths.config_dir())?;

    // Auto-install missing managed plugins (before discovery so they're found).
    // Run even with --no-input so users get the info message about missing plugins.
    if !dry_run {
        reconcile_managed_plugins(&old_lockfile, config, quiet, no_input)?;
    }

    // Safety guard: warn and skip cleanup when no directories are configured (CFG-06)
    if config.directories.is_empty() {
        if !quiet {
            eprintln!("warning: no directories configured. Run `tome init` to set up directories.");
        }
        return Ok(());
    }

    // 0. Resolve git directories (clone/update to local cache)
    let sp = show_progress.then(|| spinner("Resolving git sources..."));
    if verbose {
        eprintln!("{}", style("Resolving git sources...").dim());
    }
    let resolved_git_paths = resolve_git_directories(config, paths, dry_run, quiet, verbose);
    if let Some(sp) = sp {
        sp.finish_and_clear();
    }

    // 1. Discover
    let sp = show_progress.then(|| spinner("Discovering skills..."));
    if verbose {
        eprintln!("{}", style("Discovering skills...").dim());
    }
    let mut warnings = Vec::new();
    let skills = discover::discover_all(config, &resolved_git_paths, &mut warnings)?;
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

    // 3. Diff lockfile and triage changes (pre-cleanup snapshot for diffing)
    let pre_cleanup_lockfile = lockfile::generate(&manifest, &skills);
    if !no_triage && !quiet {
        if let Some(ref old) = old_lockfile {
            let d = update::diff(old, &pre_cleanup_lockfile);
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

    // Warn about disabled_directories that don't match any configured directory
    if !quiet {
        warn_unknown_disabled_directories(&machine_prefs, config);
    }

    // 4. Cleanup stale library entries (before distribute so counts are accurate)
    // Clear the spinner before cleanup_library runs: cleanup may show interactive
    // dialoguer prompts, and a live spinner overwrites them, causing an apparent hang.
    if verbose {
        eprintln!("{}", style("Cleaning up stale entries...").dim());
    }
    let cleanup_result = cleanup::cleanup_library(
        paths.library_dir(),
        &discovered_names,
        &mut manifest,
        dry_run,
        quiet,
        no_input,
    )?;

    // Regenerate lockfile after cleanup so it reflects removals
    let new_lockfile = lockfile::generate(&manifest, &skills);

    // 5. Distribute to directories with distribution roles
    let mut distribute_results = Vec::new();
    for (name, dir_config) in config.distribution_dirs() {
        if machine_prefs.is_directory_disabled(name.as_str()) {
            if verbose {
                eprintln!(
                    "{}",
                    style(format!(
                        "Skipping directory '{}' (disabled in machine preferences)",
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
        let result = distribute::distribute_to_directory(
            paths.library_dir(),
            name,
            dir_config,
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

    // 6. Cleanup stale symlinks from distribution directories
    let mut removed_from_targets = 0usize;
    for (_name, dir_config) in config.distribution_dirs() {
        let skills_dir = &dir_config.path;
        removed_from_targets += cleanup::cleanup_target(skills_dir, paths.library_dir(), dry_run)?;
        // Also clean up symlinks for disabled skills
        removed_from_targets +=
            cleanup_disabled_from_target(skills_dir, paths.library_dir(), &machine_prefs, dry_run)?;
    }

    // 7. Save manifest, gitignore, and lockfile
    if !dry_run && paths.config_dir().is_dir() {
        manifest::save(&manifest, paths.config_dir())?;
    }
    if !dry_run && paths.library_dir().is_dir() {
        library::generate_gitignore(paths.library_dir(), &manifest)?;
    }
    if !dry_run && paths.config_dir().is_dir() {
        generate_tome_home_gitignore(paths.config_dir())?;
        lockfile::save(&new_lockfile, paths.config_dir())
            .context("failed to save lockfile — sync completed but lockfile is stale; re-run `tome sync` to retry")?;
    }

    let report = SyncReport {
        consolidate: consolidate_result,
        distributions: distribute_results,
        cleanup: cleanup_result,
        removed_from_targets,
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
    let committed = if !dry_run && !quiet {
        offer_git_commit(
            paths.tome_home(),
            report.consolidate.created,
            report.consolidate.updated,
            report.cleanup.removed_from_library,
        )?
    } else {
        false
    };

    // Push to remote after commit (only if something was committed)
    if committed && has_remote {
        match backup::push(paths.tome_home()) {
            Ok(()) => {
                if !quiet {
                    println!("  {} Pushed to remote", console::style("↑").cyan());
                }
            }
            Err(e) => eprintln!("warning: remote push failed: {e}"),
        }
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
            "  {}: {} linked, {} unchanged{}{}{}",
            style(&dr.directory_name).bold(),
            style(dr.changed).cyan(),
            dr.unchanged,
            skipped_note(dr.skipped),
            disabled_note(dr.disabled),
            managed_note(dr.skipped_managed)
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
    let mut skills = discover::discover_all(config, &BTreeMap::new(), &mut warnings)?;
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

fn managed_note(count: usize) -> String {
    if count == 0 {
        String::new()
    } else {
        format!(", {} skipped (managed)", style(count).dim())
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
///
/// Returns `true` if a commit was created, `false` otherwise.
fn offer_git_commit(
    tome_home: &Path,
    created: usize,
    updated: usize,
    removed: usize,
) -> Result<bool> {
    if !tome_home.join(".git").exists() || !std::io::stdin().is_terminal() {
        return Ok(false);
    }

    let output = match GitCommand::new("git")
        .args(["status", "--porcelain"])
        .current_dir(tome_home)
        .output()
    {
        Ok(o) => o,
        Err(e) => {
            eprintln!("warning: could not run git status: {e}");
            return Ok(false);
        }
    };

    if !output.status.success() {
        eprintln!(
            "warning: git status returned non-zero exit code {:?}",
            output.status.code()
        );
        return Ok(false);
    }
    if output.stdout.is_empty() {
        return Ok(false);
    }

    let msg = sync_commit_message(created, updated, removed);

    let confirm = dialoguer::Confirm::new()
        .with_prompt(format!("Commit changes? ({})", msg))
        .default(true)
        .interact_opt()?;

    if confirm != Some(true) {
        return Ok(false);
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
        return Ok(false);
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
        return Ok(false);
    }

    Ok(true)
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

/// Print shell completions to stdout.
fn print_completions(shell: clap_complete::Shell) {
    let mut cmd = <cli::Cli as clap::CommandFactory>::command();
    clap_complete::generate(shell, &mut cmd, "tome", &mut std::io::stdout());
}

/// Install shell completions to the standard location for the given shell.
pub(crate) fn install_completions(shell: clap_complete::Shell) -> Result<()> {
    use clap_complete::Shell;

    let home = dirs::home_dir().context("Could not determine home directory")?;
    // Fish and Bash follow XDG conventions on all platforms (including macOS),
    // so we use XDG env vars with standard fallbacks rather than dirs::config_dir()
    // which returns ~/Library/Application Support on macOS.
    let xdg_config = std::env::var("XDG_CONFIG_HOME")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|_| home.join(".config"));
    let xdg_data = std::env::var("XDG_DATA_HOME")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|_| home.join(".local/share"));
    let dest = match shell {
        Shell::Fish => xdg_config.join("fish/completions/tome.fish"),
        Shell::Bash => xdg_data.join("bash-completion/completions/tome"),
        Shell::Zsh => home.join(".zfunc/_tome"),
        Shell::PowerShell => {
            anyhow::bail!(
                "Automatic installation not supported for PowerShell.\n\
                 Generate manually: tome completions powershell --print > tome.ps1\n\
                 Then source it from your PowerShell profile."
            );
        }
        _ => {
            anyhow::bail!("Unknown shell — cannot determine completions path");
        }
    };

    if let Some(parent) = dest.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("Could not create {}", parent.display()))?;
    }

    let mut cmd = <cli::Cli as clap::CommandFactory>::command();
    let mut buf = Vec::new();
    clap_complete::generate(shell, &mut cmd, "tome", &mut buf);
    std::fs::write(&dest, &buf).with_context(|| format!("Could not write {}", dest.display()))?;

    println!("Installed {} completions to {}", shell, dest.display());
    if shell == Shell::Zsh {
        println!(
            "Ensure ~/.zfunc is in your fpath. Add to .zshrc:\n  \
             fpath=(~/.zfunc $fpath)\n  \
             autoload -Uz compinit && compinit"
        );
    }
    Ok(())
}

/// Show or print config information.
fn show_config(config: &Config, path_only: bool, config_path: &Path) -> Result<()> {
    if path_only {
        println!("{}", config_path.display());
    } else {
        let toml_str = toml::to_string_pretty(config)?;
        println!("{}", toml_str);
    }
    Ok(())
}

/// Auto-install managed plugins that are in the lockfile but not installed locally.
fn reconcile_managed_plugins(
    old_lockfile: &Option<lockfile::Lockfile>,
    config: &config::Config,
    quiet: bool,
    no_input: bool,
) -> Result<()> {
    let Some(lf) = old_lockfile else {
        return Ok(());
    };
    let Some(json_path) = install::find_installed_plugins_json(config) else {
        return Ok(());
    };
    match install::reconcile(lf, &json_path, false, quiet, no_input) {
        Ok(n) if n > 0 && !quiet => {
            println!(
                "  {} Installed {n} managed plugin(s)",
                console::style("✓").green()
            );
        }
        Ok(_) => {}
        Err(e) => eprintln!("warning: plugin auto-install failed: {e}"),
    }
    Ok(())
}

/// Interactive prompt to add a remote for cross-machine sync after `tome backup init`.
fn offer_remote_setup(tome_home: &Path) -> Result<()> {
    let add_remote = dialoguer::Confirm::new()
        .with_prompt("Add a remote for cross-machine sync?")
        .default(false)
        .interact()?;

    if !add_remote {
        return Ok(());
    }

    let url: String = dialoguer::Input::new()
        .with_prompt("Remote URL (e.g. git@github.com:user/tome-home.git)")
        .interact_text()?;

    backup::add_remote(tome_home, &url)?;

    print!("Verifying connection... ");
    match backup::verify_remote(tome_home) {
        Ok(()) => {
            println!("{}", console::style("ok").green());
        }
        Err(e) => {
            println!("{}", console::style("failed").red());
            eprintln!("warning: {e}");
            eprintln!(
                "The remote was added but could not be reached. Fix the URL or credentials, then run `tome sync`."
            );
            return Ok(());
        }
    }

    match backup::push_initial(tome_home) {
        Ok(()) => {
            println!(
                "{} Remote configured and initial push complete",
                console::style("✓").green()
            );
        }
        Err(e) => {
            eprintln!("warning: initial push failed: {e}");
            eprintln!("The remote was added. Push will be retried on next `tome sync`.");
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::discover::SkillName;
    use std::os::unix::fs as unix_fs;
    use std::path::PathBuf;
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
    fn resolve_tome_home_cli_flag_takes_priority() {
        let result = resolve_tome_home(
            Some(Path::new("/custom/home")),
            Some(Path::new("/other/tome.toml")),
        )
        .unwrap();
        assert_eq!(result, Path::new("/custom/home"));
    }

    #[test]
    fn resolve_tome_home_config_path_returns_parent() {
        let result =
            resolve_tome_home(None, Some(Path::new("/home/user/.tome/tome.toml"))).unwrap();
        assert_eq!(result, Path::new("/home/user/.tome"));
    }

    #[test]
    fn resolve_tome_home_bare_filename_returns_error() {
        let result = resolve_tome_home(None, Some(Path::new("tome.toml")));
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
            let result = resolve_tome_home(None, Some(Path::new(path)));
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
        let result = resolve_tome_home(None, None).unwrap();
        let expected = config::default_tome_home().unwrap();
        assert_eq!(result, expected);
    }

    #[test]
    fn resolve_tome_home_tilde_expansion() {
        let result = resolve_tome_home(Some(Path::new("~/my-skills/.tome")), None).unwrap();
        let home = dirs::home_dir().unwrap();
        assert_eq!(result, home.join("my-skills/.tome"));
    }

    #[test]
    fn resolve_tome_home_relative_tome_home_returns_error() {
        let result = resolve_tome_home(Some(Path::new("relative/path")), None);
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("must be an absolute path")
        );
    }

    // --- resolve_config_path tests ---

    #[test]
    fn resolve_config_path_cli_config_takes_priority() {
        let result = resolve_config_path(
            Some(Path::new("/custom/home")),
            Some(Path::new("/explicit/config.toml")),
        )
        .unwrap();
        assert_eq!(result, Some(PathBuf::from("/explicit/config.toml")));
    }

    #[test]
    fn resolve_config_path_derives_from_tome_home() {
        let result = resolve_config_path(Some(Path::new("/my/tome-home")), None).unwrap();
        assert_eq!(result, Some(PathBuf::from("/my/tome-home/tome.toml")));
    }

    #[test]
    fn resolve_config_path_tilde_expansion() {
        let result = resolve_config_path(Some(Path::new("~/my-repo/.tome")), None).unwrap();
        let home = dirs::home_dir().unwrap();
        assert_eq!(result, Some(home.join("my-repo/.tome/tome.toml")));
    }

    #[test]
    fn resolve_config_path_none_returns_none() {
        let result = resolve_config_path(None, None).unwrap();
        assert_eq!(result, None);
    }

    #[test]
    fn init_with_no_input_does_not_bail_from_lib_run() {
        // Guard against re-introduction of the `tome init requires interactive
        // input` bail in the Init branch. A real integration test of
        // `tome init --no-input` lives in tests/cli.rs; this source-grep test
        // is a cheap compile-time-ish belt-and-braces.
        //
        // The needle is split across two concatenated string literals so this
        // test's source itself does not match its own search string — otherwise
        // `include_str!` would see the sentinel here and always fail.
        let src = include_str!("lib.rs");
        let needle = concat!("anyhow::bail!(\"tome init requires", " interactive input");
        assert!(
            !src.contains(needle),
            "lib.rs still contains the removed `tome init requires interactive input` bail"
        );
    }
}
