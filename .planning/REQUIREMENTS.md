# Requirements: tome v0.7 — Wizard Hardening

**Defined:** 2026-04-18
**Core Value:** Every AI coding tool on a developer's machine shares the same skill library without manual copying or per-tool configuration.

## v1 Requirements

Requirements for v0.7 release. Each maps to roadmap phases. The wizard code itself (WIZ-01–05) already shipped in v0.6 — this milestone closes the correctness gaps.

### Wizard Correctness

- [x] **WHARD-01**: Wizard assembles in-memory `Config`, then validates it via `Config::validate()` (or TOML round-trip) before calling `config.save()`. Invalid type/role combinations produced by the role-editing loop or custom directory flow must be rejected with a clear error, not silently written.
- [x] **WHARD-02**: Wizard detects when `library_dir` overlaps with any distribution directory (Synced or Target role) and refuses to save. Error message should suggest a non-overlapping library location.
- [x] **WHARD-03**: Wizard detects when `library_dir` is a subdirectory of a synced directory (circular symlink risk at distribute time) and surfaces this as a validation error before save.

### Wizard Test Coverage

- [x] **WHARD-04**: Pure (non-interactive) wizard helpers have unit test coverage: `find_known_directories_in`, registry lookup (`KNOWN_DIRECTORIES`), `DirectoryType::default_role`, and config assembly from a set of selected directories.
- [x] **WHARD-05**: Integration test that runs the wizard in `--dry-run` mode with `--no-input` and asserts the generated config passes `Config::validate()` and round-trips through TOML without changes.
- [x] **WHARD-06**: Test for every `(DirectoryType, DirectoryRole)` combination that could be produced by the wizard — valid combos save successfully, invalid combos error with WHARD-01's validation path.

### Wizard Display Polish

- [ ] **WHARD-07**: `show_directory_summary()` uses `tabled` instead of manual `println!` column formatting, matching the pattern in `status.rs`. Handles long paths gracefully (truncation or wrapping) without breaking the column layout.

### Documentation

- [ ] **WHARD-08**: WIZ-01 through WIZ-05 marked validated in PROJECT.md with a note that they shipped in v0.6 and were hardened in v0.7.

## v2 Requirements

Deferred to future releases.

### Registry Expansion (needs research per tool)

- **WREG-01**: Expand `KNOWN_DIRECTORIES` with Cursor global skill paths (if they exist — needs filesystem verification)
- **WREG-02**: Expand `KNOWN_DIRECTORIES` with Windsurf global skill paths
- **WREG-03**: Expand `KNOWN_DIRECTORIES` with Aider global skill paths

## Out of Scope

| Feature | Reason |
|---------|--------|
| Ground-up wizard rewrite | WIZ-01–05 already shipped in v0.6; only hardening needed |
| Interactive dialoguer flow testing | Inherent limitation of dialoguer; focus on pure helper coverage instead |
| Alternative prompt library (inquire, cliclack) | dialoguer works; migration cost not justified |
| Custom KNOWN_DIRECTORIES via config file | User can add custom entries interactively already |

## Traceability

| Requirement | Phase | Status |
|-------------|-------|--------|
| WHARD-01 | Phase 4 | Complete |
| WHARD-02 | Phase 4 | Complete |
| WHARD-03 | Phase 4 | Complete |
| WHARD-04 | Phase 5 | Complete |
| WHARD-05 | Phase 5 | Complete |
| WHARD-06 | Phase 5 | Complete |
| WHARD-07 | Phase 6 | Pending |
| WHARD-08 | Phase 6 | Pending |

**Coverage:**
- v1 requirements: 8 total
- Mapped to phases: 8 (100%)
- Unmapped: 0

---
*Requirements defined: 2026-04-18 after v0.7 milestone research*
*Traceability updated: 2026-04-18 after roadmap creation*
