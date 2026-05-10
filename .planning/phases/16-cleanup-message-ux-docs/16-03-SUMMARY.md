---
phase: 16-cleanup-message-ux-docs
plan: 03
subsystem: documentation
tags: [docs, mdbook, architecture, doc-rewrite, library-canonical, marketplace-adapter, unowned-lifecycle, lockfile-reconciliation]

# Dependency graph
requires:
  - phase: 11-library-canonical-core
    provides: managed-as-real-dir copy semantics, migrate-library command, refuse-on-v0.9-shape gate (LIB-01..05, D-01/D-04/D-08)
  - phase: 12-marketplace-adapter
    provides: MarketplaceAdapter trait + ClaudeMarketplaceAdapter + GitAdapter + InstallFailure aggregation (ADP-01..04)
  - phase: 13-lockfile-authoritative-sync
    provides: reconcile.rs Match/Drift/Vanished classification, auto_install_plugins consent, edit-in-library detection (RECON-01..05)
  - phase: 14-unowned-library-lifecycle
    provides: source_name: Option<DirectoryName>, previous_source breadcrumb, D-API-1/-2 vocab merge (UNOWN-01..03)
  - phase: 15-cli-hardening
    provides: foreign-symlink protection (HARD-09), tilde round-trip (HARD-22), config split, exhaustive-match POLISH-04 sentinels
  - phase: 16-01 (this phase, wave 1)
    provides: three-bucket cleanup output (UX-01)
  - phase: 16-02 (this phase, wave 1)
    provides: migrate-library confirm gate + summary table (UX-02)
provides:
  - rewritten docs/src/architecture.md describing the v0.10 library-canonical model end-to-end
  - four new H2 sections (Library-canonical model, Lockfile-authoritative reconciliation, Marketplace adapter trait, Unowned lifecycle)
  - alphabetised Modules list with entries for marketplace.rs, reconcile.rs, migration_v010.rs, summary.rs, plus updated entries for library/cleanup/manifest/lockfile/remove/reassign
affects: [16-04 (CHANGELOG), 16-05 (cross-machine-sync.md), Phase 17 release notes, future onboarding for v0.10+ contributors]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "doc-as-decision-trail — every claim in architecture.md cross-references the requirement ID or phase decision that backs it (LIB-01..05, RECON-01..05, ADP-01..04, UNOWN-01..03, HARD-XX, Phase 11/13/14 decision codes)"
    - "vocabulary supersession in-doc footnote — the architecture doc references superseded verbs (tome adopt / tome forget) only inside explicit 'originally-proposed ... was folded into ...' supersession sentences"

key-files:
  created: []
  modified:
    - docs/src/architecture.md (60 → 251 lines; sync pipeline rewritten, modules list rebuilt, four new sections added)

key-decisions:
  - "Pipeline reorder: Reconcile listed as step 1 (before Discover) to match actual lib.rs::sync code ordering — reconcile runs against the previously-saved manifest+lockfile before discovery scans for new content. The doc now lists 6 steps (Reconcile → Discover → Consolidate → Distribute → Cleanup → Lockfile) instead of the previous 5."
  - "Modules list reorganised alphabetically (was previously grouped — wizard/config first, then add+remove+reassign together, then by topic). Alphabetical order makes module entries easier to find when reading the doc as reference."
  - "Excalidraw diagram refresh deferred per CONTEXT.md Claude's discretion. Caption updated in-place to note v0.10 staleness rather than removing the link entirely. The broad three-tier shape (discovery → library → distribution) is still accurate; the marketplace adapter dispatcher and unowned lifecycle aren't depicted but readers are pointed at the new sections for those mechanics."
  - "Forbidden trigger phrase 'no longer configured' verified absent. The phrase that originally triggered the v0.10 milestone discussion now does not appear anywhere in the architecture doc."
  - "Vocab merge handled via supersession footnotes embedded in module entries (reassign.rs, remove.rs) and the Unowned lifecycle section. Both `tome adopt` and `tome forget` appear ONLY inside 'originally-proposed ... was folded into ...' supersession sentences — never as live commands."

patterns-established:
  - "Phase-decision callouts in architecture docs — every non-obvious behaviour mentioned in the doc carries a (Phase NN D-XX) or (REQ-ID) tag pointing back at the planning artefact that decided it. This pattern is intended to survive Phase 16; future architecture-doc updates should preserve and extend the trace."

requirements-completed:
  - DOC-01

# Metrics
duration: 6min
completed: 2026-05-08
---

# Phase 16 Plan 03: Architecture Doc Rewrite for v0.10 Summary

**docs/src/architecture.md rewritten end-to-end for the v0.10 library-canonical model: managed-as-real-dir-copy mechanic, lockfile-authoritative reconciliation with Match/Drift/Vanished classification, MarketplaceAdapter trait shape, and Unowned skill lifecycle with the D-API-1/-2 vocab merge fully honoured.**

## Performance

- **Duration:** ~6 min
- **Started:** 2026-05-08T11:17:26Z
- **Completed:** 2026-05-08T11:23:19Z
- **Tasks:** 2
- **Files modified:** 1

