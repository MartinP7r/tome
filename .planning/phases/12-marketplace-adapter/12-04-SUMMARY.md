---
phase: 12-marketplace-adapter
plan: 04
subsystem: marketplace
tags: [claude-marketplace-adapter, subprocess, refcell-cache, json-parser, heuristic-classifier, adp-02]

# Dependency graph
requires:
  - phase: 12-marketplace-adapter
    provides: "MarketplaceAdapter trait + InstalledPlugin (Plan 12-01) — ClaudeMarketplaceAdapter is the second non-test impl of the trait"
  - phase: 12-marketplace-adapter
    provides: "InstallFailure + InstallOp + InstallFailureKind (Plan 12-02) — used by build_install_failure() helper for the heuristic stderr -> kind mapping"
provides:
  - "ClaudePluginListEntry private serde shape (id, version, installPath, errors[default]) — tolerates extra fields (scope/enabled/installedAt/lastUpdated/mcpServers) and version=\"unknown\" literal"
  - "pub(crate) fn parse_claude_plugin_list_json(input: &str) -> Result<Vec<InstalledPlugin>> — pure JSON parser; testable without claude on PATH"
  - "pub(crate) fn classify_claude_install_stderr(stderr: &str) -> InstallFailureKind — pure heuristic; \"not found in marketplace\" + bare \"not found\" -> NotFound; else Unknown"
  - "pub fn is_claude_available() -> bool — mirrors git::is_git_available; probes `claude --version` exit-0"
  - "fn run_claude_subcommand(args: &[&str]) -> Result<Output> — private helper running `claude` with stdin = Stdio::null() per D-01; maps ErrorKind::NotFound to vanished-binary error"
  - "pub struct ClaudeMarketplaceAdapter { cache: RefCell<Option<Vec<InstalledPlugin>>> } — D-04 cache shape"
  - "pub fn ClaudeMarketplaceAdapter::new() -> Result<Self> — probes `claude --version` at construction, bails with actionable error message if missing (ADP-02)"
  - "#[cfg(test)] pub(crate) fn ClaudeMarketplaceAdapter::new_for_test() -> Self — bypasses binary probe so unit tests don't need claude on PATH"
  - "pub fn ClaudeMarketplaceAdapter::refresh(&self) -> Result<()> — D-04 explicit cache invalidate + re-query"
  - "pub(crate) fn ClaudeMarketplaceAdapter::build_install_failure(adapter_id, plugin_id, op, stderr) -> InstallFailure — testable wrapper that calls classify_claude_install_stderr"
  - "impl MarketplaceAdapter for ClaudeMarketplaceAdapter — id() returns \"claude-plugins\" constant; install/update use NO --scope flag (D-09); install/update on Ok auto-invalidate cache (D-04); available() reads cached errors[] field for the entry (D-02 zero extra subprocess calls); list_installed/current_version populate cache on first read"
  - "10 pure parser/heuristic unit tests + 8 ClaudeMarketplaceAdapter unit tests (use new_for_test + cache pre-populate) + 3 smoke tests gated behind is_claude_available — total 21 new tests in this plan"
