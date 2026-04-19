---
phase: 5
plan: 1
type: execute
wave: 1
depends_on: []
files_modified:
  - crates/tome/src/wizard.rs
  - crates/tome/src/lib.rs
  - crates/tome/src/cli.rs
requirements:
  - WHARD-04
  - WHARD-05
autonomous: true
must_haves:
  truths:
    - "Running `tome init --no-input` no longer bails — the wizard runs to completion using defaults at every prompt"
    - "`--dry-run` + `--no-input` together print the `Generated config:` marker and exit 0 when HOME contains no known directories"
    - "Pure config-assembly logic (the inline `Config { directories, library_dir, exclude, ..Default::default() }` step) is callable from unit tests without a TTY via `assemble_config`"
    - "The `--no-input` defaults used by the wizard match D-01 exactly: include all auto-discovered known dirs, library = `~/.tome/skills`, no exclusions, no role edits, no custom dirs added, no git init"
  artifacts:
    - path: "crates/tome/src/wizard.rs"
      provides: "`pub fn run(dry_run: bool, no_input: bool) -> Result<Config>` signature"
      contains: "no_input: bool"
    - path: "crates/tome/src/wizard.rs"
      provides: "`pub(crate) fn assemble_config` helper extracted from inline assembly"
      contains: "pub(crate) fn assemble_config"
    - path: "crates/tome/src/lib.rs"
      provides: "Init bail removed; wizard::run called with cli.dry_run and cli.no_input"
      contains: "wizard::run(cli.dry_run, cli.no_input)"
  key_links:
    - from: "crates/tome/src/lib.rs::run() Init branch"
      to: "crates/tome/src/wizard.rs::run"
      via: "direct call passing no_input"
      pattern: "wizard::run\\(cli\\.dry_run, cli\\.no_input\\)"
    - from: "crates/tome/src/wizard.rs::run"
      to: "crates/tome/src/wizard.rs::assemble_config"
      via: "helper call at end of interactive/non-interactive flow"
      pattern: "assemble_config\\("
---

<objective>
Plumb the existing `--no-input` CLI flag into the wizard and extract the inline config-assembly step into a pure `pub(crate) fn assemble_config` helper. This closes two prerequisites for Phase 5 testing:

1. WHARD-05 prerequisite: the integration test cannot run `tome init` headlessly today because `lib.rs:164-165` hard-bails when `cli.no_input` is set on `Init`.
2. WHARD-04 prerequisite: the inline assembly at `wizard.rs:292-297` and the per-entry insertion loop at `wizard.rs:421-436` are not addressable from unit tests without driving dialoguer.

Purpose: make the wizard testable without removing its interactive behavior. `--no-input` takes the default at every dialoguer call (per D-01). `assemble_config` centralizes the "selected directories + library + exclusions → Config" step so unit tests can exercise it directly.

