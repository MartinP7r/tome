# Phase 8: Safety Refactors (Partial-Failure Visibility & Cross-Platform) - Discussion Log

> **Audit trail only.** Do not use as input to planning, research, or execution agents.
> Decisions are captured in CONTEXT.md — this log preserves the alternatives considered.

**Date:** 2026-04-24
**Phase:** 08-safety-refactors-partial-failure-visibility-cross-platform
**Areas discussed:** SAFE-01 error shape & exit code; SAFE-02 clipboard/open strategy; SAFE-02 status-bar plumbing; Packaging & test strategy

---

## Gray Area Selection

User selected all four surfaced gray areas (multiSelect).

| Option | Description | Selected |
|--------|-------------|----------|
| SAFE-01 error shape & exit code | `RemoveResult.failures` type + exit behavior + save ordering | ✓ |
| SAFE-02 clipboard/open strategy | `arboard` vs hand-rolled; `open`/`xdg-open` dispatch | ✓ |
| SAFE-02 status-bar plumbing | Persistence, content, render location of action feedback | ✓ |
| Packaging & test strategy | Plan breakdown + cross-platform test approach | ✓ |

---

## Area 1: SAFE-01 — Error Shape & Exit Code

### Q1: Error type shape (clarified after "what's op-kind?" follow-up)

**User question:** "what's op-kind?" — needed definition of the `FailureKind` enum; re-asked with explicit enum variants listed.

| Option | Description | Selected |
|--------|-------------|----------|
| Typed struct with op-kind | `Vec<RemoveFailure { path, op: FailureKind, error }>` where FailureKind ∈ {Symlink, LibraryDir, LibrarySymlink, GitCache}. Enables grouped summary, tome-doctor routing, typed tests. | ✓ |
| Raw tuples + inline prefix | `Vec<(PathBuf, io::Error)>` per #413 as-written, with loop-local prefix in error message. Simplest. | |
| Pre-formatted strings | `Vec<String>` already-rendered. Dead simple; loses programmatic access. | |

**Notes:** ~15 extra LoC for the tagged struct justified by grouped-summary output and future `tome doctor` integration.

### Q2: Exit behavior

| Option | Description | Selected |
|--------|-------------|----------|
| Warn + exit non-zero | Print ✓ success line AND ⚠ warning line, return `Err(anyhow::anyhow!(...))`. Shell scripts see exit ≠ 0. Issue #413 alternative 2. | ✓ |
| Hard bail before save | First failure makes `execute()` return `Err`; config/manifest/lockfile never saved. Most conservative. | |
| Warn loudly, exit zero | Styled summary line; exit stays 0. Lowest breakage; shell scripts still can't detect. | |

### Q3: State-save ordering

| Option | Description | Selected |
|--------|-------------|----------|
| Save, then warn | Keep current order. Plan mostly executed; leftover artifacts become `tome doctor` territory + next `tome sync` cleanup. Matches user's mental model. | ✓ |
| Skip save on any failure | Reject all state changes on any failure. "Rollback" only partial — already-deleted files can't be un-deleted. | |
| Save iff critical steps succeeded | Threshold logic. Too magic, listed for completeness only. | |

---

## Area 2: SAFE-02 — Clipboard/Open Strategy

### Q1: Clipboard implementation

| Option | Description | Selected |
|--------|-------------|----------|
| `arboard` crate | One-liner call. Handles Wayland/X11/macOS internally. ~4–6 transitive deps on Linux. | ✓ |
| Handrolled cfg dispatch | Manual platform branches; no new deps. ~30 LoC. Own the "tool not installed" branches. | |
| Handrolled + graceful degradation | Same + specific "Install wl-clipboard or xclip" message. Most user-friendly; most code surface. | |

**Notes:** Dep weight accepted. Removes `sh -c` command-injection vector flagged in #414.

### Q2: Open source directory

| Option | Description | Selected |
|--------|-------------|----------|
| `cfg!` dispatch (open / xdg-open) | Single match arm. Both binaries take a path argument identically. | ✓ |
| `open` crate | Adds a tiny crate (`open = "5"`) for `open::that(path)`. | |
| Trait-based `Opener` dispatcher | Abstract for testability. Heavier for two call sites. | |

