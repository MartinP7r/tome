---
phase: 16-cleanup-message-ux-docs
plan: 05
subsystem: documentation
tags: [docs, mdbook, cross-machine, library-as-dotfiles, doc-discoverability, sync-help, cli-clap]

# Dependency graph
requires:
  - phase: 11-library-canonical-core
    provides: tome migrate-library one-shot CLI (LIB-05) + drift basis = content_hash (D-08) — both surfaced in the new page reference sections
  - phase: 13-lockfile-authoritative-sync
    provides: tome.lock semantics + AutoInstall { Always | Ask | Never } enum + --no-install global flag (RECON-01..05) — surfaced verbatim in the page consent + lockfile sections
  - phase: 14-unowned-library-lifecycle
    provides: D-API-1/-2 vocab merge (`tome reassign --to`, `tome remove skill`; NO `tome adopt`/`tome forget`) — honoured throughout the new page
  - phase: 16-02 (this phase, wave 1)
    provides: tome migrate-library confirm gate + --yes / -y bypass + --no-input bail wording (UX-02) — surfaced in the v0.9 library migration reference section
  - phase: 16-03 (this phase, wave 1)
    provides: docs/src/architecture.md v0.10 framing with H2 anchors (Library-canonical model + Lockfile-authoritative reconciliation) — cross-linked from the new page
  - phase: 16-04 (this phase, wave 1)
    provides: CHANGELOG.md v0.10 release notes — cross-linked from the new page's migration reference section
provides:
  - new docs/src/cross-machine-sync.md page (259 lines) covering library-as-dotfiles workflow end-to-end
  - mdbook TOC entry between Configuration and Development Workflow
  - Command::Sync long_about attribute referencing the new page so `tome sync --help` advertises it
  - in-prose link from architecture.md Library-canonical model section → cross-machine-sync.md
affects: [Phase 17 release readiness — DOC-03 closes the v0.10 documentation surface; rc cut now unblocked]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "doc-as-walkthrough — page leads with two numbered Machine A / Machine B walkthroughs before the reference sections (D-DOC03-2). Reader can skim the walks and dive into reference when something surprises them, instead of hitting a wall of schema docs first."
    - "cross-link rather than duplicate — page links to configuration.md (directory_overrides schema, machine preferences) and architecture.md (Library-canonical model, Lockfile-authoritative reconciliation) for full schema docs, keeping the new page focused on workflow narrative."
    - "two-direction discoverability — TOC entry in SUMMARY.md AND clap long_about attribute on Command::Sync. Users browsing the docs find the page; users running `tome sync --help` find the page. Belt-and-braces."

key-files:
  created:
    - docs/src/cross-machine-sync.md
  modified:
    - docs/src/SUMMARY.md (TOC entry inserted between Configuration and Development Workflow)
    - crates/tome/src/cli.rs (Command::Sync gains long_about attribute referencing the page)
    - docs/src/architecture.md (cross-link from Library-canonical model bullet to the new page)

