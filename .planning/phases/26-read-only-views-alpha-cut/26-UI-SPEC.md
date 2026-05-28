---
phase: 26
slug: read-only-views-alpha-cut
status: draft
shadcn_initialized: false
preset: none
created: 2026-05-28
---

# Phase 26 — UI Design Contract

> Visual and interaction contract for the **read-only alpha cut** of the tome Desktop GUI
> (Tauri 2 + React 19, macOS only). Pre-populated from `26-CONTEXT.md` (D-01..D-16),
> `26-DESIGN-EXTRACT.md` (tokens + component inventory pulled from the chosen Figma Make mockup,
> file `xl7bEUqwDz1fO6Ar83ENZI`, light Skills window `1:1602`), `REQUIREMENTS.md` (VIEW-01..06,
> NF-01..05, D-GUI-01..09), and ROADMAP Phase 26 §SC#1..7.
>
> **What this locks:** tokens, component contracts, copywriting, keyboard map, and the alpha
> shell. **What this does NOT lock:** pixel-perfect layouts (the Figma Make output is values
> reference only — fractionally-sized, absolute-positioned, Inter-not-SF-Pro). The planner builds
> real fl/grid layouts on top of the contracts below.

---

## Design System

| Property | Value | Source |
|----------|-------|--------|
| Framework | React 19 | D-GUI-04 (locked Phase 25) |
| Component primitives | **React Aria (Adobe headless)** | D-14 |
| Virtualisation | **TanStack Virtual** | D-14 (drives NF-01: 2000 skills @ 60fps) |
| Styling | **CSS Modules (`*.module.css`) + CSS custom-property tokens** | D-15 (zero-runtime, Vite-native) |
| Theme mode | **System-driven via `prefers-color-scheme`** — no in-app switcher | D-16, NF-03 |
| Markdown renderer | **`react-markdown` + `remark-gfm`** (SC#4 subset) | D-08, VIEW-04 |
| Icon set | **SF Symbols equivalents** rendered as inline SVG (e.g. `lucide-react` filtered to SF-shaped icons, OR hand-curated SVGs). Single library, single weight. | D-13 (HIG-aligned); planner picks library in 26-07 a11y plan |
| Font | macOS system stack — `-apple-system, "SF Pro Text", "SF Pro Display", system-ui, sans-serif`. **No web font.** | D-13; extract §Typography |
| Window chrome | Tauri 2 unified native titlebar + traffic lights + vibrancy sidebar + solid fallback on `prefers-reduced-transparency` | D-16 |
| Tool | **none** (no shadcn — incompatible with D-14/D-15 stack) | shadcn-gate result |
| Preset | not applicable | — |

**Why no shadcn:** D-14 mandates React Aria headless primitives + custom macOS styling, and D-15 mandates CSS Modules + CSS custom-property tokens. shadcn ships Tailwind + Radix-based opinionated components; adopting it would conflict with both decisions. The shadcn-initialisation gate from the researcher workflow is **non-applicable** here. Registry safety: not applicable.

---

## Spacing Scale

4px base grid (macOS HIG aligned). All values multiples of 4.

| Token | Value | Usage |
|-------|-------|-------|
| `--space-1` | 4px | Icon ↔ label gaps, badge inner padding, hairline offsets |
| `--space-2` | 8px | Compact rows (sidebar nav-item vertical), button inner gap |
| `--space-3` | 12px | List-row vertical padding, card body padding-y |
| `--space-4` | 16px | Default content padding, section gap, metadata grid gap |
| `--space-5` | 20px | Detail-pane header bottom margin, popover padding |
| `--space-6` | 24px | View padding-top, major section gap |
| `--space-8` | 32px | Page-level gap between large groups |
| `--space-12` | 48px | Empty-state centring offset |

**Exceptions:**

- Sidebar fixed width: **210px** (anchor from extract; not a spacing-scale token).
- Titlebar fixed height: **44px** (Tauri unified-titlebar default; anchor from extract).
- Sidebar nav-item height: **26px** (compact rhythm — Mail/Notes density, matches extract).
- List-row height: **52px** (two-line: primary 13px / secondary 12px @ 8px gap + 12px padding-y).
- Popover (preview-then-confirm) max-width: **320px**.

Confirm during 26-07 (HIG audit plan).

---

## Typography

System font stack only (D-13). No web font load.

| Role | Token | Size | Weight | Line Height | Usage |
|------|-------|------|--------|-------------|-------|
| Caption (uppercase labels) | `--text-caption` | 11px | 500 | 1.2 | Section headers ("LIBRARY", "AUTO-FIXABLE"), badge labels, metadata grid labels |
| Footnote | `--text-footnote` | 12px | 400 | 1.4 | List-row secondary line, table cell values, finding descriptions |
| Body | `--text-body` | 13px | 400 | 1.5 | Default body, list-row primary, markdown paragraph, button labels |
| Body-medium | `--text-body` | 13px | 500 | 1.5 | Emphasised inline text, selected nav-item label |
| Subhead | `--text-subhead` | 16px | 600 | 1.3 | Markdown H2/H3, "Directories" section header, popover header |
| Title | `--text-title` | 22px | 600 | 1.2 | View title in content-pane header ("Status" / "Skills" / "Health"), skill name in detail header |

**Total declared sizes: 4** (11 / 12 / 13 / 16 / 22 px — 13px counts once across two weights). **Weights: 2 effective** (400 regular + 600 semibold), with 500 used as a single mid-weight for caption/emphasis only.

Monospace family (code blocks, content hashes, paths): `ui-monospace, "SF Mono", Menlo, monospace` at `--text-footnote` (12px, weight 400).

---

## Color

CSS custom properties driven by `prefers-color-scheme`. All values declared per D-15 (zero-runtime).

### 60 / 30 / 10 split

- **60% dominant surface:** `--bg-window` (white in light, near-black in dark) — fills the content pane (right of sidebar), markdown body background, popover background.
- **30% secondary surface:** `--sidebar-material` + `--bg-subtle` (vibrancy sidebar + subtle fills on table-header zebra and search-field background).
- **10% accent:** `--accent` (system blue) — **reserved for** the five elements listed below. Nothing else.

**Accent reserved-for list (the entire 10% inventory):**

1. Selected list row fill (Skills list, Health finding rows when keyboard-focused).
2. Selected sidebar nav-item capsule fill.
3. Primary-button background ("Disable on this machine", "Apply" in preview popover).
4. Inline links inside the markdown body.
5. Focus ring on every interactive element (React Aria default focus ring, recoloured to `--accent`).

Never used for: section dividers, body text, badge backgrounds (other than the dedicated `--badge-managed` blue tint), icon fills outside the above.

### Light tokens (extracted from Figma Make file, light Skills window `1:1602`)

| Token | Value | Usage |
|-------|-------|-------|
| `--label-primary` | `#1d1d1f` | Primary text, headings |
| `--label-secondary` | `#6e6e73` | Secondary/muted text, section captions, metadata labels |
| `--bg-window` | `#ffffff` | Content-pane background, markdown body, popover, list column |
| `--bg-app` | `#f5f5f7` | App canvas (behind window when translucent), titlebar fill |
| `--bg-subtle` | `#f2f2f4` | Search field, table-header zebra, button-secondary hover |
| `--sidebar-material` | `rgba(246,246,248,0.72)` (translucent) / `#ececef` solid fallback under `prefers-reduced-transparency` | Vibrancy sidebar |
| `--separator` | `rgba(0,0,0,0.08)` | Hairlines between rows, sections; stronger variants `0.12` / `0.15` for headers |
| `--accent` | `#007aff` | See reserved-for list above |
| `--accent-pressed` | `#0a6cdc` | Pressed/hover accent state |
| `--accent-on` | `#ffffff` | Foreground when sitting on `--accent` (selected row text, primary-button label) |
| `--danger` | `#ff3b30` | Health count badge fill, error state foreground, `[Permission]` and other `TomeError` codes |
| `--success` | `#28c840` | All-clear checkmark, StatusDot (in-sync) |

### Dark tokens — **PROVISIONAL** (seeded with Apple system dark pairs; not pulled from the mockup)

> ⚠ **Open item 1** — Dark window exists in the mockup but the exact hexes were not pulled
> (researcher only extracted from the light window). These values are Apple system pairs.
> The planner OR a follow-up extract (re-run Figma MCP against the dark Skills node) must
> confirm before beta cut. Treat as binding for alpha implementation; flag for re-extraction.

| Token | Provisional value | Usage |
|-------|-------------------|-------|
| `--label-primary` | `#f5f5f7` | Primary text |
| `--label-secondary` | `#98989d` | Secondary text |
| `--bg-window` | `#1e1e1e` | Content pane, markdown body, popover |
| `--bg-app` | `#000000` | App canvas |
| `--bg-subtle` | `#2c2c2e` | Search field, table-header zebra |
| `--sidebar-material` | `rgba(40,40,42,0.72)` (translucent) / `#28282a` solid fallback | Vibrancy sidebar |
| `--separator` | `rgba(255,255,255,0.10)` | Hairlines |
| `--accent` | `#0a84ff` | Same reserved-for list |
| `--accent-pressed` | `#0070dc` | Pressed/hover accent |
| `--accent-on` | `#ffffff` | Foreground on accent |
| `--danger` | `#ff453a` | Errors, Health badge |
| `--success` | `#30d158` | All-clear, in-sync dot |

### Traffic-light controls (fixed values across modes, owned by Tauri/system)

`#ff5f57` (close) / `#febc2e` (min) / `#28c840` (max). **Do not encode** — rendered by the OS via Tauri unified titlebar.

---

## Component Contract

For every component below: purpose, props/variants, React Aria primitive (D-14 a11y contract), VoiceOver label expectation, and dark/light token bindings. Built as `*.module.css` per D-15.

### Shell (built once per D-13 — inherited by Phases 27–31)

#### `Window`
- **Purpose:** Top-level container. Provides Tauri unified titlebar + traffic lights + 3-column NavigationSplitView body (sidebar / list / detail).
- **Variants:** none.
- **Layout intent:** CSS Grid `grid-template-columns: 210px minmax(280px, 380px) 1fr`. Title centred in titlebar via Tauri config.
- **A11y:** native `<main>` landmark wraps the split.
- **Tokens:** background `--bg-app`; titlebar background `--bg-app` light / `--bg-window` dark.

#### `Titlebar`
- **Purpose:** Unified macOS titlebar; renders traffic lights (OS-owned, left) + centred title `tome — {section}` (`--text-body` weight 500).
- **Props:** `section: 'Status' | 'Skills' | 'Health'`.
- **Height:** 44px (anchor from extract).
- **A11y:** `role="banner"`, `aria-label="tome ${section}"`.

#### `Sidebar`
- **Purpose:** Vibrancy/translucent left rail. Sections: `LIBRARY` caption header → three `NavItem`s (Status / Skills / Health) → spacer → footer `tome · {N} skills`.
- **Material:** `--sidebar-material` token. **Falls back to solid** when `@media (prefers-reduced-transparency: reduce)` (D-16, NF-03).
- **Width:** 210px (anchor).
- **A11y:** `<nav aria-label="Sections">` + React Aria `ListBox` for keyboard nav with arrow-key traversal.

#### `NavItem`
- **Purpose:** Sidebar row. Icon (SF-shaped) + label.
- **Variants:** `default | hover | selected | badge`.
- **Selected:** accent-blue capsule fill (`--accent` background, `--accent-on` foreground, `--radius-md` 6px); covers full row width minus 8px gutters.
- **Badge variant:** trailing red circle (`--danger` fill, `--accent-on` text, `--text-caption`), shown when `Health` has findings (D-02). Hidden at zero findings.
- **A11y:** React Aria `ListBoxItem` with `aria-label="${label}, ${section} section${badge ? `, ${count} health issues` : ''}"`. Selected announces "selected".
- **Keyboard:** ↑/↓ to move; Enter / Space to select; ⌘1 / ⌘2 / ⌘3 jump to Status / Skills / Health (added to keyboard map below).

#### `ContentPane`
- **Purpose:** Right region (list + detail OR full-width Status/Health). Header (view title 22px + optional trailing meta like the "Updated" pill) + scrolling body.
- **Variants:** `single-pane` (Status, Health) | `split` (Skills: middle list column + right detail column).
- **A11y:** `<main aria-label="${viewTitle}">`.

### Atoms

#### `Badge`
Three subtypes — distinct token bindings to avoid 10% accent rule violation:

| Subtype | Light fill | Light fg | Dark fill | Dark fg |
|---------|-----------|----------|-----------|---------|
| `Badge--role-discovery` | `#e3f3e8` | `#1f8f4e` | `#1c3a26` | `#62d391` |
| `Badge--role-distribution` | `#e4eefb` | `#1f5fb8` | `#1a2e4d` | `#6aa7f0` |
| `Badge--type-claude-plugins` | `#efeaff` | `#6336c9` | `#2e2350` | `#b29bff` |
| `Badge--type-git` | `#fff1da` | `#a35d00` | `#3d2a10` | `#e0a060` |
| `Badge--type-directory` | `#f2f2f4` | `#6e6e73` | `#2c2c2e` | `#98989d` |
| `Badge--managed` | `#e4eefb` | `#1f5fb8` | `#1a2e4d` | `#6aa7f0` |
| `Badge--disabled` | `#f2f2f4` | `#6e6e73` | `#2c2c2e` | `#98989d` |
| `Badge--override` | `#fff1da` | `#a35d00` | `#3d2a10` | `#e0a060` |

- **Shape:** pill (`--radius-pill: 9999px`), padding `2px 8px`, `--text-caption` (11px / 500).
- **Disabled badge label:** **"Disabled"** (verbatim — D-06; the mockup's `OFF` is **superseded** — see open item 3).
- **A11y:** rendered as plain `<span>`. Badges accompanying skill name are read via the parent row's `aria-label` (see `SkillListRow`).

#### `Pill — Updated`
- **Purpose:** Transient acknowledgement of a watcher-driven refresh (D-03). Fades over ~2s (CSS transition opacity 1 → 0 between 1500ms and 2000ms after mount).
- **Position:** Inline next to the **"Last sync"** field in the Status view ContentPane header — NOT in the sidebar or titlebar.
- **Style:** pill, fill `--success` at 18% alpha (light) / 24% alpha (dark), text `--success`, label **"Updated"** (verbatim, 11px / 500). `prefers-reduced-motion: reduce` → no fade, just appears 2s then snaps to `display: none`.
- **A11y:** `role="status"` with `aria-live="polite"`, `aria-atomic="true"`. Screen reader announces "Updated" once.

#### `StatusDot`
- 8px circle, `--success` in-sync, `--danger` out-of-sync (Lockfile state in VIEW-01).
- A11y: decorative; the parent row carries the textual status ("In sync"). `aria-hidden="true"`.

#### `SeverityIcon`
- ⚠ warning (`--danger`, fixable) | ⛔ blocked (`--label-secondary`, manual). SVG, 16×16.
- A11y: `aria-hidden="true"`; severity is read via the `FindingRow` label.

#### `Button`
- **Variants:** `primary | secondary | small-fix`.
- **Primary:** `--accent` fill, `--accent-on` text, 13px / 500, padding `6px 12px`, `--radius-md` (6px). Used for: "Disable on this machine" (detail header), "Apply" (popover). Hover/pressed → `--accent-pressed`.
- **Secondary:** `--bg-window` fill, `--label-primary` text, 1px `--separator` border, same metrics. Used for: "Open source folder", "Copy path", "Cancel".
- **Small-fix:** secondary metrics with smaller padding `4px 10px`, `--text-footnote` (12px), only used inside `FindingRow`. Label **"Fix"**.
- **A11y:** React Aria `Button`. Disabled state via `aria-disabled="true"` and 50% opacity — not rendered when the action does not apply (D-12 — never a dead `Fix` button on non-fixable findings).

#### `SearchField`
- React Aria `SearchField`. Magnifier glyph (SF-shaped) left, X-clear when non-empty right.
- Background `--bg-subtle`, `--radius-md`, padding `6px 10px`, `--text-body`.
- Placeholder: **"Search skills"** (verbatim).
- **Pinned at the top of the Skills middle column** (D-04); ⌘F focuses it (see keyboard map).
- A11y: implicit `role="searchbox"` + `aria-label="Search skills"`. Each keystroke filters the list at the React-Aria-bound `Virtualizer`; matches show inline highlight via background `rgba(0,122,255,0.15)`.

#### `PopupMenu`
- React Aria `MenuTrigger` + `Menu`. Closed state: button with current value + chevron-down.
- Two instances in the Skills list-column toolbar:
  - **Sort** — items `Name` (default) / `Source` / `Recent`.
  - **Group** — items `None` (default) / `Source` / `Role`.
- A11y: native React Aria menu semantics. `aria-label="Sort skills"` / `aria-label="Group skills"`.

### Molecules — view-specific

#### `KeyValueRow` (Status dashboard)
- **Purpose:** Label + value horizontal row, optional trailing badge/dot.
- **Layout:** `display: grid; grid-template-columns: 160px 1fr auto; gap: var(--space-3);`.
- **Label:** `--text-caption` (uppercase, 11px / 500, `--label-secondary`).
- **Value:** `--text-body` (13px / 400, `--label-primary`); monospace if path/hash.
- **Used by:** every Status field — "tome home" / "Library" / "Last sync" / "Lockfile" / "Machine prefs". Trailing slot hosts `StatusDot`, badges, or the transient `Updated` pill (Last sync row only).

#### `DirectoryTable` (Status, "Directories" section)
- **Columns:** NAME (with secondary path line) / ROLE (`Badge--role-*`) / TYPE (`Badge--type-*`).
- **Header:** `--text-caption` uppercase, `--bg-subtle` zebra.
- **Row:** `--text-body` primary + `--text-footnote` `--label-secondary` secondary, 1px `--separator` bottom.
- **A11y:** native `<table>` with `<th scope="col">`. No row interaction in Phase 26.

#### `SkillListRow`
- **Purpose:** Middle-column row in Skills view. Two-line: primary skill name (13px / 500), secondary `source · managed|local` (12px / 400, `--label-secondary`). Trailing `Badge--disabled` when disabled-on-this-machine.
- **Variants:** `default | hover | selected | disabled-on-this-machine`.
- **Selected:** `--accent` background, `--accent-on` text (skill name only; secondary line `rgba(255,255,255,0.78)`), `--radius-md` capsule, 8px horizontal inset.
- **Height:** 52px (anchor).
- **Virtualisation:** rendered inside TanStack Virtual `useVirtualizer` (NF-01).
- **A11y:** React Aria `ListBoxItem`. `aria-label="${name}, source ${sourceName}, ${managed ? 'managed' : 'local'}${disabled ? ', disabled on this machine' : ''}"`. Up/Down/Home/End/PgUp/PgDn nav. Enter opens detail; right-click → `ContextMenu` (open source / copy path / disable).

#### `DetailHeader` (Skills right column, top section)
- **Purpose:** Compact metadata header above the scrolling markdown body (D-05).
- **Layout (top-to-bottom):**
  1. Row 1: skill name (`--text-title` 22px / 600) + trailing badges (`Badge--managed`, `Badge--disabled` if applicable).
  2. Row 2: metadata grid — three labelled cells: **Source path** (mono, ellipsised middle), **Content hash** (mono, truncated `sha256:abc123…`), **Last sync** (relative time, e.g. "2 minutes ago"). Each cell label uses `--text-caption`; value `--text-footnote`. 16px column gap.
  3. Row 3: three action buttons, left-aligned, 8px gap — order: `[Open source folder]` `[Copy path]` `[Disable on this machine]`. **"Disable on this machine" is the only primary** (D-06); the other two are secondary (D-07).
- **Bottom border:** 1px `--separator`. Section padding 20px / 24px.
- **A11y:** `aria-label="${skill name} details"`. Action buttons each have explicit `aria-label`:
  - Open source: `aria-label="Open source folder for ${skillName} in Finder"`.
  - Copy path: `aria-label="Copy source path for ${skillName} to clipboard"`.
  - Disable: `aria-label="Disable ${skillName} on this machine"` — and **on success**, dispatches a `role="status"` announcement: "Disabled ${skillName} on this machine." (D-06 exercises the silent-refresh loop; the file watcher refresh redraws the badge.)

#### `MarkdownBody`
- **Purpose:** Renders SKILL.md body (post-frontmatter) below the `DetailHeader` (VIEW-04, D-08).
- **Library:** `react-markdown` + `remark-gfm`.
- **Subset enforced via `allowedElements`:**
  - Headings: `h1`, `h2`, `h3` (`--text-title` 22 / `--text-subhead` 16 / 14px 600 respectively; 1.3 line-height).
  - Lists: `ul`, `ol`, `li` (16px left padding, default disc/decimal markers).
  - Links: `a` (rendered as `<a>` with `target="_blank"` + Tauri opener invocation — opens in system browser per CONTEXT §"Claude's Discretion"). Colour `--accent`, underline on hover.
  - Code: inline `code` (mono 12px, background `--bg-subtle`, `--radius-xs` 3px, padding `1px 4px`) and fenced `pre > code` (mono 12px, background `--bg-subtle`, `--radius-md` 6px, padding 12px, overflow `auto`). **Plain rendering** — no syntax highlighting in alpha (CONTEXT §"Deferred Ideas").
  - Inline emphasis: `strong` (600), `em` (italic).
- **NOT rendered (stripped by `react-markdown` allow-list):** tables, images, blockquotes, task lists, HTML passthrough, footnotes — out of SC#4 subset.
- **Scrolling:** the markdown body is the scrollable region; the `DetailHeader` stays fixed at top.
- **A11y:** `<article aria-label="${skillName} documentation">`. Heading hierarchy preserved for VoiceOver rotor navigation. Links carry `aria-label="${linkText}, opens in browser"` when the visible text is non-descriptive (URL only).
- **⚠ Open item 4:** The wording in `REQUIREMENTS.md` VIEW-04 ("same Markdown subset as `browse/markdown.rs`") is **superseded by ROADMAP SC#4** (this richer subset). D-08 records the reconciliation; the planner is expected to update VIEW-04's literal text in a follow-up requirements-doc cleanup (non-blocking for alpha implementation).

#### `SectionHeader` (Health)
- **Purpose:** Group findings into `AUTO-FIXABLE` / `NEEDS ATTENTION` sections (D-12 layout from "Claude's Discretion" — Claude chose flat-grouped over fully-flat).
- **Layout:** `--text-caption` uppercase (`--label-secondary`) on the left, count chip on the right (`(N)` in `--label-secondary`).
- 24px top margin between sections; 8px below the header before the first row.
- A11y: rendered as `<h2>` so it appears in VoiceOver's headings rotor; count is part of the heading text.

#### `FindingRow`
- **Purpose:** A single doctor finding (auto-fixable or manual). Single-row default; expands to disclose `TomeError` chain when a fix has just failed (D-11).
- **Layout (default):** `[SeverityIcon] [title — primary] [description — secondary]            [Fix button | manual hint]`
- **Title:** `--text-body` 500, `--label-primary`.
- **Description:** `--text-footnote`, `--label-secondary`. Truncated with ellipsis if needed; full text visible in VoiceOver via the row's `aria-label`.
- **Trailing slot:**
  - **Auto-fixable (RepairKind::* with a Rust handler):** `Button--small-fix` labelled **"Fix"**. Opens `PreviewPopover` (D-09).
  - **Non-fixable (`unparsable-frontmatter`, `diverging-target`):** Inline text in `--text-footnote` `--label-secondary`, **NO button** (D-12). See Copywriting §Manual remediation hints for the exact strings.
- **Failed-fix state (after Apply errored, D-11):** Row stays visible; below the primary line a disclosure shows the `TomeError` — formatted as `[Code] {message}` in `--danger` (`--text-footnote`) with a collapsible "Show context" disclosure listing the `context: Vec<String>` chain. The Fix button stays available for retry.
- **Successful-fix state:** Row removes itself on next file-watcher refresh (D-11; D-03's silent re-render reconciles). No optimistic animation in alpha — the disk truth is the source.
- **A11y:** `role="group"` with `aria-label="${severity} finding: ${title}. ${description}. ${fixable ? 'Fix available' : 'Manual remediation required'}"`. React Aria `Button` for the Fix action.

#### `PreviewPopover` (Doctor — D-09)
- **Purpose:** Preview-then-confirm sheet for each repair, satisfying NF-04. Anchored to the Fix button via React Aria `Popover` (no modal overlay — non-blocking; clicking outside cancels).
- **Width:** 320px max.
- **Layout (top-to-bottom):**
  1. Caption header **"PREVIEW"** (`--text-caption` uppercase, `--label-secondary`).
  2. Change line — one sentence describing the dry-run effect, sourced from the `RepairKind`'s human description (already lives in `doctor.rs`). `--text-body`, `--label-primary`. Path fragments rendered monospace.
  3. Helper text (`--text-footnote`, `--label-secondary`) — e.g. **"This change is reversible by running tome sync."** Optional per repair kind.
  4. Button row, right-aligned, 8px gap: `[Cancel]` (secondary) `[Apply]` (primary).
- A11y: `role="dialog"` with `aria-modal="true"` (focus trap), `aria-labelledby` → the PREVIEW header. Escape dismisses; Cancel returns focus to the Fix button. After Apply: focus moves to the (now-likely-removed) row's parent section header so VoiceOver picks up the change.

### States

#### Empty selection (Skills view, no row selected)
- **Layout:** Detail column shows neutral centred placeholder. `--text-body` (`--label-secondary`).
- **Copy:** **"Select a skill to view details"** (verbatim, see Copywriting).
- A11y: `role="status"`, `aria-live="polite"` so VoiceOver announces on the first render only.

#### All-clear health (D-12)
- **Layout:** Health ContentPane body shows centred SF-shaped checkmark glyph (32×32, `--success`) above the heading "Everything looks healthy" (`--text-subhead` 16px / 600, `--label-primary`) and a sub-line (`--text-body` `--label-secondary`).
- **Copy:** see Copywriting (heading + sub-line).
- **Sidebar:** Health `NavItem` badge variant **disappears** (D-12; cleared at zero findings).
- A11y: `<section role="status" aria-label="Library health">`. The state heading is an `<h2>` so the headings rotor lists it.

#### Transient "Updated"
- See atom `Pill — Updated` above. State is owned by Status view's KeyValueRow for "Last sync".
- **Trigger:** Any `manifest-changed` / `lockfile-changed` / `library-changed` event from the Rust file watcher (VIEW-06).
- **Selection preservation across refresh (D-03):** When Skills view receives a refresh event, the currently-open skill stays selected; if the underlying skill was removed by the external change, the detail column reverts to the **Empty selection** state and an additional one-time aria-live announcement reads "Selected skill was removed."

---

## Per-view Design

Layout intent (token-and-component composition); not pixel-perfect coordinates. The planner is free to evolve layouts per user feedback; tokens and component contracts above are the foundational lock.

### Status (default landing — D-02)

```
┌─Window──────────────────────────────────────────────────────────┐
│ Titlebar [● ● ●]              tome — Status                     │
├─Sidebar────────┬─ContentPane (single-pane)─────────────────────┤
│ LIBRARY        │ Status              (transient: Updated)       │
│ ● Status       │                                                │
│   Skills       │ ┌─KeyValueRow ────────────────────────┐         │
│   Health (3)   │ │ TOME HOME   ~/.tome                 │         │
│                │ │ LIBRARY     ~/.tome/library         │         │
│                │ │            (2,041 skills)           │         │
│                │ │ LAST SYNC   Today at 9:14 AM [Updated]│       │
│                │ │ LOCKFILE    In sync • ●green        │         │
│                │ │ MACHINE     3 skills disabled       │         │
│                │ └─────────────────────────────────────┘         │
│                │                                                │
│                │ Directories (5)                                │
│                │ ┌─DirectoryTable──────────────────────┐         │
│                │ │ NAME             ROLE   TYPE        │         │
│                │ │ claude-plugins   [Disc][CP]         │         │
│                │ │ dotfiles-skills  [Disc][Git]        │         │
│                │ │ ~/.claude/skills [Dist][Dir]        │         │
│                │ │ codex            [Dist][Dir]        │         │
│                │ │ antigravity      [Dist][Dir]        │         │
│                │ └─────────────────────────────────────┘         │
│ tome · 2041    │                                                │
└────────────────┴────────────────────────────────────────────────┘
```

- Renders every field returned by `commands.getStatus()` (the existing Phase 25 surface plus `last_sync`).
- "Last sync" field is the only host for the `Updated` pill (D-03).
- D-GUI-08 / "no JS-side business logic": React calls `commands.getStatus()`, narrows the `Result<StatusReport, TomeError>` union (App.tsx pattern from Phase 25), and renders KV rows. No client-side computation beyond the relative-time formatter on `last_sync`.

### Skills (list + detail — D-01, D-04, D-05)

```
┌─Window────────────────────────────────────────────────────────────┐
│ Titlebar                  tome — Skills                           │
├─Sidebar──────┬─List column (280-380px)──────┬─Detail column──────┤
│ Status       │ [SearchField (pinned)]        │  axiom-swiftui  [Managed]│
│ ● Skills     │ [Sort: Name] [Group: None]    │  ──────────────────────  │
│ Health (3)   │ ────────────────────────────  │  SOURCE  ~/.claude/…     │
│              │ axiom-build                   │  HASH    sha256:a3f9c1…  │
│              │   claude-plugins · managed    │  SYNC    2 minutes ago   │
│              │ axiom-concurrency             │                          │
│              │   claude-plugins · managed    │  [Open] [Copy] [DISABLE] │
│              │ ● axiom-swiftui               │  ──────────────────────  │
│              │   claude-plugins · managed    │  # axiom-swiftui         │
│              │ brainstorming                 │                          │
│              │   dotfiles-skills · local     │  Lorem ipsum dolor…      │
│              │ … (TanStack Virtual …2000)    │  ## When to use          │
│              │                               │  - bullet                │
└──────────────┴───────────────────────────────┴──────────────────────────┘
```

- **List column:** `SearchField` pinned (always visible); below it a single toolbar row with two `PopupMenu`s — `Sort` (Name | Source | Recent — default Name) and `Group` (None | Source | Role — default None). Below the toolbar: `TanStack Virtual` viewport of `SkillListRow`s.
- **Right-click on any list row** opens a context menu with three items: `Open source folder`, `Copy path`, `Disable on this machine` — D-07. React Aria `Menu` triggered by `onContextMenu`.
- **Detail column:** `DetailHeader` (fixed) above `MarkdownBody` (scrolls). Empty-selection placeholder when no row selected.
- **Disable on this machine** flow: button click → `commands.setSkillDisabled(name, true)` → Rust writes `machine.toml` → file watcher fires `machine-prefs-changed` → React re-fetches → list row + detail header re-render with the `Badge--disabled` showing the **"Disabled"** label (D-03 silent refresh; selection preserved per D-03).

### Health (with preview popover open — D-09..D-12)

```
┌─Window────────────────────────────────────────────────────────────┐
│ Titlebar                  tome — Health                           │
├─Sidebar──────┬─ContentPane (single-pane)─────────────────────────┤
│ Status       │ Health                                              │
│ Skills       │                                                     │
│ ● Health (3) │ AUTO-FIXABLE  (3)                                   │
│              │ ⚠ Broken library symlink                            │
│              │    library/legacy-helper points to a missing target │
│              │                                              [Fix]  │
│              │           ┌─PreviewPopover─────────────┐            │
│              │           │ PREVIEW                    │            │
│              │           │ Remove broken symlink      │            │
│              │           │ library/legacy-helper      │            │
│              │           │ This change is reversible. │            │
│              │           │      [Cancel] [Apply]      │            │
│              │           └────────────────────────────┘            │
│              │ ⚠ Stale manifest entry                              │
│              │    old-plugin-skill no longer on disk        [Fix]  │
│              │ ⚠ Stale target symlink                              │
│              │    in ~/.claude/skills                       [Fix]  │
│              │                                                     │
│              │ NEEDS ATTENTION  (2)                                │
│              │ ⛔ Unparsable SKILL.md frontmatter                   │
│              │    broken-frontmatter-skill                         │
│              │    Edit the file's YAML frontmatter to fix.         │
│              │ ⛔ Diverging target content                          │
│              │    drifted-skill                                    │
│              │    Re-sync or restore from backup to reconcile.     │
└──────────────┴─────────────────────────────────────────────────────┘
```

- Two `SectionHeader`s (auto-fixable / manual). Counts come from `DoctorReport`.
- Per-item only; **no "Fix all" button anywhere in Phase 26** (D-10).
- Failed-fix rows persist with their `TomeError` disclosure (D-11) — see `FindingRow` failed-fix state.
- **All-clear:** when the live `DoctorReport.findings` count is zero, the entire pane is replaced by the centred all-clear state and the sidebar `NavItem` badge clears (D-12).

### Empty-selection / All-clear / Transient Updated

See `States` subsection of Component Contract.

---

## Keyboard Map (NF-02)

Fills out CONTEXT §"Claude's Discretion" — keyboard map beyond the named ⌘F. macOS HIG conventions. **Every interactive element is reachable** via Tab order; the shortcuts below are accelerators.

| Shortcut | Action | Scope | Mapped to |
|----------|--------|-------|-----------|
| `⌘1` | Jump to **Status** | Global | `Sidebar` `NavItem` selection |
| `⌘2` | Jump to **Skills** | Global | Same |
| `⌘3` | Jump to **Health** | Global | Same |
| `⌘F` | Focus **Search skills** | Skills view (D-04) | `SearchField` focus |
| `Esc` | Clear search if focused; otherwise close `PreviewPopover` / `ContextMenu` | Contextual | React Aria primitives |
| `↑` / `↓` | Move selection in `SkillListRow` virtualizer / `NavItem` listbox / `FindingRow` list | Skills, Sidebar, Health | React Aria `ListBox` |
| `Home` / `End` | First / last row in current list | Skills, Health | React Aria `ListBox` |
| `Page Up` / `Page Down` | Page through `SkillListRow` virtualizer | Skills list | TanStack Virtual + React Aria |
| `Enter` | Activate selected row (Skills: open detail; Sidebar: switch section; Health: open `PreviewPopover` for an auto-fixable row) | Contextual | — |
| `Space` | Same as Enter for buttons/listbox items (React Aria default) | Global | — |
| `⌘C` | Copy source path of the selected skill | Skills detail or list row when focused | Wired to the same handler as the "Copy path" action button |
| `⌘O` | Open source folder of the selected skill | Skills detail or list row when focused | Same handler as "Open source folder" |
| `⌘D` | Disable selected skill on this machine | Skills detail or list row when focused | Same handler as "Disable on this machine" |
| `Shift+F10` / `⌃Space` | Open context menu on focused list row | Skills list | React Aria `Menu` |
| `⌘W` | Close window (native macOS, hands to Tauri) | Global | OS |
| `⌘,` | (Reserved for Phase 28 Settings — no-op in Phase 26 with a tooltip "Available in beta") | Global | — |

**Reserved (do NOT bind in Phase 26 — they belong to later phases per NF-02):** `⌘R` (Sync, Phase 27); `⌘N` (Add, Phase 28); `⌘Z` / `⌘⇧Z` (Undo / Redo, Phase 30 backup-restore).

### VoiceOver labels — explicit contracts

The action triplet, badges, and transient pill carry these `aria-label`s verbatim. (Component contracts above list each; consolidated for the planner's reference.)

| Element | `aria-label` template |
|---------|----------------------|
| Sidebar NavItem (selected) | `${name}, ${section} section, selected` |
| Sidebar NavItem (Health, with badge) | `Health, Health section, ${count} health issues` |
| `SkillListRow` (default) | `${name}, source ${sourceName}, ${managed ? 'managed' : 'local'}` |
| `SkillListRow` (disabled) | `${name}, source ${sourceName}, ${managed ? 'managed' : 'local'}, disabled on this machine` |
| Detail header — Open source folder | `Open source folder for ${skillName} in Finder` |
| Detail header — Copy path | `Copy source path for ${skillName} to clipboard` |
| Detail header — Disable | `Disable ${skillName} on this machine` |
| `Badge--managed` | (decorative; part of parent `aria-label`) |
| `Badge--disabled` | (decorative; part of parent `aria-label`) |
| `Pill — Updated` | `role="status"` `aria-live="polite"` reads its own text "Updated" once on appearance |
| `FindingRow` (auto-fixable) | `Warning finding: ${title}. ${description}. Fix available.` |
| `FindingRow` (manual) | `Blocked finding: ${title}. ${description}. Manual remediation required.` |
| `PreviewPopover` | `aria-labelledby` → PREVIEW header; `aria-modal="true"` |
| All-clear state | `role="status"` `aria-label="Library health: everything looks healthy"` |

---

## Copywriting Contract

All strings below are **verbatim**. Match the mockup tone — calm, descriptive, no marketing voice, no emoji.

| Element | Copy |
|---------|------|
| Primary CTA (the lone Phase 26 mutation) | **Disable on this machine** |
| Secondary CTAs | **Open source folder** · **Copy path** |
| Preview popover header | **PREVIEW** (uppercase caption) |
| Preview popover apply / cancel | **Apply** / **Cancel** |
| Preview popover helper text (fixable repair) | **This change is reversible by running tome sync.** |
| Empty-selection placeholder (detail column heading) | **Select a skill to view details** |
| Empty-selection sub-line | _(none — single-line placeholder per CONTEXT §"Claude's Discretion")_ |
| Search field placeholder | **Search skills** |
| Sidebar sections header (above the three NavItems) | **LIBRARY** (uppercase caption) |
| Sidebar footer | **tome · {N} skills** (no version number; the mockup's "tome 0.4.2" is excluded per extract §"NOT part of the design system") |
| `Pill — Updated` text | **Updated** |
| Disabled badge label | **Disabled** (verbatim; **supersedes** the mockup's `OFF` — see open item 3) |
| Status — Lockfile in-sync | **In sync** (with `StatusDot` `--success`) |
| Status — Lockfile out-of-sync | **Out of sync** (with `StatusDot` `--danger`) |
| Status — Last sync (never) | **Never** |
| Health — section heading (auto-fixable) | **AUTO-FIXABLE** (caption, with `(${count})`) |
| Health — section heading (manual) | **NEEDS ATTENTION** (caption, with `(${count})`) |
| Health — all-clear heading | **Everything looks healthy** |
| Health — all-clear sub-line | **No findings. The library, distribution targets, and manifest are in sync.** |
| Manual finding — unparsable frontmatter (title + remediation hint) | Title: **Unparsable SKILL.md frontmatter — ${skillName}** · Hint: **Edit the file's YAML frontmatter so it parses (delimiters `---`, valid keys). Then re-open Health.** |
| Manual finding — diverging target content (title + remediation hint) | Title: **Diverging target content — ${skillName}** · Hint: **Re-run `tome sync` to consolidate, or restore the affected target from backup. Then re-open Health.** |
| Fix failed (inline `TomeError` row) | `[${code}] ${message}` (e.g. `[Permission] failed to remove /path/to/link`). Below that: collapsible disclosure **Show context** → vertical list of `context: Vec<String>` entries. |
| Selected-skill-was-removed announcement (D-03 edge case) | `role="status"` reads **Selected skill was removed.** |

### Error state copy (general — for any unexpected `TomeError` not handled by a specific surface)

Reuses the Phase 25 scaffold's pattern in `App.tsx`:

```
[<code>] <message>
  • <context item 0>
  • <context item 1>
```

Rendered in a `--danger`-bordered banner at the top of whichever pane raised the error. Body copy `--text-body`, code prefix `--text-caption` (uppercase). Distinct from a per-row fix failure (those stay inline on the `FindingRow`).

### Destructive operations

- **Phase 26 has exactly one state-changing operation:** "Disable on this machine" (D-06).
- **Is it destructive?** Per NF-04, "destructive operation" = something that modifies on-disk state that the user could regret. Disabling is **reversible from any machine** (re-enabling is a Phase 28+ surface, but the CLI can re-enable today). So it is **not** classified as destructive in the NF-04 sense. **No confirmation dialog.** Single click on the primary button writes `machine.toml` and the watcher reflects the change.
- **The other state-changing operations in Phase 26 — doctor "Fix" repairs — ARE destructive** (they touch the filesystem). NF-04 is satisfied by D-09's preview-then-confirm `PreviewPopover` on every Fix.

---

## Registry Safety

| Registry | Blocks Used | Safety Gate |
|----------|-------------|-------------|
| shadcn official | none | not applicable — shadcn not initialised (D-14/D-15 stack incompatible) |
| Third-party registries | none declared | not applicable |

Component sources are direct npm packages governed by the project's existing dependency-audit policy (`cargo deny` for Rust; `npm audit` is not yet wired — the planner should add it in 26-07 if it's not already CI-green). The four new npm dependencies the planner introduces in Phase 26:

| Package | Purpose | License |
|---------|---------|---------|
| `react-aria` (or specific `@react-aria/*` subpackages) | Headless a11y primitives (D-14) | Apache-2.0 |
| `react-stately` | State for the React Aria primitives | Apache-2.0 |
| `@tanstack/react-virtual` | List virtualisation (D-14, NF-01) | MIT |
| `react-markdown` + `remark-gfm` | Markdown rendering (D-08, VIEW-04) | MIT |

No third-party shadcn registries; the npm-package vetting gate from `<design_contract_questions>` is **not applicable**.

---

## Open Items (carry to the planner — NOT silently resolved)

These four open items come straight from `26-DESIGN-EXTRACT.md` §"Open items for the researcher/planner" and the doc-consistency flag in D-08. They are **explicit follow-ups**, not blockers for plan generation. Address each during 26-07 (HIG audit) or earlier as noted.

1. **Pull exact dark tokens** (dark window node) or confirm the Apple-pair placeholders in §Color now embedded in this spec.
   - **Owner:** planner. **Trigger:** before alpha visual sign-off.
   - **Action:** Re-run the Figma desktop MCP against the dark Skills node (file `xl7bEUqwDz1fO6Ar83ENZI`, dark Skills frame) OR boot a running Tauri window with the placeholder tokens and verify against macOS Mail/Notes dark-mode chrome side-by-side. If divergence > minimal, file revisions against this spec's `--bg-window` / `--label-primary` / `--label-secondary` / `--bg-subtle` / `--sidebar-material` / `--separator` dark values.

2. **Confirm the 4px spacing grid + key anchors** (sidebar width 210px, titlebar 44px, nav-item 26px, list-row 52px, popover max-width 320px).
   - **Owner:** planner during 26-01 / 26-02 layout work.
   - **Action:** Build the shell at the declared anchors first; verify proportions feel like Mail/Notes/Xcode rather than a marketing page. If a value reads wrong at native macOS density, propose a delta in the plan's review tail rather than ad-hoc adjusting.

3. **Standardise the Disabled badge** label/styling. The mockup uses `OFF`; D-06 + this spec lock **"Disabled"** as the canonical label (consistent with `Disable on this machine` button) and `Badge--disabled` neutral styling (light `#f2f2f4` / `#6e6e73`).
   - **Status:** Resolved in this spec — recorded here so the planner does not regress to the mockup wording.

4. **Reconcile markdown subset wording.** `REQUIREMENTS.md` VIEW-04 says "same Markdown subset as `browse/markdown.rs`" — this is **superseded** by ROADMAP SC#4 + this spec's `MarkdownBody` allow-list (headings H1–H3, lists, links, code blocks, inline bold/italic/code). `browse/markdown.rs` is a ratatui-only hand-rolled renderer (headers + horizontal rules + inline bold-italic-code only — no lists, links, or code blocks); it is **not reused** for a webview.
   - **Owner:** planner. **Action:** during 26-04 (Markdown preview plan), file a small REQUIREMENTS.md cleanup commit updating VIEW-04's literal wording to point at SC#4 and this spec. Non-blocking for implementation.

---

## Checker Sign-Off

- [ ] Dimension 1 Copywriting: PASS
- [ ] Dimension 2 Visuals: PASS
- [ ] Dimension 3 Color: PASS
- [ ] Dimension 4 Typography: PASS
- [ ] Dimension 5 Spacing: PASS
- [ ] Dimension 6 Registry Safety: PASS

**Approval:** pending
