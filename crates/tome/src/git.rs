//! Git subprocess operations for cloning and updating remote skill repositories.
//!
//! All git commands clear `GIT_DIR`, `GIT_WORK_TREE`, and `GIT_INDEX_FILE` environment
//! variables to prevent interference from the calling environment (e.g., running tome
//! inside a git worktree or from a git hook).

// Functions in this module are wired into the sync pipeline in a subsequent plan.
#![allow(dead_code)]

use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use sha2::{Digest, Sha256};

/// Run a git command in the given directory with env clearing, returning raw output.
fn git_command(repo_dir: &Path, args: &[&str]) -> Result<std::process::Output> {
    std::process::Command::new("git")
        .args(args)
        .current_dir(repo_dir)
        .env_remove("GIT_DIR")
        .env_remove("GIT_WORK_TREE")
        .env_remove("GIT_INDEX_FILE")
        .output()
        .with_context(|| format!("failed to run git {}", args.join(" ")))
}

/// Run a git command and bail on non-zero exit.
fn git_success(repo_dir: &Path, args: &[&str]) -> Result<()> {
    let output = git_command(repo_dir, args)?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("git {} failed: {}", args.join(" "), stderr.trim());
    }
    Ok(())
}

/// Run a git command and return trimmed stdout.
fn git_stdout(repo_dir: &Path, args: &[&str]) -> Result<String> {
    let output = git_command(repo_dir, args)?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("git {} failed: {}", args.join(" "), stderr.trim());
    }
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

/// Compute the cache directory path for a git repo URL.
///
/// Returns `repos_dir/<sha256(url)>`, where the hash is a 64-char lowercase hex string.
/// This is deterministic and path-safe.
pub(crate) fn repo_cache_dir(repos_dir: &Path, url: &str) -> PathBuf {
    let mut hasher = Sha256::new();
    hasher.update(url.as_bytes());
    let hash = hasher
        .finalize()
        .iter()
        .map(|b| format!("{:02x}", b))
        .collect::<String>();
    repos_dir.join(hash)
}

/// Determine the ref spec for `--branch` on clone or for `git fetch origin <ref>`.
///
/// - For `branch` or `tag`: returns the value (both use `--branch` on clone).
/// - For `rev` (SHA pinning): returns `None` — SHA pinning uses a different fetch flow.
/// - For all `None`: returns `None` — track remote HEAD.
pub(crate) fn ref_spec_for_config<'a>(
    branch: Option<&'a str>,
    tag: Option<&'a str>,
    rev: Option<&'a str>,
) -> Option<&'a str> {
    let _ = rev; // rev uses a different clone flow, not --branch
    branch.or(tag)
}

/// Clone a remote repo with shallow depth.
///
/// Uses `--depth 1` for bandwidth efficiency. Supports branch/tag pinning via `--branch`,
/// and SHA pinning via a post-clone `fetch + reset` flow.
pub(crate) fn clone_repo(
    url: &str,
    dest: &Path,
    branch: Option<&str>,
    tag: Option<&str>,
    rev: Option<&str>,
) -> Result<()> {
    let dest_str = dest
        .to_str()
        .context("clone destination path is not valid UTF-8")?;

    let ref_spec = ref_spec_for_config(branch, tag, rev);

    let mut args = vec!["clone", "--depth", "1"];
    if let Some(r) = ref_spec {
        args.extend(["--branch", r]);
    }
    args.push(url);
    args.push(dest_str);

    let output = std::process::Command::new("git")
        .args(&args)
        .env_remove("GIT_DIR")
        .env_remove("GIT_WORK_TREE")
        .env_remove("GIT_INDEX_FILE")
        .output()
        .context("failed to run git clone")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("git clone failed: {}", stderr.trim());
    }

    // For SHA-pinned repos: fetch the specific commit and reset to it
    if let Some(sha) = rev {
        git_success(dest, &["fetch", "--depth", "1", "origin", sha])?;
        git_success(dest, &["reset", "--hard", "FETCH_HEAD"])?;
    }

    Ok(())
}

/// Update an existing shallow clone by fetching and resetting.
///
/// Determines the fetch ref based on config: branch name, tag name, SHA, or HEAD.
/// Uses `git fetch --depth 1 origin <ref> && git reset --hard FETCH_HEAD`.
pub(crate) fn update_repo(
    repo_dir: &Path,
    branch: Option<&str>,
    tag: Option<&str>,
    rev: Option<&str>,
) -> Result<()> {
    let fetch_ref = branch.or(tag).or(rev).unwrap_or("HEAD");
    git_success(repo_dir, &["fetch", "--depth", "1", "origin", fetch_ref])?;
    git_success(repo_dir, &["reset", "--hard", "FETCH_HEAD"])?;
    Ok(())
}

