pub mod cleanup;
pub mod cli;
pub mod config;
pub mod discover;
pub mod distribute;
pub mod doctor;
pub mod library;
pub mod mcp;
pub mod paths;
pub mod status;
pub mod wizard;

use anyhow::Result;
use console::style;

use cli::{Cli, Command};
use config::Config;

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
        if !cli.dry_run {
            sync(&config, cli.dry_run, cli.verbose)?;
        }
        return Ok(());
    }

    let config = Config::load_or_default(cli.config.as_deref())?;
    config.validate()?;

    match cli.command {
        Command::Init => unreachable!(),
        Command::Sync => sync(&config, cli.dry_run, cli.verbose)?,
        Command::Status => status::show(&config)?,
        Command::Doctor => doctor::diagnose(&config, cli.dry_run)?,
        Command::Serve => {
            tokio::runtime::Runtime::new()?.block_on(mcp::serve(config))?;
        }
        Command::List => list(&config)?,
        Command::Config { path } => show_config(&config, path)?,
    }

    Ok(())
}

/// The core sync pipeline: discover → consolidate → distribute → cleanup.
fn sync(config: &Config, dry_run: bool, verbose: bool) -> Result<()> {
    // 1. Discover
    if verbose {
        eprintln!("{}", style("Discovering skills...").dim());
    }
    let skills = discover::discover_all(config)?;

    if skills.is_empty() {
        println!("No skills found. Run `skync init` to configure sources.");
        return Ok(());
    }

    if verbose {
        eprintln!("  Found {} skills", skills.len());
    }

    // 2. Consolidate into library
    if verbose {
        eprintln!("{}", style("Consolidating to library...").dim());
    }
    let consolidate_result = library::consolidate(&skills, &config.library_dir, dry_run)?;

    // 3. Distribute to targets
    let mut distribute_results = Vec::new();
    for (name, target) in config.targets.iter() {
        if verbose {
            eprintln!("{}", style(format!("Distributing to {}...", name)).dim());
        }
        let result = distribute::distribute_to_target(&config.library_dir, name, target, dry_run)?;
        distribute_results.push(result);
    }

    // 4. Cleanup stale links
    if verbose {
        eprintln!("{}", style("Cleaning up stale links...").dim());
    }
    let cleanup_result = cleanup::cleanup_library(&config.library_dir, dry_run)?;

    let mut removed_from_targets = 0usize;
    for (_name, target) in config.targets.iter() {
        if let Some(ref skills_dir) = target.skills_dir {
            removed_from_targets +=
                cleanup::cleanup_target(skills_dir, &config.library_dir, dry_run)?;
        }
    }

    // Report
    if dry_run {
        println!("{}", style("Dry run — no changes made").yellow());
        println!();
    }

    println!("{}", style("Sync complete").green().bold());
    println!(
        "  Library: {} created, {} unchanged, {} updated",
        style(consolidate_result.created).cyan(),
        consolidate_result.unchanged,
        consolidate_result.updated
    );

    for dr in &distribute_results {
        println!(
            "  {}: {} linked, {} unchanged",
            style(&dr.target_name).bold(),
            style(dr.linked).cyan(),
            dr.unchanged
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
fn list(config: &Config) -> Result<()> {
    let skills = discover::discover_all(config)?;

    if skills.is_empty() {
        println!("No skills found. Run `skync init` to configure sources.");
        return Ok(());
    }

    println!(
        "{:<30} {:<20} {}",
        style("SKILL").bold(),
        style("SOURCE").bold(),
        style("PATH").bold()
    );

    for skill in &skills {
        println!(
            "{:<30} {:<20} {}",
            skill.name,
            style(&skill.source_name).dim(),
            style(skill.path.display()).dim()
        );
    }

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
