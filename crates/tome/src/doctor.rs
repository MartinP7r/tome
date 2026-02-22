//! Diagnose and optionally repair issues such as broken symlinks and missing source paths.

use anyhow::{Context, Result};
use console::style;
use dialoguer::Confirm;
use std::path::Path;

use crate::cleanup;
use crate::config::Config;
use crate::paths::resolve_symlink_target;

/// Diagnose and optionally repair issues.
pub fn diagnose(config: &Config, dry_run: bool) -> Result<()> {
    let mut total_issues = 0;

    // Check library
    println!("{}", style("Checking library...").bold());
    let library_issues = check_library(&config.library_dir)?;
    total_issues += library_issues;

    // Check targets
    println!("{}", style("Checking targets...").bold());
    for (name, t) in config.targets.iter() {
        if t.enabled
            && let Some(skills_dir) = t.skills_dir()
        {
            let target_issues = check_target_dir(name, skills_dir, &config.library_dir)?;
            total_issues += target_issues;
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
            let confirmed = Confirm::new()
                .with_prompt("Repair these issues?")
                .default(true)
                .interact()?;

            if confirmed {
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

                for (name, t) in config.targets.iter() {
                    if let Some(skills_dir) = t.skills_dir() {
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
    let entries = std::fs::read_dir(library_dir)
        .with_context(|| format!("failed to read library dir {}", library_dir.display()))?;

    for entry in entries {
        let entry =
            entry.with_context(|| format!("failed to read entry in {}", library_dir.display()))?;
        let path = entry.path();

        if path.is_symlink() {
            let raw_target = std::fs::read_link(&path)
                .with_context(|| format!("failed to read symlink {}", path.display()))?;
            let target = resolve_symlink_target(&path, &raw_target);
            if !target.exists() {
                println!(
                    "  {} broken symlink: {} -> {}",
                    style("x").red(),
                    path.display(),
                    raw_target.display()
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
    let entries = std::fs::read_dir(skills_dir)
        .with_context(|| format!("failed to read target dir {}", skills_dir.display()))?;

    for entry in entries {
        let entry =
            entry.with_context(|| format!("failed to read entry in {}", skills_dir.display()))?;
        let path = entry.path();

        if path.is_symlink() {
            let raw_target = std::fs::read_link(&path)
                .with_context(|| format!("failed to read symlink {}", path.display()))?;
            let target = resolve_symlink_target(&path, &raw_target);
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{Source, SourceType};
    use std::os::unix::fs as unix_fs;
    use std::path::PathBuf;
    use tempfile::TempDir;

    // -- check_library --

    #[test]
    fn check_library_missing_dir() {
        let result = check_library(Path::new("/nonexistent/library")).unwrap();
        assert_eq!(result, 1);
    }

    #[test]
    fn check_library_no_issues() {
        let lib = TempDir::new().unwrap();
        let target_dir = TempDir::new().unwrap();
        let target = target_dir.path().join("real-skill");
        std::fs::create_dir(&target).unwrap();

        unix_fs::symlink(&target, lib.path().join("my-skill")).unwrap();

        let result = check_library(lib.path()).unwrap();
        assert_eq!(result, 0);
    }

    #[test]
    fn check_library_broken_symlink() {
        let lib = TempDir::new().unwrap();
        unix_fs::symlink("/nonexistent/target", lib.path().join("broken")).unwrap();

        let result = check_library(lib.path()).unwrap();
        assert_eq!(result, 1);
    }

    // -- check_target_dir --

    #[test]
    fn check_target_dir_missing_dir() {
        let lib = TempDir::new().unwrap();
        let result =
            check_target_dir("test-target", Path::new("/nonexistent/target"), lib.path()).unwrap();
        assert_eq!(result, 1);
    }

    #[test]
    fn check_target_dir_stale_symlink() {
        let lib = TempDir::new().unwrap();
        let target_dir = TempDir::new().unwrap();

        // Symlink inside target_dir pointing into library at a path that doesn't exist
        let stale_target = lib.path().join("deleted-skill");
        unix_fs::symlink(&stale_target, target_dir.path().join("skill-link")).unwrap();

        let result = check_target_dir("test", target_dir.path(), lib.path()).unwrap();
        assert_eq!(result, 1);
    }

    #[test]
    fn check_target_dir_ignores_external_symlinks() {
        let lib = TempDir::new().unwrap();
        let target_dir = TempDir::new().unwrap();

        // Symlink pointing outside the library — doctor should ignore it
        unix_fs::symlink("/some/other/place", target_dir.path().join("external")).unwrap();

        let result = check_target_dir("test", target_dir.path(), lib.path()).unwrap();
        assert_eq!(result, 0);
    }

    // -- check_config --

    #[test]
    fn check_config_missing_source() {
        let config = Config {
            sources: vec![Source {
                name: "gone".to_string(),
                path: PathBuf::from("/nonexistent/source"),
                source_type: SourceType::Directory,
            }],
            ..Config::default()
        };

        let result = check_config(&config).unwrap();
        assert_eq!(result, 1);
    }

    #[test]
    fn check_config_valid_sources() {
        let source_dir = TempDir::new().unwrap();
        let config = Config {
            sources: vec![Source {
                name: "real".to_string(),
                path: source_dir.path().to_path_buf(),
                source_type: SourceType::Directory,
            }],
            ..Config::default()
        };

        let result = check_config(&config).unwrap();
        assert_eq!(result, 0);
    }
}
