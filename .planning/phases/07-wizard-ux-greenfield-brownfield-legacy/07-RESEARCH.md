# Phase 7: Wizard UX (Greenfield / Brownfield / Legacy) — Research

**Researched:** 2026-04-23
**Domain:** Interactive CLI wizard UX + path resolution + state detection
**Confidence:** HIGH (entire domain is in-tree, no external library questions)

## Summary

Phase 7 extends the existing `tome init` wizard so that it no longer assumes a fresh, empty machine. The wizard must (a) surface the resolved `tome_home` up front, (b) prompt for `tome_home` on greenfield installs with optional XDG persistence, (c) detect an existing `tome.toml` at the resolved tome_home and offer a use/edit/reinit/cancel branch, and (d) warn about legacy pre-v0.6 `~/.config/tome/config.toml` files and offer delete/move-aside.

All load-bearing primitives already exist in-tree. `Config::save_checked` (`config.rs:533`) gives us atomic validated writes, `--no-input` plumbing (`wizard.rs:137` + `cli.rs:47`) gives us headless mode for CI and integration tests, `resolve_tome_home` (`lib.rs:105`) already encapsulates the precedence chain, and `read_config_tome_home` (`config.rs:637`) already reads the XDG location for only the `tome_home` key — it quietly ignores everything else in that file, which means writing just `tome_home = "..."` to the XDG config is already a supported, well-tested shape. No new external dependencies are required.

**Primary recommendation:** Introduce a `MachineState` enum + `detect_machine_state(home, tome_home, xdg_config)` function as the first thing `tome init` runs, then dispatch to one of four existing-wizard entry points. Keep `assemble_config` untouched. Add a `Step 0` in the wizard for the greenfield `tome_home` prompt, but route all four flows through a single `run()` entry point so `--no-input` keeps working.

## User Constraints (from upstream input)

### Locked Decisions

From REQUIREMENTS.md (lines 25–33) and v0.8 Decisions D-1..D-4:

- **D-1**: machine.toml path overrides are NOT in v0.8 — deferred to v0.9.
- **D-2**: Persist `tome_home` via `~/.config/tome/config.toml`, not shell-rc injection.
- **D-3**: Brownfield flow default = **use existing** (safest for dotfiles-sync workflows).
- **D-4**: Legacy file detection = warn + offer delete-or-move-aside. No silent auto-delete.

### Claude's Discretion

- Exact wording of prompts, summary formatting, and error messages.
- `MachineState` enum shape and detection function signature.
- Backup filename convention for brownfield "reinitialize" (`tome.toml.backup-<timestamp>` recommended; backup.rs git-snapshot mechanism is also available).
- Move-aside filename convention for legacy file (`config.toml.legacy-backup-<timestamp>` recommended).
- Whether to add a helper function or fold detection into `wizard::run`.

### Deferred Ideas (OUT OF SCOPE)

- `TOME_HOME` env-var injection into shell-rc files (per REQUIREMENTS.md line 64).
- Brownfield "merge" mode that combines existing + new entries (per REQUIREMENTS.md line 66).
- Migration tooling for pre-v0.6 configs — we offer delete/move-aside, not migrate (per REQUIREMENTS.md line 67).
- Cross-OS path rewriting (deferred to v0.9 PORT-01..04).

## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| WUX-01 | Greenfield `tome_home` prompt (default `~/.tome/`, custom with validation) | § "WUX-01 — Greenfield tome_home prompt" |
| WUX-02 | Brownfield detect existing `tome.toml` → use/edit/reinit/cancel | § "WUX-02 — Brownfield decision", § "Current-state summary" |
| WUX-03 | Legacy `~/.config/tome/config.toml` with `[[sources]]`/`[targets.*]` → delete/move-aside | § "WUX-03 — Legacy config cleanup" |
| WUX-04 | Print `resolved tome_home: <path>` at start of every `tome init` | § "WUX-04 — Print resolved tome_home" |
| WUX-05 | On custom tome_home, offer to persist via `~/.config/tome/config.toml` | § "WUX-05 — Persist tome_home via XDG" |

## Current-State Summary

### What `tome init` does today (`lib.rs:162`–`197`)

```rust
if matches!(cli.command, Command::Init) {
    if let Err(e) = Config::load_or_default(effective_config.as_deref()) {
        eprintln!("warning: existing config is malformed ({}), the wizard will create a new one", e);
    }
    let tome_home = resolve_tome_home(cli.tome_home.as_deref(), cli.config.as_deref())?;
    let config = wizard::run(cli.dry_run, cli.no_input)?;
    config.validate()?;
    if !cli.dry_run {
        let mut expanded = config.clone();
        expanded.expand_tildes()?;
        let paths = TomePaths::new(tome_home, expanded.library_dir.clone())?;
        sync(&expanded, &paths, SyncOptions { dry_run: cli.dry_run, /* ... */ no_triage: true, /* ... */ })?;
    }
    return Ok(());
}
```

Observations:

1. `tome_home` is already resolved here (`lib.rs:169`) **but is never surfaced to the user** — it is only used to build `TomePaths` for the post-init sync. WUX-04 is a simple insertion at this site.
2. The wizard is invoked with `wizard::run(dry_run, no_input)` only — it has no knowledge of the resolved `tome_home` or any existing config.
3. A failing `Config::load_or_default` is already tolerated with a warning (the "malformed config" branch). A *successful* load is ignored — the wizard blows through and overwrites. That's the brownfield gap WUX-02 closes.
4. The wizard always calls `default_config_path()` internally (`wizard.rs:310`) to decide where to save. With `--tome-home` or `--config` on the CLI, this causes the wizard to **lie to the user**: the save path displayed may not match the one `sync()` then uses. Phase 7 should fix this by threading the resolved `tome_home` into `wizard::run`.

### What `wizard::run` does today (`wizard.rs:137`–`377`)

Step 1 (directories) → Step 2 (library) → Step 3 (exclusions) → Summary + role edit loop + custom directory loop → Save. No `tome_home` prompt, no brownfield detection, no legacy scan. The wizard owns the save path via `default_config_path()` (`wizard.rs:310`).

