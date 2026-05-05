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
//
// dead_code allow: Phase 12 only ships the contract + a `#[cfg(test)]` mock.
// Real consumers (`ClaudeMarketplaceAdapter`, `GitAdapter`) arrive in Plans
// 12-03 and 12-04; Phase 13's sync flow wires the dispatch. Drop this attr
// when the first non-test caller lands.
#[allow(dead_code)]
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
// dead_code allow: see InstalledPlugin above. Drop when Plan 12-03 / 12-04
// add real `impl MarketplaceAdapter for ...` blocks consumed from non-test
// code, or when Phase 13's sync dispatcher lands.
#[allow(dead_code)]
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;
    use std::path::PathBuf;

    /// In-memory test double for [`MarketplaceAdapter`].
    ///
    /// Combines static fixtures (`installed`, `available`) with failure
    /// injection (`fail_install`, `fail_update`) so a single mock instance
    /// can drive both happy-path and partial-failure tests.
    ///
    /// `pub(super)` so nested test fns in this same module can construct it
    /// freely. Per D-10 the mock stays `#[cfg(test)]`-only for Phase 12;
    /// Phase 13 may lift it to `pub(crate) marketplace::testing` for
    /// integration-test reuse.
    pub(super) struct MockMarketplaceAdapter {
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
    fn fixture_plugin(id: &str, version: &str) -> InstalledPlugin {
        InstalledPlugin {
            id: id.to_string(),
            version: version.to_string(),
            install_path: PathBuf::from(format!("/tmp/mock/{id}")),
            errors: Vec::new(),
        }
    }

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
}
