//! Safely relocate the skill library to a new directory.
//!
//! Follows a Plan -> Preview -> Confirm -> Execute -> Verify pattern:
//! 1. Build a plan (read-only scan)
//! 2. Show the user exactly what will happen
//! 3. Wait for confirmation (default: No)
//! 4. Execute: move directory, update config, recreate target symlinks
//! 5. Verify: run doctor check on the new state

use anyhow::{Context, Result, bail};
use console::style;
use std::path::{Path, PathBuf};

use crate::config::{Config, TargetName, expand_tilde};
use crate::discover::SkillName;
use crate::manifest;
use crate::paths::{TomePaths, resolve_symlink_target};

/// A plan describing what the relocate command will do.
#[derive(Debug)]
pub(crate) struct RelocatePlan {
    pub old_library_dir: PathBuf,
    pub new_library_dir: PathBuf,
    pub skills: Vec<SkillMoveEntry>,
    /// (target_name, symlink_count) for each target with symlinks pointing into the library.
    pub targets: Vec<(TargetName, usize)>,
    pub cross_filesystem: bool,
    pub config_path: PathBuf,
}

/// A single skill that will be moved.
#[derive(Debug)]
pub(crate) struct SkillMoveEntry {
    pub name: SkillName,
    pub is_managed: bool,
    /// For managed skills, the original symlink target (external source path).
    #[allow(dead_code)]
    pub source_path: Option<PathBuf>,
}

/// Build a relocation plan by scanning the current library state.
///
/// This is a read-only operation that validates inputs and enumerates what would change.
pub(crate) fn plan(
    config: &Config,
    paths: &TomePaths,
    new_library_dir: &Path,
    config_path: &Path,
) -> Result<RelocatePlan> {
    let old_library_dir = paths.library_dir().to_path_buf();

    // Resolve the new path: expand tilde, make absolute if relative
    let new_library_dir = {
        let expanded = expand_tilde(new_library_dir)?;
        if expanded.is_absolute() {
            expanded
        } else {
            std::env::current_dir()
                .context("failed to determine current directory")?
                .join(expanded)
        }
    };

    // Validate: old dir must exist
    if !old_library_dir.is_dir() {
        bail!(
            "current library directory does not exist: {}",
            old_library_dir.display()
        );
    }

    // Validate: not the same path (check before "exists" to give a more specific error)
    // Canonicalize both paths to resolve symlinks (e.g. /var -> /private/var on macOS)
    let canonical_old = std::fs::canonicalize(&old_library_dir).unwrap_or(old_library_dir.clone());
    let canonical_new = std::fs::canonicalize(&new_library_dir).unwrap_or(new_library_dir.clone());
    if canonical_old == canonical_new {
        bail!("source and destination are the same path");
    }

    // Validate: new dir must not exist
    if new_library_dir.exists() {
        bail!("destination already exists: {}", new_library_dir.display());
    }

    // Load manifest to enumerate skills
    let manifest = manifest::load(paths.config_dir())?;
    let mut skills = Vec::new();
    for (name, entry) in manifest.iter() {
        let source_path = if entry.managed {
            // For managed skills, read the symlink target to record the external source
            let link_path = old_library_dir.join(name.as_str());
            if link_path.is_symlink() {
                let raw_target = std::fs::read_link(&link_path).ok();
                raw_target.map(|t| resolve_symlink_target(&link_path, &t))
            } else {
                None
            }
        } else {
            None
        };

        skills.push(SkillMoveEntry {
            name: name.clone(),
            is_managed: entry.managed,
            source_path,
        });
    }

    // Count target symlinks that point into the old library
    let canonical_old_for_targets =
        std::fs::canonicalize(&old_library_dir).unwrap_or(old_library_dir.clone());
    let mut targets = Vec::new();
    for (target_name, target_config) in config.targets.iter() {
        let skills_dir = target_config.skills_dir();
        if !skills_dir.is_dir() {
            continue;
        }
        let mut count = 0usize;
        if let Ok(entries) = std::fs::read_dir(skills_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_symlink()
                    && let Ok(raw_target) = std::fs::read_link(&path)
                {
                    let target = resolve_symlink_target(&path, &raw_target);
                    if target.starts_with(&old_library_dir)
                        || target.starts_with(&canonical_old_for_targets)
                    {
                        count += 1;
                    }
                }
            }
        }
        if count > 0 {
            targets.push((target_name.clone(), count));
        }
    }

    let cross_filesystem = is_cross_filesystem(&old_library_dir, &new_library_dir);

    Ok(RelocatePlan {
        old_library_dir,
        new_library_dir,
        skills,
        targets,
        cross_filesystem,
        config_path: config_path.to_path_buf(),
    })
}

