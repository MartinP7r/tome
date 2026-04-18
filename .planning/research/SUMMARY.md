# Project Research Summary

**Project:** tome v0.7 — Wizard Hardening
**Domain:** Interactive CLI setup wizard (Rust/dialoguer)
**Researched:** 2026-04-16
**Confidence:** HIGH

## Executive Summary

The wizard rewrite that was planned for v0.7 as WIZ-01–05 is already complete. All five requirements were implemented and verified during the v0.6 phase 01-04 work: the unified `KNOWN_DIRECTORIES` registry, auto-discovery with role auto-assignment, summary table with name/path/type/role columns, custom directory addition with type-constrained role selection, and elimination of `find_source_target_overlaps()`. The v0.7 scope is therefore not a rewrite — it is hardening of a working implementation.

Four correctness gaps remain that need closing before the wizard can be considered production-quality. The most dangerous is that the wizard bypasses the deserialization-layer validation in `config.rs`, meaning an invalid type/role combination (e.g., `Git + Target`) can be saved to disk without error and only surfaces as a confusing failure at the next `tome sync`. The second critical gap is missing circular path detection: a `Synced` directory overlapping the library directory will cause `distribute.rs` to create self-referential symlinks, producing non-idempotent syncs and potential disk fill. Both must be caught at the summary-before-save step in the wizard.

The remaining gaps are lower severity but still blocking for a "hardened" milestone: zero unit test coverage on wizard logic outside of registry invariants (interactive flow cannot be tested via dialoguer, so the fix is extracting config assembly into a pure testable function), and the summary table using manual `format!` padding that truncates long paths. The `tabled` crate is already a compiled dependency used in `status.rs` and `lib.rs` — wiring it into `wizard.rs` is a one-import fix. No new dependencies are needed for any v0.7 work.

## Key Findings

### Recommended Stack

No Cargo.toml changes required. Every capability the wizard needs is already declared and compiled into the binary. The work is strictly code-level: importing `tabled` into `wizard.rs` (one `use` statement), extracting pure helper functions for testability, and adding validation calls before `config.save()`.

**Core technologies (existing, no changes):**
- `dialoguer` 0.12.0 — interactive prompts — already used throughout wizard; evaluated feature flags (`fuzzy-select`, `history`, `completion`) and rejected all as unnecessary
- `tabled` 0.20.0 — ASCII table rendering — already used in `status.rs:185-191` and `lib.rs:1128-1134`; import into `wizard.rs` with `Style::rounded()` to match existing visual language
- `console` 0.16.3 — terminal colors and sizing — already used in wizard for headers and dividers; keep as-is
- `dirs` 6.x — home directory detection — already used by `find_known_directories_in()`

### Expected Features

The core wizard flow is fully shipped. The v0.7 feature scope is polish and correctness.

**Must have (table stakes for "Hardened" milestone):**
- Config validation before save — invalid type/role combos must be caught in the wizard, not at `tome sync` time
- Circular path detection — library_dir overlap with Synced directories must be rejected before save
- Testable config assembly — pure function path for config construction that can be unit-tested without a TTY
- `tabled` summary table — path truncation in the current manual format produces visually broken output

**Should have (polish, no blocking dependency):**
- Post-init next-steps message ("Run `tome sync` to populate your library") — standard CLI wizard pattern
- Custom path existence warning — warn before saving a path that does not exist
- Existing config detection — warn on re-init instead of silently overwriting
- Empty directory selection guard — `MultiSelect` returning zero selections should warn, not silently produce a useless config
- Role recommendation hints in picker UI — `default_role()` exists but is not surfaced

**Defer to v0.8+:**
- Registry expansion (Cursor, Windsurf, Aider entries) — needs filesystem verification per tool
- Skill count per directory in summary table — data is available at discovery step but UX benefit is marginal
- Case-insensitive duplicate detection for custom directory names — edge case, `check_convention()` already warns

### Architecture Approach

