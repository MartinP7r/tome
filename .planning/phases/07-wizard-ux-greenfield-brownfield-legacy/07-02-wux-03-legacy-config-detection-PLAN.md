---
phase: 07-wizard-ux-greenfield-brownfield-legacy
plan: 02
type: execute
wave: 2
depends_on:
  - 07-01-wux-04-resolved-tome-home-info
files_modified:
  - crates/tome/src/wizard.rs
  - crates/tome/src/lib.rs
  - crates/tome/tests/cli.rs
autonomous: true
requirements:
  - WUX-03
must_haves:
  truths:
    - "A user with `~/.config/tome/config.toml` containing `[[sources]]` or `[targets.*]` sees a warning message before wizard prompts start"
    - "The user is offered three actions: leave as-is, move aside (rename to legacy-backup), or delete permanently"
    - "Under --no-input, the legacy file is left alone and a `note:` line is emitted to stderr"
    - "A v0.6+ XDG config with ONLY `tome_home = \"...\"` (and no sources/targets) is NOT flagged as legacy"
    - "A file that fails to parse as TOML is NOT flagged as legacy (graceful no-op, user can clean up manually)"
  artifacts:
    - path: "crates/tome/src/wizard.rs"
      provides: "MachineState enum + detect_machine_state + has_legacy_sections + handle_legacy_cleanup"
      contains: "enum MachineState"
    - path: "crates/tome/src/lib.rs"
      provides: "Command::Init dispatches on legacy state before calling wizard::run"
      contains: "detect_machine_state"
    - path: "crates/tome/tests/cli.rs"
      provides: "integration tests covering legacy detection and --no-input skip behavior"
      contains: "init_legacy"
  key_links:
    - from: "crates/tome/src/lib.rs Command::Init"
      to: "wizard::detect_machine_state"
      via: "function call after WUX-04 print"
      pattern: "detect_machine_state"
    - from: "crates/tome/src/wizard.rs handle_legacy_cleanup"
      to: "~/.config/tome/config.toml"
      via: "std::fs::rename or std::fs::remove_file"
      pattern: "legacy-backup-|remove_file"
---

<objective>
Detect legacy pre-v0.6 `~/.config/tome/config.toml` files (those containing `[[sources]]` or `[targets.*]` sections) and offer the user a clean-up action: leave / move-aside / delete. This is WUX-03 — closes the "silent ignore" footgun where tome v0.6+ reads only the `tome_home` key from that file and ignores everything else.

Purpose: A user upgrading from pre-v0.6 has cruft at `~/.config/tome/config.toml` that looks live but is silently ignored by current tome. Today they get no signal. This plan surfaces the file with a warning + cleanup prompt. Introduces the `MachineState` enum that plan 04 (brownfield) will also consume.

Output: `MachineState` enum, `detect_machine_state` + `has_legacy_sections` functions, `handle_legacy_cleanup` dispatcher in wizard.rs, lib.rs integration, and tests covering both the positive detection matrix and the false-positive protection.
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
@crates/tome/src/wizard.rs
@crates/tome/src/lib.rs
@crates/tome/src/config.rs
@crates/tome/tests/cli.rs

<interfaces>
<!-- APIs this plan introduces and consumes. -->

NEW (introduced by this plan, consumed by plan 04):
```rust
/// Classification of the machine state at `tome init` time.
///
/// Probes `resolve_config_dir(tome_home).join("tome.toml")` for brownfield,
/// and `~/.config/tome/config.toml` for legacy (pre-v0.6 sections).
#[derive(Debug)]
pub(crate) enum MachineState {
    Greenfield,
    Brownfield {
        existing_config_path: PathBuf,
        existing_config: anyhow::Result<Config>,
    },
    Legacy { legacy_path: PathBuf },
    BrownfieldWithLegacy {
        existing_config_path: PathBuf,
        existing_config: anyhow::Result<Config>,
        legacy_path: PathBuf,
    },
}

pub(crate) fn detect_machine_state(
    home: &Path,
    tome_home: &Path,
) -> anyhow::Result<MachineState>;

fn has_legacy_sections(path: &Path) -> anyhow::Result<Option<PathBuf>>;

/// Interactive handler for the Legacy and BrownfieldWithLegacy cases.
/// Returns Ok(()) after the user picks an action (or the --no-input default).
pub(crate) fn handle_legacy_cleanup(
    legacy_path: &Path,
    no_input: bool,
) -> anyhow::Result<()>;
```

