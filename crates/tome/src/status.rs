//! Read-only summary of the library state, configured sources, targets, and overall health.

use anyhow::Result;
use console::style;
use std::path::Path;

use crate::config::Config;
use crate::discover;

/// Display the current status of the tome system.
pub fn show(config: &Config) -> Result<()> {
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
    Ok(std::fs::read_dir(dir)?.count())
}

fn count_broken_symlinks(dir: &Path) -> Result<usize> {
    Ok(std::fs::read_dir(dir)?
        .filter_map(|e| e.ok())
        .filter(|e| {
            let path = e.path();
            // is_symlink() checks the link itself; exists() follows it —
            // a symlink that exists but whose target doesn't yields true + false
            path.is_symlink() && !path.exists()
        })
        .count())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;
    use std::path::PathBuf;

    #[test]
    fn status_handles_missing_library_gracefully() {
        let config = Config {
            library_dir: PathBuf::from("/nonexistent/tome/library"),
            ..Config::default()
        };

        // show() should not return an error — it warns and shows "?" instead
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
    fn count_entries_with_files() {
        let dir = tempfile::TempDir::new().unwrap();
        for name in ["a", "b", "c"] {
            std::fs::write(dir.path().join(name), "").unwrap();
        }
        assert_eq!(count_entries(dir.path()).unwrap(), 3);
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
