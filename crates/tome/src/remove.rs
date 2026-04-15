//! Remove a source entry from config and clean up all artifacts.
//!
//! Cleanup order:
//! 1. Remove symlinks from target directories
//! 2. Remove library entries for skills from this source
//! 3. Remove manifest entries
//! 4. Remove source entry from config
//! 5. Regenerate lockfile

use anyhow::{Context, Result, bail};
use console::style;
use std::path::PathBuf;

use crate::config::Config;
use crate::manifest::Manifest;
use crate::paths::TomePaths;

/// What will be removed.
#[derive(Debug)]
pub(crate) struct RemovePlan {
    /// Name of the source to remove.
    pub source_name: String,
    /// Skills from this source found in the manifest.
    pub skills: Vec<String>,
    /// Symlinks in target directories pointing to these skills.
    pub symlinks_to_remove: Vec<PathBuf>,
    /// Library directories for these skills.
    pub library_paths: Vec<PathBuf>,
}

impl RemovePlan {
    /// Returns true if there is nothing to clean up (source entry still removed from config).
    #[allow(dead_code)]
    pub fn is_empty(&self) -> bool {
        self.skills.is_empty()
            && self.symlinks_to_remove.is_empty()
            && self.library_paths.is_empty()
    }
}

/// Result of executing the remove plan.
pub(crate) struct RemoveResult {
    pub symlinks_removed: usize,
    pub library_entries_removed: usize,
}

/// Build a plan describing what `tome remove <name>` will do.
pub(crate) fn plan(
    name: &str,
    config: &Config,
    paths: &TomePaths,
    manifest: &Manifest,
) -> Result<RemovePlan> {
    // Validate the source exists in config
    let source_exists = config.sources.iter().any(|s| s.name == name);
    if !source_exists {
        bail!("source '{}' not found in config", name);
    }

    // Find skills from this source in the manifest
    let skills: Vec<String> = manifest
        .iter()
        .filter(|(_, entry)| entry.source_name == name)
        .map(|(skill_name, _)| skill_name.as_str().to_string())
        .collect();

    // Find symlinks to remove from targets
    let mut symlinks_to_remove = Vec::new();
    for (_target_name, target_config) in config.targets.iter() {
        let skills_dir = target_config.skills_dir();
        if !skills_dir.is_dir() {
            continue;
        }
        for entry in std::fs::read_dir(skills_dir)
            .with_context(|| format!("failed to read {}", skills_dir.display()))?
        {
            let entry = entry?;
            let path = entry.path();
            if path.is_symlink() {
                let link_name = path
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or_default();
                if skills.iter().any(|s| s == link_name) {
                    symlinks_to_remove.push(path);
                }
            }
        }
    }

    // Find library directories to remove
    let library_paths: Vec<PathBuf> = skills
        .iter()
        .map(|s| paths.library_dir().join(s))
        .filter(|p| p.exists())
        .collect();

    Ok(RemovePlan {
        source_name: name.to_string(),
        skills,
        symlinks_to_remove,
        library_paths,
    })
}

/// Render the plan to stdout.
pub(crate) fn render_plan(plan: &RemovePlan) {
    println!(
        "Remove plan for source '{}':",
        style(&plan.source_name).cyan()
    );

    if plan.skills.is_empty() {
        println!("  No skills found in library from this source.");
    } else {
        println!(
            "  Skills to remove from library: {}",
            style(plan.skills.len()).bold()
        );
        for skill in &plan.skills {
            println!("    - {}", skill);
        }
    }

    if !plan.symlinks_to_remove.is_empty() {
        println!(
            "  Symlinks to remove: {}",
            style(plan.symlinks_to_remove.len()).bold()
        );
    }

    if !plan.library_paths.is_empty() {
        println!(
            "  Library directories to remove: {}",
            style(plan.library_paths.len()).bold()
        );
    }

    println!("  Config entry will be removed.");
}