---

## Area 3: SAFE-02 — Status-Bar Plumbing

### Q1: Persistence model

| Option | Description | Selected |
|--------|-------------|----------|
| Clear on next key | `status_message: Option<String>` field; cleared at top of `handle_key`. No timers. | ✓ |
| Time-based auto-decay | Message persists N seconds. Requires `Instant` + event-loop tick changes. | |
| Sticky until dismissed | Message stays across key presses until Esc or replaced. Errors never missed; can feel noisy. | |

### Q2: What gets reported

| Option | Description | Selected |
|--------|-------------|----------|
| Both, styled differently | ✓ success in green; ⚠ failure in yellow. Positive confirmation that the action ran. | ✓ |
| Failures only | Silent success (current behavior) + surface failure. Minimal clutter; leaves user unsure. | |
| Both, but brief | Minimal glyph+word success; full-context failure. Less noise on happy path. | |

### Q3: Render location

| Option | Description | Selected |
|--------|-------------|----------|
| Replace keybind line transiently | When `status_message.is_some()`, bottom bar renders message instead of keybinds. One-line change in `ui.rs`. Works for Normal + Detail modes. | ✓ |
| Append to keybind line | Keybinds visible alongside message. Overflow risk on narrow terminals. | |
| Dedicated second bottom line | Always-visible separate status line. Costs one row of terminal real estate permanently. | |

---

## Area 4: Packaging & Test Strategy

### Q1: Plan breakdown

| Option | Description | Selected |
|--------|-------------|----------|
| 3 plans, one per SAFE-XX | Matches Phase 7 pattern. SAFE-01 ~60 LoC; SAFE-02 ~80 LoC; SAFE-03 ~10 LoC. | ✓ |
| 2 plans — SAFE-01 alone + SAFE-02/03 bundled | Bundles the "failure-surfacing one-liners"; separates SAFE-01's new types. | |
| 1 bundled plan with 3 task sections | Minimal planning overhead. Hiccup in one section blocks others. | |

### Q2: Linux/cross-platform test strategy

| Option | Description | Selected |
|--------|-------------|----------|
| CI matrix + platform-agnostic unit tests | Unit-test all OS-independent logic. Trust `ubuntu-latest` + `macos-latest` CI for platform branches. No new test scaffolding. | ✓ |
| Thin `Opener` trait + in-memory mock | Abstract dispatcher for testability on any OS. ~40 LoC of abstraction for 2 call sites. | |
| `#[cfg(target_os = "linux")]` tests only | Actually shell out to `xdg-open`/`arboard` in CI. Most realistic; only runs on Linux CI. | |

### Q3: SAFE-01 test coverage

| Option | Description | Selected |
|--------|-------------|----------|
| Unit test + lib.rs integration | Unit: failure-injection in `remove.rs` tests. Integration: `tome remove` exit ≠ 0 + ⚠ marker in stderr. | ✓ |
| Unit test only | Cover aggregation logic; skip integration. Leaves "exit non-zero" contract un-exercised. | |
| Snapshot test of caller's output | `insta` locks exact stderr bytes. Richest; highest maintenance. | |

---

## Claude's Discretion

Items where planner retains flexibility:

- Exact wording of the `⚠ K operations failed` summary line.
- Module location of `FailureKind` enum (local to `remove.rs` vs shared).
- `arboard` version pin tightness (loose major vs tight minor).
- Whether to add a `theme.warning` color field or reuse existing.
- Whether to touch the `DetailAction::Disable`/`Enable` stub (noted as out-of-scope).

## Deferred Ideas

Captured during discussion for future phases:

- Backport `eprintln!` warning pattern to all `.ok()` sites across the codebase (most are deliberate fallbacks).
- `tome doctor` integration for `RemoveFailure` routing.
- Unified `--quiet`/`--verbose` gating for SAFE-03's warning.
- Windows support (constraint violation).
- `DetailAction::Disable`/`Enable` machine.toml wiring.
- Snapshot tests for `⚠` summary line output.
- `theme.warning` color field (only if needed during implementation).
- Extracting helpers from `lib.rs::Command::Remove`'s 50-line body.
