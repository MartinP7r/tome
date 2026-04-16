# Phase 3: Import, Reassignment & Browse Polish - Research

**Researched:** 2026-04-16
**Domain:** CLI commands (tome add, reassign, fork) + ratatui TUI polish
**Confidence:** HIGH

## Summary

Phase 3 has three distinct workstreams: (1) `tome add` -- a config-only command that writes a git directory entry to `tome.toml`, (2) `tome reassign`/`tome fork` -- commands that change skill provenance between directories, and (3) browse TUI polish -- adaptive theming, fuzzy match highlighting, scrollbar, markdown rendering, vim keybindings, and help overlay.

All three workstreams build on well-established patterns in the codebase. `tome add` follows the simple config-write pattern (no plan/render/execute needed since it is non-destructive). `tome reassign`/`tome fork` follow the plan/render/execute pattern from `remove.rs`. Browse TUI polish uses ratatui 0.30 APIs already in the dependency tree (Scrollbar widget, styled spans for highlighting) and nucleo-matcher 0.3.1's `Atom::indices()` method for match position extraction.

**Primary recommendation:** Split into three plans: (1) CLI commands (`add`, `reassign`, `fork` -- these share config/manifest modification patterns), (2) browse TUI core polish (theming, scrollbar, fuzzy highlighting, vim extras), (3) markdown rendering in preview panel (isolated change to `ui.rs` rendering).

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions
- **D-01:** `tome add <url>` accepts any git URL (not limited to GitHub). Extracts repo name from URL for the directory entry name. Override with `--name <custom-name>`.
- **D-02:** Config-only operation -- writes `[directories.<name>]` entry to `tome.toml` with `type = "git"`. Does NOT trigger a sync. User runs `tome sync` separately.
- **D-03:** Supports optional pinning flags: `--branch <ref>`, `--tag <ref>`, `--rev <sha>`. Omitting all three tracks remote HEAD.
- **D-04:** No confirmation prompt needed -- adding a config entry is non-destructive and easily undone with `tome remove`. `--dry-run` available to preview the config change.
- **D-05:** Dynamic detection of reassignment approach: skill exists in target dir -> re-link/re-consolidate; skill doesn't exist -> copy from library + update provenance.
- **D-06:** Bidirectional -- works for moving ownership toward managed sources AND away from them.
- **D-07:** `tome fork <skill> --to <local-dir>` is a user-friendly alias for the copy-to-local direction.
- **D-08:** After reassignment, local copy wins via source ordering (first-source-wins). No extra suppression needed.
- **D-09:** No confirmation for `tome reassign` (metadata-only, low risk). Confirmation required for `tome fork` (copies files). `--force` flag skips confirmation on fork.
- **D-10:** Terminal-adaptive theming -- detect dark/light mode, adapt colors automatically. ANSI 256 colors.
- **D-11:** Markdown rendering: `#` headers (bold/colored), `**bold**`, `*italic*`, `` `code spans` ``, `---` separators. Skip tables; lists stay plain text.
- **D-12:** Fuzzy match highlighting in skill name column only. Preview panel stays clean.
- **D-13:** Scrollbar appears only when skill count exceeds visible viewport area.
- **D-14:** Vim-style extras: `G` bottom, `gg` top, `Ctrl+d`/`Ctrl+u` half-page, `?` help overlay.

### Claude's Discretion
- URL parsing implementation (regex vs url crate vs manual split)
- Exact ANSI color values for terminal-adaptive themes
- Markdown parser choice (hand-rolled for the subset needed vs pulldown-cmark)
- Layout proportions and scrollbar visual style
- Help overlay design and content