key-decisions:
  - "Page placement honours D-DOC03-1 reading-order intent. SUMMARY.md entry sits between Configuration and Development Workflow (line 7) so reading order is Commands → Configuration → Cross-machine sync → Development Workflow → Architecture. The new page slots adjacent to the topics it builds on (configuration schema, command set) without requiring reorder of other entries."
  - "AutoInstall variant names corrected to `Always | Ask | Never` (the actual enum in machine.rs:29-33) rather than the `Yes | Never | Prompt` reference in CONTEXT.md DOC-03 D-DOC03-2. Plan 16-03 caught the same CONTEXT.md error and propagated the correct names into architecture.md; this plan does the same for cross-machine-sync.md. The CONTEXT.md error itself was not patched (out of scope; documented here for future awareness)."
  - "Auto-install consent prompt rendered as the actual Select-dialog labels (`Yes (always — install on every sync)` / `Yes (ask me again next time)` / `No (never ask again on this machine)`) rather than the literal `[Y/n/never]` shorthand from Phase 13 D-08. The shorthand exists only in design context; the shipped UX is a `dialoguer::Select` arrow-key list per reconcile.rs::prompt_consent."
  - "Missing-claude error message reproduced verbatim from marketplace.rs::ClaudeMarketplaceAdapter::new (line 612): `claude CLI not found on PATH — install Claude Code, or remove [directories.<name>] entries with type = \"claude-plugins\" from tome.toml`. Source-of-truth for the page is the actual code, not the planner's prose sketch."
  - "Long-about link path uses the relative `docs/src/cross-machine-sync.md` form rather than a hypothetical published mdbook URL. Today there's no published-URL convention in tome's --help text (per the cli.rs audit); relative path works for users running `--help` inside a clone, which is the realistic case for v0.10."
  - "Init / MigrateLibrary commands NOT updated with their own long_about pointers (CONTEXT mentioned this as Claude's discretion). Skipped — both commands already have substantive after_help blocks; adding more text risked clutter without much discoverability gain. DOC-03's success criterion is just that `tome sync --help` references the page, which it now does."
  - "Final cross-machine-sync.md is 259 lines vs the plan's 150-250 target. The 9-line excess is from cross-link prose — both top-of-page (architecture.md anchor) and per-section (Configuration / CHANGELOG) — that the planner's example didn't cost. No content reduction was warranted; the page reads cleanly at 259 lines."

patterns-established:
  - "Walkthrough-first doc structure for cross-cutting workflow pages — open with two numbered narratives covering the main happy paths, then reference sections for schema / vocabulary / edge cases. Reusable for future cross-cutting docs (e.g. Tauri GUI v1.0 onboarding)."
  - "Cross-link rather than re-document — when a config schema is already in configuration.md, point at it; don't restate. Mirrors what Plan 16-03 (architecture.md) and Plan 16-04 (CHANGELOG.md) chose."

requirements-completed:
  - DOC-03

# Metrics
duration: 4min
completed: 2026-05-08
---

# Phase 16 Plan 05: Cross-Machine Sync Doc Summary

**Created docs/src/cross-machine-sync.md (259 lines) documenting the library-as-dotfiles workflow end-to-end with two walkthroughs (Machine A source-of-truth, Machine B fresh machine) plus five reference sections (tome.lock, auto_install_plugins consent, directory_overrides, missing-claude error, v0.9 library migration). Page is reachable via mdbook TOC AND `tome sync --help` long-about, with an in-prose cross-link from architecture.md's Library-canonical model section.**

## Performance

- **Duration:** ~4 min
- **Started:** 2026-05-08T14:22:21Z
- **Completed:** 2026-05-08T14:26:13Z
- **Tasks:** 3
- **Files created:** 1
- **Files modified:** 3

## Accomplishments

- Wrote `docs/src/cross-machine-sync.md` (259 lines) — two numbered walkthroughs (Machine A / Machine B) + five reference sections (tome.lock semantics, auto_install_plugins consent, directory_overrides, missing-claude actionable error, v0.9 library migration).
- Wired the new page into `docs/src/SUMMARY.md` between Configuration and Development Workflow (D-DOC03-1 placement).
- Added `long_about` attribute on `Command::Sync` in `crates/tome/src/cli.rs` referencing `docs/src/cross-machine-sync.md`. Verified `tome sync --help` renders the cross-machine-sync.md reference line in the long-form help.
- Added in-prose link from `architecture.md` Library-canonical model section → `cross-machine-sync.md` (D-DOC03-3 linking strategy honoured both ways: SUMMARY.md TOC + sync --help + architecture.md prose link).
- Used the corrected `AutoInstall` enum variant names (`Always | Ask | Never`) per the actual code in `machine.rs:29-33`, NOT the `Yes | Never | Prompt` reference in CONTEXT.md DOC-03 D-DOC03-2.
- All locked v0.10 vocabulary used; all forbidden phrases absent (verified via `rg` greps in acceptance criteria — `tome adopt`, `tome forget`, "no longer configured", "consolidated cache", "first-sync v0.10" all return zero matches).
- Reproduced the missing-claude error message verbatim from `marketplace.rs::ClaudeMarketplaceAdapter::new` rather than paraphrasing.
- `make ci` passes (793 lib tests + 175+ integration tests = 968+ total; fmt-check + clippy clean). Only `typos` step fails because the typos CLI isn't on the local PATH — same environmental gap noted in Plan 16-03; not introduced by this plan.

