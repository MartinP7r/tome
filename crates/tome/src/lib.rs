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
//! - [`mcp`] — MCP server for exposing skills to AI tools
//! - [`run()`] — entry point that dispatches CLI commands

pub(crate) mod cleanup;
pub mod cli;
pub mod config;
pub(crate) mod discover;
pub(crate) mod distribute;
pub(crate) mod doctor;
pub(crate) mod library;
pub(crate) mod lockfile;
pub(crate) mod manifest;
pub mod mcp;
pub(crate) mod paths;
pub(crate) mod status;
pub(crate) mod wizard;

use std::collections::HashSet;
use std::io::IsTerminal;
use std::path::Path;
use std::process::Command as GitCommand;

use anyhow::Result;
use console::style;
use indicatif::{ProgressBar, ProgressStyle};

use cleanup::CleanupResult;
use cli::{Cli, Command};
use config::Config;
use distribute::DistributeResult;
use library::ConsolidateResult;

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

/// Run the CLI with parsed arguments.
pub fn run(cli: Cli) -> Result<()> {
    if matches!(cli.command, Command::Init) {
        if let Err(e) = Config::load_or_default(cli.config.as_deref()) {
            eprintln!(
                "warning: existing config is malformed ({}), the wizard will create a new one",
                e
            );
        }
        let config = wizard::run(cli.dry_run)?;
        config.validate()?;
        if !cli.dry_run {
            sync(&config, cli.dry_run, false, cli.verbose, cli.quiet)?;
        }
        return Ok(());
    }

    let config = Config::load_or_default(cli.config.as_deref())?;
    config.validate()?;

    match cli.command {
        Command::Init => unreachable!(),
        Command::Sync { force } => sync(&config, cli.dry_run, force, cli.verbose, cli.quiet)?,
        Command::Status => status::show(&config)?,
        Command::Doctor => doctor::diagnose(&config, cli.dry_run)?,
        Command::Serve => {
            tokio::runtime::Runtime::new()?.block_on(mcp::serve(config))?;
        }
        Command::List { json } => list(&config, cli.quiet, json)?,
        Command::Config { path } => show_config(&config, path)?,
    }

    Ok(())
}

/// The core sync pipeline: discover → consolidate → distribute → cleanup.
fn sync(config: &Config, dry_run: bool, force: bool, verbose: bool, quiet: bool) -> Result<()> {
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
    let (consolidate_result, mut manifest) =
        library::consolidate(&skills, &config.library_dir, dry_run, force)?;
    if let Some(sp) = sp {
        sp.finish_and_clear();
    }

    let discovered_names: HashSet<String> =
        skills.iter().map(|s| s.name.as_str().to_string()).collect();

    // 3. Distribute to targets
    let mut distribute_results = Vec::new();
    for (name, target) in config.targets.iter() {
        let sp = show_progress.then(|| spinner(&format!("Distributing to {}...", name)));
        if verbose {
            eprintln!("{}", style(format!("Distributing to {}...", name)).dim());
        }
        let result = distribute::distribute_to_target(
            &config.library_dir,
            name,
            target,
            &manifest,
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
        &config.library_dir,
        &discovered_names,
        &mut manifest,
        dry_run,
    )?;

    let mut removed_from_targets = 0usize;
    for (_name, target) in config.targets.iter() {
        if let Some(skills_dir) = target.skills_dir() {
            removed_from_targets +=
                cleanup::cleanup_target(skills_dir, &config.library_dir, dry_run)?;
        }
    }
    // Save manifest after cleanup (may have removed entries)
    if !dry_run && config.library_dir.is_dir() {
        manifest::save(&manifest, &config.library_dir)?;
    }

    // Generate .gitignore after cleanup so stale entries are excluded
    if !dry_run && config.library_dir.is_dir() {
        library::generate_gitignore(&config.library_dir, &manifest)?;
    }

    // Generate lockfile for reproducibility
    if !dry_run && config.library_dir.is_dir() {
        let lf = lockfile::generate(&manifest, &skills);
        lockfile::save(&lf, &config.library_dir)?;
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
            &config.library_dir,
            report.consolidate.created,
            report.consolidate.updated,
            report.cleanup.removed_from_library,
        )?;
    }

    Ok(())
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
            "  {}: {} linked, {} unchanged{}",
            style(&dr.target_name).bold(),
            style(dr.changed).cyan(),
            dr.unchanged,
            skipped_note(dr.skipped)
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

/// If the library directory is a git repo with uncommitted changes, prompt the user to commit.
fn offer_git_commit(
    library_dir: &Path,
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

    if !output.status.success() || output.stdout.is_empty() {
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

    let add_status = GitCommand::new("git")
        .args(["add", "."])
        .current_dir(library_dir)
        .status()?;
    if !add_status.success() {
        eprintln!(
            "warning: git add failed (exit code {:?})",
            add_status.code()
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
}