### Deferred Ideas (OUT OF SCOPE)
None.
</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| CLI-02 | `tome add <github-url>` creates git directory entry in config from URL | URL parsing, Config::save pattern, DirectoryConfig construction, cli.rs subcommand addition |
| CLI-03 | `tome reassign <skill-name> --to <directory-name>` changes skill provenance | Manifest source_name modification, plan/render/execute from remove.rs, skill copy from library, fork alias |
| BROWSE-01 | Theming support (configurable color scheme) | Terminal dark/light detection via `$COLORFGBG` / ANSI query, Style struct abstraction in ui.rs |
| BROWSE-02 | Fuzzy match highlighting in skill list | nucleo-matcher `Atom::indices()` returns `Vec<u32>` char positions, render as styled Spans in ratatui |
| BROWSE-03 | Scrollbar indicator for long lists | ratatui `Scrollbar` + `ScrollbarState` widgets, conditional rendering when list exceeds viewport |
| BROWSE-04 | Markdown syntax rendering in preview panel | Hand-rolled line-by-line parser for subset (headers, bold, italic, code, hr) -- pulldown-cmark overkill for 5 patterns |
</phase_requirements>

## Standard Stack

### Core (already in Cargo.toml)
| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| ratatui | 0.30.0 | TUI framework | Already used for browse; provides Scrollbar, ScrollbarState, styled Spans |
| crossterm | 0.29.0 | Terminal event handling | Already used; provides KeyEvent for vim shortcuts |
| nucleo-matcher | 0.3.1 | Fuzzy matching | Already used; `Atom::indices()` returns match positions for highlighting |
| clap | 4 | CLI parsing | Already used; derive macros for new subcommands |
| serde/toml | 1 | Config serialization | Already used; Config::save() writes TOML |
| anyhow | 1 | Error handling | Already used throughout |
| console | 0.16 | Terminal colors for non-TUI output | Already used in remove.rs for styled output |

### No New Dependencies Needed

| Problem | Why No New Dep |
|---------|---------------|
| URL parsing for `tome add` | Simple string split on `/` to extract repo name; `url` crate (4.5MB compile) overkill for extracting a basename |
| Markdown rendering | 5 regex-like patterns (headers, bold, italic, code, hr); pulldown-cmark (0.13.3) is a full CommonMark parser -- massive overkill |
| Terminal dark/light detection | Check `$COLORFGBG` env var (common convention) or default to dark-friendly palette |

**Installation:** No new dependencies required. All needed APIs exist in current locked versions.

## Architecture Patterns

### Recommended Project Structure
```
crates/tome/src/
  add.rs           # tome add command (new)
  reassign.rs      # tome reassign + tome fork commands (new)
  browse/
    mod.rs         # entry point (existing)
    app.rs         # state + key handling (modify: add Help mode, ?-key)
    ui.rs          # rendering (modify: theming, scrollbar, highlighting, markdown)
    fuzzy.rs       # fuzzy matching (modify: return indices alongside scores)
    theme.rs       # terminal-adaptive color scheme (new)
    markdown.rs    # simple markdown-to-Spans renderer (new)
```

### Pattern 1: Config-Only Command (tome add)
**What:** Write a directory entry to config, print confirmation, exit.
**When to use:** Non-destructive config modifications.
**Example:**
```rust
// add.rs -- no plan/render/execute needed (non-destructive)
pub(crate) fn add(
    config: &mut Config,
    url: &str,
    name: Option<&str>,
    branch: Option<&str>,
    tag: Option<&str>,
    rev: Option<&str>,
    dry_run: bool,
) -> Result<()> {
    let dir_name = name
        .map(String::from)
        .unwrap_or_else(|| extract_repo_name(url));
    let dir_name = DirectoryName::new(&dir_name)?;
    
    ensure!(!config.directories.contains_key(&dir_name),
        "directory '{}' already exists in config", dir_name);
    
    let dir_config = DirectoryConfig {
        path: PathBuf::from(url),  // URL stored in path field for git type
        directory_type: DirectoryType::Git,
        role: None,                // default role inferred (Source for Git)
        branch: branch.map(String::from),
        tag: tag.map(String::from),
        rev: rev.map(String::from),
        subdir: None,
    };
    
    if !dry_run {
        config.directories.insert(dir_name.clone(), dir_config);
    }
    Ok(())
}
```

