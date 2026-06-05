---
phase: 26-read-only-views-alpha-cut
plan: 05
subsystem: ui
tags:
  - tauri
  - react
  - react-aria-components
  - specta
  - doctor
  - health-view
  - preview-popover
  - sidebar-badge

requires:
  - phase: 26-01
    provides: shell atoms (Pill, StatusDot, KeyValueRow); tokens.css; ContentPane/Window/Sidebar
  - phase: 26-02
    provides: 3-column NavigationSplitView shell; Sidebar NavItem badge slot
  - phase: 26-03
    provides: useSkillActions pattern (aria-live announcer); SkillFrontmatterView specta-friendly DTO precedent (BTreeMap<String,String> JSON-encoded workaround)
  - phase: 26-06
    provides: ManifestChanged / LockfileChanged / LibraryChanged typed events; useTauriEvent hook; synchronized FSEvents-registration watcher

provides:
  - tome::doctor::FindingId enum (content-aware per OQ-2 resolution) + DiagnosticIssue::id() accessor
  - tome::doctor::repair_one(finding_id, config, paths) — per-finding repair API alongside the existing dispatch_repairs batch
  - tome::doctor::repair_kind_action_label promoted to pub for Tauri command use
  - tome::doctor::collect_doctor_view — DoctorView projection consumed by the Tauri command
  - 2 new Tauri commands (get_doctor_report, doctor_repair_one)
  - React atoms: SeverityIcon, SectionHeader, PreviewPopover, FindingRow
  - HealthView with grouped sections (AUTO-FIXABLE / NEEDS ATTENTION), all-clear empty state, and inline failed-fix disclosure (D-11 / SAFE-01)
  - useDoctorReport hook subscribing to manifest + library + lockfile events (NOT machine-prefs — doctor findings don't depend on per-machine disable state)
  - Sidebar Health NavItem live badge wired through the shell-mounted useDoctorReport
  - NF-04 preview-then-confirm contract surfaced per fix (D-09)

affects:
  - phase: 26-07
    via: keyboard map audit will include HealthView's PreviewPopover focus + Esc handling; axe-core gate scans HealthView
  - phase: 26-08
    via: HealthView is one of the alpha-cut views verified by the screenshot + bench harness

tech-stack:
  added:
    - (no new npm or cargo dependencies — uses react-aria-components Popover/Dialog/DialogTrigger already installed in 26-02 + 26-03)
  patterns:
    - Per-item PreviewPopover-then-confirm (NF-04) replacing bulk fix surfaces (D-10)
    - Failed-fix inline [Code] message + collapsible Show-context disclosure with Fix button retained for retry (SAFE-01)
    - Shell-scope hook lifting (App.tsx mounts useDoctorReport once so Sidebar badge + HealthView share state)
    - Content-aware FindingId enum (OQ-2): variant carries the SkillName + RepairKind so repair_one can re-resolve without trusting the GUI's serialised ID
---

## What this plan delivered

Phase 26's Health surface (VIEW-05). The Rust side gained a per-finding repair API — `FindingId` enum (content-aware, not an opaque index, so the GUI can't be tricked into dispatching the wrong action) plus `repair_one(finding_id, config, paths)` alongside the existing `dispatch_repairs` batch. `repair_kind_action_label` was promoted to `pub` so the Tauri command can compose the per-finding dry-run description. Two Tauri commands route through these: `get_doctor_report` projects a fresh `DoctorView` for the React layer; `doctor_repair_one` applies a single repair. The React side ships the full Health view: grouped AUTO-FIXABLE / NEEDS ATTENTION sections, per-finding Fix buttons that open a React Aria DialogTrigger + Popover + Dialog satisfying the NF-04 preview-then-confirm contract (D-09), an inline failed-fix disclosure (D-11 / SAFE-01), and a centred all-clear empty state (D-12). The sidebar Health NavItem now shows a live badge sourced from a shell-scope `useDoctorReport` hook so the count clears the instant the watcher detects a relevant file change. No bulk Fix-all anywhere (D-10).

## Commits

| Commit | Subject |
|---|---|
| `17e022d` | feat(26-05): add FindingId enum + repair_one + DoctorView for GUI Health view |
| `f2cf769` | feat(26-05): wire Health view + PreviewPopover + Sidebar badge (VIEW-05) |
| _(this commit)_ | docs(26-05): complete read-only views alpha cut Health view plan |

## Files

