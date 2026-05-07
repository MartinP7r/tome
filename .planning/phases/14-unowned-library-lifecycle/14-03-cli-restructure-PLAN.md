---
phase: 14-unowned-library-lifecycle
plan: 03
type: execute
wave: 2
depends_on:
  - 14-02
files_modified:
  - crates/tome/src/cli.rs
  - crates/tome/src/lib.rs
autonomous: true
requirements:
  - UNOWN-01
  - UNOWN-02

must_haves:
  truths:
    - "`tome remove dir <name>` runs today's directory-removal flow byte-for-byte (renamed only)."
    - "`tome remove skill <name>` is recognised by clap (the implementation stub returns an `unimplemented` error per D-API-2; 14-05 lands the real flow)."
    - "`tome reassign --force` is recognised by clap and threaded through to `reassign::plan` (consumer in 14-04)."
    - "Bare `tome remove <name>` (today's shape) no longer parses — clap reports a usage error (BREAKING CHANGE, called out in 14-08 CHANGELOG)."
  artifacts:
    - path: "crates/tome/src/cli.rs"
      provides: "RemoveKind enum + Command::Remove restructure + Reassign --force flag"
      contains: "pub enum RemoveKind"
    - path: "crates/tome/src/lib.rs"
      provides: "run() dispatch on Command::Remove { kind: RemoveKind::Dir | RemoveKind::Skill } and Reassign { ..., force }"
  key_links:
    - from: "cli::Command::Remove"
      to: "lib.rs::run match arm"
      via: "RemoveKind enum + nested clap subcommand"
      pattern: "Command::Remove \\{ kind \\}"
    - from: "cli::Command::Reassign.force"
      to: "reassign::plan"
      via: "force flag threaded into plan signature in 14-04"
---

<objective>
Restructure `Command::Remove` from a single-name variant into a nested clap
subcommand `Remove { kind: RemoveKind }` with `Dir { name, force }` and
`Skill { name, yes }` variants per D-API-2. Add `force: bool` flag to
`Command::Reassign` per D-A1. Update `lib.rs::run` dispatch:

- `Remove { kind: RemoveKind::Dir { name, force } }` calls today's
  `remove::plan/execute` flow (existing code, preserved verbatim, just
  re-keyed under the new arm).
- `Remove { kind: RemoveKind::Skill { name, yes } }` calls a new
  `remove::skill_plan/skill_render_plan/skill_execute` triple — but those
  functions DO NOT YET EXIST. This plan installs a stub that returns
  `anyhow::bail!("tome remove skill is not yet implemented — landed in 14-05")`
  so the dispatch arm compiles and the CLI structure is testable.
- `Reassign { ..., force }` threads the new `force` flag through to
  `reassign::plan` — but reassign.rs doesn't accept `force` yet. Plan 14-04
  lands the implementation. This plan changes the dispatch to PASS the flag,
  but `reassign::plan`'s signature stays (force is currently unused by the
  callee; clippy will warn — add `let _ = force;` shim or pass through
  with `#[allow(unused_variables)]` annotation explained in code comment).

Purpose: shapes the public CLI surface so plans 14-04 and 14-05 can land
behaviour against a stable contract. Breaking change to `tome remove <name>`
is acceptable per project policy "Backward compat: None" — CHANGELOG entry
in 14-08.

Output: clap accepts the new shapes; `tome remove dir <name>` works exactly
as today's `tome remove <name>` did; `tome remove skill <name>` returns the
"not yet implemented" error; `tome reassign --force` is parsed.
</objective>

<execution_context>
@$HOME/.claude/get-shit-done/workflows/execute-plan.md
@$HOME/.claude/get-shit-done/templates/summary.md
</execution_context>

<context>
@.planning/PROJECT.md
@.planning/ROADMAP.md
@.planning/phases/14-unowned-library-lifecycle/14-CONTEXT.md

