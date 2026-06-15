# Phase 26: UI Generation Prompts (Figma Make)

**Created:** 2026-05-27
**Purpose:** External-mockup ideation step inserted ahead of UI-SPEC.md. Generate mockups in
Figma Make, pick a direction, then feed the chosen mockup image(s) + `26-CONTEXT.md` into
`gsd-ui-researcher` so the design contract is grounded in something real.

**Tool:** Figma Make · **Scope:** one comprehensive app prompt · **Latitude:** lock structure, explore aesthetics

**Decisions encoded (from `26-CONTEXT.md`):** D-01 (3-col split) · D-02 (flat sidebar, land on
Status, Health badge) · D-03 (silent live re-render "Updated" note) · D-04 (pinned search +
sort/group menus) · D-05 (detail = header + markdown body) · D-06/D-07 (the three actions,
disable-on-this-machine) · D-08 (SC#4 markdown subset) · D-09/D-10/D-11/D-12 (doctor
preview-then-confirm, per-item only, inline outcomes, non-fixable + all-clear) · D-13/D-14/D-15/D-16
(HIG-polished, vibrancy + unified toolbar + traffic lights, light+dark, no in-app theme switch).

---

## ▶ Prompt — paste into Figma Make

Design an interactive desktop app prototype: **tome** — a native macOS app for inspecting a developer's shared library of AI-coding-agent "skills." It is a **read-only inspector** (think a Mail / Notes / Xcode–style library browser), not an editor. Build it as a **macOS desktop window**, not a mobile or web layout.

### Window & chrome (fixed)
- Standard macOS desktop window with a **unified title bar + toolbar** (no separate title strip) and **traffic-light controls** (red / yellow / green) at the top-left.
- **Three-column layout** (NavigationSplitView style): a narrow **sidebar** on the left, a **list column** in the middle, a **detail column** on the right.
- The **sidebar uses a translucent / vibrancy material** (subtle blurred background), lighter than the content panes — the macOS sidebar look.
- Support **both light and dark appearance** following the macOS system look — show me both. Use the macOS system font (SF Pro / -apple-system; fall back to Inter).
- Density and styling should feel like a stock macOS productivity app — proper padding, hairline separators, rounded accent-blue selection highlights in lists.

### Sidebar (fixed structure)
A flat list of three sections, in this order, each with an SF-Symbol-style icon:
1. **Status** — the app launches here, selected by default
2. **Skills**
3. **Health** — shows a small **red badge with a count** (e.g. "6") when there are issues; no badge when zero.

### Screen 1 — Status (default)
A dashboard summarizing the library. Render these real fields (lay them out as you judge best — cards, grouped rows, or a hybrid):
- **tome home**: `~/.tome`
- **Library**: `~/.tome/library` · **2,041 skills**
- **Last sync**: "Today at 9:14 AM" — put a subtle, transient **"Updated" pill** next to this field, as if it just live-refreshed
- **Lockfile**: "In sync" (green)
- **Machine prefs**: "3 skills disabled on this machine"
- **Directories** (table or cards), each with a **role badge** (Discovery / Distribution) and a **type badge** (Claude Plugins / Git / Directory):
  - `claude-plugins` — Discovery · Claude Plugins
  - `dotfiles-skills` — Discovery · Git — `github.com/martinP7r/skills`
  - `~/.claude/skills` — Distribution · Directory
  - `codex` — Distribution · Directory
  - `antigravity` — Distribution · Directory

### Screen 2 — Skills (list + detail)
Middle **list column**:
- A **search field pinned at the very top** of the column, always visible ("Search skills"), with a magnifier icon — as-you-type fuzzy filtering.
- A small **toolbar row** with two popup menus: **Sort** (Name / Source / Recent — default Name) and **Group** (None / Source / Role — default None).
- A long, scrollable **list of skills** that looks like it holds thousands of rows (show ~25 visible). Each row: skill **name** (lowercase-hyphen), a small secondary line with its **source** and a tiny **managed vs. local** indicator; rows disabled-on-this-machine show a muted **"Disabled" badge**.
- Sample names: `brainstorming`, `systematic-debugging`, `test-driven-development`, `writing-skills`, `axiom-swiftui`, `axiom-concurrency`, `axiom-build`, `gsd-plan-phase`, `gsd-execute-phase`, `rust-cli`, `rust-lang`, `frontend-design`, `obsidian-markdown`, `snapshot-testing`, `swift-concurrency-pro`, `mcp-integration`…
- Right-click a row → **context menu**: "Open source folder", "Copy path", "Disable on this machine".

