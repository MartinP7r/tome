---
phase: 03-import-reassignment-browse-polish
plan: 02
subsystem: ui
tags: [ratatui, tui, theme, markdown, fuzzy-matching, scrollbar]

# Dependency graph
requires:
  - phase: none
    provides: existing browse TUI (app.rs, ui.rs, fuzzy.rs, mod.rs)
provides:
  - Terminal-adaptive theme system (dark/light via COLORFGBG)
  - Fuzzy match character highlighting in skill names
  - Scrollbar for long skill lists
  - Markdown preview rendering (headers, bold, italic, code, hr)
  - Help overlay with all keybindings
affects: []

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Theme::detect() called once per render, passed to all rendering functions"
    - "FuzzyMatch struct carries both row_index and name_indices for highlighting"
    - "Line-by-line markdown renderer with inline delimiter scanning"

key-files:
  created:
    - crates/tome/src/browse/theme.rs
    - crates/tome/src/browse/markdown.rs
  modified:
    - crates/tome/src/browse/ui.rs
    - crates/tome/src/browse/app.rs
    - crates/tome/src/browse/fuzzy.rs
    - crates/tome/src/browse/mod.rs

key-decisions:
  - "filter_rows_with_indices matches name-only for indices to avoid off-by-one on composite haystacks"
  - "Old filter_rows kept as cfg(test) only since refilter now uses filter_rows_with_indices"
  - "match_indices stored as Vec<Vec<u32>> indexed by row_index for O(1) lookup during render"

patterns-established:
  - "Theme struct centralizes all color/style values -- no hardcoded Color:: in ui.rs"
  - "Help overlay uses Clear widget + centered Rect for popup rendering"

requirements-completed: [BROWSE-01, BROWSE-02, BROWSE-03, BROWSE-04]

# Metrics
duration: 10min
completed: 2026-04-16
---

# Phase 03 Plan 02: Browse TUI Polish Summary

**Terminal-adaptive theming, fuzzy match highlighting, scrollbar, markdown preview rendering, and help overlay for the browse TUI**

## Performance

- **Duration:** 10 min
- **Started:** 2026-04-16T06:40:11Z
- **Completed:** 2026-04-16T06:50:42Z
- **Tasks:** 3
- **Files modified:** 6

## Accomplishments
- Terminal-adaptive Theme struct with dark/light palettes detected via $COLORFGBG environment variable
- Fuzzy search highlights matched characters in skill names with bold yellow styling
- Scrollbar renders only when skill list exceeds viewport height
- Preview panel renders markdown with styled headers, bold, italic, inline code, and horizontal rules
- Help overlay (?) shows all keybindings, any keypress dismisses
- All hardcoded Color values removed from ui.rs -- everything goes through the theme

## Task Commits

Each task was committed atomically:

1. **Task 1: Create theme.rs and markdown.rs modules** - `b0fa2f3` (feat)
2. **Task 2: Extend fuzzy.rs with match indices and add Help mode** - `611aa95` (feat)
3. **Task 3: Wire theme, scrollbar, highlighting, markdown, and help into ui.rs** - `2fe3010` (feat)

## Files Created/Modified
- `crates/tome/src/browse/theme.rs` - Terminal-adaptive Theme struct with dark/light palettes
- `crates/tome/src/browse/markdown.rs` - Line-by-line markdown-to-Spans renderer for preview
- `crates/tome/src/browse/fuzzy.rs` - FuzzyMatch struct with name_indices, filter_rows_with_indices
- `crates/tome/src/browse/app.rs` - Help mode, previous_mode, match_indices field
- `crates/tome/src/browse/ui.rs` - Full theme integration, scrollbar, highlighting, markdown, help overlay
- `crates/tome/src/browse/mod.rs` - Module declarations for theme and markdown

## Decisions Made
- Fuzzy match indices extracted against name-only haystack (not composite "{name} {source}") to prevent off-by-one index mapping issues
- Old filter_rows function kept as cfg(test) since refilter() now uses filter_rows_with_indices exclusively
- match_indices stored as Vec<Vec<u32>> indexed by row_index for efficient lookup during render
- Theme fields destructive/success/code_fg kept with #[allow(dead_code)] for future CLI output integration

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] Collapsed nested if statements for clippy compliance**
- **Found during:** Task 3
- **Issue:** Clippy -D warnings rejected collapsible if statements in markdown.rs inline parser
- **Fix:** Combined nested if + if-let into single let-chain expressions
- **Files modified:** crates/tome/src/browse/markdown.rs
- **Verification:** cargo clippy -p tome -- -D warnings exits 0
- **Committed in:** 2fe3010 (Task 3 commit)

**2. [Rule 3 - Blocking] Marked filter_rows as cfg(test) to resolve dead_code warning**
- **Found during:** Task 3
- **Issue:** After switching refilter() to use filter_rows_with_indices, the old filter_rows became unused (clippy dead_code error)
- **Fix:** Added #[cfg(test)] to filter_rows since it's only used in existing tests
- **Files modified:** crates/tome/src/browse/fuzzy.rs
- **Verification:** cargo clippy clean, all tests pass
- **Committed in:** 2fe3010 (Task 3 commit)

---

**Total deviations:** 2 auto-fixed (2 blocking)
**Impact on plan:** Both fixes necessary for clippy compliance. No scope creep.

## Issues Encountered
None

## Known Stubs
None -- all features are fully wired with real data sources.

## User Setup Required
None - no external service configuration required.

## Next Phase Readiness
- Browse TUI polish complete with all BROWSE requirements met
- Ready for verification pass

---
*Phase: 03-import-reassignment-browse-polish*
*Completed: 2026-04-16*