/// Render the relocation plan to stdout for user review.
pub(crate) fn render_plan(plan: &RelocatePlan) {
    println!("{}", style("Relocation plan:").bold());
    println!("  From: {}", style(plan.old_library_dir.display()).cyan());
    println!("  To:   {}", style(plan.new_library_dir.display()).cyan());
    println!();

    let local_count = plan.skills.iter().filter(|s| !s.is_managed).count();
    let managed_count = plan.skills.iter().filter(|s| s.is_managed).count();
    println!(
        "  Skills: {} total ({} local, {} managed)",
        style(plan.skills.len()).bold(),
        local_count,
        managed_count,
    );

    if !plan.targets.is_empty() {
        println!();
        println!("  Target symlinks to recreate:");
        for (name, count) in &plan.targets {
            println!("    {}: {} symlink(s)", style(name).bold(), count);
        }
    }

    println!();
    if plan.cross_filesystem {
        println!(
            "  Move type: {} (copy + verify + delete)",
            style("cross-filesystem").yellow()
        );
    } else {
        println!(
            "  Move type: {} (atomic rename)",
            style("same-filesystem").green()
        );
    }

    println!("  Config:  {}", plan.config_path.display());
}

/// Execute the relocation plan.
///
/// Moves the library directory, updates the config file, and recreates target symlinks.
pub(crate) fn execute(plan: &RelocatePlan, dry_run: bool) -> Result<()> {
    if dry_run {
        println!("\n{}", style("Dry run -- no changes made.").yellow());
        return Ok(());
    }

    // 1. Create parent directory of destination
    if let Some(parent) = plan.new_library_dir.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("failed to create parent directory {}", parent.display()))?;
    }

    // 2. Move the library directory
    if plan.cross_filesystem {
        move_cross_filesystem(plan)?;
    } else {
        std::fs::rename(&plan.old_library_dir, &plan.new_library_dir).with_context(|| {
            format!(
                "failed to rename {} -> {}",
                plan.old_library_dir.display(),
                plan.new_library_dir.display()
            )
        })?;
    }

    println!(
        "  {} Moved library to {}",
        style("done").green(),
        plan.new_library_dir.display()
    );

    // 3. Update config file
    update_config(plan)?;
    println!(
        "  {} Updated config at {}",
        style("done").green(),
        plan.config_path.display()
    );

    // 4. Recreate target symlinks
    recreate_target_symlinks(plan)?;
    for (name, count) in &plan.targets {
        println!(
            "  {} Recreated {} symlink(s) for {}",
            style("done").green(),
            count,
            name
        );
    }

    Ok(())
}

/// Verify the relocated library by running doctor checks.
///
/// Returns the number of issues found.
pub(crate) fn verify(config: &Config, new_library_dir: &Path, tome_home: &Path) -> Result<usize> {
    let paths = TomePaths::new(tome_home.to_path_buf(), new_library_dir.to_path_buf())?;
    let report = crate::doctor::check(config, &paths)?;
    let total = report.total_issues();

    println!();
    if total == 0 {
        println!(
            "{}",
            style("Verification passed -- no issues found.")
                .green()
                .bold()
        );
    } else {
        println!(
            "{}",
            style(format!("Verification found {} issue(s).", total))
                .yellow()
                .bold()
        );
        for issue in &report.library_issues {
            println!("  {} {}", style("!").yellow(), issue.message);
        }
        for (name, issues) in &report.directory_issues {
            for issue in issues {
                println!("  {} {}: {}", style("!").yellow(), name, issue.message);
            }
        }
        for issue in &report.config_issues {
            println!("  {} {}", style("!").yellow(), issue.message);
        }
    }

    Ok(total)
}

// --- Internal helpers ---

