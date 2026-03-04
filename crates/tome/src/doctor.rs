//! Diagnose and optionally repair issues such as missing entries, orphan directories,
//! and stale target symlinks.

use anyhow::{Context, Result};
use console::style;
use dialoguer::Confirm;
use std::path::Path;

use crate::cleanup;
use crate::config::Config;
use crate::manifest;
use crate::paths::resolve_symlink_target;

/// Diagnose and optionally repair issues.
pub fn diagnose(config: &Config, dry_run: bool) -> Result<()> {
    // Not yet initialised — no config file, no library directory
    if !config.library_dir.is_dir() && config.sources.is_empty() {
        println!("Not configured yet. Run `tome init` to get started.");
        return Ok(());
    }

    if dry_run {
        eprintln!(
            "{}",
            style("[dry-run] No changes will be made").yellow().bold()
        );
    }

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
                repair_library(&config.library_dir)?;

                for (name, t) in config.targets.iter() {
                    if t.enabled
                        && let Some(skills_dir) = t.skills_dir()
                    {
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

/// Check the library for issues: missing manifest entries, orphan directories, broken symlinks.
fn check_library(library_dir: &Path) -> Result<usize> {
    if !library_dir.is_dir() {
        println!("  {} library directory does not exist", style("!").yellow());
        return Ok(1);
    }

    let m = match manifest::load(library_dir) {
        Ok(m) => m,
        Err(e) => {
            println!(
                "  {} manifest is corrupted or unreadable: {}",
                style("x").red(),
                e
            );
            return Ok(1);
        }
    };
    let mut issues = 0;

    // Check manifest entries exist on disk
    for name in m.keys() {
        if !library_dir.join(name.as_str()).is_dir() {
            println!(
                "  {} manifest entry '{}' has no directory on disk",
                style("x").red(),
                name
            );
            issues += 1;
        }
    }

    // Check disk entries are in manifest (orphans)
    let entries = std::fs::read_dir(library_dir)
        .with_context(|| format!("failed to read library dir {}", library_dir.display()))?;

    for entry in entries {
        let entry =
            entry.with_context(|| format!("failed to read entry in {}", library_dir.display()))?;
        let path = entry.path();
        let name = entry.file_name().to_string_lossy().to_string();

        if path.is_dir() && !name.starts_with('.') && !m.contains_key(&name) {
            println!(
                "  {} orphan directory: {} (not in manifest)",
                style("!").yellow(),
                path.display()
            );
            issues += 1;
        }

        // Check for broken legacy symlinks
        if path.is_symlink() && !path.exists() {
            let raw_target = std::fs::read_link(&path)
                .with_context(|| format!("failed to read symlink {}", path.display()))?;
            println!(
                "  {} broken symlink: {} -> {}",
                style("x").red(),
                path.display(),
                raw_target.display()
            );
            issues += 1;
        }
    }

    if issues == 0 {
        println!("  {} library OK", style("ok").green());
    }

    Ok(issues)
}

/// Repair library issues: remove orphan manifest entries and broken symlinks.
fn repair_library(library_dir: &Path) -> Result<()> {
    let mut m = manifest::load(library_dir).with_context(|| {
        "cannot repair: manifest is unreadable. Back up .tome-manifest.json and run sync --force"
    })?;
    let mut fixed = 0;

    // Remove manifest entries missing from disk
    let missing: Vec<String> = m
        .keys()
        .filter(|name| !library_dir.join(name.as_str()).is_dir())
        .map(|name| name.as_str().to_string())
        .collect();
    for name in missing {
        m.remove(&name);
        println!(
            "  {} Removed manifest entry '{}' (directory missing)",
            style("fixed").green(),
            name
        );
        fixed += 1;
    }

    // Remove broken legacy symlinks
    let entries = std::fs::read_dir(library_dir)
        .with_context(|| format!("failed to read library dir {}", library_dir.display()))?;

    for entry in entries {
        let entry =
            entry.with_context(|| format!("failed to read entry in {}", library_dir.display()))?;
        let path = entry.path();

        if path.is_symlink() && !path.exists() {
            std::fs::remove_file(&path)
                .with_context(|| format!("failed to remove broken symlink {}", path.display()))?;
            println!(
                "  {} Removed broken symlink {}",
                style("fixed").green(),
                path.display()
            );
            fixed += 1;
        }
    }

    if fixed > 0 {
        manifest::save(&m, library_dir)?;
    }

    Ok(())
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

    // Canonicalize library_dir so starts_with works when library_dir contains
    // a symlink component (e.g., /var -> /private/var on macOS).
    let canonical_library =
        std::fs::canonicalize(library_dir).unwrap_or_else(|_| library_dir.to_path_buf());

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
            let points_into_library =
                target.starts_with(library_dir) || target.starts_with(&canonical_library);
            if points_into_library && !target.exists() {
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

        // Create a skill directory and matching manifest entry
        let skill_dir = lib.path().join("my-skill");
        std::fs::create_dir_all(&skill_dir).unwrap();

        let mut m = manifest::Manifest::default();
        m.insert(
            crate::discover::SkillName::new("my-skill").unwrap(),
            manifest::SkillEntry {
                source_path: PathBuf::from("/tmp/source/my-skill"),
                source_name: "test".to_string(),
                content_hash: "abc".to_string(),
                synced_at: "2024-01-01T00:00:00Z".to_string(),
            },
        );
        manifest::save(&m, lib.path()).unwrap();

        let result = check_library(lib.path()).unwrap();
        assert_eq!(result, 0);
    }

    #[test]
    fn check_library_missing_manifest_entry() {
        let lib = TempDir::new().unwrap();

        // Manifest entry with no directory
        let mut m = manifest::Manifest::default();
        m.insert(
            crate::discover::SkillName::new("gone").unwrap(),
            manifest::SkillEntry {
                source_path: PathBuf::from("/tmp/source/gone"),
                source_name: "test".to_string(),
                content_hash: "abc".to_string(),
                synced_at: "2024-01-01T00:00:00Z".to_string(),
            },
        );
        manifest::save(&m, lib.path()).unwrap();

        let result = check_library(lib.path()).unwrap();
        assert_eq!(result, 1);
    }

    #[test]
    fn check_library_orphan_directory() {
        let lib = TempDir::new().unwrap();

        // Directory not in manifest
        std::fs::create_dir_all(lib.path().join("orphan")).unwrap();

        let result = check_library(lib.path()).unwrap();
        assert_eq!(result, 1);
    }

    #[test]
    fn check_library_broken_legacy_symlink() {
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

        let stale_target = lib.path().join("deleted-skill");
        unix_fs::symlink(&stale_target, target_dir.path().join("skill-link")).unwrap();

        let result = check_target_dir("test", target_dir.path(), lib.path()).unwrap();
        assert_eq!(result, 1);
    }

    #[test]
    fn check_target_dir_ignores_external_symlinks() {
        let lib = TempDir::new().unwrap();
        let target_dir = TempDir::new().unwrap();

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

    // -- diagnose (pre-init guard) --

    #[test]
    fn diagnose_shows_init_prompt_when_unconfigured() {
        let config = Config {
            library_dir: PathBuf::from("/nonexistent/library"),
            ..Config::default()
        };

        let result = diagnose(&config, true);
        assert!(result.is_ok());
    }
}
