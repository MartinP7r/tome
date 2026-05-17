//! Marketplace adapter trait and shared types.
//!
//! This module defines the [`MarketplaceAdapter`] trait that pluggable
//! marketplace implementations (Claude CLI, git, future: npm/etc.) must
//! satisfy, plus the [`InstalledPlugin`] data type they return.
//!
//! Phase 12 ships the contract + adapter implementations; Phase 13 wires the
//! dispatch into `lib.rs::sync`. All trait methods return [`anyhow::Result`]
//! per the project-wide error-handling convention.

use std::cell::RefCell;
use std::path::PathBuf;

use anyhow::{Context, Result};
use console::style;
use serde::Deserialize;

use crate::config::{DirectoryConfig, GitRef};
use crate::git;
use crate::paths::TomePaths;

/// A plugin currently installed via a marketplace adapter.
///
/// Adapters return `Vec<InstalledPlugin>` from [`MarketplaceAdapter::list_installed`].
/// This type is distinct from `manifest::SkillEntry` — `SkillEntry` describes
/// what's in the library, while `InstalledPlugin` describes what's installed at
/// the marketplace level. Phase 13's reconciliation flow bridges the two.
//
// dead_code allow: Phase 12 ships the trait + adapters + tests. The first
// non-test consumer is Phase 13's sync dispatcher (RECON-*), which calls
// `list_installed()` and stores `InstalledPlugin` values for drift detection.
#[derive(Debug, Clone)]
pub struct InstalledPlugin {
    /// Stable plugin identifier from the marketplace.
    ///
    /// Claude marketplace: `"axiom@axiom-marketplace"` (qualified id).
    /// Git: the repository URL string.
    pub id: String,

    /// Display-only version string. Per Phase 11 D-08, drift detection uses
    /// `content_hash`, not `version` — wrong/stale strings degrade UX but do
    /// not produce incorrect drift behavior.
    ///
    /// Claude marketplace: semver string from `claude plugin list --json`
    /// (e.g. `"3.3.0"`; may be the literal `"unknown"` for some entries).
    /// Git: HEAD SHA from `git rev-parse HEAD`.
    pub version: String,

    /// Filesystem location of the installed artifact.
    ///
    /// Claude marketplace: cache path (e.g. `~/.claude/plugins/cache/...`).
    /// Git: repos cache path (e.g. `~/.tome/repos/<sha256(url)>/`).
    pub install_path: PathBuf,

    /// Marketplace-supplied error strings attached to this entry.
    ///
    /// Claude marketplace: from the `errors[]` JSON field of `claude plugin list --json`.
    /// Carries the "vanished plugin" signal consumed by Phase 13's RECON-04
    /// (a non-empty `errors[]` containing "not found in marketplace" indicates
    /// the plugin can no longer be obtained).
    /// Git: usually empty.
    pub errors: Vec<String>,
}

/// Trait implemented by marketplace-specific install/update backends.
///
/// Each [`crate::config::DirectoryType`] that participates in plugin lifecycle
/// (Claude marketplace, git) maps to one adapter implementation. Phase 12
/// ships the trait + concrete adapters; Phase 13 wires a `DirectoryConfig`
/// dispatcher that returns `Box<dyn MarketplaceAdapter>` for each entry.
///
/// All methods return `anyhow::Result`. `plugin_id` is `&str` (not a newtype)
/// because marketplace identifiers carry an `@marketplace` suffix that's
/// incompatible with `SkillName` validation (e.g. `"axiom@axiom-marketplace"`).
//
// dead_code allow: see InstalledPlugin above. The trait is implemented by
// `GitAdapter` (Plan 12-03) and `MockMarketplaceAdapter` (Plan 12-01 tests),
// but neither has a non-test caller yet. Drop when Phase 13's sync dispatcher
// constructs `Box<dyn MarketplaceAdapter>` for each `DirectoryConfig`.
pub trait MarketplaceAdapter {
    /// Stable identifier for this adapter instance (e.g. git URL, or
    /// `"claude-plugins"` for the singleton Claude adapter).
    fn id(&self) -> &str;

    /// Display-only version string for human-readable diffs. Per Phase 11
    /// D-08, drift detection uses `content_hash`, not `version`. Returns
    /// `Ok(None)` when the plugin isn't locally installed.
    fn current_version(&self, plugin_id: &str) -> Result<Option<String>>;

    /// Install the plugin. ClaudeMarketplaceAdapter's snapshot cache
    /// auto-invalidates on `Ok` per D-04 — callers don't need to invalidate
    /// manually.
    fn install(&self, plugin_id: &str) -> Result<()>;

    /// Update the plugin to the marketplace's latest version. No version
    /// pinning per D-09 / LIB-FUTURE-01 — `claude plugin update` doesn't
    /// accept `--version`.
    fn update(&self, plugin_id: &str) -> Result<()>;

    /// Snapshot of installed plugins from this marketplace. May be cached
    /// internally per adapter (D-04). Phase 13 calls this once per sync to
    /// drive drift detection.
    fn list_installed(&self) -> Result<Vec<InstalledPlugin>>;

    /// Returns `false` when the plugin is no longer obtainable from the
    /// marketplace (RECON-04 vanished signal). Adapters MAY satisfy this from
    /// the same cached snapshot used by [`Self::list_installed`] (e.g. by
    /// inspecting the `errors[]` field — see D-02).
    fn available(&self, plugin_id: &str) -> Result<bool>;
}

/// Which adapter operation produced an [`InstallFailure`].
///
/// `Install` originates from [`MarketplaceAdapter::install`]; `Update` from
/// [`MarketplaceAdapter::update`]. Used by the grouped failure renderer to
/// surface "what was attempted" alongside "what went wrong".
//
// dead_code allow: variants are constructed by Task 2's renderer tests in this
// same plan; the production renderer formats them via `{:?}` (Debug derive).
// First non-test producer arrives in Plan 12-04 (ClaudeMarketplaceAdapter).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InstallOp {
    Install,
    Update,
}

/// Heuristic classification of an install/update failure.
///
/// The mapping from upstream stderr to a variant is best-effort
/// (e.g. `"not found in marketplace"` -> `NotFound`). Default is `Unknown`
/// when no specific signal matches; the `source` field of [`InstallFailure`]
/// always carries the verbatim upstream error chain so the user-visible
/// grouped summary can fall back to it.
///
/// Mirrors `crate::remove::FailureKind` (POLISH-04 pattern from Phase 10) —
/// a fixed-size [`Self::ALL`] array + compile-time exhaustiveness sentinel
/// pin "every variant is enumerated" at compile time.
//
// dead_code allow: variants are first constructed in Task 2's renderer tests
// in this same plan; the renderer iterates `Self::ALL` and calls `label()`
// from production code in Task 2. First non-test producer arrives in Plan
// 12-04 (ClaudeMarketplaceAdapter heuristic stderr -> kind mapper). Drop this
// attr when the first non-test caller lands.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InstallFailureKind {
    /// Plugin / URL not in marketplace or not reachable.
    NotFound,
    /// Transient network failure — retry might succeed.
    NetworkError,
    /// Filesystem or auth permission denied.
    PermissionDenied,
    /// Catch-all; the [`InstallFailure::source`] field carries the detail.
    Unknown,
}

impl InstallFailureKind {
    /// All variants, in the order preferred for user-facing grouped output.
    ///
    /// Exposed as an associated constant so the renderer doesn't maintain a
    /// parallel hand-written array that could silently drop a variant when
    /// new variants are added. Mirrors `crate::remove::FailureKind::ALL`
    /// (POLISH-04 pattern).
    pub const ALL: [InstallFailureKind; 4] = [
        InstallFailureKind::NotFound,
        InstallFailureKind::NetworkError,
        InstallFailureKind::PermissionDenied,
        InstallFailureKind::Unknown,
    ];

