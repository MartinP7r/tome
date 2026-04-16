//! Add a git skill repository to the config.
//!
//! `tome add <url>` creates a `[directories.<name>]` entry with `type = "git"` in `tome.toml`.
//! The directory name is extracted from the URL unless overridden with `--name`.

use std::path::{Path, PathBuf};

use anyhow::{Result, bail};
use console::style;

use crate::config::{Config, DirectoryConfig, DirectoryName, DirectoryType};

/// Extract a repository name from a git URL.
///
/// Handles HTTPS and SSH URLs, strips trailing `/` and `.git` suffix.
///
/// # Examples
///
/// ```text
/// https://github.com/user/repo.git  -> repo
/// git@github.com:user/repo.git      -> repo
/// https://github.com/user/repo/     -> repo
/// ```
fn extract_repo_name(url: &str) -> String {
    let url = url.trim_end_matches('/');

    // SSH URLs: git@host:user/repo.git — no `/` in the prefix before `:`
    let segment = if let Some((prefix, path)) = url.rsplit_once(':') {
        if !prefix.contains('/') {
            // SSH-style URL — take the last segment of the path after `:`
            path.rsplit_once('/').map_or(path, |(_, last)| last)
        } else {
            // Regular URL with port or protocol — take last path segment
            url.rsplit_once('/').map_or(url, |(_, last)| last)
        }
    } else {
        url.rsplit_once('/').map_or(url, |(_, last)| last)
    };

    segment.strip_suffix(".git").unwrap_or(segment).to_string()
}

/// Options for the `tome add` command.
pub(crate) struct AddOptions<'a> {
    pub url: &'a str,
    pub name: Option<&'a str>,
    pub branch: Option<&'a str>,
    pub tag: Option<&'a str>,
    pub rev: Option<&'a str>,
    pub dry_run: bool,
    pub config_path: &'a Path,
}

/// Add a git directory entry to the config.
///
/// This is config-only — no sync is triggered. The user should run `tome sync`
/// afterwards to clone the repo and discover skills.
pub(crate) fn add(config: &mut Config, opts: AddOptions<'_>) -> Result<()> {
    let dir_name_str = match opts.name {
        Some(n) => n.to_string(),
        None => extract_repo_name(opts.url),
    };

    if dir_name_str.is_empty() {
        bail!(
            "could not extract repository name from '{}'. Use --name to specify manually.",
            opts.url
        );
    }

    let dir_name = DirectoryName::new(&dir_name_str)?;

    if config.directories.contains_key(&dir_name) {
        bail!("directory '{}' already exists in config", dir_name_str);
    }

    let dir_config = DirectoryConfig {
        path: PathBuf::from(opts.url),
        directory_type: DirectoryType::Git,
        role: None,
        branch: opts.branch.map(String::from),
        tag: opts.tag.map(String::from),
        rev: opts.rev.map(String::from),
        subdir: None,
    };

    if opts.dry_run {
        println!(
            "{} add directory '{}' (git: {})",
            style("Would").yellow(),
            style(&dir_name_str).cyan(),
            opts.url,
        );
    } else {
        config.directories.insert(dir_name, dir_config);
        config.save(opts.config_path)?;
        println!(
            "{} directory '{}' (git: {})",
            style("Added").green(),
            style(&dir_name_str).cyan(),
            opts.url,
        );
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_repo_name_https() {
        assert_eq!(
            extract_repo_name("https://github.com/user/repo.git"),
            "repo"
        );
    }

    #[test]
    fn test_extract_repo_name_https_no_git() {
        assert_eq!(
            extract_repo_name("https://github.com/user/repo"),
            "repo"
        );
    }

    #[test]
    fn test_extract_repo_name_trailing_slash() {
        assert_eq!(
            extract_repo_name("https://github.com/user/repo/"),
            "repo"
        );
    }

    #[test]
    fn test_extract_repo_name_ssh() {
        assert_eq!(
            extract_repo_name("git@github.com:user/repo.git"),
            "repo"
        );
    }

    #[test]
    fn test_extract_repo_name_ssh_no_git() {
        assert_eq!(
            extract_repo_name("git@github.com:user/repo"),
            "repo"
        );
    }
}
