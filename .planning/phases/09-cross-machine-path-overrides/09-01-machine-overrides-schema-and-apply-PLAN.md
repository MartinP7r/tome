---
phase: 09-cross-machine-path-overrides
plan: 01
type: execute
wave: 1
depends_on: []
files_modified:
  - crates/tome/src/machine.rs
  - crates/tome/src/config.rs
  - crates/tome/src/lib.rs
autonomous: true
requirements: [PORT-01, PORT-02]
issue: "https://github.com/MartinP7r/tome/issues/458"

must_haves:
  truths:
    - "User can declare `[directory_overrides.<name>] path = \"...\"` blocks in `~/.config/tome/machine.toml` and the resulting `MachinePrefs` parses without error"
    - "After config load, every `Config::directories[name].path` matches the override (when one was declared), with `~` already expanded — `tome sync` / `tome status` / `tome doctor` / `lockfile::generate` all see the merged result"
    - "Override application happens exactly once, after `Config::expand_tildes()` and before `Config::validate()` — no second code path can observe pre-override paths"
    - "When no overrides are declared in `machine.toml`, behavior is byte-identical to v0.8.1 (zero-cost path)"
  artifacts:
    - path: "crates/tome/src/machine.rs"
      provides: "`DirectoryOverride` struct (with `path: PathBuf`) and `MachinePrefs.directory_overrides: BTreeMap<DirectoryName, DirectoryOverride>` field, deserialized from `[directory_overrides.<name>]` TOML blocks"
      contains: "directory_overrides"
    - path: "crates/tome/src/config.rs"
      provides: "`Config::apply_machine_overrides(&mut self, prefs: &MachinePrefs)` method that mutates `self.directories[name].path` for each matching override and sets `override_applied = true` on each touched entry. `DirectoryConfig` gains a `#[serde(skip)] pub(crate) override_applied: bool` field. New `Config::load_with_overrides(path: &Path, prefs: &MachinePrefs) -> Result<Self>` that runs `expand_tildes → apply_machine_overrides → validate` in that order."
      contains: "apply_machine_overrides"
    - path: "crates/tome/src/lib.rs"
      provides: "Single canonical load path used by all non-Init commands: load `MachinePrefs` BEFORE `Config`, then call `Config::load_with_overrides(&effective_config_path, &machine_prefs)` instead of `Config::load_or_default`. Sync handler reuses the already-loaded `machine_prefs` instead of re-loading inside `sync()`."
      contains: "load_with_overrides"
  key_links:
    - from: "crates/tome/src/lib.rs run() (line ~282, the post-Init load block)"
      to: "config::Config::load_with_overrides"
      via: "called once per command invocation, with machine_prefs loaded immediately above it"
      pattern: "Config::load_with_overrides"
    - from: "crates/tome/src/config.rs Config::load_with_overrides"
      to: "Config::apply_machine_overrides"
      via: "invoked between expand_tildes() and validate()"
      pattern: "self\\.expand_tildes.*apply_machine_overrides.*self\\.validate"
    - from: "crates/tome/src/machine.rs MachinePrefs"
      to: "[directory_overrides.<name>] TOML table"
      via: "serde derive with #[serde(default)]"
      pattern: "directory_overrides"
---

<objective>
Add `[directory_overrides.<name>]` schema to `machine.toml` and thread it through the canonical config load path so every downstream command (`sync`, `status`, `doctor`, `lockfile::generate`) operates on the merged result. Override application happens exactly once, between `Config::expand_tildes()` and `Config::validate()`.

This is the foundation plan for Phase 9 — Plans 02 (validation surfacing) and 03 (status/doctor markers) build on the schema + `override_applied` flag introduced here.

**Closes:** PORT-01 (schema), PORT-02 (apply timing in load pipeline).

Purpose: Lets a single `tome.toml` checked into dotfiles be applied across machines with different filesystem layouts. The user adds machine-local path overrides to `~/.config/tome/machine.toml` (which is already excluded from dotfiles sync) without touching `tome.toml`.

Output: New schema in `machine.rs`, two new methods on `Config`, and a single-call-site rewrite in `lib.rs::run()` that swaps `Config::load_or_default` for the override-aware path.
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

@crates/tome/src/machine.rs
@crates/tome/src/config.rs
@crates/tome/src/lib.rs
@crates/tome/src/paths.rs
@crates/tome/src/lockfile.rs
@crates/tome/src/status.rs
@crates/tome/src/doctor.rs
@crates/tome/tests/cli.rs

<interfaces>
<!-- Key types and contracts the executor needs. Extracted from codebase. -->

From `crates/tome/src/machine.rs` (current shape — extend, do not break):
```rust
pub struct MachinePrefs {
    #[serde(default)] pub(crate) disabled: BTreeSet<SkillName>,
    #[serde(default)] pub(crate) disabled_directories: BTreeSet<DirectoryName>,
    #[serde(default)] pub(crate) directory: BTreeMap<DirectoryName, DirectoryPrefs>,
    // <-- add: pub(crate) directory_overrides: BTreeMap<DirectoryName, DirectoryOverride>,
}
pub fn load(path: &Path) -> Result<MachinePrefs>;       // already exists, reuse
pub fn default_machine_path() -> Result<PathBuf>;       // already exists
```