### Pattern 2: Plan/Render/Execute (tome reassign, tome fork)
**What:** Build a plan struct describing changes, render it for user review, execute with dry_run support.
**When to use:** Operations that modify filesystem or manifest state.
**Example follows remove.rs pattern:**
```rust
// reassign.rs
pub(crate) struct ReassignPlan {
    pub skill_name: SkillName,
    pub from_directory: String,
    pub to_directory: DirectoryName,
    pub action: ReassignAction,
}

pub(crate) enum ReassignAction {
    /// Skill already exists in target dir -- just update manifest provenance
    Relink,
    /// Skill doesn't exist in target -- copy from library, update manifest
    CopyAndRelink,
}
```

### Pattern 3: Theme Struct for Adaptive Colors
**What:** Centralize all color choices in a single struct, constructed based on terminal detection.
**When to use:** Replace hardcoded Color values in ui.rs.
```rust
// browse/theme.rs
pub struct Theme {
    pub header_fg: Color,
    pub selected_bg: Color,
    pub separator_fg: Color,
    pub status_bar_bg: Color,
    pub status_bar_fg: Color,
    pub group_header_fg: Color,
    pub preview_header: Style,
    pub preview_code: Style,
    pub preview_bold: Style,
    pub match_highlight: Style,
}

impl Theme {
    pub fn detect() -> Self {
        if is_light_terminal() {
            Self::light()
        } else {
            Self::dark()  // safe default
        }
    }
}

fn is_light_terminal() -> bool {
    // $COLORFGBG convention: "fg;bg" where bg >= 8 often means light
    std::env::var("COLORFGBG").ok()
        .and_then(|v| v.rsplit(';').next().map(String::from))
        .and_then(|bg| bg.parse::<u8>().ok())
        .is_some_and(|bg| bg >= 8 && bg != 8) // 8 is dark gray
}
```

### Pattern 4: Fuzzy Match Index Extraction
**What:** Extend `filter_rows` to also return match character indices for highlighting.
**When to use:** BROWSE-02 fuzzy highlighting.
```rust
// browse/fuzzy.rs -- extend to return indices
pub struct FuzzyMatch {
    pub row_index: usize,
    pub score: u16,
    pub name_indices: Vec<u32>,  // char positions in the name string that matched
}

pub fn filter_rows_with_indices(query: &str, rows: &[SkillRow]) -> Vec<FuzzyMatch> {
    // Use Atom::indices() instead of Atom::score()
    let mut indices_buf: Vec<u32> = Vec::new();
    // ... for each row:
    //   indices_buf.clear();
    //   let score = pattern.indices(haystack_utf32, &mut matcher, &mut indices_buf)?;
    //   Extract only indices that fall within the name portion (before the space separator)
}
```

### Anti-Patterns to Avoid
- **Hardcoded colors in render functions:** All colors must go through the Theme struct so adaptive theming works everywhere.
- **Full markdown parser dependency:** pulldown-cmark is overkill -- the subset (5 patterns) is trivially handled with line-by-line processing.
- **Modifying manifest directly without going through plan:** Reassign must use plan/render/execute to maintain the established pattern.
- **`url` crate for simple basename extraction:** A URL like `https://github.com/user/repo.git` just needs the last path segment with `.git` stripped.

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Fuzzy matching | Custom fuzzy matcher | nucleo-matcher (already integrated) | Unicode normalization, smart casing, scoring |
| Scrollbar rendering | Custom scrollbar with manual thumb positioning | ratatui `Scrollbar` + `ScrollbarState` | Handles edge cases (tiny lists, exact fit, rounding) |
| Terminal event handling | Raw stdin reading | crossterm `event::read()` (already used) | Cross-platform, modifier key detection |

**Key insight:** This phase adds zero new dependencies. Every capability needed exists in the already-locked crate versions.

## Common Pitfalls

### Pitfall 1: URL Repo Name Extraction Edge Cases
**What goes wrong:** URLs like `https://github.com/user/repo.git`, `git@github.com:user/repo`, `https://github.com/user/repo/` (trailing slash) produce different basename results.
**Why it happens:** Git URLs have multiple formats (HTTPS, SSH, trailing `.git`, trailing `/`).
**How to avoid:** Strip trailing `/`, strip trailing `.git`, then take last path segment. Test all three formats.
**Warning signs:** `tome add` creates directory names like "repo.git" or empty names.

