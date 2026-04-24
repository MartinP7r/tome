# Roadmap: tome

## Milestones

- ✅ **v0.6 Unified Directory Model** — Phases 1-3 (shipped 2026-04-16) — [archive](milestones/v0.6-ROADMAP.md)
- ✅ **v0.7 Wizard Hardening** — Phases 4-6 (shipped 2026-04-22) — [archive](milestones/v0.7-ROADMAP.md)
- 🚧 **v0.8 Wizard UX & Safety Hardening** — Phases 7-8 (active since 2026-04-23) — epic [#459](https://github.com/MartinP7r/tome/issues/459)

## Phases

<details>
<summary>✅ v0.6 Unified Directory Model (Phases 1-3) — SHIPPED 2026-04-16</summary>

- [x] Phase 1: Unified Directory Foundation (3/5 plans) — config type system, pipeline rewrite, state schema
- [x] Phase 2: Git Sources & Selection (4/4 plans) — git clone/update, per-dir filtering, tome remove
- [x] Phase 3: Import, Reassignment & Browse Polish (2/2 plans) — tome add/reassign/fork, browse TUI polish

**Known gaps:** WIZ-01 through WIZ-05 (wizard rewrite) deferred — closed as "hardened" in v0.7.

</details>

<details>
<summary>✅ v0.7 Wizard Hardening (Phases 4-6) — SHIPPED 2026-04-22</summary>

- [x] Phase 4: Wizard Correctness (3/3 plans) — `Config::validate()` Conflict+Why+Suggestion errors, library↔distribution overlap detection (Cases A/B/C), `Config::save_checked` expand→validate→round-trip→write pipeline (WHARD-01/02/03)
- [x] Phase 5: Wizard Test Coverage (4/4 plans) — `--no-input` plumbing + `assemble_config` helper extraction, pure-helper unit tests, `tome init --dry-run --no-input` integration tests, 12-combo `(DirectoryType, DirectoryRole)` matrix (WHARD-04/05/06)
- [x] Phase 6: Display Polish & Docs (2/2 plans) — wizard summary migrated to `tabled::Table` with `Style::rounded()` + `PriorityMax::right()` truncation, PROJECT.md "Hardened in v0.7" subsection, CHANGELOG WHARD-07/08 entries (WHARD-07/08)

**Closed WIZ-01..05:** v0.6's known wizard gaps are now shipped AND hardened.

</details>

### v0.8 Wizard UX & Safety Hardening

- [x] **Phase 7: Wizard UX (Greenfield / Brownfield / Legacy)** — `tome init` handles new machines, existing configs, and pre-v0.6 cruft without surprises; resolved `tome_home` is surfaced up-front and optionally persisted via XDG config *(completed 2026-04-23)*
- [ ] **Phase 8: Safety Refactors (Partial-Failure Visibility & Cross-Platform)** — destructive commands surface partial failures; browse UI's external actions work on Linux; silent `fs::read_link(..).ok()` sites are replaced with surfaced warnings

## Phase Details

### Phase 7: Wizard UX (Greenfield / Brownfield / Legacy)
**Goal**: `tome init` behaves predictably on any machine state — fresh install, dotfiles-synced home, or pre-v0.6 cruft — and tells the user which `tome_home` it is about to populate
**Depends on**: Phase 6 (v0.7 Wizard Hardening shipped — `Config::save_checked` and `--no-input` plumbing are prerequisites)
**Requirements**: WUX-01, WUX-02, WUX-03, WUX-04, WUX-05
**Success Criteria** (what must be TRUE):
  1. User running `tome init` on a greenfield machine (no `TOME_HOME`, no XDG config, no existing `.tome/tome.toml`) sees a prompt to choose `tome_home` with `~/.tome/` as the default and a custom-path option that is validated before the wizard proceeds
  2. User running `tome init` on a brownfield machine (existing `tome.toml` at the resolved `tome_home`) sees a summary of the detected config (directory count, library_dir, last-modified date) and can choose **use existing** (default), **edit existing**, **reinitialize** (with backup), or **cancel** — no path silently overwrites a valid config
  3. User with a legacy pre-v0.6 `~/.config/tome/config.toml` (contains `[[sources]]` or `[targets.*]`) sees a warning that the file is ignored by current tome and is offered a delete-or-move-aside action — no silent ignore, no auto-delete
  4. Every `tome init` invocation prints a 1-line "resolved tome_home: <path>" info message before Step 1 prompts, so the user can abort immediately if the wrong path is about to be populated
  5. When the user selects a custom `tome_home` in the greenfield flow, wizard offers to persist the choice by writing `~/.config/tome/config.toml` with a `tome_home = "..."` field; subsequent `tome sync` / `tome status` invocations find it without `TOME_HOME` env var
**Plans**: 4 plans
  - [x] 07-01-wux-04-resolved-tome-home-info-PLAN.md — print resolved tome_home + source label at start of tome init (WUX-04)
  - [x] 07-02-wux-03-legacy-config-detection-PLAN.md — MachineState + has_legacy_sections + legacy cleanup handler (WUX-03)
  - [x] 07-03-wux-01-05-tome-home-prompt-PLAN.md — Step 0 greenfield tome_home prompt + XDG persist (WUX-01, WUX-05)
  - [x] 07-04-wux-02-brownfield-decision-PLAN.md — 4-way brownfield decision + prefill plumbing (WUX-02)
**UI hint**: yes

### Phase 8: Safety Refactors (Partial-Failure Visibility & Cross-Platform)
**Goal**: Destructive commands cannot report success while partial cleanup failed; browse UI's external actions work on Linux; silent `.ok()` drops on symlink reads are replaced with surfaced warnings
**Depends on**: Phase 7 (independent changesets, but keeping linear ordering simplifies branch strategy and release cut)
**Requirements**: SAFE-01, SAFE-02, SAFE-03
**Success Criteria** (what must be TRUE):
  1. User running `tome remove <name>` in a state where some symlinks/dirs cannot be cleaned (permissions, missing files) sees a distinct "⚠ N operations failed" summary with per-item detail and the command exits non-zero — the clean success path remains quiet as before
  2. User on Linux pressing the `open` action in `tome browse` has the skill opened via `xdg-open` (and `copy path` via `wl-copy`/`xclip` or an equivalent cross-platform clipboard crate); any failure appears in the TUI status bar instead of being silently discarded by `let _ = ...`
  3. User running `tome relocate` (or any command transiting the patched `fs::read_link(..).ok()` sites) sees a stderr warning when a symlink cannot be read, with enough context (path + error) to diagnose — the command no longer silently records "no provenance" on such failures
  4. `cargo test` covers the new `RemoveResult` aggregation (including a partial-failure case) and the Linux-path branches of the browse action dispatcher (under `#[cfg(target_os = "linux")]` or via platform-agnostic abstractions)
**Plans**: 3 plans
  - [x] 08-01-safe-01-remove-partial-failure-aggregation-PLAN.md — RemoveResult aggregates per-loop FailureKind records; lib.rs Command::Remove surfaces grouped '⚠ K operations failed' summary + exits non-zero (SAFE-01 / #413)
  - [ ] 08-02-safe-02-browse-cross-platform-status-bar-PLAN.md — arboard clipboard + cfg!-dispatched open/xdg-open + App.status_message rendered in browse status bar (SAFE-02 / #414)
  - [ ] 08-03-safe-03-relocate-read-link-warning-PLAN.md — relocate.rs:93 explicit match + eprintln warning mirroring PR #448 pattern (SAFE-03 / #449)

## Progress

| Phase | Milestone | Plans Complete | Status | Completed |
|-------|-----------|----------------|--------|-----------|
| 1. Unified Directory Foundation | v0.6 | 3/5 | Complete | 2026-04-14 |
| 2. Git Sources & Selection | v0.6 | 4/4 | Complete | 2026-04-15 |
| 3. Import, Reassignment & Browse Polish | v0.6 | 2/2 | Complete | 2026-04-16 |
| 4. Wizard Correctness | v0.7 | 3/3 | Complete | 2026-04-19 |
| 5. Wizard Test Coverage | v0.7 | 4/4 | Complete | 2026-04-20 |
| 6. Display Polish & Docs | v0.7 | 2/2 | Complete | 2026-04-22 |
| 7. Wizard UX (Greenfield / Brownfield / Legacy) | v0.8 | 4/4 | Complete | 2026-04-23 |
| 8. Safety Refactors (Partial-Failure Visibility & Cross-Platform) | v0.8 | 0/3 | Planned | — |
