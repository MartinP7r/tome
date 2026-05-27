# Phase 26: Read-only views — alpha cut - Discussion Log

> **Audit trail only.** Do not use as input to planning, research, or execution agents.
> Decisions are captured in CONTEXT.md — this log preserves the alternatives considered.

**Date:** 2026-05-27
**Phase:** 26-read-only-views-alpha-cut
**Areas discussed:** App shell & navigation, Detail + preview + the lone mutation, Doctor health & fix safety, Visual fidelity & components

> **Pre-discussion note:** Phase 26 existed only as a ROADMAP checklist item + milestone-draft detail; the `### Phase 26` detail section was missing from the live `ROADMAP.md`, so `init.phase-op` reported `phase_found: false`. The user chose "Promote it, then discuss" — the detail section was transcribed from `milestones/v1.0-ROADMAP.md` (local Phase 11 → global Phase 26, +15 offset) and committed before the discussion.

---

## App shell & navigation

### Window structure
| Option | Description | Selected |
|--------|-------------|----------|
| 3-column split view | NavigationSplitView (sidebar → list → detail+preview), Mail/Notes-style; scales to Phases 27–31 | ✓ |
| Top-level tabs | Segmented control switching Status/Skills/Health full-screen views | |
| Single scrolling dashboard | Keep the Phase 25 scaffold shape | |

### Sidebar organization + landing
| Option | Description | Selected |
|--------|-------------|----------|
| Status / Skills / Health, land on Status | Flat sidebar, dashboard-first | ✓ |
| Skills / Status / Health, land on Skills | Browsing-first | |
| Grouped sidebar w/ directories | Directories as filter list | |

### File-watcher refresh (VIEW-06)
| Option | Description | Selected |
|--------|-------------|----------|
| Silent live re-render | Auto-update in place + transient "Updated" note | ✓ |
| Non-blocking "refresh available" banner | Detect but require a click to apply | |
| You decide | | |

### List controls (VIEW-02)
| Option | Description | Selected |
|--------|-------------|----------|
| Always-on search + toolbar menus | Pinned search (⌘F) + toolbar sort/group popups | ✓ |
| Search bar only; sort/group in View menu | Menu-bar-native | |
| You decide | | |

**User's choice:** 3-column split; Status/Skills/Health landing on Status; silent live re-render; always-on search + toolbar menus (defaults sort=name, group=none).
**Notes:** Health sidebar item carries a badge count when findings exist; selection preserved across refresh.

---

## Detail + preview + the lone mutation

### Detail + markdown composition
| Option | Description | Selected |
|--------|-------------|----------|
| Metadata header + scrolling preview | Header (badges/path/hash/actions) + markdown body below | ✓ |
| Tabbed: Info \| Preview | Two tabs | |
| Split: metadata pane + preview pane | Always-visible split | |

### The lone mutation ("disable on this machine")
| Option | Description | Selected |
|--------|-------------|----------|
| Ship it — write machine.toml now | Live, bounded write; exercises write→watcher→refresh loop | ✓ |
| Show read-only, defer the write | Inert until a later phase | |
| You decide | | |

### Action placement
| Option | Description | Selected |
|--------|-------------|----------|
| Detail header + right-click menu | Buttons in detail + context menu on rows | ✓ |
| Detail header buttons only | | |
| Context menu only | | |

### Markdown rendering (VIEW-04)
| Option | Description | Selected |
|--------|-------------|----------|
| JS render, fuller SC#4 subset | react-markdown: headings/lists/links/code blocks/inline | ✓ |
| Rust render command, authoritative subset | New Rust command returns sanitized HTML/tokens | |
| You decide | | |

**User's choice:** Metadata-header + scrolling preview; "disable on this machine" ships as a live write; actions in header + right-click menu; markdown via react-markdown at SC#4 subset.
**Notes:** Surfaced finding that `browse/markdown.rs` (ratatui-only, no lists/links/code blocks) conflicts with SC#4 — SC#4 wins; flagged for planner to reconcile VIEW-04 wording. Defaults captured: links open in system browser, code blocks plain.

