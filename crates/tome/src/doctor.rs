//! Diagnose and optionally repair issues such as missing entries, orphan directories,
//! and stale target symlinks.

use anyhow::{Context, Result};
use console::style;
use dialoguer::Confirm;
use std::io::IsTerminal;
use std::path::Path;

use crate::cleanup;
use crate::config::Config;
use crate::manifest;
use crate::paths::{TomePaths, resolve_symlink_target};

// -- Data structs --

/// Severity of a diagnostic issue.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IssueSeverity {
    /// Critical problem (e.g., missing directory, broken symlink).
    Error,
    /// Non-critical problem (e.g., orphan directory, missing source path).
    Warning,
}

/// A single diagnostic issue found during a health check.
#[derive(Debug, Clone)]
pub struct DiagnosticIssue {
    pub severity: IssueSeverity,
    pub message: String,
}

/// Complete diagnostic report for the tome system.
#[derive(Debug)]
pub struct DoctorReport {
    pub configured: bool,
    pub library_issues: Vec<DiagnosticIssue>,
    pub target_issues: Vec<(String, Vec<DiagnosticIssue>)>,
    pub config_issues: Vec<DiagnosticIssue>,
}

impl DoctorReport {
    pub fn total_issues(&self) -> usize {
        self.library_issues.len()
            + self
                .target_issues
                .iter()
                .map(|(_, v)| v.len())
                .sum::<usize>()
            + self.config_issues.len()
    }
}

// -- Data gathering (pure computation, no I/O) --

/// Run all diagnostic checks and return a structured report.
pub fn check(config: &Config, paths: &TomePaths) -> Result<DoctorReport> {
    let configured = paths.library_dir().is_dir() || !config.sources.is_empty();

    if !configured {
        return Ok(DoctorReport {
            configured: false,
            library_issues: Vec::new(),
            target_issues: Vec::new(),
            config_issues: Vec::new(),
        });
    }

    let library_issues = check_library(paths)?;

    let mut target_issues = Vec::new();
    for (name, t) in config.targets.iter() {
        if t.enabled {
            let issues = check_target_dir(name.as_str(), t.skills_dir(), paths.library_dir())?;
            target_issues.push((name.as_str().to_string(), issues));
        }
    }

    let config_issues = check_config(config)?;

    Ok(DoctorReport {
        configured: true,
        library_issues,
        target_issues,
        config_issues,
    })
}

// -- Rendering + control flow --

