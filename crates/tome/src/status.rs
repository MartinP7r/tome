//! Read-only summary of the library state, configured sources, targets, and overall health.

use anyhow::{Context, Result};
use console::style;
use std::path::Path;

use crate::config::Config;
use crate::discover;

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
            println!(
                "  {:<40} {} skills",
                style(source.path.display()).dim(),
                style(count).cyan()
            );
        }
    }
    println!();

    // Targets
    println!("{}", style("Targets:").bold());
    let mut any_target = false;
    for (name, t) in config.targets.iter() {
        any_target = true;
        let status = if t.enabled {
            style("enabled").green()
        } else {
            style("disabled").dim()
        };
        println!("  {:<20} {}", style(name).bold(), status);
    }
    if !any_target {
        println!("  (none configured)");
    }
    println!();

    // Health check
    let health = match count_broken_symlinks(&config.library_dir) {
        Ok(0) => format!("{}", style("All good").green()),
        Ok(n) => format!("{}", style(format!("{} broken symlinks", n)).red()),
        Err(e) => {
            eprintln!("warning: could not check library health: {}", e);
            format!("{}", style("unknown").yellow())
        }
    };
    println!("{} {}", style("Health:").bold(), health);

    Ok(())
}

fn count_entries(dir: &Path) -> Result<usize> {
    let mut count = 0;
    for entry in std::fs::read_dir(dir)
        .with_context(|| format!("failed to read directory {}", dir.display()))?
    {
        let entry = entry.with_context(|| format!("failed to read entry in {}", dir.display()))?;
        if entry.path().is_symlink() {
            count += 1;
        }
    }
    Ok(count)
}

fn count_broken_symlinks(dir: &Path) -> Result<usize> {
    let mut count = 0;
    for entry in std::fs::read_dir(dir)
        .with_context(|| format!("failed to read directory {}", dir.display()))?
    {
        let entry = entry.with_context(|| format!("failed to read entry in {}", dir.display()))?;
        let path = entry.path();
        // is_symlink() checks the link itself; exists() follows it —
        // a symlink that exists but whose target doesn't yields true + false
        if path.is_symlink() && !path.exists() {
            count += 1;
        }
    }
    Ok(count)
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

        // Pre-init guard triggers: no library dir + no sources → friendly message, no error
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

        // Guard does NOT trigger because sources is non-empty.
        // show() still returns Ok (it warns on stderr but does not error).
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
    fn count_entries_counts_only_symlinks() {
        use std::os::unix::fs as unix_fs;

        let dir = tempfile::TempDir::new().unwrap();
        let target = tempfile::TempDir::new().unwrap();

        // Two symlinks — should be counted
        unix_fs::symlink(target.path(), dir.path().join("link_a")).unwrap();
        unix_fs::symlink(target.path(), dir.path().join("link_b")).unwrap();
        // One regular file — should be ignored
        std::fs::write(dir.path().join("regular"), "data").unwrap();

        assert_eq!(count_entries(dir.path()).unwrap(), 2);
    }

    // -- count_broken_symlinks --

    #[test]
    fn count_broken_symlinks_empty_dir() {
        let dir = tempfile::TempDir::new().unwrap();
        assert_eq!(count_broken_symlinks(dir.path()).unwrap(), 0);
    }

    #[test]
    fn count_broken_symlinks_detects_broken() {
        use std::os::unix::fs as unix_fs;

        let dir = tempfile::TempDir::new().unwrap();
        let real_target = tempfile::TempDir::new().unwrap();

        // Valid symlink
        unix_fs::symlink(real_target.path(), dir.path().join("valid")).unwrap();
        // Broken symlink
        unix_fs::symlink("/nonexistent/target", dir.path().join("broken")).unwrap();
        // Regular file (not a symlink — should not be counted)
        std::fs::write(dir.path().join("regular"), "data").unwrap();

        assert_eq!(count_broken_symlinks(dir.path()).unwrap(), 1);
    }
}
