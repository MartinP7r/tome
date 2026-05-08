# Phase 15: CLI hardening - Discussion Log

> **Audit trail only.** Do not use as input to planning, research, or execution agents.
> Decisions are captured in CONTEXT.md — this log preserves the alternatives considered.

**Date:** 2026-05-08
**Phase:** 15-cli-hardening
**Areas discussed:** Plan grouping & sequencing, HARD-22 tilde preservation, HARD-21 browse Disable/Enable semantics, HARD-09 distribute clobber policy

---

## Plan grouping & sequencing

### Q1: How should the 22 HARD-* requirements be grouped into plans?

| Option | Description | Selected |
|--------|-------------|----------|
| Module-touch (~6 plans) | Group by which modules each HARD touches. Better parallelisation; matches v0.10 phase pattern (Phase 11: 5 plans, Phase 14: 8 plans). 6 plans, average ~3.7 reqs/plan. Waves: 15-01/02/03 in parallel; 15-04/05/06 after. | ✓ |
| Cluster-aligned (3 dense plans) | Map plans 1:1 onto ROADMAP success criteria (architecture / safety+tests / polish). 3 plans, average ~7.3 reqs/plan. Hard to parallelise within a plan. | |
| Fine-grained (one HARD per plan) | 22 small plans. Maximum parallelism, minimal merge risk per plan, but heavy planning overhead. | |
| Hybrid: foundation wave + bundled rest | First wave is the few HARDs that unblock or shape the rest (anyhow Result, lib.rs decomp, cli.rs split, config split). Subsequent waves bundle the rest by cluster. 8 plans, mixed grain. | |

**User's choice:** Module-touch (~6 plans, recommended)
**Notes:** Captured as D-PLAN-1. Plan layout: 15-01 (cli.rs decomp), 15-02 (config module), 15-03 (type-system tightening), 15-04 (safety guards + integration tests), 15-05 (browse UI), 15-06 (polish + older bugs).

### Q2: Beta-cut scope discipline for Phase 15: what happens if execution surfaces NEW issues touching the same modules?

| Option | Description | Selected |
|--------|-------------|----------|
| Strict | HARD-01..22 only. Anything new — even trivial — goes to Phase 16/17 or backlog. Predictable beta cut; protects v0.10 ship date. | ✓ |
| Relaxed: same-module-trivial folds in | If a discovery is in the same file as a planned HARD and is <1hr work, fold it in. Closes more issues but risks scope creep. | |
| Park-lot: triage at end of phase | Track new discoveries in a per-phase `deferred-items.md`. Don't fold mid-phase; promote to Phase 16/17/backlog at /gsd:verify-work time. | |

**User's choice:** Strict (recommended)
**Notes:** Captured as D-PLAN-2. Operational mechanism: per-phase `15-deferred-items.md` parking lot for discoveries; triage at /gsd:verify-work time.

---

## HARD-22 tilde preservation

### Q3: What's the tilde-preservation rule for `Config::save_checked` and mutating commands (HARD-22)?

| Option | Description | Selected |
|--------|-------------|----------|
| Preserve user input verbatim | If the user wrote `~/skills`, save emits `~/skills`. If they wrote `/Users/martin/skills`, save emits that. No coercion either way. | |
| Normalise to ~-shape on save | If a path is under `$HOME`, save emits `~/...`. Cross-machine portable by default but rewrites paths the user wrote as absolute. New `unexpand_tilde()` helper before serialise. | ✓ |
| Raw-text round-trip via toml_edit | Use `toml_edit` (preserves comments/formatting/whitespace) so save only rewrites fields that actually changed. New dependency; higher cost but byte-perfect. | |

**User's choice:** Normalise to ~-shape on save (auto-portable)
**Notes:** Captured as D-TILDE-1. The user prioritised cross-machine portability over verbatim preservation. Trade-off accepted: literal absolute paths under `$HOME` get rewritten to `~`-shape.

### Q4: Where should the `~`-normalise rule apply? (machine.toml is per-machine, NOT committed to git.)

| Option | Description | Selected |
|--------|-------------|----------|
| tome.toml only | Apply normalisation only to `tome.toml`. machine.toml paths preserve user input verbatim. | ✓ |
| Both files (uniform) | Apply normalisation to every TOML path field. Simpler mental model but rewrites machine-specific absolute paths the user wrote intentionally. | |

**User's choice:** tome.toml only (recommended)
**Notes:** Captured as D-TILDE-2. The portable/per-machine split mirrors v0.9's PORT-01..05 reasoning — `machine.toml` is per-machine; portability concerns don't apply there.

---

## HARD-21 browse Disable/Enable semantics

### Q5: When the user presses Disable/Enable in `tome browse`, which machine.toml surface should the action mutate?