## Task Commits

Each task was committed atomically:

1. **Task 1: Write `docs/src/cross-machine-sync.md`** — `b86db3f` (docs)
2. **Task 2: Insert TOC entry in `docs/src/SUMMARY.md`** — `1f5e3e8` (docs)
3. **Task 3: Add `long_about` to `Command::Sync` + cross-link from architecture.md** — `35b04dc` (docs)

**Plan metadata commit:** to be created after this SUMMARY (state + roadmap + summary + requirements).

## Files Created/Modified

- `docs/src/cross-machine-sync.md` — **NEW**. 259 lines. Top-level title; two walkthroughs (Machine A source-of-truth, Machine B fresh machine bootstrap); five reference sections covering tome.lock semantics, auto_install_plugins consent (Always | Ask | Never), directory_overrides path remap, missing-claude actionable error, and v0.9 library migration via `tome migrate-library`.
- `docs/src/SUMMARY.md` — One-line insert: `- [Cross-machine sync](cross-machine-sync.md)` between Configuration and Development Workflow (line 7).
- `crates/tome/src/cli.rs` — `Command::Sync` gains `long_about` attribute (`#[command(long_about = "...", after_help = "...")]`). Verified rendering via `cargo run -p tome -- sync --help`.
- `docs/src/architecture.md` — Three-line addition to the Library-canonical model section's "Cross-machine portability" bullet pointing readers at the new walkthrough page.

## Decisions Made

- **Page placement** honours D-DOC03-1 reading-order intent: between Configuration and Development Workflow. Reading order is now `Commands → Configuration → Cross-machine sync → Development Workflow → Architecture`. The new page slots adjacent to the topics it builds on without forcing reorder of other entries.
- **AutoInstall variant names corrected** to `Always | Ask | Never` per `machine.rs:29-33`, NOT the `Yes | Never | Prompt` reference in CONTEXT.md DOC-03 D-DOC03-2. Plan 16-03 caught the same CONTEXT.md drift; this plan honours the corrected vocabulary.
- **Auto-install consent prompt rendering** uses the actual `dialoguer::Select` labels (`Yes (always — install on every sync)` / `Yes (ask me again next time)` / `No (never ask again on this machine)`) rather than the literal `[Y/n/never]` shorthand from Phase 13 D-08 design context. Source of truth is the shipped code (`reconcile.rs::prompt_consent`), not the design sketch.
- **Missing-claude error reproduced verbatim** from `marketplace.rs::ClaudeMarketplaceAdapter::new` (line 612): `claude CLI not found on PATH — install Claude Code, or remove [directories.<name>] entries with type = "claude-plugins" from tome.toml`. Doc accuracy demands the actual shipped wording, not the planner's prose sketch.
- **Long-about link path** uses the relative `docs/src/cross-machine-sync.md` form. There's no published-URL convention in tome's existing --help text; relative path works for users running `--help` inside a clone, the realistic case for v0.10.
- **Init / MigrateLibrary commands NOT updated** with their own long_about pointers. Both commands already have substantive after_help blocks; adding more text risked clutter without much discoverability gain. DOC-03's success criterion is satisfied by `tome sync --help` referencing the page.
- **Final length 259 lines** (vs 150-250 target). The 9-line excess is cross-link prose — top-of-page (architecture.md anchor) and per-section (configuration.md, CHANGELOG.md) — that the planner's example didn't cost. No content reduction warranted; page reads cleanly at 259 lines.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 2 — Missing Critical] Architecture.md cross-link to cross-machine-sync.md**
- **Found during:** Task 3 (long_about wiring)
- **Issue:** The plan's `must_haves.key_links` block specifies a third link target — `docs/src/architecture.md Library-canonical model section → cross-machine-sync.md` via in-prose link — but Task 3 as written only mentions `cli.rs` + verification. Without this link, the page is reachable from SUMMARY.md TOC and `tome sync --help`, but a reader landing on architecture.md's library-canonical section has no obvious jump-off to the workflow doc.
- **Fix:** Added a three-line cross-link to architecture.md's "Cross-machine portability" bullet (under the Library-canonical model H2): `See [Cross-machine sync](cross-machine-sync.md) for the end-to-end walkthrough...`. Bundled into Task 3's commit since both edits serve D-DOC03-3 linking strategy.
- **Files modified:** `docs/src/architecture.md`
- **Verification:** `rg -n '\\[.*Cross-machine.*\\]\\(cross-machine-sync.md\\)' docs/src/architecture.md` outputs the new line.
- **Committed in:** `35b04dc` (Task 3 commit)

