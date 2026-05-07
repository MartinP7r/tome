---
phase: 14-unowned-library-lifecycle
plan: 04
type: execute
wave: 3
depends_on:
  - 14-01
  - 14-03
files_modified:
  - crates/tome/src/reassign.rs
  - crates/tome/src/lib.rs
autonomous: true
requirements:
  - UNOWN-01

must_haves:
  truths:
    - "`tome reassign <unowned-skill> --to <dir>` succeeds — the Unowned-input refusal is removed; `manifest[skill].source_name` flips from `None` to `Some(<dir>)` and `previous_source` is cleared on re-anchor."
    - "Re-assigning into a target-only directory role is rejected with a clear error (D-A2)."
    - "Re-assigning into a target where `<dir>/<skill>/` already exists with DIFFERENT content is refused unless `--force` is passed (D-A1); same-content collisions still take the existing Relink path."
    - "Owned→Owned reassign (today's behaviour) still works on every preserved test."
  artifacts:
    - path: "crates/tome/src/reassign.rs"
      provides: "Updated plan() accepting Unowned input + content-hash check + role check + --force flag; updated execute() clears previous_source on re-anchor; updated render_plan() handles None from_directory."
      contains: "from_directory: Option<DirectoryName>"
  key_links:
    - from: "reassign::plan"
      to: "manifest::hash_directory"
      via: "content-hash compare between library_skill_path and target_skill_path"
      pattern: "manifest::hash_directory"
    - from: "reassign::execute"
      to: "manifest::SkillEntry::previous_source"
      via: "explicit clear on re-anchor"
      pattern: "entry.previous_source = None"
---

<objective>
Deliver UNOWN-01 by extending `tome reassign` to accept Unowned skills as
input (D-API-1), and harden the existing Owned→Owned path with the same
defensive checks that Unowned→Owned needs (D-A1 different-content collision
+ D-A2 target-role rejection). Also clear `previous_source` on successful
re-anchor (D-C1 closure: the breadcrumb is no longer needed once owned again).

Purpose: closes the literal `reassign.rs:60` stub error pointing at "Phase 14"
and ships the user-facing UNOWN-01 behaviour under the merged verb. Plan
14-03 already added the `--force` flag in clap; this plan threads it through
the implementation.

Output: `reassign::plan` accepts Unowned skills; `reassign::execute` clears
`previous_source`; `--force` bypasses the content-hash collision check;
target-only roles are rejected.
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
@crates/tome/src/reassign.rs
@crates/tome/src/manifest.rs
@crates/tome/src/config.rs
@crates/tome/src/lib.rs

<interfaces>
<!-- Today's reassign::plan signature (reassign.rs:45-93): -->
```rust
pub(crate) fn plan(
    skill_name: &str,
    to_dir: &str,
    config: &Config,
    paths: &TomePaths,
    manifest: &Manifest,
    is_fork: bool,
) -> Result<ReassignPlan>
```

<!-- The literal stub being deleted (reassign.rs:58-63 — verbatim): -->
```rust
let from_directory = entry.source_name.clone().ok_or_else(|| {
    anyhow::anyhow!(
        "skill '{}' is Unowned (no source directory); use `tome adopt` (Phase 14) to assign a directory before reassigning",
        skill_name
    )
})?;
```

<!-- Today's ReassignPlan struct (reassign.rs:28-42): -->
```rust
pub(crate) struct ReassignPlan {
    pub skill_name: SkillName,
    pub from_directory: DirectoryName,  // <-- changes to Option<DirectoryName>
    pub to_directory: DirectoryName,
    pub action: ReassignAction,
    pub library_skill_path: PathBuf,
    pub is_fork: bool,
}
```

<!-- DirectoryRole API (config.rs:142-189): -->
- `DirectoryRole::is_discovery() -> bool` — true for Managed | Synced | Source
- `DirectoryRole::is_distribution() -> bool` — true for Synced | Target
- "target-only" per D-A2 = `!role.is_discovery()` = role is `Target` (the
  Target variant is the only role where is_discovery=false; Managed/Source
  are discovery-only-not-distribution but still are_discovery).

<!-- Existing manifest::hash_directory: -->
```rust
pub fn hash_directory(dir: &Path) -> Result<ContentHash>;
```