Right **detail column** (with `axiom-swiftui` selected):
- **Compact metadata header**: skill name (large), badges (**Managed**, plus **Disabled** when applicable), then a small grid of fields — **Source path** `~/.claude/plugins/axiom/skills/axiom-swiftui`, **Content hash** `sha256:a3f9c1…` (truncated, monospace), **Last sync** "2 minutes ago".
- Three **action buttons** in the header: **Open source folder**, **Copy path**, **Disable on this machine** (primary-styled; "Disable" is the only one that changes state).
- Below the header, a **scrolling rendered Markdown body** = the skill's SKILL.md. Render this subset: H1/H2/H3 headings, bullet + numbered lists, links, fenced code blocks (monospace, subtle background), inline **bold** / *italic* / `code`. Use realistic content — a heading "axiom-swiftui", a short paragraph, a "## When to use" list, and a small Swift code block.
- **Empty state**: when no skill is selected, the detail column shows a neutral, centered placeholder ("Select a skill to view details").

### Screen 3 — Health (Doctor)
A pane listing library-health findings. Each **finding row** shows a severity icon, a title, and a one-line description. Two kinds:
- **Auto-fixable** findings have a **"Fix" button**. Clicking "Fix" opens a small **popover that previews exactly what will change** (e.g. "Remove broken symlink `library/legacy-helper`") with a **Cancel / Apply** pair — preview-then-confirm. Show one popover open in the mockup.
- **Non-fixable** findings have **no Fix button** — instead an explanation + a manual-remediation hint (e.g. "Edit the file to fix the frontmatter").

Sample findings:
- ⚠ Broken library symlink — `library/legacy-helper` points to a missing target *(fixable)*
- ⚠ Stale manifest entry — `old-plugin-skill` no longer on disk *(fixable)*
- ⚠ Stale target symlink in `~/.claude/skills` *(fixable)*
- ⚠ Real directory where a symlink is expected — `~/.claude/skills/forked-skill` *(fixable: consolidate)*
- ⛔ Unparsable SKILL.md frontmatter — `broken-frontmatter-skill` *(not fixable — manual)*
- ⛔ Diverging target content — `drifted-skill` *(not fixable — manual)*

Also show the **all-clear / zero-findings** variant as a second frame: a calm "Everything looks healthy" empty state with a checkmark, and the sidebar Health badge gone.

### What to lock vs. explore
**Lock these — do not redesign:** the three-column split; the Status / Skills / Health sidebar and its order; landing on Status; the translucent sidebar + unified toolbar + traffic lights; light **and** dark; search pinned at the top of the list column; detail = metadata header above a scrolling Markdown body; the three actions; doctor preview-then-confirm with per-item Fix buttons; the inline non-fixable and all-clear states.

**Explore freely — this is what I want ideas on:** exact Status layout (cards vs. grouped table vs. hybrid); badge shapes and colors; spacing, density, and type scale; the color-token palette within a macOS look; whether Health findings are flat or grouped into "Auto-fixable / Needs attention"; the look of the transient "Updated" pill; the empty and all-clear states.

### Out of scope — do NOT add
No sync button, no settings / config screens, no add / remove / edit skill, no onboarding, no account or login, no in-app light/dark toggle (it follows the system). This is a **read-only inspector** — the only state-changing control anywhere is "Disable on this machine."

Give me the full app as interactive frames in **both light and dark**, desktop-sized.

---

## Follow-up refinement prompts (after the first generation)

Figma Make is iterative — use these to nudge the result without regenerating from scratch:

- *"Tighten the spacing and reduce font sizes — it should read denser, like Xcode's navigator, not a marketing page."*
- *"Make the sidebar more translucent and give the selected sidebar row a filled accent-blue capsule."*
- *"Show the dark-appearance version of every frame side-by-side with light."*
- *"Group the Health findings into two sections: 'Auto-fixable' and 'Needs attention'."* (only if you want to compare against the flat layout)
- *"Add a frame showing the skill list grouped by Source, with collapsible section headers."*
- *"Show the metadata-header field grid as a two-column key/value layout with monospace values."*

## Return trip — what to do with the mockups

1. Pick the direction(s) you like; export the frames as images (light + dark).
2. Re-run `/gsd:ui-phase 26` (or hand me the images directly) — I'll feed the chosen mockup(s)
   plus `26-CONTEXT.md` into `gsd-ui-researcher`, which writes `26-UI-SPEC.md` grounded in the
   mockup (tokens, spacing, type scale, badge styling extracted from what you picked).
3. `gsd-ui-checker` verifies the 6 dimensions; revision loop if needed.