**2. [Rule 1 — Bug] Auto-install consent prompt rendering**
- **Found during:** Task 1 (writing the auto_install_plugins reference section)
- **Issue:** The plan's CONTEXT.md `read_first` bullet for Task 1 references the literal `Auto-install missing plugins on every sync? [Y/n/never]` prompt wording from Phase 13 D-08, but inspecting `reconcile.rs::prompt_consent` (line 442-464) shows the shipped UX is a `dialoguer::Select` arrow-key list with three full-sentence option labels, not a single-line `[Y/n/never]` text input. Documenting the design-context shorthand instead of the shipped UX would have misled users on first-sync.
- **Fix:** The Walkthrough — Machine B section renders the actual three Select-dialog labels (`Yes (always — install on every sync)` / `Yes (ask me again next time)` / `No (never ask again on this machine)`) prefaced with the actual prompt question (`Tome detected N missing or out-of-date managed plugins. Install/update them now?`).
- **Files modified:** `docs/src/cross-machine-sync.md`
- **Verification:** Compared the page against `crates/tome/src/reconcile.rs:442-464` byte-for-byte for the prompt + label strings.
- **Committed in:** `b86db3f` (Task 1 commit)

**3. [Rule 1 — Bug] Missing-claude error message reproduction**
- **Found during:** Task 1 (writing the missing-claude reference section)
- **Issue:** The plan's `<action>` block for Task 1 includes a placeholder error block formatted as `error: \`claude\` binary not found on PATH / Why: ... / Suggestion: ...` (Conflict/Why/Suggestion shape). But inspecting `crates/tome/src/marketplace.rs:611-615` shows the shipped error is a single sentence: `claude CLI not found on PATH — install Claude Code, or remove [directories.<name>] entries with type = "claude-plugins" from tome.toml`. Documenting a fictional Conflict/Why/Suggestion shape would diverge from the actual stderr output.
- **Fix:** The reference section reproduces the shipped error verbatim. The surrounding prose explains the actionable resolution (install Claude Code or remove claude-plugins directory entries).
- **Files modified:** `docs/src/cross-machine-sync.md`
- **Verification:** `rg -n "claude CLI not found" crates/tome/src/marketplace.rs` confirms the shipped wording.
- **Committed in:** `b86db3f` (Task 1 commit)