Output:
- `wizard::run` takes `(dry_run: bool, no_input: bool)` and branches per-prompt between `if no_input { default } else { dialoguer_call }`.
- `wizard::assemble_config(selected, library, exclude) -> Config` callable from `wizard.rs::tests`.
- `lib.rs:164-165` bail is deleted; the wizard is invoked with `cli.no_input`.
- `cli.rs:77-78` after_help mentions `--dry-run` and `--no-input`.
- A smoke test in `lib.rs::tests` proves the bail is gone (regression guard per CONTEXT.md Claude's Discretion).

No unit tests for `assemble_config` or integration tests land in this plan — those are Plans 02 and 03 respectively. This plan is a pure plumbing + extraction refactor that leaves the interactive TTY path behaviourally unchanged.
</objective>

<execution_context>
@$HOME/.claude/get-shit-done/workflows/execute-plan.md
@$HOME/.claude/get-shit-done/templates/summary.md
</execution_context>

<context>
@.planning/PROJECT.md
@.planning/ROADMAP.md
@.planning/STATE.md
@.planning/phases/05-wizard-test-coverage/05-CONTEXT.md
@.planning/phases/04-wizard-correctness/04-CONTEXT.md

<interfaces>
<!-- Direct quotes and exact signatures required for implementation. -->

D-01 (CONTEXT.md) — `--no-input` defaults per prompt:
  - include all auto-discovered `KNOWN_DIRECTORIES`  (select-all MultiSelect default)
  - library = `PathBuf::from("~/.tome/skills")`        (default in configure_library)
  - exclude = `BTreeSet::new()`                        (empty exclusions MultiSelect)
  - no role edits                                      (edit-roles Confirm defaults false)
  - no custom dirs                                     (add-custom Confirm defaults false)
  - no `git init`                                      (backup-init Confirm defaults false)
  - on post-save "Save configuration?" Confirm → treat as `true` (default) so non-dry-run
    `--no-input` can exercise the save path (D-04). Dry-run bypasses this.

D-02 — remove `lib.rs:164-165` bail. Pass `cli.no_input` to `wizard::run`.
D-03 — signature: `pub fn run(dry_run: bool, no_input: bool) -> Result<Config>`.
D-04 — `--no-input` without `--dry-run` still saves (via `save_checked`).
D-05 — extract `pub(crate) fn assemble_config(...) -> Config` from inline assembly.
D-12 — stdout marker `"Generated config:"` at `wizard.rs:324` is already there; DO NOT touch.

Existing call site in `lib.rs` (lib.rs:163-174) — authoritative current code:
```rust
if matches!(cli.command, Command::Init) {
    if cli.no_input {
        anyhow::bail!("tome init requires interactive input — cannot use --no-input");
    }
    if let Err(e) = Config::load_or_default(effective_config.as_deref()) {
        eprintln!(
            "warning: existing config is malformed ({}), the wizard will create a new one",
            e
        );
    }
    let tome_home = resolve_tome_home(cli.tome_home.as_deref(), cli.config.as_deref())?;
    let config = wizard::run(cli.dry_run)?;
```

Existing inline assembly in `wizard.rs:292-297` (authoritative):
```rust
let config = Config {
    library_dir,
    exclude,
    directories,
    ..Default::default()
};
```

Existing per-entry insertion into `directories` at `wizard.rs:421-436` — this stays inline in
`configure_directories()`; the BTreeMap it builds is the input to `assemble_config`. Custom-dir
insertions at `wizard.rs:274-285` also stay in `wizard::run` because they happen after step 3.

Type signatures the new helper must match (from config.rs):
- `pub struct DirectoryName(String)` — newtype, `DirectoryName::new(impl Into<String>) -> Result<Self>`
- `pub struct DirectoryConfig { pub path: PathBuf, pub directory_type: DirectoryType, pub(crate) role: Option<DirectoryRole>, pub branch: ..., pub tag: ..., pub rev: ..., pub subdir: ... }`
  — note `role` is `pub(crate)` — inside the `crate::tome` crate (which wizard.rs is part of),
  direct struct-literal construction with `role:` is OK (already done at wizard.rs:429 and wizard.rs:279).
- `pub struct Config { ... directories: BTreeMap<DirectoryName, DirectoryConfig>, ... library_dir: PathBuf, exclude: BTreeSet<SkillName>, backup: BackupConfig }`

Signature for the extracted helper (D-05):
```rust
pub(crate) fn assemble_config(
    directories: BTreeMap<DirectoryName, DirectoryConfig>,
    library_dir: PathBuf,
    exclude: BTreeSet<crate::discover::SkillName>,
) -> Config
```

Rationale for taking `BTreeMap` (not `&[(DirectoryName, DirectoryConfig)]`): the interactive flow
already has a `BTreeMap<DirectoryName, DirectoryConfig>` in hand at the point of assembly
(line 294), so passing a map costs zero allocations and matches the target field shape exactly.
CONTEXT.md D-05 allows either shape ("or equivalent").
</interfaces>
</context>

<tasks>

<task type="auto" tdd="false">
  <name>Task 1: Extract `assemble_config` helper and plumb `no_input` through `wizard::run`</name>
  <files>
    crates/tome/src/wizard.rs
    crates/tome/src/lib.rs
    crates/tome/src/cli.rs
  </files>
  <read_first>
    - crates/tome/src/wizard.rs (focus on run() at line 126, dry-run branch at lines 306-325, save block at lines 326-352, configure_directories at lines 393-447, configure_library at lines 449-474, configure_exclusions at lines 476-511, inline Config assembly at lines 292-297, edit-roles loop at lines 178-224, add-custom loop at lines 227-288, find_known_directories helpers at lines 513-541, tests module at lines 543-620)
    - crates/tome/src/lib.rs (focus on lines 155-193 — the Init branch including the bail at 164-165 and the wizard::run call at 174)
    - crates/tome/src/cli.rs (focus on lines 13-44 — Cli struct with `no_input` flag at line 43 — and lines 76-78 — the Init subcommand after_help)
    - crates/tome/src/config.rs (focus on lines 13-186 — DirectoryName/DirectoryType/DirectoryRole/DirectoryConfig definitions — and lines 249-268 — Config struct)
    - .planning/phases/05-wizard-test-coverage/05-CONTEXT.md (D-01 through D-06 and D-12, authoritative)
  </read_first>
  <action>

### Part A — `crates/tome/src/wizard.rs`

Step A.1 — Change the public entry signature at line 126.

Replace:
```rust
/// Run the interactive setup wizard.
pub fn run(dry_run: bool) -> Result<Config> {
```
with:
```rust
/// Run the interactive setup wizard.
///
/// When `no_input` is true, every dialoguer prompt is replaced with its
/// documented default (per Phase 5 D-01): select all auto-discovered known
/// directories, library = `~/.tome/skills`, empty exclusions, no role edits,
/// no custom directories, no git init. Dry-run and save paths behave the same
/// as interactive mode — `no_input` only affects how prompts are resolved.
pub fn run(dry_run: bool, no_input: bool) -> Result<Config> {
```

Step A.2 — Wire `no_input` into each dialoguer call inside `wizard::run`. The following replacements are exhaustive (every `dialoguer::Confirm/Select/Input/MultiSelect` call in `run()` or its helpers called from `run()` that this plan must change).

Step A.2.a — `configure_directories` (wizard.rs:393-447) must receive and respect `no_input`. Change its signature and the `MultiSelect` call.

Replace at wizard.rs:393:
```rust
fn configure_directories() -> Result<BTreeMap<DirectoryName, DirectoryConfig>> {
```
with:
```rust
fn configure_directories(no_input: bool) -> Result<BTreeMap<DirectoryName, DirectoryConfig>> {
```

Inside `configure_directories`, replace the MultiSelect call at wizard.rs:412-419:
```rust
        let selections = MultiSelect::new()
            .with_prompt(
                "Found these directories -- select which to include\n  (space to toggle, enter to confirm)",
            )
            .items(&labels)
            .defaults(&vec![true; found.len()])
            .report(false)
            .interact()?;
```
with:
```rust
        let selections: Vec<usize> = if no_input {
            // D-01: include all auto-discovered directories.
            (0..found.len()).collect()
        } else {
            MultiSelect::new()
                .with_prompt(
                    "Found these directories -- select which to include\n  (space to toggle, enter to confirm)",
                )
                .items(&labels)
                .defaults(&vec![true; found.len()])
                .report(false)
                .interact()?
        };
```

Update the caller inside `run()`. Replace the line at wizard.rs:145:
```rust
    let mut directories = configure_directories()?;
```
with:
```rust
    let mut directories = configure_directories(no_input)?;
```

Step A.2.b — `configure_library` (wizard.rs:449-474) must receive and respect `no_input`.

Replace at wizard.rs:449:
```rust
fn configure_library() -> Result<PathBuf> {
```
with:
```rust
fn configure_library(no_input: bool) -> Result<PathBuf> {
```

Inside `configure_library`, replace the selection/custom logic at wizard.rs:459-470:
```rust
    let selection = Select::new()
        .with_prompt("Where should the skill library live?")
        .items(&options)
        .default(0)
        .interact()?;

    let path = if selection == 0 {
        default
    } else {
        let custom: String = Input::new().with_prompt("Library path").interact_text()?;
        crate::paths::collapse_home_path(&expand_tilde(&PathBuf::from(custom))?)
    };
```
with:
```rust
    let path = if no_input {
        // D-01: default library = ~/.tome/skills
        default
    } else {
        let selection = Select::new()
            .with_prompt("Where should the skill library live?")
            .items(&options)
            .default(0)
            .interact()?;

        if selection == 0 {
            default
        } else {
            let custom: String = Input::new().with_prompt("Library path").interact_text()?;
            crate::paths::collapse_home_path(&expand_tilde(&PathBuf::from(custom))?)
        }
    };
```

Update the caller inside `run()`. Replace the line at wizard.rs:168:
```rust
    let library_dir = configure_library()?;
```
with:
```rust
    let library_dir = configure_library(no_input)?;
```

Step A.2.c — `configure_exclusions` (wizard.rs:476-511) must receive and respect `no_input`.

Replace at wizard.rs:476-478:
```rust
fn configure_exclusions(
    skills: &[crate::discover::DiscoveredSkill],
) -> Result<std::collections::BTreeSet<crate::discover::SkillName>> {
```
with:
```rust
fn configure_exclusions(
    skills: &[crate::discover::DiscoveredSkill],
    no_input: bool,
) -> Result<std::collections::BTreeSet<crate::discover::SkillName>> {
```

Inside `configure_exclusions`, replace the MultiSelect block at wizard.rs:489-495. Find:
```rust
    let max_rows = Term::stderr().size().0.saturating_sub(6).max(5) as usize;
    let selections = MultiSelect::new()
        .with_prompt("Select skills to exclude (space to toggle, enter to confirm)")
        .items(&labels)
        .defaults(&vec![false; labels.len()])
        .max_length(max_rows)
        .interact()?;
```
Replace with:
```rust
    let selections: Vec<usize> = if no_input {
        // D-01: empty exclusions.
        Vec::new()
    } else {
        let max_rows = Term::stderr().size().0.saturating_sub(6).max(5) as usize;
        MultiSelect::new()
            .with_prompt("Select skills to exclude (space to toggle, enter to confirm)")
            .items(&labels)
            .defaults(&vec![false; labels.len()])
            .max_length(max_rows)
            .interact()?
    };
```

Update the caller inside `run()`. Replace the line at wizard.rs:171:
```rust
    let exclude = configure_exclusions(&discovered)?;
```
with:
```rust
    let exclude = configure_exclusions(&discovered, no_input)?;
```

Step A.2.d — Role-edit loop at wizard.rs:178-224. Under `no_input`, skip the loop entirely.

Replace the outer loop (wizard.rs:178-224):
```rust
    // Offer to edit roles
    loop {
        let edit = Confirm::new()
            .with_prompt("Would you like to edit any directory's role?")
            .default(false)
            .interact()?;

        if !edit {
            break;
        }
        ...
```
with:
```rust
    // Offer to edit roles (skipped entirely under --no-input per D-01)
    while !no_input {
        let edit = Confirm::new()
            .with_prompt("Would you like to edit any directory's role?")
            .default(false)
            .interact()?;

        if !edit {
            break;
        }
```
(Only the first two lines of the block change — `loop {` → `while !no_input {` — and the rest of
the existing block remains verbatim. The closing brace of the loop stays where it is.)

Step A.2.e — Custom-directory loop at wizard.rs:227-288. Same pattern.

Replace the outer loop header (wizard.rs:227-234):
```rust
    // Offer to add custom directories
    loop {
        let add = Confirm::new()
            .with_prompt("Add a custom directory?")
            .default(false)
            .interact()?;

        if !add {
            break;
        }
```
with:
```rust
    // Offer to add custom directories (skipped entirely under --no-input per D-01)
    while !no_input {
        let add = Confirm::new()
            .with_prompt("Add a custom directory?")
            .default(false)
            .interact()?;

        if !add {
            break;
        }
```

Step A.2.f — Save-confirm + git-init inside the non-dry-run branch at wizard.rs:326-352.

Find the `else if Confirm::new() ... "Save configuration?"` block (starts at wizard.rs:326). Under
`--no-input` we want the save to proceed (D-04: `--no-input` without `--dry-run` saves), and the
subsequent `git init` offer to be skipped (D-01: no git init).

Replace the line at wizard.rs:326:
```rust
    } else if Confirm::new()
        .with_prompt("Save configuration?")
        .default(true)
        .interact()?
    {
```
with:
```rust
    } else if no_input
        || Confirm::new()
            .with_prompt("Save configuration?")
            .default(true)
            .interact()?
    {
```

Inside that same block, replace the git-init Confirm+init at wizard.rs:342-351:
```rust
        if !tome_home.join(".git").exists() {
            let do_init = Confirm::new()
                .with_prompt("Initialize a git repo for backup tracking?")
                .default(false)
                .interact()?;
            if do_init {
                crate::backup::init(tome_home, false)
                    .unwrap_or_else(|e| eprintln!("warning: backup init failed: {e}"));
            }
        }
```
with:
```rust
        if !no_input && !tome_home.join(".git").exists() {
            let do_init = Confirm::new()
                .with_prompt("Initialize a git repo for backup tracking?")
                .default(false)
                .interact()?;
            if do_init {
                crate::backup::init(tome_home, false)
                    .unwrap_or_else(|e| eprintln!("warning: backup init failed: {e}"));
            }
        }
```

Step A.3 — Extract `assemble_config` (D-05). Add this function immediately BEFORE `fn step_divider`
(which lives at wizard.rs:361 — the top of the "Internal helpers" section marked by the `-- Internal
helpers --` comment banner). Insert it before that banner, directly after the closing brace of
`pub fn run`. Exact code to insert:

```rust
// ---------------------------------------------------------------------------
// Pure config assembly (WHARD-04 — unit-testable without dialoguer)
// ---------------------------------------------------------------------------

/// Assemble the final `Config` from wizard-produced inputs.
///
/// Pure function: no dialoguer, no filesystem, no env access. Called once at
/// the end of `run()` and driven directly by unit tests (see `wizard.rs::tests`).
///
/// Inputs:
/// - `directories`: map of selected directories (auto-discovered + custom)
/// - `library_dir`: library location (tilde-shaped or absolute; not expanded here)
/// - `exclude`: skill names to exclude
pub(crate) fn assemble_config(
    directories: BTreeMap<DirectoryName, DirectoryConfig>,
    library_dir: PathBuf,
    exclude: std::collections::BTreeSet<crate::discover::SkillName>,
) -> Config {
    Config {
        library_dir,
        exclude,
        directories,
        ..Config::default()
    }
}
```

Step A.4 — Replace the inline assembly inside `run()` at wizard.rs:292-297 with a call to the new
helper.

Replace:
```rust
    let config = Config {
        library_dir,
        exclude,
        directories,
        ..Default::default()
    };
```
with:
```rust
    let config = assemble_config(directories, library_dir, exclude);
```

### Part B — `crates/tome/src/lib.rs`

Step B.1 — Delete the bail at lib.rs:163-166 and update the `wizard::run` call.

Replace the block at lib.rs:163-174:
```rust
    if matches!(cli.command, Command::Init) {
        if cli.no_input {
            anyhow::bail!("tome init requires interactive input — cannot use --no-input");
        }
        if let Err(e) = Config::load_or_default(effective_config.as_deref()) {
            eprintln!(
                "warning: existing config is malformed ({}), the wizard will create a new one",
                e
            );
        }
        let tome_home = resolve_tome_home(cli.tome_home.as_deref(), cli.config.as_deref())?;
        let config = wizard::run(cli.dry_run)?;
```
with:
```rust
    if matches!(cli.command, Command::Init) {
        if let Err(e) = Config::load_or_default(effective_config.as_deref()) {
            eprintln!(
                "warning: existing config is malformed ({}), the wizard will create a new one",
                e
            );
        }
        let tome_home = resolve_tome_home(cli.tome_home.as_deref(), cli.config.as_deref())?;
        let config = wizard::run(cli.dry_run, cli.no_input)?;
```

Everything else in the Init branch (malformed-config warning, tome_home resolution, post-init
sync call, return statement at line 192) stays unchanged.

Step B.2 — Add a regression-guard unit test at the end of the existing `#[cfg(test)] mod tests`
block in lib.rs (the block starting at lib.rs:1438). Append this test — it documents that the
bail is gone and protects against a regression that re-adds it.

```rust
    #[test]
    fn init_with_no_input_does_not_bail_from_lib_run() {
        // Guard against re-introduction of the `tome init requires interactive input` bail
        // removed in Phase 5 Plan 01. We do not invoke wizard::run (it opens a TTY);
        // we only assert the source of lib.rs no longer contains the bail string.
        let src = include_str!("lib.rs");
        assert!(
            !src.contains("tome init requires interactive input"),
            "lib.rs still contains the removed bail — Phase 5 Plan 01 regression"
        );
    }
```

Step B.3 — No other lib.rs changes. The existing `sync(...)` call already passes `cli.no_input`
into `SyncOptions` (lib.rs:185), and the `no_triage: true` hard-code on initial sync is unrelated
to this plan.

### Part C — `crates/tome/src/cli.rs`

Step C.1 — Update the `Init` subcommand after_help (cli.rs:77-78) to mention both flags.

Replace:
```rust
    /// Interactive wizard to configure sources and targets
    #[command(after_help = "Examples:\n  tome init")]
    Init,
```
with:
```rust
    /// Interactive wizard to configure sources and targets
    #[command(
        after_help = "Examples:\n  tome init\n  tome init --dry-run\n  tome init --no-input\n  tome init --dry-run --no-input"
    )]
    Init,
```

### Part D — Run CI equivalent

```bash
cd /Users/martin/dev/opensource/tome
cargo fmt
cargo clippy --all-targets -- -D warnings
cargo test -p tome
```

Every existing wizard test must still pass (no behavior change in interactive mode). The new
`init_with_no_input_does_not_bail_from_lib_run` test must pass.

Do NOT:
- Introduce a `WizardOptions` struct (D-06: deferred).
- Change the stdout "Generated config:" marker (D-12: test depends on it).
- Move `assemble_config` into `config.rs` (CONTEXT.md specifies `wizard.rs`).
- Add env/stdin reading (D-01: `--no-input` means defaults only).
- Add tests for `assemble_config` or an integration test for `tome init --no-input` here — those
  are Plans 02 and 03 respectively.
- Rename or reorder `KNOWN_DIRECTORIES` (Plan 02 depends on entry shapes).
  </action>
  <verify>
    <automated>cd /Users/martin/dev/opensource/tome && cargo fmt -- --check && cargo clippy --all-targets -- -D warnings && cargo test -p tome --lib wizard::tests && cargo test -p tome --lib tests::init_with_no_input_does_not_bail_from_lib_run && cargo build -p tome</automated>
  </verify>
  <acceptance_criteria>
    - `rg "pub fn run\(dry_run: bool, no_input: bool\) -> Result<Config>" crates/tome/src/wizard.rs` returns 1 hit
    - `rg "pub\(crate\) fn assemble_config" crates/tome/src/wizard.rs` returns 1 hit
    - `rg "tome init requires interactive input" crates/tome/src/` returns 0 hits
    - `rg "wizard::run\(cli\.dry_run, cli\.no_input\)" crates/tome/src/lib.rs` returns 1 hit
    - `rg "fn configure_directories\(no_input: bool\)" crates/tome/src/wizard.rs` returns 1 hit
    - `rg "fn configure_library\(no_input: bool\)" crates/tome/src/wizard.rs` returns 1 hit
    - `rg "fn configure_exclusions\(" crates/tome/src/wizard.rs -A1` shows `no_input: bool` in the signature
    - `rg "while !no_input" crates/tome/src/wizard.rs` returns exactly 2 hits (role-edit loop + custom-dir loop)
    - `rg "no_input \|\| Confirm::new" crates/tome/src/wizard.rs` returns 1 hit (Save configuration? prompt)
    - `rg "if !no_input && !tome_home\.join" crates/tome/src/wizard.rs` returns 1 hit (git-init gate)
    - `rg "tome init --no-input" crates/tome/src/cli.rs` returns at least 1 hit (after_help mentions the flag)
    - `rg "assemble_config\(directories, library_dir, exclude\)" crates/tome/src/wizard.rs` returns 1 hit (run() now calls the helper)
    - `rg "let config = Config \{" crates/tome/src/wizard.rs` returns 0 hits (inline assembly is gone; the
      remaining Config references are inside `discover::discover_all` temp at line ~149 and
      `Config::default()` inside `assemble_config` itself, neither matches this pattern)
    - `cargo test -p tome --lib wizard::tests` exits 0 (all 6 existing tests still pass)
    - `cargo test -p tome --lib tests::init_with_no_input_does_not_bail_from_lib_run` exits 0
    - `cargo build -p tome` exits 0
    - `cargo clippy --all-targets -- -D warnings` exits 0
    - `cargo fmt -- --check` exits 0
  </acceptance_criteria>
  <done>
    Wizard signature is `pub fn run(dry_run: bool, no_input: bool) -> Result<Config>`. Every dialoguer call in `run()` and its helpers (`configure_directories`, `configure_library`, `configure_exclusions`, role-edit loop, custom-dir loop, save-confirm, git-init offer) branches on `no_input` and takes the D-01 default when true. `assemble_config(directories, library_dir, exclude) -> Config` exists as `pub(crate)` in `wizard.rs` and is called once at the end of `run()`. `lib.rs` no longer bails on `tome init --no-input`; it calls `wizard::run(cli.dry_run, cli.no_input)`. `cli.rs` after_help mentions `--dry-run` and `--no-input`. A regression guard test in `lib.rs::tests` asserts the bail string is gone from source. `make ci` clean.
  </done>
</task>

</tasks>

<verification>
Phase-exit checks for Plan 05-01:

1. `cd /Users/martin/dev/opensource/tome && cargo fmt -- --check` exits 0
2. `cd /Users/martin/dev/opensource/tome && cargo clippy --all-targets -- -D warnings` exits 0
3. `cd /Users/martin/dev/opensource/tome && cargo test -p tome` exits 0 (no test regressions; the new lib regression-guard passes)
4. `rg "tome init requires interactive input" crates/tome/src/` returns 0 hits
5. `rg "pub\(crate\) fn assemble_config" crates/tome/src/wizard.rs` returns 1 hit
6. `rg "wizard::run\(cli\.dry_run, cli\.no_input\)" crates/tome/src/lib.rs` returns 1 hit
</verification>

<success_criteria>
- `tome init --no-input` no longer exits early with the "requires interactive input" bail.
- `wizard::run` accepts a `no_input: bool` argument; every dialoguer call in the wizard path branches on it per D-01.
- `assemble_config` is a pure `pub(crate)` helper inside `wizard.rs` callable from unit tests; the inline `Config { ... }` assembly at the old line 292 is gone.
- `tome init` interactive behavior is byte-for-byte unchanged (same prompts in the same order; the only difference is that under `--no-input` those prompts are not shown).
- `cli.rs` help output mentions `--dry-run` and `--no-input` under `tome init` examples.
- Plans 02 and 03 are unblocked: Plan 02 can import `assemble_config` from `wizard`; Plan 03 can run `tome init --dry-run --no-input` and expect a valid generated config in stdout.
</success_criteria>

<output>
After completion, create `.planning/phases/05-wizard-test-coverage/05-01-no-input-plumbing-and-assemble-config-SUMMARY.md`.
</output>
