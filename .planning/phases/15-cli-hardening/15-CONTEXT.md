# Phase 15: CLI hardening - Context

**Gathered:** 2026-05-08
**Status:** Ready for planning

<domain>
## Phase Boundary

Land the v0.10 **beta cut** as a single hardening pass — 22 HARD-* requirements
covering refactors, safety guards, test coverage, polish, and older bug
backlog. The bundle is justified because most items touch modules already in
flux from Phases 11-14 (`lib.rs`, `config.rs`, `manifest.rs`, `lockfile.rs`,
`tests/cli.rs`, `browse/`, `wizard.rs`); doing them together is more efficient
than serialising. Beta-cut quality bar: clippy-D-warnings clean on Linux +
macOS, ≥720 tests, CI green. **No new user-facing features.**

**In scope (HARD-01..22):**

- **Architecture refactors (HARD-01..07):** `skill::parse → anyhow::Result`;
  `lib.rs::run` decomposition into per-subcommand `cmd_<name>` helpers;
  `config.rs` module split; `process::exit(1)` → downcastable `LintFailed`
  error; `scan_for_skills` → `ScanMode` enum; `Lockfile` field visibility
  tightening; `(verbose, quiet)` flags → `LogLevel` enum.
- **Safety + tests (HARD-08..14):** atomic-save preservation regression;
  `distribute` foreign-symlink protection (D-DIST-1/-2); hostile-input tests
  for `[directory_overrides]`; `tome remove dir <git-dir>` /
  `tome remove dir <claude-plugins-dir>` integration tests; ratatui
  `TestBackend` + `insta` snapshots for `browse/ui.rs`; `tests/cli.rs` split
  into per-domain files; `backup::tests::push_and_pull_roundtrip` flake
  fix.
- **Polish + older bugs (HARD-15..22):** `wizard.rs` `eprintln!` discipline;
  rename `provenance_from_link_result → warn_if_unreadable_symlink`;
  `TryFrom<String>` impls for `SkillName` / `DirectoryName`; `tome relocate`
  cross-fs cleanup recovery hint; `tome reassign` read-once filesystem
  state (eliminate plan/execute drift); manifest epoch-0 timestamp warning;
  browse UI Disable/Enable wired up (D-BROWSE-1..3); `Config::save_checked`
  tilde preservation (D-TILDE-1/-2).

**Out of scope** (handled by other phases or out of v0.10 entirely):

- Cleanup-message UX rewrite (3-bucket partition for stale skills) →
  Phase 16 (UX-01).
- Library-canonical model + adapter trait + lockfile-authoritative sync →
  shipped in Phases 11-13 (LIB-01..05, ADP-01..04, RECON-01..05).
- Unowned-library lifecycle (`tome reassign` / `tome remove skill` /
  `tome status` / `tome doctor` surfacing) → shipped in Phase 14
  (UNOWN-01..03).
- Documentation updates (`docs/src/architecture.md`, `CHANGELOG.md` v0.10
  release notes, `docs/src/cross-machine-sync.md`) → Phase 16 (DOC-01..03).
- Migration polish + UAT + cargo-dist release → Phase 17 (REL-01..05).
- New top-level commands or new user-facing features. Phase 15 is
  hardening, not feature work.
- Mid-phase scope expansion: per **D-PLAN-2 strict beta-cut scope**, new
  issues surfaced during execution go to Phase 16/17 or backlog, not into
  this phase.

</domain>

<decisions>
## Implementation Decisions

### Plan grouping & sequencing

