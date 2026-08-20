//! Safe Git coordination for the shared pool repository.
//!
//! This module intentionally owns a narrow Git surface: health preflight,
//! fast-forward pulls, the per-config-directory sync lock, and (later in this
//! phase) Tome-owned publication. It never repairs the caller's index.

use std::fs::OpenOptions;
use std::io::IsTerminal;
use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{Context, Result};

use crate::profiles::GitSyncPolicy;

/// Keeps a create-new config-directory lock alive for one sync invocation.
pub(crate) struct RepoSync {
    lock_path: PathBuf,
    repo_root: Option<PathBuf>,
}

impl Drop for RepoSync {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.lock_path);
    }
}

impl RepoSync {
    /// Acquire the sync lock and perform any consented pre-reconciliation pull.
    pub(crate) fn begin(
        config_dir: &Path,
        policy: GitSyncPolicy,
        no_input: bool,
        dry_run: bool,
    ) -> Result<Self> {
        let lock_path = config_dir.join(".tome-sync.lock");
        let mut lock = OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(&lock_path)
            .with_context(|| {
                format!(
                    "another tome sync may be running: lock exists at {} (stale locks are never removed automatically)",
                    lock_path.display()
                )
            })?;
        use std::io::Write;
        writeln!(
            lock,
            "pid={} started_at={:?}",
            std::process::id(),
            std::time::SystemTime::now()
        )
        .with_context(|| format!("failed to write sync lock {}", lock_path.display()))?;

        let repo_root = repository_root(config_dir)?;
        let session = Self {
            lock_path,
            repo_root,
        };
        session.pull_if_consented(policy, no_input, dry_run)?;
        Ok(session)
    }

    pub(crate) fn repo_root(&self) -> Option<&Path> {
        self.repo_root.as_deref()
    }

    fn pull_if_consented(
        &self,
        policy: GitSyncPolicy,
        no_input: bool,
        dry_run: bool,
    ) -> Result<()> {
        if dry_run || matches!(policy, GitSyncPolicy::Never) {
            return Ok(());
        }
        let Some(repo_root) = self.repo_root() else {
            eprintln!(
                "warning: shared-pool Git coordination is unavailable outside a Git worktree; continuing local-only"
            );
            return Ok(());
        };
        let status = status_snapshot(repo_root)?;
        if let Some(reason) = status.unsafe_reason() {
            eprintln!("warning: managed Git pull is unavailable: {reason}; continuing local-only");
            return Ok(());
        }
        let upstream = match git_stdout(
            repo_root,
            &[
                "rev-parse",
                "--abbrev-ref",
                "--symbolic-full-name",
                "@{upstream}",
            ],
        ) {
            Ok(upstream) => upstream,
            Err(_) => {
                eprintln!(
                    "warning: shared-pool Git coordination needs an upstream branch; continuing local-only"
                );
                return Ok(());
            }
        };

        let should_pull = match policy {
            GitSyncPolicy::Always => true,
            GitSyncPolicy::Ask if no_input || !std::io::stdin().is_terminal() => {
                eprintln!(
                    "warning: git_sync = \"ask\" is noninteractive; continuing local-only (use --git-sync always to opt in)"
                );
                false
            }
            GitSyncPolicy::Ask => {
                let summary = git_stdout(repo_root, &["fetch", "--dry-run", "--verbose"])
                    .unwrap_or_else(|_| format!("updates from {upstream}"));
                dialoguer::Confirm::new()
                    .with_prompt(format!(
                        "Pull the shared repository fast-forward only? This may update unrelated repository files. Remote summary: {}",
                        summary.lines().next().unwrap_or(&upstream)
                    ))
                    .default(false)
                    .interact()?
            }
            GitSyncPolicy::Never => false,
        };
        if !should_pull {
            return Ok(());
        }

        git_success(repo_root, &["fetch", "--prune"])?;
        let local = git_stdout(repo_root, &["rev-parse", "HEAD"])?;
        let remote = git_stdout(repo_root, &["rev-parse", &upstream])?;
        if local != remote {
            git_success(repo_root, &["merge", "--ff-only", &upstream])?;
            println!(
                "  {} Pulled changes from remote",
                console::style("↓").cyan()
            );
        }
        Ok(())
    }
}

#[derive(Debug, Default)]
struct StatusSnapshot {
    staged: bool,
    unmerged: bool,
    ahead: usize,
    behind: usize,
    operation_in_progress: bool,
}

