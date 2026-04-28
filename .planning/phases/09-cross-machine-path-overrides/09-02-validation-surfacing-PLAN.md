---
phase: 09-cross-machine-path-overrides
plan: 02
type: execute
wave: 2
depends_on: [09-01]
files_modified:
  - crates/tome/src/config.rs
  - crates/tome/src/lib.rs
  - crates/tome/tests/cli.rs
autonomous: true
requirements: [PORT-03, PORT-04]
issue: "https://github.com/MartinP7r/tome/issues/458"

must_haves:
  truths:
    - "An override targeting a directory name not present in `tome.toml` produces a single stderr `warning:` line naming the typo target and continues loading — does not abort"
    - "When `apply_machine_overrides` produces a path that fails `Config::validate()` (e.g., overlaps `library_dir`), the user sees a distinct error class that names `machine.toml` as the file to edit, not `tome.toml`"
    - "Validation errors NOT caused by an override (i.e., the underlying `tome.toml` was already invalid) continue to surface as before — only override-induced failures get the new wrapper"
  artifacts:
    - path: "crates/tome/src/config.rs"
      provides: "`Config::warn_unknown_overrides(prefs: &MachinePrefs, mut warn: impl FnMut(String))` helper that walks `prefs.directory_overrides` and emits a `warning:` string for any key not present in `self.directories`. `Config::load_with_overrides` is updated to: (1) call `warn_unknown_overrides` before `apply_machine_overrides`, emitting via `eprintln!`, (2) snapshot a pre-override clone, (3) wrap the post-override `validate()` Err into a new `OverrideValidationError` shape that names `machine.toml` and contrasts pre-override vs post-override paths."
      contains: "warn_unknown_overrides"
    - path: "crates/tome/src/lib.rs"
      provides: "No structural changes — `run()` already calls `Config::load_or_default_with_overrides` from Plan 01. The new warnings + error class flow through automatically."
      contains: "load_or_default_with_overrides"
    - path: "crates/tome/tests/cli.rs"
      provides: "Two integration tests: (1) override with unknown target name produces stderr warning, command still succeeds; (2) override that creates a library/distribution overlap produces a distinct error message naming `machine.toml`."
      contains: "machine_override_unknown_target_warns"
  key_links:
    - from: "crates/tome/src/config.rs Config::load_with_overrides"
      to: "Config::warn_unknown_overrides"
      via: "called once before apply_machine_overrides; emits via eprintln! closure"
      pattern: "warn_unknown_overrides"
    - from: "crates/tome/src/config.rs Config::load_with_overrides"
      to: "OverrideValidationError wrapper"
      via: "validate() Err branch wrapped only when at least one override was applied AND the same validate() succeeds on the pre-override snapshot"
      pattern: "OverrideValidationError|machine\\.toml"
---

<objective>
Surface override-related issues with the right level of noise:

- **PORT-03 (typo guard):** A `[directory_overrides.<name>]` block whose `<name>` doesn't match any directory in `tome.toml` emits a single stderr `warning:` line and load continues. Without this, a user who misspells `claude` as `claud` silently loses their override and wonders why their path didn't change.

- **PORT-04 (validation blame attribution):** When `apply_machine_overrides` rewrites a path that ends up making `validate()` fail (e.g., the new path overlaps `library_dir`), the resulting error names `machine.toml` as the file to edit — NOT `tome.toml`. Without this, the user sees "library_dir overlaps distribution directory 'work'" and wastes time editing `tome.toml`, where everything is fine.

Plan 01 left `apply_machine_overrides` as a silent infallible mutation and `load_with_overrides` returning the raw `validate()` error. This plan adds the surfacing layer on top.

**Closes:** PORT-03 (typo warning), PORT-04 (override-induced validation error class).

Purpose: Make the override mechanism a tool the user can debug, not a black box.
Output: `Config::warn_unknown_overrides` helper, `OverrideValidationError` wrapper, two integration tests.
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

@.planning/phases/09-cross-machine-path-overrides/09-01-SUMMARY.md

@crates/tome/src/config.rs
@crates/tome/src/machine.rs
@crates/tome/src/lib.rs
@crates/tome/tests/cli.rs

