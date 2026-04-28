//! Read-only summary of the library state, configured directories, and overall health.

use anyhow::{Context, Result};
use console::style;
use std::path::{Path, PathBuf};
use tabled::settings::{Modify, Style, object::Rows};

use crate::config::Config;
use crate::manifest;
use crate::paths::TomePaths;

// -- Data structs --

/// A count that may have failed with an error message.
#[derive(serde::Serialize)]
pub struct CountOrError {
    pub count: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

impl From<Result<usize, String>> for CountOrError {
    fn from(result: Result<usize, String>) -> Self {
        match result {
            Ok(n) => Self {
                count: Some(n),
                error: None,
            },
            Err(e) => Self {
                count: None,
                error: Some(e),
            },
        }
    }
}

/// Status of a single configured directory.
#[derive(serde::Serialize)]
pub struct DirectoryStatus {
    pub name: String,
    pub directory_type: String,
    pub role: String,
    pub path: String,
    /// Number of skills discovered (for discovery dirs) or symlinks present (for target dirs),
    /// or an error message if counting failed.
    pub skill_count: CountOrError,
    /// Warnings emitted during discovery.
    pub warnings: Vec<String>,
}

/// Complete status report for the tome system.
#[derive(serde::Serialize)]
pub struct StatusReport {
    pub configured: bool,
    pub library_dir: PathBuf,
    /// Number of skills consolidated in the library, or an error message.
    pub library_count: CountOrError,
    pub directories: Vec<DirectoryStatus>,
    /// Number of health issues, or an error message.
    pub health: CountOrError,
}

// -- Data gathering (pure computation, no I/O) --

/// Gather status data without producing any output.
pub fn gather(config: &Config, paths: &TomePaths) -> Result<StatusReport> {
    let configured = paths.library_dir().is_dir() || !config.directories.is_empty();

    let library_count = if paths.library_dir().is_dir() {
        count_entries(paths.library_dir()).map_err(|e| e.to_string())
    } else {
        Ok(0)
    };

    let directories: Vec<DirectoryStatus> = config
        .directories
        .iter()
        .map(|(name, dir_config)| {
            let role = dir_config.role();
            let skill_count = if role.is_discovery() {
                // For discovery directories, count SKILL.md subdirs
                count_skill_dirs(&dir_config.path).map_err(|e| e.to_string())
            } else {
                // For target-only directories, count existing symlinks
                count_symlinks(&dir_config.path).map_err(|e| e.to_string())
            };
            let warnings = Vec::new();
            DirectoryStatus {
                name: name.as_str().to_string(),
                directory_type: dir_config.directory_type.to_string(),
                role: role.description().to_string(),
                path: dir_config.path.display().to_string(),
                skill_count: skill_count.into(),
                warnings,
            }
        })
        .collect();

    let health = if paths.library_dir().is_dir() {
        count_health_issues(paths.library_dir(), paths.config_dir()).map_err(|e| e.to_string())
    } else {
        Ok(0)
    };

    Ok(StatusReport {
        configured,
        library_dir: paths.library_dir().to_path_buf(),
        library_count: library_count.into(),
        directories,
        health: health.into(),
    })
}

// -- Rendering --

/// Display the current status of the tome system.
pub fn show(config: &Config, paths: &TomePaths, json: bool) -> Result<()> {
    let report = gather(config, paths)?;
    if json {
        println!("{}", serde_json::to_string_pretty(&report)?);
    } else {
        render_status(&report);
    }
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
        crate::paths::collapse_home(&report.library_dir)
    );
    let (lib_count, lib_indicator) =
        match (&report.library_count.count, &report.library_count.error) {
            (Some(n), _) => (format!("{}", n), style("✓").green()),
            (None, Some(e)) => {
                eprintln!("warning: could not read library: {}", e);
                ("?".to_string(), style("✗").red())
            }
            (None, None) => ("0".to_string(), style("✓").green()),
        };
    println!(
        "  {} {} skills consolidated",
        lib_indicator,
        style(lib_count).cyan()
    );
    println!();