affects:
  - 13-* (sync flow — D-11 dispatcher constructs `ClaudeMarketplaceAdapter::new()` for `DirectoryType::ClaudePlugins` entries; calls list_installed/current_version/available/install/update via Box<dyn MarketplaceAdapter>)

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Pure parser + pure heuristic classifier as standalone pub(crate) functions — testable with hand-rolled fixtures in CI without requiring the upstream binary on PATH (mirrors RESEARCH §Test Strategy for Shelled Code recommendation #3)"
    - "RefCell<Option<Vec<T>>> cache populated on first read; auto-invalidated on Ok install/update; public refresh() forces re-query — single-threaded interior mutability pattern matching the v0.10 sync flow's lack of concurrency requirements"
    - "Subprocess invocation with stdin = Stdio::null() per D-01; ErrorKind::NotFound mapped to clear vanished-binary error message (mirrors install.rs:57 pattern)"
    - "Twin-constructor pattern: pub fn new() probes the binary; #[cfg(test)] pub(crate) fn new_for_test() bypasses the probe so unit tests deterministic without external dependencies"
    - "Smoke tests gated behind binary-availability check, with eprintln-and-return on absence — pragmatic CI portability without the heavyweight #[ignore] flag"

key-files:
  created: []
  modified:
    - crates/tome/src/marketplace.rs

key-decisions:
  - "Heuristic classifier collapses two if-arms with identical bodies into a single OR (clippy::if_same_then_else triggers on duplicate match arms even with distinct comments). Comment block above the OR preserves the empirical mapping derivation."
  - "Drop the `#[allow(dead_code)]` from the parser/heuristic ONLY at the function level — cargo clippy --all-targets -- -D warnings still warns the new functions because they're only reachable from `build_install_failure` (test-only) until Phase 13. Apply the attr at function scope so it's localized."
  - "Add `#[allow(dead_code)]` to ClaudeMarketplaceAdapter struct + inherent impl block (matches Plan 12-03's GitAdapter pattern). The trait impl block does NOT need its own attr — it follows the trait's reachability, and the trait was attrs-cleared in Plan 12-03 when GitAdapter became its first real impl."
  - "is_claude_available() carries its own `#[allow(dead_code)]` because both production callers (`new()` itself only test-called) AND smoke tests reach it through code paths that are dead in the lib profile."
  - "Skip extending `which` crate per RESEARCH §Binary Detection — verified Cargo.toml does NOT include `which`; `Command::new(...).arg(\"--version\").output()` mirrors git::is_git_available verbatim and is sufficient. Acceptance criterion explicitly checks `! grep -q \"which::\"`."
  - "Test fixture pre-populates `adapter.cache.borrow_mut() = Some(vec![...])` directly to exercise trait method shape without spawning a subprocess. The `cache` field's visibility (private to the struct) is fine because tests live in the same module."
  - "Smoke tests run real `claude plugin list --json` against claude 2.1.128 on the dev machine — they passed verbatim, confirming the parser handles the live shape including the 37+-entry array. CI machines without claude will see the eprintln+return skip path."

patterns-established:
  - "Pure-function parser + pure-function heuristic classifier as `pub(crate)` siblings of the production adapter — gives CI coverage of the data-shape boundary without requiring the upstream binary, and the adapter's `populate_cache()` simply forwards parsed bytes through the public function."
  - "Twin-constructor pattern (`new` + `#[cfg(test)] new_for_test`) for adapters that probe binary availability — production callers always go through `new()` (which surfaces the actionable ADP-02 error); unit tests bypass the probe via `new_for_test()`. Documented inline so the test-only constructor doesn't get accidentally exposed."

requirements-completed: [ADP-02]

# Metrics
duration: 7min
completed: 2026-05-05
---

# Phase 12 Plan 04: ClaudeMarketplaceAdapter Summary

**Production `ClaudeMarketplaceAdapter` ships with a pure JSON parser, pure heuristic stderr classifier, RefCell-backed snapshot cache, subprocess invocations using stdin = /dev/null per D-01, the D-02 zero-extra-subprocess-call vanished signal via the cached `errors[]` field, and 21 new unit + smoke tests anchoring every trait method.**

## Performance

- **Duration:** ~7 min
- **Started:** 2026-05-05T02:53:15Z
- **Completed:** 2026-05-05T03:00:45Z
- **Tasks:** 2 (autonomous, TDD-shape — both committed atomically)
- **Files modified:** 1 (`crates/tome/src/marketplace.rs`)

## Accomplishments

### Task 1 — Pure parser + heuristic classifier

- `parse_claude_plugin_list_json(input: &str) -> Result<Vec<InstalledPlugin>>` lands as a `pub(crate)` pure function. Uses a private `ClaudePluginListEntry` serde shape with `#[serde(rename = "installPath")]` for the camelCase JSON key and `#[serde(default)]` so the optional `errors` field defaults to empty `Vec<String>` when absent. Tolerates extra fields (`scope`, `enabled`, `installedAt`, `lastUpdated`, `mcpServers`) by NOT applying `#[serde(deny_unknown_fields)]`. Tolerates `version: "unknown"` literal because the field type is `String`, not a semver newtype.
- `classify_claude_install_stderr(stderr: &str) -> InstallFailureKind` lands as a `pub(crate)` pure heuristic. Substring matches `"not found in marketplace"` OR bare `"not found"` -> `InstallFailureKind::NotFound`; everything else -> `InstallFailureKind::Unknown`. `NetworkError` and `PermissionDenied` are reserved for future heuristics once empirical evidence surfaces.
- 10 new unit tests:
  - `parse_claude_plugin_list_json_empty_array` (empty `[]` -> `Ok(vec![])`)
  - `parse_claude_plugin_list_json_single_entry_no_errors` (full happy-path entry shape, errors[] defaults to empty)
  - `parse_claude_plugin_list_json_entry_with_errors` (errors[] populated path)
  - `parse_claude_plugin_list_json_version_unknown_string` (literal `"unknown"` accepted)
  - `parse_claude_plugin_list_json_extra_fields_ignored` (`mcpServers` field present in input, silently dropped)
  - `parse_claude_plugin_list_json_malformed_returns_err` (malformed input returns Err with `claude plugin list` context)
  - `classify_stderr_not_found_in_marketplace_is_not_found` (install stderr -> NotFound)
  - `classify_stderr_not_found_bare_is_not_found` (update stderr -> NotFound)
  - `classify_stderr_unrecognized_is_unknown` (novel stderr -> Unknown)
  - `classify_stderr_empty_is_unknown` (empty stderr -> Unknown)

### Task 2 — ClaudeMarketplaceAdapter

- `ClaudeMarketplaceAdapter` struct holds `cache: RefCell<Option<Vec<InstalledPlugin>>>` per D-04.
- `pub fn new() -> Result<Self>` probes `claude --version` via `is_claude_available()`; returns `Err` with the verbatim ADP-02 actionable message naming the binary AND suggesting `tome.toml` cleanup as a fallback when claude is missing.
- `#[cfg(test)] pub(crate) fn new_for_test() -> Self` bypasses the probe — used by all 8 pure unit tests in this plan to exercise trait method shapes against pre-populated cache contents.
- `pub fn refresh(&self) -> Result<()>` per D-04: clears cache then calls `populate_cache()` to force re-query.
- Private `fn populate_cache(&self) -> Result<()>` is the single read-side cache populator; no-op when cache is already `Some`. Calls `run_claude_subcommand(&["plugin", "list", "--json"])`, bails with `String::from_utf8_lossy(&output.stderr).trim()` on non-zero exit, then parses stdout via `parse_claude_plugin_list_json`.
- `pub(crate) fn build_install_failure(adapter_id, plugin_id, op, stderr) -> InstallFailure` is the testable helper that wraps stderr -> failure. Per the deviation note in `<context_notes>`, this was extracted as a `pub(crate)` helper to make the stderr->failure conversion exercisable without spawning a subprocess.
- Private `fn run_claude_subcommand(args: &[&str]) -> Result<Output>` runs `Command::new("claude").args(args).stdin(Stdio::null()).output()` per D-01. Maps `ErrorKind::NotFound` to a clear "claude CLI not found on PATH (vanished between adapter construction and `claude {args}`)" error mirroring `install.rs:57`.
- `pub fn is_claude_available() -> bool` mirrors `git::is_git_available` from `git.rs:155-164`. Verifies via `Command::new("claude").arg("--version").output().map(|o| o.status.success()).unwrap_or(false)`. The `which` crate is NOT used (verified absent from `Cargo.toml`).
- `impl MarketplaceAdapter for ClaudeMarketplaceAdapter`:
  - `id() -> &str` returns the literal constant `"claude-plugins"`.
  - `current_version(plugin_id)` populates cache, then searches by id; returns `Ok(Some(version))` or `Ok(None)`.
  - `install(plugin_id)` runs `claude plugin install <plugin_id>` with NO `--scope` flag (per D-09 default user scope). On Ok, auto-invalidates cache per D-04. On non-zero exit, bails with verbatim trimmed stderr per D-01.
  - `update(plugin_id)` mirrors `install` but with `claude plugin update <plugin_id>`.
  - `list_installed()` populates cache, returns `Ok(cache.clone().unwrap_or_default())`.
  - `available(plugin_id)` per D-02: populates cache; for each entry matching the id, returns `Ok(false)` if `errors[].any(contains("not found in marketplace"))`; otherwise `Ok(true)`. Conservative default for unknown ids = `Ok(true)`. **Zero extra subprocess calls.**
- 8 pure unit tests (always run, use `new_for_test`):
  - `claude_adapter_id_is_stable_constant` (id = `"claude-plugins"`)
  - `claude_adapter_available_returns_false_for_errored_entry` (D-02 vanished signal)
  - `claude_adapter_available_returns_true_for_clean_entry` (D-02 happy path)
  - `claude_adapter_available_returns_true_for_entry_not_in_cache` (D-02 conservative default)
  - `claude_adapter_current_version_returns_some_for_known_plugin` (Some(version))
  - `claude_adapter_current_version_returns_none_for_unknown_plugin` (None)
  - `claude_adapter_build_install_failure_uses_heuristic_for_not_found` (kind = NotFound)
  - `claude_adapter_build_install_failure_unknown_for_novel_stderr` (kind = Unknown)
- 3 smoke tests gated behind `is_claude_available()`:
  - `smoke_claude_available_or_skip` (construction succeeds when binary present)
  - `smoke_claude_marketplace_adapter_lists_installed` (real `claude plugin list --json` against ~37 entries)
  - `smoke_claude_install_nonexistent_returns_err` (real install of `definitely-nonexistent-xyz@nonexistent-marketplace-xyz` returns Err)

### Test totals after this plan

- `cargo test -p tome --lib marketplace::tests`: **41 passed** (20 from Plans 12-01..12-03 + 10 parser/heuristic + 11 ClaudeMarketplaceAdapter unit + smoke = 41).
- `cargo test -p tome --test cli`: **141 passed** — same byte-for-byte count as the baseline captured before Phase 12 started. D-05a regression contract honored across all 4 plans.
- All 3 smoke tests passed against real `claude 2.1.128` on the dev machine — confirming the parser handles the live JSON shape.

## Task Commits

1. **Task 1: Pure parser + heuristic classifier (no subprocess)** — `36eda99` (feat)
2. **Task 2: ClaudeMarketplaceAdapter struct + cache + impl + smoke tests** — `b0fc438` (feat)

## Files Created/Modified

- `crates/tome/src/marketplace.rs` — **Modified.** Net diff across the two commits: +602 / -12. File size after this plan: 1538 LOC (up from 948 after Plan 12-03). 89 functions (counting both production + tests). Added imports: `std::cell::RefCell`, `anyhow::Context`, `serde::Deserialize`. Three new top-level items + two new pub(crate) functions + the ClaudeMarketplaceAdapter struct and its inherent impl + the trait impl + 21 new tests.

## Decisions Made

- **Heuristic OR-collapse for clippy::if_same_then_else.** Initial implementation used two separate `if/else if` arms both returning `InstallFailureKind::NotFound`. Strict clippy flagged this as `if_same_then_else` (duplicate-body match arms). Collapsed into a single `||` with inline comments preserving the empirical mapping derivation (`"not found in marketplace"` = install path, bare `"not found"` = update path). Same behavior, clippy-clean. Tests verify both branches still map to `NotFound`.
- **Twin-constructor pattern (`new` + `new_for_test`).** `pub fn new() -> Result<Self>` is the production-only entry point — it probes the binary so the actionable ADP-02 error fires for users without Claude Code installed. `#[cfg(test)] pub(crate) fn new_for_test() -> Self` bypasses the probe so the 8 pure unit tests run deterministically without claude on PATH. Documented inline (`/// Test-only constructor that bypasses the binary probe.`) so the test-only constructor doesn't get accidentally exposed. The pattern mirrors common Rust testing practice (e.g., `actix-web::HttpServer::for_test`); applied here because the binary probe in `new()` would block CI without claude.
- **`build_install_failure` extracted as `pub(crate)`.** The plan's `<context_notes>` flagged: "Pull `pub(crate) fn build_install_failure(...)` out as a testable helper that wraps stderr -> InstallFailure (uses classify_claude_install_stderr internally)." Applied verbatim — the helper takes `(adapter_id: &str, plugin_id: &str, op: InstallOp, stderr: &str) -> InstallFailure`, calls `classify_claude_install_stderr` internally, and constructs the `InstallFailure` with `source = anyhow::anyhow!("{}", stderr.trim())`. Tests assert on `kind`, `adapter_id`, `plugin_id`, and `operation` fields. Phase 13's sync flow may call this helper directly when wrapping adapter `install`/`update` errors into `Vec<InstallFailure>`.
- **`#[allow(dead_code)]` localization.** Following Plan 12-03's pattern: applied at the smallest possible scope.
  - `parse_claude_plugin_list_json` and `classify_claude_install_stderr` carry their own function-level allows (only consumer is `populate_cache()` and `build_install_failure()` respectively, both reachable only from test code until Phase 13).
  - `is_claude_available()` carries a function-level allow (only callers are `new()` itself test-only + smoke tests).
  - `ClaudeMarketplaceAdapter` struct + inherent impl block carry block-level allows (mirrors `GitAdapter` pattern from Plan 12-03).
  - The `impl MarketplaceAdapter for ClaudeMarketplaceAdapter` block does NOT need its own attr — trait impl methods follow the trait's reachability, and the trait was attrs-cleared by Plan 12-03 when `GitAdapter` became its first real impl.
  - Comments name Phase 13's D-11 dispatcher as the consumer that will allow the attrs to drop.
- **Pre-existing `#[allow(dead_code)]` retained on InstalledPlugin.** Although the four-field shape is now consumed by `parse_claude_plugin_list_json` and the `populate_cache()` chain, all those consumers are themselves under dead-code allows. The InstalledPlugin attr stays until Phase 13's `list_installed()` call site lands as a non-test caller. Verified empirically: removing the `#[allow]` triggers a `dead_code` warning under `cargo clippy --all-targets -- -D warnings`.
- **Pre-existing `#[allow(dead_code)]` retained on InstallOp / InstallFailureKind / InstallFailure / format_install_failures / render_install_failures.** Same reasoning: `build_install_failure` is itself test-only-consumed, so the producers it feeds remain dead-coded in the production lib profile until Phase 13 wires the adapter.
- **`run_claude_subcommand` carries no `#[allow]`.** Reachable from `populate_cache` -> `list_installed` -> ... and from `install`/`update` -> all inside the `impl MarketplaceAdapter for ClaudeMarketplaceAdapter` block, which is reachable from `Box<dyn MarketplaceAdapter>` constructed in tests. Clippy doesn't flag this one because it's a free function whose reachability includes the trait impl method chain.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] clippy::if_same_then_else flagged duplicate heuristic arms**