### Pitfall 2: Fuzzy Indices Off-by-One with Composite Haystack
**What goes wrong:** `filter_rows` currently concatenates `"{name} {source}"` as the haystack. Indices returned by nucleo are into this composite string. If you use these indices to highlight the name column, indices that fall in the source portion would be out of bounds.
**Why it happens:** nucleo doesn't know about the name/source boundary.
**How to avoid:** Match against just the name string for highlighting purposes (separate from the full-string scoring), OR filter indices to only those < name.len().
**Warning signs:** Highlight renders show garbled characters or panic on index bounds.

### Pitfall 3: Scrollbar State Requires content_length
**What goes wrong:** Scrollbar renders blank if `ScrollbarState::content_length()` is not set.
**Why it happens:** ratatui Scrollbar needs to know total content size to compute thumb position.
**How to avoid:** Always set `content_length(total_items)` and `position(scroll_offset)` on the ScrollbarState before rendering.
**Warning signs:** Scrollbar track appears but thumb is invisible.

### Pitfall 4: Reassign to Non-Existent Directory
**What goes wrong:** `tome reassign skill-x --to nonexistent-dir` crashes or silently fails.
**Why it happens:** Target directory name doesn't exist in config.
**How to avoid:** Validate `--to` directory exists in `config.directories` early in the plan step. Produce a clear error: "directory 'nonexistent-dir' not found in config".
**Warning signs:** Opaque error messages about missing paths.

### Pitfall 5: g vs gg Vim Keybinding Collision
**What goes wrong:** Existing code has `KeyCode::Char('g')` mapped to `jump_to_top()`. The `gg` vim idiom requires two sequential `g` presses. A single `g` should not jump immediately.
**Why it happens:** The current implementation already maps single `g` to top (see app.rs line 125). This is actually non-standard vim -- vim uses `gg` (two presses).
**How to avoid:** The current behavior (single `g` = top) is already implemented and works. D-14 says "gg for top" but since the existing code already handles this with single `g`, maintain current behavior. True `gg` detection would need a timeout state machine, which adds complexity. Since `G` (shift) = bottom is already implemented too, and `Ctrl+d`/`Ctrl+u` are already implemented, the only new vim binding needed is `?` for help overlay.
**Warning signs:** Users type `g` and nothing happens (if you implement timeout-based gg).

### Pitfall 6: Theme Detection Reliability
**What goes wrong:** `$COLORFGBG` is not universally set. Many terminal emulators don't set it.
**Why it happens:** There's no universal standard for terminal background color detection.
**How to avoid:** Default to dark-friendly palette (most developer terminals are dark). Only switch to light when confident detection succeeds. ANSI 256 colors that work reasonably in both modes are ideal.
**Warning signs:** Unreadable text on light terminals.

## Code Examples

### URL Repo Name Extraction
```rust
/// Extract a directory name from a git URL.
///
/// Strips trailing `/`, strips `.git` suffix, returns last path segment.
/// Falls back to the full URL if no path segments found.
fn extract_repo_name(url: &str) -> String {
    let trimmed = url.trim_end_matches('/');
    // Handle SSH URLs like git@github.com:user/repo.git
    let path_part = trimmed
        .rsplit_once(':')
        .filter(|(prefix, _)| !prefix.contains('/')) // SSH-style, not https://
        .map(|(_, path)| path)
        .unwrap_or(trimmed);
    path_part
        .rsplit('/')
        .next()
        .unwrap_or(trimmed)
        .trim_end_matches(".git")
        .to_string()
}
```

### Scrollbar Integration
```rust
use ratatui::widgets::{Scrollbar, ScrollbarOrientation, ScrollbarState};

// In render_normal(), after rendering the body table:
let total_items = app.filtered_indices.len();
let visible = app.visible_height;
if total_items > visible {
    let mut scrollbar_state = ScrollbarState::new(total_items)
        .position(app.scroll_offset);
    let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
        .begin_symbol(None)
        .end_symbol(None);
    // Render in the left body area with inner margin
    frame.render_stateful_widget(
        scrollbar,
        body_chunks[0].inner(Margin { vertical: 0, horizontal: 0 }),
        &mut scrollbar_state,
    );
}
```