CONSUMED (existing):
```rust
// crates/tome/src/config.rs
pub fn resolve_config_dir(tome_home: &Path) -> PathBuf;  // line 664
pub fn expand_tilde(path: &Path) -> Result<PathBuf>;

// crates/tome/src/paths.rs
pub(crate) fn collapse_home(path: &Path) -> String;  // line 142
```
</interfaces>
</context>

<tasks>

<task type="auto" tdd="true">
  <name>Task 1: Add MachineState + has_legacy_sections + detect_machine_state in wizard.rs with exhaustive unit tests</name>
  <files>crates/tome/src/wizard.rs</files>
  <read_first>
    - crates/tome/src/wizard.rs (focus: the existing module structure, top imports around lines 1–50)
    - crates/tome/src/config.rs (focus: lines 664–680 — resolve_config_dir, default_config_path; line 533 — Config::save_checked; line 277 — Config::load)
    - .planning/phases/07-wizard-ux-greenfield-brownfield-legacy/07-RESEARCH.md (focus: lines 120–212 — "State Classification Design" + edge cases table)
    - .planning/phases/07-wizard-ux-greenfield-brownfield-legacy/07-RESEARCH.md (focus: lines 689–707 — "Example 1: Detecting legacy sections")
    - .planning/codebase/CONVENTIONS.md (focus: anyhow::Context, pub(crate), newtype, test co-location)
  </read_first>
  <behavior>
    - Test 1: `has_legacy_sections` returns `None` when file does not exist
    - Test 2: `has_legacy_sections` returns `Some(path)` for a file containing `[[sources]]`
    - Test 3: `has_legacy_sections` returns `Some(path)` for a file containing `[targets.foo]`
    - Test 4: `has_legacy_sections` returns `Some(path)` for a file containing BOTH `[[sources]]` and `[targets.*]`
    - Test 5: `has_legacy_sections` returns `None` for a file containing ONLY `tome_home = "~/foo"` (the critical v0.6+ false-positive protection)
    - Test 6: `has_legacy_sections` returns `None` for a malformed TOML file (does not crash)
    - Test 7: `has_legacy_sections` returns `None` for a file with `# comment mentioning [[sources]]` and no actual sources table (ensures we parse, not substring-match)
    - Test 8: `detect_machine_state` returns `Greenfield` when neither file exists
    - Test 9: `detect_machine_state` returns `Brownfield` when tome.toml exists at `<tome_home>/tome.toml`
    - Test 10: `detect_machine_state` returns `Brownfield` when tome.toml exists at `<tome_home>/.tome/tome.toml` (resolve_config_dir picks the subdir)
    - Test 11: `detect_machine_state` returns `Legacy` when only the XDG legacy file exists
    - Test 12: `detect_machine_state` returns `BrownfieldWithLegacy` when both exist
    - Test 13: `detect_machine_state` returns `Greenfield` when the XDG file contains only `tome_home = "..."` (no legacy sections)
  </behavior>
  <action>
Add to `crates/tome/src/wizard.rs`, at the bottom of the file but before the `#[cfg(test)] mod tests { ... }` block (or inside a new submodule — judge based on file size; wizard.rs is ~1003 lines so keeping it flat is preferred per RESEARCH.md § "Architecture Patterns"):

1. **Imports** (add to top of wizard.rs if not already present — grep first with `rg -n "use std::path::PathBuf|use anyhow" crates/tome/src/wizard.rs`):
```rust
// already present: use anyhow::{Context, Result};
// already present: use std::path::{Path, PathBuf};
// new:
// (no new imports required — all types already in scope)
```

2. **MachineState enum:**
```rust
/// The machine state the wizard is running against.
///
/// Probes the filesystem for:
/// - `tome.toml` at the resolved tome_home config dir (`resolve_config_dir(tome_home)`)
/// - `~/.config/tome/config.toml` for pre-v0.6 `[[sources]]` / `[targets.*]` sections
///
/// Returned by [`detect_machine_state`]. Consumed by the `tome init` dispatcher
/// in `lib.rs` (Plan 04 handles the Brownfield variants).
#[derive(Debug)]
pub(crate) enum MachineState {
    /// No tome.toml at tome_home; no legacy XDG config with [[sources]]/[targets.*].
    Greenfield,
    /// tome.toml exists at tome_home; no legacy XDG file.
    Brownfield {
        existing_config_path: PathBuf,
        existing_config: Result<Config>,
    },
    /// Legacy pre-v0.6 XDG config detected; no brownfield tome.toml at tome_home.
    Legacy { legacy_path: PathBuf },
    /// Both brownfield AND legacy present. Handled in order: legacy first, then brownfield.
    BrownfieldWithLegacy {
        existing_config_path: PathBuf,
        existing_config: Result<Config>,
        legacy_path: PathBuf,
    },
}
```

