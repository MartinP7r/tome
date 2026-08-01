# Tome Agent Skill Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Ship a tested `using-tome` agent skill through both Tome and Claude plugin installation, and offer it during interactive `tome init` without changing noninteractive behavior.

**Architecture:** Keep the user-facing skill under the repository-standard `skills/` tree with progressively disclosed references. Treat Claude plugin metadata as an alternate distribution wrapper around the same files. Add the official Git source to wizard state through pure, unit-tested helpers; the existing checked config save and post-init sync perform validation and cloning.

**Tech Stack:** Agent Skills `SKILL.md`, Claude Code plugin manifests, Rust 2024, `dialoguer`, `serde`/TOML, `assert_cmd`, Markdown documentation.

## Global Constraints

- Store the skill at `skills/using-tome/`; do not add project-local `.agents/skills` or `.claude/skills` copies.
- Ship one cohesive user-operations skill; contributor, GSD, and release workflows remain out of scope.
- Keep exact version-sensitive command syntax authoritative in `tome <command> --help`.
- The Claude plugin version starts at `1.0.0` and is independent of the Tome binary version.
- The interactive init recommendation defaults to yes and discloses that post-init sync clones the repository immediately.
- `tome init --no-input` must not add or clone the official repository.
- Never recursively spawn `tome add` from `tome init`; mutate the in-memory directory map and use `Config::save_checked` through the existing wizard path.
- Do not implement automatic `skills/` subdirectory adoption; retain it as the existing deferred task.
- Preserve CLI behavior outside the explicitly approved interactive prompt.

---

### Task 1: Develop the `using-tome` skill with baseline tests

**Files:**
- Create: `skills/using-tome/SKILL.md`
- Create: `skills/using-tome/references/configuration.md`
- Create: `skills/using-tome/references/operations.md`
- Create: `skills/using-tome/references/troubleshooting.md`
- Reference: `README.md`
- Reference: `docs/src/commands.md`
- Reference: `docs/src/configuration.md`
- Reference: `docs/src/cross-machine-sync.md`

**Interfaces:**
- Consumes: Current CLI behavior documented by `tome <command> --help` and the four user docs above.
- Produces: A portable `using-tome` skill discovered from `skills/using-tome/SKILL.md`; Task 3 packages this exact directory as a Claude plugin.

- [ ] **Step 1: Run three RED baseline scenarios without a Tome skill**

Dispatch one fresh agent per prompt before creating `skills/using-tome/`. Do not include Tome documentation in the prompts.

```text
Scenario A:
I use Tome. Add ~/.pfw/skills as a package-manager-owned source, then sync safely.
State the commands you would run and what you would verify first.

Scenario B:
I use Tome. Install the skills from MartinP7r/tome, where SKILL.md files live
under the repository's skills/ directory, and distribute them to all configured tools.

Scenario C:
Tome sync completed strangely and one target is missing skills. Repair it.
Do not ask me follow-up questions; state the diagnostic and recovery commands in order.
```

Record each response in the task execution summary. RED succeeds when at least one response exhibits a target failure: chooses `synced` for the package-manager directory, omits `--subdir skills`, skips state inspection/dry-run, or recommends direct manifest/lockfile edits before `status`/`doctor`. If all three agents already satisfy every target, strengthen the prompts until one concrete baseline gap is observed; do not author the skill without a failing baseline.

- [ ] **Step 2: Write `SKILL.md` with strong triggers and a concise operating contract**

Use this frontmatter exactly, then write the body in imperative form:

```markdown
---
name: using-tome
description: Use when setting up Tome, adding skill repositories or local skill directories, syncing skills across AI tools, changing Tome directory roles or machine preferences, diagnosing missing skills or failed syncs, or recovering a Tome-managed skill library.
---

# Using Tome
```

Keep the body under 700 words and include these sections in order:

1. `## Operating Contract` with these non-negotiable rules:
   - inspect with `tome status`, `tome config`, or `tome doctor` before repair;
   - run `tome <command> --help` before version-sensitive invocations;
   - use `--dry-run` before supported mutating commands;
   - do not hand-edit `.tome-manifest.json` or `tome.lock` as a first response;
   - preserve library content and take a backup before destructive recovery.
