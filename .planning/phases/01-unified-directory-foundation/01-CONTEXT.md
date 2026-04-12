# Phase 1: Unified Directory Foundation - Context

**Gathered:** 2026-04-12
**Status:** Ready for planning

<domain>
## Phase Boundary

Replace the artificial source/target config split with a unified `[directories.*]` model. Rewrite the wizard to use a merged directory registry with auto-role assignment. Adapt the full sync pipeline (discover, consolidate, distribute, cleanup) and state schema (manifest, lockfile, status, doctor) to work against the new config. Old-format `tome.toml` files must fail with a clear migration hint.

This phase does NOT include: git sources (Phase 2), per-directory skill selection (Phase 2), `tome remove` (Phase 2), `tome add` (Phase 3), browse polish (Phase 3).

</domain>

<decisions>
## Implementation Decisions

### Old Config Error UX
- **D-01:** Let `toml::from_str` fail naturally via `#[serde(deny_unknown_fields)]` on `Config`. After a parse failure, check the raw TOML for `[[sources]]` or `[targets.` and append a migration hint to the error message.
- **D-02:** The hint should read: `hint: tome v0.6 replaced [[sources]] and [targets.*] with [directories.*]. See CHANGELOG.md for migration instructions.`
- **D-03:** `deny_unknown_fields` catches typos and future format drift beyond just old-format keys.

### Role Defaulting in Wizard
- **D-04:** Auto-assign roles from the `KNOWN_DIRECTORIES` registry. Show a summary with **inline descriptions** per directory explaining the role in plain english (e.g. "Managed (read-only, owned by package manager)", "Synced (skills discovered here AND distributed here)").
- **D-05:** Role names are internal jargon — every user-facing display of a role MUST include a parenthetical plain-english explanation. This applies to the wizard, `tome status`, and any error messages mentioning roles.
- **D-06:** When user edits a directory's role, use a `dialoguer::Select` menu showing each valid role (filtered by directory type) with its one-line description.
- **D-07:** ClaudePlugins directories can only be Managed (no role picker shown for them).

### Migration Path
- **D-08:** Migration documented in CHANGELOG.md "Breaking Changes" section with before/after config examples. No standalone MIGRATION.md.
- **D-09:** `tome sync` with no config prints "no config found, run `tome init`" and exits. No auto-launch of wizard. Same as current behavior.

### Test Strategy
- **D-10:** Rewrite tests in lockstep with module conversion. Order: config.rs (new types + unit tests) → discover.rs → library.rs → distribute.rs → cleanup/manifest/lockfile → wizard.rs → status/doctor → cli.rs integration tests.
- **D-11:** Delete old insta snapshots and regenerate fresh via `cargo insta review`. Old snapshot diffs not useful given the scope of format changes.

### Claude's Discretion
- Exact wording of role descriptions (as long as they're plain-english and non-jargon)
- Internal module organization (e.g. whether `DirectoryName` lives in config.rs or gets its own module)
- Whether to keep `TargetName` as a type alias during transition or remove it immediately

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### Design & Architecture
- `docs/v06-implementation-plan.md` — Type definitions, PR plan, design decisions table. Primary design reference for the entire v0.6 rewrite.
- `docs/src/architecture.md` — Current sync pipeline (discover → consolidate → distribute → cleanup). Understand before modifying.

### Requirements
- `.planning/REQUIREMENTS.md` — CFG-01 through CFG-06, PIPE-01 through PIPE-05, WIZ-01 through WIZ-05, MACH-01, STATE-01 through STATE-03. All map to Phase 1.
- `.planning/ROADMAP.md` — Phase 1 success criteria (5 criteria that must be TRUE).

### Config & Types
- `crates/tome/src/config.rs` — Current `Config`, `TargetName`, `TargetConfig`, `Source`, `SourceType`. All being replaced.
- `crates/tome/src/discover.rs` — Current `SkillName`, `DiscoveredSkill`, `SkillOrigin`. Discovery logic needs role-based adaptation.
- `crates/tome/src/wizard.rs` — Current `KNOWN_SOURCES`, `KNOWN_TARGETS`, `find_source_target_overlaps()`. All being replaced with merged `KNOWN_DIRECTORIES`.

### Pipeline
- `crates/tome/src/library.rs` — Consolidation strategies (managed=symlink, local=copy). Strategy selection changes from source_type to role.
- `crates/tome/src/distribute.rs` — Distribution + `shares_tool_root()` circular symlink prevention. Being replaced with manifest-based origin check.
- `crates/tome/src/cleanup.rs` — Stale removal. Needs to iterate directories by role.

### State
- `crates/tome/src/manifest.rs` — `.tome-manifest.json` with `source_name` field. Field name preserved, populated from directory name.
- `crates/tome/src/lockfile.rs` — `tome.lock` with `source_name` field. Same treatment.
- `crates/tome/src/status.rs` — Needs DirectoryStatus replacing SourceStatus/TargetStatus.
- `crates/tome/src/doctor.rs` — Diagnostic output needs directory-role awareness.
- `crates/tome/src/machine.rs` — `disabled_targets` renamed to `disabled_directories`.

### Testing
- `crates/tome/tests/cli.rs` — Integration tests with TestEnvBuilder. Rewritten last after all modules compile.
- `docs/src/test-setup.md` — Test architecture reference.

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets
- `SkillName` newtype: Validation pattern reusable for `DirectoryName` (same validate_identifier logic)
- `TargetName` newtype: Can be renamed/replaced with `DirectoryName` — same trait impls needed (Display, AsRef, Borrow, Deserialize with validation)
- `TestEnvBuilder` in integration tests: Needs config format update but the builder pattern itself is solid
- `dialoguer` wizard patterns: Select, MultiSelect, Input, Confirm all used in current wizard — reusable for new role picker

### Established Patterns
- Newtype + validate_identifier for domain types (SkillName, TargetName → DirectoryName)
- `#[serde(transparent)]` with custom Deserialize for validated deserialization
- `BTreeMap` for ordered config maps (already used for targets, natural fit for directories)
- Atomic temp+rename writes for manifest/lockfile/machine.toml
- `pub(crate)` for internal helpers, `pub` for cross-module API

### Integration Points
- `lib.rs::sync()` — orchestrator that calls discover → consolidate → distribute → cleanup. Entry point for pipeline changes.
- `lib.rs::run()` — command dispatch. Status, doctor, lint, browse all read config and need directory-aware output.
- `config.rs::Config` — loaded early in `run()`, threaded through everything. The central type being replaced.

</code_context>

<specifics>
## Specific Ideas

- Role descriptions must be plain-english, not jargon. "Synced (skills discovered here AND distributed here)" not just "Synced".
- Error hint for old config should include a link/reference to CHANGELOG.md migration section.
- Wizard should filter role picker options by directory type (ClaudePlugins = Managed only).

</specifics>

<deferred>
## Deferred Ideas

None — discussion stayed within phase scope.

</deferred>

---

*Phase: 01-unified-directory-foundation*
*Context gathered: 2026-04-12*
