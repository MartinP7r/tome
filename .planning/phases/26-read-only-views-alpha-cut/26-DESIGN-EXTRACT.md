# Phase 26: Design Extract — tokens + component inventory

**Created:** 2026-05-28
**Source:** Figma Make mockup (file `xl7bEUqwDz1fO6Ar83ENZI`), extracted via the Figma desktop MCP
from the **light Skills window** (`1:1602`) + the six screen screenshots (Status/Skills/Health × light/dark).
**Purpose:** Input artifact for `gsd-ui-researcher`. Distills the *values* from the mockup into a
starting token set + component inventory so `26-UI-SPEC.md` locks a real design system. Realises D-13/D-14/D-15.

> ⚠ **The generated code is a values reference, not buildable layout.** Figma Make emitted
> absolutely-positioned nodes with fractional, scaled sizes (10.5/12.5/13.5px) and Tailwind
> arbitrary-value classes, using **Inter** (Make has no SF Pro). The planner rebuilds with real
> fl/grid layout, the macOS **system font stack**, React Aria primitives (D-14), TanStack Virtual
> (NF-01), and CSS Modules + custom-property tokens (D-15). Values below are **normalized to a
> clean macOS-aligned scale** — do not copy the fractional px.

---

## Design Tokens

### Color — light (extracted from this file)
| Token | Value | Usage |
|-------|-------|-------|
| `--label-primary` | `#1d1d1f` | Primary text, headings |
| `--label-secondary` | `#6e6e73` | Secondary/muted text, section headers, metadata labels |
| `--bg-window` | `#ffffff` | Window content background |
| `--bg-app` | `#f5f5f7` | App canvas / behind windows |
| `--bg-subtle` | `#f2f2f4` | Subtle fills (table header zebra, inputs) |
| `--sidebar-material` | `rgba(246,246,248,0.72)` | Vibrancy sidebar (translucent); also `rgba(236,236,239,0.85)` |
| `--separator` | `rgba(0,0,0,0.08)` | Hairlines; stronger variants `0.12` / `0.15` |
| `--accent` | `#007aff` | System blue: selection fill, primary buttons, links |
| `--accent-pressed` | `#0a6cdc` | Pressed/hover accent |
| `--danger` | `#ff3b30` | Health count badge, error states |
| traffic lights | `#ff5f57` / `#febc2e` / `#28c840` | Window controls (close/min/max) |

### Color — dark (NOT yet extracted from this file)
Dark windows exist in the mockup (verified visually) but exact hexes weren't pulled — I read only the
light window. **Starting pairs** (Apple system dark equivalents, to be confirmed against the dark
window or replaced by an exact pull):
`--accent` → `#0a84ff` · `--label-primary` → `#f5f5f7` · `--label-secondary` → `#98989d` ·
`--bg-window` → `#1e1e1e` · `--bg-app` → `#000000` · `--danger` → `#ff453a`.
All driven by `prefers-color-scheme` (D-15); respect `prefers-reduced-transparency` with a solid
sidebar fallback (D-16).

### Typography
- **Family:** macOS system stack — `-apple-system, "SF Pro Text", "SF Pro Display", system-ui, sans-serif`.
  (Mockup used Inter only because Make lacks SF Pro.)
