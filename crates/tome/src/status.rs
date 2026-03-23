//! Read-only summary of the library state, configured sources, targets, and overall health.

use anyhow::{Context, Result};
use console::style;
use std::path::{Path, PathBuf};
use tabled::settings::{Modify, Style, object::Rows};

use crate::config::Config;
use crate::discover;
use crate::manifest;
use crate::paths::TomePaths;

// -- Data structs --

/// Status of a single configured source.
pub struct SourceStatus {
    pub name: String,
    pub source_type: String,
    pub path: String,
    /// Number of skills discovered, or an error message if discovery failed.
    pub skill_count: Result<usize, String>,
    /// Warnings emitted during discovery.
    pub warnings: Vec<String>,
}

/// Status of a single configured target.
pub struct TargetStatus {
    pub name: String,
    pub enabled: bool,
    pub method: String,
}

/// Complete status report for the tome system.
pub struct StatusReport {
    pub configured: bool,
    pub library_dir: PathBuf,
    /// Number of skills consolidated in the library, or an error message.
    pub library_count: Result<usize, String>,
    pub sources: Vec<SourceStatus>,
    pub targets: Vec<TargetStatus>,
    /// Number of health issues, or an error message.
    pub health: Result<usize, String>,
}

// -- Data gathering (pure computation, no I/O) --

/// Gather status data without producing any output.
pub fn gather(config: &Config, paths: &TomePaths) -> Result<StatusReport> {
    let configured = paths.library_dir().is_dir() || !config.sources.is_empty();

    let library_count = if paths.library_dir().is_dir() {
        count_entries(paths.library_dir()).map_err(|e| e.to_string())
    } else {
        Ok(0)
    };

    let sources: Vec<SourceStatus> = config
        .sources
        .iter()
        .map(|source| {
            let mut warnings = Vec::new();
            let skill_count = discover::discover_source(source, &mut warnings)
                .map(|s| s.len())
                .map_err(|e| e.to_string());
            SourceStatus {
                name: source.name.clone(),
                source_type: source.source_type.to_string(),
                path: source.path.display().to_string(),
                skill_count,
                warnings,
            }
        })
        .collect();

    let targets: Vec<TargetStatus> = config
        .targets
        .iter()
        .map(|(name, t)| {
            let method = match &t.method {
                crate::config::TargetMethod::Symlink { .. } => "symlink",
            };
            TargetStatus {
                name: name.as_str().to_string(),
                enabled: t.enabled,
                method: method.to_string(),
            }
        })
        .collect();

    let health = if paths.library_dir().is_dir() {
        count_health_issues(paths.library_dir(), paths.tome_home()).map_err(|e| e.to_string())
    } else {
        Ok(0)
    };

    Ok(StatusReport {
        configured,
        library_dir: paths.library_dir().to_path_buf(),
        library_count,
        sources,
        targets,
        health,
    })
}

// -- Rendering --

/// Display the current status of the tome system.
pub fn show(config: &Config, paths: &TomePaths) -> Result<()> {
    let report = gather(config, paths)?;
    render_status(&report);
    Ok(())
}

