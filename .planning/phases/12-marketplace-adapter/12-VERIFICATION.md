---
phase: 12-marketplace-adapter
verified: 2026-05-05T03:09:24Z
status: passed
score: 24/24 must-haves verified
---

# Phase 12: Marketplace Adapter Verification Report

**Phase Goal:** Ship the `MarketplaceAdapter` trait + two production adapters (`ClaudeMarketplaceAdapter` + `GitAdapter`) + `InstallFailure` aggregation/renderer per ADP-01..04. NO `sync()` call-site changes — Phase 13 owns the integration.

**Verified:** 2026-05-05T03:09:24Z
**Status:** passed
**Re-verification:** No — initial verification

## Goal Achievement

### Observable Truths

#### Plan 12-01 (ADP-01) — MarketplaceAdapter trait

| # | Truth | Status | Evidence |
| --- | --- | --- | --- |
| 1 | `marketplace.rs` module declared in `lib.rs` | VERIFIED | `crates/tome/src/lib.rs:42` — `pub(crate) mod marketplace;` (alphabetically correct between `manifest` and `migration_v010`; the plan said "between library and lint" but `lint` is alphabetically before `marketplace` — the actual placement is alphabetical-correct) |
| 2 | Six locked trait method signatures | VERIFIED | `marketplace.rs:86,91,96,101,106,112` — exact verbatim signatures (id, current_version, install, update, list_installed, available) |
| 3 | `InstalledPlugin` with four locked fields | VERIFIED | `marketplace.rs:35-65` — id/version/install_path/errors, derives Debug+Clone |
| 4 | `MockMarketplaceAdapter` under `#[cfg(test)]` | VERIFIED | `marketplace.rs:777-819` — `pub(super)` mock with full trait impl |
| 5 | Trait-shape tests demonstrate `dyn` polymorphism | VERIFIED | `marketplace.rs:857,881,894,924` — `&dyn MarketplaceAdapter` and `Box<dyn MarketplaceAdapter>` exercised |

#### Plan 12-02 (ADP-04) — InstallFailure + renderer

| # | Truth | Status | Evidence |
| --- | --- | --- | --- |
| 6 | `InstallFailure` with five locked fields | VERIFIED | `marketplace.rs:241-260` — adapter_id/plugin_id/operation/kind/source, derives Debug only |
| 7 | `InstallOp { Install, Update }` | VERIFIED | `marketplace.rs:127-130` — Debug+Clone+Copy+PartialEq+Eq |
| 8 | `InstallFailureKind` four variants | VERIFIED | `marketplace.rs:151-160` — NotFound/NetworkError/PermissionDenied/Unknown |
| 9 | `ALL` is fixed-size `[_; 4]` array | VERIFIED | `marketplace.rs:169` — `pub const ALL: [InstallFailureKind; 4]` |
| 10 | Compile-time exhaustiveness sentinel | VERIFIED | `marketplace.rs:204-218` — `_ensure_install_failure_kind_all_exhaustive` const fn + `const _: () = { assert!(...len() == 4) }` |
| 11 | `render_install_failures` helper | VERIFIED | `marketplace.rs:317` — wraps `format_install_failures` (line 278) |
| 12 | Tests cover ALL exhaustiveness, label coverage, renderer shape | VERIFIED | tests `install_failure_kind_label_coverage`, `install_failure_kind_all_pinned_size_four`, `install_failure_kind_all_length_matches_variant_count`, `install_failure_kind_all_ordering_pinned`, `format_install_failures_*`, `render_install_failures_empty_is_noop` all present and passing |

#### Plan 12-03 (ADP-03) — GitAdapter

| # | Truth | Status | Evidence |
| --- | --- | --- | --- |
| 13 | `GitAdapter` impl `MarketplaceAdapter` | VERIFIED | `marketplace.rs:340,386` — struct + impl block |
| 14 | `for_directory(&DirectoryConfig, &TomePaths)` constructor | VERIFIED | `marketplace.rs:356` — extracts URL via `to_str()` (not `to_string_lossy()`), precomputes cache_dir |
| 15 | Delegates install/update to `git::clone_repo` / `git::update_repo` verbatim | VERIFIED | `marketplace.rs:399,409,395` — direct delegation, no behavior change |
| 16 | D-05a regression contract — existing git-source integration tests pass byte-for-byte | VERIFIED | `cargo test -p tome --test cli` → 141 passed (matches pre-Phase-12 baseline) |
| 17 | `available()` trusts local-clone existence | VERIFIED | `marketplace.rs:430-437` — returns `Ok(self.cache_dir.exists())` with documented rationale |