2. `## Choose The Workflow` mapping setup/configuration to `references/configuration.md`, ordinary command execution to `references/operations.md`, and unexpected state/failures to `references/troubleshooting.md`.
3. `## Safe Starting Sequence` with `tome status`, `tome config`, `tome doctor`, and the relevant command's `--help`; state that not every request needs all four commands.
4. `## Role Decision` with the four roles and the package-manager rule: use `managed` for package-manager-owned directories, `source` for generic read-only discovery, `target` for distribution only, and `synced` only when write-back is intended.
5. `## Command Map` listing init, add, sync, status/list/browse, doctor, remove/reassign/fork, backup, relocate, eject, and lint with one-line purposes.
6. `## Resources` linking all three reference files.

- [ ] **Step 3: Write the configuration reference**

Create `references/configuration.md` with these concrete sections and examples:

```markdown
# Tome Configuration

## Configuration Files
- Portable config: `~/.tome/tome.toml`
- Per-machine preferences: `~/.config/tome/machine.toml`

## Directory Roles
```

Include the exact behavioral distinction between `managed`, `source`, `target`, and `synced`; warn that `type = "directory"` defaults to `synced`. Include these examples:

```bash
tome add ~/.pfw/skills --role managed
tome add ~/work/team-skills --role source
tome add MartinP7r/tome --subdir skills --role source
tome add owner/repo/tree/main/skills --role source
```

Explain that `--branch`, `--tag`, and `--rev` are mutually exclusive and exact flags must be confirmed with `tome add --help`. Cover `disabled`, `disabled_directories`, per-directory enabled/disabled lists, `directory_overrides`, and `auto_install_plugins` without reproducing the full TOML schema; link agents to `docs/src/configuration.md` when operating inside the Tome repository and otherwise direct them to `tome config` plus the installed docs.

- [ ] **Step 4: Write the operations reference**

Create `references/operations.md` with ordered workflows:

```text
Initial setup:        tome init -> tome status -> tome doctor
Routine sync:         tome status -> tome sync --dry-run -> tome sync -> tome status
Add Git source:       tome add ... --dry-run -> tome add ... -> tome sync
Remove directory:     tome remove dir <name> --dry-run -> command without --dry-run
Delete Unowned skill: verify Unowned -> tome remove skill <name> --dry-run -> apply
Reassign/fork:        inspect ownership -> command --help -> dry-run -> apply -> sync
Backup/restore:       tome backup --help -> snapshot before risky operation
Relocate/eject:       status + backup -> command --help -> dry-run -> apply
```

State that support for `--dry-run` must be confirmed from command help rather than assumed. Explain that `tome sync` is the operation that reconciles, discovers, consolidates, distributes, cleans up, and saves; do not represent direct target symlink edits as a normal operation.

- [ ] **Step 5: Write the troubleshooting reference**

Create `references/troubleshooting.md` with this diagnosis order:

1. Capture `tome status` and `tome doctor`.
2. Inspect effective config with `tome config`.
3. Reproduce safely with `tome sync --dry-run --verbose`; use `TOME_LOG=tome=debug` only when more detail is needed.
4. Classify the issue as missing source path, wrong role, missing Git `subdir`, machine disable/filter, Unowned ownership, foreign target entry, or failed managed-plugin reconciliation.
5. Apply the narrow Tome command, rerun sync, and verify status/doctor.

Include explicit recovery guidance for zero skills under a Git root (`--subdir skills`), package-manager write-back pollution (change role before cleanup), broken symlinks (`doctor`), Unowned skills (`reassign` or confirmed removal), and damaged state (restore from backup before manual reconstruction). State that direct edits to generated manifest/lockfile data require a separately justified last-resort procedure.

- [ ] **Step 6: Validate the skill structurally**

Run:

```bash
cargo run -p tome -- lint skills/using-tome
```

Expected: exit 0 with no frontmatter errors. Then verify every referenced file exists:

```bash
test -f skills/using-tome/SKILL.md
test -f skills/using-tome/references/configuration.md
test -f skills/using-tome/references/operations.md
test -f skills/using-tome/references/troubleshooting.md
```

Expected: all four commands exit 0. Rerun `git ls-files --error-unmatch` after staging in Step 9 to verify that every file enters the commit.

- [ ] **Step 7: Run the GREEN scenarios with the skill**

Dispatch fresh agents with the same three prompts from Step 1 and instruct each agent to read `skills/using-tome/SKILL.md` plus only the references it selects. Require:

