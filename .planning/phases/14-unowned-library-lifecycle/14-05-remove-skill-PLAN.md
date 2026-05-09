---
phase: 14-unowned-library-lifecycle
plan: 05
type: execute
wave: 3
depends_on:
  - 14-01
  - 14-03
files_modified:
  - crates/tome/src/remove.rs
  - crates/tome/src/lib.rs
autonomous: true
requirements:
  - UNOWN-02

must_haves:
  truths:
    - "`tome remove skill <unowned-name>` deletes the manifest entry, the library directory, distribution symlinks, the lockfile entry, and machine.toml memberships, in one atomic-save flow (D-B1)."
    - "`tome remove skill <owned-name>` is refused with a clear error directing the user to `tome remove dir` or filesystem deletion + sync (D-B2)."
    - "Interactive confirmation defaults to `n`; `--yes` / `-y` skips the prompt (D-B3)."
    - "Partial failures aggregate via SAFE-01 pattern using a new `RemoveSkillFailureKind` enum with `ALL` array + compile-time exhaustiveness assertion (Phase 10 POLISH-04)."
  artifacts:
    - path: "crates/tome/src/remove.rs"
      provides: "RemoveSkillPlan + RemoveSkillFailureKind + RemoveSkillFailure + skill_plan/skill_render_plan/skill_execute triple"
      contains: "pub(crate) enum RemoveSkillFailureKind"
      min_lines: 200
    - path: "crates/tome/src/lib.rs"
      provides: "RemoveKind::Skill arm dispatches to skill_plan/skill_execute (replaces the 14-03 stub)"
  key_links:
    - from: "lib.rs::run RemoveKind::Skill arm"
      to: "remove::skill_plan, remove::skill_execute"
      via: "plan/render/execute pattern + atomic save chain"
    - from: "remove::skill_execute"
      to: "lockfile::save, machine::save, manifest::save"
      via: "all three saved atomically at end of execute on full success"
---

<objective>
Deliver UNOWN-02 by implementing `tome remove skill <name>` with the full
cleanup scope per D-B1: manifest entry + library directory + distribution
symlinks + lockfile entry + machine.toml `disabled` membership + machine.toml
per-directory `enabled`/`disabled` lists. Failures aggregate via a new
`RemoveSkillFailureKind` enum following the Phase 8 SAFE-01 + Phase 10
POLISH-04 patterns. Refuses Owned skills (D-B2) with an actionable hint;
default-no confirmation (D-B3) bypassed by `--yes` / `-y`.

Replaces the `tome remove skill is not yet implemented` stub from 14-03.

Purpose: explicitly delete an Unowned skill that the user no longer wants,
covering all per-machine state so machine B's next sync doesn't surprise the
user with a phantom RECON-02 install of a skill they forgot.

Output: new `skill_plan` / `skill_render_plan` / `skill_execute` triple in
remove.rs (mirroring the existing `dir`-flavour triple); new
`RemoveSkillFailureKind` + `RemoveSkillFailure` types with the same
compile-time + runtime guards as `FailureKind`; lib.rs dispatch wires them.
</objective>

<execution_context>
@$HOME/.claude/get-shit-done/workflows/execute-plan.md
@$HOME/.claude/get-shit-done/templates/summary.md
</execution_context>

<context>
@.planning/PROJECT.md
@.planning/ROADMAP.md
@.planning/phases/14-unowned-library-lifecycle/14-CONTEXT.md
@.planning/phases/14-unowned-library-lifecycle/14-01-previous-source-schema-PLAN.md
@.planning/phases/14-unowned-library-lifecycle/14-03-cli-restructure-PLAN.md

# Source-of-truth pattern files:
@crates/tome/src/remove.rs
@crates/tome/src/cleanup.rs
@crates/tome/src/manifest.rs
@crates/tome/src/lockfile.rs
@crates/tome/src/machine.rs
@crates/tome/src/lib.rs

<interfaces>
<!-- The shape we mirror — existing FailureKind enum (remove.rs:62-115): -->
```rust
pub(crate) enum FailureKind {
    DistributionSymlink,
    GitCache,
}

impl FailureKind {
    pub(crate) const ALL: [FailureKind; 2] = [FailureKind::DistributionSymlink, FailureKind::GitCache];
    pub(crate) fn label(self) -> &'static str { ... }
}

#[allow(dead_code)]
const fn _ensure_failure_kind_all_exhaustive(k: FailureKind) -> usize { ... }

const _: () = { assert!(FailureKind::ALL.len() == 2); };
```

<!-- And RemoveFailure struct (remove.rs:118-146): -->
```rust
pub(crate) struct RemoveFailure {
    pub path: PathBuf,
    pub kind: FailureKind,
    pub error: std::io::Error,
}

impl RemoveFailure {
    pub(crate) fn new(kind: FailureKind, path: PathBuf, error: std::io::Error) -> Self {
        debug_assert!(path.is_absolute(), ...);
        ...
    }
}
```

<!-- D-B1 cleanup scope (verbatim): -->
1. manifest[name] entry (Manifest::remove)
2. library_dir/<name>/ directory tree
3. Distribution symlinks for the skill in every distribution-role directory
4. tome.lock entry for the skill
5. machine.toml::disabled set membership (if present)
6. machine.toml::directories.<dir>.enabled and .disabled list memberships