The wizard is a linear dialoguer-based flow with eight sequential steps: directory selection, pre-scan discovery, library path selection, exclusion picking, summary + role editing, custom directory addition, save confirmation, and optional git backup. The architecture is complete and correct. What it lacks is a separation between the interactive layer (dialoguer calls, untestable) and the logic layer (config assembly, validation, table formatting — all pure functions that are testable).

**Components and their hardening status:**
1. `KNOWN_DIRECTORIES` const — complete, 11 entries, data-only expansion path
2. `find_known_directories_in()` — complete, uses `std::fs::metadata()` correctly
3. `configure_directories()` / `configure_library()` / `configure_exclusions()` — complete, interactive layer
4. `show_directory_summary()` — complete logic, broken presentation (manual format strings)
5. Config assembly (inline in `run()`) — complete but no validation call before save
6. Circular path detection — missing; must be added before summary step
7. Test coverage on layers 2-4 — missing; extraction into pure functions is the prerequisite

### Critical Pitfalls

1. **Config validation bypass** — wizard builds `DirectoryConfig` structs directly, skipping the `Deserialize` validation that catches invalid type/role combos. Prevention: call `Config::validate()` or round-trip through `toml::from_str(toml::to_string_pretty(&config)?)` before `config.save()`. Must land in the wizard hardening PR.

2. **Circular symlinks from Synced + library overlap** — if `library_dir` is inside or is a parent of a `Synced` directory, `distribute.rs` creates self-referential symlinks. Prevention: validate that `library_dir` does not overlap any directory where `is_distribution() == true` before the summary step. Port the existing manifest-based detection in `distribute.rs`.

3. **Zero test coverage on wizard logic** — `run()` mixes dialoguer calls with config assembly, making it untestable. Prevention: extract config assembly into a pure function (`assemble_config(selected_dirs, library_path, exclusions) -> Config`) and a validation function (`validate_wizard_config(config) -> Result<(), Vec<Warning>>`). Test those exhaustively; keep the dialoguer layer thin.

4. **Summary table path truncation** — fixed-width `{:<35}` format breaks on git source paths and long home directories. Prevention: replace `show_directory_summary()` internals with `tabled::Table` using `Style::rounded()`. Zero compile cost, one import.

5. **BTreeMap resolution order vs. display order mismatch** — the wizard displays directories in `KNOWN_DIRECTORIES` array order, but `discover_all()` resolves skill name conflicts by BTreeMap key order (alphabetical). Users see "claude-skills" listed first but "amp" may win the dedup. Prevention: sort the summary table alphabetically to match actual resolution order; add a note about first-alphabetically-wins behavior.

## Implications for Roadmap

Based on research, v0.7 work groups naturally into two phases: a correctness phase that eliminates silent failures, and a polish phase for UX improvements. The correctness phase has hard ordering requirements (validation must be in place before any polish work ships, to avoid masking bugs with prettier output).

### Phase 1: Correctness — Close the Four Hard Gaps

**Rationale:** The most dangerous bugs (validation bypass, circular detection, zero tests) must be closed first. Polish on top of broken correctness is wasted effort and can mask failures.
**Delivers:** A wizard that cannot save an invalid config, cannot create circular symlinks, has a testable core, and renders its summary table without truncation.
**Addresses:** FEATURES.md P1 (tabled table), P7 (integration tests); PITFALLS.md Pitfall 1, 2, 6, 8
**Implementation order within phase:**
1. Extract `assemble_config()` pure function + unit tests (prerequisite for everything else)
2. Add `validate_wizard_config()` with type/role combo checks + circular path detection
3. Wire validation call before `config.save()`
4. Replace `show_directory_summary()` with `tabled::Table`

**Avoids:** Pitfall 1 (validation bypass), Pitfall 2 (circular symlinks), Pitfall 6 (test gap), Pitfall 8 (truncation)

### Phase 2: Polish — UX Improvements on a Correct Base