impl StatusSnapshot {
    fn unsafe_reason(&self) -> Option<&'static str> {
        if self.unmerged {
            Some("the repository has unresolved conflicts")
        } else if self.staged {
            Some("the repository has pre-existing staged changes")
        } else if self.operation_in_progress {
            Some("a merge or rebase is in progress")
        } else if self.ahead > 0 && self.behind > 0 {
            Some("the branch has diverged from its upstream")
        } else if self.behind > 0 {
            Some("the branch is behind its upstream")
        } else {
            None
        }
    }
}

fn repository_root(config_dir: &Path) -> Result<Option<PathBuf>> {
    let output = git(config_dir, &["rev-parse", "--show-toplevel"])?;
    if !output.status.success() {
        return Ok(None);
    }
    Ok(Some(PathBuf::from(
        String::from_utf8_lossy(&output.stdout).trim(),
    )))
}

fn status_snapshot(repo_root: &Path) -> Result<StatusSnapshot> {
    let output = git(
        repo_root,
        &[
            "--no-optional-locks",
            "status",
            "--porcelain=v2",
            "--branch",
            "--show-stash",
        ],
    )?;
    if !output.status.success() {
        anyhow::bail!(
            "could not inspect Git repository health: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        );
    }
    let mut snapshot = StatusSnapshot::default();
    for line in String::from_utf8_lossy(&output.stdout).lines() {
        if let Some(ab) = line.strip_prefix("# branch.ab +") {
            let mut fields = ab.split(" -");
            snapshot.ahead = fields
                .next()
                .and_then(|value| value.parse().ok())
                .unwrap_or_default();
            snapshot.behind = fields
                .next()
                .and_then(|value| value.parse().ok())
                .unwrap_or_default();
        } else if line.starts_with("u ") {
            snapshot.unmerged = true;
        } else if (line.starts_with("1 ") || line.starts_with("2 "))
            && line
                .split_whitespace()
                .nth(1)
                .is_some_and(|xy| xy.as_bytes().first() != Some(&b'.'))
        {
            snapshot.staged = true;
        }
    }
    let git_dir = git_stdout(repo_root, &["rev-parse", "--git-dir"])?;
    let git_dir = repo_root.join(git_dir);
    snapshot.operation_in_progress = ["MERGE_HEAD", "rebase-merge", "rebase-apply"]
        .iter()
        .any(|name| git_dir.join(name).exists());
    Ok(snapshot)
}

fn git(repo_dir: &Path, args: &[&str]) -> Result<std::process::Output> {
    Command::new("git")
        .args(args)
        .current_dir(repo_dir)
        .env_remove("GIT_DIR")
        .env_remove("GIT_WORK_TREE")
        .env_remove("GIT_INDEX_FILE")
        .output()
        .with_context(|| format!("failed to run git {}", args.join(" ")))
}

fn git_stdout(repo_dir: &Path, args: &[&str]) -> Result<String> {
    let output = git(repo_dir, args)?;
    if !output.status.success() {
        anyhow::bail!(
            "git {} failed: {}",
            args.join(" "),
            String::from_utf8_lossy(&output.stderr).trim()
        );
    }
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

fn git_success(repo_dir: &Path, args: &[&str]) -> Result<()> {
    let output = git(repo_dir, args)?;
    if !output.status.success() {
        anyhow::bail!(
            "git {} failed: {}",
            args.join(" "),
            String::from_utf8_lossy(&output.stderr).trim()
        );
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lock_is_create_new_and_released_on_drop() {
        let temp = tempfile::TempDir::new().unwrap();
        let first = RepoSync::begin(temp.path(), GitSyncPolicy::Never, true, false).unwrap();
        assert!(RepoSync::begin(temp.path(), GitSyncPolicy::Never, true, false).is_err());
        drop(first);
        assert!(RepoSync::begin(temp.path(), GitSyncPolicy::Never, true, false).is_ok());
    }

    #[test]
    fn unsafe_status_reasons_prioritize_index_and_history_protection() {
        assert_eq!(
            StatusSnapshot {
                staged: true,
                ..Default::default()
            }
            .unsafe_reason(),
            Some("the repository has pre-existing staged changes")
        );
        assert_eq!(
            StatusSnapshot {
                behind: 1,
                ..Default::default()
            }
            .unsafe_reason(),
            Some("the branch is behind its upstream")
        );
        assert_eq!(
            StatusSnapshot {
                ahead: 1,
                behind: 1,
                ..Default::default()
            }
            .unsafe_reason(),
            Some("the branch has diverged from its upstream")
        );
    }
}