*Note:* The `Result<Config>` field means `MachineState` cannot derive `Clone` or `PartialEq` (because `anyhow::Error` doesn't). That's fine — callers only match on variants, not compare states.

3. **detect_machine_state function:**
```rust
/// Classify the machine state by probing two filesystem locations.
///
/// `home` is passed explicitly (not sourced from `dirs::home_dir()`) so tests
/// can isolate via `TempDir`. Production callers pass `dirs::home_dir()?`.
pub(crate) fn detect_machine_state(
    home: &Path,
    tome_home: &Path,
) -> Result<MachineState> {
    let config_path = crate::config::resolve_config_dir(tome_home).join("tome.toml");
    let legacy_path = home.join(".config/tome/config.toml");

    let brownfield = config_path.is_file();
    let legacy = has_legacy_sections(&legacy_path)?;

    Ok(match (brownfield, legacy) {
        (false, None) => MachineState::Greenfield,
        (true, None) => MachineState::Brownfield {
            existing_config: Config::load(&config_path),
            existing_config_path: config_path,
        },
        (false, Some(legacy_path)) => MachineState::Legacy { legacy_path },
        (true, Some(legacy_path)) => MachineState::BrownfieldWithLegacy {
            existing_config: Config::load(&config_path),
            existing_config_path: config_path,
            legacy_path,
        },
    })
}
```

4. **has_legacy_sections function** — CRITICAL: TOML parse, not substring match (per RESEARCH.md § "Pitfall 3"):
```rust
/// Returns `Some(path)` if the XDG file at `path` exists, parses as TOML,
/// and contains either a top-level `sources` array-of-tables or a `targets`
/// table — the pre-v0.6 schema that v0.6+ silently ignores.
///
/// Returns `None` for:
/// - missing file
/// - malformed TOML (graceful degradation — user can clean up manually)
/// - v0.6+ shape (e.g. only `tome_home = "..."` and/or `[directories.*]`)
///
/// This function MUST parse (not substring-match) so that comments like
/// `# TODO: re-add [[sources]]` do not false-positive.
fn has_legacy_sections(path: &Path) -> Result<Option<PathBuf>> {
    if !path.is_file() {
        return Ok(None);
    }
    let content = std::fs::read_to_string(path)
        .with_context(|| format!("failed to read {}", path.display()))?;
    // Malformed TOML: treat as "can't tell, not legacy". Do not crash the wizard.
    let Ok(table) = content.parse::<toml::Table>() else {
        return Ok(None);
    };
    let has_sources = table.get("sources").is_some_and(|v| v.is_array());
    let has_targets = table.get("targets").is_some_and(|v| v.is_table());
    Ok((has_sources || has_targets).then(|| path.to_path_buf()))
}
```

5. **Unit tests** inside the existing `#[cfg(test)] mod tests { ... }` in wizard.rs. If none exists, create it at the bottom of the file:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    // --- has_legacy_sections -------------------------------------------------

    #[test]
    fn has_legacy_sections_returns_none_for_missing_file() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("nope.toml");
        assert_eq!(has_legacy_sections(&path).unwrap(), None);
    }

    #[test]
    fn has_legacy_sections_detects_sources_array() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("config.toml");
        std::fs::write(
            &path,
            "[[sources]]\nname = \"old\"\npath = \"/tmp\"\ntype = \"directory\"\n",
        )
        .unwrap();
        assert_eq!(has_legacy_sections(&path).unwrap(), Some(path));
    }

    #[test]
    fn has_legacy_sections_detects_targets_table() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("config.toml");
        std::fs::write(&path, "[targets.claude]\npath = \"~/.claude/skills\"\n").unwrap();
        assert_eq!(has_legacy_sections(&path).unwrap(), Some(path));
    }

    #[test]
    fn has_legacy_sections_detects_both_sources_and_targets() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("config.toml");
        std::fs::write(
            &path,
            "[[sources]]\nname = \"old\"\npath = \"/tmp\"\ntype = \"directory\"\n\n\
             [targets.claude]\npath = \"~/.claude/skills\"\n",
        )
        .unwrap();
        assert_eq!(has_legacy_sections(&path).unwrap(), Some(path));
    }

    #[test]
    fn has_legacy_sections_ignores_v0_6_only_tome_home() {
        // Critical false-positive protection: v0.6+ users who hand-wrote the XDG
        // file with only the tome_home key must NOT be flagged as legacy.
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("config.toml");
        std::fs::write(&path, "tome_home = \"~/dotfiles/tome\"\n").unwrap();
        assert_eq!(has_legacy_sections(&path).unwrap(), None);
    }

    #[test]
    fn has_legacy_sections_ignores_malformed_toml() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("config.toml");
        std::fs::write(&path, "this is [[[ not valid toml").unwrap();
        // Graceful no-op — return Ok(None), do not crash.
        assert_eq!(has_legacy_sections(&path).unwrap(), None);
    }

    #[test]
    fn has_legacy_sections_ignores_comment_with_sources_substring() {
        // Comment mentioning [[sources]] must not trigger a false positive —
        // we parse TOML, not grep. Comments are stripped post-parse.
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("config.toml");
        std::fs::write(
            &path,
            "# TODO: migrate [[sources]] to [directories.*]\n\
             tome_home = \"~/.tome\"\n",
        )
        .unwrap();
        assert_eq!(has_legacy_sections(&path).unwrap(), None);
    }

    // --- detect_machine_state ------------------------------------------------

    #[test]
    fn detect_machine_state_greenfield() {
        let tmp = TempDir::new().unwrap();
        let home = tmp.path();
        let tome_home = home.join(".tome");
        let state = detect_machine_state(home, &tome_home).unwrap();
        assert!(matches!(state, MachineState::Greenfield));
    }

    #[test]
    fn detect_machine_state_brownfield_at_tome_home_root() {
        let tmp = TempDir::new().unwrap();
        let home = tmp.path();
        let tome_home = home.join(".tome");
        std::fs::create_dir_all(&tome_home).unwrap();
        // Minimal valid v0.6 config
        std::fs::write(
            tome_home.join("tome.toml"),
            "library_dir = \"~/.tome/skills\"\n[directories]\n",
        )
        .unwrap();
        let state = detect_machine_state(home, &tome_home).unwrap();
        assert!(matches!(state, MachineState::Brownfield { .. }));
    }

    #[test]
    fn detect_machine_state_brownfield_at_dotted_subdir() {
        // `resolve_config_dir` picks `<tome_home>/.tome/` when that subdir has
        // a tome.toml (custom-repo layout).
        let tmp = TempDir::new().unwrap();
        let home = tmp.path();
        let tome_home = home.join("dotfiles/tome");
        let subdir = tome_home.join(".tome");
        std::fs::create_dir_all(&subdir).unwrap();
        std::fs::write(
            subdir.join("tome.toml"),
            "library_dir = \"~/.tome/skills\"\n[directories]\n",
        )
        .unwrap();
        let state = detect_machine_state(home, &tome_home).unwrap();
        assert!(matches!(state, MachineState::Brownfield { .. }));
    }

    #[test]
    fn detect_machine_state_legacy_only() {
        let tmp = TempDir::new().unwrap();
        let home = tmp.path();
        let tome_home = home.join(".tome");
        let xdg = home.join(".config/tome/config.toml");
        std::fs::create_dir_all(xdg.parent().unwrap()).unwrap();
        std::fs::write(&xdg, "[[sources]]\nname = \"x\"\npath = \"/tmp\"\ntype = \"directory\"\n").unwrap();
        let state = detect_machine_state(home, &tome_home).unwrap();
        assert!(matches!(state, MachineState::Legacy { .. }));
    }

    #[test]
    fn detect_machine_state_brownfield_with_legacy() {
        let tmp = TempDir::new().unwrap();
        let home = tmp.path();
        let tome_home = home.join(".tome");
        std::fs::create_dir_all(&tome_home).unwrap();
        std::fs::write(
            tome_home.join("tome.toml"),
            "library_dir = \"~/.tome/skills\"\n[directories]\n",
        )
        .unwrap();
        let xdg = home.join(".config/tome/config.toml");
        std::fs::create_dir_all(xdg.parent().unwrap()).unwrap();
        std::fs::write(&xdg, "[[sources]]\nname = \"x\"\npath = \"/tmp\"\ntype = \"directory\"\n").unwrap();

        let state = detect_machine_state(home, &tome_home).unwrap();
        assert!(matches!(state, MachineState::BrownfieldWithLegacy { .. }));
    }

    #[test]
    fn detect_machine_state_v0_6_only_xdg_is_greenfield() {
        // XDG file exists with only `tome_home = "..."` — NOT legacy.
        let tmp = TempDir::new().unwrap();
        let home = tmp.path();
        let tome_home = home.join(".tome");
        let xdg = home.join(".config/tome/config.toml");
        std::fs::create_dir_all(xdg.parent().unwrap()).unwrap();
        std::fs::write(&xdg, "tome_home = \"~/.tome\"\n").unwrap();
        let state = detect_machine_state(home, &tome_home).unwrap();
        assert!(matches!(state, MachineState::Greenfield));
    }
}
```

*Note on `MachineState` import in lib.rs:* plan 04 will `use crate::wizard::MachineState;`. The `pub(crate)` visibility is sufficient for that.
  </action>
  <verify>
    <automated>cargo test --package tome -- wizard::tests::has_legacy_sections wizard::tests::detect_machine_state 2>&1 | tail -30</automated>
  </verify>
  <acceptance_criteria>
    - `rg -n "pub\\(crate\\) enum MachineState" crates/tome/src/wizard.rs` returns a match
    - `rg -n "pub\\(crate\\) fn detect_machine_state" crates/tome/src/wizard.rs` returns a match
    - `rg -n "fn has_legacy_sections" crates/tome/src/wizard.rs` returns a match
    - `rg -n "fn has_legacy_sections_ignores_v0_6_only_tome_home" crates/tome/src/wizard.rs` returns a match (false-positive protection test)
    - `rg -n "fn has_legacy_sections_ignores_comment_with_sources_substring" crates/tome/src/wizard.rs` returns a match (parse-not-grep test)
    - `rg -n "toml::Table" crates/tome/src/wizard.rs` returns at least one match inside has_legacy_sections (confirms we parse TOML, not substring-match)
    - `cargo test --package tome -- wizard::tests::has_legacy_sections` exits 0 and reports 7 tests passing
    - `cargo test --package tome -- wizard::tests::detect_machine_state` exits 0 and reports 6 tests passing
    - `cargo clippy --all-targets -- -D warnings` exits 0
  </acceptance_criteria>
  <done>
    `MachineState` enum with all 4 variants exists at pub(crate) visibility. `detect_machine_state` probes both the brownfield and legacy locations and returns the correct variant. `has_legacy_sections` uses `toml::Table` parsing (not substring match) and correctly rejects v0.6+-only XDG files, malformed TOML, and comments mentioning [[sources]]. 13 unit tests lock in the behavior.
  </done>
</task>

<task type="auto" tdd="true">
  <name>Task 2: Implement handle_legacy_cleanup + wire into Command::Init in lib.rs with integration tests</name>
  <files>crates/tome/src/wizard.rs, crates/tome/src/lib.rs, crates/tome/tests/cli.rs</files>
  <read_first>
    - crates/tome/src/wizard.rs (focus: the newly added MachineState + detect_machine_state from Task 1; also existing `step_divider` and `console::style` usage around lines 139–150)
    - crates/tome/src/lib.rs (focus: lines 162–197 — Command::Init branch, and the tome_home_source variable added in plan 01)
    - crates/tome/tests/cli.rs (focus: lines 3758–3890 — init test isolation patterns, especially `.env("HOME", tmp.path())`)
    - .planning/phases/07-wizard-ux-greenfield-brownfield-legacy/07-RESEARCH.md (focus: lines 236–279 — WUX-03 implementation sketch; lines 496–503 — no_input behaviors)
  </read_first>
  <behavior>
    - Test 1 (integration): `init_legacy_detected_no_input_leaves_file` — seed `$HOME/.config/tome/config.toml` with `[[sources]]`, run `init --dry-run --no-input`, assert file is byte-identical after AND stderr contains "legacy" + "skipped"
    - Test 2 (integration): `init_legacy_with_only_tome_home_not_flagged` — seed XDG with only `tome_home = "~/somewhere"`, run `init --dry-run --no-input`, assert stderr does NOT contain "legacy" warning
    - Test 3 (integration): `init_greenfield_no_legacy_warning` — empty HOME, run `init --dry-run --no-input`, assert stderr does NOT contain "legacy"
    - Test 4 (unit in wizard.rs): `handle_legacy_cleanup_no_input_leaves_file` — passes `no_input=true`, verifies file exists after call and function returns Ok
  </behavior>
  <action>
**Part A — Add `handle_legacy_cleanup` to `crates/tome/src/wizard.rs`:**

Place it right after `detect_machine_state` (and the related functions from Task 1). Use the existing `console::style` and `dialoguer::Select` imports (grep to confirm: `rg -n "use console|use dialoguer" crates/tome/src/wizard.rs`).

```rust
/// Interactive handler for the Legacy and BrownfieldWithLegacy states.
///
/// Prints a warning, then:
/// - If `no_input` is true: emit a `note:` line to stderr and return Ok(()) — leaves file alone.
/// - Otherwise: prompt the user with 3 choices (leave / move aside / delete).
///
/// Default action: Leave (action 0) under --no-input; interactive default is
/// Move Aside (action 1) for discoverability — a user pressing Enter without
/// reading gets the non-destructive backup rather than a no-op.
pub(crate) fn handle_legacy_cleanup(legacy_path: &Path, no_input: bool) -> Result<()> {
    println!();
    println!(
        "{} Legacy pre-v0.6 config detected: {}",
        style("warning:").yellow(),
        style(legacy_path.display()).cyan()
    );
    println!("  This file contains [[sources]] or [targets.*] sections, which tome v0.6+");
    println!("  does not read. It is silently ignored -- likely not what you want.");

    if no_input {
        eprintln!(
            "{} skipped legacy cleanup (--no-input). Run `tome init` interactively to handle.",
            style("note:").cyan()
        );
        return Ok(());
    }

    let items = [
        "Leave as-is (warn again next time)",
        "Move aside (rename to config.toml.legacy-backup-<timestamp>)",
        "Delete permanently",
    ];
    let selection = dialoguer::Select::new()
        .with_prompt("What do you want to do with it?")
        .items(&items)
        .default(1) // move-aside — non-destructive, sorts friendly
        .interact()?;

    match selection {
        0 => {
            println!("  {} Left unchanged.", style("note:").cyan());
        }
        1 => {
            let ts = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .context("system clock is before UNIX epoch")?
                .as_secs();
            let backup_name = format!("config.toml.legacy-backup-{ts}");
            let backup_path = legacy_path
                .parent()
                .context("legacy path has no parent directory")?
                .join(&backup_name);
            std::fs::rename(legacy_path, &backup_path)
                .with_context(|| format!(
                    "failed to rename {} -> {}",
                    legacy_path.display(),
                    backup_path.display()
                ))?;
            println!(
                "  {} Moved to: {}",
                style("done").green(),
                style(backup_path.display()).cyan()
            );
        }
        2 => {
            std::fs::remove_file(legacy_path)
                .with_context(|| format!("failed to delete {}", legacy_path.display()))?;
            println!("  {} Deleted: {}", style("done").green(), style(legacy_path.display()).cyan());
        }
        _ => unreachable!("Select returned out-of-range index"),
    }
    Ok(())
}
```

If `dialoguer::Select` is already imported at the top of wizard.rs (likely yes — `rg -n "use dialoguer" crates/tome/src/wizard.rs`), use the bare `Select` name. Otherwise use the fully qualified path as shown.

**Part B — Wire into lib.rs `Command::Init` branch:**

Edit `crates/tome/src/lib.rs` around line 169–170, AFTER the WUX-04 resolved-tome_home info line (added in plan 01), BEFORE `wizard::run` is called. The exact insertion point depends on plan 01's changes — ensure this plan's executor reads lib.rs first.

Insert:
```rust
// WUX-03: Detect and handle legacy pre-v0.6 ~/.config/tome/config.toml
let home = dirs::home_dir().context("could not determine home directory")?;
let machine_state = wizard::detect_machine_state(&home, &tome_home)?;
if let wizard::MachineState::Legacy { legacy_path }
    | wizard::MachineState::BrownfieldWithLegacy { legacy_path, .. } = &machine_state
{
    wizard::handle_legacy_cleanup(legacy_path, cli.no_input)?;
}
// machine_state is NOT consumed further in this plan; plan 04 will extend this
// block to also dispatch on the Brownfield variants.
let _ = machine_state;
```

Notes:
- The `let _ = machine_state;` silences the "unused variable after partial move" warning in the interim. Plan 04 will replace that line with the brownfield dispatch. Adding a TODO comment is acceptable: `// TODO(plan 04): replace with full match on machine_state for brownfield dispatch.`
- `dirs::home_dir()` is already a dependency (confirmed from config.rs usage).

