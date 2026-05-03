# Phase 11: Library-canonical core - Context

**Gathered:** 2026-05-03
**Status:** Ready for planning

<domain>
## Phase Boundary

The library becomes the single source of truth for skills.

- **Managed skills become real directory copies** (not symlinks into machine-specific cache paths). LIB-01.
- **Source removal preserves library content** (Case 1 → Unowned state). LIB-04.
- **A one-shot `tome migrate-library` CLI command** converts v0.9-shape libraries to v0.10-shape; deletable in v0.11+. LIB-05.
- **The manifest schema (`SkillEntry.source_name`)** widens to `Option<DirectoryName>` to represent Unowned entries. LIB-03. The lockfile schema (`LockEntry.source_name`) gets the same lift.
- **`Manifest.managed: bool` semantics shift** from "stored as symlink" to "update channel" (managed = upstream sync feeds updates into library). Field name kept; documentation updated. LIB-02.

**Out of scope for this phase** (handled by other phases or out of v0.10 entirely):

- `tome adopt` / `tome forget` lifecycle commands → Phase 14 (UNOWN-01, UNOWN-02).
- Surfacing Unowned state in `tome status` / `tome doctor` text + JSON → Phase 14 (UNOWN-03).
- `MarketplaceAdapter` trait + `ClaudeMarketplaceAdapter` + `GitAdapter` wrap → Phase 12 (ADP-01..04).
- Drift detection at sync time (Match/Drift/Vanished classification, install consent prompt, edit-in-library detection) → Phase 13 (RECON-01..05). NOTE: Phase 11 still locks the *contract* (manifest/lockfile shape) the drift detection consumes — see D-08.
- Cleanup-message UX rewrite (3-bucket partition for stale skills) → Phase 16 (UX-01).
- Local-skill source-side cleanup (e.g. removing the 15 orphaned `personal-skills` entries from `~/dev/coding-agent-files/.claude/skills/`) → **deferred past v0.10 entirely**. Manual cleanup; v0.10's new architecture prevents recurrence. See D-07.

</domain>

<decisions>
## Implementation Decisions

### Migration mechanism (LIB-05)

- **D-01 (mechanism shape):** Migration is a one-shot **`tome migrate-library`** CLI command, NOT auto-on-first-sync. No consent-persistence flag in `machine.toml` is needed. The detection + prompt + conversion logic lives in a transitional module **`crates/tome/src/migration_v010.rs`** with module-level doc comment marking it for removal in v0.11+ (file v0.11 follow-up issue at v0.10 ship time).
- **D-02 (sync-time refusal):** `tome sync` detects v0.9-shape entries and refuses with a hint. Detection is one isolated check in `lib.rs::sync` before the consolidate phase; deletes cleanly with the rest of `migration_v010.rs` in v0.11. Error message follows the existing Conflict / Why / Suggestion template (per Phase 7 D-10): something close to `error: library is in v0.9 shape (managed skills are symlinks). Run \`tome migrate-library\` first to convert to v0.10 shape.`
- **D-03 (detection precision):** Manifest-anchored. A `library_dir/<name>` qualifies for migration ONLY when ALL of: (a) the path is a symlink, AND (b) `manifest[name].managed == true`, AND (c) `manifest.contains_key(name)`. Never touches user-created symlinks tome didn't put there.
- **D-04 (broken symlinks):** Skip with stderr warning AND **preserve the broken symlink in place** (do NOT delete it). The symlink target string carries metadata about where the original source lived (e.g. `~/.claude/plugins/cache/claude-plugins-official/superpowers/5.0.7/skills/...`). Library stays partially-migrated; subsequent `tome sync` keeps refusing per D-02 until the user resolves manually.
- **D-05 (exit code on partial):** `tome migrate-library` exits non-zero on ANY failure — broken-symlink skips OR IO/permission errors. Final summary uses the SAFE-01 pattern from Phase 8 (`crates/tome/src/remove.rs::FailureKind::ALL`): `⚠ N converted · K skipped (broken source) · M failed`. Re-running is idempotent; user fixes underlying issues and re-runs.
- **D-06 (filesystem-only):** Migration is filesystem-only — no manifest mutation needed during conversion. The manifest entry for a managed skill records `source_path` (cache path), `content_hash` (hash of source content), `managed: true`. After symlink → real-dir conversion, all three fields are still correct and unchanged. Detection on re-run is purely filesystem-based via D-03. Implication: a partial migration leaves a clean, recoverable state — re-running picks up where it left off without consistency-recovery code.

