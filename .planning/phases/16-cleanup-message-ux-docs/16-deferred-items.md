# Phase 16 Deferred Items

Items discovered during phase 16 execution that are out of scope for this
phase's plans. Tracked here so they don't get lost.

## Pre-existing typos in unrelated files

Discovered during `make ci` while verifying Plan 16-01:

- `crates/tome/src/distribute.rs:177` — "mis-classified" flagged by typos as
  `mis` → `miss`/`mist`. The hyphen is intentional (compound modifier) — a
  `.typos.toml` allow-list entry would be the right fix, OR rephrase as
  "misclassified" (single word). Pre-existing in commit 98735a4 (Phase 15
  HARD-09 / 15-04).
- `crates/tome/tests/browse_snapshots.rs:167,170` — "fo" flagged by typos
  as a misspelling of `of`/`for`/etc. The string is a deliberate fuzzy-search
  fixture token. Pre-existing in commit 6944d1f (Phase 15 HARD-12 / 15-05).

These are NOT introduced by Phase 16 work. Leaving them for a follow-up
typos-allowlist tweak so Phase 16 plans can land clean.

## Action

Open a follow-up GitHub issue (`chore: typos allowlist for hyphenated
compounds and fuzzy-search test tokens`) — or add a `.typos.toml` allow
list — outside Phase 16 scope.