**Rationale:** Once correctness is guaranteed by tests, polish items are safe to add without risk of masking regressions.
**Delivers:** Post-init next-steps message, custom path existence warning, existing config detection, empty selection guard, role recommendation hints in picker.
**Addresses:** FEATURES.md P2, P3, P4, P5; PITFALLS.md Pitfall 4, 9, 11
**Note:** These are all standalone; none block each other. Ship as one PR or as individual small PRs.

**Avoids:** Pitfall 4 (empty selection), Pitfall 9 (no mention of `tome add` for git repos), Pitfall 11 (confusing "no editable directories" message)

### Phase Ordering Rationale

- Phase 1 must precede Phase 2 because polish on top of a broken validation path creates a false sense of quality
- Within Phase 1, testable core extraction comes first because it makes validation and circular detection testable as you add them
- Phase 2 items have no internal dependencies and can be sequenced opportunistically
- Registry expansion (Cursor, Windsurf) is explicitly out of scope for v0.7 — needs filesystem verification per tool, separate research task

### Research Flags

Phases with well-documented patterns (skip `/gsd:research-phase`):
- **Phase 1:** All patterns are established in the codebase. `tabled` usage: copy from `status.rs:185-191`. Validation round-trip: standard Rust pattern. Circular detection: port from existing `distribute.rs` logic.
- **Phase 2:** All items are one-function additions. No novel patterns.

No phases need deeper external research. This is entirely an internal code quality milestone.

## Confidence Assessment

| Area | Confidence | Notes |
|------|------------|-------|
| Stack | HIGH | Verified against `Cargo.toml`, existing usages in `status.rs` and `lib.rs`, and dialoguer 0.12.0 feature list |
| Features | HIGH | Based on direct code review of `wizard.rs` (603 lines) and `config.rs`; cross-checked with phase 01-04 SUMMARY and VERIFICATION |
| Architecture | HIGH | All findings from direct source analysis; no inference required |
| Pitfalls | HIGH | Code-informed: each pitfall traces to a specific file and line range |

**Overall confidence:** HIGH

### Gaps to Address

- **Registry expansion candidates** (Cursor, Windsurf, Aider): LOW confidence on whether these tools have canonical global skills directories. Needs per-tool filesystem verification before any entries are added to `KNOWN_DIRECTORIES`. Not v0.7 scope.
- **`Config::validate()` existence**: Research noted the validation lives in the `Deserialize` impl, not a standalone method. The implementation approach (round-trip vs. dedicated method) should be decided during Phase 1 execution — either works, dedicated method is cleaner for testing.
- **Dry-run serialization parity** (Pitfall 10): The dry-run path calls `toml::to_string_pretty()` directly while `config.save()` may format differently. Low severity but worth fixing as part of the `Config::to_toml_string()` extraction. Belongs in Phase 1 if touched during validation work.

## Sources

### Primary (HIGH confidence)
- `crates/tome/src/wizard.rs` (603 lines) — full implementation analysis
- `crates/tome/src/config.rs` — DirectoryType, DirectoryRole, valid_roles(), deserialization validation (lines 344-370)
- `crates/tome/src/status.rs:185-191` — verified `tabled` usage pattern
- `crates/tome/src/lib.rs:1128-1134` — verified `tabled` usage pattern
- `.planning/phases/01-unified-directory-foundation/01-04-SUMMARY.md` — WIZ-01–05 completion evidence
- `.planning/phases/01-unified-directory-foundation/01-VERIFICATION.md` — WIZ-01–05 all SATISFIED

### Secondary (MEDIUM confidence)
- [dialoguer 0.12.0 docs](https://docs.rs/dialoguer/0.12.0/dialoguer/) — feature flags, MultiSelect behavior, no mock/TTY support
- [tabled 0.20.0 docs](https://docs.rs/tabled/0.20.0/tabled/) — Table::from_iter, Style::rounded API
- [ESLint CLI init pattern](https://eslint.org/docs/latest/use/command-line-interface) — post-init next-steps message convention
- [Comparison of Rust CLI Prompts](https://fadeevab.com/comparison-of-rust-cli-prompts/) — alternative library evaluation

---
*Research completed: 2026-04-16*
*Ready for roadmap: yes*