**Part C — Unit test for `handle_legacy_cleanup` under no_input:**

Add to the existing `#[cfg(test)] mod tests` block in wizard.rs:

```rust
#[test]
fn handle_legacy_cleanup_no_input_leaves_file() {
    let tmp = TempDir::new().unwrap();
    let path = tmp.path().join("config.toml");
    std::fs::write(&path, "[[sources]]\nname = \"x\"\npath = \"/tmp\"\ntype = \"directory\"\n").unwrap();

    handle_legacy_cleanup(&path, /* no_input = */ true).unwrap();

    // File must still exist, byte-identical.
    assert!(path.is_file(), "file should still exist after no_input handler");
    let content = std::fs::read_to_string(&path).unwrap();
    assert!(
        content.contains("[[sources]]"),
        "content should be unchanged, got: {content}"
    );
}
```

Interactive branches (move-aside, delete) are NOT tested via dialoguer — use only the no_input branch. Per RESEARCH.md § "Pitfall 5: Interactive tests hanging in CI" + line 687.

**Part D — Integration tests in cli.rs:**

Add after plan 01's init_prints_resolved_tome_home_* tests:

```rust
#[test]
fn init_legacy_detected_no_input_leaves_file() {
    let tmp = TempDir::new().unwrap();
    let tome_home = tmp.path().join(".tome");
    let xdg_dir = tmp.path().join(".config/tome");
    let xdg_file = xdg_dir.join("config.toml");
    std::fs::create_dir_all(&xdg_dir).unwrap();
    let legacy_content = "[[sources]]\nname = \"old\"\npath = \"/tmp\"\ntype = \"directory\"\n";
    std::fs::write(&xdg_file, legacy_content).unwrap();

    let output = tome()
        .args(["init", "--dry-run", "--no-input"])
        .env("HOME", tmp.path())
        .env("TOME_HOME", &tome_home)
        .env("NO_COLOR", "1")
        .output()
        .unwrap();
    assert!(output.status.success(),
        "tome init failed: {}", String::from_utf8_lossy(&output.stderr));

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    // Warning appears on stdout (the `println!` in handle_legacy_cleanup)
    assert!(
        stdout.contains("Legacy pre-v0.6 config detected"),
        "stdout missing legacy warning:\n{stdout}"
    );
    // Skip note appears on stderr
    assert!(
        stderr.contains("skipped legacy cleanup"),
        "stderr missing skipped-cleanup note:\n{stderr}"
    );

    // File must be byte-identical after the run.
    let after = std::fs::read_to_string(&xdg_file).unwrap();
    assert_eq!(after, legacy_content, "legacy file should be unchanged");
}

#[test]
fn init_legacy_with_only_tome_home_not_flagged() {
    let tmp = TempDir::new().unwrap();
    let tome_home = tmp.path().join(".tome");
    let xdg_dir = tmp.path().join(".config/tome");
    std::fs::create_dir_all(&xdg_dir).unwrap();
    // v0.6+ shape — should NOT trigger legacy warning
    std::fs::write(xdg_dir.join("config.toml"), "tome_home = \"~/somewhere\"\n").unwrap();

    let output = tome()
        .args(["init", "--dry-run", "--no-input"])
        .env("HOME", tmp.path())
        .env("TOME_HOME", &tome_home)
        .env("NO_COLOR", "1")
        .output()
        .unwrap();
    assert!(output.status.success());

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !stdout.contains("Legacy pre-v0.6 config detected"),
        "v0.6+-only XDG file should NOT trigger legacy warning. stdout:\n{stdout}"
    );
    assert!(
        !stderr.contains("skipped legacy cleanup"),
        "v0.6+-only XDG file should NOT trigger skip-note. stderr:\n{stderr}"
    );
}

#[test]
fn init_greenfield_no_legacy_warning() {
    let tmp = TempDir::new().unwrap();
    let tome_home = tmp.path().join(".tome");

    let output = tome()
        .args(["init", "--dry-run", "--no-input"])
        .env("HOME", tmp.path())
        .env("TOME_HOME", &tome_home)
        .env("NO_COLOR", "1")
        .output()
        .unwrap();
    assert!(output.status.success());

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        !stdout.contains("Legacy pre-v0.6 config detected"),
        "greenfield run should NOT show legacy warning. stdout:\n{stdout}"
    );
}
```
  </action>
  <verify>
    <automated>cargo test --package tome -- wizard::tests::handle_legacy_cleanup init_legacy init_greenfield_no_legacy 2>&1 | tail -20</automated>
  </verify>
  <acceptance_criteria>
    - `rg -n "pub\\(crate\\) fn handle_legacy_cleanup" crates/tome/src/wizard.rs` returns a match
    - `rg -n "config\\.toml\\.legacy-backup-" crates/tome/src/wizard.rs` returns a match (move-aside filename pattern)
    - `rg -n "skipped legacy cleanup" crates/tome/src/wizard.rs` returns a match (stderr note text)
    - `rg -n "wizard::detect_machine_state|wizard::handle_legacy_cleanup" crates/tome/src/lib.rs` returns at least 2 matches
    - `rg -n "fn handle_legacy_cleanup_no_input_leaves_file" crates/tome/src/wizard.rs` returns a match
    - `rg -n "fn init_legacy_detected_no_input_leaves_file" crates/tome/tests/cli.rs` returns a match
    - `rg -n "fn init_legacy_with_only_tome_home_not_flagged" crates/tome/tests/cli.rs` returns a match
    - `rg -n "fn init_greenfield_no_legacy_warning" crates/tome/tests/cli.rs` returns a match
    - `cargo test --package tome -- wizard::tests::handle_legacy_cleanup_no_input_leaves_file` exits 0
    - `cargo test --package tome --test cli -- init_legacy init_greenfield_no_legacy` exits 0 and reports 3 tests passing
    - `cargo test --package tome --test cli -- init_` (all init tests) exits 0 with no regressions
    - `cargo clippy --all-targets -- -D warnings` exits 0
    - `cargo fmt -- --check` exits 0
  </acceptance_criteria>
  <done>
    A user with legacy pre-v0.6 `[[sources]]` / `[targets.*]` in their XDG config sees a warning before Step 1, and under `--no-input` the file is left alone with a `note:` line on stderr. A v0.6+ user with only `tome_home = "..."` in the XDG file is NOT flagged. Interactive branches (move-aside, delete) are implemented but not covered by automated tests (intentional — dialoguer interactive tests are outlawed per RESEARCH.md § Pitfall 5); the no_input branch is covered by both a unit test and integration tests.
  </done>
