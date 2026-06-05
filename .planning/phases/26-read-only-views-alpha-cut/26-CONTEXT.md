# Phase 26: Read-only views — alpha cut - Context

**Gathered:** 2026-05-27
**Status:** Ready for planning

<domain>
## Phase Boundary

Ship the read-only half of the tome Desktop GUI — the **first user-visible UI** — built on the React scaffold chosen in Phase 25. Five surfaces:

- **VIEW-01** Status dashboard (resolved `tome_home`, library dir, directories with role/type badges, skill count, last sync, lockfile state, machine-pref summary)
- **VIEW-02 / NF-01** Virtualised skill list (≥2000 skills @ 60fps), fuzzy search, sort, group-by
- **VIEW-03** Skill detail pane (frontmatter, source path, content hash, last sync, managed/local + disabled badges, actions)
- **VIEW-04** SKILL.md markdown preview
- **VIEW-05** Doctor health pane with one-click fixes
- **VIEW-06** File-watcher auto-refresh (no drift from on-disk state)

…all keyboard + VoiceOver accessible (**NF-02**), in a native macOS menu-bar shell (**NF-03**), concurrency-safe with the CLI sharing the same lockfile/manifest (**NF-05**). After this phase the app is useful for **inspection**; sync, configuration, and mutating operations belong to Phases 27–31.

**The one mutation in scope:** "disable on this machine" (a `machine.toml` write) ships here as a deliberate exception (see D-06) — otherwise the phase is non-mutating.

Carried-forward locked decisions (do NOT relitigate):
- **D-GUI-04** Frontend = **React 19** (irreversible from this phase). See the ADR.
- **D-GUI-07 / NF-05** App + CLI share `tome.lock` + `.tome-manifest.json`; file watcher reloads on external change. No GUI-private state.
- **D-GUI-08 / "no JS-side business logic"** Domain calls return structured types; the GUI renders results and dispatches commands only. Validation/planning/side-effects stay in Rust.
- **D-GUI-06** macOS only for v1.0.
- Phase 25 left a working scaffold: one Tauri command `get_status` → `StatusReport`, rendered as a single scrolling dashboard in `crates/tome-desktop/ui/src/App.tsx`; `TomeError` boundary, `ProgressSink`, `CancellationToken` all in place.

</domain>

<decisions>
## Implementation Decisions