---

## Doctor health & fix safety

### Confirmation model (NF-04)
| Option | Description | Selected |
|--------|-------------|----------|
| Preview-then-confirm per fix | Popover shows what changes, then Apply | ✓ |
| Tiered: silent for safe, confirm for risky | Only gate ConsolidateTargetRealDirToSymlink | |
| Single batched confirm | One confirm for all selected | |

### Fix granularity
| Option | Description | Selected |
|--------|-------------|----------|
| Per-item only for alpha | No bulk button in 26 | ✓ |
| Per-item + "Fix all auto-fixable" | Batched confirm | |
| You decide | | |

### Outcome surfacing
| Option | Description | Selected |
|--------|-------------|----------|
| Inline on the finding row | Success drops row; failure stays with TomeError + context | ✓ |
| Toast notifications | Transient corner toasts | |
| You decide | | |

### Non-fixable findings + healthy state
| Option | Description | Selected |
|--------|-------------|----------|
| Show with guidance, no button; clean empty state | Manual hint instead of Fix; explicit all-clear | ✓ |
| Separate "Needs attention" vs "Auto-fixable" sections | Grouped triage | |
| You decide | | |

**User's choice:** Preview-then-confirm per fix; per-item only; inline outcomes with visible failures; non-fixable = guidance + clean healthy state.
**Notes:** All 4 `RepairKind` variants mutate the filesystem → NF-04 applies to every fix. Failures must never be silently swallowed (SAFE-01).

---

## Visual fidelity & components

### Aesthetic bar
| Option | Description | Selected |
|--------|-------------|----------|
| HIG-polished from the start | Native macOS look now; shell inherited by all phases | ✓ |
| Functional-but-plain, polish later | | |
| You decide | | |

### Component / a11y foundation
| Option | Description | Selected |
|--------|-------------|----------|
| React Aria (headless) + own styling | Adobe primitives + TanStack Virtual; ADR lead candidate | ✓ |
| Radix Primitives + own styling | | |
| Hand-roll on semantic HTML | | |
| You decide | | |

### Styling approach
| Option | Description | Selected |
|--------|-------------|----------|
| CSS Modules + design tokens | Scoped modules + custom-property tokens, prefers-color-scheme | ✓ |
| Tailwind CSS | | |
| Plain global CSS | | |
| You decide | | |

### Window chrome (NF-03)
| Option | Description | Selected |
|--------|-------------|----------|
| Native chrome + translucent sidebar | Vibrancy material, Mail/Notes look | ✓ |
| Native chrome, solid surfaces | | |
| You decide | | |

**User's choice:** HIG-polished from the start; React Aria + custom styling + TanStack Virtual; CSS Modules + design tokens; native chrome + translucent sidebar.
**Notes:** Component-library choice is compounding/semi-irreversible (carries NF-02 across all phases) — flagged as such. Light/dark via system, no in-app switcher (NF-03); respect reduce-transparency.

## Claude's Discretion

- Default behaviors: links open in system browser; code blocks plain; empty-selection placeholder.
- Status dashboard exact field layout; doctor pane flat-vs-grouped; exact frontmatter fields + badge styling.
- React Aria vs Radix final pick (React Aria is the default; research may confirm on VoiceOver maturity).
- NF-01 perf-bench harness shape (plan 26-08); keyboard-shortcut map beyond ⌘F/⌘R (plan 26-07).

## Deferred Ideas

- Optional interim `v0.17.0` release (unreleased #542 migration + Phase 25 refactor; CLI-only since tome-desktop is cargo-dist-excluded) — user to decide separately; non-blocking.
- Bulk "Fix all" in Health pane — deferred from alpha.
- Light syntax highlighting in code blocks — optional polish.
- Sync/Config/Backup/mutating-ops UI — Phases 27–31.
- SKILL.md editing — v2 (GUI-EDIT-01).
- Stale `CLAUDE.md` "Current State" header (says v0.9.0; actual is v0.16.0) — should be refreshed.
