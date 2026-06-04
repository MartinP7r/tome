---
phase: 27
slug: sync-triage-ui
status: draft
shadcn_initialized: false
preset: none
created: 2026-06-04
---

# Phase 27 — UI Design Contract

> Visual and interaction contract for the **Sync + triage UI** of the tome Desktop GUI
> (Tauri 2 + React 19, macOS only). Pre-populated from `27-CONTEXT.md` (D-01..D-20),
> `REQUIREMENTS.md` (SYNC-01..05, NF-04 spirit, NF-05), and `ROADMAP.md` Phase 27 §SC#1..6.
>
> **Inheritance contract:** Phase 27 inherits the full Phase 26 design system **unchanged**:
> tokens (`tokens.css`), typography (4 sizes × 2 weights), color (60/30/10 split + reserved-for
> list), shell (`Window`/`Titlebar`/`Sidebar`/`NavItem`/`ContentPane`), and every atom + molecule
> already shipped at `crates/tome-desktop/ui/src/components/`. Phase 26 D-13..D-16 are
> carry-forward locked decisions per CONTEXT.md.
>
> **What this phase adds:** five sync-specific molecules (`StageStepper`, `StageRow`,
> `TriagePanel`, `TriageRow`, `MachineTomlDiff`), three sync-specific states (idle,
> in-progress, terminal-overlay), a sync-scoped copywriting block, and three new keyboard
> shortcuts (`⌘R`, `⌘.`, `⌘⇧A`).
>
> **What this phase does NOT touch:** existing Phase 26 components are reused verbatim
> (`SectionHeader`, `PreviewPopover`, `FindingRow`, `Badge`, `Button`, `PopupMenu`). Their
> shape and styling do not change. Two Phase 26 carryovers close inside this contract:
> `SectionHeader` gets wired (it exists but is currently unwired) and `synced_at` plumbing
> lands on `DiscoveredSkill` (purely a domain extension — no UI surface change beyond Skills
> view's existing Sort=Recent menu item).

---

## Design System

| Property | Value | Source |
|----------|-------|--------|
| Framework | React 19 | D-GUI-04 (locked Phase 25) |
| Component primitives | React Aria (Adobe headless) | Phase 26 D-14 — inherited |
| Virtualisation | React Aria native `<Virtualizer>` (TanStack Virtual fallback) | Phase 26 revision 2 — inherited |
| Styling | CSS Modules + CSS custom-property tokens (`tokens.css`) | Phase 26 D-15 — inherited |
| Theme mode | System-driven via `prefers-color-scheme` — no in-app switcher | Phase 26 D-16, NF-03 — inherited |
| Icon set | SF Symbols equivalents as inline SVG (single library, single weight) | Phase 26 D-13 — inherited |
| Font | macOS system stack — `-apple-system, "SF Pro Text", "SF Pro Display", system-ui, sans-serif` | Phase 26 D-13 — inherited |
| Window chrome | Tauri 2 unified native titlebar + traffic lights + vibrancy sidebar + solid fallback on `prefers-reduced-transparency` | Phase 26 D-16 — inherited |
| Tool | **none** (no shadcn — incompatible with D-14/D-15 stack) | Phase 26 shadcn-gate result — inherited |
| Preset | not applicable | — |
| Diff rendering library | **none** — small hand-rolled line-diff component (input is already structured: current+proposed TOML strings) | D-15 "Claude's Discretion" |

**Why no shadcn (carry-forward).** The Phase 26 contract locked React Aria + CSS Modules. shadcn requires Tailwind + Radix; introducing it would conflict with both the styling layer and the headless-primitive layer. The shadcn-initialisation gate from the researcher workflow remains **non-applicable**. Registry safety: not applicable.

---

## Spacing Scale

4px base grid (macOS HIG aligned). All values multiples of 4. **Inherited verbatim from Phase 26.**

| Token | Value | Usage (Phase 27 additions in **bold**) |
|-------|-------|----------------------------------------|
| `--space-1` | 4px | Icon ↔ label gaps, badge inner padding, **stepper-row chevron offset** |
| `--space-2` | 8px | Compact rows, button inner gap, **stepper connector-line gap** |
| `--space-3` | 12px | List-row vertical padding, **triage-row vertical padding**, **stage-row vertical padding** |
| `--space-4` | 16px | Default content padding, **section gap between stepper and triage panel** |
| `--space-5` | 20px | Detail-pane header bottom margin, popover padding |
| `--space-6` | 24px | View padding-top, major section gap, **stepper outer padding** |
| `--space-8` | 32px | Page-level gap between large groups, **idle-state hero offset** |
| `--space-12` | 48px | Empty-state centring offset, **idle-state CTA vertical offset** |

**Phase 27 spacing exceptions (added to Phase 26's set):**

- **Stage-row height:** **40px** (Mail-list density; single-line icon + label + duration + spinner).
- **Stage-row icon column width:** **24px** (centred 16×16 SVG; left-aligned with the row baseline).
- **Stepper connector line:** **2px** wide, between stage-row icon centres (24px column → centred at 12px from the row's left edge).
- **Triage section indent levels:** outer (NEW/CHANGED/REMOVED) at **0px**; inner (source-group) at **20px**.
- **`MachineTomlDiff` popover max-width:** **480px** (wider than Phase 26's 320px default; TOML diffs need horizontal room for indented keys). Max-height **360px** with internal scroll.

Confirm during 27-01 (progress channel + UI) and 27-03 (machine.toml preview).

---

## Typography

**4 sizes · 2 weights.** Inherited verbatim from Phase 26. No new tokens, no new weights.

| Role | Token | Size | Weight | Line Height | Usage (Phase 27 additions in **bold**) |
|------|-------|------|--------|-------------|---------------------------------------|
| Small | `--text-small` | 12px | 400 **or** 600 | 1.4 | Caption labels (uppercase, 600); footnote/secondary (400); **stage-row duration**, **`[Retry from <stage>]` button label**, **TOML diff line-numbers** |
| Body | `--text-body` | 13px | 400 **or** 600 | 1.5 | Default body; **stage-row label**, **triage-row primary**, **CTA `Run sync`**, **`Cancel sync` button**, **`Apply N decisions` button**, **partial-failure summary line** |
| Subhead | `--text-subhead` | 16px | 600 | 1.3 | Markdown H2; **idle-state heading `Last synced …`**, **terminal-state heading `Sync complete with K issues`** |
| Title | `--text-title` | 22px | 600 | 1.2 | View title — **`Sync`** (centred in `Titlebar`) |

Monospace family (TOML diff body, content hashes, paths): `ui-monospace, "SF Mono", Menlo, monospace` at `--text-small` (12px, weight 400). **Used in `MachineTomlDiff` body and `TriageRow` content-hash before/after spans.**

### Phase 27 weight bindings (additions to Phase 26's exhaustive list)

**Weight 400 (regular)** — body text, neutral information, controls without emphasis:

- `StageRow` label (e.g. "Reconcile", "Discover", "Consolidate", "Distribute", "Cleanup", "Save") — `--text-body` 13px / 400 when stage is pending or active; recoloured but same weight when complete / failed.
- `StageRow` `current_item` subtitle (e.g. "git: my-repo (4.2 MB)", "claude-plugins/axiom-build") — `--text-small` 12px / 400, `--label-secondary`, monospace when value is a path.
- `StageRow` duration text (e.g. "0.3s", "8.2s", "1m 14s") — `--text-small` 12px / 400, `--label-secondary`, right-aligned in the row.
- `TriageRow` secondary (`source · managed|local · synced 3m ago`) — `--text-small` 12px / 400, `--label-secondary`.
- `TriageRow` inline action chip default state (`[keep]` text) — `--text-small` 12px / 400 (chip's own weight; the icon is decorative).
- `MachineTomlDiff` unchanged line text — `--text-small` 12px / 400, monospace, `--label-primary`.
- `MachineTomlDiff` line-number gutter — `--text-small` 12px / 400, monospace, `--label-secondary`.
- Idle-state body sub-line (e.g. "0 new · 0 changed · 0 removed since last sync") — `--text-body` 13px / 400, `--label-secondary`.
- `Button--secondary` labels: **"Cancel sync"**, **"Dismiss"**, **"View source"** (in TriageRow's right-column action picker).
- `Button--small` partial-failure retry label: **"Retry failed items"** — `--text-small` 12px / 600 (matches Phase 26's `Button--small-fix`).

**Weight 600 (semibold)** — titles, captions in uppercase, primary actions, emphasis:

- View title `Sync` (`--text-title` 22px / 600) in `Titlebar`.
- `Titlebar` centre text `tome — Sync` — `--text-body` 13px / 600 (matches Phase 26 Titlebar binding).
- Uppercase caption labels `--text-small` 12px / 600 for: **`NEW`**, **`CHANGED`**, **`REMOVED`** (outer `SectionHeader`s in `TriagePanel`); **`PREVIEW`** (already locked in Phase 26 `PreviewPopover`).
- Inner-section source-group headers (e.g. **`PLUGINS`**, **`MY-REPO`**, **`UNOWNED`**) — `--text-small` 12px / 600 uppercase, `--label-secondary`.
- `SectionHeader` count chips `(N)` — `--text-small` 12px / 600.
- `StageRow` label when row is **active** or **failed** — `--text-body` 13px / 600 (signals "this is the row you care about right now"). Pending and completed rows stay at 400.
- `TriageRow` primary (skill name) — `--text-body` 13px / 600 when selected (matches Phase 26 `SkillListRow` selection convention); 400 otherwise.
- `Button--primary` labels: **"Run sync"** (idle-state CTA), **"Apply N decisions"** (triage panel CTA) — `--text-body` 13px / 600.
- Inline `[ErrorCode]` chip text in `StageRow` failed state — `--text-small` 12px / 600 (matches Phase 26 `FindingRow` `[Code]` convention).
- Idle-state heading "**Last synced 4 minutes ago**" — `--text-subhead` 16px / 600, `--label-primary`.
- Terminal-state summary heading "**Sync complete with K issues**" / "**Sync failed**" / "**Sync cancelled**" — `--text-subhead` 16px / 600.

Any unlisted Phase 27 text role defaults to **400** unless it's a title, uppercase caption, primary-action label, emphasised state label, or heading.

---

## Color

CSS custom properties driven by `prefers-color-scheme`. **All tokens inherited verbatim from Phase 26** (`tokens.css`). No new colour tokens introduced.

### 60 / 30 / 10 split — extended accent reserved-for list

The Phase 26 reserved-for list adds **two Phase 27 elements** (items 6 and 7 below) — both consistent with the existing "selected fill" and "primary-action background" categories. No new accent surface; the inventory grows by two items within existing categories.

**Accent reserved-for list (the complete 10% inventory across Phase 26 + 27):**

1. Selected list row fill (Skills list, Health finding rows when keyboard-focused).
2. Selected sidebar nav-item capsule fill — **including the new `Sync` nav-item** (D-01).
3. Primary-button background ("Disable on this machine", "Apply" in preview popover, **"Run sync"** (D-03 idle CTA), **"Apply N decisions"** (D-15 triage CTA)).
4. Inline links inside the markdown body.
5. Focus ring on every interactive element (React Aria default focus ring, recoloured to `--accent`).
6. **Stepper active-stage spinner colour** (D-07) — `--accent`; the only non-text use of accent on the stepper, signals "this is the live work".
7. **Selected `TriageRow` fill** (D-12) — `--accent` background, `--accent-on` foreground; identical pattern to `SkillListRow` selection.

Never used for: section dividers, body text, badge backgrounds (other than `Badge--managed` blue tint), the connector line in the stepper (which uses `--separator`), the `MachineTomlDiff` red/green line backgrounds (which use the danger/success token families at low alpha — see below).

### Phase 27 semantic-token uses (within the existing palette)

| Token | Phase 27 use |
|-------|--------------|
| `--success` | StageRow `✓` icon when complete; idle-state checkmark glyph; `Pill — Updated` after a successful sync writes new manifest entries; in-sync StatusDot (carry-forward). |
| `--danger` | StageRow `!` icon when failed; `[ErrorCode]` chip text; `MachineTomlDiff` removed-line gutter glyph (`−`); terminal-state "Sync failed" heading colour; sidebar `Sync` row badge fill when last sync had unresolved failures. |
| Amber semantic — uses `--badge-override-fg` token family (`#a35d00` light / `#e0a060` dark — from `Badge--override`) | StageRow `⊘` icon when cancelled; `[⚠ K issues]` badge on a partially-failed stage; terminal "Sync cancelled" heading colour. **No new token introduced** — reusing the override-badge family because the visual intent is the same: "attention, but not a failure". |
| `--label-secondary` | StageRow pending state (dim outline circle, dim label); future-stage rows in the stepper; "—" not-run glyph. |

### `MachineTomlDiff` line-background tokens (D-15)

The diff body uses two low-alpha fills bound to existing semantic tokens. No new tokens.

| Line type | Light fill | Light foreground | Dark fill | Dark foreground |
|-----------|-----------|------------------|-----------|-----------------|
| Removed (red, gutter `−`) | `rgba(255,59,48,0.08)` (`--danger` at 8%) | `--label-primary` | `rgba(255,69,58,0.16)` | `--label-primary` |
| Added (green, gutter `+`) | `rgba(40,200,64,0.10)` (`--success` at 10%) | `--label-primary` | `rgba(48,209,88,0.16)` | `--label-primary` |
| Unchanged | `--bg-window` | `--label-primary` | `--bg-window` | `--label-primary` |

Gutter glyphs (`−` / `+` / space) are weight 600, sized at `--text-small`, monospace family, drawn in `--danger` / `--success` / `--label-secondary` respectively.

---

## Component Contract

For every new component below: purpose, props/variants, React Aria primitive, VoiceOver label expectation, and token bindings. Built as `*.module.css` per Phase 26 D-15. Existing Phase 26 components are reused verbatim; their contracts are not restated.

### Shell additions (inherits Phase 26 shell unchanged)

#### Updated `Sidebar` — new `NavItem` row (D-01)

The existing `Sidebar` component (`crates/tome-desktop/ui/src/components/Sidebar.tsx`-equivalent — currently inline in the shell) gains a fourth `NavItem` between `Skills` and `Health`:

```
LIBRARY
  Status
  Skills
  Sync               ← new row (D-01)
  Health (3)
```

- **`NavItem` instance:** `section: 'Sync'`, icon = SF-shaped "arrow.triangle.2.circlepath" equivalent (filtered from the chosen Phase 26 SF-shaped library).
- **Three variants (Phase 27-specific):**
  - **Default** — same as any other NavItem. Decorative arrow-circular icon + label.
  - **In-progress** — inline small system spinner (`--label-secondary`, ~12×12) replaces the icon while a sync is running (D-04). Label stays "Sync"; tooltip "Sync in progress".
  - **Badge** — trailing count chip (reuses the same `Badge` primitive Phase 26's Health row uses). **Two distinct meanings depending on pipeline state**:
    - **Pre-sync (changes pending)** — `--badge-managed` family blue tint (`#e4eefb` / `#1f5fb8`), `--text-small` 12px / 600. Count = `new + changed + removed` (D-05). Cleared by Apply.
    - **Post-sync with unresolved partial failures** — `--danger` fill, `--accent-on` text (matches Phase 26 Health-badge convention). Count = unresolved-failed-items remaining after Dismiss / Retry attempts (D-20).
  - The two badge meanings are **mutually exclusive** — a sync run always clears the pre-sync count first (Apply writes machine.toml), so the failure-state badge only appears *after* a run completes with partial failures.
- **Selection:** when the user is in the Sync section, the row carries the standard `--accent` capsule fill (item 2 of the reserved-for list). The in-progress spinner is rendered on `--accent-on` colour while selected.
- **A11y:** `aria-label="Sync, Sync section${inProgress ? ', sync in progress' : ''}${badgeCount > 0 ? `, ${badgeCount} ${badgeMeaning}` : ''}"`. `badgeMeaning` is either `"pending changes"` or `"unresolved sync failures"` depending on which badge variant is rendered.
- **Keyboard:** `⌘4` jumps to Sync (added to keyboard map). Phase 26's `⌘1`/`⌘2` stay on Status/Skills; `⌘3` now jumps to **Sync** (the third sidebar item between Skills and Health) — **NO**, this would break the Phase 26 contract. Re-anchor: `⌘1`/`⌘2`/`⌘3`/`⌘4` map by position. With Sync inserted between Skills and Health, the new bindings are: `⌘1` Status, `⌘2` Skills, `⌘3` Sync, `⌘4` Health. **Phase 26's `⌘3 → Health` binding is updated by Phase 27** — this is the one breaking change to the Phase 26 keyboard map and is documented in the keyboard map section below. The planner adds a release-note line.

#### `ContentPane` (Sync section) — two variants

The existing `ContentPane` component gets two new variant compositions for the Sync section:

- **`split` (D-01)** — when a sync is in progress OR triage decisions are pending: middle column = stepper + triage list; right column = per-skill diff detail (TriageRow detail).
- **`single-pane`** — idle state and terminal state (failure / cancellation overlay; see "Per-view Design").

The 3-column shell stays unchanged; the Sync section composes inside the existing layout.

### New molecules (Phase 27 net-new)

#### `StageStepper` (D-07, D-09, D-10, D-18, D-19, D-20)

- **Purpose:** Vertical 6-stage progress visualisation. Mirrors macOS Installer / Xcode build-phases idiom (per CONTEXT.md §Specifics). Renders live progress AND terminal state in place (no separate "outcome panel"; D-18).
- **Props:**
  ```ts
  interface StageStepperProps {
    stages: StageState[];   // exactly 6 entries, indexed by SyncStage::ALL
    onCancel?: () => void;  // only present when at least one stage is `active`
    onDismiss?: () => void; // only present in terminal state
    onRetryFromStage?: (stage: SyncStage) => void; // only present when terminal + retry_from is Some
  }
  type StageStatus =
    | { kind: 'pending' }
    | { kind: 'active'; currentItem: string | null; current: number; total: number }
    | { kind: 'complete'; durationMs: number; partialFailures: PartialFailure[] }   // partialFailures.length > 0 → amber [⚠ K issues] badge (D-20)
    | { kind: 'failed'; durationMs: number; error: TomeError }
    | { kind: 'cancelled' };
  ```
- **Layout intent:**
  - Outer container: vertical stack at full middle-column width, padding `--space-6` (24px), gap `--space-3` (12px) between rows.
  - Each `StageRow` is 40px tall; rows connected by a 2px vertical line in `--separator` (drawn between the centres of adjacent stage icons).
  - **Trailing slot above the stepper:** when at least one stage is `active`, a right-aligned `Button--secondary` labelled **"Cancel sync"** (D-17) is rendered above the stepper outer container. When terminal state, two right-aligned buttons appear: `[Dismiss]` (always) + `[Retry from <stage>]` (D-19, only when `onRetryFromStage` is provided).
- **A11y:**
  - Container: `role="list"`, `aria-label="Sync pipeline progress"`.
  - Each `StageRow` is a `role="listitem"`.
  - State changes announce via the `role="status"` `aria-live="polite"` region that wraps the stepper — verbatim announcements per the Copywriting section below.

#### `StageRow` (composed inside `StageStepper`)

- **Purpose:** Single stage line in the stepper. Renders icon + label + duration / spinner / item subtitle / [ErrorCode] chip + partial-failure summary disclosure as state dictates.
- **Variants:** map 1:1 to `StageStatus.kind`.
- **Layout (per-variant):**
  - **pending** (`{ kind: 'pending' }`):
    `[○ outline] [label]                              [—]`
    Icon: 16×16 outline circle (`--label-secondary`). Label `--text-body` 13px / 400, `--label-secondary`. Right-aligned: "—" (`--text-small` 12px / 400, `--label-secondary`).
  - **active** (`{ kind: 'active'; currentItem; current; total }`):
    `[● spinner] [label]                           [running…]`
    `              [currentItem]   [bar  47/120]`
    Icon: small system spinner, `--accent`. Label `--text-body` 13px / **600**, `--label-primary`. Right-aligned top: "running…" (`--text-small` 12px / 400, `--label-secondary`). Below the label, indented by 24px (icon-column width): `currentItem` (`--text-small` 12px / 400, `--label-secondary`, monospace when value is a path or `"git: …"` format; truncate-middle with ellipsis) followed by an inline progress bar (`current / total`) rendered with `--accent` fill on `--bg-subtle` track. **When `total = 0`** (e.g., git-clone with unknown size — D-09): bar is hidden, item text alone fills the row. **When `currentItem === null`** (transient between items): subtitle line collapses.
  - **complete** (`{ kind: 'complete'; durationMs; partialFailures }`):
    `[✓] [label]                       [duration] [⚠ K issues]?`
    Icon: 16×16 `✓` checkmark, `--success`. Label `--text-body` 13px / 400, `--label-primary`. Right-aligned: duration text (e.g., "0.3s" / "8.2s" / "1m 14s" — format rule below) `--text-small` 12px / 400, `--label-secondary`. **If `partialFailures.length > 0`** (D-20): an amber `[⚠ K issues]` `Badge--override` chip renders to the right of the duration; the row expands by default to render a nested `FindingRow` list (`partialFailures.map(f => <FindingRow … />)`) below the row's primary line. Each FindingRow uses the Phase 26 contract verbatim — `[ErrorCode] message` + `▶ Show context` disclosure (Phase 26 D-11). Trailing slot below the per-item FindingRow list: a `Button--small` labelled **"Retry failed items"** (D-20) — only renders when `partialFailures` is non-empty.
  - **failed** (`{ kind: 'failed'; durationMs; error }`):
    `[!] [label]                       [duration]`
    `      [TomeErrorCode] message  [▶ Show error chain]`
    Icon: 16×16 `!` glyph, `--danger`. Label `--text-body` 13px / **600**, `--danger`. Duration as above. Below the label, indented 24px: a single `FindingRow`-shaped inline failure — `[ErrorCode]` chip (`--text-small` 12px / 600, `--danger`) + message (`--text-small` 12px / 400, `--label-primary`) + `▶ Show error chain` disclosure that lists the `context: Vec<String>` chain on expand (Phase 26 D-11).
  - **cancelled** (`{ kind: 'cancelled' }`):
    `[⊘] [label]                       [cancelled]`
    Icon: 16×16 `⊘` glyph, amber (`--badge-override-fg`). Label `--text-body` 13px / 400, `--label-secondary`. Right-aligned: "cancelled" `--text-small` 12px / 400, amber.
- **Duration format rule** (CONTEXT.md "Claude's Discretion"):
  - `< 1000ms` → `"0.3s"` (1 decimal).
  - `1000ms ≤ duration < 60_000ms` → `"8.2s"` (1 decimal).
  - `≥ 60_000ms` → `"1m 14s"` (whole seconds).
  Numbers right-aligned for vertical scanning.
- **A11y:** Each StageRow carries `aria-label="${label} stage, ${status describe}"` where `status describe` is one of:
  - pending → "pending"
  - active → "running, ${currentItem || 'preparing'}${total > 0 ? `, ${current} of ${total}` : ''}"
  - complete → "complete in ${durationText}${K > 0 ? `, ${K} issues` : ''}"
  - failed → "failed in ${durationText}, ${ErrorCode}, ${message}"
  - cancelled → "cancelled"

#### `TriagePanel` (D-11, D-12, D-13)

- **Purpose:** Sectioned list of pending lockfile-diff decisions. Three outer sections (NEW / CHANGED / REMOVED), each with inner source-group sections.
- **Props:**
  ```ts
  interface TriagePanelProps {
    diff: LockfileDiff;                          // from get_lockfile_diff command
    decisions: Map<SkillName, TriageDecision>;   // controlled state
    onDecisionChange: (skill: SkillName, decision: TriageDecision) => void;
    selectedSkill: SkillName | null;
    onSelect: (skill: SkillName | null) => void;
    onBulkAction: (scope: BulkScope, decision: TriageDecision) => void;
    onApply: () => void;                          // opens PreviewPopover anchored to the [Apply] button
  }
  type TriageDecision = 'keep' | 'disable';     // 'remove' is implicit for REMOVED section per D-13
  type BulkScope =
    | { kind: 'section'; section: 'new' }       // D-13: only NEW carries section-level bulk
    | { kind: 'source-group'; section: 'new'; source: DirectoryName }; // D-13: source-group bulk on NEW only
  ```
- **Layout intent:**
  ```
  ┌─TriagePanel (middle column under Stepper)─────────────┐
  │ ▼ NEW (8)                                  [Disable all new]│
  │   ▼ PLUGINS (5)                       [Disable all new from plugins]│
  │     ○ axiom-build                          [✓ keep]    │
  │     ○ axiom-concurrency                    [✓ keep]    │
  │     …                                                  │
  │   ▼ MY-REPO (3)                       [Disable all new from my-repo]│
  │     …                                                  │
  │ ▶ CHANGED (3)                                          │
  │ ▶ REMOVED (1)                                          │
  │ ──────────────────────────────────────────────────────│
  │                          [Apply 8 decisions]           │
  └────────────────────────────────────────────────────────┘
  ```
- **Defaults (CONTEXT.md "Claude's Discretion"):** `NEW` expanded by default (most actionable); `CHANGED` and `REMOVED` collapsed (user expands if interested).
- **SectionHeader reuse — D-11 carryover closure:** the existing Phase 26 `SectionHeader` component is used at **both nesting levels**.
  - **Outer (NEW / CHANGED / REMOVED)** — `--text-small` 12px / 600 uppercase, `--label-secondary`, count chip `(N)` trailing, plus a trailing-right `Button--secondary` bulk-action when applicable (only `NEW` carries one per D-13). Wraps in `<h2>` for VoiceOver headings rotor.
  - **Inner (source-group e.g. PLUGINS, MY-REPO, UNOWNED)** — same styling but at 20px indent, wraps in `<h3>` for proper nesting. Trailing-right `Button--secondary` "Disable all new from \<source\>" only on inner headers inside the `NEW` outer section (D-13).
- **TriageRow** (composed inside; see next entry).
- **`[Apply N decisions]` button:** `Button--primary` at the bottom-right of the panel; label uses live count of decisions where `decision !== default`. Clicking opens `PreviewPopover` anchored to the button (D-15) — content = `MachineTomlDiff`. Disabled (`aria-disabled="true"`, 50% opacity) when no non-default decisions exist (clicking would be a no-op).
- **A11y:** `<section aria-label="Triage decisions">`. Outer SectionHeaders are `<h2>`; inner are `<h3>`. Each TriageRow within is a `role="option"` inside the section's `role="listbox"`. Keyboard nav inherits Phase 26 `SkillListRow` patterns (↑/↓/Home/End/PgUp/PgDn).

#### `TriageRow`

- **Purpose:** Middle-column row inside `TriagePanel`. Two-line layout: skill name primary, source · provenance · synced-at secondary. Trailing chip = inline decision toggle (D-12).
- **Props:**
  ```ts
  interface TriageRowProps {
    skill: SkillName;
    change: SkillChange;            // Added | Changed | Removed — drives icon, prefix, and disabled state
    decision: TriageDecision;
    onDecisionToggle: () => void;   // keep ⇄ disable inline toggle (D-12)
    isSelected: boolean;
    onSelect: () => void;
  }
  ```
- **Layout:**
  - Row height: **52px** (matches Phase 26 `SkillListRow` rhythm).
  - **Primary** (line 1): skill name `--text-body` 13px / **400** default, **600** when selected (matches Phase 26).
  - **Secondary** (line 2): `${source} · ${managed ? 'managed' : 'local'} · synced ${relativeTime}` — `--text-small` 12px / 400, `--label-secondary`.
  - **Trailing slot:** inline action chip showing current decision — clickable affordance.
    - `[✓ keep]` when `decision === 'keep'` — `--text-small` 12px / 400, `--label-secondary` background.
    - `[⊘ disabled here]` when `decision === 'disable'` — `--text-small` 12px / 400, amber (`--badge-override-fg`).
    - **Inline toggle (D-12):** click cycles keep → disable → keep. The chip is the **only inline action**; the right-column detail pane carries the full radio picker (see `TriageDetail`).
    - For `REMOVED` rows the chip reads `[implicit remove]` and is non-interactive — `--text-small` 12px / 400, `--label-secondary`, 50% opacity (D-13 invariant: REMOVED has no user decision).
  - **Selected state:** `--accent` background fill, `--accent-on` text (item 7 of the accent reserved-for list). Trailing chip recolours to `rgba(255,255,255,0.18)` background, `--accent-on` foreground.
- **A11y:** `role="option"`. `aria-label="${name}, ${change.kind === 'added' ? 'new' : change.kind === 'changed' ? 'changed' : 'removed'} from source ${source}, decision: ${decision}"`. Selection announces "selected" via React Aria ListBox semantics.

#### `TriageDetail` (right column when a `TriageRow` is selected)

- **Purpose:** Per-skill diff detail + canonical full action picker (D-12). Mirrors Phase 26's `DetailHeader`+body composition but with diff-specific content.
- **Layout (top-to-bottom):**
  1. Row 1: skill name `--text-title` 22px / **600** + trailing `Badge` for change-kind (`Badge--type-git` family — `New` / `Changed` / `Removed`).
  2. Row 2: metadata grid — three labelled cells (reuse `KeyValueRow`-style):
     - **SOURCE** — directory name, monospace.
     - **CONTENT HASH** — for `Changed`: `sha256:abc1…  →  sha256:def4…` rendered in two monospace runs with an arrow glyph between (`--text-small` 12px / 400). For `Added` / `Removed`: single hash (monospace).
     - **SYNCED** — relative time (e.g., "synced 2 minutes ago" for `Changed`; "—" for `Added`; "last synced 12 days ago" for `Removed`).
  3. Row 3: **canonical action picker** — React Aria `RadioGroup` (label "Decision"), three or two radios depending on change kind and source:
     - For `Added` / `Changed` with **any** source: `(●) Keep this skill`  `(○) Disable on this machine`.
     - **Additional row if source is git-backed:** `(○) View source` — selecting this radio fires `commands.openSourceFolder(skill)` immediately (it's an action disguised as a radio that doesn't actually mutate `decision`). On fire, radio bounces back to the previously-selected decision. Tooltip: "Reveals the git-cloned repo in Finder."
     - For `Removed`: picker is **omitted** — instead, an `--text-small` 12px / 400, `--label-secondary` line reads **"This skill will be removed from the lockfile. No action required."**
  4. Row 4 (only when `Changed`): collapsed disclosure **"Show diff metadata"** → on expand, shows old vs new `registry_id`, `version`, `git_commit_sha` (if any) as KeyValueRow-style entries.
- **Empty selection** (no TriageRow selected): same neutral centred placeholder pattern as Phase 26 Skills view; copy: **"Select a change to view details"**.
- **A11y:** `aria-label="${skillName} change details"`. RadioGroup `aria-label="Decision for ${skillName}"`. Each radio has explicit label per copy block in Copywriting.

#### `MachineTomlDiff` (D-15 — slotted inside Phase 26 `PreviewPopover`)

- **Purpose:** Renders the structured `MachineTomlPreview` returned by the `preview_machine_toml` command as a left-aligned line-by-line TOML diff inside the existing Phase 26 `PreviewPopover`. **Reuses `PreviewPopover` verbatim** (D-15) — same outer shell as Doctor's Fix flow. This component only renders the *content slot* of the popover.
- **Props:**
  ```ts
  interface MachineTomlDiffProps {
    preview: MachineTomlPreview; // { lines: Vec<DiffLine> } where DiffLine = { lineNumber: number; kind: 'removed' | 'added' | 'unchanged'; content: string }
  }
  ```
- **Layout (slotted into `PreviewPopover`):**
  - **Popover width:** 480px (overrides the Phase 26 320px default for this content variant — declared in spacing exceptions).
  - **Popover layout (existing PreviewPopover shell):**
    1. PREVIEW caption (Phase 26 contract, unchanged).
    2. **Slot content (this component):**
       - Header line: `${addedLineCount} additions, ${removedLineCount} removals` — `--text-small` 12px / 400, `--label-secondary`.
       - Scrollable diff body (max-height 360px):
         - Each line renders in monospace (`--text-small` 12px / 400).
         - Three-column grid: line-number gutter (right-aligned, `--label-secondary`, 32px), change-glyph gutter (`−` / `+` / ` `, weight 600, in `--danger` / `--success` / `--label-secondary`, 16px), content (left-aligned, `--label-primary`).
         - Row background per line-type per the `MachineTomlDiff` colour table above.
         - Long lines wrap (no horizontal scroll inside this component); the popover's max-width + max-height handle the bounds.
    3. Helper text (existing PreviewPopover slot): **"Applying writes `~/.config/tome/machine.toml`. The CLI sees this change immediately."** — `--text-small` 12px / 400, `--label-secondary`.
    4. Button row (existing): `[Cancel]` (secondary) `[Apply]` (primary). On `Apply` → fires `apply_machine_toml` command; success closes popover, watcher fires `MachinePrefsChanged`, idle-state refreshes for free. Error → popover stays open and renders a `--danger`-bordered banner above the button row with `[ErrorCode] message` + Show-context disclosure (same pattern as Phase 26 doctor Fix-failed).
- **A11y:** Inherits the `PreviewPopover` `role="dialog"` `aria-modal="true"`. The diff body itself is wrapped as `<table role="table" aria-label="machine.toml diff, ${added} additions, ${removed} removals">`. Each diff line is a `<tr role="row">` with three `<td>`s; the change-glyph `<td>` carries `aria-label="removed line"` / `"added line"` / `"unchanged line"`. VoiceOver reads "removed line 14: enabled = ['axiom-build']" — the line-number gutter doubles as positional context for screen readers.

### Phase 26 components reused unchanged

These ship as-is; the contracts do not change. Phase 27 only **uses** them.

- `SectionHeader` (D-11 carryover closure) — wired into `TriagePanel` outer + inner; also retroactively wired into `SkillListView` for VIEW-02 group-by Source / Role (closes Phase 26 carryover #1).
- `PreviewPopover` (D-15) — slot content = `MachineTomlDiff` for Apply; same shell as Doctor's Fix.
- `FindingRow` (D-18, D-20) — inline failure rendering inside `StageRow` (failed + partial-failure variants).
- `Badge` (D-05, D-20) — Sync nav-item badge (managed-blue variant for pending; danger variant for unresolved failures); `Badge--override` amber variant for `[⚠ K issues]` partial-failure chip.
- `Button` (`primary` / `secondary` / `small`) — every CTA. No new variants.
- `PopupMenu` — not used in Phase 27 (no sort/group menu in the triage panel; the structure is fixed).

### States — Phase 27 specific

#### Idle state (D-03, D-06)

- **Layout:** `ContentPane` `single-pane`. Centred hero composition with `--space-12` top offset:
  1. Glyph (32×32 SF-shaped "arrow.triangle.2.circlepath" in `--label-secondary`, OR `checkmark.circle.fill` in `--success` when the user has just successfully completed a sync — the icon reflects last outcome).
  2. Heading: **"Last synced 4 minutes ago"** (relative time, `--text-subhead` 16px / 600, `--label-primary`). When never synced: **"You haven't synced yet."**
  3. Sub-line: **"0 new · 0 changed · 0 removed since last sync"** (`--text-body` 13px / 400, `--label-secondary`). When never synced, sub-line is omitted.
  4. `Button--primary` **"Run sync"** (D-03) — large primary, centred.
  5. **Collapsible disclosure** below the button: **"▶ Recent changes"** — on expand, lists the K most recent changes (up to 20) from the previous sync's apply set. Empty when no previous sync.
- **A11y:** wrapping `<section role="status" aria-label="Sync status">`. Heading is `<h1>` (rotor-discoverable).

#### In-progress state (D-04, D-07, D-17)

- **Layout:** `ContentPane` `split` variant. Middle column: `StageStepper` (with active cancel button at top) + `TriagePanel` (rendered only after Reconcile completes and `LockfileDiff` is non-empty; otherwise the stepper alone fills the middle column). Right column: `TriageDetail` for the currently-selected triage row, OR a placeholder when no row selected.
- **Navigation during run (D-04):** User can leave the Sync section (sidebar still works). The Sync `NavItem` shows the in-progress spinner variant. Returning to Sync re-renders the same `StageStepper` in whatever state it has progressed to.
- **A11y:** Middle column wrapped as `<section role="region" aria-label="Sync pipeline" aria-busy="true">`. The `aria-busy` flips to `false` when terminal state reached.

#### Terminal state (D-06, D-18)

- **Layout:** Middle column STAYS as `StageStepper` (now showing complete/failed/cancelled per row). Above the stepper, a summary heading and action buttons render:
  - **All-clear success path** (no failed rows, no partial failures, no cancellations): summary block fades out and view auto-returns to **idle state** with a transient toast **"Sync complete"** (CONTEXT §"Claude's Discretion": standard macOS toast position/duration).
  - **Cancelled path** (any stage = cancelled): summary block reads **"Sync cancelled"** heading (`--text-subhead` 16px / 600, amber `--badge-override-fg`), sub-line **"The library is in a consistent state. You can run sync again at any time."** Below: `Button--primary` **"Run sync"** (re-fire); `Button--secondary` **"Dismiss"**. Dismiss → auto-return to idle (no toast for cancellation).
  - **Failed path** (any stage = failed AND `retry_from` is `Some(stage)`): summary block reads **"Sync failed"** heading (`--text-subhead` 16px / 600, `--danger`), sub-line **"${ErrorCode}: ${message}"** (`--text-body` 13px / 400, `--label-secondary`). Below: `Button--primary` **"Retry from ${stageName}"** (D-19); `Button--secondary` **"Dismiss"**. Dismiss → auto-return to idle.
  - **Failed path** (failed AND `retry_from` is `None`): same as above but the primary button is omitted — only `Button--secondary` **"Dismiss"** is rendered (D-19 invariant: non-recoverable failures get no retry affordance).
  - **Partial-failure path** (all stages = complete AND at least one stage has `partialFailures.length > 0`): summary block reads **"Sync complete with ${K} issues"** (`--text-subhead` 16px / 600, `--label-primary`), sub-line **"Library and lockfile are saved. ${K} individual operations failed."** Below: `Button--primary` **"Retry failed items"** (D-20); `Button--secondary` **"Dismiss"**. The expanded `FindingRow` lists inside each affected `StageRow` stay visible so the user sees what failed without scrolling away.
- **Persistence (D-18):** terminal state persists until user clicks Dismiss or a retry action. No auto-dismiss timer.
- **A11y:** Summary heading is `<h1>` (rotor-discoverable). Live region announces the summary text on transition into terminal state.

---

## Per-view Design — Sync section

### Idle (default arrival state — D-03, D-06)

```
┌─Window────────────────────────────────────────────────────────────┐
│ Titlebar [● ● ●]              tome — Sync                         │
├─Sidebar──────┬─ContentPane (single-pane)─────────────────────────┤
│ LIBRARY      │                                                    │
│   Status     │                                                    │
│   Skills     │              ↺                                     │
│ ● Sync       │     Last synced 4 minutes ago                      │
│   Health (3) │     0 new · 0 changed · 0 removed since last sync  │
│              │                                                    │
│              │            [ Run sync ]                             │
│              │                                                    │
│              │     ▶ Recent changes                               │
│              │                                                    │
│              │                                                    │
│ tome · 2041  │                                                    │
└──────────────┴────────────────────────────────────────────────────┘
```

### In-progress (post-reconcile, triage pending — D-04, D-07, D-11)

```
┌─Window────────────────────────────────────────────────────────────┐
│ Titlebar                  tome — Sync                             │
├─Sidebar──────┬─Stepper + Triage (split)──────┬─TriageDetail──────┤
│   Status     │ [Cancel sync]                  │                    │
│   Skills     │ ✓ Reconcile          0.3s      │  axiom-swiftui [New]│
│ ● Sync ↺(12) │ ✓ Discover           1.1s      │  ──────────────────│
│   Health (3) │ ● Consolidate                  │  SOURCE  plugins   │
│              │   axiom-swiftui                │  HASH    sha256:…   │
│              │   ▓▓▓▓▓▓░░░░ 47/120  running…  │  SYNCED  —          │
│              │ ○ Distribute            —      │                    │
│              │ ○ Cleanup               —      │  Decision:          │
│              │ ○ Save                  —      │  (●) Keep this skill│
│              │                                │  (○) Disable here   │
│              │ ▼ NEW (8)        [Disable all] │  (○) View source    │
│              │   ▼ PLUGINS (5)                │                    │
│              │     ● axiom-swiftui            │                    │
│              │     ○ axiom-build              │                    │
│              │     …                          │                    │
│              │ ▶ CHANGED (3)                  │                    │
│              │ ▶ REMOVED (1)                  │                    │
│              │ ──────────────────────────     │                    │
│              │      [Apply 8 decisions]       │                    │
└──────────────┴────────────────────────────────┴────────────────────┘
```

### Terminal — partial-failure (D-20)

```
│ Sync complete with 2 issues                                       │
│ Library and lockfile are saved. 2 individual operations failed.   │
│                                                                   │
│ [ Retry failed items ]  [ Dismiss ]                               │
│                                                                   │
│ ✓ Reconcile          0.3s                                         │
│ ✓ Discover           1.1s                                         │
│ ✓ Consolidate        8.2s                                         │
│ ✓ Distribute         3.4s  [⚠ 2 issues]                           │
│   ⚠ [Permission] failed to remove ~/.codex/skills/foo  [▶ chain]  │
│   ⚠ [Io] broken symlink: ~/.codex/skills/bar           [▶ chain]  │
│ ✓ Cleanup            0.1s                                         │
│ ✓ Save               0.2s                                         │
```

### Terminal — failed with retry (D-18, D-19)

```
│ Sync failed                                                       │
│ [Permission]: failed to write manifest at ~/.tome/.tome-manifest.json │
│                                                                   │
│ [ Retry from Discover ]  [ Dismiss ]                              │
│                                                                   │
│ ✓ Reconcile          0.3s                                         │
│ ! Consolidate        4.7s                                         │
│   [Permission] failed to write manifest at …  [▶ Show error chain]│
│ ⊘ Distribute                                cancelled             │
│ ⊘ Cleanup                                   cancelled             │
│ ⊘ Save                                      cancelled             │
```

### Terminal — cancelled (D-17, D-18)

```
│ Sync cancelled                                                    │
│ The library is in a consistent state. You can run sync again      │
│ at any time.                                                      │
│                                                                   │
│ [ Run sync ]  [ Dismiss ]                                         │
│                                                                   │
│ ✓ Reconcile          0.3s                                         │
│ ⊘ Discover           cancelled                                    │
│ ⊘ Consolidate                                                     │
│ ⊘ Distribute                                                      │
│ ⊘ Cleanup                                                         │
│ ⊘ Save                                                            │
```

### Apply flow — `PreviewPopover` with `MachineTomlDiff` (D-15)

```
                                        ┌─PreviewPopover (480px)─────────────┐
                                        │ PREVIEW                            │
                                        │ 6 additions, 2 removals            │
                                        │ ─────────────────────────────────  │
                                        │  12   [machine_prefs]              │
                                        │  13 − enabled = ["axiom-build"]    │
                                        │  13 + enabled = [                  │
                                        │  14 +   "axiom-build",             │
                                        │  15 +   "axiom-concurrency",       │
                                        │  16 +   "axiom-swiftui",           │
                                        │  17 + ]                            │
                                        │  18                                │
                                        │  19   disabled = [                 │
                                        │  20 −   "old-helper",              │
                                        │  20 + ]                            │
                                        │ ─────────────────────────────────  │
                                        │ Applying writes                    │
                                        │ ~/.config/tome/machine.toml.       │
                                        │                                    │
                                        │            [Cancel] [Apply]        │
                                        └────────────────────────────────────┘
```

---

## Keyboard Map (NF-02)

Extends the Phase 26 map. **One Phase 26 binding is updated** (re-anchoring `⌘1..⌘4` to the new four-row sidebar order); the rest are net-new.

| Shortcut | Action | Scope | Mapped to |
|----------|--------|-------|-----------|
| `⌘1` | Jump to **Status** | Global | `Sidebar` `NavItem` selection (unchanged) |
| `⌘2` | Jump to **Skills** | Global | Same (unchanged) |
| `⌘3` | Jump to **Sync** | Global | **CHANGE** — Phase 26 mapped this to Health. Phase 27 re-anchors by position. Release note required. |
| `⌘4` | Jump to **Health** | Global | **NEW** — Phase 26 left `⌘4` unbound. |
| `⌘R` | **Run sync** (idle) / **Cancel sync** (in-progress) / **Retry failed items or Retry from stage** (terminal with retry available) | Global (Phase 27-locked) — bound on every view because Sync is a global action | Sync toolbar `[Run sync]` button (idle); cancel handler (in-progress); retry handler (terminal). When focus is inside a text input, the shortcut still fires (Sync is not an Edit action — no ambiguity). |
| `⌘.` (period) | **Cancel sync** when a run is active | Global | Same handler as `[Cancel sync]` button. Macros macOS HIG "cancel" convention (`⌘.`). When no run is active, the shortcut is a no-op (silently). |
| `⌘⇧A` | **Apply N decisions** when triage panel has non-default decisions | Sync section, when `Apply` button is enabled | Opens `PreviewPopover` (same as clicking the button). |
| `Enter` | In TriageRow: open the right-column detail (matches Skills view convention). In Stepper: no action. In Idle state Recent-changes disclosure: toggle. | Contextual | — |
| `Space` | TriageRow inline decision toggle (D-12: keep ⇄ disable) when focused | Triage panel | `onDecisionToggle` |
| `↑` / `↓` | Move selection in TriagePanel ListBox (jumps across SectionHeaders, skipping section labels themselves) | Sync section | React Aria `ListBox` |
| `Home` / `End` | First / last triage row | Sync section | React Aria `ListBox` |
| `Esc` | Close `PreviewPopover` (Apply); collapse expanded SectionHeader if focused on its trigger; **does NOT cancel an active sync** (Cancel sync requires explicit `⌘.` or button click — Esc is too easy to hit accidentally). | Contextual | React Aria primitives |

**Edit-menu predefined items (Phase 26 contract carry-forward).** `⌘C` / `⌘V` / `⌘X` / `⌘A` / `⌘Z` / `⌘⇧Z` remain owned by the macOS Edit menu via `PredefinedMenuItem`. None of Phase 27's custom shortcuts collide with these. The `⌘C` scoped-copy in the Skills view continues to gate on `activeElement`.

**Reserved (still do NOT bind):** `⌘N` (Add — Phase 28); `⌘O` (Open dialog — reserved for Phase 28+ if needed). `⌘D` remains free.

**Predefined menu changes — Library menu (D-02).** The NF-03 native macOS menu bar's `Library` menu gains a **`Sync`** item near the top, hot-keyed `⌘R`. Existing items below it (`Add Skill…`, `Show Library Folder`, etc. — Phase 28+ surface) are unaffected.

### VoiceOver labels — explicit contracts (Phase 27 additions)

| Element | `aria-label` template |
|---------|----------------------|
| Sidebar `Sync` NavItem (default) | `Sync, Sync section` |
| Sidebar `Sync` NavItem (in-progress) | `Sync, Sync section, sync in progress` |
| Sidebar `Sync` NavItem (pre-sync badge) | `Sync, Sync section, ${count} pending changes` |
| Sidebar `Sync` NavItem (post-sync failure badge) | `Sync, Sync section, ${count} unresolved sync failures` |
| `StageStepper` container | `Sync pipeline progress` |
| `StageRow` (pending) | `${stageName} stage, pending` |
| `StageRow` (active) | `${stageName} stage, running${currentItem ? `, ${currentItem}` : ''}${total > 0 ? `, ${current} of ${total}` : ''}` |
| `StageRow` (complete, no partial) | `${stageName} stage, complete in ${durationText}` |
| `StageRow` (complete, K issues) | `${stageName} stage, complete in ${durationText}, ${K} issues` |
| `StageRow` (failed) | `${stageName} stage, failed in ${durationText}, ${errorCode}, ${errorMessage}` |
| `StageRow` (cancelled) | `${stageName} stage, cancelled` |
| `TriagePanel` container | `Triage decisions` |
| Outer `SectionHeader` (NEW/CHANGED/REMOVED) | `<h2>${section}, ${count} ${section.toLowerCase()} skills</h2>` |
| Inner `SectionHeader` (source group) | `<h3>${sourceName}, ${count} skills</h3>` |
| `TriageRow` (default) | `${name}, ${changeKind} from source ${source}, decision: ${decision}` |
| `TriageRow` inline chip toggle button | `Toggle decision for ${name} between keep and disable on this machine` |
| Bulk-action button (section-level) | `Disable all new skills` |
| Bulk-action button (source-group) | `Disable all new skills from ${sourceName}` |
| `[Apply N decisions]` button | `Apply ${N} triage decisions, preview machine.toml diff` |
| `[Cancel sync]` button | `Cancel sync at next stage boundary` |
| `[Run sync]` button | `Run sync now` |
| `[Retry from ${stageName}]` button | `Retry sync from ${stageName} stage` |
| `[Retry failed items]` button | `Retry ${K} failed operations` |
| `[Dismiss]` button | `Dismiss sync result and return to idle` |
| `MachineTomlDiff` body | `machine.toml diff, ${addedCount} additions, ${removedCount} removals` |
| `MachineTomlDiff` removed line | (line-level role="row"; gutter aria-label) `removed line ${lineNumber}` |
| `MachineTomlDiff` added line | `added line ${lineNumber}` |
| `MachineTomlDiff` unchanged line | `unchanged line ${lineNumber}` (could be hidden from a11y tree to reduce noise — planner picks during 27-03) |
| Terminal-state summary heading | `Sync ${outcome}${outcome === 'complete with K issues' ? `, ${K} issues` : ''}` |
| Idle-state heading | `Last synced ${relativeTime}` OR `You haven't synced yet` |
| Toast `Sync complete` (transient) | `role="status"` `aria-live="polite"` reads "Sync complete" once on appearance |

---

## Copywriting Contract

All strings below are **verbatim**. Match Phase 26 tone — calm, descriptive, no marketing voice, no emoji. **Stage labels are plain English; the typed `SyncStage` variant name stays the internal identity** (CONTEXT.md "Claude's Discretion").

### Stage labels (D-07; tooltips match the typed variant name for engineering ergonomics)

| `SyncStage` variant | UI label | Tooltip |
|---------------------|----------|---------|
| `Reconcile` | **Reconcile** | Check for drift against the marketplace |
| `Discover` | **Discover** | Scan source directories for skills |
| `Consolidate` | **Consolidate** | Copy and hash skills into the library |
| `Distribute` | **Distribute** | Symlink the library into tool directories |
| `Cleanup` | **Cleanup** | Remove stale entries and broken symlinks |
| `Save` | **Save** | Write manifest, lockfile, and machine prefs |

### Primary actions

| Element | Copy |
|---------|------|
| **Primary CTA (idle state)** | **Run sync** |
| **Primary CTA (triage panel)** | **Apply ${N} decisions** (live count; N = decisions where `decision !== default`; disabled when N = 0) |
| **Primary CTA (terminal: failed with retry)** | **Retry from ${stageName}** |
| **Primary CTA (terminal: partial failure)** | **Retry failed items** |
| **Primary CTA (terminal: cancelled)** | **Run sync** (same as idle CTA — restart) |
| Secondary action (in-progress) | **Cancel sync** |
| Secondary action (terminal) | **Dismiss** |
| Secondary action (TriageDetail picker, git-sourced) | **View source** |
| Bulk action (NEW section) | **Disable all new** |
| Bulk action (NEW source-group) | **Disable all new from ${sourceName}** |
| Triage row inline chip — keep | **✓ keep** |
| Triage row inline chip — disable | **⊘ disabled here** |
| Triage row inline chip — removed (non-interactive) | **implicit remove** |
| TriageDetail radio — keep | **Keep this skill** |
| TriageDetail radio — disable | **Disable on this machine** |
| TriageDetail radio — view source (git-sourced only) | **View source (open in Finder)** |
| TriageDetail removed-skill helper | **This skill will be removed from the lockfile. No action required.** |
| TriageDetail empty selection | **Select a change to view details** |

### Idle state

| Element | Copy |
|---------|------|
| Heading (recent sync exists) | **Last synced ${relativeTime}** |
| Heading (never synced) | **You haven't synced yet.** |
| Sub-line (recent sync exists) | **${newCount} new · ${changedCount} changed · ${removedCount} removed since last sync** |
| Sub-line (never synced) | _(omitted — single-line idle state)_ |
| Recent-changes disclosure toggle | **Recent changes** |
| Recent-changes empty inside disclosure | **No changes recorded in the previous sync.** |

### In-progress state

| Element | Copy |
|---------|------|
| Stepper active stage trailing status | **running…** |
| Stepper future-stage placeholder | **—** |
| Stepper cancelled-stage trailing status | **cancelled** |
| Triage panel section headers | **NEW** · **CHANGED** · **REMOVED** (each with `(${count})` suffix; SectionHeader convention) |
| Triage panel source-group headers | **${sourceName.toUpperCase()}** (matches Phase 26 SectionHeader uppercase convention; e.g. **PLUGINS**, **MY-REPO**, **UNOWNED**) |

### Terminal state

| Element | Copy |
|---------|------|
| Heading — all-clear (transient toast only) | **Sync complete** |
| Heading — partial failure | **Sync complete with ${K} issues** |
| Sub-line — partial failure | **Library and lockfile are saved. ${K} individual operations failed.** |
| Heading — failed | **Sync failed** |
| Sub-line — failed | **[${errorCode}]: ${errorMessage}** |
| Heading — cancelled | **Sync cancelled** |
| Sub-line — cancelled | **The library is in a consistent state. You can run sync again at any time.** |

### Preview popover (`MachineTomlDiff` content)

| Element | Copy |
|---------|------|
| PREVIEW caption (inherited from Phase 26) | **PREVIEW** |
| Diff body header | **${addedCount} additions, ${removedCount} removals** |
| Helper text | **Applying writes `~/.config/tome/machine.toml`. The CLI sees this change immediately.** |
| Cancel button (inherited) | **Cancel** |
| Apply button (inherited) | **Apply** |

### Error and partial-failure copy (failure inline rendering — same shape as Phase 26 D-11)

`StageRow` failed inline: `[${errorCode}] ${errorMessage}` + `▶ Show error chain` disclosure → bullet list of `context: Vec<String>` entries. Code prefix `--text-small` 12px / 600 in `--danger`; message `--text-small` 12px / 400 in `--label-primary`. Same pattern Phase 26's `FindingRow` uses.

`MachineTomlDiff` apply-failed banner (inside the still-open popover): same shape — `[${errorCode}] ${errorMessage}` + Show context disclosure, with a `--danger`-bordered banner above the button row.

### Destructive operations (NF-04 audit for Phase 27)

Three Phase 27 actions touch on-disk state. Per NF-04, "destructive" means modifying state the user could regret. Classification:

- **Apply triage decisions** (writes `~/.config/tome/machine.toml`) — **classified destructive in spirit** (changes which skills the CLI sees as enabled). NF-04 satisfied by `PreviewPopover` + `MachineTomlDiff` — preview-then-confirm flow is mandatory. **No additional confirmation dialog** (Phase 26 D-09 / NF-04 ergonomics — the popover IS the confirmation).
- **Run sync** — writes library, manifest, lockfile, distribution symlinks. **NOT classified destructive** in the NF-04 sense — sync's job IS to mutate, and the user's whole reason for clicking the button is to apply the diff they've already seen. **No confirmation dialog.** The button label "Run sync" + the visible triage panel (when one exists) ARE the user's informed-consent surface. Sync is also **always recoverable** (subsequent syncs and `tome backup` for distribution).
- **Cancel sync** — domain bails at the next stage boundary; **always recoverable** per SC#4. **No confirmation dialog** (D-17) — adding one would create friction with zero protective value because the safety guarantee already exists.

All three honor NF-04's spirit ("destructive operations surface their plan and require explicit confirmation") — the triage panel + `PreviewPopover` + visible stepper are the plans, the buttons are the confirmations.

---

## Registry Safety

| Registry | Blocks Used | Safety Gate |
|----------|-------------|-------------|
| shadcn official | none | not applicable — shadcn not initialised (Phase 26 D-14/D-15 stack incompatible) |
| Third-party registries | none declared | not applicable |

Component sources are direct npm packages governed by the project's existing dependency-audit policy. **Phase 27 introduces zero new npm dependencies** — every component is built from the Phase 26 stack (`react`, `react-aria`, `react-stately`, existing CSS Modules). The `MachineTomlDiff` is a hand-rolled line-diff renderer reading a structured `MachineTomlPreview` produced by the Rust side (`preview_machine_toml` command); no diff library is brought in (D-15 "Claude's Discretion" — line-diff input is structured).

Domain-side `bindings.ts` regeneration is enforced by the existing Phase 25 CI freshness gate (`cargo run -p tome-desktop --bin gen-bindings && git diff --exit-code -- crates/tome-desktop/ui/src/bindings.ts`). Phase 27 adds the following to the shared registry (per CONTEXT §"Integration Points"):

- **5 new Tauri commands:** `start_sync`, `cancel_sync`, `get_lockfile_diff`, `preview_machine_toml`, `apply_machine_toml`.
- **1 existing event variant extended:** `SyncStageProgress` gains `item: Option<String>` (D-08).
- **0–2 new events:** optionally `SyncStarted` / `SyncFinished` for toast / auto-return wiring (planner picks; either is fine).
- **1 new domain field on a discovery type:** `DiscoveredSkill::synced_at: Option<DateTime>` (D-16, closes Phase 26 carryover #2). Threads through `ListReport` and `SkillSummary`-shaped boundary types; `bindings.ts` regenerates.

No third-party shadcn registries. The npm-package vetting gate from `<design_contract_questions>` is **not applicable**.

---

## Open Items (carry to the planner — NOT silently resolved)

1. **`SyncOutcome` IPC type shape** (CONTEXT.md "Claude's Discretion" tail). Two viable encodings: (a) wrap the command return in `SyncOutcome { result: Result<(), TomeError>, retry_from: Option<SyncStage>, partial_failures: Vec<PartialFailure> }`, OR (b) extend `TomeError` with an optional `retry_from` field. The contract here renders against shape (a) (the per-view diagrams and the `[Retry from ${stageName}]` copy assume a wrapping struct). Planner picks during 27-04 / 27-05 plan generation; if (b) is selected, the popover and stepper renderers receive `error.retry_from` rather than a sibling field — no UI contract change.
   - **Owner:** planner. **Trigger:** before 27-04.

2. **Dark-mode dark tokens** (Phase 26 Open Item 1) remain provisional for Phase 27 inheritance. Phase 27 introduces no new colour tokens, so this is a pure carry-forward of the Phase 26 open item — same owner, same trigger (before alpha visual sign-off). Phase 27 explicitly does NOT relitigate.

3. **Sidebar `⌘1..⌘4` re-anchoring release note** (D-01 + keyboard map). The Phase 26 `⌘3 → Health` binding is now `⌘3 → Sync` / `⌘4 → Health` because Sync inserts between Skills and Health. The planner writes a single-sentence release note in 27-01 ("Sync section added; Health is now `⌘4`") and the menu Help → Keyboard Shortcuts cheat-sheet (a Phase 26 26-07 artifact) updates accordingly. Non-blocking for implementation.

4. **`Sync` `NavItem` icon — exact SF-symbol shape.** D-01 calls for an `arrow.triangle.2.circlepath`-equivalent SF-shaped glyph. The icon library Phase 26 picked (`lucide-react` filtered to SF-shapes, or hand-curated SVGs — Phase 26 26-07 result) may or may not have a 1:1 match. Planner picks a substitute that reads as "sync / refresh" if no exact match exists.
   - **Owner:** planner during 27-01.

5. **`MachineTomlPreview` line-diff algorithm** (server-side). The Rust side produces the structured `lines: Vec<DiffLine>` consumed by `MachineTomlDiff`. Algorithm choice (Myers vs. naïve LCS vs. character-aware) is a domain-side decision in plan 27-03; UI contract is agnostic. Planner picks during 27-03.

---

## Checker Sign-Off

- [ ] Dimension 1 Copywriting: PASS
- [ ] Dimension 2 Visuals: PASS
- [ ] Dimension 3 Color: PASS
- [ ] Dimension 4 Typography: PASS
- [ ] Dimension 5 Spacing: PASS
- [ ] Dimension 6 Registry Safety: PASS

**Approval:** pending