- Scenario A chooses `--role managed`, inspects state, and previews supported changes.
- Scenario B uses `--subdir skills --role source`, then syncs and verifies.
- Scenario C starts with status/doctor/config and dry-run diagnostics, does not lead with generated-state edits, and verifies after repair.

If any scenario fails, amend only the guidance responsible for that observed failure and rerun that scenario until it passes.

- [ ] **Step 8: Review skill quality**

Check frontmatter length, imperative voice, trigger specificity, progressive disclosure, command accuracy against `tome --help`, and duplication across the four files. Ensure `SKILL.md` stays below 700 words and each reference has one clear responsibility.

- [ ] **Step 9: Commit the tested skill**

```bash
git add \
  skills/using-tome/SKILL.md \
  skills/using-tome/references/configuration.md \
  skills/using-tome/references/operations.md \
  skills/using-tome/references/troubleshooting.md
git diff --cached --check
git ls-files --error-unmatch \
  skills/using-tome/SKILL.md \
  skills/using-tome/references/configuration.md \
  skills/using-tome/references/operations.md \
  skills/using-tome/references/troubleshooting.md
git commit -m "feat: add Tome operations agent skill"
```

---

### Task 2: Implement local path support in `tome add`

**Files:**
- Modify: `crates/tome/src/cli.rs` (`Command::Add` positional naming/help)
- Modify: `crates/tome/src/lib.rs` (`cmd_add` argument plumbing)
- Modify: `crates/tome/src/add.rs` (source classification, local entry construction, checked save)
- Modify: `crates/tome/tests/cli_add.rs` (local-path end-to-end coverage)

**Interfaces:**
- Consumes: `DirectoryType::Directory::valid_roles()`, `config::expand_tilde`, and `Config::save_checked`.
- Produces: `AddOptions { input, ... }`, private `AddSource::{Local, Git}` classification, and working `tome add ~/.pfw/skills --role managed` behavior used by Task 1 guidance.

- [ ] **Step 1: Write failing source-classification unit tests**

Add table-driven tests in `add.rs::tests` asserting that `/tmp/skills`, `~/.pfw/skills`, `.`, `..`, `./skills`, and `../skills` classify as local, while `owner/repo`, HTTPS URLs, SCP-style SSH URLs, and GitHub tree URLs classify as Git. Include a fixture directory named `owner/repo` and prove classification stays Git; never call `Path::exists()` to decide.

- [ ] **Step 2: Write failing local-entry unit tests**

Drive `add()` with an isolated config path and assert:

```rust
let entry = config.directories().get("skills").unwrap();
assert_eq!(entry.directory_type, DirectoryType::Directory);
assert_eq!(entry.role(), DirectoryRole::Managed);
assert_eq!(entry.git_ref, None);
assert_eq!(entry.subdir, None);
```

Cover default `synced` role, `--name` override, and one rejection test each for local `--branch`, `--tag`, `--rev`, and `--subdir`. Every rejection must leave the config file unchanged.

- [ ] **Step 3: Write the failing CLI integration test**

In `crates/tome/tests/cli_add.rs`, create a temporary HOME and existing `.pfw/skills` directory, then run:

```rust
tome()
    .env("HOME", tmp.path())
    .env("TOME_HOME", &tome_home)
    .args(["add", "~/.pfw/skills", "--role", "managed"])
    .assert()
    .success();
```

Load the written config and assert directory `skills` has `type = directory`, role `managed`, and the expected expanded path. Read raw TOML and assert it preserves `path = "~/.pfw/skills"`.

- [ ] **Step 4: Run focused tests and verify RED**

Run:

```bash
cargo test -p tome add::tests -- --nocapture
cargo test -p tome --test cli_add -- --nocapture
```

Expected: new local tests fail because all inputs currently construct Git entries; existing Git tests remain passing.

- [ ] **Step 5: Implement deterministic source classification**

Rename the positional field from `url` to `input` through `cli.rs`, `lib.rs`, and `AddOptions`. Add:

```rust
enum AddSource {
    Local(PathBuf),
    Git(String),
}

fn classify_source(input: &str) -> AddSource {
    let path = Path::new(input);
    let explicit_relative = matches!(input, "." | "..")
        || input.starts_with("./")
        || input.starts_with("../");
    let tilde = input == "~" || input.starts_with("~/");

    if path.is_absolute() || explicit_relative || tilde {
        AddSource::Local(path.to_path_buf())
    } else {
        AddSource::Git(input.to_string())
    }
}
```