From `crates/tome/src/config.rs` (current shape — extend, do not break):
```rust
pub struct DirectoryConfig {
    pub path: PathBuf,
    #[serde(rename = "type", default)] pub directory_type: DirectoryType,
    #[serde(default)] pub(crate) role: Option<DirectoryRole>,
    #[serde(default, skip_serializing_if = "Option::is_none")] pub branch: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")] pub tag: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")] pub rev: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")] pub subdir: Option<String>,
    // <-- add: #[serde(skip)] pub(crate) override_applied: bool,
}

impl Config {
    pub fn load(path: &Path) -> Result<Self> { /* expand_tildes() then validate() */ }
    pub fn load_or_default(cli_path: Option<&Path>) -> Result<Self> { /* wraps load */ }
    pub(crate) fn expand_tildes(&mut self) -> Result<()>;
    pub fn validate(&self) -> Result<()>;
    pub fn save_checked(&self, path: &Path) -> Result<()>;  // expand → validate → roundtrip
}
```

From `crates/tome/src/lib.rs::run` (line 282, post-Init):
```rust
let config = Config::load_or_default(effective_config.as_deref())?;
config.validate()?;                                          // redundant — load already did this
let tome_home = resolve_tome_home(cli.tome_home.as_deref(), cli.config.as_deref())?;
let paths = TomePaths::new(tome_home, config.library_dir.clone())?;
```

From `crates/tome/src/lib.rs::sync` (line 836):
```rust
fn sync(config: &Config, paths: &TomePaths, opts: SyncOptions<'_>) -> Result<()> {
    // ...
    let machine_path = resolve_machine_path(machine_override)?;  // line 889
    let mut machine_prefs = machine::load(&machine_path)?;        // line 890
    // ...
}
```

After this plan, the load order in `run()` becomes:
```text
1. resolve_machine_path(cli.machine.as_deref())     // already exists
2. machine::load(&machine_path)                      // moved up from sync()
3. Config::load_with_overrides(effective_config_path, &machine_prefs)
   -> internally: read TOML → expand_tildes() → apply_machine_overrides(&prefs) → validate()
4. TomePaths::new(...)
5. dispatch to subcommand handler
```

The `sync()` function continues to receive `&Config`, but now also needs `&MachinePrefs` (instead of loading it itself). Pass it through `SyncOptions` or as a separate parameter — see Task 3 for the exact shape.

**Field naming:** the new MachinePrefs field is `directory_overrides` (snake_case, plural) to mirror the existing `disabled_directories` style and match the TOML key `[directory_overrides.<name>]`. Do NOT use `overrides` (too generic, could collide later).

