---
phase: 16-cleanup-message-ux-docs
plan: 04
type: execute
wave: 2
depends_on:
  - 16-01
  - 16-02
files_modified:
  - CHANGELOG.md
autonomous: true
requirements:
  - DOC-02

must_haves:
  truths:
    - "`CHANGELOG.md` `[Unreleased]` section is rebuilt as the v0.10 release notes for the upcoming release; entries call out the three breaking behavior changes explicitly: (a) library-shape conversion required (one-shot `tome migrate-library`), (b) plugin updates no longer auto-propagate via symlink — `tome sync` required, (c) `tome remove <name>` → `tome remove dir <name>` clap subcommand split (Phase 14)."
    - "The v0.10 entry leads with a clear migration-step paragraph: `tome migrate-library --dry-run` → review → `tome migrate-library` (with confirmation prompt; `--yes` to bypass)."
    - "The migration paragraph states explicitly that the prompt defaults to no (matches Plan 16-02 D-UX02-1) and that `--yes`/`-y` bypasses, mirroring `tome remove skill --yes`."
    - "The cleanup-message UX rewrite (UX-01) is documented with the three bucket names — removed-from-config / missing-from-disk / now-in-exclude-list — and a one-line description of each."
    - "Existing house style preserved: Added / Changed / Fixed / Internal sub-headers; `**BREAKING:**` prefix on breaking entries; closes-issue links where relevant."
  artifacts:
    - path: "CHANGELOG.md"
      provides: "v0.10 release notes with migration step + three breaking-change call-outs + UX-01/UX-02 surfaces"
      contains: "tome migrate-library"
  key_links:
    - from: "CHANGELOG.md v0.10 BREAKING Changes section"
      to: "tome migrate-library command (Plan 16-02)"
      via: "explicit reference to the one-shot command + the confirmation prompt UX"
      pattern: "tome migrate-library"
    - from: "CHANGELOG.md v0.10 Changed section"
      to: "Three-bucket cleanup output (Plan 16-01)"
      via: "user-facing description of bucket names + actionable hints"
      pattern: "removed-from-config|missing-from-disk|exclude list"
---

<objective>
Update `CHANGELOG.md` to capture the v0.10 release notes per DOC-02 + CONTEXT.md `<decisions>` "Claude's Discretion" CHANGELOG bullet. Today's `[Unreleased]` section already documents Phase 14 (Unowned-library lifecycle) but is missing the v0.10 framing — no migration step, no library-canonical model breakage callout, no plugin-update behavior change, no UX-01 cleanup rewrite.

After this plan: `[Unreleased]` becomes the v0.10 release notes draft with a leading migration paragraph and a `**BREAKING Changes**` block explicitly enumerating three behavior changes:
1. Library-shape conversion required (`tome migrate-library` one-shot command with confirmation prompt)
2. Plugin updates no longer auto-propagate via symlink (`tome sync` required to pull marketplace updates)
3. `tome remove <name>` is now `tome remove dir <name>` (Phase 14 D-API-2 subcommand split — already partly documented in `[Unreleased]` from Phase 14)

The migration paragraph at the top of the v0.10 section walks the user through:
```
tome migrate-library --dry-run    # preview
tome migrate-library               # confirm + run (default-no prompt)
# OR for CI / automation:
tome migrate-library --yes
```

Purpose: closes DOC-02. Anyone upgrading from v0.9.x sees the migration step first; anyone reading the changelog sees what semantically changed and why.