fn render_status(report: &StatusReport) {
    if !report.configured {
        println!("Not configured yet. Run `tome init` to get started.");
        return;
    }

    // Library
    println!(
        "{} {}",
        style("Library:").bold(),
        report.library_dir.display()
    );
    let lib_count = match &report.library_count {
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
    if report.sources.is_empty() {
        println!("  (none configured)");
    } else {
        let mut rows: Vec<[String; 4]> = Vec::with_capacity(report.sources.len() + 1);
        rows.push([
            "SOURCE".to_string(),
            "TYPE".to_string(),
            "PATH".to_string(),
            "SKILLS".to_string(),
        ]);
        for source in &report.sources {
            let count = match &source.skill_count {
                Ok(n) => format!("{}", n),
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
                source.source_type.clone(),
                source.path.clone(),
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
        for source in &report.sources {
            for w in &source.warnings {
                eprintln!("warning: {}", w);
            }
        }
    }
    println!();

    // Targets
    println!("{}", style("Targets:").bold());
    if report.targets.is_empty() {
        println!("  (none configured)");
    } else {
        let mut rows: Vec<[String; 3]> = Vec::with_capacity(report.targets.len() + 1);
        rows.push([
            "TARGET".to_string(),
            "STATUS".to_string(),
            "METHOD".to_string(),
        ]);
        for target in &report.targets {
            let status = if target.enabled {
                style("enabled").green().to_string()
            } else {
                style("disabled").dim().to_string()
            };
            rows.push([target.name.clone(), status, target.method.clone()]);
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

    // Health
    let health = match &report.health {
        Ok(0) => format!("{}", style("All good").green()),
        Ok(n) => format!("{}", style(format!("{} issue(s)", n)).red()),
        Err(e) => {
            eprintln!("warning: could not check library health: {}", e);
            format!("{}", style("unknown").yellow())
        }
    };
    println!("{} {}", style("Health:").bold(), health);
}

/// Count skill entries in the library (directories or symlinks-to-dirs), excluding hidden entries.
fn count_entries(dir: &Path) -> Result<usize> {
    let mut count = 0;
    for entry in std::fs::read_dir(dir)
        .with_context(|| format!("failed to read directory {}", dir.display()))?
    {
        let entry = entry.with_context(|| format!("failed to read entry in {}", dir.display()))?;
        let name = entry.file_name().to_string_lossy().to_string();
        if name.starts_with('.') {
            continue;
        }
        let path = entry.path();
        // Count real directories (local skills) and symlinks-to-dirs (managed skills)
        if path.is_dir() {
            count += 1;
        }
    }
    Ok(count)
}

/// Count health issues: manifest/disk mismatches.
fn count_health_issues(dir: &Path, tome_home: &Path) -> Result<usize> {
    let m = manifest::load(tome_home)?;
    let mut issues = 0;

    // Check manifest entries exist on disk
    for name in m.keys() {
        if !dir.join(name.as_str()).is_dir() {
            issues += 1;
        }
    }

    // Second pass: orphan directories and broken symlinks
    for entry in std::fs::read_dir(dir)
        .with_context(|| format!("failed to read directory {}", dir.display()))?
    {
        let entry = entry.with_context(|| format!("failed to read entry in {}", dir.display()))?;
        let path = entry.path();
        let name = entry.file_name().to_string_lossy().to_string();

        if path.is_dir() && !name.starts_with('.') && !m.contains_key(&name) {
            issues += 1; // orphan
        }
        if path.is_symlink() && !path.exists() && !m.contains_key(&name) {
            issues += 1; // broken symlink (not already counted via manifest check)
        }
    }

    Ok(issues)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;
    use std::path::PathBuf;

    // -- gather() tests --

    #[test]
    fn gather_unconfigured_returns_not_configured() {
        let config = Config {
            library_dir: PathBuf::from("/nonexistent/tome/library"),
            ..Config::default()
        };

        let report = gather(
            &config,
            &TomePaths::new(config.library_dir.clone(), config.library_dir.clone()).unwrap(),
        )
        .unwrap();
        assert!(!report.configured);
        assert!(report.sources.is_empty());
        assert!(report.targets.is_empty());
    }

    #[test]
    fn gather_with_sources_marks_configured() {
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

        let report = gather(
            &config,
            &TomePaths::new(config.library_dir.clone(), config.library_dir.clone()).unwrap(),
        )
        .unwrap();
        assert!(report.configured);
        assert_eq!(report.sources.len(), 1);
        assert_eq!(report.sources[0].name, "test");
        // Source path doesn't exist — discover_source returns Ok(empty) with a warning
        assert_eq!(report.sources[0].skill_count.as_ref().copied().unwrap(), 0);
    }

    #[test]
    fn gather_with_library_dir_counts_skills() {
        let lib_dir = tempfile::TempDir::new().unwrap();
        std::fs::create_dir_all(lib_dir.path().join("skill-a")).unwrap();
        std::fs::create_dir_all(lib_dir.path().join("skill-b")).unwrap();

        let config = Config {
            library_dir: lib_dir.path().to_path_buf(),
            ..Config::default()
        };

        let report = gather(
            &config,
            &TomePaths::new(config.library_dir.clone(), config.library_dir.clone()).unwrap(),
        )
        .unwrap();
        assert!(report.configured);
        assert_eq!(report.library_count.unwrap(), 2);
    }

    #[test]
    fn gather_with_targets_populates_target_status() {
        use crate::config::{TargetConfig, TargetMethod, TargetName};
        use std::collections::BTreeMap;

        let lib_dir = tempfile::TempDir::new().unwrap();
        let target_dir = tempfile::TempDir::new().unwrap();

        let config = Config {
            library_dir: lib_dir.path().to_path_buf(),
            targets: BTreeMap::from([(
                TargetName::new("claude").unwrap(),
                TargetConfig {
                    enabled: true,
                    method: TargetMethod::Symlink {
                        skills_dir: target_dir.path().to_path_buf(),
                    },
                },
            )]),
            ..Config::default()
        };

        let report = gather(
            &config,
            &TomePaths::new(config.library_dir.clone(), config.library_dir.clone()).unwrap(),
        )
        .unwrap();
        assert_eq!(report.targets.len(), 1);
        assert_eq!(report.targets[0].name, "claude");
        assert!(report.targets[0].enabled);
        assert_eq!(report.targets[0].method, "symlink");
    }

    #[test]
    fn gather_health_detects_orphan() {
        let lib_dir = tempfile::TempDir::new().unwrap();
        std::fs::create_dir_all(lib_dir.path().join("orphan-skill")).unwrap();

        let config = Config {
            library_dir: lib_dir.path().to_path_buf(),
            ..Config::default()
        };

        let report = gather(
            &config,
            &TomePaths::new(config.library_dir.clone(), config.library_dir.clone()).unwrap(),
        )
        .unwrap();
        assert_eq!(report.health.unwrap(), 1);
    }

    // -- Legacy tests (now calling show(), which delegates to gather() + render) --

    #[test]
    fn status_shows_init_prompt_when_unconfigured() {
        let config = Config {
            library_dir: PathBuf::from("/nonexistent/tome/library"),
            ..Config::default()
        };

        let report = gather(
            &config,
            &TomePaths::new(config.library_dir.clone(), config.library_dir.clone()).unwrap(),
        )
        .unwrap();
        assert!(!report.configured);
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

        let report = gather(
            &config,
            &TomePaths::new(config.library_dir.clone(), config.library_dir.clone()).unwrap(),
        )
        .unwrap();
        assert!(report.configured);
        assert_eq!(report.sources.len(), 1);
    }

    #[test]
    fn status_shows_tables_with_configured_sources_and_targets() {
        use crate::config::{Source, SourceType, TargetConfig, TargetMethod, TargetName};
        use std::collections::BTreeMap;

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
            targets: BTreeMap::from([(
                TargetName::new("antigravity").unwrap(),
                TargetConfig {
                    enabled: true,
                    method: TargetMethod::Symlink {
                        skills_dir: target_skill.path().to_path_buf(),
                    },
                },
            )]),
            ..Config::default()
        };

        let report = gather(
            &config,
            &TomePaths::new(config.library_dir.clone(), config.library_dir.clone()).unwrap(),
        )
        .unwrap();
        assert!(report.configured);
        assert_eq!(report.sources.len(), 1);
        assert_eq!(report.targets.len(), 1);
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
    fn count_entries_ignores_hidden_directories() {
        let dir = tempfile::TempDir::new().unwrap();

        // Visible skill dir — should be counted
        std::fs::create_dir_all(dir.path().join("my-skill")).unwrap();
        // Hidden dirs — should NOT be counted
        std::fs::create_dir_all(dir.path().join(".git")).unwrap();
        std::fs::create_dir_all(dir.path().join(".hidden")).unwrap();

        assert_eq!(count_entries(dir.path()).unwrap(), 1);
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
    fn count_health_issues_uses_tome_home() {
        let tome_home = tempfile::TempDir::new().unwrap();
        let library = tempfile::TempDir::new().unwrap();

        // Create a skill directory in the library
        std::fs::create_dir_all(library.path().join("my-skill")).unwrap();

        // Save manifest at tome_home (not library_dir)
        let mut m = manifest::Manifest::default();
        m.insert(
            crate::discover::SkillName::new("my-skill").unwrap(),
            manifest::SkillEntry {
                source_path: PathBuf::from("/tmp/source/my-skill"),
                source_name: "test".to_string(),
                content_hash: crate::validation::test_hash("abc"),
                synced_at: "2024-01-01T00:00:00Z".to_string(),
                managed: false,
            },
        );
        manifest::save(&m, tome_home.path()).unwrap();

        // Should find 0 issues when manifest is at tome_home
        assert_eq!(
            count_health_issues(library.path(), tome_home.path()).unwrap(),
            0,
            "should find no issues when manifest at tome_home matches library contents"
        );

        // Should find 1 orphan when using library_dir as tome_home (no manifest there)
        assert_eq!(
            count_health_issues(library.path(), library.path()).unwrap(),
            1,
            "should detect orphan when manifest is not at the given tome_home"
        );
    }

    #[test]
    fn count_health_issues_empty_dir() {
        let dir = tempfile::TempDir::new().unwrap();
        assert_eq!(count_health_issues(dir.path(), dir.path()).unwrap(), 0);
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
                content_hash: crate::validation::test_hash("abc"),
                synced_at: "2024-01-01T00:00:00Z".to_string(),
                managed: false,
            },
        );
        manifest::save(&m, dir.path()).unwrap();

        assert_eq!(count_health_issues(dir.path(), dir.path()).unwrap(), 1);
    }

    #[test]
    fn count_health_issues_detects_orphan_directory() {
        let dir = tempfile::TempDir::new().unwrap();

        // Create a directory not tracked by manifest
        std::fs::create_dir_all(dir.path().join("orphan-skill")).unwrap();

        assert_eq!(count_health_issues(dir.path(), dir.path()).unwrap(), 1);
    }

    #[test]
    fn count_health_issues_no_double_count_broken_managed_symlink() {
        use std::os::unix::fs as unix_fs;

        let dir = tempfile::TempDir::new().unwrap();

        // Create a managed skill manifest entry pointing to a non-existent source
        let mut m = manifest::Manifest::default();
        m.insert(
            crate::discover::SkillName::new("managed-skill").unwrap(),
            manifest::SkillEntry {
                source_path: PathBuf::from("/tmp/source"),
                source_name: "plugins".to_string(),
                content_hash: crate::validation::test_hash("abc"),
                synced_at: "2024-01-01T00:00:00Z".to_string(),
                managed: true,
            },
        );
        manifest::save(&m, dir.path()).unwrap();

        // Create a broken symlink (managed skill whose source is gone)
        unix_fs::symlink("/nonexistent/source", dir.path().join("managed-skill")).unwrap();

        // Should count exactly 1 issue (manifest-vs-disk), not 2
        assert_eq!(count_health_issues(dir.path(), dir.path()).unwrap(), 1);
    }

    #[test]
    fn count_health_issues_ignores_hidden_dirs() {
        let dir = tempfile::TempDir::new().unwrap();

        // .git dir should not be counted as an orphan
        std::fs::create_dir_all(dir.path().join(".git")).unwrap();

        assert_eq!(count_health_issues(dir.path(), dir.path()).unwrap(), 0);
    }
}