- **Found during:** Task 1 (during initial `cargo clippy --all-targets -- -D warnings` run).
- **Issue:** The plan's recommended heuristic body had two separate `if`/`else if` arms both returning `InstallFailureKind::NotFound` for "not found in marketplace" and bare "not found". Strict clippy flagged this as `if_same_then_else`: duplicate match arms with identical bodies are a code smell.
- **Fix:** Collapsed both substring matches into a single `if` using `||`. Inline comments preserve the empirical mapping derivation (which substring corresponds to which upstream code path). Same behavior, clippy-clean. Tests for both branches still pass.
- **Files modified:** `crates/tome/src/marketplace.rs` (heuristic function body)
- **Verification:** `cargo clippy -p tome --all-targets -- -D warnings` exits 0 after the collapse. `classify_stderr_not_found_in_marketplace_is_not_found` and `classify_stderr_not_found_bare_is_not_found` both pass.
- **Committed in:** `36eda99` (Task 1)

**2. [Rule 3 - Blocking] dead_code warnings on the new ClaudeMarketplaceAdapter pieces**

- **Found during:** Task 2 (after writing the adapter, running `cargo clippy --all-targets -- -D warnings`).
- **Issue:** Strict clippy flagged the new pieces as never-used in the production lib profile: `parse_claude_plugin_list_json` (only consumer is `populate_cache` in a test-only-constructed struct), `classify_claude_install_stderr` (only consumer is `build_install_failure`, itself test-only), `is_claude_available` (callers are `new()` itself test-only + smoke tests), `new` / `refresh` / `build_install_failure` associated items on `impl ClaudeMarketplaceAdapter`. The plan's clippy acceptance criterion required exit-0.
- **Fix:** Applied `#[allow(dead_code)]` at the smallest possible scope:
  - Function-level on `parse_claude_plugin_list_json`, `classify_claude_install_stderr`, `is_claude_available` (each function is independently dead-coded until its first non-test caller arrives in Phase 13).
  - Block-level on `pub struct ClaudeMarketplaceAdapter` and `impl ClaudeMarketplaceAdapter` (matches Plan 12-03's `GitAdapter` pattern). The `impl MarketplaceAdapter for ClaudeMarketplaceAdapter` block does NOT need its own attr.
  - Comments name Phase 13's D-11 dispatcher (`match dir.directory_type { ClaudePlugins => ClaudeMarketplaceAdapter::new()? }`) as the consumer that will allow the attrs to drop.
- **Files modified:** `crates/tome/src/marketplace.rs`
- **Verification:** `cargo clippy -p tome --all-targets -- -D warnings` exits 0 after the attrs land.
- **Committed in:** `b0fc438` (Task 2)

**Total deviations:** 2 auto-fixed (1 Rule 1 - bug, 1 Rule 3 - blocking).
**Impact on plan:** No scope creep. Both deviations mirror exact patterns from Plans 12-01..12-03 — the dead-code attrs drop automatically when Phase 13 wires the dispatcher; the clippy::if_same_then_else collapse is a pure-style improvement that preserves behavior.

## Authentication Gates

None. The smoke tests against real `claude 2.1.128` ran inline against the developer's existing PATH installation; no credentials, no API keys, no interactive prompts. The `claude --version` probe and `claude plugin list --json`/`claude plugin install ... </dev/null` invocations all completed without auth interaction (per D-01 stdin = /dev/null and CONTEXT.md's empirical findings on subprocess non-interactivity).

