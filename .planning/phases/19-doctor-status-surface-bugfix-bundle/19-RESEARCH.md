# Phase 19: Doctor/status surface + bugfix bundle — Research

**Researched:** 2026-05-13
**Domain:** Rust CLI diagnostic surfaces + targeted bugfix bundle (tracing-substrate post-Phase 18)
**Confidence:** HIGH (every recommendation is anchored to code grep + existing tests; one verified anomaly flagged in Open Questions)

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions

**OBS-06 — Doctor categorization model**

- **D-CAT-1 (Derive category from field structure + promote ForeignSymlink):** Add a new `IssueCategory { Library, Directory, Config, ForeignSymlink }` enum following POLISH-04 (`ALL: [IssueCategory; 4]` array + compile-time exhaustiveness sentinel). Add a `category: IssueCategory` field on `DiagnosticIssue`, computed at construction from `(which DoctorReport field, kind)`: Library/Directory/Config come from the field the issue lives in; if `kind == DiagnosticIssueKind::ForeignSymlink`, the category is promoted to `ForeignSymlink` (overrides the parent field). JSON shape gains `category` as a serialized string per issue.
- **D-CAT-2 (ForeignSymlink mutually exclusive in summary):** Each `DiagnosticIssue` belongs to exactly one category. ForeignSymlink issues count *only* in the ForeignSymlink bucket — not also under Library/Directory. Summary-line counts add up to `report.total_issues()`. The summary serializer must verify this invariant in tests (sum-of-category-counts == total_issues).
- **D-CAT-3 (Category-aware auto-fixable breakdown in summary):** When `auto_fixable_count > 0`, the summary line includes a per-category breakdown — e.g. `(N auto-fixable: Library M, Foreign-symlink K)` — surfacing which categories have auto-repair paths. Only categories with non-zero auto-fixable counts appear in the breakdown.

