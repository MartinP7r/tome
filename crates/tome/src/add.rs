//! Add a git skill repository to the config.
//!
//! `tome add <url>` creates a `[directories.<name>]` entry with `type = "git"` in `tome.toml`.
//! The directory name is extracted from the URL unless overridden with `--name`.

use std::path::{Path, PathBuf};

use anyhow::{Result, bail};
use console::style;

use crate::config::{Config, DirectoryConfig, DirectoryName, DirectoryType};

/// Expand a bare `owner/repo` slug to a full GitHub HTTPS URL.
///
/// Anything that already looks like a URL (contains `://` or starts with
/// `git@`) is returned unchanged. A string matching `<owner>/<repo>` where
/// both segments are non-empty, neither is `.` or `..`, and each contains
/// only `[A-Za-z0-9._-]` is rewritten to `https://github.com/<owner>/<repo>`.
/// Anything else passes through verbatim and is left for `git clone` to
/// reject downstream.
///
/// This is a syntactic shape check, not a GitHub-validity check. Inputs
/// like `-foo/bar` (leading hyphen) or `owner/.git` pass the shape check
/// and will be expanded; the resulting URL fails at clone time, same as
/// any other malformed reference. The narrow heuristic is intentional:
/// false-negatives (URL not expanded, user has to paste the full thing)
/// are easier to recover from than false-positives (path silently
/// rewritten to a wrong clone target).
///
/// **Two-segment relative paths are ambiguous.** A directory like `src/foo`
/// has the same shape as a slug — the helper cannot tell them apart and
/// will expand it. Pass `./src/foo` to disambiguate (the leading `./` is
/// rejected by the `.` segment check).
fn normalize_url(input: &str) -> String {
    if input.contains("://") || input.starts_with("git@") {
        return input.to_string();
    }
    let trimmed = input.trim_end_matches('/');
    let parts: Vec<&str> = trimmed.split('/').collect();
    if parts.len() != 2 {
        return input.to_string();
    }
    let owner = parts[0];
    let repo = parts[1];
    if owner.is_empty() || repo.is_empty() {
        return input.to_string();
    }
    let valid_segment = |s: &str| {
        // Reject `.` and `..` so relative paths (`../foo`, `./foo`) don't
        // sneak through with the right segment count and wrong meaning.
        if s == "." || s == ".." {
            return false;
        }
        s.chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == '.')
    };
    if !valid_segment(owner) || !valid_segment(repo) {
        return input.to_string();
    }
    format!("https://github.com/{owner}/{repo}")
}

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
    // The stored URL must be the expanded form — git clone won't resolve
    // bare slugs on its own.
    let resolved_url = normalize_url(opts.url);

    let dir_name_str = match opts.name {
        Some(n) => n.to_string(),
        None => extract_repo_name(&resolved_url),
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
        path: PathBuf::from(&resolved_url),
        directory_type: DirectoryType::Git,
        role: None,
        branch: opts.branch.map(String::from),
        tag: opts.tag.map(String::from),
        rev: opts.rev.map(String::from),
        subdir: None,
        override_applied: false,
    };

    if opts.dry_run {
        println!(
            "{} add directory '{}' (git: {})",
            style("Would").yellow(),
            style(&dir_name_str).cyan(),
            resolved_url,
        );
    } else {
        config.directories.insert(dir_name, dir_config);
        config.save(opts.config_path)?;
        println!(
            "{} directory '{}' (git: {})",
            style("Added").green(),
            style(&dir_name_str).cyan(),
            resolved_url,
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
        assert_eq!(extract_repo_name("https://github.com/user/repo"), "repo");
    }

    #[test]
    fn test_extract_repo_name_trailing_slash() {
        assert_eq!(extract_repo_name("https://github.com/user/repo/"), "repo");
    }

    #[test]
    fn test_extract_repo_name_ssh() {
        assert_eq!(extract_repo_name("git@github.com:user/repo.git"), "repo");
    }

    #[test]
    fn test_extract_repo_name_ssh_no_git() {
        assert_eq!(extract_repo_name("git@github.com:user/repo"), "repo");
    }

    #[test]
    fn normalize_url_expands_bare_slug_to_github_https() {
        assert_eq!(
            normalize_url("planetscale/database-skills"),
            "https://github.com/planetscale/database-skills"
        );
    }

    #[test]
    fn normalize_url_expands_slug_with_underscores_dots_hyphens() {
        assert_eq!(
            normalize_url("MartinP7r/some.repo_name-v2"),
            "https://github.com/MartinP7r/some.repo_name-v2"
        );
    }

    #[test]
    fn normalize_url_strips_trailing_slash_on_slug() {
        assert_eq!(
            normalize_url("planetscale/database-skills/"),
            "https://github.com/planetscale/database-skills"
        );
    }

    #[test]
    fn normalize_url_leaves_https_url_unchanged() {
        let url = "https://github.com/planetscale/database-skills";
        assert_eq!(normalize_url(url), url);
    }

    #[test]
    fn normalize_url_leaves_https_url_with_dotgit_unchanged() {
        let url = "https://github.com/planetscale/database-skills.git";
        assert_eq!(normalize_url(url), url);
    }

    #[test]
    fn normalize_url_leaves_ssh_url_unchanged() {
        let url = "git@github.com:planetscale/database-skills.git";
        assert_eq!(normalize_url(url), url);
    }

    #[test]
    fn normalize_url_leaves_three_segment_path_unchanged() {
        // Don't try to resolve `github.com/owner/repo` — it has 3 segments;
        // user must paste a full URL with scheme. Keeping the heuristic
        // narrow avoids accidentally rewriting unrelated relative paths.
        let input = "github.com/planetscale/database-skills";
        assert_eq!(normalize_url(input), input);
    }

    #[test]
    fn normalize_url_leaves_single_segment_unchanged() {
        assert_eq!(normalize_url("solo-segment"), "solo-segment");
    }

    #[test]
    fn normalize_url_leaves_empty_owner_unchanged() {
        assert_eq!(normalize_url("/repo"), "/repo");
    }

    #[test]
    fn normalize_url_leaves_empty_repo_unchanged() {
        assert_eq!(normalize_url("owner/"), "owner/");
    }

    #[test]
    fn normalize_url_leaves_segment_with_space_unchanged() {
        // Spaces aren't valid in GitHub slugs — refuse to expand to avoid
        // turning a typo into a confidently wrong clone target.
        let input = "owner/repo with space";
        assert_eq!(normalize_url(input), input);
    }

    #[test]
    fn normalize_url_leaves_three_segment_relative_path_unchanged() {
        // 3 segments → length check rejects it. The dangerous case (where
        // the `.`/`..` sentinel actually does the work) is below.
        let input = "../some/path";
        assert_eq!(normalize_url(input), input);
    }

    #[test]
    fn normalize_url_leaves_two_segment_relative_path_unchanged() {
        // `../foo` matches the 2-segment shape and `..` would otherwise
        // pass the char-class check (dot is allowed). The `.`/`..`
        // sentinel rejection is what actually saves us — without it this
        // would expand to `https://github.com/../foo` and clone-fail
        // confusingly.
        let input = "../foo";
        assert_eq!(normalize_url(input), input);
        let input = "./foo";
        assert_eq!(normalize_url(input), input);
    }

    #[test]
    fn normalize_url_leaves_absolute_path_unchanged() {
        let input = "/abs/path";
        assert_eq!(normalize_url(input), input);
    }
}