/// Detect whether source and destination are on different filesystems.
fn is_cross_filesystem(src: &Path, dst: &Path) -> bool {
    // Walk up from dst until we find an existing ancestor
    let dst_ancestor = {
        let mut p = dst.to_path_buf();
        while !p.exists() {
            if !p.pop() {
                return true;
            }
        }
        p
    };
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        match (std::fs::metadata(src), std::fs::metadata(&dst_ancestor)) {
            (Ok(s), Ok(d)) => s.dev() != d.dev(),
            _ => true,
        }
    }
    #[cfg(not(unix))]
    {
        let _ = dst_ancestor;
        true
    }
}

/// Move the library across filesystems: copy preserving symlinks, verify, then delete old.
fn move_cross_filesystem(plan: &RelocatePlan) -> Result<()> {
    // Copy
    copy_library(&plan.old_library_dir, &plan.new_library_dir)?;

    // Verify content hashes for local skills
    let manifest = manifest::load(
        // tome_home is the parent of the config path
        plan.config_path.parent().unwrap_or(Path::new("/")),
    )?;

    for entry in &plan.skills {
        if entry.is_managed {
            // Managed skills are symlinks; verify the symlink exists and points to the right place
            let new_link = plan.new_library_dir.join(entry.name.as_str());
            if !new_link.is_symlink() {
                bail!(
                    "cross-filesystem copy failed: managed skill '{}' symlink not found at {}",
                    entry.name,
                    new_link.display()
                );
            }
        } else {
            // Local skills: verify content hash matches manifest
            let skill_dir = plan.new_library_dir.join(entry.name.as_str());
            if skill_dir.is_dir() {
                let new_hash = manifest::hash_directory(&skill_dir)?;
                if let Some(manifest_entry) = manifest.get(entry.name.as_str())
                    && new_hash != manifest_entry.content_hash
                {
                    bail!(
                        "cross-filesystem copy verification failed for '{}': hash mismatch \
                         (expected {}, got {})",
                        entry.name,
                        manifest_entry.content_hash,
                        new_hash
                    );
                }
            }
        }
    }

    // All verified -- delete old directory
    std::fs::remove_dir_all(&plan.old_library_dir).with_context(|| {
        format!(
            "failed to remove old library directory {} after successful copy",
            plan.old_library_dir.display()
        )
    })?;

    Ok(())
}

/// Recursively copy a library directory, preserving symlinks.
fn copy_library(src: &Path, dst: &Path) -> Result<()> {
    for entry in walkdir::WalkDir::new(src) {
        let entry = entry?;
        let rel = entry
            .path()
            .strip_prefix(src)
            .with_context(|| "BUG: path not under root".to_string())?;
        let dest_path = dst.join(rel);

        if entry.path().is_symlink() {
            let target = std::fs::read_link(entry.path())?;
            if let Some(parent) = dest_path.parent() {
                std::fs::create_dir_all(parent)?;
            }
            std::os::unix::fs::symlink(&target, &dest_path)?;
        } else if entry.file_type().is_dir() {
            std::fs::create_dir_all(&dest_path)?;
        } else {
            if let Some(parent) = dest_path.parent() {
                std::fs::create_dir_all(parent)?;
            }
            std::fs::copy(entry.path(), &dest_path)?;
        }
    }
    Ok(())
}

/// Update the config file to point to the new library directory.
fn update_config(plan: &RelocatePlan) -> Result<()> {
    // Back up old config
    let backup_path = plan.config_path.with_extension("toml.bak");
    std::fs::copy(&plan.config_path, &backup_path).with_context(|| {
        format!(
            "failed to back up config {} -> {}",
            plan.config_path.display(),
            backup_path.display()
        )
    })?;

    // Load, update, and save
    let mut config = Config::load(&plan.config_path)?;
    config.library_dir = plan.new_library_dir.clone();
    config.save(&plan.config_path)?;

    Ok(())
}

