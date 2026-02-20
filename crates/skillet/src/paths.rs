//! Symlink path utilities — resolving relative symlink targets and comparing symlink destinations.

use std::path::{Path, PathBuf};

/// Resolve a symlink's raw target to an absolute path.
///
/// `read_link()` returns the raw stored target, which may be relative.
/// This function resolves relative targets against the symlink's parent directory.
pub fn resolve_symlink_target(link_path: &Path, raw_target: &Path) -> PathBuf {
    if raw_target.is_absolute() {
        raw_target.to_path_buf()
    } else {
        link_path.parent().unwrap_or(link_path).join(raw_target)
    }
}

/// Compare two paths for equivalence, using canonicalization when possible.
///
/// Falls back to `resolve_symlink_target` when the symlink target doesn't exist
/// (e.g., the original was deleted).
pub fn symlink_points_to(link_path: &Path, expected_target: &Path) -> bool {
    let raw_target = match std::fs::read_link(link_path) {
        Ok(t) => t,
        Err(_) => return false,
    };

    let resolved = std::fs::canonicalize(link_path)
        .unwrap_or_else(|_| resolve_symlink_target(link_path, &raw_target));
    let expected =
        std::fs::canonicalize(expected_target).unwrap_or_else(|_| expected_target.to_path_buf());

    resolved == expected
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::unix::fs as unix_fs;
    use tempfile::TempDir;

    #[test]
    fn resolve_absolute_target_unchanged() {
        let result = resolve_symlink_target(Path::new("/some/link"), Path::new("/absolute/target"));
        assert_eq!(result, PathBuf::from("/absolute/target"));
    }

    #[test]
    fn resolve_relative_target_against_parent() {
        let result = resolve_symlink_target(
            Path::new("/lib/skills/my-skill"),
            Path::new("../../sources/my-skill"),
        );
        assert_eq!(result, PathBuf::from("/lib/skills/../../sources/my-skill"));
    }

    #[test]
    fn symlink_points_to_matches_absolute() {
        let source = TempDir::new().unwrap();
        let library = TempDir::new().unwrap();

        let target_dir = source.path().join("skill");
        std::fs::create_dir_all(&target_dir).unwrap();

        let link = library.path().join("skill");
        unix_fs::symlink(&target_dir, &link).unwrap();

        assert!(symlink_points_to(&link, &target_dir));
    }

    #[test]
    fn symlink_points_to_matches_relative() {
        let tmp = TempDir::new().unwrap();

        let target_dir = tmp.path().join("sources/skill");
        std::fs::create_dir_all(&target_dir).unwrap();

        let lib_dir = tmp.path().join("library");
        std::fs::create_dir_all(&lib_dir).unwrap();

        let link = lib_dir.join("skill");
        // Create a relative symlink: ../sources/skill
        unix_fs::symlink(Path::new("../sources/skill"), &link).unwrap();

        // Should still match the absolute target
        assert!(symlink_points_to(&link, &target_dir));
    }

    #[test]
    fn symlink_points_to_detects_mismatch() {
        let tmp = TempDir::new().unwrap();

        let target_a = tmp.path().join("a");
        let target_b = tmp.path().join("b");
        std::fs::create_dir_all(&target_a).unwrap();
        std::fs::create_dir_all(&target_b).unwrap();

        let link = tmp.path().join("link");
        unix_fs::symlink(&target_a, &link).unwrap();

        assert!(!symlink_points_to(&link, &target_b));
    }
}
