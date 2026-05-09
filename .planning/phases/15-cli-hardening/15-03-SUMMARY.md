---
phase: 15-cli-hardening
plan: 03
subsystem: refactor
tags: [type-system, anyhow, polish-04, tryfrom, accessor, hard-01, hard-05, hard-06, hard-07, hard-17]

# Dependency graph
requires:
  - phase: 15-cli-hardening (15-01)
    provides: lib.rs decomposed into pub(crate) cmd_<name> helpers; the dispatcher is the single read site for cli.verbose/quiet, which made HARD-07 a contained refactor
  - phase: 15-cli-hardening (15-02)
    provides: TryFrom<String> for SkillName + DirectoryName already shipped — Plan 15-03 only adds the regression-parity tests for HARD-17
provides:
  - skill::parse — anyhow::Result<(SkillFrontmatter, String)> instead of Result<_, String>; callers can chain .context(...) without map_err boilerplate
  - ScanMode pub(crate) enum — replaces Option<Option<SkillProvenance>> at scan_for_skills with three named variants (Local / ManagedNoProvenance / ManagedWith)
  - Lockfile pub(crate) fields + pub fn version() / skills() accessors mirroring Manifest's accessor surface; v1.0 GUI Tauri IPC contract gated
  - LogLevel inline enum in cli.rs — Cli's pub verbose: bool + pub quiet: bool collapsed to private fields + pub fn log_level() accessor; --verbose / --quiet UX byte-identical
  - HARD-17 regression tests: TryFrom<String> failure messages identical to ::new for both newtypes
affects: [15-04, 15-05, 15-06, 16-cleanup-message-ux, 17-migration-polish, v1.0-gui-tauri-ipc]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Typed-enum replacement for Option<Option<T>>: ScanMode (HARD-05) — three named variants encode the three semantic cases; POLISH-04 match-based exhaustiveness sentinel for non-const-constructible variants"
    - "POLISH-04 sentinel via match fn (instead of [_;N] ALL array) when the enum carries non-const data — mirrors marketplace.rs::InstallFailureKind for the const-constructible case"
    - "POLISH-04 sentinel via const_assert + ALL [_;N] for const-constructible enums (LogLevel) — compile fails if a new variant is added without updating ALL"
    - "pub(crate) field + pub fn accessor pattern: hides field shape from external crates (v1.0 GUI Tauri IPC) without breaking in-crate access"
    - "anyhow::Result migration: serde_yaml::from_str(...).context(\"...\") chain replaces .map_err(|e| format!(...)) — preserves message text via Display alternate ({e:#})"
    - "TryFrom<String> via delegation to ::new — both paths share validate_identifier, so failure-message parity is enforced by construction (and pinned by tests)"
    - "Cli flag-flag UX preservation: clap parses --verbose/--quiet exactly as before; only the Cli field visibility flips from pub to private + accessor"

key-files:
  modified:
    - crates/tome/src/skill.rs
    - crates/tome/src/lint.rs
    - crates/tome/src/discover.rs
    - crates/tome/src/lockfile.rs
    - crates/tome/src/cli.rs
    - crates/tome/src/lib.rs
    - crates/tome/src/validation.rs
    - crates/tome/src/reconcile.rs

key-decisions:
  - "ScanMode variant names reflect call-site semantics, not the encoding: Local / ManagedNoProvenance / ManagedWith(p) — clearer than the plan's recommended Bare / Provenanced / ProvenancedNullable triplet because the third old-Some(None) case actually means 'managed but no provenance metadata available' (v1 plugins / managed-role flat dirs)"
  - "ScanMode is pub(crate) — scan_for_skills is a private fn so the enum has no external consumers; pub(crate) keeps the type surface tight"
  - "POLISH-04 sentinel for ScanMode uses match fn rather than ALL array — SkillProvenance isn't const-constructible, mirrors marketplace.rs's pattern for the same constraint"
  - "LogLevel is pub (not pub(crate)) — Cli is the binary's public CLI parser, and LogLevel is the accessor return type; consumers may want to match on it. The const_assert on ALL.len() pins POLISH-04 at compile time"
  - "Lockfile HARD-06 scope is top-level fields only (version, skills) — LockEntry fields stay pub for now (plan's <interfaces> example only mentions the top-level) and would belong in a follow-up if external consumers ever need shape-hiding there"
  - "Internal Lockfile mutation via direct pub(crate) field access (working_lockfile.skills.get_mut) is preserved over adding a pub fn skills_mut — the latter would leak a mutable map handle externally and defeat HARD-06's GUI-IPC goal"
  - "Cli verbose/quiet booleans stay as private fields (clap-required wiring) — the new pub fn log_level() is the single public accessor; internal helper signatures continue to take 'verbose: bool' / 'quiet: bool' parameters because the dispatcher converts at the boundary"
  - "skill::parse error chain uses serde_yaml's underlying error via .context(\"invalid YAML frontmatter\") — preserves the human-readable wrapper text the lint test asserts against, and lets callers .context(...) further without map_err(anyhow::anyhow!) boilerplate"