### App shell & navigation
- **D-01:** Top-level window is a **3-column NavigationSplitView** (sidebar sections → middle list → right detail+preview), Mail/Notes/Xcode style. Chosen so Phases 27–31 (Sync/Config/Backup) slot in as additional sidebar sections rather than forcing a re-layout. Replaces the scaffold's single-scroll dashboard.
- **D-02:** Sidebar is **flat: Status / Skills / Health**. App **lands on Status** on launch. The **Health item shows a badge count** when doctor findings exist (clears to none at zero findings).
- **D-03:** VIEW-06 refresh behavior = **silent live re-render** — the UI never drifts from disk; no "refresh available" prompt. A transient "Updated" note near the last-sync field acknowledges a watcher-driven refresh (fades ~2s). The **current selection (open skill) is preserved across refresh**.
- **D-04:** VIEW-02 list controls = **always-on search field** pinned at the top of the list column (⌘F focuses it; fuzzy as-you-type matching the CLI's `nucleo` ranking) + **toolbar popup menus** for sort (name/source/recent) and group-by (none/source/role). **Defaults: sort=name, group=none.**

### Detail + preview + the lone mutation
- **D-05:** Right column = **compact metadata header + scrolling markdown body**. Header shows name, managed/local + disabled badges, source path, content hash, last sync, and the action buttons; the rendered SKILL.md body scrolls beneath. Mirrors the browse TUI's skill view.
- **D-06:** **"Disable on this machine" SHIPS in Phase 26** as a live `machine.toml` write (not deferred). Rationale: it's a single, well-bounded write through the existing machine-prefs path (same as browse TUI / CLI), low-risk, genuinely useful in an inspector, keeps VIEW-03 at full parity with the TUI it replaces, and **exercises the write → file-watcher → silent-refresh loop early** (de-risks Phases 27–31). The skill then shows a "disabled" badge.
- **D-07:** The three actions (**open source dir / copy path / disable on this machine**) are accessible from **both** the detail-pane header (primary buttons) **and** a right-click context menu on list rows.
- **D-08:** Markdown preview (VIEW-04) **renders in React via a mature markdown lib** (e.g. `react-markdown`/`remark`) at the **SC#4 subset: headings, lists, links, code blocks, inline bold/italic/code**. Markdown→HTML is treated as presentation (not "business logic"), so it does not violate the no-JS-logic constraint.
  - **⚠ Doc-consistency flag for the planner:** VIEW-04's wording "same Markdown subset as `browse/markdown.rs`" is **superseded by roadmap SC#4**. `browse/markdown.rs` is a ratatui-only, hand-rolled renderer that handles ONLY headers / horizontal rules / inline bold-italic-code (no lists, links, or code blocks) and cannot be reused for a webview. The real target is the richer SC#4 set. Reconcile the requirement text.

### Doctor health & fix safety
- **D-09:** Confirmation model = **preview-then-confirm per fix**. Clicking "Fix" opens a small popover showing exactly what will change (reuse `doctor.rs` per-item dry-run/plan descriptions), then "Apply". Satisfies **NF-04** literally for every repair. (All four `RepairKind` variants mutate the filesystem — see code context.)
- **D-10:** **Per-item fixes only** in alpha. **No bulk "Fix all"** button in Phase 26 (a library with many findings is itself worth reviewing item-by-item; bulk can be added later if it proves tedious).
- **D-11:** Fix outcomes surface **inline on the finding row**: success → finding drops (watcher refresh reconciles); failure → row **stays visible** with the inline `TomeError` (`[Permission] …`) and the context chain in a disclosure. **Failures must never be silently swallowed** (SAFE-01 semantics).
- **D-12:** **Non-fixable findings** (unparsable SKILL.md frontmatter, diverging target content) render in the list with an **explanation + manual remediation hint and NO Fix button** (never a dead control). **Zero findings → explicit all-clear state**; the sidebar Health badge clears.

### Visual fidelity & components
- **D-13:** Aesthetic bar = **HIG-polished from the start**. Native-feeling macOS look (system font stack, proper spacing/typography/density, real list+sidebar styling, light+dark). The shell is inherited by every Phase 27–31 view, so the foundation is polished once rather than retrofitted at rc.
- **D-14:** Component/a11y foundation = **React Aria (Adobe headless primitives) + custom macOS styling** for NF-02 (keyboard + VoiceOver) and NF-03 (HIG), with **TanStack Virtual** for the VIEW-02 / NF-01 2000-row list. This is a **compounding, semi-irreversible choice** like the framework pick (it carries the a11y requirement across all six UI phases).
- **D-15:** Styling = **per-component CSS Modules (`*.module.css`) + a small set of CSS custom-property design tokens** (colors/spacing/type) driven by `prefers-color-scheme` for light/dark. Zero-runtime, Vite-native, no heavy dependency; tokens keep the HIG palette consistent across phases.
- **D-16:** Window chrome = **unified native titlebar/toolbar + traffic-light controls + a vibrancy/translucent sidebar material** (Tauri macOS window effects), Mail/Notes-style. **Follow system light/dark, NO in-app theme switcher** (NF-03). **Respect reduce-transparency** (solid fallback).

### Claude's Discretion
- **Default behaviors not separately asked:** SKILL.md links open in the system browser (Tauri opener); code blocks render plain (light syntax highlighting optional); empty-selection detail pane shows a neutral placeholder.
- **Status dashboard exact field layout** (cards vs table grouping) — pick a HIG-aligned arrangement that renders every `StatusReport` field; the scaffold's card+table mix is a fine starting point.
- **Doctor pane flat-vs-grouped layout** (e.g. "auto-fixable" vs "needs attention" sections) — Claude's discretion.
- **Exact frontmatter fields shown + badge styling** — render what `lint.rs` parses; style to the design tokens.
- **React Aria vs Radix** — D-14 locks "headless a11y primitives + custom styling + TanStack Virtual"; research/planning may confirm React Aria vs Radix on current VoiceOver maturity if a concern surfaces, but React Aria is the chosen default.
- **NF-01 perf-bench harness shape** (synthetic 2000-skill generator location, CI wiring) — planning detail (draft plan 26-08).
- **Keyboard-shortcut map (NF-02)** beyond the named ⌘F (search) / ⌘R (sync, later) — fill out per macOS HIG during planning (draft plan 26-07).

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### Milestone scope + locked decisions
- `.planning/REQUIREMENTS.md` — VIEW-01..06 + NF-01..05 full text; D-GUI-01..09 key decisions; constraints (no CLI regression, no JS-side business logic, strict Tauri 2.x, macOS-only, hardened runtime). **NF-04** (destructive ops confirm) governs D-09.
- `.planning/ROADMAP.md` §"Phase 26: Read-only views — alpha cut" — promoted detail section: goal, requirements, 7 success criteria, 8 draft plans. **SC#4** is the authoritative markdown-subset target (supersedes VIEW-04's "browse/markdown.rs" wording — see D-08).
- `.planning/milestones/v1.0-ROADMAP.md` §"Phase 11: Read-Only Views" — the milestone draft this phase was promoted from (local Phase 11 == global Phase 26); draft plan stubs 11-01..11-08.

### Foundation laid in Phase 25 (the substrate this phase builds on)
- `.planning/phases/25-rust-core-extraction-tauri-integration-spike/25-CONTEXT.md` — D-01..D-17: structured types stay in `crates/tome` (D-05), `specta` `bindings` feature gate (D-06), `gen-bindings` bin + committed `bindings.ts` + CI freshness gate (D-07), `SkillOwnership` enum (D-08), `ProgressSink`/`ProgressEvent`/`CancellationToken` (D-09..D-12), `TomeError`/`ErrorCode` boundary (D-13..D-16), "structure at the edge" symmetry (D-17).
- `.planning/research/v1.0-frontend-framework-decision.md` — the React ADR. Names TanStack Virtual (NF-01), React Aria / Radix (NF-02/NF-03) as the ecosystem bet (informs D-14); records the `Result<StatusReport, TomeError>` discriminated-union narrowing pattern the GUI uses everywhere; lists invalidation conditions (NF-01 must hold 60fps in React).

### Code being extended / reused
- `crates/tome-desktop/ui/src/App.tsx` — the Phase 25 scaffold (single dashboard, `commands.getStatus()`, `Result` union narrowing, `TomeError` rendering) that VIEW-01 evolves from.
- `crates/tome-desktop/src/commands.rs` — current Tauri command surface (`get_status` only). Phase 26 adds commands for list/detail/doctor/actions; follow its `load_context()` + `.map_err(TomeError::from)` boundary pattern.
- `crates/tome-desktop/ui/src/bindings.ts` — the committed generated bindings (regenerated via `gen-bindings`); new boundary types must flow through it (CI freshness gate).
- `crates/tome/src/status.rs` — `StatusReport` (VIEW-01 source).
- `crates/tome/src/doctor.rs` — `diagnose()`, `IssueSeverity`, `IssueCategory`, and the 4 `RepairKind` variants + per-item dry-run descriptions reused by D-09 (VIEW-05).
- `crates/tome/src/browse/` — the TUI being functionally replaced: `app.rs` (skill actions, selection), `fuzzy.rs` (nucleo ranking → D-04), `markdown.rs` (the minimal renderer D-08 supersedes), `theme.rs` (light/dark reference).
- `crates/tome/src/lint.rs` — frontmatter parsing reused for VIEW-03 detail.
- `.planning/codebase/ARCHITECTURE.md` — layer map; grounds where new domain calls (`list::collect`, doctor repair handlers) and any new commands belong.

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets
- **`StatusReport` (`status.rs`)** already structured + specta-derived; VIEW-01 renders it (App.tsx already does a first cut).
- **`doctor.rs::diagnose()` + 4 `RepairKind` variants** (`RemoveStaleManifestEntry`, `RemoveBrokenLibrarySymlink`, `RemoveStaleTargetSymlink`, `ConsolidateTargetRealDirToSymlink`) — VIEW-05 calls the same repair handlers the interactive CLI uses; each variant has a human dry-run description reused for D-09's confirm popover. **All four mutate the filesystem** → NF-04 applies to all (drives D-09).
- **`browse/fuzzy.rs`** (nucleo-matcher) — the ranking VIEW-02 search should match (D-04).
- **The plan/render/execute pattern** (`remove`/`reassign`/`relocate`/`eject` promoted to `pub` in Phase 25) — the GUI consumes plan structs directly; not used heavily in 26 but the pattern context matters for later phases.
- **`TomeError` boundary + `Result<T, TomeError>` tauri-specta union** — the established command-result shape; reuse for every new command (failures surface per D-11).

### Established Patterns
- **`commands.rs::load_context()`** resolves the real `tome_home` + `Config` exactly as the flag-free CLI does — new read commands should reuse it so the GUI observes the same state.
- **`.map_err(TomeError::from)` at the command edge** (D-13 from Phase 25) — keep classification at the boundary; domain stays `anyhow`.
- **Committed `bindings.ts` + `gen-bindings` + CI `git diff --exit-code`** — any new cross-boundary type must be regenerated and committed or CI fails.
- **Newtype identifiers** (`SkillName`, `DirectoryName`, `ContentHash`) cross as specta types — verify transparent-newtype handling on any new ones.

### Integration Points
- **New Tauri commands** for: skill list (`list::collect` → a `ListReport`-shaped type), skill detail (frontmatter via `lint.rs`), doctor (`diagnose` + per-`RepairKind` fix execution), and the "disable on this machine" machine-prefs write (D-06).
- **File watcher (VIEW-06):** a new watcher (Rust side, likely `notify`) over manifest/lockfile/library → emits a Tauri event the React app subscribes to → silent re-render (D-03). Must be concurrency-safe with CLI writes (NF-05).
- **Frontend deps to add:** React Aria, TanStack Virtual, a markdown lib (react-markdown/remark) — all on the React `ui/` side; bundle impact noted against the ADR's bundle concern (the NF-01 budget is met at the virtualization layer, not raw framework reactivity).

</code_context>

<specifics>
## Specific Ideas

- **Mail/Notes as the spatial reference** — the 3-column split, translucent sidebar, and unified toolbar should read as a stock macOS document/library app.
- **"GUI cannot drift from disk"** is the felt quality to optimize for (D-03): live, silent reconciliation over manual refresh prompts.
- **Parity with the browse TUI** is the bar for VIEW-03 actions (D-05/D-06/D-07) — the GUI replaces `tome browse`, so its skill view should do at least what the TUI did.
- Polish the **shell foundation once** (D-13/D-14/D-15/D-16) because Phases 27–31 inherit it — treat the alpha shell as the visual contract for the whole milestone, even though alpha is an internal cut.

</specifics>

<deferred>
## Deferred Ideas

- **Interim `v0.17.0` release** — ship the unreleased #542 `SkillOwnership` manifest migration + Phase 25 `lib.rs` refactor to CLI users *before* v1.0, isolating the schema migration in its own small reviewable release. Optional, **non-blocking for Phase 26**; user to decide separately. Note: `crates/tome-desktop` is cargo-dist-excluded (`dist = false`), so any release now is CLI-only (no GUI). Current version is `v0.16.0`. (Also: the `CLAUDE.md` "Current State" header still says v0.9.0 — stale; should be refreshed.)
- **Bulk "Fix all" in the Health pane** — explicitly deferred from alpha (D-10); revisit if per-item fixing proves tedious in practice.
- **Light syntax highlighting in code blocks** — optional polish, not required for SC#4 (D-08 default is plain).
- **Sync / Config / Backup / mutating-ops UI** — Phases 27–31; out of scope. Any such suggestion during planning is creep.
- **SKILL.md editing** — read-only in v1.0 (deferred to v2 GUI-EDIT-01).

</deferred>

---

*Phase: 26-read-only-views-alpha-cut*
*Context gathered: 2026-05-27*