| Option | Description | Selected |
|--------|-------------|----------|
| Global `disabled` set | Toggle `MachinePrefs.disabled`. Simplest semantics. | |
| Per-directory blocklist | Toggle the parent directory's `disabled` blocklist. More precise but conflicts with `enabled` allowlist if set (MACH-04 mutual-exclusion). | |
| Smart: per-directory if list exists, else global | If the parent directory has a `disabled` blocklist or `enabled` allowlist set, mutate that. Otherwise mutate the global `disabled`. | ✓ |

**User's choice:** Smart routing — with the explicit caveat that "the UI should clearly indicate whether we're toggling a directory setting"
**Notes:** Captured as D-BROWSE-1 (smart routing) + D-BROWSE-2 (UI must show scope explicitly via context-sensitive label). The user's caveat closed the hidden-state UX gap that smart routing introduces.

### Q6: Confirmation UX for the toggle?

| Option | Description | Selected |
|--------|-------------|----------|
| Instant toggle + status message | Single keystroke applies; TUI shows a `StatusMessage::Success(...)` per v0.9 POLISH-02 pattern. Reversible. | ✓ |
| Confirmation prompt | Render a modal `dialoguer`-style prompt. Adds safety friction; redundant for a fully-reversible action. | |
| Instant for Disable, confirm for Enable | Asymmetric: enabling reactivates a previously-rejected skill. | |

**User's choice:** Instant toggle + status message (recommended)
**Notes:** Captured as D-BROWSE-3. Mirrors the existing `CopyPath` action (instant + status message).

---

## HARD-09 distribute clobber policy

### Q7: When `distribute` finds a pre-existing symlink at the target path that points OUTSIDE the current library, what should happen?

| Option | Description | Selected |
|--------|-------------|----------|
| Warn-and-skip + reuse `force` | Default: warn-and-skip; existing `force: bool` param overrides. Mirrors regular-file handling at line 121-127 and SAFE-01 pattern. | ✓ |
| Hard error | Distribute fails the entire command on first foreign symlink. Loud and unambiguous but violates SAFE-01. | |
| New `--force-distribute` flag | Add a NEW dedicated flag rather than reusing `force`. Cleaner naming but increases CLI surface. | |

**User's choice:** Warn-and-skip + reuse `force` (recommended)
**Notes:** Captured as D-DIST-1. No new CLI flag; reusing `force` is consistent with its existing semantic ("overwrite stale symlinks").

### Q8: Should `tome doctor` ALSO surface foreign symlinks as a persistent diagnostic?

| Option | Description | Selected |
|--------|-------------|----------|
| Doctor surfaces too | Add `DiagnosticIssue::ForeignSymlink { ... }` to `DoctorReport`. Renders as Warning; counts toward `total_issues`. | ✓ |
| Sync warning only | Surface only at sync time. Smaller blast radius but lost in scrollback on long syncs. | |

**User's choice:** Doctor surfaces too (recommended)
**Notes:** Captured as D-DIST-2. Persistent visibility mirrors Phase 13 D-12 ("doctor reports drift unconditionally") philosophy.

---

## Claude's Discretion

The planner decides these implementation details (recommendations recorded in CONTEXT.md `<decisions>` § "Claude's Discretion"):

- **HARD-02:** `cmd_<name>` helpers inline in `lib.rs` first; lift to `commands/` module if `lib.rs` >1,500 LOC after decomposition.
- **HARD-03:** Tilde helpers stay in `paths.rs` (cross-cutting); `config/overrides.rs` for `apply_machine_overrides`; `config/validate.rs` for `validate()` Cases A/B/C.
- **HARD-13:** Per-command split (`cli_sync.rs`, `cli_doctor.rs`, `cli_remove.rs`, `cli_reassign.rs`, `cli_status.rs`, `cli_browse.rs`, `cli_init.rs`, `cli_migrate_library.rs`); shared helpers in `tests/common/mod.rs`.
- **HARD-12:** Snapshot density covers status dashboard, skill list, detail pane, help overlay, empty state, search-filter state, theme variants.
- **HARD-14:** Disable git signing in test repos via local config (`git config commit.gpgsign false`); per-test setup helper.
- **HARD-19:** Snapshot filesystem state in `reassign::plan` return value via a new `pre_state: PreReassignState` struct; `execute` consumes the snapshot.
- **HARD-08:** Real fs (matches existing test style; no mock layer).
- **HARD-04:** `LintFailed` inline in `lint.rs` as a sibling type.
- **HARD-07:** `LogLevel` inline in `cli.rs` (CLI-facing enum).
- **HARD-17:** Reuse `validate_identifier` validation for `TryFrom<String>` failure mode.
- **D-DIST-1 detail:** Use `std::fs::canonicalize` for symlink target resolution.
- **D-BROWSE-2 label:** Short, scannable copy ("Disable on this machine" vs "Disable for `<dir>`").

## Deferred Ideas

(See CONTEXT.md `<deferred>` section for the full list.)
