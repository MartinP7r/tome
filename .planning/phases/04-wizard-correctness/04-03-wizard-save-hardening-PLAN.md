---
phase: 4
plan: 3
type: execute
wave: 2
depends_on:
  - "04-01"
files_modified:
  - crates/tome/src/wizard.rs
  - crates/tome/src/config.rs
requirements:
  - WHARD-01
autonomous: true
must_haves:
  truths:
    - "User running `tome init` with an invalid type/role combo (e.g., Git + Target produced by the role-editing loop) sees a clear validation error and the config is not written to disk"
    - "A successful `tome init` round-trips: the written config passes Config::validate() and reloads without changes"
    - "`tome init --dry-run` reports the same validation errors a real save would — the dry-run branch runs the same checks"
    - "Wizard does not retry on validation failure — it returns Err and exits (D-08, D-09)"
  artifacts:
    - path: "crates/tome/src/wizard.rs"
      provides: "Save block hardened with expand_tildes() → validate() → TOML round-trip → save()"
      min_lines: 10
    - path: "crates/tome/src/wizard.rs"
      provides: "Dry-run preview branch also validates and round-trips before printing"
      contains: "dry_run"
    - path: "crates/tome/src/config.rs"
      provides: "Public `expand_tildes` method or equivalent accessor so wizard can invoke the same order as Config::load()"
      contains: "expand_tildes"
  key_links:
    - from: "crates/tome/src/wizard.rs::run()"
      to: "crates/tome/src/config.rs::Config::validate()"
      via: "direct method call before save()"
      pattern: "config\\.validate\\(\\)"
    - from: "crates/tome/src/wizard.rs::run()"
      to: "toml::to_string_pretty + toml::from_str"
      via: "round-trip equality check (D-03)"
      pattern: "toml::from_str"
---

<objective>
Harden the wizard's save path so an invalid config never reaches disk (WHARD-01). Insert the same sequence `Config::load()` uses (expand_tildes → validate) plus a TOML round-trip check (D-03) immediately before `config.save()`. Apply the same sequence in the `--dry-run` preview branch so "Generated config:" output is trustworthy. On any failure: hard error + exit (D-08, D-09) — no retry loop.

Purpose: close WHARD-01. Today, the wizard calls `config.save()` directly at `wizard.rs:317`; the role-editing loop or custom-directory flow can produce invalid type/role combos that bypass `Config::validate()` entirely, because `Config::load()` is the only existing entry point that validates.

Output: `wizard.rs` save block and dry-run preview branch both run expand → validate → round-trip before proceeding; `config.rs` exposes a stable way to trigger the expand+validate pipeline; unit tests confirm the wizard-save helper refuses invalid configs and approves valid ones without altering content.
</objective>

<execution_context>
@$HOME/.claude/get-shit-done/workflows/execute-plan.md
@$HOME/.claude/get-shit-done/templates/summary.md
</execution_context>

<context>
@.planning/PROJECT.md
@.planning/ROADMAP.md
@.planning/STATE.md
@.planning/phases/04-wizard-correctness/04-CONTEXT.md

<interfaces>
<!-- Direct quotes from 04-CONTEXT.md — load-bearing. -->

D-01: New validation checks live in Config::validate() — load-symmetric.
D-03: The wizard's save path also runs a TOML round-trip check (defense in depth):
      serialize Config to TOML, parse back, compare for equality. Catches
      serde-level regressions (e.g., a load-bearing field marked skip_serializing_if).
      Lives in the wizard save path only — not in Config::load().
D-07: Wizard save order matches load order: expand_tildes() → validate() → serialize → save.
D-08: On validation failure at save step: hard error + exit. No retry loop.
      wizard::run() returns Err(...) with context; user re-runs tome init.
D-09: Non-interactive / --no-input / no-TTY mode behaves identically to interactive
      mode on validation failure: hard error.