    /// Human-readable label used in the grouped failure summary.
    //
    // dead_code allow: Task 2 of this plan adds the production renderer that
    // calls `kind.label()`; once that lands, this attr is dropped. The method
    // is also exercised by the `install_failure_kind_label_coverage` test.
    pub fn label(self) -> &'static str {
        match self {
            InstallFailureKind::NotFound => "Not found",
            InstallFailureKind::NetworkError => "Network error",
            InstallFailureKind::PermissionDenied => "Permission denied",
            InstallFailureKind::Unknown => "Unknown",
        }
    }
}

/// Compile-time drift guard for [`InstallFailureKind::ALL`] (POLISH-04 option c).
///
/// If a new variant is added to [`InstallFailureKind`], this `const fn` fails
/// to compile because the match below is exhaustive. The fix is to (a) add an
/// arm here AND (b) append the new variant to `ALL`. Mirrors the
/// `_ensure_failure_kind_all_exhaustive` sentinel in `crate::remove`.
///
/// The function is dead-code at runtime — its sole purpose is the
/// exhaustiveness check. The `const _: () = ...` block below additionally
/// pins `ALL.len() == 4` at compile time so a hand-edit that adds a match
/// arm here without growing `ALL` (or vice versa) also fails.
const fn _ensure_install_failure_kind_all_exhaustive(k: InstallFailureKind) -> usize {
    match k {
        InstallFailureKind::NotFound => 0,
        InstallFailureKind::NetworkError => 1,
        InstallFailureKind::PermissionDenied => 2,
        InstallFailureKind::Unknown => 3,
    }
}

const _: () = {
    // If this fails: InstallFailureKind::ALL is missing or has extra variants.
    // The match arms in _ensure_install_failure_kind_all_exhaustive are the
    // source of truth — ALL must contain exactly one entry per arm.
    assert!(InstallFailureKind::ALL.len() == 4);
};

/// A single install/update failure aggregated across adapter calls.
///
/// Mirrors `crate::remove::RemoveFailure` (SAFE-01 pattern from Phase 8) but
/// with marketplace-meaningful fields:
///
/// - No `path` field — install-time failures don't have a stable filesystem
///   path the way distribution-symlink removals do.
/// - Adds `adapter_id`, `plugin_id`, and `operation` so the grouped renderer
///   can show "which adapter, which plugin, install vs update".
/// - `source` is `anyhow::Error` (vs `RemoveFailure::error: std::io::Error`)
///   to preserve the upstream `claude` / `git` error chain verbatim.
///
/// Derives `Debug` only — `anyhow::Error` is neither `Clone` nor `PartialEq`,
/// so test assertions inspect individual fields rather than struct equality.
//
// dead_code allow: Phase 12 ships the type + the renderer (Plan 12-02 Task 2).
// The first non-test producer arrives in Plan 12-04 (ClaudeMarketplaceAdapter
// constructs `InstallFailure` from upstream stderr); Phase 13 aggregates the
// `Vec<InstallFailure>` across adapter calls. Drop this attr when the first
// non-test caller lands.
#[derive(Debug)]
pub struct InstallFailure {
    /// Adapter that produced the failure — typically the adapter's
    /// [`MarketplaceAdapter::id`] (e.g. `"claude-plugins-official"`, or a git URL).
    pub adapter_id: String,

    /// Plugin identifier passed to the failed call (e.g.
    /// `"axiom@axiom-marketplace"`, or a skill name for git).
    pub plugin_id: String,

    /// Which adapter operation was attempted.
    pub operation: InstallOp,

    /// Best-effort kind classification (see [`InstallFailureKind`]).
    pub kind: InstallFailureKind,

    /// Verbatim upstream error chain — the renderer surfaces this with `{:#}`
    /// so users see the full anyhow chain.
    pub source: anyhow::Error,
}

/// Format a slice of [`InstallFailure`] into the SAFE-01 grouped failure summary.
///
/// Mirrors the inline rendering block in `lib.rs::Command::Remove` (search for
/// `"operations failed during remove"`) but adapted for marketplace adapter
/// failures. Returns the rendered string so tests can assert on exact output;
/// production callers go through [`render_install_failures`] which `eprint!`s
/// the result.
///
/// Returns an empty string when `failures` is empty so callers can safely
/// concatenate without checking length first.
//
// dead_code allow: the production caller is Phase 13's sync flow (RECON-*).
// Plan 12-02 ships only the renderer + tests; the wrapper [`render_install_failures`]
// and the renderer tests in this file exercise both functions. Drop this attr
// when Phase 13 wires the call from `lib.rs::sync`.
pub(crate) fn format_install_failures(failures: &[InstallFailure]) -> String {
    if failures.is_empty() {
        return String::new();
    }
    let k = failures.len();
    let mut out = String::new();
    out.push_str(&format!(
        "{} {} install operations failed\n",
        style("⚠").yellow(),
        k,
    ));
    for kind in InstallFailureKind::ALL {
        let group: Vec<&InstallFailure> = failures.iter().filter(|f| f.kind == kind).collect();
        if group.is_empty() {
            continue;
        }
        out.push_str(&format!("  {} ({}):\n", kind.label(), group.len()));
        for f in group {
            out.push_str(&format!(
                "    {}/{} ({:?}): {:#}\n",
                f.adapter_id, f.plugin_id, f.operation, f.source,
            ));
        }
    }
    out
}

/// Emit the SAFE-01 grouped failure summary to stderr.
///
/// Per ADP-04 / D-07: aggregates `Vec<InstallFailure>` from adapter calls into
/// a grouped summary. Phase 13's sync flow calls this before deciding the
/// process exit code; the renderer itself does not return `Err` —
/// non-zero-exit-on-partial-failure is the caller's responsibility per ADP-04.
///
/// Empty input is a no-op (zero stderr output).
pub fn render_install_failures(failures: &[InstallFailure]) {
    let rendered = format_install_failures(failures);
    if !rendered.is_empty() {
        eprint!("{rendered}");
    }
}

/// Marketplace adapter for git-type directories.
///
/// Per CONTEXT.md D-05: one `GitAdapter` per `[directories.<git-name>]` config
/// entry. The adapter is bound to a single URL + ref pin and delegates all
/// operations to [`crate::git`] verbatim — behavior is byte-for-byte unchanged
/// from v0.9 (D-05a regression contract).
///
/// The `plugin_id: &str` argument on trait methods is ignored: there's only
/// one "plugin" per git directory (the repo itself).
//
// dead_code allow: Phase 12 ships the adapter + tests. Phase 13's sync flow
// (D-11 dispatcher) is the first non-test consumer — it constructs a
// `GitAdapter::for_directory(...)` for each `DirectoryType::Git` entry. Drop
// this attr (and the `#[allow]`s on the inherent methods below) when Phase 13
// wires the dispatch.
pub struct GitAdapter {
    url: String,
    cache_dir: PathBuf,
    git_ref: Option<GitRef>,
}

impl GitAdapter {
    /// Construct a `GitAdapter` from a git-type [`DirectoryConfig`].
    ///
    /// Returns `Err` if the directory's `path` field (which holds the git URL
    /// for git-type directories) is not valid UTF-8. Mirrors the existing
    /// pattern at `crate::remove::plan` (remove.rs:241-244).
    ///
    /// The cache directory is precomputed at construction so repeat calls to
    /// trait methods don't re-hash the URL.
    pub fn for_directory(dir_config: &DirectoryConfig, paths: &TomePaths) -> Result<Self> {
        let url = dir_config
            .path
            .to_str()
            .ok_or_else(|| {
                anyhow::anyhow!(
                    "git directory path is not valid UTF-8: {}",
                    dir_config.path.display()
                )
            })?
            .to_string();
        let cache_dir = git::repo_cache_dir(&paths.repos_dir(), &url);
        Ok(Self {
            url,
            cache_dir,
            git_ref: dir_config.git_ref.clone(),
        })
    }

