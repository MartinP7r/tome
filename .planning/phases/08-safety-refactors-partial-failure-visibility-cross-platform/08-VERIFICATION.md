---
phase: 08-safety-refactors-partial-failure-visibility-cross-platform
verified: 2026-04-24T00:00:00Z
status: human_needed
score: 4/4 must-haves verified (automated); 1 Linux-specific behavior pending human verification
re_verification: null
human_verification:
  - test: "Run `tome browse` on a Linux machine (Ubuntu/Fedora/Arch), open any skill's Detail view, and press the `copy path` action"
    expected: "`arboard::Clipboard::new()` succeeds via x11/wayland backend and the status bar renders `✓ Copied: <path>` in `theme.accent`. Pasting into a terminal produces the copied path."
    why_human: "macOS CI cannot exercise the Linux clipboard provider branches inside `arboard`; no mock or trait abstraction was introduced (per CONTEXT D-17/D-19). Compile-time coverage via `ubuntu-latest` CI matrix confirms the code links, but runtime behavior of the x11/wayland backends is only observable on an actual Linux desktop session."
  - test: "On the same Linux machine, press the `open` action in `tome browse` Detail view"
    expected: "`xdg-open <path>` is invoked via `std::process::Command::new(\"xdg-open\")`; the system's default handler opens the skill directory/file. Status bar shows `✓ Opened: <path>`. On headless SSH (no display server) the command surfaces `⚠ Could not open: <error>` in `theme.alert`."
    why_human: "The `cfg!(target_os = \"macos\")` dispatch selects `xdg-open` at compile time on Linux, but the spawn outcome depends on whether `xdg-utils` is installed AND the user has an active DISPLAY/WAYLAND_DISPLAY. Not verifiable on macOS CI or dev machine."
---

# Phase 8: Safety Refactors (Partial-Failure Visibility & Cross-Platform) Verification Report

**Phase Goal:** "Destructive commands cannot report success while partial cleanup failed; browse UI's external actions work on Linux; silent `.ok()` drops on symlink reads are replaced with surfaced warnings"
**Verified:** 2026-04-24
**Status:** human_needed (all automated checks pass; 2 Linux-specific behavioral tests require hands-on verification on a Linux desktop)
**Re-verification:** No — initial verification

## Goal Achievement

### Observable Truths

| # | Truth | Status | Evidence |
| - | ----- | ------ | -------- |
| 1 | `tome remove <name>` in a partial-failure state prints a distinct `⚠ N operations failed` summary with per-item detail and exits non-zero; the clean success path remains quiet as before | ✓ VERIFIED | `lib.rs:422-456` surfacing block; `remove.rs:47-77` types; 1 unit + 1 integration test pass; success printlnchanged only for the `git_cache_removed` suffix. |
| 2 | On Linux, `tome browse` `open` action uses `xdg-open`; `copy path` uses `arboard`; any failure appears in the TUI status bar instead of being silently discarded | ? UNCERTAIN (automated) / ✓ VERIFIED (compile-time) | `app.rs:215-253` `cfg!(target_os = "macos")` dispatch + `arboard::Clipboard::new()`; `ui.rs:314-370` status-bar render; macOS CI can only exercise macOS branch at runtime — Linux clipboard/xdg-open runtime behavior flagged for human verification. |
| 3 | `tome relocate` emits a stderr warning naming the path + error when a managed-skill symlink cannot be read; no longer silently records "no provenance" | ✓ VERIFIED | `relocate.rs:89-108` explicit match; canonical PR #448 format (`warning: could not read symlink at {}: {e}`); 1 unit test engineers chmod 0o000 + asserts `source_path.is_none()`. |
| 4 | `cargo test` covers the new `RemoveResult` aggregation (including partial-failure case) and the browse action dispatcher | ✓ VERIFIED | `remove::tests::partial_failure_aggregates_symlink_error` (unit); `remove_partial_failure_exits_nonzero_with_warning_marker` (integration); `browse::app::tests::status_message_set_by_copy_path_and_cleared_by_any_key` (unit); `relocate::tests::read_link_failure_records_no_provenance` (unit). All 4 pass. |

**Score:** 4/4 truths VERIFIED automatically; truth #2 has 2 follow-up items flagged for human verification on Linux hardware.

### Required Artifacts

