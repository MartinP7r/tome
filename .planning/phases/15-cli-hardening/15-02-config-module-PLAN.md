---
phase: 15-cli-hardening
plan: 02
type: execute
wave: 1
depends_on: []
files_modified:
  - crates/tome/src/config.rs
  - crates/tome/src/config/mod.rs
  - crates/tome/src/config/types.rs
  - crates/tome/src/config/overrides.rs
  - crates/tome/src/config/validate.rs
  - crates/tome/src/paths.rs
  - crates/tome/src/lib.rs
autonomous: true
requirements:
  - HARD-03
  - HARD-22
must_haves:
  truths:
    - "config.rs as a single file is gone; the public Config API is now served from crates/tome/src/config/mod.rs which re-exports from types.rs / overrides.rs / validate.rs"
    - "Config::save_checked lives in crates/tome/src/config/mod.rs (not types.rs) — locked landing site so Plan 15-04 Task 2 has a deterministic grep target"
    - "Config::save_checked writes ~-shape paths for paths under $HOME and absolute paths otherwise; the original input is preserved when already ~-shape"
    - "MachinePrefs::save preserves user-supplied paths verbatim — no tilde rewriting on machine.toml"
    - "Override paths from machine.toml are NEVER serialised back into tome.toml (Phase 9 PORT-02 invariant preserved)"
  artifacts:
    - path: "crates/tome/src/config/mod.rs"
      provides: "Public Config API re-exports + Config::save_checked impl"
      contains: "pub fn save_checked"
    - path: "crates/tome/src/config/types.rs"
      provides: "Config, DirectoryName, DirectoryConfig, DirectoryType, DirectoryRole type definitions"
      contains: "pub struct Config"
    - path: "crates/tome/src/config/overrides.rs"
      provides: "apply_machine_overrides function"
      contains: "apply_machine_overrides"
    - path: "crates/tome/src/config/validate.rs"
      provides: "Config::validate Cases A/B/C overlap detection"
      contains: "fn validate"
    - path: "crates/tome/src/paths.rs"
      provides: "expand_tilde + new unexpand_tilde helper"
      contains: "pub fn unexpand_tilde"
  key_links:
    - from: "crates/tome/src/config/mod.rs::Config::save_checked"
      to: "crates/tome/src/paths.rs::unexpand_tilde"
      via: "serialise-time tilde normalisation"
      pattern: "unexpand_tilde"
    - from: "crates/tome/src/config/mod.rs"
      to: "config/{types,overrides,validate}"
      via: "mod + pub use re-exports"
      pattern: "(mod types|mod overrides|mod validate|pub use)"
---

