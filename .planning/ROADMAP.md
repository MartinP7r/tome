# Roadmap: tome

## Milestones

- ✅ **v0.6 Unified Directory Model** — Phases 1-3 (shipped 2026-04-16) — [archive](milestones/v0.6-ROADMAP.md)
- ✅ **v0.7 Wizard Hardening** — Phases 4-6 (shipped 2026-04-22) — [archive](milestones/v0.7-ROADMAP.md)
- ✅ **v0.8 Wizard UX & Safety Hardening** — Phases 7-8 + 8.1 hotfix (shipped 2026-04-27) — [archive](milestones/v0.8-ROADMAP.md)
- 🚧 **v0.9 Cross-Machine Config Portability & Polish** (active since 2026-04-28) — epic [#458](https://github.com/MartinP7r/tome/issues/458) + #462 + #463
- 📋 **v1.0 tome Desktop (Tauri GUI)** — drafted — see [milestones/v1.0-REQUIREMENTS.md](milestones/v1.0-REQUIREMENTS.md) and [milestones/v1.0-ROADMAP.md](milestones/v1.0-ROADMAP.md). Sequenced after v0.9 by default; ratify via `/gsd:new-milestone` when v0.9 ships.

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

<details>
<summary>✅ v0.8 Wizard UX & Safety Hardening (Phases 7-8 + 8.1) — SHIPPED 2026-04-27</summary>

- [x] Phase 7: Wizard UX — Greenfield / Brownfield / Legacy (4/4 plans) — `tome init` handles new machines, existing configs, and pre-v0.6 cruft without surprises; resolved `tome_home` surfaced up-front and optionally persisted via XDG config (WUX-01/02/03/04/05)
- [x] Phase 8: Safety Refactors — Partial-Failure Visibility & Cross-Platform (3/3 plans) — `tome remove` aggregates partial-cleanup failures with non-zero exit, `tome browse` works on Linux via `xdg-open` + `arboard`, silent `read_link().ok()` drops replaced with stderr warnings (SAFE-01/02/03)
- [x] Phase 8.1: v0.8.1 hotfix — lockfile regen + save chain (3/3 plans) — `resolved_paths_from_lockfile_cache` helper restores git-skill provenance after Remove/Reassign/Fork (H1), `Command::Remove` save chain reordered to surface partial-failure ⚠ block before save errors (H2), failure-summary wording reworded (H3)

**Released as:** v0.8.0 (2026-04-26) + v0.8.1 hotfix (2026-04-27)
**Carry-over:** 2 Linux-runtime UAT items in `08-HUMAN-UAT.md` (clipboard / xdg-open) — accepted as carry-over pending Linux desktop hardware

</details>

### v0.9 Cross-Machine Config Portability & Polish (Active)

Epic: [#458](https://github.com/MartinP7r/tome/issues/458) — `machine.toml` path overrides for cross-machine portability. Bundled with #462 (test/wording/dead-code polish) and #463 (type-design + TUI architecture polish) to clear the v0.8 post-merge review tail in one cut.

- [x] **Phase 9: Cross-Machine Path Overrides** — `[directory_overrides.<name>]` in `machine.toml` lets a single `tome.toml` work across machines with different filesystem layouts (PORT-01..05) (completed 2026-04-28)
- [ ] **Phase 10: Phase 8 Review Tail — Type Design, TUI Polish & Test Coverage** — close the 11 post-merge review items from #462 + #463 (POLISH-01..06, TEST-01..05)

### v1.0 tome Desktop — Tauri GUI (Drafted)

Forward-planning artifacts:
- [`milestones/v1.0-REQUIREMENTS.md`](milestones/v1.0-REQUIREMENTS.md) — 32 requirements across 7 categories (CORE / VIEW / SYNC / CFG / OPS / BAK / DIST) plus 5 cross-cutting NF gates.
- [`milestones/v1.0-ROADMAP.md`](milestones/v1.0-ROADMAP.md) — 7 phases (10–16) with three intermediate cuts (alpha after 11, beta after 13, rc after 15, v1.0 after 16). Rough size: 15–22 weeks of focused work.

**Sequencing:** v0.9 → v1.0 by default (D-GUI-09). v0.9 hardens `machine.toml` semantics that v1.0 leans on. Swap allowed; parallelism not recommended.

**Framework:** Tauri 2 (D-GUI-01). Reuses Rust crate as native backend; no N-API. ~8 MB bundle vs Electron's ~150 MB; built-in code-signed auto-update; same Developer ID flow as the CLI.

Phases will be planned via `/gsd:new-milestone` when v1.0 becomes active. Phase numbering assumes v0.9 takes Phases 9–10; renumber if v0.9's phase footprint differs.

## Phase Details

### Phase 9: Cross-Machine Path Overrides
**Goal**: A single `tome.toml` checked into dotfiles can be applied across machines with different filesystem layouts via per-machine `[directory_overrides.<name>]` blocks in `machine.toml`.
**Depends on**: Phase 8.1 (v0.8.1 baseline — load pipeline + machine.toml schema stable)
**Requirements**: PORT-01, PORT-02, PORT-03, PORT-04, PORT-05
**Success Criteria** (what must be TRUE):
  1. User can add `[directory_overrides.<name>]` blocks to `machine.toml` and a subsequent `tome sync` / `tome status` operates on the overridden `path` for that directory without any edits to the synced `tome.toml`.
  2. Override application happens once at config load time (after tilde expansion, before `Config::validate`), so every downstream command (`sync`, `status`, `doctor`, `lockfile::generate`) sees the same merged result — no second code path can observe pre-override paths.
  3. An override targeting a directory name that doesn't exist in `tome.toml` produces a single stderr `warning:` line naming the typo and continues loading; it does not abort the command.
  4. A validation failure caused by an override (e.g., overridden path overlaps `library_dir`) surfaces with a distinct error class that names `machine.toml` as the file to edit, not `tome.toml`.
  5. `tome status` and `tome doctor` mark each overridden directory entry visibly (e.g., `(override)` annotation or dedicated column) so the user can answer "why is this path different on this machine?" without diffing files.
**Plans**: TBD

### Phase 10: Phase 8 Review Tail — Type Design, TUI Polish & Test Coverage
**Goal**: Close the 11 post-merge review items from #462 (P1-P5) and #463 (D1-D6) so the v0.8 review tail is fully cleared in one cut.
**Depends on**: Phase 9 (sequential — keeps PORT delivery clean and avoids interleaving review-tail churn with the portability epic)
**Requirements**: POLISH-01, POLISH-02, POLISH-03, POLISH-04, POLISH-05, POLISH-06, TEST-01, TEST-02, TEST-03, TEST-04, TEST-05
**Success Criteria** (what must be TRUE):
  1. `tome browse` `open` shows an "Opening: <path>..." status before blocking on `xdg-open`/`open`, any keystrokes typed during the block are drained instead of replayed, and `ClipboardOccupied` errors are auto-retried once with a 100ms backoff before any warning reaches the status bar.
  2. `StatusMessage` is a single `Success(String) | Warning(String)` enum with `body()`/`glyph()`/`severity()` accessors, `pub(super)` visibility, and audited test-only derives — pre-formatted glyphs in `text` are gone and `ViewSource .status()` routes through a tested `status_message_from_open_result(...)` helper covering Ok+success, Ok+non-zero exit, and Err arms.
  3. `FailureKind::ALL` cannot drift from the enum (compile-enforced via `EnumIter` or equivalent), `RemoveFailure::new` either carries a real `debug_assert!` invariant or is replaced by struct-literal construction at the four call sites, and `arboard` is pinned to a patch range with a documented bump-review policy in `Cargo.toml`.
  4. `regen_warnings` ordering on the happy path is pinned in code (deferred until after the success banner OR scoped with a `[lockfile regen]` prefix) and a regression test fails if the order regresses; the dead `SkillMoveEntry.source_path` field is either removed or wired into `copy_library`/`recreate_target_symlinks`, and `#[allow(dead_code)]` is gone from `relocate.rs`.
  5. `remove_partial_failure_exits_nonzero_with_warning_marker` asserts the `✓ Removed directory` success banner is **absent** from stdout on partial failure, and an end-to-end test pins the I2/I3 retention contract: partial failure → user fixes the underlying condition → second `tome remove <name>` succeeds with empty `failures`, config entry gone, manifest empty, library dir gone.
**Plans**: TBD
**UI hint**: yes

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
| 8. Safety Refactors (Partial-Failure Visibility & Cross-Platform) | v0.8 | 3/3 | Complete | 2026-04-24 |
| 8.1. v0.8.1 hotfix — lockfile regen + save chain | v0.8 | 3/3 | Complete | 2026-04-27 |
| 9. Cross-Machine Path Overrides | v0.9 | 2/3 | Complete    | 2026-04-28 |
| 10. Phase 8 Review Tail — Type Design, TUI Polish & Test Coverage | v0.9 | 1/3 | In Progress|  |