Output: rewritten `[Unreleased]` section in `/Users/martin/dev/opensource/tome/CHANGELOG.md`. Length: ~80-120 lines for the full v0.10 block (today's `[Unreleased]` is ~12 lines).
</objective>

<execution_context>
@$HOME/.claude/get-shit-done/workflows/execute-plan.md
@$HOME/.claude/get-shit-done/templates/summary.md
</execution_context>

<context>
@.planning/PROJECT.md
@.planning/ROADMAP.md
@.planning/REQUIREMENTS.md
@.planning/phases/16-cleanup-message-ux-docs/16-CONTEXT.md
@.planning/phases/16-cleanup-message-ux-docs/16-01-SUMMARY.md
@.planning/phases/16-cleanup-message-ux-docs/16-02-SUMMARY.md

@CHANGELOG.md

<interfaces>
Existing house style (from CHANGELOG.md v0.9 / v0.8 / v0.7 entries):

```
## [X.Y.Z] - YYYY-MM-DD

The **vX.Y <Title>** milestone. <One-paragraph blurb>.

### BREAKING Changes
- **BREAKING:** <one-line summary of breaking change>. <Detail paragraph
  explaining why + migration path>. ([#issue](url))

### Added
- <feature line>. ([#issue](url))

### Changed
- <change line>. ([#issue](url))

### Fixed
- <fix line>. ([#issue](url))

### Internal
- <internal line>.
```

Today's `[Unreleased]` section in CHANGELOG.md (lines 8-29 — Phase 14 entries already documented):

```markdown
## [Unreleased]

The **v0.10 Library-canonical Model + Cross-Machine Plugin Reconciliation**
milestone is in progress. Phase 14 (Unowned-library lifecycle) lands the
user-facing flows for skills whose source has been removed from `tome.toml`.
Behaviour follows UNOWN-01..03; CLI vocabulary differs from the original
proposal — see "BREAKING Changes" below for the migration.

### Added
- `tome reassign <skill> --to <dir>` accepts Unowned skills (...)
- `tome remove skill <name>` deletes an Unowned skill: ...
- `tome reassign --force` flag bypasses the new D-A1 different-content collision check ...
- `tome reassign` rejects target-only directory roles (D-A2): ...
- `tome status` and `tome doctor` show an `Unowned skills (N):` section ...
- `SkillEntry.previous_source` and `LockEntry.previous_source` schema fields ...

### Changed
- **BREAKING:** `tome remove <name>` is now `tome remove dir <name>` (D-API-2). ...
- The literal stub error in `reassign.rs` pointing at "Phase 14 / `tome adopt`" is deleted; ...
```

After this plan, the `[Unreleased]` section gets a major expansion: leading migration paragraph + new BREAKING-changes for library-shape + plugin-updates-need-sync + the existing remove-subcommand-split. New Added entries for `tome migrate-library` itself, the three-bucket cleanup output, the marketplace adapter (Phase 12), reconcile sync (Phase 13), and the CLI hardening cluster (Phase 15 — 22 issues).

Phase 11 D-01 supersession of UX-02 wording: migration is `tome migrate-library` (one-shot), NOT auto-on-first-sync.

Reference details for migration paragraph (from Plan 16-02 SUMMARY):
- The confirmation prompt defaults to NO (Phase 14 D-B3 mirror)
- `--yes` / `-y` bypasses the prompt
- `--no-input` without `--yes` bails with a Conflict/Why/Suggestion error mentioning `--yes`
- `--dry-run` always skips the prompt

Reference details for cleanup-rewrite paragraph (from Plan 16-01 SUMMARY):
- Bucket A: removed-from-config — manifest entries whose source was removed; library content preserved as Unowned
- Bucket B: missing-from-disk — manifest entries whose source dir is configured but file vanished; library content removed
- Bucket C: now-in-exclude-list — skills added to `machine.toml::disabled` or per-directory disable lists; distribution symlinks removed, library content preserved
- All buckets emit per-skill inline actionable hints to stderr

Phases 11-15 surfaces to capture in the Added / Changed sections (each line should reference the GitHub issue it closes when applicable):
- Phase 11: `tome migrate-library` command (LIB-05); `consolidate_managed` rewrite (LIB-01); `source_name: Option<DirectoryName>` schema (LIB-03); source-removal preserves content (LIB-04)
- Phase 12: `MarketplaceAdapter` trait + `ClaudeMarketplaceAdapter` + `GitAdapter` (ADP-01..04)
- Phase 13: lockfile-authoritative `tome sync` reconcile + `auto_install_plugins` consent prompt + `--no-install` flag (RECON-01..05)
- Phase 14: already in [Unreleased] — KEEP intact; reorganize into the new structure
- Phase 15: 22 HARD requirements — issue closures listed in REQUIREMENTS.md table; bundle into a single "CLI hardening" Changed entry with the issue list, NOT one entry per issue (CHANGELOG would balloon to 22 lines otherwise)
- Phase 16: this phase — UX-01 three-bucket cleanup + UX-02 migration confirmation + DOC-01..03 docs (DOC entries usually go under Internal or Docs, not user-facing Added)
</interfaces>
</context>

<tasks>

<task type="auto">
  <name>Task 1: Rewrite [Unreleased] as the v0.10 release notes draft</name>
  <files>CHANGELOG.md</files>
  <read_first>
    - CHANGELOG.md (entire file — locate `[Unreleased]` section at line 8; observe the v0.9 / v0.8 / v0.7 / v0.6 entry structure to mirror)
    - .planning/REQUIREMENTS.md (for the full v0.10 requirements list and issue numbers — bottom of the file has the traceability table mapping each HARD-NN to its issue number)
    - .planning/ROADMAP.md (Phase 11-16 sections — for the success-criteria language to crib)
    - .planning/phases/16-cleanup-message-ux-docs/16-CONTEXT.md (`<decisions>` "Claude's Discretion" CHANGELOG section — three breaking-change call-outs spelled out + migration-step paragraph requirement)
    - .planning/phases/16-cleanup-message-ux-docs/16-01-SUMMARY.md (final bucket wording from Plan 16-01)
    - .planning/phases/16-cleanup-message-ux-docs/16-02-SUMMARY.md (final migration command UX wording from Plan 16-02)
    - .planning/phases/14-unowned-library-lifecycle/14-CONTEXT.md (D-API-1 / D-API-2 vocab merge — the Phase 14 entries already in [Unreleased] are correct; preserve them)
  </read_first>
  <action>
    **Step 1: Replace the `## [Unreleased]` header content (lines 8-29) with a new v0.10 release notes block.** Keep the header as `## [Unreleased]` for now (the v0.10 ship date will be filled in by Phase 17 REL-05). Use the following structure:

    ```markdown
    ## [Unreleased]

    The **v0.10 Library-canonical Model + Cross-Machine Plugin Reconciliation**
    milestone. Makes tome's library a single source of truth — managed AND
    local skills are stored as real-directory copies — with a lockfile-
    authoritative `tome sync` flow that reconciles installed plugins to the
    lockfile state on every machine via marketplace adapters. Closes the
    library-as-dotfiles workflow gap (epic [#459](https://github.com/MartinP7r/tome/issues/459)).

    ### Migration from v0.9.x

    v0.10 changes the library shape (managed skills are now real-directory
    copies, not symlinks into a marketplace cache). Pre-v0.10 libraries must
    run a one-shot conversion command before `tome sync` will operate on them
    — `tome sync` refuses with a Conflict/Why/Suggestion error pointing at the
    new command on a v0.9-shape library:

    ```bash
    tome migrate-library --dry-run    # preview the conversion plan
    tome migrate-library               # run it (confirmation prompt; default no)
    # for CI / automation:
    tome migrate-library --yes
    ```

    The dry-run and the live run both render a summary table (count of symlinks
    to convert, approximate additional disk usage, per-skill SOURCE / SIZE /
    STATUS columns) before any conversion happens. The live run prompts via
    `dialoguer::Confirm` defaulting to no — pressing anything other than `y`
    aborts cleanly with no filesystem mutation. The conversion is one-way —
    there is no `--undo-migrate`. Commit your library directory to git (or
    back it up some other way) before running.

    Broken managed symlinks (target gone) are SKIPPED and preserved in place
    so you can recover manually; idempotent on re-run.

    ### BREAKING Changes

    - **BREAKING:** Library shape conversion required. v0.9 libraries store
      managed skills (Claude plugins, git-cloned repos) as symlinks into a
      package-manager-owned cache. v0.10 stores them as real-directory copies
      (LIB-01 / LIB-02). Run `tome migrate-library` once to convert; see
      "Migration from v0.9.x" above. ([#459](https://github.com/MartinP7r/tome/issues/459))
    - **BREAKING:** Plugin updates no longer auto-propagate via symlink. Pre-
      v0.10, a `claude plugin update foo` would transparently update tome's
      library copy because tome's library entry was a symlink into the Claude
      cache. Post-v0.10, the library is a real-directory copy; plugin updates
      reach tome's distribution only through `tome sync`, which now reconciles
      installed plugins against `tome.lock` via the new `MarketplaceAdapter`
      trait. Drift, missing-from-marketplace, and edit-in-library cases all
      surface interactively (RECON-01..05).
    - **BREAKING:** `tome remove <name>` is now `tome remove dir <name>` (Phase
      14 D-API-2). Bare `tome remove <name>` no longer parses. The new sibling
      `tome remove skill <name>` deletes an Unowned skill from the library
      (manifest entry, library directory, distribution symlinks, lockfile
      entry, and `machine.toml` memberships all cleaned). Project policy
      `Backward compat: None` makes this acceptable; users running shell
      aliases or scripts must update them.

    ### Added

    - `tome migrate-library` one-shot CLI command for v0.9 → v0.10 library
      conversion. Idempotent on re-run. `--dry-run` previews; `--yes` / `-y`
      skips the confirmation prompt; `--no-input` without `--yes` bails with
      a Conflict/Why/Suggestion error pointing at `--yes`. Detection: a
      library entry qualifies for migration ONLY when it is a symlink AND
      `manifest[name].managed == true` AND the manifest contains the entry —
      tome never touches user-created symlinks. Broken-source symlinks are
      preserved per Phase 11 D-04. (LIB-05)
    - `tome sync` cleanup output partitions stale-candidate skills into three
      named buckets with per-skill actionable hints, all rendered to stderr:
      **removed-from-config** (source dir removed from `tome.toml` — manifest
      transitions to Unowned, library content preserved per LIB-04),
      **missing-from-disk** (source dir still configured but file vanished —
      library copy removed), and **now-in-exclude-list** (skill added to
      `machine.toml::disabled` or a per-directory disable list — distribution
      symlinks removed, library copy preserved). The original "no longer
      configured" wording — the trigger for the v0.10 milestone discussion —
      is gone. (UX-01)
    - `MarketplaceAdapter` trait isolates marketplace-specific install /
      update / availability logic. Two production adapters: `ClaudeMarketplaceAdapter`
      (subprocess to `claude plugin install/update/list --json`, with a
      `RefCell<Option<Vec<InstalledPlugin>>>` cache that auto-invalidates on
      `Ok` install / update calls) and `GitAdapter` (thin shim over `git.rs`).
      Adapter `install` / `update` failures aggregate into `Vec<InstallFailure>`
      and surface as a grouped `⚠ N install operations failed` summary
      (mirrors v0.8 SAFE-01 `RemoveFailure` pattern). (ADP-01..04)
    - `tome.lock`-authoritative `tome sync`. Reconciles every managed skill
      against the lockfile and classifies as Match / Drift / Vanished
      (`reconcile.rs::ReconcileClass`). Per-class summary on every sync
      (`✓ N match · ⚠ N drift · ⚠ N vanished`). On Drift, applies
      installs/updates via the marketplace adapter (subject to consent) and
      verifies the resulting `content_hash` against the lockfile. Edit-in-
      library detection prompts fork / revert / skip (default fork
      interactively, default skip with warning under `--no-input`). Drift
      basis is content_hash, not version (Phase 11 D-08). (RECON-01..05)
    - `auto_install_plugins` per-machine consent flow. First sync with non-
      empty drift prompts `Auto-install missing plugins on every sync? [Y/n/never]`;
      choice persists in `machine.toml::auto_install_plugins`. Global flag
      `--no-install` overrides the persisted choice for the current invocation
      (mirrors Cargo's `--frozen` / `--locked`). (RECON-02)
    - **(from Phase 14)** `tome reassign <skill> --to <dir>` accepts Unowned
      skills (re-anchors per UNOWN-01 / D-API-1). Replaces the proposed
      `tome adopt` command — same mechanical work as Owned→Owned reassign,
      single verb regardless of starting state.
    - **(from Phase 14)** `tome remove skill <name>` deletes an Unowned skill:
      manifest entry, library directory, distribution symlinks, lockfile
      entry, and `machine.toml` memberships (`disabled` set + per-directory
      `enabled` / `disabled` lists) all cleaned (UNOWN-02 / D-API-2 / D-B1).
      Replaces the proposed `tome forget` command. Confirmation prompt
      defaults to no; `--yes` / `-y` skips. Owned skills are refused with a
      hint to `tome remove dir` first (D-B2).
    - **(from Phase 14)** `tome reassign --force` flag bypasses the new D-A1
      different-content collision check. Same flag also covers the Fork
      path's existing collision check.
    - **(from Phase 14)** `tome reassign` rejects target-only directory roles
      (D-A2): a target-only dir cannot receive reassigned skills since
      nothing rediscovers them on next sync.
    - **(from Phase 14)** `tome status` and `tome doctor` show an `Unowned
      skills (N):` section with NAME / LAST-KNOWN SOURCE / SYNCED columns;
      JSON output gains `unowned` (`StatusReport`) / `unowned_skills`
      (`DoctorReport`) arrays of `SkillSummary` entries. Per Phase 14 D-D3,
      the unowned set is informational and does not contribute to
      `tome doctor` exit code (UNOWN-03).
    - `SkillEntry.previous_source` and `LockEntry.previous_source` schema
      fields capture the last directory that owned a skill before transition
      to Unowned (Phase 14 D-C1). Closes the Phase 13 D-13 lossy fork-in-
      place gap.

    ### Changed

    - **CLI hardening cluster (22 issues closed):** Refactors — `skill::parse`
      returns `anyhow::Result` ([#485](https://github.com/MartinP7r/tome/issues/485));
      `lib.rs::run` decomposed into per-subcommand `cmd_<name>` helpers
      ([#486](https://github.com/MartinP7r/tome/issues/486)); `config.rs`
      split into `config/{mod,types,overrides,validate}.rs`
      ([#487](https://github.com/MartinP7r/tome/issues/487));
      `process::exit(1)` in lint flow replaced with downcastable `LintFailed`
      error ([#488](https://github.com/MartinP7r/tome/issues/488));
      `scan_for_skills` adopts `ScanMode` enum
      ([#491](https://github.com/MartinP7r/tome/issues/491));
      `Lockfile.{skills,version}` tightened to `pub(crate)`
      ([#492](https://github.com/MartinP7r/tome/issues/492));
      `(verbose, quiet)` flags collapsed into `LogLevel` enum
      ([#493](https://github.com/MartinP7r/tome/issues/493)). Safety —
      atomic-save preservation regression test
      ([#494](https://github.com/MartinP7r/tome/issues/494));
      `distribute` refuses to clobber pre-existing symlinks pointing outside
      the library ([#495](https://github.com/MartinP7r/tome/issues/495));
      hostile-input tests for `[directory_overrides]`
      ([#496](https://github.com/MartinP7r/tome/issues/496));
      `tome remove <git-dir>` end-to-end integration tests
      ([#497](https://github.com/MartinP7r/tome/issues/497)). Coverage —
      `browse/ui.rs` ratatui `TestBackend` + `insta` snapshots
      ([#498](https://github.com/MartinP7r/tome/issues/498));
      `tests/cli.rs` (5580 LOC) split into per-domain `cli_*.rs` files
      ([#499](https://github.com/MartinP7r/tome/issues/499));
      `backup::tests::push_and_pull_roundtrip` flake fix
      ([#500](https://github.com/MartinP7r/tome/issues/500)). Polish —
      `wizard.rs` diagnostic prints to `eprintln!`
      ([#501](https://github.com/MartinP7r/tome/issues/501));
      `relocate.rs::provenance_from_link_result` renamed to
      `warn_if_unreadable_symlink` ([#502](https://github.com/MartinP7r/tome/issues/502));
      `TryFrom<String>` for `SkillName` / `DirectoryName`
      ([#503](https://github.com/MartinP7r/tome/issues/503)). Older bugs —
      `tome relocate` cross-fs cleanup recovery hint
      ([#416](https://github.com/MartinP7r/tome/issues/416));
      `tome reassign` plan/execute reads filesystem state once
      ([#430](https://github.com/MartinP7r/tome/issues/430));
      manifest epoch-0 timestamp warning
      ([#433](https://github.com/MartinP7r/tome/issues/433));
      browse UI Disable/Enable wired
      ([#447](https://github.com/MartinP7r/tome/issues/447));
      `Config::save_checked` preserves tilde-shaped paths instead of
      expanding to absolute ([#457](https://github.com/MartinP7r/tome/issues/457)).
      All 22 HARD requirements landed as a single bundle in Phase 15.
    - `Manifest.managed: bool` semantics shift from "stored as symlink" to
      "update channel" (managed = upstream sync feeds updates into the
      library; local = library is canonical). Field name kept; documentation
      updated. (LIB-02)
    - **BREAKING (already noted above):** `tome remove <name>` is now
      `tome remove dir <name>` (Phase 14 D-API-2). The literal stub error in
      `reassign.rs` pointing at "Phase 14 / `tome adopt`" is deleted; Unowned
      input is now accepted directly.

    ### Internal

    - Source removal preserves library content (LIB-04). Cleanup phase no
      longer auto-deletes orphaned skills; manifest entries transition to
      Unowned (`source_name: None`). The configured-source-removed case is
      surfaced via the new "removed-from-config" cleanup bucket (UX-01).
    - `migration_v010` module (transitional) detects v0.9-shape libraries via
      manifest-anchored heuristic and converts them to v0.10 shape. Slated
      for removal in v0.11+ once all known users have migrated.

    ### Docs

    - `docs/src/architecture.md` rewritten for v0.10: managed-as-copy
      consolidation, lockfile-authoritative reconciliation, marketplace
      adapter trait, Unowned lifecycle. Old "library is a consolidated cache"
      framing removed. (DOC-01)
    - New page `docs/src/cross-machine-sync.md` documents the dotfiles
      workflow end-to-end (committing the library to git, `tome.lock`
      semantics on Machine B, `auto_install_plugins` consent flow, missing-
      `claude` behaviour, migrating a v0.9 library on Machine B). Linked
      from `docs/src/SUMMARY.md` and `tome sync --help`. (DOC-03)
    ```

    **Step 2: Verify house-style preservation.** The new v0.10 block must use the same sub-headers (`### Migration from v0.9.x`, `### BREAKING Changes`, `### Added`, `### Changed`, `### Internal`, `### Docs`) and the same `**BREAKING:**` prefix style as v0.9 / v0.8 entries. Compare visually against the v0.9.0 entry around line 30.

    **Step 3: Cross-check actual landed wording.** Read `.planning/phases/16-cleanup-message-ux-docs/16-01-SUMMARY.md` (when available — it lands at the end of Plan 16-01 execution) and `.planning/phases/16-cleanup-message-ux-docs/16-02-SUMMARY.md` to confirm:
    - The bucket names landed exactly as written in the changelog (planner picked the bucket header phrasing within Claude's Discretion in Plan 16-01; this CHANGELOG entry must match).
    - The migration prompt wording landed as written (Plan 16-02 SUMMARY captures the final summary line + Phase 7 D-10 bail message).

    If either summary's locked wording differs from this plan's draft text, update the changelog entry to match the locked wording.

    **Step 4: Verify forbidden phrases are absent.** The CHANGELOG MUST NOT contain:
    - `tome adopt` as a live command (only acceptable in a "Replaces the proposed `tome adopt`" supersession sentence)
    - `tome forget` as a live command (only acceptable in a "Replaces the proposed `tome forget`" supersession sentence)
    - `auto-on-first-sync` or "first-sync v0.10 migration prompt" (Phase 11 D-01 supersession — migration is the one-shot CLI command)
    - "no longer configured" (UX-01 trigger phrase removed)
  </action>
  <verify>
    <automated>rg -n "tome migrate-library" /Users/martin/dev/opensource/tome/CHANGELOG.md | head -5 &amp;&amp; rg -n "BREAKING" /Users/martin/dev/opensource/tome/CHANGELOG.md | head -10</automated>
  </verify>
  <acceptance_criteria>
    - `rg -n 'tome migrate-library' CHANGELOG.md` outputs at least three matches in the v0.10 / `[Unreleased]` block (migration paragraph, BREAKING entry, Added entry)
    - `rg -n '^### Migration from v0.9' CHANGELOG.md` outputs one match (migration paragraph header present)
    - `rg -n '^### BREAKING Changes' CHANGELOG.md | head -1` shows the BREAKING block exists in the v0.10 section
    - `rg -nC 1 'BREAKING:.*[Ll]ibrary shape' CHANGELOG.md` outputs at least one match (BREAKING #1)
    - `rg -nC 1 'BREAKING:.*[Pp]lugin updates' CHANGELOG.md` outputs at least one match (BREAKING #2)
    - `rg -nC 1 'BREAKING:.*tome remove' CHANGELOG.md` outputs at least one match (BREAKING #3)
    - `rg -n 'removed-from-config' CHANGELOG.md` outputs at least one match (UX-01 bucket name surfaced)
    - `rg -n 'now-in-exclude-list|now in exclude list' CHANGELOG.md` outputs at least one match (UX-01 Bucket C surfaced)
    - `rg -n '^auto.on.first.sync|first.sync v0\.10 migration prompt' CHANGELOG.md` outputs zero matches (Phase 11 D-01 supersession honored)
    - `rg -n 'no longer configured' CHANGELOG.md` outputs zero matches (UX-01 trigger phrase removed)
    - `rg -n '#485|#486|#487|#488|#491|#492|#493|#494|#495|#496|#497|#498|#499|#500|#501|#502|#503' CHANGELOG.md` outputs at least 17 matches (HARD-01..17 cluster issues all linked)
    - `rg -n '#416|#430|#433|#447|#457' CHANGELOG.md` outputs at least 5 matches (HARD-18..22 older bugs linked)
    - `rg -n '#459' CHANGELOG.md` outputs at least one match in the v0.10 block (epic linked)
  </acceptance_criteria>
  <done>
    `[Unreleased]` is rebuilt as the v0.10 release notes draft: leading milestone blurb + Migration from v0.9.x walkthrough + 3-item BREAKING Changes block + Added/Changed/Internal/Docs sub-sections covering Phases 11-16. House style preserved. All forbidden phrases absent. Issue links present for the 22 HARD requirements + epic #459.
  </done>
</task>

</tasks>

<verification>
- `rg -n '^## \[' CHANGELOG.md | head -3` shows `[Unreleased]` as the top entry, followed by `[0.9.0]`
- The `[Unreleased]` block is between 80 and 150 lines (today: ~22 lines; after this plan: greatly expanded)
- All three breaking-change call-outs are present and clearly marked
- Migration step paragraph leads the v0.10 section so upgraders see it first
- Existing Phase 14 entries are preserved (with reorganization into the new structure as needed)
- House style matches v0.9 / v0.8 entries (sub-header structure, BREAKING prefix, issue link style)
</verification>

<success_criteria>
- DOC-02 satisfied: CHANGELOG.md `[Unreleased]` documents the v0.10 release surface comprehensively
- Three breaking changes called out explicitly per CONTEXT.md `<decisions>` Claude's Discretion bullet
- Migration step paragraph leads the v0.10 section with three-command walkthrough
- Phase 11 D-01 vocabulary supersession honored (migration is `tome migrate-library`, NOT auto-on-first-sync)
- Phase 14 D-API-1/-2 vocabulary supersession honored (no `tome adopt` / `tome forget` as live commands)
- UX-01 trigger phrase ("no longer configured") absent
- 22 HARD-cluster issue links + 5 older-bug links + #459 epic link all present
</success_criteria>

<output>
After completion, create `.planning/phases/16-cleanup-message-ux-docs/16-04-SUMMARY.md` documenting:
- Final line count of `[Unreleased]` block
- Whether the migration paragraph wording matches Plan 16-02's locked Phase-7-D-10 bail-message text exactly
- Whether the cleanup bucket names match Plan 16-01's locked header phrases exactly
- Any HARD issues that needed deeper context than a single line could carry (e.g. flake fix #500 may warrant a sentence-long explanation)
- Whether Phase 16 itself (UX-01, UX-02, DOC-01..03) needs a closing paragraph at the bottom of the v0.10 block ("Phase 16 cleanup-message UX + docs" rollup) or whether the Added/Docs entries are sufficient
</output>