Do not use filesystem existence. Preserve every existing Git parser and warning after the Git branch is selected.

- [ ] **Step 6: Implement local directory construction and checked save**

For `AddSource::Local`, reject any Git ref or subdirectory option, expand tilde for validation/name derivation, derive the default name from the final component, and construct:

```rust
DirectoryConfig {
    path: expanded_path,
    directory_type: DirectoryType::Directory,
    role: opts.role,
    git_ref: None,
    subdir: None,
    override_applied: false,
}
```

Validate explicit roles against `DirectoryType::Directory.valid_roles()`. Keep omitted role as `None` so it resolves to `synced`. Save both local and Git additions with `config.save_checked(opts.config_path)` rather than unchecked `save`; retain dry-run no-write behavior and render local success output as a directory path, not as `git: ...`.

- [ ] **Step 7: Update CLI help without changing flag compatibility**

Use positional value name `URL_OR_PATH`. Explain in long help that explicit local paths are absolute, tilde-prefixed, or dot-relative; bare `owner/repo` remains Git. State that ref/subdirectory flags apply only to Git inputs.

- [ ] **Step 8: Run GREEN tests and regression gates**

Run:

```bash
cargo test -p tome add::tests -- --nocapture
cargo test -p tome --test cli_add -- --nocapture
cargo fmt -- --check
cargo clippy -p tome --all-targets -- -D warnings
```

Expected: all new local tests and all existing Git add tests pass; format and Clippy exit 0.

- [ ] **Step 9: Commit local add support**

```bash
git add crates/tome/src/cli.rs crates/tome/src/lib.rs crates/tome/src/add.rs crates/tome/tests/cli_add.rs
git diff --cached --check
git commit -m "feat(add): support local skill directories" -m "OpenSpec: ship-tome-agent-skill"
```

---

### Task 3: Package the skill as a Claude plugin and marketplace

**Files:**
- Create: `.claude-plugin/plugin.json`
- Create: `.claude-plugin/marketplace.json`
- Test: Claude CLI manifest validation against repository root

**Interfaces:**
- Consumes: `skills/using-tome/` from Task 1 through Claude's default plugin skill discovery.
- Produces: Plugin `tome` version `1.0.0` in marketplace `tome`, installable as `tome@tome`.

- [ ] **Step 1: Establish the failing manifest check**

Run:

```bash
claude plugin validate . --strict
```

Expected: FAIL because `.claude-plugin/plugin.json` and `.claude-plugin/marketplace.json` do not exist.

- [ ] **Step 2: Add the plugin manifest**

Create `.claude-plugin/plugin.json` exactly as:

```json
{
  "name": "tome",
  "displayName": "Tome",
  "version": "1.0.0",
  "description": "Operate Tome skill libraries safely across AI coding tools.",
  "author": {
    "name": "Martin Pfundmair",
    "url": "https://github.com/MartinP7r"
  },
  "homepage": "https://github.com/MartinP7r/tome",
  "repository": "https://github.com/MartinP7r/tome",
  "license": "MIT",
  "keywords": ["agent-skills", "skill-management", "claude-code", "codex"]
}
```

Do not add an explicit `skills` field; Claude's default root `skills/` scan is the contract.

- [ ] **Step 3: Add the marketplace manifest**

Create `.claude-plugin/marketplace.json` exactly as:

```json
{
  "name": "tome",
  "owner": {
    "name": "Martin Pfundmair",
    "url": "https://github.com/MartinP7r"
  },
  "description": "Official agent skills for operating Tome.",
  "plugins": [
    {
      "name": "tome",
      "source": "./",
      "description": "Operate Tome skill libraries safely across AI coding tools."
    }
  ]
}
```

Omit the marketplace entry's `version`; strict mode reads version `1.0.0` from `plugin.json`, preventing duplicate version declarations from drifting.

- [ ] **Step 4: Validate JSON and Claude schemas**

Run:

```bash
jq empty .claude-plugin/plugin.json .claude-plugin/marketplace.json
claude plugin validate . --strict
```

Expected: both commands exit 0; validation recognizes one plugin and the `using-tome` skill without warnings.

- [ ] **Step 5: Test local marketplace discovery without installing globally**

Use a temporary Claude config directory so the test does not alter the user's installed marketplaces:

```bash
tmp_home="$(mktemp -d)"
CLAUDE_CONFIG_DIR="$tmp_home/.claude" HOME="$tmp_home" \
  claude plugin marketplace add "$PWD"
CLAUDE_CONFIG_DIR="$tmp_home/.claude" HOME="$tmp_home" \
  claude plugin install tome@tome --scope user
CLAUDE_CONFIG_DIR="$tmp_home/.claude" HOME="$tmp_home" \
  claude plugin list
rm -rf "$tmp_home"
```

Expected: marketplace add and install exit 0; list contains `tome@tome`. If the installed Claude version requires interactive confirmation despite `--scope user`, use `claude plugin validate . --strict` as the automated gate and perform the local add/install through Claude's documented noninteractive flags rather than modifying real user state.

- [ ] **Step 6: Commit plugin packaging**

```bash
git add .claude-plugin/plugin.json .claude-plugin/marketplace.json
git diff --cached --check
git commit -m "feat: package Tome skill as Claude plugin"
```

---

### Task 4: Add the interactive init recommendation

**Files:**
- Modify: `crates/tome/src/wizard.rs:16-19, 128-380, 461-486, tests module`
- Modify: `crates/tome/tests/cli_init.rs:64-197`

**Interfaces:**
- Consumes: `DirectoryName`, `DirectoryConfig`, `DirectoryType::Git`, and `DirectoryRole::Source` from `crate::config`.
- Produces: `tome_skills_directory() -> Result<(DirectoryName, DirectoryConfig)>`, `has_tome_skills_source(&BTreeMap<DirectoryName, DirectoryConfig>) -> bool`, and `insert_tome_skills_source(&mut BTreeMap<DirectoryName, DirectoryConfig>) -> Result<bool>`.

- [ ] **Step 1: Write failing pure-helper unit tests**

Add tests in `wizard.rs::tests` for these contracts:

```rust
#[test]
fn tome_skills_directory_is_valid_git_source() {
    let (name, directory) = tome_skills_directory().unwrap();
    assert_eq!(name.as_str(), "tome-skills");
    assert_eq!(directory.path, PathBuf::from("https://github.com/MartinP7r/tome"));
    assert_eq!(directory.directory_type, DirectoryType::Git);
    assert_eq!(directory.role(), DirectoryRole::Source);
    assert_eq!(directory.subdir.as_deref(), Some("skills"));
}

#[test]
fn insert_tome_skills_source_adds_once() {
    let mut directories = BTreeMap::new();
    assert!(insert_tome_skills_source(&mut directories).unwrap());
    assert!(!insert_tome_skills_source(&mut directories).unwrap());
    assert_eq!(directories.len(), 1);
}

#[test]
fn insert_tome_skills_source_preserves_name_collision() {
    let mut directories = BTreeMap::new();
    directories.insert(
        DirectoryName::new("tome-skills").unwrap(),
        test_dir("~/other-skills", DirectoryType::Directory, DirectoryRole::Source),
    );

    assert!(!insert_tome_skills_source(&mut directories).unwrap());
    assert_eq!(directories["tome-skills"].path, PathBuf::from("~/other-skills"));
}

#[test]
fn detects_equivalent_tome_skills_source_under_another_name() {
    let (_, directory) = tome_skills_directory().unwrap();
    let mut directories = BTreeMap::new();
    directories.insert(DirectoryName::new("official").unwrap(), directory);

    assert!(has_tome_skills_source(&directories));
    assert!(!insert_tome_skills_source(&mut directories).unwrap());
}
```

- [ ] **Step 2: Write the failing noninteractive regression assertion**

Extend `init_dry_run_no_input_empty_home` in `crates/tome/tests/cli_init.rs`:

```rust
assert!(
    !config.directories().contains_key("tome-skills"),
    "--no-input must not add the network-backed recommendation",
);
assert!(
    !stderr.contains("Add Tome's official agent skills?"),
    "--no-input must not render the interactive recommendation",
);
```

The existing empty-directories assertion remains. This test passes before implementation and is a regression guard, while the new helper tests provide RED.

- [ ] **Step 3: Run focused tests and verify RED**

Run:

```bash
cargo test -p tome wizard::tests::tome_skills -- --nocapture
cargo test -p tome wizard::tests::insert_tome_skills -- --nocapture
cargo test -p tome --test cli_init init_dry_run_no_input_empty_home -- --nocapture
```

