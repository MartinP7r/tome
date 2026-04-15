# Roadmap: tome v0.6 — Unified Directory Model

## Overview

tome v0.6 replaces the artificial source/target config split with a unified directory model, then layers on git-based skill sources and quality-of-life CLI features. Three phases: an atomic foundation rewrite, additive feature work unlocked by the new model, and polish/convenience tooling.

## Phases

**Phase Numbering:**
- Integer phases (1, 2, 3): Planned milestone work
- Decimal phases (2.1, 2.2): Urgent insertions (marked with INSERTED)

Decimal phases appear between their surrounding integers in numeric order.

- [ ] **Phase 1: Unified Directory Foundation** - Replace source/target config with unified directories, rewrite wizard, adapt pipeline and state schema
- [ ] **Phase 2: Git Sources & Selection** - Git-backed skill repos, per-directory skill selection, tome remove
- [ ] **Phase 3: Import, Reassignment & Browse Polish** - GitHub standalone imports, source reassignment, browse TUI improvements

## Phase Details

### Phase 1: Unified Directory Foundation
**Goal**: Users configure tome with a single unified `[directories.*]` model instead of separate sources and targets, and the full sync pipeline works against the new config
**Depends on**: Nothing (first phase)
**Requirements**: CFG-01, CFG-02, CFG-03, CFG-04, CFG-05, CFG-06, PIPE-01, PIPE-02, PIPE-03, PIPE-04, PIPE-05, WIZ-01, WIZ-02, WIZ-03, WIZ-04, WIZ-05, MACH-01, STATE-01, STATE-02, STATE-03
**Success Criteria** (what must be TRUE):
  1. User can write a `tome.toml` with `[directories.*]` entries (path, type, role) and `tome sync` completes successfully
  2. Running `tome init` presents auto-discovered directories with roles from a merged registry, and writes valid unified config
  3. Old-format `tome.toml` files fail to parse with a clear error message (not silent empty config)
  4. `tome status` and `tome doctor` report directories with their roles instead of separate source/target sections
  5. Cleanup with an empty directories map does not delete the library (safety guard fires)
**Plans:** 3/5 plans executed

Plans:
- [x] 01-01-PLAN.md — Config type system (DirectoryName, DirectoryType, DirectoryRole, DirectoryConfig, Config rewrite)
- [x] 01-02-PLAN.md — Pipeline core (discover, library, distribute, cleanup)
- [x] 01-03-PLAN.md — State schema (manifest, lockfile, machine, status, doctor)
- [ ] 01-04-PLAN.md — Wizard rewrite (KNOWN_DIRECTORIES, auto-role, summary table)
- [ ] 01-05-PLAN.md — Integration wiring (lib.rs sync, remaining modules, integration tests, CHANGELOG)

### Phase 2: Git Sources & Selection
**Goal**: Users can add remote git repos as skill sources and control which skills reach which directories on a per-machine basis
**Depends on**: Phase 1
**Requirements**: GIT-01, GIT-02, GIT-03, GIT-04, GIT-05, GIT-06, GIT-07, GIT-08, MACH-02, MACH-03, MACH-04, MACH-05, CLI-01
**Success Criteria** (what must be TRUE):
  1. User can add a `type = "git"` directory with a repo URL, and `tome sync` clones the repo and discovers its skills
  2. Subsequent `tome sync` fetches updates from the remote without re-cloning; pinned refs (branch/tag/SHA) are respected
  3. User can set per-directory `disabled` or `enabled` skill lists in `machine.toml` and only the appropriate skills reach that directory
  4. `tome remove <name>` deletes a directory entry from config and cleans up its library artifacts and symlinks
  5. Failed git operations (network down, bad URL) fall back gracefully without aborting sync of local directories
**Plans:** 4 plans

Plans:
- [ ] 02-01-PLAN.md — Git module, config subdir field, TomePaths repos_dir
- [ ] 02-02-PLAN.md — Per-directory skill filtering in machine.toml
- [ ] 02-03-PLAN.md — Git resolution wiring in sync pipeline + distribute filtering
- [ ] 02-04-PLAN.md — tome remove command + integration tests

### Phase 3: Import, Reassignment & Browse Polish
**Goal**: Users can import standalone skills from GitHub, reassign skill provenance, and enjoy a polished browse experience
**Depends on**: Phase 2
**Requirements**: CLI-02, CLI-03, BROWSE-01, BROWSE-02, BROWSE-03, BROWSE-04
**Success Criteria** (what must be TRUE):
  1. `tome add <github-url>` creates a git directory entry in config from a GitHub URL
  2. `tome reassign <skill> --to <dir>` changes which directory owns a skill
  3. Browse TUI has theming, fuzzy match highlighting, scrollbar indicators, and markdown rendering in the preview panel
**Plans**: TBD
**UI hint**: yes

## Progress

**Execution Order:**
Phases execute in numeric order: 1 -> 2 -> 3

| Phase | Plans Complete | Status | Completed |
|-------|----------------|--------|-----------|
| 1. Unified Directory Foundation | 2/5 | In Progress|  |
| 2. Git Sources & Selection | 1/4 | In Progress | - |
| 3. Import, Reassignment & Browse Polish | 0/? | Not started | - |
