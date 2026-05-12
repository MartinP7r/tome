---
status: deferred
phase: 08-safety-refactors-partial-failure-visibility-cross-platform
source: [08-VERIFICATION.md]
started: 2026-04-24T00:00:00Z
updated: 2026-05-12T00:00:00Z
deferred_to: v1.0
deferred_at: 2026-05-12
deferred_rationale: |
  Both items have been carried over for FIVE consecutive milestone cuts
  (v0.8 → v0.8.1 → v0.9 → v0.10 → v0.11) without Linux desktop hardware
  becoming available for end-to-end verification. The underlying
  arboard / xdg-open runtime behaviors:

  - have not produced reported bugs from any tome user (Martin is the
    sole user, on macOS)
  - are isolated to single-call code paths that compile and link clean
    on the ubuntu-latest CI matrix
  - have unit-level test coverage on the dispatch logic (the `xdg-open`
    vs `open` cfg branch is exercised on every CI run)

  Unit + compile coverage + zero reported bugs across five milestones
  = adequate confidence to ship without the manual UAT. Re-validating
  every milestone has 0% chance of surfacing a regression nobody is
  hitting in production.

  The items are not abandoned — they're explicitly bundled into the
  v1.0 (Tauri Desktop GUI) milestone, which will require Linux
  hardware for the GUI build target anyway. That's the natural moment
  to verify these runtime paths end-to-end.
---

## Current Test

[deferred to v1.0 — see frontmatter `deferred_rationale`]

## Tests

### 1. Linux clipboard runtime — `tome browse` → `copy path`
expected: |
  On a Linux desktop session (Ubuntu/Fedora/Arch, X11 or Wayland) with `arboard` 3.6.1's platform backends reachable: run `tome browse`, navigate to any skill, enter Detail view, and press the `copy path` keybinding. The status bar (bottom line of the TUI) replaces the keybind hint with `✓ Copied: <path>` styled in `theme.accent` color. Pasting (Ctrl+V / Cmd+V / shell paste) into another terminal or editor yields the exact copied path. The status message clears on the next keypress.
why_human: |
  `arboard` dispatches to x11 (`x11rb`) and wayland (`wayland-client`) backends at runtime — these code paths cannot execute on macOS. The `ubuntu-latest` CI matrix links the crate at compile time but does not exercise a display server, so only a real Linux desktop session verifies end-to-end behavior. Per CONTEXT.md D-17/D-19, no trait abstractions or mocks were introduced.
result: deferred to v1.0

### 2. Linux xdg-open runtime — `tome browse` → `open`
expected: |
  On the same Linux desktop session with `xdg-utils` installed and an active `DISPLAY` or `WAYLAND_DISPLAY`: press the `open` action (ViewSource) in `tome browse` Detail view. The system's default handler opens the skill directory or file via `xdg-open <path>`. Status bar shows `✓ Opened: <path>` in `theme.accent`. Separately, on a headless SSH session (no display server) the same action surfaces `⚠ Could not open: <error>` in `theme.alert`.
why_human: |
  The `cfg!(target_os = "macos")` dispatch at `browse/app.rs:215` selects `xdg-open` at compile time on Linux, but the spawn outcome depends on `xdg-utils` being installed AND a reachable display server. macOS cannot exercise the `xdg-open` branch at all; headless Linux CI can only confirm the failure path indirectly. A real Linux desktop session is the only way to verify the success path behaves as expected.
result: deferred to v1.0

## Summary

total: 2
passed: 0
issues: 0
pending: 0
skipped: 0
blocked: 0
deferred: 2

## Gaps
