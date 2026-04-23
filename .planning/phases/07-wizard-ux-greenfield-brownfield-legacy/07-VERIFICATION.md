---
phase: 07-wizard-ux-greenfield-brownfield-legacy
verified: 2026-04-23T00:00:00Z
status: passed
score: 5/5 must-haves verified
---

# Phase 7: Wizard UX (Greenfield / Brownfield / Legacy) Verification Report

**Phase Goal:** `tome init` behaves predictably on any machine state — fresh install, dotfiles-synced home, or pre-v0.6 cruft — and tells the user which `tome_home` it is about to populate
**Verified:** 2026-04-23
**Status:** passed
**Re-verification:** No — initial verification

## Goal Achievement

### Observable Truths (Success Criteria from ROADMAP.md lines 45-49)

| #   | Truth                                                                                                                         | Status     | Evidence                                                                                                                                                                        |
| --- | ----------------------------------------------------------------------------------------------------------------------------- | ---------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| 1   | Greenfield: user sees tome_home prompt with `~/.tome/` default + validated custom-path option                                | VERIFIED   | wizard.rs Step 0 block at L161-219 gates on `TomeHomeSource::Default && !no_input`; validator rejects non-absolute / non-directory paths; 4 integration tests + 1 unit test cover behavior |
| 2   | Brownfield: user sees summary + 4-way decision (use/edit/reinit/cancel) — no silent overwrite                                 | VERIFIED   | `brownfield_decision` at wizard.rs:962 emits summary (path + directories count + library_dir + last-modified); lib.rs:214-244 dispatches all 4 arms; 3 integration tests + 3 unit tests |
| 3   | Legacy: warning + delete-or-move-aside action — no silent ignore, no auto-delete                                              | VERIFIED   | `handle_legacy_cleanup` at wizard.rs:867 prints yellow warning + offers Leave/Move-aside/Delete; `--no-input` leaves file alone with `note:` on stderr; 3 integration tests + 7 unit tests for false-positive protection |
| 4   | Every `tome init` prints 1-line "resolved tome_home: <path>" BEFORE Step 1 prompts                                            | VERIFIED   | lib.rs:178-183 prints `resolved tome_home: <path> (from <source>)` in both interactive and `--no-input` modes; 4 integration tests including ordering invariant; manual spot-check PASS |
| 5   | Custom greenfield tome_home is persisted to `~/.config/tome/config.toml` on user's request                                    | VERIFIED   | `write_xdg_tome_home` at config.rs:671 does atomic merge-preserve temp+rename; wizard.rs:210 calls it after Confirm prompt; 3 unit tests lock in create/preserve/atomic behavior |

**Score:** 5/5 truths verified

### Required Artifacts

| Artifact                         | Expected                                                                     | Status    | Details                                                                                                                                      |
| -------------------------------- | ---------------------------------------------------------------------------- | --------- | -------------------------------------------------------------------------------------------------------------------------------------------- |
| `crates/tome/src/config.rs`      | `TomeHomeSource` enum + `resolve_tome_home_with_source` + `write_xdg_tome_home` + `read_config_tome_home` visibility | VERIFIED  | `pub(crate) enum TomeHomeSource` at L733; `pub(crate) fn resolve_tome_home_with_source` at L774; `pub(crate) fn write_xdg_tome_home` at L671; `pub(crate) fn read_config_tome_home` at L637 |
| `crates/tome/src/wizard.rs`      | `MachineState` + `detect_machine_state` + `has_legacy_sections` + `handle_legacy_cleanup` + `BrownfieldAction` + `brownfield_decision` + `backup_brownfield_config` + 5-arg `wizard::run` + Step 0 + prefill plumbing | VERIFIED  | `MachineState` at L773, `detect_machine_state` at L803, `has_legacy_sections` at L836, `handle_legacy_cleanup` at L867, `BrownfieldAction` at L943, `brownfield_decision` at L962, `backup_brownfield_config` at L1061, `wizard::run` 5-arg at L137, Step 0 at L161-219, prefill threaded through `configure_directories` (L531), `configure_library` (L623), `configure_exclusions` (L670) |
| `crates/tome/src/lib.rs`         | Command::Init dispatch (info line → legacy → brownfield match → wizard::run)  | VERIFIED  | L169-253: prints `resolved tome_home:` at L178-183, calls `detect_machine_state` at L189, matches `Legacy`/`BrownfieldWithLegacy` at L190-194, matches `Brownfield`/`BrownfieldWithLegacy` dispatching all 4 `BrownfieldAction` arms at L204-245, passes `prefill.as_ref()` to `wizard::run` at L247-253 |
| `crates/tome/tests/cli.rs`       | Integration tests for greenfield / brownfield / legacy / info-line         | VERIFIED  | 14 new init_* tests: 4 WUX-04 (default/env/flag source + ordering), 3 WUX-03 (legacy detect/skip/greenfield-clean), 4 WUX-01/05 (step 0 skip, no XDG write, flag-source skip, library derivation), 3 WUX-02 (brownfield keep-existing / invalid-cancels / with-legacy) |

