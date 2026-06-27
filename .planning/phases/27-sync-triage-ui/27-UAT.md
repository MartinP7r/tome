---
status: partial
phase: 27-sync-triage-ui
source: [27-01a-SUMMARY.md, 27-01b-SUMMARY.md, 27-02-SUMMARY.md, 27-02b-SUMMARY.md, 27-03-SUMMARY.md, 27-04-SUMMARY.md, 27-05-SUMMARY.md]
started: 2026-06-07T04:35:00Z
updated: 2026-06-27T00:00:00Z
---

## Current Test

[testing paused — 1 item outstanding (test 7 blocked on prior finding)]

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
result: pass
observation: |
  User feedback: "yes, this shows, though it's disappearing much too fast to have enough meaningful impact."
  All UI elements render per contract, but on small libraries each stage completes in milliseconds — the user only catches the terminal state. No minimum dwell time on stage transitions; no way to inspect what happened in each stage after the fact.
  Filed as backlog 999.3 (StageStepper dwell time + post-hoc per-stage inspection).
  NOT reopening test 4 (contract met); UX improvement candidate.

### 5. Cancel During Sync
expected: |
  While a sync is in progress, click Cancel. The pipeline stops at the next stage boundary.
  Stepper enters a "cancelled" terminal state inline (no toast — D-18 supersession honored).
  After cancel, library state is consistent: no half-written manifest, no partial lockfile.
  Re-running `tome status` from the CLI should show pre-sync state preserved (or fully consistent post-sync if the cancel landed at a clean boundary).
result: pass

### 6. Triage Panel After Sync
expected: |
  After a sync that produces a lockfile diff, TriagePanel appears (right side of split pane).
  Three section headers visible: NEW / CHANGED / REMOVED with per-group counts.
  Each row has an inline [✓ keep] chip toggle. Bulk actions for NEW (e.g. "disable all new from <source>") are visible.
  Right column TriageDetail shows the selected skill's diff metadata.
  Bottom shows "[Apply N decisions]" button (disabled until at least one non-keep decision is made).