/// Execute the remove plan.
pub(crate) fn execute(
    plan: &RemovePlan,
    config: &mut Config,
    manifest: &mut Manifest,
    dry_run: bool,
) -> Result<RemoveResult> {
    let mut symlinks_removed = 0;
    let mut library_entries_removed = 0;

    // 1. Remove symlinks from target directories
    for symlink in &plan.symlinks_to_remove {
        if !dry_run && let Err(e) = std::fs::remove_file(symlink) {
            eprintln!(
                "warning: failed to remove symlink {}: {}",
                symlink.display(),
                e
            );
        }
        symlinks_removed += 1;
    }

    // 2. Remove library directories
    for lib_path in &plan.library_paths {
        if !dry_run {
            if lib_path.is_symlink() {
                if let Err(e) = std::fs::remove_file(lib_path) {
                    eprintln!(
                        "warning: failed to remove library symlink {}: {}",
                        lib_path.display(),
                        e
                    );
                }
            } else if lib_path.is_dir()
                && let Err(e) = std::fs::remove_dir_all(lib_path)
            {
                eprintln!(
                    "warning: failed to remove library directory {}: {}",
                    lib_path.display(),
                    e
                );
            }
        }
        library_entries_removed += 1;
    }

    // 3. Remove manifest entries
    if !dry_run {
        for skill in &plan.skills {
            manifest.remove(skill);
        }
    }

    // 4. Remove source entry from config
    if !dry_run {
        config.sources.retain(|s| s.name != plan.source_name);
    }

    Ok(RemoveResult {
        symlinks_removed,
        library_entries_removed,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{Config, Source, SourceType, TargetConfig, TargetMethod, TargetName};
    use crate::discover::SkillName;
    use crate::manifest::{Manifest, SkillEntry};
    use crate::validation::ContentHash;
    use std::collections::BTreeMap;
    use std::os::unix::fs as unix_fs;
    use tempfile::TempDir;

    fn test_hash() -> ContentHash {
        ContentHash::new("a".repeat(64)).unwrap()
    }

    fn make_test_setup() -> (TempDir, Config, TomePaths, Manifest) {
        let tmp = TempDir::new().unwrap();
        let library_dir = tmp.path().join("library");
        std::fs::create_dir_all(&library_dir).unwrap();

        let source_dir = tmp.path().join("source");
        std::fs::create_dir_all(&source_dir).unwrap();

        let target_dir = tmp.path().join("target");
        std::fs::create_dir_all(&target_dir).unwrap();

        // Create a skill in the library
        let skill_dir = library_dir.join("my-skill");
        std::fs::create_dir_all(&skill_dir).unwrap();
        std::fs::write(skill_dir.join("SKILL.md"), "# my-skill").unwrap();

        // Create a symlink in the target
        unix_fs::symlink(&skill_dir, target_dir.join("my-skill")).unwrap();

        let config = Config {
            library_dir: library_dir.clone(),
            sources: vec![Source {
                name: "test-source".to_string(),
                path: source_dir,
                source_type: SourceType::Directory,
            }],
            targets: BTreeMap::from([(
                TargetName::new("test-target").unwrap(),
                TargetConfig {
                    enabled: true,
                    method: TargetMethod::Symlink {
                        skills_dir: target_dir,
                    },
                },
            )]),
            ..Default::default()
        };

        let paths = TomePaths::new(tmp.path().to_path_buf(), library_dir).unwrap();

        let mut manifest = Manifest::default();
        manifest.insert(
            SkillName::new("my-skill").unwrap(),
            SkillEntry::new(
                tmp.path().join("source/my-skill"),
                "test-source".to_string(),
                test_hash(),
                false,
            ),
        );

        (tmp, config, paths, manifest)
    }

    #[test]
    fn plan_errors_on_nonexistent_source() {
        let (_tmp, config, paths, manifest) = make_test_setup();
        let result = plan("nonexistent", &config, &paths, &manifest);
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("not found in config")
        );
    }

    #[test]
    fn plan_finds_skills_and_symlinks() {
        let (_tmp, config, paths, manifest) = make_test_setup();
        let p = plan("test-source", &config, &paths, &manifest).unwrap();
        assert_eq!(p.skills.len(), 1);
        assert_eq!(p.skills[0], "my-skill");
        assert_eq!(p.symlinks_to_remove.len(), 1);
        assert_eq!(p.library_paths.len(), 1);
    }

    #[test]
    fn execute_removes_artifacts() {
        let (_tmp, mut config, paths, mut manifest) = make_test_setup();
        let p = plan("test-source", &config, &paths, &manifest).unwrap();

        let result = execute(&p, &mut config, &mut manifest, false).unwrap();
        assert_eq!(result.symlinks_removed, 1);
        assert_eq!(result.library_entries_removed, 1);
        assert!(config.sources.is_empty());
        assert!(manifest.is_empty());
    }

    #[test]
    fn execute_dry_run_preserves_state() {
        let (_tmp, mut config, paths, mut manifest) = make_test_setup();
        let p = plan("test-source", &config, &paths, &manifest).unwrap();

        let result = execute(&p, &mut config, &mut manifest, true).unwrap();
        assert_eq!(result.symlinks_removed, 1);
        assert_eq!(result.library_entries_removed, 1);
        // Config and manifest should not be modified
        assert_eq!(config.sources.len(), 1);
        assert!(!manifest.is_empty());
    }
}
