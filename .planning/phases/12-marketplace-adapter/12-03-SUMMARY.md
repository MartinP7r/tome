---
phase: 12-marketplace-adapter
plan: 03
subsystem: marketplace
tags: [git-adapter, marketplace-adapter, shim, regression-contract, d-05a]

# Dependency graph
requires:
  - phase: 12-marketplace-adapter
    provides: "MarketplaceAdapter trait + InstalledPlugin (Plan 12-01) — GitAdapter is the first non-test impl of the trait"
  - phase: 06-unified-directory-model (v0.6, shipped)
    provides: "crate::git helpers (clone_repo, update_repo, read_head_sha, repo_cache_dir) — pub(crate) and visible from sibling marketplace.rs without widening"
provides:
  - "GitAdapter struct (url, cache_dir, git_ref) — one adapter per [directories.<git-name>] entry per D-05"
  - "GitAdapter::for_directory(&DirectoryConfig, &TomePaths) -> Result<Self> — fallible constructor mirroring remove.rs:241-244 URL extraction pattern"
  - "impl MarketplaceAdapter for GitAdapter — id() returns the URL string; current_version/list_installed/available trust local-clone existence; install/update delegate verbatim to crate::git::clone_repo / update_repo"
  - "9 GitAdapter unit tests — empty-cache path (id, current_version=None, available=false, list=empty, url+ref extraction) + post-install path (clone_repo invocation, 40-char SHA, list entry shape, available=true)"
affects:
  - 12-04-PLAN (ClaudeMarketplaceAdapter — second MarketplaceAdapter impl, lands in same marketplace.rs file)
  - 13-* (sync flow — D-11 dispatcher constructs `GitAdapter::for_directory(...)` for every DirectoryType::Git entry)

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Thin shim adapter — delegates every trait method to existing pub(crate) helpers verbatim; D-05a regression contract anchors that the wrap doesn't change semantics for existing git directories"
    - "Fallible constructor returning Result — for_directory propagates URL UTF-8 validation as anyhow Err, matching remove.rs's existing pattern (preferred over PathBuf::to_string_lossy because it surfaces invalid-UTF-8 paths instead of silently mangling them)"
    - "TempDir + `git init` fixture builder for adapter tests — mirrors git.rs:252-289 (no network calls; file paths work as git URLs for clone_repo)"

key-files:
  created: []
  modified:
    - crates/tome/src/marketplace.rs

key-decisions:
  - "Drop the `#[allow(dead_code)]` from `MarketplaceAdapter` trait (it now has a real impl: GitAdapter). InstalledPlugin keeps the attr until Phase 13's sync dispatcher consumes the type from non-test code."
  - "Add `#[allow(dead_code)]` to `GitAdapter` + its inherent impl block. Phase 13's D-11 dispatcher is the first non-test consumer (`match dir.directory_type { Git => GitAdapter::for_directory(...) }`); until then strict clippy `-D warnings` flags the adapter as never-constructed."
  - "Use `dir_config.path.to_str().ok_or_else(|| anyhow::anyhow!(...))?` for URL extraction (matches remove.rs:241-244), NOT `to_string_lossy()`. Bails on invalid UTF-8 instead of silently producing a mangled URL."
  - "9 tests instead of the plan's listed 7 — added two post-install assertions (`list_installed_after_install_returns_one_entry`, `available_returns_true_after_install`) so the post-install path of every trait method is anchored, not just current_version. All cheap; same TempDir/origin fixture each time."
  - "Test fixture builder uses a real working repo (`git init` + commit), NOT `git init --bare`. The plan's behavior section briefly mentioned `--bare` but `git::clone_repo` calls `git clone` which expects a regular repo or bare repo — a regular working repo with at least one commit is the simplest clonable source on POSIX. Mirrors git.rs::tests::read_head_sha_returns_40_char_hex (git.rs:252-289)."

patterns-established:
  - "When a Phase 12 trait gets its first concrete impl, drop the trait's `#[allow(dead_code)]` and add a fresh one to the new impl type. Keeps the lint surface localised: adapters get the carry-over until Phase 13 dispatches them."
  - "Adapter shim tests should anchor BOTH the empty-state path AND the post-action path of every trait method. The empty path catches `Result<Option<...>>` fallback shape; the post-action path catches the delegate's actual behavior."

requirements-completed: [ADP-03]

