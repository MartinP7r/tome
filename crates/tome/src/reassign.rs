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
    /// Current source_name from manifest. `None` when reassigning an
    /// Unowned skill (D-API-1 / Phase 14 UNOWN-01).
    pub from_directory: Option<DirectoryName>,
    /// Target directory to reassign to.
    pub to_directory: DirectoryName,
    /// What filesystem action is needed.
    pub action: ReassignAction,
    /// Path to skill directory in the library.
    pub library_skill_path: PathBuf,
    /// Whether this was invoked via `tome fork`.
    pub is_fork: bool,
    /// Bypass D-A1 different-content collision refusal. Same-content
    /// collisions always take the Relink path regardless.
    ///
    /// Stored for introspection (e.g. `p.force` in unit tests) — production
    /// callers consume the flag inside `plan()` itself when deciding whether
    /// to bail on a different-content collision.
    #[allow(dead_code)]
    pub force: bool,
}

/// Build a plan describing what the reassign/fork will do.
pub(crate) fn plan(
    skill_name: &str,
    to_dir: &str,
    config: &Config,
    paths: &TomePaths,
    manifest: &Manifest,
    is_fork: bool,
    force: bool,
) -> Result<ReassignPlan> {
    // Validate skill exists in manifest
    let entry = manifest
        .get(skill_name)
        .ok_or_else(|| anyhow::anyhow!("skill '{}' not found in library", skill_name))?;

    // D-API-1 (Phase 14): Unowned skills are valid input. The ReassignPlan
    // carries `from_directory: Option<DirectoryName>` so render_plan can
    // distinguish "Unowned → <to>" from "<from> → <to>". The previous stub
    // error pointing at `tome adopt` is removed.
    let from_directory = entry.source_name.clone();

    // Validate target directory exists in config
    let to_dir_name =
        DirectoryName::new(to_dir).with_context(|| format!("invalid directory name: {to_dir}"))?;
    let to_dir_config = config
        .directories
        .get(&to_dir_name)
        .ok_or_else(|| anyhow::anyhow!("directory '{}' not found in config", to_dir))?;

    // D-A2 (Phase 14): refuse target-only roles. Reassigning into a
    // target-only dir leaves the skill stranded — nothing rediscovers
    // it on next sync.
    if !to_dir_config.role().is_discovery() {
        anyhow::bail!(
            "directory '{}' has role 'target-only' and cannot receive \
             reassigned skills (next sync would not rediscover them). \
             Reassign into a discovery or mixed-role directory.",
            to_dir,
        );
    }

    // Determine action: does the skill already exist in the target directory?
    let target_dir_for_skill = crate::config::expand_tilde(&to_dir_config.path)?.join(skill_name);
    let target_skill_md = target_dir_for_skill.join("SKILL.md");

    let library_skill_path = paths.library_dir().join(skill_name);

    let action = if target_skill_md.exists() {
        // D-A1 (Phase 14): content-hash collision check. If the target
        // dir's <skill>/ already exists, hash both sides; same content =
        // Relink (manifest-only flip), different content = refuse unless
        // --force.
        let target_hash =
            crate::manifest::hash_directory(&target_dir_for_skill).with_context(|| {
                format!(
                    "failed to hash existing target skill {}",
                    target_dir_for_skill.display()
                )
            })?;
        let library_hash = if library_skill_path.is_dir() {
            crate::manifest::hash_directory(&library_skill_path).with_context(|| {
                format!(
                    "failed to hash library skill {}",
                    library_skill_path.display()
                )
            })?
        } else {
            // Library copy missing — defer to existing error path; bail
            // with a recognisable message so callers don't see a confusing
            // "different content" error.
            anyhow::bail!(
                "skill '{}' is missing from the library at {}; cannot reassign",
                skill_name,
                library_skill_path.display(),
            );
        };

        if target_hash == library_hash {
            ReassignAction::Relink
        } else if force {
            ReassignAction::CopyAndRelink
        } else {
            anyhow::bail!(
                "skill '{}' already exists in '{}' with different content. \
                 Use --force to overwrite, or remove the existing entry first.",
                skill_name,
                to_dir,
            );
        }
    } else {
        ReassignAction::CopyAndRelink
    };

    Ok(ReassignPlan {
        skill_name: SkillName::new(skill_name)?,
        from_directory,
        to_directory: to_dir_name,
        action,
        library_skill_path,
        is_fork,
        force,
    })
}