## Issues Encountered

- **Initial workspace `cargo fmt --all -- --check` shows pre-existing drift in unrelated files** (cleanup.rs, library.rs, lockfile.rs, manifest.rs, migration_v010.rs, remove.rs, tests/cli.rs) carried over from before Phase 12. Per Plan 12-01's deferred-items.md and Plan 12-03's notes, this is out-of-scope for this plan. The narrowly-scoped `rustfmt --check crates/tome/src/marketplace.rs` exits clean. No new fmt drift introduced by Plan 12-04.
- **Two-task plan, single-file edits.** Per the Wave-2 file-sharing note in the plan, both Task 1 and Task 2 edit the same `crates/tome/src/marketplace.rs` file. Atomic per-task commits achieved by: (a) implementing both tasks fully, (b) verifying clippy + tests + integration tests pass, (c) saving the final state to `/tmp`, (d) reverting marketplace.rs to HEAD, (e) re-applying Task 1 changes only, committing as `36eda99`, (f) restoring the saved final state, committing as `b0fc438`. Both commits' final state is byte-identical to the verified-passing checkpoint.

## Deferred Issues

- None new. The pre-existing fmt drift in 7 unrelated files (carried over from before Phase 12) and the silenced `#[allow(dead_code)]` attrs on `InstalledPlugin`, `InstallOp`, `InstallFailureKind`, `InstallFailure`, `format_install_failures`, `render_install_failures`, `parse_claude_plugin_list_json`, `classify_claude_install_stderr`, `is_claude_available`, `ClaudeMarketplaceAdapter` (struct + inherent impl) all remain — they drop automatically when Phase 13's D-11 dispatcher wires the adapter into `lib.rs::sync` (`match dir.directory_type { ClaudePlugins => ClaudeMarketplaceAdapter::new()? }`).