<!-- D-B2 verbatim error message: -->
```
error: skill 'foo' is owned by directory 'bar' (source_name = bar).
Remove the source directory with `tome remove dir bar` first, or
remove the file from disk and re-sync.
```

<!-- D-B3 verbatim prompt: -->
```
Are you sure you want to forget skill 'foo'? [y/N]
```

<!-- machine.rs::MachinePrefs internal field shapes (machine.rs:69-95) -->
- `disabled: BTreeSet<SkillName>` (pub(crate))
- `directory: BTreeMap<DirectoryName, DirectoryPrefs>` where DirectoryPrefs has `disabled: BTreeSet<SkillName>` and `enabled: Option<BTreeSet<SkillName>>`. All fields pub(crate).

<!-- The fields are pub(crate) — accessible from remove.rs because both -->
<!-- live in the same crate. Mutation pattern: machine_prefs.disabled.remove(&skill_name). -->
</interfaces>
</context>

<tasks>

<task type="auto" tdd="true">
  <name>Task 1: Add `RemoveSkillFailureKind` + `RemoveSkillFailure` + `RemoveSkillPlan` types</name>
  <read_first>
    - crates/tome/src/remove.rs (entire file — particularly FailureKind at 62-115, the const-fn drift guard, RemoveFailure at 118-146, and the test module's failure_kind_* tests at 637-691)
    - crates/tome/src/manifest.rs (Manifest::remove method)
    - .planning/phases/14-unowned-library-lifecycle/14-CONTEXT.md (D-B1 cleanup scope, D-B2/D-B3 messages, "Discretion" recommendation about variants: LibraryDir, DistributionSymlink, Lockfile, MachineToml)
  </read_first>
  <behavior>
    - Test 1: `RemoveSkillFailureKind::ALL.len() == 4` and contains every variant exactly once.
    - Test 2: `RemoveSkillFailureKind::label()` produces the user-visible label for each variant.
    - Test 3 (compile-time): adding a new variant without growing `ALL` fails to compile (validated indirectly via the same const-fn pattern + `const _: () = { assert!(...) };`).
    - Test 4: `RemoveSkillFailure::new` debug-assertion on absolute paths (mirror Phase 10 POLISH-05).
  </behavior>
  <action>
    1. **Add `RemoveSkillFailureKind` enum to remove.rs.** Insert directly below the `FailureKind` block (after line 115, before line 117 where `RemoveFailure` starts):

    ```rust
    /// Which step of `tome remove skill` produced a partial-cleanup failure.
    ///
    /// Variants follow D-B1 cleanup scope:
    /// 1. `LibraryDir` — `remove_dir_all` failed on `library_dir/<name>/`
    /// 2. `DistributionSymlink` — `remove_file` failed on a per-skill
    ///    distribution symlink in some Target/Synced directory
    /// 3. `Lockfile` — `lockfile::save` failed after removing the entry
    /// 4. `MachineToml` — `machine::save` failed after removing memberships
    ///
    /// Manifest mutation is in-memory and saves last; if `manifest::save`
    /// fails the error propagates via `?` and never lands here. The aggregate
    /// failure-summary semantic only kicks in for filesystem-touch steps that
    /// need group reporting (Phase 8 SAFE-01).
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub(crate) enum RemoveSkillFailureKind {
        LibraryDir,
        DistributionSymlink,
        Lockfile,
        MachineToml,
    }

    impl RemoveSkillFailureKind {
        /// All variants, in the order preferred for user-facing grouped output.
        pub(crate) const ALL: [RemoveSkillFailureKind; 4] = [
            RemoveSkillFailureKind::LibraryDir,
            RemoveSkillFailureKind::DistributionSymlink,
            RemoveSkillFailureKind::Lockfile,
            RemoveSkillFailureKind::MachineToml,
        ];

        pub(crate) fn label(self) -> &'static str {
            match self {
                RemoveSkillFailureKind::LibraryDir => "Library directory",
                RemoveSkillFailureKind::DistributionSymlink => "Distribution symlinks",
                RemoveSkillFailureKind::Lockfile => "Lockfile",
                RemoveSkillFailureKind::MachineToml => "Machine prefs",
            }
        }
    }

    /// Compile-time drift guard for `RemoveSkillFailureKind::ALL` (POLISH-04 option c).
    /// Mirrors `_ensure_failure_kind_all_exhaustive` for `FailureKind`.
    #[allow(dead_code)]
    const fn _ensure_remove_skill_failure_kind_all_exhaustive(k: RemoveSkillFailureKind) -> usize {
        match k {
            RemoveSkillFailureKind::LibraryDir => 0,
            RemoveSkillFailureKind::DistributionSymlink => 1,
            RemoveSkillFailureKind::Lockfile => 2,
            RemoveSkillFailureKind::MachineToml => 3,
        }
    }

    const _: () = {
        // If this fails: RemoveSkillFailureKind::ALL is missing or has extra
        // variants. The match arms in
        // _ensure_remove_skill_failure_kind_all_exhaustive are the source
        // of truth — ALL must contain exactly one entry per arm.
        assert!(RemoveSkillFailureKind::ALL.len() == 4);
    };

    /// A single partial-cleanup failure aggregated from `skill_execute`.
    /// Mirror of `RemoveFailure` for the `skill` flavour.
    #[derive(Debug)]
    pub(crate) struct RemoveSkillFailure {
        pub path: PathBuf,
        pub kind: RemoveSkillFailureKind,
        pub error: std::io::Error,
    }

    impl RemoveSkillFailure {
        pub(crate) fn new(
            kind: RemoveSkillFailureKind,
            path: PathBuf,
            error: std::io::Error,
        ) -> Self {
            debug_assert!(
                path.is_absolute(),
                "RemoveSkillFailure::path must be absolute, got: {}",
                path.display()
            );
            RemoveSkillFailure { kind, path, error }
        }
    }
    ```

    2. **Add `RemoveSkillPlan` struct to remove.rs**, immediately after the `RemoveSkillFailure` block:

    ```rust
    /// What `tome remove skill <name>` will do (per D-B1).
    #[derive(Debug)]
    pub(crate) struct RemoveSkillPlan {
        /// Skill name being deleted.
        pub skill_name: SkillName,
        /// Library directory path (`library_dir/<skill_name>/`). Absolute.
        pub library_path: PathBuf,
        /// Distribution symlinks pointing at this skill in target/synced dirs.
        /// Each path is absolute.
        pub symlinks_to_remove: Vec<PathBuf>,
        /// Whether the skill has a lockfile entry that needs deleting.
        pub has_lockfile_entry: bool,
        /// Whether the skill is in `machine.toml::disabled`.
        pub in_machine_disabled: bool,
        /// Per-directory machine.toml memberships to clean (directory_name, in_enabled, in_disabled).
        /// Empty when the skill isn't referenced by any per-directory list.
        pub per_directory_memberships: Vec<(DirectoryName, bool, bool)>,
    }
    ```

    Use `crate::discover::SkillName` for the import — already in scope as `crate::discover::SkillName`.

    3. **Add unit tests for the new types** (in `#[cfg(test)] mod tests`):

    ```rust
    #[test]
    fn remove_skill_failure_kind_all_pinned_size_four() {
        assert_eq!(RemoveSkillFailureKind::ALL.len(), 4);
        assert!(RemoveSkillFailureKind::ALL.contains(&RemoveSkillFailureKind::LibraryDir));
        assert!(RemoveSkillFailureKind::ALL.contains(&RemoveSkillFailureKind::DistributionSymlink));
        assert!(RemoveSkillFailureKind::ALL.contains(&RemoveSkillFailureKind::Lockfile));
        assert!(RemoveSkillFailureKind::ALL.contains(&RemoveSkillFailureKind::MachineToml));
    }

    #[test]
    fn remove_skill_failure_kind_label_coverage() {
        assert_eq!(RemoveSkillFailureKind::LibraryDir.label(), "Library directory");
        assert_eq!(RemoveSkillFailureKind::DistributionSymlink.label(), "Distribution symlinks");
        assert_eq!(RemoveSkillFailureKind::Lockfile.label(), "Lockfile");
        assert_eq!(RemoveSkillFailureKind::MachineToml.label(), "Machine prefs");
    }

    #[test]
    fn remove_skill_failure_kind_all_unique() {
        let all = RemoveSkillFailureKind::ALL;
        for (i, a) in all.iter().enumerate() {
            for b in all.iter().skip(i + 1) {
                assert_ne!(a, b, "ALL contains duplicate variant {a:?}");
            }
        }
    }

    #[test]
    fn remove_skill_failure_new_relative_path_panics_in_debug() {
        let result = std::panic::catch_unwind(|| {
            RemoveSkillFailure::new(
                RemoveSkillFailureKind::LibraryDir,
                PathBuf::from("relative/path"),
                std::io::Error::other("test"),
            )
        });
        if cfg!(debug_assertions) {
            assert!(result.is_err());
        } else {
            assert!(result.is_ok());
        }
    }
    ```
  </action>
  <verify>
    <automated>cargo test -p tome --lib remove::tests::remove_skill_failure_kind_all_pinned_size_four remove::tests::remove_skill_failure_kind_label_coverage remove::tests::remove_skill_failure_kind_all_unique</automated>
  </verify>
  <acceptance_criteria>
    - `grep -q "pub(crate) enum RemoveSkillFailureKind" crates/tome/src/remove.rs` succeeds
    - All 4 variants present: `grep -q "RemoveSkillFailureKind::LibraryDir" crates/tome/src/remove.rs && grep -q "RemoveSkillFailureKind::DistributionSymlink" crates/tome/src/remove.rs && grep -q "RemoveSkillFailureKind::Lockfile" crates/tome/src/remove.rs && grep -q "RemoveSkillFailureKind::MachineToml" crates/tome/src/remove.rs` succeeds
    - `grep -q "pub(crate) const ALL: \[RemoveSkillFailureKind; 4\]" crates/tome/src/remove.rs` succeeds
    - `grep -q "_ensure_remove_skill_failure_kind_all_exhaustive" crates/tome/src/remove.rs` succeeds (the compile-time guard)
    - `grep -q "assert!(RemoveSkillFailureKind::ALL.len() == 4)" crates/tome/src/remove.rs` succeeds (the const _: () pin)
    - `grep -q "pub(crate) struct RemoveSkillPlan" crates/tome/src/remove.rs` succeeds
    - `grep -q "pub(crate) struct RemoveSkillFailure" crates/tome/src/remove.rs` succeeds
    - `cargo test -p tome --lib remove::tests::remove_skill_failure_kind_all_pinned_size_four` exits 0
    - `cargo test -p tome --lib remove::tests::remove_skill_failure_kind_label_coverage` exits 0
    - `cargo test -p tome --lib remove::tests::remove_skill_failure_kind_all_unique` exits 0
  </acceptance_criteria>
  <done>
    `RemoveSkillFailureKind` (4 variants), `RemoveSkillFailure`, and `RemoveSkillPlan` types exist with compile-time exhaustiveness assertion + runtime tests. Mirrors `FailureKind` shape exactly.
  </done>
</task>

<task type="auto" tdd="true">
  <name>Task 2: Implement `skill_plan` / `skill_render_plan` / `skill_execute` triple in remove.rs</name>
  <read_first>
    - crates/tome/src/remove.rs (entire file — both the existing `plan`/`render_plan`/`execute` triple AND the new types added in Task 1)
    - crates/tome/src/manifest.rs (Manifest::remove)
    - crates/tome/src/lockfile.rs (Lockfile struct, save())
    - crates/tome/src/machine.rs (MachinePrefs.disabled, .directory; pub(crate) field access)
    - crates/tome/src/config.rs (DirectoryRole.is_distribution())
    - .planning/phases/14-unowned-library-lifecycle/14-CONTEXT.md (D-B1, D-B2 — verbatim error text)
  </read_first>
  <behavior>
    - Test 1 (D-B2 owned guard): `skill_plan("owned-skill", ...)` where the skill has `source_name = Some(_)` returns an error with the verbatim D-B2 message.
    - Test 2 (D-B1 happy path): `skill_execute` removes manifest entry, library directory, distribution symlinks, lockfile entry, and machine.toml memberships in one shot.
    - Test 3 (skill not in lockfile is OK): if `lockfile.skills.get(name).is_none()`, no error — just skip the lockfile delete step.
    - Test 4 (skill not in machine.toml is OK): if `machine_prefs.disabled.contains(name) == false` and no per-dir memberships, no error.
    - Test 5 (partial failure aggregation): if a distribution-symlink delete fails, the failure is recorded with `RemoveSkillFailureKind::DistributionSymlink` and `skill_execute` returns the failures vec; manifest/library/lockfile mutations DO NOT persist on partial failure (matching the existing dir-flavour I2/I3 retention semantic — atomic-save chain happens only on full success).
    - Test 6 (dry_run): nothing is mutated; counts reflect would-be operations.
  </behavior>
  <action>
    1. **Add `skill_plan` function** to remove.rs. Place it after the existing `dir`-flavour `plan/render_plan/execute` (after line 377, after the existing `execute` returns).

    ```rust
    /// Build a plan for `tome remove skill <name>`. Refuses Owned skills (D-B2).
    pub(crate) fn skill_plan(
        name: &str,
        config: &Config,
        paths: &TomePaths,
        manifest: &Manifest,
        lockfile: Option<&crate::lockfile::Lockfile>,
        machine_prefs: &crate::machine::MachinePrefs,
    ) -> Result<RemoveSkillPlan> {
        // Validate skill exists in manifest.
        let entry = manifest
            .get(name)
            .ok_or_else(|| anyhow::anyhow!("skill '{}' not found in library", name))?;

        // D-B2 owned guard: refuse to operate on Owned skills.
        if let Some(owner) = &entry.source_name {
            anyhow::bail!(
                "skill '{}' is owned by directory '{}' (source_name = {}). \
                 Remove the source directory with `tome remove dir {}` first, \
                 or remove the file from disk and re-sync.",
                name,
                owner,
                owner,
                owner,
            );
        }

        let skill_name = SkillName::new(name)?;
        let library_path = paths.library_dir().join(name);

        // Find distribution symlinks pointing at this skill.
        let mut symlinks_to_remove = Vec::new();
        for (_other_name, other_config) in &config.directories {
            let role = other_config.role();
            if !role.is_distribution() {
                continue;
            }
            let skills_dir = match crate::config::expand_tilde(&other_config.path) {
                Ok(p) => p,
                Err(_) => continue,
            };
            if !skills_dir.is_dir() {
                continue;
            }
            let candidate = skills_dir.join(name);
            if candidate.is_symlink() {
                symlinks_to_remove.push(candidate);
            }
        }

        // Lockfile membership.
        let has_lockfile_entry = lockfile
            .map(|lf| lf.skills.contains_key(&skill_name))
            .unwrap_or(false);

        // machine.toml memberships.
        let in_machine_disabled = machine_prefs.is_disabled(name);

        let mut per_directory_memberships: Vec<(DirectoryName, bool, bool)> = Vec::new();
        // Iterate the per-directory prefs map (pub(crate)) and check both lists.
        for (dir_name, dir_prefs) in &machine_prefs.directory {
            let in_enabled = dir_prefs
                .enabled
                .as_ref()
                .map(|set| set.iter().any(|s| s.as_str() == name))
                .unwrap_or(false);
            let in_disabled = dir_prefs.disabled.iter().any(|s| s.as_str() == name);
            if in_enabled || in_disabled {
                per_directory_memberships.push((dir_name.clone(), in_enabled, in_disabled));
            }
        }

        Ok(RemoveSkillPlan {
            skill_name,
            library_path,
            symlinks_to_remove,
            has_lockfile_entry,
            in_machine_disabled,
            per_directory_memberships,
        })
    }
    ```

    Note: the `MachinePrefs.directory` and `DirectoryPrefs.{enabled, disabled}` fields are `pub(crate)`. Since `remove.rs` is in the same crate, direct field access works. If clippy complains about visibility within the crate, use the existing `MachinePrefs::is_disabled` for the `disabled` set check (that's why it's used above) and accept direct field access for the per-directory map.

    2. **Add `skill_render_plan` function:**

    ```rust
    /// Render the skill-removal plan to stdout.
    pub(crate) fn skill_render_plan(plan: &RemoveSkillPlan) {
        println!(
            "Forget skill plan for '{}':",
            style(plan.skill_name.as_str()).cyan()
        );
        if plan.library_path.exists() {
            println!(
                "  Library directory will be removed: {}",
                style(crate::paths::collapse_home(&plan.library_path)).dim()
            );
        }
        if !plan.symlinks_to_remove.is_empty() {
            println!(
                "  Distribution symlinks to remove: {}",
                style(plan.symlinks_to_remove.len()).bold()
            );
        }
        if plan.has_lockfile_entry {
            println!("  Lockfile entry will be removed.");
        }
        if plan.in_machine_disabled {
            println!("  Membership in `machine.toml::disabled` will be removed.");
        }
        if !plan.per_directory_memberships.is_empty() {
            println!(
                "  Per-directory machine.toml memberships to clean: {}",
                style(plan.per_directory_memberships.len()).bold()
            );
            for (dir, in_e, in_d) in &plan.per_directory_memberships {
                let parts: Vec<&str> = match (in_e, in_d) {
                    (true, true) => vec!["enabled", "disabled"],
                    (true, false) => vec!["enabled"],
                    (false, true) => vec!["disabled"],
                    (false, false) => continue,
                };
                println!("    - {}: {}", dir, parts.join(", "));
            }
        }
    }
    ```

    3. **Add `skill_execute` function:**

    ```rust
    /// Result of `skill_execute`.
    pub(crate) struct RemoveSkillResult {
        pub library_removed: bool,
        pub symlinks_removed: usize,
        pub lockfile_entry_removed: bool,
        pub machine_disabled_removed: bool,
        pub per_directory_cleanups: usize,
        pub failures: Vec<RemoveSkillFailure>,
    }

    /// Execute the skill-removal plan. On full success, mutates manifest,
    /// lockfile, and machine_prefs in memory; the caller is responsible
    /// for calling manifest::save / lockfile::save / machine::save
    /// (atomic temp+rename). On partial failure, returns failures without
    /// mutating in-memory state (matches dir-flavour I2/I3 retention).
    pub(crate) fn skill_execute(
        plan: &RemoveSkillPlan,
        manifest: &mut Manifest,
        lockfile: &mut Option<crate::lockfile::Lockfile>,
        machine_prefs: &mut crate::machine::MachinePrefs,
        dry_run: bool,
    ) -> Result<RemoveSkillResult> {
        let mut failures: Vec<RemoveSkillFailure> = Vec::new();
        let mut library_removed = false;
        let mut symlinks_removed = 0usize;

        // 1. Remove library directory.
        if plan.library_path.exists() {
            if dry_run {
                library_removed = true;
            } else {
                match std::fs::remove_dir_all(&plan.library_path) {
                    Ok(_) => library_removed = true,
                    Err(e) => failures.push(RemoveSkillFailure::new(
                        RemoveSkillFailureKind::LibraryDir,
                        plan.library_path.clone(),
                        e,
                    )),
                }
            }
        }

        // 2. Remove distribution symlinks.
        for symlink in &plan.symlinks_to_remove {
            if dry_run {
                symlinks_removed += 1;
            } else {
                match std::fs::remove_file(symlink) {
                    Ok(_) => symlinks_removed += 1,
                    Err(e) => failures.push(RemoveSkillFailure::new(
                        RemoveSkillFailureKind::DistributionSymlink,
                        symlink.clone(),
                        e,
                    )),
                }
            }
        }

        // On partial filesystem failure: bail out before mutating in-memory
        // state. The caller will not call any save() on this branch, so disk
        // state remains consistent (matches dir-flavour I2/I3 retention).
        let mut lockfile_entry_removed = false;
        let mut machine_disabled_removed = false;
        let mut per_directory_cleanups = 0usize;

        if failures.is_empty() && !dry_run {
            // 3. Remove lockfile entry (in-memory).
            if let Some(lf) = lockfile.as_mut() {
                if lf.skills.remove(&plan.skill_name).is_some() {
                    lockfile_entry_removed = true;
                }
            }

            // 4. Remove machine.toml::disabled membership (in-memory).
            if machine_prefs.disabled.iter().any(|s| s.as_str() == plan.skill_name.as_str()) {
                machine_prefs
                    .disabled
                    .retain(|s| s.as_str() != plan.skill_name.as_str());
                machine_disabled_removed = true;
            }

            // 5. Remove per-directory memberships (in-memory).
            for (dir_name, _in_e, _in_d) in &plan.per_directory_memberships {
                if let Some(dir_prefs) = machine_prefs.directory.get_mut(dir_name) {
                    let before_e = dir_prefs.enabled.as_ref().map(|s| s.len()).unwrap_or(0);
                    if let Some(enabled) = dir_prefs.enabled.as_mut() {
                        enabled.retain(|s| s.as_str() != plan.skill_name.as_str());
                    }
                    let after_e = dir_prefs.enabled.as_ref().map(|s| s.len()).unwrap_or(0);
                    let before_d = dir_prefs.disabled.len();
                    dir_prefs
                        .disabled
                        .retain(|s| s.as_str() != plan.skill_name.as_str());
                    let after_d = dir_prefs.disabled.len();
                    if (before_e > after_e) || (before_d > after_d) {
                        per_directory_cleanups += 1;
                    }
                }
            }

            // 6. Remove manifest entry (in-memory).
            manifest.remove(plan.skill_name.as_str());
        } else if dry_run {
            lockfile_entry_removed = plan.has_lockfile_entry;
            machine_disabled_removed = plan.in_machine_disabled;
            per_directory_cleanups = plan.per_directory_memberships.len();
        }

        Ok(RemoveSkillResult {
            library_removed,
            symlinks_removed,
            lockfile_entry_removed,
            machine_disabled_removed,
            per_directory_cleanups,
            failures,
        })
    }
    ```

    4. **Add unit tests** in `#[cfg(test)] mod tests`:

    ```rust
    #[test]
    fn skill_plan_refuses_owned_skill() {
        let (_tmp, config, paths, manifest) = make_test_setup();
        // make_test_setup creates an Owned "my-skill" in test-source.
        let lockfile = None;
        let machine_prefs = crate::machine::MachinePrefs::default();
        let err = skill_plan("my-skill", &config, &paths, &manifest, lockfile, &machine_prefs)
            .err()
            .expect("must refuse Owned per D-B2")
            .to_string();
        assert!(err.contains("is owned by directory"), "got: {err}");
        assert!(err.contains("Remove the source directory"), "got: {err}");
        assert!(err.contains("tome remove dir"), "got: {err}");
    }

    #[test]
    fn skill_plan_skill_not_in_library() {
        let (_tmp, config, paths, manifest) = make_test_setup();
        let lockfile = None;
        let machine_prefs = crate::machine::MachinePrefs::default();
        let err = skill_plan(
            "nonexistent",
            &config,
            &paths,
            &manifest,
            lockfile,
            &machine_prefs,
        )
        .err()
        .unwrap()
        .to_string();
        assert!(err.contains("not found in library"));
    }

    #[test]
    fn skill_execute_full_cleanup_happy_path() {
        let (tmp, config, paths, mut manifest) = make_test_setup();
        // Transition my-skill to Unowned for this test.
        let entry = manifest.skills_get_mut("my-skill").unwrap();
        entry.source_name = None;

        // Build fake lockfile with my-skill.
        use crate::lockfile::{LockEntry, Lockfile};
        use std::collections::BTreeMap;
        let mut skills = BTreeMap::new();
        skills.insert(
            SkillName::new("my-skill").unwrap(),
            LockEntry {
                source_name: None,
                previous_source: None,
                content_hash: test_hash(),
                registry_id: None,
                version: None,
                git_commit_sha: None,
            },
        );
        let mut lockfile = Some(Lockfile { version: 1, skills });

        // Build machine_prefs with my-skill disabled.
        let mut machine_prefs = crate::machine::MachinePrefs::default();
        machine_prefs.disable(SkillName::new("my-skill").unwrap());

        let plan = skill_plan(
            "my-skill",
            &config,
            &paths,
            &manifest,
            lockfile.as_ref(),
            &machine_prefs,
        )
        .unwrap();
        assert!(plan.has_lockfile_entry);
        assert!(plan.in_machine_disabled);

        let result = skill_execute(&plan, &mut manifest, &mut lockfile, &mut machine_prefs, false)
            .unwrap();
        assert!(result.library_removed);
        assert_eq!(result.symlinks_removed, 1, "1 dist symlink in fixture");
        assert!(result.lockfile_entry_removed);
        assert!(result.machine_disabled_removed);
        assert!(result.failures.is_empty());

        // Verify in-memory state.
        assert!(!manifest.contains_key("my-skill"));
        assert!(!lockfile.as_ref().unwrap().skills.contains_key(&SkillName::new("my-skill").unwrap()));
        assert!(!machine_prefs.is_disabled("my-skill"));

        // Verify on-disk state.
        assert!(!tmp.path().join("library").join("my-skill").exists());
        assert!(!tmp.path().join("target").join("my-skill").exists());
    }

    #[test]
    fn skill_execute_partial_failure_preserves_in_memory_state() {
        let (tmp, config, paths, mut manifest) = make_test_setup();
        let entry = manifest.skills_get_mut("my-skill").unwrap();
        entry.source_name = None;

        let mut lockfile: Option<crate::lockfile::Lockfile> = None;
        let mut machine_prefs = crate::machine::MachinePrefs::default();

        // Pre-delete the dist symlink so std::fs::remove_file fails with ENOENT.
        let dist_symlink = tmp.path().join("target").join("my-skill");
        std::fs::remove_file(&dist_symlink).ok();

        let plan = skill_plan(
            "my-skill",
            &config,
            &paths,
            &manifest,
            lockfile.as_ref(),
            &machine_prefs,
        )
        .unwrap();

        let result = skill_execute(&plan, &mut manifest, &mut lockfile, &mut machine_prefs, false)
            .unwrap();
        assert!(
            !result.failures.is_empty(),
            "expected DistributionSymlink failure"
        );
        assert!(
            result.failures.iter().any(|f| f.kind == RemoveSkillFailureKind::DistributionSymlink),
            "expected DistributionSymlink kind, got: {:?}", result.failures
        );

        // I2/I3 retention: manifest entry retained on partial failure.
        assert!(
            manifest.contains_key("my-skill"),
            "manifest entry must be preserved on partial failure for retry"
        );
    }

    #[test]
    fn skill_execute_dry_run_no_mutation() {
        let (tmp, config, paths, mut manifest) = make_test_setup();
        let entry = manifest.skills_get_mut("my-skill").unwrap();
        entry.source_name = None;
        let mut lockfile: Option<crate::lockfile::Lockfile> = None;
        let mut machine_prefs = crate::machine::MachinePrefs::default();

        let plan = skill_plan(
            "my-skill",
            &config,
            &paths,
            &manifest,
            lockfile.as_ref(),
            &machine_prefs,
        )
        .unwrap();

        skill_execute(&plan, &mut manifest, &mut lockfile, &mut machine_prefs, true).unwrap();
        // Manifest still has it.
        assert!(manifest.contains_key("my-skill"));
        // Library still on disk.
        assert!(tmp.path().join("library").join("my-skill").exists());
    }
    ```

    Note: in the `skill_execute_full_cleanup_happy_path` test, `LockEntry`'s shape now includes `previous_source` (added in 14-01). Make sure the test's `LockEntry { ... }` literal has all required fields.

    5. Run `cargo test -p tome --lib remove::tests` to confirm all tests pass.
  </action>
  <verify>
    <automated>cargo test -p tome --lib remove::tests::skill_plan_refuses_owned_skill remove::tests::skill_plan_skill_not_in_library remove::tests::skill_execute_full_cleanup_happy_path remove::tests::skill_execute_partial_failure_preserves_in_memory_state remove::tests::skill_execute_dry_run_no_mutation</automated>
  </verify>
  <acceptance_criteria>
    - `grep -q "pub(crate) fn skill_plan" crates/tome/src/remove.rs` succeeds
    - `grep -q "pub(crate) fn skill_render_plan" crates/tome/src/remove.rs` succeeds
    - `grep -q "pub(crate) fn skill_execute" crates/tome/src/remove.rs` succeeds
    - `grep -q "is owned by directory" crates/tome/src/remove.rs` succeeds (D-B2 message)
    - `grep -q "Remove the source directory with" crates/tome/src/remove.rs` succeeds (D-B2 actionable hint)
    - `cargo test -p tome --lib remove::tests::skill_plan_refuses_owned_skill` exits 0
    - `cargo test -p tome --lib remove::tests::skill_execute_full_cleanup_happy_path` exits 0
    - `cargo test -p tome --lib remove::tests::skill_execute_partial_failure_preserves_in_memory_state` exits 0
    - `cargo test -p tome --lib remove::tests::skill_execute_dry_run_no_mutation` exits 0
    - `cargo clippy --all-targets -p tome -- -D warnings` exits 0
  </acceptance_criteria>
  <done>
    Plan/render/execute triple for `tome remove skill` lives in `remove.rs` and mirrors the existing `dir`-flavour structure. D-B1 cleanup scope covered (manifest + library + dist symlinks + lockfile + machine.toml). D-B2 owned guard refuses with verbatim message. Partial-failure aggregation works. Dry-run safe.
  </done>
</task>

<task type="auto">
  <name>Task 3: Wire the `RemoveKind::Skill` arm in `lib.rs::run` (replaces the 14-03 stub)</name>
  <read_first>
    - crates/tome/src/lib.rs (the `Command::Remove { kind } => match kind` block; the `RemoveKind::Skill { name, yes }` stub from 14-03 with the `anyhow::bail!("tome remove skill is not yet implemented")` line)
    - crates/tome/src/remove.rs (skill_plan/skill_render_plan/skill_execute from Task 2, RemoveSkillFailureKind::ALL)
    - crates/tome/src/lib.rs lines 411-515 (the existing `RemoveKind::Dir` arm — the SAFE-01 grouped failure-summary pattern is the model for the `Skill` arm)
    - .planning/phases/14-unowned-library-lifecycle/14-CONTEXT.md (D-B3 — verbatim confirmation prompt text)
  </read_first>
  <action>
    1. **Locate** the stub `RemoveKind::Skill { name, yes }` arm in `lib.rs::run` (added by 14-03; contains `anyhow::bail!("tome remove skill is not yet implemented...")`).

    2. **Replace** the entire arm body with:

    ```rust
    cli::RemoveKind::Skill { name, yes } => {
        let manifest = manifest::load(paths.config_dir())?;
        let lockfile = lockfile::load(paths.config_dir())?;
        let machine_path = resolve_machine_path(cli.machine.as_deref())?;
        let machine_prefs = machine::load(&machine_path)?;

        let plan = remove::skill_plan(
            &name,
            &config,
            &paths,
            &manifest,
            lockfile.as_ref(),
            &machine_prefs,
        )?;
        remove::skill_render_plan(&plan);

        if cli.dry_run {
            println!("\n{}", style("Dry run — no changes made.").yellow());
            return Ok(());
        }

        // D-B3: confirmation default-no, --yes / -y bypasses.
        if !yes {
            if !cli.no_input && std::io::stdin().is_terminal() {
                let confirmed = dialoguer::Confirm::new()
                    .with_prompt(format!(
                        "Are you sure you want to forget skill '{}'?",
                        name
                    ))
                    .default(false)
                    .interact()?;
                if !confirmed {
                    println!("Aborted.");
                    return Ok(());
                }
            } else {
                anyhow::bail!(
                    "tome remove skill requires confirmation — use --yes in non-interactive mode"
                );
            }
        }

        let mut manifest = manifest;
        let mut lockfile = lockfile;
        let mut machine_prefs = machine_prefs;
        let result = remove::skill_execute(
            &plan,
            &mut manifest,
            &mut lockfile,
            &mut machine_prefs,
            false,
        )?;

        // SAFE-01 grouped partial-failure summary BEFORE saves.
        if !result.failures.is_empty() {
            let k = result.failures.len();
            eprintln!(
                "{} {} operations failed during remove of skill '{}' — \
                 in-memory state retained so you can retry after addressing these. \
                 Run {} after resolving:",
                style("⚠").yellow(),
                k,
                name,
                style("`tome doctor`").bold(),
            );
            for kind in remove::RemoveSkillFailureKind::ALL {
                let group: Vec<&remove::RemoveSkillFailure> =
                    result.failures.iter().filter(|f| f.kind == kind).collect();
                if group.is_empty() {
                    continue;
                }
                eprintln!("  {} ({}):", kind.label(), group.len());
                for f in group {
                    eprintln!("    {}: {}", paths::collapse_home(&f.path), f.error);
                }
            }
            return Err(anyhow::anyhow!(
                "tome remove skill completed with {k} failures"
            ));
        }

        // Atomic-save chain (D-B1): manifest + lockfile + machine.toml.
        manifest::save(&manifest, paths.config_dir())?;
        if let Some(lf) = &lockfile {
            lockfile::save(lf, paths.config_dir())?;
        }
        machine::save(&machine_prefs, &machine_path)?;

        // Success banner.
        println!(
            "\n{} Forgot skill '{}' — library, {} symlinks, lockfile{}{} cleaned.",
            style("✓").green(),
            name,
            result.symlinks_removed,
            if result.machine_disabled_removed {
                ", machine.toml disabled"
            } else {
                ""
            },
            if result.per_directory_cleanups > 0 {
                format!(", {} per-directory entries", result.per_directory_cleanups)
            } else {
                String::new()
            },
        );
    }
    ```

    Notes:
    - `IsTerminal` and `dialoguer` and `style` are already imported at the top of lib.rs.
    - `manifest::load` and `lockfile::load` and `machine::load` and `machine::save` are crate-internal modules; check the existing imports — `lockfile` is `pub(crate)` (uses `lockfile::load(...)` elsewhere in lib.rs at line 1080), `machine::load` is similarly accessible.
    - The `resolve_machine_path` helper is already defined at lib.rs:100-105.

    3. **Verify the dispatch** by running `cargo build -p tome` then `cargo test -p tome`.
  </action>
  <verify>
    <automated>cargo test -p tome --lib; cargo test -p tome --test cli; cargo clippy --all-targets -p tome -- -D warnings</automated>
  </verify>
  <acceptance_criteria>
    - `! grep -q "tome remove skill is not yet implemented" crates/tome/src/lib.rs` (the stub message is gone)
    - `grep -q "remove::skill_plan(" crates/tome/src/lib.rs` succeeds
    - `grep -q "remove::skill_execute(" crates/tome/src/lib.rs` succeeds
    - `grep -q "Are you sure you want to forget skill" crates/tome/src/lib.rs` succeeds (D-B3 prompt)
    - `grep -q "RemoveSkillFailureKind::ALL" crates/tome/src/lib.rs` succeeds (SAFE-01 grouped summary)
    - `cargo test -p tome` exits 0
    - `cargo clippy --all-targets -p tome -- -D warnings` exits 0
  </acceptance_criteria>
  <done>
    `tome remove skill <name>` is fully wired: plan/render/confirm/execute/save chain. SAFE-01 grouped failure summary on partial failure. The 14-03 stub error is replaced.
  </done>
</task>

</tasks>

<verification>
- `cargo test -p tome` exits 0 (full suite)
- `cargo clippy --all-targets -p tome -- -D warnings` exits 0
- The 14-03 stub `"tome remove skill is not yet implemented"` is removed
- A manual smoke test (or integration test in 14-08): `tome remove skill <unowned> --yes` deletes everything; `tome remove skill <owned>` is refused with the D-B2 message
</verification>

<success_criteria>
- UNOWN-02 delivered: `tome remove skill <name>` removes manifest + library + dist symlinks + lockfile + machine.toml memberships in one atomic-save flow.
- D-B1 cleanup scope covered.
- D-B2 owned guard refuses with verbatim message.
- D-B3 confirmation default-no; `--yes`/`-y` bypasses.
- `RemoveSkillFailureKind` mirrors `FailureKind` shape: ALL array, compile-time exhaustiveness, runtime test pinning length.
- Partial-failure aggregation works; in-memory state retained on failure (I2/I3 retention).
- Dry-run safe.
</success_criteria>

<output>
After completion, create `.planning/phases/14-unowned-library-lifecycle/14-05-SUMMARY.md`
</output>