- created:
  - `crates/tome-desktop/ui/src/views/HealthView.tsx`
  - `crates/tome-desktop/ui/src/views/HealthView.module.css`
  - `crates/tome-desktop/ui/src/components/SectionHeader.tsx`
  - `crates/tome-desktop/ui/src/components/SectionHeader.module.css`
  - `crates/tome-desktop/ui/src/components/FindingRow.tsx`
  - `crates/tome-desktop/ui/src/components/FindingRow.module.css`
  - `crates/tome-desktop/ui/src/components/PreviewPopover.tsx`
  - `crates/tome-desktop/ui/src/components/PreviewPopover.module.css`
  - `crates/tome-desktop/ui/src/components/SeverityIcon.tsx`
  - `crates/tome-desktop/ui/src/hooks/useDoctorReport.ts`
- modified:
  - `crates/tome/src/doctor.rs` (FindingId + DiagnosticIssue::id() + repair_one + DoctorView projection + repair_kind_action_label pub)
  - `crates/tome/src/lib.rs` (re-export surface widened narrowly)
  - `crates/tome-desktop/src/commands.rs` (get_doctor_report + doctor_repair_one)
  - `crates/tome-desktop/src/lib.rs` (collect_commands! registration)
  - `crates/tome-desktop/ui/src/bindings.ts` (regenerated — DoctorView_Serialize / DoctorFinding_Serialize / FindingId / RepairKind / IssueSeverity / IssueCategory types)
  - `crates/tome-desktop/ui/src/App.tsx` (mounts useDoctorReport at shell scope; replaces HealthPlaceholder with HealthView; passes badgeCount to Sidebar)
  - `crates/tome-desktop/ui/src/shell/Sidebar.tsx` (consumes badgeCount prop, clears at zero per D-12)

Total: 17 files changed, +2083 / −52.

## Watcher contract (carried from 26-06)

`useDoctorReport` subscribes to `manifest_changed` + `library_changed` + `lockfile_changed`. It deliberately does NOT subscribe to `machine_prefs_changed` — per-machine disable state never affects doctor findings, so refetching on every "Disable on this machine" mutation would waste IPC + render cycles. Manifest covers orphan-skill detection; library covers nested SKILL.md edits; lockfile covers stale-entry diagnostics. The hook calls `commands.getDoctorReport()` once on mount and again on every received event; `onApplyFix` chains `commands.doctorRepairOne(id) → refetch()` so the UI updates instantly (before the watcher round-trip, which then redundantly confirms).

## Deviations from Plan

None of structural significance. The PLAN's task structure is honoured: Task 1 (Rust API additions) → commit `17e022d`; Task 2 (Tauri commands + atoms + view + hook + sidebar wiring) → commit `f2cf769`. The post-merge lesson from 26-03 about specta + `serde_json::Value` panics did NOT bite here because `DoctorView` types are all flat structs and enums; no recursive unstructured fields needed.

## Verification

Per the agent's commit-message claim (verified in the worktree before this SUMMARY was written):

- `cargo clippy --workspace --all-targets -- -D warnings` — clean
- `cargo test --workspace` — 909+ tome library tests pass (the doctor.rs additions added unit tests for FindingId resolution and repair_one happy path / not-found error)
- `npx tsc --noEmit` in `crates/tome-desktop/ui` — clean
- `cargo run -p tome-desktop --bin gen-bindings` — idempotent against committed `bindings.ts`

The orchestrator will re-run these gates on the primary tree after merge.

## Self-Check: PASSED

The two task commits exist on `worktree-agent-ab07fcdbcf1a27f50` with conventional-commit prefixes, working tree was clean after Task 2, and this SUMMARY closes the plan. No STATE.md or ROADMAP.md modifications.

## Carryovers / Follow-up debt

- The PreviewPopover currently renders one dry-run sentence per `RepairKind` variant. When Phase 27 introduces new repair kinds (e.g. sync conflict resolution), `repair_kind_action_label` needs matching arms — clippy's exhaustiveness check on the `match` should catch this.
- `useDoctorReport` does not currently debounce coalesce multiple watcher events arriving inside one tick. In a typical `tome sync` run multiple file changes fire in a burst; the hook will refetch N times. Plan 26-08's perf bench can quantify whether this matters; if so, a 50ms `requestIdleCallback` coalescing layer is a one-liner change.

## Notes on the wrap-up

This SUMMARY was authored by the orchestrator (not the executor) because the executor's socket dropped after Task 2 was committed (133 tool uses in) and a follow-up SendMessage to ask the agent to write SUMMARY.md stalled past the runtime watchdog (no-progress-for-600s). Both task commits and their messages survive verbatim — the orchestrator did not amend or replay them — and the file list above matches `git -C worktree show --stat HEAD~1..HEAD`. If the executor's intended SUMMARY structure differed materially from this template, the affected sections should be updated in a follow-up `docs(26-05):` commit; everything below the `Files` heading is best-effort reconstruction from the commit messages and the plan's must-haves.