</task>

</tasks>

<verification>
- `cargo test --package tome` exits 0 (all new + pre-existing tests pass)
- `cargo clippy --all-targets -- -D warnings` exits 0
- `cargo fmt -- --check` exits 0
- `make ci` exits 0
- A manual check (not automated, but verifiable by reading wizard.rs): `has_legacy_sections` uses `toml::Table::parse` — NOT `content.contains("[[sources]]")`
</verification>

<success_criteria>
- WUX-03 success criterion from ROADMAP.md line 47: "User with a legacy pre-v0.6 `~/.config/tome/config.toml` (contains `[[sources]]` or `[targets.*]`) sees a warning that the file is ignored by current tome and is offered a delete-or-move-aside action — no silent ignore, no auto-delete" — demonstrably TRUE via `init_legacy_detected_no_input_leaves_file` integration test + interactive `handle_legacy_cleanup` implementation
- Infrastructure delivered: `MachineState` enum + `detect_machine_state` available at `pub(crate)` visibility for plan 04 to consume
- False-positive protection: v0.6+ XDG files and files with commented-out `[[sources]]` strings do NOT trigger the warning
</success_criteria>

<risks>
- **Phase 7 plans must land together.** Plan 02 inserts `// TODO(plan 04): replace with full match on machine_state for brownfield dispatch.` into `lib.rs::Command::Init` as a provisional stub. Plan 04 replaces it with the full `match machine_state` dispatch. Do NOT merge Plan 02 alone — all four Phase 7 plans must land in the same PR (or in strict sequence on the same branch) before merging to main.
</risks>

<output>
After completion, create `.planning/phases/07-wizard-ux-greenfield-brownfield-legacy/07-02-SUMMARY.md` with:
- MachineState variants + exact struct fields (for plan 04 to match on)
- Import path: `use crate::wizard::{MachineState, detect_machine_state};`
- Line numbers of the new public API in wizard.rs
- Note that plan 04 will replace the `let _ = machine_state;` placeholder with full brownfield dispatch
</output>