    fn ref_branch(&self) -> Option<&str> {
        self.git_ref.as_ref().and_then(|r| r.branch())
    }
    fn ref_tag(&self) -> Option<&str> {
        self.git_ref.as_ref().and_then(|r| r.tag())
    }
    fn ref_rev(&self) -> Option<&str> {
        self.git_ref.as_ref().and_then(|r| r.rev())
    }
}

impl MarketplaceAdapter for GitAdapter {
    fn id(&self) -> &str {
        &self.url
    }

    fn current_version(&self, _plugin_id: &str) -> Result<Option<String>> {
        if !self.cache_dir.exists() {
            return Ok(None);
        }
        git::read_head_sha(&self.cache_dir).map(Some)
    }

    fn install(&self, _plugin_id: &str) -> Result<()> {
        git::clone_repo(
            &self.url,
            &self.cache_dir,
            self.ref_branch(),
            self.ref_tag(),
            self.ref_rev(),
        )
    }

    fn update(&self, _plugin_id: &str) -> Result<()> {
        git::update_repo(
            &self.cache_dir,
            self.ref_branch(),
            self.ref_tag(),
            self.ref_rev(),
        )
    }

    fn list_installed(&self) -> Result<Vec<InstalledPlugin>> {
        if !self.cache_dir.exists() {
            return Ok(vec![]);
        }
        let version = git::read_head_sha(&self.cache_dir)?;
        Ok(vec![InstalledPlugin {
            id: self.url.clone(),
            version,
            install_path: self.cache_dir.clone(),
            errors: vec![],
        }])
    }

    fn available(&self, _plugin_id: &str) -> Result<bool> {
        // Per RESEARCH Q #5: trust local-clone existence. Git URLs don't
        // "vanish" the way marketplace plugins do; a missing local clone is a
        // "not yet installed" signal, not a "vanished" signal. If a future
        // feature needs a true vanished probe, add a
        // `git ls-remote --exit-code` call here.
        Ok(self.cache_dir.exists())
    }
}

// ---- ClaudeMarketplaceAdapter — production adapter shelling to `claude` CLI ----
//
// Per CONTEXT.md D-01..D-04, D-08, D-09 (locked, verbatim):
// - D-01: subprocesses run with `stdin = Stdio::null()`, no env tweaks, stderr
//   captured verbatim, non-zero exit = failure, no timeout.
// - D-02: `available()` reads the cached `claude plugin list --json` snapshot's
//   `errors[]` field for the entry — zero extra subprocess calls.
// - D-04: internal `RefCell<Option<Vec<InstalledPlugin>>>` cache. First read
//   populates; `install()`/`update()` returning `Ok` auto-invalidates; public
//   `refresh()` forces re-query.
// - D-08: id() is the stable string `"claude-plugins"`.
// - D-09: install/update use default scope (user) — no `--scope` flag.

/// Wire format for `claude plugin list --json` entries.
///
/// Verified live 2026-05-05 against claude 2.1.128. The full shape includes
/// additional fields (`scope`, `enabled`, `installedAt`, `lastUpdated`,
/// `mcpServers`) that the adapter does NOT consume — serde silently drops the
/// rest because we don't apply `#[serde(deny_unknown_fields)]`. The `errors`
/// field is OPTIONAL (absent when an entry has no marketplace errors);
/// `#[serde(default)]` handles the absence case.
#[derive(Deserialize)]
struct ClaudePluginListEntry {
    id: String,
    version: String,
    #[serde(rename = "installPath")]
    install_path: PathBuf,
    #[serde(default)]
    errors: Vec<String>,
}

/// Parse `claude plugin list --json` output into `Vec<InstalledPlugin>`.
///
/// Pure function (no I/O) so the JSON deserialization can be tested with
/// hand-rolled fixtures in CI environments that don't have `claude`
/// installed. Tolerates extra fields (the live snapshot includes `scope`,
/// `enabled`, `installedAt`, `lastUpdated`, `mcpServers`) and the literal
/// `version: "unknown"` string observed on some entries.
pub(crate) fn parse_claude_plugin_list_json(input: &str) -> Result<Vec<InstalledPlugin>> {
    let entries: Vec<ClaudePluginListEntry> = serde_json::from_str(input)
        .context("failed to parse `claude plugin list --json` output")?;
    Ok(entries
        .into_iter()
        .map(|e| InstalledPlugin {
            id: e.id,
            version: e.version,
            install_path: e.install_path,
            errors: e.errors,
        })
        .collect())
}

/// Heuristic mapping from a `claude plugin install/update` stderr message to
/// an [`InstallFailureKind`] variant.
///
/// Substring-based and best-effort. Empirical mapping (verified against
/// claude 2.1.128, 2026-05-05):
///
/// - `Plugin "X" not found in marketplace "Y"` -> `NotFound`
/// - `Plugin "X" not found` (update path) -> `NotFound`
/// - all else -> `Unknown` (the [`InstallFailure::source`] field carries the
///   verbatim error chain so the user-visible grouped summary still surfaces
///   the upstream message).
///
/// `NetworkError` and `PermissionDenied` are reserved for future heuristics
/// once empirical evidence surfaces; they exist in the enum so the renderer's
/// grouped output is forward-compatible.
//
// Currently exercised only via tests; production callers will land when
// `lib.rs::sync` wraps adapter `install`/`update` errors into
// `Vec<InstallFailure>` (tracked separately — see #518 ClaudeMarketplaceAdapter
// error-chain capture). The `_coverage` and `_basic` unit tests pin the
// behavior so callers can rely on it when they come online.
#[allow(dead_code)]
pub(crate) fn classify_claude_install_stderr(stderr: &str) -> InstallFailureKind {
    // Both "not found in marketplace" and bare "not found" map to NotFound,
    // expressed as a single OR rather than two arms with identical bodies
    // (clippy::if_same_then_else). Comments below preserve the empirical
    // mapping the heuristic was derived from.
    if stderr.contains("not found in marketplace")
        // ^ install path: `Plugin "X" not found in marketplace "Y"`
        || stderr.contains("not found")
    // ^ update path (less specific): `Plugin "X" not found`
    {
        InstallFailureKind::NotFound
    } else {
        InstallFailureKind::Unknown
    }
}

