//! Tome — sync AI coding skills across tools.
//! Re-exports all modules and contains the core `sync()` pipeline: discover, consolidate, distribute, cleanup.

pub(crate) mod cleanup;
pub mod cli;
pub mod config;
pub(crate) mod discover;
pub(crate) mod distribute;
pub(crate) mod doctor;
pub(crate) mod library;
pub mod mcp;
pub(crate) mod paths;
pub(crate) mod status;
pub(crate) mod wizard;

use anyhow::Result;
use console::style;
use indicatif::{ProgressBar, ProgressStyle};

use cli::{Cli, Command};
use config::Config;

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
        Command::List => list(&config, cli.quiet)?,
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
    let skills = discover::discover_all(config, quiet)?;
    if let Some(sp) = sp {
        sp.finish_and_clear();
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

    // 2. Consolidate into library
    let sp = show_progress.then(|| spinner("Consolidating to library..."));
    if verbose {
        eprintln!("{}", style("Consolidating to library...").dim());
    }
    let consolidate_result = library::consolidate(&skills, &config.library_dir, dry_run, force)?;
    if let Some(sp) = sp {
        sp.finish_and_clear();
    }

    // 3. Distribute to targets
    let mut distribute_results = Vec::new();
    for (name, target) in config.targets.iter() {
        let sp = show_progress.then(|| spinner(&format!("Distributing to {}...", name)));
        if verbose {
            eprintln!("{}", style(format!("Distributing to {}...", name)).dim());
        }
        let result =
            distribute::distribute_to_target(&config.library_dir, name, target, dry_run, force)?;
        distribute_results.push(result);
        if let Some(sp) = sp {
            sp.finish_and_clear();
        }
    }

    // 4. Cleanup stale links
    let sp = show_progress.then(|| spinner("Cleaning up stale links..."));
    if verbose {
        eprintln!("{}", style("Cleaning up stale links...").dim());
    }
    let cleanup_result = cleanup::cleanup_library(&config.library_dir, dry_run)?;

    let mut removed_from_targets = 0usize;
    for (_name, target) in config.targets.iter() {
        if let Some(skills_dir) = target.skills_dir() {
            removed_from_targets +=
                cleanup::cleanup_target(skills_dir, &config.library_dir, dry_run)?;
        }
    }
    if let Some(sp) = sp {
        sp.finish_and_clear();
    }

    if quiet {
        return Ok(());
    }

    // Report
    println!("{}", style("Sync complete").green().bold());
    let lib_skipped = if consolidate_result.skipped > 0 {
        format!(
            ", {} skipped (path conflict)",
            style(consolidate_result.skipped).yellow()
        )
    } else {
        String::new()
    };
    println!(
        "  Library: {} created, {} unchanged, {} updated{}",
        style(consolidate_result.created).cyan(),
        consolidate_result.unchanged,
        consolidate_result.updated,
        lib_skipped
    );

    for dr in &distribute_results {
        let skipped_note = if dr.skipped > 0 {
            format!(", {} skipped (path conflict)", style(dr.skipped).yellow())
        } else {
            String::new()
        };
        println!(
            "  {}: {} linked, {} unchanged{}",
            style(&dr.target_name).bold(),
            style(dr.changed).cyan(),
            dr.unchanged,
            skipped_note
        );
    }

    if cleanup_result.removed_from_library > 0 {
        println!(
            "  Cleaned {} stale link(s)",
            style(cleanup_result.removed_from_library).yellow()
        );
    }

    if removed_from_targets > 0 {
        println!(
            "  Cleaned {} stale target link(s)",
            style(removed_from_targets).yellow()
        );
    }

    Ok(())
}

/// List all discovered skills.
fn list(config: &Config, quiet: bool) -> Result<()> {
    let skills = discover::discover_all(config, quiet)?;

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