<interfaces>
<!-- Key types and contracts the executor needs. After Plan 01. -->

From `crates/tome/src/config.rs` (after Plan 01):
```rust
pub fn load_with_overrides(path: &Path, prefs: &MachinePrefs) -> Result<Self> {
    let mut config = /* parse TOML or default */;
    config.expand_tildes()?;
    config.apply_machine_overrides(prefs)?;   // <-- silent for unknown targets
    config.validate()?;                        // <-- raw validate() error
    Ok(config)
}

pub(crate) fn apply_machine_overrides(&mut self, prefs: &MachinePrefs) -> Result<()>;
```

From `crates/tome/src/config.rs::validate()` — error message style template (already established by D-10 Conflict+Why+Suggestion):
```text
library_dir overlaps distribution directory 'work'
Conflict: library_dir (/foo) is the same path as directory 'work' (/foo)
Why: this directory has role <X>; tome would try to distribute the library into itself...
hint: choose a library_dir outside any distribution directory, such as '~/.tome/skills'.
```

The Plan 02 wrapper PRESERVES the original error text and PREPENDS a header that names `machine.toml`:
```text
override-induced config error from machine.toml

The following directory paths come from `[directory_overrides.<name>]` overrides:
  - work: /foo  (was: /old, in tome.toml)

These overrides made an otherwise-valid `tome.toml` fail validation:

<original validate() error here, indented 2 spaces>

To fix: edit `<machine_toml_path>` (NOT tome.toml). Either remove the override
or change it to a path that doesn't overlap library_dir.
```

From `crates/tome/src/lib.rs::run` (after Plan 01):
```rust
let machine_path = resolve_machine_path(cli.machine.as_deref())?;
let machine_prefs = machine::load(&machine_path)?;
let config = Config::load_or_default_with_overrides(effective_config.as_deref(), &machine_prefs)?;
```

The new error wrapper needs to know `machine_path` to put it in the message. Two options:
- (a) Pass `machine_path: &Path` into `load_with_overrides` as a third arg.
- (b) Resolve `machine::default_machine_path()` at error-construction time inside `load_with_overrides`.

**Choose (a)** — explicit threading is clearer than reaching for a default that may not match `cli.machine.as_deref()`. The `lib.rs::run` already has `machine_path` in scope; passing it costs one more parameter.

**Existing pattern to mirror:** `lib.rs::warn_unknown_disabled_directories` (line ~679) handles the typo case for `disabled_directories`. The same shape works here: take `(prefs: &MachinePrefs, config: &Config)`, walk the relevant map, `eprintln!` for each miss. Keep the warning string parallel:

```text
warning: directory_overrides target 'claud' in machine.toml does not match any configured directory
```

Note: `warn_unknown_disabled_directories` uses an `eprintln!` directly. We have two choices for `warn_unknown_overrides`:
- Match that pattern (write `eprintln!` inline), OR
- Take an `impl FnMut(String)` so it's testable without capturing stderr.

**Choose the FnMut shape** — matching the existing pattern is fine, but `warn_unknown_disabled_directories` doesn't have unit tests; we want `warn_unknown_overrides` to be unit-testable so the warning string format is locked in. The caller in `Config::load_with_overrides` adapts via `|s| eprintln!("warning: {}", s)`.
</interfaces>
</context>

<tasks>

<task type="auto" tdd="true">
  <name>Task 1: Add `Config::warn_unknown_overrides` helper</name>
  <files>crates/tome/src/config.rs</files>
  <read_first>
    - crates/tome/src/lib.rs lines 677–688 (`warn_unknown_disabled_directories` — mirror this shape)
    - crates/tome/src/config.rs (the `apply_machine_overrides` method added in Plan 01)
    - crates/tome/src/machine.rs (the `directory_overrides` field added in Plan 01)
  </read_first>
  <behavior>
    - Test 1 (`warn_unknown_overrides_no_overrides_emits_nothing`): Empty `directory_overrides` → `warn` closure called 0 times.
    - Test 2 (`warn_unknown_overrides_known_target_emits_nothing`): Override target `x` exists in `config.directories` → `warn` called 0 times.
    - Test 3 (`warn_unknown_overrides_unknown_target_emits_one_warning`): Override target `claud` (typo) does NOT exist in `config.directories` → `warn` called exactly once with a message containing `"claud"`, `"machine.toml"`, AND either `"directory_overrides"` or `"override"`. (Verifies user can grep stderr for any of the three keywords.)
    - Test 4 (`warn_unknown_overrides_multiple_unknowns_emit_one_each`): Two unknown targets `a` and `b` → `warn` called exactly twice (once per target). Order is alphabetical (`BTreeMap` iteration).
    - Test 5 (`warn_unknown_overrides_does_not_mutate_config`): Calling the helper does not mutate `self` — it's `&self`, not `&mut self`.
  </behavior>
  <action>
