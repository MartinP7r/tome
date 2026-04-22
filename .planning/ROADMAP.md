# Roadmap: tome

## Milestones

- ✅ **v0.6 Unified Directory Model** — Phases 1-3 (shipped 2026-04-16) — [archive](milestones/v0.6-ROADMAP.md)
- ✅ **v0.7 Wizard Hardening** — Phases 4-6 (shipped 2026-04-22) — [archive](milestones/v0.7-ROADMAP.md)
- 📋 **v0.8** — TBD (run `/gsd:new-milestone` to plan)

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

### 📋 v0.8 (Not yet planned)

Run `/gsd:new-milestone` to scope the next milestone. Candidate themes tracked in `.planning/PROJECT.md` (Active Requirements section).

## Progress

| Phase | Milestone | Plans Complete | Status | Completed |
|-------|-----------|----------------|--------|-----------|
| 1. Unified Directory Foundation | v0.6 | 3/5 | Complete | 2026-04-14 |
| 2. Git Sources & Selection | v0.6 | 4/4 | Complete | 2026-04-15 |
| 3. Import, Reassignment & Browse Polish | v0.6 | 2/2 | Complete | 2026-04-16 |
| 4. Wizard Correctness | v0.7 | 3/3 | Complete | 2026-04-19 |
| 5. Wizard Test Coverage | v0.7 | 4/4 | Complete | 2026-04-20 |
| 6. Display Polish & Docs | v0.7 | 2/2 | Complete | 2026-04-22 |