## Known Stubs

None. The adapter is fully implemented per CONTEXT.md D-01..D-04, D-08, D-09 + RESEARCH.md's verified JSON shape. The `#[allow(dead_code)]` attrs are not stubs — they're temporary suppression markers tied to Phase 13's D-11 dispatcher; the actual code is production-shape and test-covered (10 parser/heuristic tests + 8 adapter unit tests + 3 smoke tests = 21 new tests anchoring every trait method against both pre-populated cache and real-claude paths).

## User Setup Required

None — no external service configuration required. Pure unit tests use only the in-process serde JSON parser; ClaudeMarketplaceAdapter unit tests use `new_for_test()` to bypass the binary probe; smoke tests gate behind `is_claude_available()` and exit cleanly via `eprintln!("SKIP ...")` if the binary is missing. CI machines without Claude Code installed will see the smoke tests pass-with-skip.

## Next Plan Readiness

- **Phase 12 complete.** All 4 ADP requirements wired across the four plans:
  - **ADP-01** (trait + InstalledPlugin + MockMarketplaceAdapter) — Plan 12-01.
  - **ADP-02** (ClaudeMarketplaceAdapter shells out to `claude plugin install/update/list --json`; surfaces "claude not on PATH" as a clear error) — **THIS PLAN**.
  - **ADP-03** (GitAdapter wraps existing `git.rs::clone_repo`/`update_repo`; behavior unchanged) — Plan 12-03.
  - **ADP-04** (Vec<InstallFailure> aggregates; grouped failure renderer mirrors v0.8 SAFE-01) — Plan 12-02.