| Artifact | Expected | Status | Details |
| -------- | -------- | ------ | ------- |
| `crates/tome/src/remove.rs` | `FailureKind` enum (4 variants), `RemoveFailure` struct, `RemoveResult.failures: Vec<RemoveFailure>`, 4 per-loop pushes replacing eprintln | ✓ VERIFIED | `pub(crate) enum FailureKind` at line 47 with all 4 variants (Symlink/LibraryDir/LibrarySymlink/GitCache); `pub(crate) struct RemoveFailure` at line 60; `failures: Vec<RemoveFailure>` field at line 76; 4 `failures.push(RemoveFailure { … })` sites (lines 225/241/250/273); no `eprintln!("warning: failed to remove` strings remain in `execute`. |
| `crates/tome/src/lib.rs` | `Command::Remove` handler emits grouped `⚠ K operations failed` summary and returns `Err(anyhow!("remove completed with {k} failures"))` | ✓ VERIFIED | Lines 420-456 contain the surfacing block. Uses `style("⚠").yellow()` (line 426), `paths::collapse_home(&f.path)` (line 452), and the four group labels ("Distribution symlinks" / "Library entries" / "Library symlinks" / "Git cache"). Returns `Err(anyhow::anyhow!("remove completed with {k} failures"))` at line 456. Success `println!` extended with `", git cache"` suffix at lines 413-417 consumes `git_cache_removed`. |
| `crates/tome/tests/cli.rs` | Integration test `remove_partial_failure_exits_nonzero_with_warning_marker` | ✓ VERIFIED | Test defined at line 3377. Uses `chmod 0o500` on target (not 0o000 — documented correction), restores 0o755 at line 3435 before assertions. Passes. |
| `Cargo.toml` | `arboard = { version = "3", default-features = false }` | ✓ VERIFIED | Line 15. `cargo tree -p tome -i image` returns "package ID specification did not match" — confirming the image crate is NOT pulled in. |
| `crates/tome/Cargo.toml` | `arboard = { workspace = true }` | ✓ VERIFIED | Line 16. `cargo tree -p tome -i arboard` shows `arboard v3.6.1`. |
| `crates/tome/src/browse/app.rs` | `status_message: Option<String>` field, clear in `handle_key`, `execute_action` rewrite with `cfg!(target_os)` dispatch + `arboard::Clipboard` | ✓ VERIFIED | `pub status_message: Option<String>` at line 89; `status_message: None` init at line 114; `self.status_message = None` (count=1) at line 127 (first statement of `handle_key`); `cfg!(target_os = "macos")` at line 215 dispatches between `open` and `xdg-open`; `arboard::Clipboard::new()` at line 242; success glyphs (`✓ Opened:`, `✓ Copied:`) and failure glyphs (`⚠ Could not open:`, `⚠ Could not copy:`) all present; no `pbcopy`, no `sh -c`, no `let _ = Command::new("open")` silent-drop. |
| `crates/tome/src/browse/ui.rs` | Conditional status-bar render in Normal + Detail modes with `theme.alert`/`theme.accent` glyph dispatch | ✓ VERIFIED | Two `if let Some(msg) = &app.status_message` sites at lines 314 and 363; `msg.starts_with('⚠')` glyph dispatch at lines 315 and 364; `theme.alert` and `theme.accent` both referenced; `theme.rs` does NOT define a new `warning` field (only existing `accent` + `alert` reused per Pitfall 5). |
| `crates/tome/src/relocate.rs` | Explicit match replacing `.ok()`; `warning: could not read symlink at {}: {e}` shape | ✓ VERIFIED | Lines 89-108. `match std::fs::read_link(&link_path)` at line 96; warning string `"warning: could not read symlink at {}: {e}"` at line 100; `link_path.display()` at line 101 (matches PR #448 format exactly). No `std::fs::read_link(&link_path).ok()` substring remains. |
| `CHANGELOG.md` | Three bullets under `[Unreleased] ### Fixed` for #413, #414, #449 | ✓ VERIFIED | Lines 12, 13, 15 reference #413/#414/#449 respectively; SAFE-01 bullet contains "aggregates partial-cleanup"; SAFE-02 bullet contains "xdg-open" + "arboard" + "status bar"; SAFE-03 bullet contains "could not read symlink" + "PR #448". No Cargo.toml version bump. |

### Key Link Verification

| From | To | Via | Status | Details |
| ---- | -- | --- | ------ | ------- |
| `remove.rs::execute` four failure loops | `failures.push(RemoveFailure { … })` | match arm replacing former eprintln! | ✓ WIRED | `rg -c failures\\.push\\(RemoveFailure` = 4 sites (lines 225/241/250/273); zero `eprintln!("warning: failed to remove` strings remain inside `execute`. |
| `lib.rs Command::Remove` | stderr grouped summary + `anyhow::anyhow!("remove completed with {k} failures")` | `if !result.failures.is_empty() { … return Err(…) }` block | ✓ WIRED | Lines 422-456. Early-return with anyhow Err prevents exit-code regression; integration test observes non-zero exit + stderr markers. |
| `app.rs execute_action` | `self.status_message = Some(format!(…))` | match arms on Ok/Err for both ViewSource and CopyPath | ✓ WIRED | Lines 222, 228, 247, 251. Both Ok and Err paths set `status_message`. |
| `app.rs handle_key` | `self.status_message = None` | first statement before mode dispatch | ✓ WIRED | Line 127 (single occurrence in file). |
| `ui.rs render_status_bar` (Normal) + `render_detail` (Detail) | `Line::from(vec![Span::styled(msg, …)])` | `if let Some(msg) = &app.status_message { … }` | ✓ WIRED | Two occurrences at lines 314 and 363 — one per call site as specified. |
| `relocate.rs::plan()` managed-skill block | stderr via `eprintln!("warning: could not read symlink at {}: {e}", …)` | match arm on `Err(e)` returning `None` | ✓ WIRED | Line 100, matching PR #448 format exactly. |

### Data-Flow Trace (Level 4)

| Artifact | Data Variable | Source | Produces Real Data | Status |
| -------- | ------------- | ------ | ------------------ | ------ |
| `remove.rs::execute` | `failures: Vec<RemoveFailure>` | Populated by `.push()` in each of 4 loops on `Err(e)` from `std::fs::remove_file` / `std::fs::remove_dir_all` | Yes — integration test observes real `EACCES` errors under `chmod 0o500` fixture | ✓ FLOWING |
| `lib.rs Command::Remove` surfacing block | `result.failures` | Returned by `remove::execute(…)` | Yes — integration test verifies stderr contains "⚠", "operations failed", "remove completed with" under fixture | ✓ FLOWING |
| `app.rs execute_action` (CopyPath) | `arboard::Clipboard::new().and_then(|mut cb| cb.set_text(…))` | Real `arboard::Clipboard` — not mocked | Yes on dev machines with a clipboard; `Err` path surfaces via status bar on headless runners | ✓ FLOWING (dev) / ⚠ HUMAN (Linux) |
| `app.rs execute_action` (ViewSource) | `Command::new(binary).arg(&path).spawn()` where `binary = cfg!(target_os)` | Real `std::process::Command` | Yes on macOS; Linux branch needs runtime verification | ✓ FLOWING (macOS) / ⚠ HUMAN (Linux) |
| `ui.rs` status bar | `app.status_message: Option<String>` | Set by `execute_action` Ok/Err arms; cleared by `handle_key` first statement | Yes — unit test `status_message_set_by_copy_path_and_cleared_by_any_key` exercises the full lifecycle | ✓ FLOWING |
| `relocate.rs::plan()` | `source_path: Option<PathBuf>` | `match std::fs::read_link` Ok → `Some(...)`, Err → warning + None | Yes — unit test engineers the `Err` path via chmod and verifies `source_path.is_none()` contract holds (plan() still succeeds) | ✓ FLOWING |

### Behavioral Spot-Checks

| Behavior | Command | Result | Status |
| -------- | ------- | ------ | ------ |
| Crate builds clean | `cargo build -p tome` | Compiled in 1.51s, zero warnings | ✓ PASS |
| Binary runs and shows expected subcommands | `./target/debug/tome --help` | Lists add/init/sync/status/doctor/list/lint/browse/eject/remove/… | ✓ PASS |
| Unit tests pass | `cargo test -p tome --lib` | 453 passed; 0 failed | ✓ PASS |
| Integration tests pass | `cargo test -p tome --test cli` | 123 passed; 0 failed | ✓ PASS |
| SAFE-01 unit test | `cargo test -p tome --lib remove::tests::partial_failure_aggregates_symlink_error` | 1 passed | ✓ PASS |
| SAFE-01 integration test | `cargo test -p tome --test cli remove_partial_failure_exits_nonzero_with_warning_marker` | 1 passed | ✓ PASS |
| SAFE-02 unit test | `cargo test -p tome --lib browse::app::tests::status_message_set_by_copy_path_and_cleared_by_any_key` | 1 passed | ✓ PASS |
| SAFE-03 unit test | `cargo test -p tome --lib relocate::tests::read_link_failure_records_no_provenance` | 1 passed | ✓ PASS |
| `image` crate NOT pulled via arboard (Pitfall 1) | `cargo tree -p tome -i image` | "package ID specification `image` did not match any packages" | ✓ PASS |
| `arboard` present in dep tree | `cargo tree -p tome -i arboard` | `arboard v3.6.1` | ✓ PASS |
| `sh -c / pbcopy` gone from browse | `rg "sh -c\|pbcopy" crates/tome/src/browse/app.rs` | no matches | ✓ PASS |
| `.ok()` on read_link gone from relocate | `rg "std::fs::read_link\\(&link_path\\)\\.ok\\(\\)" crates/tome/src/relocate.rs` | no matches | ✓ PASS |
| D-14 preserved sites untouched | `rg "\\.ok\\(\\)" browse/theme.rs git.rs` | theme.rs:115,117 env parse fallback intact | ✓ PASS |

**Test totals:** 453 unit + 123 integration = **576 tests, all green** (+2 vs. SAFE-01 summary's 574 baseline, one per SAFE-02/03 unit test; SUMMARY 02 recorded 575 total; current count of 576 suggests one additional test was added since or is an unrelated addition — not a regression).

### Requirements Coverage

| Requirement | Source Plan | Description | Status | Evidence |
| ----------- | ----------- | ----------- | ------ | -------- |
| SAFE-01 | 08-01-safe-01-remove-partial-failure-aggregation-PLAN.md | `remove::execute` aggregates partial-cleanup failures into `RemoveResult`; caller surfaces them via non-zero exit + `⚠ N operations failed` summary | ✓ SATISFIED | `.planning/REQUIREMENTS.md:39` marked `[x]`; all artifacts verified above; 2 tests pass. |
| SAFE-02 | 08-02-safe-02-browse-cross-platform-status-bar-PLAN.md | Browse UI's `ViewSource` and `CopyPath` work on Linux via `xdg-open` + `arboard`; failures surface in TUI status bar | ✓ SATISFIED (automated) / ? NEEDS HUMAN (Linux runtime) | `.planning/REQUIREMENTS.md:40` marked `[x]`; compile-time coverage via CI matrix + dev-machine unit test pass. Linux runtime behavior flagged for human verification (see `human_verification` block). |
| SAFE-03 | 08-03-safe-03-relocate-read-link-warning-PLAN.md | `relocate.rs:93` surfaces symlink-read failures as stderr warnings | ✓ SATISFIED | `.planning/REQUIREMENTS.md:41` marked `[x]`; artifact verified; 1 test passes. |

**Note on REQUIREMENTS.md traceability table:** Lines 79-81 of REQUIREMENTS.md still mark SAFE-01/02/03 as "Not started" in the status column, while lines 39-41 correctly mark them `[x]`. This is cosmetic staleness in the lower traceability table — not a verification failure. The authoritative state is the `[x]` checkboxes at the top of the file, which match the ROADMAP.md status at lines 67-69 (all three `[x]`).

### Anti-Patterns Found

| File | Line | Pattern | Severity | Impact |
| ---- | ---- | ------- | -------- | ------ |
| _(none)_ | _(none)_ | _(none)_ | — | No TODO/FIXME/stub/placeholder comments introduced in the 8 modified files. No hardcoded empty returns. No console.log-only handlers. No anti-patterns detected. |

### Human Verification Required

Two items flagged — both relate to the Linux-only runtime surface of SAFE-02 which cannot be exercised on macOS CI/dev hardware and which CONTEXT.md D-17/D-19 intentionally chose NOT to mock (no `trait Opener` / `trait ClipboardBackend` / direct `#[cfg(target_os = "linux")]` test abstractions):

#### 1. Linux clipboard runtime (SAFE-02)

**Test:** On a Linux desktop (x11 or wayland), run `tome browse`, navigate Detail view, press the `copy path` action.
**Expected:** Status bar renders `✓ Copied: <~/-prefixed path>` styled in `theme.accent`. Pasting into a terminal (e.g., `Ctrl+Shift+V`) produces the copied path. On `handle_key`'s next keypress, the status bar reverts to the keybind hint line.
**Why human:** `arboard 3.6.1` dispatches to x11 (`libxcb`) or wayland (`wayland-client`) at runtime; macOS CI can only compile the Linux branch, not exercise it. No mocking was introduced per CONTEXT.md D-19.

#### 2. Linux xdg-open runtime (SAFE-02)

**Test:** On the same Linux desktop, press the `open` (ViewSource) action.
**Expected:** `xdg-open <path>` spawns; the system default handler opens the skill directory. Status bar shows `✓ Opened: …`. On headless SSH (no DISPLAY / WAYLAND_DISPLAY), `spawn()` fails and status bar shows `⚠ Could not open: <error>` styled in `theme.alert`.
**Why human:** `cfg!(target_os = "macos")` selects `xdg-open` at compile time on Linux, but the spawn success depends on `xdg-utils` being installed AND a desktop session being present. Not observable from macOS.

### Gaps Summary

No blocking gaps found. All four observable truths from the success criteria are VERIFIED automatically. The phase goal is achieved:

- **Partial-failure visibility for `tome remove`** (SAFE-01 / #413) — `FailureKind` + `RemoveFailure` records wire from 4 per-loop push sites in `remove.rs::execute` into a grouped stderr summary in `lib.rs::Command::Remove` that uses `paths::collapse_home` + `console::style` glyph vocabulary; exit code is non-zero via `anyhow::anyhow!` return; integration test confirms end-to-end observable behavior under a `chmod 0o500` fixture. Clean success path is unchanged apart from a `", git cache"` suffix when `git_cache_removed` (required to consume the field after dropping `#[allow(dead_code)]`).

- **Cross-platform browse actions + status bar** (SAFE-02 / #414) — `sh -c | pbcopy` command-injection vector removed; `arboard 3.6.1` added with `default-features = false` (Pitfall 1 satisfied: no `image` crate in dep tree); `cfg!(target_os = "macos")` dispatch for `open` vs `xdg-open`; `App.status_message: Option<String>` with any-key-dismisses semantics (cleared as first statement of `handle_key`); both Ok/Err paths route into status bar via glyph-prefix color dispatch (✓ → `theme.accent`, ⚠ → `theme.alert`); no new `theme.warning` field added (Pitfall 5 satisfied); duplicate conditional render in both Normal-mode `render_status_bar` and Detail-mode inline site. Linux runtime behaviors (clipboard providers, xdg-open spawn) flagged for human verification — CONTEXT.md D-17/D-19 explicitly chose compile-time coverage only (CI matrix's `ubuntu-latest` exercises the branch at link time).

- **Surfaced symlink-read warnings in `tome relocate`** (SAFE-03 / #449) — silent `std::fs::read_link(&link_path).ok()` drop replaced with explicit match; `Err` arm emits `warning: could not read symlink at {}: {e}` matching PR #448's canonical format verbatim; `None` fallback preserved so `plan()` still completes; unit test documents the Unix platform semantic caveat (`is_symlink()` + `read_link()` share parent-search-permission gate) and verifies observable side-effect (`source_path.is_none()`) without introducing a `gag` dev-dep.

- **Test coverage for new aggregation and action dispatcher** (success criteria #4) — 4 new tests total (`partial_failure_aggregates_symlink_error`, `remove_partial_failure_exits_nonzero_with_warning_marker`, `status_message_set_by_copy_path_and_cleared_by_any_key`, `read_link_failure_records_no_provenance`). All pass; 576 tests overall pass with zero failures.

No blocker anti-patterns detected in the 8 modified files. No Cargo.toml version bump (release flow untouched). No untouched-per-D-14 sites (`theme.rs:115-117` env parse `.ok()`, `git.rs:69` `let _ = rev`) were modified.

**Recommendation:** Once the two Linux human-verification items above are confirmed on hardware, the phase can be treated as fully complete. All automated evidence satisfies SAFE-01/SAFE-02/SAFE-03 observable contracts.

---

_Verified: 2026-04-24_
_Verifier: Claude (gsd-verifier)_
