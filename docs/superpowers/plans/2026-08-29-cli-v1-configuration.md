# CLI v1.0 Tag Routing Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development or superpowers:executing-plans to execute task-by-task.

**Goal:** Make Tome the shared authority for skill tags and destination selection: sources import skills with provenance, tags classify individual library skills, and destinations receive skills matching any selected tag.

**Architecture:** Add shared tags to manifest entries and preserve them across consolidation. Profiles and project `.tome.toml` files own destination routes containing tag selectors and per-skill exclusions. The four-layer resolver returns `EffectiveContext { config, routing, ... }`; `SyncOptions` carries `routing` so all sync callers, including Desktop, use the same eligibility decision.

**Tech Stack:** Rust 2024, `serde`, `toml`, `clap`, `tempfile`, `assert_cmd`.

## Global Constraints

- Desktop phases remain paused and are outside acceptance for this CLI release; do not modify or test Desktop. Compatibility exports used only by Desktop may remain temporarily.
- Sources determine provenance only. They never determine routing.
- Tags are shared library metadata in `.tome-manifest.json`; `tome.lock` remains provenance-only.
- A destination receives a skill when it has at least one selected tag; untagged skills distribute nowhere.
- A destination exclusion overrides a matching tag; no per-skill inclusion rule exists.
- `tome add` registers sources only. It has no `--to` flag or routing prompt.
- Every pool, profile, project, settings, manifest, and route write is TOML/JSON round-trip checked and atomic.
- Project configuration is additive, discovered upward as `.tome.toml`, and may add destinations only.
- Released commands do not read or write `machine.toml`; `MachinePrefs` may remain only as an in-memory projection for unchanged safety filters.
- Remove `--machine`, `migrate-library`, `version`, and `remove pool`; retain `--version`, `pool exclude`, and `pool restore`.
- Do not redesign otherwise viable command names or shapes in this change.

## Task 1: Preserve Shared Skill Tags

**Files:**
- Modify: `crates/tome/src/manifest.rs`
- Modify: `crates/tome/src/library.rs`
- Modify: `crates/tome/src/lib.rs`
- Modify: `crates/tome-desktop/tests/perf/synthetic_skills.rs`
- Test: `crates/tome/src/manifest.rs`
- Test: `crates/tome/src/library.rs`

**Interfaces:**
- Produce `SkillTag` as a validated identifier newtype.
- Add `SkillEntry::tags: BTreeSet<SkillTag>` with `#[serde(default)]`.
- Add manifest mutations `add_tag`, `remove_tag`, and `tags_for`.

- [ ] Write failing tests proving invalid tags are rejected, legacy manifest JSON loads as an empty tag set, and a tag survives a content-changing consolidation of the same skill.
- [ ] Run: `cargo test -p tome manifest::tests`; expect failure because `SkillTag` and manifest tags do not exist.
- [ ] Define `SkillTag` beside `SkillName`, rejecting empty, whitespace-only, or path-separator values. Add tags to `SkillEntry`, defaulting missing serialized data to an empty set.
- [ ] In `library::record_in_manifest`, copy tags from the previous manifest entry before replacing it. Update every `SkillEntry` fixture across core and Desktop to include `tags: BTreeSet::new()`.
- [ ] Run: `cargo test -p tome manifest::tests` and `cargo test -p tome library::tests`; expect pass.
- [ ] Commit: `feat: preserve shared library skill tags`.

## Task 2: Define Tag Routes and Four-Layer Resolution

**Files:**
- Create: `crates/tome/src/routing.rs`
- Create: `crates/tome/src/project.rs`
- Modify: `crates/tome/src/profiles.rs`, `crates/tome/src/migration_profiles.rs`, and `crates/tome/src/cli.rs`
- Modify: `crates/tome/src/lib.rs`
- Modify: `crates/tome-desktop/src/commands.rs`
- Test: `crates/tome/src/routing.rs`
- Test: `crates/tome/src/project.rs`
- Test: `crates/tome/tests/cli_profiles.rs`
- Test: `crates/tome/tests/cli_config.rs`