Add to `impl Config` in `crates/tome/src/config.rs`, immediately after `apply_machine_overrides` (the method added in Plan 01):

```rust
/// Emit a warning for each `[directory_overrides.<name>]` entry whose `<name>`
/// does not match any key in `self.directories`. Caller-supplied `warn` closure
/// receives the formatted message body (without the `warning:` prefix), so the
/// caller decides whether to `eprintln!`, push to a Vec, or do something else.
///
/// Used by `Config::load_with_overrides` to surface PORT-03 typo guards.
/// Mirrors `lib.rs::warn_unknown_disabled_directories` (which handles the same
/// typo case for `disabled_directories`).
///
/// **Order:** call this BEFORE `apply_machine_overrides` so the user sees
/// warnings about typos even if the apply step never touches them. (Apply is
/// silent for unknown targets — see Plan 01.)
pub(crate) fn warn_unknown_overrides(
    &self,
    prefs: &crate::machine::MachinePrefs,
    mut warn: impl FnMut(String),
) {
    for name in prefs.directory_overrides.keys() {
        if !self.directories.contains_key(name.as_str()) {
            warn(format!(
                "directory_overrides target '{name}' in machine.toml does not match any configured directory"
            ));
        }
    }
}
```

**Implementation notes:**
- `prefs.directory_overrides` is `BTreeMap<DirectoryName, DirectoryOverride>` (Plan 01 exact shape) — iteration is alphabetical and deterministic.
- The warning string format MUST contain `"directory_overrides target '<name>' in machine.toml"` so it's structurally similar to (but distinguishable from) the existing `warn_unknown_disabled_directories` line `"disabled directory '<name>' in machine.toml does not match any configured directory"`. Tests assert on the keyword set, not exact wording — this gives us room to tweak phrasing later.
- Visibility: `pub(crate)` — only `load_with_overrides` (same module) needs to call it.

Add the 5 unit tests inside the existing `#[cfg(test)] mod tests` block of `config.rs`. Test pattern (use a captured `Vec<String>` for assertions):
```rust
#[test]
fn warn_unknown_overrides_unknown_target_emits_one_warning() {
    let mut config = Config::default();
    config.directories.insert(
        DirectoryName::new("real").unwrap(),
        DirectoryConfig { /* ... minimal */ },
    );
    let mut prefs = crate::machine::MachinePrefs::default();
    prefs.directory_overrides.insert(
        DirectoryName::new("claud").unwrap(),
        crate::machine::DirectoryOverride { path: PathBuf::from("/p") },
    );
    let mut warnings: Vec<String> = Vec::new();
    config.warn_unknown_overrides(&prefs, |w| warnings.push(w));
    assert_eq!(warnings.len(), 1);
    assert!(warnings[0].contains("claud"));
    assert!(warnings[0].contains("machine.toml"));
    assert!(warnings[0].contains("directory_overrides") || warnings[0].contains("override"));
}
```

