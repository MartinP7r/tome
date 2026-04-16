# Phase 3: Import, Reassignment & Browse Polish - Context

**Gathered:** 2026-04-16
**Status:** Ready for planning

<domain>
## Phase Boundary

Users can quickly add git skill repos via `tome add <url>`, reassign skill provenance between directories (including forking managed skills to local for customization), and enjoy a polished browse TUI with adaptive theming, fuzzy match highlighting, scrollbar indicators, vim-style navigation, and markdown rendering in the preview panel.

This phase does NOT include: new discovery strategies, config format changes, wizard enhancements, or new sync pipeline stages.

</domain>

<decisions>
## Implementation Decisions

### tome add
- **D-01:** `tome add <url>` accepts any git URL (not limited to GitHub). Extracts repo name from URL for the directory entry name. Override with `--name <custom-name>`.
- **D-02:** Config-only operation — writes `[directories.<name>]` entry to `tome.toml` with `type = "git"`. Does NOT trigger a sync. User runs `tome sync` separately.
- **D-03:** Supports optional pinning flags: `--branch <ref>`, `--tag <ref>`, `--rev <sha>`. Omitting all three tracks remote HEAD (consistent with Phase 2 D-03).
- **D-04:** No confirmation prompt needed — adding a config entry is non-destructive and easily undone with `tome remove`. `--dry-run` available to preview the config change.

### tome reassign
- **D-05:** Dynamic detection of reassignment approach:
  - If the skill already exists in the target directory → re-link/re-consolidate from there (clean transition)
  - If the skill doesn't exist in the target → copy skill files from library into target directory, then update provenance
- **D-06:** Bidirectional — works for moving ownership toward managed sources (consolidation) AND away from them (customization/forking).
- **D-07:** `tome fork <skill> --to <local-dir>` is a user-friendly alias for the copy-to-local direction. Same mechanics as `tome reassign` but clearer intent for the customization use case.
- **D-08:** After reassignment, local copy wins via source ordering (first-source-wins). No extra suppression of the original managed version needed. Next sync respects the reassignment — discovery does not overwrite manual reassignment entries in the manifest.
- **D-09:** No confirmation for `tome reassign` (metadata-only, low risk). Confirmation required for `tome fork` (copies files). `--force` flag skips confirmation on fork.

### Browse TUI Polish
- **D-10:** Terminal-adaptive theming — detect terminal dark/light mode and adapt colors automatically. No user configuration needed. Uses ANSI 256 colors that look decent in both modes.
- **D-11:** Markdown rendering in preview panel: render `#` headers (bold/colored), `**bold**`, `*italic*`, `` `code spans` ``, and `---` separators. Skip tables; lists stay plain text. Covers 90% of SKILL.md content.
- **D-12:** Fuzzy match highlighting in the skill name column only (list view). Preview panel stays clean with markdown rendering. No highlighting in preview.
- **D-13:** Scrollbar appears only when skill count exceeds visible viewport area. Hidden when list fits entirely.
- **D-14:** Add vim-style keyboard extras: `G` for bottom, `gg` for top, `Ctrl+d`/`Ctrl+u` for half-page scroll, `?` for help overlay showing all keybindings.

### Claude's Discretion
- URL parsing implementation (regex vs url crate vs manual split)
- Exact ANSI color values for terminal-adaptive themes
- Markdown parser choice (hand-rolled for the subset needed vs pulldown-cmark)
- Layout proportions and scrollbar visual style
- Help overlay design and content

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### Design & Architecture
- `docs/v06-implementation-plan.md` — Original v0.6 design. Type definitions and CLI command specs.
- `docs/src/architecture.md` — Sync pipeline flow (discover -> consolidate -> distribute -> cleanup).
- `docs/src/commands.md` — Existing CLI command reference.

### Requirements
- `.planning/REQUIREMENTS.md` — CLI-02 (tome add), CLI-03 (tome reassign), BROWSE-01 through BROWSE-04.
- `.planning/ROADMAP.md` — Phase 3 success criteria.

### Prior Phase Context
- `.planning/phases/01-unified-directory-foundation/01-CONTEXT.md` — Phase 1 decisions (D-01 through D-11).
- `.planning/phases/02-git-sources-selection/02-CONTEXT.md` — Phase 2 decisions. Especially D-01 (URL hashing), D-12-D-14 (remove UX pattern).

### Key Source Files
- `crates/tome/src/git.rs` — `clone_repo`, `repo_cache_dir`, `update_repo` — reusable for `tome add` validation.
- `crates/tome/src/remove.rs` — Plan/render/execute pattern to follow for `add`, `reassign`, `fork`.
- `crates/tome/src/config.rs` — `DirectoryConfig`, `DirectoryType::Git`, `Config::save()`.
- `crates/tome/src/manifest.rs` — `source_name` field that reassign modifies.
- `crates/tome/src/browse/` — Existing browse TUI (app.rs, ui.rs, fuzzy.rs, mod.rs). ~1200 lines.
- `crates/tome/src/browse/app.rs` — `Mode` enum (Normal, Search, Detail), `SortMode`, key handling.
- `crates/tome/src/browse/ui.rs` — `render()` function, hardcoded colors to replace with adaptive theming.
- `crates/tome/src/browse/fuzzy.rs` — nucleo-matcher integration, currently returns match scores but no character indices for highlighting.
- `crates/tome/src/cli.rs` — Subcommand definitions. Needs `Add`, `Reassign`, `Fork` added.

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets
- `git.rs::repo_cache_dir(url)` — computes SHA-256 cache dir for a URL. Reusable for `tome add` to validate the URL hashing.
- `remove.rs` plan/render/execute pattern — established in Phase 2 for destructive commands. Fork should follow the same pattern.
- `nucleo-matcher` — already integrated for fuzzy matching. Supports returning match indices for highlighting (currently only scores are used).
- `ratatui::widgets::Scrollbar` — available in ratatui 0.30, not yet used.

### Established Patterns
- Destructive commands use plan/render/execute with `--dry-run` and `--force` (remove.rs).
- Config modifications use `Config::save(&paths.config_path())` for atomic writes.
- Manifest modifications use `manifest::save(&manifest, paths.config_dir())`.
- All git subprocess calls clear `GIT_DIR`, `GIT_WORK_TREE`, `GIT_INDEX_FILE` env vars (git.rs pattern).

### Integration Points
- `cli.rs` — New `Add`, `Reassign`, `Fork` subcommand variants in `Command` enum.
- `lib.rs::run()` — Command dispatch for new subcommands.
- `browse/ui.rs::render()` — Entry point for all TUI rendering changes.
- `browse/app.rs` — Key event handling for new vim shortcuts.

</code_context>

<specifics>
## Specific Ideas

- `tome fork` as a distinct command (alias for reassign-to-local) with its own help text emphasizing the "customize a managed skill" workflow
- Dynamic reassign detection makes the command intuitive regardless of whether the skill has already been moved or not

</specifics>

<deferred>
## Deferred Ideas

None — discussion stayed within phase scope.

</deferred>

---

*Phase: 03-import-reassignment-browse-polish*
*Context gathered: 2026-04-16*
