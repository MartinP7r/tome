# Phase 27: Sync + triage UI - Discussion Log

> **Audit trail only.** Do not use as input to planning, research, or execution agents.
> Decisions are captured in CONTEXT.md — this log preserves the alternatives considered.

**Date:** 2026-06-02
**Phase:** 27-sync-triage-ui
**Areas discussed:** Sync entry point + spatial layout, Progress shape + current-directory indicator, Triage panel design + bulk actions + carryover folding, Failure + cancellation UX

---

## Sync entry point + spatial layout

### Q1 — Where does the Sync UI render in the existing 3-column shell?

| Option | Description | Selected |
|--------|-------------|----------|
| New "Sync" sidebar section | 4th sidebar row alongside Status/Skills/Health. Middle = progress + triage; right = per-skill diff. Toolbar [⌘R Sync] auto-switches to section; badge counts pending decisions. | ✓ |
| Modal/sheet overlay (PreviewPopover-style scaled up) | Transient overlay floats above current section; trigger from toolbar / ⌘R. No permanent UI region. | |
| Persistent ambient progress + section takeover on triage | Mini progress widget in toolbar while pipeline runs (user keeps navigating). Auto-switch to Sync section when triage needed. | |

**User's choice:** New "Sync" sidebar section
**Notes:** Phase 28+ can follow the same pattern (5th/6th sidebar rows). Mail/Notes spatial reference.

### Q2 — Idle state of Sync section?

| Option | Description | Selected |
|--------|-------------|----------|
| Last-sync summary + "Run sync" CTA | Last-sync timestamp + counts + collapsible recent changes; empty state only if never synced. | ✓ |
| Pure CTA + brief explainer | Section is just a launch surface. No history. | |
| Section header IS the trigger | Sidebar row click = run sync (when idle). | |

**User's choice:** Last-sync summary + "Run sync" CTA

### Q3 — Can the user navigate away during pipeline?

| Option | Description | Selected |
|--------|-------------|----------|
| Free nav, sidebar working spinner | Status/Skills/Health remain interactive (NF-05 ensures concurrent-write safety). Cancel button stays on Sync section. | ✓ |
| Free nav + warn on leave with pending triage | Confirm "leave anyway?" if mid-triage. | |
| UI locked to Sync section | Other rows disabled until done. | |

**User's choice:** Free navigation with sidebar working indicator

### Q4 — Trigger placement beyond toolbar?

| Option | Description | Selected |
|--------|-------------|----------|
| Toolbar + ⌘R + Library menu "Sync" | Three discoverable entry points. | ✓ |
| Toolbar + ⌘R only | No menu bar entry. | |
| Toolbar + ⌘R + File menu "Sync" | Less idiomatic than Library. | |

**User's choice:** Toolbar + ⌘R + Library menu

### Q5 — Post-completion view?

| Option | Description | Selected |
|--------|-------------|----------|
| Auto-return to idle + transient toast | Idle view IS the post-sync summary. | ✓ |
| Persistent result summary until dismiss/next run | Third section view. | |
| Result summary with auto-dismiss timer | Magical UI; HIG-discouraged. | |

**User's choice:** Auto-return to idle + transient toast

---

## Progress shape + current-directory indicator

### Q1 — Progress visualisation?

| Option | Description | Selected |
|--------|-------------|----------|
| 6-stage vertical stepper | Mirrors macOS Installer / Xcode build phases; per-stage timing. | ✓ |
| Single bar + stage label + subtitle | One bar with rotating label. | |
| Compact pill strip + active-stage detail | Dense; better for narrow widths. | |

**User's choice:** 6-stage vertical stepper
**Notes:** Honest to the pipeline's actual structure; per-stage timing helps diagnose slow stages.

### Q2 — Current-item subtitle data plumbing?

| Option | Description | Selected |
|--------|-------------|----------|
| Add `item: Option<String>` to `ProgressEvent::SyncStageProgress` | Single variant; free-form string per stage; bindings.ts regen + CI gate. | ✓ |
| Sibling event variant `SyncStageItem { stage, item }` | Two events to correlate; variant proliferation. | |
| Don't change domain — omit subtitle | Conflicts with SC#1 requirement. | |

**User's choice:** Add `item: Option<String>` field

### Q3 — How do GitCloneProgress and BackupSnapshot surface in the stepper?

| Option | Description | Selected |
|--------|-------------|----------|
| Both fold into active stage's `item` subtitle | Uniform stepper rendering; sink formats strings. | ✓ |
| Git-clone as sub-element under Reconcile | Sub-list per repo; non-uniform. | |
| Floating mini-widget for git-clone | Two visual paths. | |

**User's choice:** Fold into active stage subtitle

### Q4 — Show per-stage durations on completed rows?

| Option | Description | Selected |
|--------|-------------|----------|
| Show durations on completed rows | UI records wall-clock at events; no domain change. | ✓ |
| Show only on hover/disclosure | Lower visual weight. | |
| Omit timing display | Simplest; durations don't help most users. | |

**User's choice:** Show durations on completed rows

---

## Triage panel design + bulk actions + carryover folding

### Q1 — Triage list layout?

| Option | Description | Selected |
|--------|-------------|----------|
| Three vertical sections (NEW/CHANGED/REMOVED), grouped by source within each | SectionHeader at both nesting levels; closes Phase 26 carryover #1. | ✓ |
| Flat virtualised list grouped by source, with change-type badges | Filter pills toggle bucket visibility. | |
| Tab/segmented control over change types | Better when one bucket dominates; risk of missing others. | |

