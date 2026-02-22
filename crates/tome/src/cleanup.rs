//! Remove broken and stale symlinks from the library and target directories.

use anyhow::{Context, Result};
use std::path::Path;

use crate::paths::resolve_symlink_target;

/// Result of cleanup operation.
#[derive(Debug, Default)]
pub struct CleanupResult {
    pub removed_from_library: usize,
}

/// Remove stale symlinks from the library (broken or pointing to deleted sources).
pub fn cleanup_library(library_dir: &Path, dry_run: bool) -> Result<CleanupResult> {
    let mut result = CleanupResult::default();

    if !library_dir.is_dir() {
        return Ok(result);
    }

    let entries = std::fs::read_dir(library_dir)
        .with_context(|| format!("failed to read library dir {}", library_dir.display()))?;

    for entry in entries {
        let entry =
            entry.with_context(|| format!("failed to read entry in {}", library_dir.display()))?;
        let path = entry.path();

        if path.is_symlink() {
            let raw_target = std::fs::read_link(&path)
                .with_context(|| format!("failed to read symlink {}", path.display()))?;
            let target = resolve_symlink_target(&path, &raw_target);
            // Check if the symlink target still exists
            if !target.exists() {
                if !dry_run {
                    std::fs::remove_file(&path).with_context(|| {
                        format!("failed to remove broken symlink {}", path.display())
                    })?;
                }
                result.removed_from_library += 1;
            }
        }
    }

    Ok(result)
}

/// Remove stale symlinks from a target directory.
pub fn cleanup_target(target_dir: &Path, library_dir: &Path, dry_run: bool) -> Result<usize> {
    if !target_dir.is_dir() {
        return Ok(0);
    }

    let mut removed = 0;

    let entries = std::fs::read_dir(target_dir)
        .with_context(|| format!("failed to read target dir {}", target_dir.display()))?;

    for entry in entries {
        let entry =
            entry.with_context(|| format!("failed to read entry in {}", target_dir.display()))?;
        let path = entry.path();

        if path.is_symlink() {
            let raw_target = std::fs::read_link(&path)
                .with_context(|| format!("failed to read symlink {}", path.display()))?;
            let target = resolve_symlink_target(&path, &raw_target);

            // Remove if it points into the library dir but the library entry is gone
            if target.starts_with(library_dir) && !target.exists() {
                if !dry_run {
                    std::fs::remove_file(&path).with_context(|| {
                        format!("failed to remove stale symlink {}", path.display())
                    })?;
                }
                removed += 1;
            }
        }
    }

    Ok(removed)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::unix::fs as unix_fs;
    use tempfile::TempDir;

    #[test]
    fn cleanup_removes_broken_library_symlinks() {
        let library = TempDir::new().unwrap();
        let source = TempDir::new().unwrap();

        // Create a valid skill + symlink
        let skill_dir = source.path().join("my-skill");
        std::fs::create_dir_all(&skill_dir).unwrap();
        unix_fs::symlink(&skill_dir, library.path().join("my-skill")).unwrap();

        // Create a broken symlink
        unix_fs::symlink("/nonexistent/path", library.path().join("broken")).unwrap();

        let result = cleanup_library(library.path(), false).unwrap();
        assert_eq!(result.removed_from_library, 1);
        assert!(library.path().join("my-skill").exists());
        assert!(!library.path().join("broken").exists());
    }

    #[test]
    fn cleanup_dry_run_preserves_links() {
        let library = TempDir::new().unwrap();
        unix_fs::symlink("/nonexistent", library.path().join("broken")).unwrap();

        let result = cleanup_library(library.path(), true).unwrap();
        assert_eq!(result.removed_from_library, 1);
        // Should still exist in dry run
        assert!(library.path().join("broken").is_symlink());
    }

    #[test]
    fn cleanup_target_removes_stale_links() {
        let library = TempDir::new().unwrap();
        let target = TempDir::new().unwrap();

        // Symlink in target pointing to a non-existent library entry
        let phantom = library.path().join("deleted-skill");
        unix_fs::symlink(&phantom, target.path().join("deleted-skill")).unwrap();

        let removed = cleanup_target(target.path(), library.path(), false).unwrap();
        assert_eq!(removed, 1);
    }

    #[test]
    fn cleanup_handles_relative_symlinks() {
        let tmp = TempDir::new().unwrap();
        let library = tmp.path().join("library");
        std::fs::create_dir_all(&library).unwrap();

        // Valid relative symlink: library/my-skill -> ../sources/my-skill
        let source_dir = tmp.path().join("sources/my-skill");
        std::fs::create_dir_all(&source_dir).unwrap();
        unix_fs::symlink(
            std::path::Path::new("../sources/my-skill"),
            library.join("my-skill"),
        )
        .unwrap();

        // Broken relative symlink: library/gone -> ../sources/gone (doesn't exist)
        unix_fs::symlink(
            std::path::Path::new("../sources/gone"),
            library.join("gone"),
        )
        .unwrap();

        let result = cleanup_library(&library, false).unwrap();
        assert_eq!(result.removed_from_library, 1);
        // Valid relative symlink should be preserved
        assert!(library.join("my-skill").is_symlink());
        // Broken one should be removed
        assert!(!library.join("gone").exists());
    }

    #[test]
    fn cleanup_target_preserves_external_symlinks() {
        let library = TempDir::new().unwrap();
        let target = TempDir::new().unwrap();

        // Broken symlink pointing INTO library dir (should be removed)
        let library_phantom = library.path().join("deleted-skill");
        unix_fs::symlink(&library_phantom, target.path().join("library-link")).unwrap();

        // Broken symlink pointing OUTSIDE library dir (should be preserved)
        unix_fs::symlink("/some/external/path", target.path().join("external-link")).unwrap();

        let removed = cleanup_target(target.path(), library.path(), false).unwrap();
        assert_eq!(removed, 1);
        // Library-pointing broken link was removed
        assert!(!target.path().join("library-link").exists());
        // External broken link is preserved
        assert!(target.path().join("external-link").is_symlink());
    }
}