- **D-PLAN-1 (module-touch grouping):** 22 HARD-* requirements organise
  into **6 plans** by which modules each touches, averaging ~3.7
  reqs/plan. Matches v0.10 phase pattern (Phase 11 = 5 plans, Phase 14 =
  8 plans). Better wave parallelisation than cluster-aligned grouping
  (which would yield 3 dense plans of 7-8 reqs each) and lower planning
  overhead than fine-grained one-HARD-per-plan (22 plans).

  ```
  Plan 15-01: cli.rs decomposition
              HARD-02 (lib.rs::run → cmd_<name>)
              HARD-13 (tests/cli.rs split + common/ helpers)
  Plan 15-02: config module
              HARD-03 (config.rs split into config/{mod,types,overrides,validate}.rs)
              HARD-22 (Config::save_checked tilde preservation per D-TILDE-1/-2)
  Plan 15-03: Type-system tightening
              HARD-01 (skill::parse → anyhow::Result)
              HARD-05 (scan_for_skills → ScanMode enum)
              HARD-06 (Lockfile fields → pub(crate) + accessors)
              HARD-07 ((verbose, quiet) → LogLevel enum)
              HARD-17 (TryFrom<String> for SkillName/DirectoryName)
  Plan 15-04: Safety guards + integration tests
              HARD-04 (LintFailed error + main.rs exit-code mapping)
              HARD-08 (atomic-save preservation regression test)
              HARD-09 (distribute foreign-symlink refuse per D-DIST-1/-2)
              HARD-10 (hostile-input tests for [directory_overrides])
              HARD-11 (tome remove dir <git-dir>/<claude-plugins-dir> e2e tests)
  Plan 15-05: Browse UI
              HARD-12 (browse/ui.rs ratatui TestBackend + insta snapshots)
              HARD-21 (Disable/Enable wired per D-BROWSE-1..3)
  Plan 15-06: Polish + older bugs
              HARD-14 (backup test flake fix — disable git signing)
              HARD-15 (wizard.rs println → eprintln!)
              HARD-16 (rename provenance_from_link_result → warn_if_unreadable_symlink)
              HARD-18 (tome relocate cross-fs cleanup recovery hint)
              HARD-19 (tome reassign read-once filesystem state)
              HARD-20 (manifest epoch-0 timestamp warning)
  ```

  **Wave structure:**
  - Wave 1 (parallel): 15-01, 15-02, 15-03 — independent module surfaces,
    no shared file mutations.
  - Wave 2 (after Wave 1): 15-04, 15-05, 15-06 — depend on Wave 1
    landings (15-04's `tome remove dir` integration tests may want to
    land in the new `cli_remove.rs` from 15-01; 15-04's HARD-04 depends
    on the HARD-02 dispatch shape; 15-05's snapshot-test scaffold may
    benefit from 15-01's helper extraction).

  Sub-plan sequencing inside a plan is at the planner's discretion; the
  module-touch grouping makes per-plan diffs locally coherent.

- **D-PLAN-2 (strict beta-cut scope):** HARD-01..22 only. New issues
  surfaced during execution go to Phase 16/17 or backlog (per-phase
  `deferred-items.md` parking lot pattern, established Phase 11 onward).
  No mid-phase fold-ins, even for trivial same-module fixes. Predictable
  scope protects v0.10 ship date; the bundle is already wide.

  Operationally: if execution surfaces a HARD-23-shaped issue, the
  executor records it in `15-deferred-items.md` and continues; the
  triage decision (Phase 16 / Phase 17 / backlog) happens at
  `/gsd:verify-work 15` time.

### HARD-22 tilde preservation

- **D-TILDE-1 (auto-portable normalisation on save):** When
  `Config::save_checked` writes `tome.toml`, paths under `$HOME` are
  emitted as `~`-shape; paths outside `$HOME` stay absolute. New
  `unexpand_tilde()` helper inverts `paths::expand_tilde()`. The save
  path no longer calls `expand_tildes()` on the serialised copy;
  validation runs against an in-memory expanded clone, but the on-disk
  representation gets the `unexpand_tilde()` pass.

  ```
  config.toml IN  : library_dir = "~/skills"
  config.toml OUT : library_dir = "~/skills"          ✓ (preserved)

  config.toml IN  : library_dir = "/Users/martin/skills"
  config.toml OUT : library_dir = "~/skills"          (rewritten — auto-portable)

  config.toml IN  : library_dir = "/var/lib/skills"
  config.toml OUT : library_dir = "/var/lib/skills"   (no ~ possible — kept absolute)
  ```

  **Trade-off accepted:** literal absolute paths the user wrote get
  rewritten to `~`-shape if they're under `$HOME`. The cross-machine
  dotfiles workflow benefit (single `tome.toml` works on every machine
  without per-machine path edits) outweighs the surprise of seeing
  `~/skills` appear where `/Users/martin/skills` was typed. v0.9's
  `[directory_overrides.<name>]` (PORT-01..05) handles per-machine
  divergence; this rule keeps `tome.toml` portable.

  **Override interaction:** `Config::apply_machine_overrides` mutates a
  load-time-only copy. `save_checked` operates on the unmutated config,
  so override paths from `machine.toml` are NEVER written back to
  `tome.toml`. Mutating commands (`tome add`, `tome remove dir`,
  `tome reassign`, `tome fork`) work on the unmutated config and store
  user-supplied paths verbatim; the `~`-normalisation runs at serialise
  time only.

- **D-TILDE-2 (scope: tome.toml only):** The `~`-normalise rule applies
  ONLY to `Config::save_checked` (writes `tome.toml`).
  `MachinePrefs::save` (writes `machine.toml`) preserves user input
  verbatim — paths under `$HOME` written as absolute stay absolute;
  `~`-shape paths stay tilde. `machine.toml` is per-machine and never
  committed to git, so portability is not a concern; coercing user
  input would just be surprising. Particular concern: a user might
  intentionally write `/Volumes/External/skills` in a
  `[directory_overrides.<name>]` block — that path is intentionally
  absolute and machine-specific.

### HARD-21 browse Disable/Enable wiring

- **D-BROWSE-1 (smart-routing toggle scope):** When the user presses
  Disable/Enable in `tome browse`, the action mutates the most-specific
  `machine.toml` surface available:

  - **If the parent directory has a `disabled` blocklist set in
    `machine.toml::directories.<dir>.disabled`** — toggle that list
    (insert on Disable, remove on Enable).
  - **If the parent directory has an `enabled` allowlist set in
    `machine.toml::directories.<dir>.enabled`** — toggle that list
    (remove on Disable, insert on Enable). Note the inverted polarity:
    membership in `enabled` means "include"; absence means "exclude".
  - **Otherwise** — toggle the global `MachinePrefs.disabled` set.

  Honors MACH-04 mutual-exclusion: only one of `disabled` / `enabled`
  is ever set per directory; the toggle cannot accidentally violate
  the invariant because it operates on whichever list is present.

  Existing API surface in `machine.rs`: `MachinePrefs::disable_skill`
  and the per-directory mutators are pre-built; the browse wiring is
  the only new code.

- **D-BROWSE-2 (UI-explicit scope):** The action label and any
  status-message confirmation must make scope explicit. Recommended
  text shapes (planner can refine):

  - Global toggle: `Disable on this machine` / `Enable on this machine`
  - Per-directory blocklist: `Disable for <dir-name>` / `Enable for <dir-name>`
  - Per-directory allowlist: `Disable for <dir-name>` / `Enable for <dir-name>`
    (semantics differ but user-facing label can be the same — they ARE
    disabling for that directory)

  Status-message after toggle mirrors the label (e.g.,
  `StatusMessage::Success("Disabled foo for my-dir")`). The user sees
  WHICH list they mutated without needing to inspect `machine.toml`.

  Implementation note: the existing `DetailAction::label()` returns
  `&'static str` (line 178-186 in `browse/app.rs`); this lift requires
  it to become context-sensitive (return `String` or take a
  `&SkillRow` parameter). Mechanically a small refactor.

- **D-BROWSE-3 (instant toggle + StatusMessage):** Single keystroke
  applies the mutation; TUI shows a `StatusMessage::Success(...)` per
  v0.9 POLISH-02 pattern (`Success | Warning | Pending` enum with
  `body`/`glyph`/`severity` accessors). No confirmation prompt — the
  action is fully reversible (press the inverse to undo). Mirrors the
  existing TUI pattern for `CopyPath` (instant action + status
  message).

  After toggle:
  1. Mutate `MachinePrefs` in-memory.
  2. Save `machine.toml` atomically (existing temp+rename pattern).
  3. Re-render the row's action label so it flips Disable ↔ Enable
     immediately. The skill row itself stays in the list (disable
     doesn't remove from `tome browse` view; the user can re-enable
     from the same view).
  4. Surface `StatusMessage::Success("Disabled foo for my-dir")` in
     the status bar; auto-fades per existing POLISH-02 timing.

### HARD-09 distribute clobber policy

- **D-DIST-1 (warn-and-skip foreign symlinks):** When `distribute`
  finds a pre-existing symlink at the target path that points OUTSIDE
  the current library, the default behaviour is **warn-and-skip**:

  ```
  warning: ~/.claude/skills/foo is a foreign symlink
           (→ /Users/martin/other-tome/library/foo); skipping.
           Pass --force to overwrite, or remove manually.
  ```

  Increment `result.skipped`; continue with the next skill. The
  existing `force: bool` parameter (line 111 in `distribute.rs`)
  bypasses the check (opt-in clobber). No new CLI flag needed —
  reusing `force` is consistent with its existing semantic ("overwrite
  stale symlinks").

  Mirrors the existing regular-file handling at `distribute.rs:121-127`
  (warn-and-skip on non-symlink at target) and matches Phase 8 SAFE-01
  aggregated-failure surfacing pattern (one foreign symlink shouldn't
  abort distribution for every other skill).

  Detection mechanism: read the symlink target via `fs::read_link`,
  canonicalise both target and current `library_dir`, compare path
  prefixes. The "current library" is determined from `paths.library_dir`
  (the active `TomePaths`).

- **D-DIST-2 (doctor surfaces too):** Add
  `DiagnosticIssue::ForeignSymlink { target_path, actual_target }` to
  `doctor::DoctorReport` (likely in `directory_issues` since it's a
  per-directory health concern, but planner can place it elsewhere if
  the data shape doesn't fit). Renders as **Warning severity**;
  contributes to `total_issues`.

  Persistent visibility: a sync-time warning gets lost in scrollback;
  `tome doctor` shows the conflict whenever the user runs it. Mirrors
  Phase 13 D-12 ("doctor reports drift unconditionally") philosophy:
  doctor is the diagnostic tool; foreign-symlink presence is a fact.

  JSON shape: extends the existing `DiagnosticIssue` enum's serde
  representation. Backward-compat: this is a NEW variant, so
  `DiagnosticIssue::ALL` array (POLISH-04) gets one more entry; the
  compile-time exhaustiveness sentinel auto-adjusts.

### Carried forward from prior phases (locked, do not re-decide)

- **Phase 1+ (everywhere):** Plan/render/execute pattern for any
  state-mutating flow. `--dry-run` is free.
- **Phase 1+ (everywhere):** Atomic temp+rename for any on-disk writes
  (`Config::save_checked`, `MachinePrefs::save`, `Manifest::save`,
  `Lockfile::save`).
- **Phase 7 D-10:** Conflict / Why / Suggestion error template for
  user-facing `bail!` messages. Phase 15's new error sites (LintFailed,
  foreign-symlink warning, `unexpand_tilde` failures) follow this
  shape.
- **Phase 8 SAFE-01:** Grouped failure summary pattern. Phase 15
  applies it to the foreign-symlink case (warn + count, don't hard
  fail).
- **Phase 9 PORT-01..05:** `[directory_overrides.<name>]` schema in
  `machine.toml`; `Config::apply_machine_overrides` runs after
  tilde-expand at load time. **Save flow must NEVER write
  override-applied paths back to `tome.toml`** — load mutates a copy;
  the saved config preserves the original input.
- **Phase 10 POLISH-04:** Compile-time exhaustiveness guard for
  `*Kind::ALL`-style arrays. Phase 15's any-new-enum work
  (`LogLevel`, `ScanMode`, new `DiagnosticIssue` variant) gets an
  ALL-array + sentinel where appropriate.
- **Phase 10 POLISH-02:** `StatusMessage::{Success | Warning | Pending}`
  enum. HARD-21 D-BROWSE-3 toggle uses `Success`.
- **Phase 11 D-08:** Hash-based drift basis. Phase 15 doesn't change
  this; HARD-08 atomic-save tests verify hash-based round-trips.
- **Phase 11 D-12, D-14:** `source_name: Option<DirectoryName>` schema
  on `SkillEntry` and `LockEntry`. HARD-06 tightens visibility; the
  field shape is unchanged.
- **Phase 13 D-22:** Lockfile per-skill in-memory updates with atomic
  end-of-loop save. HARD-08 atomic-save tests verify the rename-failure
  preservation invariant for this path.
- **Phase 14 D-API-1, D-API-2:** `tome reassign` accepts Unowned input;
  `tome remove dir <name>` (renamed from bare `tome remove`); new
  `tome remove skill <name>` subcommand. HARD-11 integration tests
  cover the `dir` flavour for git + claude-plugins types. HARD-19
  reassign read-once builds on the now-stable `reassign::plan`/`execute`
  shape.
- **Phase 14 D-C1:** `previous_source: Option<DirectoryName>` field on
  `SkillEntry` and `LockEntry`, captured at all 3 Owned→Unowned
  transition sites. HARD-08 atomic-save tests should round-trip this
  field.
- **Project policy "Backward compat: None":** Breaking refactors
  (HARD-02 lib.rs decomposition, HARD-03 config.rs split, HARD-06
  Lockfile field visibility, HARD-07 LogLevel enum, HARD-17
  TryFrom<String>, HARD-22 tilde rewrite) ship without compat shims.
  CHANGELOG callouts in Phase 16 cover the user-visible behaviour
  changes.

### Claude's Discretion

The following are implementation details not worth user input; planner
follows established codebase conventions:

- **HARD-02 `cmd_<name>` location:** Inline in `lib.rs` (single match
  arm calls a `pub(crate) fn cmd_<name>(...)` defined later in the
  same file) or extracted to a new `commands/` module. Recommendation:
  **inline in `lib.rs` first** to land HARD-02 with minimal churn;
  the file shrinks dramatically (currently 2,251 LOC, mostly the
  `run` match) so further splitting is optional. If the planner finds
  the inline approach still leaves `lib.rs` >1,500 LOC, lift to a
  `commands/` module.
- **HARD-03 `config.rs` internal layout:** REQUIREMENTS calls for
  `config/{mod,types,overrides,validate}.rs`. Whether `paths.rs`
  helpers (e.g. `expand_tilde`, `unexpand_tilde`) live in `config/` or
  the existing `paths.rs` module is the planner's call.
  Recommendation: **keep tilde helpers in `paths.rs`** (cross-cutting
  utility); `config/overrides.rs` for `apply_machine_overrides` and
  `config/validate.rs` for `validate()` Cases A/B/C.
- **HARD-13 `tests/cli.rs` split granularity:** REQUIREMENTS suggests
  per-domain (`cli_sync.rs`, `cli_doctor.rs`, etc.) with shared
  `common/` helpers. Specific file boundaries are the planner's call;
  expected files (non-exhaustive): `cli_sync.rs`, `cli_doctor.rs`,
  `cli_remove.rs`, `cli_reassign.rs`, `cli_status.rs`, `cli_browse.rs`,
  `cli_init.rs`, `cli_migrate_library.rs`, plus the existing
  `cli_sync_reconcile.rs` (already follows the pattern). Common
  helpers go in `tests/common/mod.rs` (cargo's `common` convention
  for shared test code).
- **HARD-12 browse snapshot scope:** REQUIREMENTS lists "status
  dashboard, skill list, detail pane, help overlay". Planner adds:
  empty state (no skills), search-filter state, theme variants
  (light + dark), at minimum. `insta` snapshot density is the
  planner's call.
- **HARD-14 backup flake fix scope:** Disable git signing in test
  repos via local config (`git config commit.gpgsign false`). Whether
  this lives as a per-test setup helper or a shared `common/`
  test-helper is the planner's call. Scope is just `backup::tests`;
  other modules don't run real git commands.
- **HARD-19 reassign read-once mechanism:** Snapshot filesystem state
  in `reassign::plan`'s return value (current shape: `ReassignPlan`
  carries plan data; extend with a `pre_state: PreReassignState`
  struct that captures what was on disk at plan time). `execute`
  consumes the snapshot rather than re-reading. Implementation detail.
- **HARD-08 atomic-save test mechanism:** Whether to use a
  fault-injection layer (mock fs that fails on rename) or real fs
  with a permission-denied trick is the planner's call.
  Recommendation: **real fs** (matches existing test style; mock
  layers add infrastructure for one test).
- **HARD-04 `LintFailed` placement:** New error type in `lint.rs` or
  `errors.rs` (if a new module is justified). Recommendation: **inline
  in `lint.rs`** as a sibling type.
- **HARD-07 `LogLevel` location:** New module `crates/tome/src/log.rs`
  or inline in `cli.rs`. Recommendation: **inline in `cli.rs`** —
  it's a CLI-facing enum.
- **HARD-17 `TryFrom<String>` failure mode:** Reuse the existing
  `validate_identifier` validation; failure is the same `anyhow::Error`
  the existing `SkillName::new` returns. Implementation detail.
- **D-DIST-1 implementation detail:** Whether the canonicalisation
  uses `std::fs::canonicalize` or `path.starts_with` after lexical
  normalisation. Recommendation: **`canonicalize`** to handle
  symlinks-in-the-middle correctly.
- **D-BROWSE-2 label string format:** Exact wording within the bounds
  of "must show scope". Planner picks short, scannable copy.

### Folded Todos

(None — `gsd-tools.cjs todo match-phase 15` returned 0 matches.)

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### Phase 15 specification

- `.planning/REQUIREMENTS.md` §"CLI hardening (HARD)" — HARD-01..22
  verbatim. Each requirement names the closing GitHub issue (#485..#503,
  plus #416, #430, #433, #447, #457).
- `.planning/ROADMAP.md` §"Phase 15: CLI hardening" — success criteria
  1-4 (architecture cluster / safety+tests cluster / polish+older bugs
  cluster / CI green target). Test count target: ≥720 tests.
- `.planning/PROJECT.md` §"Current Milestone: v0.10" + §"Phase progress
  (v0.10)" — Phase 15 is the **beta cut**.

### Phase predecessors (locked decisions Phase 15 builds on)

- `.planning/phases/13-lockfile-authoritative-sync/13-CONTEXT.md` —
  D-22 (lockfile per-skill in-memory updates + atomic end-of-loop save)
  is the contract HARD-08 atomic-save tests verify. D-12 (doctor
  reports drift unconditionally) is the philosophical anchor for
  D-DIST-2 (doctor surfaces foreign symlinks too).
- `.planning/phases/14-unowned-library-lifecycle/14-CONTEXT.md` —
  D-API-1/-2 vocabulary merge ships in Phase 14; HARD-11's
  `tome remove dir <name>` integration tests use the post-merge shape
  (BREAKING from the bare `tome remove <name>`). HARD-19's `tome
  reassign` read-once builds on Phase 14's stable `reassign::plan`
  signature accepting Unowned input. Phase 14 explicitly forward-flags
  HARD-02, HARD-13, HARD-22 in its "Adjacent issues" section
  (`14-CONTEXT.md` lines 519-532).
- `.planning/phases/11-library-canonical-core/11-CONTEXT.md` — D-08
  (hash-based drift basis), D-12, D-14 (`source_name: Option<...>`
  schema). HARD-06 tightens `Lockfile` field visibility; field shape
  is unchanged.
- `.planning/phases/10/...` (POLISH-02, POLISH-04) — `StatusMessage`
  enum used by D-BROWSE-3; `*Kind::ALL` + sentinel pattern used by
  D-DIST-2's new `ForeignSymlink` variant.
- `.planning/phases/09-cross-machine-path-overrides/...` —
  PORT-01..05 schema (`[directory_overrides.<name>]` in
  `machine.toml`). D-TILDE-1's "save flow must NEVER write
  override-applied paths back" depends on PORT-02's
  `apply_machine_overrides` running on a load-time-only mutation.

### v0.10 design + planning

- `.planning/research/v0.10-library-canonical-design.md` — original
  v0.10 design exploration. Phase 15 is the operational hardening
  pass on top.
- `.planning/STATE.md` §"v0.10 design context (consume during
  planning)" — recap of milestone-level design decisions.

### Codebase modules being changed in Phase 15 (with confirmed LOC)

- `crates/tome/src/lib.rs` (2,251 LOC) — HARD-02: decompose
  `pub fn run(cli: Cli)` (entry at line 164, dispatch match at
  line 325) into per-subcommand `cmd_<name>` helpers. Existing
  `Command::Init` and `Command::Version` early-return shape (lines
  165-172) is preserved.
- `crates/tome/src/config.rs` (3,122 LOC) — HARD-03: split into
  `config/{mod,types,overrides,validate}.rs`. HARD-22: rewrite
  `Config::save_checked` (line 832) to drop the pre-save
  `expand_tildes()` call and add a `unexpand_tilde()` pass on the
  serialised copy (per D-TILDE-1).
- `crates/tome/src/cli.rs` — HARD-07: replace `verbose: bool, quiet:
  bool` (lines 31, 34-35) with `LogLevel` enum. HARD-04: `Lint`
  command exits via downcastable error rather than `process::exit(1)`
  in `lib.rs::run` (line 394).
- `crates/tome/src/skill.rs` (124 LOC) — HARD-01: change
  `pub fn parse(content: &str) -> Result<(SkillFrontmatter, String), String>`
  (line 55) to return `anyhow::Result`.
- `crates/tome/src/discover.rs` — HARD-05: replace
  `scan_for_skills(... Option<Option<SkillProvenance>>, ...)` (line
  445) with named `ScanMode` enum.
- `crates/tome/src/lockfile.rs` (938 LOC) — HARD-06: tighten
  `Lockfile.{skills,version}` to `pub(crate)` with accessor methods
  mirroring `Manifest`'s shape.
- `crates/tome/src/distribute.rs` (627 LOC) — HARD-09: extend
  symlink-handling block (lines 110-128) per D-DIST-1 — read foreign
  symlink target, warn-and-skip when outside library, reuse `force`
  param for opt-in clobber.
- `crates/tome/src/doctor.rs` — D-DIST-2: add
  `DiagnosticIssue::ForeignSymlink { target_path, actual_target }`
  variant. Update `DiagnosticIssue::ALL` + sentinel.
- `crates/tome/src/browse/app.rs` (1,415 LOC) — HARD-21: drop
  `#[allow(dead_code)]` at line 168; wire
  `DetailAction::{Disable, Enable}` per D-BROWSE-1..3. Existing
  comment at lines 337-342 ("show Disable if enabled, Enable if
  disabled — never both") points at the same render-time logic.
  Refactor `DetailAction::label()` (lines 178-186) to be context-
  sensitive per D-BROWSE-2.
- `crates/tome/src/browse/ui.rs` (537 LOC) — HARD-12: ratatui
  `TestBackend` + `insta` snapshot tests. New
  `tests/browse_snapshots/` directory.
- `crates/tome/src/wizard.rs` (1,819 LOC) — HARD-15: convert
  diagnostic `println!` calls to `eprintln!`. Mechanical replacement;
  user-facing output (the wizard's interactive prompts) stays on
  stdout.
- `crates/tome/src/relocate.rs` (1,012 LOC) — HARD-16: rename
  `provenance_from_link_result → warn_if_unreadable_symlink`.
  HARD-18: cross-fs cleanup recovery hint when orphan-copy
  preservation kicks in.
- `crates/tome/src/reassign.rs` (719 LOC) — HARD-19: snapshot
  filesystem state once in `reassign::plan`; `execute` consumes the
  snapshot rather than re-reading. Builds on Phase 14's plan/execute
  triple shape.
- `crates/tome/src/manifest.rs` (667 LOC) — HARD-20: epoch-0
  timestamp fallback. Likely in `SkillEntry::new` or a load-time
  validator. Surfaces as warning rather than silent garbage.
- `crates/tome/src/backup.rs` — HARD-14: disable git signing in test
  repos via local config (`git config commit.gpgsign false` per-test
  setup). Closes the intermittent flake.
- `crates/tome/src/remove.rs` — HARD-11: integration tests for
  `tome remove dir <git-dir>` and `tome remove dir <claude-plugins-dir>`.
  Existing `RemovePlan`/`RemoveFailureKind` infrastructure is reused.
- `crates/tome/src/paths.rs` — D-TILDE-1: new `unexpand_tilde()`
  helper (inverse of existing `expand_tilde`). HARD-03 keeps tilde
  helpers in `paths.rs` (cross-cutting; not in `config/`).
- `crates/tome/src/lint.rs` — HARD-04: new `LintFailed` error type
  (sibling to existing types). `main.rs` maps via downcast to exit
  code 1.
- `crates/tome/src/main.rs` — HARD-04: extend the existing top-level
  error handler to downcast for `LintFailed` and exit 1; other errors
  exit 1 by default (anyhow shape).
- `crates/tome/tests/cli.rs` (6,703 LOC) — HARD-13: split into
  per-domain `tests/cli_*.rs` files with shared `tests/common/`
  helpers. Existing `tests/cli_sync_reconcile.rs` already follows
  the pattern.

### Patterns to follow (no behaviour change to these modules; prior art)

- **Plan/render/execute** for any state-mutating flow (existing in
  `add.rs`, `remove.rs`, `reassign.rs`, `relocate.rs`, `eject.rs`,
  `migration_v010.rs`).
- **Atomic temp+rename** in `Config::save_checked`,
  `MachinePrefs::save`, `Manifest::save`, `Lockfile::save`. HARD-08
  regression test verifies all four preserve previous contents on
  rename failure.
- **`#[serde(default, skip_serializing_if = "Option::is_none")]`**
  for new optional fields (none added in Phase 15, but the pattern
  applies to any backward-compat schema additions).
- **Newtype + `#[serde(transparent)]` + custom validating Deserialize**
  for `DirectoryName`, `SkillName`. HARD-17's `TryFrom<String>` impls
  follow the same validation contract.
- **`anyhow::Result` + `.with_context()`** everywhere; non-zero exit
  on partial failure; SAFE-01 grouped failure rendering. HARD-01
  brings `skill::parse` into the same convention.
- **`*Kind::ALL` + compile-time exhaustiveness sentinel** (POLISH-04
  pattern in `remove.rs::FailureKind`, `marketplace.rs::InstallFailureKind`).
  Any new enum-with-array in Phase 15 (`LogLevel`, `ScanMode`,
  `DiagnosticIssue` extension) follows it.
- **`StatusMessage` enum with body/glyph/severity** (POLISH-02 in
  `browse/`). D-BROWSE-3 reuses `Success` variant.
- **`tabled::Table::from_iter` + `Style::blank()` + bold header**
  pattern (`status.rs`, `doctor.rs`). No new tables in Phase 15.

### Tests to write (not exhaustive — research/planner extends)

- **Unit (`paths.rs`):** `unexpand_tilde()` round-trips
  `expand_tilde(unexpand_tilde(x)) == x` for paths under and outside
  `$HOME`; idempotent.
- **Unit (`config.rs`):** `Config::save_checked` round-trip preserves
  `~`-shape inputs and normalises `/Users/martin/...` → `~/...` outputs;
  paths outside `$HOME` stay absolute; override paths from
  `machine.toml` are NOT serialised back.
- **Unit (`distribute.rs`):** Foreign symlink at target is detected
  and warn-skipped; `force=true` clobbers; non-symlink at target
  still warn-skips (existing behaviour, regression-pin).
- **Unit (`doctor.rs`):** `DiagnosticIssue::ForeignSymlink` variant
  contributes to `total_issues`; renders as Warning; ALL-array
  exhaustiveness asserts at compile time.
- **Unit (`browse/app.rs`):** Smart-routing toggle picks per-directory
  list when set; falls back to global when no list set; honors MACH-04
  invariant (never mutates both). Status message reflects scope.
- **Unit (`lint.rs`):** `LintFailed` downcasts; `main.rs` exits 1.
- **Unit (`discover.rs`):** `ScanMode` enum variants exercise all
  paths today's `Option<Option<SkillProvenance>>` argument covers.
- **Snapshot (`browse/ui.rs`):** ratatui `TestBackend` + `insta`
  snapshots for status dashboard, skill list, detail pane, help
  overlay; empty state; search-filter state.
- **Integration (`tests/cli_remove.rs`):** `tome remove dir
  <git-dir>` cleans git cache + dist symlinks + manifest entries
  cleanly; partial-failure surfaces via SAFE-01 grouped summary.
  Same for `<claude-plugins-dir>`.
- **Integration (`tests/cli_overrides.rs`):** Hostile-input cases —
  `..` traversal in override path; symlink loop; two directories
  overriding to the same path.
- **Integration (atomic-save):** Mid-save rename failure preserves
  previous on-disk contents for manifest, lockfile, machine.toml,
  tome.toml.
- **Integration (`tests/cli_sync.rs`):** `distribute` foreign-symlink
  case end-to-end (set up two tome installs sharing a target dir).
- **Integration (`tests/cli_browse.rs`):** Ratatui-driven end-to-end
  test for Disable/Enable flow if possible (otherwise unit-only;
  ratatui interactive test infrastructure may not cover this).
- **Integration (`tests/cli_relocate.rs`):** Cross-fs orphan-copy
  preservation now surfaces a recovery hint.

### Adjacent issues (won't fix in Phase 15, but be aware)

- **UX-01 (Phase 16):** Cleanup-message rewrite (3-bucket partition).
  Phase 15's HARD-09 distribute warning text intersects (foreign
  symlink is sync-time concern); Phase 16 rewrites the broader
  cleanup UX.
- **DOC-01..03 (Phase 16):** v0.10 architecture docs, CHANGELOG.md
  release notes. Phase 15's BREAKING refactors (HARD-02, HARD-03,
  HARD-06, HARD-07, HARD-17) get CHANGELOG entries in Phase 16.
- **REL-01 (Phase 17):** PR #484 (chore/v0.10-prep doc drift) and
  PR #504 (refactor/v0.10-phase-c type lifts) merged before Phase 17
  starts. Phase 15 doesn't depend on these directly, but conflict
  potential exists if PR #504 lifts overlapping types.
- **REL-02 (Phase 17):** Issue triage close-loops. Phase 15's
  closing PRs link via `Closes #485..` etc.; Phase 17 verifies the
  GitHub state.

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable assets

- **`crates/tome/src/paths.rs::expand_tilde`** — invertible by a new
  `unexpand_tilde` helper. D-TILDE-1 mechanism.
- **`crates/tome/src/distribute.rs::symlink_points_to`** — already
  resolves a symlink and compares targets. D-DIST-1 extends with a
  "points outside library" predicate.
- **`crates/tome/src/machine.rs::MachinePrefs::disable_skill,
  is_disabled`** — global toggle API. D-BROWSE-1 fallback path.
- **`crates/tome/src/machine.rs::DirectoryEntry.{disabled, enabled}`**
  — per-directory list mutators. D-BROWSE-1 primary path.
- **`crates/tome/src/browse/app.rs::DetailAction`** — enum already
  has `Disable`/`Enable` variants stubbed (lines 167-186);
  `#[allow(dead_code)]` drops at line 168 once wired.
- **`crates/tome/src/browse/ui.rs::StatusMessage`** (POLISH-02) —
  `Success | Warning | Pending` enum with body/glyph/severity.
  D-BROWSE-3 reuses `Success`.
- **`crates/tome/src/remove.rs::RemovePlan`,
  `reassign.rs::ReassignPlan`** — plan/render/execute triple shapes.
  HARD-19 read-once is a small extension to `ReassignPlan`.
- **`crates/tome/src/cli.rs::Verbose|Quiet` flags** — existing global
  flag wiring. HARD-07's `LogLevel` enum collapses both into one type.
- **`crates/tome/src/manifest.rs::SkillEntry` serde defaults** —
  pattern for graceful field additions (HARD-08 regression-pin
  verifies `previous_source` round-trips after Phase 14 added it).
- **`crates/tome/src/doctor.rs::DiagnosticIssue, IssueSeverity,
  DiagnosticIssue::ALL`** — extend with `ForeignSymlink` variant per
  D-DIST-2.

### Established patterns

- **Plan/render/execute** with `--dry-run` carriage on the plan struct
  (Phase 1+).
- **Atomic temp+rename** in every `*::save` method (Phase 1+).
- **SAFE-01 grouped failure summary** + **POLISH-04 ALL-array sentinel**
  (Phase 8 + Phase 10).
- **Conflict / Why / Suggestion** error template for user-facing
  `bail!` messages (Phase 7 D-10).
- **`#[serde(default, skip_serializing_if = "Option::is_none")]`** for
  optional schema fields.
- **Newtype + `#[serde(transparent)]` + custom validating Deserialize**
  for `DirectoryName` / `SkillName`.
- **`anyhow::Result + .with_context()`** for application errors;
  `bail!` for early returns; `ensure!` for invariants.
- **Apply-overrides-on-a-load-copy** (Phase 9 D-2): `Config::expand_tildes`
  + `Config::apply_machine_overrides` mutate at load time only;
  `save_checked` operates on the unmutated config.

### Integration points

- **`crates/tome/src/lib.rs::run`** — HARD-02 decomposition target;
  also dispatches `Command::Lint` (line 377) where HARD-04 replaces
  the `process::exit(1)` at line 394.
- **`crates/tome/src/cli.rs::Cli`** — HARD-07's `LogLevel` enum
  replaces `verbose: bool, quiet: bool` (lines 31, 34-35).
- **`crates/tome/src/main.rs`** — HARD-04: extend top-level error
  handler to downcast `LintFailed` and exit 1.
- **`crates/tome/src/config.rs::Config::save_checked`** (line 832) —
  D-TILDE-1 mechanism. `apply_machine_overrides` (line 663+) is
  unaffected (it doesn't run on the save path).
- **`crates/tome/src/distribute.rs::distribute_to_target`** — HARD-09
  symlink-handling block (lines 110-128).
- **`crates/tome/src/browse/app.rs::App` + `DetailAction`** — HARD-21
  wiring + label refactor.
- **`crates/tome/src/doctor.rs::DoctorReport`** — D-DIST-2
  `ForeignSymlink` variant addition.
- **`crates/tome/tests/cli.rs`** (6,703 LOC) — HARD-13 split source.
  HARD-11 (remove dir tests), Phase 14's existing integration tests,
  and any new Phase 15 tests all flow through the same split.

### Constraints from existing architecture

- Phase 9 PORT-02 (`apply_machine_overrides`) runs at load time on a
  load-time-only mutation. The save path operates on the unmutated
  config — this is the load-vs-save asymmetry HARD-22 D-TILDE-1
  depends on (override paths from `machine.toml` MUST NOT be
  serialised into `tome.toml`).
- Phase 11 D-08 made hash-based drift the canonical drift signal.
  HARD-08 atomic-save tests verify hash round-trips for manifest +
  lockfile.
- Phase 14 D-API-2 made `tome remove dir <name>` the renamed shape.
  HARD-11 tests use the post-merge name.
- `MachinePrefs::DirectoryEntry` (machine.rs:54) carries a
  mutually-exclusive `disabled` blocklist OR `enabled` allowlist
  (MACH-04). D-BROWSE-1 smart-routing must honor the invariant.
- `tome.lock` is per-machine generated but committed to git
  (Martin's dotfiles workflow). HARD-06 visibility tightening doesn't
  change schema; Phase 11 D-12, D-14 stay locked.
- `machine.toml` is per-machine and NOT committed to git. D-TILDE-2
  preserves user input verbatim there; portability concerns don't
  apply.
- Phase 15 has 22 requirements — the largest in v0.10. Module-touch
  grouping (D-PLAN-1) keeps each plan locally coherent and
  reviewable; strict beta-cut scope (D-PLAN-2) protects the v0.10
  ship date.

</code_context>

<specifics>
## Specific Ideas

- **Module-touch grouping is the v0.10 housekeeping pattern.** Phase 11
  used 5 plans by concern; Phase 14 used 8 plans by concern. Phase 15's
  22 reqs grouped into 6 plans (~3.7 reqs/plan) follows the same
  cadence. Cluster-aligned grouping (3 dense plans of 7-8 reqs each)
  was rejected as harder to parallelise and review.

- **D-TILDE-1's auto-portable normalisation is opinionated.** The user
  explicitly chose normalisation over verbatim preservation: even
  literal absolute paths under `$HOME` get rewritten to `~`-shape on
  save. The cross-machine dotfiles workflow benefit (single
  `tome.toml` works on every machine) outweighs the surprise of
  seeing tildes appear where absolute paths were typed. This is a
  firmer stance than "preserve user intent" — the file's job is
  portability, and the rule reflects that.

- **D-TILDE-2 (machine.toml stays verbatim) follows from the
  portable/per-machine split.** machine.toml is the per-machine file;
  user-typed absolute paths there are intentional (e.g. external
  drive mounts in `[directory_overrides]`). Coercing them to `~`
  would surprise the user.

- **D-BROWSE-1's smart routing acknowledges existing user setup.**
  Users who've configured per-directory `disabled` blocklists already
  signaled "I want per-directory granularity"; the toggle should
  respect that. Users who haven't configured any list get the simpler
  global toggle. The "smart" branch is a 3-line conditional, not a
  behaviour-flip flag.

- **D-BROWSE-2 closes the smart-routing UX gap.** Hidden state ("which
  list got mutated?") is the failure mode of D-BROWSE-1's smart
  routing. Making the label and status-message scope-explicit removes
  the failure mode without adding configuration complexity.

- **D-DIST-1 reuses `force` rather than adding a new flag.** The
  existing `force: bool` parameter at `distribute.rs:111` already
  gates the "recreate stale symlinks even when points-to-library
  matches" semantic. Extending it to also bypass the foreign-symlink
  check is semantically consistent ("force overrides safety
  defaults") and avoids CLI surface bloat.

- **D-DIST-2 routes foreign-symlink visibility through doctor for
  persistence.** A sync-time warning is once-and-forget; doctor is
  the diagnostic tool the user revisits. Phase 13 D-12 ("doctor
  reports drift unconditionally") is the philosophical anchor.

- **Strict beta-cut scope (D-PLAN-2) is a process decision, not a
  technical one.** A 22-requirement phase that quietly grows to 30 is
  a v0.10 ship-date hazard. The per-phase `deferred-items.md` parking
  lot pattern (Phase 11+) gives the executor a place to record
  discoveries without conflating them with the planned work.

- **Phase 14 explicitly forward-flagged HARD-02, HARD-13, HARD-22 in
  its CONTEXT.md "Adjacent issues" section.** Phase 15 inherits the
  hand-off — the planner should expect to see Phase 14 integration
  tests landing in `tests/cli.rs` that HARD-13 will move into
  `cli_remove.rs`/`cli_status.rs`/`cli_reassign.rs`.

</specifics>

<deferred>
## Deferred Ideas

- **Per-command files for `cmd_<name>` helpers (HARD-02 future
  iteration).** Recommendation lands the helpers inline in `lib.rs`
  first; if the file is still >1,500 LOC after decomposition, a
  follow-up phase can lift to a `commands/` module. Not a v0.10
  blocker.
- **`config_dir` / `paths.rs` co-location with config split (HARD-03
  future iteration).** Tilde helpers stay in `paths.rs` for now; if
  `paths.rs` grows substantially in v0.11+, the planner can
  reconsider whether `paths/` should become a module. Not in scope
  for HARD-03.
- **Browse UI Disable/Enable confirmation prompt (HARD-21 alt).**
  D-BROWSE-3 chose instant toggle for reversibility; if user feedback
  requests confirmation later, adding an optional prompt is small.
  Not in v0.10.
- **Doctor severity for `ForeignSymlink` (D-DIST-2 alt).** Currently
  Warning; if the conflict turns out to be common in practice (e.g.
  multiple tome installs is a typical setup), Error severity may be
  more appropriate. Defer the calibration to user feedback.
- **Bulk operations on the unowned set** (deferred from Phase 14):
  `tome reassign --all-unowned --to <dir>`, `tome remove skill
  --orphans-only`. Not in v0.10.
- **Provenance backfill** (deferred from Phase 14): one-shot
  `tome migrate-library --backfill-provenance` for pre-Phase-14
  Unowned entries. Not in v0.10; D-C2 graceful fallback covers the
  read-time UX.
- **`unexpand_tilde` invariants on Windows** — `$HOME` doesn't exist
  on Windows in the same shape; the rule would need adjustment. Out
  of scope per project policy "Platform: Unix-only (symlinks)".
- **Per-test `tome` binary pre-built once for `tests/cli_*.rs` files
  (HARD-13 perf optimisation).** `assert_cmd::Command::cargo_bin`
  rebuilds the binary on first call per process; splitting into N
  test files multiplies invocations. If wall-time regresses
  noticeably, a shared `OnceCell` in `tests/common/mod.rs` helps.
  Defer to performance-actual-measurement; not in scope unless the
  split causes a measurable regression.

### Reviewed Todos (not folded)

(None — `gsd-tools.cjs todo match-phase 15` returned 0 matches.)

</deferred>

---

*Phase: 15-cli-hardening*
*Context gathered: 2026-05-08*