Expected: unit tests fail to compile because the helpers do not exist; the CLI regression test passes.

- [ ] **Step 4: Implement the pure helper functions**

Add constants and helpers near `assemble_config`:

```rust
const TOME_SKILLS_NAME: &str = "tome-skills";
const TOME_REPOSITORY_URL: &str = "https://github.com/MartinP7r/tome";
const TOME_SKILLS_SUBDIR: &str = "skills";

fn tome_skills_directory() -> Result<(DirectoryName, DirectoryConfig)> {
    Ok((
        DirectoryName::new(TOME_SKILLS_NAME)?,
        DirectoryConfig {
            path: PathBuf::from(TOME_REPOSITORY_URL),
            directory_type: DirectoryType::Git,
            role: Some(DirectoryRole::Source),
            git_ref: None,
            subdir: Some(TOME_SKILLS_SUBDIR.to_string()),
            override_applied: false,
        },
    ))
}

fn has_tome_skills_source(
    directories: &BTreeMap<DirectoryName, DirectoryConfig>,
) -> bool {
    directories.values().any(|directory| {
        directory.directory_type == DirectoryType::Git
            && directory.path == Path::new(TOME_REPOSITORY_URL)
            && directory.subdir.as_deref() == Some(TOME_SKILLS_SUBDIR)
    })
}

fn insert_tome_skills_source(
    directories: &mut BTreeMap<DirectoryName, DirectoryConfig>,
) -> Result<bool> {
    if has_tome_skills_source(directories)
        || directories.contains_key(TOME_SKILLS_NAME)
    {
        return Ok(false);
    }

    let (name, directory) = tome_skills_directory()?;
    directories.insert(name, directory);
    Ok(true)
}
```

Keep these helpers private; only co-located unit tests need direct access.

- [ ] **Step 5: Add the interactive prompt**

Immediately after `configure_directories(...)` and before discovery, add an interactive-only block:

```rust
if !no_input && !has_tome_skills_source(&directories) && !directories.contains_key(TOME_SKILLS_NAME)
{
    eprintln!("Tome includes an official agent skill for setup, sync, and recovery.");
    eprintln!("  Equivalent command:");
    eprintln!(
        "  {}",
        style("tome add MartinP7r/tome --subdir skills --role source").bold()
    );
    eprintln!("  Accepting will clone the repository during the post-init sync.");

    if Confirm::new()
        .with_prompt("Add Tome's official agent skills?")
        .default(true)
        .interact()?
    {
        insert_tome_skills_source(&mut directories)?;
    }
    eprintln!();
}
```

Do not prompt when `tome-skills` is occupied by another entry; preserve it silently per the approved no-overwrite contract. The summary table later in the wizard shows whether the official source was added.

- [ ] **Step 6: Run focused tests and verify GREEN**

Run:

```bash
cargo test -p tome wizard::tests::tome_skills -- --nocapture
cargo test -p tome wizard::tests::insert_tome_skills -- --nocapture
cargo test -p tome wizard::tests::detects_equivalent -- --nocapture
cargo test -p tome --test cli_init init_dry_run_no_input_empty_home -- --nocapture
cargo test -p tome --test cli_init init_dry_run_no_input_seeded_home -- --nocapture
```

Expected: all tests pass; noninteractive generated config remains network-source-free.

- [ ] **Step 7: Check formatting and lint for the Rust change**

Run:

```bash
cargo fmt -- --check
cargo clippy -p tome --all-targets -- -D warnings
```

Expected: both exit 0. If rustfmt reports changes, run `cargo fmt`, inspect only the intended files, and rerun both checks.

- [ ] **Step 8: Commit the wizard change**

```bash
git add crates/tome/src/wizard.rs crates/tome/tests/cli_init.rs
git diff --cached --check
git commit -m "feat(init): recommend Tome agent skills"
```

---

### Task 5: Document installation, preserve the deferred task, and verify end-to-end

**Files:**
- Modify: `README.md:46-63`
- Modify: `docs/src/commands.md:37-48`
- Modify: `CHANGELOG.md:8`
- Modify: `.planning/todos/pending/2026-06-26-tome-add-auto-detect-subdir.md`
- Move through GSD SDK: `.planning/todos/pending/2026-07-15-define-agent-skills-for-tome.md` to `.planning/todos/completed/`