/// Render the plan to stdout.
pub(crate) fn render_plan(plan: &ReassignPlan) {
    let skill = style(plan.skill_name.as_str()).cyan();
    let from_label = match &plan.from_directory {
        Some(d) => style(d.as_str().to_string()).cyan().to_string(),
        None => style("Unowned").yellow().to_string(),
    };
    let to = style(AsRef::<str>::as_ref(&plan.to_directory)).cyan();

    match (&plan.action, plan.is_fork) {
        (ReassignAction::Relink, _) => {
            println!(
                "Reassign '{}' from '{}' to '{}' (skill already present in target)",
                skill, from_label, to,
            );
        }
        (ReassignAction::CopyAndRelink, true) => {
            println!(
                "Fork '{}' from '{}' to '{}' (copy files to target directory)",
                skill, from_label, to,
            );
        }
        (ReassignAction::CopyAndRelink, false) => {
            println!(
                "Reassign '{}' from '{}' to '{}' (copy files to target directory)",
                skill, from_label, to,
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
        let dest = target_dir_path.join(plan.skill_name.as_str());
        copy_dir_recursive(&plan.library_skill_path, &dest)
            .with_context(|| format!("failed to copy skill to {}", dest.display()))?;
    }

    // Update manifest: set source_name = Some(to_directory), clear
    // previous_source (D-C1 closure: the skill is owned again). Works for
    // both Owned→Owned (today, via `update_source_name`) and
    // Unowned→Owned (D-API-1, via `skills_get_mut`) starting states.
    //
    // We try `update_source_name` first — it's the public API for the
    // common Owned→Owned case. If the entry is Unowned (returns false),
    // we fall through to `skills_get_mut` to set `source_name` directly.
    // Either path then uses `skills_get_mut` once more to clear
    // `previous_source` per D-C1.
    let _ = manifest.update_source_name(plan.skill_name.as_str(), &plan.to_directory);
    let entry = manifest
        .skills_get_mut(plan.skill_name.as_str())
        .ok_or_else(|| {
            anyhow::anyhow!(
                "skill '{}' disappeared from manifest during reassignment",
                plan.skill_name.as_str()
            )
        })?;
    entry.source_name = Some(plan.to_directory.clone());
    entry.previous_source = None;

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

        let result = plan(
            "nonexistent",
            "some-dir",
            &config,
            &paths,
            &manifest,
            false,
            false,
        );
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
                git_ref: None,
                subdir: None,
                override_applied: false,
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
                DirectoryName::new("old-dir").unwrap(),
                ContentHash::new("a".repeat(64)).unwrap(),
                false,
            ),
        );

        let result = plan(
            "test-skill",
            "target-dir",
            &config,
            &paths,
            &manifest,
            false,
            false,
        )
        .unwrap();
        assert_eq!(result.skill_name.as_str(), "test-skill");
        assert_eq!(
            result.from_directory.as_ref().map(|d| d.as_str()),
            Some("old-dir")
        );
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

        // Create skill dir in the target AND in the library with identical
        // content so D-A1's content-hash compare resolves to Relink.
        let target_skill = tmp.path().join("target-dir").join("test-skill");
        std::fs::create_dir_all(&target_skill).unwrap();
        std::fs::write(target_skill.join("SKILL.md"), "# test").unwrap();
        let library_skill = paths.library_dir().join("test-skill");
        std::fs::create_dir_all(&library_skill).unwrap();
        std::fs::write(library_skill.join("SKILL.md"), "# test").unwrap();

        use crate::manifest::SkillEntry;
        use crate::validation::ContentHash;
        manifest.insert(
            SkillName::new("test-skill").unwrap(),
            SkillEntry::new(
                PathBuf::from("/some/path"),
                DirectoryName::new("old-dir").unwrap(),
                ContentHash::new("a".repeat(64)).unwrap(),
                false,
            ),
        );

        let result = plan(
            "test-skill",
            "target-dir",
            &config,
            &paths,
            &manifest,
            true,
            false,
        )
        .unwrap();
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
                DirectoryName::new("old-dir").unwrap(),
                ContentHash::new("a".repeat(64)).unwrap(),
                false,
            ),
        );

        let result = plan(
            "test-skill",
            "nonexistent",
            &config,
            &paths,
            &manifest,
            false,
            false,
        );
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(
            err.contains("not found in config"),
            "expected 'not found in config' in error: {err}"
        );
    }

    fn write_skill_in_dir(dir: &std::path::Path, skill: &str, body: &str) {
        let skill_dir = dir.join(skill);
        std::fs::create_dir_all(&skill_dir).unwrap();
        std::fs::write(skill_dir.join("SKILL.md"), body).unwrap();
    }

    #[test]
    fn plan_accepts_unowned_input() {
        let tmp = TempDir::new().unwrap();
        let paths = test_paths(&tmp);
        let config = make_config_with_dir(&tmp, "target-dir");
        let mut manifest = Manifest::default();

        // Insert an Unowned skill (source_name = None).
        use crate::manifest::SkillEntry;
        use crate::validation::ContentHash;
        manifest.insert(
            SkillName::new("orphan-skill").unwrap(),
            SkillEntry::new_unowned(
                PathBuf::from("/tmp/old/orphan-skill"),
                ContentHash::new("a".repeat(64)).unwrap(),
                false,
                Some(crate::config::DirectoryName::new("removed-dir").unwrap()),
            ),
        );

        let result = plan(
            "orphan-skill",
            "target-dir",
            &config,
            &paths,
            &manifest,
            false,
            false,
        );
        assert!(
            result.is_ok(),
            "Unowned input must NOT be refused (D-API-1): {:?}",
            result.err()
        );
        let plan = result.unwrap();
        assert!(
            plan.from_directory.is_none(),
            "Unowned input → from_directory = None"
        );
    }

    #[test]
    fn plan_rejects_target_only_role() {
        use crate::config::{DirectoryConfig, DirectoryRole, DirectoryType};

        let tmp = TempDir::new().unwrap();
        let paths = test_paths(&tmp);
        let dir_path = tmp.path().join("claude-target");
        std::fs::create_dir_all(&dir_path).unwrap();
        let mut config = Config::default();
        config.directories.insert(
            crate::config::DirectoryName::new("claude-target").unwrap(),
            DirectoryConfig {
                path: dir_path,
                directory_type: DirectoryType::Directory,
                role: Some(DirectoryRole::Target), // target-only
                git_ref: None,
                subdir: None,
                override_applied: false,
            },
        );
        let mut manifest = Manifest::default();

        use crate::manifest::SkillEntry;
        use crate::validation::ContentHash;
        manifest.insert(
            SkillName::new("my-skill").unwrap(),
            SkillEntry::new(
                PathBuf::from("/tmp/x"),
                crate::config::DirectoryName::new("old-dir").unwrap(),
                ContentHash::new("a".repeat(64)).unwrap(),
                false,
            ),
        );

        let result = plan(
            "my-skill",
            "claude-target",
            &config,
            &paths,
            &manifest,
            false,
            false,
        );
        let err = result
            .expect_err("must reject target-only role per D-A2")
            .to_string();
        assert!(
            err.contains("target-only"),
            "error must mention 'target-only', got: {err}"
        );
        assert!(
            err.contains("Reassign into a discovery or mixed-role directory"),
            "error must include the actionable hint per D-A2, got: {err}"
        );
    }

    #[test]
    fn plan_refuses_different_content_collision_without_force() {
        let tmp = TempDir::new().unwrap();
        let paths = test_paths(&tmp);
        let config = make_config_with_dir(&tmp, "target-dir");

        // Library has skill content "library version".
        write_skill_in_dir(paths.library_dir(), "test-skill", "library version");
        // Target dir already has skill with DIFFERENT content.
        let target_dir = tmp.path().join("target-dir");
        write_skill_in_dir(&target_dir, "test-skill", "different version");

        let mut manifest = Manifest::default();
        use crate::manifest::SkillEntry;
        use crate::validation::ContentHash;
        manifest.insert(
            SkillName::new("test-skill").unwrap(),
            SkillEntry::new(
                PathBuf::from("/tmp/x"),
                crate::config::DirectoryName::new("old").unwrap(),
                ContentHash::new("a".repeat(64)).unwrap(),
                false,
            ),
        );

        let err = plan(
            "test-skill",
            "target-dir",
            &config,
            &paths,
            &manifest,
            false,
            false,
        )
        .expect_err("must refuse different-content collision per D-A1")
        .to_string();
        assert!(err.contains("with different content"), "got: {err}");
        assert!(err.contains("Use --force"), "got: {err}");
    }

    #[test]
    fn plan_force_bypasses_different_content_collision() {
        let tmp = TempDir::new().unwrap();
        let paths = test_paths(&tmp);
        let config = make_config_with_dir(&tmp, "target-dir");

        write_skill_in_dir(paths.library_dir(), "test-skill", "library version");
        let target_dir = tmp.path().join("target-dir");
        write_skill_in_dir(&target_dir, "test-skill", "different version");

        let mut manifest = Manifest::default();
        use crate::manifest::SkillEntry;
        use crate::validation::ContentHash;
        manifest.insert(
            SkillName::new("test-skill").unwrap(),
            SkillEntry::new(
                PathBuf::from("/tmp/x"),
                crate::config::DirectoryName::new("old").unwrap(),
                ContentHash::new("a".repeat(64)).unwrap(),
                false,
            ),
        );

        let p = plan(
            "test-skill",
            "target-dir",
            &config,
            &paths,
            &manifest,
            false,
            true, // force
        )
        .expect("--force must bypass D-A1 collision");
        assert!(matches!(p.action, ReassignAction::CopyAndRelink));
        assert!(p.force);
    }

    #[test]
    fn plan_same_content_collision_takes_relink_path() {
        let tmp = TempDir::new().unwrap();
        let paths = test_paths(&tmp);
        let config = make_config_with_dir(&tmp, "target-dir");

        // SAME content in both library and target.
        write_skill_in_dir(paths.library_dir(), "test-skill", "same content");
        let target_dir = tmp.path().join("target-dir");
        write_skill_in_dir(&target_dir, "test-skill", "same content");

        let mut manifest = Manifest::default();
        use crate::manifest::SkillEntry;
        use crate::validation::ContentHash;
        manifest.insert(
            SkillName::new("test-skill").unwrap(),
            SkillEntry::new(
                PathBuf::from("/tmp/x"),
                crate::config::DirectoryName::new("old").unwrap(),
                ContentHash::new("a".repeat(64)).unwrap(),
                false,
            ),
        );

        let p = plan(
            "test-skill",
            "target-dir",
            &config,
            &paths,
            &manifest,
            false,
            false,
        )
        .unwrap();
        assert!(matches!(p.action, ReassignAction::Relink));
    }

    #[test]
    fn execute_clears_previous_source_on_re_anchor() {
        let tmp = TempDir::new().unwrap();
        let paths = test_paths(&tmp);
        let config = make_config_with_dir(&tmp, "new-dir");

        write_skill_in_dir(paths.library_dir(), "orphan-skill", "library content");

        let mut manifest = Manifest::default();
        use crate::manifest::SkillEntry;
        use crate::validation::ContentHash;
        manifest.insert(
            SkillName::new("orphan-skill").unwrap(),
            SkillEntry::new_unowned(
                PathBuf::from("/tmp/old/orphan-skill"),
                ContentHash::new("a".repeat(64)).unwrap(),
                false,
                Some(crate::config::DirectoryName::new("removed-dir").unwrap()),
            ),
        );

        let p = plan(
            "orphan-skill",
            "new-dir",
            &config,
            &paths,
            &manifest,
            false,
            false,
        )
        .unwrap();

        let target_path = tmp.path().join("new-dir");
        execute(&p, &mut manifest, &target_path, false).unwrap();

        let entry = manifest.get("orphan-skill").unwrap();
        assert_eq!(
            entry.source_name,
            Some(crate::config::DirectoryName::new("new-dir").unwrap()),
            "re-anchor must set source_name"
        );
        assert_eq!(
            entry.previous_source, None,
            "re-anchor must clear previous_source per D-C1 closure"
        );
    }
}
