---
phase: 07-wizard-ux-greenfield-brownfield-legacy
plan: 03
type: execute
wave: 2
depends_on:
  - 07-01-wux-04-resolved-tome-home-info
files_modified:
  - crates/tome/src/wizard.rs
  - crates/tome/src/config.rs
  - crates/tome/src/lib.rs
  - crates/tome/tests/cli.rs
autonomous: true
requirements:
  - WUX-01
  - WUX-05
must_haves:
  truths:
    - "On a greenfield machine (TomeHomeSource::Default), `tome init` prompts the user to choose tome_home with `~/.tome/` as option 0 (default) and `Custom path...` as option 1"
    - "A user-entered custom tome_home path is validated: absolute paths accepted, relative paths rejected, paths that exist but are not directories rejected, nonexistent paths accepted (will be created on save)"
    - "When the user chooses a custom tome_home, wizard offers to persist the choice by writing `~/.config/tome/config.toml` with merge-preserve semantics (existing keys kept, only `tome_home` inserted/overwritten)"
    - "The wizard's save path reflects the chosen tome_home (not the stale `default_config_path()` result) — fixes the latent bug at wizard.rs:310"
    - "Under --no-input, no greenfield prompt is shown AND no XDG file is written"
    - "When tome_home source is NOT Default (flag/env/XDG), the greenfield prompt is skipped entirely"
  artifacts:
    - path: "crates/tome/src/wizard.rs"
      provides: "wizard::run signature extended with tome_home + source; Step 0 tome_home prompt; configure_library honors chosen tome_home"
      contains: "Step 0"
    - path: "crates/tome/src/config.rs"
      provides: "write_xdg_tome_home helper for atomic merge-write"
      contains: "fn write_xdg_tome_home"
    - path: "crates/tome/tests/cli.rs"
      provides: "integration tests covering --no-input greenfield skip, XDG file untouched under --no-input"
      contains: "init_greenfield_no_input"
  key_links:
    - from: "crates/tome/src/lib.rs Command::Init"
      to: "wizard::run"
      via: "new signature: run(dry_run, no_input, tome_home, tome_home_source)"
      pattern: "wizard::run\\("
    - from: "crates/tome/src/wizard.rs configure_library"
      to: "chosen tome_home path"
      via: "derives default library as `<tome_home>/skills` not hardcoded `~/.tome/skills`"
      pattern: "skills"
    - from: "crates/tome/src/wizard.rs run"
      to: "wizard save path"
      via: "resolve_config_dir(&tome_home).join(\"tome.toml\") replaces default_config_path()"
      pattern: "resolve_config_dir"
    - from: "crates/tome/src/config.rs write_xdg_tome_home"
      to: "~/.config/tome/config.toml"
      via: "parse existing TOML table, insert tome_home key, atomic temp+rename"
      pattern: "rename"
---

<objective>
Add a Step 0 to the wizard that prompts the user for `tome_home` on greenfield machines (default `~/.tome/`, custom with validation), and when a custom path is chosen, offer to persist it to `~/.config/tome/config.toml` so subsequent `tome sync` / `tome status` invocations find the choice without needing `TOME_HOME=`. This covers WUX-01 + WUX-05 — coupled because both hinge on the new `tome_home` argument threading into `wizard::run`.

Purpose: A user on a new machine has no idea `TOME_HOME` / `--tome-home` / XDG config exist — today the wizard silently picks `~/.tome/` and gives them no choice. This plan adds a Step 0 gated on `TomeHomeSource::Default` (so users with an explicit choice are not re-prompted), validates custom paths, and offers XDG persistence so the choice propagates to future invocations.

Secondary fix: The current wizard calls `default_config_path()` at wizard.rs:310 to compute the save path, which re-probes TOME_HOME+XDG and may lie to the user (the displayed save path may not match what `sync()` then uses). This plan threads the resolved `tome_home` into `wizard::run` and uses it as the single source of truth for the save path.

Output: Extended `wizard::run` signature, Step 0 prompt with validation, XDG persist prompt, `write_xdg_tome_home` helper in config.rs, and integration tests covering the --no-input auto behaviors.
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
@.planning/phases/07-wizard-ux-greenfield-brownfield-legacy/07-RESEARCH.md
@.planning/phases/07-wizard-ux-greenfield-brownfield-legacy/07-01-SUMMARY.md
@crates/tome/src/wizard.rs
@crates/tome/src/config.rs
@crates/tome/src/lib.rs
@crates/tome/src/paths.rs
@crates/tome/tests/cli.rs

