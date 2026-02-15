use anyhow::Result;
use console::style;
use std::path::Path;

use crate::cleanup;
use crate::config::Config;

/// Diagnose and optionally repair issues.
pub fn diagnose(config: &Config, dry_run: bool) -> Result<()> {
    let mut total_issues = 0;

    // Check library
    println!("{}", style("Checking library...").bold());
    let library_issues = check_library(&config.library_dir)?;
    total_issues += library_issues;

    // Check targets
    println!("{}", style("Checking targets...").bold());
    let targets = [
        ("antigravity", &config.targets.antigravity),
        ("codex", &config.targets.codex),
        ("openclaw", &config.targets.openclaw),
    ];

    for (name, target) in &targets {
        if let Some(t) = target {
            if t.enabled {
                if let Some(ref skills_dir) = t.skills_dir {
                    let target_issues = check_target_dir(name, skills_dir, &config.library_dir)?;
                    total_issues += target_issues;
                }
            }
        }
    }

    // Check config
    println!("{}", style("Checking config...").bold());
    let config_issues = check_config(config)?;
    total_issues += config_issues;

    println!();
    if total_issues == 0 {
        println!("{}", style("No issues found.").green().bold());
    } else {
        println!(
            "{}",
            style(format!("Found {} issue(s).", total_issues))
                .yellow()
                .bold()
        );

        if !dry_run {
            println!();
            println!("{}", style("Repairing...").bold());
            let cleanup_result = cleanup::cleanup_library(&config.library_dir, false)?;
            if cleanup_result.removed_from_library > 0 {
                println!(
                    "  {} Removed {} broken symlink(s) from library",
                    style("fixed").green(),
                    cleanup_result.removed_from_library
                );
            }

            for (name, target) in &targets {
                if let Some(t) = target {
                    if let Some(ref skills_dir) = t.skills_dir {
                        let removed =
                            cleanup::cleanup_target(skills_dir, &config.library_dir, false)?;
                        if removed > 0 {
                            println!(
                                "  {} Removed {} stale symlink(s) from {}",
                                style("fixed").green(),
                                removed,
                                name
                            );
                        }
                    }
                }
            }
        } else {
            println!("  (dry run — no changes made)");
        }
    }

    Ok(())
}

fn check_library(library_dir: &Path) -> Result<usize> {
    if !library_dir.is_dir() {
        println!("  {} library directory does not exist", style("!").yellow());
        return Ok(1);
    }

    let mut issues = 0;
    let entries = std::fs::read_dir(library_dir)?;

    for entry in entries {
        let entry = entry?;
        let path = entry.path();

        if path.is_symlink() {
            let target = std::fs::read_link(&path)?;
            if !target.exists() {
                println!(
                    "  {} broken symlink: {} -> {}",
                    style("x").red(),
                    path.display(),
                    target.display()
                );
                issues += 1;
            }
        }
    }

    if issues == 0 {
        println!("  {} library OK", style("ok").green());
    }

    Ok(issues)
}

fn check_target_dir(name: &str, skills_dir: &Path, library_dir: &Path) -> Result<usize> {
    if !skills_dir.is_dir() {
        println!(
            "  {} {}: target directory does not exist ({})",
            style("!").yellow(),
            name,
            skills_dir.display()
        );
        return Ok(1);
    }

    let mut issues = 0;
    let entries = std::fs::read_dir(skills_dir)?;

    for entry in entries {
        let entry = entry?;
        let path = entry.path();

        if path.is_symlink() {
            let target = std::fs::read_link(&path)?;
            if target.starts_with(library_dir) && !target.exists() {
                println!(
                    "  {} {}: stale symlink {}",
                    style("x").red(),
                    name,
                    path.display()
                );
                issues += 1;
            }
        }
    }

    if issues == 0 {
        println!("  {} {}: OK", style("ok").green(), name);
    }

    Ok(issues)
}

fn check_config(config: &Config) -> Result<usize> {
    let mut issues = 0;

    for source in &config.sources {
        if !source.path.exists() {
            println!(
                "  {} source '{}' path does not exist: {}",
                style("!").yellow(),
                source.name,
                source.path.display()
            );
            issues += 1;
        }
    }

    if issues == 0 {
        println!("  {} config OK", style("ok").green());
    }

    Ok(issues)
}