**FIX-01 — Auto-fixable definition (closes #530)**

- **D-REPAIR-1 (Typed `RepairKind` enum + ALL sentinel):** Introduce `RepairKind { RemoveBrokenSymlink, RemoveStaleManifestEntry, ... }` (specific variants TBD by researcher/planner — one per real auto-repair handler in `doctor.rs`). Follows POLISH-04 (`ALL` array + compile-time exhaustiveness sentinel). Add `repair_kind: Option<RepairKind>` field on `DiagnosticIssue`. `Some(kind)` ↔ auto-fixable; the global repair dispatcher matches on `RepairKind` so adding a variant without a handler fails to compile.
- **D-REPAIR-2 (Skip global prompt at zero):** When `auto_fixable_count == 0`, the global `Apply N auto-fixable repairs? [Y/n]` prompt is skipped entirely. Interactive issues (orphan directories, etc.) still receive their existing per-item prompts. This is the literal #530 fix — no `(no auto-repair available)` follow-up to a non-zero count.
- **D-REPAIR-3 (Substring matching removed):** Existing message-substring matching (`i.message.contains("orphan directory") || i.message.contains("tracked in git")`) at `doctor.rs:267-285` is replaced by `repair_kind`-based discrimination. Substring matching is anti-pattern and brittle to message-wording changes (this is what made FIX-03's stale check hard to find).

**OBS-07 — Status richer surface**

- **D-LSYNC-1 (Explicit header field):** Add `last_synced_at: Option<String>` to the **manifest header** (NOT per-entry — separate concept from `synced_at`). Type is `Option<String>` for additive-schema compatibility: pre-v0.11 manifests deserialize the field as `None`, no migration required. Format: RFC-3339 (`now_iso8601()`).
- **D-LSYNC-2 (`never` rendering):** `tome status` text output prints `Last sync: never` when manifest doesn't exist OR `last_synced_at` is `None`. JSON shape: `last_sync: Option<String>` — `null` for never, RFC-3339 string otherwise.
- **D-LSYNC-3 (Full successful sync only):** `last_synced_at` is stamped as the final step of `sync()`, after distribute + cleanup succeed. Mid-sync panic or aborted reconcile leaves the previous value unchanged. Honest reporting: `Last sync: <ts>` always reflects a sync that completed cleanly through the cleanup phase.
- **D-DIR-1 (Per-directory skill count in text):** `DirectoryStatus.skill_count` already exists in the JSON shape — Phase 19 surfaces it in the text rendering of the Directories section. Existing `(override)` annotation from PORT-05 is preserved. Column order: `name | type | role | skill_count | path` (or similar — researcher decides exact rendering; planner pins it).

**FIX-02 — Timing flake (closes #511 + HARD-14)**

- **D-FLAKE-1 (Relaxed bound + root-cause comment):** Bump `copy_path_retry_helper_returns_within_bound` upper bound from 600ms to ~2000ms (researcher confirms exact value via local measurement). Add `// SAFETY:` comment explaining the assertion is a regression guard against actual hangs, not a perf gate, and naming `arboard`/parallel-test contention as the root cause. ROADMAP explicitly permits this approach. ~5 LOC change.
- **D-FLAKE-2 (HARD-14 same treatment):** Apply identical pattern (relaxed bound + named-root-cause comment) to `backup::tests::push_and_pull_roundtrip` since the milestone description bundles both flakes together. If investigation reveals a different root cause class, planner re-opens this decision.
- **D-FLAKE-3 (Out of scope: clock injection):** Deterministic clock injection (introducing `trait Clock` across `browse::app`) is explicitly rejected for v0.11 polish scope. If the relaxed bound flakes again post-fix, the abstraction can be introduced in a future phase.

**FIX-03 — Stale "tracked in git" check (closes #532)**

- **D-FIX03-1 (Delete entirely):** Remove the `"N managed symlink(s) tracked in git"` check (currently at `crates/tome/src/doctor.rs:665` and its render path at `:383-394`) wholesale. v0.10 made managed skills real directory copies; the check's original concern (machine-specific symlinks in git) no longer applies. No replacement check is added — if a real failure mode emerges, it will get its own ticket.
- **D-FIX03-2 (Regression test):** New integration test asserts that a clean v0.10-shape library produces zero "tracked in git" warnings from `tome doctor`. The test fixture is a fresh real-directory-copy library.

**FIX-04 — ANSI width in wizard summary (closes #454)**

- **D-FIX04-1 (`strip-ansi-escapes` crate):** Add `strip-ansi-escapes` as a regular dep (not dev-dep — runtime path). Strip ANSI escapes before passing strings to `tabled`'s width measurement. Apply to the wizard summary table's `Width::increase`/`Width::truncate` cell handling.
- **D-FIX04-2 (Snapshot test):** New snapshot test renders a styled summary table (`console::style(...).bold()` cells) and asserts column alignment is correct under ANSI-aware width.

**FIX-05 — Wizard library default (closes #453 + #456)**

- **D-FIX05-1 (Library default tracks tome_home):** `wizard::configure_library` proposes `<resolved_tome_home>/skills` as the library default, NOT hardcoded `~/.tome/skills`. The library-default derivation must use the resolved `tome_home` value (after tilde expansion and any `TOME_HOME` env-var override). Verified by wizard integration test driving a custom `tome_home` in `--no-input` mode.
- **D-FIX05-2 (No fallback chain):** When `tome_home` is set, library default is unconditionally `<tome_home>/skills`. No fallback to `~/.tome/skills` if that path doesn't exist. The wizard's existing path-creation flow handles the missing-directory case.

**FIX-06 — `make release` CHANGELOG date-stamp (closes #533)**

- **D-FIX06-1 (Inline `sed` in Makefile recipe):** Add a single `sed -i ''` line to the existing `make release` recipe (`Makefile:14-32`) between the `cargo check` step and the branch-creation step. Replaces `## [Unreleased]` with `## [$$SEMVER] - $$(date -u +%Y-%m-%d)` in `CHANGELOG.md`. Style matches the existing `sed -i '' "s/^version = ..."` line.
- **D-FIX06-2 (Idempotency / safety):** If `CHANGELOG.md` lacks an `[Unreleased]` section, `sed` is a no-op — release proceeds without the changelog edit.
- **D-FIX06-3 (Test):** Script-level test (or a documented `--dry-run` smoke) shows the substitution against a fixture changelog. No GitHub-API mock needed.

### Claude's Discretion

- **`RepairKind` enum specific variants** — derive from inventory of actual auto-repair handlers in current `doctor.rs`. Each handler = one variant. **(Resolved below.)**
- **`IssueCategory` enum serialization format** — researcher chooses between `"library"` (snake_case) vs `"Library"` (PascalCase) for JSON. Recommendation: snake_case to match existing JSON conventions (`override_applied`, `skill_count`). **(Resolved below.)**
- **Manifest-header field placement** — researcher decides whether `last_synced_at` lives at the top of `Manifest` struct or inside a new `Header` struct. Either is fine. **(Resolved below.)**
- **Exact text rendering of Directories table** — column widths, separator style, whether `skill_count` appears as `5` or `5 skills` — planner pins it after researcher prototypes. **(Resolved below.)**
- **Test-count target** — ROADMAP targets ≥1000 tests at v0.11 ship time (was 987 at v0.10.0, currently 808+ unit + CLI suites after Phase 18). **(Resolved below — actual count is 994; we cross 1000 organically.)**

### Deferred Ideas (OUT OF SCOPE)

- **Deterministic clock injection (`trait Clock` in `browse::app`)** — Rejected for v0.11 polish scope (D-FLAKE-3). Future phase if relaxed bound flakes again.
- **Replacement for the "tracked in git" check** — D-FIX03-1 deletes wholesale. If a real-world failure mode emerges, file a new ticket and address in a future phase.
- **cargo-dist hook for CHANGELOG date-stamping** — D-FIX06-1 picks inline `sed` for style consistency.
- **JSON `auto_fixable_count` breakdown in OBS-06 summary object** — D-CAT-3 specifies category-aware breakdown in *text* output. Whether to also include an `auto_fixable_by_category` map in JSON is left to planner judgement.
- **Test-count budgeting beyond ≥1000** — Opportunistic only, not scope-creep.

</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| OBS-06 | `tome doctor` richer surface — categorize issues (Library / Directory / Config / Foreign-symlink) with per-category counts in text; JSON gains `category` field | RepairKind Variant Inventory + IssueCategory Decisions sections below |
| OBS-07 | `tome status` richer surface — per-directory skill counts (already in JSON, surface in text), `Last sync` timestamp; JSON shape parity | OBS-07 Rendering Specifics section below |
| FIX-01 | `tome doctor` auto-fixable count + prompt exclude items with no auto-repair (closes #530) | RepairKind Variant Inventory ties each `repair_kind: Some(...)` to a real handler arm |
| FIX-02 | Browse timing flake fix (closes #511 + HARD-14 carry-over) | FIX-02 specifics — 2000ms bound rationale, root-cause comment template |
| FIX-03 | Remove stale "tracked in git" doctor check (closes #532) | FIX-03 specifics — exact code anchors `doctor.rs:665-682` (check) + `:383-448` (interactive render) + `:687-730` (helper to delete) |
| FIX-04 | Wizard summary ANSI width misalignment (closes #454) | FIX-04 specifics — **anomaly flagged**: `tabled = { features = ["ansi"] }` already enabled (commit 0803afb, April 2026); #454 still OPEN — investigate before applying D-FIX04-1 verbatim |
| FIX-05 | Wizard library default tracks `tome_home` (closes #453 + #456) | FIX-05 specifics — **already implemented at `wizard.rs:637`**; gap is a pinning integration test |
| FIX-06 | `make release` stamps CHANGELOG date (closes #533) | FIX-06 specifics — exact `sed` line + Makefile insertion point |

</phase_requirements>

## Summary

Phase 19 is the v0.11 finalization phase: a richer `tome doctor`/`tome status` surface (`IssueCategory` + `RepairKind` typed enums, last-sync timestamp, per-directory skill counts in text) plus six independent bugfixes. CONTEXT.md is the contract — every implementation decision is locked. Research only filled the explicit Claude's Discretion gaps and audited code anchors.

Major findings:

1. **The `RepairKind` inventory is small and well-bounded.** `doctor.rs` currently has exactly **three** real auto-repair handlers (broken-symlink removal, stale-manifest-entry removal, stale-target-symlink removal) plus one interactive-only path (orphan directories). Recommendation: **3-variant enum** (`RemoveBrokenLibrarySymlink`, `RemoveStaleManifestEntry`, `RemoveStaleTargetSymlink`); orphan-directory handling stays interactive-only (no `repair_kind` field). The git-tracked-symlinks handler at `doctor.rs:383-448` is deleted entirely by FIX-03 — so it does NOT become a `RepairKind` variant.
2. **Test count is already 994.** ROADMAP targets ≥1000 at v0.11 ship. Adding the regression tests required by D-CAT-2 invariant + D-FIX03-2 + D-FIX04-2 + D-FIX05-1 + D-FIX06-3 + OBS-07 last-sync round-trip easily clears 1000 (estimated +12 to +18 tests). No special "test-count growth" plan needed; the natural test-per-FIX-item discipline carries the count over.
3. **Two FIX items have already-landed partial fixes that the planner must reconcile.** FIX-04 (#454) — commit `0803afb` (April 2026) added `tabled = { features = ["ansi"] }` with a detailed root-cause analysis matching the issue verbatim, yet GitHub #454 remains OPEN. Either the fix was insufficient on some code path, or the issue was never closed administratively. FIX-05 (#453+#456) — `wizard.rs:637` already derives `<tome_home>/skills` correctly; the gap is the lack of an integration test pinning the behavior. The planner needs to audit both before committing to fresh code changes.
4. **`last_synced_at` placement: top-level `Manifest` field, not a new `Header` struct.** Lower blast radius (existing `Manifest` is `BTreeMap<SkillName, SkillEntry>`-shaped; adding a sibling field via a private-fields refactor or a thin wrapper preserves the v0.10 schema). A new `Header` struct would re-shape JSON and break the existing serialization. Recommendation justified in OBS-07 Rendering Specifics.
5. **FIX-06 must land in an early wave.** Phase 18 wrote v0.11 work under `[Unreleased]`. If the v0.11 release cut runs `make release` BEFORE FIX-06 lands, the CHANGELOG won't auto-stamp. Recommend Wave 1.

**Primary recommendation:** Plan with three waves: (W1) `RepairKind` + `IssueCategory` substrate (single plan, anchors OBS-06 + FIX-01 + FIX-03 since they all touch `doctor.rs`); (W1 parallel) FIX-06 Makefile; (W2) OBS-07 last-sync + skill-count rendering in `status.rs` + `manifest.rs` + `lib.rs::sync`; (W2 parallel) FIX-02 + FIX-04 + FIX-05 (each a small standalone plan). Then verification + CHANGELOG entry.

## RepairKind Variant Inventory

The CONTEXT.md "Claude's Discretion" point 1 requires inventorying actual auto-repair handlers in current `doctor.rs`. Each handler = one variant.

### Code-anchored handler inventory

Reading `doctor.rs:267-453` (the interactive flow in `diagnose()`) plus `render_repair_plan_auto` (`:459-497`) plus `repair_library` (`:822-882`):

| Handler | Code location | Trigger predicate (current substring) | Action | Becomes RepairKind |
|---------|---------------|---------------------------------------|--------|--------------------|
| **Stale manifest entry — no directory on disk** | `repair_library` :834-854 (loops `missing`) | `i.message.contains("no directory on disk")` → emit issue at `:621-626` | `m.remove(name)` + (if symlink) `std::fs::remove_file` | `RemoveStaleManifestEntry` |
| **Broken managed symlink** | `repair_library` :840-846 (same loop, handles symlink branch) + `:599-605` issue emit | Issue: "managed skill 'X' has a broken symlink (source may have been uninstalled)" at `:614-619` | `std::fs::remove_file` + `m.remove(name)` | `RemoveBrokenLibrarySymlink` |
| **Broken legacy symlink (not in manifest)** | `repair_library` :856-875 (read_dir loop) + `:647-661` issue emit | `is_symlink() && !path.exists()` & not managed → emit "broken legacy symlink: X -> Y" | `std::fs::remove_file` | (Folds into `RemoveBrokenLibrarySymlink` — same action; cause differs but action is identical) |
| **Stale target symlink (in distribution dir)** | Triggered inline at `:297-307` via `cleanup::cleanup_target` after auto-confirm | `i.message.contains("stale symlink")` → `cleanup::cleanup_target` removes broken symlinks pointing into library | `cleanup::cleanup_target` | `RemoveStaleTargetSymlink` |
| Orphan directory in library | `:312-381` interactive Select with keep/delete/skip | `i.message.contains("orphan directory")` | User-decided per-item; not auto-fixable | **NONE** (interactive-only) |
| Managed symlinks tracked in git | `:383-448` interactive Confirm + `git rm --cached` | `i.message.contains("managed symlink(s) tracked in git")` | **Deleted entirely by FIX-03 D-FIX03-1** | **NONE** (removed in this phase) |

**Inventory result: three auto-repair RepairKind variants.**

### Recommended enum

```rust
/// Categorizes the auto-repair available for a `DiagnosticIssue`.
///
/// `Some(kind)` on `DiagnosticIssue::repair_kind` ↔ the issue is auto-fixable
/// and the global repair dispatcher has a handler arm for `kind`. `None`
/// means the issue requires user interaction (e.g. orphan directories,
/// which use a per-item Select prompt) or is informational only.
///
/// Per POLISH-04: `ALL` array + compile-time exhaustiveness sentinel keep
/// every variant pinned. Adding a variant without updating `ALL` is a
/// compile error; adding a variant without a dispatcher handler arm is also
/// a compile error (the dispatcher exhausts the enum).
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RepairKind {
    /// Remove a manifest entry whose library directory is missing on
    /// disk (emit site: `check_library` "manifest entry 'X' has no
    /// directory on disk" + "managed skill 'X' has a broken symlink").
    /// Action: `Manifest::remove(name)` + (if symlink) `remove_file`.
    RemoveStaleManifestEntry,
    /// Remove a broken legacy symlink in the library directory (not
    /// referenced by manifest). Emit site: `check_library` "broken
    /// legacy symlink: X -> Y". Action: `remove_file(path)`.
    RemoveBrokenLibrarySymlink,
    /// Remove a stale symlink from a distribution directory (target's
    /// library entry was removed). Emit site: `check_distribution_dir`
    /// "stale symlink X". Action: `cleanup::cleanup_target` removes it.
    RemoveStaleTargetSymlink,
}

impl RepairKind {
    pub const ALL: [Self; 3] = [
        Self::RemoveStaleManifestEntry,
        Self::RemoveBrokenLibrarySymlink,
        Self::RemoveStaleTargetSymlink,
    ];
}

#[allow(dead_code)]
const fn _repair_kind_exhaustiveness_sentinel(k: RepairKind) {
    match k {
        RepairKind::RemoveStaleManifestEntry => {}
        RepairKind::RemoveBrokenLibrarySymlink => {}
        RepairKind::RemoveStaleTargetSymlink => {}
    }
}
const _: () = {
    assert!(RepairKind::ALL.len() == 3);
};
```

**JSON serialization** (snake_case, matches existing `override_applied`/`skill_count`/`source_path` conventions in the codebase): `"remove_stale_manifest_entry"`, `"remove_broken_library_symlink"`, `"remove_stale_target_symlink"`.

### Dispatcher shape

The current substring-matching at `doctor.rs:267-285` is replaced by a typed dispatch. Recommended shape (in `render_repair_plan_auto` and `repair_library`):

```rust
// Issue construction at emit site (in `check_library`):
DiagnosticIssue {
    severity: IssueSeverity::Error,
    message: format!("manifest entry '{}' has no directory on disk", name),
    kind: None,
    repair_kind: Some(RepairKind::RemoveStaleManifestEntry),
    category: IssueCategory::Library,
}

// Dispatcher match (replaces line :283 `auto_fixable` substring math):
fn auto_fixable_count(report: &DoctorReport) -> usize {
    report
        .all_issues()
        .filter(|i| i.repair_kind.is_some())
        .count()
}

// Repair execution (replaces line :283-309 chain):
for issue in report.all_issues() {
    match issue.repair_kind {
        Some(RepairKind::RemoveStaleManifestEntry) => repair_stale_manifest_entry(...)?,
        Some(RepairKind::RemoveBrokenLibrarySymlink) => repair_broken_symlink(...)?,
        Some(RepairKind::RemoveStaleTargetSymlink) => repair_stale_target_symlink(...)?,
        None => continue,  // interactive (orphan dirs) or informational
    }
}
```

`all_issues()` is a recommended convenience method on `DoctorReport` that flattens `library_issues + directory_issues[*].issues + config_issues`. The dispatcher must `match` exhaustively on `Option<RepairKind>` so a future variant added without a handler fails to compile.

### Note on issue-emit-site changes

Currently each emit site builds with `DiagnosticIssue::untyped(...)` or `DiagnosticIssue::typed(...)`. To carry `repair_kind`, the planner needs ONE of:

1. **Extend constructors** — `DiagnosticIssue::repairable(severity, message, repair_kind)` + retrofit `category` derivation onto every emit site (recommended — symmetric with existing `untyped`/`typed`).
2. **Builder pattern** — `DiagnosticIssue::new(severity, message).with_repair_kind(...)` etc. More flexible but breaks existing call-site shape.

**Recommendation: option 1.** It keeps emit-site brevity and matches the existing pattern. Approximately 8 emit sites need updating (in `check_library`, `check_distribution_dir`, `check_config`).

## IssueCategory Decisions

### Serialization format: snake_case (recommended)

Per CONTEXT.md gap-2 explicit recommendation. Matches existing JSON conventions in the codebase verified by grep:
- `crates/tome/src/doctor.rs:122` — `override_applied: bool`
- `crates/tome/src/status.rs:45` — `skill_count: CountOrError`
- `crates/tome/src/manifest.rs:117` — `source_name`, `previous_source`, `source_path`, `content_hash`, `synced_at`

No existing field uses PascalCase in serialized output. Snake_case is unambiguously the project convention.

Recommended attribute:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum IssueCategory {
    Library,
    Directory,
    Config,
    ForeignSymlink,
}
```

Wire-form: `"library"`, `"directory"`, `"config"`, `"foreign_symlink"`.

### Derivation logic for ForeignSymlink promotion

Per D-CAT-1, category is computed at construction. Logic:

```rust
fn category_for(field: DoctorField, kind: Option<DiagnosticIssueKind>) -> IssueCategory {
    if matches!(kind, Some(DiagnosticIssueKind::ForeignSymlink)) {
        IssueCategory::ForeignSymlink  // promotes regardless of source field
    } else {
        match field {
            DoctorField::Library => IssueCategory::Library,
            DoctorField::Directory => IssueCategory::Directory,
            DoctorField::Config => IssueCategory::Config,
        }
    }
}
```

The current ForeignSymlink emit site is `check_distribution_dir` (which feeds `DirectoryDiagnostic.issues`) — so without promotion, it would land under `Directory`. D-CAT-1 promotes it out. **D-CAT-2 invariant test:** sum-of-per-category-counts == `report.total_issues()`. The recommended unit test:

```rust
#[test]
fn category_counts_sum_to_total_issues_invariant() {
    // Synth a DoctorReport with one issue per category, including
    // a ForeignSymlink (kind=Some(ForeignSymlink)) that lives in
    // directory_issues. Assert the four per-category counts add to
    // report.total_issues() — and that the ForeignSymlink issue
    // counts ONLY in the ForeignSymlink bucket.
    ...
}
```

### POLISH-04 sentinel for IssueCategory

```rust
impl IssueCategory {
    pub const ALL: [Self; 4] = [
        Self::Library,
        Self::Directory,
        Self::Config,
        Self::ForeignSymlink,
    ];
}

#[allow(dead_code)]
const fn _issue_category_exhaustiveness_sentinel(c: IssueCategory) {
    match c {
        IssueCategory::Library => {}
        IssueCategory::Directory => {}
        IssueCategory::Config => {}
        IssueCategory::ForeignSymlink => {}
    }
}
const _: () = { assert!(IssueCategory::ALL.len() == 4); };
```

Template: `doctor.rs:60` `_diagnostic_issue_kind_exhaustiveness_sentinel` (one-variant version of the same pattern).

## OBS-07 Rendering Specifics

### Manifest header placement: top-level `Manifest` field (recommended)

CONTEXT.md gap-3 leaves this open. Reading the current `Manifest` struct at `manifest.rs:22-26`:

```rust
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Manifest {
    skills: BTreeMap<SkillName, SkillEntry>,
}
```

Two options:

**Option A — Top-level field (RECOMMENDED, lower blast radius):**

```rust
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Manifest {
    skills: BTreeMap<SkillName, SkillEntry>,
    /// Timestamp of last successful `tome sync` completion (post-cleanup).
    /// Stamped by `sync()` after distribute + cleanup succeed (D-LSYNC-3).
    /// `None` for pre-v0.11 manifests; renders as "never" in `tome status`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    last_synced_at: Option<String>,
}
```

JSON shape (additive — pre-v0.11 manifests still parse):

```json
{
  "skills": { ... },
  "last_synced_at": "2026-05-13T10:30:00Z"
}
```

**Option B — New Header struct:** Requires a wrapper around `Manifest`, breaks all 50+ `manifest.iter()`/`manifest.get()`/`manifest.insert()` call sites, and reshapes JSON. Blast radius is high. Skip.

**Recommendation: Option A.** Add a `pub fn last_synced_at(&self) -> Option<&str>` accessor and a `pub(crate) fn stamp_last_synced_at(&mut self)` method that calls `now_iso8601()`. Keeps `skills` private (matches existing visibility); avoids exposing the field directly.

### Stamp point in `sync()`

Per D-LSYNC-3, stamp after distribute + cleanup succeed. Reading `lib.rs:1779-1789` (the existing manifest save):

```rust
// 7. Save manifest, gitignore, and lockfile
if !dry_run && paths.config_dir().is_dir() {
    manifest::save(&manifest, paths.config_dir())?;  // <-- existing save
}
```

The stamp lands **immediately before this save**:

```rust
// 7. Save manifest, gitignore, and lockfile
if !dry_run && paths.config_dir().is_dir() {
    manifest.stamp_last_synced_at();  // <-- new line (D-LSYNC-3)
    manifest::save(&manifest, paths.config_dir())?;
}
```

This places the stamp AFTER:
- cleanup_library (line 1694) ✓
- reconcile (line 1535) ✓
- discover (line 1584) ✓
- consolidate (line 1641) ✓
- distribute (line 1710) ✓
- target cleanup (line 1755-1777) ✓

And BEFORE:
- lockfile.save (line 1788)
- doctor::check post-sync health check (line 1829)

A mid-sync panic or early-return (e.g. `bail!` on distribution failure) bypasses the stamp — the previous `last_synced_at` value remains untouched. This matches D-LSYNC-3's "Honest reporting" contract.

**Edge case — `dry_run` mode:** The stamp is gated on `!dry_run` (it's inside the existing `if !dry_run` block). Dry-run syncs do NOT update `last_synced_at`. This is correct: a dry-run didn't actually sync anything.

**Edge case — partial reconcile failures:** The `reconcile_install_failures.is_empty()` bail at line 1878-1884 fires AFTER the manifest save. By the time we reach `bail!`, `last_synced_at` is already stamped. This is acceptable per D-LSYNC-3 wording: cleanup completed (target cleanup at line 1755 ran), distribute ran successfully (we didn't bail there), so the user-facing semantics are "the sync completed through cleanup; the install-failure exit-code is a downstream concern." If the planner prefers stricter semantics, the stamp can move to AFTER the two bail guards — but D-LSYNC-3's wording "after distribute + cleanup succeed" puts it where the current save is.

### Text rendering of Directories table

Current `render_status` (`status.rs:204-300`) renders the Directories section as a 4-column `tabled::Table`: NAME / TYPE / ROLE / PATH (where PATH already includes the styled `(override)` annotation when applicable). Per D-DIR-1, surface `skill_count`.

**Prototype rendering recommendation: 5 columns, NAME / TYPE / ROLE / PATH / SKILLS, count rendered as `✓ N` or `✗ ?` (matching the existing CountOrError pattern at status.rs:246-253).**

The existing JSON-side rendering already has this:

```rust
let count = match (&dir.skill_count.count, &dir.skill_count.error) {
    (Some(n), _) => format!("✓ {}", n),
    (None, Some(e)) => { eprintln!(...); "✗ ?".to_string() }
    (None, None) => "✓ 0".to_string(),
};
```

So the text-mode work is just **adding the count to the row vector and the header row**. Concrete patch shape:

```rust
let mut rows: Vec<[String; 5]> = Vec::with_capacity(report.directories.len() + 1);
rows.push([
    "NAME".to_string(),
    "TYPE".to_string(),
    "ROLE".to_string(),
    "PATH".to_string(),
    "SKILLS".to_string(),
]);
for dir in &report.directories {
    let count = /* existing CountOrError match */;
    rows.push([
        dir.name.clone(),
        dir.directory_type.clone(),
        dir.role.clone(),
        format_dir_path_column(&dir.path, dir.override_applied),
        count,
    ]);
}
```

**Column-width policy:** No `Width::*` setting added — same `Style::blank()` + header-bold pattern as today. tabled handles column widths automatically. Empirically the existing 4-column table fits well in 80 cols and an extra short SKILLS column adds ≤6 cols (worst case `✓ 99`).

**`SKILLS` vs `SKILL COUNT` vs `5 skills`:** Recommendation: bare `SKILLS` header with the `✓ N` / `✗ ?` glyph-prefixed body (consistency with the Library line's `✓ 5 skills consolidated`). Single header word matches the brevity of the other column headers (NAME / TYPE / ROLE / PATH).

### `Last sync` line rendering

Per D-LSYNC-2, top of `render_status` after `Library:` block:

```rust
// Library line stays the same...
println!("{} {}", style("Library:").bold(), collapse_home(...));
println!("  {} {} skills consolidated", lib_indicator, style(lib_count).cyan());

// NEW: Last sync line (D-LSYNC-2). Reads from manifest header.
let last_sync_str = match manifest_last_synced_at {
    Some(ts) => ts.clone(),
    None => "never".to_string(),
};
println!("  {} {}", style("Last sync:").bold(), style(last_sync_str).cyan());

println!();
// Directories block...
```

`tome status --json` shape gains a sibling field on `StatusReport`:

```rust
pub struct StatusReport {
    pub configured: bool,
    pub library_dir: PathBuf,
    pub library_count: CountOrError,
    /// RFC-3339 timestamp of last successful sync; `null` if never synced
    /// or pre-v0.11 manifest. Per D-LSYNC-2 ("never" in text; null in JSON).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_sync: Option<String>,
    pub directories: Vec<DirectoryStatus>,
    pub unowned: Vec<crate::summary::SkillSummary>,
    pub health: CountOrError,
}
```

The JSON serialization SHOULD use `#[serde(skip_serializing_if = "Option::is_none")]` to match existing patterns (e.g. `kind` in `DiagnosticIssue`). **OR** — alternative pragmatic choice per D-LSYNC-2 — emit `"last_sync": null` for stable-shape consumers. Recommend the **latter** (no skip) for `last_sync` specifically because the JSON contract pinned in CONTEXT.md is `Option<String>` — `null` for never, RFC-3339 string otherwise. Stable-shape parallels `unowned: []` always present at status.rs:946-970.

### Where to read `manifest_last_synced_at` in `gather()`

`status::gather()` already loads the manifest at line 123. Pipe the timestamp through:

```rust
let last_sync = match manifest::load(paths.config_dir()) {
    Ok(m) => m.last_synced_at().map(String::from),
    Err(_) => None,
};
```

Then thread into `StatusReport`. Pre-v0.11 manifests parse with `last_synced_at: None` per the additive-schema policy.

## FIX Item Specifics

### FIX-02 (timing flake — closes #511 + HARD-14)

**Code anchor:** `crates/tome/src/browse/app.rs:1782-1810` (`copy_path_retry_helper_returns_within_bound`).

**Current bound:** `600ms`. Per D-FLAKE-1, bump to "~2000ms (researcher confirms exact value via local measurement)."

**Recommended bound: `2000ms`.** Rationale:

- The bug class is unbounded contention from `arboard::Clipboard::new()` (NSPasteboard, `WinClipboard`, X11 clipboard server) under `cargo test --test-threads=N`. Per the existing comment at `:1790-1795`: happy path 5-500ms, retry path 100-600ms.
- 600ms catches a SECOND-retry regression (which would add ~100ms) but does NOT tolerate slow-CI variance. The flake report shows 600-1500ms run-time on flaked iterations.
- 2000ms still catches a real regression (a `loop` over retries with 100ms backoffs would be unbounded; even a 5-retry regression hits ~700ms = inside 2000ms but well above 600ms; a 10-retry regression hits ~1100ms; an unbounded `loop` hits the test harness timeout). A SAFER value would be `3000ms` or `5000ms` if we want maximal flake suppression.
- Recommendation: **2000ms** to match the CONTEXT.md gap-5 hint and balance regression-catching with flake suppression.

**Empirical verification:** Running the test in isolation locally returns sub-200ms; running with `--test-threads=8` shows 50-300ms on M1 macOS in 10 consecutive runs (no flakes observed locally during research; cannot easily reproduce the under-load failure). The CI variance is what's flaking — local timing is insufficient to bound. Recommendation derives from the existing comment's empirical breakdown + CONTEXT.md's hint.

**Root-cause comment template** (per CONTEXT.md specifics):

```rust
// FLAKE-FIX (#511 / HARD-14): bound relaxed from 600ms to 2000ms.
// arboard clipboard contention under --test-threads=N can pause threads
// ≫ 600ms regardless of helper performance — NSPasteboard / X11 clipboard
// server / WinClipboard arbitration is opaque to user code. This assertion
// guards against actual hangs (an unbounded retry `loop`), NOT perf
// regressions. A 2000ms bound catches a 10×-retry regression while
// tolerating realistic parallel-test contention.
//
// Deterministic clock injection (trait Clock in browse::app) was
// considered but rejected for v0.11 scope (D-FLAKE-3). If this bound
// flakes again post-fix, the abstraction can be introduced.
```

**Regression test outline:** No new test needed; the existing test IS the regression test. The bound change IS the fix.

### FIX-02 sibling: `backup::tests::push_and_pull_roundtrip` (D-FLAKE-2)

**Code anchor:** `crates/tome/src/backup.rs:548-590`.

**Root cause:** Phase 15 HARD-14 work folded git-signing-disable into `setup_git_config(&repo_b)` (file line 548 in test fixture). Reading lines 553-590, the test does: init_test_repo → clone-bare → setup_git_config → write/snapshot/push → pull/assert. **No explicit timing assertions visible in the test.** The flake mechanism is therefore NOT a timing bound — likely git-subprocess transient errors (filesystem timestamp resolution, network/file lock contention from parallel TempDir tests).

**Verification required by planner:** Per D-FLAKE-2 conditional ("If investigation reveals a different root cause class, planner re-opens this decision"), this MAY need a different treatment from FIX-02. The test has no upper bound to relax. Recommendation: have the planner read `backup.rs:548-590` carefully and decide whether (a) the flake is in `git push`/`git pull` subprocesses and needs a retry wrapper, (b) the test is fundamentally fine and the flake is in CI env, or (c) some upstream tempfile race needs an explicit ordering guarantee.

**Flag for planner:** This is the most ambiguous FIX item. Recommend a dedicated planning task to reproduce the flake before committing to a fix shape. CONTEXT.md authorizes this re-opening per D-FLAKE-2.

**Regression test outline:** Same as FIX-02 — the existing test IS the regression test once the flake is addressed.

### FIX-03 (stale "tracked in git" check — closes #532)

**Code anchors to DELETE:**
- `doctor.rs:665-682` — the check in `check_library` that emits the issue (the `if git_dir.exists() || library_dir.parent().is_some_and(|p| p.join(".git").exists())` block).
- `doctor.rs:687-730` — the `tracked_managed_symlinks` helper function.
- `doctor.rs:383-448` — the interactive `has_git_tracked` block in `diagnose()` (substring-detection at `:387` + render+confirm at `:392-447`).

**Total ~80 lines deleted, plus any tests in the `tests` module that reference `tracked_managed_symlinks` (none found via grep — the function is currently exercised only via the integration path).**

**Recommended fix shape:** Delete the three code blocks. Verify the `git` module imports at the top of `doctor.rs` aren't orphaned (the `std::process::Command::new("git")` calls all live in the deleted blocks).

**Regression test (D-FIX03-2):** New integration test in `crates/tome/tests/cli_doctor.rs`:

```rust
#[test]
fn doctor_clean_v010_library_emits_no_tracked_in_git_warning() {
    // Fixture: fresh v0.10-shape library with one real-directory-copy
    // managed skill (i.e. NOT a symlink). Run `tome doctor` and assert
    // the stderr+stdout contains zero occurrences of "tracked in git".
    let tmp = TempDir::new().unwrap();
    let lib = tmp.path().join("library");
    fs::create_dir_all(lib.join("my-managed-skill")).unwrap();
    // ... seed manifest with managed: true entry ...
    let output = Command::cargo_bin("tome").unwrap()
        .args(["--tome-home", tmp.path().to_str().unwrap(), "doctor"])
        .output().unwrap();
    let combined = format!("{}{}",
        String::from_utf8_lossy(&output.stderr),
        String::from_utf8_lossy(&output.stdout));
    assert!(!combined.contains("tracked in git"),
        "v0.10-shape library must not emit stale 'tracked in git' warning: {combined}");
}
```

### FIX-04 (wizard summary ANSI width — closes #454)

**⚠ ANOMALY:** `tabled = { features = ["ansi"] }` is already enabled in `Cargo.toml:32`. Commit `0803afb` (Thu Apr 23 13:36:28 2026, before v0.7.0 ship) added this feature with a commit message describing the exact bug from #454 verbatim. Yet GitHub #454 is still OPEN (verified via `gh issue view 454` — `state: OPEN`, no closedAt).

**Possible causes:**
1. The fix was applied but the issue was never closed administratively (most likely).
2. The fix works for the main summary table (`show_directory_summary` at `wizard.rs:499-539`) but a DIFFERENT styled table in the wizard flow is still misaligned. Grep shows only one styled `tabled::Table` in wizard.rs (line 532).
3. The fix works in some terminals but not others (unlikely — tabled's `ansi` feature uses `ansi-str`/`ansitok`, which is broadly compatible).
4. The fix regressed in some later commit. Re-verifying current state: `cargo run -p tome -- init` against an existing config skips the summary table; harder to reproduce without a greenfield fixture.

**Recommendation for planner:**
1. **First, reproduce the bug.** Run `tome init` in a greenfield TempDir (no existing config) via `script(1)` to force-TTY. Capture the summary table. If it's still misaligned, proceed with D-FIX04-1.
2. **If the bug does NOT reproduce**, the issue is administrative. Close #454 with reference to commit `0803afb` and ship D-FIX04-2 (snapshot test) as a pinning measure. Do NOT add `strip-ansi-escapes` unnecessarily — it's redundant with `tabled[ansi]`.
3. **If the bug reproduces**, follow D-FIX04-1 — add `strip-ansi-escapes` as a regular dep, strip ANSI from cells BEFORE passing to `tabled::Table::from_iter`. The exact insertion point is `wizard.rs:514-521` (the row-construction loop). Strip via `strip_ansi_escapes::strip_str(s)` per the published API (returns `String` from `&str`).

**`strip-ansi-escapes` crate version:** Latest is **0.2.1** (verified via [crates.io search](https://crates.io/crates/strip-ansi-escapes)). API per [docs.rs](https://docs.rs/strip-ansi-escapes/latest/strip_ansi_escapes/):
- `fn strip(data: &[u8]) -> Vec<u8>`
- `fn strip_str(data: &str) -> String` (use this)

Add to `Cargo.toml` workspace deps:

```toml
strip-ansi-escapes = "0.2"
```

**Regression test (D-FIX04-2):** Snapshot test in `wizard.rs` `#[cfg(test)] mod tests`:

```rust
#[test]
fn show_directory_summary_aligns_header_with_body_under_ansi() {
    // Reproduce #454 conditions: ANSI bold on header (via console::style),
    // body rows with realistic NAME/TYPE/ROLE/PATH content. Capture stderr
    // via `console::set_colors_enabled(true)` + a captured writer.
    // Snapshot assertion: every `│` divider in the header row appears at
    // the same column index as in body rows.
    let mut dirs = BTreeMap::new();
    dirs.insert(DirectoryName::new("claude-skills").unwrap(),
        DirectoryConfig { /* ... */ });
    // Snapshot the rendered output and assert column alignment.
    // (insta::assert_snapshot! pattern.)
}
```

A bonus: this snapshot test would also catch a regression if someone removes `features = ["ansi"]` from `Cargo.toml`.

### FIX-05 (wizard library default — closes #453 + #456)

**Code anchor:** `wizard.rs:632-677` (`configure_library`).

**Current behavior (verified by reading line 637):**

```rust
let default = crate::paths::collapse_home_path(&tome_home.join("skills"));
```

The library default is ALREADY derived from `tome_home`. The function signature at line 632 takes `tome_home: &Path`, and the call chain (from `lib.rs::run_wizard` or similar) threads the resolved `tome_home` value. So **the implementation already satisfies D-FIX05-1**.

**Gap analysis:**
1. ✓ D-FIX05-1 implementation: ALREADY DONE.
2. ✗ Integration test pinning the behavior: MISSING. CONTEXT.md mandates "Verified by wizard integration test driving a custom `tome_home` in `--no-input` mode (e.g., `tome_home = ~/dev/coding-agent-files/.tome` → library default = `~/dev/coding-agent-files/.tome/skills`)."

**Recommended fix shape:** Add a wizard integration test in `crates/tome/tests/cli_init.rs` (or wherever wizard tests live):

```rust
#[test]
fn wizard_library_default_follows_custom_tome_home() {
    let tmp = TempDir::new().unwrap();
    let custom_tome_home = tmp.path().join("custom-tome");
    fs::create_dir_all(&custom_tome_home).unwrap();

    let output = Command::cargo_bin("tome").unwrap()
        .env("TOME_HOME", &custom_tome_home)
        .args(["init", "--dry-run", "--no-input"])
        .output().unwrap();

    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);
    let combined = format!("{stderr}{stdout}");
    let expected = format!("{}/skills", custom_tome_home.display());
    assert!(combined.contains(&expected),
        "library default must follow TOME_HOME ({expected}): {combined}");
    // Also assert it does NOT contain the hardcoded fallback:
    assert!(!combined.contains("~/.tome/skills"),
        "wizard must not fall back to ~/.tome/skills when TOME_HOME is set: {combined}");
}
```

**Caveat:** If `tome init` in `--no-input` mode against a TOME_HOME with existing config short-circuits to the "Existing config detected" branch (as observed in the smoke run during research), the test fixture must NOT seed any config — pure greenfield. The wizard's library-default selection only appears in the greenfield path.

**Verify D-FIX05-2 (no fallback chain):** Read lines 632-677 — no fallback to `~/.tome/skills`. The path-creation flow at `wizard.rs:430-460` (handles missing dir) is orthogonal. ✓.

### FIX-06 (`make release` CHANGELOG date-stamp — closes #533)

**Code anchor:** `Makefile:14-32` (the `release` recipe).

**Exact insertion point** — between line 19 (`cargo check --quiet;`) and line 20 (`BRANCH="chore/release-$$TAG";`). Add one line:

```makefile
sed -i '' "s/^## \[Unreleased\]/## [$$SEMVER] - $$(date -u +%Y-%m-%d)/" CHANGELOG.md; \
```

And update line 23 (`git add Cargo.toml Cargo.lock;`) to also include CHANGELOG.md:

```makefile
git add Cargo.toml Cargo.lock CHANGELOG.md; \
```

**Idempotency per D-FIX06-2:** GNU `sed` is silently a no-op if the pattern doesn't match — desired behavior. BSD `sed` (macOS default) behaves the same way. No-pattern-found does NOT return non-zero exit. **Verification: BSD `sed -i '' "s/foo/bar/" /tmp/test`** on a file without `foo` exits 0. ✓.

**Add a Makefile comment per D-FIX06-2:**

```makefile
# Stamp the release date in CHANGELOG.md by replacing [Unreleased] with [VERSION] - DATE.
# Idempotent: if CHANGELOG.md lacks an [Unreleased] section, sed is a no-op and the
# release proceeds without the changelog edit. Style matches the Cargo.toml version-bump
# sed line above.
sed -i '' "s/^## \[Unreleased\]/## [$$SEMVER] - $$(date -u +%Y-%m-%d)/" CHANGELOG.md; \
```

**Date format `YYYY-MM-DD`:** Matches existing CHANGELOG entries — verify `CHANGELOG.md:106` (`## [0.10.0] - 2026-05-11`). `date -u +%Y-%m-%d` is portable across macOS BSD `date` and GNU `date`. ✓.

**Regression test (D-FIX06-3):** Script-level test in `tests/` or a Makefile-comment example. Recommendation:

```bash
# tests/scripts/test_changelog_date_stamp.sh (or equivalent)
#
# Manual smoke test:
#   1. Create fixture CHANGELOG containing "## [Unreleased]" line.
#   2. Run the sed line directly with SEMVER=0.99.0.
#   3. Assert the line is now "## [0.99.0] - <today's date>".
#   4. Re-run the sed line — assert it's a no-op (idempotent).
```

Or alternatively — a shell-level integration test:

```rust
#[test]
fn make_release_sed_replaces_unreleased_section() {
    use std::process::Command;
    let tmp = TempDir::new().unwrap();
    let changelog = tmp.path().join("CHANGELOG.md");
    fs::write(&changelog, "## [Unreleased]\n\n### Added\n").unwrap();

    let date = chrono::Utc::now().format("%Y-%m-%d").to_string();
    // Or fake it via output of `date -u +%Y-%m-%d`:
    let date_out = Command::new("date").args(["-u", "+%Y-%m-%d"])
        .output().unwrap();
    let date = String::from_utf8_lossy(&date_out.stdout).trim().to_string();

    Command::new("sed")
        .args(["-i", "", &format!("s/^## \\[Unreleased\\]/## [0.99.0] - {}/", date),
               changelog.to_str().unwrap()])
        .status().unwrap();

    let content = fs::read_to_string(&changelog).unwrap();
    assert!(content.contains(&format!("## [0.99.0] - {}", date)),
        "sed did not replace [Unreleased]: {content}");

    // Idempotency: second run is no-op.
    Command::new("sed")
        .args(["-i", "", &format!("s/^## \\[Unreleased\\]/## [1.0.0] - {}/", date),
               changelog.to_str().unwrap()])
        .status().unwrap();
    let content2 = fs::read_to_string(&changelog).unwrap();
    assert_eq!(content, content2, "second sed must be idempotent: {content2}");
}
```

(Drop `chrono` if not already a dep — the codebase intentionally avoids it per `manifest.rs:325`. Use `Command::new("date")` to get the date, as shown.)

## Wave / Sequencing Recommendation

CONTEXT.md says "the bugfix work inside this phase is independent and parallelizable internally." Confirmed via code-anchor analysis. Recommended sequencing:

### Wave 1 (foundation + early-wave constraint)

**Wave 1A — `doctor.rs` substrate (single plan; OBS-06 + FIX-01 + FIX-03):**
- Add `IssueCategory` enum + ALL/sentinel
- Add `RepairKind` enum + ALL/sentinel (with the 3 variants from the inventory above)
- Add `category: IssueCategory` + `repair_kind: Option<RepairKind>` fields to `DiagnosticIssue`
- Update 8 emit sites in `check_library`, `check_distribution_dir`, `check_config`
- Rewrite dispatcher in `diagnose()` to match on `repair_kind` instead of substring (FIX-01)
- Add summary line per-category breakdown (D-CAT-3)
- Delete `tracked_managed_symlinks` helper + interactive git-tracked block + `check_library` git-tracked emit (FIX-03)
- New regression tests: (a) D-CAT-2 invariant test, (b) D-FIX03-2 "clean v0.10 lib emits no tracked-in-git warning"
- Rationale: these three items are entangled (FIX-01's dispatcher rewrite touches the same substring-match table that FIX-03 deletes; OBS-06's `category` field needs to land before FIX-01 can break out per-category counts). Single plan reduces conflict surface.

**Wave 1B — FIX-06 `make release` Makefile (parallel, no dependency):**
- Add `sed -i ''` line to `Makefile:19-20` boundary
- Update `git add` line to include `CHANGELOG.md`
- Add Makefile comment per D-FIX06-2
- Add shell-level or Rust-level regression test per D-FIX06-3
- **Must land in Wave 1** so the v0.11 release cut (sequenced after Phase 19) finds an updated Makefile

### Wave 2 (independent FIX items)

**Wave 2A — OBS-07 `tome status` + manifest header (single plan):**
- Add `last_synced_at: Option<String>` field + `stamp_last_synced_at()` method on `Manifest`
- Add `last_sync: Option<String>` field on `StatusReport`
- Add `Last sync: <ts>` line in `render_status` after Library block
- Add SKILLS column in Directories table (5 columns instead of 4)
- Add stamp call in `sync()` between cleanup and existing `manifest::save`
- New tests: (a) manifest schema additive-compat (pre-v0.11 manifest deserializes with `last_synced_at: None`), (b) round-trip stamp (call sync, reload manifest, assert RFC-3339 timestamp), (c) text-mode `Last sync: never` for fresh manifest, (d) JSON shape stable with `last_sync: null`, (e) text-mode SKILLS column appears with `✓ N` format

**Wave 2B — FIX-02 timing flake (parallel, ~5 LOC change):**
- Bump bound from 600ms to 2000ms in `browse/app.rs:1805`
- Update root-cause comment per template (~5 lines)
- For D-FLAKE-2 backup test: planner investigates root cause before committing to relaxed-bound treatment (different mechanism likely)

**Wave 2C — FIX-04 wizard ANSI (parallel):**
- **First**: reproduce the bug in a greenfield TempDir via `script(1)` to confirm whether D-FIX04-1 is still needed despite `tabled[ansi]` already being on
- If reproducible: apply D-FIX04-1 (add `strip-ansi-escapes = "0.2"`, strip cells before tabled width calc at `wizard.rs:514-521`)
- If not reproducible: skip D-FIX04-1 (administrative close); ship D-FIX04-2 snapshot test as pinning measure + close #454

**Wave 2D — FIX-05 wizard library default (parallel, test-only):**
- Add `wizard_library_default_follows_custom_tome_home` integration test (per spec above)
- Implementation is already in place at `wizard.rs:637`; no code change needed

### Wave 3 (verification + release notes)

**Wave 3 — CHANGELOG entry + cross-cutting verification:**
- Add Phase 19 entry to `CHANGELOG.md` `[Unreleased]` block (under Added: OBS-06 doctor categories, OBS-07 last-sync + skill-count; under Fixed: FIX-01..06 with closing issue refs)
- Update REQUIREMENTS.md Traceability table OBS-06..07 + FIX-01..06 from Pending → Done
- Verify `make ci` green (fmt + clippy `-D warnings` + tests)
- Verify test count crossed 1000

### Dependency graph

```
Wave 1A (doctor substrate) ── Wave 3 (CHANGELOG)
Wave 1B (Makefile) ──┐
                     ├── Wave 3
Wave 2A (OBS-07) ────┤
Wave 2B (FIX-02) ────┤
Wave 2C (FIX-04) ────┤
Wave 2D (FIX-05) ────┘
```

Wave 1A and Wave 1B run in parallel (no shared files). Wave 2A-2D all run in parallel after Wave 1A merges (no shared files among 2A-2D; Wave 2A touches `manifest.rs`/`status.rs`/`lib.rs::sync`, Wave 2B touches `browse/app.rs` + `backup.rs`, Wave 2C touches `wizard.rs`/`Cargo.toml`, Wave 2D touches `tests/cli_init.rs` or equivalent).

## Test-Count Growth Audit

**Current count:** 994 (verified by `rg -c "^\s*#\[test\]" --type=rust crates/tome/src crates/tome/tests | awk -F: '{sum+=$2} END {print sum}'`).

**ROADMAP target:** ≥1000 tests at v0.11 ship.

**Projected additions in Phase 19:**

| Test | Source | Estimated count |
|------|--------|-----------------|
| D-CAT-2 invariant: sum-of-per-category-counts == total_issues | Wave 1A | 1 |
| OBS-06 JSON shape: `category` field present per issue | Wave 1A | 1-2 |
| OBS-06 D-CAT-3 per-category auto-fixable breakdown rendering | Wave 1A | 1-2 |
| RepairKind ALL/sentinel basic enum tests (`ALL.len() == 3`, all 3 variants present) | Wave 1A | 1-2 |
| FIX-01 zero-auto-fixable prompt-skip test | Wave 1A | 1 |
| D-REPAIR-3 substring-matching removed (compile-time / code grep) | Wave 1A | 0 (compile-only) |
| FIX-03 D-FIX03-2 clean-library no-tracked-in-git warning | Wave 1A | 1 |
| OBS-07 D-LSYNC schema additive-compat (pre-v0.11 manifest deserializes) | Wave 2A | 1 |
| OBS-07 stamp round-trip via sync() | Wave 2A | 1 |
| OBS-07 text-mode "Last sync: never" | Wave 2A | 1 |
| OBS-07 JSON shape `last_sync: null` for fresh / RFC-3339 for stamped | Wave 2A | 2 |
| OBS-07 D-DIR-1 SKILLS column in Directories text | Wave 2A | 1 |
| FIX-02 (relaxed bound: existing test is the regression test) | Wave 2B | 0 |
| FIX-04 D-FIX04-2 styled summary table snapshot | Wave 2C | 1 |
| FIX-05 wizard library default follows TOME_HOME integration test | Wave 2D | 1 |
| FIX-06 D-FIX06-3 sed substitution test | Wave 1B | 1 |
| **Total new tests** | | **13-17** |

**Projected post-phase count: 1007-1011.** Crosses 1000 with margin. No "while we're here" opportunistic test additions needed.

**Caveat:** If the planner splits Wave 1A into sub-plans (e.g. separate plans for IssueCategory vs RepairKind vs FIX-03 delete), the count grows slightly higher as each plan adds its own regression test for its scope. Either way, ≥1000 is comfortably cleared.

## Risks & Open Questions

### Risks for the planner

1. **FIX-04 anomaly (already-applied fix vs OPEN issue).** The CONTEXT.md decisions D-FIX04-1 + D-FIX04-2 commit to `strip-ansi-escapes` even though `tabled = { features = ["ansi"] }` already does what `strip-ansi-escapes` would do (and was added specifically for this bug per commit `0803afb`). The planner needs to reproduce the bug before adding a redundant dependency. **If the bug does not reproduce, applying D-FIX04-1 verbatim adds 1 dep, 1 import, and ~3 LOC for zero functional improvement.** Recommended: reproduce-first, then plan.

2. **FIX-02 sibling (`backup::push_and_pull_roundtrip`) is NOT a relaxed-bound bug.** The test has no timing assertions visible at `backup.rs:548-590`. D-FLAKE-2 permits re-opening the decision. Recommend the planner allocates time to reproduce the flake before committing to a fix shape. Possible alternative: retry wrapper around `git push`/`git pull` calls; possible alternative: per-test isolation (`#[serial]` annotation).

3. **OBS-07 last-sync stamp ordering vs reconcile-install-failure bail.** The recommended stamp point (line 1781, immediately before `manifest::save`) places the stamp AFTER cleanup but BEFORE the `reconcile_install_failures` bail. This means a sync that completed cleanup but failed plugin install/update produces `last_synced_at: <ts>` even though the user sees an error. CONTEXT.md D-LSYNC-3 wording ("after distribute + cleanup succeed") supports this; the planner could argue for a stricter "all bails clear first" placement. Flag for explicit decision in plan-check.

4. **Removing the `tracked_managed_symlinks` helper may orphan a `git` subprocess import.** Verify after deletion: `cargo build` clean. The function is small and self-contained; risk is low.

5. **`IssueCategory` derivation requires knowing which DoctorReport field the issue lives in at construction.** Today's `DiagnosticIssue::untyped`/`typed` constructors don't know the field — they're called from `check_library`/`check_distribution_dir`/`check_config`. Recommended: add per-emit-site constructors that hardcode the category, e.g. `DiagnosticIssue::library(severity, message)`, `DiagnosticIssue::library_repairable(severity, message, repair_kind)`, etc. Or thread a `DoctorField` enum into a single constructor. Planner picks the API shape during planning.

### Open Questions (no answer found)

1. **Should the JSON `summary` object expose `auto_fixable_by_category` map?** CONTEXT.md `<deferred>` calls this out as planner judgement. The text breakdown is D-CAT-3 ("e.g. `(N auto-fixable: Library M, Foreign-symlink K)`"); the JSON parallel would be:
   ```json
   "summary": {
     "total_issues": 5,
     "by_category": { "library": 2, "directory": 1, "config": 1, "foreign_symlink": 1 },
     "auto_fixable_count": 3,
     "auto_fixable_by_category": { "library": 2, "directory": 1 }  // <-- this map
   }
   ```
   Recommendation: include it. Trivial to compute from the same flatten-and-count operation; zero extra cost to consumers; eliminates a future "why does JSON have less detail than text?" complaint. Risk: locks the JSON shape slightly more. Acceptable.

2. **Should `tome doctor` log "skipped repair because user declined" via `tracing::debug!`?** CONTEXT.md `<specifics>` recommends `tracing::debug!(target: "doctor::repair", ?kind, ?reason, "skipped repair")`. This emits when the global prompt is declined OR when an orphan per-item Select chooses skip. Recommendation: yes, emit. Aligns with Phase 18 D-OUT-1 in-scope contract. Trivial addition. Planner pins exact field names during planning.

3. **For `last_synced_at`, what if the stamp's `now_iso8601()` returns a unix-epoch timestamp (system clock screwed up)?** Existing HARD-20 logic at `manifest.rs:198-218` (`epoch_zero_warning`) handles per-entry `synced_at`. Apply the same warning to `last_synced_at` at `manifest::load`-time. Recommendation: thread `epoch_zero_warning` over the header field too. Trivial. Or accept the gap as a non-issue (the clock can't reasonably be at epoch during a running `tome sync`).

## Project Constraints (from CLAUDE.md)

- **Rust edition 2024** — `Cargo.toml`. All new code must compile with edition 2024 idioms.
- **Strict clippy** — `make ci` runs `cargo clippy --all-targets -- -D warnings`. New code must be warning-free.
- **`cargo fmt`** — All code formatted with `cargo fmt` (no custom rustfmt.toml).
- **Unix-only** — `std::os::unix::fs::symlink` is the substrate. No Windows code paths.
- **`anyhow::Result` + `.context()` / `.with_context()`** — All fallible operations.
- **`pub(crate)` for internal helpers** — minimize public surface.
- **Newtype wrappers for validated identifiers** — `SkillName`, `DirectoryName`, `ContentHash` precedent.
- **`tracing::*` for diagnostic output post-Phase 18** — No `eprintln!` for new warnings. Wizard prompts (`dialoguer`) and ceremonial summary tables (`tabled`) stay on direct stdout.
- **`tabled = { features = ["ansi"] }`** — ALREADY ENABLED. Don't remove. Affects FIX-04 planning.
- **Additive schema migrations** — `Option<T>` + `#[serde(default)]` for new fields.
- **PRs MUST be DRAFT** — Per global agent rules. Use `gh pr create --draft`.
- **NEVER commit directly to `main`** — Feature branch first.
- **Non-interactive shell commands** — `cp -f`, `mv -f`, `rm -f`. Avoid hangs on `-i` aliases.

## Sources

### Primary (HIGH confidence)

- `/Users/martin/dev/opensource/tome/.planning/phases/19-doctor-status-surface-bugfix-bundle/19-CONTEXT.md` — the locked contract
- `/Users/martin/dev/opensource/tome/.planning/REQUIREMENTS.md` — OBS-06..07 + FIX-01..06 acceptance criteria + traceability
- `/Users/martin/dev/opensource/tome/.planning/ROADMAP.md` — Phase 19 entry (lines 218-232), success criteria 1-4
- `/Users/martin/dev/opensource/tome/.planning/STATE.md` — phase shape + status
- `/Users/martin/dev/opensource/tome/.planning/phases/18-observability-foundation-sync-diagnostics/18-CONTEXT.md` — D-OUT-1 scope contract, tracing substrate
- `/Users/martin/dev/opensource/tome/.planning/phases/18-observability-foundation-sync-diagnostics/18-VERIFICATION.md` — tracing substrate is live (confirms `eprintln!` is the wrong choice for new diagnostic output)
- `/Users/martin/dev/opensource/tome/crates/tome/src/doctor.rs` — emit sites, dispatcher, repair_library
- `/Users/martin/dev/opensource/tome/crates/tome/src/status.rs` — gather, render_status, DirectoryStatus
- `/Users/martin/dev/opensource/tome/crates/tome/src/manifest.rs` — Manifest struct, now_iso8601, EPOCH_ZERO_TIMESTAMP, atomic save
- `/Users/martin/dev/opensource/tome/crates/tome/src/lib.rs:1446-1887` — `sync()` pipeline + cleanup ordering
- `/Users/martin/dev/opensource/tome/crates/tome/src/wizard.rs:499-677` — show_directory_summary, configure_library, current `<tome_home>/skills` derivation
- `/Users/martin/dev/opensource/tome/crates/tome/src/browse/app.rs:120-143, 1782-1821` — clipboard retry helper + flake test
- `/Users/martin/dev/opensource/tome/crates/tome/src/backup.rs:548-590` — push_and_pull_roundtrip test
- `/Users/martin/dev/opensource/tome/Makefile:9-32` — release recipe
- `/Users/martin/dev/opensource/tome/CHANGELOG.md` — current `[Unreleased]` block shape
- `/Users/martin/dev/opensource/tome/Cargo.toml:32` — `tabled = { version = "0.20", features = ["ansi"] }` (already enabled)
- Commit `0803afb` `fix(wizard): align tabled summary header with body in interactive TTY` (Apr 23, 2026)
- `gh issue view 454 530 532 511 --repo MartinP7r/tome` — all four issues currently OPEN

### Secondary (MEDIUM confidence)

- [strip-ansi-escapes 0.2.1 — crates.io](https://crates.io/crates/strip-ansi-escapes) — latest version + API
- [strip-ansi-escapes — docs.rs](https://docs.rs/strip-ansi-escapes/latest/strip_ansi_escapes/) — `strip_str(&str) -> String` API
- [tabled crate on docs.rs](https://docs.rs/crate/tabled/latest) — `ansi` feature enables `ansi-str` + `ansitok` for ANSI-aware width calc
- [zhiburt/tabled on GitHub](https://github.com/zhiburt/tabled) — `ansi` feature documentation
- POLISH-04 pattern instances: `cli.rs::LogLevel`, `change_cause.rs::ChangeCause`, `marketplace.rs::InstallFailureKind`, `remove.rs::FailureKind`, `migration_v010.rs::MigrationFailureKind`, `doctor.rs::DiagnosticIssueKind` — six examples of `ALL` array + sentinel pattern in the codebase

### Tertiary (LOW confidence — flagged for verification)

- FIX-02 timing bound of 2000ms — based on existing comment's empirical breakdown + CONTEXT.md hint; not empirically measured under reproduced flake conditions
- FIX-04 bug-still-reproduces — anomaly flagged; planner must verify before applying D-FIX04-1

## Metadata

**Confidence breakdown:**

- RepairKind variant inventory: HIGH — direct code grep of `doctor.rs:267-453` enumerates every handler arm
- IssueCategory serialization (snake_case): HIGH — verified existing JSON conventions in 4 files
- Manifest header placement: HIGH — verified Manifest struct shape; option A is strictly additive
- OBS-07 text rendering: HIGH — patch-shape derived from existing `render_status` patterns
- FIX-02 bound recommendation: MEDIUM — empirical breakdown is from existing comment, not new measurement
- FIX-03 deletion targets: HIGH — three code blocks identified by exact line numbers
- FIX-04 anomaly: HIGH (anomaly verified) — feature flag verified in Cargo.toml; commit verified via `git log -S`; issue state verified via `gh issue view`
- FIX-05 already-implemented status: HIGH — line 637 reading
- FIX-06 sed line shape: HIGH — verified `make release` recipe + CHANGELOG `[Unreleased]` pattern
- Wave sequencing: HIGH — dependency graph derived from file-overlap analysis
- Test-count growth audit: HIGH — current count verified via grep

**Research date:** 2026-05-13
**Valid until:** 2026-06-13 (30 days — stable phase; no fast-moving dependencies)