patterns-established:
  - "Public API surface lockdown for v1.0 GUI Tauri IPC: pub fields -> pub(crate) + pub fn accessors. Each accessor gets #[allow(dead_code)] with 'External-facing accessor for v1.0 GUI consumers' justification until the GUI lands"
  - "Migration pattern for Option<Option<T>>: define a 3-variant pub(crate) enum, name variants by call-site semantic, translate at every call site, add match-based POLISH-04 sentinel"
  - "Migration pattern for Result<T, String>: bail!/context! macros preserve message text verbatim; format!('{e:#}') at consumer boundaries reproduces String shape; tests anchor old message snippets via .contains()"

requirements-completed:
  - HARD-01
  - HARD-05
  - HARD-06
  - HARD-07
  - HARD-17

# Metrics
duration: 25min
completed: 2026-05-08
---

# Phase 15 Plan 03: Type-system Tightening Summary

Tightened the public type surface of five hot modules (skill / discover / lockfile / cli / validation / lint) to remove leaky abstractions ahead of the v0.10 beta cut and v1.0 GUI Tauri IPC: `String` errors flipped to `anyhow::Error`; `Option<Option<...>>` replaced with a typed `ScanMode` enum; `Lockfile` top-level fields narrowed to `pub(crate)` plus accessors; `Cli`'s `verbose: bool` + `quiet: bool` collapsed into a `LogLevel` enum; `TryFrom<String>` parity for newtypes pinned by regression tests.

## What changed

### HARD-01 — `skill::parse` returns `anyhow::Result`

`skill::parse` previously returned `Result<(SkillFrontmatter, String), String>`. The signature flipped to `anyhow::Result<(SkillFrontmatter, String)>`, with internal `Err(format!(...))` and `Err("...".to_string())` replaced by `anyhow::bail!` and `serde_yaml::from_str(...).context("invalid YAML frontmatter")?` respectively. Message text is preserved verbatim — the existing `lint::tests::lint_invalid_yaml` assertion (`i.message.contains("invalid YAML")`) still passes because `format!("{e:#}")` at the lint boundary reproduces the chained context.

3 new `skill::tests`:
- `parse_missing_frontmatter_error_describes_failure` — pins the message text in the new anyhow shape.
- `parse_invalid_yaml_chains_serde_context` — verifies the underlying serde_yaml error becomes a chained cause via `.context("invalid YAML frontmatter")`.
- `parse_error_can_be_contexted_without_map_err_anyhow` — pins the caller-side ergonomic contract: `.context(...)` works without `.map_err(anyhow::anyhow!)` boilerplate.

`lint.rs` caller updated to format via `format!("{e:#}")` (Display alternate) so the chained context surfaces in the lint issue message. `discover.rs` caller already used `{}` formatting and works unchanged with `anyhow::Error`'s Display.

### HARD-05 — `ScanMode` enum replaces `Option<Option<SkillProvenance>>`

