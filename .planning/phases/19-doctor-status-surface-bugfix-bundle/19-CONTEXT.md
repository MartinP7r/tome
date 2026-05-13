# Phase 19: Doctor/status surface + bugfix bundle - Context

**Gathered:** 2026-05-13
**Status:** Ready for planning
**Requirements:** OBS-06, OBS-07, FIX-01, FIX-02, FIX-03, FIX-04, FIX-05, FIX-06

<domain>
## Phase Boundary

Polish phase — richer diagnostic surface for `tome doctor` / `tome status` plus closure of the v0.10-surfaced bug backlog. **In scope:** issue categorization and auto-repair semantics (OBS-06, FIX-01), per-directory skill counts + last-sync timestamp (OBS-07), six targeted bugfixes (FIX-02..FIX-06). **Out of scope:** new diagnostic checks beyond what closes #530/#532, output redesigns, schema changes beyond the single additive `last_synced_at` manifest header field, behavioral changes to existing repair flows.

</domain>

<decisions>
## Implementation Decisions

### OBS-06 — Doctor categorization model

- **D-CAT-1 (Derive category from field structure + promote ForeignSymlink):** Add a new `IssueCategory { Library, Directory, Config, ForeignSymlink }` enum following POLISH-04 (`ALL: [IssueCategory; 4]` array + compile-time exhaustiveness sentinel). Add a `category: IssueCategory` field on `DiagnosticIssue`, computed at construction from `(which DoctorReport field, kind)`: Library/Directory/Config come from the field the issue lives in; if `kind == DiagnosticIssueKind::ForeignSymlink`, the category is promoted to `ForeignSymlink` (overrides the parent field). JSON shape gains `category` as a serialized string per issue.
- **D-CAT-2 (ForeignSymlink mutually exclusive in summary):** Each `DiagnosticIssue` belongs to exactly one category. ForeignSymlink issues count *only* in the ForeignSymlink bucket — not also under Library/Directory. Summary-line counts add up to `report.total_issues()`. The summary serializer must verify this invariant in tests (sum-of-category-counts == total_issues).
- **D-CAT-3 (Category-aware auto-fixable breakdown in summary):** When `auto_fixable_count > 0`, the summary line includes a per-category breakdown — e.g. `(N auto-fixable: Library M, Foreign-symlink K)` — surfacing which categories have auto-repair paths. Only categories with non-zero auto-fixable counts appear in the breakdown.

### FIX-01 — Auto-fixable definition (closes #530)

