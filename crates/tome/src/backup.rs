//! Git-backed backup and restore for the tome home directory (`~/.tome/`).
//!
//! The git repo is scoped to `~/.tome/` (not just the library subdirectory),
//! so it tracks skills, `tome.toml`, `tome.lock`, and any future config.
//! All git operations use `std::process::Command`.

use std::path::Path;

use anyhow::{Context, Result};

/// Run a git command in the given directory, returning its raw output.
fn git(repo_dir: &Path, args: &[&str]) -> Result<std::process::Output> {
    let output = std::process::Command::new("git")
        .args(args)
        .current_dir(repo_dir)
        .output()
        .with_context(|| format!("failed to run git {}", args.join(" ")))?;
    Ok(output)
}

/// Run a git command and bail if it fails.
fn git_success(repo_dir: &Path, args: &[&str]) -> Result<()> {
    let output = git(repo_dir, args)?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("git {} failed: {}", args.join(" "), stderr.trim());
    }
    Ok(())
}

/// Run a git command and return its stdout as a trimmed string.
fn git_stdout(repo_dir: &Path, args: &[&str]) -> Result<String> {
    let output = git(repo_dir, args)?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("git {} failed: {}", args.join(" "), stderr.trim());
    }
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

/// Ensure the git repo has a user identity configured (needed for commits).
/// Sets a local fallback if neither local nor global identity exists.
fn ensure_git_identity(repo_dir: &Path) -> Result<()> {
    let has_name = git(repo_dir, &["config", "user.name"])
        .map(|o| o.status.success())
        .unwrap_or(false);
    let has_email = git(repo_dir, &["config", "user.email"])
        .map(|o| o.status.success())
        .unwrap_or(false);
    if !has_name {
        git_success(repo_dir, &["config", "user.name", "tome"])?;
    }
    if !has_email {
        git_success(repo_dir, &["config", "user.email", "tome@localhost"])?;
    }
    Ok(())
}

/// Check whether the tome home directory contains a git repository.
pub(crate) fn has_repo(repo_dir: &Path) -> bool {
    repo_dir.join(".git").exists()
}

/// Initialize a git repository in the tome home directory.
pub(crate) fn init(repo_dir: &Path, dry_run: bool) -> Result<()> {
    if has_repo(repo_dir) {
        println!("Git repo already exists in {}", repo_dir.display());
        return Ok(());
    }
    if dry_run {
        println!("Would initialize git repo in {}", repo_dir.display());
        return Ok(());
    }
    std::fs::create_dir_all(repo_dir)?;
    git_success(repo_dir, &["init"])?;
    // Set fallback git identity if none configured (CI, fresh machines)
    ensure_git_identity(repo_dir)?;
    // Initial commit
    git_success(repo_dir, &["add", "-A"])?;
    let output = git(repo_dir, &["status", "--porcelain"])?;
    let status = String::from_utf8_lossy(&output.stdout);
    if !status.trim().is_empty() {
        git_success(repo_dir, &["commit", "-m", "Initial tome backup"])?;
    }
    println!("{} Initialized backup repo", console::style("✓").green());
    Ok(())
}

/// Create a snapshot (git commit) of the current tome home state.
///
/// Returns `true` if a commit was created, `false` if there was nothing to commit.
pub(crate) fn snapshot(repo_dir: &Path, message: Option<&str>, dry_run: bool) -> Result<bool> {
    if !has_repo(repo_dir) {
        anyhow::bail!("no backup repo found — run `tome backup init` first");
    }
    // Stage all changes (gitignore handles managed skill exclusion)
    git_success(repo_dir, &["add", "-A"])?;
    // Check if there's anything to commit
    let output = git(repo_dir, &["status", "--porcelain"])?;
    let status = String::from_utf8_lossy(&output.stdout);
    if status.trim().is_empty() {
        if !dry_run {
            println!("Nothing to snapshot — library is clean");
        }
        return Ok(false);
    }
    if dry_run {
        println!("Would snapshot {} changed file(s)", status.lines().count());
        return Ok(true);
    }
    let msg = message.unwrap_or("tome backup snapshot");
    git_success(repo_dir, &["commit", "-m", msg])?;
    println!("{} Snapshot created: {}", console::style("✓").green(), msg);
    Ok(true)
}

/// A single entry in the backup history.
pub(crate) struct BackupEntry {
    pub hash: String,
    pub date: String,
    pub message: String,
}