#### Plan 12-04 (ADP-02) — ClaudeMarketplaceAdapter

| # | Truth | Status | Evidence |
| --- | --- | --- | --- |
| 18 | `new()` probes `claude --version`, errors with actionable message | VERIFIED | `marketplace.rs:613-623` — bails with binary-naming hint if missing |
| 19 | Subprocess invocation with `stdin = Stdio::null()` per D-01 | VERIFIED | `marketplace.rs:560` — `.stdin(std::process::Stdio::null())` |
| 20 | `list_installed` parses `claude plugin list --json` into `Vec<InstalledPlugin>` | VERIFIED | `marketplace.rs:478-490` (parser) + `marketplace.rs:732-735` (trait impl) |
| 21 | `available()` reads cached snapshot's `errors[]` — zero extra subprocess calls | VERIFIED | `marketplace.rs:737-758` — only calls `populate_cache()` (which is no-op when populated), then iterates cached entries |
| 22 | `RefCell<Option<Vec<InstalledPlugin>>>` cache; auto-invalidates on Ok install/update; `pub fn refresh()` | VERIFIED | `marketplace.rs:601` (cache field), `marketplace.rs:718,728` (auto-invalidate), `marketplace.rs:645-648` (refresh) |
| 23 | Heuristic stderr → InstallFailureKind mapping | VERIFIED | `marketplace.rs:513-527` — substring-based; "not found in marketplace" / "not found" → NotFound; else Unknown |
| 24 | Pure parser tests + smoke tests gated behind `is_claude_available()` | VERIFIED | 6 parser tests + 4 classifier tests + 7 pure adapter tests (use `new_for_test()`); 3 smoke tests skip cleanly when claude is missing |

**Score:** 24/24 truths verified

### Required Artifacts

| Artifact | Expected | Status | Details |
| --- | --- | --- | --- |
| `crates/tome/src/marketplace.rs` | Trait + InstalledPlugin + Mock + InstallFailure types + renderer + GitAdapter + ClaudeMarketplaceAdapter + tests | VERIFIED | 1538 lines; all symbols present (verified via grep against locked patterns) |
| `crates/tome/src/lib.rs` | `pub(crate) mod marketplace;` declaration | VERIFIED | Line 42; alphabetical placement |

### Key Link Verification

| From | To | Via | Status | Details |
| --- | --- | --- | --- | --- |
| `lib.rs` | `marketplace.rs` | module declaration | VERIFIED | `pub(crate) mod marketplace;` at lib.rs:42 |
| `MarketplaceAdapter` trait | `InstalledPlugin` | `list_installed` return | VERIFIED | `fn list_installed(&self) -> Result<Vec<InstalledPlugin>>` at line 106 |
| `InstallFailureKind` | `InstallFailureKind::ALL` | associated const | VERIFIED | `pub const ALL: [InstallFailureKind; 4]` at line 169 |
| `format_install_failures` | `InstallFailureKind::ALL` | iterates ALL for grouping | VERIFIED | `for kind in InstallFailureKind::ALL` at line 289 |
| `GitAdapter::install` | `git::clone_repo` | direct delegation | VERIFIED | `git::clone_repo(...)` at line 399 |
| `GitAdapter::update` | `git::update_repo` | direct delegation | VERIFIED | `git::update_repo(...)` at line 409 |
| `GitAdapter::current_version` | `git::read_head_sha` | direct delegation | VERIFIED | `git::read_head_sha(&self.cache_dir).map(Some)` at line 395 |
| `ClaudeMarketplaceAdapter::install` | `Command::new("claude")` | subprocess via `run_claude_subcommand` | VERIFIED | line 712: `run_claude_subcommand(&["plugin", "install", plugin_id])` (helper at 557 invokes `Command::new("claude")` with `Stdio::null()`) |
| `ClaudeMarketplaceAdapter::list_installed` | `claude plugin list --json` | subprocess + serde_json parse | VERIFIED | `populate_cache()` at line 660 invokes `["plugin", "list", "--json"]` then `parse_claude_plugin_list_json` |
| `ClaudeMarketplaceAdapter::available` | cache `errors[]` field | cached snapshot scan | VERIFIED | line 744-756 — searches cached list for `entry.errors.iter().any(|e| e.contains("not found in marketplace"))` |