### What `resolve_tome_home` does today (`lib.rs:105`)

```
1. --tome-home flag (absolute path; tilde-expanded)
2. --config flag (tome_home = parent dir of config file; must be absolute)
3. config::default_tome_home():
   a. TOME_HOME env var (non-empty, tilde-expanded)
   b. ~/.config/tome/config.toml tome_home key
   c. ~/.tome/ (hardcoded default)
```

`default_tome_home` returns the resolved path but **does not tell the caller which branch it took** — the "source of truth" attribution for WUX-04 has to be reconstructed at the call site. See § "Risks & open questions" for a design note on this.

### What `read_config_tome_home` does today (`config.rs:637`)

Reads `~/.config/tome/config.toml` as a `toml::Table`, extracts the `tome_home` string key, **silently ignores everything else**. This means:

- Legacy files with `[[sources]]` or `[targets.*]` and no `tome_home` key → return `None`, fall through to default — **exactly the "silent ignore" footgun WUX-03 closes.**
- A user who manually added `tome_home = "~/dotfiles/tome"` to that file works today, and continues to work after Phase 7.
- Writing a new file with **only** `tome_home = "..."` is a fully supported shape — no format changes needed for WUX-05.

### Dependencies (Cargo.toml)

Everything Phase 7 needs is already in the dependency tree. No additions required.

| Need | Crate | Version | Status |
|------|-------|---------|--------|
| Interactive prompts (Confirm, Input, Select) | `dialoguer` | 0.12 | In tree |
| TOML parse (detect legacy, read XDG) | `toml` | 1 | In tree |
| Atomic temp+rename writes | n/a | std | Pattern already in machine.rs, lockfile.rs |
| Colored output | `console` | 0.16 | In tree |
| Tempdir tests | `tempfile` | 3 | In tree (dev) |
| Integration tests | `assert_cmd` + `predicates` | 2 / 3 | In tree (dev) |

## State Classification Design

### Proposed `MachineState` enum

```rust
/// The machine state the wizard is running against, determined by probing
/// the resolved `tome_home` and the XDG `~/.config/tome/config.toml`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum MachineState {
    /// No tome.toml at tome_home; no legacy XDG config with [[sources]]/[targets.*].
    Greenfield,
    /// tome.toml exists at tome_home; legacy XDG file (if present) is clean.
    Brownfield { existing_config_path: PathBuf, existing_config: Result<Config> },
    /// Legacy pre-v0.6 XDG config file detected (contains [[sources]] or [targets.*]).
    /// Independent of brownfield — can coexist.
    Legacy { legacy_path: PathBuf },
    /// Both brownfield AND legacy present — handled in order: legacy first, then brownfield.
    BrownfieldWithLegacy { existing_config_path: PathBuf, existing_config: Result<Config>, legacy_path: PathBuf },
}
```

**Alternative:** split into two orthogonal probes instead of one enum. A single enum keeps the dispatch obvious, but a `(Option<Brownfield>, Option<Legacy>)` tuple may be cleaner. Planner's call — the phase can decide.

### Proposed detection function signature

```rust
/// Classify the machine state.
///
/// Pure of `dirs::home_dir()` — callers pass `home` explicitly so tests can
/// isolate with a TempDir. `resolve_config_dir` is used to pick the right
/// config location (tome_home root vs .tome/ subdir).
pub(crate) fn detect_machine_state(
    home: &Path,
    tome_home: &Path,
) -> Result<MachineState> {
    let config_path = crate::config::resolve_config_dir(tome_home).join("tome.toml");
    let legacy_path = home.join(".config/tome/config.toml");

    let brownfield = config_path.is_file();
    let legacy = has_legacy_sections(&legacy_path)?;

    match (brownfield, legacy) {
        (false, None) => Ok(MachineState::Greenfield),
        (true, None) => Ok(MachineState::Brownfield {
            existing_config_path: config_path.clone(),
            existing_config: Config::load(&config_path),
        }),
        (false, Some(p)) => Ok(MachineState::Legacy { legacy_path: p }),
        (true, Some(p)) => Ok(MachineState::BrownfieldWithLegacy {
            existing_config_path: config_path,
            existing_config: Config::load(&config_path),
            legacy_path: p,
        }),
    }
}

/// Returns Some(path) if `~/.config/tome/config.toml` exists and contains
/// `[[sources]]` or a `[targets.*]` table. Returns None if the file is missing,
/// unparseable, or contains only v0.6+ keys (e.g. only `tome_home = "..."`).
fn has_legacy_sections(path: &Path) -> Result<Option<PathBuf>> {
    if !path.is_file() {
        return Ok(None);
    }
    let content = std::fs::read_to_string(path)
        .with_context(|| format!("failed to read {}", path.display()))?;
    // Parse to toml::Table rather than substring-matching, so comments and
    // key values containing "[[sources]]" as text don't false-positive.
    let Ok(table) = content.parse::<toml::Table>() else {
        // Malformed file — treat as "not legacy, can't tell" and move on.
        // The wizard will still work; user can delete the file manually.
        return Ok(None);
    };
    let has_sources = table.get("sources").is_some_and(|v| v.is_array());
    let has_targets = table.get("targets").is_some_and(|v| v.is_table());
    if has_sources || has_targets {
        Ok(Some(path.to_path_buf()))
    } else {
        Ok(None)
    }
}
```

### Edge cases

