//! Reassign or fork a skill to a different directory.
//!
//! `tome reassign <skill> --to <dir>` changes which directory owns a skill in the manifest.
//! `tome fork <skill> --to <dir>` copies skill files to the target and updates provenance.
//!
//! Follows the plan/render/execute pattern from `remove.rs`.

use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use console::style;

use crate::config::{Config, DirectoryName};
use crate::discover::SkillName;
use crate::manifest::Manifest;
use crate::paths::TomePaths;

/// What needs to happen to reassign a skill.
#[derive(Debug)]
pub(crate) enum ReassignAction {
    /// Skill already exists in target dir — just update manifest.
    Relink,
    /// Skill not in target dir — copy from library, then update manifest.
    CopyAndRelink,
}

/// Plan for reassigning or forking a skill.
#[derive(Debug)]
pub(crate) struct ReassignPlan {
    /// Name of the skill being reassigned.
    pub skill_name: SkillName,
    /// Current source_name from manifest.
    pub from_directory: String,
    /// Target directory to reassign to.
    pub to_directory: DirectoryName,
    /// What filesystem action is needed.
    pub action: ReassignAction,
    /// Path to skill directory in the library.
    pub library_skill_path: PathBuf,
    /// Whether this was invoked via `tome fork`.
    pub is_fork: bool,
}

/// Build a plan describing what the reassign/fork will do.
pub(crate) fn plan(
    skill_name: &str,
    to_dir: &str,
    config: &Config,
    paths: &TomePaths,
    manifest: &Manifest,
    is_fork: bool,
) -> Result<ReassignPlan> {
    // Validate skill exists in manifest
    let entry = manifest
        .get(skill_name)
        .ok_or_else(|| anyhow::anyhow!("skill '{}' not found in library", skill_name))?;

    let from_directory = entry.source_name.clone();

    // Validate target directory exists in config
    let to_dir_name =
        DirectoryName::new(to_dir).with_context(|| format!("invalid directory name: {to_dir}"))?;
    let to_dir_config = config
        .directories
        .get(&to_dir_name)
        .ok_or_else(|| anyhow::anyhow!("directory '{}' not found in config", to_dir))?;

    // Determine action: does the skill already exist in the target directory?
    let target_skill_path = crate::config::expand_tilde(&to_dir_config.path)?
        .join(skill_name)
        .join("SKILL.md");
    let action = if target_skill_path.exists() {
        ReassignAction::Relink
    } else {
        ReassignAction::CopyAndRelink
    };

    let library_skill_path = paths.library_dir().join(skill_name);

    Ok(ReassignPlan {
        skill_name: SkillName::new(skill_name)?,
        from_directory,
        to_directory: to_dir_name,
        action,
        library_skill_path,
        is_fork,
    })
}

/// Render the plan to stdout.
pub(crate) fn render_plan(plan: &ReassignPlan) {
    let skill = style(plan.skill_name.as_str()).cyan();
    let from = style(&plan.from_directory).cyan();
    let to = style(AsRef::<str>::as_ref(&plan.to_directory)).cyan();

    match (&plan.action, plan.is_fork) {
        (ReassignAction::Relink, _) => {
            println!(
                "Reassign '{}' from '{}' to '{}' (skill already present in target)",
                skill, from, to,
            );
        }
        (ReassignAction::CopyAndRelink, true) => {
            println!(
                "Fork '{}' from '{}' to '{}' (copy files to target directory)",
                skill, from, to,
            );
        }
        (ReassignAction::CopyAndRelink, false) => {
            println!(
                "Reassign '{}' from '{}' to '{}' (copy files to target directory)",
                skill, from, to,
            );
        }
    }
}

/// Execute the reassign/fork plan.
pub(crate) fn execute(
    plan: &ReassignPlan,
    manifest: &mut Manifest,
    target_dir_path: &Path,
    dry_run: bool,
) -> Result<()> {
    let verb = if plan.is_fork { "Fork" } else { "Reassign" };

    if dry_run {
        match &plan.action {
            ReassignAction::Relink => {
                println!(
                    "{} reassign '{}' to '{}'",
                    style("Would").yellow(),
                    style(plan.skill_name.as_str()).cyan(),
                    style(AsRef::<str>::as_ref(&plan.to_directory)).cyan(),
                );
            }
            ReassignAction::CopyAndRelink => {
                println!(
                    "{} {} '{}' to '{}'",
                    style("Would").yellow(),
                    verb.to_lowercase(),
                    style(plan.skill_name.as_str()).cyan(),
                    style(AsRef::<str>::as_ref(&plan.to_directory)).cyan(),
                );
            }
        }
        return Ok(());
    }

    // Copy files if needed
    if matches!(plan.action, ReassignAction::CopyAndRelink) {
        let dest = target_dir_path
            .join(plan.skill_name.as_str());
        copy_dir_recursive(&plan.library_skill_path, &dest)
            .with_context(|| format!("failed to copy skill to {}", dest.display()))?;
    }

    // Update manifest source_name
    if !manifest.update_source_name(
        plan.skill_name.as_str(),
        plan.to_directory.as_ref(),
    ) {
        anyhow::bail!(
            "skill '{}' disappeared from manifest during reassignment",
            plan.skill_name.as_str()
        );
    }

    Ok(())
}

