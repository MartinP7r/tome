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
// `browse` is normally `pub(crate)` so the rest of v1.0's GUI Tauri IPC
// surface doesn't accidentally bind to it. When the `test-support`
// feature is enabled (used by integration tests in `tests/browse_snapshots/`
// per HARD-12) it widens to `pub` so the snapshot harness can construct
// `App` fixtures and call `ui::render` against a `TestBackend`. Production
// builds (`cargo build` without features) keep the old `pub(crate)`
// visibility byte-for-byte.
#[cfg(any(test, feature = "test-support"))]
pub mod browse;
#[cfg(not(any(test, feature = "test-support")))]
pub(crate) mod browse;
pub(crate) mod cleanup;
pub mod cli;
pub mod config;
pub(crate) mod discover;
pub(crate) mod distribute;
pub(crate) mod doctor;
pub(crate) mod eject;
pub(crate) mod git;
pub(crate) mod library;
pub(crate) mod lint;
pub(crate) mod lockfile;
pub(crate) mod machine;
pub(crate) mod manifest;
pub mod marketplace;
pub(crate) mod migration_v010;
pub(crate) mod paths;
pub(crate) mod reassign;
pub(crate) mod reconcile;
pub(crate) mod relocate;
pub(crate) mod remove;
pub(crate) mod skill;
pub(crate) mod status;
pub(crate) mod summary;
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

/// Re-exported for integration tests so the synthetic-fixture builder in
/// `tests/cli.rs` can hash directories with the exact same algorithm the
/// production manifest uses (avoids a duplicated SHA-256 helper that could
/// drift). Production code should still call `manifest::hash_directory`
/// directly via the crate path.
pub use manifest::hash_directory;

/// HARD-04: surface lint-failure and migrate-failure typed errors so the
/// thin `main.rs` binary can downcast and map them to exit code 1 without
/// the library calling `process::exit` itself.
pub use lint::LintFailed;
pub use migration_v010::MigrationPartialOrFailed;

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

/// Resolve the machine preferences path from an optional CLI flag,
/// falling back to the default `~/.config/tome/machine.toml`.
fn resolve_machine_path(cli_machine: Option<&Path>) -> Result<std::path::PathBuf> {
    match cli_machine {
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
                        Err(e) => {
                            // The Edit action is only offered when the existing
                            // config parses cleanly (see wizard::brownfield_decision).
                            // Reaching this arm with Err means a refactor broke that
                            // invariant; bail with a recoverable error so the user
                            // gets an actionable message instead of a panic.
                            anyhow::bail!(
                                "internal: brownfield Edit reached with unparsable config: {e:#}"
                            );
                        }
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
            // Load machine prefs once at the top of the post-Init sync path
            // (mirrors the canonical `run()` load order). Init does NOT use
            // `Config::load_with_overrides` because the wizard runs against
            // the bare tome.toml that the user is about to write — overrides
            // would mask schema errors the wizard wants to surface.
            let machine_path = resolve_machine_path(cli.machine.as_deref())?;
            let machine_prefs = machine::load(&machine_path)?;
            sync(
                &expanded,
                &paths,
                SyncOptions {
                    dry_run: cli.dry_run,
                    force: false,
                    no_triage: true, // skip on initial sync after init
                    no_input: cli.no_input,
                    no_install: false,
                    verbose: cli.log_level().is_verbose(),
                    quiet: cli.log_level().is_quiet(),
                    machine_path: &machine_path,
                    machine_prefs: &machine_prefs,
                },
            )?;
        }
        return Ok(());
    }

    // Load per-machine preferences first — they may rewrite directory paths via
    // `[directory_overrides.<name>]` entries, which `Config::load_with_overrides`
    // applies between `expand_tildes()` and `validate()` (PORT-02 / I2 invariant).
    let machine_path = resolve_machine_path(cli.machine.as_deref())?;
    let machine_prefs = machine::load(&machine_path)?;

    let config = Config::load_or_default_with_overrides(
        effective_config.as_deref(),
        &machine_path,
        &machine_prefs,
    )?;
    // Note: load_or_default_with_overrides already runs validate() internally —
    // no separate config.validate()? call here.
    let tome_home = resolve_tome_home(cli.tome_home.as_deref(), cli.config.as_deref())?;
    let paths = TomePaths::new(tome_home, config.library_dir.clone())?;

    // HARD-02: dispatch via per-subcommand `cmd_<name>` helpers defined later
    // in this file. Each match arm is a one-line call into the helper, keeping
    // `run` itself a thin router. Init and Version are dispatched via
    // early-returns above, so the corresponding arms here are unreachable
    // contract guards.
    match cli.command {
        Command::Init => unreachable_early_return("Command::Init"),
        Command::Version => unreachable_early_return("Command::Version"),
        Command::Add {
            url,
            name,
            branch,
            tag,
            rev,
        } => cmd_add(url, name, branch, tag, rev, config, &paths, cli.dry_run),
        Command::Sync {
            force,
            no_triage,
            no_install,
        } => {
            let log = cli.log_level();
            cmd_sync(
                force,
                no_triage,
                no_install,
                &config,
                &paths,
                &machine_path,
                &machine_prefs,
                cli.dry_run,
                cli.no_input,
                log.is_verbose(),
                log.is_quiet(),
            )
        }
        Command::Status { json } => cmd_status(&config, &paths, json),
        Command::Doctor { json } => cmd_doctor(&config, &paths, cli.dry_run, cli.no_input, json),
        Command::Lint { path, format } => cmd_lint(path, format, &paths),
        Command::Browse => cmd_browse(&config, &paths, cli.log_level().is_quiet()),
        Command::Remove { kind } => cmd_remove(
            kind,
            config,
            &paths,
            cli.machine.as_deref(),
            cli.dry_run,
            cli.no_input,
        ),
        Command::Reassign { skill, to, force } => {
            cmd_reassign(skill, to, force, &config, &paths, cli.dry_run)
        }
        Command::Fork { skill, to, force } => {
            cmd_fork(skill, to, force, &config, &paths, cli.dry_run, cli.no_input)
        }
        Command::MigrateLibrary { dry_run } => cmd_migrate_library(&paths, dry_run || cli.dry_run),
        Command::Eject => cmd_eject(&config, &paths, cli.dry_run),
        Command::Relocate { new_path } => cmd_relocate(
            new_path,
            &config,
            &paths,
            cli.config.as_deref(),
            cli.dry_run,
        ),
        Command::Completions { shell, print } => cmd_completions(shell, print),
        Command::List { json } => cmd_list(&config, cli.log_level().is_quiet(), json),
        Command::Config { path } => cmd_config(&config, path, &paths),
        Command::Backup { sub } => cmd_backup(sub, &paths, cli.dry_run),
    }
}