<interfaces>
<!-- APIs this plan consumes from plan 01 -->

From crates/tome/src/config.rs (added in plan 01):
```rust
pub(crate) enum TomeHomeSource {
    CliTomeHome, CliConfig, EnvVar, XdgConfig, Default,
}
impl TomeHomeSource { pub(crate) fn label(self) -> &'static str; }

pub(crate) fn resolve_tome_home_with_source(
    cli_tome_home: Option<&Path>,
    cli_config: Option<&Path>,
) -> Result<(PathBuf, TomeHomeSource)>;

pub(crate) fn read_config_tome_home() -> Result<Option<PathBuf>>;  // visibility widened in plan 01
```

<!-- Existing APIs this plan extends -->

From crates/tome/src/wizard.rs (current):
```rust
pub fn run(dry_run: bool, no_input: bool) -> Result<Config>;  // line 137 — will gain 2 params
fn configure_library(no_input: bool) -> Result<PathBuf>;       // line 519 — will gain 1 param
```

From crates/tome/src/config.rs:
```rust
pub fn resolve_config_dir(tome_home: &Path) -> PathBuf;  // line 664
pub fn expand_tilde(path: &Path) -> Result<PathBuf>;
```

From crates/tome/src/paths.rs:
```rust
pub(crate) fn collapse_home_path(path: &Path) -> PathBuf;  // line 148
pub(crate) fn collapse_home(path: &Path) -> String;          // line 142
```

<!-- Target new signatures -->

```rust
// crates/tome/src/wizard.rs (new):
pub(crate) fn run(
    dry_run: bool,
    no_input: bool,
    tome_home: &Path,                   // NEW: resolved by lib.rs before call
    tome_home_source: TomeHomeSource,   // NEW: gates Step 0 greenfield prompt
) -> Result<Config>;

fn configure_library(no_input: bool, tome_home: &Path) -> Result<PathBuf>;  // NEW param

// crates/tome/src/config.rs (new):
pub(crate) fn write_xdg_tome_home(tome_home: &Path) -> Result<()>;
```
</interfaces>
</context>

<tasks>

<task type="auto" tdd="true">
  <name>Task 1: Add write_xdg_tome_home helper in config.rs with merge-preserve semantics</name>
  <files>crates/tome/src/config.rs</files>
  <read_first>
    - crates/tome/src/config.rs (focus: lines 637–680 — read_config_tome_home and resolve_config_dir; also lines 533+ save_checked for atomic write pattern reference)
    - crates/tome/src/paths.rs (focus: lines 142–155 — collapse_home + collapse_home_path)
    - crates/tome/src/machine.rs (focus: atomic temp+rename write pattern — the canonical example in the codebase)
    - .planning/phases/07-wizard-ux-greenfield-brownfield-legacy/07-RESEARCH.md (focus: lines 433–491 — WUX-05 implementation sketch; lines 710–745 — Example 2: Atomic XDG write)
  </read_first>
  <behavior>
    - Test 1: `write_xdg_tome_home(<custom>)` creates `~/.config/tome/config.toml` with `tome_home = "<custom-collapsed>"` when the file does not exist (parent dir is also created)
    - Test 2: Given an existing XDG file with `other_key = "value"`, `write_xdg_tome_home(<new>)` preserves `other_key` AND adds/overwrites `tome_home` (merge-preserve, not clobber)
    - Test 3: Given an existing XDG file with `tome_home = "~/old"`, `write_xdg_tome_home(<new>)` overwrites the value
    - Test 4: Write is atomic — the temp file does not remain after a successful write (`config.toml.tmp` must not exist)
    - Test 5: The written value uses the `~/`-collapsed form when the path is under `$HOME` (portable across machines)
  </behavior>
  <action>
Add to `crates/tome/src/config.rs`, near the existing `read_config_tome_home` function (around line 657, immediately after it):