/// Check whether the `claude` CLI is available on PATH.
///
/// Mirrors [`crate::git::is_git_available`]. Used at adapter construction to
/// fail fast with a clear actionable error if the binary is missing, and by
/// smoke tests to skip subprocess-dependent assertions in CI environments
/// that lack the binary.
//
// dead_code allow: production caller is `ClaudeMarketplaceAdapter::new`, which
// itself has no non-test consumer until Phase 13's D-11 dispatcher constructs
// the adapter. Smoke tests in this module also call this directly. Drop this
// attr when the dispatcher lands.
pub fn is_claude_available() -> bool {
    std::process::Command::new("claude")
        .arg("--version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// Run a `claude` subcommand with stdin closed (per D-01).
///
/// Returns the raw `Output` so callers can inspect stdout AND stderr. Maps
/// `ErrorKind::NotFound` to a clear error message naming the binary — this
/// shouldn't happen in practice because [`ClaudeMarketplaceAdapter::new`]
/// probes `claude --version` at construction, but provides a safety net if
/// the binary disappears between construction and use (mirrors
/// `install.rs:57` for the existing pattern).
fn run_claude_subcommand(args: &[&str]) -> Result<std::process::Output> {
    match std::process::Command::new("claude")
        .args(args)
        .stdin(std::process::Stdio::null())
        .output()
    {
        Ok(output) => Ok(output),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Err(anyhow::anyhow!(
            "claude CLI not found on PATH (vanished between adapter construction and `claude {}`)",
            args.join(" ")
        )),
        Err(e) => {
            Err(anyhow::Error::from(e)
                .context(format!("failed to run `claude {}`", args.join(" "))))
        }
    }
}

/// Marketplace adapter for the Claude Code plugin ecosystem.
///
/// Per CONTEXT.md D-01..D-04, D-08, D-09: shells out synchronously to
/// `claude plugin install/update/list`, holds an internal cache of the
/// `claude plugin list --json` snapshot to amortize subprocess cost, and
/// auto-invalidates the cache on successful install/update.
///
/// - D-01: stdin closed; no env tweaks; stderr captured verbatim; non-zero
///   exit = failure; no timeout.
/// - D-02: `available()` reads the cached snapshot's `errors[]` field for
///   the entry — zero extra subprocess calls.
/// - D-04: `RefCell<Option<Vec<InstalledPlugin>>>` cache; first read
///   populates; `install()`/`update()` returning `Ok` auto-invalidates;
///   public [`Self::refresh`] forces re-query.
/// - D-08: [`MarketplaceAdapter::id`] returns the stable string
///   `"claude-plugins"`.
/// - D-09: uses default scope (user) — no `--scope` flag passed.
//
// dead_code allow: Phase 12 ships the adapter + tests. Phase 13's sync flow
// (D-11 dispatcher) is the first non-test consumer — it constructs a
// `ClaudeMarketplaceAdapter::new()` for `DirectoryType::ClaudePlugins`
// directories. Drop this attr (and the `#[allow]` on the inherent impl block
// below) when the dispatcher lands. The trait impl block follows the trait's
// reachability and does not need its own attr.
pub struct ClaudeMarketplaceAdapter {
    cache: RefCell<Option<Vec<InstalledPlugin>>>,
}

impl ClaudeMarketplaceAdapter {
    /// Construct a `ClaudeMarketplaceAdapter`, probing `claude --version` to
    /// fail fast if the binary is missing.
    ///
    /// Per ADP-02: surfaces "claude not on PATH" as a clear, actionable
    /// error naming the binary and suggesting the user either install Claude
    /// Code or remove `[directories.<name>]` entries with
    /// `type = "claude-plugins"` from `tome.toml`.
    pub fn new() -> Result<Self> {
        if !is_claude_available() {
            anyhow::bail!(
                "claude CLI not found on PATH — install Claude Code, \
                 or remove [directories.<name>] entries with \
                 type = \"claude-plugins\" from tome.toml"
            );
        }
        Ok(Self {
            cache: RefCell::new(None),
        })
    }

    /// Test-only constructor that bypasses the binary probe.
    ///
    /// Used by unit tests that exercise trait methods against a
    /// pre-populated cache without requiring `claude` on PATH. Production
    /// code MUST go through [`Self::new`] so the actionable error message
    /// fires for users who haven't installed Claude Code.
    #[cfg(test)]
    pub(crate) fn new_for_test() -> Self {
        Self {
            cache: RefCell::new(None),
        }
    }

    /// Force re-query of the `claude plugin list --json` snapshot.
    ///
    /// Per D-04: `install()`/`update()` auto-invalidate on success — explicit
    /// `refresh()` is for cases where external state may have changed
    /// between trait calls (e.g. user ran `claude plugin install` from
    /// another shell while `tome` was running).
    pub fn refresh(&self) -> Result<()> {
        *self.cache.borrow_mut() = None;
        self.populate_cache()
    }

    /// Run `claude plugin list --json`, parse the output, and populate the
    /// cache.
    ///
    /// No-op if the cache is already populated. Called by the read-side
    /// trait methods (`current_version`, `list_installed`, `available`) and
    /// by `refresh()`.
    fn populate_cache(&self) -> Result<()> {
        if self.cache.borrow().is_some() {
            return Ok(());
        }
        let output = run_claude_subcommand(&["plugin", "list", "--json"])?;
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!("`claude plugin list --json` failed: {}", stderr.trim());
        }
        let stdout = String::from_utf8_lossy(&output.stdout);
        let parsed = parse_claude_plugin_list_json(&stdout)?;
        *self.cache.borrow_mut() = Some(parsed);
        Ok(())
    }

    /// Construct an [`InstallFailure`] from a stderr string using the
    /// heuristic classifier.
    ///
    /// Pulled out as a testable `pub(crate)` helper so the stderr-to-failure
    /// conversion can be exercised in unit tests without spawning a
    /// subprocess. The production sync flow may call this helper directly
    /// when it wraps adapter `install`/`update` errors into
    /// `Vec<InstallFailure>` for the grouped renderer (tracked separately).
    #[allow(dead_code)]
    pub(crate) fn build_install_failure(
        adapter_id: &str,
        plugin_id: &str,
        op: InstallOp,
        stderr: &str,
    ) -> InstallFailure {
        InstallFailure {
            adapter_id: adapter_id.to_string(),
            plugin_id: plugin_id.to_string(),
            operation: op,
            kind: classify_claude_install_stderr(stderr),
            source: anyhow::anyhow!("{}", stderr.trim()),
        }
    }
}

impl MarketplaceAdapter for ClaudeMarketplaceAdapter {
    fn id(&self) -> &str {
        "claude-plugins"
    }

    fn current_version(&self, plugin_id: &str) -> Result<Option<String>> {
        self.populate_cache()?;
        Ok(self
            .cache
            .borrow()
            .as_ref()
            .and_then(|list| list.iter().find(|p| p.id == plugin_id))
            .map(|p| p.version.clone()))
    }

    fn install(&self, plugin_id: &str) -> Result<()> {
        // Per D-09: no `--scope` flag — uses the default scope (user).
        let output = run_claude_subcommand(&["plugin", "install", plugin_id])?;
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!("{}", stderr.trim());
        }
        // Per D-04: auto-invalidate cache on Ok so the next read re-queries.
        *self.cache.borrow_mut() = None;
        Ok(())
    }

    fn update(&self, plugin_id: &str) -> Result<()> {
        let output = run_claude_subcommand(&["plugin", "update", plugin_id])?;
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!("{}", stderr.trim());
        }
        *self.cache.borrow_mut() = None;
        Ok(())
    }

    fn list_installed(&self) -> Result<Vec<InstalledPlugin>> {
        self.populate_cache()?;
        let cache = self.cache.borrow();
        // populate_cache()? returning Ok() should always leave cache as
        // Some(_) — if it doesn't, a downstream caller (reconcile) would
        // treat empty as "no plugins installed" and silently skip every
        // managed update. Per #514 we don't panic from trait methods, but
        // we DO surface the invariant violation as a warn-level event so
        // it's visible in `--verbose` / `TOME_LOG=warn` runs instead of
        // disappearing entirely.
        if cache.is_none() {
            tracing::warn!(
                "ClaudeMarketplaceAdapter::list_installed: populate_cache() \
                 succeeded but cache is None — returning empty list. This is \
                 a programming-error invariant violation; reconcile may skip \
                 managed-plugin updates this sync."
            );
        }
        Ok(cache.clone().unwrap_or_default())
    }

    fn available(&self, plugin_id: &str) -> Result<bool> {
        // Per D-02: zero extra subprocess calls. Reads the cached snapshot's
        // errors[] field for the entry.
        self.populate_cache()?;
        let cache = self.cache.borrow();
        // #514: never panic from a trait method called in the production sync
        // flow. populate_cache()? guarantees Some(_) on success today, but
        // .as_deref().unwrap_or(&[]) degrades safely if a future refactor
        // ever leaves the cache None transiently. The other two cache-read
        // sites (current_version, list_installed) already use this pattern.
        let list = cache.as_deref().unwrap_or(&[]);
        let errored = list.iter().find(|p| p.id == plugin_id).is_some_and(|e| {
            e.errors
                .iter()
                .any(|s| s.contains("not found in marketplace"))
        });
        // Conservative default: entry exists with no marketplace error, OR
        // plugin isn't in the snapshot at all (not yet installed), OR the
        // cache is empty — report `available = true`. The "vanished" signal
        // is purely the errors[] substring.
        Ok(!errored)
    }
}

