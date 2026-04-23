---
phase: 07-wizard-ux-greenfield-brownfield-legacy
plan: 04
type: execute
wave: 3
depends_on:
  - 07-01-wux-04-resolved-tome-home-info
  - 07-02-wux-03-legacy-config-detection
  - 07-03-wux-01-05-tome-home-prompt
files_modified:
  - crates/tome/src/wizard.rs
  - crates/tome/src/lib.rs
  - crates/tome/tests/cli.rs
autonomous: true
requirements:
  - WUX-02
must_haves:
  truths:
    - "When `tome init` detects an existing tome.toml at the resolved tome_home, the user sees a summary (directory count, library_dir, last-modified info) and a 4-option prompt: use existing, edit existing, reinitialize, cancel"
    - "Default action is 'use existing' (D-3 locked decision), so under --no-input the existing config is left untouched"
    - "When 'reinitialize' is chosen, the existing tome.toml is copied to `tome.toml.backup-<timestamp>` before the wizard overwrites it — the original is never lost"
    - "When 'edit existing' is chosen, the wizard starts with directory selections, library_dir, and exclusions pre-filled from the existing config; custom directories (not in KNOWN_DIRECTORIES) are preserved through edit (Pitfall 2)"
    - "When 'cancel' is chosen, `tome init` exits cleanly (exit code 0) without running the post-init sync"
    - "When the existing config fails to parse, only 'reinitialize' and 'cancel' are offered (not 'use existing' or 'edit')"
  artifacts:
    - path: "crates/tome/src/wizard.rs"
      provides: "BrownfieldAction enum; brownfield_action dispatcher; wizard::run accepts Option<&Config> prefill; configure_directories/library/exclusions accept prefill"
      contains: "enum BrownfieldAction"
    - path: "crates/tome/src/lib.rs"
      provides: "Command::Init matches on MachineState brownfield variants and dispatches to UseExisting/Edit/Reinit/Cancel"
      contains: "BrownfieldAction"
    - path: "crates/tome/tests/cli.rs"
      provides: "integration tests: brownfield --no-input keeps existing; parse failure shows reduced options; custom directories preserved through edit"
      contains: "init_brownfield"
  key_links:
    - from: "crates/tome/src/lib.rs Command::Init"
      to: "wizard::run with prefill"
      via: "matches on MachineState::Brownfield + BrownfieldWithLegacy after plan 02's legacy cleanup"
      pattern: "MachineState::Brownfield"
    - from: "crates/tome/src/wizard.rs configure_directories"
      to: "prefill entries"
      via: "union with existing map so custom directories survive edit"
      pattern: "prefill"
    - from: "crates/tome/src/wizard.rs reinitialize path"
      to: "backup file"
      via: "std::fs::copy to tome.toml.backup-<unix-timestamp>"
      pattern: "backup-"
---

<objective>
Complete Phase 7 by implementing WUX-02: on a brownfield machine (existing `tome.toml` at the resolved tome_home), show the user a summary and offer a 4-way decision (use existing / edit existing / reinitialize / cancel) before any destructive action. This is the largest of the four plans because "edit existing" requires plumbing `Option<&Config>` prefill through every wizard helper.

Purpose: Today, `tome init` on a brownfield machine blows through and overwrites the existing config — the dotfiles-sync workflow that triggered the whole v0.8 milestone. This plan closes the gap by: (1) detecting brownfield via the `MachineState` from plan 02, (2) presenting a non-destructive default ("use existing"), (3) backing up the existing file before reinitialize, (4) pre-filling the wizard when the user chooses "edit", (5) preserving custom directories through edit.

Output: `BrownfieldAction` enum + summary + dispatch in wizard.rs, `wizard::run` accepts `Option<&Config>` prefill, `configure_directories/library/exclusions` accept prefill, `lib.rs` dispatches on all 4 MachineState variants, and integration tests lock in --no-input safety + parse-failure handling + edit-preserves-custom-directories.
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
@.planning/phases/07-wizard-ux-greenfield-brownfield-legacy/07-02-SUMMARY.md
@.planning/phases/07-wizard-ux-greenfield-brownfield-legacy/07-03-SUMMARY.md
@crates/tome/src/wizard.rs
@crates/tome/src/lib.rs
@crates/tome/src/config.rs
@crates/tome/tests/cli.rs

<interfaces>
<!-- Consumed from plan 02 -->
```rust
pub(crate) enum MachineState {
    Greenfield,
    Brownfield { existing_config_path: PathBuf, existing_config: Result<Config> },
    Legacy { legacy_path: PathBuf },
    BrownfieldWithLegacy { existing_config_path: PathBuf, existing_config: Result<Config>, legacy_path: PathBuf },
}
pub(crate) fn detect_machine_state(home: &Path, tome_home: &Path) -> Result<MachineState>;
pub(crate) fn handle_legacy_cleanup(legacy_path: &Path, no_input: bool) -> Result<()>;
```

<!-- Consumed from plan 03 -->
```rust
pub(crate) fn run(
    dry_run: bool,
    no_input: bool,
    tome_home: &Path,
    tome_home_source: TomeHomeSource,
) -> Result<Config>;                                    // will gain 5th param in THIS plan

fn configure_library(no_input: bool, tome_home: &Path) -> Result<PathBuf>;  // will gain prefill
```