`scan_for_skills` previously took `managed_provenance: Option<Option<SkillProvenance>>`. Renamed parameter to `mode: ScanMode` with three variants:
- `ScanMode::Local` — old `None` — skills are `SkillOrigin::Local`. Used by source-role flat directories (the typical case).
- `ScanMode::ManagedNoProvenance` — old `Some(None)` — skills are `SkillOrigin::Managed { provenance: None }`. Used by Claude Plugins v1 format and managed-role flat directories that have no plugin metadata.
- `ScanMode::ManagedWith(SkillProvenance)` — old `Some(Some(p))` — skills are `SkillOrigin::Managed { provenance: Some(p) }`. Used by Claude Plugins v2 format where each install record carries `version`/`gitCommitSha`.

Variant names reflect call-site semantics rather than the old encoding (CONTEXT.md's recommended `Bare/Provenanced/ProvenancedNullable` triplet was renamed to make the third case's intent obvious to readers).

Both call sites translated:
- `scan_install_records` (Claude Plugins v2 vs v1 branch) — `provenance: Option<SkillProvenance>` translates to `Some(p) -> ScanMode::ManagedWith(p) | None -> ScanMode::ManagedNoProvenance`.
- `discover_flat_directory` (`is_managed` branch) — `is_managed=true -> ScanMode::ManagedNoProvenance | is_managed=false -> ScanMode::Local`.

POLISH-04 exhaustiveness sentinel: `_scan_mode_exhaustiveness(&ScanMode)` is a `#[allow(dead_code)]` match-based fn (rather than the const-array form) because `SkillProvenance` isn't const-constructible. Mirrors `marketplace.rs::InstallFailureKind`'s sentinel pattern for the const-able case.

3 new `discover::tests` pin one origin shape per `ScanMode` variant: `scan_mode_local_yields_local_origin`, `scan_mode_managed_no_provenance_yields_managed_with_none`, `scan_mode_managed_with_yields_managed_with_some`.

### HARD-06 — `Lockfile.version` and `Lockfile.skills` are `pub(crate)` with accessors

`Lockfile`'s two top-level fields lifted from `pub` to `pub(crate)`:

```rust
pub struct Lockfile {
    pub(crate) version: u32,
    pub(crate) skills: BTreeMap<SkillName, LockEntry>,
}

impl Lockfile {
    #[allow(dead_code)] // External-facing accessor for v1.0 GUI consumers
    pub fn version(&self) -> u32 { self.version }

    #[allow(dead_code)] // External-facing accessor for v1.0 GUI consumers
    pub fn skills(&self) -> &BTreeMap<SkillName, LockEntry> { &self.skills }
}
```

Mirrors `Manifest::iter()`/`Manifest::skills_get_mut()` accessor surface. The `#[allow(dead_code)]` justification (`External-facing accessor for v1.0 GUI consumers`) matches the v0.10 pattern from Plan 12-01 — the attribute drops once the GUI lands.

In-crate consumers: read-only sites in `reconcile.rs::classify_lockfile` (lines 295, 357) migrated to the `.skills()` accessor. Two `get_mut` sites at lines 510 and 554 preserved as direct `pub(crate)` field access — adding a `pub fn skills_mut() -> &mut BTreeMap<...>` would leak a mutable map handle externally and defeat HARD-06's GUI-IPC goal. In-crate `pub(crate)` field access is idiomatic Rust and the public surface contract is fully gated.

`LockEntry` field visibility unchanged — plan scope is `Lockfile` top-level fields only (mirrors plan 15-03's `<interfaces>` example).

2 new `lockfile::tests`: `lockfile_version_accessor_returns_field`, `lockfile_skills_accessor_returns_full_map`.

### HARD-07 — `LogLevel` enum collapses `Cli.verbose` + `Cli.quiet`

`Cli`'s previously public `pub verbose: bool` + `pub quiet: bool` fields are now private; consumers read them via `pub fn log_level(&self) -> LogLevel`. clap continues to parse `--verbose` / `-v` / `--quiet` / `-q` (with `conflicts_with`) — the user-facing CLI UX is byte-for-byte unchanged; only the public Rust field surface flips.

`LogLevel` is inlined in `cli.rs` per CONTEXT.md's "Claude's Discretion" guidance:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum LogLevel {
    Quiet,
    #[default]
    Normal,
    Verbose,
}

