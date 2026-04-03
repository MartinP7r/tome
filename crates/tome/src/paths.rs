//! Filesystem path groupings and symlink utilities.
//!
//! Defines [`TomePaths`] to bundle `tome_home` and `library_dir` into a single value,
//! preventing accidental parameter swaps. Also provides helpers for resolving relative
//! symlink targets and comparing symlink destinations.

use std::path::{Path, PathBuf};

use anyhow::Result;

/// Resolved filesystem paths for a tome instance.
///
/// Groups `tome_home` (root of managed content), `library_dir` (skill storage),
/// and `config_dir` (where tome.toml, tome.lock, and .tome-manifest.json live)
/// into a single value to prevent accidental parameter swaps.
///
/// Config files may live at `tome_home` directly (default layout) or in a
/// `.tome/` subdirectory (custom repo layout). Smart detection picks the right
/// one based on which `tome.toml` exists.
#[derive(Debug, Clone)]
pub struct TomePaths {
    /// Root of everything tome manages. Typically `~/.tome/` or a custom repo root.
    tome_home: PathBuf,
    /// Directory where skill contents are stored. Typically `<tome_home>/skills/`.
    library_dir: PathBuf,
    /// Directory where config files live (tome.toml, tome.lock, .tome-manifest.json).
    /// Either `tome_home` itself (default) or `tome_home/.tome/` (custom repo).
    config_dir: PathBuf,
}

impl TomePaths {
    pub fn new(tome_home: PathBuf, library_dir: PathBuf) -> Result<Self> {
        anyhow::ensure!(
            !tome_home.as_os_str().is_empty(),
            "tome_home path cannot be empty"
        );
        anyhow::ensure!(
            !library_dir.as_os_str().is_empty(),
            "library_dir path cannot be empty"
        );
        anyhow::ensure!(
            tome_home.is_absolute(),
            "tome_home must be an absolute path: {}",
            tome_home.display()
        );
        anyhow::ensure!(
            library_dir.is_absolute(),
            "library_dir must be an absolute path: {}",
            library_dir.display()
        );
        let config_dir = crate::config::resolve_config_dir(&tome_home);
        Ok(Self {
            tome_home,
            library_dir,
            config_dir,
        })
    }

    /// Returns the tome home directory path (root of managed content).
    pub fn tome_home(&self) -> &Path {
        &self.tome_home
    }

    /// Returns the library directory path.
    pub fn library_dir(&self) -> &Path {
        &self.library_dir
    }

    /// Returns the config directory path (where tome.toml, lockfile, manifest live).
    pub fn config_dir(&self) -> &Path {
        &self.config_dir
    }

    /// Path to the config file.
    pub fn config_path(&self) -> PathBuf {
        self.config_dir.join("tome.toml")
    }

    /// Path to the manifest file.
    pub fn manifest_path(&self) -> PathBuf {
        self.config_dir.join(crate::manifest::MANIFEST_FILENAME)
    }

    /// Path to the lockfile.
    pub fn lockfile_path(&self) -> PathBuf {
        self.config_dir.join(crate::lockfile::LOCKFILE_NAME)
    }
}

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

    let resolved = std::fs::canonicalize(link_path).unwrap_or_else(|e| {
        if link_path.exists() {
            eprintln!(
                "warning: could not canonicalize {}: {}",
                link_path.display(),
                e
            );
        }
        resolve_symlink_target(link_path, &raw_target)
    });
    let expected = std::fs::canonicalize(expected_target).unwrap_or_else(|e| {
        if expected_target.exists() {
            eprintln!(
                "warning: could not canonicalize {}: {}",
                expected_target.display(),
                e
            );
        }
        expected_target.to_path_buf()
    });

    resolved == expected
}

/// Collapse the user's home directory prefix to `~/` for display.
pub(crate) fn collapse_home(path: &Path) -> String {
    collapse_home_path(path).display().to_string()
}

/// Collapse the user's home directory prefix to `~/`, returning a PathBuf.
/// Used to write portable paths in config files.
pub(crate) fn collapse_home_path(path: &Path) -> PathBuf {
    if let Ok(home) = std::env::var("HOME") {
        let home_path = Path::new(&home);
        if let Ok(rel) = path.strip_prefix(home_path) {
            return PathBuf::from("~").join(rel);
        }
    }
    path.to_path_buf()
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

    #[test]
    fn tome_paths_new_stores_fields() {
        let paths = TomePaths::new(
            PathBuf::from("/home/.tome"),
            PathBuf::from("/home/.tome/skills"),
        )
        .unwrap();
        assert_eq!(paths.tome_home(), Path::new("/home/.tome"));
        assert_eq!(paths.library_dir(), Path::new("/home/.tome/skills"));
    }

    #[test]
    fn tome_paths_manifest_path() {
        let paths = TomePaths::new(
            PathBuf::from("/home/.tome"),
            PathBuf::from("/home/.tome/skills"),
        )
        .unwrap();
        assert_eq!(
            paths.manifest_path(),
            PathBuf::from("/home/.tome/.tome-manifest.json")
        );
    }

    #[test]
    fn tome_paths_lockfile_path() {
        let paths = TomePaths::new(
            PathBuf::from("/home/.tome"),
            PathBuf::from("/home/.tome/skills"),
        )
        .unwrap();
        assert_eq!(
            paths.lockfile_path(),
            PathBuf::from("/home/.tome/tome.lock")
        );
    }

    #[test]
    fn tome_paths_rejects_empty_tome_home() {
        let result = TomePaths::new(PathBuf::from(""), PathBuf::from("/home/.tome/skills"));
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("tome_home path cannot be empty")
        );
    }

    #[test]
    fn tome_paths_rejects_empty_library_dir() {
        let result = TomePaths::new(PathBuf::from("/home/.tome"), PathBuf::from(""));
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("library_dir path cannot be empty")
        );
    }

    #[test]
    fn tome_paths_rejects_relative_tome_home() {
        let result = TomePaths::new(
            PathBuf::from("relative/path"),
            PathBuf::from("/home/.tome/skills"),
        );
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("tome_home must be an absolute path")
        );
    }

    #[test]
    fn tome_paths_rejects_relative_library_dir() {
        let result = TomePaths::new(PathBuf::from("/home/.tome"), PathBuf::from("relative/path"));
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("library_dir must be an absolute path")
        );
    }

    #[test]
    fn tome_paths_accepts_both_absolute() {
        let result = TomePaths::new(
            PathBuf::from("/home/.tome"),
            PathBuf::from("/home/.tome/skills"),
        );
        assert!(result.is_ok());
    }
}
