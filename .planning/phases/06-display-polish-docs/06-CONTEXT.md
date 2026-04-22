# Phase 6: Display Polish & Docs - Context

**Gathered:** 2026-04-21
**Status:** Ready for planning

<domain>
## Phase Boundary

Close the v0.7 milestone with two "milestone-close" items that share a housekeeping character:

1. **WHARD-07** — Migrate `wizard::show_directory_summary()` (`wizard.rs:413-436`) from manual `println!` column formatting to the `tabled` crate. Long paths (especially `~/.tome/repos/<sha256>/...` from git sources) must render without breaking column alignment — truncate with ellipsis when the table would exceed terminal width, never overflow.
2. **WHARD-08** — Update `PROJECT.md` so WIZ-01 through WIZ-05 are explicitly marked as validated with a note that they shipped in v0.6 and were hardened in v0.7 (Phases 4+5). Remove the now-outdated "Known Gaps (deferred from v0.6)" entry that still describes them as deferred.

Out of scope for this phase:
- Upgrading `status.rs` to `Style::rounded()` for cross-command consistency (deferred — separate polish pass).
- Registry expansion for new tools (Cursor, Windsurf, Aider — v2 requirements WREG-01/02/03).
- Any further wizard rewrite (WIZ-01–05 themselves are closed; a ground-up rewrite is not on the roadmap).
- Snapshot tests for the wizard summary — substring-level assertions are sufficient for this polish pass (follows Phase 5 D-09 precedent for combo-matrix assertions).

</domain>

<decisions>
## Implementation Decisions

### WHARD-07: Tabled Migration

**Table style**

- **D-01:** Use `tabled::settings::Style::rounded()` for the wizard summary. This diverges intentionally from `status.rs`, which uses `Style::blank()`. Rationale: `tome init` is a one-shot ceremonial summary where a "finished" bordered look is appropriate; `tome status` is a repeated-inspection view where borderless minimalism scans better. The ROADMAP Phase 6 criterion #1 literal wording ("with `Style::rounded()`") takes precedence over the "matching visual language" phrase in the same criterion — the two-different-contexts argument explains why the divergence is intentional rather than an oversight.

**Columns**