- **Phase 13 (lockfile-authoritative sync — alpha cut)** can now build on the complete adapter trait. The D-11 dispatcher in `lib.rs::sync` is the next logical wiring point: `match dir.directory_type { Git => GitAdapter::for_directory(dir, paths)?, ClaudePlugins => ClaudeMarketplaceAdapter::new()?, Directory => /* no adapter, skill discovery only */ }` -> `Box<dyn MarketplaceAdapter>`. When Phase 13 wires this, the `#[allow(dead_code)]` attrs on `InstalledPlugin`, `InstallOp`, `InstallFailureKind`, `InstallFailure`, `format_install_failures`, `render_install_failures`, the parser, the classifier, `is_claude_available`, `ClaudeMarketplaceAdapter` (struct + inherent impl), and `GitAdapter` (struct + inherent impl) all drop automatically as Phase 13's reachability analysis discovers them.
- **D-02 vanished signal is wired and ready.** Phase 13's RECON-04 vanished-plugin detection comes "for free" — `available(plugin_id)` reads the cached `errors[]` field from the same snapshot used by `list_installed()`, with zero extra subprocess calls. Phase 13's drift classification can call `available()` after `list_installed()` and they share the cache populated on the first read of the sync flow.

## Self-Check: PASSED

Verified via:
- `[ -f crates/tome/src/marketplace.rs ] && echo FOUND` -> FOUND (1538 LOC, 89 functions)
- `git log --oneline | grep -E "36eda99|b0fc438"` -> both present
- `cargo test -p tome --lib marketplace::tests --quiet` -> 41 passed, 0 failed
- `cargo test -p tome --test cli --quiet` -> 141 passed, 0 failed (D-05a byte-for-byte regression contract: same as baseline)
- `cargo clippy -p tome --all-targets -- -D warnings` -> exits 0
- `rustfmt --check crates/tome/src/marketplace.rs` -> no diffs
- `cargo check -p tome` -> exits 0
- `rg -q "pub\(crate\) fn parse_claude_plugin_list_json" crates/tome/src/marketplace.rs` -> match
- `rg -q "pub\(crate\) fn classify_claude_install_stderr" crates/tome/src/marketplace.rs` -> match
- `rg -q 'rename = "installPath"' crates/tome/src/marketplace.rs` -> match
- `rg -q 'serde\(default\)' crates/tome/src/marketplace.rs` -> match (errors field optional)
- `rg -q "pub struct ClaudeMarketplaceAdapter" crates/tome/src/marketplace.rs` -> match
- `rg -q "impl MarketplaceAdapter for ClaudeMarketplaceAdapter" crates/tome/src/marketplace.rs` -> match
- `rg -q "pub fn is_claude_available" crates/tome/src/marketplace.rs` -> match
- `rg -q "RefCell<Option<Vec<InstalledPlugin>>>" crates/tome/src/marketplace.rs` -> match (D-04 cache shape)
- `rg -q "Stdio::null" crates/tome/src/marketplace.rs` -> match (D-01 stdin closed)
- `rg "plugin.*list.*--json" crates/tome/src/marketplace.rs` -> matches in production code + doc comments
- `rg 'plugin", "install"' crates/tome/src/marketplace.rs` -> match (install subprocess)
- `rg 'plugin", "update"' crates/tome/src/marketplace.rs` -> match (update subprocess)
- `rg -q "not found in marketplace" crates/tome/src/marketplace.rs` -> match (D-02 vanished signal substring + heuristic)
- `rg -q "pub fn refresh" crates/tome/src/marketplace.rs` -> match (D-04 explicit refresh)
- `rg -q "build_install_failure" crates/tome/src/marketplace.rs` -> match (testable failure-construction helper)
- `rg -q "which::" crates/tome/src/marketplace.rs` -> NO match (the `which` crate is NOT used per RESEARCH "Binary Detection")
- `rg -- '"--scope"' crates/tome/src/marketplace.rs` -> NO match (no `--scope` argument string literal in any subprocess invocation per D-09; all `--scope` mentions are documentation comments noting D-09 forbids it)

---
*Phase: 12-marketplace-adapter*
*Completed: 2026-05-05*