# Source-of-truth pattern files:
@crates/tome/src/cli.rs
@crates/tome/src/lib.rs

<interfaces>
<!-- Today's clap shape for Command::Remove (cli.rs:174-182): -->
```rust
Remove {
    #[arg(value_name = "NAME")]
    name: String,
    #[arg(long)]
    force: bool,
},
```

<!-- Today's dispatch arm (lib.rs:411-515): preserves the entire 100-line -->
<!-- block; the only change is moving it under `RemoveKind::Dir`. -->

<!-- Today's clap shape for Command::Reassign (cli.rs:184-193): -->
```rust
Reassign {
    #[arg(value_name = "SKILL")]
    skill: String,
    #[arg(long)]
    to: String,
},
```

<!-- Pattern for nested clap subcommand — see Command::Backup at -->
<!-- cli.rs:241-282 (BackupCommand sibling enum). Replicate this shape. -->

<!-- D-API-2 verbatim from CONTEXT.md: -->
```rust
Remove {
    #[command(subcommand)]
    kind: RemoveKind,
}

enum RemoveKind {
    Dir { name: String, #[arg(long)] force: bool },
    Skill { name: String, #[arg(long, short)] yes: bool },
}
```
</interfaces>
</context>

<tasks>

<task type="auto">
  <name>Task 1: Restructure `Command::Remove` and add `Reassign --force` in cli.rs</name>
  <read_first>
    - crates/tome/src/cli.rs (entire file — particularly Command::Remove at lines 171-182, Command::Reassign at 184-193, Command::Backup at 241-249 as the nested-subcommand pattern reference, and BackupCommand enum at 251-282)
    - .planning/phases/14-unowned-library-lifecycle/14-CONTEXT.md (D-API-2 — exact clap shape; D-A1 — --force on reassign)
  </read_first>
  <action>
    1. **Replace the `Remove` variant** (cli.rs:171-182). Locate the block:

    ```rust
    /// Remove a directory entry and clean up its artifacts
    #[command(
        after_help = "Examples:\n  tome remove my-git-source\n  tome remove my-git-source --dry-run\n  tome remove my-git-source --force"
    )]
    Remove {
        /// Name of the directory to remove (as shown in `tome status`)
        #[arg(value_name = "NAME")]
        name: String,
        /// Skip confirmation prompt
        #[arg(long)]
        force: bool,
    },
    ```

    Replace with:

    ```rust
    /// Manage skills and directories — remove a configured directory entry
    /// or delete an Unowned skill from the library.
    #[command(
        after_help = "Examples:\n  tome remove dir my-git-source\n  tome remove dir my-git-source --force\n  tome remove skill orphaned-foo\n  tome remove skill orphaned-foo --yes"
    )]
    Remove {
        #[command(subcommand)]
        kind: RemoveKind,
    },
    ```

    2. **Add the `RemoveKind` enum** to cli.rs. Place it directly above the existing `BackupCommand` enum declaration (cli.rs:251) so the file groups subcommand-helper enums together:

    ```rust
    /// Variant of `tome remove` — directory removal vs unowned-skill deletion.
    /// Per D-API-2 (Phase 14): the merge replaces today's `tome remove <name>`
    /// shape (BREAKING). `tome remove dir` keeps today's directory-removal
    /// behaviour; `tome remove skill` deletes an Unowned skill from the library.
    #[derive(Debug, Subcommand)]
    pub enum RemoveKind {
        /// Remove a directory entry from `tome.toml` and clean up its artifacts
        /// (today's `tome remove <name>` behaviour, renamed). Owned skills
        /// transition to Unowned per LIB-04.
        Dir {
            /// Name of the directory to remove (as shown in `tome status`)
            #[arg(value_name = "NAME")]
            name: String,
            /// Skip confirmation prompt
            #[arg(long)]
            force: bool,
        },
        /// Delete an Unowned skill from the library — manifest entry, library
        /// directory, distribution symlinks, lockfile entry, and machine.toml
        /// membership all cleaned. Owned skills are refused with a hint to
        /// run `tome remove dir` first (D-B2).
        Skill {
            /// Skill name to forget (must currently be Unowned)
            #[arg(value_name = "NAME")]
            name: String,
            /// Skip confirmation prompt
            #[arg(long, short)]
            yes: bool,
        },
    }
    ```

    3. **Update the `Reassign` variant** (cli.rs:184-193). Replace with:

    ```rust
    /// Reassign a skill to a different directory. Accepts both Owned skills
    /// (today's behaviour) and Unowned skills (re-anchors them per UNOWN-01 /
    /// D-API-1). Refuses to overwrite an existing skill in the target with
    /// different content unless `--force` is passed (D-A1).
    #[command(
        after_help = "Examples:\n  tome reassign my-skill --to local-skills\n  tome reassign orphaned-foo --to local-skills\n  tome reassign my-skill --to local-skills --force"
    )]
    Reassign {
        /// Skill name to reassign
        #[arg(value_name = "SKILL")]
        skill: String,
        /// Target directory name
        #[arg(long)]
        to: String,
        /// Overwrite an existing skill in the target if its content hash
        /// differs from the library copy (D-A1). Same-content collisions
        /// always relink without `--force`.
        #[arg(long)]
        force: bool,
    },
    ```

    4. **Verify clap parses the new shapes** at the unit-test level by adding cli.rs unit tests. If `Cli` can be invoked via `Cli::try_parse_from`, add tests in a new `#[cfg(test)] mod tests` block at the bottom of cli.rs:

    ```rust
    #[cfg(test)]
    mod tests {
        use super::*;
        use clap::Parser;

        #[test]
        fn parse_remove_dir_with_force() {
            let cli = Cli::try_parse_from(["tome", "remove", "dir", "my-source", "--force"]).unwrap();
            match cli.command {
                Command::Remove { kind: RemoveKind::Dir { name, force } } => {
                    assert_eq!(name, "my-source");
                    assert!(force);
                }
                other => panic!("expected Remove::Dir, got {:?}", std::any::type_name_of_val(&other)),
            }
        }

        #[test]
        fn parse_remove_skill_with_yes() {
            let cli = Cli::try_parse_from(["tome", "remove", "skill", "orphan-foo", "--yes"]).unwrap();
            match cli.command {
                Command::Remove { kind: RemoveKind::Skill { name, yes } } => {
                    assert_eq!(name, "orphan-foo");
                    assert!(yes);
                }
                _ => panic!("expected Remove::Skill"),
            }
        }

        #[test]
        fn parse_remove_skill_short_y() {
            let cli = Cli::try_parse_from(["tome", "remove", "skill", "orphan-foo", "-y"]).unwrap();
            match cli.command {
                Command::Remove { kind: RemoveKind::Skill { yes, .. } } => assert!(yes),
                _ => panic!("expected Remove::Skill"),
            }
        }

        #[test]
        fn parse_reassign_force_flag_recognised() {
            let cli = Cli::try_parse_from(["tome", "reassign", "my-skill", "--to", "dst", "--force"]).unwrap();
            match cli.command {
                Command::Reassign { skill, to, force } => {
                    assert_eq!(skill, "my-skill");
                    assert_eq!(to, "dst");
                    assert!(force);
                }
                _ => panic!("expected Reassign"),
            }
        }

        #[test]
        fn old_shape_remove_with_bare_name_fails() {
            // Today's `tome remove my-source` should NO LONGER parse —
            // clap requires an explicit subcommand. BREAKING per D-API-2.
            let result = Cli::try_parse_from(["tome", "remove", "my-source"]);
            assert!(
                result.is_err(),
                "bare `tome remove <name>` must fail post-restructure (BREAKING)"
            );
        }
    }
    ```

    Note on the panic-arm `std::any::type_name_of_val` — if that's stabilised, use it; otherwise just `panic!("expected Remove::Dir")` is fine. Keep tests simple.
  </action>
  <verify>
    <automated>cargo test -p tome --lib cli::tests</automated>
  </verify>
  <acceptance_criteria>
    - `grep -q "pub enum RemoveKind" crates/tome/src/cli.rs` succeeds
    - `grep -q "Dir {" crates/tome/src/cli.rs` succeeds (RemoveKind::Dir variant)
    - `grep -q "Skill {" crates/tome/src/cli.rs` succeeds (RemoveKind::Skill variant)
    - `grep -q "kind: RemoveKind," crates/tome/src/cli.rs` succeeds (Command::Remove uses the nested subcommand)
    - `grep -q '#\[command(subcommand)\]' crates/tome/src/cli.rs` succeeds (the attribute on the kind field)
    - `grep -A8 "Reassign {" crates/tome/src/cli.rs | grep -q "force: bool"` succeeds
    - `cargo test -p tome --lib cli::tests::parse_remove_dir_with_force` exits 0
    - `cargo test -p tome --lib cli::tests::parse_remove_skill_with_yes` exits 0
    - `cargo test -p tome --lib cli::tests::parse_reassign_force_flag_recognised` exits 0
    - `cargo test -p tome --lib cli::tests::old_shape_remove_with_bare_name_fails` exits 0
  </acceptance_criteria>
  <done>
    cli.rs declares `RemoveKind` with `Dir` and `Skill` variants; `Command::Remove` uses the nested subcommand; `Command::Reassign` carries `force: bool`. 5 clap-parse tests pass.
  </done>