### Data-Flow Trace (Level 4)

Phase 12 ships library code (trait + adapters) intended to be called by Phase 13. The data flow is from the trait surface to the underlying delegate (git or claude CLI):

| Artifact | Data Variable | Source | Produces Real Data | Status |
| --- | --- | --- | --- | --- |
| `GitAdapter::list_installed` | `version` | `git::read_head_sha(&cache_dir)` | Yes — real shell `git rev-parse HEAD` (verified by `git_adapter_current_version_after_install_is_head_sha` test asserting 40-char hex) | FLOWING |
| `ClaudeMarketplaceAdapter::list_installed` | cached `Vec<InstalledPlugin>` | `parse_claude_plugin_list_json(stdout from claude plugin list --json)` | Yes — real subprocess (verified by `smoke_claude_marketplace_adapter_lists_installed`) | FLOWING |
| `format_install_failures` rendering | per-failure lines | iterates input slice; uses `kind.label()` and `f.source` directly | Yes — verified by `format_install_failures_groups_by_kind_and_skips_empty_groups` asserting exact substrings | FLOWING |
| `MockMarketplaceAdapter` (test-only) | static fixtures | constructor-injected `Vec<InstalledPlugin>` and `HashSet<String>` | Yes — exercised by 4 mock tests | FLOWING |

NOTE: Phase 12 explicitly does NOT integrate the adapters into `sync()` (per scope boundary — Phase 13 owns the dispatch). The adapters' production callers are zero in this phase by design; the `#[allow(dead_code)]` markers on each item document this and reference Phase 13 as the future caller. This is intended scope, not a hollow-wiring failure.

### Behavioral Spot-Checks

| Behavior | Command | Result | Status |
| --- | --- | --- | --- |
| `cargo check -p tome` | (build) | Finished `dev` profile in 1.66s | PASS |
| Marketplace lib tests | `cargo test -p tome --lib marketplace::tests` | 41 passed; 0 failed (matches expected count) | PASS |
| CLI integration tests (D-05a regression contract) | `cargo test -p tome --test cli` | 141 passed; 0 failed (matches expected pre-Phase-12 baseline) | PASS |
| Strict clippy | `cargo clippy -p tome --all-targets -- -D warnings` | exit 0; clean | PASS |
| `cargo fmt --check` (marketplace.rs only) | `rustfmt --check crates/tome/src/marketplace.rs` | exit 0 | PASS |
| `cargo fmt --check` (workspace) | `cargo fmt --check` | reports diffs in cleanup.rs / library.rs / lockfile.rs — pre-existing from Phase 11; **NOT a Phase 12 regression** (marketplace.rs not implicated; those files were not modified in Phase 12) | PASS (within Phase 12 scope) |
| Full `cargo test -p tome` | (run) | 598 passed; 1 flaky failure on `backup::tests::list_returns_entries` ("fatal: failed to write commit object" — same global-git-config race noted in CLAUDE.md and the prompt). Re-ran in isolation: PASSED. NOT a Phase 12 regression. | PASS (re-ran flaky) |

### Requirements Coverage

| Requirement | Source Plan | Description | Status | Evidence |
| --- | --- | --- | --- | --- |
| ADP-01 | 12-01 | `MarketplaceAdapter` trait + module | SATISFIED | trait at marketplace.rs:83-113; module declared at lib.rs:42 |
| ADP-02 | 12-04 | `ClaudeMarketplaceAdapter` shells out + clear PATH error | SATISFIED | adapter at marketplace.rs:600-759; `new()` actionable error at 614-619 |
| ADP-03 | 12-03 | `GitAdapter` wraps `git::clone_repo`/`update_repo`; behavior unchanged | SATISFIED | adapter at marketplace.rs:340-438; D-05a regression: `cargo test --test cli` 141 passed (no change) |
| ADP-04 | 12-02 | `InstallFailure` aggregation + grouped renderer (SAFE-01 mirror) | SATISFIED | types at 127-260; renderer at 278-322; renderer NOT yet called from `sync()` (Phase 13 owns — explicit scope boundary, not a gap) |