**User's choice:** Three vertical sections grouped by source within each

### Q2 — Per-skill inline action affordance?

| Option | Description | Selected |
|--------|-------------|----------|
| Inline chip toggles keep ⇄ disable; right column has full picker | One-click for common; right column for advanced. | ✓ |
| Inline chip read-only; right column is only picker | Strictest detail-pane consistency; more friction. | |
| Inline chip opens dropdown menu | All actions inline; right column = diff only. | |

**User's choice:** Inline toggle + right column full picker

### Q3 — Bulk actions?

| Option | Description | Selected |
|--------|-------------|----------|
| Per-section header + per-source-group header | Two granularities; only NEW gets both. | ✓ |
| Per-source-group only | Forces deliberate scope; more clicks. | |
| Centralised toolbar dropdown | Single dropdown with named presets. | |

**User's choice:** Per-section + per-source-group

### Q4 — "View source" for git skills?

| Option | Description | Selected |
|--------|-------------|----------|
| Reveal cloned repo directory in Finder | Reuses Phase 26 `open_source_folder` command; zero new IPC. | ✓ |
| Open source URL in browser | Useful for upstream inspection; new command needed. | |
| Both — context menu with both | Two affordances per row; decision fatigue. | |

**User's choice:** Reveal cloned repo in Finder

### Q5 — Apply flow / SYNC-03 machine.toml diff preview?

| Option | Description | Selected |
|--------|-------------|----------|
| PreviewPopover anchored to Apply button | Reuses Phase 26 D-09; consistent NF-04 ergonomic. | ✓ |
| Modal sheet with full TOML before/after side-by-side | Bigger surface; new modal pattern. | |
| Inline banner with summary + expandable TOML disclosure | Lightest; preview in same column. | |

**User's choice:** PreviewPopover with inline TOML diff

---

## Failure + cancellation UX

### Q1 — Cancel behavior during pipeline?

| Option | Description | Selected |
|--------|-------------|----------|
| Always-visible Cancel button + immediate cancel + safe stage boundary | SC#4's consistency guarantee makes confirm dialogs zero-value. | ✓ |
| Always-visible Cancel + confirm if past Distribute stage | Late-cancel friction surfaces practical cost. | |
| Confirm dialog on every cancel | Maximum protection; risk of dialog fatigue. | |

**User's choice:** Always-visible Cancel + immediate cancel

### Q2 — Stepper after cancel or failure?

| Option | Description | Selected |
|--------|-------------|----------|
| Stepper transforms in place — failed/cancelled stage shows error icon + message | One coherent surface; same component for live + terminal. | ✓ |
| Replace stepper with dedicated outcome panel | Cleaner transition; loses at-a-glance continuity. | |
| Toast notification + stepper auto-clears to idle | Risk of missing failure detail. | |

**User's choice:** Stepper transforms in place

### Q3 — Retry strategy?

| Option | Description | Selected |
|--------|-------------|----------|
| Domain returns `retry_from: Option<SyncStage>` hint; UI renders single button | No JS-side business logic; Rust knows safety rules. | ✓ |
| UI offers both "Retry from failed stage" AND "Start over from Reconcile" | User picks; risk of picking unsafe option. | |
| Always show single "Retry sync" button (full re-run) | Simplest; ignores SC#5 "where possible". | |

**User's choice:** Domain-driven retry hint

### Q4 — Partial-failure rendering (SAFE-01 K-failures)?

| Option | Description | Selected |
|--------|-------------|----------|
| Stage shows ✓ + amber `[⚠ K issues]` badge, expandable per-operation list | Honors Phase 26 D-11 without alarming about successful sync. | ✓ |
| Always demote stage icon to `!` if any operation failed | Easiest rule; ignores genuine completion. | |
| Stage stays ✓; failures only in post-sync summary panel | Decouples stepper from failure reporting. | |

**User's choice:** ✓ + amber badge + expandable list

---

## Claude's Discretion

The following calls were left to Claude (captured in CONTEXT.md `<decisions>` → "Claude's Discretion" subsection):

- Stage label wording (plain-English labels with `SyncStage` variant as internal identity)
- Toast positioning, duration, dismissal mechanics
- Sidebar working-spinner visual style
- TOML diff exact rendering inside PreviewPopover (line-numbering, colors, font)
- Default expansion state of triage section headers (NEW expanded; CHANGED/REMOVED collapsed)
- `[Retry failed items]` exact scope (per-operation retries within stage)
- Stage-duration display format (`0.1s` / `8.2s` / `1m 14s`, right-aligned)
- Stepper layout responsiveness (vertical stack at full Sync-column width; no horizontal collapse needed)
- `item: Option<String>` exact emission for git-clone fold-in (sink-side formatting)
- `retry_from` hint placement in IPC type system (`SyncOutcome` wrapper vs. on `TomeError`)

## Deferred Ideas

The following were noted but explicitly OUT of Phase 27 scope (preserved in CONTEXT.md `<deferred>`):

- Opening upstream URLs in browser for git-sourced skills (alpha = Finder only)
- Bulk "Retry all failed items" with selective per-item retry granularity
- `CHANGED` bulk-disable action
- Sync activity log / sync history view
- SKILL.md content diff rendering for CHANGED skills
- Real-time auto-sync on watcher events (out-of-scope per v1.0 REQUIREMENTS.md)
- STATE.md staleness fix (separate one-line commit)
- CLAUDE.md "Current State" header staleness (carry-over from Phase 26 deferred items)
- Interim v0.17.0 release of #542 SkillOwnership migration (optional pre-v1.0)