### Key Link Verification

| From                                      | To                                       | Via                                                                           | Status   | Details                                                                                                                                                 |
| ----------------------------------------- | ---------------------------------------- | ----------------------------------------------------------------------------- | -------- | ------------------------------------------------------------------------------------------------------------------------------------------------------- |
| lib.rs Command::Init                      | config::resolve_tome_home_with_source    | function call returning (PathBuf, TomeHomeSource)                             | WIRED    | lib.rs:177 — `config::resolve_tome_home_with_source(cli.tome_home.as_deref(), cli.config.as_deref())?`                                                  |
| lib.rs                                    | stdout                                    | `println!` with `source.label()`                                              | WIRED    | lib.rs:179-183 — `println!("resolved tome_home: {} (from {})", ..., tome_home_source.label());`                                                         |
| lib.rs Command::Init                      | wizard::detect_machine_state              | function call after WUX-04 print                                              | WIRED    | lib.rs:189 — `wizard::detect_machine_state(&home, &tome_home)?`                                                                                          |
| wizard::handle_legacy_cleanup             | `~/.config/tome/config.toml`              | `std::fs::rename` or `std::fs::remove_file`                                   | WIRED    | wizard.rs:909 uses `std::fs::rename` for move-aside with `config.toml.legacy-backup-<ts>`; wizard.rs:921 uses `std::fs::remove_file` for delete         |
| lib.rs Command::Init                      | wizard::run                              | new 5-arg signature                                                           | WIRED    | lib.rs:247-253 — `wizard::run(cli.dry_run, cli.no_input, &tome_home, tome_home_source, prefill.as_ref())?`                                             |
| wizard::run Step 0                        | config::write_xdg_tome_home              | persist prompt                                                                | WIRED    | wizard.rs:211 — `crate::config::write_xdg_tome_home(&chosen_tome_home)?;`                                                                               |
| wizard::run save path                     | resolve_config_dir(&tome_home)           | replaces default_config_path()                                                | WIRED    | wizard.rs:383 — `let config_path = crate::config::resolve_config_dir(tome_home).join("tome.toml");` (only reference to `default_config_path()` in wizard.rs is in a comment) |
| wizard::configure_library default         | `<tome_home>/skills`                     | `collapse_home_path(&tome_home.join("skills"))`                              | WIRED    | wizard.rs:628 — `let default = crate::paths::collapse_home_path(&tome_home.join("skills"));`                                                            |
| lib.rs brownfield dispatch                | wizard::backup_brownfield_config         | Reinit action calls backup before fresh wizard                                | WIRED    | lib.rs:229 — `let backup = wizard::backup_brownfield_config(existing_config_path)?;`                                                                     |
| wizard::configure_directories             | prefill entries                          | `entry().or_insert_with` union preserves custom directories                   | WIRED    | wizard.rs union inside configure_directories (Pitfall 2 fix); locked by `configure_directories_preserves_custom_prefill` unit test                      |

### Data-Flow Trace (Level 4)

| Artifact                           | Data Variable              | Source                                            | Produces Real Data | Status    |
| ---------------------------------- | -------------------------- | ------------------------------------------------- | ------------------ | --------- |
| lib.rs Command::Init info line    | `tome_home_source`         | `resolve_tome_home_with_source` (5-branch enum)   | Yes — one of 5 labels | FLOWING   |
| lib.rs brownfield dispatch        | `machine_state`            | `detect_machine_state` (probes `tome.toml` + XDG) | Yes — enum variant based on filesystem | FLOWING |
| lib.rs prefill                    | `Option<Config>`           | `existing_config.clone()` from MachineState       | Yes — cloned Config when Edit chosen | FLOWING |
| wizard::run library_dir           | `PathBuf`                  | `configure_library(no_input, tome_home, prefill)` | Yes — prefill or derived default | FLOWING |
| wizard::run directories           | `BTreeMap<DirectoryName,..>` | `configure_directories` with prefill union     | Yes — discovered + prefill merged | FLOWING |
| config.rs XDG write tome_home    | `PathBuf`                  | `chosen_tome_home` expanded from user input       | Yes — collapsed to `~/`-form and written | FLOWING |

### Behavioral Spot-Checks

