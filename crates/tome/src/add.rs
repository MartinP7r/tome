//! Add a git skill repository to the config.
//!
//! `tome add <url>` creates a `[directories.<name>]` entry with `type = "git"` in `tome.toml`.
//! The directory name is extracted from the URL unless overridden with `--name`.
//!
//! ## URL forms
//!
//! - Full HTTPS URL: `https://github.com/owner/repo`
//! - SSH URL: `git@github.com:owner/repo.git`
//! - Bare GitHub slug: `owner/repo` (expanded to HTTPS by `normalize_url`)
//! - **GitHub `/tree/<ref>/<subdir>` suffix** (v0.13+): `owner/repo/tree/main/skills`
//!   or `https://github.com/owner/repo/tree/main/skills`. The suffix is
//!   stripped and the extracted `<ref>` becomes the default branch + the
//!   extracted `<subdir>` becomes the directory's discovery subdir.
//!   Mimics how skill repos are typically linked in their README. Explicit
//!   `--branch`/`--tag`/`--subdir` flags override the URL-embedded values
//!   (with a warning surfacing the conflict).

use std::path::{Path, PathBuf};

use anyhow::{Result, bail};
use console::style;
use tracing::warn;

use crate::config::{Config, DirectoryConfig, DirectoryName, DirectoryRole, DirectoryType, GitRef};

/// Result of parsing a GitHub `/tree/<ref>/<subdir>` suffix off the input URL.
///
/// Vercel-style URL targeting: a user can paste a GitHub tree URL
/// (`github.com/owner/repo/tree/<branch>/<path>`) — the same URL format the
/// browser uses when navigating into a subdir — and `tome add` strips the
/// suffix, using `<branch>` as the git ref and `<path>` as the discovery
/// subdir.
#[derive(Debug, PartialEq, Eq)]
pub(crate) struct ParsedTreeSuffix {
    /// Input URL or slug with the `/tree/<ref>/<subdir>` suffix removed.
    /// Pass to [`normalize_url`] for the usual slug-expansion path.
    pub base: String,
    /// Branch / ref extracted from the URL. `None` if no `/tree/<ref>/...`
    /// segment was present.
    pub branch: Option<String>,
    /// Subdir extracted from the URL. `None` when the URL ends at
    /// `/tree/<ref>` with no path component, OR when no `/tree/` segment
    /// was present at all.
    pub subdir: Option<String>,
}