<!-- Target new signatures -->
```rust
pub(crate) fn run(
    dry_run: bool,
    no_input: bool,
    tome_home: &Path,
    tome_home_source: TomeHomeSource,
    prefill: Option<&Config>,           // NEW for WUX-02 edit mode
) -> Result<Config>;

fn configure_directories(
    no_input: bool,
    prefill: Option<&BTreeMap<DirectoryName, DirectoryConfig>>,  // NEW
) -> Result<BTreeMap<DirectoryName, DirectoryConfig>>;

fn configure_library(
    no_input: bool,
    tome_home: &Path,
    prefill: Option<&Path>,             // NEW
) -> Result<PathBuf>;

fn configure_exclusions(
    skills: &[crate::discover::DiscoveredSkill],
    no_input: bool,
    prefill: Option<&BTreeSet<SkillName>>,  // NEW
) -> Result<BTreeSet<SkillName>>;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum BrownfieldAction { UseExisting, Edit, Reinit, Cancel }

pub(crate) fn brownfield_decision(
    existing_config_path: &Path,
    existing_config: &Result<Config>,
    no_input: bool,
) -> Result<BrownfieldAction>;

/// Copies the existing tome.toml to `<parent>/tome.toml.backup-<unix-ts>`.
pub(crate) fn backup_brownfield_config(existing_config_path: &Path) -> Result<PathBuf>;
```
</interfaces>
</context>

<tasks>

<task type="auto" tdd="true">
  <name>Task 1: Add BrownfieldAction enum, brownfield_decision, and backup_brownfield_config in wizard.rs</name>
  <files>crates/tome/src/wizard.rs</files>
  <read_first>
    - crates/tome/src/wizard.rs (focus: the MachineState block added in plan 02; the imports at the top of the file; the `step_divider` helper and `console::style` usage)
    - crates/tome/src/config.rs (focus: the `Config::directories()`, `Config::library_dir()` accessors)
    - crates/tome/src/paths.rs (focus: `collapse_home` at line 142)
    - .planning/phases/07-wizard-ux-greenfield-brownfield-legacy/07-RESEARCH.md (focus: lines 281–381 — WUX-02 Brownfield decision; lines 362–372 — reinit backup approach; lines 496–502 — no_input behaviors)
  </read_first>
  <behavior>
    - Test 1: `brownfield_decision(path, &Ok(config), /* no_input = */ true)` returns `BrownfieldAction::UseExisting` (D-3)
    - Test 2: `brownfield_decision(path, &Err(...), /* no_input = */ true)` returns `BrownfieldAction::UseExisting` — but this behavior is undefined for invalid configs; prefer returning `BrownfieldAction::Cancel` to avoid advancing with an invalid config in headless mode (pick one, document choice)
    - Test 3: `backup_brownfield_config(<existing>)` copies the file to `<parent>/tome.toml.backup-<ts>` and the original remains (copy, not rename)
    - Test 4: `backup_brownfield_config` returns the backup path so callers can surface it to the user
  </behavior>
  <action>
**Part A — Add `BrownfieldAction` enum** (place near the `MachineState` enum from plan 02):

```rust
/// User's choice for how to handle an existing tome.toml at the resolved tome_home.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum BrownfieldAction {
    /// Exit the wizard cleanly, leaving the existing file untouched. Default for --no-input.
    UseExisting,
    /// Run the wizard with existing values pre-filled. Preserves custom directories.
    Edit,
    /// Back up the existing file to `tome.toml.backup-<unix-ts>` and run wizard as greenfield.
    Reinit,
    /// Exit the wizard without sync; print a short confirmation line to stdout.
    Cancel,
}
```

**Part B — Add `brownfield_decision` function:**

```rust
/// Display the brownfield summary and prompt the user for an action.
///
/// - `no_input=true` returns `UseExisting` (D-3 locked decision — safest for dotfiles workflow)
///   when the existing config parses successfully, or `Cancel` when it does not (no silent
///   advance with invalid config).
/// - Otherwise, Select with default=0 (UseExisting) for parseable configs, or a reduced
///   `[Reinitialize, Cancel]` menu for unparseable configs.
pub(crate) fn brownfield_decision(
    existing_config_path: &Path,
    existing_config: &Result<Config>,
    no_input: bool,
) -> Result<BrownfieldAction> {
    step_divider("Existing config detected");
    println!(
        "  {} {}",
        style("path:").bold(),
        style(existing_config_path.display()).cyan()
    );
    match existing_config {
        Ok(c) => {
            println!("  directories: {}", c.directories().len());
            println!(
                "  library_dir: {}",
                crate::paths::collapse_home(&c.library_dir)
            );
            // Last-modified summary; relative-friendly if possible, else ISO.
            if let Ok(meta) = std::fs::metadata(existing_config_path) {
                if let Ok(mtime) = meta.modified() {
                    if let Ok(dur) = std::time::SystemTime::now().duration_since(mtime) {
                        println!("  last modified: {} ago", format_duration(dur));
                    }
                }
            }
        }
        Err(e) => {
            println!("  {} {:#}", style("invalid:").red(), e);
            println!("  ('use existing' and 'edit' unavailable while config is invalid)");
        }
    }
    println!();

    // --no-input: D-3 says default = UseExisting. But refuse to default when
    // the config doesn't parse — in headless mode, advancing with an invalid
    // config would be surprising. Return Cancel so the caller exits cleanly
    // and the user investigates.
    if no_input {
        return Ok(if existing_config.is_ok() {
            BrownfieldAction::UseExisting
        } else {
            BrownfieldAction::Cancel
        });
    }

    // Interactive: offer different menus based on whether the config parses.
    let selection = if existing_config.is_ok() {
        let items = [
            "Use existing (exit wizard, run `tome sync`)",
            "Edit existing (pre-fill wizard with current values)",
            "Reinitialize (backup + overwrite)",
            "Cancel",
        ];
        Select::new()
            .with_prompt("What do you want to do?")
            .items(&items)
            .default(0)
            .interact()?
    } else {
        // No "use existing" or "edit" when parse failed
        let items = ["Reinitialize (backup + overwrite)", "Cancel"];
        let idx = Select::new()
            .with_prompt("What do you want to do?")
            .items(&items)
            .default(0)
            .interact()?;
        return Ok(if idx == 0 { BrownfieldAction::Reinit } else { BrownfieldAction::Cancel });
    };
    Ok(match selection {
        0 => BrownfieldAction::UseExisting,
        1 => BrownfieldAction::Edit,
        2 => BrownfieldAction::Reinit,
        3 => BrownfieldAction::Cancel,
        _ => unreachable!("Select returned out-of-range index"),
    })
}

/// Best-effort human-readable duration for last-modified display.
fn format_duration(dur: std::time::Duration) -> String {
    let secs = dur.as_secs();
    if secs < 60 {
        format!("{}s", secs)
    } else if secs < 3600 {
        format!("{}m", secs / 60)
    } else if secs < 86400 {
        format!("{}h", secs / 3600)
    } else {
        format!("{}d", secs / 86400)
    }
}
```

