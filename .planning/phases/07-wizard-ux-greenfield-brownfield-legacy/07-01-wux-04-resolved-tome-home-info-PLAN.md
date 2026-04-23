---
phase: 07-wizard-ux-greenfield-brownfield-legacy
plan: 01
type: execute
wave: 1
depends_on: []
files_modified:
  - crates/tome/src/lib.rs
  - crates/tome/src/config.rs
  - crates/tome/tests/cli.rs
autonomous: true
requirements:
  - WUX-04
must_haves:
  truths:
    - "Every `tome init` invocation prints a single-line 'resolved tome_home: <path> (from <source>)' message before any Step 1 prompts"
    - "The source label accurately reflects the resolution branch: --tome-home flag, --config flag, TOME_HOME env, XDG config, or default"
    - "The info line is printed in both interactive and --no-input modes (it is informational, not a prompt)"
  artifacts:
    - path: "crates/tome/src/config.rs"
      provides: "TomeHomeSource enum + resolve_tome_home_with_source helper + pub(crate) read_config_tome_home visibility"
      contains: "enum TomeHomeSource"
    - path: "crates/tome/src/lib.rs"
      provides: "Command::Init branch prints resolved_tome_home line before calling wizard::run"
      contains: "resolved tome_home:"
    - path: "crates/tome/tests/cli.rs"
      provides: "integration tests covering every TomeHomeSource label branch"
      contains: "init_prints_resolved_tome_home"
  key_links:
    - from: "crates/tome/src/lib.rs Command::Init"
      to: "config::resolve_tome_home_with_source"
      via: "function call returning (PathBuf, TomeHomeSource)"
      pattern: "resolve_tome_home_with_source"
    - from: "crates/tome/src/lib.rs"
      to: "stdout"
      via: "println! with source.label()"
      pattern: "resolved tome_home:.*\\(from"
---

<objective>
Surface the resolved `tome_home` path and its resolution source to the user at the start of every `tome init` invocation. This is WUX-04 — the simplest phase 7 requirement, and the foundation for WUX-01 (which gates the greenfield prompt on `TomeHomeSource::Default`).

Purpose: Today `resolve_tome_home` silently returns a `PathBuf` and the wizard blows through without showing the user which path is about to be populated. A user who accidentally has `TOME_HOME=/wrong/path` exported in their shell rc gets no chance to abort. This plan prints a one-line `resolved tome_home: <path> (from <source>)` info message before Step 1 prompts, so users can Ctrl-C before any destructive write happens.

Output: `TomeHomeSource` enum, `resolve_tome_home_with_source` helper, updated `Command::Init` branch in `lib.rs`, and integration tests.
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
@crates/tome/src/lib.rs
@crates/tome/src/config.rs
@crates/tome/tests/cli.rs

<interfaces>
<!-- Existing APIs this plan touches. Copy these signatures verbatim into new code. -->

From crates/tome/src/lib.rs:
```rust
/// Resolution order:
/// 1. `--tome-home` CLI flag (highest priority)
/// 2. `--config` CLI flag (tome home = parent directory of config file)
/// 3. `TOME_HOME` env var (checked inside `default_tome_home()`)
/// 4. `~/.tome/` (default)
fn resolve_tome_home(
    cli_tome_home: Option<&Path>,
    cli_config: Option<&Path>,
) -> Result<std::path::PathBuf>;
```

From crates/tome/src/config.rs:
```rust
/// Default tome home directory.
///
/// Resolution order:
/// 1. `TOME_HOME` environment variable (if set and non-empty)
/// 2. `~/.config/tome/config.toml` -> `tome_home` field
/// 3. `~/.tome/`
pub fn default_tome_home() -> Result<PathBuf>;

/// Read `tome_home` from the machine-level config at `~/.config/tome/config.toml`.
fn read_config_tome_home() -> Result<Option<PathBuf>>;  // currently private

pub fn expand_tilde(path: &Path) -> Result<PathBuf>;
```

Target new API (from 07-RESEARCH.md § "Example 3"):
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
    pub fn label(self) -> &'static str { /* e.g. "--tome-home flag", "default" */ }
}