### Local-skill source-side cleanup (deliberately deferred)

- **D-07 (manual cleanup):** v0.10 does NOT touch source-side duplicates of local skills (e.g. the 15 orphaned `personal-skills` entries at `~/dev/coding-agent-files/.claude/skills/`). The user cleans these up manually after running `tome migrate-library`. Rationale: v0.10's new architecture (LIB-04 source removal preserves library content) prevents recurrence; existing duplicates are a one-time manual fix; expanding scope to delete user files outside `library_dir` is too risky.

### Drift detection contract (cross-phase; locks Phase 11 schema)

- **D-08 (drift basis):** Drift detection for managed sources (Phase 13 / RECON-01..05) uses **content_hash comparison**, NOT version strings. On every `tome sync`, walk each managed source dir, compute fresh `content_hash`, compare against the manifest's recorded hash. Drift = hash differs. The `version` string from `claude plugin list --json` becomes display-only — used in human-readable diff output ("plugin X: 5.0.5 → 5.0.7") but never consulted for the drift signal.

  This decision lives outside Phase 11 implementation work but **PINS the manifest/lockfile field contract** Phase 11 ships: `content_hash: ContentHash` is the authoritative drift signal; `version: Option<String>` stays as a nice-to-have for UX. RECON-01 wording in REQUIREMENTS.md will need updating to reflect this; flag in the planner's traceability check.

### Source-removal → Unowned transition (LIB-04)

