# Phase 12 — Deferred Items (Out-of-Scope Discoveries)

Tracking items found during Phase 12 execution that are out-of-scope per the
Rule 1-3 scope boundary (only auto-fix issues directly caused by the current
task's changes).

## Pre-existing `cargo fmt` drift (discovered during 12-01)

`cargo fmt --check` reports formatting drift in files **not modified by Phase
12**. These predate Phase 12 (likely accumulated from Phases 11 and earlier
without a final `cargo fmt` pass) and are unrelated to the marketplace adapter
work.

Files with pre-existing fmt drift (counts as of 12-01 execution):
- `crates/tome/src/cleanup.rs:292`
- `crates/tome/src/library.rs:947, 1200`
- `crates/tome/src/lockfile.rs:854, 864`
- `crates/tome/src/manifest.rs:147, 497, 527`
- `crates/tome/src/migration_v010.rs:380, 510, 551, 635, 673, 712`
- `crates/tome/src/remove.rs:75`
- `crates/tome/tests/cli.rs:5788, 5795, 6109`

`crates/tome/src/marketplace.rs` and the `lib.rs` edits made by Phase 12 are
fmt-clean. Project-wide `cargo fmt` should be run as a separate cleanup
commit (or as part of the Phase 15 CLI hardening bundle which already touches
many of these files).

## Pre-existing `cargo clippy --all-targets -- -D warnings` failure (discovered during 12-01)

`crates/tome/src/manifest.rs::SkillEntry::new_unowned` (added by Phase 11
commit `f869e03` — LIB-03) is reported as dead code by strict clippy. It IS
exercised by `manifest::tests::*` (lines 497, 527 in manifest.rs) so the
function is reachable under `#[cfg(test)]`, but it is not yet called from
any non-test path — that consumer arrives in Phase 13 (RECON-01..05) and
Phase 14 (UNOWN-01..03).

Verified at baseline commit `70cb4fe` (pre-Phase 12):

```
$ cargo clippy -p tome --all-targets -- -D warnings
error: associated function `new_unowned` is never used
  --> crates/tome/src/manifest.rs:150:12
```

This is a pre-existing baseline failure, NOT caused by Phase 12 changes.
Resolution paths:

1. **Preferred:** lands naturally when Phase 14 (`tome adopt` / `tome forget`)
   adds the first non-test consumer of `new_unowned`.
2. **Workaround:** add `#[allow(dead_code)]` to `new_unowned` with a "drop
   when Phase 14 lands" comment. Same shape Phase 12 used for
   `MarketplaceAdapter` and `InstalledPlugin`.

Phase 12 does not modify `manifest.rs` (out of scope per task boundaries).
The acceptance criterion `cargo clippy -p tome --all-targets -- -D warnings`
exits 0 in the plan was authored against an assumed-clean baseline; it
actually requires resolving this pre-existing item too. Documented here so
the verifier and the orchestrator have full context.

