# Phase 12: Marketplace adapter - Context

**Gathered:** 2026-05-05
**Status:** Ready for planning

<domain>
## Phase Boundary

A pluggable `MarketplaceAdapter` trait isolates marketplace-specific install/update logic. v0.10 ships **two production adapters** plus partial-failure aggregation matching the v0.8 SAFE-01 pattern.

**In scope:**

- The `MarketplaceAdapter` trait (`crates/tome/src/marketplace.rs`) with the surface locked by ROADMAP SC1: `id()`, `current_version()`, `install()`, `update()`, `list_installed()`, `available()` — all returning `anyhow::Result`.
- `ClaudeMarketplaceAdapter` — shells out to `claude plugin install/update/list` (ROADMAP SC2 / ADP-02).
- `GitAdapter` — wraps existing `crates/tome/src/git.rs` (ROADMAP SC3 / ADP-03). Behavior byte-for-byte unchanged for existing git directories.
- `InstallFailure` struct + `InstallFailureKind` enum + compile-time `ALL` array (POLISH-04 pattern from Phase 10).
- A `MockMarketplaceAdapter` test double that exercises the trait shape in unit tests.

**Out of scope** (handled by other phases):

- Drift detection / Match/Drift/Vanished classification → Phase 13 (RECON-01..05).
- Install-consent prompt + `auto_install_plugins` machine.toml flag → Phase 13 (RECON-02).
- Per-skill diff rendering during sync → Phase 13.
- Edit-in-library detection (managed + content_hash mismatch) → Phase 13 (RECON-05).
- Wiring the adapter into `lib.rs::sync` invocation flow → Phase 13. **Phase 12 ships the trait + adapters + tests; no `tome sync` call-site changes.** This keeps Phase 12 focused on contract + adapter correctness and lets Phase 13 own the integration surface entirely.

</domain>

<decisions>
## Implementation Decisions

### ClaudeMarketplaceAdapter subprocess policy (ADP-02)

- **D-01 (subprocess invocation):** Run all `claude plugin install/update/list` subprocesses with `stdin = /dev/null`. Env unchanged (no `CI=true`/`NO_COLOR=1` — empirically not needed; see `<empirical_findings>`). Capture stderr verbatim into `InstallFailure::source`. Surface non-zero exit as failure. **No probing or feature-detection** — empirical evidence (probes against `claude 2.1.126`) shows install/update/list/failure-paths all exit promptly and non-interactively when stdin is closed.
- **D-02 (`available(plugin_id)` signal source):** `available()` parses the cached `claude plugin list --json` snapshot's `errors` field for the entry. If the entry's `errors[]` contains a "not found in marketplace" string, returns `Ok(false)`; otherwise `Ok(true)`. **Zero extra subprocess calls.** This makes Phase 13's RECON-04 vanished-plugin detection essentially free — it falls out of the same snapshot already used for `list_installed()` and `current_version()`.
- **D-03 (no timeout in v0.10):** Subprocess calls are not wrapped in a timeout. Probes show normal/failure paths exit promptly. Defer timeout knob to a future hardening phase if a real-world hang surfaces. Adds zero code, zero config surface.

### Snapshot cache (ADP-01 implementation detail)

- **D-04 (internal cache + auto-invalidate + public refresh):** `ClaudeMarketplaceAdapter` holds an internal `RefCell<Option<Vec<InstalledPlugin>>>` (or equivalent — `Cell` + `Option<Arc<...>>` if `RefCell` clashes with Send/Sync requirements; planner picks). First call to any read method (`list_installed`, `current_version`, `available`) populates the cache. Each `install()` / `update()` returning `Ok` automatically invalidates the cache (next read re-runs `claude plugin list --json`). A public `refresh(&self) -> Result<()>` method allows explicit re-query for callers that want forced freshness. **Result:** one subprocess call per `tome sync` (unless install/update happen mid-flow); fast; no caller-side bookkeeping in Phase 13.

### GitAdapter shape (ADP-03)