### Fuzzy Highlight Rendering
```rust
use ratatui::text::{Line, Span};

fn highlight_name<'a>(name: &'a str, indices: &[u32], theme: &Theme) -> Line<'a> {
    if indices.is_empty() {
        return Line::from(name);
    }
    let index_set: std::collections::HashSet<u32> = indices.iter().copied().collect();
    let spans: Vec<Span> = name.char_indices().map(|(byte_pos, ch)| {
        // Convert byte index to char index for comparison
        let char_idx = name[..byte_pos].chars().count() as u32;
        if index_set.contains(&char_idx) {
            Span::styled(ch.to_string(), theme.match_highlight)
        } else {
            Span::raw(ch.to_string())
        }
    }).collect();
    Line::from(spans)
}
```

### Simple Markdown Renderer
```rust
fn render_markdown(raw: &str, theme: &Theme) -> Vec<Line<'_>> {
    raw.lines().map(|line| {
        if let Some(rest) = line.strip_prefix("# ") {
            Line::from(Span::styled(rest, theme.preview_header))
        } else if let Some(rest) = line.strip_prefix("## ") {
            Line::from(Span::styled(rest, theme.preview_header))
        } else if line.starts_with("---") {
            Line::from(Span::styled("─".repeat(40), Style::default().fg(Color::DarkGray)))
        } else {
            render_inline_markdown(line, theme)
        }
    }).collect()
}

fn render_inline_markdown<'a>(line: &'a str, theme: &Theme) -> Line<'a> {
    // Parse **bold**, *italic*, `code` spans within a line
    // Use a simple state machine scanning for delimiter pairs
    // Return Vec<Span> with appropriate styles
}
```

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| ratatui Scrollbar in experimental | Scrollbar stable in ratatui 0.30 | ratatui 0.29+ | Can use directly, no feature flags |
| nucleo score-only API | nucleo `Atom::indices()` for match positions | Always available in 0.3.x | Enables highlighting without a second pass |
| Hardcoded terminal colors | Theme struct pattern | Common ratatui community pattern | Enables adaptive theming |

## Open Questions

1. **`gg` vs single `g` for jump-to-top**
   - What we know: Current code maps single `g` to top. D-14 says "gg for top".
   - What's unclear: Whether to implement true two-key `gg` detection (needs timeout/state) or keep current single-`g` behavior.
   - Recommendation: Keep single `g` = top (already works, simpler). The spirit of D-14 is already satisfied. True `gg` detection adds timeout complexity for minimal UX gain with a single user.

2. **Reassign manifest update atomicity**
   - What we know: Manifest uses `insert()` and `remove()` individually.
   - What's unclear: Whether reassign should be atomic (update source_name in place) or remove+insert.
   - Recommendation: Modify `source_name` field in place via a new `Manifest::update_source()` method. This preserves the hash and synced_at metadata.

## Sources

### Primary (HIGH confidence)
- Codebase analysis: `browse/app.rs`, `browse/ui.rs`, `browse/fuzzy.rs` -- current TUI implementation
- Codebase analysis: `remove.rs` -- plan/render/execute pattern
- Codebase analysis: `config.rs` -- DirectoryConfig, Config::save()
- Codebase analysis: `git.rs` -- repo_cache_dir(), URL handling
- nucleo-matcher 0.3.1 source: `pattern.rs` -- `Atom::indices()` API confirmed
- ratatui-widgets 0.3.0 source: `scrollbar.rs` -- `Scrollbar` + `ScrollbarState` confirmed

### Secondary (MEDIUM confidence)
- `$COLORFGBG` convention for terminal background detection (widely used but not universal)

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH - all dependencies already locked, APIs verified in source
- Architecture: HIGH - follows established codebase patterns (remove.rs, browse/)
- Pitfalls: HIGH - derived from actual code analysis (fuzzy index boundaries, vim keybinding collision, scrollbar state requirements)

**Research date:** 2026-04-16
**Valid until:** 2026-05-16 (stable -- no fast-moving dependencies)
