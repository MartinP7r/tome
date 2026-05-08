---
phase: 15-cli-hardening
plan: 03
type: execute
wave: 1
depends_on: []
files_modified:
  - crates/tome/src/skill.rs
  - crates/tome/src/discover.rs
  - crates/tome/src/lockfile.rs
  - crates/tome/src/cli.rs
  - crates/tome/src/lib.rs
  - crates/tome/src/validation.rs
  - crates/tome/src/lint.rs
autonomous: true
requirements:
  - HARD-01
  - HARD-05
  - HARD-06
  - HARD-07
  - HARD-17
must_haves:
  truths:
    - "skill::parse returns anyhow::Result<(SkillFrontmatter, String)> — no String error type leaks at the API boundary"
    - "scan_for_skills takes a typed ScanMode enum, not Option<Option<SkillProvenance>>"
    - "Lockfile.skills and Lockfile.version are pub(crate); accessor methods exist mirroring Manifest's shape"
    - "Cli has a single LogLevel enum field, not separate verbose: bool + quiet: bool flags"
    - "SkillName and DirectoryName each implement TryFrom<String> reusing validate_identifier"
  artifacts:
    - path: "crates/tome/src/skill.rs"
      provides: "skill::parse → anyhow::Result"
      contains: "anyhow::Result"
    - path: "crates/tome/src/discover.rs"
      provides: "ScanMode enum replacing Option<Option<SkillProvenance>>"
      contains: "enum ScanMode"
    - path: "crates/tome/src/lockfile.rs"
      provides: "pub(crate) fields + pub accessors"
      contains: "pub(crate) skills"
    - path: "crates/tome/src/cli.rs"
      provides: "LogLevel enum"
      contains: "enum LogLevel"
    - path: "crates/tome/src/validation.rs"
      provides: "TryFrom<String> impls for SkillName + DirectoryName"
      contains: "impl TryFrom<String>"
  key_links:
    - from: "crates/tome/src/discover.rs::scan_for_skills"
      to: "ScanMode enum"
      via: "function parameter type"
      pattern: "scan_for_skills.*ScanMode"
    - from: "crates/tome/src/lockfile.rs::Lockfile"
      to: "pub fn skills() / pub fn version()"
      via: "accessor pattern mirroring Manifest"
      pattern: "pub fn (skills|version)\\("
---

<objective>
Tighten the public type surface of five hot modules to remove leaky abstractions: `String` errors → `anyhow` (HARD-01); `Option<Option<...>>` → typed enum (HARD-05); `pub` fields → `pub(crate)` + accessors (HARD-06); two booleans → one enum (HARD-07); add `TryFrom<String>` for newtype constructors (HARD-17).

Purpose: Stable type surface for v0.10's beta cut and future v1.0 GUI Tauri IPC. Each change closes a specific GitHub issue from the v0.9 review.
Output: `skill::parse` returns `anyhow::Result`; `scan_for_skills` takes `ScanMode`; `Lockfile` fields are `pub(crate)` with accessors; `Cli` uses `LogLevel`; `SkillName` + `DirectoryName` impl `TryFrom<String>`.
</objective>

<execution_context>
@$HOME/.claude/get-shit-done/workflows/execute-plan.md
@$HOME/.claude/get-shit-done/templates/summary.md
</execution_context>

<context>
@.planning/PROJECT.md
@.planning/ROADMAP.md
@.planning/STATE.md
@.planning/REQUIREMENTS.md
@.planning/phases/15-cli-hardening/15-CONTEXT.md

@crates/tome/src/skill.rs
@crates/tome/src/discover.rs
@crates/tome/src/lockfile.rs
@crates/tome/src/cli.rs
@crates/tome/src/validation.rs
@crates/tome/src/manifest.rs
@crates/tome/src/lib.rs

<interfaces>
<!-- Existing types being modified. -->

From crates/tome/src/skill.rs (line 55, current shape):
```rust
pub fn parse(content: &str) -> Result<(SkillFrontmatter, String), String>
```
Target shape (HARD-01):
```rust
pub fn parse(content: &str) -> anyhow::Result<(SkillFrontmatter, String)>
```

