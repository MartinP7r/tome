# Phase 14: Unowned-library lifecycle - Context

**Gathered:** 2026-05-07
**Status:** Ready for planning

<domain>
## Phase Boundary

Surface and manage skills whose source has been removed from `tome.toml` (the
"Unowned" set, formalised in Phase 11 LIB-04). The CLI vocabulary captured by
UNOWN-01..03 (`tome adopt`, `tome forget`, plus `tome status`/`tome doctor`
surfacing) **does not** ship as two new top-level commands — discussion in this
phase folded both verbs into existing commands. The user-facing behaviour the
requirements describe (re-anchoring an Unowned skill, deleting an Unowned
skill, showing the unowned set in status/doctor) is delivered in full; the
**API shape** is different from what UNOWN-01..03 verbatim assumed.

**In scope:**

- **Extend `tome reassign`** to accept Unowned input. Drops the existing refusal
  in `reassign.rs:58-63` (which today errors with "use `tome adopt` (Phase 14)
  to assign a directory before reassigning"). The same command now handles
  Owned→Owned reassign AND Unowned→Owned re-anchoring (UNOWN-01 behaviour).
- **Restructure `tome remove`** as a `clap` nested subcommand: `tome remove dir
  <name>` (today's behaviour, renamed) and `tome remove skill <name>` (new,
  per-skill deletion that delivers UNOWN-02 behaviour). Breaking change to
  today's `tome remove <name>` shape; project policy "Backward compat: None"
  makes this acceptable.
- **Tighten `tome reassign`** at the same time: refuse different-content
  collisions instead of silently relinking; reject target-only directory roles.
  These hardenings apply to BOTH the existing Owned→Owned path and the new
  Unowned→Owned path.
- **Add `previous_source: Option<DirectoryName>`** to `SkillEntry` and
  `LockEntry`. Written at all three Unowned-transition sites (cleanup
  Case 1, `remove::execute` for `dir` subcommand, Phase 13 fork-in-place
  flip in `reconcile.rs`). Closes the lossy gap noted in Phase 13 D-13.
- **Surface unowned skills** in `tome status` text + JSON output and
  `tome doctor` text + JSON output, per UNOWN-03 success criteria.
  Section omits cleanly when the unowned set is empty.

**Out of scope** (handled by other phases or out of v0.10 entirely):

- Cleanup-message UX rewrite (3-bucket partition for stale skills) →
  Phase 16 (UX-01). Phase 14 may emit text mentioning Unowned; the
  3-bucket cleanup partition is Phase 16's job.
- `lib.rs::run` decomposition (the new `Remove { #[command(subcommand)] }`
  shape will land mid-`run`; Phase 15 HARD-02 will tidy this) → Phase 15.
- `tests/cli.rs` per-domain split (HARD-13) → Phase 15. Phase 14's
  integration tests land in `tests/cli.rs` today; Phase 15 splits them
  later.
- Backfill of `previous_source` for pre-Phase-14 entries → explicitly
  not done. Pre-Phase-14 Unowned entries fall back to rendering
  `source_path` (D-C2). One-time UX gap; new transitions get clean
  provenance.
- Per-directory bulk operations (e.g. `tome reassign --all-unowned --to
  <dir>`, `tome remove skill --orphans-only`) → not in v0.10; possible
  v0.11+ if needed.

</domain>

<decisions>
## Implementation Decisions

### API merge: drop the proposed new commands, extend existing ones

- **D-API-1 (no `tome adopt`):** UNOWN-01's "re-anchor an unowned skill" is
  delivered by **extending `tome reassign`**, not by a new top-level command.
  Mechanically `adopt` and `reassign` do the same work — copy library content
  into a configured directory's path, update `manifest[skill].source_name`.
  The only difference is the starting state (`None` vs `Some(_)`). The current
  refusal in `reassign.rs:58-63` is removed; the function signature accepts
  `from_directory: Option<DirectoryName>` and renders the plan accordingly
  (no source rendered when `None`). Smaller API surface; one verb to
  re-anchor a skill regardless of its current state. The literal stub error
  message in `reassign.rs:60` ("use `tome adopt` (Phase 14)...") is deleted.

- **D-API-2 (no `tome forget`; subcommand split for `tome remove`):**
  UNOWN-02's "delete an Unowned skill" is delivered as **`tome remove skill
  <name>`** — a new subcommand variant of an existing top-level command.
  At the same time, today's `tome remove <name>` becomes **`tome remove dir
  <name>`**. The clap shape:
  ```rust
  Remove {
      #[command(subcommand)]
      kind: RemoveKind,
  }

  enum RemoveKind {
      Dir { name: String, #[arg(long)] force: bool },
      Skill { name: String, #[arg(long, short)] yes: bool },
  }
  ```
  This is a **breaking change** to today's `tome remove <name>` shape.
  Project policy "Backward compat: None" makes this acceptable; the migration
  is a documentation update only. CHANGELOG.md v0.10 entry must call this
  out.

  Rationale for splitting rather than overloading on `<NAME>` alone: skill
  names and directory names share the same `validate_identifier` namespace
  in principle, so `tome remove foo` would be ambiguous if a skill and
  directory shared a name (rare in practice, but a real API hygiene risk).

### Reassign behaviour (Area 1 — UNOWN-01 delivery)

- **D-A1 (different-content collision):** `reassign::plan` adds a content-hash
  check. When the target dir's `<dir-path>/<skill>/` already exists with
  content whose SHA-256 differs from the library copy, refuse with:
  ```
  error: skill 'foo' already exists in 'my-dir' with different content.
  Use --force to overwrite, or remove the existing entry first.
  ```
  New `--force` flag on `tome reassign` bypasses the check (overwrites the
  target with library content). Same-content collision (hashes match) keeps
  the existing `ReassignAction::Relink` path — manifest-only flip, no copy.
  Hardens behaviour for BOTH Owned→Owned and Unowned→Owned reassigns;
  closes a today-existing silent-discordance footgun.

- **D-A2 (target role restriction):** `tome reassign --to <dir>` rejects
  target-only directory roles (the role where `is_discovery() == false`).
  Reassigning into a target-only dir leaves the skill stranded — nothing
  rediscovers it on next sync. Discovery and mixed roles are accepted.
  Error message:
  ```
  error: directory 'my-target' has role 'target-only' and cannot receive
  reassigned skills (next sync would not rediscover them). Reassign into
  a discovery or mixed-role directory.
  ```

### Remove-skill behaviour (Area 2 — UNOWN-02 delivery)

- **D-B1 (cleanup scope):** `tome remove skill <name>` deletes:
  1. `manifest[name]` entry (`Manifest::remove`)
  2. `library_dir/<name>/` directory tree
  3. Distribution symlinks for the skill in every distribution-role
     directory (mirror `cleanup::cleanup_target` per-skill)
  4. **`tome.lock`** entry for the skill (`LockEntry` removal). Required
     for the cross-machine workflow — without this, machine B's next sync
     would see a lockfile entry for a missing skill and trigger a RECON-02
     "missing-from-machine" install attempt for a skill the user
     explicitly forgot.
  5. **`machine.toml::disabled`** set membership (if present).
  6. **`machine.toml::directories.<dir>.enabled`** and
     **`machine.toml::directories.<dir>.disabled`** list memberships
     (if present, across all directory entries).

  Failures aggregate via SAFE-01 pattern (new `RemoveSkillFailureKind` enum
  with its own `ALL` array + compile-time exhaustiveness assertion). Exits
  non-zero on any failure; lockfile/machine.toml saves use atomic
  temp+rename.

- **D-B2 (Owned guard):** `tome remove skill <name>` refuses to operate on
  a skill where `manifest[name].source_name.is_some()`. No `--force`
  bypass. Error message:
  ```
  error: skill 'foo' is owned by directory 'bar' (source_name = bar).
  Remove the source directory with `tome remove dir bar` first, or
  remove the file from disk and re-sync.
  ```
  Rationale: `--force`-bypassing this guard would be misleading — the
  source file would still be on disk, and the next `tome sync` would
  re-discover and re-create the manifest entry. The hint is
  actionable: the user has a real path to deletion via `tome remove dir`
  or filesystem deletion + sync.

- **D-B3 (confirmation default):** Interactive `dialoguer::Confirm` with
  default `n`. `Are you sure you want to forget skill 'foo'? [y/N]`.
  `--yes` (or `-y`) skips the prompt. Matches the safer default for
  destructive operations; mirrors the existing `tome remove dir`
  confirmation default.

### Provenance: `previous_source` (Area 3 — supports UNOWN-03)

- **D-C1 (schema addition):** Add a new field to `SkillEntry`:
  ```rust
  /// Last directory that owned this skill before transition to Unowned.
  /// Surfaced in `tome status`/`tome doctor` Unowned section. Cleared
  /// (set to None) when an Unowned skill is re-anchored via
  /// `tome reassign`.
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub previous_source: Option<DirectoryName>,
  ```
  Mirror the same field on `LockEntry` for cross-machine surfacing.
  Backward-compat with existing manifests/lockfiles is automatic via
  `#[serde(default)]`.

  **Written at all three transition sites:**
  1. `cleanup::cleanup_library` Case 1 (orphan detection): before
     setting `entry.source_name = None`, capture
     `entry.previous_source = entry.source_name.take()` (or clone-then-
     None — whichever is cleanest in Rust ownership terms).
  2. `remove::execute` for the `dir` subcommand: when the directory
     removal flips owned manifest entries to Unowned, capture
     `previous_source = old source_name` for each.
  3. `reconcile::apply_edit_decisions` (Phase 13's fork-in-place flip):
     when the user picks "fork" and the manifest is flipped from
     `managed=true, source_name=Some(dir)` to `managed=false,
     source_name=None`, capture `previous_source = old source_name`.

  **Cleared on re-anchor:** when `tome reassign` accepts an Unowned skill
  and re-anchors it (sets `source_name = Some(<new_dir>)`), the
  `previous_source` field is set to `None`. The skill is owned again;
  the breadcrumb is no longer needed.

- **D-C2 (pre-Phase-14 fallback):** When `previous_source.is_none()`
  on an Unowned entry (typically a Phase-13-shipped fork-in-place that
  was forked before Phase 14 landed), `tome status`/`tome doctor` falls
  back to rendering `source_path` via `paths::collapse_home`. The
  display column shows the path string instead of a clean directory name.
  Imperfect but informative.

  No backfill is performed. The "lossy gap" Phase 13 D-13 acknowledged
  remains for already-shipped fork-in-place entries; new transitions get
  clean provenance.

### Status/doctor surfacing (Area 4 — UNOWN-03 delivery)

- **D-D1 (rendering shape):** Tabled, mirroring the Directories section
  style. Columns:
  | NAME | LAST-KNOWN SOURCE | SYNCED |
  Uses `tabled::Table::from_iter` + `Style::blank()` + bold header row
  via `Modify::new(Rows::first())` — the existing pattern. The
  LAST-KNOWN SOURCE column renders `previous_source` (D-C1) or
  `source_path` collapsed via `paths::collapse_home` (D-C2 fallback).
  SYNCED renders `manifest[name].synced_at` (the original-consolidation
  timestamp, preserved across Unowned transition per Phase 11 manifest
  semantics).

  Heading: `Unowned skills (N):` (count in parens). Section omits
  cleanly when the unowned set is empty (no empty header rendered).

- **D-D2 (placement in `tome status`):** After the Directories table,
  before the Health line. Reading order:
  1. Library: <path> + count
  2. Directories: <table>
  3. **Unowned skills (N): <table>**  (this section)
  4. Health: <summary>
  Reads naturally as "configured directories, then skills not covered by
  any directory, then overall health." Section omits cleanly when empty
  (no blank line, no header).

- **D-D3 (doctor severity):** Unowned skills are **informational**, not
  warnings or errors. They render as a separate parallel section in
  `DoctorReport` (new field `unowned_skills: Vec<SkillSummary>` alongside
  the existing `library_issues`, `directory_issues`, `config_issues`).
  They do NOT count toward `DoctorReport::total_issues`. `tome doctor`
  exit code is unaffected by unowned skills.

  Rationale: Unowned is intentional state (the user removed a directory).
  Conflating it with actionable malfunctions (broken symlinks,
  missing-from-disk, etc.) would be noisy. The user has the unowned set
  visible in both `tome status` and `tome doctor` output; the action
  decision (`tome reassign` to re-anchor or `tome remove skill` to
  delete) is the user's call, not a diagnosis.

  JSON shape: `unowned: [SkillSummary]` array on both `StatusReport` and
  `DoctorReport`. `SkillSummary` is a new public type:
  ```rust
  #[derive(Debug, Clone, serde::Serialize)]
  pub struct SkillSummary {
      pub name: String,
      pub previous_source: Option<String>, // DirectoryName as string
      pub source_path_display: String,     // collapse_home rendering
      pub synced_at: String,
      pub managed: bool,
  }
  ```
  `previous_source` is the clean directory name when present;
  `source_path_display` is always populated (the fallback for D-C2 and
  supplementary info even when `previous_source` is set).

### Carried forward from prior phases (locked, do not re-decide)

- **Phase 11 D-12, D-14:** `source_name: Option<DirectoryName>` schema
  on `SkillEntry` and `LockEntry`. Phase 14 reads this; doesn't change.
- **Phase 11 D-09, D-10:** Source removal triggers (cleanup orphan
  detection + `tome remove dir`) already transition entries to Unowned.
  Phase 14 only adds USER-FACING lifecycle on top.
- **Phase 11 D-11:** Unowned skills distribute to targets normally;
  disabling distribution is `tome remove skill` (this phase) or
  `machine.toml::disabled`. Phase 14's distribute path is unchanged.
- **Phase 11 D-13:** `SkillEntry::new_unowned` constructor exists.
  Phase 14 either calls it directly OR uses `entry.source_name = None`
  in-place transitions. The `#[allow(dead_code)]` on `new_unowned` can
  be dropped (Phase 14 has callers).
- **Phase 13 D-13:** Fork-in-place is lossy for pre-Phase-14 entries.
  Closed forward via D-C1 (capture `previous_source` at fork time) and
  D-C2 (graceful fallback for pre-existing entries). Phase 13's
  `apply_edit_decisions` callsite needs a small patch in Phase 14.
- **Phase 13 D-12:** `tome doctor` reports drift unconditionally.
  Phase 14 adds an unowned section; doesn't touch drift reporting.
- **Phase 8 SAFE-01:** Grouped failure summary pattern. Phase 14
  follows it for `tome remove skill` (new `RemoveSkillFailureKind`
  enum mirroring `RemoveFailure` shape).
- **Phase 10 POLISH-04:** Compile-time exhaustiveness guard for
  `FailureKind::ALL`-style arrays. Phase 14's `RemoveSkillFailureKind`
  uses the same `const _: () = { assert!(...len() == N); };` guard.
- **Plan/render/execute pattern** for any flow that mutates filesystem
  state. Both `tome remove skill` and the extended `tome reassign`
  follow this; `--dry-run` is free.
- **Atomic temp+rename** for any on-disk writes (`manifest::save`,
  `lockfile::save`, `machine::save`). Phase 14 calls all three for
  `tome remove skill`'s cleanup loop.

### Claude's Discretion

The following are implementation details not worth user input; they
follow established codebase conventions:

- Exact wording of error messages (within the existing Conflict / Why /
  Suggestion template per Phase 7 D-10) — D-A1, D-A2, D-B2.
- Exact prompt copy text for confirmation prompts (within the bounds
  of D-B3).
- Internal organisation of the new `tome remove skill` work — whether
  it lives in `remove.rs` (alongside `tome remove dir`) or a new
  `remove_skill.rs` module. Recommendation: extend `remove.rs` with a
  `pub(crate) plan_skill / render_skill_plan / execute_skill` set
  mirroring the existing `dir`-flavoured triple, sharing the
  `RemoveFailure` infrastructure.
- Whether `RemoveSkillFailureKind` is a separate enum or `RemoveFailure`
  gets a generic `kind` parameter. Recommendation: separate enum
  (different failure modes from `dir` removal — e.g. `LibraryDir`,
  `DistributionSymlink`, `Lockfile`, `MachineToml` rather than
  `dir`-flavoured `DistributionSymlink` + `GitCache`).
- Whether `SkillSummary` lives in `status.rs`, `doctor.rs`, or a new
  shared module. Recommendation: a new `summary.rs` (or in `manifest.rs`
  near `SkillEntry`) since both `status::StatusReport` and
  `doctor::DoctorReport` consume it.
- The `tome reassign` `--force` flag's interaction with `--dry-run`
  (recommendation: `--dry-run` always wins; `--force` is an apply-time
  decision).
- Whether the unowned section in `tome doctor`'s text rendering reuses
  `render_issues_for_directory`-style helpers or has its own renderer.
  Recommendation: own renderer (different data shape — no severity).
- Per-directory ordering of unowned skills in the table (recommendation:
  sorted by name, ASC).

### Folded Todos

(None — `gsd-tools.cjs todo match-phase 14` returned 0 matches.)

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### v0.10 design + planning

- `.planning/research/v0.10-library-canonical-design.md` §"Source removal
  preserves library content" — original `tome adopt`/`tome forget` design
  intent; superseded by D-API-1 and D-API-2.
- `.planning/REQUIREMENTS.md` §"Unowned-library lifecycle (UNOWN)" —
  UNOWN-01..03 verbatim. **Note for traceability:** UNOWN-01's "tome adopt"
  wording is superseded by D-API-1 (folded into `tome reassign`); UNOWN-02's
  "tome forget" wording is superseded by D-API-2 (subcommand on `tome
  remove`). Behaviour delivered in full; verbs are different. Planner
  should flag this in the traceability check (mirrors how Phase 13's D-01
  flagged RECON-01's "version differs" wording).
- `.planning/ROADMAP.md` §"Phase 14: Unowned-library lifecycle" — success
  criteria 1–3 are the verification anchors; **the wording of criteria
  1 and 2** ("tome adopt..." and "tome forget...") is superseded by
  D-API-1/-2 and needs updating in `gsd-roadmap` post-merge or as
  part of Phase 14's plan checklist.
- `.planning/PROJECT.md` §"Key Decisions" line 142 ("`tome adopt
  <skill> <dir>` and `tome forget <skill>`") and the Decisions table
  entry ("`tome adopt`/`forget` for unowned library entries (v0.10 /
  D-LIB-04)") need text updates to reflect the merge — planner picks
  this up.

### Phase 11 (predecessor — manifest schema + transition logic)

- `.planning/phases/11-library-canonical-core/11-CONTEXT.md` — D-09, D-10,
  D-11, D-12, D-13, D-14 lock the manifest/lockfile schema and transition
  triggers Phase 14 builds on. D-13 specifically left `new_unowned`
  with `#[allow(dead_code)]` for Phase 14 to consume.

### Phase 13 (predecessor — fork-in-place flip + lockfile reconciliation)

- `.planning/phases/13-lockfile-authoritative-sync/13-CONTEXT.md` —
  D-13 (fork-in-place lossy semantic) is what D-C1 retroactively
  patches. D-12 (doctor reports drift unconditionally) is the pattern
  for D-D3's informational treatment of unowned. The
  `apply_edit_decisions` callsite Phase 13 added needs a small patch
  in Phase 14 to capture `previous_source` (D-C1).

### Codebase modules being changed in Phase 14

- `crates/tome/src/cli.rs::Command` — `Remove { ... }` variant becomes
  `Remove { #[command(subcommand)] kind: RemoveKind }` per D-API-2.
  `Reassign { ... }` adds `#[arg(long)] force: bool` per D-A1.
- `crates/tome/src/reassign.rs` — drop the Unowned refusal at lines
  58-63 (D-API-1). Add content-hash check in `plan` (D-A1). Add target
  role validation in `plan` (D-A2). Update `ReassignPlan.from_directory`
  to `Option<DirectoryName>` (matches new schema). Update `render_plan`
  to handle the `None` case ("Unowned" instead of a source dir name).
  Patch `execute` to clear `previous_source` on re-anchor (D-C1).
- `crates/tome/src/remove.rs` — refactor: today's `plan`/`render_plan`/
  `execute` for the `dir` flavour stays; add new
  `plan_skill`/`render_skill_plan`/`execute_skill` for the `skill`
  flavour (D-API-2 + D-B1..D-B3). Likely add a new
  `RemoveSkillFailureKind` enum mirroring `FailureKind`. The
  `dir` execute path also needs the `previous_source = old source_name`
  capture before flipping to Unowned (D-C1, transition site 2).
- `crates/tome/src/lib.rs::run` — dispatch `Command::Remove { kind:
  RemoveKind::Dir { ... } }` to `remove::dir_*` (existing call shape) and
  `Command::Remove { kind: RemoveKind::Skill { ... } }` to the new
  `remove::skill_*` flow.
- `crates/tome/src/cleanup.rs::cleanup_library` Case 1 — capture
  `previous_source = entry.source_name.clone()` before setting
  `entry.source_name = None` (D-C1 transition site 1).
- `crates/tome/src/reconcile.rs::apply_edit_decisions` — fork-in-place
  flip captures `previous_source = old source_name` before flipping
  manifest fields (D-C1 transition site 3).
- `crates/tome/src/manifest.rs::SkillEntry` — add
  `previous_source: Option<DirectoryName>` field with serde defaults.
  Update `SkillEntry::new` to default `previous_source = None` (Owned
  skills have no previous). Update `SkillEntry::new_unowned` signature
  to optionally accept previous_source. Drop the `#[allow(dead_code)]`
  on `new_unowned` (consumed by Phase 14).
- `crates/tome/src/lockfile.rs::LockEntry` — same field addition for
  cross-machine surfacing symmetry (D-C1).
- `crates/tome/src/machine.rs::MachinePrefs` — no schema change. The
  `tome remove skill` cleanup mutates the existing `disabled` set and
  per-directory `enabled`/`disabled` lists (D-B1).
- `crates/tome/src/status.rs::StatusReport` — add
  `unowned: Vec<SkillSummary>` field. `gather` populates it by
  iterating `manifest.iter()` filtering for `source_name.is_none()`.
  `render_status` adds the Unowned-skills section between Directories
  and Health (D-D1, D-D2). JSON shape is the new field (D-D3 anchor).
- `crates/tome/src/doctor.rs::DoctorReport` — add
  `unowned_skills: Vec<SkillSummary>` field. `check` populates it.
  Render in a parallel section that does NOT contribute to
  `total_issues` (D-D3). JSON shape is the new field.
- `crates/tome/src/distribute.rs` — no behaviour change. Unowned
  skills already distribute (Phase 11 D-11). The skill-removal cleanup
  in `tome remove skill` does its own distribution-symlink cleanup
  rather than going through `distribute`.

### New types likely added

- `crates/tome/src/cli.rs::RemoveKind { Dir { name, force }, Skill { name, yes } }`
- `crates/tome/src/remove.rs::RemoveSkillPlan` (mirror of `RemovePlan`)
- `crates/tome/src/remove.rs::RemoveSkillFailureKind` (with `ALL` array
  + compile-time exhaustiveness guard, mirror of `FailureKind` shape;
  variants likely: `LibraryDir`, `DistributionSymlink`, `Lockfile`,
  `MachineToml`)
- `crates/tome/src/summary.rs::SkillSummary` (or in `manifest.rs`) —
  shared between `status::StatusReport` and `doctor::DoctorReport`.

### Patterns to follow (no behaviour change to these modules; prior art)

- `crates/tome/src/remove.rs::FailureKind`, `RemoveFailure`,
  `FailureKind::ALL` + compile-time exhaustiveness assertion (Phase 8
  SAFE-01 + Phase 10 POLISH-04). Direct model for
  `RemoveSkillFailureKind`.
- `crates/tome/src/reassign.rs::ReassignAction`, `ReassignPlan`,
  `plan`/`render_plan`/`execute` triple. Direct model for the
  `tome remove skill` triple.
- `crates/tome/src/manifest.rs::save`, `lockfile.rs::save`,
  `machine.rs::save` — atomic temp+rename pattern. `tome remove skill`
  calls all three at the end of its cleanup loop.
- `crates/tome/src/status.rs::format_dir_path_column` + tabled rendering
  with `Style::blank()` + bold header — direct model for the Unowned
  section's table renderer.
- `crates/tome/src/doctor.rs::DiagnosticIssue`, `IssueSeverity` — show
  what the existing `Error`/`Warning`-style rendering looks like.
  Phase 14's unowned section is parallel (separate field, separate
  renderer, no severity).
- `crates/tome/src/paths::collapse_home` — used everywhere user-facing
  paths render. D-C2 fallback uses it; D-D1 LAST-KNOWN SOURCE column
  uses it for the path-fallback case.
- `dialoguer::Confirm::default(false)` — pattern for D-B3's destructive-
  default-no prompt. Verify the existing `tome remove dir`
  confirmation matches; if not, Phase 14 standardises both via this
  decision.

### Tests to write (not exhaustive — research/planner can extend)

- Unit: `manifest.rs::tests` — `previous_source` round-trip
  (`#[serde(default, skip_serializing_if = "Option::is_none")]` shape);
  old-shape manifest deserialise without `previous_source` key.
- Unit: `lockfile.rs::tests` — `previous_source` round-trip on
  `LockEntry`.
- Unit: `cleanup.rs::tests` — Case 1 transition records
  `previous_source = old source_name` before flipping `source_name = None`.
- Unit: `remove.rs::tests` — `dir`-flavour execute records
  `previous_source` for each owned manifest entry it transitions.
- Unit: `remove.rs::tests` — `skill`-flavour: refuse on Owned (D-B2);
  cleanup scope (D-B1: manifest + library + dist + lockfile + machine.toml
  paths); failure aggregation when a dist-symlink delete fails; atomic
  save round-trip on lockfile + machine.toml.
- Unit: `reassign.rs::tests` — Unowned input no longer refused (D-API-1);
  content-hash check refuses on different-content (D-A1); `--force`
  bypasses; same-content collision relinks; target-only role rejected
  (D-A2); re-anchor clears `previous_source` (D-C1 closure).
- Unit: `status.rs::tests` — `gather` populates `unowned` field;
  rendering omits cleanly on empty set; rendering shows table with
  expected columns (D-D1, D-D2).
- Unit: `doctor.rs::tests` — `check` populates `unowned_skills` field;
  unowned skills don't contribute to `total_issues` (D-D3); JSON shape
  matches.
- Integration (`tests/cli.rs` — to be split per HARD-13 in Phase 15):
  - Full reassign flow with Unowned input (`tome reassign foo --to bar`
    where `foo.source_name == None`).
  - `tome reassign foo --to target-only-dir` rejected.
  - `tome reassign foo --to bar` with content collision: refused; with
    `--force`: succeeds.
  - `tome remove skill foo` happy path (manifest + library + dist
    + lockfile + machine.toml all cleaned).
  - `tome remove skill foo` on Owned skill: refused with hint.
  - `tome remove skill foo --yes` skips confirmation.
  - `tome status` text + `--json` show Unowned section correctly;
    empty case omits cleanly.
  - `tome doctor` text + `--json` show Unowned section correctly;
    `total_issues` doesn't count unowned skills.

### Adjacent issues (won't fix in Phase 14, but be aware)

- **HARD-02** (Phase 15): `lib.rs::run` decomposition. Phase 14's new
  `Command::Remove { kind: RemoveKind::Skill { ... } }` dispatch follows
  current pattern (inline match arm); Phase 15's `cmd_remove` helper
  decomposes it.
- **HARD-13** (Phase 15): `tests/cli.rs` split. Phase 14's new
  integration tests land in `tests/cli.rs` today; Phase 15 splits them
  into `tests/cli_remove.rs` + `tests/cli_status.rs` + `tests/cli_reassign.rs`.
- **HARD-22** (Phase 15): `Config::save_checked` tilde preservation.
  Phase 14's `tome remove skill` doesn't write tome.toml; unaffected.
- **UX-01** (Phase 16): cleanup-message rewrite. Phase 14 may emit
  "info: skill 'foo' (from 'bar') no longer in any source — preserving
  as Unowned" (today's Phase 11 wording) during sync; Phase 16
  rewrites this into the 3-bucket UX.

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable assets

- **`crates/tome/src/reassign.rs::plan, render_plan, execute`** — three-
  function plan/render/execute triple; mirror for `tome remove skill`.
  Drop the `from_directory` Unowned-refusal in `plan` to deliver
  D-API-1.
- **`crates/tome/src/remove.rs::FailureKind`, `RemoveFailure`,
  `FailureKind::ALL` + compile-time guard** — direct model for
  `RemoveSkillFailureKind`.
- **`crates/tome/src/manifest.rs::Manifest::skills_get_mut`** —
  `pub(crate)` raw mutable access. Used by `cleanup_library` for
  in-place transitions; Phase 14 uses it for `previous_source` capture.
- **`crates/tome/src/manifest.rs::hash_directory`** — deterministic
  SHA-256 directory hash. Used by D-A1's content-hash collision check.
- **`crates/tome/src/paths::collapse_home`** — path display helper.
  D-C2 fallback + D-D1 LAST-KNOWN SOURCE rendering both use it.
- **`crates/tome/src/status.rs` tabled rendering** — `Table::from_iter`
  + `Style::blank()` + bold header pattern. D-D1 reuses it.
- **`dialoguer::Confirm::default(false)`** — D-B3 uses default no.
  Same pattern as existing `tome remove dir` confirmation.

### Established patterns

- **Plan/render/execute** for any flow that mutates filesystem state
  (`add`, `remove`, `reassign`, `relocate`, `eject`, `migrate-library`).
  Phase 14's `tome remove skill` and the extended `tome reassign`
  follow it; `--dry-run` is free.
- **Atomic temp+rename** for any on-disk writes
  (`manifest::save`/`lockfile::save`/`machine::save`).
- **SAFE-01 grouped failure summary** (Phase 8) + compile-time
  exhaustiveness assertion (Phase 10 POLISH-04). Direct mirror for
  `RemoveSkillFailureKind`.
- **`#[serde(default, skip_serializing_if = "Option::is_none")]`** for
  new optional fields (D-C1's `previous_source` adds two: one on
  `SkillEntry`, one on `LockEntry`).
- **Newtype + `#[serde(transparent)]` + custom validating Deserialize**
  for `DirectoryName`. `Option<DirectoryName>` inherits this for free
  on the new field.
- **`anyhow::Result + .with_context()`** everywhere; non-zero exit on
  partial failure; SAFE-01 grouped failure rendering.
- **Plan-flag carriage** for `--dry-run` and `--force`-style flags via
  `Plan` struct fields, not free-function parameters. Today's `RemovePlan`
  doesn't carry `force`; Phase 14's `RemoveSkillPlan` follows whatever
  shape the planner picks (recommendation: carry it on the plan for
  symmetry with `--dry-run`).

### Integration points

- **`crates/tome/src/lib.rs::run`** — dispatch `Command::Remove { kind }`
  into the new `remove::dir_*` and `remove::skill_*` flows.
  `Command::Reassign { ..., force }` propagates the new `--force` flag
  into `reassign::plan`.
- **`crates/tome/src/lib.rs::sync`** — unchanged in Phase 14. The
  cleanup phase (Phase 11 D-09 Case 1) gains a one-line capture of
  `previous_source` before the in-place transition; this is the only
  sync-flow change.
- **`crates/tome/src/reconcile.rs::apply_edit_decisions`** — fork-in-
  place flip captures `previous_source = old source_name` before
  flipping (D-C1 transition site 3). Phase 13's logic stays; one
  additional line.
- **`crates/tome/src/cli.rs::Command::Remove`** — variant becomes
  `Remove { #[command(subcommand)] kind: RemoveKind }` (D-API-2).
- **`crates/tome/src/cli.rs::Command::Reassign`** — adds
  `#[arg(long)] force: bool` (D-A1).
- **`crates/tome/src/status.rs::StatusReport`,
  `crates/tome/src/doctor.rs::DoctorReport`** — both gain a parallel
  field for unowned skills (D-D2, D-D3). Both rendering paths add a
  new section (D-D1).

### Constraints from existing architecture

- Phase 11 made `consolidate_local` and `consolidate_managed` both
  produce real-directory copies in the library. Phase 14's
  `tome remove skill` deletes a real directory; behaves the same for
  managed and local. No conditional logic on `managed` flag for
  the deletion path.
- `cleanup::cleanup_library` Case 1's transition is the canonical
  Owned → Unowned mover. Phase 14 doesn't add a parallel transition
  path; it just enriches the data captured at the existing transition
  sites (D-C1).
- `tome.lock` is per-machine generated but committed to git (Martin's
  dotfiles workflow). D-B1's lockfile cleanup ensures cross-machine
  drift doesn't surprise machine B with a phantom skill (RECON-02).
- `machine.toml` is per-machine, NOT committed to git. D-B1's
  machine.toml cleanup is local hygiene only — won't affect other
  machines but keeps this machine tidy.

</code_context>

<specifics>
## Specific Ideas

- **API merge as a documentation problem more than a code problem.**
  The user pushed back on the proposed `tome adopt`/`tome forget`
  vocabulary because: (a) `adopt` and `reassign` do nearly identical
  mechanical work, (b) `forget` is a sibling of `tome remove` rather
  than a separate verb. The merge produces a smaller, more discoverable
  CLI surface (`tome reassign` covers the full re-anchor space;
  `tome remove dir`/`skill` covers the full delete space) at the cost
  of a doc update across REQUIREMENTS.md, ROADMAP.md, PROJECT.md, and
  CHANGELOG.md.

- **The `reassign.rs:60` stub literally points to "Phase 14".** The
  message reads `"use \`tome adopt\` (Phase 14) to assign a directory
  before reassigning"`. Deleting that error path AS PART OF Phase 14
  closes the loop tidily; the planner's traceability check will catch
  the wording mismatch if it's left dangling.

- **D-C1's `previous_source` is the data that should have lived on
  `SkillEntry` since Phase 11.** Phase 11 D-13 acknowledged the lossy
  gap; Phase 13 D-13 inherited it; Phase 14 closes it. This isn't a
  policy reversal — it's the natural moment to add the field, when
  the surfacing finally has a consumer (UNOWN-03 status/doctor).

- **`tome remove skill` cleanup scope deliberately bigger than the
  UNOWN-02 floor.** UNOWN-02 says "manifest entry, library directory,
  downstream distribution symlinks". D-B1 adds lockfile + machine.toml
  per-directory enable/disable lists because (a) the dotfiles cross-
  machine workflow makes lockfile drift a real footgun, and
  (b) machine.toml hygiene is essentially free to do at remove time.
  These are conservative additions that prevent surprising downstream
  behaviour rather than scope creep.

- **No content-hash check on `tome remove skill` itself.** The user
  is intentionally deleting; we don't need a "different content"
  guard like reassign's D-A1 (the destination is /dev/null, not a
  potentially-divergent dir). The confirmation prompt (D-B3) is the
  only safety net.

</specifics>

<deferred>
## Deferred Ideas

- **Bulk operations on the unowned set.** `tome reassign --all-unowned
  --to <dir>` and `tome remove skill --orphans-only` came up
  conceptually but neither is in v0.10. Possible v0.11+ if the
  unowned set grows large in real use.

- **Per-cause grouping in status/doctor rendering.** "Group unowned
  skills by transition cause (cleanup-orphan vs `tome remove dir` vs
  fork-in-place)" was rejected as visually noisy (D-D1). If user
  feedback later wants this, the `previous_source` + `managed` fields
  give us enough to derive the grouping without schema changes.

- **`previous_source` backfill for pre-Phase-14 entries.** D-C2
  intentionally skips backfill — pre-Phase-14 fork-in-place entries
  fall back to `source_path` rendering. If this proves insufficient
  in practice, a one-shot `tome migrate-library --backfill-provenance`
  could add it; not in v0.10.

- **`tome reassign --force` interaction with Owned source-side
  collisions.** D-A1's `--force` only addresses target-side collisions
  (different content at `<to>/<skill>`). If the source dir and library
  ever drift (e.g. user manually edits the library copy of an Owned
  skill), reassign would carry the library version forward. This is
  pre-existing behaviour, not Phase-14 work.

- **REQUIREMENTS.md / ROADMAP.md / PROJECT.md text updates** to remove
  `tome adopt`/`tome forget` mentions and replace with the merge
  vocabulary. Mechanically a planning-doc edit; the planner picks this
  up and lands it as part of the phase plan (or a sibling planning-doc
  PR before phase execution starts).

### Reviewed Todos (not folded)

(None — `gsd-tools.cjs todo match-phase 14` returned 0 matches.)

</deferred>

---

*Phase: 14-unowned-library-lifecycle*
*Context gathered: 2026-05-07*