/// List the most recent backup entries.
pub(crate) fn list(repo_dir: &Path, count: usize) -> Result<Vec<BackupEntry>> {
    if !has_repo(repo_dir) {
        anyhow::bail!("no backup repo found — run `tome backup init` first");
    }
    let format = "--format=%h\t%ci\t%s";
    let count_arg = format!("-{}", count);
    let stdout = git_stdout(repo_dir, &["log", &count_arg, format])?;
    let entries = stdout
        .lines()
        .filter(|l| !l.is_empty())
        .map(|line| {
            let parts: Vec<&str> = line.splitn(3, '\t').collect();
            BackupEntry {
                hash: parts.first().unwrap_or(&"").to_string(),
                date: parts.get(1).unwrap_or(&"").to_string(),
                message: parts.get(2).unwrap_or(&"").to_string(),
            }
        })
        .collect();
    Ok(entries)
}

/// Restore the tome home to a previous snapshot.
///
/// Automatically creates a pre-restore snapshot of the current state before
/// checking out files from the target ref.
pub(crate) fn restore(repo_dir: &Path, target: &str, dry_run: bool) -> Result<()> {
    if !has_repo(repo_dir) {
        anyhow::bail!("no backup repo found — run `tome backup init` first");
    }
    if dry_run {
        println!("Would restore library to {}", target);
        return Ok(());
    }
    // Auto-snapshot current state before restoring
    let _ = snapshot(repo_dir, Some("pre-restore auto-snapshot"), false);
    // Restore files from target ref
    git_success(repo_dir, &["checkout", target, "--", "."])?;
    println!(
        "{} Restored to {}. Run {} to re-distribute.",
        console::style("✓").green(),
        target,
        console::style("tome sync").cyan(),
    );
    Ok(())
}

/// Show a diff stat of the working tree against a target ref.
pub(crate) fn diff(repo_dir: &Path, target: &str) -> Result<String> {
    if !has_repo(repo_dir) {
        anyhow::bail!("no backup repo found — run `tome backup init` first");
    }
    // Show diff of working tree against target
    let stdout = git_stdout(repo_dir, &["diff", target, "--stat"])?;
    Ok(stdout)
}

/// Check whether the repo has a remote named "origin" configured.
pub(crate) fn has_remote(repo_dir: &Path) -> bool {
    git(repo_dir, &["remote", "get-url", "origin"])
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// Pull from remote (fast-forward only).
///
/// Returns `Ok(true)` if changes were pulled, `Ok(false)` if already up-to-date.
/// Bails on diverged histories — the user must resolve manually.
pub(crate) fn pull(repo_dir: &Path) -> Result<bool> {
    git_success(repo_dir, &["fetch", "origin"])?;

    // Determine the remote branch: use whatever origin/HEAD points to,
    // falling back to origin/main then origin/master.
    let remote_ref = detect_remote_branch(repo_dir)?;

    let local = git_stdout(repo_dir, &["rev-parse", "HEAD"])?;
    let remote = git_stdout(repo_dir, &["rev-parse", &remote_ref])?;

    if local == remote {
        return Ok(false);
    }

    let output = git(repo_dir, &["merge", "--ff-only", &remote_ref])?;
    if !output.status.success() {
        anyhow::bail!(
            "remote has diverged — resolve manually with `cd {} && git pull`",
            repo_dir.display()
        );
    }
    Ok(true)
}

/// Push the current branch to origin.
pub(crate) fn push(repo_dir: &Path) -> Result<()> {
    let branch = git_stdout(repo_dir, &["rev-parse", "--abbrev-ref", "HEAD"])?;
    let output = git(repo_dir, &["push", "origin", &branch])?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("git push failed: {}", stderr.trim());
    }
    Ok(())
}

/// Add a remote named "origin" to the repo.
pub(crate) fn add_remote(repo_dir: &Path, url: &str) -> Result<()> {
    git_success(repo_dir, &["remote", "add", "origin", url])
}

/// Verify the remote is reachable.
///
/// Accepts exit code 0 (refs found) and 2 (connected but empty repo).
pub(crate) fn verify_remote(repo_dir: &Path) -> Result<()> {
    let output = git(repo_dir, &["ls-remote", "--exit-code", "origin"])?;
    let code = output.status.code().unwrap_or(-1);
    // 0 = success, 2 = connected but no matching refs (empty repo)
    if code != 0 && code != 2 {
        anyhow::bail!("could not connect to remote — check the URL and your credentials");
    }
    Ok(())
}

/// Push the current branch to origin for the first time, setting up tracking.
pub(crate) fn push_initial(repo_dir: &Path) -> Result<()> {
    let branch = git_stdout(repo_dir, &["rev-parse", "--abbrev-ref", "HEAD"])?;
    git_success(repo_dir, &["push", "-u", "origin", &branch])
}