From crates/tome/src/discover.rs (~line 445, current shape):
```rust
pub fn scan_for_skills(
    root: &Path,
    skill_provenance: Option<Option<SkillProvenance>>,  // ← HARD-05 target
    ...
) -> Vec<DiscoveredSkill>
```

From crates/tome/src/lockfile.rs (current shape):
```rust
pub struct Lockfile {
    pub version: u32,
    pub skills: BTreeMap<SkillName, LockEntry>,
}
```
Target (HARD-06):
```rust
pub struct Lockfile {
    pub(crate) version: u32,
    pub(crate) skills: BTreeMap<SkillName, LockEntry>,
}
impl Lockfile {
    pub fn version(&self) -> u32 { self.version }
    pub fn skills(&self) -> &BTreeMap<SkillName, LockEntry> { &self.skills }
}
```

From crates/tome/src/manifest.rs (mirror this accessor pattern for HARD-06):
```rust
// Manifest already exposes pub fn skills(&self) -> &BTreeMap<...>
// Lockfile should mirror exactly.
```

From crates/tome/src/cli.rs (lines 31, 34-35, current shape):
```rust
pub struct Cli {
    #[arg(long, short)]
    pub verbose: bool,         // ← HARD-07 target
    #[arg(long, short, conflicts_with = "verbose")]
    pub quiet: bool,           // ← HARD-07 target
    ...
}
```

From crates/tome/src/validation.rs (current shape):
```rust
pub fn validate_identifier(s: &str) -> anyhow::Result<()>
// SkillName::new and DirectoryName::new call validate_identifier internally.
```
</interfaces>
</context>

<tasks>

