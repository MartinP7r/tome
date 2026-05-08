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
use crate::manifest::{Manifest, SkillEntry};
use crate::paths::TomePaths;
use crate::validation::ContentHash;

/// What needs to happen to reassign a skill.
#[derive(Debug)]
pub(crate) enum ReassignAction {
    /// Skill already exists in target dir — just update manifest.
    Relink,
    /// Skill not in target dir — copy from library, then update manifest.
    CopyAndRelink,
}

/// Filesystem and manifest state captured by `plan()` so that `execute()`
/// never has to re-read the same data — eliminating the plan/execute drift
/// risk per HARD-19 (closes #430).
///
/// All fields are observations made during `plan()`. Callers must NOT
/// mutate this struct after construction; production callers treat it as
/// an immutable witness.
///
/// Why "snapshot at plan, consume at execute": between the two phases a
/// concurrent `tome` process or hand-edit could mutate the manifest or
/// library. Reading once means execute() acts on a consistent view, even
/// if the live state has drifted.
///
/// Field-level `dead_code` allows: `target_existed_at_plan`,
/// `source_hash_at_plan`, and `target_hash_at_plan` are forensic
/// observations captured for testability and future consumers (e.g.
/// `tome doctor` could surface them). `execute()` only consumes
/// `manifest_entry_at_plan` today; the other three are pinned by unit
/// tests. Once a production consumer arrives the attrs come off.
#[derive(Debug, Clone)]
pub(crate) struct PreReassignState {
    /// Manifest entry as observed at plan time. `None` only if the skill
    /// is missing from the manifest — but `plan()` already bails in that
    /// case, so production callers always see `Some`.
    pub manifest_entry_at_plan: Option<SkillEntry>,
    /// Whether `<target_dir>/<skill>/SKILL.md` existed at plan time.
    /// Drives the Relink-vs-CopyAndRelink decision and the D-A1 collision
    /// check.
    #[allow(dead_code)]
    pub target_existed_at_plan: bool,
    /// Library-side content hash at plan time. Used for the D-A1 same-vs-
    /// different content compare. Always `Some` when the library copy
    /// exists (production path); `None` only on the bail path.
    #[allow(dead_code)]
    pub source_hash_at_plan: Option<ContentHash>,
    /// Target-side content hash at plan time. `Some` when
    /// `target_existed_at_plan` is true; `None` otherwise. The pair
    /// (`source_hash`, `target_hash`) drives D-A1.
    #[allow(dead_code)]
    pub target_hash_at_plan: Option<ContentHash>,
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
    /// Snapshot of the manifest + library + target state at plan time.
    /// `execute()` consumes this rather than re-reading, closing the
    /// plan/execute drift risk per HARD-19.
    pub pre_state: PreReassignState,
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

    // HARD-19: build the snapshot WHILE we make decisions. Each filesystem /
    // manifest read flows into `pre_state` so `execute()` never re-reads.
    let target_existed_at_plan = target_skill_md.exists();
    let mut source_hash_at_plan: Option<ContentHash> = None;
    let mut target_hash_at_plan: Option<ContentHash> = None;

    let action = if target_existed_at_plan {
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
        target_hash_at_plan = Some(target_hash.clone());
        source_hash_at_plan = Some(library_hash.clone());

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
        // Library hash is still useful for the snapshot even on the
        // CopyAndRelink path: it pins the content the eventual copy will
        // be made from. If the library copy vanished after the manifest
        // entry was inserted (corruption / hand-edit), bail before we
        // commit to a plan we can't execute.
        if library_skill_path.is_dir() {
            source_hash_at_plan = Some(
                crate::manifest::hash_directory(&library_skill_path).with_context(|| {
                    format!(
                        "failed to hash library skill {}",
                        library_skill_path.display()
                    )
                })?,
            );
        }
        ReassignAction::CopyAndRelink
    };

    let pre_state = PreReassignState {
        manifest_entry_at_plan: Some(entry.clone()),
        target_existed_at_plan,
        source_hash_at_plan,
        target_hash_at_plan,
    };