pub(crate) fn resolve_tome_home_with_source(
    cli_tome_home: Option<&Path>,
    cli_config: Option<&Path>,
) -> Result<(PathBuf, TomeHomeSource)>;
```
</interfaces>
</context>

<tasks>

<task type="auto" tdd="true">
  <name>Task 1: Add TomeHomeSource enum + resolve_tome_home_with_source helper in config.rs</name>
  <files>crates/tome/src/config.rs</files>
  <read_first>
    - crates/tome/src/config.rs (focus: lines 610–680 — default_tome_home, read_config_tome_home, resolve_config_dir)
    - crates/tome/src/lib.rs (focus: lines 100–130 — resolve_tome_home for the flag/config branch logic to mirror)
    - .planning/phases/07-wizard-ux-greenfield-brownfield-legacy/07-RESEARCH.md (focus: § "Example 3: Tome-home-with-source for WUX-04" lines 748–803)
    - .planning/codebase/CONVENTIONS.md (focus: pub(crate) idiom, error handling with anyhow::Context)
  </read_first>
  <behavior>
    - Test 1: `resolve_tome_home_with_source(Some(abs_path), None)` returns `(path, TomeHomeSource::CliTomeHome)` and label() == "--tome-home flag"
    - Test 2: `resolve_tome_home_with_source(None, Some(abs_config))` returns parent + `TomeHomeSource::CliConfig` and label() == "--config flag"
    - Test 3: With `TOME_HOME=/custom` env set, returns `(/custom, TomeHomeSource::EnvVar)` and label() == "TOME_HOME env"
    - Test 4: With XDG config containing `tome_home = "~/foo"`, returns `(<HOME>/foo, TomeHomeSource::XdgConfig)` and label() == "~/.config/tome/config.toml"
    - Test 5: Empty env + no XDG file + no CLI flags returns `(<HOME>/.tome, TomeHomeSource::Default)` and label() == "default"
    - Test 6: `--tome-home` with relative path returns Err (must be absolute) — matches existing resolve_tome_home behavior
    - All tests use `tempfile::TempDir` and manipulate `HOME` / `TOME_HOME` via temp_env or a saved/restored guard
  </behavior>
  <action>
Add the following to `crates/tome/src/config.rs` (after the existing `default_config_path` function around line 680, before the DEPRECATED COMPATIBILITY SHIMS section):

1. **Define `TomeHomeSource` enum as `pub(crate)`:**

```rust
/// Where the resolved `tome_home` came from in the resolution chain.
///
/// Used by the `tome init` WUX-04 info line to tell the user which branch
/// produced the path they are about to populate (e.g. "from TOME_HOME env"
/// vs "from default"). Also used by the wizard to decide whether to prompt
/// for a custom tome_home on greenfield (WUX-01 gates on Default).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum TomeHomeSource {
    CliTomeHome,
    CliConfig,
    EnvVar,
    XdgConfig,
    Default,
}

impl TomeHomeSource {
    pub(crate) fn label(self) -> &'static str {
        match self {
            Self::CliTomeHome => "--tome-home flag",
            Self::CliConfig   => "--config flag",
            Self::EnvVar      => "TOME_HOME env",
            Self::XdgConfig   => "~/.config/tome/config.toml",
            Self::Default     => "default",
        }
    }
}
```

2. **Widen `read_config_tome_home` visibility from private to `pub(crate)`** (line 637). Rename is not required; just change the `fn` prefix to `pub(crate) fn`. This unblocks the new helper calling it from a distinct code path while keeping the external API surface unchanged.

3. **Add `resolve_tome_home_with_source` as `pub(crate)`:**

```rust
/// Like `crate::resolve_tome_home` but also returns the resolution source.
///
/// Used by the `tome init` entry point to print the WUX-04 info line and
/// (via Plan 03) to gate the greenfield tome_home prompt on `Default`.
///
/// Resolution order mirrors `resolve_tome_home` + `default_tome_home` exactly,
/// split apart so we can attribute each branch:
/// 1. `--tome-home` flag (CliTomeHome)
/// 2. `--config` flag (CliConfig; tome_home = parent of config file)
/// 3. `TOME_HOME` env var, non-empty (EnvVar)
/// 4. `~/.config/tome/config.toml` `tome_home` key (XdgConfig)
/// 5. `~/.tome/` (Default)
pub(crate) fn resolve_tome_home_with_source(
    cli_tome_home: Option<&Path>,
    cli_config: Option<&Path>,
) -> Result<(PathBuf, TomeHomeSource)> {
    if let Some(p) = cli_tome_home {
        let expanded = expand_tilde(p)?;
        anyhow::ensure!(
            expanded.is_absolute(),
            "--tome-home path '{}' must be an absolute path",
            p.display()
        );
        return Ok((expanded, TomeHomeSource::CliTomeHome));
    }
    if let Some(p) = cli_config {
        anyhow::ensure!(
            p.is_absolute(),
            "config path '{}' must be an absolute path",
            p.display()
        );
        let parent = p.parent().context("config path has no parent directory")?;
        return Ok((parent.to_path_buf(), TomeHomeSource::CliConfig));
    }
    match std::env::var("TOME_HOME") {
        Ok(val) if !val.is_empty() => {
            return Ok((expand_tilde(Path::new(&val))?, TomeHomeSource::EnvVar));
        }
        Ok(_) => {}
        Err(std::env::VarError::NotPresent) => {}
        Err(std::env::VarError::NotUnicode(_)) => {
            anyhow::bail!("TOME_HOME environment variable contains invalid Unicode");
        }
    }
    if let Some(path) = read_config_tome_home()? {
        return Ok((path, TomeHomeSource::XdgConfig));
    }
    Ok((
        dirs::home_dir()
            .context("could not determine home directory")?
            .join(".tome"),
        TomeHomeSource::Default,
    ))
}
```

4. **Add unit tests** co-located under the existing `#[cfg(test)] mod tests` block in config.rs. Env-var isolation pattern: serialize the env-manipulating tests with a `Mutex` guard (or mark them `#[serial]` if the `serial_test` crate is already in the dep tree — check `Cargo.toml`; if not, use the std-based approach below).