- **D-05 (one adapter per git directory):** Each `[directories.<name>]` config entry with `type = "git"` instantiates its own `GitAdapter` bound to that single URL. Adapter state holds: URL, repos cache dir, optional ref pin (branch/tag/rev). Mapping:
  - `id()` → the git URL string.
  - `list_installed()` → `vec![one_entry]` describing the local clone (id = URL, version = HEAD SHA, install path = `repos_dir/<sha256(url)>/`).
  - `current_version()` → `Ok(Some(head_sha))` from `git.rs::read_head_sha()`. `Ok(None)` if not yet cloned.
  - `available()` → `Ok(true)` if local clone exists OR a cheap `git ls-remote --exit-code <url> HEAD` succeeds (planner decides whether to actually probe network or just trust local-clone existence — the cheaper option is preferred since git directories don't have a "vanished" lifecycle the way marketplace plugins do).
  - `install()` → delegates to `git::clone_repo` (existing helper, unchanged).
  - `update()` → delegates to `git::update_repo` (existing helper, unchanged).
- **D-05a (regression contract):** Existing git-source integration tests in `crates/tome/tests/cli.rs` continue to pass byte-for-byte. The trait wrap is a thin shim over `git.rs`; no behavior change for existing git directories.

### InstallFailure shape (ADP-04)

- **D-06 (marketplace-specific struct):** New types in `marketplace.rs`:
  ```rust
  pub struct InstallFailure {
      pub adapter_id: String,      // e.g. "claude-plugins-official", or git URL
      pub plugin_id: String,       // e.g. "axiom@axiom-marketplace", or skill name for git
      pub operation: InstallOp,    // Install | Update
      pub kind: InstallFailureKind,
      pub source: anyhow::Error,   // verbatim stderr / underlying error
  }

  pub enum InstallOp { Install, Update }

  pub enum InstallFailureKind {
      NotFound,           // plugin/url not in marketplace / not reachable
      NetworkError,       // transient — retry might succeed
      PermissionDenied,   // filesystem or auth issue
      Unknown,            // catch-all; source carries the detail
  }

  impl InstallFailureKind {
      pub const ALL: &'static [InstallFailureKind] = &[ ... ];  // POLISH-04 exhaustiveness
  }
  ```
  Compile-time `ALL` array sentinel (POLISH-04 from Phase 10) pins exhaustiveness. Mirrors `crates/tome/src/remove.rs::FailureKind` pattern but with marketplace-meaningful fields (no `path` — install-time failures don't have a stable filesystem path).
- **D-07 (grouping summary):** `Vec<InstallFailure>` aggregates across all adapter calls during a sync. Renders as `⚠ N install operations failed` header + per-kind grouped lines (mirrors SAFE-01 visual layout from Phase 8, see `crates/tome/src/lib.rs` for the existing `RemoveFailure` rendering helper). Phase 12 ships the rendering function as a free function in `marketplace.rs` (or `lib.rs`, planner picks); Phase 13 will call it from the sync flow.

### Trait surface (ADP-01)

- **D-08 (signature shape):** Method signatures, locked from ROADMAP SC1 + the cache decision (D-04):
  ```rust
  pub trait MarketplaceAdapter {
      fn id(&self) -> &str;
      fn current_version(&self, plugin_id: &str) -> Result<Option<String>>;
      fn install(&self, plugin_id: &str) -> Result<()>;
      fn update(&self, plugin_id: &str) -> Result<()>;
      fn list_installed(&self) -> Result<Vec<InstalledPlugin>>;
      fn available(&self, plugin_id: &str) -> Result<bool>;
  }
  ```
  `plugin_id` is `&str` (not `SkillName` / `DirectoryName`) because marketplace ids carry a `@marketplace` suffix that doesn't fit `SkillName`'s validation (e.g. `axiom@axiom-marketplace`). Adapter implementations validate as appropriate.

  `InstalledPlugin` struct (defined in `marketplace.rs`):
  ```rust
  pub struct InstalledPlugin {
      pub id: String,             // e.g. "axiom@axiom-marketplace" or git URL
      pub version: String,        // semver string for claude, HEAD SHA for git
      pub install_path: PathBuf,  // claude: cache path; git: repos cache path
      pub errors: Vec<String>,    // claude: from `errors[]` JSON field; git: usually empty
  }
  ```

- **D-09 (install scope hardcoded):** `ClaudeMarketplaceAdapter::install` invokes `claude plugin install <plugin>@<marketplace>` with the **default scope (user)** — no explicit `--scope` flag passed. Rationale: matches what users do manually; tome's manifest tracks provenance separately so the scope doesn't need to carry that signal. If a future requirement needs project-scope or local-scope, add an `install_with_scope()` method then.

### Mock adapter (ADP-01 test surface)

- **D-10 (mock location):** `MockMarketplaceAdapter` lives in `crates/tome/src/marketplace.rs` under `#[cfg(test)]` for unit tests. If Phase 13 needs the same mock from integration tests in `crates/tome/tests/cli.rs`, lift it to a `pub(crate)` `marketplace::testing` module then. Phase 12 keeps it `#[cfg(test)]`-scoped.

### Adapter selection (no new config surface)

- **D-11 (selection by DirectoryType):** Each `DirectoryType` maps 1:1 to an adapter constructor. `DirectoryType::Git` → `GitAdapter::for_directory(&dir_config)`. `DirectoryType::ClaudePlugins` → `ClaudeMarketplaceAdapter::new()` (singleton — there's only one `claude` binary). `DirectoryType::Directory` (local) → no adapter (skill discovery only). **No new field in `tome.toml`.** Phase 13 will define the dispatch helper that takes a `DirectoryConfig` and returns `Box<dyn MarketplaceAdapter>` (or `None` for local). Phase 12 ships the trait + adapters; Phase 13 ships the dispatcher.

### Carried forward from prior phases (locked, do not re-decide)

- **D-LIB-03** (PROJECT.md): Adapter trait, not direct shell-out — ✓ honored by D-08.
- **D-08** (Phase 11): Drift basis is `content_hash`, not `version`. `current_version()` is **display-only** for human-readable diffs (Phase 13). Never consulted as the drift signal. Adapter implementation does not need to be fast or precise on `current_version()` parsing — wrong/stale version strings degrade UX but do not cause incorrect drift behavior.
- **SAFE-01** (Phase 8): grouped failure summary uses an exhaustive enum + `ALL` array. → D-06, D-07.
- **POLISH-04** (Phase 10): `FailureKind::ALL` compile-time exhaustiveness via const-assert pattern. → D-06.

### Claude's Discretion

- Exact rendering text of `⚠ N install operations failed` summary (within SAFE-01 visual conventions).
- Exact mapping from claude stderr strings to `InstallFailureKind` variants (parser is a heuristic; default to `Unknown`; planner can refine over time).
- Internal organization of `RefCell<Option<Vec<InstalledPlugin>>>` cache vs `OnceCell` vs `Mutex<Option<...>>` — pick what compiles cleanly with Send/Sync requirements imposed by future Phase 13 sync flow.
- Whether `GitAdapter::available()` actually probes network or just checks local-clone existence (recommendation: trust local-clone existence; git URLs don't "vanish" in practice).
- How `ClaudeMarketplaceAdapter` detects missing `claude` binary (recommendation: `which::which("claude")` or `std::process::Command::new("claude").arg("--version").output()` at adapter construction; surface as clear error per ADP-02).
- Whether `MockMarketplaceAdapter` exposes constructor knobs for failure injection or just static fixtures (planner decides based on what unit tests need).
- Whether the `InstallFailure` rendering helper lives in `marketplace.rs` or `lib.rs`.

### Folded Todos

(None — `gsd-tools.cjs todo match-phase 12` returned no matches.)

</decisions>

<empirical_findings>
## Empirical findings (from Phase 12 discussion probes)

These probes ran against `claude 2.1.126 (Claude Code)` on macOS during the discuss-phase session (2026-05-04). Recorded here so the planner doesn't have to re-probe.

**Subprocess non-interactivity (Probe 1):**
- `claude plugin install <plugin>@<marketplace> </dev/null` → exits 0 promptly when already installed (`✔ Plugin "axiom@axiom-marketplace" is already installed (scope: user)`).
- `claude plugin update <plugin> </dev/null` → exits 1 promptly with stderr (`✘ Failed to update plugin "axiom": Plugin "axiom" not found`). May need `<plugin>@<marketplace>` qualifier — planner verifies the exact id format.
- `claude plugin install <nonexistent>@<nonexistent> </dev/null` → exits 1 promptly with stderr (`✘ Failed to install plugin ...: Plugin ... not found in marketplace ...`).
- **Conclusion:** stdin closed is sufficient for non-interactive guarantee. No env vars needed.

**Marketplace `--scope` flag (Probe 2):**
- `claude plugin install --scope managed ...` → rejected with `Invalid scope: managed. Must be one of: user, project, local.`
- `claude plugin update --scope managed` → accepted (per help text), but rationale absent.
- **Conclusion:** Stick with default `user` scope per D-09. `--scope managed` is not viable for install.

**Vanished-plugin signal (Probe 3):**
- `claude plugin list --json` returns a flat array of `{ id, version, scope, enabled, installPath, installedAt, lastUpdated, errors? }`.
- The `errors` field, when present, contains strings like `"Plugin claude-md-management not found in marketplace claude-plugins-official"`.
- `claude plugin list --available --json` returns a different shape: `{ "installed": [...], "available": [...] }` (object, not array).
- **Conclusion:** `available(plugin_id)` parses the `errors[]` field of the cached `list --json` snapshot — zero extra calls. Phase 13's RECON-04 vanished detection comes for free. The `--available` form is not needed for Phase 12 or 13.

</empirical_findings>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### v0.10 design

- `.planning/research/v0.10-library-canonical-design.md` — full v0.10 design (468 lines, 9 OQs resolved). Section on adapter trait + marketplace abstraction is the architectural baseline.
- `.planning/PROJECT.md` §"Current Milestone: v0.10" — adapter rationale + decision D-LIB-03.

### Phase 11 (immediate predecessor — manifest/lockfile contract)

- `.planning/phases/11-library-canonical-core/11-CONTEXT.md` — D-08 (drift basis = content_hash; version is display-only) directly constrains how `current_version()` is used downstream.
- `.planning/phases/11-library-canonical-core/11-VERIFICATION.md` — confirms the manifest schema Phase 12 builds on (`SkillEntry.source_name: Option<DirectoryName>`, `LockEntry.source_name: Option<DirectoryName>`).

### SAFE-01 / POLISH-04 patterns to mirror

- `crates/tome/src/remove.rs` — `FailureKind`, `RemoveFailure`, the `ALL` const array, the const-assert sentinel. **The reference implementation** for D-06 / D-07.
- `crates/tome/src/lib.rs` — existing `RemoveFailure` rendering helper (search for "install operations failed" / "remove" failure summary). The rendering layout to mirror.
- `.planning/phases/08-safety-refactors-partial-failure-visibility-cross-platform/08-CONTEXT.md` — SAFE-01 design rationale.
- `.planning/phases/10-phase-8-review-tail/10-CONTEXT.md` — POLISH-04 (`ALL` array compile-time exhaustiveness).

### Existing code to wrap or call

- `crates/tome/src/git.rs` — `clone_repo`, `update_repo`, `read_head_sha`, `repo_cache_dir`, `ref_spec_for_config`. **GitAdapter wraps this verbatim.**
- `crates/tome/src/install.rs` — existing `installed_plugins.json` parser. **NOT directly used by ClaudeMarketplaceAdapter** (we use `claude plugin list --json` instead, which is canonical), but install.rs is the reference for the prior approach and may share parsing helpers.
- `crates/tome/src/config.rs` — `DirectoryType`, `DirectoryConfig`, `DirectoryName`. Phase 12 references these for adapter dispatch (D-11).

### Requirements

- `.planning/REQUIREMENTS.md` §"Marketplace adapter (ADP)" — ADP-01..04 verbatim. Cross-check the planner's traceability against these.
- `.planning/ROADMAP.md` §"Phase 12: Marketplace adapter" — success criteria 1-4 are the verification anchors.

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable assets

- **`crates/tome/src/git.rs`** — All git operations already abstracted into `clone_repo`, `update_repo`, `read_head_sha`, `repo_cache_dir`. GitAdapter is a thin trait shim over these. Existing tests cover the underlying behavior; GitAdapter just needs to verify the wrap doesn't change semantics.
- **`crates/tome/src/remove.rs::FailureKind` pattern** — Exact template for `InstallFailureKind` (POLISH-04 `ALL` array, const-assert sentinel, exhaustiveness drift guard).
- **`crates/tome/src/install.rs`** — Existing parser for `installed_plugins.json`. **Not used by ClaudeMarketplaceAdapter directly**, but shares JSON-parsing patterns. Worth reading for prior-art context.
- **`crates/tome/src/config.rs::DirectoryType`** — Phase 11's `Git`, `ClaudePlugins`, `Directory` enum drives adapter dispatch in D-11.

### Established patterns

- **`anyhow::Result` everywhere** — No custom error types unless needed. `.context()` chains for diagnostic depth.
- **`pub(crate)` for internal helpers, `pub` for the trait surface** — Mirrors existing module conventions (e.g., `git.rs` keeps helpers crate-private).
- **Subprocess invocation: `std::process::Command` directly, no async** — `git.rs` shells out synchronously; Phase 12 mirrors this for `claude` calls. No tokio dependency added.
- **`#[cfg(test)] mod tests`** — All Phase 12 unit tests co-located in `marketplace.rs`. Integration tests (if needed) go in `crates/tome/tests/cli.rs` or a new `crates/tome/tests/marketplace.rs`.

### Integration points

- **`crates/tome/src/lib.rs`** — Will need a `pub(crate) mod marketplace;` declaration. **No `sync()` call-site changes in Phase 12.** Phase 13 wires the dispatch.
- **`crates/tome/src/manifest.rs`** — `InstalledPlugin` struct (D-08) is a Phase 12 type, separate from `SkillEntry`. They're different concepts: `SkillEntry` is what's in the library; `InstalledPlugin` is what's installed at the marketplace level. Phase 13 reconciles between them.
- **`crates/tome/src/cli.rs`** — No new CLI commands in Phase 12. Phase 14 adds `tome adopt`/`forget`; Phase 13 wires sync drift output. Phase 12 is library-only.

### Constraints from existing architecture

- **Unix-only** (per project policy) — adapter assumes POSIX `std::os::unix::fs` semantics where relevant. No cross-platform code in Phase 12.
- **No tokio / async** — All adapter ops are synchronous subprocess calls. Match the rest of the crate.
- **Edition 2024 / strict clippy** — `#[deny(warnings)]` in CI. Plan for `#[cfg(test)] use ...` patterns and `#[allow(dead_code)]` only with justification.

</code_context>

<specifics>
## Specific Ideas

- **`current_version()` returns `Result<Option<String>>`** — `None` when the plugin isn't installed locally. Caller in Phase 13 maps this to "Vanished" semantics in coordination with `available()`.
- **`InstalledPlugin.errors` is the upstream signal carrier** — preserve it verbatim; don't filter or pretty-print at the adapter layer. Phase 13's drift-summary rendering can decide how to display.
- **`update()` exact id format requires verification** — Probe 1b showed `claude plugin update axiom` failed with "Plugin axiom not found". The planner should verify whether `update` needs the `@marketplace` qualifier or accepts the bare id, and document the exact contract in code.

</specifics>

<deferred>
## Deferred Ideas

- **Subprocess timeout knob** (e.g., `[install] timeout_seconds = 60` in `tome.toml`) — deferred per D-03. Add only if a real-world hang surfaces. Future hardening phase territory.
- **`--scope project` / `--scope local` support** for `ClaudeMarketplaceAdapter::install` — deferred per D-09. Add when a concrete user need surfaces.
- **`claude plugin list --available --json` catalog query** — not needed for Phase 12 or 13 (the `errors[]` field of `list --json` carries the vanished signal). Re-evaluate if a future feature needs the full marketplace catalog.
- **`MarketplaceAdapter` async variant** — current trait is fully synchronous. If a future phase needs concurrent install/update across many plugins, consider async at that point. v0.10 ships sync-only.
- **Adapter for non-Claude marketplaces** (npm, pip, OS package managers) — out of v0.10 scope. Trait is designed to accommodate but no concrete adapter ships beyond Claude + git.
- **`MockMarketplaceAdapter` integration-test surface** — D-10 keeps it `#[cfg(test)]`-only for Phase 12. Phase 13 may need to lift it to `pub(crate)` for integration test reuse; that's a tactical Phase 13 decision.

### Reviewed Todos (not folded)

(None — `gsd-tools.cjs todo match-phase 12` returned no matches.)

</deferred>

---

*Phase: 12-marketplace-adapter*
*Context gathered: 2026-05-05*