<objective>
Split the 3,122 LOC `crates/tome/src/config.rs` into `crates/tome/src/config/{mod,types,overrides,validate}.rs` (HARD-03, closes #487) and rewrite `Config::save_checked` to preserve `~`-shape paths via a new `paths::unexpand_tilde()` helper, with auto-portable normalisation for paths under `$HOME` (HARD-22, closes #457).

Purpose: Make `config.rs` reviewable and unblock the dotfiles workflow — committing `tome.toml` to a git repo no longer rewrites `~/skills` to `/Users/martin/skills` on every save.
Output: New `config/` module with re-exported public API; `paths::unexpand_tilde` helper; updated `Config::save_checked` (landed in `config/mod.rs`) that operates on the unmutated config and rewrites paths to `~`-shape at serialise time.
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

@crates/tome/src/config.rs
@crates/tome/src/paths.rs
@crates/tome/src/machine.rs

<interfaces>
<!-- Existing config.rs surface (must be preserved post-split). Extracted from crates/tome/src/config.rs. -->

```rust
// Public API the rest of the crate consumes — preserve every name through the
// new config/mod.rs.

pub struct Config { pub directories: BTreeMap<DirectoryName, DirectoryConfig>, pub library_dir: PathBuf, pub tome_home: PathBuf, ... }

impl Config {
    pub fn load(path: &Path) -> anyhow::Result<Self>
    pub fn load_or_default(path: &Path) -> anyhow::Result<Self>
    pub fn save_checked(&self, path: &Path) -> anyhow::Result<()>  // ← line 832 — HARD-22 rewrite target; LANDS IN config/mod.rs
    pub fn validate(&self) -> anyhow::Result<()>                    // ← Cases A/B/C overlap detection → validate.rs
    pub fn apply_machine_overrides(&mut self, prefs: &MachinePrefs) // ← line 663+ → overrides.rs
    pub fn expand_tildes(&mut self)                                  // ← stays in paths.rs as helper consumer
}

pub struct DirectoryName(String);   // newtype + transparent serde
pub struct DirectoryConfig { ... }
pub enum DirectoryType { Directory, ClaudePlugins, Git, ... }
pub enum DirectoryRole { Discovery, Synced, Target, Managed, ... }
```

From crates/tome/src/paths.rs (existing):
```rust
pub fn expand_tilde(p: &Path) -> PathBuf;  // resolves ~/foo → /Users/martin/foo
pub struct TomePaths { ... }
```

From crates/tome/src/machine.rs (Phase 9 PORT-02):
```rust
// apply_machine_overrides mutates a load-time-only copy of Config.
// save_checked operates on the unmutated config — overrides MUST NOT round-trip
// through tome.toml.
pub struct MachinePrefs { ... }
pub fn directory_overrides(&self) -> &BTreeMap<DirectoryName, DirectoryOverride>;
```
</interfaces>
</context>

<tasks>

<task type="auto">
  <name>Task 1: Split config.rs into config/{mod,types,overrides,validate}.rs (HARD-03)</name>
  <files>crates/tome/src/config.rs, crates/tome/src/config/mod.rs, crates/tome/src/config/types.rs, crates/tome/src/config/overrides.rs, crates/tome/src/config/validate.rs, crates/tome/src/lib.rs</files>
  <read_first>
    - crates/tome/src/config.rs (current 3,122 LOC — read fully to map functions to target files)
    - crates/tome/src/lib.rs (find `mod config;` declaration; preserve the lib.rs-level visibility)
    - .planning/phases/15-cli-hardening/15-CONTEXT.md §"HARD-03 config.rs internal layout" (Claude's Discretion: tilde helpers stay in paths.rs)
    - .planning/REQUIREMENTS.md §"HARD-03"
  </read_first>
  <action>
    Move `crates/tome/src/config.rs` (3,122 LOC) into a `crates/tome/src/config/` module directory with the layout below. Per CONTEXT.md "Claude's Discretion": **tilde helpers (`expand_tilde`, the new `unexpand_tilde`) stay in `paths.rs`** — they are cross-cutting utilities, not config-specific.

    Target layout:

    ```
    crates/tome/src/config/
      mod.rs        ← public API surface; declares submodules; re-exports types; HOSTS Config::load + Config::save_checked
      types.rs      ← Config struct definition, DirectoryName, DirectoryConfig, DirectoryType, DirectoryRole + their impls (Display, AsRef, serde Deserialize, etc.)
      overrides.rs  ← apply_machine_overrides function and any private helpers it uses
      validate.rs   ← Config::validate (Cases A/B/C overlap detection); private helpers
    ```

    Steps:

    1. **Delete** `crates/tome/src/config.rs` after copying its contents into the new module files.

    2. **`config/mod.rs`** — declares `mod types; mod overrides; mod validate;` and re-exports the public surface used by the rest of the crate. Use `pub use types::{Config, DirectoryName, DirectoryConfig, DirectoryType, DirectoryRole};` for the names that already are `pub` in current `config.rs`.

       **LOCKED landing site (per S3 revision):** `Config::load`, `Config::load_or_default`, and `Config::save_checked` are implemented as `impl Config { ... }` blocks INSIDE `config/mod.rs` (not `types.rs`). Rationale: `save_checked` depends on `validate.rs`'s `Config::validate` and on the `apply_machine_overrides` invariant from `overrides.rs`; co-locating these top-level lifecycle methods in `mod.rs` keeps the dependency direction clear (mod.rs depends on submodules, not vice versa) and gives Plan 15-04 Task 2 (HARD-08 atomic-save regression) a deterministic grep target.

    3. **`config/types.rs`** — type definitions only: `Config` struct (fields), `DirectoryName` (newtype + transparent serde + custom Deserialize), `DirectoryConfig`, `DirectoryType` enum, `DirectoryRole` enum. Includes derive impls (`Debug`, `Clone`, `Serialize`, `Deserialize`) and any trait impls (`Display`, `AsRef`, `Borrow`, `TryFrom<String>` is added in Plan 15-03 not here). Do NOT include `Config::load`, `Config::save_checked`, `Config::validate`, or `Config::apply_machine_overrides` here — those land in `mod.rs` / `validate.rs` / `overrides.rs` respectively.

    4. **`config/overrides.rs`** — extract `Config::apply_machine_overrides` (line 663+ in current config.rs) plus any private helpers it uses (e.g. typo-warning emission from PORT-02). Make it `impl Config { pub fn apply_machine_overrides(&mut self, ...) { ... } }` in the new file.

    5. **`config/validate.rs`** — extract `Config::validate` plus the Cases A/B/C overlap detection logic (Phase 4 WHARD-01). Same `impl Config` shape.

    6. **`lib.rs`** — change `mod config;` declaration to ensure the module path resolves to `config/mod.rs` (Rust auto-resolves `mod config;` to either `config.rs` or `config/mod.rs`). Verify no stale references to `crate::config` from sibling modules need adjusting (the public surface should be byte-identical from consumers' perspective).

    7. Run `cargo check -p tome`, `cargo clippy --all-targets -- -D warnings`, and `cargo test -p tome` — no test count change expected.
  </action>
  <verify>
    <automated>cargo build -p tome &amp;&amp; cargo clippy --all-targets -- -D warnings &amp;&amp; cargo test -p tome 2>&amp;1 | tee /tmp/15-02-task1.log</automated>
  </verify>
  <acceptance_criteria>
    - `fd '^config\.rs$' crates/tome/src --max-depth 2` returns nothing (no top-level `crates/tome/src/config.rs`).
    - `fd '^(mod|types|overrides|validate)\.rs$' crates/tome/src/config` returns at least 4 files: `mod.rs`, `types.rs`, `overrides.rs`, `validate.rs`.
    - `grep -l "pub fn apply_machine_overrides" crates/tome/src/config/overrides.rs` finds the symbol in `overrides.rs` (and not in `types.rs` or `mod.rs`).
    - `grep -l "fn validate" crates/tome/src/config/validate.rs` finds the symbol in `validate.rs`.
    - `grep "pub use" crates/tome/src/config/mod.rs` re-exports at least `Config`, `DirectoryName`, `DirectoryConfig`, `DirectoryType`, `DirectoryRole`.
    - **S3 lock:** `grep -E "pub fn save_checked" crates/tome/src/config/mod.rs` returns ≥1 match (Config::save_checked LIVES in mod.rs).
    - **S3 lock:** `grep -E "pub fn save_checked" crates/tome/src/config/types.rs` returns 0 matches (NOT in types.rs).
    - `cargo build -p tome` exits 0.
    - `cargo clippy --all-targets -- -D warnings` exits 0.
    - `cargo test -p tome` passes; baseline test count preserved.
    - `rg "use crate::config::Config" crates/tome/src` still resolves (consumers see byte-identical public API).
  </acceptance_criteria>
  <done>
    `config.rs` is gone; `config/` module exists with the four-file split. `Config::save_checked` lives in `config/mod.rs`. Public API surface is byte-identical to consumers. Tests pass; clippy is clean.
  </done>
</task>

<task type="auto" tdd="true">
  <name>Task 2: Add paths::unexpand_tilde + tilde-preserving Config::save_checked (HARD-22)</name>
  <files>crates/tome/src/paths.rs, crates/tome/src/config/mod.rs</files>
  <read_first>
    - crates/tome/src/paths.rs (current `expand_tilde` impl — the new `unexpand_tilde` is its inverse)
    - crates/tome/src/config.rs (pre-split: `Config::save_checked` at line 832 — read the current expand_tildes pre-save shape) OR `crates/tome/src/config/mod.rs` (post-Task-1)
    - crates/tome/src/machine.rs (`MachinePrefs::save` — D-TILDE-2 says machine.toml stays verbatim; verify save shape doesn't tilde-rewrite)
    - .planning/phases/15-cli-hardening/15-CONTEXT.md §"HARD-22 tilde preservation" §"D-TILDE-1 (auto-portable normalisation on save)" + §"D-TILDE-2 (scope: tome.toml only)"
    - .planning/REQUIREMENTS.md §"HARD-22"
    - .planning/phases/09-cross-machine-path-overrides/09-CONTEXT.md (PORT-02 invariant: apply_machine_overrides mutates load-time-only copy)
  </read_first>
  <behavior>
    `unexpand_tilde` round-trip and idempotency:
    - Test: `unexpand_tilde(expand_tilde(Path::new("~/skills"))) == Path::new("~/skills")` (round-trip)
    - Test: `unexpand_tilde(Path::new("~/skills")) == Path::new("~/skills")` (idempotent on already-tilde)
    - Test: `unexpand_tilde(Path::new("/Users/martin/skills"))` (where $HOME = "/Users/martin") returns `Path::new("~/skills")`
    - Test: `unexpand_tilde(Path::new("/var/lib/skills"))` returns `Path::new("/var/lib/skills")` (outside $HOME → kept absolute)
    - Test: `unexpand_tilde(Path::new(""))` returns empty path unchanged
    - Test: `unexpand_tilde(Path::new("/Users/martin"))` returns `Path::new("~")` (exact $HOME maps to bare ~)

    `Config::save_checked` round-trip (Config::save_checked LIVES in `crates/tome/src/config/mod.rs` per Task 1 S3 lock):
    - Test: load `tome.toml` containing `library_dir = "~/skills"`, save back, file content includes `library_dir = "~/skills"` (preserved)
    - Test: load `tome.toml` containing `library_dir = "/Users/martin/skills"`, save back, file content includes `library_dir = "~/skills"` (rewritten — auto-portable)
    - Test: load `tome.toml` containing `library_dir = "/var/lib/skills"`, save back, file content includes `library_dir = "/var/lib/skills"` (kept absolute — outside $HOME)
    - Test: when `apply_machine_overrides` mutated the in-memory config (override path was applied), `save_checked` writes the ORIGINAL path from `tome.toml` — NOT the override path from `machine.toml`. (Phase 9 PORT-02 invariant.)
    - Test: every path field in `tome.toml` (library_dir, tome_home, every `directories.<name>.path`) participates in the unexpand pass.

    `MachinePrefs::save` non-rewrite (D-TILDE-2):
    - Test: load `machine.toml` with `[directory_overrides.foo] path = "/Users/martin/external"`, save back, `path` value is byte-identical (no tilde rewrite).
    - Test: load `machine.toml` with `[directory_overrides.foo] path = "~/skills"`, save back, `path` value preserved as `~/skills`.
    - Test: load `machine.toml` with `[directory_overrides.foo] path = "/Volumes/External/skills"`, save back, `path` preserved verbatim.
  </behavior>
  <action>
    **Step A: Add `paths::unexpand_tilde` helper.**

    In `crates/tome/src/paths.rs`, add:

    ```rust
    /// Inverse of `expand_tilde`: rewrites a path under `$HOME` to `~/...` shape.
    /// Paths outside `$HOME` are returned unchanged. Idempotent on already-tilde paths.
    /// Used by `Config::save_checked` to keep `tome.toml` cross-machine portable.
    pub fn unexpand_tilde(p: &Path) -> PathBuf {
        // 1. Already-tilde input (starts with "~"): return unchanged.
        if p.starts_with("~") {
            return p.to_path_buf();
        }
        // 2. Resolve $HOME via the same mechanism expand_tilde uses (dirs::home_dir
        //    or std::env::var("HOME") — match expand_tilde's existing strategy).
        let home = match dirs::home_dir() {
            Some(h) => h,
            None => return p.to_path_buf(),  // No HOME — give up, keep absolute.
        };
        // 3. If p == home → return PathBuf::from("~")
        // 4. If p is under home → return PathBuf::from("~").join(p.strip_prefix(&home).unwrap())
        // 5. Else → return p.to_path_buf()
        if p == home {
            PathBuf::from("~")
        } else if let Ok(rest) = p.strip_prefix(&home) {
            PathBuf::from("~").join(rest)
        } else {
            p.to_path_buf()
        }
    }
    ```

    Mirror `expand_tilde`'s exact `$HOME` resolution strategy — if `expand_tilde` uses `dirs::home_dir()`, so does `unexpand_tilde`; if it uses `std::env::var("HOME")`, match that. Round-trip identity must hold: `unexpand_tilde(expand_tilde(x)) == x` for all paths under `$HOME` and outside.

    **Step B: Rewrite `Config::save_checked` (in `crates/tome/src/config/mod.rs`) to drop pre-save expansion.**

    Behaviour table (verbatim from CONTEXT.md D-TILDE-1):

    ```
    config.toml IN  : library_dir = "~/skills"
    config.toml OUT : library_dir = "~/skills"          (preserved)

    config.toml IN  : library_dir = "/Users/martin/skills"
    config.toml OUT : library_dir = "~/skills"          (rewritten — auto-portable)

    config.toml IN  : library_dir = "/var/lib/skills"
    config.toml OUT : library_dir = "/var/lib/skills"   (kept absolute — outside $HOME)
    ```

    Current shape (~line 832 in pre-split config.rs): `save_checked` calls `self.expand_tildes()` on a clone, then validates, then writes the expanded copy. Rewrite as:

    1. **Validation copy:** Build an in-memory expanded clone for `validate()` (validation logic needs absolute paths to detect overlaps). Do NOT mutate `self`.

    2. **Serialisation copy:** Build a separate clone where every `PathBuf` field passes through `paths::unexpand_tilde()`. Path fields covered: `Config.library_dir`, `Config.tome_home`, every `DirectoryConfig.path` value in `Config.directories`. Whatever the user wrote (already-tilde or absolute-under-home) becomes `~`-shape; absolute-outside-home stays absolute.

    3. **Write the unexpand-passed clone** via the existing atomic temp+rename pattern.

    4. **Override interaction (PORT-02 invariant):** `apply_machine_overrides` mutates a load-time-only copy. `save_checked` operates on `&self` (the unmutated config) — therefore override paths from `machine.toml` are NEVER serialised back to `tome.toml`. This invariant is preserved automatically because we never call `apply_machine_overrides` in the save path. **Add a unit test** verifying this: load a config + machine.toml with overrides, call `apply_machine_overrides` (simulating load-time), save the unmutated config, assert the saved tome.toml has the original (pre-override) path. CONTEXT.md is explicit: "override paths from `machine.toml` are NEVER written back to `tome.toml`".

    **Step C: Verify `MachinePrefs::save` does NOT tilde-rewrite (D-TILDE-2).**

    Read `crates/tome/src/machine.rs::MachinePrefs::save`. Confirm it serialises path fields verbatim — no `expand_tildes` or `unexpand_tilde` call. If `MachinePrefs::save` accidentally rewrites (e.g. via shared serialisation helper), explicitly fence the `paths::unexpand_tilde` call to `Config::save_checked` only. Add a regression test (per `<behavior>` above) to pin verbatim preservation.

    **Step D: Unit tests.**

    Add a `tests` mod inside `crates/tome/src/paths.rs` covering every case in `<behavior>`:

    ```rust
    #[cfg(test)]
    mod tests {
        use super::*;
        use std::path::PathBuf;

        // Note: tests that depend on $HOME use std::env::set_var to control it
        // — match the existing expand_tilde test style (look in paths.rs).

        #[test]
        fn unexpand_tilde_idempotent_on_tilde() { ... }
        #[test]
        fn unexpand_tilde_rewrites_home_subpath() { ... }
        #[test]
        fn unexpand_tilde_preserves_outside_home() { ... }
        #[test]
        fn unexpand_tilde_round_trip_identity() {
            // expand_tilde(unexpand_tilde(p)) == p for every meaningful input
        }
        // ... etc per <behavior>
    }
    ```

    Add tilde-preservation tests in `crates/tome/src/config/mod.rs` (Config::save_checked LIVES here per S3 lock) covering the IN/OUT table verbatim. Also add the override-non-roundtrip regression test.

    Add a single regression test in `crates/tome/src/machine.rs` for D-TILDE-2 (verbatim preservation in machine.toml).
  </action>
  <verify>
    <automated>cargo test -p tome paths::tests &amp;&amp; cargo test -p tome config::tests &amp;&amp; cargo test -p tome machine::tests &amp;&amp; cargo clippy --all-targets -- -D warnings</automated>
  </verify>
  <acceptance_criteria>
    - `grep -nE "pub fn unexpand_tilde" crates/tome/src/paths.rs` returns at least one line.
    - `grep -nE "unexpand_tilde" crates/tome/src/config/mod.rs` finds at least one call site (Config::save_checked).
    - `grep -nE "expand_tildes\\(\\)" crates/tome/src/config/mod.rs` shows expand_tildes is no longer called in the save path (only on the validation clone, if at all).
    - `grep -nE "unexpand_tilde" crates/tome/src/machine.rs` returns NOTHING (D-TILDE-2 — machine.toml stays verbatim).
    - `cargo test -p tome paths::tests::unexpand_tilde_round_trip_identity` passes.
    - `cargo test -p tome config::tests` passes (new tilde-preservation tests included).
    - `cargo test -p tome machine::tests` passes (D-TILDE-2 verbatim regression test included).
    - At least one regression test asserts override paths from machine.toml are NOT written back to tome.toml (PORT-02 invariant verbatim assertion). Search: `grep -rE "override.*save_checked|apply_machine_overrides.*save" crates/tome/src/config/`.
    - `cargo clippy --all-targets -- -D warnings` exits 0.
  </acceptance_criteria>
  <done>
    `paths::unexpand_tilde` exists and is round-trip-correct with `expand_tilde`. `Config::save_checked` (in `config/mod.rs`) preserves tilde inputs and auto-rewrites under-home absolute paths to `~`-shape on save. `MachinePrefs::save` is verified to NOT rewrite. Override paths never round-trip through tome.toml.
  </done>
</task>

</tasks>

<verification>
- `cargo build -p tome` exits 0
- `cargo clippy --all-targets -- -D warnings` exits 0
- `cargo test -p tome` passes; new tilde-preservation tests added (target: ≥6 new tests across paths::tests, config::tests, machine::tests)
- `crates/tome/src/config.rs` is gone; `crates/tome/src/config/{mod,types,overrides,validate}.rs` exists
- `Config::save_checked` lives in `crates/tome/src/config/mod.rs` (S3 lock — Plan 15-04 Task 2 grep target)
- `crates/tome/src/paths.rs` exposes `pub fn unexpand_tilde(...)`
- Round-trip identity holds: `expand_tilde(unexpand_tilde(p)) == expand_tilde(p)` for every test path
- Loading + saving a config with `library_dir = "~/skills"` produces an output with `library_dir = "~/skills"` (preserved)
- Loading + saving a config with `library_dir = "/Users/$USER/skills"` produces an output with `library_dir = "~/skills"` (rewritten)
</verification>

<success_criteria>
- HARD-03: `config.rs` splits into `config/{mod,types,overrides,validate}.rs`; `Config::save_checked` lands in `config/mod.rs` (S3 lock); public API byte-identical to consumers; tilde helpers stay in `paths.rs` (per CONTEXT.md "Claude's Discretion") (closes #487)
- HARD-22: `Config::save_checked` preserves `~`-shape paths and auto-rewrites under-home absolute paths to `~`-shape; `MachinePrefs::save` preserves user input verbatim per D-TILDE-2; override paths never round-trip through `tome.toml` (closes #457)
- New `paths::unexpand_tilde` helper round-trips with existing `expand_tilde`
- Test count grows by ≥6 (round-trip + IN/OUT table + override non-roundtrip + machine.toml verbatim)
</success_criteria>

<output>
After completion, create `.planning/phases/15-cli-hardening/15-02-SUMMARY.md` recording:
- Final config/ module file sizes (compare to pre-split 3,122 LOC config.rs)
- Confirmation that Config::save_checked landed in config/mod.rs (S3 lock)
- New tests added to paths::tests, config::tests, machine::tests
- Confirmation that PORT-02 invariant is preserved (override paths never serialised back)
- Issues closed: #487 (HARD-03), #457 (HARD-22)
</output>