/// Guard for command variants whose handling is dispatched via an early
/// return at the top of `run()` (Init, Version). Reaching the post-setup
/// match for one of these means a refactor broke the early-return contract;
/// bail so the user sees an actionable error instead of a silent fallthrough.
#[cold]
fn unreachable_early_return(variant: &str) -> Result<()> {
    anyhow::bail!(
        "internal: {variant} reached the main dispatch but should have been handled by the early-return path"
    )
}

// ---------------------------------------------------------------------------
// Per-subcommand dispatch helpers (HARD-02)
//
// One `cmd_<name>` per `cli::Command` variant. Each helper consumes args
// already extracted from the variant plus the shared state `run()` resolves
// once (paths, config, machine prefs). Helpers do NOT re-load config or paths.
// ---------------------------------------------------------------------------

/// `tome add <url>` — register a git directory in config from a URL.
#[allow(clippy::too_many_arguments)]
pub(crate) fn cmd_add(
    url: String,
    name: Option<String>,
    branch: Option<String>,
    tag: Option<String>,
    rev: Option<String>,
    config: Config,
    paths: &TomePaths,
    dry_run: bool,
) -> Result<()> {
    let mut config = config;
    add::add(
        &mut config,
        add::AddOptions {
            url: &url,
            name: name.as_deref(),
            branch: branch.as_deref(),
            tag: tag.as_deref(),
            rev: rev.as_deref(),
            dry_run,
            config_path: &paths.config_path(),
        },
    )?;
    Ok(())
}

/// `tome sync` — run the full discover → consolidate → distribute → cleanup pipeline.
#[allow(clippy::too_many_arguments)]
pub(crate) fn cmd_sync(
    force: bool,
    no_triage: bool,
    no_install: bool,
    config: &Config,
    paths: &TomePaths,
    machine_path: &Path,
    machine_prefs: &machine::MachinePrefs,
    dry_run: bool,
    no_input: bool,
    verbose: bool,
    quiet: bool,
) -> Result<()> {
    sync(
        config,
        paths,
        SyncOptions {
            dry_run,
            force,
            no_triage: no_triage || no_input,
            no_input,
            no_install,
            verbose,
            quiet,
            machine_path,
            machine_prefs,
        },
    )
}

/// `tome status` — read-only summary of library, directories, and health.
pub(crate) fn cmd_status(config: &Config, paths: &TomePaths, json: bool) -> Result<()> {
    status::show(config, paths, json)
}

/// `tome doctor` — diagnose and (optionally) repair library/symlink issues.
pub(crate) fn cmd_doctor(
    config: &Config,
    paths: &TomePaths,
    dry_run: bool,
    no_input: bool,
    json: bool,
) -> Result<()> {
    doctor::diagnose(config, paths, dry_run, no_input, json)
}