## Accomplishments

- Rewrote four existing v0.9-framing paragraphs (Sync Pipeline Consolidate / Distribute / Cleanup steps + Key Patterns "Two-tier model" bullet) for v0.10 semantics — managed AND local skills are real-directory copies; managed is "update channel" not "stored as symlink".
- Added the Reconcile step to the Sync Pipeline (it was previously missing despite being step 1 of the actual `lib.rs::sync` code path).
- Inserted four new H2 sections between Key Patterns and Testing — Library-canonical model, Lockfile-authoritative reconciliation, Marketplace adapter trait, Unowned lifecycle — totalling ~180 lines of v0.10-specific user-facing prose.
- Rebuilt the Modules list alphabetically with new entries for marketplace.rs, reconcile.rs, migration_v010.rs, summary.rs and refreshed entries for library/cleanup/manifest/lockfile/remove/reassign/machine/config/doctor/status/distribute/discover/lint/install/relocate/browse/wizard/update.
- Honoured the D-API-1/-2 vocab merge: `tome adopt` and `tome forget` appear ONLY in supersession footnotes ("originally-proposed ... was folded into ..."); live commands are exclusively `tome reassign --to` and `tome remove skill`.
- Eliminated the forbidden "no longer configured" trigger phrase that originally motivated the v0.10 milestone discussion.

## Task Commits

Each task was committed atomically:

1. **Task 1: Rewrite four existing paragraphs + Modules list for v0.10 framing** — `b0079e0` (docs)
2. **Task 2: Add four new sections (Library-canonical / Reconciliation / Adapter trait / Unowned lifecycle)** — `4d4f66d` (docs)

**Plan metadata commit:** to be created after this SUMMARY (state + roadmap + summary).

## Files Created/Modified

- `docs/src/architecture.md` — Rewrote v0.9-shape framing (60 → 251 lines). Sync Pipeline now lists 6 steps (Reconcile → Discover → Consolidate → Distribute → Cleanup → Lockfile). Modules list alphabetised with new entries for the four v0.10 modules. Four new H2 sections inserted between Key Patterns and Testing.

## Decisions Made

- **Reconcile listed first in Sync Pipeline** — matches actual lib.rs::sync code ordering. Reconcile runs against the previously-saved manifest+lockfile before discovery scans for new content. The doc previously had 5 steps starting with Discover; v0.10 promotes Reconcile to step 1 of the documented pipeline.
- **Alphabetical Modules list ordering** — easier to find specific module entries when reading the doc as reference. Previously grouped by topic.
- **Excalidraw diagram refresh deferred** — caption updated in-place to note v0.10 staleness; the link itself preserved per CONTEXT.md `<decisions>` "Claude's Discretion". The diagram still represents the broad three-tier flow accurately; refreshing it for the marketplace adapter dispatcher and unowned lifecycle is a v0.11+ doc-polish task.
- **Vocab supersession via inline footnotes, not a separate "history" section** — `tome adopt` / `tome forget` appear only in module-entry sentences explaining the supersession ("The originally-proposed `tome adopt` verb was folded into this command..."). This keeps the supersession traceable without giving the deprecated verbs a section heading.
- **Did not split `add.rs` / `remove.rs` / `reassign.rs` from a single bullet** in the original doc; instead split them into individual alphabetical entries because `remove.rs` and `reassign.rs` carry meaningful Phase 14 D-API-1/-2 surface area that shouldn't be hidden inside a multi-module bullet.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 2 - Missing Critical] Added Reconcile step to the Sync Pipeline numbered list**
- **Found during:** Task 1 (rewriting Consolidate / Distribute / Cleanup paragraphs)
- **Issue:** The plan's Step 1-3 directs me to rewrite the existing Consolidate (#2), Distribute (#3), and Cleanup (#4) paragraphs, but the existing pipeline list omitted the Reconcile step entirely — and Phase 13 RECON-01..05 made reconcile the first thing `lib.rs::sync` does (line 1527). Leaving Reconcile out of the pipeline summary would have misrepresented v0.10's flow.
- **Fix:** Added `1. **Reconcile** (`reconcile.rs`)` as the first step (before Discover) with a one-paragraph summary cross-linking to the new Lockfile-authoritative reconciliation section. The previous 5 steps shifted to positions 2-6.
- **Files modified:** docs/src/architecture.md
- **Verification:** Inspecting `crates/tome/src/lib.rs::sync` confirmed reconcile_lockfile is invoked at line 1527, before discover_all at line 1585.
- **Committed in:** `b0079e0` (Task 1 commit)