If no env-serialization helper exists, add one at the top of the test module:

```rust
#[cfg(test)]
mod tests {
    // ... existing tests above ...

    // Serialize tests that manipulate TOME_HOME / HOME; env is process-wide.
    // Matches the pattern used elsewhere in this file when env is touched.
    static ENV_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

    fn with_env<F, R>(vars: &[(&str, Option<&std::ffi::OsStr>)], f: F) -> R
    where
        F: FnOnce() -> R,
    {
        let _guard = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let saved: Vec<(String, Option<std::ffi::OsString>)> = vars
            .iter()
            .map(|(k, _)| (k.to_string(), std::env::var_os(k)))
            .collect();
        for (k, v) in vars {
            match v {
                Some(val) => unsafe { std::env::set_var(k, val) },
                None => unsafe { std::env::remove_var(k) },
            }
        }
        let result = f();
        for (k, v) in saved {
            match v {
                Some(val) => unsafe { std::env::set_var(&k, val) },
                None => unsafe { std::env::remove_var(&k) },
            }
        }
        result
    }
}
```
*Note:* `std::env::set_var` / `remove_var` are unsafe in edition 2024; use `unsafe { ... }` blocks. If the rest of the codebase already wraps env-manipulation in a shared helper, use that instead — grep `rg -n "set_var|remove_var" crates/tome/src` to find it first.

Then add the six tests from `<behavior>`. Label assertions compare equality against the expected `&'static str` returned by `TomeHomeSource::label()`.
  </action>
  <verify>
    <automated>cargo test --package tome -- config::tests::resolve_tome_home_with_source 2>&1 | tail -20</automated>
  </verify>
  <acceptance_criteria>
    - `rg -n "pub\\(crate\\) enum TomeHomeSource" crates/tome/src/config.rs` returns at least one match
    - `rg -n "pub\\(crate\\) fn resolve_tome_home_with_source" crates/tome/src/config.rs` returns at least one match
    - `rg -n "pub\\(crate\\) fn read_config_tome_home" crates/tome/src/config.rs` returns at least one match (visibility widened)
    - `rg -n "fn label" crates/tome/src/config.rs` shows the label() method returning &'static str
    - `cargo test --package tome -- config::tests::resolve_tome_home_with_source` exits 0 and reports at least 6 tests passing
    - `cargo clippy --all-targets -- -D warnings` exits 0
    - `cargo fmt -- --check` exits 0
  </acceptance_criteria>
  <done>
    TomeHomeSource enum exists at pub(crate) visibility with 5 variants and a label() method returning the exact strings from RESEARCH.md. `resolve_tome_home_with_source` covers every branch of the existing resolution chain and returns a tagged source. Unit tests lock in the label strings and branch behavior. Clippy + fmt clean.
  </done>
</task>