# Metrics
duration: 4min
completed: 2026-05-05
---

# Phase 12 Plan 03: GitAdapter Marketplace Shim Summary

**GitAdapter implements MarketplaceAdapter as a thin shim over crate::git — every trait method delegates verbatim to the existing v0.6 helpers, anchored by 9 unit tests and the D-05a byte-for-byte regression contract on tests/cli.rs (141 tests passing, same as baseline).**

## Performance

- **Duration:** ~4 min
- **Started:** 2026-05-05T02:45:38Z
- **Completed:** 2026-05-05T02:49:09Z
- **Tasks:** 1 (autonomous, TDD-shape — production + tests in single commit)
- **Files modified:** 1 (`crates/tome/src/marketplace.rs`)

## Accomplishments

- `pub struct GitAdapter` (url, cache_dir, git_ref) lands in `crates/tome/src/marketplace.rs` per CONTEXT.md D-05.
- `pub fn for_directory(dir_config: &DirectoryConfig, paths: &TomePaths) -> Result<Self>` extracts URL from `dir_config.path` via `to_str().ok_or_else(...)?` (matches remove.rs:241-244), precomputes `cache_dir` via `git::repo_cache_dir(&paths.repos_dir(), &url)`, and clones the `git_ref` from the directory config. Fallible because URL UTF-8 validation can fail.
- `impl MarketplaceAdapter for GitAdapter` ships every trait method as a thin delegate:
  - `id()` returns `&self.url`.
  - `current_version(_)` returns `Ok(None)` if `cache_dir` doesn't exist; otherwise `git::read_head_sha(&self.cache_dir).map(Some)`.
  - `install(_)` calls `git::clone_repo(&self.url, &self.cache_dir, branch, tag, rev)`.
  - `update(_)` calls `git::update_repo(&self.cache_dir, branch, tag, rev)`.
  - `list_installed()` returns `Ok(vec![])` if not cloned; otherwise returns one `InstalledPlugin { id: url, version: HEAD SHA, install_path: cache_dir, errors: vec![] }`.
  - `available(_)` returns `Ok(self.cache_dir.exists())` per RESEARCH Q #5 (git URLs don't vanish; trust local existence).
- The `plugin_id: &str` argument is ignored (`_plugin_id`) — there's only one "plugin" per git directory (the repo itself), as specified in CONTEXT.md D-05.
- 9 new unit tests pass: `git_adapter_id_returns_url`, `git_adapter_current_version_none_when_not_cloned`, `git_adapter_available_returns_false_when_not_cloned`, `git_adapter_list_installed_empty_when_not_cloned`, `git_adapter_for_directory_extracts_url_and_ref`, `git_adapter_install_invokes_clone_repo`, `git_adapter_current_version_after_install_is_head_sha`, `git_adapter_list_installed_after_install_returns_one_entry`, `git_adapter_available_returns_true_after_install`.
- Marketplace test count: 11 (Plan 12-01 + 12-02) + 9 (this plan) = **20 tests**, all passing.
- D-05a regression contract: `cargo test -p tome --test cli` passes with **141 tests** — same byte-for-byte count as the baseline captured before this plan started. `crates/tome/src/git.rs` and `crates/tome/tests/cli.rs` are both unchanged (verified via `git diff --stat`).
- Full verification suite passes: `cargo check -p tome`, `cargo test -p tome --lib marketplace::tests` (20/20), `cargo test -p tome --test cli` (141/141), `cargo clippy -p tome --all-targets -- -D warnings` (clean), `rustfmt --check crates/tome/src/marketplace.rs` (clean).

## Task Commits

1. **Task 1: Implement GitAdapter struct + MarketplaceAdapter impl + 9 unit tests** — `5bd556f` (feat)

## Files Created/Modified