**Part C — Add `backup_brownfield_config` function:**

```rust
/// Copy `existing_config_path` to `<parent>/tome.toml.backup-<unix-ts>`.
///
/// Uses copy (not rename) so that a Cancel later in the flow leaves the original intact.
/// Returns the backup path.
pub(crate) fn backup_brownfield_config(existing_config_path: &Path) -> Result<PathBuf> {
    let ts = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .context("system clock is before UNIX epoch")?
        .as_secs();
    let backup_name = format!("tome.toml.backup-{ts}");
    let parent = existing_config_path
        .parent()
        .context("existing config path has no parent directory")?;
    let backup_path = parent.join(&backup_name);
    std::fs::copy(existing_config_path, &backup_path).with_context(|| {
        format!(
            "failed to copy {} -> {}",
            existing_config_path.display(),
            backup_path.display()
        )
    })?;
    Ok(backup_path)
}
```

**Part D — Unit tests** inside `#[cfg(test)] mod tests` in wizard.rs:

```rust
#[test]
fn brownfield_decision_no_input_returns_use_existing_for_valid_config() {
    let tmp = TempDir::new().unwrap();
    let path = tmp.path().join("tome.toml");
    std::fs::write(&path, "library_dir = \"~/.tome/skills\"\n[directories]\n").unwrap();
    let cfg: Result<Config> = Config::load(&path);
    assert!(cfg.is_ok());

    let action = brownfield_decision(&path, &cfg, /* no_input = */ true).unwrap();
    assert_eq!(action, BrownfieldAction::UseExisting);
}

#[test]
fn brownfield_decision_no_input_returns_cancel_for_invalid_config() {
    let tmp = TempDir::new().unwrap();
    let path = tmp.path().join("tome.toml");
    std::fs::write(&path, "this is [[[ not valid toml").unwrap();
    let cfg: Result<Config> = Config::load(&path);
    assert!(cfg.is_err());

    let action = brownfield_decision(&path, &cfg, /* no_input = */ true).unwrap();
    assert_eq!(action, BrownfieldAction::Cancel);
}

#[test]
fn backup_brownfield_config_copies_file() {
    let tmp = TempDir::new().unwrap();
    let path = tmp.path().join("tome.toml");
    let original_content = "library_dir = \"~/.tome/skills\"\n";
    std::fs::write(&path, original_content).unwrap();

    let backup_path = backup_brownfield_config(&path).unwrap();
    assert!(backup_path.exists(), "backup file should exist");
    assert!(path.exists(), "original should still exist (copy, not rename)");
    assert_eq!(
        std::fs::read_to_string(&backup_path).unwrap(),
        original_content,
        "backup should have identical content"
    );
    assert!(
        backup_path.file_name().unwrap().to_str().unwrap().starts_with("tome.toml.backup-"),
        "backup filename must start with tome.toml.backup-: {:?}",
        backup_path.file_name()
    );
}
```
  </action>
  <verify>
    <automated>cargo test --package tome -- wizard::tests::brownfield_decision wizard::tests::backup_brownfield 2>&1 | tail -20</automated>
  </verify>
  <acceptance_criteria>
    - `rg -n "pub\\(crate\\) enum BrownfieldAction" crates/tome/src/wizard.rs` returns a match
    - `rg -n "UseExisting|Edit|Reinit|Cancel" crates/tome/src/wizard.rs` shows all 4 variants
    - `rg -n "pub\\(crate\\) fn brownfield_decision" crates/tome/src/wizard.rs` returns a match
    - `rg -n "pub\\(crate\\) fn backup_brownfield_config" crates/tome/src/wizard.rs` returns a match
    - `rg -n "tome\\.toml\\.backup-" crates/tome/src/wizard.rs` returns a match (backup filename pattern)
    - `rg -n "fs::copy" crates/tome/src/wizard.rs` returns a match inside backup_brownfield_config (copy, not rename)
    - `cargo test --package tome -- wizard::tests::brownfield_decision_no_input_returns_use_existing_for_valid_config` exits 0
    - `cargo test --package tome -- wizard::tests::brownfield_decision_no_input_returns_cancel_for_invalid_config` exits 0
    - `cargo test --package tome -- wizard::tests::backup_brownfield_config_copies_file` exits 0
    - `cargo clippy --all-targets -- -D warnings` exits 0
  </acceptance_criteria>
  <done>
    `BrownfieldAction` enum with 4 variants exists. `brownfield_decision` under --no-input returns `UseExisting` for valid configs and `Cancel` for invalid ones (no silent advance with broken config). `backup_brownfield_config` creates a timestamped `.backup-<ts>` file via copy (not rename). Unit tests lock in these behaviors.
  </done>