/// Read the HEAD commit SHA from a git repository.
///
/// Returns the full 40-character hexadecimal SHA string.
pub(crate) fn read_head_sha(repo_dir: &Path) -> Result<String> {
    git_stdout(repo_dir, &["rev-parse", "HEAD"])
}

/// Compute the effective discovery path for a git directory.
///
/// If `subdir` is `Some`, returns `clone_path/<subdir>`. Otherwise returns `clone_path` unchanged.
pub(crate) fn effective_path(clone_path: &Path, subdir: Option<&str>) -> PathBuf {
    match subdir {
        Some(s) => clone_path.join(s),
        None => clone_path.to_path_buf(),
    }
}

/// Check whether git is available on the system.
///
/// Probes `git --version` with environment clearing. Returns `true` if exit code is 0.
pub(crate) fn is_git_available() -> bool {
    std::process::Command::new("git")
        .arg("--version")
        .env_remove("GIT_DIR")
        .env_remove("GIT_WORK_TREE")
        .env_remove("GIT_INDEX_FILE")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    // -- repo_cache_dir tests --

    #[test]
    fn repo_cache_dir_returns_sha256_hex_subdir() {
        let repos = Path::new("/tmp/repos");
        let result = repo_cache_dir(repos, "https://github.com/user/repo.git");
        let dirname = result.file_name().unwrap().to_str().unwrap();
        assert_eq!(dirname.len(), 64, "hash should be 64 hex chars");
        assert!(
            dirname.chars().all(|c| c.is_ascii_hexdigit()),
            "hash should be hex"
        );
        assert!(result.starts_with(repos));
    }

    #[test]
    fn repo_cache_dir_different_urls_different_paths() {
        let repos = Path::new("/tmp/repos");
        let a = repo_cache_dir(repos, "https://github.com/user/repo-a.git");
        let b = repo_cache_dir(repos, "https://github.com/user/repo-b.git");
        assert_ne!(a, b);
    }

    #[test]
    fn repo_cache_dir_deterministic() {
        let repos = Path::new("/tmp/repos");
        let url = "https://github.com/user/repo.git";
        let a = repo_cache_dir(repos, url);
        let b = repo_cache_dir(repos, url);
        assert_eq!(a, b);
    }

    // -- ref_spec_for_config tests --

    #[test]
    fn ref_spec_with_branch() {
        assert_eq!(ref_spec_for_config(Some("main"), None, None), Some("main"));
    }

    #[test]
    fn ref_spec_with_tag() {
        assert_eq!(ref_spec_for_config(None, Some("v1.0"), None), Some("v1.0"));
    }

    #[test]
    fn ref_spec_with_rev_returns_none() {
        assert_eq!(ref_spec_for_config(None, None, Some("abc123")), None);
    }

    #[test]
    fn ref_spec_all_none_returns_none() {
        assert_eq!(ref_spec_for_config(None, None, None), None);
    }

    // -- effective_path tests --

    #[test]
    fn effective_path_with_subdir() {
        let clone = Path::new("/tmp/repos/abc123");
        let result = effective_path(clone, Some("skills"));
        assert_eq!(result, PathBuf::from("/tmp/repos/abc123/skills"));
    }

    #[test]
    fn effective_path_without_subdir() {
        let clone = Path::new("/tmp/repos/abc123");
        let result = effective_path(clone, None);
        assert_eq!(result, PathBuf::from("/tmp/repos/abc123"));
    }

    // -- is_git_available test --

    #[test]
    fn git_is_available_on_dev_machine() {
        // This test verifies git is present; CI also has git
        assert!(is_git_available());
    }

    // -- read_head_sha test (requires real git repo) --

    #[test]
    fn read_head_sha_returns_40_char_hex() {
        let tmp = TempDir::new().unwrap();
        let dir = tmp.path();
        // Init a repo with a commit
        std::process::Command::new("git")
            .args(["init"])
            .current_dir(dir)
            .output()
            .unwrap();
        std::process::Command::new("git")
            .args(["config", "user.email", "test@test.com"])
            .current_dir(dir)
            .output()
            .unwrap();
        std::process::Command::new("git")
            .args(["config", "user.name", "Test"])
            .current_dir(dir)
            .output()
            .unwrap();
        std::fs::write(dir.join("file.txt"), "content").unwrap();
        std::process::Command::new("git")
            .args(["add", "-A"])
            .current_dir(dir)
            .output()
            .unwrap();
        std::process::Command::new("git")
            .args(["commit", "-m", "initial"])
            .current_dir(dir)
            .output()
            .unwrap();

        let sha = read_head_sha(dir).unwrap();
        assert_eq!(sha.len(), 40, "SHA should be 40 hex chars, got: {sha}");
        assert!(
            sha.chars().all(|c| c.is_ascii_hexdigit()),
            "SHA should be hex, got: {sha}"
        );
    }
}