    // Directories
    println!("{}", style("Directories:").bold());
    if report.directories.is_empty() {
        println!("  (none configured)");
    } else {
        let mut rows: Vec<[String; 5]> = Vec::with_capacity(report.directories.len() + 1);
        rows.push([
            "NAME".to_string(),
            "TYPE".to_string(),
            "ROLE".to_string(),
            "PATH".to_string(),
            "SKILLS".to_string(),
        ]);
        for dir in &report.directories {
            let count = match (&dir.skill_count.count, &dir.skill_count.error) {
                (Some(n), _) => format!("✓ {}", n),
                (None, Some(e)) => {
                    eprintln!("warning: could not count skills in '{}': {}", dir.name, e);
                    "✗ ?".to_string()
                }
                (None, None) => "✓ 0".to_string(),
            };
            rows.push([
                dir.name.clone(),
                dir.directory_type.clone(),
                dir.role.clone(),
                crate::paths::collapse_home(std::path::Path::new(&dir.path)),
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
        for dir in &report.directories {
            for w in &dir.warnings {
                eprintln!("warning: {}", w);
            }
        }
    }
    println!();

    // Health
    let health = match (&report.health.count, &report.health.error) {
        (Some(0), _) => format!("{} {}", style("✓").green(), style("All good").green()),
        (Some(n), _) => format!(
            "{} {}",
            style("⚠").yellow(),
            style(format!("{} issue(s) — run `tome doctor` for details", n)).yellow()
        ),
        (None, Some(e)) => {
            eprintln!("warning: could not check library health: {}", e);
            format!("{} {}", style("✗").red(), style("unknown").red())
        }
        (None, None) => format!("{} {}", style("✓").green(), style("All good").green()),
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

/// Count subdirectories that look like skills (contain SKILL.md or are directories).
fn count_skill_dirs(dir: &Path) -> Result<usize> {
    if !dir.is_dir() {
        return Ok(0);
    }
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
        if path.is_dir() {
            count += 1;
        }
    }
    Ok(count)
}

/// Count symlinks in a directory (for target-only directories).
fn count_symlinks(dir: &Path) -> Result<usize> {
    if !dir.is_dir() {
        return Ok(0);
    }
    let mut count = 0;
    for entry in std::fs::read_dir(dir)
        .with_context(|| format!("failed to read directory {}", dir.display()))?
    {
        let entry = entry.with_context(|| format!("failed to read entry in {}", dir.display()))?;
        let path = entry.path();
        if path.is_symlink() {
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
    use crate::config::{Config, DirectoryConfig, DirectoryName, DirectoryRole, DirectoryType};
    use std::collections::BTreeMap;
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
        assert!(report.directories.is_empty());
    }

    #[test]
    fn gather_with_directories_marks_configured() {
        let config = Config {
            library_dir: PathBuf::from("/nonexistent/tome/library"),
            directories: BTreeMap::from([(
                DirectoryName::new("test").unwrap(),
                DirectoryConfig {
                    path: PathBuf::from("/nonexistent/source"),
                    directory_type: DirectoryType::Directory,
                    role: Some(DirectoryRole::Source),
                    branch: None,
                    tag: None,
                    rev: None,

                    subdir: None,
                    override_applied: false,
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
        assert_eq!(report.directories.len(), 1);
        assert_eq!(report.directories[0].name, "test");
        // Source path doesn't exist — count_skill_dirs returns Ok(0)
        assert_eq!(report.directories[0].skill_count.count, Some(0));
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
        assert_eq!(report.library_count.count, Some(2));
    }

    #[test]
    fn gather_with_target_directory_populates_status() {
        let lib_dir = tempfile::TempDir::new().unwrap();
        let target_dir = tempfile::TempDir::new().unwrap();

        let config = Config {
            library_dir: lib_dir.path().to_path_buf(),
            directories: BTreeMap::from([(
                DirectoryName::new("claude").unwrap(),
                DirectoryConfig {
                    path: target_dir.path().to_path_buf(),
                    directory_type: DirectoryType::Directory,
                    role: Some(DirectoryRole::Target),
                    branch: None,
                    tag: None,
                    rev: None,

                    subdir: None,
                    override_applied: false,
                },
            )]),
            ..Config::default()
        };

        let report = gather(
            &config,
            &TomePaths::new(config.library_dir.clone(), config.library_dir.clone()).unwrap(),
        )
        .unwrap();
        assert_eq!(report.directories.len(), 1);
        assert_eq!(report.directories[0].name, "claude");
        assert!(report.directories[0].role.contains("Target"));
    }

    #[test]
    fn gather_directory_status_includes_role_description() {
        let lib_dir = tempfile::TempDir::new().unwrap();

        let config = Config {
            library_dir: lib_dir.path().to_path_buf(),
            directories: BTreeMap::from([(
                DirectoryName::new("my-skills").unwrap(),
                DirectoryConfig {
                    path: lib_dir.path().to_path_buf(),
                    directory_type: DirectoryType::Directory,
                    role: Some(DirectoryRole::Synced),
                    branch: None,
                    tag: None,
                    rev: None,

                    subdir: None,
                    override_applied: false,
                },
            )]),
            ..Config::default()
        };

        let report = gather(
            &config,
            &TomePaths::new(config.library_dir.clone(), config.library_dir.clone()).unwrap(),
        )
        .unwrap();
        assert_eq!(report.directories.len(), 1);
        assert!(
            report.directories[0].role.contains("Synced"),
            "role should contain Synced, got: {}",
            report.directories[0].role
        );
        assert!(
            report.directories[0]
                .role
                .contains("discovered here AND distributed here"),
            "role should include description, got: {}",
            report.directories[0].role
        );
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
        assert_eq!(report.health.count, Some(1));
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

    // -- count_skill_dirs --

    #[test]
    fn count_skill_dirs_nonexistent_returns_zero() {
        assert_eq!(count_skill_dirs(Path::new("/nonexistent/dir")).unwrap(), 0);
    }

    #[test]
    fn count_skill_dirs_counts_subdirs() {
        let dir = tempfile::TempDir::new().unwrap();
        std::fs::create_dir_all(dir.path().join("skill-a")).unwrap();
        std::fs::create_dir_all(dir.path().join("skill-b")).unwrap();
        std::fs::write(dir.path().join("not-a-skill.txt"), "").unwrap();

        assert_eq!(count_skill_dirs(dir.path()).unwrap(), 2);
    }

    // -- count_symlinks --

    #[test]
    fn count_symlinks_nonexistent_returns_zero() {
        assert_eq!(count_symlinks(Path::new("/nonexistent/dir")).unwrap(), 0);
    }

    #[test]
    fn count_symlinks_counts_only_symlinks() {
        use std::os::unix::fs as unix_fs;

        let dir = tempfile::TempDir::new().unwrap();
        let target = tempfile::TempDir::new().unwrap();

        unix_fs::symlink(target.path(), dir.path().join("linked")).unwrap();
        std::fs::create_dir_all(dir.path().join("real-dir")).unwrap();
        std::fs::write(dir.path().join("file.txt"), "").unwrap();

        assert_eq!(count_symlinks(dir.path()).unwrap(), 1);
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