**4. [Rule 1 — Bug] AutoInstall variant names — `Always | Ask | Never`, NOT `Yes | Prompt | Never`**
- **Found during:** Task 1 (writing the auto_install_plugins reference section)
- **Issue:** The plan's `<action>` example TOML block shows `auto_install_plugins = "yes"` / `"never"` / `"prompt"`. CONTEXT.md DOC-03 D-DOC03-2 uses the same incorrect labels. The actual `crates/tome/src/machine.rs:29-33` enum is `AutoInstall { Always, Ask, Never }` with `#[serde(rename_all = "lowercase")]`. Documenting `yes`/`prompt` would have been a hard error — `machine.toml` parsing would refuse those values.
- **Fix:** The reference section uses `auto_install_plugins = "always"` / `"never"` / `"ask"` with the corresponding semantic labels in prose. (Plan 16-03 caught the same CONTEXT.md drift and made the same correction in architecture.md; this plan propagates it to cross-machine-sync.md.)
- **Files modified:** `docs/src/cross-machine-sync.md`
- **Verification:** `rg -n 'AutoInstall::' crates/tome/src/machine.rs` confirms variants are `Always`, `Ask`, `Never`.
- **Committed in:** `b86db3f` (Task 1 commit)

---

**Total deviations:** 4 auto-fixed (3 Rule 1 — bug fixes / accuracy corrections, 1 Rule 2 — missing critical cross-link). All four corrections were necessary for doc accuracy or to fully satisfy the plan's `must_haves.key_links` invariant. None expanded scope; all edits stayed within the planned files.

## Issues Encountered

- **typos CLI not on PATH** — `make ci`'s final `typos` step fails with `make: typos: No such file or directory`. Same environmental gap noted in Plan 16-03's SUMMARY. Not caused by this plan; the new page uses well-formed English with no obvious typos visible to manual proofreading.
- **mdbook not on PATH** — couldn't run `mdbook build` to verify the doc renders. The acceptance criteria don't require this; standard markdown anchors and relative links should work without mdbook validation.
- **Page length slightly over target (259 vs 150-250)** — accepted; cross-link prose adds value and the page reads cleanly. Plan target was approximate.

## Next Phase Readiness

- DOC-03 closes; the v0.10 documentation surface is complete (DOC-01 architecture.md ✓ via Plan 16-03; DOC-02 CHANGELOG.md ✓ via Plan 16-04; DOC-03 cross-machine-sync.md ✓ via this plan).
- Phase 16 is fully complete pending the verifier run. All five plans (16-01 through 16-05) shipped successfully.
- Phase 17 (Migration polish + UAT + release — REL-01..05) is unblocked by this plan; the rc cut can proceed once verifier validates Phase 16.
- Future architecture-doc updates should preserve the cross-link from architecture.md to cross-machine-sync.md (added in this plan, Rule 2 deviation).
- The CONTEXT.md DOC-03 D-DOC03-2 reference to `Yes | Never | Prompt` is still in `.planning/phases/16-cleanup-message-ux-docs/16-CONTEXT.md` (unpatched, out of scope for this plan). If a future agent re-reads CONTEXT.md for any v0.10 doc work, they should cross-check against `machine.rs:29-33` for the correct enum names.

## Self-Check: PASSED

- `docs/src/cross-machine-sync.md` — FOUND (259 lines, 7 H2 sections: 2 Walkthroughs + 5 References)
- `docs/src/SUMMARY.md` — FOUND with cross-machine-sync.md TOC entry on line 7
- `crates/tome/src/cli.rs` — FOUND with `long_about` attribute on `Command::Sync` referencing `docs/src/cross-machine-sync.md`
- `docs/src/architecture.md` — FOUND with cross-link from Library-canonical model section
- `.planning/phases/16-cleanup-message-ux-docs/16-05-SUMMARY.md` — FOUND
- Commit `b86db3f` (Task 1) — FOUND
- Commit `1f5e3e8` (Task 2) — FOUND
- Commit `35b04dc` (Task 3) — FOUND
- `tome sync --help` renders cross-machine-sync.md reference — VERIFIED via `cargo run -p tome -- sync --help`
- `make ci` (excluding typos) — PASSED: fmt-check + clippy + 968+ tests, 0 failures
- All forbidden phrases absent from cross-machine-sync.md — VERIFIED via `rg` greps (`tome adopt`, `tome forget`, "no longer configured", "consolidated cache", "first-sync v0.10" all zero matches)

---
*Phase: 16-cleanup-message-ux-docs*
*Completed: 2026-05-08*