- `crates/tome/src/marketplace.rs` — **Modified.** Added 3 imports (`crate::config::{DirectoryConfig, GitRef}`, `crate::git`, `crate::paths::TomePaths`); dropped `#[allow(dead_code)]` from `MarketplaceAdapter` trait now that GitAdapter implements it (kept on `InstalledPlugin` until Phase 13's first non-test consumer); added `pub struct GitAdapter` with three private fields + 4 inherent methods (for_directory, ref_branch, ref_tag, ref_rev); added `impl MarketplaceAdapter for GitAdapter` with 6 thin-shim methods; added 9 GitAdapter unit tests inside the existing `#[cfg(test)] mod tests` block. Net diff: +301 / -7. File size after this plan: ~957 LOC.

## Decisions Made

- **Drop `#[allow(dead_code)]` from `MarketplaceAdapter` trait, keep on `InstalledPlugin`.** The trait now has a real production impl (`GitAdapter`), so clippy's reachability analysis sees it as used. `InstalledPlugin` doesn't yet have a non-test caller (Phase 13's sync dispatcher will consume it) so its attr stays. Mirrors Plan 12-01 / 12-02's incremental-attr-drop pattern.
- **Add `#[allow(dead_code)]` to `GitAdapter` + its inherent impl block.** Phase 13's D-11 dispatcher (`match dir.directory_type { Git => GitAdapter::for_directory(...) }`) is the first non-test consumer; until that lands, strict clippy `-D warnings` flags `GitAdapter::for_directory`, `ref_branch`, `ref_tag`, `ref_rev` as never-called outside tests. Comment names the consumer phase (Phase 13) so the attr is dropped automatically when the dispatcher arrives.
- **URL extraction via `path.to_str().ok_or_else(...)?`** (NOT `to_string_lossy()`). Matches remove.rs:241-244 verbatim — bails on invalid UTF-8 instead of silently mangling. The plan and CONTEXT.md both flagged this preference.
- **Trust local-clone existence in `available()`.** Per RESEARCH Q #5: git URLs don't "vanish" the way marketplace plugins do. A missing local clone means "not yet installed", not "vanished". Documented in a doc comment so a future feature that adds true vanished detection (`git ls-remote --exit-code`) knows where to plug in.
- **Test fixture uses a real working repo, not bare.** `git::clone_repo` shells out to `git clone`, which clones from a working repo or bare repo — a working repo with one commit is the smallest reproducible source on POSIX. Mirrors `git.rs::tests::read_head_sha_returns_40_char_hex` (git.rs:252-289). The plan's behavior section briefly mentioned `--bare` but the actual code-path needs a clonable source, which a working repo satisfies.
- **9 tests instead of the plan's listed 7.** Added `git_adapter_list_installed_after_install_returns_one_entry` and `git_adapter_available_returns_true_after_install` so every trait method has both an empty-state and post-install assertion. Cheap (same fixture each time); covers the post-install path of `list_installed` (entry shape) and `available` (round-trip Ok(true)) which the plan's 7-test list left implicit.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] `#[allow(dead_code)]` on `GitAdapter` + impl block**
- **Found during:** Task 1 (after writing the adapter, running `cargo clippy -p tome --all-targets -- -D warnings`).
- **Issue:** Strict clippy flagged `GitAdapter` (struct never constructed in non-test code), `for_directory` (never called outside tests), `ref_branch` / `ref_tag` / `ref_rev` (helper methods never called outside the trait impl that's also only test-exercised). The plan's clippy acceptance criterion required exit-0 for `--all-targets -- -D warnings`, which fires on the lib (production) profile separately from the lib-test profile.
- **Fix:** Added `#[allow(dead_code)]` to the `GitAdapter` struct and its inherent `impl GitAdapter { ... }` block (one attr covers all four inherent methods). Comment names Phase 13's D-11 dispatcher as the first non-test consumer that will drop the attr.
- **Files modified:** `crates/tome/src/marketplace.rs`
- **Verification:** `cargo clippy -p tome --all-targets -- -D warnings` exits 0 after the attrs land. The `impl MarketplaceAdapter for GitAdapter` block does NOT need `#[allow(dead_code)]` (trait impl methods follow the trait's reachability — the trait itself was already attrs-cleared in this plan).
- **Committed in:** `5bd556f` (Task 1).

**Total deviations:** 1 auto-fixed (1 Rule 3 - blocking).
**Impact on plan:** No scope creep. Mirrors Plans 12-01 / 12-02's exact deviation pattern; the attrs drop automatically when Phase 13 wires the D-11 dispatcher. The plan's clippy acceptance criterion was authored against an assumed-clean baseline that didn't account for the lib-vs-lib-test profile split.

## Issues Encountered

- **Initial rustfmt re-flow.** The first edit produced multi-line `TomePaths::new(..)` invocations in the test fixtures because each parameter was already on its own line in the heredoc. `rustfmt` collapsed them to single-line forms (well within the ~100-char width). Re-ran `rustfmt /path/to/marketplace.rs` (single file, NOT `cargo fmt -- ...` to avoid the workspace-wide fmt drift documented in Plan 12-01's deferred-items.md) and re-verified `rustfmt --check` exits clean.

## Deferred Issues

- None new. The pre-existing fmt drift in 7 unrelated files (cleanup.rs, library.rs, lockfile.rs, manifest.rs unrelated lines, migration_v010.rs, remove.rs, tests/cli.rs) and the silenced `SkillEntry::new_unowned` lint from Plan 12-01 remain in `.planning/phases/12-marketplace-adapter/deferred-items.md`. This plan modifies only `crates/tome/src/marketplace.rs` and introduces no new pre-existing-issue carry-overs.

## Known Stubs

None. The adapter is fully implemented per CONTEXT.md D-05 and the GitAdapter section of RESEARCH.md. The `#[allow(dead_code)]` attrs are not stubs — they're temporary suppression markers tied to Phase 13's D-11 dispatcher, and the actual code is production-shape and test-covered (9 unit tests anchor the empty-cache and post-install paths of every trait method).

## User Setup Required

None — no external service configuration required. Tests use only `tempfile::TempDir` and local `git init` repos; CI's `git` install (verified by Plan 12-01 baseline) covers the subprocess invocations.

## Next Plan Readiness

- **Plan 12-04 (ClaudeMarketplaceAdapter)** can append to `crates/tome/src/marketplace.rs`: GitAdapter is in place; the `MarketplaceAdapter` trait surface is locked and clippy-clean; the `InstallFailure` family from Plan 12-02 is ready for Plan 12-04's heuristic stderr-to-`InstallFailureKind` mapper. The only `#[allow(dead_code)]` attrs that 12-04 must consider dropping are: `InstallOp` and `InstallFailureKind` (12-04's heuristic mapper is the first non-test producer), `InstallFailureKind::label` (12-04's renderer indirectly exercises it), `format_install_failures` / `render_install_failures` (still gated on Phase 13's sync flow — keep until then), `InstallFailure` (12-04 constructs it). Plan 12-01's `InstalledPlugin` and `MarketplaceAdapter` stay attr-cleared because GitAdapter is now a real consumer (`InstalledPlugin` keeps its attr only until Phase 13's `list_installed()` call site lands).
- **Phase 13 (sync wiring)** drops `#[allow(dead_code)]` on `GitAdapter` + impl block automatically when `lib.rs::sync` invokes `GitAdapter::for_directory(...)` for each `DirectoryType::Git` entry. The expected pattern: `match dir.directory_type { DirectoryType::Git => Some(Box::new(GitAdapter::for_directory(dir, paths)?) as Box<dyn MarketplaceAdapter>), ... }`.

## Self-Check: PASSED

Verified via:
- `[ -f crates/tome/src/marketplace.rs ] && echo FOUND` → FOUND
- `git log --oneline | grep -E "5bd556f"` → present
- `cargo test -p tome --lib marketplace::tests --quiet` → 20 passed, 0 failed (11 from Plans 12-01/12-02 + 9 new GitAdapter)
- `cargo test -p tome --test cli --quiet` → 141 passed, 0 failed (D-05a byte-for-byte regression contract: same as baseline)
- `cargo clippy -p tome --all-targets -- -D warnings` → exits 0
- `rustfmt --check crates/tome/src/marketplace.rs` → no diffs
- `cargo check -p tome` → exits 0
- `git diff --stat crates/tome/src/git.rs crates/tome/tests/cli.rs` → empty (D-05a contract: both files unchanged)
- `rg -q "pub struct GitAdapter" crates/tome/src/marketplace.rs` → match
- `rg -q "impl MarketplaceAdapter for GitAdapter" crates/tome/src/marketplace.rs` → match
- `rg -q "pub fn for_directory" crates/tome/src/marketplace.rs` → match
- `rg -q "git::clone_repo\(" crates/tome/src/marketplace.rs` → match
- `rg -q "git::update_repo\(" crates/tome/src/marketplace.rs` → match
- `rg -q "git::read_head_sha\(" crates/tome/src/marketplace.rs` → match

---
*Phase: 12-marketplace-adapter*
*Completed: 2026-05-05*
