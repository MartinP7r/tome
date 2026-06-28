# Checkpoint: 27-03 Task 1 — `similar 3.1.1` package legitimacy verification

**Plan:** 27-03 (SYNC-03 previewable machine.toml writes)
**Task:** Task 1 (`checkpoint:human-verify`, `gate="blocking-human"`)
**Reached:** 2026-06-06T14:32:39Z

## What was built

The planner audited `similar 3.1.1` (MIT, `mitsuhiko/similar`) and recorded an `ASSUMED-equivalent` disposition because the `slopcheck` tool was unavailable in this environment. The package is:

- Authored by **Armin Ronacher** (mitsuhiko — Flask, sentry-sdk, insta).
- First published 2021-03; current `3.1.1` published 2024-04.
- ~12M total downloads on crates.io.
- Present in the reverse-dependency graph of `insta` and `cargo-mutants` (already vetted upstream).
- MIT licensed (matches `deny.toml` allowlist).
- Will be added with `default-features=false, features=["text"]` and pinned to `=3.1.1`.

The audit table is captured at `.planning/phases/27-sync-triage-ui/27-RESEARCH.md` §"Package Legitimacy Audit" (lines 142–153).

## How to verify

1. Open https://crates.io/crates/similar in a browser.
   - Confirm the latest version is `3.1.x`.
   - Confirm the repository link points to `github.com/mitsuhiko/similar`.
   - Confirm the listed downloads count is in the millions.
2. Open https://github.com/mitsuhiko/similar.
   - Confirm an active issue tracker.
   - Confirm recent commits within the last 12 months.
   - Confirm MIT license.
   - Confirm no recent security advisories.
3. Confirm the planner's RESEARCH disposition is acceptable for adding the dep.
4. Approve or veto.

## Resume signal

Type **"approved"** to proceed to Task 2 (runs `cargo add similar --no-default-features --features text`, pins to `=3.1.1`, then `cargo deny check`).

OR describe the issue you found — the planner pre-recorded a fallback in RESEARCH §"Alternatives Considered": replace `similar` with a hand-rolled LCS diff in `machine.rs`. If you veto, document the issue here and the executor will switch to the fallback path.

## Pre-checkpoint state

No prior commits in this plan execution — Task 1 is the very first task in 27-03. Nothing to commit yet. After resume, Task 2 begins with `cargo add similar` + types + `preview_save` + tests.
