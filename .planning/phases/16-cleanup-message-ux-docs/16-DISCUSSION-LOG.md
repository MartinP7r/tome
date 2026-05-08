# Phase 16: Cleanup-message UX + docs - Discussion Log

> **Audit trail only.** Do not use as input to planning, research, or execution agents.
> Decisions are captured in CONTEXT.md — this log preserves the alternatives considered.

**Date:** 2026-05-08
**Phase:** 16-cleanup-message-ux-docs
**Areas discussed:** Cleanup three-bucket scope & layout, Migration prompt confirmation + summary shape, Architecture doc scope, Cross-machine doc placement

---

## Gray Area Selection

| Option | Description | Selected |
|--------|-------------|----------|
| Cleanup three-bucket scope & layout | UX-01. Today's `cleanup_library` is 2-bucket (Case 1 unowned-transition, Case 2 missing-from-disk). The third 'now-in-exclude-list' bucket is structurally different — distribution symlinks vs library content. | ✓ |
| Migration prompt confirmation + summary shape | UX-02. `tome migrate-library` currently has NO confirm gate — `render_plan` then `execute()` runs unconditionally. | ✓ |
| Architecture doc scope | DOC-01. `architecture.md` is 60+ lines today, 'managed = symlink' framing throughout. | ✓ |
| Cross-machine doc placement | DOC-03. Standalone `docs/src/cross-machine-sync.md` vs subsection of architecture.md vs cookbook in configuration.md. | ✓ |

**User's choice:** All four areas selected.
**Notes:** Phase 16 spans both UX work (UX-01, UX-02) and three documentation artifacts; user wanted the substantive decisions on each captured before planning.

---

## Cleanup three-bucket scope

### Q1 — What does the third cleanup bucket ('now-in-exclude-list') cover, mechanically?

| Option | Description | Selected |
|--------|-------------|----------|
| Distribution-side only | Bucket #3 = library skills whose distribution symlinks were just removed because user added them to `machine.toml::disabled`. Library content stays untouched (matches LIB-04 preservation). Cleanup output unifies `cleanup_library` + `cleanup_target`. | ✓ |
| Library-side too | Bucket #3 = library entries currently excluded AND removed from library entirely. Contradicts LIB-04 'library is canonical' and breaks 'enable later to restore' workflow. | |
| Pure messaging (no behavior change) | Surface what `cleanup_target` already does silently; both phases keep current responsibilities. | |

**User's choice:** Distribution-side only.
**Notes:** Honors LIB-04 invariant; library content always preserved. Unification of `cleanup_library` + `cleanup_target` output is the implementation work.

### Q2 — How should the unified three-bucket cleanup output render?

| Option | Description | Selected |
|--------|-------------|----------|
| Per-bucket header + per-entry inline hint | Each bucket gets a colored heading line, then per-entry lines with hint inline next to skill name. Matches today's `cleanup_library` Case-2 'N skill(s) missing' style; per-skill hints because directory varies. | ✓ |
| Per-bucket header + end-of-bucket hint summary | Heading + flat skill list + ONE end-of-bucket hint. Less noise but per-skill hints missing where directory name varies. | |
| Single unified table (tabled rounded) | One `tabled` output with columns SKILL \| BUCKET \| LAST-KNOWN-SOURCE \| HINT. Heavyweight for the common case. | |

**User's choice:** Per-bucket header + per-entry inline hint.

### Q3 — Where do bucket headers + hints render — stdout or stderr?

