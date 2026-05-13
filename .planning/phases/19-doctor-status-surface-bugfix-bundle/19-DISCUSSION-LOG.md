# Phase 19: Doctor/status surface + bugfix bundle - Discussion Log

> **Audit trail only.** Do not use as input to planning, research, or execution agents.
> Decisions are captured in CONTEXT.md — this log preserves the alternatives considered.

**Date:** 2026-05-13
**Phase:** 19-doctor-status-surface-bugfix-bundle
**Areas discussed:** Doctor categorization model (OBS-06), Auto-fixable definition (FIX-01), last_sync semantics (OBS-07), FIX-02 timing flake approach
**Defaults confirmed:** FIX-03 delete strategy, FIX-04 strip-ansi-escapes, FIX-06 inline sed, OBS-06 category-aware auto-fixable breakdown

---

## Doctor categorization model (OBS-06)

### Q1: How should the 4 OBS-06 categories be derived?

| Option | Description | Selected |
|--------|-------------|----------|
| Derive from field structure + promote ForeignSymlink | `category` field on `DiagnosticIssue`, computed from (which field, kind) at construction. Library/Directory/Config from field; ForeignSymlink wins if kind matches. Smallest blast radius — no field-structure changes. | ✓ |
| Promote IssueCategory to a separate enum stamped at emission | New `IssueCategory` enum with POLISH-04 pattern; explicitly set at each emission site. More explicit but redundant with field structure. | |
| Keep Kind enum, derive category in renderer/serializer only | No data-model change. Renderer/serializer computes the bucket on the fly. Lightest change but no compile-time guarantee. | |

**User's choice:** Derive from field structure + promote ForeignSymlink
**Notes:** Builds on existing data layout (library_issues/directory_issues/config_issues already there). Foreign-symlink promotion handles the cross-cutting case cleanly. JSON shape gains `category` string per issue.

### Q2: How should `ForeignSymlink` issues be counted in the summary line?

| Option | Description | Selected |
|--------|-------------|----------|
| Only in Foreign-symlink (mutually exclusive) | Each issue belongs to exactly one category. Summary counts sum to `total_issues()`. Simpler mental model. | ✓ |
| In both parent and Foreign-symlink (overlapping) | Foreign-symlinks count in both their parent (Library/Directory) AND Foreign-symlink. Cross-cutting view but counts don't add up. | |

**User's choice:** Only in Foreign-symlink (mutually exclusive)
**Notes:** Invariant: sum of per-category counts == total_issues. Unit test will guard this.

---

## Auto-fixable definition (FIX-01)

### Q3: What determines `DiagnosticIssue` is auto-fixable?

| Option | Description | Selected |
|--------|-------------|----------|
| `repair_kind: Option<RepairKind>` field, with RepairKind enum + ALL sentinel | New `RepairKind` enum following POLISH-04; `Some(_)` = auto-fixable; dispatcher matches on it. Compile-time guarantee that every variant has a handler. | ✓ |
| `has_auto_repair() -> bool` method derived from `DiagnosticIssueKind` | Add method to existing enum. Simpler but couples bool and dispatcher by convention only. | |
| Keep substring matching, invert to positive list | Smallest change. Hardcoded positive substring matches. Most fragile. | |

**User's choice:** `repair_kind: Option<RepairKind>` field, with RepairKind enum + ALL sentinel
**Notes:** Matches POLISH-04 pattern locked in earlier phases. Removes the substring-matching anti-pattern that made FIX-03 hard to find.

### Q4: Should the repair prompt also skip when `total_issues > 0` but `auto_fixable_count == 0`?

| Option | Description | Selected |
|--------|-------------|----------|
| Skip prompt entirely; list interactive issues with their per-issue prompts only | When zero auto-fixable, no global prompt. Interactive issues still get their own prompts. The literal #530 fix. | ✓ |
| Always show the prompt with `0 auto-fixable` count for consistency | Cleaner code path but mildly confusing UX. | |

**User's choice:** Skip prompt entirely; list interactive issues with their per-issue prompts only
**Notes:** Closes #530 cleanly. No `(no auto-repair available)` follow-up to a non-zero count.

---

## last_sync semantics (OBS-07)

### Q5: What should `last_sync` mean in `tome status`?

| Option | Description | Selected |
|--------|-------------|----------|
| Explicit `last_synced_at` header field in manifest, updated every sync | Additive schema change. `Option<String>` deserializes `None` for pre-v0.11 manifests. Matches user mental model. | ✓ |
| Max of `manifest.synced_at` across entries | Pure derivation, zero schema change. But no-op syncs don't update. | |
| Manifest file mtime | Simplest, no schema change. Risk: any tool rewriting manifest updates mtime. | |