<task type="auto" tdd="true">
  <name>Task 2: Print resolved tome_home line in Command::Init + integration tests</name>
  <files>crates/tome/src/lib.rs, crates/tome/tests/cli.rs</files>
  <read_first>
    - crates/tome/src/lib.rs (focus: lines 153–197 — `pub fn run(cli: Cli)` and the Command::Init branch)
    - crates/tome/src/config.rs (focus: the newly added TomeHomeSource + resolve_tome_home_with_source from Task 1)
    - crates/tome/tests/cli.rs (focus: lines 3758–3949 — existing init_dry_run_no_input_* harness patterns, `.env("HOME", ...)` + `.env("TOME_HOME", ...)` isolation)
    - .planning/phases/07-wizard-ux-greenfield-brownfield-legacy/07-RESEARCH.md (focus: § "WUX-04 — Print resolved tome_home (simplest, do first)" lines 215–234)
  </read_first>
  <behavior>
    - Test 1 (integration): `init_prints_resolved_tome_home_with_default_source` — empty HOME, no TOME_HOME, stdout contains "resolved tome_home:" AND "(from default)"
    - Test 2 (integration): `init_prints_resolved_tome_home_with_env_source` — `TOME_HOME=<tmp>/.tome`, stdout contains "resolved tome_home:" + the path + "(from TOME_HOME env)"
    - Test 3 (integration): `init_prints_resolved_tome_home_with_flag_source` — `--tome-home <tmp>/custom`, stdout contains "(from --tome-home flag)"
    - Test 4 (integration): info line appears BEFORE the first "Step" prompt output — asserted by substring-position check (stdout.find("resolved tome_home:") < stdout.find("Step 1"))
  </behavior>
  <action>
**Part A — `crates/tome/src/lib.rs` edit:**

In the `Command::Init` branch (around line 162–197), replace the current `resolve_tome_home` call with a call to the new source-aware helper and print the info line immediately after it.

Current code at lib.rs:169:
```rust
let tome_home = resolve_tome_home(cli.tome_home.as_deref(), cli.config.as_deref())?;
let config = wizard::run(cli.dry_run, cli.no_input)?;
```

Replace with:
```rust
let (tome_home, tome_home_source) =
    config::resolve_tome_home_with_source(cli.tome_home.as_deref(), cli.config.as_deref())?;
println!();
println!(
    "resolved tome_home: {} (from {})",
    console::style(tome_home.display()).cyan(),
    tome_home_source.label()
);
let config = wizard::run(cli.dry_run, cli.no_input)?;
```

Notes:
- Use `console::style` (already imported/used elsewhere in lib.rs — verify with `rg -n "use console" crates/tome/src/lib.rs`; if not, add `use console::style;` to the existing imports at the top of lib.rs).
- Do NOT delete the existing `resolve_tome_home` function — the non-init code path at lib.rs:201 still uses it. Only the Init branch needs the tagged variant.
- The info line is printed in BOTH interactive and `--no-input` modes (it is informational per RESEARCH.md § "No-input behavior" line 234).
- The `tome_home_source` variable is bound but will only be consumed later (by plans 03 and 04). For this plan, a single call-site use (the println!) is sufficient — clippy will NOT flag it as unused because it is used in the println!. If it IS flagged, suppress with `let _ = tome_home_source;` after the println! and leave a comment: `// Used by WUX-01 prompt gating in plan 03.` — preferred over `#[allow]`.

**Part B — `crates/tome/tests/cli.rs` edits:**

Add four integration tests after the existing `init_no_input_writes_config_and_reloads` test (currently the last init test around line 3894+). Use the existing `tome()` helper and the HOME/TOME_HOME isolation pattern from lines 3762–3772.

```rust
#[test]
fn init_prints_resolved_tome_home_with_default_source() {
    // No TOME_HOME set, HOME has no ~/.config/tome/config.toml → Default source.
    let tmp = TempDir::new().unwrap();

    let output = tome()
        .args(["init", "--dry-run", "--no-input"])
        .env("HOME", tmp.path())
        .env_remove("TOME_HOME")
        .env("NO_COLOR", "1")
        .output()
        .unwrap();
    assert!(output.status.success(),
        "tome init failed: {}",
        String::from_utf8_lossy(&output.stderr));
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("resolved tome_home:"),
        "stdout missing resolved tome_home line:\n{stdout}"
    );
    assert!(
        stdout.contains("(from default)"),
        "stdout missing '(from default)' source label:\n{stdout}"
    );
}

#[test]
fn init_prints_resolved_tome_home_with_env_source() {
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
        stdout.contains("(from TOME_HOME env)"),
        "stdout missing '(from TOME_HOME env)' label:\n{stdout}"
    );
    assert!(
        stdout.contains(tome_home.display().to_string().as_str()),
        "stdout missing TOME_HOME path:\n{stdout}"
    );
}

#[test]
fn init_prints_resolved_tome_home_with_flag_source() {
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
    assert!(output.status.success(),
        "stderr: {}", String::from_utf8_lossy(&output.stderr));
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("(from --tome-home flag)"),
        "stdout missing '--tome-home flag' label:\n{stdout}"
    );
}

#[test]
fn init_resolved_tome_home_line_precedes_step_prompts() {
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
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();

    let resolved_idx = stdout.find("resolved tome_home:").expect("missing info line");
    let step1_idx = stdout.find("Step 1").expect("missing Step 1 prompt header");
    assert!(
        resolved_idx < step1_idx,
        "resolved tome_home line must come BEFORE Step 1.\n\
         resolved_idx={resolved_idx}, step1_idx={step1_idx}\nstdout:\n{stdout}"
    );
}
```