/// Detect the remote branch to merge from.
///
/// Tries `origin/main`, then `origin/master`, then `origin/<current-branch>`.
/// Bails if none of the candidates exist on the remote.
fn detect_remote_branch(repo_dir: &Path) -> Result<String> {
    let branch = git_stdout(repo_dir, &["rev-parse", "--abbrev-ref", "HEAD"])?;
    let candidates = [
        "origin/main".to_string(),
        "origin/master".to_string(),
        format!("origin/{branch}"),
    ];
    for candidate in &candidates {
        let output = git(repo_dir, &["rev-parse", "--verify", candidate])?;
        if output.status.success() {
            return Ok(candidate.clone());
        }
    }
    anyhow::bail!("no remote branch found — tried {}", candidates.join(", "));
}

/// Render backup entries to stdout.
pub(crate) fn render_list(entries: &[BackupEntry]) {
    if entries.is_empty() {
        println!("No backups found");
        return;
    }
    for entry in entries {
        println!(
            "{} {} {}",
            console::style(&entry.hash).yellow(),
            console::style(&entry.date).dim(),
            entry.message,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn setup_git_config(dir: &Path) {
        let _ = std::process::Command::new("git")
            .args(["config", "user.email", "test@test.com"])
            .current_dir(dir)
            .output();
        let _ = std::process::Command::new("git")
            .args(["config", "user.name", "Test"])
            .current_dir(dir)
            .output();
    }

    fn init_test_repo(dir: &Path) {
        git_success(dir, &["init"]).unwrap();
        setup_git_config(dir);
        // Create an initial file so we can make the initial commit
        std::fs::write(dir.join(".gitkeep"), "").unwrap();
        git_success(dir, &["add", "-A"]).unwrap();
        git_success(dir, &["commit", "-m", "initial"]).unwrap();
    }

    #[test]
    fn init_creates_git_repo() {
        let tmp = TempDir::new().unwrap();
        let lib_dir = tmp.path().join("library");
        std::fs::create_dir_all(&lib_dir).unwrap();
        // Set git config globally for the test process scope via env
        init(&lib_dir, false).unwrap();
        setup_git_config(&lib_dir);
        assert!(lib_dir.join(".git").exists());
    }

    #[test]
    fn init_idempotent() {
        let tmp = TempDir::new().unwrap();
        let lib_dir = tmp.path().join("library");
        std::fs::create_dir_all(&lib_dir).unwrap();
        init(&lib_dir, false).unwrap();
        setup_git_config(&lib_dir);
        // Second call should not error
        init(&lib_dir, false).unwrap();
    }

    #[test]
    fn snapshot_creates_commit() {
        let tmp = TempDir::new().unwrap();
        let lib_dir = tmp.path().join("library");
        std::fs::create_dir_all(&lib_dir).unwrap();
        init_test_repo(&lib_dir);

        // Add a file and snapshot
        std::fs::write(lib_dir.join("test-skill.md"), "# Test").unwrap();
        let created = snapshot(&lib_dir, Some("added test skill"), false).unwrap();
        assert!(created);

        // Verify git log has the entry
        let stdout = git_stdout(&lib_dir, &["log", "--oneline"]).unwrap();
        assert!(stdout.contains("added test skill"));
    }

    #[test]
    fn snapshot_nothing_to_commit() {
        let tmp = TempDir::new().unwrap();
        let lib_dir = tmp.path().join("library");
        std::fs::create_dir_all(&lib_dir).unwrap();
        init_test_repo(&lib_dir);

        let created = snapshot(&lib_dir, None, false).unwrap();
        assert!(!created);
    }

    #[test]
    fn list_returns_entries() {
        let tmp = TempDir::new().unwrap();
        let lib_dir = tmp.path().join("library");
        std::fs::create_dir_all(&lib_dir).unwrap();
        init_test_repo(&lib_dir);

        // Create 3 snapshots
        for i in 1..=3 {
            std::fs::write(lib_dir.join(format!("file{i}.txt")), format!("content {i}")).unwrap();
            snapshot(&lib_dir, Some(&format!("snapshot {i}")), false).unwrap();
        }

        let entries = list(&lib_dir, 10).unwrap();
        // initial commit + 3 snapshots = 4
        assert_eq!(entries.len(), 4);
        // Most recent first
        assert_eq!(entries[0].message, "snapshot 3");
        assert_eq!(entries[1].message, "snapshot 2");
        assert_eq!(entries[2].message, "snapshot 1");
        // Check that hash and date are populated
        assert!(!entries[0].hash.is_empty());
        assert!(!entries[0].date.is_empty());
    }

    #[test]
    fn restore_reverts_changes() {
        let tmp = TempDir::new().unwrap();
        let lib_dir = tmp.path().join("library");
        std::fs::create_dir_all(&lib_dir).unwrap();
        init_test_repo(&lib_dir);

        // Create a file and snapshot
        std::fs::write(lib_dir.join("skill.md"), "original").unwrap();
        snapshot(&lib_dir, Some("original state"), false).unwrap();

        // Modify the file and snapshot again
        std::fs::write(lib_dir.join("skill.md"), "modified").unwrap();
        snapshot(&lib_dir, Some("modified state"), false).unwrap();

        // Restore to HEAD~1 (the "original state" commit)
        restore(&lib_dir, "HEAD~1", false).unwrap();

        // File should be back to original content
        let content = std::fs::read_to_string(lib_dir.join("skill.md")).unwrap();
        assert_eq!(content, "original");
    }

    #[test]
    fn diff_shows_changes() {
        let tmp = TempDir::new().unwrap();
        let lib_dir = tmp.path().join("library");
        std::fs::create_dir_all(&lib_dir).unwrap();
        init_test_repo(&lib_dir);

        // Create a file and snapshot
        std::fs::write(lib_dir.join("skill.md"), "original").unwrap();
        snapshot(&lib_dir, Some("baseline"), false).unwrap();

        // Modify the file (unstaged)
        std::fs::write(lib_dir.join("skill.md"), "changed content here").unwrap();

        let output = diff(&lib_dir, "HEAD").unwrap();
        assert!(output.contains("skill.md"), "diff output: {output}");
    }

    #[test]
    fn dry_run_snapshot_no_commit() {
        let tmp = TempDir::new().unwrap();
        let lib_dir = tmp.path().join("library");
        std::fs::create_dir_all(&lib_dir).unwrap();
        init_test_repo(&lib_dir);

        // Add a file
        std::fs::write(lib_dir.join("new-file.md"), "content").unwrap();

        // Dry run should say it would snapshot but not actually commit
        let result = snapshot(&lib_dir, Some("dry run test"), true).unwrap();
        assert!(result); // There are changes to snapshot

        // Count commits — should still be just the initial one
        let log = git_stdout(&lib_dir, &["log", "--oneline"]).unwrap();
        let commit_count = log.lines().count();
        assert_eq!(commit_count, 1, "dry run should not create a commit");
    }

    #[test]
    fn has_remote_false_without_remote() {
        let tmp = TempDir::new().unwrap();
        let dir = tmp.path().join("repo");
        std::fs::create_dir_all(&dir).unwrap();
        init_test_repo(&dir);
        assert!(!has_remote(&dir));
    }

    #[test]
    fn has_remote_true_with_remote() {
        let tmp = TempDir::new().unwrap();
        let dir = tmp.path().join("repo");
        std::fs::create_dir_all(&dir).unwrap();
        init_test_repo(&dir);

        // Create a bare remote from existing repo
        let remote_dir = tmp.path().join("remote.git");
        git_success(
            &dir,
            &["clone", "--bare", ".", remote_dir.to_str().unwrap()],
        )
        .unwrap();

        add_remote(&dir, remote_dir.to_str().unwrap()).unwrap();
        assert!(has_remote(&dir));
    }

    #[test]
    fn push_and_pull_roundtrip() {
        let tmp = TempDir::new().unwrap();

        // Create repo A first (so we have commits to push)
        let repo_a = tmp.path().join("repo_a");
        std::fs::create_dir_all(&repo_a).unwrap();
        init_test_repo(&repo_a);

        // Create a bare remote from repo A
        let remote_dir = tmp.path().join("remote.git");
        git_success(
            &repo_a,
            &["clone", "--bare", ".", remote_dir.to_str().unwrap()],
        )
        .unwrap();
        add_remote(&repo_a, remote_dir.to_str().unwrap()).unwrap();

        // Clone into repo B
        let repo_b = tmp.path().join("repo_b");
        git_success(
            tmp.path(),
            &["clone", remote_dir.to_str().unwrap(), "repo_b"],
        )
        .unwrap();
        setup_git_config(&repo_b);

        // Add a file in repo A and push
        std::fs::write(repo_a.join("new-skill.md"), "# Skill").unwrap();
        snapshot(&repo_a, Some("add skill"), false).unwrap();
        push(&repo_a).unwrap();

        // Pull in repo B — should get the new file
        let pulled = pull(&repo_b).unwrap();
        assert!(pulled, "should have pulled changes");
        assert!(
            repo_b.join("new-skill.md").exists(),
            "pulled file should exist"
        );

        // Pull again — should be up-to-date
        let pulled_again = pull(&repo_b).unwrap();
        assert!(!pulled_again, "should be up-to-date");
    }
}
