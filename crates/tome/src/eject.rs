//! Eject tome's symlinks from all target tool directories.
//!
//! Removes symlinks that point into the library from each configured target,
//! leaving the library and config intact. Reversible via `tome sync`.

use anyhow::{Context, Result};
use console::style;
use std::path::PathBuf;

use crate::config::{Config, TargetName};
use crate::paths::TomePaths;

/// Plan describing what eject will remove.
pub(crate) struct EjectPlan {
    pub targets: Vec<TargetEjectEntry>,
    pub total_symlinks: usize,
}

pub(crate) struct TargetEjectEntry {
    pub name: TargetName,
    pub symlinks: Vec<PathBuf>,
}

/// Build an eject plan by scanning target directories for symlinks into the library.
pub(crate) fn plan(config: &Config, paths: &TomePaths) -> Result<EjectPlan> {
    let mut targets = Vec::new();
    let mut total = 0;

    for (target_name, target_config) in config.targets.iter() {
        let skills_dir = target_config.skills_dir();
        if !skills_dir.is_dir() {
            continue;
        }

        let mut symlinks = Vec::new();
        for entry in std::fs::read_dir(skills_dir)
            .with_context(|| format!("failed to read {}", skills_dir.display()))?
        {
            let entry = entry?;
            let path = entry.path();
            if path.is_symlink()
                && let Ok(target) = std::fs::read_link(&path)
            {
                let resolved = crate::paths::resolve_symlink_target(&path, &target);
                if resolved.starts_with(paths.library_dir()) {
                    symlinks.push(path);
                }
            }
        }

        total += symlinks.len();
        if !symlinks.is_empty() {
            targets.push(TargetEjectEntry {
                name: target_name.clone(),
                symlinks,
            });
        }
    }

    Ok(EjectPlan {
        targets,
        total_symlinks: total,
    })
}

/// Render the eject plan to stdout.
pub(crate) fn render_plan(plan: &EjectPlan) {
    if plan.total_symlinks == 0 {
        println!("Nothing to eject — no library symlinks found in any target.");
        return;
    }

    println!("Eject plan:");
    for entry in &plan.targets {
        println!(
            "  {}: {} symlink(s) to remove",
            style(entry.name.as_str()).cyan(),
            entry.symlinks.len()
        );
    }
    println!(
        "\nTotal: {} symlink(s) across {} target(s)",
        plan.total_symlinks,
        plan.targets.len()
    );
}

/// Execute the eject plan — remove all identified symlinks.
pub(crate) fn execute(plan: &EjectPlan, dry_run: bool) -> Result<usize> {
    let mut removed = 0;
    for entry in &plan.targets {
        for symlink in &entry.symlinks {
            if !dry_run {
                std::fs::remove_file(symlink)
                    .with_context(|| format!("failed to remove {}", symlink.display()))?;
            }
            removed += 1;
        }
    }
    Ok(removed)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{Config, TargetConfig, TargetMethod, TargetName};
    use std::collections::BTreeMap;
    use std::os::unix::fs as unix_fs;
    use std::path::PathBuf;
    use tempfile::TempDir;

    fn make_config_with_target(
        library_dir: PathBuf,
        target_name: &str,
        target_skills_dir: PathBuf,
    ) -> Config {
        let mut targets = BTreeMap::new();
        targets.insert(
            TargetName::new(target_name).unwrap(),
            TargetConfig {
                enabled: true,
                method: TargetMethod::Symlink {
                    skills_dir: target_skills_dir,
                },
            },
        );
        Config {
            library_dir,
            targets,
            ..Config::default()
        }
    }

    #[test]
    fn eject_removes_library_symlinks() {
        let library = TempDir::new().unwrap();
        let target = TempDir::new().unwrap();

        // Create a skill dir in the library
        let skill_dir = library.path().join("my-skill");
        std::fs::create_dir_all(&skill_dir).unwrap();

        // Create a symlink in the target pointing into the library
        unix_fs::symlink(&skill_dir, target.path().join("my-skill")).unwrap();

        let config = make_config_with_target(
            library.path().to_path_buf(),
            "test-target",
            target.path().to_path_buf(),
        );
        let paths =
            TomePaths::new(library.path().to_path_buf(), library.path().to_path_buf()).unwrap();

        let p = plan(&config, &paths).unwrap();
        assert_eq!(p.total_symlinks, 1);

        let removed = execute(&p, false).unwrap();
        assert_eq!(removed, 1);
        assert!(
            !target.path().join("my-skill").exists(),
            "symlink should be removed"
        );
        assert!(
            skill_dir.is_dir(),
            "library skill directory should remain intact"
        );
    }

    #[test]
    fn eject_preserves_external_symlinks() {
        let library = TempDir::new().unwrap();
        let target = TempDir::new().unwrap();
        let external = TempDir::new().unwrap();

        // Create an external dir and symlink it in the target
        let ext_dir = external.path().join("ext-skill");
        std::fs::create_dir_all(&ext_dir).unwrap();
        unix_fs::symlink(&ext_dir, target.path().join("ext-skill")).unwrap();

        let config = make_config_with_target(
            library.path().to_path_buf(),
            "test-target",
            target.path().to_path_buf(),
        );
        let paths =
            TomePaths::new(library.path().to_path_buf(), library.path().to_path_buf()).unwrap();

        let p = plan(&config, &paths).unwrap();
        assert_eq!(
            p.total_symlinks, 0,
            "should not include symlinks pointing outside library"
        );
        assert!(
            target.path().join("ext-skill").is_symlink(),
            "external symlink should be untouched"
        );
    }

    #[test]
    fn eject_dry_run_preserves_symlinks() {
        let library = TempDir::new().unwrap();
        let target = TempDir::new().unwrap();

        let skill_dir = library.path().join("my-skill");
        std::fs::create_dir_all(&skill_dir).unwrap();
        unix_fs::symlink(&skill_dir, target.path().join("my-skill")).unwrap();

        let config = make_config_with_target(
            library.path().to_path_buf(),
            "test-target",
            target.path().to_path_buf(),
        );
        let paths =
            TomePaths::new(library.path().to_path_buf(), library.path().to_path_buf()).unwrap();

        let p = plan(&config, &paths).unwrap();
        assert_eq!(p.total_symlinks, 1);

        let removed = execute(&p, true).unwrap();
        assert_eq!(removed, 1, "should count the would-be removal");
        assert!(
            target.path().join("my-skill").is_symlink(),
            "dry-run should not actually remove the symlink"
        );
    }
}