**Interfaces:**

```rust
pub(crate) struct Route {
    tags: BTreeSet<SkillTag>,
    exclude: BTreeSet<SkillName>,
}

pub(crate) struct RoutingPolicy {
    routes: BTreeMap<DirectoryName, Route>,
}

impl RoutingPolicy {
    pub(crate) fn allows(
        &self,
        destination: &DirectoryName,
        skill: &SkillName,
        tags: &BTreeSet<SkillTag>,
    ) -> bool;
}
```

- [ ] Write failing unit tests: a route matches any selected tag, rejects an excluded skill, rejects an untagged skill, and rejects invalid tags.
- [ ] Run: `cargo test -p tome routing::tests`; expect failure because `routing` does not exist.
- [ ] Add `routes: BTreeMap<DirectoryName, Route>` to `MachineProfile`. Add `ProjectConfig { directories, routes }` and `find_project_config(cwd)`, which searches upward for `.tome.toml`.
- [ ] Add `save_project_checked` using serialization round-trip validation and the existing temp-plus-rename pattern. Test a failed write leaves the original project config intact.
- [ ] Delete the released profile-migration command and recovery path before changing the resolver API. A flat legacy config must fail with an error naming the required layered files and must not mention `tome migrate profiles`.
- [ ] Change `load_effective_context` to accept `cwd: Option<&Path>` and return `routing`. CLI commands pass `Some(current_dir)`; paused Desktop commands pass `None` because they have no selected project context. Merge pool Git sources, profile directories, and project destinations before `Config::validate`. Reject duplicate names across every layer before insertion, including Git-source versus destination collisions. Reject a project config with sources, non-target roles, unknown route destinations, or invalid TOML without fallback.
- [ ] Define one routing carrier through the sync API: add `routing: RoutingPolicy` and `settings_path: &Path` to `SyncOptions`, update every CLI, unit-test, migration, and Desktop constructor, and pass routing through `sync`, distribution, and cleanup. Update Desktop `start_sync`, `retry_sync_from`, and `retry_failed_items` together.
- [ ] Replace reconcile's `machine.toml` consent write with `profiles::save_managed_plugin_install(settings_path, value)`: load `LocalSettings`, update only `managed_plugin_install`, then call the existing round-trip checked atomic settings writer. Add a test that reconcile consent updates `settings.toml` and leaves `machine.toml` untouched.
- [ ] Run separately: `cargo test -p tome routing::tests`; `cargo test -p tome project::tests`; `cargo test -p tome --test cli_config`; `cargo test -p tome --test cli_profiles`; `cargo test -p tome-desktop`.
- [ ] Commit: `feat: resolve profile and project tag routes`.

## Task 3: Route Distribution and Remove Stale Links

**Files:**
- Modify: `crates/tome/src/distribute.rs`
- Modify: `crates/tome/src/cleanup.rs`
- Modify: `crates/tome/src/lib.rs`
- Test: `crates/tome/src/distribute.rs`
- Test: `crates/tome/tests/cli_sync.rs`

**Interfaces:**
- `distribute_to_directory_with_sources` receives `&RoutingPolicy`.
- A skill is eligible only when `routing.allows(...)` is true, then existing machine safety filters apply.

- [ ] Write failing tests for tagged matched skill, tagged unmatched skill, untagged skill, and matching tagged skill excluded at one destination.
- [ ] Run: `cargo test -p tome distribute::tests`; expect failure because distribution currently links every library entry.
- [ ] Read tags from the manifest and route-gate before `MachinePrefs::is_skill_allowed`. Keep existing same-source and same-tool package-manager protections after the route gate; provenance remains necessary for those safety checks.
- [ ] Reuse the same eligibility predicate in target cleanup. Remove only symlinks resolving inside the library when a skill becomes unrouted; preserve foreign-symlink protection.
- [ ] Add integration coverage showing an upstream source with two tagged skills routes each to different profile/project destinations, while a new untagged upstream skill remains only in the library.
- [ ] Run: `cargo test -p tome distribute::tests`; `cargo test -p tome --test cli_sync`.
- [ ] Commit: `feat: distribute skills by shared tags`.