</task>

<task type="auto" tdd="true">
  <name>Task 2: Plumb Option<&Config> prefill through wizard::run and configure_* helpers; preserve custom directories on edit</name>
  <files>crates/tome/src/wizard.rs</files>
  <read_first>
    - crates/tome/src/wizard.rs (focus: the run signature from plan 03; configure_directories starting around line 400+; configure_library ~519; configure_exclusions ~551; assemble_config)
    - crates/tome/src/config.rs (focus: Config::directories accessor, DirectoryConfig, DirectoryName types)
    - crates/tome/src/discover.rs (focus: SkillName, DiscoveredSkill)
    - .planning/phases/07-wizard-ux-greenfield-brownfield-legacy/07-RESEARCH.md (focus: lines 342–363 — edit existing plumbing; lines 556–560 — Pitfall 2 drift; lines 658–666 — custom-directory preservation)
  </read_first>
  <behavior>
    - Test 1: `configure_directories(no_input=true, Some(prefill_with_custom_dir))` returns a map that INCLUDES the custom directory (not in KNOWN_DIRECTORIES) — Pitfall 2 fix
    - Test 2: `configure_library(no_input=true, tome_home, Some(&prefilled_path))` returns the prefilled path (not the computed default)
    - Test 3: `configure_exclusions(skills, no_input=true, Some(&prefill_set))` returns the prefill set (not empty)
    - Test 4: `wizard::run` accepts the new 5th argument `Option<&Config>` and compiles
  </behavior>
  <action>
**Part A — Update `wizard::run` signature:**

Change wizard.rs:137 signature (as modified by plan 03) from:
```rust
pub(crate) fn run(
    dry_run: bool,
    no_input: bool,
    tome_home: &Path,
    tome_home_source: crate::config::TomeHomeSource,
) -> Result<Config>
```
to:
```rust
pub(crate) fn run(
    dry_run: bool,
    no_input: bool,
    tome_home: &Path,
    tome_home_source: crate::config::TomeHomeSource,
    prefill: Option<&Config>,
) -> Result<Config>
```

**Part B — Update the helper calls inside `run`:**