/// Extract the `/tree/<ref>/<subdir>` suffix from a GitHub URL or slug.
///
/// Recognized patterns (HTTPS and bare-slug forms both supported):
///
/// - `https://github.com/owner/repo/tree/<ref>/<path...>`
/// - `owner/repo/tree/<ref>/<path...>`
/// - `https://github.com/owner/repo/tree/<ref>` (no subdir; just sets the ref)
/// - `owner/repo/tree/<ref>` (same)
///
/// SSH URLs (`git@github.com:owner/repo.git`) don't support the `/tree/`
/// syntax — they're returned unchanged.
///
/// Inputs without `/tree/` segment are returned with `branch: None,
/// subdir: None`. Multi-segment subdirs (`tree/main/path/to/skills`) are
/// preserved verbatim as a single subdir string.
pub(crate) fn parse_tree_suffix(input: &str) -> ParsedTreeSuffix {
    // SSH form bypasses entirely — git@host:owner/repo has no path
    // segments that could carry the tree syntax.
    if input.starts_with("git@") {
        return ParsedTreeSuffix {
            base: input.to_string(),
            branch: None,
            subdir: None,
        };
    }

    // Strip query/fragment if any (defensive — github.com `/tree/` URLs
    // shouldn't have these, but a copy-paste might).
    let cleaned = input.split(['?', '#']).next().unwrap_or(input);
    let trimmed = cleaned.trim_end_matches('/');

    // Find the FIRST `/tree/` segment. Anything before is the base; what
    // follows is `<ref>/<subdir...>`.
    let Some(idx) = trimmed.find("/tree/") else {
        return ParsedTreeSuffix {
            base: input.to_string(),
            branch: None,
            subdir: None,
        };
    };

    let base = &trimmed[..idx];
    let after_tree = &trimmed[idx + "/tree/".len()..]; // "<ref>" or "<ref>/<subdir...>"

    let (branch, subdir) = match after_tree.split_once('/') {
        Some((b, s)) => (
            (!b.is_empty()).then(|| b.to_string()),
            (!s.is_empty()).then(|| s.to_string()),
        ),
        None => (
            (!after_tree.is_empty()).then(|| after_tree.to_string()),
            None,
        ),
    };

    // Edge case: input was `.../tree/` (empty `<ref>`) — surface as a
    // no-op rather than constructing a malformed config entry.
    if branch.is_none() && subdir.is_none() {
        return ParsedTreeSuffix {
            base: input.to_string(),
            branch: None,
            subdir: None,
        };
    }

    ParsedTreeSuffix {
        base: base.to_string(),
        branch,
        subdir,
    }
}

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
    /// Explicit `--subdir <PATH>` flag. When set, discovery scans
    /// `<clone>/<PATH>/*/SKILL.md` instead of `<clone>/*/SKILL.md`.
    /// Overrides any subdir parsed from a `/tree/<ref>/<subdir>` URL
    /// suffix (with a warning surfacing the conflict).
    pub subdir: Option<&'a str>,
    /// Explicit `--role <ROLE>` flag (Phase 20 / v0.14). When `None`, the
    /// directory's `role` falls back to `DirectoryType::default_role()` —
    /// the same default `Config::load` applies. When `Some`, the choice
    /// is validated against `DirectoryType::valid_roles()` for this
    /// type; an incompatible combination bails with a clear error
    /// naming the valid roles. Setting `role` explicitly lets the user
    /// avoid the `directory → synced` default-write-back trap (the pfw
    /// dogfooding pain that surfaced this phase).
    pub role: Option<DirectoryRole>,
    pub dry_run: bool,
    pub config_path: &'a Path,
}