```rust
/// Write (merge) `tome_home = <collapsed-path>` into `~/.config/tome/config.toml`.
///
/// Semantics:
/// - If the file does not exist: create parent dir, write a new TOML with just `tome_home`.
/// - If the file exists: parse as `toml::Table`, insert/overwrite the `tome_home` key,
///   preserve all other keys, write back. Comments are NOT preserved (toml crate limitation).
/// - The value is stored in `~/`-collapsed form (via `paths::collapse_home_path`) so the
///   file is portable across machines. `read_config_tome_home` tilde-expands on read.
/// - Write is atomic via temp+rename, matching the pattern in `machine.rs` / `lockfile.rs`.
///
/// Used by the wizard Step 0 (WUX-05) when the user chose a custom `tome_home` and
/// accepted the persist-prompt.
pub(crate) fn write_xdg_tome_home(tome_home: &Path) -> Result<()> {
    let home = dirs::home_dir().context("could not determine home directory")?;
    let path = home.join(".config/tome/config.toml");

    let mut table: toml::Table = if path.is_file() {
        std::fs::read_to_string(&path)
            .with_context(|| format!("failed to read {}", path.display()))?
            .parse()
            .with_context(|| format!("invalid TOML in {}", path.display()))?
    } else {
        toml::Table::new()
    };

    let collapsed = crate::paths::collapse_home_path(tome_home);
    table.insert(
        "tome_home".into(),
        toml::Value::String(collapsed.to_string_lossy().into_owned()),
    );

    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("failed to create {}", parent.display()))?;
    }

    let tmp = path.with_extension("toml.tmp");
    let content = toml::to_string_pretty(&table).context("serialize XDG config")?;
    std::fs::write(&tmp, &content)
        .with_context(|| format!("failed to write {}", tmp.display()))?;
    std::fs::rename(&tmp, &path).with_context(|| {
        format!(
            "failed to rename {} -> {}",
            tmp.display(),
            path.display()
        )
    })?;
    Ok(())
}
```

**Unit tests** inside the existing `#[cfg(test)] mod tests` in config.rs. Use the `with_env` helper from plan 01 (or create one if plan 01 put it elsewhere). Env isolation is required because `write_xdg_tome_home` reads `dirs::home_dir()` which honors `HOME`.

```rust
#[test]
fn write_xdg_tome_home_creates_new_file() {
    let tmp = TempDir::new().unwrap();
    with_env(&[("HOME", Some(tmp.path().as_os_str()))], || {
        let custom = tmp.path().join("dotfiles/tome");
        write_xdg_tome_home(&custom).unwrap();

        let xdg = tmp.path().join(".config/tome/config.toml");
        assert!(xdg.is_file(), "XDG file should be created");
        let content = std::fs::read_to_string(&xdg).unwrap();
        let table: toml::Table = content.parse().unwrap();
        let tome_home = table.get("tome_home").and_then(|v| v.as_str()).unwrap();
        // Path is under HOME → collapsed form
        assert_eq!(tome_home, "~/dotfiles/tome");
    });
}

#[test]
fn write_xdg_tome_home_preserves_other_keys() {
    let tmp = TempDir::new().unwrap();
    with_env(&[("HOME", Some(tmp.path().as_os_str()))], || {
        let xdg = tmp.path().join(".config/tome/config.toml");
        std::fs::create_dir_all(xdg.parent().unwrap()).unwrap();
        std::fs::write(&xdg, "other_key = \"preserve-me\"\ntome_home = \"~/old\"\n").unwrap();

        let custom = tmp.path().join("dotfiles/tome");
        write_xdg_tome_home(&custom).unwrap();

        let content = std::fs::read_to_string(&xdg).unwrap();
        let table: toml::Table = content.parse().unwrap();
        // tome_home overwritten
        assert_eq!(table.get("tome_home").and_then(|v| v.as_str()), Some("~/dotfiles/tome"));
        // other_key preserved
        assert_eq!(table.get("other_key").and_then(|v| v.as_str()), Some("preserve-me"));
    });
}

#[test]
fn write_xdg_tome_home_is_atomic() {
    let tmp = TempDir::new().unwrap();
    with_env(&[("HOME", Some(tmp.path().as_os_str()))], || {
        let custom = tmp.path().join("dotfiles/tome");
        write_xdg_tome_home(&custom).unwrap();

        let tmp_file = tmp.path().join(".config/tome/config.toml.tmp");
        assert!(!tmp_file.exists(), "temp file should be removed after successful rename");
    });
}
```
  </action>
  <verify>
    <automated>cargo test --package tome -- config::tests::write_xdg_tome_home 2>&1 | tail -15</automated>
  </verify>
  <acceptance_criteria>
    - `rg -n "pub\\(crate\\) fn write_xdg_tome_home" crates/tome/src/config.rs` returns a match
    - `rg -n "collapse_home_path" crates/tome/src/config.rs` returns at least one match inside `write_xdg_tome_home` (confirms collapsing is applied)
    - `rg -n "fs::rename" crates/tome/src/config.rs` returns at least one match inside `write_xdg_tome_home` (confirms atomic write pattern)
    - `rg -n "toml::Table::new\\(\\)" crates/tome/src/config.rs` returns at least one match (confirms merge-preserve semantics — starts from existing table)
    - `cargo test --package tome -- config::tests::write_xdg_tome_home` exits 0 and reports 3 tests passing
    - `cargo clippy --all-targets -- -D warnings` exits 0
    - `cargo fmt -- --check` exits 0
  </acceptance_criteria>
  <done>
    `write_xdg_tome_home` exists at pub(crate) visibility, writes via atomic temp+rename, preserves existing keys in the XDG file, collapses paths to `~/`-form for portability, and creates the parent directory if needed. Unit tests lock in creation, merge-preserve, and atomicity.
  </done>