## Task 4: Add Tag and Destination-Route Commands

**Files:**
- Modify: `crates/tome/src/cli.rs`
- Modify: `crates/tome/src/add.rs`
- Modify: `crates/tome/src/profiles.rs`
- Modify: `crates/tome/src/project.rs`
- Modify: `crates/tome/src/lib.rs`
- Test: `crates/tome/tests/cli_add.rs`
- Test: `crates/tome/tests/cli_config.rs`

**CLI contract:**

```text
tome add <git-url> [--name] [--branch|--tag|--rev] [--subdir]
tome tag add <skill> <tag>
tome tag remove <skill> <tag>
tome tag list [<skill>]
tome route tag add --to <destination> <tag>
tome route tag remove --to <destination> <tag>
tome route exclude add --to <destination> <skill>
tome route exclude remove --to <destination> <skill>
```

- [ ] Write failing CLI tests: Git add writes only repository policy and rejects `--role`; tag mutations persist in the manifest; a profile route is checked-written; a project route is checked-written only inside its project; unknown skill, tag, or destination errors perform no write.
- [ ] Run: `cargo test -p tome --test cli_add`; `cargo test -p tome --test cli_config`; expect failure.
- [ ] Move Git source declarations to pool policy and remove `--to` from `Add`. Keep local paths profile-owned. Reject `--role` for Git inputs because all shared Git sources are discovery-only.
- [ ] Implement tag commands against atomic manifest mutation. Implement route commands against the owning profile or discovered project config; project route mutation outside the project tree must fail.
- [ ] Run: `cargo test -p tome --test cli_add`; `cargo test -p tome --test cli_config`.
- [ ] Commit: `feat: manage shared tags and destination routes`.

## Task 5: Remove Obsolete CLI Surface

**Files:**
- Modify: `crates/tome/src/cli.rs`, `crates/tome/src/lib.rs`, `crates/tome/src/main.rs`, `crates/tome/src/library.rs`
- Delete: `crates/tome/src/migration_v010.rs`
- Delete: `crates/tome/tests/cli_migrate_library.rs`
- Modify: tests covering CLI parsing, help, migration, and pool removal

- [ ] Add failing parser/help tests proving `migrate-library`, `version`, and `remove pool` are rejected, while `--version`, `pool exclude`, and `pool restore` still parse. Global `--machine` remains until its callers are removed in Task 6.
- [ ] Remove `Command::MigrateLibrary`, `Command::Version`, and `RemoveKind::Pool`, plus their dispatch helpers and command-specific tests. Delete `migration_v010`, its crate export and `main.rs` error downcast, `sync()` v0.9-shape detection/hint, `library.rs` v0.9 symlink fallback/hint, and `cli_migrate_library.rs`.
- [ ] Keep early `--version` behavior owned by Clap. Keep shared pool exclusion and restoration only under `tome pool`.
- [ ] Remove obsolete examples and comments from the affected command definitions without renaming or restructuring other commands.
- [ ] Run: `cargo test -p tome cli::tests`; `cargo test -p tome --test cli_misc`; `cargo test -p tome --test cli_pool`; `cargo clippy -p tome --all-targets -- -D warnings`.
- [ ] Commit: `refactor: remove obsolete CLI compatibility commands`.

## Task 6: Remove Core File-Backed Machine Preferences

**Files:**
- Modify: `crates/tome/src/cli.rs`, `crates/tome/src/machine.rs`, `crates/tome/src/actions.rs`, `crates/tome/src/browse/app.rs`, `crates/tome/src/browse/mod.rs`, `crates/tome/src/cleanup.rs`, `crates/tome/src/status.rs`, `crates/tome/src/skill.rs`, `crates/tome/src/reconcile.rs`, `crates/tome/src/remove.rs`, and `crates/tome/src/lib.rs`
- Modify: `crates/tome/src/config/mod.rs`
- Delete: `crates/tome/src/config/overrides.rs`
- Modify or delete: legacy core machine-preference tests
- Test: `crates/tome/tests/cli_profiles.rs`, `crates/tome/tests/cli_status.rs`, `crates/tome/tests/cli_sync_reconcile.rs`, `crates/tome/tests/cli_remove.rs`