Run: `cargo test -p tome --lib config::tests::warn_unknown_overrides`
  </action>
  <verify>
    <automated>cargo test -p tome --lib config::tests::warn_unknown_overrides 2>&1 | tail -15</automated>
  </verify>
  <acceptance_criteria>
    - `rg -n "pub\\(crate\\) fn warn_unknown_overrides" crates/tome/src/config.rs` returns exactly 1 match.
    - `cargo test -p tome --lib config::tests::warn_unknown_overrides` runs ≥ 5 tests, all pass.
    - `cargo clippy -p tome --all-targets -- -D warnings` is clean.
    - The helper does NOT mutate `self` (`rg -n "fn warn_unknown_overrides\\(\\s*&mut self" crates/tome/src/config.rs` returns 0 matches).
  </acceptance_criteria>
  <done>
    `Config::warn_unknown_overrides` exists with `&self` + `impl FnMut(String)` callback, 5 unit tests cover empty/known/unknown/multiple/no-mutation cases, clippy is clean.
  </done>
</task>

<task type="auto" tdd="true">
  <name>Task 2: Wrap override-induced validation failures with `OverrideValidationError` shape</name>
  <files>crates/tome/src/config.rs</files>
  <read_first>
    - crates/tome/src/config.rs lines 277–300 (existing `Config::load`)
    - crates/tome/src/config.rs (the `load_with_overrides` method added in Plan 01)
    - crates/tome/src/config.rs lines 351–492 (`Config::validate` — the bail messages we're wrapping)
  </read_first>
  <behavior>
    - Test 1 (`load_with_overrides_pre_override_invalid_returns_raw_error`): `tome.toml` has `library_dir = /foo`, `directories.work.path = /foo` (overlap, invalid in v0.8.1). MachinePrefs has empty `directory_overrides`. `load_with_overrides` returns Err whose message contains the existing `library_dir overlaps distribution directory 'work'` text — NOT wrapped. (Pre-override invalid stays raw.)
    - Test 2 (`load_with_overrides_override_induces_invalid_returns_wrapped_error`): `tome.toml` has `library_dir = /lib`, `directories.work.path = /work` (valid). MachinePrefs has `directory_overrides.work.path = /lib` (overrides into the library). `load_with_overrides` returns Err whose message contains: (a) `"machine.toml"`, (b) `"directory_overrides"` or the literal override path `/lib`, (c) the original `library_dir overlaps` text from `validate()`, AND (d) does NOT direct the user to edit `tome.toml`. Specifically: assert message contains `"machine.toml"` AND does not contain `"edit tome.toml"`.
    - Test 3 (`load_with_overrides_override_unrelated_to_failure_returns_raw_error`): `tome.toml` has `library_dir = /lib`, `directories.work.path = /lib` (overlap, invalid). MachinePrefs has `directory_overrides.unrelated.path = /elsewhere` (an override that doesn't exist as a target — typo). `load_with_overrides`: warns about unknown override target `unrelated`, then returns Err with the RAW `library_dir overlaps` error (no machine.toml wrapper), because removing the override would NOT fix the underlying tome.toml problem. (Discriminator: the wrapper applies only when the pre-override config validates AND the post-override config does not.)
    - Test 4 (`load_with_overrides_path_appears_in_wrapper_message`): The wrapped error message includes the override target name (`work`), the new path (`/lib`), AND the old (pre-override) path. Reason: the user needs to see what changed to debug.
  </behavior>
  <action>
**Step 1 — Update `Config::load_with_overrides` to take `machine_path: &Path`:**

Change the signature from Plan 01:
```rust
pub fn load_with_overrides(path: &Path, prefs: &MachinePrefs) -> Result<Self>
```
to:
```rust
pub fn load_with_overrides(
    path: &Path,
    machine_path: &Path,
    prefs: &MachinePrefs,
) -> Result<Self>
```

And update `load_or_default_with_overrides` similarly:
```rust
pub fn load_or_default_with_overrides(
    cli_path: Option<&Path>,
    machine_path: &Path,
    prefs: &MachinePrefs,
) -> Result<Self>
```

Then update the single call site in `lib.rs::run` (added by Plan 01) to pass `&machine_path`:
```rust
let config = Config::load_or_default_with_overrides(
    effective_config.as_deref(),
    &machine_path,
    &machine_prefs,
)?;
```

**Step 2 — Rewrite the body of `load_with_overrides` to add warnings + the wrapping branch:**

```rust
pub fn load_with_overrides(
    path: &Path,
    machine_path: &Path,
    prefs: &crate::machine::MachinePrefs,
) -> Result<Self> {
    // Parse TOML (or default if missing) — same as Config::load.
    let mut config = if path.exists() {
        let content = std::fs::read_to_string(path)
            .with_context(|| format!("failed to read {}", path.display()))?;
        toml::from_str::<Config>(&content).map_err(|e| {
            let mut msg = format!("failed to parse {}: {e}", path.display());
            if content.contains("[[sources]]") || content.contains("[targets.") {
                msg.push_str("\nhint: tome v0.6 replaced [[sources]] and [targets.*] with [directories.*]. See CHANGELOG.md for migration instructions.");
            }
            anyhow::anyhow!("{msg}")
        })?
    } else {
        Self::default()
    };

    config.expand_tildes()?;

    // PORT-03: warn about typos before applying.
    config.warn_unknown_overrides(prefs, |w| eprintln!("warning: {w}"));

    // Snapshot pre-override paths for the PORT-04 wrapper.
    // Only the directory paths matter for the diff; clone is fine for v0.9.
    let pre_override_paths: std::collections::BTreeMap<String, std::path::PathBuf> = config
        .directories
        .iter()
        .map(|(name, dir)| (name.as_str().to_string(), dir.path.clone()))
        .collect();

    config.apply_machine_overrides(prefs)?;

    // PORT-04: if validation fails, attribute the failure to machine.toml
    // ONLY when removing the overrides would have produced a valid config.
    // Otherwise, the underlying tome.toml is what's broken — pass the raw
    // error through.
    if let Err(post_err) = config.validate() {
        // Reconstruct the pre-override config (cheap — only paths changed).
        let mut pre_override_config = config.clone();
        for (name, dir) in pre_override_config.directories.iter_mut() {
            if let Some(orig) = pre_override_paths.get(name.as_str()) {
                dir.path = orig.clone();
                dir.override_applied = false;
            }
        }
        let pre_override_valid = pre_override_config.validate().is_ok();

        if pre_override_valid && config.directories.values().any(|d| d.override_applied) {
            return Err(format_override_validation_error(
                &post_err,
                &pre_override_paths,
                &config,
                machine_path,
            ));
        }
        return Err(post_err);
    }
    Ok(config)
}
```

**Step 3 — Add the formatter as a free function** (place above `load_with_overrides`, or after it — your choice):

```rust
/// Wrap a `Config::validate()` error that was caused by `[directory_overrides.*]`
/// rewriting paths into something invalid. Names `machine.toml` as the file to
/// edit (NOT `tome.toml`) and shows the pre-override vs post-override paths so
/// the user can see what changed.
///
/// Only called when:
///   - pre-override config validates,
///   - at least one override was applied,
///   - post-override config fails validation.
fn format_override_validation_error(
    post_err: &anyhow::Error,
    pre_override_paths: &std::collections::BTreeMap<String, std::path::PathBuf>,
    config: &Config,
    machine_path: &std::path::Path,
) -> anyhow::Error {
    let mut diff_lines = Vec::new();
    for (name, dir) in &config.directories {
        if dir.override_applied {
            let was = pre_override_paths
                .get(name.as_str())
                .map(|p| p.display().to_string())
                .unwrap_or_else(|| "<unknown>".to_string());
            diff_lines.push(format!(
                "  - {}: {} (was: {}, in tome.toml)",
                name,
                dir.path.display(),
                was,
            ));
        }
    }

    // Indent the original validate() error by 2 spaces so it visually nests
    // under our wrapper text. Multi-line errors stay readable.
    let indented = format!("{post_err:#}")
        .lines()
        .map(|l| format!("  {l}"))
        .collect::<Vec<_>>()
        .join("\n");

    anyhow::anyhow!(
        "override-induced config error from machine.toml\n\
         \n\
         The following directory paths come from `[directory_overrides.<name>]` overrides:\n\
         {}\n\
         \n\
         These overrides made an otherwise-valid `tome.toml` fail validation:\n\
         \n\
         {}\n\
         \n\
         To fix: edit `{}` (NOT tome.toml). Either remove the override(s) above \
         or change them to paths that don't conflict.",
        diff_lines.join("\n"),
        indented,
        machine_path.display(),
    )
}
```

**Implementation notes:**
- The wrapper does NOT change the `Result` type of `load_with_overrides` — it stays `Result<Self>` (anyhow::Error). The "distinct error class" PORT-04 calls for is achieved by **message content + structure** rather than a typed error variant. This matches the existing tome convention (everything is `anyhow::Result` with grep-able message conventions like "Conflict: ... Why: ... hint: ..."). If a future caller needs to programmatically detect override errors, we can add a marker line like `class: override-induced` or migrate to a typed error then. **Track this as a v1.0 follow-up issue if it comes up; do not introduce a typed error class in v0.9.**
- The discriminator (`pre_override_valid && any override_applied`) intentionally allows both conditions: if no overrides were applied, the wrapper isn't relevant. If pre-override config is also invalid, blaming machine.toml would be wrong.
- Indentation by `format!("{post_err:#}")` — using the `:#` formatter so anyhow's chained context shows up. Verify by writing one test that constructs a multi-line validate error and checks both lines appear in the wrapped message.
- The test in `Test 3` exercises the discriminator: pre-override invalid + override that doesn't fix it → raw error.

Add the 4 unit tests inside the `#[cfg(test)] mod tests` block. Use the existing `make_*` helpers and `tempfile::TempDir`. The library_dir/distribution-dir overlap is the cleanest validate failure to trigger; reuse the patterns from existing `validate_rejects_library_equals_distribution` test (~line 1536).

Run: `cargo test -p tome --lib config::tests::load_with_overrides`
  </action>
  <verify>
    <automated>cargo test -p tome --lib config::tests::load_with_overrides 2>&1 | tail -20 && cargo clippy -p tome --all-targets -- -D warnings 2>&1 | tail -5</automated>
  </verify>
  <acceptance_criteria>
    - `rg -n "fn format_override_validation_error" crates/tome/src/config.rs` returns exactly 1 match.
    - `rg -n "override-induced config error from machine.toml" crates/tome/src/config.rs` returns exactly 1 match.
    - `rg -n "load_or_default_with_overrides" crates/tome/src/lib.rs` shows the call site passing 3 args (path + machine_path + prefs).
    - `cargo test -p tome --lib config::tests::load_with_overrides_pre_override_invalid_returns_raw_error` passes.
    - `cargo test -p tome --lib config::tests::load_with_overrides_override_induces_invalid_returns_wrapped_error` passes.
    - `cargo test -p tome --lib config::tests::load_with_overrides_override_unrelated_to_failure_returns_raw_error` passes.
    - `cargo test -p tome --lib config::tests::load_with_overrides_path_appears_in_wrapper_message` passes.
    - `cargo test -p tome --lib config::tests::load_with_overrides_runs_in_order_expand_apply_validate` (from Plan 01) STILL passes — wrapper changes did not regress the order test.
    - `cargo clippy -p tome --all-targets -- -D warnings` is clean.
  </acceptance_criteria>
  <done>
    `load_with_overrides` and `load_or_default_with_overrides` take `machine_path: &Path`, run `warn_unknown_overrides` before apply, snapshot pre-override paths, and wrap post-override `validate()` errors with `format_override_validation_error` only when (pre-override valid AND override applied). 4 unit tests cover the matrix.
  </done>
</task>

<task type="auto" tdd="true">
  <name>Task 3: Integration tests — typo warning + override-induced validation error end-to-end</name>
  <files>crates/tome/tests/cli.rs</files>
  <read_first>
    - crates/tome/tests/cli.rs lines 1–80 (test infrastructure: `tome()` builder, `create_skill`, `TempDir`)
    - crates/tome/tests/cli.rs lines 300–340 (existing `--machine` flag tests for shape reference)
    - The smoke test `machine_override_rewrites_directory_path_for_status` (added in Plan 01, Task 3) — match its style
  </read_first>
  <action>
Add two integration tests to `crates/tome/tests/cli.rs`, immediately after `machine_override_rewrites_directory_path_for_status` (added in Plan 01).

**Test 1 — PORT-03 unknown target warning:**

```rust
#[cfg(unix)]
#[test]
fn machine_override_unknown_target_warns_and_continues() {
    // PORT-03: an override targeting a directory name not present in tome.toml
    // produces a stderr `warning:` line (typo guard) without aborting load.
    let tmp = TempDir::new().unwrap();
    let real_skills = tmp.path().join("real-skills");
    create_skill(&real_skills, "x");

    let tome_toml = format!(
        "library_dir = \"{}/library\"\n\
         \n\
         [directories.work]\n\
         path = \"{}\"\n\
         type = \"directory\"\n\
         role = \"source\"\n",
        tmp.path().display(),
        real_skills.display(),
    );
    std::fs::write(tmp.path().join("tome.toml"), tome_toml).unwrap();

    // Override target `claud` is a typo — `claude` doesn't exist either, but
    // that's fine: the warning fires for any unknown name.
    let machine_toml = format!(
        "[directory_overrides.claud]\npath = \"{}/elsewhere\"\n",
        tmp.path().display(),
    );
    let machine_path = tmp.path().join("machine.toml");
    std::fs::write(&machine_path, machine_toml).unwrap();

    let assert = tome()
        .args([
            "--tome-home",
            tmp.path().to_str().unwrap(),
            "--machine",
            machine_path.to_str().unwrap(),
            "status",
        ])
        .env("NO_COLOR", "1")
        .assert()
        .success();   // <-- does NOT abort, only warns
    let stderr = String::from_utf8(assert.get_output().stderr.clone()).unwrap();
    assert!(
        stderr.contains("warning:") && stderr.contains("claud") && stderr.contains("machine.toml"),
        "expected stderr warning naming 'claud' and 'machine.toml', got:\n{stderr}"
    );
}
```

**Test 2 — PORT-04 override-induced validation error:**

```rust
#[cfg(unix)]
#[test]
fn machine_override_validation_failure_blames_machine_toml() {
    // PORT-04: validation failures triggered by an override surface as a
    // distinct error class that names machine.toml (not tome.toml) as the
    // file to edit.
    let tmp = TempDir::new().unwrap();
    let library_dir = tmp.path().join("library");
    std::fs::create_dir_all(&library_dir).unwrap();

    // tome.toml is valid: library_dir and directories.work.path are disjoint.
    let work_dir = tmp.path().join("work-skills");
    std::fs::create_dir_all(&work_dir).unwrap();
    let tome_toml = format!(
        "library_dir = \"{}\"\n\
         \n\
         [directories.work]\n\
         path = \"{}\"\n\
         type = \"directory\"\n\
         role = \"synced\"\n",
        library_dir.display(),
        work_dir.display(),
    );
    std::fs::write(tmp.path().join("tome.toml"), tome_toml).unwrap();

    // machine.toml override forces directories.work.path == library_dir.
    // After apply_machine_overrides, validate() will fail with the existing
    // "library_dir overlaps distribution directory 'work'" error.
    let machine_toml = format!(
        "[directory_overrides.work]\npath = \"{}\"\n",
        library_dir.display(),
    );
    let machine_path = tmp.path().join("machine.toml");
    std::fs::write(&machine_path, machine_toml).unwrap();

    let assert = tome()
        .args([
            "--tome-home",
            tmp.path().to_str().unwrap(),
            "--machine",
            machine_path.to_str().unwrap(),
            "status",
        ])
        .env("NO_COLOR", "1")
        .assert()
        .failure();
    let stderr = String::from_utf8(assert.get_output().stderr.clone()).unwrap();

    // The wrapped error MUST mention machine.toml (so the user knows where to look)
    assert!(
        stderr.contains("machine.toml"),
        "expected stderr to name machine.toml, got:\n{stderr}"
    );
    // And include the original validate() error text (preserved inside the wrapper)
    assert!(
        stderr.contains("library_dir") && stderr.contains("overlaps"),
        "expected wrapped error to preserve the original validate() text, got:\n{stderr}"
    );
    // And reference the override-induced classification
    assert!(
        stderr.contains("override-induced") || stderr.contains("directory_overrides"),
        "expected wrapped error to identify itself as override-induced, got:\n{stderr}"
    );
    // And NOT direct the user to edit tome.toml (negative assertion — discriminator)
    assert!(
        !stderr.contains("edit tome.toml") && !stderr.contains("Edit tome.toml"),
        "wrapped error must NOT direct the user to edit tome.toml, got:\n{stderr}"
    );
}
```

**Implementation notes:**
- These tests share the `--tome-home` + `--machine` flag pattern with the Plan 01 smoke test. If `--machine` is not yet a direct CLI flag (it is — see `cli.rs:39 pub machine: Option<PathBuf>`), no plumbing changes needed.
- Both tests run `tome status` (read-only) — no skill consolidation, no symlink writes — so they're fast and don't need the full sync pipeline.
- The assertion in Test 2 about `"override-induced" || "directory_overrides"` matches the wrapper text from Task 2: `"override-induced config error from machine.toml"` and `"[directory_overrides.<name>]"` both appear. The `||` allows future wording tweaks without test churn.

Run: `cargo test -p tome --test cli machine_override_unknown_target machine_override_validation_failure`
  </action>
  <verify>
    <automated>cargo test -p tome --test cli machine_override_unknown_target_warns_and_continues machine_override_validation_failure_blames_machine_toml</automated>
  </verify>
  <acceptance_criteria>
    - `cargo test -p tome --test cli machine_override_unknown_target_warns_and_continues` passes.
    - `cargo test -p tome --test cli machine_override_validation_failure_blames_machine_toml` passes.
    - `make ci` passes (no regressions to existing 590+ tests).
  </acceptance_criteria>
  <done>
    Both PORT-03 and PORT-04 are pinned end-to-end by integration tests that exercise the full CLI binary against a real `tome.toml` + `machine.toml` fixture pair.
  </done>
</task>

</tasks>

<verification>
- `cargo test -p tome --lib config::tests::warn_unknown_overrides` — ≥ 5 tests pass.
- `cargo test -p tome --lib config::tests::load_with_overrides_pre_override_invalid_returns_raw_error` — passes.
- `cargo test -p tome --lib config::tests::load_with_overrides_override_induces_invalid_returns_wrapped_error` — passes.
- `cargo test -p tome --lib config::tests::load_with_overrides_override_unrelated_to_failure_returns_raw_error` — passes.
- `cargo test -p tome --lib config::tests::load_with_overrides_path_appears_in_wrapper_message` — passes.
- `cargo test -p tome --test cli machine_override_unknown_target_warns_and_continues` — passes.
- `cargo test -p tome --test cli machine_override_validation_failure_blames_machine_toml` — passes.
- `make ci` — clean (no regressions to existing test suite).
</verification>

<success_criteria>
- An override targeting an unknown directory name produces a stderr `warning:` line and load continues (PORT-03).
- An override that makes `Config::validate()` fail surfaces with a wrapper that names `machine.toml`, preserves the original validate text, shows pre/post override paths, and explicitly discourages editing `tome.toml` (PORT-04).
- The wrapper applies ONLY when pre-override config validates AND ≥ 1 override was applied — otherwise the raw `validate()` error passes through unchanged (correct discrimination).
- `Config::warn_unknown_overrides` is unit-testable via `impl FnMut(String)` callback.
- Integration tests pin both PORT-03 and PORT-04 end-to-end through the real CLI binary.
</success_criteria>

<output>
After completion, create `.planning/phases/09-cross-machine-path-overrides/09-02-SUMMARY.md` recording:
- New `Config::warn_unknown_overrides(&self, prefs, warn)` signature.
- Updated `Config::load_with_overrides` and `Config::load_or_default_with_overrides` signatures (now take `machine_path`).
- New `format_override_validation_error` formatter signature + the wrapper text template.
- Discriminator logic summary: wrapper applies iff (pre-override valid) && (≥ 1 override applied).
- Test names added (config.rs ≥ 5 + 4 = 9, tests/cli.rs = 2).
- One-line confirmation: PORT-03 + PORT-04 closed.
- A note for v1.0: if a future caller needs to programmatically detect override errors, consider migrating to a typed error variant (`OverrideValidationError` enum). For v0.9, the message-content approach is sufficient and matches existing anyhow conventions.
</output>