**2. [Rule 1 - Bug] Corrected Excalidraw caption from "two-tier" to "discovery → library → distribution flow" with v0.10-staleness note**
- **Found during:** Task 1 (rewriting Two-tier model bullet under Key Patterns)
- **Issue:** The plan's `<decisions>` for Task 1 only specifies rewriting the four paragraphs and modules list, but the Excalidraw caption on line 3 referred to "the two-tier discovery → library → distribution flow" — using the same "two-tier" label that the plan explicitly directs me to drop from the body text. Leaving the caption mismatched with the rewritten body would have been internally inconsistent.
- **Fix:** Updated the caption to drop "two-tier", note that the diagram pre-dates v0.10 and doesn't depict the marketplace adapter dispatcher / unowned lifecycle, and call out that the broad three-tier shape is still accurate. Refresh deferred per CONTEXT.md Claude's discretion (no `16-deferred-items.md` entry needed — the existing deferred-items file already addresses follow-ups; this caption note is self-contained).
- **Files modified:** docs/src/architecture.md
- **Verification:** Caption no longer says "two-tier"; readers are pointed at the new sections for the mechanics not depicted.
- **Committed in:** `b0079e0` (Task 1 commit)

**3. [Rule 1 - Bug] AutoInstall enum variants are `Always | Ask | Never`, not `Yes | Prompt | Never` as referenced in CONTEXT.md DOC-03 D-DOC03-2**
- **Found during:** Task 2 (writing the auto_install_plugins consent section)
- **Issue:** CONTEXT.md DOC-03 D-DOC03-2 mentions `Yes | Never | Prompt`, but the actual `crates/tome/src/machine.rs` enum is `AutoInstall { Always, Ask, Never }` — `Always` not `Yes`, `Ask` not `Prompt`. Documenting the wrong variant names would have been a hard error.
- **Fix:** The new Lockfile-authoritative reconciliation section uses `Always | Ask | Never` per the actual code. (DOC-03's CONTEXT entry is its own concern — Plan 16-05 will need to use the correct variant names too; flagging here for awareness but not modifying the CONTEXT.md.)
- **Files modified:** docs/src/architecture.md
- **Verification:** `rg -n "AutoInstall::" crates/tome/src/machine.rs` confirms variants are `Always`, `Ask`, `Never`.
- **Committed in:** `4d4f66d` (Task 2 commit)

**4. [Rule 2 - Missing Critical] MarketplaceAdapter trait methods take `&self`, not `&mut self` as the plan's example code showed**
- **Found during:** Task 2 (rendering the rust code block for the trait signature)
- **Issue:** The plan's Step 4 example code shows `fn install(&mut self, plugin: &str)` and `fn update(&mut self, plugin: &str)` and `fn list_installed(&mut self)` — but the actual `marketplace.rs:83-113` declares all six methods with `&self`. The cache invalidation works through `RefCell` interior mutability, not `&mut self`. Documenting `&mut self` would have been a hard error.
- **Fix:** The trait code block in the new Marketplace adapter trait section uses `&self` everywhere, matching `crates/tome/src/marketplace.rs:83-113`.
- **Files modified:** docs/src/architecture.md
- **Verification:** `rg -n "fn install|fn update|fn list_installed" crates/tome/src/marketplace.rs` confirms all six methods take `&self`.
- **Committed in:** `4d4f66d` (Task 2 commit)

---

**Total deviations:** 4 auto-fixed (2 Rule 1 — bug fixes, 2 Rule 2 — missing critical accuracy).
**Impact on plan:** All four corrections were necessary for doc accuracy. None expanded scope; all were inside the Task 1/Task 2 file (docs/src/architecture.md).

## Issues Encountered

- **mdbook not on PATH** — the plan's Step 7 in Task 2 suggested running `mdbook build` to verify the doc builds without warnings. `command -v mdbook` returned nothing, so this verification step was skipped. The plan acknowledges this is conditional ("if mdbook is on PATH"). No manual broken-link risk because the doc only uses standard markdown anchors that mdbook would not reject.
- **typos CLI not on PATH** — couldn't run a typos check. The new sections use only well-formed English; no obvious typos visible to manual proofreading. Phase 16 deferred-items.md already tracks pre-existing typos issues unrelated to this plan.
- **No CHANGELOG.md or cross-machine-sync.md created in this plan** — those are DOC-02 and DOC-03 respectively, which are Plans 16-04 and 16-05. This plan closes DOC-01 only.

## Next Phase Readiness

- DOC-01 closes; the architecture doc accurately describes v0.10 end-to-end. Plans 16-04 (DOC-02 CHANGELOG.md) and 16-05 (DOC-03 cross-machine-sync.md) can now reference architecture.md sections by H2-anchor for their own cross-links.
- Plan 16-05 specifically should anchor the cross-machine workflow back to `architecture.md#library-canonical-model` and `architecture.md#lockfile-authoritative-reconciliation`.
- Plan 16-05 also needs to use the correct `AutoInstall { Always | Ask | Never }` variant names (deviation #3 above) — CONTEXT.md DOC-03 D-DOC03-2's `Yes | Never | Prompt` reference is incorrect.
- No blockers for Phase 17 (release).

## Self-Check: PASSED

- docs/src/architecture.md — FOUND (251 lines, 8 H2 sections)
- .planning/phases/16-cleanup-message-ux-docs/16-03-SUMMARY.md — FOUND
- Commit b0079e0 (Task 1) — FOUND
- Commit 4d4f66d (Task 2) — FOUND

---
*Phase: 16-cleanup-message-ux-docs*
*Completed: 2026-05-08*