- **D-09 (scope):** Case 1 only. **Source removed from `tome.toml`** triggers the transition; manifest entries' `source_name` becomes `None`, library content is preserved. **Case 2** (source still configured but file deleted from disk) keeps today's behavior — library copy is also deleted on next sync. Rationale: a configured source removing a file is intentional; tome respects the user's deletion. LIB-04 wording in REQUIREMENTS.md is consistent with this (specifies "removing a `[directories.*]` entry from `tome.toml`").
- **D-10 (trigger points — hybrid):** Two trigger points:
  1. **`tome remove <dir>`** explicitly sets `source_name = None` on all manifest entries it owns when removing the directory entry from `tome.toml` (the explicit path).
  2. **Cleanup phase during `tome sync`** detects orphans (manifest entries whose `source_name` isn't a key in current `config.directories`) and transitions them. This is the safety net for users who manually edit `tome.toml` outside `tome remove`.

  Both checks are simple; both are deletable in one place each in the future if the model evolves.
- **D-11 (distribution semantics):** Unowned skills are distributed to targets normally. The library is canonical; its content gets symlinked into `synced` / `target` directories like any other library entry. User stops distribution explicitly via `tome forget <skill>` (Phase 14) or by adding the skill to `machine.toml::disabled`. Removing a source from config does NOT auto-stop distribution.

### Manifest + lockfile schema (LIB-03)

- **D-12 (Option<DirectoryName>):** `SkillEntry.source_name` becomes `Option<DirectoryName>` with `#[serde(default, skip_serializing_if = "Option::is_none")]`. Old manifests (`source_name: "foo"`) parse as `Some(DirectoryName::new("foo")?)` automatically via serde's natural Option handling + DirectoryName's transparent deserialize. New Unowned manifests (`source_name: null` or missing) parse as `None`. **No custom deserializer needed; no one-shot migration code.**
- **D-13 (constructor shape):** `SkillEntry::new` API: two constructors. Keep `SkillEntry::new(source_path, source_name: DirectoryName, content_hash, managed)` for owned entries (call-sites unchanged from PR #504). Add `SkillEntry::new_unowned(source_path, content_hash, managed)` for Unowned construction. Avoids forcing every owned-entry call-site to wrap source_name in `Some(...)`. Most call-sites know which case they're constructing.
- **D-14 (lockfile mirror):** `LockEntry.source_name` also becomes `Option<DirectoryName>` with the same serde attributes as D-12. Lockfile representation of Unowned skills: `"source_name": null` (or omitted via `skip_serializing_if`). Phase 13's drift detection reads this; Phase 14's status/doctor surfacing reads this.

### Claude's Discretion

The following are implementation details not worth user input; they follow established codebase conventions:

- Exact text of `tome migrate-library` output (within the SAFE-01 visual conventions: ✓ for success, ⚠ for warnings, grouped failure summary, `paths::collapse_home` for ~/-prefixed paths).
- Exact text of `tome sync`'s "library is in v0.9 shape" error (within style of the existing Conflict / Why / Suggestion template per Phase 7 D-10).
- Implementation of `--dry-run` for `tome migrate-library` (project convention: every destructive command has one; render the plan without executing — see `add::add(opts.dry_run)`, `remove::execute(dry_run)` etc.).
- Whether `migration_v010.rs` exposes its inner functions for testing (recommendation: `pub(crate)` — testable from tests within the crate without exposing externally).
- Internal organization of the new `SkillEntry::new_unowned` constructor (e.g., whether it shares a private `inner_new` helper or duplicates body).
- Whether to add a fresh `MigrationFailureKind` enum for SAFE-01 grouping or reuse `RemoveFailure` shape directly (recommendation: fresh enum named for the migration's failure modes; matches POLISH-04 compile-time-enforcement pattern from Phase 10).

### Folded Todos

(None — `gsd-tools.cjs todo match-phase 11` returned no matches.)

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### Design + planning context (this milestone)

- `.planning/research/v0.10-library-canonical-design.md` — v0.10 design exploration with 9 OQs resolved (rationale + alternatives + risk per each).
- `.planning/REQUIREMENTS.md` — LIB-01..05 are the requirements this phase delivers; D-08 above also locks the cross-phase contract for RECON-01..05 (Phase 13).
- `.planning/ROADMAP.md` — Phase 11 section: goal, success criteria, dependencies.
- `.planning/PROJECT.md` — Key Decisions D-LIB-01..05 (milestone-level decisions).

### Codebase modules being changed in Phase 11

- `crates/tome/src/library.rs` — `consolidate_managed`, `consolidate_local`, `classify_destination`. `consolidate_managed` is rewritten from symlink-creation to copy. `consolidate_local` largely unchanged.
- `crates/tome/src/manifest.rs` — `Manifest`, `SkillEntry`, `SkillEntry::new`. Schema change to `Option<DirectoryName>` (D-12) + new `new_unowned` constructor (D-13).
- `crates/tome/src/lockfile.rs` — `LockEntry`. Same schema change as manifest (D-14).
- `crates/tome/src/cleanup.rs` — `cleanup_library`. New branch logic for Case 1 (transition to Unowned, preserve library content) vs Case 2 (today's delete behavior). Per D-09 and D-10.
- `crates/tome/src/discover.rs` — `DiscoveredSkill`. `source_name` already `DirectoryName` post-PR #504; no schema change. Unowned manifest entries don't go through discover.
- `crates/tome/src/remove.rs` — `tome remove` execute path. Per D-10, sets `source_name = None` on owned manifest entries when removing directory from config (explicit trigger).
- `crates/tome/src/cli.rs` — `Command` enum. New `MigrateLibrary { dry_run: bool }` variant.
- `crates/tome/src/lib.rs` — `sync()` orchestration. Adds v0.9-shape detection + refuse-with-hint check before consolidate phase (D-02).

### New code shipped in this phase

- `crates/tome/src/migration_v010.rs` — NEW, transitional. Module-level doc comment marks for removal in v0.11+. Contains: detection (D-03), plan, render, execute (with broken-symlink handling per D-04 + failure aggregation per D-05). The file is the canonical home for everything the migration does.

### Patterns to follow (no behavior change to these modules; they're prior art)

- `crates/tome/src/relocate.rs` — `copy_library`. Recursive copy preserving symlinks. Migration's copy operation is similar in shape but **resolves symlinks** (follows them once during conversion to copy the actual content) instead of preserving the symlink shape. Iteration + error handling pattern is reused.
- `crates/tome/src/remove.rs` — `RemovePlan`, `RemoveResult`, `RemoveFailure`, `FailureKind::ALL`. Phase 8 SAFE-01 pattern + Phase 10 POLISH-04 compile-time-enforcement (`const _: () = { assert!(FailureKind::ALL.len() == N); };`). Direct model for migration's failure aggregation.
- `crates/tome/src/manifest.rs::save` / `lockfile.rs::save` / `machine.rs::save` — atomic temp+rename pattern. Migration is mostly filesystem-only (D-06), but if any new on-disk state files are added, they follow this pattern.

### Tests to write/extend (not exhaustive — research-phase or planner can add)

- `crates/tome/src/library.rs::tests::*` — re-write `consolidate_managed` tests for new copy semantics (was: assert symlink → assert real dir + content_hash matches). Verify `classify_destination::Symlink` for a managed entry now triggers refuse, not happy-path.
- `crates/tome/src/manifest.rs::tests::*` — old-shape deserialize round-trip (`source_name: "foo"` → `Some(DirectoryName::new("foo")?)`); Unowned entry round-trip (`source_name: null` ↔ `None`); `SkillEntry::new_unowned` constructor.
- `crates/tome/src/cleanup.rs::tests::*` — Case 1 transition (orphan with source removed from config → manifest entry's `source_name` becomes None, library content preserved, no delete). Case 2 retention of today's behavior (file missing from configured source → cleanup deletes library copy as today).
- `crates/tome/src/remove.rs::tests::*` — `tome remove <dir>` sets `source_name = None` on all manifest entries it owns (D-10 explicit trigger).
- `crates/tome/tests/cli.rs` — new file `tests/cli_migrate.rs` (or section in `tests/cli.rs` per HARD-13 split): synthetic v0.9 library setup → `tome migrate-library` → assert v0.10 shape (no symlinks for managed entries, content_hash match against original source). Cover broken-symlink case (D-04: skip + preserve). `tome sync` refuse-with-hint test on un-migrated library (D-02).

### Adjacent issues (won't fix in Phase 11, but be aware)

- **Issue #459** — milestone epic; v0.10 closes it on ship.
- **PR #504 (merged)** — `source_name: String` → `DirectoryName` already done; Phase 11 just widens to `Option<DirectoryName>`.
- **Issue #500** — backup test signing flake. Phase 15 work; affects CI behavior during Phase 11 development on Martin's machine specifically.

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable assets

- **`crates/tome/src/relocate.rs::copy_library`** — recursive directory copy. Migration uses similar shape but resolves symlinks (follows them) instead of preserving them. Iteration + error pattern reused.
- **`crates/tome/src/manifest.rs::hash_directory`** — deterministic SHA-256 of directory contents. Reused for content_hash post-migration verification (every successful conversion verifies the copy hashes identically to the source).
- **`crates/tome/src/remove.rs::FailureKind`, `RemoveFailure`, `FailureKind::ALL`** (compile-time-enforced via `const _: () = { assert!(...len() == 4); };` per Phase 10 POLISH-04) — direct model for `MigrationFailureKind`.
- **`dialoguer::Confirm`** — prompt vocabulary (used by wizard, doctor, remove). `tome migrate-library` may use it for the convert-N-skills confirmation prompt before execute.
- **`paths::collapse_home`** — existing path-display helper. Used everywhere user-facing paths render.

### Established patterns

- **Plan / render / execute** (used by `add`, `remove`, `reassign`, `relocate`, `eject`) — natural fit for `tome migrate-library`. Plan = identify v0.9-shape entries; render = print summary; execute = convert. Dry-run is free with this pattern.
- **Atomic temp+rename** for any on-disk writes (`manifest.rs::save`, `lockfile.rs::save`, `machine.rs::save`). Migration is mostly filesystem-only (D-06); if any new state files are added, they follow this.
- **Newtype + `#[serde(transparent)]` + custom validating Deserialize** — `DirectoryName` already does this; `Option<DirectoryName>` inherits it for free.
- **`anyhow::Result + .with_context()`** everywhere; non-zero exit on partial failure (SAFE-01 pattern from Phase 8).

### Integration points

- `crates/tome/src/lib.rs::run` — dispatch `Command::MigrateLibrary { dry_run }` to the new module. Per HARD-02 (Phase 15), `lib.rs::run` is being decomposed; new dispatch follows whatever shape Phase 15 settles on. For Phase 11, follow the current pattern (inline in the match arm).
- `crates/tome/src/lib.rs::sync` — add v0.9-shape detection + refuse-with-hint check before the consolidate phase (D-02). One isolated check; deletable with the rest of `migration_v010.rs` in v0.11.
- `crates/tome/src/cli.rs::Command` — add `MigrateLibrary { dry_run: bool }` variant.

### Constraints from existing architecture

- `library.rs::consolidate_managed` today depends on `unix_fs::symlink`. After change, it depends on a recursive copy (resolves source symlink, copies bytes). The `classify_destination` enum branches need updating: `DestinationState::Symlink` for a managed entry today is the happy path (verify pointer); after Phase 11, it means "v0.9-shape, refuse" — handled by the refuse-with-hint check upstream in sync (D-02).
- `cleanup_library` today's "stale = delete" branch becomes "stale = transition to Unowned (Case 1) OR delete (Case 2)" — needs an extra check against `config.directories.contains_key(&manifest_entry.source_name)`.
- `consolidate_local` is largely unchanged (already creates real-dir copies). The only relevant change: when `source_name` becomes Unowned (None), distribution still happens (D-11) — but distribute already iterates the library directly, so no code change needed there.

</code_context>

<specifics>
## Specific Ideas

- **The user's library setup as the canonical reference state for testing.** Martin's `~/dev/coding-agent-files/skills/` has 104 entries (~62 symlinks managed + ~42 real local) — represents a realistic v0.9 → v0.10 migration. Use a similar synthetic setup in integration tests:
  - 5–10 managed symlinks (target a fake plugin cache fixture)
  - 5 local real-dir entries (already v0.10-shape)
  - 1 broken symlink (target deleted) — exercises D-04
  - 1 user-created symlink that's NOT in the manifest (exercise D-03 conservatism)
  After `tome migrate-library`: managed → real dirs (content_hash matches original), local unchanged, broken preserved + warned, user-created symlink untouched.

- **Library-as-dotfiles assumption.** The library is committed to git (Martin's case: `coding-agent-files`). Migration's safety net is the user's existing git practices, not a tool-specific backup. v0.10 doesn't auto-snapshot before migrate; documenting "commit your library before running migrate-library" in the command's help text is sufficient.

- **The 15-skill `personal-skills` orphan case** (came up during discussion). After `tome migrate-library` runs on Martin's machine: managed entries convert to copies (v0.10 shape), local-skill duplicates at `~/dev/coding-agent-files/.claude/skills/` remain untouched, library content is preserved. Martin handles those 15 manually (`rm -rf` the source dir after confirming library has identical content via `manifest::hash_directory` comparison).

- **The v0.9 → v0.10 transition is asymmetric.** Once a machine has migrated, there's no path back to v0.9 shape (project policy: Backward compat: None). Document this in the migrate-library help text + `--help` output.

- **No machine.toml::migration_v010_acknowledged field.** The original design doc proposed this for the auto-on-first-sync flow. The CLI-command shape (D-01) makes it unnecessary — running the command IS the consent + acknowledgement. Skipping the flag also keeps the code clean for v0.11 deletion (one fewer schema field to remove).

</specifics>

<deferred>
## Deferred Ideas

These came up during discussion but belong in other phases or out of v0.10 entirely:

- **`tome adopt <skill> <directory>` and `tome forget <skill>`** — Phase 14 (UNOWN-01, UNOWN-02). Phase 11 makes the `Unowned` state representable + reachable via `tome remove`; Phase 14 adds the lifecycle commands.
- **Surfacing Unowned skills in `tome status` / `tome doctor` (text + JSON)** — Phase 14 (UNOWN-03). Phase 11 provides the data structure (D-12, D-14); Phase 14 wires the user-facing surfacing.
- **Cleanup-message UX rewrite (3-bucket partition for stale skills)** — Phase 16 (UX-01). Phase 11 changes cleanup logic per D-09 + D-10 but keeps today's user-facing message; Phase 16 rewrites the message to partition into "removed-from-config / missing-from-disk / now-excluded".
- **Drift detection on `tome sync` (Match/Drift/Vanished classification, install consent prompt, edit-in-library detection)** — Phase 13 (RECON-01..05). D-08 above PINS the contract Phase 11 ships (content_hash as drift signal); Phase 13 implements the detection + reconciliation flow itself.
- **Local-skill source-side cleanup tooling (e.g. `tome migrate-library --consolidate-local`)** — explicitly DEFERRED past v0.10 (D-07). User chose manual cleanup for the existing duplicates. If a future need arises, file a v0.11+ requirement.
- **Edit-in-library detection** — Phase 13 (RECON-05). Editing a now-real-directory library entry is possible after v0.10; the prompt-on-edit-detected behavior (fork/revert/skip) is RECON territory.
- **Re-derivation of broken-source-target metadata** — D-04 preserves the broken symlink for its target-path metadata. Surfacing that metadata in `tome status` ("source path: <path>; reachable: no") is Phase 14 territory and worth a follow-up note.
- **RECON-01 wording update.** D-08 changes the drift-detection basis from version to content_hash. RECON-01 in REQUIREMENTS.md needs rewording. Flag for the planner / Phase 13 discuss-phase to pick up.

### Reviewed Todos (not folded)

(None — `gsd-tools.cjs todo match-phase 11` returned no matches.)

</deferred>

---

*Phase: 11-library-canonical-core*
*Context gathered: 2026-05-03*