/// Add a git directory entry to the config.
///
/// This is config-only — no sync is triggered. The user should run `tome sync`
/// afterwards to clone the repo and discover skills.
pub(crate) fn add(config: &mut Config, opts: AddOptions<'_>) -> Result<()> {
    // Strip the GitHub `/tree/<ref>/<subdir>` suffix first; its extracted
    // branch + subdir become defaults that explicit --branch / --subdir
    // flags can override.
    let parsed = parse_tree_suffix(opts.url);

    // The stored URL must be the expanded form — git clone won't resolve
    // bare slugs on its own.
    let resolved_url = normalize_url(&parsed.base);

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

    // CLI clap config enforces mutual exclusion of branch/tag/rev via
    // `conflicts_with_all`, so at most one of these is `Some`. The URL-
    // embedded branch (from `/tree/<ref>/...`) acts as a default that
    // explicit --branch wins over (with a warning); --tag and --rev
    // semantically supersede the URL-embedded ref entirely (different
    // ref kinds), so we warn-and-prefer-the-flag in those cases too.
    let url_branch = parsed.branch.clone();
    let git_ref = match (
        opts.branch.map(String::from),
        opts.tag.map(String::from),
        opts.rev.map(String::from),
        url_branch,
    ) {
        // Explicit --branch always wins; warn if it disagrees with URL.
        (Some(b), None, None, url_b) => {
            if let Some(u) = url_b
                && u != b
            {
                warn!(
                    "URL-embedded branch '/tree/{}/...' overridden by explicit --branch '{}'",
                    u, b
                );
            }
            Some(GitRef::Branch(b))
        }
        // --tag or --rev wins over URL-embedded branch (different ref kinds).
        (None, Some(t), None, url_b) => {
            if let Some(u) = url_b {
                warn!(
                    "URL-embedded branch '/tree/{}/...' overridden by explicit --tag '{}' (different ref kind)",
                    u, t
                );
            }
            Some(GitRef::Tag(t))
        }
        (None, None, Some(r), url_b) => {
            if let Some(u) = url_b {
                warn!(
                    "URL-embedded branch '/tree/{}/...' overridden by explicit --rev '{}' (different ref kind)",
                    u, r
                );
            }
            Some(GitRef::Rev(r))
        }
        // No flags set; fall back to URL-embedded branch if any.
        (None, None, None, Some(u)) => Some(GitRef::Branch(u)),
        (None, None, None, None) => None,
        // Unreachable in practice because clap enforces mutual exclusion;
        // bail with a recoverable error rather than panicking if a future
        // refactor removes that constraint.
        _ => bail!("internal: --branch, --tag, --rev are mutually exclusive"),
    };

    // Explicit --subdir wins over URL-embedded subdir; warn on conflict.
    let final_subdir = match (opts.subdir.map(String::from), parsed.subdir.clone()) {
        (Some(s), Some(u)) if s != u => {
            warn!(
                "URL-embedded subdir '/tree/.../{}' overridden by explicit --subdir '{}'",
                u, s
            );
            Some(s)
        }
        (Some(s), _) => Some(s),
        (None, url_sub) => url_sub,
    };

    // Phase 20 (v0.14): explicit --role flag wins over the type-default.
    // Validate against valid_roles() to fail fast on incompatible combos
    // (e.g. `--role target` for a git type — git is discovery-only).
    if let Some(r) = opts.role {
        let valid = DirectoryType::Git.valid_roles();
        if !valid.contains(&r) {
            let valid_str: Vec<String> = valid.iter().map(|r| r.kebab_case().to_string()).collect();
            bail!(
                "role '{}' is not valid for type 'git' — valid roles: {}",
                r.kebab_case(),
                valid_str.join(", ")
            );
        }
    }
    let dir_config = DirectoryConfig {
        path: PathBuf::from(&resolved_url),
        directory_type: DirectoryType::Git,
        role: opts.role,
        git_ref,
        subdir: final_subdir,
        override_applied: false,
    };

    // Echo the resolved role in the success message (Phase 20). Falls back
    // to the type-default when --role wasn't passed so the user sees the
    // value tome will actually use, not just "None". Closes the gap where
    // a synced-default surprise (writing into the source dir) had no
    // signal at add time.
    let resolved_role = opts
        .role
        .unwrap_or_else(|| DirectoryType::Git.default_role());

    if opts.dry_run {
        println!(
            "{} add directory '{}' (git: {}, role: {})",
            style("Would").yellow(),
            style(&dir_name_str).cyan(),
            resolved_url,
            style(resolved_role.kebab_case()).yellow(),
        );
    } else {
        config.directories.insert(dir_name, dir_config);
        config.save(opts.config_path)?;
        println!(
            "{} directory '{}' (git: {}, role: {})",
            style("Added").green(),
            style(&dir_name_str).cyan(),
            resolved_url,
            style(resolved_role.kebab_case()).cyan(),
        );
        println!(
            "  {}",
            style(format!("→ {}", resolved_role.description())).dim(),
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

    // -- parse_tree_suffix tests (Layer 1: URL-suffix parsing) --

    #[test]
    fn parse_tree_suffix_extracts_branch_and_subdir_from_https_url() {
        let parsed =
            parse_tree_suffix("https://github.com/signerlabs/shipswift-skills/tree/main/skills");
        assert_eq!(
            parsed.base,
            "https://github.com/signerlabs/shipswift-skills"
        );
        assert_eq!(parsed.branch.as_deref(), Some("main"));
        assert_eq!(parsed.subdir.as_deref(), Some("skills"));
    }

    #[test]
    fn parse_tree_suffix_extracts_branch_and_subdir_from_bare_slug() {
        let parsed = parse_tree_suffix("signerlabs/shipswift-skills/tree/main/skills");
        assert_eq!(parsed.base, "signerlabs/shipswift-skills");
        assert_eq!(parsed.branch.as_deref(), Some("main"));
        assert_eq!(parsed.subdir.as_deref(), Some("skills"));
    }

    #[test]
    fn parse_tree_suffix_extracts_branch_only_when_no_subdir() {
        // GitHub URLs that point at a branch root: /tree/<ref> with no
        // further path. The ref becomes the branch; no subdir.
        let parsed = parse_tree_suffix("https://github.com/owner/repo/tree/dev");
        assert_eq!(parsed.base, "https://github.com/owner/repo");
        assert_eq!(parsed.branch.as_deref(), Some("dev"));
        assert_eq!(parsed.subdir, None);
    }

    #[test]
    fn parse_tree_suffix_preserves_multi_segment_subdir() {
        let parsed = parse_tree_suffix("owner/repo/tree/main/path/to/skills");
        assert_eq!(parsed.base, "owner/repo");
        assert_eq!(parsed.branch.as_deref(), Some("main"));
        assert_eq!(parsed.subdir.as_deref(), Some("path/to/skills"));
    }

    #[test]
    fn parse_tree_suffix_strips_trailing_slash() {
        let parsed = parse_tree_suffix("owner/repo/tree/main/skills/");
        assert_eq!(parsed.base, "owner/repo");
        assert_eq!(parsed.branch.as_deref(), Some("main"));
        assert_eq!(parsed.subdir.as_deref(), Some("skills"));
    }

    #[test]
    fn parse_tree_suffix_passes_through_inputs_without_tree_segment() {
        let parsed = parse_tree_suffix("owner/repo");
        assert_eq!(parsed.base, "owner/repo");
        assert_eq!(parsed.branch, None);
        assert_eq!(parsed.subdir, None);

        let parsed = parse_tree_suffix("https://github.com/owner/repo.git");
        assert_eq!(parsed.base, "https://github.com/owner/repo.git");
        assert_eq!(parsed.branch, None);
        assert_eq!(parsed.subdir, None);
    }

    #[test]
    fn parse_tree_suffix_leaves_ssh_url_unchanged() {
        // SSH form doesn't carry tree segments in its URL grammar — the
        // path after `:` isn't navigable via /tree/. Pass through.
        let parsed = parse_tree_suffix("git@github.com:owner/repo.git");
        assert_eq!(parsed.base, "git@github.com:owner/repo.git");
        assert_eq!(parsed.branch, None);
        assert_eq!(parsed.subdir, None);
    }

    #[test]
    fn parse_tree_suffix_strips_query_and_fragment() {
        // Copy-pasting a GitHub URL with `?foo=bar` or `#fragment` should
        // still find the tree segment.
        let parsed =
            parse_tree_suffix("https://github.com/owner/repo/tree/main/skills?ref=foo#readme");
        assert_eq!(parsed.base, "https://github.com/owner/repo");
        assert_eq!(parsed.branch.as_deref(), Some("main"));
        assert_eq!(parsed.subdir.as_deref(), Some("skills"));
    }

    #[test]
    fn parse_tree_suffix_no_op_for_empty_after_tree() {
        // Malformed: `.../tree/` with nothing after — surface as a no-op
        // so the user gets a clear error from clone time rather than a
        // half-constructed config.
        let parsed = parse_tree_suffix("owner/repo/tree/");
        // We treat the whole input as base since parsing yielded nothing.
        assert_eq!(parsed.branch, None);
        assert_eq!(parsed.subdir, None);
    }

    // -- add() integration tests (Layer 1 + 2: URL parsing + --subdir wiring) --

    #[test]
    fn add_with_tree_url_writes_branch_and_subdir() {
        let tmp = tempfile::TempDir::new().unwrap();
        let config_path = tmp.path().join("tome.toml");
        let lib_dir = tmp.path().join("library");
        std::fs::create_dir_all(&lib_dir).unwrap();
        let mut config = Config {
            library_dir: lib_dir,
            ..Config::default()
        };

        let opts = AddOptions {
            url: "signerlabs/shipswift-skills/tree/main/skills",
            name: None,
            branch: None,
            tag: None,
            rev: None,
            subdir: None,
            role: None,
            dry_run: false,
            config_path: &config_path,
        };
        add(&mut config, opts).unwrap();

        let dir_name = DirectoryName::new("shipswift-skills").unwrap();
        let entry = config.directories.get(&dir_name).expect("directory added");
        assert_eq!(
            entry.path,
            PathBuf::from("https://github.com/signerlabs/shipswift-skills")
        );
        assert!(matches!(entry.git_ref, Some(GitRef::Branch(ref b)) if b == "main"));
        assert_eq!(entry.subdir.as_deref(), Some("skills"));
    }

    #[test]
    fn add_with_subdir_flag_sets_subdir() {
        let tmp = tempfile::TempDir::new().unwrap();
        let config_path = tmp.path().join("tome.toml");
        let lib_dir = tmp.path().join("library");
        std::fs::create_dir_all(&lib_dir).unwrap();
        let mut config = Config {
            library_dir: lib_dir,
            ..Config::default()
        };

        let opts = AddOptions {
            url: "owner/repo",
            name: None,
            branch: None,
            tag: None,
            rev: None,
            subdir: Some("packages"),
            role: None,
            dry_run: false,
            config_path: &config_path,
        };
        add(&mut config, opts).unwrap();

        let dir_name = DirectoryName::new("repo").unwrap();
        let entry = config.directories.get(&dir_name).expect("directory added");
        assert_eq!(entry.subdir.as_deref(), Some("packages"));
        // No URL-embedded branch, no --branch flag → no git_ref.
        assert!(entry.git_ref.is_none());
    }

    #[test]
    fn add_explicit_subdir_overrides_url_subdir() {
        let tmp = tempfile::TempDir::new().unwrap();
        let config_path = tmp.path().join("tome.toml");
        let lib_dir = tmp.path().join("library");
        std::fs::create_dir_all(&lib_dir).unwrap();
        let mut config = Config {
            library_dir: lib_dir,
            ..Config::default()
        };

        // URL says `skills`, flag says `packages` — flag should win.
        let opts = AddOptions {
            url: "owner/repo/tree/main/skills",
            name: None,
            branch: None,
            tag: None,
            rev: None,
            subdir: Some("packages"),
            role: None,
            dry_run: false,
            config_path: &config_path,
        };
        add(&mut config, opts).unwrap();

        let dir_name = DirectoryName::new("repo").unwrap();
        let entry = config.directories.get(&dir_name).expect("directory added");
        assert_eq!(entry.subdir.as_deref(), Some("packages"));
        // URL branch still applies since --branch wasn't set.
        assert!(matches!(entry.git_ref, Some(GitRef::Branch(ref b)) if b == "main"));
    }

    #[test]
    fn add_explicit_branch_overrides_url_branch() {
        let tmp = tempfile::TempDir::new().unwrap();
        let config_path = tmp.path().join("tome.toml");
        let lib_dir = tmp.path().join("library");
        std::fs::create_dir_all(&lib_dir).unwrap();
        let mut config = Config {
            library_dir: lib_dir,
            ..Config::default()
        };

        // URL says `main`, --branch says `dev` — flag wins.
        let opts = AddOptions {
            url: "owner/repo/tree/main/skills",
            name: None,
            branch: Some("dev"),
            tag: None,
            rev: None,
            subdir: None,
            role: None,
            dry_run: false,
            config_path: &config_path,
        };
        add(&mut config, opts).unwrap();

        let dir_name = DirectoryName::new("repo").unwrap();
        let entry = config.directories.get(&dir_name).expect("directory added");
        assert!(matches!(entry.git_ref, Some(GitRef::Branch(ref b)) if b == "dev"));
        // URL subdir still applies since --subdir wasn't set.
        assert_eq!(entry.subdir.as_deref(), Some("skills"));
    }

    // -- Phase 20 (v0.14): --role flag tests --

    #[test]
    fn add_with_explicit_role_writes_role_to_config() {
        let tmp = tempfile::TempDir::new().unwrap();
        let config_path = tmp.path().join("tome.toml");
        let lib_dir = tmp.path().join("library");
        std::fs::create_dir_all(&lib_dir).unwrap();
        let mut config = Config {
            library_dir: lib_dir,
            ..Config::default()
        };
        let opts = AddOptions {
            url: "owner/repo",
            name: None,
            branch: None,
            tag: None,
            rev: None,
            subdir: None,
            role: Some(DirectoryRole::Source),
            dry_run: false,
            config_path: &config_path,
        };
        add(&mut config, opts).unwrap();
        let dir_name = DirectoryName::new("repo").unwrap();
        let entry = config.directories.get(&dir_name).expect("directory added");
        assert_eq!(entry.role, Some(DirectoryRole::Source));
    }

    #[test]
    fn add_with_no_role_leaves_role_none_for_type_default() {
        let tmp = tempfile::TempDir::new().unwrap();
        let config_path = tmp.path().join("tome.toml");
        let lib_dir = tmp.path().join("library");
        std::fs::create_dir_all(&lib_dir).unwrap();
        let mut config = Config {
            library_dir: lib_dir,
            ..Config::default()
        };
        let opts = AddOptions {
            url: "owner/repo",
            name: None,
            branch: None,
            tag: None,
            rev: None,
            subdir: None,
            role: None,
            dry_run: false,
            config_path: &config_path,
        };
        add(&mut config, opts).unwrap();
        let dir_name = DirectoryName::new("repo").unwrap();
        let entry = config.directories.get(&dir_name).expect("directory added");
        assert_eq!(
            entry.role, None,
            "no --role should leave role unset for type-default"
        );
        assert_eq!(entry.role(), DirectoryRole::Source);
    }

    #[test]
    fn add_with_invalid_role_for_git_type_bails() {
        // Git directories can only be Source. --role target is invalid;
        // expect a clear error naming the invalid role + the valid roles.
        let tmp = tempfile::TempDir::new().unwrap();
        let config_path = tmp.path().join("tome.toml");
        let lib_dir = tmp.path().join("library");
        std::fs::create_dir_all(&lib_dir).unwrap();
        let mut config = Config {
            library_dir: lib_dir,
            ..Config::default()
        };
        let opts = AddOptions {
            url: "owner/repo",
            name: None,
            branch: None,
            tag: None,
            rev: None,
            subdir: None,
            role: Some(DirectoryRole::Target),
            dry_run: false,
            config_path: &config_path,
        };
        let err = add(&mut config, opts).unwrap_err();
        let msg = format!("{err:#}");
        assert!(
            msg.contains("target") && msg.contains("not valid"),
            "error should name the invalid role + 'not valid'; got: {msg}"
        );
        assert!(
            msg.contains("source"),
            "error should list valid roles; got: {msg}"
        );
    }

    #[test]
    fn directory_role_kebab_case_matches_serde_wire_format() {
        // The accessor used in success-message echo + error messages must
        // match what serde reads from tome.toml AND what clap parses from
        // --role on the CLI. Compile-time exhaustiveness via match arms.
        assert_eq!(DirectoryRole::Managed.kebab_case(), "managed");
        assert_eq!(DirectoryRole::Synced.kebab_case(), "synced");
        assert_eq!(DirectoryRole::Source.kebab_case(), "source");
        assert_eq!(DirectoryRole::Target.kebab_case(), "target");
    }
}
