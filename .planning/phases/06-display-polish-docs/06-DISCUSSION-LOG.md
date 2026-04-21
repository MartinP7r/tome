# Phase 6: Display Polish & Docs - Discussion Log

> **Audit trail only.** Do not use as input to planning, research, or execution agents.
> Decisions are captured in CONTEXT.md — this log preserves the alternatives considered.

**Date:** 2026-04-21
**Phase:** 06-display-polish-docs
**Areas discussed:** Table style reconciliation, Long path handling, Column set & ordering, PROJECT.md WHARD-08 structure

---

## Area Selection

| Option | Description | Selected |
|--------|-------------|----------|
| Table style reconciliation | ROADMAP says `Style::rounded()`; status.rs uses `Style::blank()`. Pick which one wizard should use and why. | ✓ |
| Long path handling strategy | Truncate middle, truncate end, wrap, or let tabled auto-width handle it. | ✓ |
| Column set & ordering | Wizard currently Name/Path/Type/Role. Status NAME/TYPE/ROLE/PATH/SKILLS. | ✓ |
| PROJECT.md WHARD-08 structure | How to update "Known Gaps" section for WIZ-01–05. | ✓ |

**User's choice:** All four areas selected.

---

## Table Style Reconciliation

| Option | Description | Selected |
|--------|-------------|----------|
| `Style::blank()` (match status.rs) | Strict visual consistency with `tome status`. Quiet, borderless look. Override the ROADMAP phrasing. | |
| `Style::rounded()` (honor ROADMAP) | More "ceremonial" look for the one-shot init summary. Diverges from status.rs but gives init a finished feel. | ✓ |
| `Style::modern()` (middle ground) | Standard ASCII-ish borders with square corners. Neither minimal nor ornate. | |

**User's choice:** `Style::rounded()`.
**Notes:** Wizard summary diverges visually from `tome status` intentionally. Init is ceremonial (one-shot); status is repetitive (daily use). ROADMAP literal wording takes precedence over the "matching visual language" reference.

---

## Long Path Handling

**User asked:** "What's the common practice here?"

**Claude summarized:** Modern Rust CLIs (docker ps, kubectl get, gh) detect terminal width and truncate the "overflow" column (path/URL/description) with `…`. Wrapping is less common in tables because uneven row heights make them noisy. Fixed caps without width detection break on narrow terminals. Idiomatic `tabled` pattern: `Width::truncate(term_cols).priority(PriorityMax)` automatically shrinks the widest column first. `terminal_size` crate for reading current cols with fallback to 80.

Also noted: `tome init` is a one-shot ceremonial summary, so minor truncation is acceptable — the full config is in `~/.tome/tome.toml` anyway.

**User's choice:** Follow recommendation — terminal-width detection + `Width::truncate(term_cols).priority(PriorityMax)` with ellipsis and 80-col fallback.

---

## Column Set & Ordering

| Option | Description | Selected |
|--------|-------------|----------|
| Match status.rs: NAME / TYPE / ROLE / PATH | Same order as `tome status` minus SKILLS. Consistent mental model across wizard and status. | ✓ |
| Keep current wizard: NAME / PATH / TYPE / ROLE | Preserve existing mental model: identify → where → what. | |
| NAME / ROLE / TYPE / PATH | Leads with identity and purpose. TYPE is implementation detail pushed right. PATH last plays nicer with truncation. | |

**User's choice:** Match status.rs ordering.
**Notes:** PATH last also benefits the `PriorityMax` truncation strategy — the widest column (usually PATH with git repo paths) is the one truncated first, and putting it last keeps the visual flow consistent.

---

## PROJECT.md WHARD-08 Structure

| Option | Description | Selected |
|--------|-------------|----------|
| Dedicated "Hardened in v0.7" block | New subsection listing WIZ-01 through WIZ-05 each with description and "shipped v0.6, hardened v0.7 (Phases 4+5)" note. Remove from Known Gaps. | ✓ |
| Retrofit WIZ-XX labels onto existing "Previously Validated" entries | Keep existing section, prefix bullets with WIZ-XX labels. Minimal restructuring but labels feel bolted-on. | |
| Remove Known Gaps entirely; consolidate into one v0.7 closure note | Drop "Known Gaps" section. Single "v0.7 closure" paragraph. Cleanest narrative but loses granularity. | |

**User's choice:** Dedicated "Hardened in v0.7" block with WIZ-01–05 individually labeled.
**Notes:** Satisfies ROADMAP literal wording ("lists WIZ-01 through WIZ-05 as validated") and gives future readers clear WIZ-XX traceability.

---

## Claude's Discretion

Following items were assigned to Claude's discretion with acceptable variants noted in CONTEXT.md:

- Exact title of the new PROJECT.md subsection (`### Hardened in v0.7`, `### Wizard Hardening Closure`, etc.)
- Whether WIZ-XX bullets sit under a new header or nest under existing `### Validated in v0.7`.
- Specific one-line descriptions for each WIZ-XX.
- Whether to also add a CHANGELOG note.
- Import style for `Style::rounded()` (grouped use statement vs inline qualified).
- Whether `terminal_size` is pulled in directly or via transitive exposure (check `dialoguer`/`console`).
- Whether to add a fast sanity test (`show_directory_summary(&empty_map)` doesn't panic). Nice-to-have.

## Deferred Ideas

- Upgrade `status.rs` to `Style::rounded()` for visual parity across commands — separate polish pass.
- Snapshot tests for wizard summary output — high maintenance churn; substring assertions sufficient per Phase 5 D-09.
- Registry expansion (Cursor/Windsurf/Aider) — v2 requirements WREG-01/02/03.
- `NO_COLOR` env var handling for bolded headers — already no-ops via `console::style()`; verify only.
- Reworking overall PROJECT.md taxonomy (three validation sections) — not Phase 6's concern.
- Env var / flag to override terminal width — not required.