**User's choice:** Explicit `last_synced_at` header field in manifest, updated every sync
**Notes:** Additive `Option<String>` field — pre-v0.11 manifests display `never`. No migration tooling needed.

### Q6: What happens for partial / failed syncs?

| Option | Description | Selected |
|--------|-------------|----------|
| Update last_synced_at only on full successful sync (after cleanup) | Mid-sync panic leaves previous value. Honest reporting. | ✓ |
| Update on any sync invocation, even with errors | Reflects "last attempt" rather than "last successful sync". Risk: misleading after broken syncs. | |

**User's choice:** Update last_synced_at only on full successful sync (after cleanup)
**Notes:** Stamped as the final step of `sync()`, after distribute + cleanup return Ok.

---

## FIX-02 timing flake approach

### Q7: How should the `copy_path_retry_helper` timing flake be fixed?

| Option | Description | Selected |
|--------|-------------|----------|
| Relaxed bound + explicit comment naming the root cause | Bump 600ms → ~2000ms with `// SAFETY:` comment naming arboard contention. ~5 LOC, no architecture change. | ✓ |
| Deterministic clock injection (`trait Clock` in `browse::app`) | Bulletproof determinism but new abstraction for a single test. Likely overkill for v0.11 polish scope. | |
| Serialize the test (`#[serial]` or own test module) | Cheap but introduces new dev-dep. Doesn't help if scheduler is just slow. | |

**User's choice:** Relaxed bound + explicit comment naming the root cause
**Notes:** ROADMAP explicitly permits this approach. Polish-phase scope discipline favors the minimal-change route.

### Q8: Does the same approach also resolve HARD-14 `backup::push_and_pull_roundtrip`?

| Option | Description | Selected |
|--------|-------------|----------|
| Fold into FIX-02 — same root-cause class, same approach | Single decision applied twice. PROJECT.md milestone description bundles them. | ✓ |
| Treat HARD-14 as separate — different root cause needs different fix | Keep FIX-02 narrow to #511; address HARD-14 as its own task within the phase. | |

**User's choice:** Fold into FIX-02 — same root-cause class, same approach
**Notes:** If investigation reveals a different root cause during planning, this decision re-opens.

---

## Defaults confirmed (single multi-select)

### Q9: Anything else? A few items have obvious technical answers I'd resolve during planning without further input — confirm or override.

| Option | Description | Selected |
|--------|-------------|----------|
| FIX-03 stale 'tracked in git' check — DELETE entirely (not rewrite) | v0.10 made managed skills real directory copies; check is obsolete. | ✓ |
| FIX-04 ANSI width — use `strip-ansi-escapes` crate | Standard Rust idiom. Add as regular dep. | ✓ |
| FIX-06 CHANGELOG date-stamp — inline `sed` in Makefile recipe | Matches existing `sed -i ''` line for Cargo.toml version bump. | ✓ |
| OBS-06 'auto-fixable' count is now category-aware in summary line | Surface `(N auto-fixable across Library/Foreign-symlink)` style breakdown when count > 0. | ✓ |

**User's choice:** All four defaults confirmed
**Notes:** No overrides — researcher and planner proceed with these as locked decisions.

---

## Claude's Discretion

(Items where user said "you decide" or deferred to Claude — see CONTEXT.md `<decisions>` section "Claude's Discretion" for the full list. Highlights:)

- `RepairKind` enum specific variants — derive from inventory of actual auto-repair handlers in current `doctor.rs`.
- `IssueCategory` enum serialization format — researcher chooses snake_case vs PascalCase for JSON (recommendation: snake_case).
- Manifest-header field placement (`last_synced_at` at top of Manifest struct vs new `Header` struct).
- Exact text rendering of Directories table (column widths, separator style).
- Test-count target — organic growth past ≥1000 verified by planner.

## Deferred Ideas

(See CONTEXT.md `<deferred>` section. Summary:)

- Deterministic clock injection for `browse::app` — rejected for v0.11 polish; future phase if flakes recur.
- Replacement check for the deleted "tracked in git" warning — new ticket if a real failure mode emerges.
- cargo-dist hook for CHANGELOG date-stamping — migration candidate if cargo-dist exposes a clean release-time hook.
- JSON `auto_fixable_by_category` map — planner judgement during plan-phase.