    Ok(ReassignPlan {
        skill_name: SkillName::new(skill_name)?,
        from_directory,
        to_directory: to_dir_name,
        action,
        library_skill_path,
        is_fork,
        force,
        pre_state,
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
///
/// Per HARD-19 (closes #430): consume `plan.pre_state` rather than
/// re-reading the manifest or library. The only writes here are the
/// directory copy (for the CopyAndRelink action) and the manifest mutation;
/// neither performs a fresh read that could disagree with `plan()`.
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

    // Copy files if needed.
    if matches!(plan.action, ReassignAction::CopyAndRelink) {
        let dest = target_dir_path.join(plan.skill_name.as_str());
        copy_dir_recursive(&plan.library_skill_path, &dest)
            .with_context(|| format!("failed to copy skill to {}", dest.display()))?;
    }

    // HARD-19: insert the planned post-state straight from the snapshot.
    // The previous flow re-read the manifest entry via `update_source_name`
    // (which checks the live `source_name`) and then mutated it. If the
    // live manifest had drifted between plan() and execute(), the
    // re-anchor would be applied on top of stale state. By starting from
    // `plan.pre_state.manifest_entry_at_plan` (cloned at plan time) we
    // set `source_name` and `previous_source` deterministically — every
    // observation execute() needs comes from `plan.pre_state.<field>`,
    // never from the live manifest or a fresh filesystem stat.
    let mut planned_entry = plan
        .pre_state
        .manifest_entry_at_plan
        .clone()
        .ok_or_else(|| {
            anyhow::anyhow!(
                "BUG: plan().pre_state.manifest_entry_at_plan was None for skill '{}'; \
             plan() should bail before producing such a snapshot",
                plan.skill_name.as_str()
            )
        })?;

    // Defensive invariant: when `plan.pre_state.target_existed_at_plan` is
    // true, the snapshot must also carry both content hashes — that pair is
    // what plan() used for the D-A1 same-vs-different compare. A mismatch
    // would mean the snapshot is internally inconsistent and execute must
    // not silently proceed (the only expected mismatch path is the
    // CopyAndRelink-with-no-target case, where target_hash is legitimately
    // None and the assertion is skipped).
    debug_assert!(
        !plan.pre_state.target_existed_at_plan
            || (plan.pre_state.source_hash_at_plan.is_some()
                && plan.pre_state.target_hash_at_plan.is_some()),
        "snapshot invariant violated: target_existed_at_plan implies both hashes"
    );

    // D-C1 closure: re-anchoring an Unowned skill clears previous_source;
    // re-anchoring an Owned skill leaves no breadcrumb because the skill is
    // owned again. Either way, previous_source becomes None.
    planned_entry.source_name = Some(plan.to_directory.clone());
    planned_entry.previous_source = None;
    manifest.insert(plan.skill_name.clone(), planned_entry);

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

    // ---- HARD-19 read-once snapshot (closes #430) ---------------------
    // The drift risk: between `plan()` and `execute()`, the manifest entry
    // or library directory could be mutated by another tome process or by
    // a hand-edit. `plan()` already inspects the manifest and computes
    // content hashes — capturing that state in a `PreReassignState`
    // snapshot means `execute()` consumes the snapshot rather than
    // re-reading and silently drifting.

    #[test]
    fn pre_state_captured_at_plan_time() {
        let tmp = TempDir::new().unwrap();
        let paths = test_paths(&tmp);
        let config = make_config_with_dir(&tmp, "target-dir");

        // Library has a real skill so plan() hashes it.
        write_skill_in_dir(paths.library_dir(), "test-skill", "library content");

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

        // Snapshot must capture the manifest entry observed at plan time.
        let snap = &p.pre_state;
        let entry = snap
            .manifest_entry_at_plan
            .as_ref()
            .expect("plan() must capture the manifest entry");
        assert_eq!(
            entry.source_name,
            Some(DirectoryName::new("old-dir").unwrap()),
            "snapshot must mirror the manifest at plan time"
        );

        // Target dir does NOT have the skill yet (CopyAndRelink path), so
        // pre_state.target_existed_at_plan must be false and target_hash None.
        assert!(
            !snap.target_existed_at_plan,
            "target dir is empty at plan time"
        );
        assert!(snap.target_hash_at_plan.is_none());
    }

    #[test]
    fn pre_state_captures_target_hash_when_target_skill_exists() {
        let tmp = TempDir::new().unwrap();
        let paths = test_paths(&tmp);
        let config = make_config_with_dir(&tmp, "target-dir");

        // Both library and target have the same content (D-A1 Relink path).
        write_skill_in_dir(paths.library_dir(), "test-skill", "same content");
        let target_dir = tmp.path().join("target-dir");
        write_skill_in_dir(&target_dir, "test-skill", "same content");

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
        let snap = &p.pre_state;
        assert!(
            snap.target_existed_at_plan,
            "target skill exists at plan time — must be captured"
        );
        let target_hash = snap
            .target_hash_at_plan
            .as_ref()
            .expect("target hash must be captured when target exists");
        let library_hash = snap
            .source_hash_at_plan
            .as_ref()
            .expect("library hash must be captured");
        assert_eq!(
            target_hash, library_hash,
            "Relink path → snapshot hashes must agree (D-A1)"
        );
    }

    #[test]
    fn execute_consumes_pre_state_not_live() {
        // The drift contract: a manifest mutation between plan() and
        // execute() must NOT change execute's behavior. execute()
        // operates on the snapshot.
        let tmp = TempDir::new().unwrap();
        let paths = test_paths(&tmp);
        let config = make_config_with_dir(&tmp, "target-dir");

        write_skill_in_dir(paths.library_dir(), "test-skill", "library content");

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

        // Mutate the live manifest between plan() and execute(): change
        // source_name to a totally different directory. execute() must
        // still re-anchor the entry to `to_directory` (the planned target),
        // not to the mutated value, because the snapshot pinned the
        // pre-execute observation.
        manifest.update_source_name("test-skill", &DirectoryName::new("other-dir").unwrap());

        let target_path = tmp.path().join("target-dir");
        execute(&p, &mut manifest, &target_path, false).unwrap();

        // Final state must reflect the planned re-anchor — drift is closed.
        let entry = manifest.get("test-skill").unwrap();
        assert_eq!(
            entry.source_name,
            Some(DirectoryName::new("target-dir").unwrap()),
            "execute must re-anchor to planned target, ignoring live mutations"
        );
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
