---
phase: 5
plan: 3
type: execute
wave: 2
depends_on:
  - "05-01"
files_modified:
  - crates/tome/tests/cli.rs
requirements:
  - WHARD-05
autonomous: true
must_haves:
  truths:
    - "`tome init --dry-run --no-input` with `HOME` overridden to an empty TempDir exits 0, prints `Generated config:`, and the trailing TOML block parses, passes `Config::validate()`, and round-trips through TOML byte-equal"
    - "`tome init --dry-run --no-input` with `HOME` seeded with both a managed known dir (`.claude/plugins`) and a synced known dir (`.claude/skills`) produces a config whose `directories` map contains `claude-plugins` (ClaudePlugins/Managed) and `claude-skills` (Directory/Synced)"
    - "The generated-config block parses via `toml::from_str::<Config>` — no custom parsing, no snapshot brittleness"
    - "Both tests use `NO_COLOR=1` and `HOME` env override; no dependency on the real user home"
    - "Test bodies read Config state through the `pub fn directories()`, `pub fn library_dir()`, `pub fn exclude()` accessors added by Plan 05-01 — integration tests in `crates/tome/tests/cli.rs` compile as a SEPARATE crate and cannot reach `pub(crate)` fields directly"
  artifacts:
    - path: "crates/tome/tests/cli.rs"
      provides: "Integration test `init_dry_run_no_input_empty_home` exercising `tome init --dry-run --no-input` against an empty TempDir HOME"
      contains: "init_dry_run_no_input_empty_home"
    - path: "crates/tome/tests/cli.rs"
      provides: "Integration test `init_dry_run_no_input_seeded_home` exercising `tome init --dry-run --no-input` against a TempDir HOME with pre-seeded known directories"
      contains: "init_dry_run_no_input_seeded_home"
    - path: "crates/tome/tests/cli.rs"
      provides: "Helper `parse_generated_config(stdout)` that splits on `Generated config:` marker and parses the trailing block as `tome::config::Config`"
      contains: "fn parse_generated_config"
  key_links:
    - from: "crates/tome/tests/cli.rs::init_dry_run_no_input_*"
      to: "crates/tome binary via `assert_cmd::Command`"
      via: "`tome()` helper with `init --dry-run --no-input`, HOME override, NO_COLOR=1"
      pattern: "\\.args\\(\\[\"init\", \"--dry-run\", \"--no-input\"\\]\\)"
    - from: "parse_generated_config"
      to: "tome::config::Config"
      via: "`toml::from_str::<tome::config::Config>` on the post-marker block"
      pattern: "toml::from_str::<tome::config::Config>"
    - from: "crates/tome/tests/cli.rs::init_dry_run_no_input_* assertions"
      to: "tome::config::Config accessors (from Plan 05-01 Part D)"
      via: "`config.directories()`, `config.library_dir()`, `config.exclude()` method calls (tests/cli.rs is an EXTERNAL crate — field access won't compile)"
      pattern: "config\\.(directories|library_dir|exclude)\\(\\)"
---

<objective>
Close WHARD-05 with one pair of `assert_cmd` integration tests in `crates/tome/tests/cli.rs`:

1. `init_dry_run_no_input_empty_home` — HOME is an empty TempDir; no known directories are
   auto-discovered. The wizard still completes (per Plan 05-01's `--no-input` plumbing),
   produces a `Generated config:` block in stdout, and that block parses → validates →
   round-trips through TOML byte-equal.

2. `init_dry_run_no_input_seeded_home` — HOME is a TempDir pre-seeded with a managed
   `.claude/plugins/` subtree and a synced `.claude/skills/` subtree. The resulting Config
   contains both `claude-plugins` (ClaudePlugins/Managed) and `claude-skills`
   (Directory/Synced), `library_dir` is `<TMP>/.tome/skills` (per D-01 default), and the
   config validates + round-trips.

Both tests:
- Use `assert_cmd::Command::cargo_bin("tome")` via the existing `tome()` helper.
- Override `HOME` via `.env("HOME", tmp.path())`.
- Set `NO_COLOR=1` (existing convention for stable substring matching).
- Set `TOME_HOME` explicitly to `<TMP>/.tome` so `default_tome_home()` does not fall through to
  the user's real `~/.config/tome/config.toml` during the run (avoids cross-test contamination).
- Parse the `Generated config:` marker at `wizard.rs:324` as the splitting point per D-12.
- Skip on Windows by virtue of the crate being Unix-only; no `cfg` gate is needed because the
  crate already fails to compile on Windows (symlink code).
- Read the parsed Config's state exclusively through the `pub fn` accessor methods added by
  Plan 05-01 Part D — `config.directories()`, `config.library_dir()`, `config.exclude()`.
  `tests/cli.rs` compiles as a SEPARATE crate from the `tome` library and has NO access to
  `pub(crate)` items; only `pub` items (including the new accessors, `validate()`, and the
  `DirectoryConfig` public fields/methods) are reachable.

Purpose: close WHARD-05. This is the only integration test this phase adds — it confirms the
entire `tome init --dry-run --no-input` path is behaviorally correct end-to-end, bypassing
dialoguer because `--no-input` takes every default (Plan 05-01).

Output: two new test functions at the bottom of `crates/tome/tests/cli.rs` plus one small
helper (`parse_generated_config`). No production code changes — Plan 05-01 already added the
three `Config` accessor methods this plan consumes.
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
@.planning/phases/05-wizard-test-coverage/05-01-no-input-plumbing-and-assemble-config-PLAN.md

<interfaces>
<!-- Load-bearing facts for writing this test. -->

Dependency flow: Plan 05-01 removed the `lib.rs:164-165` bail and plumbed `--no-input` through
`wizard::run`, so `tome init --dry-run --no-input` now completes instead of exiting early.
Plan 05-01 Part D also added three `pub fn` read-only accessor methods on `Config` that this
test crate depends on. Without Plan 05-01 these tests cannot run and cannot compile.

**CRITICAL — crate boundary rule (corrects a prior error in this plan):**

`crates/tome/tests/cli.rs` compiles as a SEPARATE crate from the `tome` library. It links to the
`tome` library as an external consumer, exactly like a downstream user would. This is Rust's
standard integration-test layout: anything under `crates/tome/tests/` gets its own crate root.

Consequences for this plan:
- `pub` items from the `tome` crate ARE reachable via `use tome::...` — this includes `Config`,
  `DirectoryName`, `DirectoryType`, `DirectoryRole`, `DirectoryConfig` (the struct itself, not
  its `pub(crate)` fields), `Config::validate()`, `Config::load()`, `DirectoryConfig::role()`,
  and the new accessors `Config::directories()`, `Config::library_dir()`, `Config::exclude()`.
- `pub(crate)` items from the `tome` crate are NOT reachable. This includes the fields
  `Config::directories` (note: the FIELD, not the method), `Config::library_dir` (field),
  `Config::exclude` (field), and `DirectoryConfig::role` (field — but `DirectoryConfig::role()`
  the method IS `pub` and works).
- Therefore: **write `config.directories()` not `config.directories`**. The method call is the
  contract. Field access would fail to compile with `E0616: field is private`.

(For contrast: unit tests in `crates/tome/src/wizard.rs::tests` or `config.rs::tests` ARE in the
same crate as `tome` and therefore CAN access `pub(crate)` fields directly. That's why Plan 05-02
constructs `DirectoryConfig { role: Some(role), ... }` inline without an accessor. The mental
model differs between the two test locations — see CONTEXT.md `<test_conventions>` for the
one-line summary.)

Accessor signatures this test consumes (EXACT — from Plan 05-01 Part D):
```rust
impl Config {
    pub fn directories(&self) -> &BTreeMap<DirectoryName, DirectoryConfig> { ... }
    pub fn library_dir(&self) -> &Path { ... }
    pub fn exclude(&self) -> &BTreeSet<SkillName> { ... }
    pub fn validate(&self) -> Result<()> { ... }  // already pub, unchanged
}
```

`DirectoryConfig` public surface this test consumes (already `pub` — unchanged by Plan 05-01):
```rust
pub struct DirectoryConfig {
    pub path: PathBuf,
    pub directory_type: DirectoryType,
    pub(crate) role: Option<DirectoryRole>,   // field is pub(crate) — use role() accessor
    pub branch: Option<String>,
    pub tag: Option<String>,
    pub rev: Option<String>,
    pub subdir: Option<String>,
}
impl DirectoryConfig {
    pub fn role(&self) -> DirectoryRole { ... }  // already pub, safe to call from tests/cli.rs
}
```

Existing conventions in `crates/tome/tests/cli.rs`:
- `fn tome() -> Command` at line 8 — use this to spawn.
- `snapshot_settings(tmp)` at line 13 — available but not needed here (plain assertions suffice
  per CONTEXT.md D-07 and Claude's-Discretion "snapshots are an upgrade").
- `NO_COLOR=1` env applied via `.env("NO_COLOR", "1")` (pattern at line 327 in existing sync test).

Wizard stdout marker (wizard.rs:324, authoritative — do NOT modify):
```rust
println!("{}", style("Generated config:").bold());
```
With `NO_COLOR=1` this emits literally `Generated config:` on its own line, followed by the
TOML on subsequent lines. The marker is the split point (D-12).

Wizard print BEFORE the marker — these lines must be discarded by the split:
- wizard.rs:128: `"Welcome to tome setup!"` (bold+cyan but NO_COLOR strips it to `"Welcome to tome setup!"`)
- wizard.rs:129-142: intro block
- step dividers via `step_divider()` at wizard.rs:361 printing `-- {label} ----------------------------------`
- `configure_directories` prints `"  v 0 directory(ies) selected"` (under empty HOME) or
  `"  v 2 directory(ies) selected"` (under seeded HOME)
- `configure_library` prints nothing under `--no-input` (Select was skipped)
- `configure_exclusions` prints `"  (no skills discovered yet -- exclusions can be added manually to config)"`
  if no skills, or nothing if no prompt was shown (with `--no-input` the MultiSelect is skipped
  and the function just returns `BTreeSet::new()` — and the `"(no skills discovered yet…)"` line
  only prints when `skills.is_empty()` because the early-return is before the selector)
- summary table (but headers are also no-color stripped, and rows use plain-text `cfg.path.display()`)
- wizard.rs:302-304: `"Config will be saved to: <path>"`
- wizard.rs:307: `"  (dry run -- not saving)"`

After the marker, wizard.rs:325 prints `println!("{}", toml_str);` — the entire serialized TOML
followed by a final newline. Nothing further is printed in the `--dry-run` branch (the else-if
`Save configuration?` branch is not reached because `dry_run` is true).

So: `stdout.split_once("Generated config:\n")` yields `(preamble, body)` where `body` is pure
TOML (with a trailing newline from the `println!`). That's the parse input.

To parse into `tome::config::Config`, the integration-test crate needs access to the public
`Config` type. `crates/tome/src/lib.rs:30` declares `pub mod config;` — so `tome::config::Config`
is reachable from `tests/cli.rs` via `use tome::config::Config;` because the integration-test
crate links against the `tome` library crate (binary + library coexist in `crates/tome` —
`src/main.rs` and `src/lib.rs`). The existing `tests/cli.rs` does not yet use `tome::` paths;
this plan introduces the first such import.

TOML round-trip: `toml::to_string_pretty(&config)` followed by `toml::from_str::<Config>` followed
by a second `to_string_pretty` → byte-equality check. Exactly the same check `Config::save_checked`
runs (confirmed at config.rs:485-517) — so a successful round-trip in save_checked implies a
successful round-trip here too; this test is a cross-check that holds even if save_checked were
removed. `Serialize` and `Deserialize` are derived on `Config`, so `toml::to_string_pretty(&config)`
and `toml::from_str::<Config>(body)` work from `tests/cli.rs` without any additional `pub` exposure.

Environment isolation:
- `HOME` override alone is insufficient: `default_tome_home()` at config.rs:568 checks
  `TOME_HOME` env first, then falls through to reading `~/.config/tome/config.toml` from the real
  home if `HOME` is set. Because we override `HOME` to the TempDir, the "real user" fallback is
  already neutralised — but to be maximally defensive and deterministic, the test also sets
  `TOME_HOME=<TMP>/.tome`. That also skips the `read_config_tome_home()` fallback entirely.
- Also unset any leaked `NO_COLOR=0` or similar noise: `.env_clear()` is too aggressive (it
  drops PATH, cargo test invocation would fail). Instead, just explicitly set the three vars we
  care about: `HOME`, `TOME_HOME`, `NO_COLOR`.

Expected `directories` in the seeded-HOME test (per D-01 + KNOWN_DIRECTORIES):
- `claude-plugins` → type ClaudePlugins, role Managed, path `"~/.claude/plugins"` (tilde-shaped;
  the wizard inserts `PathBuf::from("~").join(kd.default_path)` at wizard.rs:427).
- `claude-skills`  → type Directory, role Synced, path `"~/.claude/skills"`.

Because the wizard stores tilde-shaped paths in the generated Config (wizard.rs:427) BUT
`configure_library` under `--no-input` returns `PathBuf::from("~/.tome/skills")` (tilde-shaped
too, per Plan 05-01's branch), and the dry-run branch runs `config.clone().expand_tildes()`
before printing (wizard.rs:311-317), THE PRINTED TOML HAS EXPANDED PATHS. So the TOML on stdout
under the TempDir HOME will have `/tmp/.tmpXXX/.claude/plugins` etc. — matching
`tmp.path().join(".claude/plugins")`.

This is subtle: the `--dry-run` branch preview shows the *post-expand-tildes* TOML, because
Plan 04-03 inserted a `.expand_tildes()` call before serialization. So tests must compare against
expanded paths, not tilde paths. `Config::library_dir()` returns `&Path` — compare it against
`&tmp.path().join(".tome/skills")` (reference-to-reference) or dereference with `*` as needed.

An empty HOME yields an empty `directories` BTreeMap AND `library_dir = <TMP>/.tome/skills`
(because `configure_library` under no_input returned `"~/.tome/skills"` and expand_tildes replaced
`~` with HOME = tmp.path()).
</interfaces>
</context>

<tasks>

<task type="auto" tdd="false">
  <name>Task 1: Add two integration tests plus a parsing helper to crates/tome/tests/cli.rs</name>
  <files>
    crates/tome/tests/cli.rs
  </files>
  <read_first>
    - crates/tome/tests/cli.rs (focus on `fn tome()` at line 8, `snapshot_settings` at line 13, the existing `sync_copies_skills_to_library`-style test pattern at approx line 300+ using `.env("NO_COLOR", "1")`)
    - crates/tome/src/wizard.rs (focus on lines 300-325 — the Config-will-be-saved + dry-run branch emitting `Generated config:` + `println!("{}", toml_str)` — and the expand_tildes call at line 313 whose effect must match this test's assertions)
    - crates/tome/src/config.rs (focus on Config struct at lines 249-268; Config accessor methods added by Plan 05-01 Part D placed immediately before `pub fn validate` at line 331; default_tome_home at 568)
    - crates/tome/src/lib.rs (focus on the Init branch at lines 163-193; Plan 05-01 must have landed — bail at 164-165 is gone, wizard::run is called with both flags)
    - .planning/phases/05-wizard-test-coverage/05-CONTEXT.md (D-01, D-10, D-11, D-12 and the `<test_conventions>` crate-boundary rule — authoritative)
    - .planning/phases/05-wizard-test-coverage/05-01-no-input-plumbing-and-assemble-config-PLAN.md Part D (prerequisite — the three accessor signatures this plan calls)
  </read_first>
  <action>

### Part A — `crates/tome/tests/cli.rs`

Step A.1 — At the top of the file, add one new import line. Existing `use` block (cli.rs:1-6) already imports:
```rust
use assert_cmd::{Command, cargo_bin_cmd};
use assert_fs::TempDir;
use insta::Settings;
use predicates::prelude::*;
use std::path::{Path, PathBuf};
use std::process::Command as StdCommand;
```
Add AFTER these, before any `fn`:
```rust
// For wizard integration tests. `tests/cli.rs` compiles as a SEPARATE crate from the `tome`
// library, so it can only reach `pub` items. Plan 05-01 Part D added `pub fn directories()`,
// `pub fn library_dir()`, and `pub fn exclude()` on Config for this exact purpose — direct
// `pub(crate)` field access would not compile from this crate.
use tome::config::{Config, DirectoryName, DirectoryRole, DirectoryType};
```

Step A.2 — APPEND the following helper and two tests at the end of the file (after the last
existing `#[test]` function). The helper is a plain `fn`, not inside a `mod`, matching the
style of `write_config` / `create_skill` at cli.rs:33-78.

```rust
// --------------------------------------------------------------------------
// Wizard integration tests (WHARD-05)
//
// These tests drive `tome init --dry-run --no-input` end-to-end with HOME
// overridden to a TempDir. They confirm:
//   - the --no-input plumbing from Plan 05-01 works (no bail, no dialoguer)
//   - the generated config passes Config::validate()
//   - the generated config round-trips through TOML byte-equal
//
// Crate-boundary note: this file is a separate crate from `tome`, so Config
// state is read via `pub fn` accessors (directories(), library_dir(), exclude())
// added by Plan 05-01 Part D. `pub(crate)` field access would NOT compile here.
// --------------------------------------------------------------------------

/// Split stdout on the wizard's `Generated config:` marker (wizard.rs:324)
/// and parse the trailing block as a `tome::config::Config`.
///
/// The `--dry-run` branch of the wizard runs `expand_tildes()` before emitting,
/// so the returned Config has absolute paths — tilde-relative comparisons do
/// NOT work; test callers must compare against expanded (TempDir-prefixed) paths.
fn parse_generated_config(stdout: &str) -> Config {
    let (_preamble, body) = stdout
        .split_once("Generated config:\n")
        .unwrap_or_else(|| panic!("missing `Generated config:` marker in stdout:\n{stdout}"));
    toml::from_str::<Config>(body)
        .unwrap_or_else(|e| panic!("generated TOML did not parse: {e}\n---\n{body}"))
}

/// Assert a Config round-trips: serialize, parse back, re-serialize, compare bytes.
/// Mirrors `Config::save_checked`'s round-trip guard (D-03).
fn assert_config_roundtrips(config: &Config) {
    let emitted = toml::to_string_pretty(config).expect("serialize Config");
    let reparsed: Config = toml::from_str(&emitted).expect("reparse Config");
    let reemitted = toml::to_string_pretty(&reparsed).expect("re-serialize Config");
    assert_eq!(
        emitted, reemitted,
        "Config round-trip mismatch — a field is not reversibly (de)serializable.\n\
         --- first emit ---\n{emitted}\n--- second emit ---\n{reemitted}",
    );
}

#[test]
fn init_dry_run_no_input_empty_home() {
    // HOME has nothing under it → no known directories auto-discovered.
    // Wizard should still complete and print a valid, empty-directories Config.
    let tmp = TempDir::new().unwrap();
    let tome_home = tmp.path().join(".tome");

    let output = tome()
        .args(["init", "--dry-run", "--no-input"])
        .env("HOME", tmp.path())
        .env("TOME_HOME", &tome_home)
        .env("NO_COLOR", "1")
        .output()
        .unwrap();

    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    assert!(
        output.status.success(),
        "tome init --dry-run --no-input failed (empty HOME).\nstdout:\n{stdout}\nstderr:\n{stderr}",
    );

    let config = parse_generated_config(&stdout);

    // 1. directories is empty (nothing auto-discovered). Use the pub accessor —
    //    the pub(crate) field is not reachable from this (external) test crate.
    assert!(
        config.directories().is_empty(),
        "expected empty directories on empty HOME, got: {:?}",
        config.directories().keys().collect::<Vec<_>>(),
    );

    // 2. library_dir is the expanded default (HOME/.tome/skills). Accessor returns &Path.
    assert_eq!(
        config.library_dir(),
        tmp.path().join(".tome/skills").as_path(),
        "library_dir should be <HOME>/.tome/skills after tilde expansion",
    );

    // 3. exclude set is empty (D-01 default).
    assert!(
        config.exclude().is_empty(),
        "expected empty exclude set, got: {:?}",
        config.exclude(),
    );

    // 4. Config::validate() passes (WHARD-05 acceptance criterion).
    config.validate().unwrap_or_else(|e| {
        panic!("generated config failed Config::validate(): {e:#}\nstdout:\n{stdout}")
    });

    // 5. TOML round-trip is byte-equal (WHARD-05 acceptance criterion).
    assert_config_roundtrips(&config);
}

#[test]
fn init_dry_run_no_input_seeded_home() {
    // Seed HOME with one managed known dir and one synced known dir.
    // Wizard should auto-discover both, assign the expected type+role, and the
    // resulting Config should validate + round-trip.
    let tmp = TempDir::new().unwrap();
    let tome_home = tmp.path().join(".tome");

    std::fs::create_dir_all(tmp.path().join(".claude/plugins")).unwrap();
    std::fs::create_dir_all(tmp.path().join(".claude/skills")).unwrap();

    let output = tome()
        .args(["init", "--dry-run", "--no-input"])
        .env("HOME", tmp.path())
        .env("TOME_HOME", &tome_home)
        .env("NO_COLOR", "1")
        .output()
        .unwrap();

    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    assert!(
        output.status.success(),
        "tome init --dry-run --no-input failed (seeded HOME).\nstdout:\n{stdout}\nstderr:\n{stderr}",
    );

    let config = parse_generated_config(&stdout);

    // 1. Both entries present. directories() returns &BTreeMap<DirectoryName, DirectoryConfig>.
    assert_eq!(
        config.directories().len(),
        2,
        "expected exactly 2 directories (claude-plugins + claude-skills), got {}: {:?}",
        config.directories().len(),
        config.directories().keys().collect::<Vec<_>>(),
    );

    // 2. claude-plugins entry: ClaudePlugins type, Managed role, expanded path.
    //    `DirectoryConfig::role` field is pub(crate) — use the pub `role()` method.
    //    `DirectoryConfig::path` and `DirectoryConfig::directory_type` are already pub.
    let plugins = config
        .directories()
        .get(&DirectoryName::new("claude-plugins").unwrap())
        .unwrap_or_else(|| panic!(
            "missing claude-plugins entry; got: {:?}",
            config.directories().keys().collect::<Vec<_>>(),
        ));
    assert_eq!(plugins.directory_type, DirectoryType::ClaudePlugins);
    assert_eq!(plugins.role(), DirectoryRole::Managed);
    assert_eq!(
        plugins.path,
        tmp.path().join(".claude/plugins"),
        "claude-plugins path should be <HOME>/.claude/plugins after tilde expansion",
    );

    // 3. claude-skills entry: Directory type, Synced role, expanded path.
    let skills = config
        .directories()
        .get(&DirectoryName::new("claude-skills").unwrap())
        .unwrap_or_else(|| panic!(
            "missing claude-skills entry; got: {:?}",
            config.directories().keys().collect::<Vec<_>>(),
        ));
    assert_eq!(skills.directory_type, DirectoryType::Directory);
    assert_eq!(skills.role(), DirectoryRole::Synced);
    assert_eq!(
        skills.path,
        tmp.path().join(".claude/skills"),
        "claude-skills path should be <HOME>/.claude/skills after tilde expansion",
    );

    // 4. library_dir is the expanded default. library_dir() returns &Path.
    assert_eq!(
        config.library_dir(),
        tmp.path().join(".tome/skills").as_path(),
        "library_dir should be <HOME>/.tome/skills after tilde expansion",
    );

    // 5. Config::validate() passes.
    config.validate().unwrap_or_else(|e| {
        panic!("generated config failed Config::validate(): {e:#}\nstdout:\n{stdout}")
    });

    // 6. TOML round-trip is byte-equal.
    assert_config_roundtrips(&config);
}
```

### Part B — Run CI equivalent

```bash
cd /Users/martin/dev/opensource/tome
cargo fmt
cargo clippy --all-targets -- -D warnings
cargo test -p tome --test cli init_dry_run_no_input_empty_home init_dry_run_no_input_seeded_home
```

Both tests must pass. The full `cargo test -p tome` must also pass (no integration regressions).

Do NOT:
- Use `insta::assert_snapshot!` — plain assertions satisfy the success criteria (D-07
  Claude's Discretion: snapshots are an upgrade, not required).
- Parse stdout manually with byte offsets or regex — `split_once("Generated config:\n")` is
  the contract per D-12.
- Assert on intermediate wizard prompt text (e.g. `Welcome to tome setup!`) — those lines are
  UI polish and may change in Phase 6 without breaking behavioral correctness.
- Seed additional known directories beyond `claude-plugins` and `claude-skills` — the seeded-home
  test's point is "multiple entries with distinct types+roles", not registry exhaustiveness
  (that's Plan 05-02's job).
- Unset or clear environment variables with `.env_clear()` — on macOS this breaks `cargo_bin_cmd`
  path resolution. Stick with explicit `.env("HOME", ...)` / `.env("TOME_HOME", ...)` / `.env("NO_COLOR", "1")`.
- Import `tome::*` broadly — only import the four symbols the tests use.
- Modify the `parse_generated_config` split marker string ever (it is the contract at
  `wizard.rs:324`).
- **Write `config.directories` or `config.library_dir` or `config.exclude` without the parens.**
  Those are `pub(crate)` fields; from this (external) test crate they are unreachable. Always
  use the accessor methods: `config.directories()`, `config.library_dir()`, `config.exclude()`.
- **Write `plugins.role` (field) instead of `plugins.role()` (method).** The field is
  `pub(crate)`; the method is `pub`.
  </action>
  <verify>
    <automated>cd /Users/martin/dev/opensource/tome && cargo fmt -- --check && cargo clippy --all-targets -- -D warnings && cargo test -p tome --test cli init_dry_run_no_input_empty_home init_dry_run_no_input_seeded_home && cargo test -p tome</automated>
  </verify>
  <acceptance_criteria>
    - `rg "fn init_dry_run_no_input_empty_home" crates/tome/tests/cli.rs` returns 1 hit
    - `rg "fn init_dry_run_no_input_seeded_home" crates/tome/tests/cli.rs` returns 1 hit
    - `rg "fn parse_generated_config" crates/tome/tests/cli.rs` returns 1 hit
    - `rg "fn assert_config_roundtrips" crates/tome/tests/cli.rs` returns 1 hit
    - `rg "use tome::config::" crates/tome/tests/cli.rs` returns 1 hit
    - `rg "\"Generated config:" crates/tome/tests/cli.rs` returns 1 hit (marker literal match)
    - `rg "\\.env\\(\"HOME\"" crates/tome/tests/cli.rs` returns at least 2 hits (one per test)
    - `rg "\\.env\\(\"TOME_HOME\"" crates/tome/tests/cli.rs` returns at least 2 hits
    - `rg "\\.env\\(\"NO_COLOR\", \"1\"\\)" crates/tome/tests/cli.rs -c` shows count ≥ 2 more than before (Plan 05-03 adds ≥2 usages)
    - `rg "config\\.directories\\(\\)" crates/tome/tests/cli.rs -c` returns at least 6 (empty-home test: 2 uses; seeded-home test: 4 uses in len, keys, get, keys)
    - `rg "config\\.library_dir\\(\\)" crates/tome/tests/cli.rs -c` returns at least 2 (one per test)
    - `rg "config\\.exclude\\(\\)" crates/tome/tests/cli.rs -c` returns at least 2 (two uses in empty-home test)
    - `rg "plugins\\.role\\(\\)" crates/tome/tests/cli.rs` returns 1 hit (DirectoryConfig::role() method call — NOT field access)
    - `rg "skills\\.role\\(\\)" crates/tome/tests/cli.rs` returns 1 hit (DirectoryConfig::role() method call — NOT field access)
    - `rg "config\\.directories[^(]" crates/tome/tests/cli.rs` returns 0 hits (catches accidental bare-field access without parens)
    - `rg "config\\.library_dir[^(]" crates/tome/tests/cli.rs` returns 0 hits
    - `rg "config\\.exclude[^(]" crates/tome/tests/cli.rs` returns 0 hits
    - `cargo test -p tome --test cli init_dry_run_no_input_empty_home` exits 0
    - `cargo test -p tome --test cli init_dry_run_no_input_seeded_home` exits 0
    - `cargo test -p tome` exits 0 (no regression in other integration tests)
    - `cargo clippy --all-targets -- -D warnings` exits 0
    - `cargo fmt -- --check` exits 0
  </acceptance_criteria>
  <done>
    `crates/tome/tests/cli.rs` has two new `#[test]` functions exercising `tome init --dry-run --no-input` on an empty HOME and a seeded HOME, plus a `parse_generated_config` helper and an `assert_config_roundtrips` helper. Both tests spawn the real `tome` binary, override HOME + TOME_HOME + NO_COLOR, parse the `Generated config:` block as `Config`, and assert `validate().is_ok()` + TOML round-trip equality. All Config state reads go through the `pub fn` accessors (`directories()`, `library_dir()`, `exclude()`) added by Plan 05-01 Part D — compatible with the external-crate boundary at `tests/cli.rs`. `make ci` clean. No production code touched in this plan.
  </done>
</task>

</tasks>

<verification>
Phase-exit checks for Plan 05-03:

1. `cd /Users/martin/dev/opensource/tome && cargo fmt -- --check` exits 0
2. `cd /Users/martin/dev/opensource/tome && cargo clippy --all-targets -- -D warnings` exits 0
3. `cd /Users/martin/dev/opensource/tome && cargo test -p tome` exits 0
4. `rg "init_dry_run_no_input_" crates/tome/tests/cli.rs` returns 2 function-definition hits (one per test fn name)
5. `rg "Generated config:" crates/tome/tests/cli.rs` returns 1 hit (the split marker literal)
6. `rg "config\\.directories\\(\\)" crates/tome/tests/cli.rs` returns at least 6 hits (accessor calls, not field access)
7. `rg "config\\.library_dir\\(\\)" crates/tome/tests/cli.rs` returns at least 2 hits
8. `rg "config\\.exclude\\(\\)" crates/tome/tests/cli.rs` returns at least 2 hits
</verification>

<success_criteria>
- WHARD-05 satisfied: one integration test per HOME shape (empty, seeded) runs `tome init --dry-run --no-input`, parses the generated TOML, asserts `Config::validate().is_ok()`, and asserts the config round-trips through TOML byte-equal.
- No reliance on the user's real HOME — both tests use `TempDir` + `HOME`/`TOME_HOME` env overrides.
- Parsing is marker-based (`Generated config:`) and future-proof: any reformatting of the wizard's pre-marker chatter does not break the tests.
- Test bodies respect the external-crate boundary at `tests/cli.rs`: every Config state read goes through the `pub fn` accessors added by Plan 05-01 Part D. No `pub(crate)` field access.
- Plan completes in a ~15-30 min execution window; two tests + two tiny helpers, no production code changes.
</success_criteria>

<output>
After completion, create `.planning/phases/05-wizard-test-coverage/05-03-init-integration-test-SUMMARY.md`.
</output>
</content>
</invoke>