Notes:
- `env_remove("TOME_HOME")` clears any inherited env so the Default branch is exercised cleanly.
- `NO_COLOR=1` strips ANSI codes so substring matching works reliably.
- The "Step 1" header is emitted by `configure_directories` via `step_divider` — grep confirms it exists: `rg -n 'Step 1' crates/tome/src/wizard.rs` should show a match in `configure_directories`.
  </action>
  <verify>
    <automated>cargo test --package tome --test cli -- init_prints_resolved_tome_home init_resolved_tome_home_line_precedes 2>&1 | tail -20</automated>
  </verify>
  <acceptance_criteria>
    - `rg -n "resolved tome_home:" crates/tome/src/lib.rs` returns at least one match
    - `rg -n "resolve_tome_home_with_source" crates/tome/src/lib.rs` returns at least one match (lib.rs calls the new helper for Init)
    - `rg -n "fn init_prints_resolved_tome_home_with_default_source" crates/tome/tests/cli.rs` returns a match
    - `rg -n "fn init_prints_resolved_tome_home_with_env_source" crates/tome/tests/cli.rs` returns a match
    - `rg -n "fn init_prints_resolved_tome_home_with_flag_source" crates/tome/tests/cli.rs` returns a match
    - `rg -n "fn init_resolved_tome_home_line_precedes_step_prompts" crates/tome/tests/cli.rs` returns a match
    - `cargo test --package tome --test cli -- init_prints_resolved_tome_home` exits 0 and reports 3 tests passing
    - `cargo test --package tome --test cli -- init_resolved_tome_home_line_precedes` exits 0 and reports 1 test passing
    - All existing init_* tests in cli.rs still pass: `cargo test --package tome --test cli -- init_` exits 0
    - `cargo clippy --all-targets -- -D warnings` exits 0
    - `cargo fmt -- --check` exits 0
  </acceptance_criteria>
  <done>
    `tome init` prints a `resolved tome_home: <path> (from <source>)` line to stdout before any Step 1 prompts in both interactive and --no-input modes. The source label matches the active resolution branch. Integration tests lock in the behavior across the default / env / flag branches AND the ordering-before-Step-1 invariant.
  </done>
</task>

</tasks>

<verification>
- `cargo test --package tome` exits 0 (all unit + integration tests pass, no regressions in existing init_dry_run_no_input_empty_home / _seeded_home / _writes_config_and_reloads)
- `cargo clippy --all-targets -- -D warnings` exits 0
- `cargo fmt -- --check` exits 0
- `make ci` exits 0
- Every one of the 5 TomeHomeSource::label() strings is a substring of stdout for at least one code path
</verification>

<success_criteria>
- WUX-04 success criterion from ROADMAP.md line 48: "Every `tome init` invocation prints a 1-line 'resolved tome_home: <path>' info message before Step 1 prompts, so the user can abort immediately if the wrong path is about to be populated" — demonstrably TRUE via `init_resolved_tome_home_line_precedes_step_prompts` integration test
- Foundation laid for WUX-01 (plan 03): `TomeHomeSource` enum is importable from `config` at `pub(crate)` visibility
- No breaking changes to existing `resolve_tome_home` — the helper is additive
</success_criteria>

<output>
After completion, create `.planning/phases/07-wizard-ux-greenfield-brownfield-legacy/07-01-SUMMARY.md` with:
- TomeHomeSource variants and their label() strings (for downstream plans to reference)
- Exact import path: `use crate::config::{TomeHomeSource, resolve_tome_home_with_source};`
- Line numbers of the new function + enum in config.rs (for plan 03 to know where to look)
</output>
