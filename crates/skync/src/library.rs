use anyhow::{Context, Result};
use std::os::unix::fs as unix_fs;
use std::path::Path;

use crate::discover::DiscoveredSkill;
use crate::paths::symlink_points_to;

/// Result of a consolidation operation.
#[derive(Debug, Default)]
pub struct ConsolidateResult {
    pub created: usize,
    pub unchanged: usize,
    pub updated: usize,
}

/// Consolidate discovered skills into the library directory via symlinks.
///
/// Each skill gets a symlink: `library_dir/{skill_name}` → `{skill.path}`
pub fn consolidate(
    skills: &[DiscoveredSkill],
    library_dir: &Path,
    dry_run: bool,
) -> Result<ConsolidateResult> {
    if !dry_run {
        std::fs::create_dir_all(library_dir)
            .with_context(|| format!("failed to create library dir {}", library_dir.display()))?;
    }

    let mut result = ConsolidateResult::default();

    for skill in skills {
        let link_path = library_dir.join(&skill.name);

        if link_path.is_symlink() {
            if symlink_points_to(&link_path, &skill.path) {
                result.unchanged += 1;
                continue;
            }

            // Points somewhere else — update it
            if !dry_run {
                std::fs::remove_file(&link_path).with_context(|| {
                    format!("failed to remove old symlink {}", link_path.display())
                })?;
                unix_fs::symlink(&skill.path, &link_path).with_context(|| {
                    format!(
                        "failed to symlink {} -> {}",
                        link_path.display(),
                        skill.path.display()
                    )
                })?;
            }
            result.updated += 1;
        } else if link_path.exists() {
            // Something else exists at this path (not a symlink) — skip
            eprintln!(
                "warning: {} exists and is not a symlink, skipping",
                link_path.display()
            );
            continue;
        } else {
            // Create new symlink
            if !dry_run {
                unix_fs::symlink(&skill.path, &link_path).with_context(|| {
                    format!(
                        "failed to symlink {} -> {}",
                        link_path.display(),
                        skill.path.display()
                    )
                })?;
            }
            result.created += 1;
        }
    }

    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn make_skill(dir: &Path, name: &str) -> DiscoveredSkill {
        let skill_dir = dir.join(name);
        std::fs::create_dir_all(&skill_dir).unwrap();
        std::fs::write(skill_dir.join("SKILL.md"), "# test").unwrap();
        DiscoveredSkill {
            name: name.to_string(),
            path: skill_dir,
            source_name: "test".into(),
        }
    }

    #[test]
    fn consolidate_creates_symlinks() {
        let source = TempDir::new().unwrap();
        let library = TempDir::new().unwrap();
        let skill = make_skill(source.path(), "my-skill");

        let result = consolidate(&[skill], library.path(), false).unwrap();
        assert_eq!(result.created, 1);
        assert_eq!(result.unchanged, 0);

        let link = library.path().join("my-skill");
        assert!(link.is_symlink());
    }

    #[test]
    fn consolidate_idempotent() {
        let source = TempDir::new().unwrap();
        let library = TempDir::new().unwrap();
        let skill = make_skill(source.path(), "my-skill");

        consolidate(std::slice::from_ref(&skill), library.path(), false).unwrap();
        let result = consolidate(std::slice::from_ref(&skill), library.path(), false).unwrap();
        assert_eq!(result.created, 0);
        assert_eq!(result.unchanged, 1);
    }

    #[test]
    fn consolidate_idempotent_with_relative_symlink() {
        use std::os::unix::fs as unix_fs;

        let tmp = TempDir::new().unwrap();
        let source_dir = tmp.path().join("sources/my-skill");
        std::fs::create_dir_all(&source_dir).unwrap();
        std::fs::write(source_dir.join("SKILL.md"), "# test").unwrap();

        let lib_dir = tmp.path().join("library");
        std::fs::create_dir_all(&lib_dir).unwrap();

        // Manually create a relative symlink: library/my-skill -> ../sources/my-skill
        unix_fs::symlink(
            std::path::Path::new("../sources/my-skill"),
            lib_dir.join("my-skill"),
        )
        .unwrap();

        let skill = DiscoveredSkill {
            name: "my-skill".to_string(),
            path: source_dir,
            source_name: "test".into(),
        };

        let result = consolidate(&[skill], &lib_dir, false).unwrap();
        assert_eq!(
            result.unchanged, 1,
            "relative symlink should be recognized as matching"
        );
        assert_eq!(result.updated, 0);
        assert_eq!(result.created, 0);
    }

    #[test]
    fn consolidate_dry_run_no_changes() {
        let source = TempDir::new().unwrap();
        let library = TempDir::new().unwrap();
        let skill = make_skill(source.path(), "my-skill");

        let result = consolidate(&[skill], library.path(), true).unwrap();
        assert_eq!(result.created, 1);

        // Symlink should NOT exist
        assert!(!library.path().join("my-skill").exists());
    }

    #[test]
    fn consolidate_updates_changed_target() {
        let source1 = TempDir::new().unwrap();
        let source2 = TempDir::new().unwrap();
        let library = TempDir::new().unwrap();

        let skill1 = make_skill(source1.path(), "my-skill");
        consolidate(&[skill1], library.path(), false).unwrap();

        let skill2 = make_skill(source2.path(), "my-skill");
        let result = consolidate(std::slice::from_ref(&skill2), library.path(), false).unwrap();
        assert_eq!(result.updated, 1);

        let actual_target = std::fs::read_link(library.path().join("my-skill")).unwrap();
        assert_eq!(actual_target, skill2.path);
    }

    #[test]
    fn consolidate_dry_run_doesnt_create_dir() {
        let tmp = TempDir::new().unwrap();
        let nonexistent_lib = tmp.path().join("does-not-exist");
        let source = TempDir::new().unwrap();
        let skill = make_skill(source.path(), "my-skill");

        let result = consolidate(&[skill], &nonexistent_lib, true).unwrap();
        assert_eq!(result.created, 1);
        assert!(!nonexistent_lib.exists());
    }

    #[test]
    fn consolidate_skips_non_symlink_collision() {
        let source = TempDir::new().unwrap();
        let library = TempDir::new().unwrap();

        let skill = make_skill(source.path(), "my-skill");

        // Pre-create a regular file at the library link path (collision)
        std::fs::write(library.path().join("my-skill"), "not a symlink").unwrap();

        let result = consolidate(&[skill], library.path(), false).unwrap();
        assert_eq!(result.created, 0);
        assert_eq!(result.unchanged, 0);
        assert_eq!(result.updated, 0);

        // The regular file should be unchanged
        let content = std::fs::read_to_string(library.path().join("my-skill")).unwrap();
        assert_eq!(content, "not a symlink");
    }
}