/// Test-support surface — intentionally feature-gated.
///
/// Per OQ-2 in `13-RESEARCH.md`: integration tests in
/// `crates/tome/tests/cli_sync_reconcile.rs` need a `MarketplaceAdapter` that
/// doesn't shell out to `claude`. Production builds (no features, no
/// `cfg(test)`) MUST NOT compile this module — the gate `cfg(any(test,
/// feature = "test-support"))` excludes it from `cargo build -p tome` and
/// from any v1.0 GUI build that consumes `marketplace::*` via Tauri IPC.
///
/// `pub` (not `pub(crate)`) so external test crates can name
/// `tome::marketplace::testing::MockMarketplaceAdapter`. The `marketplace`
/// module itself is `pub` at lib.rs:42 for the same reason.
#[cfg(any(test, feature = "test-support"))]
pub mod testing {
    use std::collections::HashSet;
    use std::path::PathBuf;

    use anyhow::Result;

    use super::{InstalledPlugin, MarketplaceAdapter};

    /// In-memory `MarketplaceAdapter` for unit + integration tests.
    ///
    /// Construct with explicit fields; no builder. Failure injection via
    /// `fail_install` / `fail_update` `HashSet<String>` lookup. Mirrors the
    /// `#[cfg(test)] pub(super)` mock that lived inside `mod tests` in Phase
    /// 12 — same shape, lifted to the feature-gated surface.
    pub struct MockMarketplaceAdapter {
        pub id: String,
        pub installed: Vec<InstalledPlugin>,
        pub available: HashSet<String>,
        pub fail_install: HashSet<String>,
        pub fail_update: HashSet<String>,
    }

    impl MarketplaceAdapter for MockMarketplaceAdapter {
        fn id(&self) -> &str {
            &self.id
        }

        fn current_version(&self, plugin_id: &str) -> Result<Option<String>> {
            Ok(self
                .installed
                .iter()
                .find(|p| p.id == plugin_id)
                .map(|p| p.version.clone()))
        }

        fn install(&self, plugin_id: &str) -> Result<()> {
            if self.fail_install.contains(plugin_id) {
                anyhow::bail!("mock: install failed for {plugin_id}");
            }
            Ok(())
        }

        fn update(&self, plugin_id: &str) -> Result<()> {
            if self.fail_update.contains(plugin_id) {
                anyhow::bail!("mock: update failed for {plugin_id}");
            }
            Ok(())
        }

        fn list_installed(&self) -> Result<Vec<InstalledPlugin>> {
            Ok(self.installed.clone())
        }

        fn available(&self, plugin_id: &str) -> Result<bool> {
            Ok(self.available.contains(plugin_id))
        }
    }