Inside `run`, change (from plan 03's version):

```rust
let mut directories = configure_directories(no_input)?;
```
to:
```rust
let mut directories = configure_directories(
    no_input,
    prefill.map(|c| c.directories()),
)?;
```

```rust
let library_dir = configure_library(no_input, tome_home)?;
```
to:
```rust
let library_dir = configure_library(
    no_input,
    tome_home,
    prefill.map(|c| c.library_dir.as_path()),
)?;
```

```rust
let exclude = configure_exclusions(&discovered, no_input)?;
```
to:
```rust
let exclude = configure_exclusions(
    &discovered,
    no_input,
    prefill.map(|c| c.exclude()),
)?;
```

Note: verify exact accessor names — `rg -n "pub fn directories|pub fn library_dir|pub fn exclude" crates/tome/src/config.rs`. If `library_dir` is a direct public field (it IS — `pub library_dir: PathBuf`), use `&c.library_dir` directly. If `exclude` is a method vs field, adjust accordingly.

**Part C — Update `configure_directories` signature:**

```rust
fn configure_directories(
    no_input: bool,
    prefill: Option<&std::collections::BTreeMap<
        crate::config::DirectoryName,
        crate::config::DirectoryConfig,
    >>,
) -> Result<std::collections::BTreeMap<crate::config::DirectoryName, crate::config::DirectoryConfig>> {
    // ... existing body to auto-discover known directories ...
    //
    // After the selection/build step, UNION with any prefill entries not present
    // in the auto-discovered map (Pitfall 2 from RESEARCH.md).
    //
    // At the end, before returning `directories`:
    if let Some(prefill_map) = prefill {
        for (name, cfg) in prefill_map {
            directories.entry(name.clone()).or_insert_with(|| cfg.clone());
        }
    }
    Ok(directories)
}
```

**Critical:** The union must be added at the END of the function body, after the existing auto-discovery + user-selection logic, so that:
- Pre-filled entries that overlap with known directories use the auto-discovered version (user's selections win)
- Pre-filled entries that DO NOT overlap (custom directories like `my-team`) survive the edit — Pitfall 2

If the existing function has multiple return paths, ensure the union runs on all of them.

Under --no-input with prefill: today the function auto-selects all known directories. With prefill, it should ADDITIONALLY pick up any prefill entries not in KNOWN_DIRECTORIES. The `entry().or_insert_with()` union achieves both.

**Part D — Update `configure_library` signature:**

```rust
fn configure_library(
    no_input: bool,
    tome_home: &Path,
    prefill: Option<&Path>,
) -> Result<PathBuf> {
    step_divider("Step 2: Library location");

    let default = crate::paths::collapse_home_path(&tome_home.join("skills"));

    // Under --no-input: use prefill if given, else default.
    if no_input {
        return Ok(prefill.map(|p| p.to_path_buf()).unwrap_or(default));
    }

    // Interactive: build options with prefill as an extra leading choice if it differs from default.
    let mut options: Vec<String> = Vec::new();
    let mut paths: Vec<PathBuf> = Vec::new();
    if let Some(prefilled) = prefill {
        if prefilled != default {
            options.push(format!("{} (current)", crate::paths::collapse_home(prefilled)));
            paths.push(prefilled.to_path_buf());
        }
    }
    options.push(format!("{} (default)", default.display()));
    paths.push(default.clone());
    options.push("Custom path...".to_string());
    // (Custom path handled via an explicit Input below)

    let selection = Select::new()
        .with_prompt("Where should the skill library live?")
        .items(&options)
        .default(0)
        .interact()?;

    if selection == options.len() - 1 {
        // "Custom path..." — last option
        let custom: String = Input::new().with_prompt("Library path").interact_text()?;
        Ok(crate::paths::collapse_home_path(&expand_tilde(&PathBuf::from(custom))?))
    } else {
        Ok(paths[selection].clone())
    }
}
```

**Part E — Update `configure_exclusions` signature:**

```rust
fn configure_exclusions(
    skills: &[crate::discover::DiscoveredSkill],
    no_input: bool,
    prefill: Option<&std::collections::BTreeSet<crate::discover::SkillName>>,
) -> Result<std::collections::BTreeSet<crate::discover::SkillName>> {
    step_divider("Step 3: Exclusions");

    // Under --no-input: use prefill if given, else empty.
    if no_input {
        return Ok(prefill.cloned().unwrap_or_default());
    }

    if skills.is_empty() {
        println!("  (no skills discovered yet -- exclusions can be added manually to config)");
        println!();
        return Ok(prefill.cloned().unwrap_or_default());
    }

    let labels: Vec<String> = skills.iter().map(|s| s.name.to_string()).collect();
    // When prefill is provided, pre-select those already-excluded skills.
    let defaults: Vec<bool> = skills
        .iter()
        .map(|s| prefill.is_some_and(|p| p.contains(&s.name)))
        .collect();

    let max_rows = console::Term::stderr().size().0.saturating_sub(6).max(5) as usize;
    let selections: Vec<usize> = MultiSelect::new()
        .with_prompt("Select skills to exclude (space to toggle, enter to confirm)")
        .items(&labels)
        .defaults(&defaults)
        .max_length(max_rows)
        .interact()?;

    let mut exclude = prefill.cloned().unwrap_or_default();
    // Clear then rebuild so de-selecting a previously-excluded skill actually removes it.
    exclude.clear();
    for idx in selections {
        exclude.insert(skills[idx].name.clone());
    }
    Ok(exclude)
}
```

**Part F — Unit tests** for prefill paths (no_input branches only, per Pitfall 5):

```rust
#[test]
fn configure_directories_preserves_custom_prefill() {
    // A custom directory (not in KNOWN_DIRECTORIES) must survive through edit.
    // Under --no-input, auto-discovery on an empty HOME returns nothing,
    // so the result map should equal the prefill.
    use crate::config::{DirectoryConfig, DirectoryName, DirectoryRole, DirectoryType};

    let mut prefill_map = std::collections::BTreeMap::new();
    prefill_map.insert(
        DirectoryName::new("my-team").unwrap(),
        DirectoryConfig {
            path: PathBuf::from("/tmp/my-team"),
            directory_type: DirectoryType::Directory,
            role: Some(DirectoryRole::Synced),
            branch: None,
            tag: None,
            rev: None,
            subdir: None,
        },
    );

    // Isolate HOME so auto-discovery doesn't match anything real
    let tmp = TempDir::new().unwrap();
    with_env(&[("HOME", Some(tmp.path().as_os_str()))], || {
        let result = configure_directories(true, Some(&prefill_map)).unwrap();
        assert!(
            result.contains_key(&DirectoryName::new("my-team").unwrap()),
            "custom directory 'my-team' should survive edit. Got: {:?}",
            result.keys().collect::<Vec<_>>()
        );
    });
}

#[test]
fn configure_library_no_input_uses_prefill() {
    let prefilled = PathBuf::from("/custom/library");
    let tome_home = Path::new("/tmp/any");
    let result = configure_library(true, tome_home, Some(&prefilled)).unwrap();
    assert_eq!(result, prefilled);
}

#[test]
fn configure_library_no_input_uses_derived_default_when_no_prefill() {
    let tome_home = Path::new("/tmp/zzz-not-under-home");
    let result = configure_library(true, tome_home, None).unwrap();
    assert_eq!(result, PathBuf::from("/tmp/zzz-not-under-home/skills"));
}

#[test]
fn configure_exclusions_no_input_uses_prefill() {
    use crate::discover::SkillName;
    let mut prefill = std::collections::BTreeSet::new();
    prefill.insert(SkillName::new("skill-a").unwrap());
    prefill.insert(SkillName::new("skill-b").unwrap());

    let result = configure_exclusions(&[], true, Some(&prefill)).unwrap();
    assert_eq!(result.len(), 2);
    assert!(result.contains(&SkillName::new("skill-a").unwrap()));
    assert!(result.contains(&SkillName::new("skill-b").unwrap()));
}
```

*Note on `with_env`:* reuse the helper added in plan 01 (or copy it into wizard.rs's test module if scoping issues arise).

*Note on `DirectoryConfig` field names:* the `rev`, `branch`, etc. field list must exactly match the current `DirectoryConfig` struct definition — verify with `rg -n "pub struct DirectoryConfig" crates/tome/src/config.rs` and adapt if the struct differs. Use `..Default::default()` if `DirectoryConfig: Default` is implemented.
  </action>
  <verify>
    <automated>cargo test --package tome -- wizard::tests::configure_directories_preserves wizard::tests::configure_library wizard::tests::configure_exclusions 2>&1 | tail -20</automated>
  </verify>
  <acceptance_criteria>
    - `rg -n "prefill: Option<&Config>" crates/tome/src/wizard.rs` returns a match in the `run` signature
    - `rg -n "prefill: Option<&std::collections::BTreeMap" crates/tome/src/wizard.rs` returns a match in `configure_directories`
    - `rg -n "prefill: Option<&Path>" crates/tome/src/wizard.rs` returns a match in `configure_library`
    - `rg -n "prefill: Option<&std::collections::BTreeSet" crates/tome/src/wizard.rs` returns a match in `configure_exclusions`
    - `rg -n "entry\\(name\\.clone\\(\\)\\)\\.or_insert_with" crates/tome/src/wizard.rs` returns a match in `configure_directories` (union for Pitfall 2)
    - `rg -n "fn configure_directories_preserves_custom_prefill" crates/tome/src/wizard.rs` returns a match
    - `rg -n "fn configure_library_no_input_uses_prefill" crates/tome/src/wizard.rs` returns a match
    - `rg -n "fn configure_exclusions_no_input_uses_prefill" crates/tome/src/wizard.rs` returns a match
    - `cargo test --package tome -- wizard::tests::configure_directories_preserves` exits 0
    - `cargo test --package tome -- wizard::tests::configure_library` exits 0 (2 tests: uses_prefill + derived_default)
    - `cargo test --package tome -- wizard::tests::configure_exclusions_no_input_uses_prefill` exits 0
    - `cargo clippy --all-targets -- -D warnings` exits 0
    - `cargo fmt -- --check` exits 0
  </acceptance_criteria>
  <done>
    `wizard::run` accepts `Option<&Config>` prefill; all three helpers accept their narrow prefill types; `configure_directories` unions prefill with auto-discovered entries (custom directories survive); `configure_library` uses prefill under --no-input and shows it as a "current" option interactively; `configure_exclusions` uses prefill defaults. Unit tests lock in the Pitfall 2 fix and the prefill-under-no_input behavior.
  </done>
</task>

<task type="auto" tdd="true">
  <name>Task 3: Wire brownfield dispatch in lib.rs Command::Init; integration tests</name>
  <files>crates/tome/src/lib.rs, crates/tome/tests/cli.rs</files>
  <read_first>
    - crates/tome/src/lib.rs (focus: Command::Init branch with the plan 01 + plan 02 + plan 03 edits already in place; the `let _ = machine_state;` placeholder from plan 02 must be replaced)
    - crates/tome/src/wizard.rs (focus: the newly-added BrownfieldAction, brownfield_decision, backup_brownfield_config; the updated run signature with prefill)
    - crates/tome/tests/cli.rs (focus: existing init tests + plan 02's legacy tests for isolation patterns)
    - .planning/phases/07-wizard-ux-greenfield-brownfield-legacy/07-RESEARCH.md (focus: lines 281–381 — Brownfield decision; open question lines 563–568 — cancel exit path)
  </read_first>
  <behavior>
    - Test 1 (integration): `init_brownfield_no_input_keeps_existing` — seed a valid tome.toml at TOME_HOME/tome.toml, run `init --no-input`, assert the file is byte-identical after AND no post-init sync occurs (no library dir created)
    - Test 2 (integration): `init_brownfield_invalid_config_no_input_cancels` — seed a malformed tome.toml, run `init --no-input`, assert exit code 0 (clean cancel, not error) AND file is unchanged
    - Test 3 (integration): `init_brownfield_with_legacy_runs_both_cleanups` — seed BOTH a tome.toml AND a legacy XDG file, run `init --no-input`, assert both files unchanged AND stdout has both the legacy warning AND the brownfield summary
  </behavior>
  <action>
**Part A — Replace the `let _ = machine_state;` placeholder in lib.rs** (added in plan 02):

Locate the Command::Init branch and find the block plan 02 added (after the `resolved tome_home:` print). Replace the placeholder with the full dispatch:

```rust
// WUX-03 + WUX-02: handle MachineState
let home = dirs::home_dir().context("could not determine home directory")?;
let machine_state = wizard::detect_machine_state(&home, &tome_home)?;

// First: legacy cleanup (may be present in both Legacy and BrownfieldWithLegacy)
if let wizard::MachineState::Legacy { legacy_path }
    | wizard::MachineState::BrownfieldWithLegacy { legacy_path, .. } = &machine_state
{
    wizard::handle_legacy_cleanup(legacy_path, cli.no_input)?;
}

// Second: brownfield decision (Brownfield and BrownfieldWithLegacy)
let prefill: Option<Config> = match &machine_state {
    wizard::MachineState::Brownfield { existing_config_path, existing_config }
    | wizard::MachineState::BrownfieldWithLegacy { existing_config_path, existing_config, .. } => {
        let action = wizard::brownfield_decision(
            existing_config_path,
            existing_config,
            cli.no_input,
        )?;
        match action {
            wizard::BrownfieldAction::UseExisting => {
                println!("  Config unchanged. Run `tome sync` to apply.");
                return Ok(());
            }
            wizard::BrownfieldAction::Cancel => {
                println!("Wizard cancelled. Existing config left unchanged.");
                return Ok(());
            }
            wizard::BrownfieldAction::Reinit => {
                let backup = wizard::backup_brownfield_config(existing_config_path)?;
                println!(
                    "  Backed up existing config to: {}",
                    console::style(backup.display()).cyan()
                );
                None // proceed as greenfield
            }
            wizard::BrownfieldAction::Edit => {
                // Load the existing config for prefill (may re-read if existing_config was Ok).
                // If existing_config is Err, Edit wouldn't have been offered — unreachable.
                match existing_config {
                    Ok(c) => Some(c.clone()),
                    Err(_) => unreachable!("Edit action not offered for unparseable configs"),
                }
            }
        }
    }
    _ => None,
};

let config = wizard::run(
    cli.dry_run,
    cli.no_input,
    &tome_home,
    tome_home_source,
    prefill.as_ref(),
)?;
```

Remove the `let _ = machine_state;` line. The old `let config = wizard::run(...)?;` call from plan 03 is replaced by the new call with `prefill.as_ref()`.

*Note:* `Config: Clone` must hold — grep to confirm: `rg -n "#\\[derive.*Clone.*\\]" crates/tome/src/config.rs | head -5` and look for the derive on `pub struct Config`. If Clone isn't derived, either add it (low-risk — Config is a data struct) or pass `existing_config` by reference through `wizard::run` (requires a lifetime parameter). Cloning is simpler.

**Part B — Integration tests in cli.rs:**

Add after plan 03's tests. These tests need to seed a `tome.toml` at the brownfield location determined by `resolve_config_dir(tome_home)`. Given the test uses TOME_HOME as `tmp/.tome`, and TOME_HOME has no `.tome/` subdirectory pre-existing, `resolve_config_dir` returns `tmp/.tome` itself, so the brownfield file lives at `tmp/.tome/tome.toml`.

```rust
#[test]
fn init_brownfield_no_input_keeps_existing() {
    let tmp = TempDir::new().unwrap();
    let tome_home = tmp.path().join(".tome");
    std::fs::create_dir_all(&tome_home).unwrap();
    let config_path = tome_home.join("tome.toml");
    let seed = "library_dir = \"~/.tome/skills\"\n[directories]\n";
    std::fs::write(&config_path, seed).unwrap();

    let output = tome()
        .args(["init", "--no-input"]) // NOT --dry-run — we want the actual no-op path
        .env("HOME", tmp.path())
        .env("TOME_HOME", &tome_home)
        .env("NO_COLOR", "1")
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "tome init should succeed; stderr:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("Existing config detected"),
        "stdout missing brownfield summary:\n{stdout}"
    );

    // File must be byte-identical
    let after = std::fs::read_to_string(&config_path).unwrap();
    assert_eq!(after, seed, "brownfield --no-input must not modify existing config");

    // No sync side-effect: library dir should not be created
    let library = tmp.path().join(".tome/skills");
    assert!(
        !library.exists(),
        "use-existing path must not run post-init sync (library dir present at {})",
        library.display()
    );
}

#[test]
fn init_brownfield_invalid_config_no_input_cancels() {
    let tmp = TempDir::new().unwrap();
    let tome_home = tmp.path().join(".tome");
    std::fs::create_dir_all(&tome_home).unwrap();
    let config_path = tome_home.join("tome.toml");
    let seed = "this is [[[ not valid toml";
    std::fs::write(&config_path, seed).unwrap();

    let output = tome()
        .args(["init", "--no-input"])
        .env("HOME", tmp.path())
        .env("TOME_HOME", &tome_home)
        .env("NO_COLOR", "1")
        .output()
        .unwrap();
    // Must exit 0 (clean cancel), not an error
    assert!(
        output.status.success(),
        "invalid-config no-input path should cancel cleanly (exit 0); stderr:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("invalid:") || stdout.contains("cancelled"),
        "stdout should indicate invalid config or cancellation:\n{stdout}"
    );

    let after = std::fs::read_to_string(&config_path).unwrap();
    assert_eq!(after, seed, "invalid-config no-input must not modify the file");
}

#[test]
fn init_brownfield_with_legacy_runs_both_cleanups() {
    let tmp = TempDir::new().unwrap();
    let tome_home = tmp.path().join(".tome");
    std::fs::create_dir_all(&tome_home).unwrap();
    let config_path = tome_home.join("tome.toml");
    let brownfield_seed = "library_dir = \"~/.tome/skills\"\n[directories]\n";
    std::fs::write(&config_path, brownfield_seed).unwrap();

    let xdg_dir = tmp.path().join(".config/tome");
    let xdg_file = xdg_dir.join("config.toml");
    std::fs::create_dir_all(&xdg_dir).unwrap();
    let legacy_seed = "[[sources]]\nname = \"old\"\npath = \"/tmp\"\ntype = \"directory\"\n";
    std::fs::write(&xdg_file, legacy_seed).unwrap();

    let output = tome()
        .args(["init", "--no-input"])
        .env("HOME", tmp.path())
        .env("TOME_HOME", &tome_home)
        .env("NO_COLOR", "1")
        .output()
        .unwrap();
    assert!(output.status.success());

    let stdout = String::from_utf8_lossy(&output.stdout);
    // Both cleanup paths ran and printed their headers
    assert!(
        stdout.contains("Legacy pre-v0.6 config detected"),
        "stdout missing legacy warning:\n{stdout}"
    );
    assert!(
        stdout.contains("Existing config detected"),
        "stdout missing brownfield summary:\n{stdout}"
    );

    // Both files unchanged under --no-input
    assert_eq!(std::fs::read_to_string(&config_path).unwrap(), brownfield_seed);
    assert_eq!(std::fs::read_to_string(&xdg_file).unwrap(), legacy_seed);
}
```
  </action>
  <verify>
    <automated>cargo test --package tome --test cli -- init_brownfield 2>&1 | tail -25</automated>
  </verify>
  <acceptance_criteria>
    - `rg -n "wizard::BrownfieldAction" crates/tome/src/lib.rs` returns at least 3 matches (match arms for UseExisting, Cancel, Reinit, Edit)
    - `rg -n "wizard::backup_brownfield_config" crates/tome/src/lib.rs` returns a match (Reinit path calls it)
    - `rg -n "let _ = machine_state" crates/tome/src/lib.rs` returns 0 matches (the plan 02 placeholder is replaced)
    - `rg -n "prefill\\.as_ref\\(\\)" crates/tome/src/lib.rs` returns a match (wizard::run called with prefill)
    - `rg -n "fn init_brownfield_no_input_keeps_existing" crates/tome/tests/cli.rs` returns a match
    - `rg -n "fn init_brownfield_invalid_config_no_input_cancels" crates/tome/tests/cli.rs` returns a match
    - `rg -n "fn init_brownfield_with_legacy_runs_both_cleanups" crates/tome/tests/cli.rs` returns a match
    - `cargo test --package tome --test cli -- init_brownfield` exits 0 and reports 3 tests passing
    - ALL previously-written init tests (from plans 01, 02, 03 + pre-existing) still pass: `cargo test --package tome --test cli -- init_` exits 0
    - `cargo test --package tome` exits 0 (full test suite green)
    - `cargo clippy --all-targets -- -D warnings` exits 0
    - `cargo fmt -- --check` exits 0
    - `make ci` exits 0
  </acceptance_criteria>
  <done>
    `tome init` on a machine with an existing valid `tome.toml` prints a summary, offers 4 choices (default=UseExisting), and under --no-input leaves the file untouched and skips post-init sync. An invalid config cancels cleanly under --no-input (exit 0, file unchanged). A BrownfieldWithLegacy state runs BOTH the legacy cleanup prompt AND the brownfield summary, both leaving files untouched under --no-input. Edit and Reinit paths are implemented (Edit: prefill threaded through all helpers; Reinit: backup via `backup_brownfield_config` + proceed as greenfield) but covered by unit tests rather than interactive integration tests (per RESEARCH.md § Pitfall 5).
  </done>
</task>

</tasks>

<verification>
- `cargo test --package tome` exits 0 (all unit + integration tests across all 4 plans pass)
- `cargo clippy --all-targets -- -D warnings` exits 0
- `cargo fmt -- --check` exits 0
- `make ci` exits 0
- Phase 7 end-to-end check: `rg -n "Step 0:|Step 1:|Step 2:|Step 3:|Existing config detected|Legacy pre-v0.6 config detected|resolved tome_home:" crates/tome/src/wizard.rs crates/tome/src/lib.rs` shows each of the 7 UX surfaces exists in source
</verification>

<success_criteria>
- WUX-02 success criterion from ROADMAP.md line 46: "User running `tome init` on a brownfield machine ... sees a summary of the detected config (directory count, library_dir, last-modified date) and can choose use existing (default), edit existing, reinitialize (with backup), or cancel — no path silently overwrites a valid config" — demonstrably TRUE via the `brownfield_decision` 4-option Select + the `init_brownfield_no_input_keeps_existing` integration test (locks in the --no-input default) + `backup_brownfield_config` (locks in the `tome.toml.backup-<ts>` pre-overwrite copy)
- The 5th success criterion in ROADMAP.md (WUX-05 XDG persistence) remains satisfied via plan 03
- Phase 7 as a whole: all 5 WUX requirements covered; no requirement unmapped
- Pitfall 2 from RESEARCH.md resolved: custom directories survive edit via the union in `configure_directories` (locked by `configure_directories_preserves_custom_prefill` unit test)
</success_criteria>

<output>
After completion, create `.planning/phases/07-wizard-ux-greenfield-brownfield-legacy/07-04-SUMMARY.md` with:
- The final `wizard::run` signature (5 args)
- Full list of BrownfieldAction variants + lib.rs dispatch pattern for future reference
- Any deviations from RESEARCH.md (e.g. the `Cancel` behavior for invalid configs under --no-input — RESEARCH didn't lock this, we picked Cancel over UseExisting to avoid silently proceeding with broken config)
- Phase 7 completion checklist: WUX-01 ✓ (plan 03), WUX-02 ✓ (this plan), WUX-03 ✓ (plan 02), WUX-04 ✓ (plan 01), WUX-05 ✓ (plan 03)
- Suggested manual smoke test script for interactive coverage: run `tome init` against a seeded brownfield tmpdir and walk through each of the 4 options
</output>