/// Recursively copy a directory and its contents.
fn copy_dir_recursive(src: &Path, dst: &Path) -> Result<()> {
    std::fs::create_dir_all(dst)
        .with_context(|| format!("failed to create directory {}", dst.display()))?;

    for entry in std::fs::read_dir(src)
        .with_context(|| format!("failed to read directory {}", src.display()))?
    {
        let entry = entry?;
        let src_path = entry.path();
        let dst_path = dst.join(entry.file_name());

        if src_path.is_dir() {
            copy_dir_recursive(&src_path, &dst_path)?;
        } else {
            std::fs::copy(&src_path, &dst_path).with_context(|| {
                format!(
                    "failed to copy {} to {}",
                    src_path.display(),
                    dst_path.display()
                )
            })?;
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;
    use crate::manifest::Manifest;
    use crate::paths::TomePaths;
    use tempfile::TempDir;

    fn test_paths(tmp: &TempDir) -> TomePaths {
        let tome_home = tmp.path().join("tome_home");
        let library = tome_home.join("library");
        std::fs::create_dir_all(&library).unwrap();
        TomePaths::new(tome_home, library).unwrap()
    }

    #[test]
    fn test_plan_skill_not_found() {
        let tmp = TempDir::new().unwrap();
        let paths = test_paths(&tmp);
        let config = Config::default();
        let manifest = Manifest::default();

        let result = plan("nonexistent", "some-dir", &config, &paths, &manifest, false);
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(
            err.contains("not found in library"),
            "expected 'not found in library' in error: {err}"
        );
    }

    fn make_config_with_dir(tmp: &TempDir, name: &str) -> Config {
        use crate::config::{DirectoryConfig, DirectoryType};
        let dir_path = tmp.path().join(name);
        std::fs::create_dir_all(&dir_path).unwrap();
        let mut config = Config::default();
        config.directories.insert(
            DirectoryName::new(name).unwrap(),
            DirectoryConfig {
                path: dir_path,
                directory_type: DirectoryType::Directory,
                role: None,
                branch: None,
                tag: None,
                rev: None,
                subdir: None,
            },
        );
        config
    }

    #[test]
    fn test_plan_happy_path_copy_and_relink() {
        let tmp = TempDir::new().unwrap();
        let paths = test_paths(&tmp);
        let config = make_config_with_dir(&tmp, "target-dir");
        let mut manifest = Manifest::default();

        use crate::manifest::SkillEntry;
        use crate::validation::ContentHash;
        manifest.insert(
            SkillName::new("test-skill").unwrap(),
            SkillEntry::new(
                PathBuf::from("/some/path"),
                "old-dir".to_string(),
                ContentHash::new("a".repeat(64)).unwrap(),
                false,
            ),
        );

        let result = plan("test-skill", "target-dir", &config, &paths, &manifest, false).unwrap();
        assert_eq!(result.skill_name.as_str(), "test-skill");
        assert_eq!(result.from_directory, "old-dir");
        assert_eq!(AsRef::<str>::as_ref(&result.to_directory), "target-dir");
        assert!(matches!(result.action, ReassignAction::CopyAndRelink));
        assert!(!result.is_fork);
    }

    #[test]
    fn test_plan_relink_when_skill_exists_in_target() {
        let tmp = TempDir::new().unwrap();
        let paths = test_paths(&tmp);
        let config = make_config_with_dir(&tmp, "target-dir");
        let mut manifest = Manifest::default();

        // Create skill dir in the target so it detects as Relink
        let target_skill = tmp.path().join("target-dir").join("test-skill");
        std::fs::create_dir_all(&target_skill).unwrap();
        std::fs::write(target_skill.join("SKILL.md"), "# test").unwrap();

        use crate::manifest::SkillEntry;
        use crate::validation::ContentHash;
        manifest.insert(
            SkillName::new("test-skill").unwrap(),
            SkillEntry::new(
                PathBuf::from("/some/path"),
                "old-dir".to_string(),
                ContentHash::new("a".repeat(64)).unwrap(),
                false,
            ),
        );

        let result = plan("test-skill", "target-dir", &config, &paths, &manifest, true).unwrap();
        assert!(matches!(result.action, ReassignAction::Relink));
        assert!(result.is_fork);
    }

    #[test]
    fn test_plan_dir_not_found() {
        let tmp = TempDir::new().unwrap();
        let paths = test_paths(&tmp);
        let config = Config::default();
        let mut manifest = Manifest::default();

        // Add a skill to the manifest
        use crate::manifest::SkillEntry;
        use crate::validation::ContentHash;
        manifest.insert(
            SkillName::new("test-skill").unwrap(),
            SkillEntry::new(
                PathBuf::from("/some/path"),
                "old-dir".to_string(),
                ContentHash::new("a".repeat(64)).unwrap(),
                false,
            ),
        );

        let result = plan("test-skill", "nonexistent", &config, &paths, &manifest, false);
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(
            err.contains("not found in config"),
            "expected 'not found in config' in error: {err}"
        );
    }
}