No orphaned requirements: REQUIREMENTS.md maps ADP-01..ADP-04 to Phase 12 and all four are claimed by the four phase-12 plans.

### Scope Boundary Verification

| Boundary | Expected | Actual | Status |
| --- | --- | --- | --- |
| `lib.rs::sync()` call-site | UNCHANGED (Phase 13 owns dispatch) | Diff vs phase-11-end shows ONLY `pub(crate) mod marketplace;` line added; `sync()` body unchanged | RESPECTED |
| `crates/tome/src/git.rs` | UNCHANGED (no visibility widening needed) | `git diff ed81b23..HEAD -- crates/tome/src/git.rs` empty | RESPECTED |
| `crates/tome/tests/cli.rs` | UNCHANGED (D-05a contract) | `git diff ed81b23..HEAD -- crates/tome/tests/cli.rs` empty | RESPECTED |
| `crates/tome/src/cli.rs` | UNCHANGED (no new commands) | `git diff ed81b23..HEAD -- crates/tome/src/cli.rs` empty | RESPECTED |
| `Cargo.toml` (workspace + crate) | UNCHANGED (no new deps; `which` MUST NOT be added) | `git diff ed81b23..HEAD -- **/Cargo.toml` empty; `rg "\\bwhich\\b"` in Cargo files returns no matches | RESPECTED |
| `tome.toml` schema | UNCHANGED (config.rs untouched) | `git diff ed81b23..HEAD -- crates/tome/src/config.rs` empty | RESPECTED |
| No `--scope` flag in claude subprocess | (forbidden per D-09) | `rg '"--scope"' marketplace.rs` returns no matches; only doc-comment mentions saying it is NOT used | RESPECTED |

### Anti-Patterns Found

| File | Line | Pattern | Severity | Impact |
| --- | --- | --- | --- | --- |
| `crates/tome/src/marketplace.rs` | 33, 82, 125, 149, 181, 240, 277, 316, 339, 346, 511, 540, 599, 604 | `#[allow(dead_code)]` markers | Info | Documented and intentional per Phase 12 scope: every marker is annotated with the future Phase-13 consumer. Will be dropped when Phase 13 wires the dispatcher. NOT a stub indicator. |

No TODO / FIXME / placeholder comments. No `return Ok(())` stubs in production paths. No empty bodies. No console.log-only handlers.

### Pre-Existing Issues (NOT Phase 12 Regressions)

| Issue | Source | Impact | Action |
| --- | --- | --- | --- |
| `cargo fmt --check` reports diffs in `cleanup.rs`, `library.rs`, `lockfile.rs` | Pre-existing from Phase 11 (untouched in Phase 12; verified `git diff` shows no changes) | Phase-12 verification only — does not block Phase 12 goal | Should be cleaned up in a separate trivial commit on main; NOT scoped to Phase 12 |
| `backup::tests::list_returns_entries` flaky on parallel run | Global git-config race documented in CLAUDE.md and the verification prompt; not the same test name as the pre-known flake but same failure mode (git commit object write race) | Passes on retry in isolation | Pre-existing pattern; Phase 12 did not touch `backup.rs` |

### Human Verification Required

None. All phase 12 goals are programmatically verifiable and verified.

### Gaps Summary

No gaps found. The phase achieved its goal:

- All four plan must-haves match the real codebase.
- All locked types (per CONTEXT.md D-06, D-08) are present verbatim.
- All locked decisions (D-01 stdin closed, D-02 zero-extra-subprocess available, D-04 RefCell cache + auto-invalidate, D-09 no --scope) are honored.
- D-05a regression contract is preserved (`cargo test --test cli` = 141 passed, byte-for-byte parity with pre-Phase-12 baseline).
- Phase-13 scope boundary is respected: NO `sync()` call-site changes, NO new CLI commands, NO new config surface, NO new dependencies.
- 41 marketplace tests pass; production code compiles cleanly under strict clippy.

The phase produced exactly what the goal stated: trait + two production adapters + failure aggregation/renderer, ready for Phase 13 to wire into the sync flow.

---

_Verified: 2026-05-05T03:09:24Z_
_Verifier: Claude (gsd-verifier)_