| Option | Description | Selected |
|--------|-------------|----------|
| stderr | All cleanup output to stderr. Matches today's `eprintln!` discipline (`cleanup.rs:111, 178`) and HARD-15 wizard-chrome-to-stderr precedent. stdout reserved for machine-readable status. | ✓ |
| stdout for headers, stderr for warnings | Mixed streams; harder to grep; contradicts HARD-15. | |
| stdout (matches today's interactive `cleanup_library` Case-2) | Today's interactive Case-2 prompt uses `println!`. Contradicts HARD-15. | |

**User's choice:** stderr.
**Notes:** Today's `println!` for the interactive Case-2 prompt header (`cleanup.rs:146`) drops away when `dialoguer::Confirm` (which writes to stderr by default) takes over.

### Continuation choice

**User's choice:** Next area.
**Notes:** The four decisions captured (bucket #3 scope, unified output, per-entry inline hints, stderr discipline) are enough to plan; specific hint wording, color choices, and bucket ordering can be Claude's discretion within established patterns.

---

## Migration prompt confirmation + summary shape

### Q1 — What's the default for the migration confirmation prompt?

| Option | Description | Selected |
|--------|-------------|----------|
| Default-no, require explicit y | `dialoguer::Confirm::default(false)`. User must press 'y'. Safest — never silently mutates. Matches Phase 14 D-B3 (`tome remove skill`) and `cleanup.rs:168` Case-2 default(false). | ✓ |
| Default-yes, Enter accepts | `dialoguer::Confirm::default(true)`. Lower friction. Inconsistent with cleanup + `tome remove skill` conventions; one Enter slip = irreversible migration. | |
| No default, require y or n | Forces explicit choice. Breaks `--no-input` non-interactive runs. | |

**User's choice:** Default-no.

### Q2 — What's the migration plan summary format?

| Option | Description | Selected |
|--------|-------------|----------|
| Inline summary line + per-skill table | Top: bold inline summary line. Below: `tabled::Style::rounded()` table with SKILL \| SOURCE \| SIZE \| STATUS. Matches WHARD-07 wizard precedent + UX-02's literal 'summary table' wording. | ✓ |
| Inline lines only (today's `render_plan` format) | Keep per-skill bullet-list format, just add confirmation prompt + disk-estimate line above. Doesn't quite deliver UX-02's 'summary table' wording. | |
| Single-row stats summary, skill list collapsed | Just '62 symlinks → real dirs (~30 MB), 0 broken' — no per-skill list unless `--verbose`. Loses transparency. | |

**User's choice:** Inline summary line + per-skill table.

### Q3 — How should the disk estimate be computed?

| Option | Description | Selected |
|--------|-------------|----------|
| `metadata().len()` walk during `plan()` | ~20 ms / skill = ~1.2s for 62 skills. Acceptable for one-shot ceremonial command. Byte-accurate. | ✓ |
| Stub from manifest content_hash + count | Skip the disk number entirely; user can `du -sh` themselves. Simplest. | |
| Real `du`-style block walk | Most accurate but slowest (~100ms/skill = ~6s on 62 skills). Overkill. | |

**User's choice:** `metadata().len()` walk.

### Q4 — How should the migration prompt behave under `--no-input` (CI / scripted)?

| Option | Description | Selected |
|--------|-------------|----------|
| Add `--yes` flag, `--no-input` requires `--yes` | Mirrors Phase 14 D-B3. Without `--yes`: bail with clear message. With `--yes`: skip prompt and proceed. Matches existing CLI conventions. | ✓ |
| Default to abort under `--no-input` (no `--yes` flag) | Skip prompt entirely; refuse to proceed. Blocks any CI/scripted migration. | |
| Default to confirm under `--no-input` (auto-yes) | `--no-input` → implicit yes. Risky — trips destructive action without typing y. | |
| More questions about migration prompt | Continue exploring this area. | |

**User's choice:** Add `--yes` flag, `--no-input` requires `--yes`.

---

## Architecture doc scope

### Q1 — What's the scope of the `architecture.md` update for v0.10?

| Option | Description | Selected |
|--------|-------------|----------|
| Targeted rewrites of changed paragraphs + 4 new sections | Keep existing skeleton. Rewrite 3-4 paragraphs containing v0.9 framing. Add 4 new sections: Library-canonical model, Lockfile-authoritative reconciliation, Marketplace adapter trait, Unowned lifecycle. ~150-200 new lines. | ✓ |
| Major rewrite from scratch | Throw out current shape. Loses institutional memory. Probably overkill since 80% of doc is still accurate. | |
| Minimal patch (just fix factual lies) | Edit only paragraphs with false statements. Falls short of DOC-01 success criterion calling for explicit sections on the 4 v0.10 concepts. | |

**User's choice:** Targeted rewrites + 4 new sections.

### Continuation choice

**User's choice:** Next area.
**Notes:** Section ordering, prose style, diagram update (the Excalidraw link), and CHANGELOG.md tone for DOC-02 can be Claude's discretion within standard doc conventions.

---

## Cross-machine doc placement

### Q1 — Where does the cross-machine workflow doc live, and how is it linked?

| Option | Description | Selected |
|--------|-------------|----------|
| Standalone `docs/src/cross-machine-sync.md`, linked from SUMMARY + `tome sync --help` | New top-level page. Listed in SUMMARY.md alongside Configuration / Commands / Architecture. Linked from `tome sync --help` long description and architecture.md. Matches PROJECT.md framing of library-as-dotfiles as core value. | ✓ |
| Subsection of `architecture.md` | Inline workflow into architecture.md as a new section. Mixes reference (architecture) with tutorial (workflow). | |
| Cookbook in `configuration.md` | Add cross-machine sync section as TOML examples. Configuration.md is schema-focused; would shift its tone. | |

**User's choice:** Standalone page.

### Q2 — How should the page be structured — walkthrough or reference?

| Option | Description | Selected |
|--------|-------------|----------|
| Walkthrough first, reference second | Lead with numbered walkthroughs (Machine A → Machine B). Below: reference covering tome.lock semantics, `auto_install_plugins` values, `directory_overrides`, missing-claude error, v0.10 migration step. Concrete first, abstract second — matches mdbook conventions. | ✓ |
| Reference-only | Each piece as standalone reference, no walkthrough. Less obvious where to start. | |
| Walkthrough-only | Just numbered Machine A / Machine B steps. Doesn't anchor lockfile / consent / overrides for debugging. | |

**User's choice:** Walkthrough first, reference second.

---

## Final continuation

**User's choice:** I'm ready for context.

---

## Claude's Discretion

(Areas where the user said "you decide" or implementation details are bounded enough to defer to Claude:)

- Exact wording of cleanup hint strings (within Conflict/Why/Suggestion shape)
- Bucket ordering in cleanup output (recommend A → B → C)
- Color / glyph choices per bucket (recommend reusing today's `console::style().yellow().bold()` pattern)
- Exact text of bucket header lines (number agreement, plurals)
- `CleanupSummary` struct shape vs side-channel `Vec<ExcludedSkill>`
- Migration table column widths, overflow behaviour, truncation policy (>~100 skills)
- `--yes` short form `-y` (recommend yes — Phase 14 D-B3 + Unix conventions)
- `MigrationEntry.byte_size` field name and serialization
- Helper for human-readable byte size (`humansize` crate or inline 10-LOC helper)
- CHANGELOG.md tone — match existing house style
- Excalidraw diagram update — defer to a follow-up issue if existing diagram still represents the broad two-tier flow
- Whether to update `docs/src/roadmap.md` — likely yes
- Section ordering of new architecture.md sections

## Deferred Ideas

- JSON shape for cleanup output (post-v0.10)
- Excalidraw architecture diagram refresh (v0.11+)
- `tome migrate-library --revert` / undo flag (out of scope per Phase 11 D-04)
- Localized docs (en/de/ja) — defer indefinitely
- Doctor command remediation hints expansion (Phase 17 or v0.11+)
- CHANGELOG.md prior-milestone shape audit (match what's there; flag if awkward)