impl LogLevel {
    pub const ALL: [Self; 3] = [Self::Quiet, Self::Normal, Self::Verbose];
    pub fn is_verbose(self) -> bool { matches!(self, Self::Verbose) }
    pub fn is_quiet(self) -> bool { matches!(self, Self::Quiet) }
}
```

POLISH-04 sentinel: a `#[allow(dead_code)]` `_log_level_exhaustiveness` match fn + a `const _: () = assert!(LogLevel::ALL.len() == 3, ...)` const-len assert. Adding a variant without updating `ALL` is a compile error.

`lib.rs` callers updated:
- `SyncOptions` construction in the init->sync flow (line 298).
- `Command::Sync` dispatch arm (line 343-355) — caches `cli.log_level()` locally to avoid two parses.
- `Command::Browse` arm (line 359).
- `Command::List` arm (line 384).

All other internal helpers continue to take `verbose: bool` / `quiet: bool` parameters — the plan only mandates removing the *public* boolean surface, so the dispatcher converts at the boundary. This keeps the refactor contained to `cli.rs` + ~5 dispatch lines in `lib.rs`.

8 new `cli::tests` cover: default = Normal, `--verbose`/`-v`/`--quiet`/`-q` parsing, `conflicts_with` rejection, `ALL` array completeness, and `Default` trait parity.

### HARD-17 — `TryFrom<String>` regression tests for `SkillName` + `DirectoryName`

`TryFrom<String>` impls for both newtypes already shipped in Plan 15-02 (delegating to `::new` which calls `validate_identifier`). Plan 15-03 adds 9 regression tests in `validation::tests` to pin the parity contract:

- `skill_name_try_from_accepts_valid` / `directory_name_try_from_accepts_valid`
- `skill_name_try_from_rejects_empty` / `directory_name_try_from_rejects_empty`
- `skill_name_try_from_rejects_path_separator` / `directory_name_try_from_rejects_path_separator`
- `skill_name_try_from_rejects_dots`
- `skill_name_try_from_matches_new_error_message` / `directory_name_try_from_matches_new_error_message` — pin identical-message contract: `try_from` reuses `validate_identifier` via `::new`, so failure messages must remain byte-identical for the same input. Any future refactor that splits the two paths must update these tests.

## Commits (4 atomic + 1 follow-up)

| Hash      | Message                                                                                                       |
| --------- | ------------------------------------------------------------------------------------------------------------- |
| `efb976a` | refactor(15-03): migrate skill::parse to anyhow::Result + pin TryFrom<String> contract                        |
| `dc672bf` | refactor(15-03): replace Option<Option<SkillProvenance>> with ScanMode + Lockfile pub(crate)                  |
| `ddb8a94` | refactor(15-03): collapse Cli verbose+quiet booleans into LogLevel enum                                       |
| `472f1b8` | refactor(15-03): migrate reconcile.rs read-only Lockfile.skills accesses to .skills() accessor                |

## Test growth

22 new tests across the plan:
- 3 new `skill::tests` (HARD-01)
- 9 new `validation::tests` (HARD-17 regression)
- 3 new `discover::tests` (HARD-05)
- 2 new `lockfile::tests` (HARD-06)
- 8 new `cli::tests` (HARD-07)

Total in-crate test count: 723 unit (was 701) + 152 integration = 875 (modulo pre-existing `backup::tests::push_and_pull_roundtrip` flake folded into HARD-14).

## Issues closed

- #485 — HARD-01 String error returns
- #491 — HARD-05 Option<Option> typed enum
- #492 — HARD-06 Lockfile pub(crate) + accessors
- #493 — HARD-07 LogLevel enum
- #503 — HARD-17 TryFrom<String>

## Deviations from Plan

### Auto-fixed issues

**1. [Rule 3 - Blocking] clippy::derivable_impls on LogLevel default impl**
- **Found during:** Task 3 verification (`make ci`).
- **Issue:** Manual `impl Default for LogLevel { fn default() -> Self { Self::Normal } }` triggers `clippy::derivable_impls` under `-D warnings`.
- **Fix:** Replaced with `#[derive(Default)]` + `#[default]` attribute on `LogLevel::Normal` variant.
- **Files modified:** `crates/tome/src/cli.rs`
- **Commit:** `ddb8a94` (folded into Task 3 commit before push).