    /// Build an `InstalledPlugin` fixture with sensible defaults.
    pub fn fixture_plugin(id: &str, version: &str) -> InstalledPlugin {
        InstalledPlugin {
            id: id.to_string(),
            version: version.to_string(),
            install_path: PathBuf::from(format!("/tmp/mock/{id}")),
            errors: Vec::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::testing::{MockMarketplaceAdapter, fixture_plugin};
    use super::*;
    use std::collections::HashSet;
    use std::path::PathBuf;

    /// Build a mock with one installed plugin "known@mp" at version 1.2.3,
    /// "present" available, "doomed" failing both install and update.
    fn make_mock() -> MockMarketplaceAdapter {
        let mut available = HashSet::new();
        available.insert("present".to_string());

        let mut fail_install = HashSet::new();
        fail_install.insert("doomed".to_string());

        let mut fail_update = HashSet::new();
        fail_update.insert("doomed".to_string());

        MockMarketplaceAdapter {
            id: "mock-marketplace".to_string(),
            installed: vec![fixture_plugin("known@mp", "1.2.3")],
            available,
            fail_install,
            fail_update,
        }
    }

    #[test]
    fn mock_lists_installed_and_resolves_versions() {
        let mock = make_mock();
        // Exercise via &dyn MarketplaceAdapter to prove object-safety at the
        // call site (Phase 13 stores adapters as `Box<dyn MarketplaceAdapter>`).
        let adapter: &dyn MarketplaceAdapter = &mock;

        // id() — adapter identity
        assert_eq!(adapter.id(), "mock-marketplace");

        // list_installed() — returns the static fixture verbatim
        let installed = adapter.list_installed().unwrap();
        assert_eq!(installed.len(), 1);
        assert_eq!(installed[0].id, "known@mp");
        assert_eq!(installed[0].version, "1.2.3");
        assert!(installed[0].errors.is_empty());

        // current_version("known") — Some(version)
        let v = adapter.current_version("known@mp").unwrap();
        assert_eq!(v.as_deref(), Some("1.2.3"));

        // current_version("unknown") — Ok(None) per the trait contract
        let v = adapter.current_version("unknown@mp").unwrap();
        assert_eq!(v, None);
    }

    #[test]
    fn mock_available_returns_set_membership() {
        let mock = make_mock();
        let adapter: &dyn MarketplaceAdapter = &mock;

        // present in the available set → Ok(true)
        assert!(adapter.available("present").unwrap());

        // not in the available set → Ok(false) (the RECON-04 "vanished"
        // signal that Phase 13 keys off of for drift classification)
        assert!(!adapter.available("vanished").unwrap());
    }

    #[test]
    fn mock_install_and_update_failure_injection() {
        let mock = make_mock();
        let adapter: &dyn MarketplaceAdapter = &mock;

        // install("doomed") is in fail_install → Err
        let err = adapter.install("doomed").unwrap_err();
        assert!(
            err.to_string().contains("install failed"),
            "expected install-failure error, got: {err}"
        );

        // install("happy") not in fail_install → Ok
        assert!(adapter.install("happy").is_ok());

        // update("doomed") is in fail_update → Err
        let err = adapter.update("doomed").unwrap_err();
        assert!(
            err.to_string().contains("update failed"),
            "expected update-failure error, got: {err}"
        );

        // update("happy") not in fail_update → Ok
        assert!(adapter.update("happy").is_ok());
    }

    #[test]
    fn trait_is_object_safe() {
        // If `MarketplaceAdapter` were not object-safe (e.g. had a generic
        // method or returned `Self`), this line would fail to compile.
        // Phase 13 will store these in collections, so object-safety is a
        // contract requirement of the trait surface.
        let mock = make_mock();
        let _boxed: Box<dyn MarketplaceAdapter> = Box::new(mock);
    }

    // ---- InstallFailureKind tests (mirrors remove::tests::failure_kind_*) ----

    #[test]
    fn install_failure_kind_label_coverage() {
        assert_eq!(InstallFailureKind::NotFound.label(), "Not found");
        assert_eq!(InstallFailureKind::NetworkError.label(), "Network error");
        assert_eq!(
            InstallFailureKind::PermissionDenied.label(),
            "Permission denied"
        );
        assert_eq!(InstallFailureKind::Unknown.label(), "Unknown");
    }

    /// `InstallFailureKind::ALL` is consumed by the grouped failure summary;
    /// pinning length to 4 also pairs with the const-fn drift guard
    /// `_ensure_install_failure_kind_all_exhaustive` so a hand-edit that
    /// grows the enum without growing ALL fails to compile.
    #[test]
    fn install_failure_kind_all_pinned_size_four() {
        assert_eq!(InstallFailureKind::ALL.len(), 4);
        assert!(InstallFailureKind::ALL.contains(&InstallFailureKind::NotFound));
        assert!(InstallFailureKind::ALL.contains(&InstallFailureKind::NetworkError));
        assert!(InstallFailureKind::ALL.contains(&InstallFailureKind::PermissionDenied));
        assert!(InstallFailureKind::ALL.contains(&InstallFailureKind::Unknown));
    }

    // POLISH-04: Pins the runtime drift check that complements the
    // compile-time `_ensure_install_failure_kind_all_exhaustive` sentinel.
    // Uses a hand-rolled uniqueness check (InstallFailureKind only derives
    // PartialEq/Eq, not Ord/Hash, so BTreeSet/HashSet are unavailable).
    #[test]
    fn install_failure_kind_all_length_matches_variant_count() {
        let all = InstallFailureKind::ALL;
        assert_eq!(
            all.len(),
            4,
            "InstallFailureKind::ALL must contain every variant exactly once"
        );
        // Pairwise-unique: no duplicates in ALL.
        for (i, a) in all.iter().enumerate() {
            for b in all.iter().skip(i + 1) {
                assert_ne!(
                    a, b,
                    "InstallFailureKind::ALL contains duplicate variant {a:?}"
                );
            }
        }
        // Membership: every variant appears.
        assert!(all.contains(&InstallFailureKind::NotFound));
        assert!(all.contains(&InstallFailureKind::NetworkError));
        assert!(all.contains(&InstallFailureKind::PermissionDenied));
        assert!(all.contains(&InstallFailureKind::Unknown));
    }

    // POLISH-04: The grouped failure-summary output iterates
    // InstallFailureKind::ALL in declaration order. The user-visible grouping
    // therefore depends on this exact order. A reorder is a UI change and
    // must require an explicit code edit (this test fails on reorder).
    #[test]
    fn install_failure_kind_all_ordering_pinned() {
        assert_eq!(
            InstallFailureKind::ALL,
            [
                InstallFailureKind::NotFound,
                InstallFailureKind::NetworkError,
                InstallFailureKind::PermissionDenied,
                InstallFailureKind::Unknown,
            ],
            "InstallFailureKind::ALL ordering is part of the user-visible grouping contract"
        );
    }

    // ---- render_install_failures / format_install_failures tests ----

    fn make_failure(
        adapter: &str,
        plugin: &str,
        op: InstallOp,
        kind: InstallFailureKind,
        msg: &str,
    ) -> InstallFailure {
        InstallFailure {
            adapter_id: adapter.to_string(),
            plugin_id: plugin.to_string(),
            operation: op,
            kind,
            source: anyhow::anyhow!("{msg}"),
        }
    }

    #[test]
    fn format_install_failures_empty_returns_empty_string() {
        assert_eq!(format_install_failures(&[]), "");
    }

    #[test]
    fn format_install_failures_groups_by_kind_and_skips_empty_groups() {
        let failures = vec![
            make_failure(
                "claude-plugins",
                "axiom@m1",
                InstallOp::Install,
                InstallFailureKind::NotFound,
                "boom-1",
            ),
            make_failure(
                "claude-plugins",
                "beta@m1",
                InstallOp::Install,
                InstallFailureKind::NotFound,
                "boom-2",
            ),
            make_failure(
                "git+ssh://example",
                "repo",
                InstallOp::Update,
                InstallFailureKind::Unknown,
                "boom-3",
            ),
        ];
        let out = format_install_failures(&failures);

        // Header: count + summary text.
        assert!(
            out.contains("3 install operations failed"),
            "header missing count/text; got: {out}"
        );

        // Groups present (non-empty).
        assert!(out.contains("Not found (2):"), "got: {out}");
        assert!(out.contains("Unknown (1):"), "got: {out}");

        // Empty groups skipped (no NetworkError or PermissionDenied entries).
        assert!(
            !out.contains("Network error"),
            "empty group should not appear: {out}"
        );
        assert!(
            !out.contains("Permission denied"),
            "empty group should not appear: {out}"
        );

        // Per-failure lines: adapter_id/plugin_id (Op): source.
        assert!(
            out.contains("claude-plugins/axiom@m1 (Install): boom-1"),
            "missing per-failure detail line for axiom; got: {out}"
        );
        assert!(
            out.contains("claude-plugins/beta@m1 (Install): boom-2"),
            "missing per-failure detail line for beta; got: {out}"
        );
        assert!(
            out.contains("git+ssh://example/repo (Update): boom-3"),
            "missing per-failure detail line for git; got: {out}"
        );

        // Declaration-order grouping: NotFound appears before Unknown.
        let np = out.find("Not found").expect("'Not found' header missing");
        let up = out.find("Unknown").expect("'Unknown' header missing");
        assert!(
            np < up,
            "ordering pinned: NotFound must precede Unknown; got: {out}"
        );
    }

    #[test]
    fn render_install_failures_empty_is_noop() {
        // Pure no-op for empty input — exercising both the wrapper and the
        // formatter's empty-string short-circuit.
        render_install_failures(&[]);
    }

    // ---- GitAdapter tests (mirrors git.rs::tests::read_head_sha_returns_40_char_hex
    // pattern at git.rs:252-289 — real `git init` repos in TempDirs, no network) ----

    /// Initialize a real git repo in `tmp/origin` with one commit. The
    /// resulting directory's path is a valid `clone` source for
    /// `git::clone_repo` — file paths work as git URLs.
    fn make_local_test_repo(tmp: &std::path::Path) -> std::path::PathBuf {
        let repo = tmp.join("origin");
        std::fs::create_dir_all(&repo).unwrap();
        std::process::Command::new("git")
            .args(["init"])
            .current_dir(&repo)
            .output()
            .unwrap();
        std::process::Command::new("git")
            .args(["config", "user.email", "test@test.com"])
            .current_dir(&repo)
            .output()
            .unwrap();
        std::process::Command::new("git")
            .args(["config", "user.name", "Test"])
            .current_dir(&repo)
            .output()
            .unwrap();
        std::fs::write(repo.join("README.md"), "hi").unwrap();
        std::process::Command::new("git")
            .args(["add", "-A"])
            .current_dir(&repo)
            .output()
            .unwrap();
        std::process::Command::new("git")
            .args(["commit", "-m", "init"])
            .current_dir(&repo)
            .output()
            .unwrap();
        repo
    }

    fn make_test_dir_config(path: std::path::PathBuf, git_ref: Option<GitRef>) -> DirectoryConfig {
        DirectoryConfig {
            path,
            directory_type: crate::config::DirectoryType::Git,
            role: None,
            git_ref,
            subdir: None,
            override_applied: false,
        }
    }

    #[test]
    fn git_adapter_id_returns_url() {
        let tmp = tempfile::TempDir::new().unwrap();
        let dir_config = make_test_dir_config(
            std::path::PathBuf::from("https://example.com/repo.git"),
            None,
        );
        let paths = TomePaths::new(tmp.path().to_path_buf(), tmp.path().join("library")).unwrap();
        let adapter = GitAdapter::for_directory(&dir_config, &paths).unwrap();
        assert_eq!(adapter.id(), "https://example.com/repo.git");
    }

    #[test]
    fn git_adapter_current_version_none_when_not_cloned() {
        let tmp = tempfile::TempDir::new().unwrap();
        let dir_config = make_test_dir_config(
            std::path::PathBuf::from("https://example.com/never-cloned.git"),
            None,
        );
        let paths = TomePaths::new(tmp.path().to_path_buf(), tmp.path().join("library")).unwrap();
        let adapter = GitAdapter::for_directory(&dir_config, &paths).unwrap();
        assert_eq!(adapter.current_version("ignored").unwrap(), None);
    }

    #[test]
    fn git_adapter_available_returns_false_when_not_cloned() {
        let tmp = tempfile::TempDir::new().unwrap();
        let dir_config = make_test_dir_config(
            std::path::PathBuf::from("https://example.com/never-cloned.git"),
            None,
        );
        let paths = TomePaths::new(tmp.path().to_path_buf(), tmp.path().join("library")).unwrap();
        let adapter = GitAdapter::for_directory(&dir_config, &paths).unwrap();
        assert!(!adapter.available("ignored").unwrap());
    }

    #[test]
    fn git_adapter_list_installed_empty_when_not_cloned() {
        let tmp = tempfile::TempDir::new().unwrap();
        let dir_config = make_test_dir_config(
            std::path::PathBuf::from("https://example.com/never-cloned.git"),
            None,
        );
        let paths = TomePaths::new(tmp.path().to_path_buf(), tmp.path().join("library")).unwrap();
        let adapter = GitAdapter::for_directory(&dir_config, &paths).unwrap();
        assert!(adapter.list_installed().unwrap().is_empty());
    }

    #[test]
    fn git_adapter_for_directory_extracts_url_and_ref() {
        let tmp = tempfile::TempDir::new().unwrap();
        let dir_config = make_test_dir_config(
            std::path::PathBuf::from("https://example.com/repo.git"),
            Some(GitRef::Branch("main".into())),
        );
        let paths = TomePaths::new(tmp.path().to_path_buf(), tmp.path().join("library")).unwrap();
        let adapter = GitAdapter::for_directory(&dir_config, &paths).unwrap();
        assert_eq!(adapter.url, "https://example.com/repo.git");
        assert_eq!(
            adapter.git_ref.as_ref().and_then(|r| r.branch()),
            Some("main")
        );
    }

    #[test]
    fn git_adapter_install_invokes_clone_repo() {
        // D-05a regression anchor: install delegates verbatim to git::clone_repo.
        let tmp = tempfile::TempDir::new().unwrap();
        let origin = make_local_test_repo(tmp.path());
        let dir_config = make_test_dir_config(origin.clone(), None);
        let paths = TomePaths::new(tmp.path().to_path_buf(), tmp.path().join("library")).unwrap();
        let adapter = GitAdapter::for_directory(&dir_config, &paths).unwrap();
        adapter.install("ignored").unwrap();
        assert!(
            adapter.cache_dir.join(".git").is_dir(),
            "expected .git in cache dir after install, cache_dir={}",
            adapter.cache_dir.display()
        );
    }

    #[test]
    fn git_adapter_current_version_after_install_is_head_sha() {
        let tmp = tempfile::TempDir::new().unwrap();
        let origin = make_local_test_repo(tmp.path());
        let dir_config = make_test_dir_config(origin.clone(), None);
        let paths = TomePaths::new(tmp.path().to_path_buf(), tmp.path().join("library")).unwrap();
        let adapter = GitAdapter::for_directory(&dir_config, &paths).unwrap();
        adapter.install("ignored").unwrap();
        let v = adapter.current_version("ignored").unwrap();
        let sha = v.expect("current_version should be Some after install");
        assert_eq!(sha.len(), 40, "expected 40-char SHA, got: {sha}");
        assert!(
            sha.chars().all(|c| c.is_ascii_hexdigit()),
            "SHA must be hex: {sha}"
        );
    }

    #[test]
    fn git_adapter_list_installed_after_install_returns_one_entry() {
        let tmp = tempfile::TempDir::new().unwrap();
        let origin = make_local_test_repo(tmp.path());
        let dir_config = make_test_dir_config(origin.clone(), None);
        let paths = TomePaths::new(tmp.path().to_path_buf(), tmp.path().join("library")).unwrap();
        let adapter = GitAdapter::for_directory(&dir_config, &paths).unwrap();
        adapter.install("ignored").unwrap();
        let entries = adapter.list_installed().unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].id, adapter.url);
        assert_eq!(entries[0].version.len(), 40);
        assert!(entries[0].errors.is_empty());
        assert_eq!(entries[0].install_path, adapter.cache_dir);
    }