- **D-02:** Column set and ordering: `NAME / TYPE / ROLE / PATH`. Matches the ordering in `status.rs` (minus `SKILLS`, which doesn't apply at wizard time because no sync has run yet). Consistent mental model between `tome init` and `tome status`.
- **D-03:** Header row styling matches `status.rs`: apply `Modify::new(Rows::first()).with(Format::content(|s| style(s).bold().to_string()))` to bold the header row, rather than per-column inline `style().bold()` on each literal. Keeps the code shape parallel to `status.rs`'s `render_status` (see `status.rs:185-191`).

**Long path handling**

- **D-04:** Detect terminal width via the `terminal_size` crate (new dependency) and apply `Width::truncate(term_cols).priority(PriorityMax)` to the whole table. `PriorityMax` shrinks the widest column first — in practice the `PATH` column when git repo paths are present, otherwise potentially `ROLE` (which contains the plain-english parenthetical). Truncated text gets an ellipsis (`…`) suffix per tabled default.
- **D-05:** Fallback when terminal width cannot be detected (piped output, non-TTY, or CI): assume 80 columns. This matches common CLI conventions (git, cargo) and keeps output deterministic in non-interactive tests. The `--dry-run` branch (`wizard.rs:306-322`) that prints the same summary after `Generated config:` uses the same 80-col fallback since that output is typically captured rather than interactively viewed.
- **D-06:** Existing `paths::collapse_home()` is applied to `PATH` cell values before width calculation. Matches `status.rs:181`. This keeps paths starting with `~/` (shorter than `/Users/martin/`) and usually avoids needing truncation at all.

**Empty state**

- **D-07:** The current `"(no directories configured)"` message on the empty-directories branch is kept verbatim. No tabled rendering when the map is empty — just the plain message. The `tabled` rendering path is guarded by `if directories.is_empty()` exactly as it is today.

### WHARD-08: PROJECT.md Update

- **D-08:** Add a new dedicated subsection under `## Requirements` (or equivalent location) titled `### Hardened in v0.7` (or similar) that lists WIZ-01 through WIZ-05 as individual bullets, each with:
  - The WIZ-XX label
  - A one-line description of what the item covered (e.g., "WIZ-01: Merged `KNOWN_DIRECTORIES` registry")
  - The "Shipped v0.6, hardened v0.7 (Phases 4+5)" note, either per-bullet or as a subsection header
- **D-09:** Remove the existing `### Known Gaps (deferred from v0.6)` subsection (or at least the WIZ-01–05 entry within it). This bullet is now factually wrong — WIZ-01–05 are not low priority anymore, they've been closed.
- **D-10:** Do not modify the existing `### Previously Validated (re-verified in v0.7 research)` subsection — those entries already stand as shipped. Adding WIZ-XX labels retroactively would clutter without clarifying. The new "Hardened in v0.7" section is the explicit-traceability surface.
- **D-11:** The "last updated" footer at the bottom of PROJECT.md gets a new entry noting Phase 6 completion and the WIZ-01–05 closure.

### Claude's Discretion

- Exact title of the new PROJECT.md subsection (`### Hardened in v0.7`, `### Wizard Hardening Closure`, `### v0.7 Wizard Closure (WIZ-01–05)` — all acceptable).
- Whether WIZ-XX bullets sit directly under a new header or are nested under an existing structure like `### Validated in v0.7`.
- Specific one-line descriptions for each WIZ-XX — the descriptions should match what actually shipped in v0.6 and what Phases 4+5 hardened.
- Whether to also add a short CHANGELOG note (new subsection or appending to existing entries) for the tabled migration and doc cleanup — WHARD-08 doesn't literally require a CHANGELOG edit but a release pass usually includes one.
- Whether `Style::rounded()` is imported via `use tabled::settings::Style;` (adding `rounded` alongside `blank` — but each file has its own use statement) or qualified inline.
- Whether the `terminal_size` crate is added as a direct dependency or used transitively through another dep that re-exports it (check if `dialoguer` or `console` already pulls it in — both are already direct deps).

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### Requirements & Roadmap

- `.planning/REQUIREMENTS.md` — WHARD-07 and WHARD-08 definitions and Phase 6 traceability.
- `.planning/ROADMAP.md` §"Phase 6: Display Polish & Docs" — three success criteria that must be TRUE after this phase. Note the contradiction between "matching the visual language of `tome status`" and "with `Style::rounded()`" in criterion #1, resolved in favor of rounded per D-01.
- `.planning/PROJECT.md` — File under edit for WHARD-08. Especially:
  - `## Requirements` → `### Active`, `### Validated in v0.7`, `### Previously Validated (re-verified in v0.7 research)`
  - `### Known Gaps (deferred from v0.6)` — contains outdated WIZ-01–05 bullet to remove per D-09

### Prior Phase Context (decisions carried forward)

- `.planning/phases/01-unified-directory-foundation/01-CONTEXT.md` — Phase 1. D-05/D-06 (plain-english role parenthetical via `DirectoryRole::description()`). Directly relevant because ROLE cells render via `description()` (longest cell content; target for potential truncation).
- `.planning/phases/04-wizard-correctness/04-CONTEXT.md` — Phase 4. D-08 (hard error + exit on validation failure — not relevant to this phase directly, but confirms wizard save path structure stays the same). D-11 (plain-english role parenthetical in error messages — parallels the same parenthetical in ROLE column rendering).
- `.planning/phases/05-wizard-test-coverage/05-CONTEXT.md` — Phase 5. D-06 (no `WizardInputs` struct refactor — same frame applies: this phase should not expand scope into structural refactors). D-09 (substring matching > snapshots for polish-layer assertions). Crate-boundary visibility rule re `pub(crate)` vs `pub` accessors is reference material if new tests are added.

### Key Source Files

- `crates/tome/src/wizard.rs:413-436` — `show_directory_summary()`. **Primary site for WHARD-07.** Full replacement with tabled-based rendering.
- `crates/tome/src/wizard.rs:181-183`, `231-233`, `297-299` — Three call sites of `show_directory_summary()`. No change needed; they continue to call the function. `wizard.rs:306-322` — `--dry-run` branch prints summary; same function is called.
- `crates/tome/src/status.rs:185-193` — **Reference implementation** for tabled usage pattern. Structure of `Table::from_iter(rows).with(Style::…).with(Modify::new(Rows::first()).with(Format::content(…)))` is the template. Only difference is `Style::rounded()` instead of `Style::blank()` and different column set.
- `crates/tome/src/status.rs:6` — `use tabled::settings::{Modify, Style, object::Rows};` — imports to mirror (+ `Width`, `PriorityMax` for truncation).
- `crates/tome/src/paths.rs::collapse_home()` — used for PATH cell rendering per D-06.
- `crates/tome/src/config.rs:142-186` — `DirectoryRole::description()` — provider of the plain-english ROLE cell content.
- `Cargo.toml:24` — `tabled = "0.20"` (already present). New dependency to add: `terminal_size` (latest major, likely 0.4.x).

### Documentation Files

- `PROJECT.md` (root of repo) — no changes (user-facing project overview, not the planning file).
- `.planning/PROJECT.md` — file under edit for WHARD-08 per D-08/D-09/D-11.
- `CHANGELOG.md` — optional touch per Claude's discretion; would include WHARD-07 (wizard summary tabled migration) and WHARD-08 (docs update) under the v0.7 unreleased section.

### Test Coverage Expectations

- No new unit tests strictly required for WHARD-07 — tabled is a third-party crate producing strings; the logic being added is mostly wiring. A single sanity test that `show_directory_summary(&empty_map)` and `show_directory_summary(&one_entry_map)` don't panic is worthwhile but not formally required by the phase criteria. Skip if it adds noise.
- WHARD-08 is a doc edit; test is "the markdown parses and grep finds WIZ-01 through WIZ-05 in the Validated section" — informal CI-grep level, not a Rust test.
- The existing Phase 5 integration test at `tests/cli.rs` (if it parses the wizard's Generated config: stdout) may need minor fixture updates because the summary block between the banner and the TOML will now be a tabled table. Check that the parser splits on a stable marker (`Generated config:`) rather than counting lines.

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets

- **`tabled` Table construction pattern** (`status.rs:185-193`) — ready-made template. Swap `Style::blank()` → `Style::rounded()`, drop `SKILLS` column, done. About 15-20 lines of code.
- **`paths::collapse_home()`** — converts `/Users/martin/...` → `~/...` for shorter PATH cells. Already used in `status.rs:181`. Reuse verbatim.
- **`DirectoryRole::description()`** — produces the full plain-english parenthetical. Unchanged; just rendered in a cell now.
- **`DirectoryType: Display`** — prints lowercase-hyphen form (`claude-plugins`, `git`). Already used at `wizard.rs:431`.
- **`console::style()`** — already imported in `wizard.rs`. Continue using for bold formatting in the header row via tabled's `Format::content` closure.

### Established Patterns

- Tabled imports grouped at top of file: `use tabled::settings::{Modify, Style, object::Rows};` (`status.rs:6`). Add `Width` and `peaker::PriorityMax` (the exact path depends on tabled 0.20 — verify via `cargo doc --open` or the tabled source).
- Header bolding via `Modify::new(Rows::first()).with(Format::content(|s| style(s).bold().to_string()))` — one pattern; don't diverge.
- Empty-state guard: `if directories.is_empty() { println!("  (no directories configured)"); return; }` — pattern reused verbatim per D-07.
- Cargo.toml alphabetical ordering of `[dependencies]` — when adding `terminal_size`, insert in alphabetical order (between `tempfile` and `tome`-adjacent entries; check before inserting).

### Integration Points

- Call sites of `show_directory_summary()`: three `show_directory_summary(&directories)` invocations in `wizard.rs` (lines 183, 233, 299). Signature unchanged; callers don't care about the implementation swap.
- The `--dry-run` branch at `wizard.rs:306-322` calls `show_directory_summary()` right before the `Generated config:` marker and TOML body. Phase 5's integration test splits stdout on `Generated config:`, so the new tabled-rendered summary sits *before* that marker — should not break parsing.
- `Cargo.toml` root (workspace manifest) is where `terminal_size = "…"` lands. The crate `crates/tome/Cargo.toml` already lists `tabled = { workspace = true }`-style dep resolution (check current shape before writing).

### Blast Radius

- Code changes: `wizard.rs::show_directory_summary` body only (20-ish LoC replaced). Possibly one import line at top of `wizard.rs`. Optional one CHANGELOG line.
- Dependency: add `terminal_size` to root `Cargo.toml` `[dependencies]` section. Cargo.lock updated.
- Docs: `.planning/PROJECT.md` edited for D-08/D-09/D-11.
- No changes to: `config.rs`, `lib.rs`, `status.rs`, `discover.rs`, `library.rs`, `distribute.rs`, TUI (`browse/`), tests in `tests/cli.rs` (other than possible fixture update if parsing is fragile).

</code_context>

<specifics>
## Specific Ideas

- The `Style::rounded()` choice is a *deliberate aesthetic divergence* from `status.rs`. If a future reader asks "why do init and status look different?", the answer is in D-01: init is ceremonial, status is repetitive. Document inline if useful.
- `terminal_size::terminal_size()` returns `Option<(Width, Height)>`. Unwrap via `.map(|(w, _)| w.0 as usize).unwrap_or(80)` — no fancy fallback logic needed.
- `tabled::settings::peaker::PriorityMax` (or `tabled::settings::width::PriorityMax` depending on the 0.20 API surface) is the priority strategy. Verify API path before writing — don't guess.
- The ROLE column contains `DirectoryRole::description()` output, which for Synced is: `"Synced (skills discovered here AND distributed here)"` — 49 chars. In combination with 80-col terminal width minus 3 other columns + 5 border chars, the ROLE column alone can blow the budget. Truncation may need to prefer PATH over ROLE first (PATH is usually shorter once `collapse_home()` runs), which is why `PriorityMax` (shrink widest) is correct — it dynamically picks.
- WHARD-08 text should mention that v0.7 hardening specifically added: (a) `Config::validate()` path-overlap checks (Cases A/B/C), (b) `Config::save_checked` with TOML round-trip, (c) `--no-input` plumbing, (d) unit + integration test coverage for pure wizard helpers, (e) the 12-combo cross-product validation test. That's the "hardened" deliverable.
- CHANGELOG.md if touched: one bullet per HARD-XX under the v0.7 unreleased section. Short prose: "Migrated wizard summary table to `tabled` with rounded borders and terminal-width-aware truncation" + "Marked WIZ-01–05 closed in PROJECT.md".

</specifics>

<deferred>
## Deferred Ideas

- **Upgrade `status.rs` to `Style::rounded()` for visual parity** — Out of scope; would change every `tome status` invocation and needs its own discussion. Revisit post-v0.7 if the visual divergence feels wrong in practice.
- **Snapshot tests for `show_directory_summary` output** — Would lock in the exact bytes including terminal-width-dependent truncation points. High maintenance churn (breaks on terminal-size changes, `Style::rounded()` variant bumps). Skip per Phase 5 D-09 precedent.
- **Registry expansion (Cursor / Windsurf / Aider)** — WREG-01/02/03 (v2 requirements). Separate phase in a future milestone.
- **`tome init --no-color` respecting `NO_COLOR` env var for the summary** — Tabled's output is colored via the header bolding closure. If `NO_COLOR=1`, `console::style()` is a no-op (already handled). No extra work needed — but worth verifying once.
- **Reworking the "Active" / "Validated" / "Previously Validated" taxonomy in PROJECT.md** — the file has three overlapping validation sections and could be tightened. Not Phase 6's concern; WHARD-08 only adds one focused subsection.
- **Env var / flag to override terminal width** — e.g., `TOME_TABLE_WIDTH=120` for wide-terminal users or testing. Nice but not required. Skip.

</deferred>

---

*Phase: 06-display-polish-docs*
*Context gathered: 2026-04-21*