**Future-extension shape:** the issue (#458) calls out `role`, `type`, `subdir` as future override fields. v0.9 ships ONLY `path`. Do NOT add `Option<...>` placeholders for unused fields — they would be dead code and serde would happily accept them in machine.toml without ever applying them. Add them in a future phase when there's a concrete user need. (YAGNI per Pragmatic Programmer.)
</interfaces>
</context>

<tasks>

<task type="auto" tdd="true">
  <name>Task 1: Add `DirectoryOverride` struct + `directory_overrides` field to `MachinePrefs`</name>
  <files>crates/tome/src/machine.rs</files>
  <read_first>
    - crates/tome/src/machine.rs (full file — 475 lines; mirror the `DirectoryPrefs` shape)
    - crates/tome/src/config.rs lines 18–95 (DirectoryName Borrow/Display impls — needed because the override map is keyed by `DirectoryName`)
  </read_first>
  <behavior>
    - Test 1 (`directory_overrides_default_empty`): `MachinePrefs::default().directory_overrides` is empty.
    - Test 2 (`directory_overrides_parses_from_toml`): TOML `[directory_overrides.claude-skills]\npath = "/work/skills"` parses into `prefs.directory_overrides["claude-skills"].path == PathBuf::from("/work/skills")`.
    - Test 3 (`directory_overrides_with_tilde_path_is_preserved_unexpanded`): TOML `[directory_overrides.x]\npath = "~/work/skills"` parses with `path == PathBuf::from("~/work/skills")` — tilde expansion is the responsibility of `Config::apply_machine_overrides` (Task 2), NOT of machine.rs deserialization. **The executor MUST include this comment in the test body** so the design intent is documented at the point of test:
      ```rust
      // serde::Deserialize for PathBuf treats `~` as a literal char; tilde
      // expansion is delayed to Config::apply_machine_overrides so override
      // paths follow the same expansion semantics as paths in tome.toml.
      ```
    - Test 4 (`directory_overrides_roundtrip`): Constructing a `MachinePrefs` with one override, serializing via `toml::to_string_pretty`, then parsing back yields equal `directory_overrides`.
    - Test 5 (`existing_machine_toml_without_overrides_still_parses`): A TOML string with only `disabled = ["x"]` parses with `directory_overrides` defaulting to empty (`#[serde(default)]` works).
    - Test 6 (`directory_overrides_save_skips_when_empty`): With no overrides set, `toml::to_string_pretty(&prefs)` does NOT emit a `[directory_overrides]` table heading. (Use `#[serde(skip_serializing_if = "BTreeMap::is_empty")]` so empty maps stay invisible in the on-disk file.)
    - Test 7 (`directory_overrides_unknown_extra_field_rejected`): TOML `[directory_overrides.x]\npath = "/p"\nbogus = "y"` fails to parse. (Use `#[serde(deny_unknown_fields)]` on `DirectoryOverride` so future-renamed fields don't silently swallow typos.)
  </behavior>
  <action>
Add the new struct and field to `crates/tome/src/machine.rs`. Place `DirectoryOverride` immediately above the `MachinePrefs` struct definition (around line 35) so it reads top-down: helpers → MachinePrefs → impl → load/save → tests.

```rust
/// Per-machine path override for a specific directory.
///
/// Allows a single `tome.toml` checked into dotfiles to be applied across
/// machines with different filesystem layouts. The override is applied at
/// config load time (between `Config::expand_tildes()` and `Config::validate()`)
/// so every downstream command operates on the merged result.
///
/// Schema (v0.9): only `path` is supported. Future versions may add
/// `role`/`type`/`subdir` overrides — track via #458 follow-ups.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DirectoryOverride {
    /// Replaces `directories.<name>.path` on this machine. Tilde-expansion
    /// happens in `Config::apply_machine_overrides`, not here.
    pub path: PathBuf,
}
```

Add the field to `MachinePrefs`:
```rust
pub struct MachinePrefs {
    // ... existing fields ...

    /// Per-machine path overrides for entries in `tome.toml::directories`.
    /// Keyed by directory name; only the `path` field is currently supported (PORT-01).
    /// See `Config::apply_machine_overrides` for the apply step.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub(crate) directory_overrides: BTreeMap<DirectoryName, DirectoryOverride>,
}
```

**Implementation notes:**
- `BTreeMap` is already imported from `std::collections` at the top of the file.
- `PathBuf` is already imported.
- Keep visibility `pub(crate)` to match `disabled` and `directory` — external API is via `Config::apply_machine_overrides` (Task 2), not direct field access.
- Do NOT extend `MachinePrefs::validate()` in this task — there's no machine.toml-internal invariant to check. Cross-validation against `Config.directories` happens in `Config::apply_machine_overrides` (Task 2).
- Do NOT add convenience methods like `is_overridden(name)` here — the only consumer is `Config::apply_machine_overrides`, which iterates the map directly.

Add the 7 unit tests inside the existing `#[cfg(test)] mod tests {}` at the bottom of `machine.rs`, after the `disabled_directories_toml_format` test (~line 460). Reuse the existing test patterns: `tempfile::TempDir`, `toml::from_str`, `toml::to_string_pretty`, direct field access via `pub(crate)` (the tests are in the same module).

Run: `cargo test -p tome machine::tests::directory_overrides`
  </action>
  <verify>
    <automated>cargo test -p tome machine::tests::directory_overrides_ -- --exact-skip false 2>&1 | tail -20</automated>
  </verify>
  <acceptance_criteria>
    - `rg -n "pub struct DirectoryOverride" crates/tome/src/machine.rs` returns exactly 1 match.
    - `rg -n "directory_overrides:" crates/tome/src/machine.rs` returns at least 1 match (the field declaration).
    - `cargo test -p tome machine::tests::directory_overrides` runs ≥ 7 tests, all pass.
    - `cargo clippy -p tome --all-targets -- -D warnings` is clean.
    - `rg -n "deny_unknown_fields" crates/tome/src/machine.rs` returns at least 1 match (on `DirectoryOverride`).
    - `rg -n "skip_serializing_if = \"BTreeMap::is_empty\"" crates/tome/src/machine.rs` returns at least 1 match.
  </acceptance_criteria>
  <done>
    `DirectoryOverride` struct exists, `MachinePrefs.directory_overrides` field exists with `#[serde(default)]` + `#[serde(skip_serializing_if = "BTreeMap::is_empty")]`, 7 unit tests pass, clippy is clean.
  </done>
</task>

<task type="auto" tdd="true">
  <name>Task 2: Add `Config::apply_machine_overrides` + `Config::load_with_overrides` + `override_applied` field</name>
  <files>crates/tome/src/config.rs</files>
  <read_first>
    - crates/tome/src/config.rs lines 196–235 (DirectoryConfig struct — add the new field here)
    - crates/tome/src/config.rs lines 273–520 (Config impl — load, expand_tildes, validate, save_checked)
    - crates/tome/src/config.rs lines 580–620 (`expand_tilde` free function — reused for override path expansion)
    - crates/tome/src/machine.rs (the new `DirectoryOverride` + `directory_overrides` field from Task 1)
  </read_first>
  <behavior>
    - Test 1 (`apply_machine_overrides_no_overrides_is_noop`): Config with one directory, MachinePrefs with empty `directory_overrides` → after apply, `directories[name].path` is unchanged AND `directories[name].override_applied == false`.
    - Test 2 (`apply_machine_overrides_replaces_path`): Config with `directories.x.path = /old`, MachinePrefs with `directory_overrides.x.path = /new` → after apply, `directories.x.path == /new` AND `directories.x.override_applied == true`.
    - Test 3 (`apply_machine_overrides_expands_tilde_in_override_path`): Config with `directories.x.path = /old`, MachinePrefs with `directory_overrides.x.path = ~/work` → after apply, `directories.x.path` starts with the resolved home dir (no leading `~`) AND `override_applied == true`.
    - Test 4 (`apply_machine_overrides_unknown_target_does_not_panic`): Config with `directories.x`, MachinePrefs with `directory_overrides.bogus.path = /p` (no matching directory) → apply returns `Ok(())`, `directories.x` is unchanged, `directories.x.override_applied == false`. (PORT-03 will add the warning emission in Plan 02; in this task, it's a silent no-op for unknown targets — verify via behavior, no warning string check.)
    - Test 5 (`apply_machine_overrides_idempotent`): Calling `apply_machine_overrides` twice in a row produces the same result as calling it once (no double-expansion of tilde, no flag flip).
    - Test 6 (`load_with_overrides_runs_in_order_expand_apply_validate`): Write a `tome.toml` with `directories.x.path = "~/old"` and a fake MachinePrefs with `directory_overrides.x.path = "~/new"`. `Config::load_with_overrides(path, &prefs)` returns Ok with `directories.x.path` resolved to `<home>/new` and `override_applied == true`. (Verifies the I2 invariant: override happens AFTER expand_tildes — `~` in the override path is expanded — and BEFORE validate.)
    - Test 7 (`load_with_overrides_validate_failure_propagates`): Config has `directories.x.role = "managed"` with `type = "directory"` (an invalid combo) — `load_with_overrides` returns Err with the existing role/type conflict message. (Verifies validate still runs after override apply.)
    - Test 8 (`save_checked_does_not_serialize_override_applied`): Build a Config in-memory with one directory whose `override_applied = true`, call `save_checked`, then read the resulting TOML — `override_applied` MUST NOT appear in the file. (Verifies `#[serde(skip)]` on the new field.)
    - Test 9 (`override_applied_field_starts_false_after_load`): Config with one directory, MachinePrefs with empty overrides — `load_with_overrides` produces `directories[x].override_applied == false`. (Default-initialized via `#[serde(skip)] + Default`.)
  </behavior>
  <action>
**Step 1 — Add `override_applied` field to `DirectoryConfig`** (line ~196 in `config.rs`):

```rust
pub struct DirectoryConfig {
    pub path: PathBuf,
    #[serde(rename = "type", default)] pub directory_type: DirectoryType,
    #[serde(default)] pub(crate) role: Option<DirectoryRole>,
    #[serde(default, skip_serializing_if = "Option::is_none")] pub branch: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")] pub tag: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")] pub rev: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")] pub subdir: Option<String>,

    /// True iff this directory's `path` was rewritten by a `[directory_overrides.<name>]`
    /// entry in `machine.toml` during config load. Set in `Config::apply_machine_overrides`.
    /// `#[serde(skip)]` ensures this never appears in `tome.toml` (it's machine-local state,
    /// not portable config). Default = `false`.
    #[serde(skip)]
    pub(crate) override_applied: bool,
}
```

**`Default` derive:** `DirectoryConfig` does NOT currently derive `Default` — bool defaults to false naturally via `#[serde(skip)]` (serde calls `Default::default()` to populate skipped fields). Verify the existing struct has `#[derive(...)]` line; if `Default` is not in the list, serde's `#[serde(skip)]` requires it. Add it. If adding `Default` triggers cascading requirements (e.g., `DirectoryType` must derive `Default` — check whether it already does at line 92), follow the chain. The existing `DirectoryType` already has a `#[derive(..., Default)]` (verify at line ~92) and defaults to `Directory`; if not, fix that too. The existing `DirectoryRole` is `Option<>` so its default is `None` automatically.

**Alternative path** (cleaner if the Default chain explodes): use `#[serde(skip, default)]` so serde uses `bool::default()` directly — this works without requiring `DirectoryConfig: Default`. Prefer this if available.

**Step 1.5 — Update all `DirectoryConfig` struct literal sites to include `override_applied: false`.**

Adding the new field (even with `#[serde(skip, default)]`) requires every direct struct-literal construction across the crate to set the field explicitly. Find them:

```bash
rg -n "DirectoryConfig \{" crates/tome/src/
```

Expected ~17 sites across `eject.rs`, `wizard.rs` (×5), `relocate.rs`, `status.rs` (×3), `doctor.rs` (×3), `distribute.rs`, `reassign.rs`. The two `lockfile.rs` constructors use `toml::from_str` and are immune. Update each by adding `override_applied: false,` as the last field. Run `cargo build -p tome` to verify all sites are updated before proceeding to Step 2.

**Step 2 — Add `apply_machine_overrides` method to `impl Config`** (place after `expand_tildes`, around line 522):

```rust
/// Apply per-machine path overrides from `[directory_overrides.<name>]` entries
/// in `machine.toml`. Mutates `self.directories[name].path` and sets
/// `override_applied = true` on each matched entry.
///
/// **Order constraint (I2 invariant):** Call this AFTER `expand_tildes()` and
/// BEFORE `validate()`. The single canonical caller is `Config::load_with_overrides`.
///
/// **Override path expansion:** the override's own `path` is tilde-expanded here
/// (mirrors what `expand_tildes` did to the original path), so `~/...` works in
/// `machine.toml` exactly as it does in `tome.toml`.
///
/// **Unknown override targets:** silently ignored at this layer. The Plan 02
/// follow-up (`Config::warn_unknown_overrides`) emits stderr warnings; we keep
/// them separate so this method stays infallible and side-effect-free apart
/// from mutating `self`.
///
/// **Idempotent:** safe to call multiple times — the override path is read
/// from `prefs`, not from `self`, and tilde expansion is itself idempotent
/// (already-absolute paths pass through unchanged).
pub(crate) fn apply_machine_overrides(
    &mut self,
    prefs: &crate::machine::MachinePrefs,
) -> Result<()> {
    for (name, override_) in &prefs.directory_overrides {
        if let Some(dir) = self.directories.get_mut(name.as_str()) {
            dir.path = expand_tilde(&override_.path)?;
            dir.override_applied = true;
        }
        // Unknown override targets: no-op here. PORT-03 (Plan 02) handles warnings.
    }
    Ok(())
}
```

**Step 3 — Add `Config::load_with_overrides`** (place immediately after `load_or_default`, around line 316):

```rust
/// Load config and apply per-machine path overrides in one shot.
///
/// **Order (I2 invariant — must not change):**
///   1. Read TOML from `path` (or build defaults if missing — same as `Config::load`)
///   2. `expand_tildes()` on the raw config
///   3. `apply_machine_overrides(prefs)` — rewrites paths per `[directory_overrides.<name>]`
///   4. `validate()` — sees the merged result, so any override that produces an
///      invalid config (e.g., overridden path overlaps `library_dir`) surfaces here
///
/// Plan 02 (PORT-04) wraps the validate step in a distinct error class so the
/// user knows to fix `machine.toml`, not `tome.toml`. This method intentionally
/// returns the raw `validate()` error for now — Plan 02 introduces the wrapping.
///
/// Used by `lib.rs::run()` for every non-Init command. `tome init` does NOT use
/// this path — the wizard runs against the bare `tome.toml` that the user is
/// about to write.
pub fn load_with_overrides(
    path: &Path,
    prefs: &crate::machine::MachinePrefs,
) -> Result<Self> {
    let mut config = if path.exists() {
        let content = std::fs::read_to_string(path)
            .with_context(|| format!("failed to read {}", path.display()))?;
        let config: Config = toml::from_str(&content).map_err(|e| {
            let mut msg = format!("failed to parse {}: {e}", path.display());
            if content.contains("[[sources]]") || content.contains("[targets.") {
                msg.push_str("\nhint: tome v0.6 replaced [[sources]] and [targets.*] with [directories.*]. See CHANGELOG.md for migration instructions.");
            }
            anyhow::anyhow!("{msg}")
        })?;
        config
    } else {
        Self::default()
    };

    config.expand_tildes()?;
    config.apply_machine_overrides(prefs)?;
    config.validate()?;
    Ok(config)
}
```

**Step 4 — Add a `load_or_default_with_overrides` wrapper for the `cli_path: Option<&Path>` shape** (mirroring `load_or_default`, placed immediately after the new `load_with_overrides`):

```rust
/// CLI-aware variant of `load_with_overrides`. See `load_or_default` for the
/// missing-file vs. missing-parent-dir semantics.
pub fn load_or_default_with_overrides(
    cli_path: Option<&Path>,
    prefs: &crate::machine::MachinePrefs,
) -> Result<Self> {
    let path = match cli_path {
        Some(p) => {
            if !p.exists() {
                let parent_exists = p.parent().is_some_and(|d| d.exists());
                anyhow::ensure!(parent_exists, "config file not found: {}", p.display());
            }
            p.to_path_buf()
        }
        None => default_config_path()?,
    };
    Self::load_with_overrides(&path, prefs)
}
```

**Implementation notes:**
- DO NOT modify `Config::load` or `Config::load_or_default`. Tests + the Init wizard still use them. Adding two new methods (additive) is the minimum-blast-radius path.
- `expand_tilde` (the free function at line ~580) is the same one used by `expand_tildes` — reuse it.
- The `DirectoryName` keys in `prefs.directory_overrides` are `DirectoryName`s; the keys in `self.directories` are also `DirectoryName`s; use `.get_mut(name.as_str())` because `BTreeMap` lookup borrows. (Verify against existing code patterns at lines ~360 and ~445.)
- `save_checked` (line ~533) currently does TOML round-trip equality. Adding `#[serde(skip)] override_applied` to `DirectoryConfig` MUST NOT break this — the field is invisible to serde, so the round-trip stays identical. Verify by running `cargo test -p tome --lib config::tests::save_checked_writes_valid_config_and_reloads_unchanged` (existing test, line 1752) after the change.

Add the 9 tests inside the existing `#[cfg(test)] mod tests` block of `config.rs`. Reuse existing test helpers (`make_*`, `TempDir`, etc.). For tests that need a `MachinePrefs`, build it directly:
```rust
use crate::machine::{DirectoryOverride, MachinePrefs};
let mut prefs = MachinePrefs::default();
prefs.directory_overrides.insert(
    DirectoryName::new("x").unwrap(),
    DirectoryOverride { path: PathBuf::from("/new") },
);
```

Run: `cargo test -p tome config::tests::apply_machine_overrides config::tests::load_with_overrides config::tests::override_applied`
  </action>
  <verify>
    <automated>cargo test -p tome --lib config::tests 2>&1 | tail -30 && cargo clippy -p tome --all-targets -- -D warnings 2>&1 | tail -5</automated>
  </verify>
  <acceptance_criteria>
    - `rg -n "pub\\(crate\\) fn apply_machine_overrides" crates/tome/src/config.rs` returns exactly 1 match.
    - `rg -n "pub fn load_with_overrides" crates/tome/src/config.rs` returns exactly 1 match.
    - `rg -n "pub fn load_or_default_with_overrides" crates/tome/src/config.rs` returns exactly 1 match.
    - `rg -n "override_applied" crates/tome/src/config.rs` returns at least 4 matches (field decl + apply method + at least 2 tests).
    - `rg -n "#\\[serde\\(skip" crates/tome/src/config.rs | rg "override_applied"` matches the field annotation. (Use either `#[serde(skip)]` or `#[serde(skip, default)]` per the action's "Alternative path" note.)
    - `cargo test -p tome --lib config::tests::apply_machine_overrides_` runs ≥ 5 tests, all pass.
    - `cargo test -p tome --lib config::tests::load_with_overrides_` runs ≥ 2 tests, all pass.
    - `cargo test -p tome --lib config::tests::save_checked_writes_valid_config_and_reloads_unchanged` still passes (regression — `#[serde(skip)]` must not break the round-trip).
    - `cargo clippy -p tome --all-targets -- -D warnings` is clean.
    - After all sites are updated (Step 1.5), `cargo build -p tome` is clean (zero "missing field `override_applied`" errors).
    - `rg -c "DirectoryConfig \{" crates/tome/src/` matches the count taken before the change (i.e., no struct-literal construction sites were accidentally rewritten or removed during Step 1.5).
  </acceptance_criteria>
  <done>
    `DirectoryConfig.override_applied` field exists with `#[serde(skip)]`, `Config::apply_machine_overrides` mutates path + flag in one pass, `Config::load_with_overrides` chains expand → apply → validate, and `Config::load_or_default_with_overrides` wraps the CLI-path shape. 9 unit tests pass, save_checked round-trip regression test still passes.
  </done>
</task>

<task type="auto" tdd="true">
  <name>Task 3: Wire `load_with_overrides` into the canonical load path in `lib.rs::run`</name>
  <files>crates/tome/src/lib.rs</files>
  <read_first>
    - crates/tome/src/lib.rs lines 90–100 (`resolve_machine_path` helper — already exists, reuse)
    - crates/tome/src/lib.rs lines 154–290 (`run()` — the load block that needs editing is at line ~282)
    - crates/tome/src/lib.rs lines 690–700 (`SyncOptions` struct definition)
    - crates/tome/src/lib.rs lines 836–895 (`sync()` — currently loads MachinePrefs internally at line 889; it'll receive prefs from caller now)
    - crates/tome/src/config.rs (the new `load_or_default_with_overrides` method from Task 2)
  </read_first>
  <action>
**Step 1 — Replace the post-Init load block in `run()`** (`lib.rs` line 282–285):

Find this block:
```rust
let config = Config::load_or_default(effective_config.as_deref())?;
config.validate()?;
let tome_home = resolve_tome_home(cli.tome_home.as_deref(), cli.config.as_deref())?;
let paths = TomePaths::new(tome_home, config.library_dir.clone())?;
```

Replace with:
```rust
// Load per-machine preferences first — they may rewrite directory paths via
// `[directory_overrides.<name>]` entries, which `Config::load_with_overrides`
// applies between `expand_tildes()` and `validate()` (PORT-02 / I2 invariant).
let machine_path = resolve_machine_path(cli.machine.as_deref())?;
let machine_prefs = machine::load(&machine_path)?;

let config =
    Config::load_or_default_with_overrides(effective_config.as_deref(), &machine_prefs)?;
// Note: `load_or_default_with_overrides` already runs `validate()` internally —
// no separate `config.validate()?` call here (was redundant in the old code too,
// since `Config::load` also called `validate`).
let tome_home = resolve_tome_home(cli.tome_home.as_deref(), cli.config.as_deref())?;
let paths = TomePaths::new(tome_home, config.library_dir.clone())?;
```

**Step 2 — Update `SyncOptions` to carry both the path and the loaded prefs.**

`run()` loads `MachinePrefs` once at the top, so `sync()` no longer needs to re-load. Both fields are required: `machine_prefs` for read-only usage during sync, `machine_path` for the `machine::save(&machine_prefs, &machine_path)?` call at the end of the triage block (line ~966).

In `SyncOptions` (line ~691), replace `machine_override: Option<&'a Path>` with two fields:

```rust
struct SyncOptions<'a> {
    dry_run: bool,
    force: bool,
    no_triage: bool,
    no_input: bool,
    verbose: bool,
    quiet: bool,
    machine_path: &'a Path,
    machine_prefs: &'a machine::MachinePrefs,
}
```

In `sync()` (line ~836), update the destructure and remove the inline `resolve_machine_path` + `machine::load` calls (currently at lines ~889–890):

```rust
let SyncOptions {
    dry_run, force, no_triage, no_input, verbose, quiet,
    machine_path, machine_prefs: prefs_in,
} = opts;
let mut machine_prefs = prefs_in.clone();   // clone so triage can mutate locally
```

Update the two `SyncOptions { ... }` constructors at lines ~268 and ~313 to pass `machine_path: &machine_path, machine_prefs: &machine_prefs` instead of `machine_override`.

**Why both fields:** The `machine_path` is needed for `machine::save` after triage; the `machine_prefs` (already loaded at `run()` entry) avoids a redundant `machine::load` inside `sync()`. Loading once at the top guarantees the override-apply step in `Config::load_with_overrides` and the disabled-skill filtering inside `sync()` see identical prefs.

**Step 3 — Init handler**. The `Command::Init` branch (line ~247) calls `sync(...)` after wizard run. It currently passes `machine_override: cli.machine.as_deref()`. Update it to load `machine_prefs` once at the top of the Init branch:
```rust
// Inside `if matches!(cli.command, Command::Init)` block, before the sync call:
let machine_path = resolve_machine_path(cli.machine.as_deref())?;
let machine_prefs = machine::load(&machine_path)?;

// ... existing wizard code ...

if !cli.dry_run {
    // ... existing expanded config setup ...
    sync(
        &expanded,
        &paths,
        SyncOptions {
            dry_run: cli.dry_run,
            force: false,
            no_triage: true,
            no_input: cli.no_input,
            verbose: cli.verbose,
            quiet: cli.quiet,
            machine_path: &machine_path,
            machine_prefs: &machine_prefs,
        },
    )?;
}
```

**Important — the Init pre-load probe at line 163:**
```rust
if let Err(e) = Config::load_or_default(effective_config.as_deref()) {
    eprintln!("warning: existing config is malformed (...)", e);
}
```
Leave this unchanged. It probes whether the existing file parses; overrides aren't relevant to malformed-detection, and applying them would mask schema errors the wizard wants to surface.

**Step 4 — Backup command path** (line ~610): `let new_config = Config::load(&config_path)?;`. This is read inside backup-restore for a snapshot path. Leave it on plain `Config::load` — backup snapshots are restored verbatim and overrides are a runtime concern, not a snapshot concern. Add a one-line code comment noting why:
```rust
// Use plain Config::load (no overrides) — backup restore reads a snapshot exactly as written.
let new_config = Config::load(&config_path)?;
```

**Step 5 — Run the existing CLI integration test suite** to confirm nothing regressed:
```bash
cargo build -p tome
cargo test -p tome --lib
cargo test -p tome --test cli
```

Known fragile integration tests to watch: any test that constructs `SyncOptions` directly (search with `rg -n "machine_override:" crates/tome/`). After this change, those tests must use `machine_path` + `machine_prefs` instead. Update mechanically — there should be zero non-`lib.rs` `SyncOptions` constructions because `SyncOptions` is `struct` (not `pub`) with `pub(crate)`-or-tighter visibility (verify before starting).

**Add a smoke test** in `crates/tome/tests/cli.rs` that proves the wiring works end-to-end:

```rust
#[cfg(unix)]
#[test]
fn machine_override_rewrites_directory_path_for_status() {
    // PORT-01 + PORT-02 smoke: declare an override in machine.toml and
    // confirm `tome status --json` reports the OVERRIDDEN path, proving
    // the load pipeline applied the override before status::gather ran.
    let tmp = TempDir::new().unwrap();
    let real_skills = tmp.path().join("real-skills");
    create_skill(&real_skills, "x");

    // tome.toml points at a path that does NOT exist.
    let tome_toml = format!(
        "library_dir = \"{}/library\"\n\
         \n\
         [directories.work]\n\
         path = \"{}/does-not-exist\"\n\
         type = \"directory\"\n\
         role = \"source\"\n",
        tmp.path().display(),
        tmp.path().display(),
    );
    std::fs::write(tmp.path().join("tome.toml"), tome_toml).unwrap();

    // machine.toml overrides directories.work.path to the real path.
    let machine_toml = format!(
        "[directory_overrides.work]\npath = \"{}\"\n",
        real_skills.display(),
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
            "--json",
        ])
        .env("NO_COLOR", "1")
        .assert()
        .success();
    let stdout = String::from_utf8(assert.get_output().stdout.clone()).unwrap();
    let report: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    let dirs = report["directories"].as_array().unwrap();
    let work = dirs.iter().find(|d| d["name"] == "work").unwrap();
    let path = work["path"].as_str().unwrap();
    assert!(
        path.contains("real-skills"),
        "expected status to report overridden path, got: {path}"
    );
}
```

Run: `cargo test -p tome --test cli machine_override_rewrites_directory_path_for_status`
  </action>
  <verify>
    <automated>cargo build -p tome 2>&1 | tail -10 && cargo test -p tome --lib 2>&1 | tail -5 && cargo test -p tome --test cli machine_override_rewrites_directory_path_for_status</automated>
  </verify>
  <acceptance_criteria>
    - `rg -n "Config::load_or_default_with_overrides" crates/tome/src/lib.rs` returns at least 1 match.
    - `rg -n "Config::load_or_default\\(" crates/tome/src/lib.rs` returns exactly 1 match (the Init pre-load probe at line ~163; the post-Init call site at line ~282 is gone).
    - `rg -n "machine_override:" crates/tome/src/lib.rs` returns 0 matches (replaced by `machine_path` + `machine_prefs`).
    - `rg -n "machine_path:|machine_prefs:" crates/tome/src/lib.rs` returns at least 4 matches (struct decl + 2 sync call sites + sync body destructure).
    - `cargo build -p tome` is clean.
    - `cargo test -p tome --lib` passes (existing 464+ unit tests; no regressions).
    - `cargo test -p tome --test cli` passes (existing integration tests + the new smoke test).
    - `cargo clippy -p tome --all-targets -- -D warnings` is clean.
    - `make ci` passes.
  </acceptance_criteria>
  <done>
    `run()` loads `MachinePrefs` once, threads it through `Config::load_or_default_with_overrides` and `SyncOptions` to `sync()`. The Init handler also loads prefs at the top of its branch. The `machine_override` field on `SyncOptions` is gone. The new smoke integration test proves end-to-end that an override declared in `machine.toml` reaches `tome status --json` output.
  </done>
</task>

</tasks>

<verification>
- `cargo test -p tome machine::tests::directory_overrides` — ≥ 7 tests pass.
- `cargo test -p tome --lib config::tests::apply_machine_overrides_` — ≥ 5 tests pass.
- `cargo test -p tome --lib config::tests::load_with_overrides_` — ≥ 2 tests pass.
- `cargo test -p tome --lib config::tests::save_checked_writes_valid_config_and_reloads_unchanged` — passes (regression guard for `#[serde(skip)]`).
- `cargo test -p tome --test cli machine_override_rewrites_directory_path_for_status` — passes (end-to-end I2 smoke test).
- `make ci` — clean.
- `rg -n "Config::load_or_default\\(" crates/tome/src/lib.rs` — exactly 1 match (Init malformed-config probe only).
- `rg -n "machine_override:" crates/tome/src/lib.rs` — 0 matches.
</verification>

<success_criteria>
- `DirectoryOverride` struct + `MachinePrefs.directory_overrides` parsed from `[directory_overrides.<name>]` TOML blocks (PORT-01).
- `Config::apply_machine_overrides` mutates `directories[name].path` + sets `override_applied = true`, called between `expand_tildes()` and `validate()` (PORT-02 / I2 invariant).
- A single canonical load path (`Config::load_or_default_with_overrides`) is used by every non-Init command via `lib.rs::run()`. Sync, status, doctor, lockfile::generate all see the merged result without re-loading prefs.
- The Init pre-load probe (line ~163) and Init wizard run unchanged — overrides do not interfere with malformed-config detection or wizard prompts.
- `#[serde(skip)] override_applied` is invisible to `tome.toml` round-trips — `save_checked` byte-equality is preserved.
- End-to-end smoke test proves `tome status --json` reports the overridden path.
</success_criteria>

<output>
After completion, create `.planning/phases/09-cross-machine-path-overrides/09-01-SUMMARY.md` recording:
- New `DirectoryOverride` struct + `MachinePrefs.directory_overrides` field signatures.
- New `Config::apply_machine_overrides` + `Config::load_with_overrides` + `Config::load_or_default_with_overrides` signatures.
- New `DirectoryConfig.override_applied` field (with `#[serde(skip)]`).
- The exact `lib.rs::run()` line range that was rewritten.
- Test names added (machine.rs ≥ 7, config.rs ≥ 9, tests/cli.rs = 1).
- One-line confirmation: PORT-01 + PORT-02 closed.
- Any deviations from the plan (e.g., if `Default` chain expansion was needed on `DirectoryType`).
</output>
