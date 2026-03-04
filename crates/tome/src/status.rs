//! Read-only summary of the library state, configured sources, targets, and overall health.

use anyhow::{Context, Result};
use console::style;
use std::path::Path;
use tabled::settings::{Modify, Style, object::Rows};

use crate::config::Config;
use crate::discover;
use crate::manifest;

/// Display the current status of the tome system.
pub fn show(config: &Config) -> Result<()> {
    // Not yet initialised — no config file, no library directory
    if !config.library_dir.is_dir() && config.sources.is_empty() {
        println!("Not configured yet. Run `tome init` to get started.");
        return Ok(());
    }

    println!(
        "{} {}",
        style("Library:").bold(),
        config.library_dir.display()
    );

    // Count skills in library
    let lib_count = match count_entries(&config.library_dir) {
        Ok(n) => format!("{}", n),
        Err(e) => {
            eprintln!("warning: could not read library: {}", e);
            "?".to_string()
        }
    };
    println!("  {} skills consolidated", style(lib_count).cyan());
    println!();

    // Sources
    println!("{}", style("Sources:").bold());
    if config.sources.is_empty() {
        println!("  (none configured)");
    } else {
        let mut rows: Vec<[String; 4]> = Vec::with_capacity(config.sources.len() + 1);
        rows.push([
            "SOURCE".to_string(),
            "TYPE".to_string(),
            "PATH".to_string(),
            "SKILLS".to_string(),
        ]);
        for source in &config.sources {
            let count = match discover::discover_source(source) {
                Ok(s) => format!("{}", s.len()),
                Err(e) => {
                    eprintln!(
                        "warning: could not discover skills from '{}': {}",
                        source.name, e
                    );
                    "?".to_string()
                }
            };
            rows.push([
                source.name.clone(),
                source.source_type.to_string(),
                source.path.display().to_string(),
                count,
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
    }
    println!();

    // Targets
    println!("{}", style("Targets:").bold());
    let target_entries: Vec<_> = config.targets.iter().collect();
    if target_entries.is_empty() {
        println!("  (none configured)");
    } else {
        let mut rows: Vec<[String; 3]> = Vec::with_capacity(target_entries.len() + 1);
        rows.push([
            "TARGET".to_string(),
            "STATUS".to_string(),
            "METHOD".to_string(),
        ]);
        for (name, t) in &target_entries {
            let status = if t.enabled {
                style("enabled").green().to_string()
            } else {
                style("disabled").dim().to_string()
            };
            let method = match &t.method {
                crate::config::TargetMethod::Symlink { .. } => "symlink",
                crate::config::TargetMethod::Mcp { .. } => "mcp",
            };
            rows.push([name.to_string(), status, method.to_string()]);
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
    }
    println!();

    // Health check
    let health = match count_health_issues(&config.library_dir) {
        Ok(0) => format!("{}", style("All good").green()),
        Ok(n) => format!("{}", style(format!("{} issue(s)", n)).red()),
        Err(e) => {
            eprintln!("warning: could not check library health: {}", e);
            format!("{}", style("unknown").yellow())
        }
    };
    println!("{} {}", style("Health:").bold(), health);

    Ok(())
}

/// Count skill directories in the library.
fn count_entries(dir: &Path) -> Result<usize> {
    let mut count = 0;
    for entry in std::fs::read_dir(dir)
        .with_context(|| format!("failed to read directory {}", dir.display()))?
    {
        let entry = entry.with_context(|| format!("failed to read entry in {}", dir.display()))?;
        if entry.path().is_dir() {
            count += 1;
        }
    }
    Ok(count)
}

/// Count health issues: manifest/disk mismatches.
fn count_health_issues(dir: &Path) -> Result<usize> {
    let m = manifest::load(dir)?;
    let mut issues = 0;

    // Check manifest entries exist on disk
    for name in m.keys() {
        if !dir.join(name.as_str()).is_dir() {
            issues += 1;
        }
    }

    // Check disk entries are in manifest (orphans)
    for entry in std::fs::read_dir(dir)
        .with_context(|| format!("failed to read directory {}", dir.display()))?
    {
        let entry = entry.with_context(|| format!("failed to read entry in {}", dir.display()))?;
        let name = entry.file_name().to_string_lossy().to_string();
        if entry.path().is_dir() && !name.starts_with('.') && !m.contains_key(&name) {
            issues += 1;
        }
    }

    // Also count any broken symlinks (leftover from v0.1.x)
    for entry in std::fs::read_dir(dir)
        .with_context(|| format!("failed to read directory {}", dir.display()))?
    {
        let entry = entry.with_context(|| format!("failed to read entry in {}", dir.display()))?;
        let path = entry.path();
        if path.is_symlink() && !path.exists() {
            issues += 1;
        }
    }

    Ok(issues)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;
    use std::path::PathBuf;

    #[test]
    fn status_shows_init_prompt_when_unconfigured() {
        let config = Config {
            library_dir: PathBuf::from("/nonexistent/tome/library"),
            ..Config::default()
        };

        let result = show(&config);
        assert!(result.is_ok());
    }

    #[test]
    fn status_warns_when_library_missing_but_sources_configured() {
        use crate::config::{Source, SourceType};

        let config = Config {
            library_dir: PathBuf::from("/nonexistent/tome/library"),
            sources: vec![Source {
                name: "test".to_string(),
                path: PathBuf::from("/nonexistent/source"),
                source_type: SourceType::Directory,
            }],
            ..Config::default()
        };

        let result = show(&config);
        assert!(result.is_ok());
    }

    #[test]
    fn status_shows_tables_with_configured_sources_and_targets() {
        use crate::config::{Source, SourceType, TargetConfig, TargetMethod, Targets};

        let lib_dir = tempfile::TempDir::new().unwrap();
        let source_dir = tempfile::TempDir::new().unwrap();
        let target_skill = tempfile::TempDir::new().unwrap();

        // Create a skill in the source
        let skill = source_dir.path().join("my-skill");
        std::fs::create_dir_all(&skill).unwrap();
        std::fs::write(skill.join("SKILL.md"), "# My Skill").unwrap();

        // Create a real directory in the library (v0.2 style)
        let lib_skill = lib_dir.path().join("my-skill");
        std::fs::create_dir_all(&lib_skill).unwrap();
        std::fs::write(lib_skill.join("SKILL.md"), "# My Skill").unwrap();

        let config = Config {
            library_dir: lib_dir.path().to_path_buf(),
            sources: vec![Source {
                name: "test-source".to_string(),
                path: source_dir.path().to_path_buf(),
                source_type: SourceType::Directory,
            }],
            targets: Targets {
                antigravity: Some(TargetConfig {
                    enabled: true,
                    method: TargetMethod::Symlink {
                        skills_dir: target_skill.path().to_path_buf(),
                    },
                }),
                ..Default::default()
            },
            ..Config::default()
        };

        let result = show(&config);
        assert!(result.is_ok());
    }

    // -- count_entries --

    #[test]
    fn count_entries_empty_dir() {
        let dir = tempfile::TempDir::new().unwrap();
        assert_eq!(count_entries(dir.path()).unwrap(), 0);
    }

    #[test]
    fn count_entries_ignores_regular_files() {
        let dir = tempfile::TempDir::new().unwrap();
        for name in ["a", "b", "c"] {
            std::fs::write(dir.path().join(name), "").unwrap();
        }
        assert_eq!(count_entries(dir.path()).unwrap(), 0);
    }

    #[test]
    fn count_entries_counts_directories() {
        let dir = tempfile::TempDir::new().unwrap();

        // Two directories — should be counted
        std::fs::create_dir_all(dir.path().join("skill-a")).unwrap();
        std::fs::create_dir_all(dir.path().join("skill-b")).unwrap();
        // One regular file — should be ignored
        std::fs::write(dir.path().join(".tome-manifest.json"), "{}").unwrap();

        assert_eq!(count_entries(dir.path()).unwrap(), 2);
    }

    // -- count_health_issues --

    #[test]
    fn count_health_issues_empty_dir() {
        let dir = tempfile::TempDir::new().unwrap();
        assert_eq!(count_health_issues(dir.path()).unwrap(), 0);
    }

    #[test]
    fn count_health_issues_detects_manifest_disk_mismatch() {
        let dir = tempfile::TempDir::new().unwrap();

        // Create a manifest entry with no corresponding directory
        let mut m = manifest::Manifest::default();
        m.insert(
            crate::discover::SkillName::new("missing").unwrap(),
            manifest::SkillEntry {
                source_path: PathBuf::from("/tmp/source"),
                source_name: "test".to_string(),
                content_hash: "abc".to_string(),
                synced_at: "2024-01-01T00:00:00Z".to_string(),
            },
        );
        manifest::save(&m, dir.path()).unwrap();

        assert_eq!(count_health_issues(dir.path()).unwrap(), 1);
    }

    #[test]
    fn count_health_issues_detects_orphan_directory() {
        let dir = tempfile::TempDir::new().unwrap();

        // Create a directory not tracked by manifest
        std::fs::create_dir_all(dir.path().join("orphan-skill")).unwrap();

        assert_eq!(count_health_issues(dir.path()).unwrap(), 1);
    }

    #[test]
    fn count_health_issues_ignores_hidden_dirs() {
        let dir = tempfile::TempDir::new().unwrap();

        // .git dir should not be counted as an orphan
        std::fs::create_dir_all(dir.path().join(".git")).unwrap();

        assert_eq!(count_health_issues(dir.path()).unwrap(), 0);
    }
}
