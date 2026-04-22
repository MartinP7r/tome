---
status: complete
phase: 06-display-polish-docs
source: [06-01-wizard-summary-tabled-SUMMARY.md, 06-02-project-md-wiz-closure-SUMMARY.md]
started: 2026-04-21T14:18:38Z
updated: 2026-04-22T00:45:00Z
---

## Current Test

[testing complete]

## Session Notes

**Binary-version skew discovered at Test 4** — the `tome` binary on the user's
`$PATH` (`/opt/homebrew/bin/tome` → Homebrew bottle `0.6.1`, installed
2026-04-17) pre-dates both Phase 5 (`--no-input` plumbing) and Phase 6 (tabled
migration). Tests 1-3's initial reports were captured from that stale binary,
not from the Phase 6 code under verification. Tests 1 and 4 were re-verified by
running the fresh release build (`target/release/tome` v0.6.2) directly — all
binary-dependent tests now pass. Homebrew bottle update is a follow-up
release-engineering task, not a Phase 6 gap.

## Tests

### 1. Wizard summary renders as rounded-border table
expected: Run `tome init --dry-run --no-input`. Directory summary appears inside a rounded-corner box (╭ ╮ ╰ ╯) with aligned column separators, header clearly demarcated (bold in TTY, plain in pipes).
result: pass
note: |
  Initial user reports ("headers are not lined up correctly" / "still broken") were against the stale `/opt/homebrew/bin/tome` 0.6.1 binary, which predates the Phase 6 tabled migration. Re-verified by building the current branch (`cargo build --release`) and running `target/release/tome` (v0.6.2) directly — output shows:

  ```
  ╭────────────────┬────────────────┬──────────────────────┬─────────────────────╮
  │ NAME           │ TYPE           │ ROLE                 │ PATH                │
  ├────────────────┼────────────────┼──────────────────────┼─────────────────────┤
  │ antigravity    │ directory      │ Synced (skills disco │ ~/.gemini/antigravi │
  │ claude-plugins │ claude-plugins │ Managed (read-only,  │ ~/.claude/plugins   │
  │ claude-skills  │ directory      │ Synced (skills disco │ ~/.claude/skills    │
  │ codex          │ directory      │ Synced (skills disco │ ~/.codex/skills     │
  │ codex-agents   │ directory      │ Synced (skills disco │ ~/.agents/skills    │
  ╰────────────────┴────────────────┴──────────────────────┴─────────────────────╯
  ```

  Rounded corners ╭ ╮ ╰ ╯ present; header `│` dividers align with body `│` dividers; horizontal-rule `┼` junctions match column boundaries; no spurious trailing column. Header bolding not visible in piped output (expected — `console::style(…).bold()` auto-strips ANSI when stdout is not a TTY); will appear bold in interactive TTY use.

### 2. Columns are NAME / TYPE / ROLE / PATH
expected: In the same `tome init --dry-run --no-input` output, the table header row shows exactly those four columns left-to-right: NAME, TYPE, ROLE, PATH.
result: pass
note: Confirmed via Test 1 screenshot (stale binary) AND fresh Phase 6 build — column order matches NAME / TYPE / ROLE / PATH in both.

### 3. Home paths render with ~/ prefix
expected: Any PATH cell pointing under your home dir (e.g. `~/.claude`, `~/.tome`, `~/.config/…`) appears with a literal `~/` prefix — no `/Users/martin/…` absolute paths in the table.
result: pass
note: Confirmed via both user screenshots AND fresh Phase 6 build — all five rows show `~/.gemini/...`, `~/.claude/plugins`, `~/.claude/skills`, `~/.codex/skills`, `~/.agents/skills`. No absolute paths visible.

### 4. Narrow-terminal truncation keeps table aligned
expected: When terminal width is constrained, table stays inside the rounded box — rightmost/widest column (PATH or ROLE) is truncated rather than overflowing or wrapping, and column borders stay vertically aligned.
result: pass
note: |
  Verified against fresh `target/release/tome` 0.6.2 with output piped (hits the 80-column fallback branch per Plan 06-01 D-05). Truncation visible in output — ROLE cells show `Synced (skills disco` (truncated), PATH cells show `~/.gemini/antigravi` (truncated). Borders remain aligned inside the rounded box. `PriorityMax::right()` correctly shrinks the widest column first.

  Side note on test command: the `COLUMNS=60` env var does NOT affect rendering because the `terminal_size` crate queries TTY dimensions via ioctl (not env). The 80-column fallback is the deterministic CI/piped path. For true-narrow-TTY verification, user would need to physically resize an interactive terminal and re-run.