</task>

<task type="auto" tdd="true">
  <name>Task 2: Thread tome_home + TomeHomeSource into wizard::run; fix save path; derive library default from tome_home</name>
  <files>crates/tome/src/wizard.rs, crates/tome/src/lib.rs</files>
  <read_first>
    - crates/tome/src/wizard.rs (focus: lines 137–150 — run signature; lines 305–374 — save path computation and save_checked call; lines 519–549 — configure_library)
    - crates/tome/src/lib.rs (focus: lines 162–197 — Command::Init branch, including plan 01's resolve_tome_home_with_source addition)
    - crates/tome/src/config.rs (focus: lines 664–680 — resolve_config_dir; also the newly-added write_xdg_tome_home from Task 1)
    - .planning/phases/07-wizard-ux-greenfield-brownfield-legacy/07-RESEARCH.md (focus: lines 77–78 — the wizard.rs:310 bug; lines 382–431 — WUX-01 implementation; lines 550–572 — Pitfall 1 library default derivation)
  </read_first>
  <behavior>
    - Test 1 (unit): `configure_library` with `no_input=true, tome_home=/foo` returns a library path derived from `/foo/skills` (collapsed to `~/skills` if under HOME, else `/foo/skills` literal) — NOT the hardcoded `~/.tome/skills`
    - Test 2 (compile-time): `wizard::run` signature accepts `(dry_run, no_input, tome_home, tome_home_source)`
    - Test 3 (integration, via pre-existing tests): existing `init_dry_run_no_input_empty_home` + `init_dry_run_no_input_seeded_home` + `init_no_input_writes_config_and_reloads` still pass with the new signature (backward-compat via the new params being wired by lib.rs)
  </behavior>
  <action>
**Part A — Change `wizard::run` signature to accept `tome_home: &Path` and `tome_home_source: TomeHomeSource`:**

At wizard.rs:137, replace:
```rust
pub fn run(dry_run: bool, no_input: bool) -> Result<Config> {
```
With:
```rust
pub(crate) fn run(
    dry_run: bool,
    no_input: bool,
    tome_home: &Path,
    tome_home_source: crate::config::TomeHomeSource,
) -> Result<Config> {
```

Note: change `pub` to `pub(crate)` as part of this — the only caller is `lib.rs`, and this matches the convention RESEARCH.md § "Risk: Wizard signature change ripples" endorses.

**Part B — Replace the `default_config_path()` save-path call at wizard.rs:310:**

Find (around line 310):
```rust
let config_path = default_config_path()?;
```

Replace with:
```rust
// Save path is derived from the resolved tome_home threaded in from lib.rs,
// not from default_config_path() (which would re-probe TOME_HOME+XDG and may
// disagree with what sync() uses below). This fixes the latent bug where the
// wizard could display a save path that differed from the one sync() used.
let config_path = crate::config::resolve_config_dir(tome_home).join("tome.toml");
```

Remove the `use crate::config::default_config_path` import if it was local; keep it at module scope if other callers need it (grep first: `rg -n "default_config_path" crates/tome/src/wizard.rs`).

**Part C — Update `configure_library` to derive default from `tome_home`:**

At wizard.rs:519, change the signature:
```rust
fn configure_library(no_input: bool, tome_home: &Path) -> Result<PathBuf> {
```

Inside the function, replace (around line 522):
```rust
let default = PathBuf::from("~/.tome/skills");
```
with:
```rust
// Default library = <tome_home>/skills, collapsed to ~/ form when possible
// for TOML portability (matches existing tilde preservation convention).
let default = crate::paths::collapse_home_path(&tome_home.join("skills"));
```

Then update the call site inside `wizard::run` (currently line 179):
```rust
let library_dir = configure_library(no_input, tome_home)?;
```

**Part D — Update call site in lib.rs:**

At lib.rs:170, change:
```rust
let config = wizard::run(cli.dry_run, cli.no_input)?;
```
to:
```rust
let config = wizard::run(cli.dry_run, cli.no_input, &tome_home, tome_home_source)?;
```

`tome_home` and `tome_home_source` are already in scope from plan 01. The `let _ = tome_home_source;` line from plan 01 (if it was added) must now be removed — the variable IS consumed by this call.

**Part E — Add Step 0 greenfield prompt in `wizard::run`:**

Insert immediately after the welcome banner (around wizard.rs:153, before `// Step 1: Auto-discover and select directories`):

```rust
use crate::config::TomeHomeSource;

// Step 0: Greenfield tome_home prompt (WUX-01)
// Only runs when:
// - the user has NOT already indicated a tome_home (flag, env, or XDG),
// - AND we're not in --no-input mode.
// If the user picks a custom path, also offer to persist via XDG (WUX-05).
let mut tome_home = tome_home.to_path_buf();
if matches!(tome_home_source, TomeHomeSource::Default) && !no_input {
    step_divider("Step 0: Tome home location");
    let default_home = dirs::home_dir()
        .context("could not determine home directory")?
        .join(".tome");
    let options = vec![
        format!("{} (default)", crate::paths::collapse_home(&default_home)),
        "Custom path...".to_string(),
    ];
    let selection = Select::new()
        .with_prompt("Where should tome_home live?")
        .items(&options)
        .default(0)
        .interact()?;
    if selection == 1 {
        let custom: String = Input::<String>::new()
            .with_prompt("tome_home path")
            .validate_with(|s: &String| -> std::result::Result<(), String> {
                let path = PathBuf::from(s);
                let expanded = expand_tilde(&path).map_err(|_| "could not expand ~".to_string())?;
                if !expanded.is_absolute() {
                    return Err("must be absolute".to_string());
                }
                if expanded.exists() && !expanded.is_dir() {
                    return Err("path exists but is not a directory".to_string());
                }
                Ok(())
            })
            .interact_text()?;
        tome_home = expand_tilde(&PathBuf::from(custom))?;

        // WUX-05: offer to persist custom choice to XDG
        let persist = Confirm::new()
            .with_prompt(
                "Persist this choice to ~/.config/tome/config.toml?\n  \
                 (otherwise subsequent `tome sync`/`tome status` need TOME_HOME=... or --tome-home=...)",
            )
            .default(true)
            .interact()?;
        if persist {
            crate::config::write_xdg_tome_home(&tome_home)?;
            println!(
                "  {} Wrote tome_home to ~/.config/tome/config.toml",
                style("done").green()
            );
        }
    }
    println!();
}
let tome_home = tome_home.as_path();  // reborrow as &Path for downstream calls
```

Notes:
- `expand_tilde`, `Select`, `Input`, `Confirm`, and `step_divider` are already imported at the top of wizard.rs — grep to confirm: `rg -n "use dialoguer|fn step_divider" crates/tome/src/wizard.rs`.
- The `TomeHomeSource` import inside `run` is local-scoped with `use crate::config::TomeHomeSource;` at the top of the function; alternatively add it to the top-of-file imports.
- The `let mut tome_home = tome_home.to_path_buf();` then `let tome_home = tome_home.as_path();` dance is the idiomatic way to rebind a parameter for interior mutation. If clippy objects to the shadowing, use a different local name like `chosen_tome_home`.
- `validate_with` uses `Err(String)` (not `Err(&str)`) to satisfy newer dialoguer signatures — see RESEARCH.md § "Risk: Dialoguer Input::validate_with ... type quirks" line 562.

**Part F — Unit test for configure_library default derivation:**

Add inside `#[cfg(test)] mod tests` in wizard.rs (same block as plan 02's tests):

```rust
#[test]
fn configure_library_no_input_derives_from_tome_home() {
    // Under --no-input, configure_library returns <tome_home>/skills (collapsed).
    // With tome_home = /tmp/custom (not under HOME), no collapsing happens.
    // With tome_home = ~/.tome (under HOME), result is "~/.tome/skills".
    let custom = Path::new("/tmp/zzz-test-custom-tome-home");
    let result = configure_library(true, custom).unwrap();
    // When tome_home is outside HOME, collapse_home_path is a no-op → literal path
    assert_eq!(result, PathBuf::from("/tmp/zzz-test-custom-tome-home/skills"));
}
```

*Note:* This test intentionally uses an absolute path outside HOME to avoid HOME-expansion pitfalls. The "collapse to ~/" case is implicitly covered by existing integration tests (which use HOME-relative paths).
  </action>
  <verify>
    <automated>cargo test --package tome 2>&1 | tail -20</automated>
  </verify>
  <acceptance_criteria>
    - `rg -n "pub\\(crate\\) fn run\\(" crates/tome/src/wizard.rs` shows run() accepts 4 args (dry_run, no_input, tome_home, tome_home_source)
    - `rg -n "tome_home_source: crate::config::TomeHomeSource|tome_home_source: TomeHomeSource" crates/tome/src/wizard.rs` returns at least one match
    - `rg -n "resolve_config_dir\\(tome_home\\)" crates/tome/src/wizard.rs` returns a match (confirms save path fix)
    - `rg -n "default_config_path\\(\\)" crates/tome/src/wizard.rs` returns 0 matches OR only in comments (the bug fix)
    - `rg -n "Step 0:" crates/tome/src/wizard.rs` returns a match
    - `rg -n "Where should tome_home live" crates/tome/src/wizard.rs` returns a match
    - `rg -n "write_xdg_tome_home" crates/tome/src/wizard.rs` returns a match (WUX-05 persist call)
    - `rg -n "tome_home\\.join\\(\"skills\"\\)" crates/tome/src/wizard.rs` returns a match (configure_library derives default from tome_home)
    - `rg -n "wizard::run\\(.*tome_home_source" crates/tome/src/lib.rs` returns a match (updated call site)
    - `rg -n "fn configure_library_no_input_derives_from_tome_home" crates/tome/src/wizard.rs` returns a match
    - `cargo test --package tome` exits 0 (all tests pass — including the existing init_dry_run_no_input_* integration tests which MUST still pass with the new signature)
    - `cargo clippy --all-targets -- -D warnings` exits 0
    - `cargo fmt -- --check` exits 0
  </acceptance_criteria>
  <done>
    `wizard::run` accepts `tome_home` and `tome_home_source`, `configure_library` derives its default from `tome_home` instead of hardcoding `~/.tome/skills`, the save path uses `resolve_config_dir(tome_home)` (not `default_config_path()`), and Step 0 prompts greenfield users with validation and XDG-persist offer. All existing tests still pass.
  </done>
</task>

<task type="auto" tdd="true">
  <name>Task 3: Integration tests for Step 0 skip under --no-input and XDG-write abstinence</name>
  <files>crates/tome/tests/cli.rs</files>
  <read_first>
    - crates/tome/tests/cli.rs (focus: lines 3758–3949 — existing init tests for HOME+TOME_HOME isolation patterns)
    - crates/tome/src/wizard.rs (focus: the Step 0 block added in Task 2, to understand when it's skipped vs shown)
    - .planning/phases/07-wizard-ux-greenfield-brownfield-legacy/07-RESEARCH.md (focus: lines 492–503 — --no-input behaviors table)
  </read_first>
  <behavior>
    - Test 1 (integration): `init_greenfield_no_input_skips_step_0_prompt` — with no TOME_HOME and empty HOME, `init --no-input` does NOT print "Step 0:" header (headless default = no prompt)
    - Test 2 (integration): `init_greenfield_no_input_does_not_write_xdg` — with no TOME_HOME and empty HOME, `init --no-input` does NOT create `$HOME/.config/tome/config.toml`
    - Test 3 (integration): `init_with_flag_source_skips_step_0_even_interactive` — with `--tome-home` flag (TomeHomeSource::CliTomeHome), the Step 0 prompt is NOT shown even without --no-input (we would not actually run interactively in tests, so assert via `--no-input` + explicit flag: stdout has NO "Step 0:" because source != Default)
  </behavior>
  <action>
Add to `crates/tome/tests/cli.rs` after plan 02's legacy tests:

```rust
#[test]
fn init_greenfield_no_input_skips_step_0_prompt() {
    // TomeHomeSource::Default + --no-input → Step 0 prompt must be skipped.
    let tmp = TempDir::new().unwrap();

    let output = tome()
        .args(["init", "--dry-run", "--no-input"])
        .env("HOME", tmp.path())
        .env_remove("TOME_HOME")
        .env("NO_COLOR", "1")
        .output()
        .unwrap();
    assert!(output.status.success(),
        "tome init failed: {}", String::from_utf8_lossy(&output.stderr));

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        !stdout.contains("Step 0:"),
        "--no-input must skip Step 0 prompt, but stdout contains it:\n{stdout}"
    );
    // WUX-04 info line still prints (informational, not a prompt)
    assert!(
        stdout.contains("resolved tome_home:"),
        "resolved tome_home line must still appear in --no-input mode:\n{stdout}"
    );
}

#[test]
fn init_greenfield_no_input_does_not_write_xdg() {
    // --no-input must NOT write to ~/.config/tome/config.toml even under greenfield.
    // (RESEARCH.md § "Integration with no_input" — "Skip" row for WUX-05.)
    let tmp = TempDir::new().unwrap();

    let output = tome()
        .args(["init", "--dry-run", "--no-input"])
        .env("HOME", tmp.path())
        .env_remove("TOME_HOME")
        .env("NO_COLOR", "1")
        .output()
        .unwrap();
    assert!(output.status.success());

    let xdg = tmp.path().join(".config/tome/config.toml");
    assert!(
        !xdg.exists(),
        "--no-input must not write XDG config, but {} exists",
        xdg.display()
    );
}

#[test]
fn init_with_flag_source_skips_step_0() {
    // TomeHomeSource::CliTomeHome (from --tome-home flag) → Step 0 MUST be skipped
    // even without --no-input, because the user already indicated a choice.
    // We test via --no-input to keep the test headless; the key assertion is on
    // the "Step 0:" header absence.
    let tmp = TempDir::new().unwrap();
    let custom = tmp.path().join("custom-home");

    let output = tome()
        .args([
            "init",
            "--dry-run",
            "--no-input",
            "--tome-home",
            custom.to_str().unwrap(),
        ])
        .env("HOME", tmp.path())
        .env_remove("TOME_HOME")
        .env("NO_COLOR", "1")
        .output()
        .unwrap();
    assert!(output.status.success());

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        !stdout.contains("Step 0:"),
        "--tome-home flag (CliTomeHome source) must skip Step 0:\n{stdout}"
    );
    assert!(
        stdout.contains("(from --tome-home flag)"),
        "source label should confirm flag branch:\n{stdout}"
    );
}

#[test]
fn init_derived_library_default_under_custom_tome_home() {
    // When tome_home = /tmp/custom-tome (non-default), library default should derive
    // as /tmp/custom-tome/skills (NOT ~/.tome/skills). Tests the Pitfall 1 fix.
    let tmp = TempDir::new().unwrap();
    let custom = tmp.path().join("custom-tome");

    let output = tome()
        .args([
            "init",
            "--dry-run",
            "--no-input",
            "--tome-home",
            custom.to_str().unwrap(),
        ])
        .env("HOME", tmp.path())
        .env_remove("TOME_HOME")
        .env("NO_COLOR", "1")
        .output()
        .unwrap();
    assert!(output.status.success(),
        "stderr: {}", String::from_utf8_lossy(&output.stderr));

    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let config = parse_generated_config(&stdout);
    // library_dir after tilde expansion should be under the custom tome_home,
    // NOT under tmp/.tome/skills.
    assert_eq!(
        config.library_dir(),
        custom.join("skills"),
        "library default should derive from --tome-home, got {:?}",
        config.library_dir()
    );
}
```
  </action>
  <verify>
    <automated>cargo test --package tome --test cli -- init_greenfield_no_input init_with_flag_source_skips init_derived_library_default 2>&1 | tail -20</automated>
  </verify>
  <acceptance_criteria>
    - `rg -n "fn init_greenfield_no_input_skips_step_0_prompt" crates/tome/tests/cli.rs` returns a match
    - `rg -n "fn init_greenfield_no_input_does_not_write_xdg" crates/tome/tests/cli.rs` returns a match
    - `rg -n "fn init_with_flag_source_skips_step_0" crates/tome/tests/cli.rs` returns a match
    - `rg -n "fn init_derived_library_default_under_custom_tome_home" crates/tome/tests/cli.rs` returns a match
    - `cargo test --package tome --test cli -- init_greenfield_no_input init_with_flag_source_skips init_derived_library_default` exits 0 and reports 4 tests passing
    - All existing init_* tests still pass: `cargo test --package tome --test cli -- init_` exits 0
    - `cargo clippy --all-targets -- -D warnings` exits 0
    - `cargo fmt -- --check` exits 0
  </acceptance_criteria>
  <done>
    Integration tests lock in: (a) Step 0 is skipped under --no-input, (b) --no-input does NOT write to the XDG config location, (c) Step 0 is also skipped when TomeHomeSource is NOT Default (flag branch), and (d) library default derives from the chosen tome_home (Pitfall 1 from RESEARCH.md).
  </done>
</task>

</tasks>

<verification>
- `cargo test --package tome` exits 0 (all pre-existing + new tests pass)
- `cargo clippy --all-targets -- -D warnings` exits 0
- `cargo fmt -- --check` exits 0
- `make ci` exits 0
- The wizard.rs:310 bug fix is verifiable: `rg -n "default_config_path\\(\\)" crates/tome/src/wizard.rs` returns 0 non-comment matches
</verification>

<success_criteria>
- WUX-01 success criterion from ROADMAP.md line 45: "User running `tome init` on a greenfield machine (no `TOME_HOME`, no XDG config, no existing `.tome/tome.toml`) sees a prompt to choose `tome_home` with `~/.tome/` as the default and a custom-path option that is validated before the wizard proceeds" — demonstrably TRUE via the Step 0 prompt code gated on `TomeHomeSource::Default`, validated by `configure_library_no_input_derives_from_tome_home` unit test + `init_greenfield_no_input_skips_step_0_prompt` integration test (covers the no_input branch; interactive branch is tested by manual exercise only, per RESEARCH.md § Pitfall 5)
- WUX-05 success criterion from ROADMAP.md line 49: "When the user selects a custom `tome_home` in the greenfield flow, wizard offers to persist the choice by writing `~/.config/tome/config.toml`" — demonstrably TRUE via `write_xdg_tome_home` helper + its 3 unit tests + the Confirm prompt in the wizard. --no-input does NOT write XDG (locked by `init_greenfield_no_input_does_not_write_xdg`)
- Latent wizard.rs:310 bug fixed: save path now uses `resolve_config_dir(tome_home)` consistently with what `sync()` uses
- Pitfall 1 fixed: library default now derives from chosen tome_home, locked by `init_derived_library_default_under_custom_tome_home`
</success_criteria>

<output>
After completion, create `.planning/phases/07-wizard-ux-greenfield-brownfield-legacy/07-03-SUMMARY.md` with:
- New `wizard::run` signature (4 args) for plan 04 to extend
- Exact line number of the Step 0 block in wizard.rs (for plan 04 to insert brownfield pre-flight after)
- Exact line number of the `configure_library` call (for plan 04 to pass prefill through)
- Note that plan 04 will add a 5th param `prefill: Option<&Config>` to `run` and each helper
</output>
