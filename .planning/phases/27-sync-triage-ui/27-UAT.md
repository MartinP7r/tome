---
status: testing
phase: 27-sync-triage-ui
source: [27-01a-SUMMARY.md, 27-01b-SUMMARY.md, 27-02-SUMMARY.md, 27-02b-SUMMARY.md, 27-03-SUMMARY.md, 27-04-SUMMARY.md, 27-05-SUMMARY.md]
started: 2026-06-07T04:35:00Z
updated: 2026-06-07T04:41:00Z
---

## Current Test

number: 4
name: Run Sync — In-Progress UI
expected: |
  Click "Run sync". StageStepper appears with rows for Discover / Reconcile / Distribute / Cleanup / Save.
  Active stage shows a spinner + current item (skill name or path). Sidebar Sync NavItem shows a spinner badge while running.
awaiting: user response

## Tests

### 1. Cold Start Smoke Test
expected: |
  Desktop app boots; sidebar shows 4 nav items in order: Status / Skills / Sync / Health. No console errors.
result: pass

### 2. Sync NavItem + ⌘3/⌘4 Re-anchoring
expected: |
  Sync is the 4th sidebar item (after Status, Skills) but reachable via ⌘3.
  ⌘3 jumps to Sync, ⌘4 jumps to Health (note: Phase 26's ⌘3 used to go to Health — that mapping has moved).
  Health is still reachable, just via ⌘4 now.
result: pass

### 3. Sync Idle State
expected: |
  Click Sync in the sidebar (or ⌘3). The view shows an idle hero with "Last synced X ago" (or "You haven't synced yet" if first time),
  a prominent "Run sync" button, and a recent-changes disclosure section below.
result: pass
observation: |
  Spec elements all present, but user noted the screen feels empty — "no information about past runs."
  The recent-changes disclosure is present but conveys little signal on a no-history machine.
  Filed as backlog 999.2 (Sync idle state — surface past-run history more prominently).
  NOT reopening test 3 (the contract was met); this is a UX improvement candidate for a future phase.

### 4. Run Sync — In-Progress UI
expected: |
  Click "Run sync". A StageStepper appears showing each stage (Discover / Reconcile / Distribute / Cleanup / Save) as a row.
  The active stage shows a spinner and the current item being processed (skill name or path).
  Sidebar's Sync NavItem shows a spinner badge while running.
result: [pending]

### 5. Cancel During Sync
expected: |
  While a sync is in progress, click Cancel. The pipeline stops at the next stage boundary.
  Stepper enters a "cancelled" terminal state inline (no toast — D-18 supersession honored).
  After cancel, library state is consistent: no half-written manifest, no partial lockfile.
  Re-running `tome status` from the CLI should show pre-sync state preserved (or fully consistent post-sync if the cancel landed at a clean boundary).
result: [pending]

### 6. Triage Panel After Sync
expected: |
  After a sync that produces a lockfile diff, TriagePanel appears (right side of split pane).
  Three section headers visible: NEW / CHANGED / REMOVED with per-group counts.
  Each row has an inline [✓ keep] chip toggle. Bulk actions for NEW (e.g. "disable all new from <source>") are visible.
  Right column TriageDetail shows the selected skill's diff metadata.
  Bottom shows "[Apply N decisions]" button (disabled until at least one non-keep decision is made).
result: [pending]

### 7. Apply Triage with machine.toml Diff Preview
expected: |
  Toggle at least one skill off (or pick a non-keep action). [Apply N decisions] enables.
  Click it. PreviewPopover opens showing a line-by-line machine.toml diff:
    - Removed lines: red background
    - Added lines: green background
    - Unchanged lines: neutral, with line numbers
  Cancel discards changes (no write to disk). Apply commits — machine.toml on disk updates atomically.
result: [pending]

### 8. Skills View Sort=Recent + Group=Source/Role
expected: |
  Navigate to Skills (⌘2). Change Sort dropdown to "Recent". Skill list reorders by `synced_at` (most-recent first).
  Change Group dropdown to "Source" (or "Role"). Skill list now renders section headers between groups, each with the source/role name + per-group skill count.
  VoiceOver (or accessibility tree) sees section headers as proper landmarks.
result: [pending]

### 9. Sync Terminal States — Success + Failure
expected: |
  Success path: a clean sync completes; stepper enters green terminal state; transient "Sync complete" toast appears at the bottom
  (auto-dismisses after ~5s, also has a "Dismiss" button to close immediately).
  Failure path: trigger a sync that fails partway (e.g. unreachable git source). Stepper enters red terminal state;
  SyncSummary below shows "Retry from <stage>" action; sidebar Sync NavItem shows a failure badge.
  Partial-failure path (success-with-warnings): per the SUMMARY, the wire/UI is structurally ready but `partial_failures` is empty for now
  (carry-forward documented in 27-05-SUMMARY) — so this branch may not be triggerable in this phase. Confirm the success and failure
  paths render; partial-failure visual state is acceptable as "documented gap until sync() inline-surfaces SAFE-01".
result: [pending]

### 10. Doctor PreviewPopover Refactor Regression
expected: |
  Navigate to Health (⌘4). Click any Finding row's "Fix" action.
  PreviewPopover opens with the existing single-sentence dryRun description and Apply/Cancel buttons (unchanged behavior).
  This confirms Pitfall 3's atomic slot refactor in 27-03 didn't break the Doctor caller — same behavior, different internals.
result: [pending]

## Summary

total: 10
passed: 3
issues: 0
pending: 7
skipped: 0
blocked: 0

## Gaps

<!-- Appended as issues are reported -->