### 5. PROJECT.md has "Hardened in v0.7" subsection for WIZ-01..05
expected: Open `.planning/PROJECT.md`. Under `## Requirements` there is a `### Hardened in v0.7` subsection (placed after `### Previously Validated`) with five bullets for WIZ-01 through WIZ-05, each carrying "Shipped v0.6, hardened v0.7 (Phases 4+5)" provenance. The old `### Known Gaps (deferred from v0.6)` subsection is gone.
result: pass
note: |
  Self-verified via `rg -n '^### (Hardened in v0.7|Previously Validated|Known Gaps)|WIZ-0[1-5]' .planning/PROJECT.md`. Found:
  - L49: `### Previously Validated (re-verified in v0.7 research)`
  - L57: `### Hardened in v0.7` (correctly positioned AFTER Previously Validated)
  - L61-65: five bullets WIZ-01 through WIZ-05, each carrying `Shipped v0.6, hardened v0.7 (Phase N / WHARD-XX)` provenance
  - No `### Known Gaps (deferred from v0.6)` heading (only a retrospective decision-log table row at L116 — different context, not the removed subsection)

### 6. PROJECT.md footer dated 2026-04-21 / Phase 6 complete
expected: Last line of `.planning/PROJECT.md` reads something like `*Last updated: 2026-04-21 — Phase 6 (Display Polish & Docs) complete …*` — mentions WHARD-07 and WHARD-08.
result: pass
note: |
  Self-verified via `tail -3 .planning/PROJECT.md`. Footer reads: `*Last updated: 2026-04-21 — Phase 6 (Display Polish & Docs) complete — wizard summary migrated to `tabled` (WHARD-07); WIZ-01–05 marked validated as hardened in v0.7 (WHARD-08)*`. Both WHARD-07 and WHARD-08 cited by name.

### 7. CHANGELOG.md has v0.7 Wizard Hardening block under [Unreleased]
expected: Open `CHANGELOG.md`. Under `## [Unreleased]` there is a `### Changed — v0.7 Wizard Hardening` subsection citing both WHARD-07 (wizard summary → tabled) and WHARD-08 (PROJECT.md WIZ closure). `Cargo.toml` version is unchanged (still on the v0.6.x line — no release cut).
result: pass
note: |
  Self-verified via `rg -n '^## \[Unreleased\]|^### Changed — v0.7 Wizard Hardening|WHARD-07|WHARD-08' CHANGELOG.md` + Cargo.toml grep:
  - CHANGELOG.md L8 `## [Unreleased]`, L10 `### Changed — v0.7 Wizard Hardening`, L12 WHARD-07 (tabled migration), L13 WHARD-08 (PROJECT.md closure)
  - `Cargo.toml:version = "0.6.2"` — no v0.7.0 release-cut bump (matches the "don't bump Cargo.toml; make release handles it" workflow)

## Summary

total: 7
passed: 7
issues: 0
pending: 0
skipped: 0
blocked: 0
observations: 1

## Gaps

[Test 1 gap invalidated 2026-04-22 — see Session Notes above. The "headers not lined up" evidence was from the stale Homebrew tome 0.6.1 binary, not from Phase 6 code. Fresh `target/release/tome` 0.6.2 renders correctly-aligned tabled output.]

# Secondary observations (not tied to a test — logged for triage)

- truth: "Homebrew bottle `tome 0.6.1` is stale on the user's machine"
  status: observation
  reason: "User's `$PATH` resolves to /opt/homebrew/bin/tome which symlinks to Cellar/tome/0.6.1/bin/tome (installed 2026-04-17), pre-dating Phases 5 and 6"
  severity: minor
  test: general
  artifacts: []
  missing:
    - "Next release (v0.7.0) needs a Homebrew formula bump so `brew upgrade` picks up the wizard hardening work"
    - "For local development verification, prefer `./target/release/tome ...` over bare `tome` to avoid this class of skew"
  debug_session: ""
  hypothesis: "Release workflow is expected (`make release` bumps version and triggers cargo-dist → Homebrew). This observation is informational for the v0.7.0 cut, not a Phase 6 regression."

- truth: "`tome init --dry-run --no-input` exercises the rendering path but does not exercise the interactive-decision surface"
  status: observation
  reason: "User observed: 'dry-run seems to skip over all decisions' (from stale 0.6.1 run — may not reproduce the same way against 0.7.x)"
  severity: minor
  test: general
  artifacts: []
  missing:
    - "If UAT needs to verify interactive flow, use `tome init --dry-run` (no `--no-input`) inside a real TTY"
  debug_session: ""
  hypothesis: "`--no-input` is designed to be non-interactive (Plan 05-01). The combined `--dry-run --no-input` command exists for CI/test use. The comment may have reflected 0.6.1 behavior; observation remains for future UX polish decisions but not a Phase 6 gap."