<task type="auto" tdd="true">
  <name>Task 1: HARD-01 (skill::parse anyhow) + HARD-17 (TryFrom<String>)</name>
  <files>crates/tome/src/skill.rs, crates/tome/src/validation.rs, crates/tome/src/discover.rs, crates/tome/src/lint.rs</files>
  <read_first>
    - crates/tome/src/skill.rs (current `pub fn parse(...) -> Result<_, String>` at line 55)
    - crates/tome/src/validation.rs (newtype + validate_identifier shape)
    - crates/tome/src/discover.rs (callers of skill::parse — they currently `.map_err(anyhow::anyhow!)?` or similar)
    - crates/tome/src/lint.rs (caller of skill::parse)
    - .planning/phases/15-cli-hardening/15-CONTEXT.md §"HARD-17 TryFrom<String> failure mode" (Claude's Discretion: reuse validate_identifier)
    - .planning/REQUIREMENTS.md §"HARD-01" §"HARD-17"
  </read_first>
  <behavior>
    skill::parse returns anyhow::Result:
    - Test: missing frontmatter delimiter → `Err(anyhow::Error)` whose `to_string()` describes the parse failure (preserve the existing message text where possible).
    - Test: invalid YAML in frontmatter → `Err(anyhow::Error)` with serde_yaml context.
    - Test: valid frontmatter → `Ok((frontmatter, body_string))`.
    - Test: a parse-failure error returned to a caller can be `.context(...)`-ed without `.map_err(anyhow::anyhow!)` boilerplate.

    TryFrom<String> for SkillName:
    - Test: `SkillName::try_from("valid-name".to_string())` returns `Ok(SkillName(...))`.
    - Test: `SkillName::try_from("".to_string())` returns `Err` (empty name rejected by validate_identifier).
    - Test: `SkillName::try_from("path/with/slash".to_string())` returns `Err` (path separator rejected).
    - Test: `SkillName::try_from(".".to_string())` returns `Err` (dot rejected).
    - Test: failure mode: error type is `anyhow::Error`, message identical to `SkillName::new` for the same input.

    TryFrom<String> for DirectoryName: parallel coverage to SkillName (same validate_identifier rules).
  </behavior>
  <action>
    **Step A: HARD-01 — `skill::parse` → `anyhow::Result`.**

    In `crates/tome/src/skill.rs` line 55, change:

    ```rust
    pub fn parse(content: &str) -> Result<(SkillFrontmatter, String), String>
    ```

    to:

    ```rust
    pub fn parse(content: &str) -> anyhow::Result<(SkillFrontmatter, String)>
    ```

    Inside `parse`, replace `Err(format!(...))` constructions with `anyhow::bail!(...)` (preserving message text verbatim) and `Err("...")` with `anyhow::bail!`. For YAML-parse-from-error path, use `serde_yaml::from_str(...).context("...")?` to chain the underlying error.

    Update all callers: search for `skill::parse` across the crate (likely `discover.rs`, `lint.rs`, possibly `validation.rs`). At each call site, drop any `.map_err(...)` adapter that was converting `String` → `anyhow::Error` — the new signature returns `anyhow::Result` directly so `?` works without adaptation.

    **Step B: HARD-17 — `TryFrom<String>` for `SkillName` and `DirectoryName`.**

    In `crates/tome/src/validation.rs` (or the same file as `SkillName`/`DirectoryName` definitions — check `discover.rs` for `SkillName` and `config/types.rs` (post-15-02) for `DirectoryName`):

    ```rust
    impl TryFrom<String> for SkillName {
        type Error = anyhow::Error;
        fn try_from(s: String) -> anyhow::Result<Self> {
            crate::validation::validate_identifier(&s)?;
            Ok(SkillName(s))
        }
    }

    impl TryFrom<String> for DirectoryName {
        type Error = anyhow::Error;
        fn try_from(s: String) -> anyhow::Result<Self> {
            crate::validation::validate_identifier(&s)?;
            Ok(DirectoryName(s))
        }
    }
    ```

    The failure message must be identical to what `SkillName::new`/`DirectoryName::new` returns for the same input — both go through `validate_identifier`, so this is automatic if the impls reuse the helper. Verify by writing a regression test that asserts identical error strings between `::new(...)` and `::try_from(...)` for the same bad input.

    Per CONTEXT.md "Claude's Discretion": **reuse the existing `validate_identifier` validation; failure is the same `anyhow::Error` the existing `SkillName::new` returns.**

    **Step C: tests.**

    Add unit tests in the relevant modules covering `<behavior>` above. Place skill::parse tests in `skill.rs::tests`; place `TryFrom` tests next to existing newtype tests.
  </action>
  <verify>
    <automated>cargo test -p tome skill::tests &amp;&amp; cargo test -p tome validation::tests &amp;&amp; cargo test -p tome discover::tests &amp;&amp; cargo build -p tome &amp;&amp; cargo clippy --all-targets -- -D warnings</automated>
  </verify>
  <acceptance_criteria>
    - `grep -E "pub fn parse\(content: &str\) -> anyhow::Result" crates/tome/src/skill.rs` returns at least one match.
    - `grep -E "Result<.*, String>" crates/tome/src/skill.rs` returns NOTHING (no leftover String error returns in the public surface).
    - `grep -E "impl TryFrom<String> for (SkillName|DirectoryName)" crates/tome/src` returns at least 2 matches.
    - Both `TryFrom<String>` impls call `validate_identifier(&s)?` (verify via `grep -A3 "impl TryFrom<String> for"`).
    - At least 4 new test fns covering `<behavior>` (skill::parse Ok/Err paths + TryFrom Ok/Err paths for both newtypes).
    - `cargo build -p tome` exits 0; `cargo clippy --all-targets -- -D warnings` exits 0.
    - `cargo test -p tome` passes; baseline + new tests all green.
    - No remaining `.map_err(anyhow::anyhow!)` adapter at skill::parse call sites: `rg "skill::parse.*map_err" crates/tome/src` returns 0 results.
  </acceptance_criteria>
  <done>
    `skill::parse` returns `anyhow::Result`; `SkillName` and `DirectoryName` impl `TryFrom<String>` reusing `validate_identifier`. Tests cover both Ok and Err paths.
  </done>
</task>

<task type="auto" tdd="true">
  <name>Task 2: HARD-05 ScanMode + HARD-06 Lockfile pub(crate)</name>
  <files>crates/tome/src/discover.rs, crates/tome/src/lockfile.rs, crates/tome/src/lib.rs, crates/tome/src/manifest.rs</files>
  <read_first>
    - crates/tome/src/discover.rs (line 445: `scan_for_skills(... Option<Option<SkillProvenance>>, ...)` — read all 3 call sites to understand which combinations are used)
    - crates/tome/src/lockfile.rs (current `pub version` + `pub skills` field declarations)
    - crates/tome/src/manifest.rs (mirror its accessor shape — `Manifest::skills()` etc.)
    - crates/tome/src/lib.rs (callers of `scan_for_skills` and `Lockfile.skills`/`Lockfile.version`)
    - .planning/phases/15-cli-hardening/15-CONTEXT.md §"HARD-05" §"HARD-06"
    - .planning/REQUIREMENTS.md §"HARD-05" §"HARD-06"
    - .planning/phases/10-phase-8-review-tail/10-CONTEXT.md (POLISH-04 ALL-array sentinel pattern — apply to ScanMode)
  </read_first>
  <behavior>
    ScanMode replaces Option<Option<SkillProvenance>>:
    - Test: each variant of ScanMode covers exactly one of the semantic cases the double-Option encodes. Decode the cases by reading discover.rs:445 callers.
    - Likely 3 variants based on the Option<Option<X>> pattern:
      - `Bare` (= `None` outer — "no provenance, no need to construct one")
      - `Provenanced(SkillProvenance)` (= `Some(Some(p))` — "use this exact provenance")
      - `ProvenancedNullable` (= `Some(None)` — "construct provenance per-skill from context")
    - The third variant's exact name and semantics depend on what the inner `None` actually means at the call site — read all 3 call sites first to confirm.
    - Test: `ScanMode::ALL` array contains every variant; compile-time exhaustiveness sentinel matches POLISH-04 pattern.

    Lockfile pub(crate) + accessors:
    - Test: `Lockfile::skills(&self) -> &BTreeMap<SkillName, LockEntry>` exists and returns the same data as the old `pub skills` field.
    - Test: `Lockfile::version(&self) -> u32` exists.
    - Test: external (out-of-crate) access to `lock.skills` field fails to compile (verify by attempting to access from a top-level integration test if cross-crate; otherwise grep-verify the declaration switched from `pub` to `pub(crate)`).
    - Test: every internal call site that previously read `lock.skills` now uses `lock.skills()` accessor — preserves shape so refactor is mechanical.
  </behavior>
  <action>
    **HARD-05: Replace `Option<Option<SkillProvenance>>` with `ScanMode` enum.**

    In `crates/tome/src/discover.rs`, define:

    ```rust
    /// How a scan should attach (or not attach) provenance metadata to discovered skills.
    /// Replaces the legacy `Option<Option<SkillProvenance>>` argument shape.
    #[derive(Debug, Clone)]
    pub enum ScanMode {
        /// No provenance attached. Used when the scanner only cares about discovery, not source identity.
        Bare,
        /// Use the provided SkillProvenance for every discovered skill.
        Provenanced(SkillProvenance),
        /// (Name to be confirmed by reading existing call sites.) Indicates "construct provenance
        /// from per-skill context" — the per-skill construction logic is unchanged.
        ProvenancedNullable,
    }

    impl ScanMode {
        /// Compile-time exhaustiveness sentinel (POLISH-04 pattern).
        pub const ALL: [Self; 3] = [Self::Bare, Self::Provenanced(/* placeholder */), Self::ProvenancedNullable];
    }
    ```

    **Important:** before naming the third variant or writing `ALL`, **read all current call sites** of `scan_for_skills(... Option<Option<SkillProvenance>>, ...)`. The `Some(None)` case has a specific semantic at the call site — the variant name should reflect that semantic, not the encoding. The 3 variants above are CONTEXT.md's recommendation; planner may rename `ProvenancedNullable` to something more descriptive once the call-site semantics are confirmed.

    Note: The const ALL array can't directly contain `Provenanced(SkillProvenance)` if SkillProvenance isn't const-constructible. If that's the case, follow POLISH-04's approach in `marketplace.rs::InstallFailureKind::ALL` — use a function or skip the const for variants with non-const data, but keep an exhaustiveness sentinel match:

    ```rust
    // POLISH-04 exhaustiveness sentinel — compile fails if a new variant is added
    // without updating ALL.
    fn _exhaustiveness_check(m: ScanMode) {
        match m {
            ScanMode::Bare => {}
            ScanMode::Provenanced(_) => {}
            ScanMode::ProvenancedNullable => {}
        }
    }
    ```

    Update `scan_for_skills` signature to take `mode: ScanMode` instead of `Option<Option<SkillProvenance>>`. Update all 3 call sites. Translation table:
    - Old `None` → New `ScanMode::Bare`
    - Old `Some(Some(p))` → New `ScanMode::Provenanced(p)`
    - Old `Some(None)` → New `ScanMode::ProvenancedNullable` (or whatever name reflects call-site semantic)

    **HARD-06: Tighten `Lockfile` field visibility + add accessors.**

    In `crates/tome/src/lockfile.rs`:

    ```rust
    pub struct Lockfile {
        pub(crate) version: u32,
        pub(crate) skills: BTreeMap<SkillName, LockEntry>,
        // ... any other existing fields, also lifted to pub(crate)
    }

    impl Lockfile {
        pub fn version(&self) -> u32 { self.version }
        pub fn skills(&self) -> &BTreeMap<SkillName, LockEntry> { &self.skills }
        pub fn skills_mut(&mut self) -> &mut BTreeMap<SkillName, LockEntry> { &mut self.skills }
    }
    ```

    Mirror `Manifest`'s accessor surface exactly — read `crates/tome/src/manifest.rs` first to confirm shape (look for `Manifest::skills()`, `Manifest::skills_get_mut()` per Phase 11 plan 11-01 SUMMARY).

    Schema is unchanged (Phase 11 D-12, D-14 stay locked). Only field visibility changes.

    Update all callers across the crate to use `.skills()` / `.version()` accessor methods instead of direct field access. Search: `rg "lock(file)?\.(skills|version)\b" crates/tome/src --type rust` — every match needs to flip to method-call syntax.

    Add unit tests verifying accessor parity with the previous direct-field access (round-trip a Lockfile instance, assert `lock.skills()` returns the same content as before).
  </action>
  <verify>
    <automated>cargo test -p tome discover::tests &amp;&amp; cargo test -p tome lockfile::tests &amp;&amp; cargo build -p tome &amp;&amp; cargo clippy --all-targets -- -D warnings</automated>
  </verify>
  <acceptance_criteria>
    - `grep -E "pub enum ScanMode" crates/tome/src/discover.rs` returns at least one match.
    - `grep -E "Option<Option<SkillProvenance>>" crates/tome/src` returns NOTHING (legacy shape gone).
    - `grep -E "scan_for_skills.*ScanMode" crates/tome/src/discover.rs` shows the function signature uses ScanMode.
    - `grep -E "ScanMode::ALL|fn _exhaustiveness_check" crates/tome/src/discover.rs` returns at least one match (POLISH-04 sentinel).
    - `grep -E "pub\(crate\) skills:" crates/tome/src/lockfile.rs` returns one match.
    - `grep -E "pub\(crate\) version:" crates/tome/src/lockfile.rs` returns one match.
    - `grep -E "pub fn skills\(&self\)" crates/tome/src/lockfile.rs` returns one match (accessor exists).
    - `grep -E "pub fn version\(&self\)" crates/tome/src/lockfile.rs` returns one match (accessor exists).
    - `rg "lock(file)?\.skills\b" crates/tome/src --type rust` returns NOTHING (or only field declaration line) — every consumer uses `.skills()` accessor.
    - `cargo build -p tome` exits 0; `cargo clippy --all-targets -- -D warnings` exits 0.
    - `cargo test -p tome` passes; baseline preserved.
  </acceptance_criteria>
  <done>
    `scan_for_skills` takes a typed `ScanMode` enum with `ALL` array + exhaustiveness sentinel. `Lockfile` fields are `pub(crate)` with `pub fn skills()` / `pub fn version()` accessors mirroring `Manifest`. All call sites updated; tests pass.
  </done>
</task>

<task type="auto" tdd="true">
  <name>Task 3: HARD-07 LogLevel enum (verbose/quiet collapse)</name>
  <files>crates/tome/src/cli.rs, crates/tome/src/lib.rs</files>
  <read_first>
    - crates/tome/src/cli.rs (lines 31, 34-35: current `verbose: bool` + `quiet: bool` declarations)
    - crates/tome/src/lib.rs (every reader of `cli.verbose` and `cli.quiet` — these are the consumers that need the new enum)
    - .planning/phases/15-cli-hardening/15-CONTEXT.md §"HARD-07 LogLevel location" (Claude's Discretion: inline in cli.rs)
    - .planning/REQUIREMENTS.md §"HARD-07"
    - .planning/phases/10-phase-8-review-tail/10-CONTEXT.md (POLISH-04 ALL-array sentinel pattern)
  </read_first>
  <behavior>
    LogLevel enum:
    - Test: parsing CLI args with `--verbose` produces `LogLevel::Verbose`.
    - Test: parsing CLI args with `--quiet` produces `LogLevel::Quiet`.
    - Test: parsing CLI args with neither produces `LogLevel::Normal`.
    - Test: passing both `--verbose` and `--quiet` is rejected (clap conflicts_with → parse error). Preserves existing behaviour.
    - Test: `LogLevel::ALL == [Quiet, Normal, Verbose]`; exhaustiveness sentinel matches POLISH-04.
    - Test: every existing call site that previously read `cli.verbose` now reads via the `LogLevel` enum (e.g. `if matches!(cli.log_level, LogLevel::Verbose)` or via a `cli.log_level.is_verbose()` helper).
  </behavior>
  <action>
    Per CONTEXT.md "Claude's Discretion": **inline `LogLevel` in `cli.rs`** — it's a CLI-facing enum, not worth a separate `log.rs` module.

    In `crates/tome/src/cli.rs`, replace the two boolean flags with:

    ```rust
    #[derive(clap::ValueEnum, Debug, Clone, Copy, PartialEq, Eq)]
    pub enum LogLevel {
        Quiet,
        Normal,
        Verbose,
    }

    impl LogLevel {
        pub const ALL: [Self; 3] = [Self::Quiet, Self::Normal, Self::Verbose];

        pub fn is_verbose(self) -> bool { matches!(self, Self::Verbose) }
        pub fn is_quiet(self) -> bool { matches!(self, Self::Quiet) }
    }

    impl Default for LogLevel {
        fn default() -> Self { Self::Normal }
    }

    // POLISH-04 exhaustiveness sentinel
    #[allow(dead_code)]
    fn _log_level_exhaustiveness(l: LogLevel) {
        match l {
            LogLevel::Quiet => {}
            LogLevel::Normal => {}
            LogLevel::Verbose => {}
        }
    }
    ```

    Replace the two boolean fields on `Cli`:

    ```rust
    // BEFORE (lines 31, 34-35):
    // #[arg(long, short)]
    // pub verbose: bool,
    // #[arg(long, short, conflicts_with = "verbose")]
    // pub quiet: bool,

    // AFTER:
    #[arg(long, short = 'v', conflicts_with_all = ["quiet"])]
    verbose: bool,  // private — exposed only via .log_level()

    #[arg(long, short = 'q', conflicts_with_all = ["verbose"])]
    quiet: bool,    // private — exposed only via .log_level()
    ```

    Add a method on `Cli` to map the parsed flags into the enum:

    ```rust
    impl Cli {
        pub fn log_level(&self) -> LogLevel {
            if self.verbose { LogLevel::Verbose }
            else if self.quiet { LogLevel::Quiet }
            else { LogLevel::Normal }
        }
    }
    ```

    Note: clap's `--verbose` / `--quiet` flag UX is preserved exactly — same flag spellings, same conflicts. Only the public field surface changes from `pub verbose: bool` + `pub quiet: bool` to a single `pub fn log_level(&self) -> LogLevel`. (Alternative: use a derived enum field via clap's `flatten` — depends on clap version capabilities. Pick whichever keeps the user-facing CLI identical.)

    Update all consumers in `lib.rs`: `if cli.verbose` → `if cli.log_level().is_verbose()` (or `matches!(cli.log_level(), LogLevel::Verbose)`). `if cli.quiet` → `cli.log_level().is_quiet()`. Search: `rg "cli\.(verbose|quiet)\b" crates/tome/src` — every match flips to the accessor.

    Add unit tests covering all 3 ways to invoke (verbose / quiet / neither) plus the conflicts_with parse-failure case.
  </action>
  <verify>
    <automated>cargo test -p tome cli::tests &amp;&amp; cargo build -p tome &amp;&amp; cargo clippy --all-targets -- -D warnings</automated>
  </verify>
  <acceptance_criteria>
    - `grep -E "pub enum LogLevel" crates/tome/src/cli.rs` returns one match.
    - `grep -E "LogLevel::ALL.*\[" crates/tome/src/cli.rs` returns one match.
    - `grep -E "fn _log_level_exhaustiveness|LogLevel::ALL" crates/tome/src/cli.rs` confirms exhaustiveness sentinel exists.
    - `grep -E "pub verbose: bool|pub quiet: bool" crates/tome/src/cli.rs` returns NOTHING (the public boolean surface is gone).
    - `grep -E "pub fn log_level\(&self\) -> LogLevel" crates/tome/src/cli.rs` returns one match (or equivalent accessor pattern).
    - `rg "cli\.(verbose|quiet)\b" crates/tome/src --type rust` returns NOTHING outside `cli.rs` itself.
    - At least 3 new unit tests covering verbose/quiet/normal parsing.
    - Existing CLI integration tests that pass `--verbose` or `--quiet` still pass.
    - `cargo build -p tome` exits 0; `cargo clippy --all-targets -- -D warnings` exits 0.
  </acceptance_criteria>
  <done>
    `Cli` exposes a single `LogLevel` enum via `cli.log_level()` (or equivalent); existing `--verbose` / `--quiet` UX preserved; POLISH-04 exhaustiveness sentinel in place. All consumers updated.
  </done>
</task>

</tasks>

<verification>
- `cargo build -p tome` exits 0
- `cargo clippy --all-targets -- -D warnings` exits 0
- `cargo test -p tome` passes
- `rg "Result<.*, String>" crates/tome/src/skill.rs` returns 0 results (HARD-01 done)
- `rg "Option<Option<SkillProvenance>>" crates/tome/src` returns 0 results (HARD-05 done)
- `Lockfile.skills` and `Lockfile.version` are `pub(crate)` with `pub fn` accessors mirroring `Manifest` (HARD-06 done)
- `Cli` has no public `verbose: bool` / `quiet: bool` fields; `log_level()` accessor returns `LogLevel` enum (HARD-07 done)
- `SkillName` and `DirectoryName` impl `TryFrom<String>` (HARD-17 done)
- POLISH-04 exhaustiveness sentinels exist for `ScanMode` and `LogLevel`
- Test count grows by ≥10 (skill::parse cases + TryFrom cases + ScanMode + Lockfile accessor + LogLevel parsing)
</verification>

<success_criteria>
- HARD-01: `skill::parse` returns `anyhow::Result<(SkillFrontmatter, String)>` (closes #485)
- HARD-05: `scan_for_skills` takes a `ScanMode` enum with POLISH-04 ALL-array + sentinel (closes #491)
- HARD-06: `Lockfile.skills` + `Lockfile.version` are `pub(crate)` with mirroring accessors (closes #492)
- HARD-07: `Cli` exposes a `LogLevel` enum via `log_level()` accessor; existing `--verbose` / `--quiet` UX preserved (closes #493)
- HARD-17: `SkillName` and `DirectoryName` impl `TryFrom<String>` reusing `validate_identifier` (closes #503)
- All five changes are pure type-system tightening — no behaviour change for users
</success_criteria>

<output>
After completion, create `.planning/phases/15-cli-hardening/15-03-SUMMARY.md` recording:
- Per-HARD test additions
- ScanMode variant names + the call-site semantics each captures (3 variants finalised)
- Lockfile accessor methods added (mirror Manifest)
- LogLevel + POLISH-04 sentinel in place
- Issues closed: #485 (HARD-01), #491 (HARD-05), #492 (HARD-06), #493 (HARD-07), #503 (HARD-17)
</output>