/// Recreate target symlinks to point to the new library location.
fn recreate_target_symlinks(plan: &RelocatePlan) -> Result<()> {
    // We need to load config to get target info, but at this point the config
    // has already been updated so we can use it.
    let config = Config::load(&plan.config_path)?;

    let canonical_old =
        std::fs::canonicalize(&plan.old_library_dir).unwrap_or(plan.old_library_dir.clone());

    for (target_name, _) in &plan.targets {
        let target_config = match config.targets.get(target_name.as_str()) {
            Some(tc) => tc,
            None => continue,
        };
        let skills_dir = target_config.skills_dir();
        if !skills_dir.is_dir() {
            continue;
        }

        let entries = std::fs::read_dir(skills_dir)
            .with_context(|| format!("failed to read target dir {}", skills_dir.display()))?;

        for entry in entries {
            let entry = entry?;
            let path = entry.path();
            if !path.is_symlink() {
                continue;
            }

            let raw_target = std::fs::read_link(&path)?;
            let resolved = resolve_symlink_target(&path, &raw_target);

            // Check if this symlink points into the OLD library
            let points_into_old =
                resolved.starts_with(&plan.old_library_dir) || resolved.starts_with(&canonical_old);

            if points_into_old {
                // Determine which skill this symlink points to
                let skill_name = entry.file_name();

                // Remove old symlink
                std::fs::remove_file(&path).with_context(|| {
                    format!("failed to remove old target symlink {}", path.display())
                })?;

                // Create new symlink pointing to new library
                let new_target = plan.new_library_dir.join(&skill_name);
                std::os::unix::fs::symlink(&new_target, &path).with_context(|| {
                    format!(
                        "failed to create target symlink {} -> {}",
                        path.display(),
                        new_target.display()
                    )
                })?;
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{Config, TargetConfig, TargetMethod};
    use crate::manifest::{self, SkillEntry};
    use std::collections::BTreeMap;
    use std::os::unix::fs as unix_fs;
    use tempfile::TempDir;

    /// Helper to create a minimal config for testing.
    fn test_config(library_dir: PathBuf, targets: BTreeMap<TargetName, TargetConfig>) -> Config {
        Config {
            library_dir,
            exclude: Default::default(),
            sources: Vec::new(),
            targets,
            ..Default::default()
        }
    }

    /// Helper to set up a library with one local skill and a manifest.
    fn setup_library(tome_home: &Path, library_dir: &Path, skill_name: &str) -> manifest::Manifest {
        let skill_dir = library_dir.join(skill_name);
        std::fs::create_dir_all(&skill_dir).unwrap();
        std::fs::write(skill_dir.join("SKILL.md"), "# Test Skill\n").unwrap();

        let hash = manifest::hash_directory(&skill_dir).unwrap();
        let mut manifest = manifest::Manifest::default();
        manifest.insert(
            SkillName::new(skill_name).unwrap(),
            SkillEntry::new(
                PathBuf::from("/tmp/source").join(skill_name),
                "test-source".to_string(),
                hash,
                false,
            ),
        );
        manifest::save(&manifest, tome_home).unwrap();
        manifest
    }

    #[test]
    fn plan_counts_skills() {
        let tome_home = TempDir::new().unwrap();
        let library_dir = tome_home.path().join("skills");
        std::fs::create_dir_all(&library_dir).unwrap();

        setup_library(tome_home.path(), &library_dir, "my-skill");

        let config = test_config(library_dir.clone(), BTreeMap::new());
        let paths = TomePaths::new(tome_home.path().to_path_buf(), library_dir.clone()).unwrap();
        let new_dir = tome_home.path().join("new-skills");

        let p = plan(
            &config,
            &paths,
            &new_dir,
            &tome_home.path().join("tome.toml"),
        )
        .unwrap();
        assert_eq!(p.skills.len(), 1);
        assert_eq!(p.skills[0].name.as_str(), "my-skill");
        assert!(!p.skills[0].is_managed);
        assert!(p.skills[0].source_path.is_none());
    }

    #[test]
    fn plan_rejects_existing_destination() {
        let tome_home = TempDir::new().unwrap();
        let library_dir = tome_home.path().join("skills");
        std::fs::create_dir_all(&library_dir).unwrap();

        let new_dir = tome_home.path().join("new-skills");
        std::fs::create_dir_all(&new_dir).unwrap(); // destination exists

        let config = test_config(library_dir.clone(), BTreeMap::new());
        let paths = TomePaths::new(tome_home.path().to_path_buf(), library_dir).unwrap();

        let result = plan(
            &config,
            &paths,
            &new_dir,
            &tome_home.path().join("tome.toml"),
        );
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(
            err.contains("destination already exists"),
            "unexpected error: {err}"
        );
    }

    #[test]
    fn plan_rejects_same_path() {
        let tome_home = TempDir::new().unwrap();
        let library_dir = tome_home.path().join("skills");
        std::fs::create_dir_all(&library_dir).unwrap();

        let config = test_config(library_dir.clone(), BTreeMap::new());
        let paths = TomePaths::new(tome_home.path().to_path_buf(), library_dir.clone()).unwrap();

        let result = plan(
            &config,
            &paths,
            &library_dir,
            &tome_home.path().join("tome.toml"),
        );
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("same path"), "unexpected error: {err}");
    }

    #[test]
    fn execute_moves_library_same_fs() {
        let tome_home = TempDir::new().unwrap();
        let library_dir = tome_home.path().join("skills");
        std::fs::create_dir_all(&library_dir).unwrap();

        setup_library(tome_home.path(), &library_dir, "test-skill");

        // Create a config file
        let config_path = tome_home.path().join("tome.toml");
        let config = test_config(library_dir.clone(), BTreeMap::new());
        config.save(&config_path).unwrap();

        let paths = TomePaths::new(tome_home.path().to_path_buf(), library_dir.clone()).unwrap();
        let new_dir = tome_home.path().join("new-skills");

        let p = plan(&config, &paths, &new_dir, &config_path).unwrap();
        execute(&p, false).unwrap();

        // Old location should be gone
        assert!(!library_dir.exists(), "old library dir should not exist");
        // New location should have the skill
        assert!(
            new_dir.join("test-skill").join("SKILL.md").exists(),
            "skill should exist at new location"
        );

        // Config should be updated
        let updated_config = Config::load(&config_path).unwrap();
        assert_eq!(updated_config.library_dir, new_dir);
    }

    #[test]
    fn execute_dry_run_changes_nothing() {
        let tome_home = TempDir::new().unwrap();
        let library_dir = tome_home.path().join("skills");
        std::fs::create_dir_all(&library_dir).unwrap();

        setup_library(tome_home.path(), &library_dir, "test-skill");

        let config_path = tome_home.path().join("tome.toml");
        let config = test_config(library_dir.clone(), BTreeMap::new());
        config.save(&config_path).unwrap();

        let paths = TomePaths::new(tome_home.path().to_path_buf(), library_dir.clone()).unwrap();
        let new_dir = tome_home.path().join("new-skills");

        let p = plan(&config, &paths, &new_dir, &config_path).unwrap();
        execute(&p, true).unwrap();

        // Library should still be at old location
        assert!(
            library_dir.exists(),
            "library should still exist at old location"
        );
        assert!(
            library_dir.join("test-skill").join("SKILL.md").exists(),
            "skill should still exist at old location"
        );
        // New location should NOT exist
        assert!(!new_dir.exists(), "new location should not exist");

        // Config should be unchanged
        let unchanged_config = Config::load(&config_path).unwrap();
        assert_eq!(unchanged_config.library_dir, library_dir);
    }

    #[test]
    fn execute_recreates_target_symlinks() {
        let tome_home = TempDir::new().unwrap();
        let library_dir = tome_home.path().join("skills");
        std::fs::create_dir_all(&library_dir).unwrap();

        setup_library(tome_home.path(), &library_dir, "my-skill");

        // Create a target directory with a symlink into the library
        let target_dir = tome_home.path().join("target-skills");
        std::fs::create_dir_all(&target_dir).unwrap();
        unix_fs::symlink(library_dir.join("my-skill"), target_dir.join("my-skill")).unwrap();

        let mut targets = BTreeMap::new();
        targets.insert(
            TargetName::new("test-target").unwrap(),
            TargetConfig {
                enabled: true,
                method: TargetMethod::Symlink {
                    skills_dir: target_dir.clone(),
                },
            },
        );

        let config_path = tome_home.path().join("tome.toml");
        let config = test_config(library_dir.clone(), targets);
        config.save(&config_path).unwrap();

        let paths = TomePaths::new(tome_home.path().to_path_buf(), library_dir.clone()).unwrap();
        let new_dir = tome_home.path().join("new-skills");

        let p = plan(&config, &paths, &new_dir, &config_path).unwrap();
        assert_eq!(p.targets.len(), 1);
        assert_eq!(p.targets[0].1, 1); // 1 symlink

        execute(&p, false).unwrap();

        // Target symlink should now point to new location
        let target_link = target_dir.join("my-skill");
        assert!(
            target_link.is_symlink(),
            "target symlink should still exist"
        );
        let link_target = std::fs::read_link(&target_link).unwrap();
        assert!(
            link_target.starts_with(&new_dir),
            "target symlink should point to new library location, but points to {}",
            link_target.display()
        );
        // And it should be valid (skill exists at new location)
        assert!(
            target_link.exists(),
            "target symlink should point to an existing directory"
        );
    }

    #[test]
    fn execute_preserves_managed_symlinks() {
        let tome_home = TempDir::new().unwrap();
        let library_dir = tome_home.path().join("skills");
        std::fs::create_dir_all(&library_dir).unwrap();

        // Create an external source directory (simulating a package manager)
        let external_source = TempDir::new().unwrap();
        let ext_skill_dir = external_source.path().join("managed-skill");
        std::fs::create_dir_all(&ext_skill_dir).unwrap();
        std::fs::write(ext_skill_dir.join("SKILL.md"), "# Managed\n").unwrap();

        // Create managed symlink in library pointing to external source
        unix_fs::symlink(&ext_skill_dir, library_dir.join("managed-skill")).unwrap();

        // Set up manifest with managed entry
        let hash = manifest::hash_directory(&ext_skill_dir).unwrap();
        let mut manifest = manifest::Manifest::default();
        manifest.insert(
            SkillName::new("managed-skill").unwrap(),
            SkillEntry::new(
                ext_skill_dir.clone(),
                "plugins".to_string(),
                hash,
                true, // managed
            ),
        );
        manifest::save(&manifest, tome_home.path()).unwrap();

        let config_path = tome_home.path().join("tome.toml");
        let config = test_config(library_dir.clone(), BTreeMap::new());
        config.save(&config_path).unwrap();

        let paths = TomePaths::new(tome_home.path().to_path_buf(), library_dir.clone()).unwrap();
        let new_dir = tome_home.path().join("new-skills");

        let p = plan(&config, &paths, &new_dir, &config_path).unwrap();

        // Verify the plan sees the managed skill
        let managed = p
            .skills
            .iter()
            .find(|s| s.name.as_str() == "managed-skill")
            .unwrap();
        assert!(managed.is_managed);
        assert!(managed.source_path.is_some());

        execute(&p, false).unwrap();

        // The managed skill symlink at the new location should point to the
        // original external source (not into the old library)
        let new_link = new_dir.join("managed-skill");
        assert!(
            new_link.is_symlink(),
            "managed skill should be a symlink at new location"
        );
        let link_target = std::fs::read_link(&new_link).unwrap();
        assert_eq!(
            link_target, ext_skill_dir,
            "managed skill symlink should still point to original external source"
        );
        // And it should be a valid symlink (external source still exists)
        assert!(
            new_link.exists(),
            "managed skill symlink should resolve to existing external source"
        );
    }

    #[test]
    fn is_cross_filesystem_same_fs() {
        let tmp = TempDir::new().unwrap();
        let src = tmp.path().join("src");
        std::fs::create_dir_all(&src).unwrap();
        let dst = tmp.path().join("dst");
        // dst doesn't exist, but its parent does (same tmpdir)
        assert!(
            !is_cross_filesystem(&src, &dst),
            "paths on same tmpdir should be same filesystem"
        );
    }

    #[test]
    fn plan_nonexistent_library_fails() {
        let tome_home = TempDir::new().unwrap();
        let library_dir = tome_home.path().join("nonexistent-library");
        // Don't create it

        let config = test_config(library_dir.clone(), BTreeMap::new());
        let paths = TomePaths::new(tome_home.path().to_path_buf(), library_dir).unwrap();
        let new_dir = tome_home.path().join("new-skills");

        let result = plan(
            &config,
            &paths,
            &new_dir,
            &tome_home.path().join("tome.toml"),
        );
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("does not exist"), "unexpected error: {err}");
    }
}