/// `tome lint` — validate skill frontmatter; exits 1 when errors are found.
pub(crate) fn cmd_lint(
    path: Option<PathBuf>,
    format: cli::LintFormat,
    paths: &TomePaths,
) -> Result<()> {
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
    // HARD-04: bubble up a downcastable error rather than `process::exit(1)`
    // so embedding callers can decide how to translate the failure.
    // `main.rs` downcasts and maps to exit code 1.
    if report.has_errors() {
        anyhow::bail!(lint::LintFailed {
            violations: report.error_count(),
        });
    }
    Ok(())
}

/// `tome browse` — interactive TUI browser for the discovered skills.
pub(crate) fn cmd_browse(config: &Config, paths: &TomePaths, quiet: bool) -> Result<()> {
    let mut warnings = Vec::new();
    let skills = discover::discover_all(config, &BTreeMap::new(), &mut warnings)?;
    if !quiet {
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
    Ok(())
}

/// `tome remove dir|skill` — directory removal vs Unowned-skill deletion (D-API-2).
pub(crate) fn cmd_remove(
    kind: cli::RemoveKind,
    config: Config,
    paths: &TomePaths,
    cli_machine: Option<&Path>,
    dry_run: bool,
    no_input: bool,
) -> Result<()> {
    match kind {
        cli::RemoveKind::Dir { name, force } => {
            cmd_remove_dir(name, force, config, paths, dry_run, no_input)
        }
        cli::RemoveKind::Skill { name, yes } => {
            cmd_remove_skill(name, yes, &config, paths, cli_machine, dry_run, no_input)
        }
    }
}

/// `tome remove dir <name>` — remove a directory entry from `tome.toml` and
/// clean up its artifacts (D-API-2). Owned skills transition to Unowned.
fn cmd_remove_dir(
    name: String,
    force: bool,
    config: Config,
    paths: &TomePaths,
    dry_run: bool,
    no_input: bool,
) -> Result<()> {
    let manifest = manifest::load(paths.config_dir())?;
    let plan = remove::plan(&name, &config, paths, &manifest)?;
    remove::render_plan(&plan);

    if dry_run {
        println!("\n{}", style("Dry run — no changes made.").yellow());
        return Ok(());
    }

    if !force {
        if !no_input && std::io::stdin().is_terminal() {
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

    // Surface partial-cleanup failures BEFORE the save chain. If any of
    // config.save / manifest::save / discover_all / lockfile::save returns
    // Err, `?` would otherwise propagate and the user would only see a
    // disk-write error — never the ⚠ block or the I2/I3 retention messaging
    // ("config entry and manifest retained so you can retry"). Returning
    // here also means in-memory mutations to `config` and `manifest` are
    // never persisted on the failure path, which is correct: remove::execute
    // deliberately leaves them in their pre-mutation state when failures
    // occur, so the disk state on retry matches the in-memory state.
    if !result.failures.is_empty() {
        let k = result.failures.len();
        eprintln!(
            "{} {} operations failed during remove of '{}' — config entry and \
             manifest retained so you can retry after addressing these. \
             Run {} after resolving:",
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

    // Save updated config
    config.save(&paths.config_path())?;
    // Save updated manifest
    manifest::save(&manifest, paths.config_dir())?;
    // Regenerate lockfile. Recover git-skill provenance offline from
    // the previous lockfile + on-disk cache so git-type directories
    // are not silently dropped during regen (#461 H1). Warnings
    // collected here are deferred until AFTER the success banner —
    // see comment below (TEST-04 option a).
    let (resolved_paths, mut regen_warnings) =
        lockfile::resolved_paths_from_lockfile_cache(&config, paths);
    let skills = discover::discover_all(&config, &resolved_paths, &mut regen_warnings)?;
    let lockfile = lockfile::generate(&manifest, &skills);
    lockfile::save(&lockfile, paths.config_dir())?;

    // Success banner FIRST (TEST-04 option a — deferred regen-warnings).
    // The banner is the user's anchor for "what just happened"; warnings
    // come after as a footnote. Without this ordering, multi-warning
    // regen output buries the green ✓ confirmation and the user has to
    // scroll up to find it. The deferred ordering is regression-tested
    // by `lib_rs_remove_handler_prints_success_banner_before_regen_warnings`
    // in tests/cli.rs.
    println!(
        "\n{} Removed directory '{}': {} library entries kept as Unowned, {} symlinks{}",
        style("✓").green(),
        name,
        result.library_entries_transitioned_to_unowned,
        result.symlinks_removed,
        if result.git_cache_removed {
            ", git cache"
        } else {
            ""
        },
    );
    for w in &regen_warnings {
        eprintln!("warning: {}", w);
    }
    Ok(())
}

/// `tome remove skill <name>` — delete an Unowned skill from the library
/// (manifest entry, library directory, distribution symlinks, lockfile entry,
/// machine.toml memberships) per Phase 14 D-B1.
fn cmd_remove_skill(
    name: String,
    yes: bool,
    config: &Config,
    paths: &TomePaths,
    cli_machine: Option<&Path>,
    dry_run: bool,
    no_input: bool,
) -> Result<()> {
    // Load all the pieces skill_plan needs to compute the cleanup
    // scope (D-B1): manifest entry, lockfile entry, and machine
    // prefs memberships.
    let manifest = manifest::load(paths.config_dir())?;
    let lockfile = lockfile::load(paths.config_dir())?;
    let machine_path = resolve_machine_path(cli_machine)?;
    let machine_prefs = machine::load(&machine_path)?;

    let plan = remove::skill_plan(
        &name,
        config,
        paths,
        &manifest,
        lockfile.as_ref(),
        &machine_prefs,
    )?;
    remove::skill_render_plan(&plan);

    if dry_run {
        println!("\n{}", style("Dry run — no changes made.").yellow());
        return Ok(());
    }

    // D-B3: confirmation default-no, --yes / -y bypasses. Mirrors
    // the existing `tome remove dir` confirmation default.
    if !yes {
        if !no_input && std::io::stdin().is_terminal() {
            let confirmed = dialoguer::Confirm::new()
                .with_prompt(format!("Are you sure you want to forget skill '{}'?", name))
                .default(false)
                .interact()?;
            if !confirmed {
                println!("Aborted.");
                return Ok(());
            }
        } else {
            anyhow::bail!(
                "tome remove skill requires confirmation — use --yes in non-interactive mode"
            );
        }
    }

    let mut manifest = manifest;
    let mut lockfile = lockfile;
    let mut machine_prefs = machine_prefs;
    let result = remove::skill_execute(
        &plan,
        &mut manifest,
        &mut lockfile,
        &mut machine_prefs,
        false,
    )?;

    // SAFE-01 grouped partial-failure summary BEFORE any save call.
    // skill_execute deliberately leaves manifest/lockfile/machine_prefs
    // unchanged on partial failure (matches the dir-flavour I2/I3
    // retention semantic), so returning here without saving keeps
    // disk state consistent with in-memory state for retry.
    if !result.failures.is_empty() {
        let k = result.failures.len();
        eprintln!(
            "{} {} operations failed during remove of skill '{}' — \
             in-memory state retained so you can retry after addressing these. \
             Run {} after resolving:",
            style("⚠").yellow(),
            k,
            name,
            style("`tome doctor`").bold(),
        );
        for kind in remove::RemoveSkillFailureKind::ALL {
            let group: Vec<&remove::RemoveSkillFailure> =
                result.failures.iter().filter(|f| f.kind == kind).collect();
            if group.is_empty() {
                continue;
            }
            eprintln!("  {} ({}):", kind.label(), group.len());
            for f in group {
                eprintln!("    {}: {}", paths::collapse_home(&f.path), f.error);
            }
        }
        return Err(anyhow::anyhow!(
            "tome remove skill completed with {k} failures"
        ));
    }

    // D-B1 atomic-save chain: manifest + lockfile + machine.toml
    // (each uses temp+rename internally).
    manifest::save(&manifest, paths.config_dir())?;
    if let Some(lf) = &lockfile {
        lockfile::save(lf, paths.config_dir())?;
    }
    machine::save(&machine_prefs, &machine_path)?;

    // Success banner. Reports each step that actually cleaned
    // something so the user sees the full scope of the operation
    // (library, symlinks, lockfile, machine.toml). Counters that
    // were no-ops (e.g. skill had no lockfile entry) are omitted.
    let mut parts: Vec<String> = Vec::new();
    if result.library_removed {
        parts.push("library".to_string());
    }
    if result.symlinks_removed > 0 {
        parts.push(format!("{} symlinks", result.symlinks_removed));
    }
    if result.lockfile_entry_removed {
        parts.push("lockfile entry".to_string());
    }
    if result.machine_disabled_removed {
        parts.push("machine.toml disabled".to_string());
    }
    if result.per_directory_cleanups > 0 {
        parts.push(format!(
            "{} per-directory entries",
            result.per_directory_cleanups
        ));
    }
    let summary = if parts.is_empty() {
        "manifest entry only (nothing else to clean)".to_string()
    } else {
        parts.join(", ")
    };
    println!(
        "\n{} Forgot skill '{}' — cleaned: {}.",
        style("✓").green(),
        name,
        summary,
    );
    Ok(())
}

/// `tome reassign <skill> --to <dir>` — change skill provenance.
pub(crate) fn cmd_reassign(
    skill: String,
    to: String,
    force: bool,
    config: &Config,
    paths: &TomePaths,
    dry_run: bool,
) -> Result<()> {
    let mut manifest = manifest::load(paths.config_dir())?;
    let plan = reassign::plan(&skill, &to, config, paths, &manifest, false, force)?;
    reassign::render_plan(&plan);

    let target_dir_path = config
        .directories
        .get(&config::DirectoryName::new(&to)?)
        .map(|d| config::expand_tilde(&d.path))
        .transpose()?
        .ok_or_else(|| anyhow::anyhow!("directory '{}' not found in config", to))?;

    reassign::execute(&plan, &mut manifest, &target_dir_path, dry_run)?;
    if !dry_run {
        manifest::save(&manifest, paths.config_dir())?;
        // Regenerate lockfile to keep it in sync. Recover git-skill
        // provenance offline from the previous lockfile + on-disk
        // cache so git-type directories are not silently dropped
        // during regen (#461 H1).
        let (resolved_paths, mut regen_warnings) =
            lockfile::resolved_paths_from_lockfile_cache(config, paths);
        let skills = discover::discover_all(config, &resolved_paths, &mut regen_warnings)?;
        for w in &regen_warnings {
            eprintln!("warning: {}", w);
        }
        let lockfile_data = lockfile::generate(&manifest, &skills);
        lockfile::save(&lockfile_data, paths.config_dir())?;
        let from_label = match &plan.from_directory {
            Some(d) => style(d.as_str().to_string()).cyan().to_string(),
            None => style("Unowned").yellow().to_string(),
        };
        println!(
            "{} '{}' from '{}' to '{}'",
            style("Reassigned").green(),
            style(&skill).cyan(),
            from_label,
            style(&to).cyan(),
        );
    }
    Ok(())
}

/// `tome fork <skill> --to <dir>` — fork a managed skill to a local directory.
pub(crate) fn cmd_fork(
    skill: String,
    to: String,
    force: bool,
    config: &Config,
    paths: &TomePaths,
    dry_run: bool,
    no_input: bool,
) -> Result<()> {
    let mut manifest = manifest::load(paths.config_dir())?;
    // Phase 14 D-A1: Fork shares the reassign::plan path, so Fork's
    // existing --force flag (skip-confirmation) now also bypasses
    // the D-A1 different-content collision refusal. The user's
    // mental model — "--force on fork bypasses safety checks" —
    // still holds; the surface is just slightly bigger.
    let plan = reassign::plan(&skill, &to, config, paths, &manifest, true, force)?;
    reassign::render_plan(&plan);

    if !force {
        if !no_input && std::io::stdin().is_terminal() {
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
            anyhow::bail!("tome fork requires confirmation — use --force in non-interactive mode");
        }
    }

    let target_dir_path = config
        .directories
        .get(&config::DirectoryName::new(&to)?)
        .map(|d| config::expand_tilde(&d.path))
        .transpose()?
        .ok_or_else(|| anyhow::anyhow!("directory '{}' not found in config", to))?;

    reassign::execute(&plan, &mut manifest, &target_dir_path, dry_run)?;
    if !dry_run {
        manifest::save(&manifest, paths.config_dir())?;
        // Regenerate lockfile to keep it in sync. Recover git-skill
        // provenance offline from the previous lockfile + on-disk
        // cache so git-type directories are not silently dropped
        // during regen (#461 H1).
        let (resolved_paths, mut regen_warnings) =
            lockfile::resolved_paths_from_lockfile_cache(config, paths);
        let skills = discover::discover_all(config, &resolved_paths, &mut regen_warnings)?;
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
    Ok(())
}

/// `tome migrate-library` — one-shot v0.9 → v0.10 library migration.
/// Per D-05: any skip or failure means non-zero exit.
pub(crate) fn cmd_migrate_library(paths: &TomePaths, dry_run: bool) -> Result<()> {
    let result = migration_v010::run_migrate_library(paths, dry_run)?;
    // HARD-04 sibling: bubble through anyhow rather than `process::exit(1)`.
    // `main.rs` downcasts `MigrationPartialOrFailed` and exits with code 1.
    if result.is_partial_or_failed() {
        anyhow::bail!(migration_v010::MigrationPartialOrFailed {
            skipped_broken_source: result.skipped_broken_source,
            failed: result.failed,
        });
    }
    Ok(())
}

/// `tome eject` — remove tome's symlinks from all distribution directories.
pub(crate) fn cmd_eject(config: &Config, paths: &TomePaths, dry_run: bool) -> Result<()> {
    let plan = eject::plan(config, paths)?;
    eject::render_plan(&plan);

    if plan.total_symlinks == 0 {
        return Ok(());
    }

    if dry_run {
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
    Ok(())
}

/// `tome relocate <new_path>` — move the skill library to a new location safely.
pub(crate) fn cmd_relocate(
    new_path: PathBuf,
    config: &Config,
    paths: &TomePaths,
    cli_config: Option<&Path>,
    dry_run: bool,
) -> Result<()> {
    let config_path = cli_config
        .map(|p| p.to_path_buf())
        .unwrap_or_else(|| paths.config_path());

    let plan = relocate::plan(config, paths, &new_path, &config_path)?;
    relocate::render_plan(&plan);

    if dry_run {
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

    // Use plain Config::load (no overrides) — relocate verifies the
    // newly-written config exactly as it lives on disk. Applying
    // machine overrides here would mask the relocation result.
    let new_config = Config::load(&config_path)?;
    relocate::verify(&new_config, &plan.new_library_dir, paths.tome_home())?;
    Ok(())
}

/// `tome completions <shell>` — print or install shell completions.
pub(crate) fn cmd_completions(shell: clap_complete::Shell, print: bool) -> Result<()> {
    if print {
        print_completions(shell);
        Ok(())
    } else {
        install_completions(shell)
    }
}

/// `tome list` — list all discovered skills (text or JSON).
pub(crate) fn cmd_list(config: &Config, quiet: bool, json: bool) -> Result<()> {
    list(config, quiet, json)
}

/// `tome config` — show resolved config (TOML) or just the path.
pub(crate) fn cmd_config(config: &Config, path: bool, paths: &TomePaths) -> Result<()> {
    show_config(config, path, &paths.config_path())
}

/// `tome backup <sub>` — git-backed snapshot/restore for the library.
pub(crate) fn cmd_backup(sub: cli::BackupCommand, paths: &TomePaths, dry_run: bool) -> Result<()> {
    match sub {
        cli::BackupCommand::Init => {
            backup::init(paths.tome_home(), dry_run)?;
            // Offer remote setup after successful init (interactive only)
            if !dry_run && std::io::stdin().is_terminal() && !backup::has_remote(paths.tome_home())
            {
                offer_remote_setup(paths.tome_home())?;
            }
        }
        cli::BackupCommand::Snapshot { message } => {
            backup::snapshot(paths.tome_home(), message.as_deref(), dry_run)?;
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
            backup::restore(paths.tome_home(), &target, dry_run)?;
        }
        cli::BackupCommand::Diff { target } => {
            let diff = backup::diff(paths.tome_home(), &target)?;
            if diff.is_empty() {
                println!("No changes since {}", target);
            } else {
                println!("{}", diff);
            }
        }
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
    no_install: bool,
    verbose: bool,
    quiet: bool,
    /// Path where `machine.toml` should be saved after triage. Loaded once
    /// at `run()` entry alongside `machine_prefs` so the override-apply step
    /// in `Config::load_with_overrides` and the disabled-skill filtering
    /// inside `sync()` see identical prefs.
    machine_path: &'a Path,
    /// Per-machine preferences already loaded by the caller. `sync()` clones
    /// these locally so triage can mutate without affecting the caller's copy.
    machine_prefs: &'a machine::MachinePrefs,
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
                dir_config.git_ref.as_ref().and_then(|r| r.branch()),
                dir_config.git_ref.as_ref().and_then(|r| r.tag()),
                dir_config.git_ref.as_ref().and_then(|r| r.rev()),
            )
        } else {
            // Fresh clone (GIT-02)
            if verbose {
                eprintln!("  Cloning git directory '{}'...", name);
            }
            git::clone_repo(
                &url,
                &cache_dir,
                dir_config.git_ref.as_ref().and_then(|r| r.branch()),
                dir_config.git_ref.as_ref().and_then(|r| r.tag()),
                dir_config.git_ref.as_ref().and_then(|r| r.rev()),
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

/// Build the marketplace adapter for `[directories.<name>] type = "claude-plugins"`.
///
/// Returns `Ok(None)` when no claude-plugins directory is configured. Returns
/// `Err` per D-20 when at least one claude-plugins entry exists but the
/// `claude` binary is missing — the error message names the binary and points
/// to the actionable remedy.
///
/// Per D-11: dispatch is by `DirectoryType`. Per D-21: GitAdapter is NOT
/// constructed here — git directories flow through `resolve_git_directories`
/// instead, preserving the v0.9 byte-for-byte regression contract.
fn build_claude_adapter(config: &Config) -> Result<Option<marketplace::ClaudeMarketplaceAdapter>> {
    let needs_claude = config
        .directories
        .values()
        .any(|d| d.directory_type == DirectoryType::ClaudePlugins);
    if !needs_claude {
        return Ok(None);
    }
    // D-20: hard error with the Conflict / Why / Suggestion shape.
    let adapter = marketplace::ClaudeMarketplaceAdapter::new().with_context(|| {
        "claude binary not found on PATH.\n\n\
         Why: at least one [directories.<name>] in tome.toml has type = \
         \"claude-plugins\" — tome reconciles those entries via the claude CLI.\n\
         Suggestion: install Claude Code (https://claude.ai/code), or remove the \
         claude-plugins directory entry from tome.toml."
    })?;
    Ok(Some(adapter))
}

/// Apply the user's edit-in-library decisions to the on-disk manifest.
///
/// Per RECON-05 D-13:
/// - Fork: `managed: true → false`, `source_name: Some → None`. Library
///   content stays in place. Provenance history is dropped (one-time UX gap;
///   Phase 14 may add fields retroactively).
/// - Revert: leave the manifest untouched here — the apply_drift loop in
///   reconcile_lockfile would have applied an `adapter.update()` (revert
///   degenerates to "force a drift apply"); if the user picked revert here,
///   they're saying "I want the upstream copy" — emit a warning that revert
///   is not yet wired (deferred to a follow-up; D-16's safety guarantee is
///   "never silently overwrite", and revert is opt-in so a warn-and-skip is
///   acceptable for v0.10).
/// - Skip: emit nothing additional (the per-skill warning already fired in
///   `handle_edited`).
fn apply_edit_decisions(
    report: &reconcile::ReconcileReport,
    paths: &TomePaths,
    dry_run: bool,
) -> Result<()> {
    if report.edited.is_empty() || dry_run {
        return Ok(());
    }
    debug_assert_eq!(report.edit_decisions.len(), report.edited.len());

    let mut manifest = manifest::load(paths.config_dir())?;
    let mut mutated = false;

    for (edit, decision) in report.edited.iter().zip(report.edit_decisions.iter()) {
        match decision {
            reconcile::EditDecision::Fork => {
                if let Some(entry) = manifest.skills_get_mut(edit.name.as_str()) {
                    entry.managed = false;
                    // Per D-C1 (Phase 14, transition site 3): capture
                    // previous_source before clearing source_name. Closes the
                    // Phase 13 D-13 lossy fork-in-place gap.
                    entry.previous_source = entry.source_name.take();
                    mutated = true;
                }
            }
            reconcile::EditDecision::Revert => {
                eprintln!(
                    "warning: revert chosen for {} but is not wired in v0.10 — \
                     left as-is. Re-run with `tome sync` after manually \
                     deleting library/{} to force a fresh install.",
                    edit.name.as_str(),
                    edit.name.as_str(),
                );
            }
            reconcile::EditDecision::Skip => {}
        }
    }

    if mutated {
        manifest::save(&manifest, paths.config_dir())
            .context("failed to save manifest after edit-in-library fork-in-place flip")?;
    }
    Ok(())
}

/// Move install_failures out of a ReconcileReport so the caller can hold
/// them across the rest of the sync flow without keeping the report alive.
fn take_install_failures(
    mut report: reconcile::ReconcileReport,
) -> Vec<marketplace::InstallFailure> {
    std::mem::take(&mut report.install_failures)
}

/// The core sync pipeline: discover → consolidate → distribute → cleanup.
fn sync(config: &Config, paths: &TomePaths, opts: SyncOptions<'_>) -> Result<()> {
    let SyncOptions {
        dry_run,
        force,
        no_triage,
        no_input,
        no_install,
        verbose,
        quiet,
        machine_path,
        machine_prefs: prefs_in,
    } = opts;
    if dry_run && !quiet {
        eprintln!(
            "{}",
            style("[dry-run] No changes will be made").yellow().bold()
        );
    }

    // RESEARCH OQ-6: surface non-zero exit when reconcile failed any
    // install/update. Declared up here so the bail at end-of-sync can read
    // it after the reconcile block (below) populates it.
    let mut reconcile_install_failures: Vec<marketplace::InstallFailure> = Vec::new();

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

    // Per-machine preferences (disabled skills and targets) are loaded once
    // in `run()` so the override-apply step in `Config::load_with_overrides`
    // and the disabled-skill filtering below see identical prefs. We clone
    // here so triage (below) can mutate locally without affecting the caller.
    let mut machine_prefs = prefs_in.clone();

    // Load existing lockfile for diffing and reconciliation
    let old_lockfile = lockfile::load(paths.config_dir())?;
    // Load manifest once for reconcile's edit-in-library detection. (sync()
    // reloads it later post-consolidate; reading it twice is cheap and keeps
    // reconcile's signature simple.)
    let manifest_for_reconcile = manifest::load(paths.config_dir())?;

    // v0.10 RECON-01..05: replaces the v0.9 reconcile_managed_plugins flow.
    // Adapter dispatch by DirectoryType (D-11); git stays separate (D-21).
    if let Some(claude_adapter) = build_claude_adapter(config)? {
        let report = reconcile::reconcile_lockfile(
            old_lockfile.as_ref(),
            &manifest_for_reconcile,
            paths.library_dir(),
            &claude_adapter,
            &mut machine_prefs,
            machine_path,
            paths,
            reconcile::ReconcileOpts {
                dry_run,
                no_input,
                no_install,
                quiet,
                verbose,
            },
        )?;

        if !quiet {
            reconcile::render_summary(&report, quiet);
        }

        // Apply edit-in-library decisions to the manifest. The manifest is
        // owned by sync(); reconcile_lockfile only proposed the user's
        // choice (RECON-05 D-13).
        apply_edit_decisions(&report, paths, dry_run)?;

        // ADP-04: render grouped install failures. Sync exits non-zero at end
        // when this Vec is non-empty (RESEARCH OQ-6).
        if !report.install_failures.is_empty() {
            marketplace::render_install_failures(&report.install_failures);
            reconcile_install_failures = take_install_failures(report);
        }
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

    // v0.10 D-02: refuse to sync against a v0.9-shape library. Detection is an
    // isolated check; the entire migration_v010 module deletes cleanly with
    // this check in v0.11+.
    {
        let manifest_for_detection = manifest::load(paths.config_dir())?;
        if migration_v010::detect_v09_shape(paths.library_dir(), &manifest_for_detection) {
            anyhow::bail!(
                "library is in v0.9 shape (one or more managed skills are stored as symlinks).\n\
                 \n\
                 Why: v0.10 stores managed skills as real directory copies (LIB-01).\n\
                 Run `tome migrate-library` to convert the library, then re-run this command.\n\
                 Pass `--dry-run` first to preview changes without touching the filesystem."
            );
        }
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
                    machine::save(&machine_prefs, machine_path)?;
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
        config,
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

    // RESEARCH OQ-6: surface non-zero exit when reconcile failed any
    // install/update. The grouped failure summary already printed via
    // marketplace::render_install_failures; this bail surfaces the exit
    // code only.
    if !reconcile_install_failures.is_empty() {
        anyhow::bail!(
            "{} plugin install/update operation(s) failed during reconcile (see \
             grouped summary above)",
            reconcile_install_failures.len()
        );
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
            s.source_name.as_str().to_string(),
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

    // -- apply_edit_decisions tests (Phase 14 / D-C1 transition site 3) --

    #[test]
    fn apply_edit_decisions_fork_records_previous_source() {
        // Build a minimal manifest with one Owned managed skill and exercise
        // apply_edit_decisions with EditDecision::Fork — verify the manifest
        // entry transitions to Unowned (managed=false, source_name=None) AND
        // captures previous_source per D-C1 / Phase 13 D-13 closure.
        let tmp = TempDir::new().unwrap();
        let library = tmp.path().join("library");
        std::fs::create_dir_all(&library).unwrap();
        let paths = TomePaths::new(tmp.path().to_path_buf(), library).unwrap();
        std::fs::create_dir_all(paths.config_dir()).unwrap();

        let mut manifest = manifest::Manifest::default();
        manifest.insert(
            discover::SkillName::new("plug").unwrap(),
            manifest::SkillEntry::new(
                PathBuf::from("/tmp/plug"),
                config::DirectoryName::new("claude-plugins").unwrap(),
                validation::test_hash("h"),
                true, // managed
            ),
        );
        manifest::save(&manifest, paths.config_dir()).unwrap();

        // Build a ReconcileReport with one Edited entry and Fork decision.
        let report = reconcile::ReconcileReport {
            edited: vec![reconcile::Edited {
                name: discover::SkillName::new("plug").unwrap(),
                old_source: config::DirectoryName::new("claude-plugins").unwrap(),
                old_version: Some("1.0.0".to_string()),
            }],
            edit_decisions: vec![reconcile::EditDecision::Fork],
            ..Default::default()
        };

        apply_edit_decisions(&report, &paths, false).unwrap();

        let reloaded = manifest::load(paths.config_dir()).unwrap();
        let entry = reloaded.get("plug").unwrap();
        assert_eq!(entry.source_name, None, "fork-in-place clears source_name");
        assert!(!entry.managed, "fork-in-place clears managed");
        assert_eq!(
            entry.previous_source,
            Some(config::DirectoryName::new("claude-plugins").unwrap()),
            "fork-in-place must record previous_source per D-C1 / Phase 13 D-13 closure"
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
