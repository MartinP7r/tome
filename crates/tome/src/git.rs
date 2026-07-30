//! Git subprocess operations for cloning and updating remote skill repositories.
//!
//! All git commands clear `GIT_DIR`, `GIT_WORK_TREE`, and `GIT_INDEX_FILE` environment
//! variables to prevent interference from the calling environment (e.g., running tome
//! inside a git worktree or from a git hook).

use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use sha2::{Digest, Sha256};

use crate::errors::{DomainErrorKind, WithDomainKind};
use crate::progress::{CancelToken, ProgressEvent, ProgressSink};

/// Run a git command in the given directory with env clearing, returning raw output.
///
/// `GIT_CEILING_DIRECTORIES` is set to `repo_dir`'s **parent** so git's
/// repository discovery cannot walk above the cache directory. Without it,
/// running a command in a directory that is not itself a repository silently
/// targets the nearest ancestor repository — and the repo cache lives inside the
/// library, which is commonly a git repo itself, so a `reset --hard` would land
/// on the user's library instead of the cache.
///
/// The value must be the parent, not `repo_dir`: the variable lists directories
/// git may not *chdir up into*, and discovery starts already inside `repo_dir`.
/// Listing `repo_dir` therefore permits the very first upward step, which is the
/// one that escapes. Verified against git's behaviour, not assumed.
fn git_command(repo_dir: &Path, args: &[&str]) -> Result<std::process::Output> {
    let mut cmd = std::process::Command::new("git");
    cmd.args(args)
        .current_dir(repo_dir)
        .env_remove("GIT_DIR")
        .env_remove("GIT_WORK_TREE")
        .env_remove("GIT_INDEX_FILE");
    if let Some(parent) = repo_dir.parent() {
        cmd.env("GIT_CEILING_DIRECTORIES", parent);
    }
    cmd.output()
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

/// Whether `dir` is the root of a git repository.
///
/// Checks for a `.git` entry directly inside `dir` — deliberately *not* using
/// `git rev-parse`, which walks up to ancestor repositories and would report
/// `true` for any directory nested inside one. The repo cache lives inside the
/// library, which is frequently a git repo, so upward-walking checks are unsafe
/// here.
///
/// `.git` may be a directory (normal clone) or a file (worktree/submodule
/// gitlink), so both are accepted.
pub(crate) fn is_git_repo(dir: &Path) -> bool {
    dir.join(".git").exists()
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
///
/// `sink` receives a [`ProgressEvent::GitCloneProgress`] when the clone begins
/// (D-11: the git long-op family adopts the [`ProgressSink`] vocabulary now).
/// `cancel` is checked before the clone subprocess launches so a cancellation
/// requested at the previous stage boundary is honored before any network I/O
/// (D-12). `git` itself is a blocking subprocess — we emit a coarse
/// "started clone" event rather than streaming git's byte counter; the typed
/// event shape (`received: u64`) is ready for a future packet-progress parser
/// without changing the signature.
pub(crate) fn clone_repo(
    url: &str,
    dest: &Path,
    branch: Option<&str>,
    tag: Option<&str>,
    rev: Option<&str>,
    sink: &dyn ProgressSink,
    cancel: &CancelToken,
) -> Result<()> {
    // Tag every failure of this op with the `Git` sentinel (CORE-05 / D-14) so
    // the GUI boundary classifies it as `ErrorCode::Git` via downcast. The tag
    // is transparent — the human-readable `{e:#}` chain (and the CLI's
    // warn-and-continue messages) are unchanged.
    clone_repo_inner(url, dest, branch, tag, rev, sink, cancel)
        .with_domain_kind(DomainErrorKind::Git)
}

fn clone_repo_inner(
    url: &str,
    dest: &Path,
    branch: Option<&str>,
    tag: Option<&str>,
    rev: Option<&str>,
    sink: &dyn ProgressSink,
    cancel: &CancelToken,
) -> Result<()> {
    if cancel.is_cancelled() {
        anyhow::bail!("git clone cancelled before start");
    }

    let dest_str = dest
        .to_str()
        .context("clone destination path is not valid UTF-8")?;

    // Derive a stable directory label from the cache path so the GUI can
    // associate the byte counter with the directory being cloned. The cache
    // dir basename is the SHA-256 of the URL (see `repo_cache_dir`); the URL
    // itself is the most human-meaningful identifier, so emit that.
    sink.emit(ProgressEvent::GitCloneProgress {
        directory: url.to_string(),
        received: 0,
    });

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
///
/// Mirrors [`clone_repo`]'s progress + cancellation contract (D-11/D-12):
/// emits a [`ProgressEvent::GitCloneProgress`] when the fetch begins and
/// checks `cancel` before launching the subprocess.
pub(crate) fn update_repo(
    repo_dir: &Path,
    branch: Option<&str>,
    tag: Option<&str>,
    rev: Option<&str>,
    sink: &dyn ProgressSink,
    cancel: &CancelToken,
) -> Result<()> {
    // Tag every failure with the `Git` sentinel (CORE-05 / D-14); transparent to
    // the CLI's `{e:#}` output (mirrors `clone_repo`).
    update_repo_inner(repo_dir, branch, tag, rev, sink, cancel)
        .with_domain_kind(DomainErrorKind::Git)
}

fn update_repo_inner(
    repo_dir: &Path,
    branch: Option<&str>,
    tag: Option<&str>,
    rev: Option<&str>,
    sink: &dyn ProgressSink,
    cancel: &CancelToken,
) -> Result<()> {
    if cancel.is_cancelled() {
        anyhow::bail!("git update cancelled before start");
    }
    // Refuse to fetch/reset unless the directory is itself a repository. A
    // previous clone that failed part-way leaves the cache directory present
    // but without `.git`; updating it would let git discovery escape upward and
    // `reset --hard` the enclosing repository. Callers treat this as
    // "needs a fresh clone".
    if !is_git_repo(repo_dir) {
        anyhow::bail!(
            "{} exists but is not a git repository (incomplete clone); a fresh clone is required",
            repo_dir.display()
        );
    }
    sink.emit(ProgressEvent::GitCloneProgress {
        directory: repo_dir.to_string_lossy().into_owned(),
        received: 0,
    });
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
    use crate::progress::NullSink;
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

    // -- is_git_repo / enclosing-repo safety tests --

    #[test]
    fn is_git_repo_false_for_plain_directory() {
        let tmp = TempDir::new().unwrap();
        assert!(!is_git_repo(tmp.path()));
    }

    #[test]
    fn is_git_repo_false_for_directory_nested_inside_a_repo() {
        // The repo cache lives inside the library, which is often a git repo.
        // An upward-walking check (`git rev-parse`) would answer `true` here;
        // that is exactly the confusion that let `reset --hard` hit the library.
        let outer = TempDir::new().unwrap();
        std::fs::create_dir(outer.path().join(".git")).unwrap();
        let nested = outer.path().join("repos").join("deadbeef");
        std::fs::create_dir_all(&nested).unwrap();

        assert!(is_git_repo(outer.path()), "outer is a repo");
        assert!(
            !is_git_repo(&nested),
            "nested cache dir must not count as a repo just because an ancestor is one"
        );
    }

    #[test]
    fn is_git_repo_true_when_dot_git_is_a_file() {
        // Worktrees and submodules use a `.git` *file* containing a gitdir pointer.
        let tmp = TempDir::new().unwrap();
        std::fs::write(tmp.path().join(".git"), "gitdir: /elsewhere\n").unwrap();
        assert!(is_git_repo(tmp.path()));
    }

    #[test]
    fn update_repo_refuses_incomplete_clone_and_leaves_enclosing_repo_untouched() {
        // Regression: a clone that failed part-way leaves the cache directory
        // present but without `.git`. Updating it used to run
        // `git fetch && git reset --hard FETCH_HEAD` there; git discovery walked
        // up to the enclosing library repository and reset the user's work.
        let library = TempDir::new().unwrap();
        run_git(library.path(), &["init", "--initial-branch", "main"]);
        configure_test_identity(library.path());
        std::fs::write(library.path().join("tome.toml"), "committed = true\n").unwrap();
        run_git(library.path(), &["add", "."]);
        run_git(library.path(), &["commit", "-m", "initial"]);

        // Uncommitted local edit, standing in for the user's work.
        std::fs::write(library.path().join("tome.toml"), "locally = edited\n").unwrap();

        // Empty cache dir, as left behind by a failed clone.
        let cache = library.path().join("repos").join("aabbcc");
        std::fs::create_dir_all(&cache).unwrap();

        let err = update_repo(&cache, None, None, None, &NullSink, &CancelToken::new())
            .expect_err("must refuse to update a directory that is not a repository");
        let msg = format!("{err:#}");
        assert!(
            msg.contains("not a git repository"),
            "unexpected error: {msg}"
        );

        // The enclosing repository must be untouched.
        assert_eq!(
            std::fs::read_to_string(library.path().join("tome.toml")).unwrap(),
            "locally = edited\n",
            "enclosing repo working tree was modified"
        );
    }

    #[test]
    fn git_command_cannot_reach_an_enclosing_repository() {
        // Second line of defence, independent of the `is_git_repo` guard: even if
        // some future caller reaches `git_command` with a non-repo directory, the
        // ceiling must stop discovery from escaping upward.
        let library = TempDir::new().unwrap();
        run_git(library.path(), &["init", "--initial-branch", "main"]);
        configure_test_identity(library.path());
        std::fs::write(library.path().join("file.txt"), "committed\n").unwrap();
        run_git(library.path(), &["add", "."]);
        run_git(library.path(), &["commit", "-m", "initial"]);
        std::fs::write(library.path().join("file.txt"), "edited\n").unwrap();

        let cache = library.path().join("repos").join("aabbcc");
        std::fs::create_dir_all(&cache).unwrap();

        // Bypasses update_repo entirely — calls the command layer directly.
        let out = git_command(&cache, &["reset", "--hard", "HEAD"]).unwrap();
        assert!(
            !out.status.success(),
            "reset should fail: no repository is reachable from the cache dir"
        );
        assert_eq!(
            std::fs::read_to_string(library.path().join("file.txt")).unwrap(),
            "edited\n",
            "enclosing repo must be untouched"
        );
    }

    fn run_git(dir: &Path, args: &[&str]) {
        let out = std::process::Command::new("git")
            .args(args)
            .current_dir(dir)
            .env_remove("GIT_DIR")
            .env_remove("GIT_WORK_TREE")
            .env_remove("GIT_INDEX_FILE")
            .output()
            .unwrap();
        assert!(
            out.status.success(),
            "git {:?} failed: {}",
            args,
            String::from_utf8_lossy(&out.stderr)
        );
    }

    fn configure_test_identity(dir: &Path) {
        for args in [
            ["config", "--local", "user.email", "test@test.com"].as_slice(),
            ["config", "--local", "user.name", "Test"].as_slice(),
            ["config", "--local", "commit.gpgsign", "false"].as_slice(),
        ] {
            run_git(dir, args);
        }
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
        // Init a repo with a commit. HARD-14: disable signing per-repo so
        // the test does not flake on developer machines that have global
        // gpg-signing turned on (closes #500).
        std::process::Command::new("git")
            .args(["init"])
            .current_dir(dir)
            .output()
            .unwrap();
        for args in [
            ["config", "--local", "commit.gpgsign", "false"].as_slice(),
            ["config", "--local", "tag.gpgsign", "false"].as_slice(),
            ["config", "--local", "user.email", "test@test.com"].as_slice(),
            ["config", "--local", "user.name", "Test"].as_slice(),
        ] {
            std::process::Command::new("git")
                .args(args)
                .current_dir(dir)
                .output()
                .unwrap();
        }
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