- **Weights:** 400 (regular), 500 (medium — dominant), 600 (semibold). No bold.
- **Scale** (normalized from Make's 9.5/10.5/11.88/12/12.5/13/13.5/16/22):

| Token | Size | Weight | Role |
|-------|------|--------|------|
| `--text-caption` | 11px | 500 | Section headers ("LIBRARY"), badges, metadata labels |
| `--text-footnote` | 12px | 400/500 | List-row secondary line, table values |
| `--text-body` | 13px | 400/500 | Default body, list-row primary, markdown paragraph |
| `--text-emphasis` | 14px | 500/600 | Emphasized rows, button labels |
| `--text-subhead` | 16px | 600 | Sub-section headers (markdown H2/H3, "Directories") |
| `--text-title` | 22px | 600 | View title ("Status"/"Health"), skill name |

### Radii
`--radius-xs: 3px` · `--radius-sm: 4px` · `--radius-md: 6px` (dominant — cards, buttons, list selection)
· `--radius-lg: 8px` (popover/cards) · `--radius-pill: 9999px` (capsule badges: role, "Updated").

### Spacing
Not reliably extractable (Make output is absolute-positioned). Recommend a **4px base grid**
(4 / 8 / 12 / 16 / 20 / 24) per macOS HIG, confirmed during the researcher/planner step.
Observed anchors: sidebar width ~210px, titlebar height 44px, nav-item height 26px.

---

## Component Inventory

### Shell (shared — built once per D-13)
- **Window** — unified titlebar + body; white content, vibrancy sidebar.
- **Titlebar** — traffic lights (left) + centered title ("tome — {Section}"); height 44px.
- **Sidebar** — vibrancy material; `LIBRARY` section header; nav items; footer ("tome · N skills").
- **NavItem** — icon + label; states: default / hover / **selected** (accent-filled capsule); **badge variant** (Health, red count).
- **ContentPane** — scrolling region right of sidebar; view title (22px) + optional trailing meta.

### Atoms
- **Badge — role** (Discovery / Distribution): tinted pill.
- **Badge — type** (Claude Plugins / Git / Directory): neutral pill.
- **Badge — Managed** (blue), **Badge — Disabled** (muted; mockup labels it `OFF` → standardize to "Disabled" per D-06).
- **Badge — override** ("local override"), **Pill — Updated** (green, transient, fades ~2s per D-03), **Badge — count** (red circle, sidebar Health).
- **StatusDot** (green = in sync), **SeverityIcon** (⚠ fixable / ⛔ manual).
- **Button — primary** (accent filled: "Disable on this machine", "Apply"), **— secondary** (bordered: "Open source folder", "Copy path", "Cancel"), **— Fix** (small).
- **SearchField** (magnifier + placeholder), **PopupMenu** (Sort, Group).

### Molecules / view-specific
- **KeyValueRow** (Status: label + value, optional trailing badge/dot).
- **DirectoryTable** (NAME / ROLE / TYPE columns; row = name + secondary path + role badge + type badge).
- **SkillListRow** (name + source + managed indicator + optional Disabled; selected = accent fill).
- **DetailHeader** (title + Managed/Disabled badges + metadata grid [source path / content hash / last sync] + 3 action buttons).
- **MarkdownBody** (H1–H3, ul/ol, links, inline bold/italic/`code`, fenced code block — SC#4 subset per D-08).
- **SectionHeader** (Health: "AUTO-FIXABLE" / "NEEDS ATTENTION" with count).
- **FindingRow** (severity icon + title + description + **Fix** button | **Manual** label + remediation hint).
- **PreviewPopover** ("PREVIEW" header + change line + helper text + Cancel/Apply — D-09).

### States
- **Empty selection** — neutral centered placeholder in detail pane.
- **All-clear health** — checkmark + "Everything looks healthy"; sidebar badge cleared (D-12).
- **Transient "Updated"** — watcher-driven refresh acknowledgement (D-03).

---

## NOT part of the design system (prototype scaffolding — exclude)
The "DESKTOP PROTOTYPE" title, `LIGHT/DARK APPEARANCE` labels, the `Findings / All clear` segmented
control, the "both windows share state" caption, and the invented footer version "tome 0.4.2" are
Figma Make's demo frame — **not** tome UI. Only the macOS window contents are in scope.

## Open items for the researcher/planner
1. Pull **exact dark tokens** (dark window node) or confirm the Apple-pairs above.
2. Confirm the **4px spacing grid** + key dimensions (sidebar width, row heights, popover width).
3. Standardize the **Disabled** badge label/styling (mockup shows `OFF`).
4. Reconcile **markdown subset** wording: SC#4 (rich) supersedes `browse/markdown.rs` (per D-08).