/// Diagnose and optionally repair issues.
pub fn diagnose(config: &Config, paths: &TomePaths, dry_run: bool) -> Result<()> {
    let report = check(config, paths)?;

    if !report.configured {
        println!("Not configured yet. Run `tome init` to get started.");
        return Ok(());
    }

    if dry_run {
        eprintln!(
            "{}",
            style("[dry-run] No changes will be made").yellow().bold()
        );
    }

    // Render results
    println!("{}", style("Checking library...").bold());
    render_issues(&report.library_issues, "library");

    println!("{}", style("Checking targets...").bold());
    for (name, issues) in &report.target_issues {
        render_issues_for_target(name, issues);
    }

    println!("{}", style("Checking config...").bold());
    render_issues(&report.config_issues, "config");

    let total = report.total_issues();

    println!();
    if total == 0 {
        println!("{}", style("No issues found.").green().bold());
    } else {
        println!(
            "{}",
            style(format!("Found {} issue(s).", total)).yellow().bold()
        );

        if !dry_run {
            let confirmed = if std::io::stdin().is_terminal() {
                Confirm::new()
                    .with_prompt("Repair these issues?")
                    .default(true)
                    .interact()?
            } else {
                eprintln!("info: non-interactive mode — skipping repair prompt");
                false
            };

            if confirmed {
                println!();
                println!("{}", style("Repairing...").bold());
                repair_library(paths)?;

                for (name, t) in config.targets.iter() {
                    if t.enabled {
                        let removed =
                            cleanup::cleanup_target(t.skills_dir(), paths.library_dir(), false)?;
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

fn render_issues(issues: &[DiagnosticIssue], section: &str) {
    if issues.is_empty() {
        println!("  {} {} OK", style("ok").green(), section);
    } else {
        for issue in issues {
            let marker = match issue.severity {
                IssueSeverity::Error => style("x").red(),
                IssueSeverity::Warning => style("!").yellow(),
            };
            println!("  {} {}", marker, issue.message);
        }
    }
}

fn render_issues_for_target(name: &str, issues: &[DiagnosticIssue]) {
    if issues.is_empty() {
        println!("  {} {}: OK", style("ok").green(), name);
    } else {
        for issue in issues {
            let marker = match issue.severity {
                IssueSeverity::Error => style("x").red(),
                IssueSeverity::Warning => style("!").yellow(),
            };
            println!("  {} {}: {}", marker, name, issue.message);
        }
    }
}

// -- Check functions (return structured data) --

fn check_library(paths: &TomePaths) -> Result<Vec<DiagnosticIssue>> {
    let library_dir = paths.library_dir();
    let tome_home = paths.tome_home();
    let mut issues = Vec::new();

    if !library_dir.is_dir() {
        issues.push(DiagnosticIssue {
            severity: IssueSeverity::Warning,
            message: "library directory does not exist".to_string(),
        });
        return Ok(issues);
    }

    let m = match manifest::load(tome_home) {
        Ok(m) => m,
        Err(e) => {
            issues.push(DiagnosticIssue {
                severity: IssueSeverity::Error,
                message: format!("manifest is corrupted or unreadable: {}", e),
            });
            return Ok(issues);
        }
    };

    // Check manifest entries exist on disk
    for name in m.keys() {
        let entry_path = library_dir.join(name.as_str());
        if !entry_path.is_dir() {
            let entry = m.get(name.as_str());
            let is_managed = entry.is_some_and(|e| e.managed);
            if is_managed && entry_path.is_symlink() {
                issues.push(DiagnosticIssue {
                    severity: IssueSeverity::Error,
                    message: format!(
                        "managed skill '{}' has a broken symlink (source may have been uninstalled)",
                        name
                    ),
                });
            } else {
                issues.push(DiagnosticIssue {
                    severity: IssueSeverity::Error,
                    message: format!("manifest entry '{}' has no directory on disk", name),
                });
            }
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
            issues.push(DiagnosticIssue {
                severity: IssueSeverity::Warning,
                message: format!("orphan directory: {} (not in manifest)", path.display()),
            });
        }

        // Check for broken symlinks — either managed skills or legacy v0.1.x
        if path.is_symlink() && !path.exists() {
            let is_managed = m.get(&name).is_some_and(|e| e.managed);
            if !is_managed {
                let raw_target = std::fs::read_link(&path)
                    .with_context(|| format!("failed to read symlink {}", path.display()))?;
                issues.push(DiagnosticIssue {
                    severity: IssueSeverity::Error,
                    message: format!(
                        "broken legacy symlink: {} -> {}",
                        path.display(),
                        raw_target.display()
                    ),
                });
            }
        }
    }

    Ok(issues)
}

fn check_target_dir(
    _name: &str,
    skills_dir: &Path,
    library_dir: &Path,
) -> Result<Vec<DiagnosticIssue>> {
    let mut issues = Vec::new();

    if !skills_dir.is_dir() {
        issues.push(DiagnosticIssue {
            severity: IssueSeverity::Warning,
            message: format!("target directory does not exist ({})", skills_dir.display()),
        });
        return Ok(issues);
    }

    // Canonicalize library_dir so starts_with works when library_dir contains
    // a symlink component (e.g., /var -> /private/var on macOS).
    let canonical_library = std::fs::canonicalize(library_dir).unwrap_or_else(|e| {
        eprintln!(
            "warning: could not canonicalize library path {}: {}",
            library_dir.display(),
            e
        );
        library_dir.to_path_buf()
    });

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
                issues.push(DiagnosticIssue {
                    severity: IssueSeverity::Error,
                    message: format!("stale symlink {}", path.display()),
                });
            }
        }
    }

    Ok(issues)
}

fn check_config(config: &Config) -> Result<Vec<DiagnosticIssue>> {
    let mut issues = Vec::new();

    for source in &config.sources {
        if !source.path.exists() {
            issues.push(DiagnosticIssue {
                severity: IssueSeverity::Warning,
                message: format!(
                    "source '{}' path does not exist: {}",
                    source.name,
                    source.path.display()
                ),
            });
        }
    }

    Ok(issues)
}

/// Repair library issues: remove orphan manifest entries and broken symlinks.
fn repair_library(paths: &TomePaths) -> Result<()> {
    let library_dir = paths.library_dir();
    let tome_home = paths.tome_home();
    let mut m = manifest::load(tome_home).with_context(|| {
        "cannot repair: manifest is unreadable. Back up .tome-manifest.json and run sync --force"
    })?;
    let mut fixed = 0;

    // Remove manifest entries missing from disk (includes managed broken symlinks)
    let missing: Vec<String> = m
        .keys()
        .filter(|name| !library_dir.join(name.as_str()).is_dir())
        .map(|name| name.as_str().to_string())
        .collect();
    for name in &missing {
        let entry_path = library_dir.join(name.as_str());
        // Clean up broken managed symlinks
        if entry_path.is_symlink() {
            std::fs::remove_file(&entry_path).with_context(|| {
                format!("failed to remove broken symlink {}", entry_path.display())
            })?;
        }
        m.remove(name);
        println!(
            "  {} Removed manifest entry '{}' (directory missing)",
            style("fixed").green(),
            name
        );
        fixed += 1;
    }

    // Remove broken legacy symlinks (not in manifest)
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
        manifest::save(&m, tome_home)?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{Config, Source, SourceType};
    use std::os::unix::fs as unix_fs;
    use std::path::PathBuf;
    use tempfile::TempDir;

    // -- check() tests --

    #[test]
    fn check_unconfigured_returns_not_configured() {
        let config = Config {
            library_dir: PathBuf::from("/nonexistent/library"),
            ..Config::default()
        };

        let tmp = TempDir::new().unwrap();
        let report = check(
            &config,
            &TomePaths::new(tmp.path().to_path_buf(), config.library_dir.clone()).unwrap(),
        )
        .unwrap();
        assert!(!report.configured);
        assert_eq!(report.total_issues(), 0);
    }

    #[test]
    fn check_healthy_library_returns_no_issues() {
        let lib = TempDir::new().unwrap();
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
                managed: false,
            },
        );
        manifest::save(&m, lib.path()).unwrap();

        let config = Config {
            library_dir: lib.path().to_path_buf(),
            ..Config::default()
        };

        let report = check(
            &config,
            &TomePaths::new(lib.path().to_path_buf(), config.library_dir.clone()).unwrap(),
        )
        .unwrap();
        assert!(report.configured);
        assert_eq!(report.total_issues(), 0);
    }

    #[test]
    fn check_detects_orphan_directory() {
        let lib = TempDir::new().unwrap();
        std::fs::create_dir_all(lib.path().join("orphan")).unwrap();

        let config = Config {
            library_dir: lib.path().to_path_buf(),
            ..Config::default()
        };

        let report = check(
            &config,
            &TomePaths::new(lib.path().to_path_buf(), config.library_dir.clone()).unwrap(),
        )
        .unwrap();
        assert_eq!(report.library_issues.len(), 1);
        assert_eq!(report.library_issues[0].severity, IssueSeverity::Warning);
        assert!(report.library_issues[0].message.contains("orphan"));
    }

    #[test]
    fn check_detects_missing_source_path() {
        let lib = TempDir::new().unwrap();

        let config = Config {
            library_dir: lib.path().to_path_buf(),
            sources: vec![Source {
                name: "gone".to_string(),
                path: PathBuf::from("/nonexistent/source"),
                source_type: SourceType::Directory,
            }],
            ..Config::default()
        };

        let report = check(
            &config,
            &TomePaths::new(lib.path().to_path_buf(), config.library_dir.clone()).unwrap(),
        )
        .unwrap();
        assert_eq!(report.config_issues.len(), 1);
        assert!(report.config_issues[0].message.contains("gone"));
    }

    // -- check_library --

    #[test]
    fn check_library_missing_dir() {
        let tmp = TempDir::new().unwrap();
        let result = check_library(
            &TomePaths::new(
                tmp.path().to_path_buf(),
                Path::new("/nonexistent/library").to_path_buf(),
            )
            .unwrap(),
        )
        .unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].severity, IssueSeverity::Warning);
    }

    #[test]
    fn check_library_no_issues() {
        let lib = TempDir::new().unwrap();
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
                managed: false,
            },
        );
        manifest::save(&m, lib.path()).unwrap();

        let result = check_library(
            &TomePaths::new(lib.path().to_path_buf(), lib.path().to_path_buf()).unwrap(),
        )
        .unwrap();
        assert!(result.is_empty());
    }

    #[test]
    fn check_library_missing_manifest_entry() {
        let lib = TempDir::new().unwrap();

        let mut m = manifest::Manifest::default();
        m.insert(
            crate::discover::SkillName::new("gone").unwrap(),
            manifest::SkillEntry {
                source_path: PathBuf::from("/tmp/source/gone"),
                source_name: "test".to_string(),
                content_hash: "abc".to_string(),
                synced_at: "2024-01-01T00:00:00Z".to_string(),
                managed: false,
            },
        );
        manifest::save(&m, lib.path()).unwrap();

        let result = check_library(
            &TomePaths::new(lib.path().to_path_buf(), lib.path().to_path_buf()).unwrap(),
        )
        .unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].severity, IssueSeverity::Error);
    }

    #[test]
    fn check_library_orphan_directory() {
        let lib = TempDir::new().unwrap();
        std::fs::create_dir_all(lib.path().join("orphan")).unwrap();

        let result = check_library(
            &TomePaths::new(lib.path().to_path_buf(), lib.path().to_path_buf()).unwrap(),
        )
        .unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].severity, IssueSeverity::Warning);
    }

    #[test]
    fn check_library_broken_legacy_symlink() {
        let lib = TempDir::new().unwrap();
        unix_fs::symlink("/nonexistent/target", lib.path().join("broken")).unwrap();

        let result = check_library(
            &TomePaths::new(lib.path().to_path_buf(), lib.path().to_path_buf()).unwrap(),
        )
        .unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].severity, IssueSeverity::Error);
    }

    // -- check_target_dir --

    #[test]
    fn check_target_dir_missing_dir() {
        let lib = TempDir::new().unwrap();
        let result =
            check_target_dir("test-target", Path::new("/nonexistent/target"), lib.path()).unwrap();
        assert_eq!(result.len(), 1);
    }

    #[test]
    fn check_target_dir_stale_symlink() {
        let lib = TempDir::new().unwrap();
        let target_dir = TempDir::new().unwrap();

        let stale_target = lib.path().join("deleted-skill");
        unix_fs::symlink(&stale_target, target_dir.path().join("skill-link")).unwrap();

        let result = check_target_dir("test", target_dir.path(), lib.path()).unwrap();
        assert_eq!(result.len(), 1);
    }

    #[test]
    fn check_target_dir_ignores_external_symlinks() {
        let lib = TempDir::new().unwrap();
        let target_dir = TempDir::new().unwrap();

        unix_fs::symlink("/some/other/place", target_dir.path().join("external")).unwrap();

        let result = check_target_dir("test", target_dir.path(), lib.path()).unwrap();
        assert!(result.is_empty());
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
        assert_eq!(result.len(), 1);
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
        assert!(result.is_empty());
    }

    // -- diagnose (pre-init guard) --

    #[test]
    fn diagnose_shows_init_prompt_when_unconfigured() {
        let config = Config {
            library_dir: PathBuf::from("/nonexistent/library"),
            ..Config::default()
        };

        let tmp = TempDir::new().unwrap();
        let result = diagnose(
            &config,
            &TomePaths::new(tmp.path().to_path_buf(), config.library_dir.clone()).unwrap(),
            true,
        );
        assert!(result.is_ok());
    }

    // -- repair_library --

    #[test]
    fn check_library_uses_tome_home_for_manifest() {
        let tome_home = TempDir::new().unwrap();
        let library = TempDir::new().unwrap();

        // Create a skill directory in the library
        let skill_dir = library.path().join("my-skill");
        std::fs::create_dir_all(&skill_dir).unwrap();

        // Save manifest at tome_home (not library_dir)
        let mut m = manifest::Manifest::default();
        m.insert(
            crate::discover::SkillName::new("my-skill").unwrap(),
            manifest::SkillEntry {
                source_path: PathBuf::from("/tmp/source/my-skill"),
                source_name: "test".to_string(),
                content_hash: "abc".to_string(),
                synced_at: "2024-01-01T00:00:00Z".to_string(),
                managed: false,
            },
        );
        manifest::save(&m, tome_home.path()).unwrap();

        // check_library should read manifest from tome_home, not library_dir
        let issues = check_library(
            &TomePaths::new(tome_home.path().to_path_buf(), library.path().to_path_buf()).unwrap(),
        )
        .unwrap();
        assert!(
            issues.is_empty(),
            "should find no issues when manifest is at tome_home and skill exists in library"
        );

        // Verify it would fail if we pointed tome_home at the wrong place
        // (library_dir has no manifest, so it loads an empty one and sees an orphan)
        let issues = check_library(
            &TomePaths::new(library.path().to_path_buf(), library.path().to_path_buf()).unwrap(),
        )
        .unwrap();
        assert_eq!(
            issues.len(),
            1,
            "should detect orphan when manifest is not at the given tome_home"
        );
    }

    #[test]
    fn repair_library_uses_tome_home_for_manifest() {
        // Verify that repair_library reads the manifest from tome_home
        // and operates on the separate library_dir.
        let tome_home = TempDir::new().unwrap();
        let library = TempDir::new().unwrap();

        // Create a manifest at tome_home with an orphan entry (no dir in library)
        let mut m = manifest::Manifest::default();
        m.insert(
            crate::discover::SkillName::new("orphan-skill").unwrap(),
            manifest::SkillEntry {
                source_path: PathBuf::from("/tmp/source/orphan-skill"),
                source_name: "test".to_string(),
                content_hash: "abc".to_string(),
                synced_at: "2024-01-01T00:00:00Z".to_string(),
                managed: false,
            },
        );
        manifest::save(&m, tome_home.path()).unwrap();

        // Repair should read manifest from tome_home and check library_dir
        repair_library(
            &TomePaths::new(tome_home.path().to_path_buf(), library.path().to_path_buf()).unwrap(),
        )
        .unwrap();

        // The orphan entry should be removed from the manifest at tome_home
        let after = manifest::load(tome_home.path()).unwrap();
        assert!(
            !after.contains_key("orphan-skill"),
            "repair should remove orphan manifest entry when using separate tome_home"
        );
    }

    #[test]
    fn repair_library_removes_orphan_manifest_entry() {
        let lib = TempDir::new().unwrap();

        // Create a manifest entry with no corresponding directory
        let mut m = manifest::Manifest::default();
        m.insert(
            crate::discover::SkillName::new("ghost").unwrap(),
            manifest::SkillEntry {
                source_path: PathBuf::from("/tmp/source/ghost"),
                source_name: "test".to_string(),
                content_hash: "abc".to_string(),
                synced_at: "2024-01-01T00:00:00Z".to_string(),
                managed: false,
            },
        );
        manifest::save(&m, lib.path()).unwrap();

        repair_library(
            &TomePaths::new(lib.path().to_path_buf(), lib.path().to_path_buf()).unwrap(),
        )
        .unwrap();

        let after = manifest::load(lib.path()).unwrap();
        assert!(
            !after.contains_key("ghost"),
            "repair should remove manifest entry without directory"
        );
    }

    #[test]
    fn repair_library_removes_broken_managed_symlink() {
        let lib = TempDir::new().unwrap();

        // Create a broken managed symlink + manifest entry
        unix_fs::symlink("/nonexistent/source", lib.path().join("broken-plugin")).unwrap();
        let mut m = manifest::Manifest::default();
        m.insert(
            crate::discover::SkillName::new("broken-plugin").unwrap(),
            manifest::SkillEntry {
                source_path: PathBuf::from("/nonexistent/source"),
                source_name: "plugins".to_string(),
                content_hash: "abc".to_string(),
                synced_at: "2024-01-01T00:00:00Z".to_string(),
                managed: true,
            },
        );
        manifest::save(&m, lib.path()).unwrap();

        repair_library(
            &TomePaths::new(lib.path().to_path_buf(), lib.path().to_path_buf()).unwrap(),
        )
        .unwrap();

        assert!(
            !lib.path().join("broken-plugin").exists(),
            "broken managed symlink should be removed"
        );
        let after = manifest::load(lib.path()).unwrap();
        assert!(!after.contains_key("broken-plugin"));
    }

    #[test]
    fn repair_library_removes_broken_legacy_symlink() {
        let lib = TempDir::new().unwrap();

        // Broken legacy symlink (not in manifest)
        unix_fs::symlink("/nonexistent/v01/skill", lib.path().join("legacy")).unwrap();

        repair_library(
            &TomePaths::new(lib.path().to_path_buf(), lib.path().to_path_buf()).unwrap(),
        )
        .unwrap();

        assert!(
            !lib.path().join("legacy").exists(),
            "broken legacy symlink should be removed"
        );
    }

    #[test]
    fn repair_library_healthy_is_noop() {
        let lib = TempDir::new().unwrap();
        let skill_dir = lib.path().join("healthy-skill");
        std::fs::create_dir_all(&skill_dir).unwrap();

        let mut m = manifest::Manifest::default();
        m.insert(
            crate::discover::SkillName::new("healthy-skill").unwrap(),
            manifest::SkillEntry {
                source_path: PathBuf::from("/tmp/source/healthy-skill"),
                source_name: "test".to_string(),
                content_hash: "abc".to_string(),
                synced_at: "2024-01-01T00:00:00Z".to_string(),
                managed: false,
            },
        );
        manifest::save(&m, lib.path()).unwrap();

        repair_library(
            &TomePaths::new(lib.path().to_path_buf(), lib.path().to_path_buf()).unwrap(),
        )
        .unwrap();

        let after = manifest::load(lib.path()).unwrap();
        assert!(after.contains_key("healthy-skill"));
        assert!(skill_dir.exists());
    }
}
