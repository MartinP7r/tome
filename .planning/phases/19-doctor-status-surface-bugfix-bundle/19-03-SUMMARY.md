---
phase: 19-doctor-status-surface-bugfix-bundle
plan: 03
subsystem: status + manifest
tags: [OBS-07, D-LSYNC-1, D-LSYNC-2, D-LSYNC-3, D-DIR-1, status, manifest]
dependency_graph:
  requires: [OBS-06]
  provides: [last_synced_at-manifest-header, status.last_sync-field, status-skills-column]
  affects: [crates/tome/src/manifest.rs, crates/tome/src/status.rs, crates/tome/src/lib.rs, crates/tome/tests/cli_status.rs]
tech_stack:
  added: []
  patterns: [additive-serde-Option-default, stable-shape-JSON-emit-null]
key_files:
  created: []
  modified:
    - crates/tome/src/manifest.rs
    - crates/tome/src/status.rs
    - crates/tome/src/lib.rs
    - crates/tome/tests/cli_status.rs
    - crates/tome/tests/snapshots/cli_status__status_empty_library.snap
decisions:
  - "Stamp call lives inside the existing `if !dry_run && paths.config_dir().is_dir()` guard at lib.rs:1789, immediately before `manifest::save` — dry-run does NOT stamp."
  - "Threaded `last_synced_at` off the existing `manifest::load` call in `status::gather` that already populates `unowned` — one read, two pieces of header data."
  - "`StatusReport.last_sync` emits literal `null` in JSON for fresh manifests (no `skip_serializing_if`), matching the stable-shape pattern already used by `unowned: []`."
  - "Reconcile-install-failure bail at end of sync() leaves the stamp in place — per RESEARCH OQ-3, the user-facing semantics are 'cleanup completed; install-failure exit is downstream.'"
metrics:
  duration_seconds: 467
  completed_date: 2026-05-13
  tasks_completed: 2
  files_changed: 5
  tests_added: 9
requirements: [OBS-07]
---

# Phase 19 Plan 03: Status last-sync header + per-directory SKILLS column Summary

`tome status` gains a top-line `Last sync: <RFC-3339>` (or `never`) plus a SKILLS column on the Directories table; JSON parity via a new top-level `last_sync: Option<String>` field. Manifest gains an additive `last_synced_at: Option<String>` header field stamped at the end of every successful `sync()`.

## Final Placement of `stamp_last_synced_at()` in `sync()`

`crates/tome/src/lib.rs:1789` — inside the existing `if !dry_run && paths.config_dir().is_dir()` block at line 1786, immediately before the existing `manifest::save(&manifest, paths.config_dir())?` call at line 1790. The stamp is INSIDE the `!dry_run` guard so dry-run does NOT update `last_synced_at` (honest reporting per D-LSYNC-3).

```rust
// 7. Save manifest, gitignore, and lockfile
if !dry_run && paths.config_dir().is_dir() {
    // D-LSYNC-3 (OBS-07): stamp after distribute + cleanup succeed,
    // before persist. The stamp is INSIDE the `!dry_run` guard so
    // dry-run does NOT update last_synced_at — honest reporting.
    //
    // Note: a subsequent reconcile-install-failure bail (`bail!` at
    // the end of sync()) still treats `last_synced_at` as stamped —
    // the user-facing semantics are "cleanup completed; install-
    // failure exit is downstream." Per RESEARCH OQ-3.
    manifest.stamp_last_synced_at();
    manifest::save(&manifest, paths.config_dir())?;
}
```

## Reconcile-Install-Failure Bail Ordering

No additional handling required beyond the inline comment. The `reconcile_install_failures` bail at lib.rs:1880-1886 sits AFTER the manifest save block, so a stamp + save pair completes before the bail surfaces a non-zero exit. This is the documented semantics per D-LSYNC-3 ("after distribute + cleanup succeed") and RESEARCH OQ-3.

## Final Directories Table Under Realistic Data