Discretion per D-03 (Claude's Discretion bullets in CONTEXT.md):
- Whether the TOML round-trip helper lives in config.rs (e.g. Config::save_checked) or wizard.rs → **This plan puts it in config.rs as `Config::save_checked`**, because it's exactly "what save-with-safety-rails" looks like and keeps the wizard thin.
- Whether PartialEq is derived on Config or the round-trip check compares TOML strings directly after canonicalizing → **This plan compares the re-emitted TOML string for byte equality**, because toml::to_string_pretty is deterministic and string comparison is simpler than deriving PartialEq on Config (which contains BTreeMap<DirectoryName, DirectoryConfig> and would require PartialEq on every leaf type).
- Whether Config::validate() stays a single method or splits → **Stays a single method** (no split needed; Plan 04-02 extended it in place).

Existing save call site (wizard.rs:312-317):
    } else if Confirm::new()
        .with_prompt("Save configuration?")
        .default(true)
        .interact()?
    {
        config.save(&config_path)?;

Existing dry-run branch (wizard.rs:306-311):
    if dry_run {
        println!("  (dry run -- not saving)");
        let toml_str = toml::to_string_pretty(&config)?;
        println!();
        println!("{}", style("Generated config:").bold());
        println!("{}", toml_str);

Existing Config::load order (config.rs:274-287) — the canonical sequence we MUST mirror:
    let mut config: Config = toml::from_str(&content).map_err(...)?;
    config.expand_tildes()?;
    config.validate()?;

Note: `Config::expand_tildes` is currently `fn expand_tildes(&mut self) -> Result<()>` (private) at
config.rs:403. This plan promotes it to `pub(crate) fn expand_tildes(&mut self)` so wizard.rs can call it.
</interfaces>
</context>

<tasks>

<task type="auto" tdd="true">
  <name>Task 1: Add `Config::save_checked` and expose `expand_tildes`; wire wizard save + dry-run branches</name>
  <files>
    crates/tome/src/config.rs
    crates/tome/src/wizard.rs
  </files>
  <read_first>
    - crates/tome/src/config.rs (focus on Config::load at lines 274-293, Config::save at lines 315-323, expand_tildes at lines 403-409, validate at lines 331-379, the test module starting at line 530)
    - crates/tome/src/wizard.rs (focus on run() entry at line 126, dry-run branch at lines 306-311, save block at lines 312-334)
    - .planning/phases/04-wizard-correctness/04-CONTEXT.md (D-01, D-03, D-07, D-08, D-09 — authoritative)
  </read_first>
  <behavior>
    After this task:

    Test 1 — `Config::save_checked` rejects invalid config (role/type conflict):
      A Config built with `role: Some(DirectoryRole::Target)` and `directory_type: DirectoryType::Git`
      returns Err from `save_checked(&path)`, and the file at `path` is NOT created (or is unchanged).
      The error message contains "Target (skills distributed here, not discovered here)" and "hint:"
      (inherited from Plan 04-01's upgraded validate() error).

    Test 2 — `Config::save_checked` rejects invalid config (library overlap):
      A Config where library_dir equals a Synced directory path returns Err from save_checked and
      does not write the file. Error contains "Conflict:" and "hint:".

    Test 3 — `Config::save_checked` rejects round-trip divergence (synthetic):
      Not directly testable against prod code (to observe divergence you'd need to mutate the round-trip
      output), but assertable via a dedicated helper test that runs save_checked against a known-good
      config and confirms the written file parses back to an equal Config. So: the positive round-trip
      test lives here — a known-good config save_checked → Config::load from the same path → re-serialize
      → byte-equal to the initial TOML string. (If in-repo PartialEq is unavailable, compare re-serialized
      TOML strings for equality.)

    Test 4 — Positive path: a valid Config saves successfully and the file exists.

    Plus (wizard-side, no test in this phase — integration tests come in Phase 5 per WHARD-05/06):
    - Wizard save block now calls `config.save_checked(&config_path)?` instead of `config.save(&config_path)?`.
    - Wizard dry-run branch now calls the same expand+validate+round-trip pipeline BEFORE printing,
      so `tome init --dry-run` exits with an error on invalid config.
    - On validation failure, wizard::run() returns Err (propagated by `?`) — hard exit, no retry loop.
  </behavior>
  <action>

### Part A — `crates/tome/src/config.rs`

Step A.1 — Change the visibility of `expand_tildes` from private to `pub(crate)` (line 403):

Replace:
```rust
    /// Expand `~` in all path fields.
    fn expand_tildes(&mut self) -> Result<()> {
```
with:
```rust
    /// Expand `~` in all path fields.
    pub(crate) fn expand_tildes(&mut self) -> Result<()> {
```

Step A.2 — Add a new public method `save_checked` on `Config`, immediately after the existing `save` method (config.rs:315-323). Insert before the `validate` method so method order reads `load → save → save_checked → validate → ...`:

```rust
    /// Save config, but first run the same expand + validate pipeline that
    /// `Config::load()` runs, followed by a TOML round-trip equality check
    /// (D-03: defence in depth — catches serde drift such as a field that
    /// accidentally disappears across a serialize/deserialize cycle).
    ///
    /// Returns `Err` without writing anything if any stage fails.
    ///
    /// Call this instead of `save()` from the wizard or any other code that
    /// produces a Config in-memory rather than loading it from disk.
    pub fn save_checked(&self, path: &Path) -> Result<()> {
        // Mirror Config::load order: expand → validate.
        // We operate on a clone so the caller's Config is not mutated.
        let mut expanded = self.clone();
        expanded.expand_tildes()?;
        expanded.validate()?;

        // TOML round-trip (D-03): serialize, parse back, re-serialize,
        // compare the two TOML strings for byte equality. If they differ,
        // a field has been silently dropped or rewritten by serde.
        let emitted =
            toml::to_string_pretty(&expanded).context("failed to serialize config (pre-check)")?;
        let reparsed: Config =
            toml::from_str(&emitted).context("round-trip: generated TOML did not reparse")?;
        let reemitted = toml::to_string_pretty(&reparsed)
            .context("failed to serialize config (round-trip)")?;
        anyhow::ensure!(
            emitted == reemitted,
            "round-trip mismatch: serialized config differs after parse+reserialize — this is a serde bug in a tome type, not a user error.\n\
             Conflict: emit/reparse produced different TOML\n\
             Why: a field is not reversibly (de)serializable; saving would lose data.\n\
             hint: report this as a tome bug and share the generated output below.\n\
             --- first emit ---\n{emitted}\n--- second emit ---\n{reemitted}"
        );

        // Safe to save — write the same bytes we verified.
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("failed to create {}", parent.display()))?;
        }
        std::fs::write(path, &emitted)
            .with_context(|| format!("failed to write {}", path.display()))
    }
```

Note on `self.clone()`: `Config` already derives `Clone` (config.rs:251). No new derive required. No new derive of `PartialEq` on `Config` is introduced — the round-trip check compares TOML strings, not Configs.

Step A.3 — Add these unit tests to the end of the `#[cfg(test)] mod tests` block in config.rs. Append after the overlap tests from Plan 04-02 (or in the same region — ordering within the test module is insignificant):

```rust
    // --- save_checked tests (WHARD-01) ---

    #[test]
    fn save_checked_rejects_role_type_conflict() {
        let tmp = tempfile::TempDir::new().unwrap();
        let path = tmp.path().join("tome.toml");
        let config = Config {
            library_dir: PathBuf::from("/tmp/lib-sc-1"),
            directories: BTreeMap::from([(
                DirectoryName::new("bad").unwrap(),
                DirectoryConfig {
                    path: PathBuf::from("/tmp/src"),
                    directory_type: DirectoryType::Git,
                    role: Some(DirectoryRole::Target),
                    branch: None,
                    tag: None,
                    rev: None,
                    subdir: None,
                },
            )]),
            ..Default::default()
        };
        let err = config.save_checked(&path).unwrap_err();
        assert!(
            err.to_string()
                .contains("Target (skills distributed here, not discovered here)"),
            "expected role parenthetical, got: {err}"
        );
        assert!(!path.exists(), "save_checked must not write on validation failure");
    }

    #[test]
    fn save_checked_rejects_library_overlap() {
        let tmp = tempfile::TempDir::new().unwrap();
        let path = tmp.path().join("tome.toml");
        let config = Config {
            library_dir: PathBuf::from("/tmp/shared-sc"),
            directories: BTreeMap::from([(
                DirectoryName::new("shared").unwrap(),
                DirectoryConfig {
                    path: PathBuf::from("/tmp/shared-sc"),
                    directory_type: DirectoryType::Directory,
                    role: Some(DirectoryRole::Synced),
                    branch: None,
                    tag: None,
                    rev: None,
                    subdir: None,
                },
            )]),
            ..Default::default()
        };
        let err = config.save_checked(&path).unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("Conflict:"), "missing Conflict: {msg}");
        assert!(msg.contains("hint:"), "missing hint: {msg}");
        assert!(!path.exists(), "save_checked must not write on validation failure");
    }

    #[test]
    fn save_checked_writes_valid_config_and_reloads_unchanged() {
        let tmp = tempfile::TempDir::new().unwrap();
        let path = tmp.path().join("tome.toml");
        let config = Config {
            library_dir: PathBuf::from("/tmp/lib-sc-ok"),
            directories: BTreeMap::from([(
                DirectoryName::new("ok").unwrap(),
                DirectoryConfig {
                    path: PathBuf::from("/tmp/ok"),
                    directory_type: DirectoryType::Directory,
                    role: Some(DirectoryRole::Synced),
                    branch: None,
                    tag: None,
                    rev: None,
                    subdir: None,
                },
            )]),
            ..Default::default()
        };
        config.save_checked(&path).expect("valid config must save");
        assert!(path.exists(), "file must exist after save_checked");

        // Reload and re-emit: must be byte-equal to the on-disk file.
        let on_disk = std::fs::read_to_string(&path).unwrap();
        let reloaded = Config::load(&path).expect("saved config must reload");
        let reemitted = toml::to_string_pretty(&reloaded).unwrap();
        assert_eq!(on_disk, reemitted, "saved file must round-trip exactly");
    }

    #[test]
    fn save_checked_does_not_mutate_caller() {
        // Caller's library_dir uses tilde; save_checked must not rewrite it in the caller's Config.
        let tmp = tempfile::TempDir::new().unwrap();
        let path = tmp.path().join("tome.toml");
        let config = Config {
            library_dir: PathBuf::from("~/some/lib-not-real"),
            ..Default::default()
        };
        let _ = config.save_checked(&path); // may fail on library_dir-is-a-file or succeed; irrelevant
        assert_eq!(
            config.library_dir,
            PathBuf::from("~/some/lib-not-real"),
            "save_checked must operate on a clone and leave the caller untouched"
        );
    }
```

### Part B — `crates/tome/src/wizard.rs`

Step B.1 — Import nothing new. `Config` is already in scope at `wizard.rs:13`.

Step B.2 — Replace the save block at wizard.rs:312-334 so `save_checked` is used. The `git init` offer runs only on successful save (unchanged). Exact diff:

Replace:
```rust
    } else if Confirm::new()
        .with_prompt("Save configuration?")
        .default(true)
        .interact()?
    {
        config.save(&config_path)?;
        println!("{} Config saved!", style("done").green());

        // Offer to git-init the tome home directory for backup tracking
        let tome_home = config_path
            .parent()
            .expect("config path should have a parent");
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
    }
```
with:
```rust
    } else if Confirm::new()
        .with_prompt("Save configuration?")
        .default(true)
        .interact()?
    {
        // D-01/D-03/D-07/D-08: expand → validate → round-trip → save.
        // On any failure, return Err — no retry loop (D-08/D-09).
        config
            .save_checked(&config_path)
            .context("wizard save aborted: configuration is invalid")?;
        println!("{} Config saved!", style("done").green());

        // Offer to git-init the tome home directory for backup tracking
        let tome_home = config_path
            .parent()
            .expect("config path should have a parent");
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
    }
```

`Context` is already imported at `wizard.rs:7` (`use anyhow::{Context, Result};`). No new import needed.

Step B.3 — Harden the dry-run branch at wizard.rs:306-311 so the preview is trustworthy (runs validate + round-trip before printing). Replace:
```rust
    if dry_run {
        println!("  (dry run -- not saving)");
        let toml_str = toml::to_string_pretty(&config)?;
        println!();
        println!("{}", style("Generated config:").bold());
        println!("{}", toml_str);
```
with:
```rust
    if dry_run {
        println!("  (dry run -- not saving)");
        // D-07/D-08/D-09: validate the same way a real save would, but without
        // writing to disk. Use a clone so we can expand tildes without mutating
        // the original Config (which might be returned to the caller).
        let mut expanded = config.clone();
        expanded
            .expand_tildes()
            .context("wizard dry-run: tilde expansion failed")?;
        expanded
            .validate()
            .context("wizard dry-run: configuration is invalid")?;
        let toml_str = toml::to_string_pretty(&expanded)
            .context("wizard dry-run: failed to serialize config")?;
        // Defense-in-depth (D-03): reparse to confirm round-trip integrity.
        let _: Config = toml::from_str(&toml_str)
            .context("wizard dry-run: generated TOML did not reparse")?;
        println!();
        println!("{}", style("Generated config:").bold());
        println!("{}", toml_str);
```

Note: the dry-run branch previews the expanded form (absolute paths after `~` expansion). This is intentional — the point of the preview is to show what would be validated-and-saved, and `save_checked` is the thing it's previewing.

### Part C — Run CI equivalent

```bash
cd /Users/martin/dev/opensource/tome
cargo fmt
cargo clippy --all-targets -- -D warnings
cargo test -p tome
```

Do NOT:
- Add a retry loop or prompt on validation failure (D-08, D-09).
- Move overlap detection into `wizard.rs` (D-01: it stays in `Config::validate()`).
- Derive `PartialEq` on `Config` (round-trip compares TOML strings).
- Introduce new public API beyond `Config::save_checked` and `pub(crate)` visibility bump for `expand_tildes`.
  </action>
  <verify>
    <automated>cd /Users/martin/dev/opensource/tome && cargo test -p tome --lib config::tests::save_checked_rejects_role_type_conflict config::tests::save_checked_rejects_library_overlap config::tests::save_checked_writes_valid_config_and_reloads_unchanged config::tests::save_checked_does_not_mutate_caller && cargo build -p tome</automated>
  </verify>
  <acceptance_criteria>
    - `rg "pub fn save_checked" crates/tome/src/config.rs` returns 1 hit
    - `rg "pub\(crate\) fn expand_tildes" crates/tome/src/config.rs` returns 1 hit (visibility bump applied)
    - `rg "save_checked" crates/tome/src/wizard.rs` returns ≥ 1 hit (wizard uses the checked save)
    - `rg "config\.save\(" crates/tome/src/wizard.rs` returns 0 hits (legacy save() no longer called from wizard)
    - `rg "round-trip" crates/tome/src/config.rs` returns ≥ 1 hit inside save_checked
    - `rg "wizard dry-run: configuration is invalid" crates/tome/src/wizard.rs` returns 1 hit
    - `rg "wizard save aborted" crates/tome/src/wizard.rs` returns 1 hit
    - `cargo test -p tome --lib config::tests::save_checked_rejects_role_type_conflict` exits 0
    - `cargo test -p tome --lib config::tests::save_checked_rejects_library_overlap` exits 0
    - `cargo test -p tome --lib config::tests::save_checked_writes_valid_config_and_reloads_unchanged` exits 0
    - `cargo test -p tome --lib config::tests::save_checked_does_not_mutate_caller` exits 0
    - `cargo build -p tome` exits 0 (wizard.rs still compiles with the new call)
    - `cargo clippy --all-targets -- -D warnings` exits 0
    - `cargo fmt -- --check` exits 0
  </acceptance_criteria>
  <done>
    `Config::save_checked` exists and enforces expand → validate → round-trip → write. `Config::expand_tildes` is `pub(crate)` so the wizard's dry-run branch can call it. Wizard save path uses `save_checked`; dry-run branch validates + round-trips before printing. Validation failure returns Err — no retry. Four new unit tests cover rejection and positive paths. `make ci` clean.
  </done>
</task>

</tasks>

<verification>
Phase-exit checks for Plan 04-03:

1. `cargo fmt -- --check` exits 0
2. `cd /Users/martin/dev/opensource/tome && cargo clippy --all-targets -- -D warnings` exits 0
3. `cd /Users/martin/dev/opensource/tome && cargo test -p tome` exits 0
4. `rg "config\.save\(" crates/tome/src/wizard.rs` returns 0 hits (wizard exclusively uses save_checked)
5. `rg "save_checked" crates/tome/src/` returns hits in both config.rs and wizard.rs
6. Manual (optional, not part of automated gate): `cargo run -p tome -- init --dry-run` against a contrived invalid starting state exits non-zero with the validation message. (Full integration coverage is Phase 5 — WHARD-05.)
</verification>

<success_criteria>
- Wizard never calls `config.save()` directly; it calls `Config::save_checked` which runs expand → validate → round-trip → write.
- Dry-run branch runs the same expand + validate + round-trip before printing, so `tome init --dry-run` is trustworthy.
- Failure behaviour matches D-08/D-09: hard exit via `?`, no retry prompt.
- Valid configs continue to round-trip unchanged (must_have #4 from ROADMAP).
- `make ci` clean.
</success_criteria>

<output>
After completion, create `.planning/phases/04-wizard-correctness/04-03-SUMMARY.md`.
</output>
