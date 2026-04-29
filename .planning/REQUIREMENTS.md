# v0.9 Requirements: Cross-Machine Config Portability & Polish

## Milestone Goals

A single `tome.toml` checked into dotfiles can be applied across machines with different filesystem layouts — without manual edits per machine. Bundled with two polish backlogs (#462, #463) to clear the v0.8 post-merge review tail in one cut.

**Scope anchor:** Epic [#458](https://github.com/MartinP7r/tome/issues/458) (primary) + #462 (test/wording/dead-code) + #463 (type-design + TUI architecture).

## v1 Requirements

### Cross-Machine Portability (PORT)

Source: [#458](https://github.com/MartinP7r/tome/issues/458). Mechanism: new `[directory_overrides.<name>]` table in `machine.toml`.

- [x] **PORT-01**: User can declare `[directory_overrides.<name>]` blocks in `~/.config/tome/machine.toml` (or per-tome-home equivalent) to remap a directory's `path` field on this machine, without editing the synced `tome.toml`.
- [x] **PORT-02**: Per-machine overrides apply at config load time (after tilde expansion, before `Config::validate`), so all downstream code (`sync`, `status`, `doctor`, `lockfile::generate`) operates on the merged result.
- [x] **PORT-03**: An override that targets a directory name not present in `tome.toml` produces a stderr `warning:` line (typo guard) without aborting load.
- [x] **PORT-04**: Validation failures triggered by an override (e.g., overridden path now overlaps `library_dir`) surface as a distinct error class so the user knows to fix `machine.toml`, not `tome.toml`.
- [x] **PORT-05**: `tome status` and `tome doctor` indicate which directory entries are subject to a per-machine override, so the user can answer "why is this path different on this machine?" without diffing files.

### Type-Design + TUI Architecture Polish (POLISH)

Source: [#463](https://github.com/MartinP7r/tome/issues/463). Each item closes a specific D# from the post-merge review.

- [x] **POLISH-01** (D1): `tome browse` `open` action surfaces an "Opening: <path>..." status message immediately before blocking on `xdg-open`/`open`, and any keystrokes that arrived during the block are drained from the tty buffer afterward instead of replayed as actions.
- [x] **POLISH-02** (D2): `StatusMessage` is a single enum (`Success(String) | Warning(String)`) with accessor methods (`body()`, `glyph()`, `severity()`) — pre-formatted glyph in `text` is gone. UI formats `"{glyph} {body}"` at render time. Visibility narrowed to `pub(super)`. Test-only `Clone`/`PartialEq`/`Eq` derives audited.
- [x] **POLISH-03** (D3): `ClipboardOccupied` errors auto-retry once with a 100ms backoff before the warning reaches the status bar.
- [x] **POLISH-04** (D4): `FailureKind::ALL` cannot drift from the enum — either compile-enforced (e.g., `strum::EnumIter` or exhaustive-match sentinel) or replaced by an iteration mechanism that doesn't require manual sync.
- [x] **POLISH-05** (D5): `RemoveFailure::new` either gains a real invariant (`debug_assert!(path.is_absolute(), ...)`) or is removed in favor of struct-literal construction at the four call sites in `execute`.
- [x] **POLISH-06** (D6): `arboard` is pinned to a patch-version range with a documented "review changelog on bump" policy in `Cargo.toml` (or a `#[cfg(test)]` enum-growth canary).

### Test Coverage + Wording + Dead Code (TEST)

Source: [#462](https://github.com/MartinP7r/tome/issues/462). Each item closes a specific P# from the post-merge review.

- [x] **TEST-01** (P1): `remove_partial_failure_exits_nonzero_with_warning_marker` asserts the `✓ Removed directory` success banner is **absent** from stdout on partial failure (not just that the `⚠` block is present).
- [x] **TEST-02** (P2): End-to-end test pins the I2/I3 retention contract: partial failure → user fixes the underlying condition → second `tome remove <name>` succeeds with empty `failures`, config entry gone, manifest empty, library dir gone.
- [x] **TEST-03** (P3): `ViewSource .status()` match is factored into a `status_message_from_open_result(...)` helper, with unit tests covering all three arms (Ok+success, Ok+non-zero exit, Err) using synthetic `ExitStatus` values via `ExitStatusExt::from_raw`.
- [x] **TEST-04** (P4): `regen_warnings` no longer print **before** the success banner on the happy path. Either deferred until after the banner (option a) or scoped with a `[lockfile regen]` prefix (option b) — pin the choice in code and regression-test the ordering.
- [x] **TEST-05** (P5): The dead `SkillMoveEntry.source_path` field is either removed (option a) or wired into `copy_library` / `recreate_target_symlinks` (option b) — `#[allow(dead_code)]` is gone from `relocate.rs`.

## Future Requirements

Deferred to a later milestone:

- **Linux runtime UAT** — 2 carry-over items from v0.8's `08-HUMAN-UAT.md` (clipboard runtime, `xdg-open` runtime). Not blocking v0.9; will resolve when the user has Linux desktop hardware.
- **Pre-existing flake** — `backup::tests::push_and_pull_roundtrip` intermittent failure in full suite. Out of scope for v0.9 unless it surfaces during polish work.
- **`KNOWN_DIRECTORIES` registry expansion** — adding Cursor / Windsurf / Aider entries. Not driven by user request yet.
- **Path templates / variable expansion** (`${HOME}/...`, `${WORK_ROOT}/...`) — explicitly rejected in #458 in favor of `[directory_overrides.<name>]`. Re-evaluate post-v0.9 if override semantics prove insufficient.

## Out of Scope

Explicit exclusions for v0.9:

- **Multiple named configs** (`tome.macos.toml`, `tome.linux.toml`) — rejected in #458 in favor of single-file + machine.toml overrides.
- **Migration tooling** for users adopting overrides — single-user constraint; documented manually if schema changes.
- **Connector trait abstraction** (#192) — unified directory model already solves config flexibility.
- **Watch mode** (#59) — low priority; orthogonal to portability.
- **Format transforms / rule syncing** (#57, #193, #194) — different concern entirely.

## Traceability

Phase mapping is filled by `/gsd:plan-phase` after roadmap creation.

| Requirement | Phase | Status |
|-------------|-------|--------|
| PORT-01 | Phase 9 | Complete |
| PORT-02 | Phase 9 | Complete |
| PORT-03 | Phase 9 | Complete |
| PORT-04 | Phase 9 | Complete |
| PORT-05 | Phase 9 | Complete |
| POLISH-01 | Phase 10 | Complete |
| POLISH-02 | Phase 10 | Complete |
| POLISH-03 | Phase 10 | Complete |
| POLISH-04 | Phase 10 | Complete |
| POLISH-05 | Phase 10 | Complete |
| POLISH-06 | Phase 10 | Complete |
| TEST-01 | Phase 10 | Complete |
| TEST-02 | Phase 10 | Complete |
| TEST-03 | Phase 10 | Complete |
| TEST-04 | Phase 10 | Complete |
| TEST-05 | Phase 10 | Complete |