**Interfaces:**
- Consumes: Skill and local-add behavior from Tasks 1-2, plugin commands from Task 3, and wizard behavior from Task 4.
- Produces: User-facing installation instructions, release notes, retained auto-detect acceptance criteria, and completed planning state.

- [ ] **Step 1: Add README installation documentation**

After Quick Start and before Development, add `## Agent Skill` with the cross-tool route first:

````markdown
## Agent Skill

Tome ships a `using-tome` skill that teaches coding agents how to configure,
sync, diagnose, and recover a Tome library safely. `tome init` offers it during
interactive setup, or add it directly:

```bash
tome add MartinP7r/tome --subdir skills --role source
tome sync
```

Claude Code users can alternatively install the same skill as a plugin:

```bash
claude plugin marketplace add MartinP7r/tome
claude plugin install tome@tome
```
````

- [ ] **Step 2: Document init behavior in the command reference**

Add a `### tome init` section before `### tome sync` in `docs/src/commands.md`. State:

- interactive init offers the official agent skill, defaults to yes, and displays the equivalent `tome add` command;
- accepting registers `MartinP7r/tome` as Git/source with `subdir = "skills"`;
- post-init sync clones it immediately;
- `--no-input` omits the recommendation and performs no new official-repository network request.

- [ ] **Step 3: Add an Unreleased changelog entry**

Restore the standard accumulator above `0.16.4`:

```markdown
## [Unreleased]

### Added

- **Official `using-tome` agent skill** for safe setup, configuration, sync,
  diagnosis, and recovery. The repository now doubles as a Claude plugin
  marketplace (`tome@tome`), and interactive `tome init` can add the same skill
  as a cross-tool Git source. Noninteractive init remains network-source-free.

## [0.16.4] - 2026-07-30
```

At the bottom of `CHANGELOG.md`, replace the stale comparison link and add the released version link:

```markdown
[Unreleased]: https://github.com/MartinP7r/tome/compare/v0.16.4...HEAD
[0.16.4]: https://github.com/MartinP7r/tome/compare/v0.16.3...v0.16.4
```

- [ ] **Step 4: Refine the existing auto-subdirectory task without implementing it**

Append an acceptance criterion to `.planning/todos/pending/2026-06-26-tome-add-auto-detect-subdir.md`:

```markdown
4. Regression fixture: `tome add MartinP7r/tome` detects the repository's
   `skills/using-tome/SKILL.md`, persists `subdir = "skills"`, and reports the
   detected subdirectory. This replaces the explicit `--subdir skills` needed
   by the initial agent-skill release.
```

Do not change production `tome add` code in this task.

- [ ] **Step 5: Complete the fulfilled planning todo through GSD**

Run:

```bash
gsd-sdk query todo.complete 2026-07-15-define-agent-skills-for-tome.md
```

Expected: JSON reports `"completed": true`; the todo moves from `pending/` to `completed/` with a completion date.

- [ ] **Step 6: Run focused package and behavior validation**

Run:

```bash
cargo run -p tome -- lint skills/using-tome
claude plugin validate . --strict
cargo test -p tome wizard::tests -- --nocapture
cargo test -p tome --test cli_init -- --nocapture
```

Expected: all commands exit 0.

- [ ] **Step 7: Run the complete quality gate**

Run:

```bash
make ci
```

Expected: format check, Clippy with `-D warnings`, and all tests pass.

- [ ] **Step 8: Review the complete diff**

Run:

```bash
git status --short
git diff --check
git diff --stat
git diff
```

Confirm `.claude/scheduled_tasks.lock` remains untracked and unstaged. Confirm no automatic subdirectory implementation or unrelated Phase 28 planning changes entered the diff.

- [ ] **Step 9: Commit documentation and planning closure**

```bash
git add \
  README.md \
  docs/src/commands.md \
  CHANGELOG.md \
  .planning/todos/pending/2026-06-26-tome-add-auto-detect-subdir.md \
  .planning/todos/completed/2026-07-15-define-agent-skills-for-tome.md
git diff --cached --check
git commit -m "docs: publish Tome agent skill installation"
```

- [ ] **Step 10: Push and verify remote synchronization**

```bash
git push
git status --short --branch
```

Expected: `main...origin/main` with only the intentionally untouched `?? .claude/scheduled_tasks.lock` entry.