- **D-REPAIR-1 (Typed `RepairKind` enum + ALL sentinel):** Introduce `RepairKind { RemoveBrokenSymlink, RemoveStaleManifestEntry, ... }` (specific variants TBD by researcher/planner — one per real auto-repair handler in `doctor.rs`). Follows POLISH-04 (`ALL` array + compile-time exhaustiveness sentinel). Add `repair_kind: Option<RepairKind>` field on `DiagnosticIssue`. `Some(kind)` ↔ auto-fixable; the global repair dispatcher matches on `RepairKind` so adding a variant without a handler fails to compile.
- **D-REPAIR-2 (Skip global prompt at zero):** When `auto_fixable_count == 0`, the global `Apply N auto-fixable repairs? [Y/n]` prompt is skipped entirely. Interactive issues (orphan directories, etc.) still receive their existing per-item prompts. This is the literal #530 fix — no `(no auto-repair available)` follow-up to a non-zero count.
- **D-REPAIR-3 (Substring matching removed):** Existing message-substring matching (`i.message.contains("orphan directory") || i.message.contains("tracked in git")`) at `doctor.rs:267-285` is replaced by `repair_kind`-based discrimination. Substring matching is anti-pattern and brittle to message-wording changes (this is what made FIX-03's stale check hard to find).

### OBS-07 — Status richer surface

- **D-LSYNC-1 (Explicit header field):** Add `last_synced_at: Option<String>` to the **manifest header** (NOT per-entry — separate concept from `synced_at`). Type is `Option<String>` for additive-schema compatibility: pre-v0.11 manifests deserialize the field as `None`, no migration required. Format: RFC-3339 (`now_iso8601()`).
- **D-LSYNC-2 (`never` rendering):** `tome status` text output prints `Last sync: never` when manifest doesn't exist OR `last_synced_at` is `None`. JSON shape: `last_sync: Option<String>` — `null` for never, RFC-3339 string otherwise.
- **D-LSYNC-3 (Full successful sync only):** `last_synced_at` is stamped as the final step of `sync()`, after distribute + cleanup succeed. Mid-sync panic or aborted reconcile leaves the previous value unchanged. Honest reporting: `Last sync: <ts>` always reflects a sync that completed cleanly through the cleanup phase.
- **D-DIR-1 (Per-directory skill count in text):** `DirectoryStatus.skill_count` already exists in the JSON shape — Phase 19 surfaces it in the text rendering of the Directories section. Existing `(override)` annotation from PORT-05 is preserved. Column order: `name | type | role | skill_count | path` (or similar — researcher decides exact rendering; planner pins it).

### FIX-02 — Timing flake (closes #511 + HARD-14)

- **D-FLAKE-1 (Relaxed bound + root-cause comment):** Bump `copy_path_retry_helper_returns_within_bound` upper bound from 600ms to ~2000ms (researcher confirms exact value via local measurement). Add `// SAFETY:` comment explaining the assertion is a regression guard against actual hangs, not a perf gate, and naming `arboard`/parallel-test contention as the root cause. ROADMAP explicitly permits this approach. ~5 LOC change.
- **D-FLAKE-2 (HARD-14 same treatment):** Apply identical pattern (relaxed bound + named-root-cause comment) to `backup::tests::push_and_pull_roundtrip` since the milestone description bundles both flakes together. If investigation reveals a different root cause class, planner re-opens this decision.
- **D-FLAKE-3 (Out of scope: clock injection):** Deterministic clock injection (introducing `trait Clock` across `browse::app`) is explicitly rejected for v0.11 polish scope. If the relaxed bound flakes again post-fix, the abstraction can be introduced in a future phase.

### FIX-03 — Stale "tracked in git" check (closes #532)

- **D-FIX03-1 (Delete entirely):** Remove the `"N managed symlink(s) tracked in git"` check (currently at `crates/tome/src/doctor.rs:665` and its render path at `:383-394`) wholesale. v0.10 made managed skills real directory copies; the check's original concern (machine-specific symlinks in git) no longer applies. No replacement check is added — if a real failure mode emerges, it will get its own ticket.
- **D-FIX03-2 (Regression test):** New integration test asserts that a clean v0.10-shape library produces zero "tracked in git" warnings from `tome doctor`. The test fixture is a fresh real-directory-copy library.

### FIX-04 — ANSI width in wizard summary (closes #454)

- **D-FIX04-1 (`strip-ansi-escapes` crate):** Add `strip-ansi-escapes` as a regular dep (not dev-dep — runtime path). Strip ANSI escapes before passing strings to `tabled`'s width measurement. Apply to the wizard summary table's `Width::increase`/`Width::truncate` cell handling.
- **D-FIX04-2 (Snapshot test):** New snapshot test renders a styled summary table (`console::style(...).bold()` cells) and asserts column alignment is correct under ANSI-aware width.

### FIX-05 — Wizard library default (closes #453 + #456)

- **D-FIX05-1 (Library default tracks tome_home):** `wizard::configure_library` proposes `<resolved_tome_home>/skills` as the library default, NOT hardcoded `~/.tome/skills`. The library-default derivation must use the resolved `tome_home` value (after tilde expansion and any `TOME_HOME` env-var override). Verified by wizard integration test driving a custom `tome_home` in `--no-input` mode (e.g., `tome_home = ~/dev/coding-agent-files/.tome` → library default = `~/dev/coding-agent-files/.tome/skills`).
- **D-FIX05-2 (No fallback chain):** When `tome_home` is set, library default is unconditionally `<tome_home>/skills`. No fallback to `~/.tome/skills` if that path doesn't exist. The wizard's existing path-creation flow handles the missing-directory case.

### FIX-06 — `make release` CHANGELOG date-stamp (closes #533)

- **D-FIX06-1 (Inline `sed` in Makefile recipe):** Add a single `sed -i ''` line to the existing `make release` recipe (Makefile:14-32) between the `cargo check` step and the branch-creation step. Replaces `## [Unreleased]` with `## [$$SEMVER] - $$(date -u +%Y-%m-%d)` in `CHANGELOG.md`. Style matches the existing `sed -i '' "s/^version = ...` line that bumps `Cargo.toml`. The CHANGELOG.md edit is staged and committed alongside the version bump.
- **D-FIX06-2 (Idempotency / safety):** If `CHANGELOG.md` lacks an `[Unreleased]` section (someone already cut a release without re-adding it), `sed` is a no-op — release proceeds without the changelog edit. Document this in a Makefile comment; don't fail the release on missing `[Unreleased]`.
- **D-FIX06-3 (Test):** Script-level test (or a documented `--dry-run` smoke) shows the substitution against a fixture changelog. No GitHub-API mock needed.

### Claude's Discretion

The following are not gray areas — the researcher/planner picks the technically clean approach during planning, guided by the locked decisions above:

- **`RepairKind` enum specific variants** — derive from inventory of actual auto-repair handlers in current `doctor.rs`. Each handler = one variant.
- **`IssueCategory` enum serialization format** — researcher chooses between e.g. `"library"` (snake_case) vs `"Library"` (PascalCase) for JSON. Recommendation: snake_case to match existing JSON conventions (`override_applied`, `skill_count`).
- **Manifest-header field placement** — researcher decides whether `last_synced_at` lives at the top of `Manifest` struct or inside a new `Header` struct. Either is fine.
- **Exact text rendering of Directories table** — column widths, separator style, whether `skill_count` appears as `5` or `5 skills` — planner pins it after researcher prototypes.
- **Test-count target** — ROADMAP targets ≥1000 tests at v0.11 ship time (was 987 at v0.10.0, currently 808+ unit + CLI suites after Phase 18). Planner verifies the count organically grows past 1000 through the regression tests required by D-FIX03-2 / D-FIX04-2 / D-FLAKE-1 / etc.

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### Phase + milestone context

- `.planning/ROADMAP.md` — Phase 19 entry (§ "Phase 19: Doctor/status surface + bugfix bundle") with success criteria 1-4; the locked goal contract.
- `.planning/REQUIREMENTS.md` lines 20-32, 66-73 — OBS-06/07 + FIX-01..06 acceptance criteria and Traceability table.
- `.planning/PROJECT.md` § "Current Milestone: v0.11 Polish + Observability" — milestone goal and bundling rationale (#511 + HARD-14 paired).
- `.planning/phases/18-observability-foundation-sync-diagnostics/18-CONTEXT.md` — Phase 18 locked decisions (D-OUT-1 doc-enforced scope contract, D-ENV-1 TOME_LOG precedence, tracing substrate behavior).
- `.planning/phases/18-observability-foundation-sync-diagnostics/18-VERIFICATION.md` — confirms tracing substrate is live (any new diagnostic emission MUST route through `tracing::*`, not `eprintln!`).

### Locked patterns (cross-phase)

- POLISH-04 pattern (referenced in `15-CONTEXT.md` + applied throughout): `enum E { ... } impl E { const ALL: [E; N] = [...]; } const fn _sentinel(x: E) { match x { ... } }` — compile-time exhaustiveness guard. Used here for both `IssueCategory` (D-CAT-1) and `RepairKind` (D-REPAIR-1).
- SAFE-01 / `*FailureKind` enums (Phase 8): grouped-renderer pattern for collected failures. Applies to any new aggregated failure rendering in this phase.
- HARD-20 epoch-zero handling (`crates/tome/src/manifest.rs:198-218`): pattern for surfacing degenerate timestamp values. May inform `last_synced_at` edge cases.

### GitHub issues (close-this-issue contracts)

- [#530](https://github.com/MartinP7r/tome/issues/530) — FIX-01 / OBS-06 contradiction
- [#511](https://github.com/MartinP7r/tome/issues/511) — FIX-02 timing flake
- [#532](https://github.com/MartinP7r/tome/issues/532) — FIX-03 stale check
- [#454](https://github.com/MartinP7r/tome/issues/454) — FIX-04 ANSI width
- [#453](https://github.com/MartinP7r/tome/issues/453) + [#456](https://github.com/MartinP7r/tome/issues/456) — FIX-05 wizard library default
- [#533](https://github.com/MartinP7r/tome/issues/533) — FIX-06 CHANGELOG date-stamp

### Code anchors (where the work lands)

- `crates/tome/src/doctor.rs` — `DiagnosticIssue`, `DiagnosticIssueKind`, `DoctorReport`, render functions, repair dispatcher. OBS-06 + FIX-01 + FIX-03 land here.
- `crates/tome/src/status.rs` — `StatusReport`, `DirectoryStatus`, `gather`, `render_status`. OBS-07 lands here.
- `crates/tome/src/manifest.rs` — `Manifest` struct, `synced_at` per-entry, `now_iso8601()`. D-LSYNC-1 adds `last_synced_at` to the header.
- `crates/tome/src/lib.rs::sync()` — full sync orchestration. D-LSYNC-3 final-step stamp lands here (after cleanup).
- `crates/tome/src/wizard.rs` — `configure_library` (FIX-05) and styled summary tables (FIX-04 ANSI width).
- `crates/tome/src/browse/app.rs:1804` — `copy_path_retry_helper_returns_within_bound` test (FIX-02).
- `crates/tome/src/backup.rs` — `push_and_pull_roundtrip` test (HARD-14 carry-over folded into FIX-02 via D-FLAKE-2).
- `Makefile` lines 9-32 — `release` recipe (FIX-06 inline sed addition).

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets

- **`DoctorReport` field-based categorization (`doctor.rs:117-131`)** — `library_issues`, `directory_issues`, `config_issues` already separate issues by category internally. D-CAT-1 builds on this: derive `IssueCategory` from the field structure rather than re-architecting the data layout.
- **`DiagnosticIssueKind` POLISH-04 scaffold (`doctor.rs:39-68`)** — Single-variant enum (`ForeignSymlink`) with `ALL` array + sentinel already in place. Pattern to clone for `IssueCategory` and `RepairKind`.
- **`DirectoryStatus.skill_count` (`status.rs:46`)** — Field already exists with `CountOrError` type. OBS-07 surfaces it in text rendering; JSON shape unchanged.
- **`now_iso8601()` + `EPOCH_ZERO_TIMESTAMP` in manifest** — Timestamp helpers ready for `last_synced_at`. HARD-20 epoch-zero warning pattern may inform edge cases for unmigrated manifests.
- **`console::style(...).bold()` + `tabled::Table` already used in wizard** — Existing wizard code (`wizard.rs:8,12-13`) is the exact failure site for FIX-04. The fix integrates with the existing render path, no new framework adoption.
- **`make release` recipe style (`Makefile:14-32`)** — Existing `sed -i ''` line for Cargo.toml version bump establishes the pattern; FIX-06 adds a sibling `sed` line for CHANGELOG.md.

### Established Patterns

- **`tracing::*` for all warnings/info post-Phase 18** — Any new diagnostic output (e.g., "skipping repair because…") routes through `tracing::warn!`/`info!`, NOT `eprintln!`. Preserves the OBS-01 byte-identical-stdout commitment. Wizard prompts and user-facing summary tables remain on direct stdout per Phase 18 scope discipline.
- **JSON ↔ text shape parity** (PORT-05, UNOWN-03, D-D3) — Every text-visible field has a JSON counterpart with documented semantics. Applies to `category` (D-CAT-1), `last_sync` (D-LSYNC-2), per-directory `skill_count` (D-DIR-1).
- **Additive schema migrations** (LIB / RECON precedent): new struct fields use `Option<T>` + `#[serde(default)]` so pre-v0.11 manifests deserialize cleanly. Applied by D-LSYNC-1.
- **Regression test per FIX item** — ROADMAP success criterion 3 mandates one. Tests pin the specific failure mode (e.g., FIX-03's "clean v0.10 library produces zero `tracked in git` warnings").

### Integration Points

- **`DoctorReport::total_issues()`** (current `doctor.rs:140`) — Used in summary computation; D-CAT-2's mutually-exclusive invariant gets a unit test here (sum of per-category counts == total_issues).
- **Repair dispatcher in `doctor.rs:459` (`render_repair_plan_auto`) + `:380+` interactive path** — D-REPAIR-1's `RepairKind` becomes the dispatcher's match arms. The new compile-time invariant: every variant has a handler arm.
- **`Manifest::save` (atomic temp+rename)** — D-LSYNC-1 piggybacks on existing atomic write flow. Stamp `last_synced_at` in the report-building step, persist via the existing save call.
- **`sync()` final cleanup phase** — D-LSYNC-3 stamps after `cleanup()` returns Ok. The exact wiring point is the last manifest write before the function returns.

</code_context>

<specifics>
## Specific Ideas

- **POLISH-04 sentinel is mandatory for new enums** — `IssueCategory` and `RepairKind` both need `ALL` array + compile-time exhaustiveness sentinel. Researcher uses `_diagnostic_issue_kind_exhaustiveness_sentinel` at `doctor.rs:60` as the template.
- **`tracing` instrumentation for repair decisions** — When the dispatcher skips a repair (e.g., user declined, or no handler), emit `tracing::debug!(target: "doctor::repair", ?kind, ?reason, "skipped repair")`. Aligns with Phase 18 D-OUT-1 in-scope contract.
- **FIX-02 root-cause comment template** — `// FLAKE-FIX (#511 / HARD-14): bound relaxed from 600ms to 2000ms. arboard clipboard contention under --test-threads=N can pause threads ≫ 600ms regardless of helper performance. This assertion guards against actual hangs, not perf regressions.`
- **CHANGELOG `[Unreleased]` rule of thumb** — Phase 18 wrote v0.11 work under `[Unreleased]`. After Phase 19 lands, the v0.11 release cut renames it to `[0.11.0] - <date>`. FIX-06's `make release` automation must be in place BEFORE that cut — sequence Phase 19 plans so FIX-06 lands early.

</specifics>

<deferred>
## Deferred Ideas

- **Deterministic clock injection (`trait Clock` in `browse::app`)** — Rejected for v0.11 polish scope (D-FLAKE-3). Future phase if relaxed bound flakes again. Triggered when timing-flake recurrence justifies the abstraction cost.
- **Replacement for the "tracked in git" check** — D-FIX03-1 deletes wholesale. If a real-world failure mode emerges that warrants detection, file a new ticket and address in a future phase.
- **cargo-dist hook for CHANGELOG date-stamping** — D-FIX06-1 picks inline `sed` for style consistency. If cargo-dist upgrades expose a clean release-time hook in future, migration could simplify the Makefile.
- **JSON `auto_fixable_count` breakdown in OBS-06 summary object** — D-CAT-3 specifies category-aware breakdown in *text* output. The JSON `summary` object exposes per-category counts (per ROADMAP success criterion 1); whether to also include an `auto_fixable_by_category: { Library: M, ForeignSymlink: K }` map is left to planner judgement.
- **Test-count budgeting beyond ≥1000** — Researcher/planner may surface "while we're in here" test additions; treat as opportunistic, not scope-creep, only if zero-cost.

### Reviewed Todos (not folded)

None — no pending todos surfaced by `cross_reference_todos` step.

</deferred>

---

*Phase: 19-doctor-status-surface-bugfix-bundle*
*Context gathered: 2026-05-13*