| Behavior                                           | Command                                                                                                                            | Result                                                                    | Status |
| -------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------- | ------ |
| `tome init` prints resolved tome_home info line    | `HOME=$TMP TOME_HOME=$TMP/.tome NO_COLOR=1 ./target/debug/tome init --dry-run --no-input`                                           | `resolved tome_home: /var/folders/.../.tome (from TOME_HOME env)` printed before Step 1 | PASS   |
| `cargo fmt -- --check` passes                      | `cargo fmt -- --check`                                                                                                             | Clean (no output, exit 0)                                                 | PASS   |
| `cargo clippy --all-targets -- -D warnings` passes | `cargo clippy --all-targets -- -D warnings`                                                                                        | Clean (`Finished` exit 0)                                                 | PASS   |
| Full `cargo test --package tome` passes            | `cargo test --package tome`                                                                                                        | 450 unit + 122 integration = 572 tests passed; 0 failed                   | PASS   |
| All 21 `init_*` integration tests pass             | `cargo test --package tome --test cli -- init_`                                                                                    | 21 passed, 0 failed (includes all 14 new Phase 7 tests)                   | PASS   |

### Requirements Coverage

| Requirement | Source Plan | Description                                                                                                 | Status    | Evidence                                                                                                                       |
| ----------- | ----------- | ----------------------------------------------------------------------------------------------------------- | --------- | ------------------------------------------------------------------------------------------------------------------------------ |
| WUX-01      | 07-03       | Greenfield tome_home prompt (default `~/.tome/`, validated custom)                                          | SATISFIED | wizard.rs:170 Step 0 gate; validator in `validate_with` at L187-198; 4 integration tests                                       |
| WUX-02      | 07-04       | Brownfield summary + 4-way decision (use/edit/reinit/cancel)                                                | SATISFIED | wizard.rs:962 `brownfield_decision` with 4-option Select; lib.rs:220-242 dispatches all 4 arms; backup in Reinit; 3 integration tests |
| WUX-03      | 07-02       | Legacy detection + delete-or-move-aside prompt (no silent ignore, no auto-delete)                           | SATISFIED | wizard.rs:867 `handle_legacy_cleanup` with Leave/Move-aside/Delete; parse-based detection (not substring) at wizard.rs:843; 3 integration + 7 unit tests |
| WUX-04      | 07-01       | `resolved tome_home:` info line before Step 1 prompts                                                       | SATISFIED | lib.rs:178-183 prints in both interactive and `--no-input`; `init_resolved_tome_home_line_precedes_step_prompts` locks ordering |
| WUX-05      | 07-03       | Persist custom tome_home choice to `~/.config/tome/config.toml`                                             | SATISFIED | config.rs:671 `write_xdg_tome_home` with atomic temp+rename + merge-preserve; wizard.rs:211 calls after Confirm prompt; 3 unit tests |

No orphaned requirements — all 5 WUX-* requirements declared in phase plan frontmatter are implemented AND listed in REQUIREMENTS.md as checked `[x]`.

### Anti-Patterns Found

No blocker or warning anti-patterns. The code:
- Uses parse-based legacy detection (`toml::Table`), not substring matching
- Returns `Ok(None)` gracefully on malformed TOML (no panic)
- Uses copy-not-rename for brownfield backup (Cancel-safe)
- Validates custom tome_home paths (absolute, directory-or-nonexistent)
- Defaults to non-destructive actions under `--no-input` (leave legacy, UseExisting brownfield, Cancel on invalid)
- Keeps `unreachable!()` assertion in lib.rs Edit arm to fail fast on menu refactors

One pre-existing `TODO` note: `typos` CLI not installed (documented in 07-04-SUMMARY.md as out-of-scope; affects `make ci` only, not fmt/clippy/test gates). Not introduced by this phase.

### Human Verification Required

None at the automated level. Interactive menu branches (Move-aside / Delete for legacy; Edit / Reinit for brownfield; custom tome_home validation prompt) are tested at the no-input layer + unit-test layer per the RESEARCH.md Pitfall 5 rule (no dialoguer interactive tests in CI). A manual smoke-test script is provided in 07-04-SUMMARY.md lines 150-167 but is NOT required for verification — the headless paths cover the observable behaviors.

### Gaps Summary

No gaps. Every observable truth has a passing automated test backing it. Every must-have artifact exists at the expected location with the expected symbols. Every key link is wired correctly. All 5 WUX requirements are accounted for with implementation evidence in both the code and the tests. `cargo fmt -- --check`, `cargo clippy --all-targets -- -D warnings`, and the full `cargo test --package tome` suite (572 tests) pass cleanly.

The phase goal — `tome init` behaves predictably on any machine state (greenfield / brownfield / legacy) and surfaces the resolved `tome_home` up front — is demonstrably achieved.

---

_Verified: 2026-04-23_
_Verifier: Claude (gsd-verifier)_
