# Roadmap: tome

## Milestones

- ✅ **v0.6 Unified Directory Model** — Phases 1-3 (shipped 2026-04-16) — [archive](milestones/v0.6-ROADMAP.md)
- 🚧 **v0.7 Wizard Hardening** — Phases 4-6 (active since 2026-04-18)

## Phases

<details>
<summary>✅ v0.6 Unified Directory Model (Phases 1-3) — SHIPPED 2026-04-16</summary>

- [x] Phase 1: Unified Directory Foundation (3/5 plans) — config type system, pipeline rewrite, state schema
- [x] Phase 2: Git Sources & Selection (4/4 plans) — git clone/update, per-dir filtering, tome remove
- [x] Phase 3: Import, Reassignment & Browse Polish (2/2 plans) — tome add/reassign/fork, browse TUI polish

**Known gaps:** WIZ-01 through WIZ-05 (wizard rewrite) deferred — old wizard code still functional.

</details>

### v0.7 Wizard Hardening

- [ ] **Phase 4: Wizard Correctness** — Wizard rejects invalid configs and circular library paths before save
- [ ] **Phase 5: Wizard Test Coverage** — Pure helpers and config assembly have unit + integration test coverage
- [ ] **Phase 6: Display Polish & Docs** — Summary table uses `tabled`; PROJECT.md validates WIZ-01–05 shipped/hardened

## Phase Details

### Phase 4: Wizard Correctness
**Goal**: Wizard cannot save a config that would fail at sync time
**Depends on**: Phase 3 (v0.6 unified directory model shipped)
**Requirements**: WHARD-01, WHARD-02, WHARD-03
**Success Criteria** (what must be TRUE):
  1. User running `tome init` with an invalid type/role combo (e.g., Git + Target) sees a clear validation error and the config is not written to disk
  2. User who picks a `library_dir` that overlaps a Synced/Target directory sees an error suggesting a non-overlapping location and the config is not written
  3. User who picks a `library_dir` that is a subdirectory of a synced directory sees a circular-symlink validation error before save
  4. A successful `tome init` still round-trips: the written config passes `Config::validate()` and reloads without changes
**Plans**: 3 plans
- [x] 04-01-validate-error-template-PLAN.md — Upgrade existing `Config::validate()` errors to the D-10 Conflict+Why+Suggestion template
- [ ] 04-02-library-overlap-validation-PLAN.md — Add Cases A/B/C overlap detection between `library_dir` and distribution dirs (WHARD-02/03)
- [ ] 04-03-wizard-save-hardening-PLAN.md — Wizard save path calls `Config::save_checked` (expand → validate → round-trip → write); dry-run branch validates too (WHARD-01)

### Phase 5: Wizard Test Coverage
**Goal**: Wizard logic is testable without a TTY and has enforced coverage of valid/invalid combinations
**Depends on**: Phase 4
**Requirements**: WHARD-04, WHARD-05, WHARD-06
**Success Criteria** (what must be TRUE):
  1. `cargo test` exercises unit tests for `find_known_directories_in`, `KNOWN_DIRECTORIES` registry lookup, `DirectoryType::default_role`, and pure config-assembly helpers
  2. An integration test runs `tome init --dry-run --no-input` and asserts the generated config passes validation and round-trips through TOML unchanged
  3. Every `(DirectoryType, DirectoryRole)` combination the wizard can produce has a test: valid combos save successfully; invalid combos are rejected by the Phase 4 validation path
  4. CI (ubuntu + macos) passes with the new tests as non-optional gates
**Plans**: TBD

### Phase 6: Display Polish & Docs
**Goal**: Wizard summary renders cleanly on any terminal width and v0.7 scope is marked complete in project docs
**Depends on**: Phase 5
**Requirements**: WHARD-07, WHARD-08
**Success Criteria** (what must be TRUE):
  1. User running `tome init` sees the directory summary rendered via `tabled` with `Style::rounded()`, matching the visual language of `tome status`
  2. Long paths (e.g., git repo clones under `~/.tome/repos/<sha>/`) render without breaking column alignment — either truncated or wrapped, never overflowing
  3. `PROJECT.md` lists WIZ-01 through WIZ-05 as validated with a note that they shipped in v0.6 and were hardened in v0.7
**Plans**: TBD
**UI hint**: yes

## Progress

| Phase | Milestone | Plans Complete | Status | Completed |
|-------|-----------|----------------|--------|-----------|
| 1. Unified Directory Foundation | v0.6 | 3/5 | Complete | 2026-04-14 |
| 2. Git Sources & Selection | v0.6 | 4/4 | Complete | 2026-04-15 |
| 3. Import, Reassignment & Browse Polish | v0.6 | 2/2 | Complete | 2026-04-16 |
| 4. Wizard Correctness | v0.7 | 0/3 | Planned | — |
| 5. Wizard Test Coverage | v0.7 | 0/0 | Not started | — |
| 6. Display Polish & Docs | v0.7 | 0/0 | Not started | — |
