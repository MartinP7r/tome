---
phase: 19-doctor-status-surface-bugfix-bundle
plan: 05
subsystem: wizard
tags: [fix, wizard, tabled, ansi, regression-test, administrative-close]
status: complete
wave: 2
requirements: [FIX-04]
github_issues: [454]
dependency_graph:
  requires: [Plan 01 — doctor substrate (independent in practice; Wave 2 parallel)]
  provides: [render_directory_summary_table testable helper, FIX-04 regression guard test]
  affects: [crates/tome/src/wizard.rs]
tech_stack:
  added: []
  patterns: [reproduce-first verification gate, TDD RED→GREEN]
key_files:
  created: []
  modified:
    - crates/tome/src/wizard.rs
decisions:
  - "D-FIX04-1 skipped: reproduction confirmed bug does NOT manifest in current code; no strip-ansi-escapes dep added (would be redundant with already-enabled `tabled[ansi]` feature)"
  - "D-FIX04-2 shipped: snapshot test pins header/body column alignment under ANSI-bold cells, guarding against future removal of the tabled `ansi` feature"
  - "#454 queued for administrative close in Plan 07 wrap-up, referencing commit 0803afb as the actual fix and Plan 05 snapshot test as the regression guard"
metrics:
  duration: "~4 minutes"
  completed_date: "2026-05-13"
  tasks_planned: 2
  tasks_completed: 2
  commits: 2
  files_modified: 1
  files_created: 0
  tests_added: 1
  tests_passing: 821 (lib) + 40 (wizard module)
---

# Phase 19 Plan 05: Wizard ANSI-aware width Summary

ANSI width alignment in the wizard summary table closed administratively for #454: reproduce-first verification proved the `tabled = { features = ["ansi"] }` fix from commit `0803afb` (April 2026) is sufficient; a snapshot test now pins the alignment behaviour so a future removal of that feature flag is caught immediately.

## What Shipped

A single behaviour change and one regression test:

1. **`show_directory_summary` refactored** to delegate table construction to a new `render_directory_summary_table(directories, term_cols) -> String` helper. The public-print side (`eprintln!`) stays in `show_directory_summary`; the helper is `pub(super)`-style (module-private fn) and exists solely so the regression test can capture the rendered output deterministically at a fixed terminal width.
2. **New unit test** `wizard::tests::show_directory_summary_aligns_header_with_body_under_ansi`: forces `console::set_colors_enabled(true)` so the bold header cells actually emit `\x1b[1m...\x1b[0m`, renders the table at `term_cols = 120`, strips ANSI escapes from each line, and asserts every `│` divider in the header row sits at the same visible column index as in body rows.
3. **In-source FIX-04 reference comment** above `show_directory_summary` documenting the administrative-close path (Plan 05 reproduce-first → bug does not manifest → snapshot test pins it → #454 closes administratively in Plan 07's CHANGELOG sweep).

No `strip-ansi-escapes` dep added; no behavioural change to user-visible output.

## Path Decision (Task 1 Reproduce-First)

**Outcome: Path 2B — administrative close + snapshot test only.**

### Reproduction method

- Path A from the plan (`script(1)` on macOS with `FORCE_COLOR=1` / `CLICOLOR_FORCE=1`).
- Greenfield TempDir, fresh `TOME_HOME`, `tome init --dry-run --no-input`.
- Terminal: `xterm-ghostty` (executor host).

### Sample output

```
╭────────────────┬────────────────┬────────────────────────┬───────────────────╮
│ NAME           │ TYPE           │ ROLE                   │ PATH              │
├────────────────┼────────────────┼────────────────────────┼───────────────────┤
│ claude-plugins │ claude-plugins │ Managed (read-only, ow │ ~/.claude/plugins │
╰────────────────┴────────────────┴────────────────────────┴───────────────────╯
```

(ANSI escapes around `NAME`/`TYPE`/`ROLE`/`PATH` headers omitted from the snippet above for readability — the captured raw stream includes `\x1b[1m` wraps on header cells.)

### Pipe-position measurement

After stripping CSI escapes:

| Row    | `│` positions (visible chars) | Visible length |
| ------ | ----------------------------- | -------------- |
| Header | `[0, 17, 34, 59, 79]`         | `81`           |
| Body   | `[0, 17, 34, 59, 79]`         | `81`           |

Byte-identical divider positions. Bug does NOT reproduce — `tabled[ansi]` from commit `0803afb` is doing its job.

### Why D-FIX04-1 was skipped

Per RESEARCH risk #1 + plan guidance ("If the bug does NOT reproduce, the issue is administrative. Do NOT add `strip-ansi-escapes` unnecessarily — it's redundant with `tabled[ansi]`."), adding the dep + `strip_str` call would be:

- 1 extra workspace dependency for zero functional improvement
- 1 extra import + ~3 LOC churn in `wizard.rs`
- A silent loss of bold styling on cells (the strip-then-feed-to-tabled approach swallows the ANSI escapes that produce the bold)

The snapshot test alone gives us the regression guard the planner wanted, at zero cost.

## Snapshot test pin shape

The assertion uses a hand-rolled ANSI-stripper (CSI escape pattern: `ESC [` … letter) and compares `char_indices()` positions of `│` across header and body lines:

```rust
fn visible_pipe_positions(line: &str) -> Vec<usize> {
    // strip CSI escapes, then return positions of `│` in the visible string
    // ...
    out.char_indices().filter(|(_, c)| *c == '│').map(|(i, _)| i).collect()
}

let header_pipes = visible_pipe_positions(data_lines[0]);
for body_line in &data_lines[1..] {
    let body_pipes = visible_pipe_positions(body_line);
    assert_eq!(header_pipes, body_pipes, ...);
}
```

`char_indices()` is used rather than `match_indices('│').map(|(i,_)| i)` because `│` is a multi-byte UTF-8 character (3 bytes) — byte indices and character indices diverge after the first divider. The test's invariant is visual column equality, not byte-offset equality.

The test does NOT depend on `insta::assert_snapshot!`. The plan suggested it as one option, but a structural assertion (same divider positions across rows) is more durable against incidental table-shape changes (column widths, content padding) while still failing fast if alignment regresses.

### Why this catches a future regression

If someone removes `features = ["ansi"]` from `Cargo.toml`, `tabled` reverts to byte-counting widths. The bold header cells (8 non-printing bytes each: `\x1b[1m` + content + `\x1b[0m`) would inflate the header-row width calc by ~8 bytes per cell. Body rows have no ANSI escapes, so their width calc stays at the actual content length. tabled then picks the wider value (header) as the column width — but pads body cells to the bytecount-inflated header width, NOT the visible-character header width. Visible column dividers drift apart. The snapshot test fails because `header_pipes != body_pipes`.

## Commits

| Step  | Commit    | Description                                                                                              |
| ----- | --------- | -------------------------------------------------------------------------------------------------------- |
| RED   | `8e8f0bc` | `test(19-05): add failing test for FIX-04 ANSI alignment snapshot`                                       |
| GREEN | `1dbd5c7` | `feat(19-05): extract render_directory_summary_table helper (FIX-04)`                                    |

No REFACTOR step needed — the GREEN diff is already minimal (extract helper + comment, no behavioural change in user output).

## Verification

| Check                                                                              | Result                                            |
| ---------------------------------------------------------------------------------- | ------------------------------------------------- |
| `rg 'features = \["ansi"\]' Cargo.toml`                                            | 1 match (pre-existing fix still present)          |
| `rg "show_directory_summary_aligns_header_with_body_under_ansi" crates/tome/src/wizard.rs` | 1 match (snapshot test ships)                     |
| `rg "FIX-04 \(#454\) reference" crates/tome/src/wizard.rs`                         | 1 match (administrative-close comment)            |
| `rg "strip-ansi-escapes" Cargo.toml`                                               | 0 matches (Path 2B — dep correctly skipped)       |
| `cargo test -p tome --lib wizard::tests::show_directory_summary_aligns_header_with_body_under_ansi` | PASS                                              |
| `cargo test -p tome --lib`                                                         | 821 passed; 0 failed (was 820; +1 from this plan) |
| `cargo clippy --all-targets -- -D warnings`                                        | clean                                             |
| `cargo fmt -- --check`                                                             | clean                                             |
| Manual smoke (re-run Task 1 reproduction)                                          | columns already align, no functional change       |

## Deviations from Plan

None — the plan explicitly anticipated both Path 2A and Path 2B outcomes. Path 2B was followed verbatim:

- Step 1 (confirm pre-existing fix present): ✓
- Step 2 (reproduce in greenfield TempDir via `script(1)`): ✓
- Step 3 (inspect output for misalignment): bug DOES NOT reproduce
- Step 4 (decision): Path 2B
- Task 2 Path 2B: dep skipped, snapshot test added, admin-close comment added

The plan's recommendation to "use `pub(crate) fn render_directory_summary_table`" was implemented as a module-private `fn` (no visibility modifier) since the test lives in the same module — `pub(crate)` would be wider than necessary.

## Follow-up Actions

### For Plan 07 (CHANGELOG and Phase verification)

Add a CHANGELOG entry under `[Unreleased]` along the lines of:

```markdown
- **FIX-04** Wizard summary table column-alignment: verified the
  `tabled = { features = ["ansi"] }` fix from commit 0803afb is sufficient;
  added a regression-guard snapshot test pinning header/body divider
  alignment. Closes #454 administratively.
```

### For GitHub #454 close action

Comment + close:

```
v0.11 Plan 19-05 re-verified the bug under reproduction:
`tome init --no-input --dry-run` in a forced-TTY (script(1) + FORCE_COLOR=1)
greenfield TempDir renders the summary table with header and body column
dividers at byte-identical positions. The fix from commit
[0803afb](https://github.com/MartinP7r/tome/commit/0803afb) (tabled `ansi`
feature) is doing its job.

A regression-guard snapshot test now pins the alignment behaviour:
`wizard::tests::show_directory_summary_aligns_header_with_body_under_ansi`
in `crates/tome/src/wizard.rs`.

Closing administratively. If anyone hits a misalignment in a different
terminal configuration, please re-open with terminal type + raw output —
the snapshot test gives us a fast bisection target.
```

This close should happen during Plan 07's CHANGELOG sweep (or via `gh issue close 454 --comment` if Plan 07's scope drifts).

### No further code work

The fix is in place; the regression guard is in place. No additional code work needed in v0.11 for FIX-04.

## Self-Check: PASSED

All required artifacts present:
- ✓ `crates/tome/src/wizard.rs` exists and contains `render_directory_summary_table`
- ✓ Snapshot test `show_directory_summary_aligns_header_with_body_under_ansi` shipped
- ✓ FIX-04 administrative-close comment shipped
- ✓ Commit `8e8f0bc` (RED) exists
- ✓ Commit `1dbd5c7` (GREEN) exists
- ✓ No `strip-ansi-escapes` added (Path 2B correctness)
- ✓ All 821 lib tests pass; clippy clean; fmt clean