| Case | Behavior |
|------|----------|
| Symlink to tome.toml | `is_file()` follows symlinks → treated as brownfield (correct). |
| `tome_home` resolves to nonexistent path | `is_file()` returns false → greenfield. Wizard proceeds; `save_checked` will `create_dir_all` at write time. Correct behavior. |
| `~/.config/tome/config.toml` exists but is unreadable (permission denied) | `read_to_string` returns `Err`. We bail with context — user should see the problem. |
| `~/.config/tome/config.toml` with `tome_home = "..."` **and** legacy `[[sources]]` | Classified as Legacy. User is warned. After cleanup, the `tome_home` resolution re-runs on next invocation — if the user deleted the file they lose the pointer, but that's the documented tradeoff. WUX-05 lets them re-persist. |
| Tome.toml at tome_home root AND at `<tome_home>/.tome/tome.toml` | `resolve_config_dir` picks the `.tome/` subdir (see `config.rs:664`). We follow the same rule for detection. |
| `--config` flag points at a path **outside** tome_home | `resolve_config_path` uses that path; `resolve_tome_home` uses its parent. Consistent with the rest of the codebase. |
| Read-only filesystem | Save fails with a clear error. No change — existing `save_checked` contract. |
| Brownfield config exists but fails to parse | `Config::load` returns `Err`. The existing "malformed config" warning (`lib.rs:163`) already handles this — we preserve that behavior. Offer "reinitialize" only for this case (not "edit existing"). |

## Per-Requirement Implementation Sketch

### WUX-04 — Print resolved tome_home (simplest, do first)

**Where:** `lib.rs` inside the `Command::Init` branch, after `resolve_tome_home` returns but before `wizard::run` is called (around `lib.rs:170`).

**What:**
```rust
// After: let tome_home = resolve_tome_home(...)?;
let (tome_home, source) = resolve_tome_home_with_source(cli.tome_home.as_deref(), cli.config.as_deref())?;
println!();
println!("resolved tome_home: {} (from {})",
    style(tome_home.display()).cyan(),
    source); // e.g. "--tome-home flag", "TOME_HOME env", "XDG config", "default"
```

**Why with source:** The upstream question 6 asks whether to include the resolution source. Recommendation: **yes**. A user seeing "from XDG config" vs "from default" knows whether the XDG file is in play without running additional diagnostics. Cheap to compute, high diagnostic value.

**Implementation:** Return `(PathBuf, &'static str)` from a new `resolve_tome_home_with_source` (or add an out-param) instead of `PathBuf` from `resolve_tome_home`. Internal change, no API break.

**No-input behavior:** Print anyway. The line is informational only, not a prompt.

### WUX-03 — Legacy config cleanup (run before any other prompts)

**Where:** `lib.rs` `Command::Init` branch, immediately after the WUX-04 print, before the brownfield branch.

**Order rationale:** The legacy XDG file can contain `tome_home = "..."` that influences resolution. By scanning for legacy sections but *not* touching the file before the user decides, we guarantee:

- If the user deletes the legacy file, next invocation resolves tome_home through the remaining chain.
- If the user moves aside, same outcome (the `.legacy-backup-<ts>` filename doesn't match the `config.toml` glob, so `read_config_tome_home` ignores it).

**Detection:** As described above, via `has_legacy_sections` that returns `Some(path)` when the file parses as TOML and contains a top-level `sources` array-of-tables or a `targets` table.

**Prompt shape (pseudocode):**
```rust
if let MachineState::Legacy { legacy_path } | MachineState::BrownfieldWithLegacy { legacy_path, .. } = &state {
    println!("{} Legacy pre-v0.6 config detected: {}",
        style("warning:").yellow(),
        legacy_path.display());
    println!("  This file contains [[sources]] or [targets.*] sections, which tome v0.6+");
    println!("  does not read. It is silently ignored — likely not what you want.");

    let action = if no_input {
        // Leave it alone in headless mode — don't make destructive decisions.
        eprintln!("{} skipped legacy cleanup (--no-input). Run `tome init` interactively to handle.",
            style("note:").cyan());
        LegacyAction::Leave
    } else {
        Select::new()
            .with_prompt("What do you want to do with it?")
            .items(&[
                "Leave as-is (warn again next time)",
                "Move aside (rename to config.toml.legacy-backup-<timestamp>)",
                "Delete permanently",
            ])
            .default(1) // default to move-aside — non-destructive
            .interact()?
    };
}
```

**Move-aside filename:** `<parent>/config.toml.legacy-backup-<unix-timestamp>`. Unix timestamp over ISO-8601 because no chrono dep, and `std::time::SystemTime::now().duration_since(UNIX_EPOCH)` is one line. Sort-friendly in `ls`.

**Delete behavior:** `std::fs::remove_file(&legacy_path)?`. If there's any anxiety about a second user-valued field being lost, the "move aside" default covers it — deleting is explicit consent.

**Critical invariant:** False-positive rule. A v0.6+ user who hand-wrote `~/.config/tome/config.toml` with **only** `tome_home = "~/dotfiles/tome"` must NOT be flagged as legacy. The `has_legacy_sections` function handles this by checking for the `sources` or `targets` key specifically, not by substring-matching. Added protection: comments mentioning `[[sources]]` inside `#` lines won't trigger because we parse the TOML, not grep it.

### WUX-02 — Brownfield decision

**Where:** `lib.rs` `Command::Init` branch, after legacy cleanup and WUX-04, before invoking `wizard::run`.

**Summary display** (for existing `Config::load` result):

```rust
fn show_brownfield_summary(path: &Path, config: &Result<Config>) {
    println!();
    println!("{} {}",
        style("existing config:").bold(),
        style(path.display()).cyan());
    match config {
        Ok(c) => {
            println!("  directories: {}", c.directories().len());
            println!("  library_dir: {}", crate::paths::collapse_home(c.library_dir()));
            if let Ok(meta) = std::fs::metadata(path) {
                if let Ok(mtime) = meta.modified() {
                    // Format as relative-friendly (e.g. "modified 3 days ago") or ISO.
                    // std only; no chrono. Simple SystemTime formatter acceptable.
                    println!("  last modified: {}", format_mtime(mtime));
                }
            }
        }
        Err(e) => {
            println!("  {} {:#}", style("invalid:").red(), e);
            println!("  (edit/reinit will overwrite; 'use existing' unavailable)");
        }
    }
}
```

**Action prompt:**

```rust
enum BrownfieldAction { UseExisting, Edit, Reinit, Cancel }

let valid_items: &[&str] = if config.is_ok() {
    &["Use existing (exit wizard, run `tome sync`)",
      "Edit existing (pre-fill wizard with current values)",
      "Reinitialize (backup + overwrite)",
      "Cancel"]
} else {
    // No "use existing" option when config is invalid
    &["Reinitialize (backup + overwrite)", "Cancel"]
};

let action = if no_input {
    BrownfieldAction::UseExisting  // D-3 locked decision
} else {
    Select::new().with_prompt("...").items(valid_items).default(0).interact()?
};
```

**Default = `UseExisting`** per D-3 locked decision. Safest for dotfiles-sync case where the config came from git.

**"Use existing":**
```rust
println!("  Config unchanged. Run `tome sync` to apply.");
return Ok(());  // exit tome init cleanly, no post-init sync
```

**"Edit existing":** Pre-fill the wizard with current values. This is the largest refactor in the phase. Current wizard helpers (`configure_directories`, `configure_library`, `configure_exclusions`) don't accept a pre-fill argument. Options:

1. **Plumb an `Option<&Config>` into `wizard::run` and each helper.** Minimal signature change, each helper reads defaults from the pre-fill if present. Recommended.
2. Build a separate `wizard::run_edit(&Config, ...)` that reuses the helpers with pre-fill extraction. More code, clearer separation, probably overkill.

Signature after Option 1:

```rust
pub fn run(
    dry_run: bool,
    no_input: bool,
    tome_home: &Path,                 // new — for WUX-04 echo and save path
    prefill: Option<&Config>,         // new — for brownfield edit
) -> Result<Config>
```

For each helper:
- `configure_directories(no_input, prefill: Option<&BTreeMap<..>>)` — start the MultiSelect with `defaults = [entry_exists_in_prefill; found.len()]` and merge in any prefill entries not auto-discovered.
- `configure_library(no_input, prefill: Option<&Path>)` — if prefill is Some and not `~/.tome/skills`, show it as option 0 (now-default), the standard default as option 1, custom as option 2.
- `configure_exclusions(..., prefill: Option<&BTreeSet<SkillName>>)` — start MultiSelect with `defaults` set to match the prefill.

**"Reinitialize":** Backup + proceed as greenfield.

```rust
let backup_name = format!("tome.toml.backup-{}",
    SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs());
let backup_path = existing_config_path.parent().unwrap().join(&backup_name);
std::fs::copy(&existing_config_path, &backup_path)?;  // copy, not rename — rename would make Confirm-cancel awkward
println!("  backed up existing config to: {}", backup_path.display());
// then continue to wizard::run with prefill = None
```

**Note on `backup.rs`:** The existing `crate::backup` is a git-snapshot mechanism for the whole `tome_home`. Heavy for a single file. Recommend a simple `fs::copy` to `tome.toml.backup-<ts>` instead, documented in a one-liner comment. If the user has `tome backup` configured, the git repo already has history and they don't need the file-level backup to be in git.

**"Cancel":**
```rust
println!("Wizard cancelled. Existing config left unchanged.");
return Ok(());  // exit 0; stderr message optional
```

### WUX-01 — Greenfield tome_home prompt

**Where:** New `Step 0` inside `wizard::run`, before Step 1 directories. Only runs when `prefill.is_none()` and the CLI didn't already lock in a tome_home via `--tome-home` / `--config` / `TOME_HOME`.

**Critical question: when do we prompt?** The `tome_home` was already resolved in `lib.rs` before `wizard::run` was called. On a pure greenfield machine (no flags, no env, no XDG file), `resolve_tome_home` returns `~/.tome/` from the hardcoded default. That's the signal to prompt.

The simplest rule: **prompt iff the resolution source was "default"**. This is why WUX-04's `resolve_tome_home_with_source` is useful beyond just printing — it also gates WUX-01.

```rust
// In wizard::run, before Step 1
step_divider("Step 0: Tome home location");
let tome_home_source: TomeHomeSource = /* passed in from lib.rs */;
let prompt_for_tome_home = matches!(tome_home_source, TomeHomeSource::Default) && !no_input;

if prompt_for_tome_home {
    let default = dirs::home_dir()?.join(".tome");
    let options = vec![
        format!("{} (default)", crate::paths::collapse_home(&default)),
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
            .validate_with(|s: &String| -> Result<(), &str> {
                let path = PathBuf::from(s);
                // Accept ~ prefix; reject relative; reject file-at-path; accept nonexistent
                let expanded = expand_tilde(&path).map_err(|_| "could not expand ~")?;
                if !expanded.is_absolute() { return Err("must be absolute"); }
                if expanded.exists() && !expanded.is_dir() { return Err("exists but is not a directory"); }
                Ok(())
            })
            .interact_text()?;
        tome_home = expand_tilde(&PathBuf::from(custom))?;
    }
    // fall through to WUX-05 persistence prompt
}
```

**Why `validate_with` over a manual loop:** dialoguer's `Input::validate_with` re-prompts on invalid input and surfaces the error message inline. Same pattern used in the wider ecosystem (e.g. cargo-generate). No manual loop required.

**Permission pre-check:** Skip. `Config::save_checked` already fails with a clear error if the directory can't be created. Re-checking at prompt time is redundant.

**`--no-input` behavior:** Skip the prompt entirely, use the resolved default (`~/.tome/`). Already true above.

**Where to mutate:** After this step, the wizard's save path must reflect the chosen `tome_home`. Today the wizard calls `default_config_path()` (`wizard.rs:310`), which re-probes TOME_HOME + XDG. We need to replace that with `resolve_config_dir(&tome_home).join("tome.toml")` so the wizard honors the prompted choice. Clean signature change in `wizard::run`.

### WUX-05 — Persist tome_home via XDG

**Where:** Immediately after WUX-01, when the user chose the "custom" path (i.e. the resolved tome_home differs from `~/.tome/`).

**Prompt:**
```rust
let custom_chosen = tome_home != dirs::home_dir()?.join(".tome");
if custom_chosen && !no_input {
    let persist = Confirm::new()
        .with_prompt(format!(
            "Persist this choice to ~/.config/tome/config.toml?\n\
             Without this, subsequent `tome sync` / `tome status` need TOME_HOME=... or --tome-home=...",
        ))
        .default(true)
        .interact()?;
    if persist {
        write_xdg_tome_home(&tome_home)?;
    }
}
```

**Write implementation:** The XDG file may already exist with user-valued content (other fields we don't recognize, comments, etc.). Two options:

1. **Merge:** Parse existing TOML, set `tome_home`, write back. Preserves comments? No — `toml` crate doesn't preserve comments on round-trip. Preserves other fields, yes. This is the safer option.

2. **Overwrite:** Clobber the file with just `tome_home = "..."`. Simpler. May destroy user-valued content.

**Recommendation:** Option 1 (merge). Code:

```rust
fn write_xdg_tome_home(tome_home: &Path) -> Result<()> {
    let path = dirs::home_dir().context("home")?.join(".config/tome/config.toml");
    let collapsed = crate::paths::collapse_home_path(tome_home);
    let as_str = collapsed.to_string_lossy().into_owned();

    let mut table: toml::Table = if path.is_file() {
        std::fs::read_to_string(&path)?.parse().context("parse existing XDG config")?
    } else {
        toml::Table::new()
    };
    table.insert("tome_home".to_string(), toml::Value::String(as_str));

    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    // Atomic write: temp + rename (matches machine.rs, lockfile.rs pattern)
    let tmp = path.with_extension("toml.tmp");
    std::fs::write(&tmp, toml::to_string_pretty(&table)?)?;
    std::fs::rename(&tmp, &path)?;
    Ok(())
}
```

**Collapse vs expand:** Write with `~/` form (`paths::collapse_home_path`) so the file is portable across machines. `read_config_tome_home` tilde-expands on read.

**Warn if file already has a different `tome_home`:** The merge step would silently overwrite. Defensive behavior: if the existing value differs from what we're about to write, surface it and ask for confirmation. Keep scope-creep in check though — this can also be a follow-up issue if time is tight.

**No-input:** Do not write XDG config in headless mode. The user is likely a CI job or an integration test — side effects outside `TOME_HOME` would be surprising.

## Integration with `no_input` (Auto Behaviors)

| Prompt | Headless behavior | Rationale |
|--------|------------------|-----------|
| WUX-04 "resolved tome_home" info line | **Print** | Info, not a prompt |
| WUX-03 Legacy delete/move-aside | **Leave** | Destructive; silent deletion would surprise CI users |
| WUX-02 Brownfield use/edit/reinit/cancel | **Use existing** | D-3 locked decision; safest for dotfiles workflow |
| WUX-01 Greenfield tome_home | **Default to `~/.tome/`** | Same as current behavior |
| WUX-05 Persist via XDG | **Skip** | Don't touch XDG in headless mode |

For the "Leave" and "Skip" branches, emit a stderr `note:` line so CI log readers know a step was skipped, mirroring the existing git-init-for-backup skip note at `wizard.rs:357`.

## Testing Strategy

### Unit tests (co-located in `wizard.rs`, `lib.rs`, `config.rs`)

- `detect_machine_state`:
  - Greenfield (empty home, empty tome_home).
  - Brownfield (tome.toml exists at tome_home/tome.toml).
  - Brownfield with `.tome/` subdir layout.
  - Legacy (XDG file with `[[sources]]` only).
  - Legacy (XDG file with `[targets.*]` only).
  - Legacy (both sections).
  - NOT legacy: XDG file with only `tome_home = "..."`.
  - NOT legacy: XDG file that fails to parse (return None, don't crash).
  - Brownfield + legacy (both flags set).
  - Symlinked tome.toml → brownfield.

- `has_legacy_sections`: isolate from `detect_machine_state` so false-positive matrix is directly covered.

- `write_xdg_tome_home`:
  - Writes fresh file with `tome_home` key.
  - Merges into existing XDG file, preserving other keys (insert new `tome_home`, preserve `anything_else = "value"`).
  - Overwrites an existing `tome_home` key.
  - Creates parent dir if missing.
  - Atomic: temp file does not survive on error.

- `resolve_tome_home_with_source`: returns the correct source label for every branch (flag, env, XDG, default).

### Integration tests (in `tests/cli.rs`)

Build on the existing `init_dry_run_no_input_empty_home` / `init_dry_run_no_input_seeded_home` / `init_no_input_writes_config_and_reloads` harness. All three already use `.env("HOME", tmp.path())` + `.env("TOME_HOME", ...)` to isolate from the real user home. This isolation pattern handles WUX-03 naturally: legacy-file detection looks under `HOME/.config/tome/config.toml`, which resolves inside the TempDir.

Add:

- `init_prints_resolved_tome_home` — stdout contains `"resolved tome_home: <path>"` line on every invocation.
- `init_prints_source_label_default` / `_env` / `_flag` — source label matches the resolution branch.
- `init_brownfield_no_input_keeps_existing` — seed tome.toml at TOME_HOME/tome.toml, run `init --no-input`, verify the file is byte-identical before and after, no post-init sync side effects in the library.
- `init_brownfield_reinit_backs_up` — same seed, but run an interactive test that picks "reinitialize"; assert `tome.toml.backup-*` exists and the new file validates. (Note: interactive tests via `assert_cmd` require piping stdin; dialoguer reads from tty by default. The existing `--no-input` plumbing avoids this problem for headless tests; for interactive coverage, prefer unit tests that drive the branch dispatcher directly without going through dialoguer.)
- `init_legacy_detected_no_input_leaves_file` — seed `$HOME/.config/tome/config.toml` with `[[sources]]`, run `init --no-input`, assert the file is unchanged and stderr contains "legacy" + "skipped".
- `init_legacy_with_only_tome_home_not_flagged` — seed XDG with `tome_home = "~/somewhere"`, run `init --no-input`, assert NO legacy warning appears.

### Isolation from real `~/.config/tome/`

`HOME` env-var isolation already works for the XDG location because `read_config_tome_home` uses `dirs::home_dir()`, which honors `HOME`. All existing init tests (`tests/cli.rs:3759+`) use this pattern successfully. **No new isolation mechanism needed.** Do NOT parameterize `config_dir` / `tome_home_resolver` for DI — the `HOME` env knob is sufficient and matches the rest of the test suite.

## Risks & Open Questions

### Risk: Wizard signature change ripples

Adding `tome_home: &Path, prefill: Option<&Config>` to `wizard::run` is a breaking change to the `pub fn` surface. `wizard` is `pub(crate)` (`lib.rs:50`), so the only caller is `lib.rs::run`. Safe to change. No external consumers.

### Risk: `resolve_tome_home` → `resolve_tome_home_with_source` requires plumbing

Current callers of `resolve_tome_home` (lib.rs for every subcommand) only need the path. Rather than changing the shared helper, add a sibling `resolve_tome_home_with_source` used only by the init path. Keeps the blast radius small.

### Risk: Prefill-aware `configure_library` default behavior

When prefill has a non-default library path, current wizard shows `[~/.tome/skills (default), Custom path...]`. With prefill, we should show `[<prefill> (current), ~/.tome/skills (default), Custom path...]` — 3 options instead of 2, with "current" as the default. Small UX difference, but worth flagging so planner can treat it as a distinct task.

### Risk: Dialoguer `Input::validate_with` on `&String` vs `&str` type quirks

Recent dialoguer versions tightened the validator signature. If clippy complains about `&String`, use `.map(|s| s.as_str())`. Minor; no blocker.

### Open question: Brownfield "cancel" return path

When the user picks "Cancel", `tome init` should exit cleanly with code 0. Simple `return Ok(())` before `sync()` is the right shape. Confirm with planner: should "cancel" skip the post-init sync AND emit a final `"No changes."` line to stdout? Recommend yes.

### Open question: Display of last-modified time

`std::time::SystemTime` has no nice display impl. Options:
- Compute `now - mtime` and print `"3 days ago"`, `"5 minutes ago"` etc. — lightweight; std only.
- ISO-8601 format via manual arithmetic — readable but verbose.
- Add `chrono` — dependency bloat for one formatting operation.

**Recommendation:** Relative time with std (`SystemTime::now().duration_since(mtime)` → human-format bucket). The chrono cost isn't worth it.

### Open question: Legacy file probe in non-init commands

Should `tome sync` / `tome status` also warn on legacy XDG file? REQUIREMENTS.md only lists WUX-03 for the wizard. Stick to init for v0.8; file a follow-up issue if the UX gap surfaces in practice.

## Standard Stack

All items already in tree; no installations required.

| Crate | Version | Purpose | Notes |
|-------|---------|---------|-------|
| `dialoguer` | 0.12 | `Confirm`, `Input`, `Select` — same prompts the current wizard already uses | `validate_with` for path validation |
| `toml` | 1 | Parse XDG file, detect legacy sections, write merged XDG file | Use `toml::Table` for the legacy detector, not substring matching |
| `console` | 0.16 | `style(...)` colored output for info / warn / note | Existing pattern throughout wizard |
| `anyhow` | 1 | `Context`, `bail!`, `ensure!` | Existing convention |
| `tempfile` (dev) | 3 | `TempDir` for every unit and integration test | Existing convention |
| `assert_cmd` (dev) | 2 | `tome()` harness for integration tests | Existing convention |
| `predicates` (dev) | 3 | `predicate::str::contains(...)` for stderr assertions | Existing convention |

**Verification of versions:** These match the workspace `Cargo.toml` on this checkout; see STACK.md (imported via CLAUDE.md technology-stack block). No `npm view` equivalent needed — Rust crate versions are locked in `Cargo.lock`.

## Architecture Patterns

### Recommended new structure

```
crates/tome/src/
├── wizard.rs                # Extend: add tome_home step, prefill support
├── wizard/                  # Optional: split if wizard.rs grows too large
│   ├── mod.rs
│   ├── state.rs             # MachineState + detect_machine_state + has_legacy_sections
│   ├── brownfield.rs        # Brownfield prompt + backup + prefill plumbing
│   └── legacy.rs            # Legacy detect + delete/move-aside
├── lib.rs                   # Thread MachineState dispatch into Command::Init
└── config.rs                # Extend: write_xdg_tome_home helper
```

**Recommendation:** Keep wizard.rs as a single file; Rust idiom prefers one file until it exceeds ~1500 lines or domain boundaries are very clear. wizard.rs today is 1003 lines; adding ~400 lines for Phase 7 puts it at ~1400, still manageable. Splitting into a `wizard/` directory is a cleanup that can follow in a later phase.

### Pattern 1: Enum-driven dispatch

`MachineState` → `match state { ... }` dispatch in `lib.rs::Command::Init` branch. Mirrors existing patterns in `remove.rs` (plan/render/execute) and `update.rs` (diff triage).

### Pattern 2: Plumbing a prefill instead of building a separate "edit mode"

Recommended. See WUX-02 § "Edit existing" above. Mirrors how `sync` plumbs `SyncOptions` through the pipeline — a single entry point with optional knobs.

### Anti-patterns to avoid

- **Substring-matching TOML content to detect `[[sources]]`.** Use `toml::Table` parsing. A config file with a comment `# TODO: re-add [[sources]]` must not trigger a legacy warning.
- **Overwriting the XDG config with just `tome_home = "..."`.** Merge into existing TOML to preserve user-valued content.
- **Silent auto-delete of the legacy file.** Locked out by D-4.
- **Re-implementing `Config::save_checked`.** Use it. It already does expand → validate → round-trip → write atomically.
- **New environment variables.** The phase adds zero new env vars. All behavior is derived from existing resolution chain + filesystem probes.

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Validating a config before write | Ad-hoc error checks | `Config::save_checked` (config.rs:533) | Does expand → validate → TOML round-trip → atomic write; already handles every failure mode |
| Reading `~/.config/tome/config.toml` for `tome_home` | New TOML parser | `read_config_tome_home` (config.rs:637) | Already handles missing file, bad TOML, non-string value |
| Silencing prompts | `eprintln!` guard macros | `no_input: bool` parameter threading | Existing idiom; `cli.rs:47` is the source of truth |
| Atomic TOML file write | Manual temp+rename | Match the pattern in `machine.rs` / `lockfile.rs` | Same three-line dance; keep consistent |
| Path expansion / collapse | Manual string prefix logic | `config::expand_tilde` / `paths::collapse_home_path` | Handle edge cases (missing HOME, absolute paths, non-tilde relative) |
| Tempdir isolation in tests | Custom fixture crate | `tempfile::TempDir` + `HOME` env override | Existing convention; every init test uses this |
| CLI arg parsing for future flags | Ad-hoc flag mutation | `clap` `#[arg(long)]` on `Cli` struct | Already in use |

## Common Pitfalls

### Pitfall 1: Resolution chain re-evaluates after XDG write

**What goes wrong:** User picks custom tome_home + "persist to XDG". Post-init sync reads `config.library_dir` from the wizard-returned Config, which was assembled before WUX-01 knew the custom path.

**Why it happens:** In the current wizard, `configure_library` defaults to `~/.tome/skills`, which is a hardcoded path — not derived from tome_home. After WUX-01 changes tome_home to `/opt/tome`, the library default should logically become `/opt/tome/skills`, not `~/.tome/skills`.

**How to avoid:** Make `configure_library`'s default derive from the chosen `tome_home` (`<tome_home>/skills`). This is a small change to `wizard.rs:522` (`let default = PathBuf::from("~/.tome/skills");`). Note: this overlaps with the deferred #456 "library default derivation" quick-win; either ship it in Phase 7 or explicitly scope it out.

**Warning signs:** A wizard run with a custom tome_home produces a `tome.toml` whose `library_dir = "~/.tome/skills"` but whose directories are outside `~/.tome/`. Won't fail `validate()` unless there's overlap, but it's surprising.

### Pitfall 2: Brownfield edit prefill drift

**What goes wrong:** User edits, saves, and the resulting config is missing fields that were in the original (e.g. a custom role on a directory, a non-default branch on a git dir).

**Why it happens:** `configure_directories` rebuilds the map from the `KNOWN_DIRECTORIES` registry match. A prefilled custom directory (not in registry) won't appear in the auto-discovered list and will be silently dropped.

**How to avoid:** After Step 1 builds the directory map from selections, **union** the map with any prefill entries not already present. This keeps custom directories alive through an edit.

**Warning signs:** Edit + save produces a `tome.toml` with fewer entries than the pre-edit file. Integration test: seed with a custom `[directories.my-team]`, edit, assert `my-team` still present.

### Pitfall 3: Legacy detector flags a v0.6+ XDG config

**What goes wrong:** User has `~/.config/tome/config.toml` containing `tome_home = "~/custom"`. The TOML parser would return a `Table`, and if we check for `sources`/`targets` keys naively, we'd return None — but if some future contributor adds substring matching for robustness, `# mentions [[sources]] in comments` would false-positive.

**Why it happens:** TOML comments are stripped after parse, but mid-refactor someone might "help" by adding a belt-and-suspenders substring check.

**How to avoid:** Only use `toml::Table::get("sources") / get("targets")`. Document why in a comment pointing at this research doc.

**Warning signs:** User reports "legacy warning on a clean v0.6 config."

### Pitfall 4: `Input::validate_with` eats the actual error

**What goes wrong:** A complex validator returns `Err(&'static str)` with a canned message; the real underlying error (e.g. "permission denied expanding symlink") is lost.

**How to avoid:** For path validation, keep the validator simple (absolute / file-at-path) and let `Config::save_checked` surface the actual filesystem error on the downstream write. Don't try to pre-check every failure mode in the validator.

### Pitfall 5: Interactive tests hanging in CI

**What goes wrong:** Test author writes an interactive flow test, dialoguer blocks on stdin.

**How to avoid:** All new integration tests MUST pass `--no-input`. For interactive-branch coverage, use unit tests that call the underlying dispatcher functions directly, bypassing dialoguer.

## Code Examples

### Example 1: Detecting legacy sections (core of WUX-03)

```rust
// Source: pattern from config.rs read_config_tome_home + toml::Table usage
fn has_legacy_sections(path: &Path) -> Result<Option<PathBuf>> {
    if !path.is_file() {
        return Ok(None);
    }
    let content = std::fs::read_to_string(path)
        .with_context(|| format!("failed to read {}", path.display()))?;
    let Ok(table) = content.parse::<toml::Table>() else {
        return Ok(None); // malformed → not flagged; user can clean up manually
    };
    let has_sources = table.get("sources").is_some_and(|v| v.is_array());
    let has_targets = table.get("targets").is_some_and(|v| v.is_table());
    Ok((has_sources || has_targets).then(|| path.to_path_buf()))
}
```

### Example 2: Atomic XDG write (core of WUX-05)

```rust
// Source: pattern from machine.rs temp+rename write
fn write_xdg_tome_home(tome_home: &Path) -> Result<()> {
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
    std::fs::rename(&tmp, &path)
        .with_context(|| format!("failed to rename {}", tmp.display()))?;
    Ok(())
}
```

### Example 3: Tome-home-with-source for WUX-04

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum TomeHomeSource {
    CliTomeHome,  // --tome-home
    CliConfig,    // --config flag (parent dir)
    EnvVar,       // TOME_HOME env
    XdgConfig,    // ~/.config/tome/config.toml
    Default,      // ~/.tome/
}

impl TomeHomeSource {
    pub fn label(self) -> &'static str {
        match self {
            Self::CliTomeHome => "--tome-home flag",
            Self::CliConfig   => "--config flag",
            Self::EnvVar      => "TOME_HOME env",
            Self::XdgConfig   => "~/.config/tome/config.toml",
            Self::Default     => "default",
        }
    }
}

fn resolve_tome_home_with_source(
    cli_tome_home: Option<&Path>,
    cli_config: Option<&Path>,
) -> Result<(PathBuf, TomeHomeSource)> {
    if let Some(p) = cli_tome_home {
        let expanded = config::expand_tilde(p)?;
        anyhow::ensure!(expanded.is_absolute(), "--tome-home must be absolute");
        return Ok((expanded, TomeHomeSource::CliTomeHome));
    }
    if let Some(p) = cli_config {
        anyhow::ensure!(p.is_absolute(), "config path must be absolute");
        let parent = p.parent().context("config path has no parent")?;
        return Ok((parent.to_path_buf(), TomeHomeSource::CliConfig));
    }
    // Probe env and XDG separately so we can attribute the source.
    match std::env::var("TOME_HOME") {
        Ok(v) if !v.is_empty() => return Ok((config::expand_tilde(Path::new(&v))?, TomeHomeSource::EnvVar)),
        _ => {}
    }
    // Re-implement the XDG branch inline so we can tag the source.
    // (Or: extract read_config_tome_home as pub(crate) and call it here.)
    if let Some(path) = config::read_config_tome_home_pub()? {
        return Ok((path, TomeHomeSource::XdgConfig));
    }
    Ok((
        dirs::home_dir().context("home")?.join(".tome"),
        TomeHomeSource::Default,
    ))
}
```

(Note: `read_config_tome_home` is currently private; visibility widens to `pub(crate)` with minimal risk.)

## State of the Art

No state-of-the-art changes apply — this is an in-tree UX extension, not a library-stack refresh. All patterns (dialoguer, toml, atomic writes, tempdir tests) are already established conventions in the codebase.

## Open Questions (ranked by blocker-to-non-blocker)

1. **Should "edit existing" also re-run auto-discovery to offer newly-appeared directories?**
   - What we know: prefill reconstructs existing entries. Auto-discovery finds new `~/.claude/skills` etc.
   - What's unclear: do we want the edit path to be "strict preserve" or "preserve + pick up new dirs".
   - Recommendation: preserve + pick up new dirs (additive). MultiSelect already supports this — existing entries start checked, new entries start unchecked.

2. **Relative-time mtime formatting vs ISO-8601** — see § "Open question: Display of last-modified time". Non-blocking.

3. **Library default derivation** — coupled to #456 / #457. Scope-creep risk. Recommend flagging to planner as a decision point.

4. **Warning when WUX-05 overwrites an existing different `tome_home`** — see § "WUX-05 — Persist tome_home via XDG". Nice-to-have; can be follow-up.

## Environment Availability

| Dependency | Required By | Available | Version | Fallback |
|------------|-------------|-----------|---------|----------|
| Rust toolchain | Build | ✓ | 1.85.0+ | — |
| `cargo test` | Tests | ✓ | Rust stable | — |
| `dialoguer` | Prompts | ✓ | 0.12 (in Cargo.toml) | — |
| `toml` | Parse + emit | ✓ | 1 (in Cargo.toml) | — |
| `tempfile` | Tests | ✓ | 3 (dev in Cargo.toml) | — |
| Network access | — | N/A | — | — |
| External services | — | N/A | — | — |

No missing dependencies, no blocking gaps.

## Sources

### Primary (HIGH confidence) — in-tree source files

- `crates/tome/src/config.rs` (1856 lines) — `default_tome_home` (line 616), `read_config_tome_home` (line 637), `resolve_config_dir` (line 664), `default_config_path` (line 677), `Config::load` (line 277), `Config::save_checked` (line 533).
- `crates/tome/src/lib.rs` — `resolve_tome_home` (line 105), `resolve_config_path` (line 137), `Command::Init` branch (line 162–197).
- `crates/tome/src/wizard.rs` (1003 lines) — `run` (line 137), `assemble_config` (line 392), `configure_directories` / `_library` / `_exclusions`.
- `crates/tome/src/paths.rs` — `TomePaths::new` (line 32), `resolve_config_dir` semantics (line 51), `collapse_home_path` (line 148).
- `crates/tome/src/cli.rs` — `Cli` struct (`no_input`, `tome_home`, `config` flags).
- `crates/tome/src/backup.rs` — git-snapshot pattern (available but likely not needed for file-level backup).
- `crates/tome/src/machine.rs` — atomic temp+rename pattern (mirror for XDG write).
- `crates/tome/tests/cli.rs` — headless init tests (lines 3758–3949), TOME_HOME tests (1895+).

### Primary (HIGH confidence) — planning docs

- `.planning/REQUIREMENTS.md` — WUX-01..05 spec (lines 25–33), Out of Scope table (60–68).
- `.planning/ROADMAP.md` — Phase 7 goal + 5 success criteria (lines 40–50).
- `.planning/STATE.md` — Decisions D-1..D-4 (lines 53–58).
- `.planning/PROJECT.md` — milestone context (lines 82–102).
- `.planning/codebase/CONVENTIONS.md` — newtype, error handling, pub(crate), atomic writes.
- `.planning/codebase/TESTING.md` — unit + integration + `tempfile::TempDir` conventions.

### Tertiary (not used)

No WebSearch / Context7 / WebFetch queries — the entire phase domain is in-tree. External sources would not add value.

## Metadata

**Confidence breakdown:**

- State classification: HIGH — all inputs are filesystem probes of paths we already manage.
- Brownfield flow: HIGH — existing `Config::load` + `Config::save_checked` do the heavy lifting.
- Legacy cleanup: HIGH — single-file detection + move/delete; no cross-system coordination.
- WUX-04 print: HIGH — trivial.
- WUX-05 XDG persist: HIGH — read → merge → atomic write; matches existing machine.rs pattern exactly.
- Integration with `--no-input`: HIGH — existing plumbing, every new prompt has documented auto-behavior.
- Test strategy: HIGH — existing `HOME`-env isolation pattern already in use for init tests; no new fixture needed.

**Research date:** 2026-04-23
**Valid until:** 2026-05-23 (30 days — stable in-tree domain)

## RESEARCH COMPLETE