result: issue
reported: "I changed some text in ~/.claude/skills/asc-app-create-ui/SKILL.md but it's not caught when I run sync"
severity: major
root_cause: |
  get_lockfile_diff (commands.rs) builds the prospective lockfile via
  tome::lockfile::generate(&manifest, &skills), which copies content_hash FROM the
  stored manifest and uses the freshly-discovered `skills` only for provenance — never
  re-hashing the on-disk skill dirs. Because generate() iterates manifest.iter(), the
  "proposed" lockfile is structurally a copy of the manifest. The diff is therefore
  manifest-vs-lockfile, and those two always agree (written together by the last sync),
  so the panel is ~always empty and cannot surface NEW (not yet in manifest), CHANGED
  (manifest hash stale), or REMOVED (still in manifest) skills from a source edit.
  Compounding: start_sync runs the full tome::sync pipeline (consolidate re-hashes +
  save) in one shot — there is no discover→pause→triage→apply gate, so a real edit is
  applied immediately, leaving nothing to triage. Verified: manifest hash == lockfile
  hash == 6649…dcdf5f for asc-app-create-ui, both pre-edit. Environment confirmed
  healthy (coding-agent-files/.tome/*, 189 skills consistent; the .local/share 35-skill
  library is dead legacy, not used by the app).
artifacts:
  - path: "crates/tome/src/lockfile.rs"
    issue: "generate() copies content_hash from manifest instead of re-hashing discovered skill dirs"
  - path: "crates/tome-desktop/src/commands.rs"
    issue: "get_lockfile_diff builds prospective lockfile from manifest, not a fresh dry-run consolidate"
  - path: "crates/tome-desktop/ui/src/hooks/useSync.tsx"
    issue: "start_sync runs full pipeline with no triage gate before apply"
missing:
  - "Triage diff must compute prospective hashes from a fresh re-hash of discovered skills (dry-run consolidate), not the stored manifest"
  - "Sync flow needs a discover→preview→triage→apply gate so pending changes are shown BEFORE they are applied"

### 7. Apply Triage with machine.toml Diff Preview
expected: |
  Toggle at least one skill off (or pick a non-keep action). [Apply N decisions] enables.
  Click it. PreviewPopover opens showing a line-by-line machine.toml diff:
    - Removed lines: red background
    - Added lines: green background
    - Unchanged lines: neutral, with line numbers
  Cancel discards changes (no write to disk). Apply commits — machine.toml on disk updates atomically.
result: blocked
blocked_by: prior-finding
reason: "Depends on a populated triage panel (test 6). Can't exercise Apply with no pending decisions until the test-6 diff defect is fixed. The machine.toml write boundary itself (preview/apply commands, PreviewPopover slot) is independently unit-tested green in 27-03; this is the end-to-end path only."

### 8. Skills View Sort=Recent + Group=Source/Role
expected: |
  Navigate to Skills (⌘2). Change Sort dropdown to "Recent". Skill list reorders by `synced_at` (most-recent first).
  Change Group dropdown to "Source" (or "Role"). Skill list now renders section headers between groups, each with the source/role name + per-group skill count.
  VoiceOver (or accessibility tree) sees section headers as proper landmarks.
result: skipped
reason: user requested skip

### 9. Sync Terminal States — Success + Failure
expected: |
  Success path: a clean sync completes; stepper enters green terminal state; transient "Sync complete" toast appears at the bottom
  (auto-dismisses after ~5s, also has a "Dismiss" button to close immediately).
  Failure path: trigger a sync that fails partway (e.g. unreachable git source). Stepper enters red terminal state;
  SyncSummary below shows "Retry from <stage>" action; sidebar Sync NavItem shows a failure badge.
  Partial-failure path (success-with-warnings): per the SUMMARY, the wire/UI is structurally ready but `partial_failures` is empty for now
  (carry-forward documented in 27-05-SUMMARY) — so this branch may not be triggerable in this phase. Confirm the success and failure
  paths render; partial-failure visual state is acceptable as "documented gap until sync() inline-surfaces SAFE-01".
result: skipped
reason: user requested skip

### 10. Doctor PreviewPopover Refactor Regression
expected: |
  Navigate to Health (⌘4). Click any Finding row's "Fix" action.
  PreviewPopover opens with the existing single-sentence dryRun description and Apply/Cancel buttons (unchanged behavior).
  This confirms Pitfall 3's atomic slot refactor in 27-03 didn't break the Doctor caller — same behavior, different internals.
result: skipped
reason: user requested skip

## Summary

total: 10
passed: 5
issues: 1
pending: 0
skipped: 3
blocked: 1

## Gaps

- truth: "Triage panel lists NEW/CHANGED/REMOVED skills that a sync would apply, so the user can triage before applying (SYNC-02, ROADMAP SC#2)"
  status: failed
  reason: "User edited a SKILL.md in a source dir; the change is not caught — the triage panel stays empty."
  severity: major
  test: 6
  root_cause: "get_lockfile_diff builds the prospective lockfile via lockfile::generate(&manifest,&skills), which reuses the manifest's stored content_hash instead of re-hashing discovered skill dirs. Diff is manifest-vs-lockfile (always agree → empty). No discover→triage→apply gate; start_sync applies the full pipeline in one shot."
  artifacts:
    - path: "crates/tome/src/lockfile.rs"
      issue: "generate() copies content_hash from manifest, not a fresh re-hash"
    - path: "crates/tome-desktop/src/commands.rs"
      issue: "get_lockfile_diff projects from manifest, not a dry-run consolidate"
    - path: "crates/tome-desktop/ui/src/hooks/useSync.tsx"
      issue: "start_sync runs full pipeline with no pre-apply triage gate"
  missing:
    - "Compute prospective hashes via a fresh re-hash of discovered skills (dry-run consolidate)"
    - "Add a discover→preview→triage→apply gate so changes show BEFORE they are applied"
  debug_session: ""
  fix: |
    Added lockfile::generate_prospective(skills) in crates/tome/src/lockfile.rs —
    re-hashes each skill's source dir on disk. Wired get_lockfile_diff in
    crates/tome-desktop/src/commands.rs to call it instead of generate(&manifest, &skills).
    Commit: eb0f423. All tests green.