- [ ] Inventory every core production and test call to `machine::load`, `machine::save`, `default_machine_path`, `Cli::machine`, machine preview/apply APIs, and machine-path overrides. Classify each as delete, local-settings replacement, route-exclusion replacement, or in-memory projection.
- [ ] Write failing tests proving no released CLI flow creates or changes `machine.toml`, legacy files cannot alter tag-route eligibility, status reports the selected profile and project-context destinations, and invalid project config fails without fallback.
- [ ] Remove global `--machine` together with every core `cli.machine` and `resolve_machine_path` caller. Keep `MachinePrefs` and `DirectoryPrefs` as in-memory projections constructed by `profiles::load_effective_context`.
- [ ] Remove core use of file loading, atomic saving, preview diffs, path overrides, and default machine-path resolution. Retain exported persistence APIs used only by paused Desktop as temporary compatibility code; no released CLI command may call them.
- [ ] Keep managed-plugin consent in `LocalSettings`. Make destination-ambiguous TUI/Desktop disable actions unavailable with an actionable `tome route exclude` message; do not choose a destination implicitly.
- [ ] Remove machine preference summaries, cleanup membership edits and guidance, legacy override loading, and stale core tests.
- [ ] Run core checks only while iterating: `cargo test -p tome --test cli_profiles`; `cargo test -p tome --test cli_status`; `cargo test -p tome --test cli_sync_reconcile`; `cargo test -p tome --test cli_remove`; `cargo clippy -p tome --all-targets -- -D warnings`.
- [ ] Commit: `refactor: remove core machine preference persistence`.

## Task 7: Document, Verify, and Release-Gate

**Files:**
- Modify: `Makefile`, `README.md`, `docs/src/configuration.md`, `docs/src/commands.md`, `docs/src/features.md`, `docs/src/introduction.md`, `docs/src/architecture.md`, `docs/src/cross-machine-sync.md`, `docs/src/vercel-skills-comparison.md`, `CHANGELOG.md`
- Modify: CLI and Desktop fixtures affected by the new manifest field

- [ ] Document repository sources, shared manifest tags, OR-match destination routes, project `.tome.toml` discovery, and the absence of `tome add --to` and `--machine`.
- [ ] Document that native tool plugins remain tool-specific adapters while Tome owns desired skill/plugin state and portable-skill routing.
- [ ] Remove documentation for `machine.toml`, `migrate-library`, `tome version`, and `tome remove pool`. Document `tome --version`, `tome pool exclude`, `tome pool restore`, and explicit destination exclusions.
- [ ] Add an Unreleased breaking-change note for untagged-by-default distribution, layered configuration, and obsolete CLI removal.
- [ ] Add `.PHONY` targets `test-core` and `test-desktop`; implement `test-core` as `cargo test -p tome`, `test-desktop` as `cargo test -p tome-desktop`, and keep `test` as their aggregate.
- [ ] Run focused aggregate gates separately: `make fmt-check`; `cargo clippy -p tome --all-targets -- -D warnings`; `make test-core`; `make typos`; `git diff --check`. Do not run Desktop builds or tests.
- [ ] Commit: `docs: describe tag-based Tome configuration`.

## Plan Self-Review

- Shared tags and tag preservation: Task 1.
- OR tag-match, exclusions, untagged behavior, and stale-link cleanup: Task 3.
- Repository/profile/project ownership and atomic project writes: Task 2.
- Source registration separated from routing: Task 4.
- Obsolete commands and flags: Task 5.
- Core file-backed legacy callers plus status behavior: Task 6.
- Paused Desktop remains explicitly outside this release scope.
- Documentation and core verification: Task 7.
