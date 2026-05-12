# Roadmap: tome

## Milestones

- ✅ **v0.6 Unified Directory Model** — Phases 1-3 (shipped 2026-04-16) — [archive](milestones/v0.6-ROADMAP.md)
- ✅ **v0.7 Wizard Hardening** — Phases 4-6 (shipped 2026-04-22) — [archive](milestones/v0.7-ROADMAP.md)
- ✅ **v0.8 Wizard UX & Safety Hardening** — Phases 7-8 + 8.1 hotfix (shipped 2026-04-27) — [archive](milestones/v0.8-ROADMAP.md)
- ✅ **v0.9 Cross-Machine Config Portability & Polish** — Phases 9-10 (shipped 2026-04-29) — [archive](milestones/v0.9-ROADMAP.md)
- ✅ **v0.10 Library-canonical Model + Cross-Machine Plugin Reconciliation** — Phases 11-17 (shipped 2026-05-11) — closes epic [#459](https://github.com/MartinP7r/tome/issues/459) — [archive](milestones/v0.10-ROADMAP.md)
- 📋 **v0.11** — Polish + observability (next milestone) — to be defined via `/gsd:new-milestone`
- 📋 **v1.0 tome Desktop (Tauri GUI)** — drafted, deferred to after v0.11 — see [milestones/v1.0-REQUIREMENTS.md](milestones/v1.0-REQUIREMENTS.md) and [milestones/v1.0-ROADMAP.md](milestones/v1.0-ROADMAP.md)

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

<details>
<summary>✅ v0.9 Cross-Machine Config Portability & Polish (Phases 9-10) — SHIPPED 2026-04-29</summary>

- [x] Phase 9: Cross-Machine Path Overrides (3/3 plans) — `[directory_overrides.<name>]` schema in `machine.toml`, override-apply timing in load pipeline, typo warning, distinct `machine.toml` error class, `(override)` annotation in `tome status`/`tome doctor` text+JSON (PORT-01..05)
- [x] Phase 10: Phase 8 Review Tail (3/3 plans) — `StatusMessage` enum redesign, `status_message_from_open_result` helper, "Opening: ..." pre-block UX, `ClipboardOccupied` retry, `FailureKind::ALL` compile-enforcement, `RemoveFailure::new` invariant, `arboard` patch-pin, deferred regen-warnings, banner-absence + retry e2e tests, dead `source_path` removal (POLISH-01..06 + TEST-01..05)

**Released as:** v0.9.0 (2026-04-29). Includes the bare-slug `tome add` improvement (PR #471) bundled in.

</details>

<details>
<summary>✅ v0.10 Library-canonical Model + Cross-Machine Plugin Reconciliation (Phases 11-17) — SHIPPED 2026-05-11</summary>

**Cuts:** Phase 13 = alpha · Phase 15 = beta · Phase 16 = rc · Phase 17 = v0.10 final

- [x] Phase 11: Library-canonical core (5/5 plans, LIB-01..05) — completed 2026-05-03
- [x] Phase 12: Marketplace adapter (4/4 plans, ADP-01..04) — completed 2026-05-05
- [x] Phase 13: Lockfile-authoritative sync (5/5 plans, RECON-01..05) — completed 2026-05-05 — **alpha cut**
- [x] Phase 14: Unowned-library lifecycle (8/8 plans, UNOWN-01..03) — completed 2026-05-07
- [x] Phase 15: CLI hardening (6/6 plans, HARD-01..22) — completed 2026-05-08 — **beta cut**
- [x] Phase 16: Cleanup-message UX + docs (5/5 plans, UX-01..02 + DOC-01..03) — completed 2026-05-08 — **rc cut**
- [x] Phase 17: Migration polish + UAT + release (operational, REL-01..05) — completed 2026-05-12 — **v0.10 final** (cargo-dist tag 578f787, GitHub Release 11 assets)

Full archive: [milestones/v0.10-ROADMAP.md](milestones/v0.10-ROADMAP.md). Closes epic [#459](https://github.com/MartinP7r/tome/issues/459).

</details>

### 📋 v0.11 (Next milestone — to be defined)

Run `/gsd:new-milestone` to scope. Likely candidates:
- #530 doctor "auto-fixable" UX bug
- #511 timing flake under parallel test contention
- "57 managed symlink(s) tracked in git" doctor false-positive
- `make release` should stamp CHANGELOG date automatically
- Wizard polish (#453, #454, #456)
- Items from Phase 11/12/13 review followup bundles (#517, #518, #519)


## Progress

**Execution Order:**
Phases execute in numeric order: 11 → 12 → 13 (alpha) → 14 → 15 (beta) → 16 (rc) → 17 (v0.10 final)

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
| 9. Cross-Machine Path Overrides | v0.9 | 3/3 | Complete | 2026-04-28 |
| 10. Phase 8 Review Tail — Type Design, TUI Polish & Test Coverage | v0.9 | 3/3 | Complete | 2026-04-29 |
| 11. Library-canonical core | v0.10 | 5/5 | Complete    | 2026-05-03 |
| 12. Marketplace adapter | v0.10 | 4/4 | Complete    | 2026-05-05 |
| 13. Lockfile-authoritative sync (alpha) | v0.10 | 5/5 | Complete    | 2026-05-05 |
| 14. Unowned-library lifecycle | v0.10 | 8/8 | Complete    | 2026-05-07 |
| 15. CLI hardening (beta) | v0.10 | 6/6 | Complete    | 2026-05-08 |
| 16. Cleanup-message UX + docs (rc) | v0.10 | 5/5 | Complete    | 2026-05-08 |
| 17. Migration polish + UAT + release (v0.10 final) | v0.10 | 5/5 | Complete    | 2026-05-12 |