    #[test]
    fn git_adapter_available_returns_true_after_install() {
        let tmp = tempfile::TempDir::new().unwrap();
        let origin = make_local_test_repo(tmp.path());
        let dir_config = make_test_dir_config(origin.clone(), None);
        let paths = TomePaths::new(tmp.path().to_path_buf(), tmp.path().join("library")).unwrap();
        let adapter = GitAdapter::for_directory(&dir_config, &paths).unwrap();
        adapter.install("ignored").unwrap();
        assert!(adapter.available("ignored").unwrap());
    }

    // ---- parse_claude_plugin_list_json tests (no subprocess) ----
    //
    // Hand-rolled JSON fixtures verified live 2026-05-05 against claude
    // 2.1.128. Pure parser tests give CI coverage of the JSON shape without
    // requiring `claude` on PATH.

    #[test]
    fn parse_claude_plugin_list_json_empty_array() {
        let result = parse_claude_plugin_list_json("[]").unwrap();
        assert!(result.is_empty());
    }

    #[test]
    fn parse_claude_plugin_list_json_single_entry_no_errors() {
        let json = r#"[
            {
                "id": "axiom@axiom-marketplace",
                "version": "3.3.0",
                "scope": "user",
                "enabled": true,
                "installPath": "/Users/x/.claude/plugins/cache/axiom-marketplace/axiom/3.3.0",
                "installedAt": "2026-03-17T12:18:08.229Z",
                "lastUpdated": "2026-05-04T11:49:50.948Z"
            }
        ]"#;
        let result = parse_claude_plugin_list_json(json).unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].id, "axiom@axiom-marketplace");
        assert_eq!(result[0].version, "3.3.0");
        assert_eq!(
            result[0].install_path,
            std::path::PathBuf::from(
                "/Users/x/.claude/plugins/cache/axiom-marketplace/axiom/3.3.0"
            )
        );
        assert!(
            result[0].errors.is_empty(),
            "errors field should default to empty when absent"
        );
    }

    #[test]
    fn parse_claude_plugin_list_json_entry_with_errors() {
        let json = r#"[
            {
                "id": "claude-md-management@claude-plugins-official",
                "version": "1.0.0",
                "installPath": "/path",
                "errors": ["Plugin claude-md-management not found in marketplace claude-plugins-official"]
            }
        ]"#;
        let result = parse_claude_plugin_list_json(json).unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].errors.len(), 1);
        assert!(result[0].errors[0].contains("not found in marketplace"));
    }

    #[test]
    fn parse_claude_plugin_list_json_version_unknown_string() {
        // Per RESEARCH "claude CLI JSON Shape": `version` is sometimes the
        // literal string "unknown". Adapter must accept it without parsing
        // as semver — the type is `String`, not a semver newtype.
        let json = r#"[{"id": "x@y", "version": "unknown", "installPath": "/p"}]"#;
        let result = parse_claude_plugin_list_json(json).unwrap();
        assert_eq!(result[0].version, "unknown");
    }

    #[test]
    fn parse_claude_plugin_list_json_extra_fields_ignored() {
        // `mcpServers` is a real field in the live snapshot we don't consume.
        // The parser uses the default serde behavior (drop unknown keys), NOT
        // `#[serde(deny_unknown_fields)]`, so future claude versions can add
        // fields without breaking parsing.
        let json = r#"[{"id": "x@y", "version": "1.0", "installPath": "/p", "mcpServers": [{"name": "x"}]}]"#;
        let result = parse_claude_plugin_list_json(json).unwrap();
        assert_eq!(result.len(), 1);
    }

    #[test]
    fn parse_claude_plugin_list_json_malformed_returns_err() {
        let result = parse_claude_plugin_list_json("not json");
        assert!(result.is_err());
        let msg = format!("{:#}", result.unwrap_err());
        assert!(
            msg.contains("claude plugin list"),
            "context should mention the command: {msg}"
        );
    }

    // ---- classify_claude_install_stderr tests ----

    #[test]
    fn classify_stderr_not_found_in_marketplace_is_not_found() {
        let stderr =
            r#"✘ Failed to install plugin "x@y": Plugin "x" not found in marketplace "y"."#;
        assert_eq!(
            classify_claude_install_stderr(stderr),
            InstallFailureKind::NotFound
        );
    }

    #[test]
    fn classify_stderr_not_found_bare_is_not_found() {
        let stderr = r#"✘ Failed to update plugin "x": Plugin "x" not found"#;
        assert_eq!(
            classify_claude_install_stderr(stderr),
            InstallFailureKind::NotFound
        );
    }

    #[test]
    fn classify_stderr_unrecognized_is_unknown() {
        let stderr = "some entirely novel error from a future claude version";
        assert_eq!(
            classify_claude_install_stderr(stderr),
            InstallFailureKind::Unknown
        );
    }

    #[test]
    fn classify_stderr_empty_is_unknown() {
        assert_eq!(
            classify_claude_install_stderr(""),
            InstallFailureKind::Unknown
        );
    }

    // ---- ClaudeMarketplaceAdapter pure unit tests (no subprocess) ----
    //
    // Use `new_for_test()` to bypass the binary probe; tests pre-populate
    // the cache directly so they exercise trait method shape without
    // requiring `claude` on PATH.

    fn make_test_plugin(id: &str, version: &str, errors: Vec<String>) -> InstalledPlugin {
        InstalledPlugin {
            id: id.to_string(),
            version: version.to_string(),
            install_path: PathBuf::from("/test"),
            errors,
        }
    }

    #[test]
    fn claude_adapter_id_is_stable_constant() {
        let adapter = ClaudeMarketplaceAdapter::new_for_test();
        assert_eq!(adapter.id(), "claude-plugins");
    }

    #[test]
    fn claude_adapter_available_returns_false_for_errored_entry() {
        let adapter = ClaudeMarketplaceAdapter::new_for_test();
        // Pre-populate the cache directly with an errored entry — bypasses
        // the subprocess call so the test runs deterministically without
        // claude on PATH.
        *adapter.cache.borrow_mut() = Some(vec![make_test_plugin(
            "vanished@m1",
            "1.0.0",
            vec!["Plugin vanished not found in marketplace m1".into()],
        )]);
        assert!(!adapter.available("vanished@m1").unwrap());
    }

    #[test]
    fn claude_adapter_available_returns_true_for_clean_entry() {
        let adapter = ClaudeMarketplaceAdapter::new_for_test();
        *adapter.cache.borrow_mut() = Some(vec![make_test_plugin("happy@m1", "1.0.0", vec![])]);
        assert!(adapter.available("happy@m1").unwrap());
    }

    #[test]
    fn claude_adapter_available_returns_true_for_entry_not_in_cache() {
        // Conservative default: if the plugin isn't in the snapshot at all,
        // it's "available" — only `errors[]` containing the substring marks
        // unavailable. Per D-02.
        let adapter = ClaudeMarketplaceAdapter::new_for_test();
        *adapter.cache.borrow_mut() = Some(vec![]);
        assert!(adapter.available("never-seen").unwrap());
    }

    #[test]
    fn claude_adapter_current_version_returns_some_for_known_plugin() {
        let adapter = ClaudeMarketplaceAdapter::new_for_test();
        *adapter.cache.borrow_mut() = Some(vec![make_test_plugin("axiom@m1", "3.3.0", vec![])]);
        assert_eq!(
            adapter.current_version("axiom@m1").unwrap(),
            Some("3.3.0".to_string())
        );
    }

    #[test]
    fn claude_adapter_current_version_returns_none_for_unknown_plugin() {
        let adapter = ClaudeMarketplaceAdapter::new_for_test();
        *adapter.cache.borrow_mut() = Some(vec![]);
        assert_eq!(adapter.current_version("never-seen").unwrap(), None);
    }

    #[test]
    fn claude_adapter_build_install_failure_uses_heuristic_for_not_found() {
        let f = ClaudeMarketplaceAdapter::build_install_failure(
            "claude-plugins",
            "x@y",
            InstallOp::Install,
            r#"Plugin "x" not found in marketplace "y""#,
        );
        assert_eq!(f.kind, InstallFailureKind::NotFound);
        assert_eq!(f.adapter_id, "claude-plugins");
        assert_eq!(f.plugin_id, "x@y");
        assert_eq!(f.operation, InstallOp::Install);
    }

    #[test]
    fn claude_adapter_build_install_failure_unknown_for_novel_stderr() {
        let f = ClaudeMarketplaceAdapter::build_install_failure(
            "claude-plugins",
            "x@y",
            InstallOp::Update,
            "some entirely novel error",
        );
        assert_eq!(f.kind, InstallFailureKind::Unknown);
        assert_eq!(f.operation, InstallOp::Update);
    }

    // ---- Smoke tests (gated behind is_claude_available) ----
    //
    // Per RESEARCH "Test Strategy for Shelled Code" recommendation #3: smoke
    // tests run against real `claude` when available, eprintln+return when
    // not. CI without claude exits cleanly; dev machines exercise the real
    // subprocess path.

    #[test]
    fn smoke_claude_available_or_skip() {
        if !is_claude_available() {
            eprintln!("SKIP smoke_claude_available_or_skip: claude CLI not on PATH");
            return;
        }
        // If we're here, claude is on PATH — construction must succeed.
        assert!(ClaudeMarketplaceAdapter::new().is_ok());
    }

    #[test]
    fn smoke_claude_marketplace_adapter_lists_installed() {
        if !is_claude_available() {
            eprintln!(
                "SKIP smoke_claude_marketplace_adapter_lists_installed: claude CLI not on PATH"
            );
            return;
        }
        let adapter = ClaudeMarketplaceAdapter::new().unwrap();
        let list = adapter
            .list_installed()
            .expect("list_installed should succeed when claude is on PATH");
        // Don't assert on the count — only that the call shape works. On
        // Martin's machine this is ~37; on a fresh machine it may be 0.
        let _ = list.len();
    }

    #[test]
    fn smoke_claude_install_nonexistent_returns_err() {
        if !is_claude_available() {
            eprintln!("SKIP smoke_claude_install_nonexistent_returns_err: claude CLI not on PATH");
            return;
        }
        let adapter = ClaudeMarketplaceAdapter::new().unwrap();
        let result = adapter.install("definitely-nonexistent-xyz@nonexistent-marketplace-xyz");
        assert!(result.is_err(), "install of nonexistent plugin should fail");
    }

    #[test]
    fn testing_module_visible_under_test_cfg() {
        // Proves `crate::marketplace::testing::*` resolves when cfg(test)
        // is on. Plan 13-05's integration tests rely on the same path
        // resolving when feature = "test-support" is on.
        let adapter = MockMarketplaceAdapter {
            id: "visibility-probe".to_string(),
            installed: vec![],
            available: HashSet::new(),
            fail_install: HashSet::new(),
            fail_update: HashSet::new(),
        };
        let dyn_ref: &dyn MarketplaceAdapter = &adapter;
        assert_eq!(dyn_ref.id(), "visibility-probe");
    }
}