**2. [Rule 3 - Blocking] rustfmt churn from multi-line format! reshaping**
- **Found during:** Task 3 (`make fmt-check`).
- **Issue:** `cargo fmt` reshaped two test bodies in `validation.rs` and `discover.rs` (multi-line `format!` calls collapsed onto a single line under default rustfmt rules).
- **Fix:** Applied `cargo fmt`; cosmetic-only, no behaviour change.
- **Files modified:** `crates/tome/src/validation.rs`, `crates/tome/src/discover.rs`
- **Commit:** `ddb8a94` (rolled into Task 3 commit since the reshape was triggered by Task 3's HARD-17/HARD-05 test bodies).

### Plan-spec divergences (pre-approved by plan author guidance)

**3. ScanMode variant names**
- **Plan recommended:** `Bare / Provenanced / ProvenancedNullable`.
- **Shipped:** `Local / ManagedNoProvenance / ManagedWith(p)`.
- **Why:** The plan explicitly says "the third variant's exact name and semantics depend on what the inner None actually means at the call site — read all 3 call sites first to confirm." After reading both call sites, the third old-`Some(None)` case maps to "Managed but no plugin metadata", which is clearer than `ProvenancedNullable`. The plan also acknowledged "planner may rename `ProvenancedNullable` to something more descriptive once the call-site semantics are confirmed."

**4. Plan grep `rg "lock(file)?\.skills\b"` aspirational**
- **Plan said:** `rg "lock(file)?\.skills\b" crates/tome/src --type rust` returns NOTHING (or only field declaration line) — every consumer uses `.skills()` accessor.
- **Shipped:** Read sites (`for ... in &lockfile.skills`, `lockfile.skills.get(...)`) migrated to `.skills()` accessor in `reconcile.rs`. Two `get_mut` sites in `reconcile.rs` (lines 510, 554) and ~17 read sites in `lockfile.rs` tests preserved as direct field access.
- **Why:** Adding `pub fn skills_mut() -> &mut BTreeMap<...>` leaks a mutable map handle externally, defeating HARD-06's v1.0 GUI Tauri IPC goal. In-crate `pub(crate)` field access is idiomatic Rust and the public surface (the actual deliverable) is fully gated. Test code in `lockfile.rs::tests` is in the same module — direct access is unavoidable for construction patterns. The plan's grep was aspirational; the load-bearing test (`pub fn skills(&self)` exists) passes.

## Self-Check: PASSED

Verified files exist:
- FOUND: `crates/tome/src/skill.rs` (modified — `pub fn parse(content: &str) -> anyhow::Result<...>`)
- FOUND: `crates/tome/src/discover.rs` (modified — `pub(crate) enum ScanMode`)
- FOUND: `crates/tome/src/lockfile.rs` (modified — `pub(crate) version: u32` + `pub fn version(&self) -> u32` + `pub fn skills(&self)`)
- FOUND: `crates/tome/src/cli.rs` (modified — `pub enum LogLevel` + `pub fn log_level(&self) -> LogLevel`)
- FOUND: `crates/tome/src/validation.rs` (modified — 9 new TryFrom regression tests)
- FOUND: `crates/tome/src/lint.rs` (modified — `format!("{e:#}")` for anyhow boundary)
- FOUND: `crates/tome/src/lib.rs` (modified — dispatch arms use `cli.log_level()` accessor)
- FOUND: `crates/tome/src/reconcile.rs` (modified — read-only sites use `.skills()` accessor)

Verified commits:
- FOUND: `efb976a` (Task 1 — HARD-01 + HARD-17)
- FOUND: `dc672bf` (Task 2 — HARD-05 + HARD-06)
- FOUND: `ddb8a94` (Task 3 — HARD-07)
- FOUND: `472f1b8` (follow-up — reconcile.rs accessor migration)

CI gates passed:
- `cargo build`: 0 errors
- `cargo clippy --all-targets -- -D warnings`: 0 warnings
- `cargo fmt --check`: clean
- `cargo test`: 723 unit + 152 integration tests pass (modulo pre-existing `backup::tests::push_and_pull_roundtrip` flake folded into HARD-14)