<!-- D-A1 verbatim error message (from 14-CONTEXT.md): -->
```
error: skill 'foo' already exists in 'my-dir' with different content.
Use --force to overwrite, or remove the existing entry first.
```

<!-- D-A2 verbatim error message: -->
```
error: directory 'my-target' has role 'target-only' and cannot receive
reassigned skills (next sync would not rediscover them). Reassign into
a discovery or mixed-role directory.
```
</interfaces>
</context>

<tasks>

<task type="auto" tdd="true">
  <name>Task 1: Update `reassign::plan` to accept Unowned input + role check + content-hash check</name>
  <read_first>
    - crates/tome/src/reassign.rs (entire file)
    - crates/tome/src/config.rs (DirectoryRole at lines 142-189; DirectoryConfig.role() method)
    - crates/tome/src/manifest.rs (hash_directory function)
    - .planning/phases/14-unowned-library-lifecycle/14-CONTEXT.md (D-API-1, D-A1, D-A2 — verbatim error message text)
    - .planning/phases/14-unowned-library-lifecycle/14-01-previous-source-schema-PLAN.md (the previous_source field that this plan clears on re-anchor)
  </read_first>
  <behavior>
    - Test 1 (D-API-1 happy path): `plan("orphan-skill", "good-dir", ...)` where `manifest["orphan-skill"].source_name == None` succeeds and returns a plan with `from_directory = None`.
    - Test 2 (D-API-1 stub deleted): the previous "use `tome adopt` (Phase 14)" error string is gone from the source file.
    - Test 3 (D-A2 role rejection): `plan(..., to_dir="target-only-dir", ...)` errors with the verbatim D-A2 message text.
    - Test 4 (D-A1 same-content collision): existing skill at `<to>/<skill>/` with same content_hash → `ReassignAction::Relink` (today's behaviour preserved).
    - Test 5 (D-A1 different-content collision without --force): errors with the verbatim D-A1 message text.
    - Test 6 (D-A1 different-content collision WITH --force=true): succeeds and returns `ReassignAction::CopyAndRelink` with `force=true` recorded on the plan.
    - Test 7 (Owned→Owned regression): pre-existing `test_plan_happy_path_copy_and_relink` still passes (renamed if signature change forces it).
  </behavior>
  <action>
    1. **Update `ReassignPlan` struct** (reassign.rs:28-42):
       - Change `from_directory: DirectoryName` → `from_directory: Option<DirectoryName>`.
       - Add `force: bool` field.

    Final shape:

    ```rust
    #[derive(Debug)]
    pub(crate) struct ReassignPlan {
        pub skill_name: SkillName,
        /// Current source_name from manifest. `None` when reassigning an
        /// Unowned skill (D-API-1 / Phase 14 UNOWN-01).
        pub from_directory: Option<DirectoryName>,
        pub to_directory: DirectoryName,
        pub action: ReassignAction,
        pub library_skill_path: PathBuf,
        pub is_fork: bool,
        /// Bypass D-A1 different-content collision refusal. Same-content
        /// collisions always take the Relink path regardless.
        pub force: bool,
    }
    ```

    2. **Update `plan()` signature** to accept `force`:

    ```rust
    pub(crate) fn plan(
        skill_name: &str,
        to_dir: &str,
        config: &Config,
        paths: &TomePaths,
        manifest: &Manifest,
        is_fork: bool,
        force: bool,
    ) -> Result<ReassignPlan>
    ```

    3. **Delete the Unowned-refusal block** (reassign.rs:58-63) and replace with:

    ```rust
    // D-API-1 (Phase 14): Unowned skills are valid input. The ReassignPlan
    // carries `from_directory: Option<DirectoryName>` so render_plan can
    // distinguish "Unowned → <to>" from "<from> → <to>". The previous stub
    // error pointing at `tome adopt` is removed.
    let from_directory = entry.source_name.clone();
    ```

    4. **Add target-role check (D-A2)** immediately after the existing target-directory lookup. Today's code at reassign.rs:66-71 reads:

    ```rust
    let to_dir_name =
        DirectoryName::new(to_dir).with_context(|| format!("invalid directory name: {to_dir}"))?;
    let to_dir_config = config
        .directories
        .get(&to_dir_name)
        .ok_or_else(|| anyhow::anyhow!("directory '{}' not found in config", to_dir))?;
    ```

    Insert AFTER the `.ok_or_else` line:

    ```rust
    // D-A2 (Phase 14): refuse target-only roles. Reassigning into a
    // target-only dir leaves the skill stranded — nothing rediscovers
    // it on next sync.
    if !to_dir_config.role().is_discovery() {
        anyhow::bail!(
            "directory '{}' has role 'target-only' and cannot receive \
             reassigned skills (next sync would not rediscover them). \
             Reassign into a discovery or mixed-role directory.",
            to_dir,
        );
    }
    ```

    5. **Add D-A1 content-hash collision check.** Today's action determination is at reassign.rs:73-82:

    ```rust
    let target_skill_path = crate::config::expand_tilde(&to_dir_config.path)?
        .join(skill_name)
        .join("SKILL.md");
    let action = if target_skill_path.exists() {
        ReassignAction::Relink
    } else {
        ReassignAction::CopyAndRelink
    };
    ```

    Replace with:

    ```rust
    let target_dir_for_skill = crate::config::expand_tilde(&to_dir_config.path)?
        .join(skill_name);
    let target_skill_md = target_dir_for_skill.join("SKILL.md");

    let library_skill_path = paths.library_dir().join(skill_name);

    let action = if target_skill_md.exists() {
        // D-A1 (Phase 14): content-hash collision check. If the target
        // dir's <skill>/ already exists, hash both sides; same content =
        // Relink (manifest-only flip), different content = refuse unless
        // --force.
        let target_hash = crate::manifest::hash_directory(&target_dir_for_skill)
            .with_context(|| {
                format!(
                    "failed to hash existing target skill {}",
                    target_dir_for_skill.display()
                )
            })?;
        let library_hash = if library_skill_path.is_dir() {
            crate::manifest::hash_directory(&library_skill_path)
                .with_context(|| {
                    format!(
                        "failed to hash library skill {}",
                        library_skill_path.display()
                    )
                })?
        } else {
            // Library copy missing — defer to existing error path; bail
            // with a recognisable message so callers don't see a confusing
            // "different content" error.
            anyhow::bail!(
                "skill '{}' is missing from the library at {}; cannot reassign",
                skill_name,
                library_skill_path.display(),
            );
        };

        if target_hash == library_hash {
            ReassignAction::Relink
        } else if force {
            ReassignAction::CopyAndRelink
        } else {
            anyhow::bail!(
                "skill '{}' already exists in '{}' with different content. \
                 Use --force to overwrite, or remove the existing entry first.",
                skill_name,
                to_dir,
            );
        }
    } else {
        ReassignAction::CopyAndRelink
    };
    ```

    6. **Update the `Ok(ReassignPlan { ... })` literal** at the end of `plan()` to include the new `force` field:

    ```rust
    Ok(ReassignPlan {
        skill_name: SkillName::new(skill_name)?,
        from_directory,
        to_directory: to_dir_name,
        action,
        library_skill_path,
        is_fork,
        force,
    })
    ```

    7. **Update `render_plan`** (reassign.rs:96-121) to handle `from_directory: Option<DirectoryName>`. The current code unconditionally uses `&plan.from_directory`; replace each rendering with conditional rendering. Concrete update — replace the body with:

    ```rust
    pub(crate) fn render_plan(plan: &ReassignPlan) {
        let skill = style(plan.skill_name.as_str()).cyan();
        let from_label = match &plan.from_directory {
            Some(d) => style(d.as_str().to_string()).cyan().to_string(),
            None => style("Unowned").yellow().to_string(),
        };
        let to = style(AsRef::<str>::as_ref(&plan.to_directory)).cyan();

        match (&plan.action, plan.is_fork) {
            (ReassignAction::Relink, _) => {
                println!(
                    "Reassign '{}' from '{}' to '{}' (skill already present in target)",
                    skill, from_label, to,
                );
            }
            (ReassignAction::CopyAndRelink, true) => {
                println!(
                    "Fork '{}' from '{}' to '{}' (copy files to target directory)",
                    skill, from_label, to,
                );
            }
            (ReassignAction::CopyAndRelink, false) => {
                println!(
                    "Reassign '{}' from '{}' to '{}' (copy files to target directory)",
                    skill, from_label, to,
                );
            }
        }
    }
    ```

    8. **Update `execute`** (reassign.rs:124-171) to:
       (a) handle `Option<DirectoryName>` from-directory by always using `Manifest::skills_get_mut` to clear `previous_source` AND set `source_name`, regardless of starting state. The current code uses `manifest.update_source_name(...)` which only handles Owned→Owned (rejects None per its doc comment). For Unowned→Owned we need to set `source_name = Some(<new>)` AND clear `previous_source`.

    Replace the body of the post-copy section (the part starting at "Update manifest source_name", reassign.rs:162-168) with:

    ```rust
    // Update manifest: set source_name = Some(to_directory), clear
    // previous_source (D-C1 closure: the skill is owned again). Works for
    // both Owned→Owned (today) and Unowned→Owned (D-API-1) starting states.
    let entry = manifest
        .skills_get_mut(plan.skill_name.as_str())
        .ok_or_else(|| {
            anyhow::anyhow!(
                "skill '{}' disappeared from manifest during reassignment",
                plan.skill_name.as_str()
            )
        })?;
    entry.source_name = Some(plan.to_directory.clone());
    entry.previous_source = None;
    ```

    9. **Add unit tests in `#[cfg(test)] mod tests`** (reassign.rs:201-onwards):

    ```rust
    fn write_skill_in_dir(dir: &std::path::Path, skill: &str, body: &str) {
        let skill_dir = dir.join(skill);
        std::fs::create_dir_all(&skill_dir).unwrap();
        std::fs::write(skill_dir.join("SKILL.md"), body).unwrap();
    }

    #[test]
    fn plan_accepts_unowned_input() {
        let tmp = TempDir::new().unwrap();
        let paths = test_paths(&tmp);
        let config = make_config_with_dir(&tmp, "target-dir");
        let mut manifest = Manifest::default();

        // Insert an Unowned skill (source_name = None).
        use crate::manifest::SkillEntry;
        use crate::validation::ContentHash;
        manifest.insert(
            SkillName::new("orphan-skill").unwrap(),
            SkillEntry::new_unowned(
                PathBuf::from("/tmp/old/orphan-skill"),
                ContentHash::new("a".repeat(64)).unwrap(),
                false,
                Some(crate::config::DirectoryName::new("removed-dir").unwrap()),
            ),
        );

        let result = plan(
            "orphan-skill",
            "target-dir",
            &config,
            &paths,
            &manifest,
            false,
            false,
        );
        assert!(
            result.is_ok(),
            "Unowned input must NOT be refused (D-API-1): {:?}",
            result.err()
        );
        let plan = result.unwrap();
        assert!(plan.from_directory.is_none(), "Unowned input → from_directory = None");
    }

    #[test]
    fn plan_rejects_target_only_role() {
        use crate::config::{DirectoryConfig, DirectoryRole, DirectoryType};

        let tmp = TempDir::new().unwrap();
        let paths = test_paths(&tmp);
        let dir_path = tmp.path().join("claude-target");
        std::fs::create_dir_all(&dir_path).unwrap();
        let mut config = Config::default();
        config.directories.insert(
            crate::config::DirectoryName::new("claude-target").unwrap(),
            DirectoryConfig {
                path: dir_path,
                directory_type: DirectoryType::Directory,
                role: Some(DirectoryRole::Target),  // target-only
                git_ref: None,
                subdir: None,
                override_applied: false,
            },
        );
        let mut manifest = Manifest::default();

        use crate::manifest::SkillEntry;
        use crate::validation::ContentHash;
        manifest.insert(
            SkillName::new("my-skill").unwrap(),
            SkillEntry::new(
                PathBuf::from("/tmp/x"),
                crate::config::DirectoryName::new("old-dir").unwrap(),
                ContentHash::new("a".repeat(64)).unwrap(),
                false,
            ),
        );

        let result = plan(
            "my-skill",
            "claude-target",
            &config,
            &paths,
            &manifest,
            false,
            false,
        );
        let err = result.err().expect("must reject target-only role per D-A2").to_string();
        assert!(
            err.contains("target-only"),
            "error must mention 'target-only', got: {err}"
        );
        assert!(
            err.contains("Reassign into a discovery or mixed-role directory"),
            "error must include the actionable hint per D-A2, got: {err}"
        );
    }

    #[test]
    fn plan_refuses_different_content_collision_without_force() {
        let tmp = TempDir::new().unwrap();
        let paths = test_paths(&tmp);
        let config = make_config_with_dir(&tmp, "target-dir");

        // Library has skill content "library version".
        write_skill_in_dir(paths.library_dir(), "test-skill", "library version");
        // Target dir already has skill with DIFFERENT content.
        let target_dir = tmp.path().join("target-dir");
        write_skill_in_dir(&target_dir, "test-skill", "different version");

        let mut manifest = Manifest::default();
        use crate::manifest::SkillEntry;
        use crate::validation::ContentHash;
        manifest.insert(
            SkillName::new("test-skill").unwrap(),
            SkillEntry::new(
                PathBuf::from("/tmp/x"),
                crate::config::DirectoryName::new("old").unwrap(),
                ContentHash::new("a".repeat(64)).unwrap(),
                false,
            ),
        );

        let err = plan(
            "test-skill",
            "target-dir",
            &config,
            &paths,
            &manifest,
            false,
            false,
        )
        .err()
        .expect("must refuse different-content collision per D-A1")
        .to_string();
        assert!(err.contains("with different content"), "got: {err}");
        assert!(err.contains("Use --force"), "got: {err}");
    }

    #[test]
    fn plan_force_bypasses_different_content_collision() {
        let tmp = TempDir::new().unwrap();
        let paths = test_paths(&tmp);
        let config = make_config_with_dir(&tmp, "target-dir");

        write_skill_in_dir(paths.library_dir(), "test-skill", "library version");
        let target_dir = tmp.path().join("target-dir");
        write_skill_in_dir(&target_dir, "test-skill", "different version");

        let mut manifest = Manifest::default();
        use crate::manifest::SkillEntry;
        use crate::validation::ContentHash;
        manifest.insert(
            SkillName::new("test-skill").unwrap(),
            SkillEntry::new(
                PathBuf::from("/tmp/x"),
                crate::config::DirectoryName::new("old").unwrap(),
                ContentHash::new("a".repeat(64)).unwrap(),
                false,
            ),
        );

        let p = plan(
            "test-skill",
            "target-dir",
            &config,
            &paths,
            &manifest,
            false,
            true,  // force
        )
        .expect("--force must bypass D-A1 collision");
        assert!(matches!(p.action, ReassignAction::CopyAndRelink));
        assert!(p.force);
    }

    #[test]
    fn plan_same_content_collision_takes_relink_path() {
        let tmp = TempDir::new().unwrap();
        let paths = test_paths(&tmp);
        let config = make_config_with_dir(&tmp, "target-dir");

        // SAME content in both library and target.
        write_skill_in_dir(paths.library_dir(), "test-skill", "same content");
        let target_dir = tmp.path().join("target-dir");
        write_skill_in_dir(&target_dir, "test-skill", "same content");

        let mut manifest = Manifest::default();
        use crate::manifest::SkillEntry;
        use crate::validation::ContentHash;
        manifest.insert(
            SkillName::new("test-skill").unwrap(),
            SkillEntry::new(
                PathBuf::from("/tmp/x"),
                crate::config::DirectoryName::new("old").unwrap(),
                ContentHash::new("a".repeat(64)).unwrap(),
                false,
            ),
        );

        let p = plan(
            "test-skill",
            "target-dir",
            &config,
            &paths,
            &manifest,
            false,
            false,
        )
        .unwrap();
        assert!(matches!(p.action, ReassignAction::Relink));
    }

    #[test]
    fn execute_clears_previous_source_on_re_anchor() {
        let tmp = TempDir::new().unwrap();
        let paths = test_paths(&tmp);
        let config = make_config_with_dir(&tmp, "new-dir");

        write_skill_in_dir(paths.library_dir(), "orphan-skill", "library content");

        let mut manifest = Manifest::default();
        use crate::manifest::SkillEntry;
        use crate::validation::ContentHash;
        manifest.insert(
            SkillName::new("orphan-skill").unwrap(),
            SkillEntry::new_unowned(
                PathBuf::from("/tmp/old/orphan-skill"),
                ContentHash::new("a".repeat(64)).unwrap(),
                false,
                Some(crate::config::DirectoryName::new("removed-dir").unwrap()),
            ),
        );

        let p = plan(
            "orphan-skill",
            "new-dir",
            &config,
            &paths,
            &manifest,
            false,
            false,
        )
        .unwrap();

        let target_path = tmp.path().join("new-dir");
        execute(&p, &mut manifest, &target_path, false).unwrap();

        let entry = manifest.get("orphan-skill").unwrap();
        assert_eq!(
            entry.source_name,
            Some(crate::config::DirectoryName::new("new-dir").unwrap()),
            "re-anchor must set source_name"
        );
        assert_eq!(
            entry.previous_source, None,
            "re-anchor must clear previous_source per D-C1 closure"
        );
    }
    ```

    10. **Update existing tests** in this module that call `plan(...)` with 6 args to pass the new 7th `force` arg as `false`. Likely sites: `test_plan_skill_not_found`, `test_plan_happy_path_copy_and_relink`, `test_plan_relink_when_skill_exists_in_target`, `test_plan_dir_not_found`. Add `false` as the final arg.

    11. **Verify the literal stub error string is gone:**
        `! grep -q '"use \`tome adopt\` (Phase 14)' crates/tome/src/reassign.rs`
        (the bang is for the acceptance criterion below — the line must be absent).
  </action>
  <verify>
    <automated>cargo test -p tome --lib reassign::tests</automated>
  </verify>
  <acceptance_criteria>
    - The literal stub error string is gone: `! grep -q "use \`tome adopt\` (Phase 14)" crates/tome/src/reassign.rs`
    - `grep -q "from_directory: Option<DirectoryName>" crates/tome/src/reassign.rs` succeeds
    - `grep -q "pub force: bool" crates/tome/src/reassign.rs` succeeds (on ReassignPlan)
    - `grep -q "is_discovery()" crates/tome/src/reassign.rs` succeeds (D-A2 check uses this method)
    - `grep -q "with different content" crates/tome/src/reassign.rs` succeeds (D-A1 message)
    - `grep -q "target-only" crates/tome/src/reassign.rs` succeeds (D-A2 message)
    - `grep -q "entry.previous_source = None" crates/tome/src/reassign.rs` succeeds (D-C1 closure)
    - `cargo test -p tome --lib reassign::tests::plan_accepts_unowned_input` exits 0
    - `cargo test -p tome --lib reassign::tests::plan_rejects_target_only_role` exits 0
    - `cargo test -p tome --lib reassign::tests::plan_refuses_different_content_collision_without_force` exits 0
    - `cargo test -p tome --lib reassign::tests::plan_force_bypasses_different_content_collision` exits 0
    - `cargo test -p tome --lib reassign::tests::plan_same_content_collision_takes_relink_path` exits 0
    - `cargo test -p tome --lib reassign::tests::execute_clears_previous_source_on_re_anchor` exits 0
    - Pre-existing reassign tests still pass: `cargo test -p tome --lib reassign::tests` exits 0
  </acceptance_criteria>
  <done>
    The Phase-14 stub error is deleted. `plan()` accepts Unowned input, refuses target-only roles (D-A2), refuses different-content collisions without `--force` (D-A1), and `execute()` clears `previous_source` on re-anchor. All 6 new tests + existing tests green.
  </done>
</task>

<task type="auto">
  <name>Task 2: Wire `--force` from clap into `reassign::plan` in lib.rs</name>
  <read_first>
    - crates/tome/src/lib.rs (the Command::Reassign arm; after 14-03 it is `Command::Reassign { skill, to, force } => { ... let _ = force; ... }`)
    - crates/tome/src/reassign.rs (the new plan() signature with `force` arg from Task 1)
  </read_first>
  <action>
    1. **Locate** the `Command::Reassign { skill, to, force } => { ... }` arm in `lib.rs::run` (around line 516-551).
    2. **Remove** the `let _ = force;` placeholder line (added by 14-03).
    3. **Update the `reassign::plan` call** to pass `force` as the 7th argument:

    ```rust
    let plan = reassign::plan(&skill, &to, &config, &paths, &manifest, false, force)?;
    ```

    4. **Locate the `Command::Fork` arm** (lib.rs:552-onwards). Today's call site is `reassign::plan(&skill, &to, &config, &paths, &manifest, true)?` — add `force` as the 7th arg. The existing `Fork` variant already carries a `force: bool` (cli.rs:208). Update:

    ```rust
    let plan = reassign::plan(&skill, &to, &config, &paths, &manifest, true, force)?;
    ```

    Note: `Fork.force` was previously the "skip confirmation" flag for fork. Phase 14's reassign `force` is a different concept (D-A1 content-hash bypass). For Fork, the `force` semantics now MERGE: it both skips confirmation AND bypasses D-A1. This is acceptable per D-A1 ("Hardens behaviour for BOTH Owned→Owned and Unowned→Owned reassigns") — fork uses the same plan/execute machinery. If the executor judges this change to fork's semantics as out of scope, document the deviation in the task summary; the simplest path is: fork's `force` flag means both things, and the user's existing mental model ("--force on fork bypasses safety checks") still holds.

    5. **Update the rendered output line** in the Reassign arm. Today's lib.rs:543-548 reads:

    ```rust
    println!(
        "{} '{}' from '{}' to '{}'",
        style("Reassigned").green(),
        style(&skill).cyan(),
        style(&plan.from_directory).cyan(),
        style(&to).cyan(),
    );
    ```

    Replace the `style(&plan.from_directory)` substitution with conditional rendering since `from_directory` is now `Option<DirectoryName>`:

    ```rust
    let from_label = match &plan.from_directory {
        Some(d) => style(d.as_str().to_string()).cyan().to_string(),
        None => style("Unowned").yellow().to_string(),
    };
    println!(
        "{} '{}' from '{}' to '{}'",
        style("Reassigned").green(),
        style(&skill).cyan(),
        from_label,
        style(&to).cyan(),
    );
    ```

    6. **Search for any other call sites** of `reassign::plan` in the codebase: `rg "reassign::plan\(" crates/tome/src crates/tome/tests`. Update each to pass the new 7th arg.

    7. Run `cargo test -p tome` end-to-end and `cargo clippy --all-targets -p tome -- -D warnings` to verify the wiring.
  </action>
  <verify>
    <automated>cargo test -p tome --lib reassign::tests; cargo test -p tome --test cli; cargo clippy --all-targets -p tome -- -D warnings</automated>
  </verify>
  <acceptance_criteria>
    - `grep -q "reassign::plan(&skill, &to, &config, &paths, &manifest, false, force)" crates/tome/src/lib.rs` succeeds (Reassign arm)
    - `grep -q "reassign::plan(&skill, &to, &config, &paths, &manifest, true, force)" crates/tome/src/lib.rs` succeeds (Fork arm)
    - `! grep -q "let _ = force;" crates/tome/src/lib.rs` (the placeholder is gone)
    - `cargo test -p tome --lib reassign::tests` exits 0
    - `cargo test -p tome --test cli` exits 0
    - `cargo clippy --all-targets -p tome -- -D warnings` exits 0
  </acceptance_criteria>
  <done>
    `--force` flag wired from CLI through to `reassign::plan`. Fork uses the same path with `is_fork=true, force=force`. Output rendering handles `from_directory: Option<DirectoryName>`. All tests pass.
  </done>
</task>

</tasks>

<verification>
- `cargo test -p tome` exits 0
- `cargo clippy --all-targets -p tome -- -D warnings` exits 0
- The Phase-14-pointing stub error message is gone from the codebase
- Manual smoke test: `tome reassign <unowned-skill> --to <good-dir>` succeeds; the same command into a target-only dir returns the D-A2 error
</verification>

<success_criteria>
- UNOWN-01 delivered: `tome reassign <unowned-skill> --to <dir>` re-anchors the skill (source_name flips None→Some, previous_source clears).
- D-A1: different-content collision refused with verbatim message; `--force` bypasses; same-content relinks.
- D-A2: target-only directory rejected with verbatim message.
- D-C1 closure: `previous_source` cleared on re-anchor.
- D-API-1 stub error string deleted.
- 6 new unit tests + Owned→Owned regression tests all pass.
</success_criteria>

<output>
After completion, create `.planning/phases/14-unowned-library-lifecycle/14-04-SUMMARY.md`
</output>