</task>

<task type="auto">
  <name>Task 2: Update `lib.rs::run` dispatch for the new shapes</name>
  <read_first>
    - crates/tome/src/lib.rs (lines 411-515 — the existing Command::Remove arm; lines 516-551 — the existing Command::Reassign arm)
    - crates/tome/src/cli.rs (after Task 1 — the new RemoveKind enum)
    - .planning/phases/14-unowned-library-lifecycle/14-CONTEXT.md (D-API-1 / D-API-2 dispatch expectations)
  </read_first>
  <action>
    1. **Locate the existing `Command::Remove { name, force }` match arm** in lib.rs (starts around line 411 with `Command::Remove { name, force } => {`). The arm body is ~104 lines long and includes plan/render/execute/save chain logic.

    2. **Replace the `match` arm head** with a nested-match shape:

    ```rust
    Command::Remove { kind } => match kind {
        cli::RemoveKind::Dir { name, force } => {
            // ENTIRE EXISTING BODY of the previous Command::Remove arm
            // goes here verbatim, with `name` and `force` already in scope
            // from the destructure.
            //
            // Do not modify the body — preserve byte-for-byte. Only the
            // outer `Command::Remove { name, force } =>` head is changed.
        }
        cli::RemoveKind::Skill { name, yes } => {
            // Stub for Phase 14 plan 14-05 to replace.
            let _ = (name, yes);
            anyhow::bail!(
                "tome remove skill is not yet implemented — see Phase 14 plan 14-05"
            );
        }
    },
    ```

    Concrete instruction: take the existing arm body (everything between the opening `{` after `Command::Remove { name, force } =>` and its matching closing `}`), insert it verbatim under the `RemoveKind::Dir { name, force }` arm of the new nested match.

    3. **Update the `Command::Reassign` arm** to destructure `force`. The current shape (lib.rs:516-551) is:

    ```rust
    Command::Reassign { skill, to } => {
        let mut manifest = manifest::load(paths.config_dir())?;
        let plan = reassign::plan(&skill, &to, &config, &paths, &manifest, false)?;
        // ... rest of arm
    }
    ```

    Change the destructure head to `Command::Reassign { skill, to, force } => {` and bind force locally so plan 14-04 can wire it. The `reassign::plan` signature still accepts `is_fork: bool` as the 6th arg (NOT force). For now, add `let _ = force;` immediately after the `let mut manifest = ...;` line with a comment:

    ```rust
    Command::Reassign { skill, to, force } => {
        let mut manifest = manifest::load(paths.config_dir())?;
        // force flag is consumed by reassign::plan in 14-04 (Phase 14
        // D-A1). Currently a placeholder; the call below still passes
        // is_fork=false. 14-04 changes reassign::plan's signature to
        // accept force.
        let _ = force;
        let plan = reassign::plan(&skill, &to, &config, &paths, &manifest, false)?;
        // ... rest of arm UNCHANGED
    }
    ```

    4. **Confirm the binary compiles** by running `cargo build -p tome`. The `let _ = force;` shim suppresses the unused-variable lint without `#[allow(...)]`.

    5. **Run the existing integration tests** in `tests/cli.rs` to confirm `tome remove dir <name>` works byte-for-byte the same as the previous `tome remove <name>`. Search for tests invoking `["remove", "<name>"]` and update them to `["remove", "dir", "<name>"]`. Use `rg '"remove"' crates/tome/tests/cli.rs` to find call sites. Each call site of the old shape must be migrated. Document in the task summary how many sites changed.
  </action>
  <verify>
    <automated>cargo test -p tome --test cli</automated>
  </verify>
  <acceptance_criteria>
    - `grep -q "Command::Remove { kind } => match kind" crates/tome/src/lib.rs` succeeds (or equivalent — `grep -q "Command::Remove { kind }" crates/tome/src/lib.rs` if the formatting is on multiple lines)
    - `grep -q "RemoveKind::Dir" crates/tome/src/lib.rs` succeeds
    - `grep -q "RemoveKind::Skill" crates/tome/src/lib.rs` succeeds
    - `grep -q "tome remove skill is not yet implemented" crates/tome/src/lib.rs` succeeds (the stub error message)
    - `grep -q "Command::Reassign { skill, to, force }" crates/tome/src/lib.rs` succeeds
    - `cargo build -p tome` exits 0
    - `cargo test -p tome --lib` exits 0 (all unit tests still pass)
    - `cargo test -p tome --test cli` exits 0 (all integration tests pass after the `remove dir` rename)
    - `cargo clippy --all-targets -p tome -- -D warnings` exits 0
  </acceptance_criteria>
  <done>
    `lib.rs::run` dispatches the new clap shape: `Remove::Dir` runs the existing flow byte-for-byte; `Remove::Skill` stubs to the "not yet implemented" error; `Reassign` destructures `force` (held for 14-04). Existing integration tests updated to use `tome remove dir <name>` and pass.
  </done>
</task>

</tasks>

<verification>
- `cargo test -p tome` exits 0 (all tests pass)
- `cargo clippy --all-targets -p tome -- -D warnings` exits 0
- `tome --help` (running the built binary) shows `remove` with subcommands `dir` and `skill`; `tome remove --help` lists both
- Integration tests in `tests/cli.rs` updated to use `["remove", "dir", "<name>"]`
</verification>

<success_criteria>
- New clap shape parses: `tome remove dir`, `tome remove skill`, `tome reassign --force`.
- Old shape `tome remove <name>` (no subcommand) errors out (BREAKING — documented in 14-08 CHANGELOG).
- Behaviour for `tome remove dir <name>` is byte-for-byte the same as today's `tome remove <name>`.
- `tome remove skill <name>` returns `anyhow::bail!("tome remove skill is not yet implemented — see Phase 14 plan 14-05")` exit code 1; full implementation lands in 14-05.
- Existing integration tests in `tests/cli.rs` migrated and passing.
</success_criteria>

<output>
After completion, create `.planning/phases/14-unowned-library-lifecycle/14-03-SUMMARY.md`
</output>