Smoke from `cargo run -- status` against the real machine config (Martin's `~/dev/coding-agent-files/skills` library, 106 skills consolidated, 5 directories):

```
Library: ~/dev/coding-agent-files/skills
  ✓ 106 skills consolidated
  Last sync: never

Directories:
 NAME             TYPE             ROLE                                                   PATH                           SKILLS
 antigravity      directory        Synced (skills discovered here AND distributed here)   ~/.gemini/antigravity/skills   ✓ 105
 claude-plugins   claude-plugins   Managed (read-only, owned by package manager)          ~/.claude/plugins              ✓ 6
 claude-skills    directory        Synced (skills discovered here AND distributed here)   ~/.claude/skills               ✓ 123
 codex            directory        Synced (skills discovered here AND distributed here)   ~/.codex/skills                ✓ 106
 codex-agents     directory        Synced (skills discovered here AND distributed here)   ~/.agents/skills               ✓ 105
```

Column widths flex with content (no explicit `Width::*` settings added — same `Style::blank()` + header-bold pattern as today). The SKILLS column slots cleanly at the right edge; cells render `✓ N` for successful counts and would render `✗ ?` on error per the existing CountOrError glyph pattern.

The `(override)` annotation from PORT-05 remains preserved via the existing `format_dir_path_column` helper, which I did not modify.

## JSON `last_sync` Emits Literal `null` for Fresh Manifests

Confirmed via smoke + by integration test `status_json_last_sync_null_for_fresh`. The field has no `skip_serializing_if` attribute, matching the stable-shape pattern already used for `unowned: []`:

```json
{
  "configured": true,
  "library_dir": "/Users/martin/dev/coding-agent-files/skills",
  "last_sync": null,
  ...
  "directories": [
    {
      "name": "antigravity",
      ...
      "skill_count": { "count": 105 },
      ...
    }
  ]
}
```

JSON consumers can rely on `last_sync` being present in every `status --json` payload, with value `null` (fresh / pre-v0.11 manifest) or RFC-3339 string (post-successful-sync).

## Test Count Delta

- **+4 manifest unit tests** (Task 1):
  - `manifest_pre_v011_json_deserializes_with_none_last_synced_at`
  - `manifest_stamp_round_trip_preserves_timestamp`
  - `manifest_default_skips_last_synced_at_in_json`
  - `manifest_last_synced_at_accessor_shape`
- **+5 integration tests** (Task 2, in `crates/tome/tests/cli_status.rs`):
  - `status_last_sync_never_for_fresh_manifest`
  - `status_last_sync_renders_after_sync`
  - `status_json_last_sync_null_for_fresh`
  - `status_json_last_sync_string_after_sync`
  - `status_skills_column_present_in_text`

**Total: +9 tests.** Matches plan target.

## Commits

- `e7b31e7` — `feat(19-03): add last_synced_at header to Manifest (OBS-07 / D-LSYNC-1)` (Task 1)
- `7803602` — `feat(19-03): stamp + surface last_sync; SKILLS column in status (OBS-07)` (Task 2)

## Quality Gates

- `cargo test -p tome` — 824 unit tests + all integration suites pass (no failures, no skips)
- `cargo clippy --all-targets -- -D warnings` — clean
- `cargo fmt -- --check` — clean
- Schema-compat smoke: pre-v0.11 manifest JSON (`{"skills": {}}`) deserializes cleanly via `serde_json::from_str::<Manifest>` (pinned by `manifest_pre_v011_json_deserializes_with_none_last_synced_at`)
- Manual smoke: `cargo run -- status` emits `Last sync: never` + 5-column SKILLS table on a real config

## Deviations from Plan

None. Two minor notes:

1. The Directories table was already 5 columns with a SKILLS header in the merged-in state of `status.rs` from Wave 1 / Plan 01 (which restructured doctor and brought adjacent status changes along). The plan described the column work as "change from 4 columns to 5" but the actual codebase already had the layout; this plan's Task 2 ensures the JSON + integration-test parity for SKILLS, plus adds the new `Last sync:` line above the Directories block. Final shape is identical to the plan's spec.
2. Test fixture maintenance: two existing `StatusReport { ... }` literals in `status.rs` unit tests (`json_status_always_includes_unowned_field`, `json_status_serializes_unowned_skill_summaries`) needed `last_sync: None` added to compile. This is mechanical fixture upkeep, not a behavioral change.

## Known Stubs

None. No hardcoded `=[]`/`=None` values flow to UI rendering — `last_sync: None` is the legitimate "never synced" state and renders as `Last sync: never` accordingly.

## Self-Check: PASSED

- FOUND: `crates/tome/src/manifest.rs` (modified, contains `last_synced_at`)
- FOUND: `crates/tome/src/status.rs` (modified, contains `last_sync`)
- FOUND: `crates/tome/src/lib.rs` (modified, line 1789 contains `manifest.stamp_last_synced_at()`)
- FOUND: `crates/tome/tests/cli_status.rs` (modified, 5 new tests)
- FOUND: `crates/tome/tests/snapshots/cli_status__status_empty_library.snap` (updated)
- FOUND: commit `e7b31e7`
- FOUND: commit `7803602`
