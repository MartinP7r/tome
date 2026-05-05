//! Marketplace adapter trait and shared types.
//!
//! This module defines the [`MarketplaceAdapter`] trait that pluggable
//! marketplace implementations (Claude CLI, git, future: npm/etc.) must
//! satisfy, plus the [`InstalledPlugin`] data type they return.
//!
//! Phase 12 ships the contract + adapter implementations; Phase 13 wires the
//! dispatch into `lib.rs::sync`. All trait methods return [`anyhow::Result`]
//! per the project-wide error-handling convention.

use std::path::PathBuf;

use anyhow::Result;

/// A plugin currently installed via a marketplace adapter.
///
/// Adapters return `Vec<InstalledPlugin>` from [`MarketplaceAdapter::list_installed`].
/// This type is distinct from `manifest::SkillEntry` — `SkillEntry` describes
/// what's in the library, while `InstalledPlugin` describes what's installed at
/// the marketplace level. Phase 13's reconciliation flow bridges the two.
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
