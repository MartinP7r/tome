---
phase: 16-cleanup-message-ux-docs
verified: 2026-05-08T14:32:50Z
status: passed
score: 5/5 truths verified (all Success Criteria pass); traceability-table gap auto-fixed by orchestrator post-verifier
gaps_resolved:
  - truth: "REQUIREMENTS.md traceability table marks all five phase requirements as Validated"
    original_status: failed
    resolution: "Orchestrator fixed three table rows (UX-01, UX-02, DOC-01) from `Pending` → `Validated` post-verifier. Body checkboxes were already `[x]` for all five requirements; this was a clerical drift in the traceability table that earlier plan-execute agents missed. DOC-02 and DOC-03 rows were already `Validated`. Resolution committed alongside phase-completion metadata."
---

# Phase 16: Cleanup-message UX + Docs Verification Report

**Phase Goal:** Rewrite the cleanup message that originally triggered this milestone discussion into three actionable buckets. Document the library-canonical model + cross-machine workflow + behavior change in user-facing docs.

**Verified:** 2026-05-08T14:32:50Z
**Status:** gaps_found
**Re-verification:** No — initial verification

## Goal Achievement

### Observable Truths (Success Criteria from ROADMAP.md)

| # | Truth (Success Criterion) | Status | Evidence |
| - | ------------------------- | ------ | -------- |
| 1 | `tome sync` cleanup output partitions stale-candidate skills into three named buckets (removed-from-config / missing-from-disk / now-in-exclude-list) with per-bucket header + count + per-entry actionable hint; original "no longer configured" wording absent | VERIFIED | `cleanup.rs::render_cleanup_buckets` lines 90-188 emits three named buckets with locked phrases ("no longer in any source", "missing from configured source on disk", "now in exclude list") + per-skill hints (`tome reassign --to`, `tome remove skill`, `machine.toml::disabled` mutations). `lib.rs::sync` line 1749-1754 invokes the renderer to stderr after distribute cleanup. Forbidden phrase `no longer configured` returns ZERO matches in `cleanup.rs` and `lib.rs`. Integration test `cleanup_renders_all_three_buckets_with_distinct_phrasing` (cli_sync.rs:1819) passes |
| 2 | Migration prompt renders summary table before any conversion runs (count, disk usage, affected skills); user confirms or aborts; aborted migrations leave library byte-for-byte unchanged (integration-test verified) | VERIFIED | `migration_v010::render_plan_to` lines 304-399 emits bold summary line `Will convert N symlink(s) → real director{y\|ies} (~X.Y UNIT additional disk).` + tabled SKILL/SOURCE/SIZE/STATUS plan + closing one-way warning. `prompt_confirmation` (line 547) implements three-arm gate (yes/no-input/interactive). `cmd_migrate_library` (lib.rs:989-1034) drives plan → render → confirm → execute. Integration test `migrate_library_no_input_without_yes_bails` (cli_migrate_library.rs:589-644) asserts byte-for-byte symlink preservation on the bail path. All 3 UX-02 behaviour-matrix tests pass |
| 3 | `docs/src/architecture.md` updated for v0.10: managed-as-copy, lockfile-authoritative reconciliation, marketplace adapter trait, unowned lifecycle; old "library is a consolidated cache" framing removed | VERIFIED | architecture.md grew 60 → 254 lines. Four new H2 sections present: `## Library-canonical model` (line 66), `## Lockfile-authoritative reconciliation` (line 114), `## Marketplace adapter trait` (line 163), `## Unowned lifecycle` (line 203). Sync Pipeline now lists 6 steps starting with Reconcile. "consolidated cache" framing absent from architecture.md (only appears once in CHANGELOG.md line 209 as a META reference describing what was removed — accurate prose). `tome adopt` / `tome forget` appear ONLY in supersession sentences (lines 45, 48, 232) |
| 4 | `CHANGELOG.md` v0.10 release notes call out plugin-update-no-longer-auto-propagates AND first-sync conversion behavior changes; migration step documented at top of v0.10 section | VERIFIED | `[Unreleased]` block (lines 8-215) leads with milestone blurb (lines 10-15), `### Migration from v0.9.x` walkthrough (lines 17-45), then `### BREAKING Changes` (lines 47-68) listing the three explicit behavior changes: (a) library shape conversion required; (b) plugin updates no longer auto-propagate via symlink — `tome sync` required; (c) `tome remove <name>` → `tome remove dir <name>`. Migration walkthrough cites the locked summary-line wording verbatim. `tome adopt` / `tome forget` only appear in `Replaces the proposed ...` supersession sentences (lines 115, 121, 193). Forbidden `no longer configured` and `auto-on-first-sync` both return ZERO matches |
| 5 | New page `docs/src/cross-machine-sync.md` exists; documents library-as-dotfiles workflow, tome.lock semantics, auto_install_plugins consent, Machine B bootstrap; linked from SUMMARY.md and `tome sync --help` | VERIFIED | cross-machine-sync.md is 259 lines: 2 walkthroughs (Machine A line 18, Machine B line 62) + 5 reference sections (tome.lock semantics 106, auto_install_plugins consent 134 with corrected `Always \| Ask \| Never` variants, directory_overrides 161, missing-claude error 193 reproducing marketplace.rs verbatim, v0.9 migration 223). SUMMARY.md line 7 wires it into TOC between Configuration and Development Workflow. `cli.rs` Command::Sync `long_about` (lines 159-165) references the page. `cargo run -- sync --help` confirms the line renders in long-form help |

**Score:** 5/5 success criteria verified.

### Required Artifacts

| Artifact | Expected | Status | Details |
| -------- | -------- | ------ | ------- |
| `crates/tome/src/cleanup.rs` | three-bucket renderer, ExcludedSkill type, bucket Vec fields on CleanupResult | VERIFIED | `ExcludedSkill` (line 48), `bucket_a_removed_from_config` (line 68), `bucket_b_missing_from_disk` (line 72), `render_cleanup_buckets` (line 90). All three bucket headers + per-skill hints present. Forbidden phrase absent. |
| `crates/tome/src/lib.rs::sync` | wires bucket A/B from cleanup_library + bucket C from cleanup_disabled_from_target into render_cleanup_buckets to stderr | VERIFIED | excluded_skills accumulator (line 1724), per-distribution-dir cleanup_disabled_from_target call (line 1731), render_cleanup_buckets stderr invocation (lines 1749-1754). cleanup_disabled_from_target signature now returns (usize, Vec<ExcludedSkill>) with per-directory disable detection via is_skill_allowed |
| `crates/tome/src/migration_v010.rs` | byte_size walk, render_plan_to renderer, prompt_confirmation 3-arm gate, humanize_bytes | VERIFIED | `byte_size: Option<u64>` field (line 162), `walk_byte_size` (line 44), `humanize_bytes` (line 63), `render_plan_to` (line 304) emits locked summary line + tabled SKILL/SOURCE/SIZE/STATUS, `prompt_confirmation` (line 547) implements yes-bypass / no-input-bails / interactive-default-false matrix. Bail message contains `destructive`, `--yes`, `--no-input` per Phase 7 D-10 shape |
| `crates/tome/src/cli.rs` | Command::MigrateLibrary { yes }, Command::Sync long_about → cross-machine-sync.md | VERIFIED | `MigrateLibrary { dry_run, yes }` (lines 235-244) with `#[arg(long, short = 'y')]`, `Command::Sync` `long_about` attribute (lines 160-165) references docs/src/cross-machine-sync.md. 4 clap-parse unit tests (lines 545-585) pin --yes / -y / default-false / --dry-run --yes |
| `docs/src/architecture.md` | v0.10 framing — managed-as-copy, lockfile-authoritative reconciliation, marketplace adapter trait, unowned lifecycle | VERIFIED | 254 lines, four H2 sections present, six-step Sync Pipeline, alphabetised Modules list with new entries for marketplace.rs/reconcile.rs/migration_v010.rs/summary.rs, cross-link to cross-machine-sync.md from Library-canonical model bullet (line 87) |
| `CHANGELOG.md` | v0.10 release notes with three BREAKING call-outs + migration walkthrough leading | VERIFIED | `[Unreleased]` 209 lines (lines 8-215). Migration walkthrough leads. Three BREAKING entries explicitly enumerated. 22 HARD-cluster issue links + 5 older-bug links + #459 epic link present. House style preserved |
| `docs/src/cross-machine-sync.md` | Machine A/B walkthroughs + tome.lock + consent + directory_overrides + migration | VERIFIED | 259 lines, 7 H2 sections, AutoInstall variants correctly named `Always \| Ask \| Never`, missing-claude error reproduced verbatim from marketplace.rs:611-615 |
| `docs/src/SUMMARY.md` | TOC entry for cross-machine-sync.md | VERIFIED | Line 7: `- [Cross-machine sync](cross-machine-sync.md)` between Configuration and Development Workflow |

### Key Link Verification

| From | To | Via | Status | Details |
| ---- | -- | --- | ------ | ------- |
| `lib.rs::sync` | `cleanup::render_cleanup_buckets` | direct call to stderr | WIRED | lib.rs:1749-1754 calls renderer with all 3 buckets, gated on `!quiet`. Output goes to `std::io::stderr().lock()` |
| `cleanup::cleanup_library` | `CleanupResult.bucket_a/bucket_b` | populates fields during stale-candidate partition | WIRED | cleanup.rs:290 (bucket_a) and 326 (bucket_b) populated inside cleanup_library; sync drains them post-call |
| `lib.rs::cleanup_disabled_from_target` | `Vec<ExcludedSkill>` | returns excluded list per directory; both global + per-dir disables | WIRED | lib.rs:1859 signature returns `Result<(usize, Vec<ExcludedSkill>)>`; lib.rs:1731-1739 collects across all distribution dirs |
| `cli.rs::Command::MigrateLibrary` | `lib.rs::cmd_migrate_library` | dispatcher destructures { dry_run, yes } | WIRED | lib.rs:418 calls `cmd_migrate_library(&paths, dry_run \|\| cli.dry_run, yes, cli.no_input)` |
| `cmd_migrate_library` | `migration_v010::{plan, render_plan, prompt_confirmation, execute, render_result}` | composed pipeline | WIRED | lib.rs:1004-1023 drives all five primitives in order |
| `cli.rs::Command::Sync` long_about | `docs/src/cross-machine-sync.md` | clap `long_about` attribute string | WIRED | cli.rs:160-165 contains the link; verified rendered via `cargo run -- sync --help` |
| `docs/src/architecture.md` Library-canonical model | `cross-machine-sync.md` | in-prose link | WIRED | architecture.md:87 links `[Cross-machine sync](cross-machine-sync.md)` |
| `docs/src/SUMMARY.md` TOC | `cross-machine-sync.md` | mdbook TOC entry | WIRED | SUMMARY.md:7 |

### Data-Flow Trace (Level 4)

| Artifact | Data Variable | Source | Produces Real Data | Status |
| -------- | ------------- | ------ | ------------------ | ------ |
| `render_cleanup_buckets` output | bucket_a / bucket_b / bucket_c | populated by `cleanup_library` (manifest stale-candidate partition) and `cleanup_disabled_from_target` (machine_prefs `is_skill_allowed`) | Yes — real manifest entries + real machine_prefs lookups, not hardcoded | FLOWING |
| `render_plan_to` output | plan.entries[i].byte_size | `walk_byte_size(library_path)` via walkdir+follow_links(false), populated for source_reachable entries in `migration_v010::plan` | Yes — actual filesystem walk, not static | FLOWING |
| `cmd_migrate_library` plan | manifest::load(paths.config_dir()) | real manifest.json read | Yes | FLOWING |
| `tome sync --help` long_about | static string from cli.rs | clap macro literal | Yes (real string rendered in help output) | FLOWING |

### Behavioral Spot-Checks

| Behavior | Command | Result | Status |
| -------- | ------- | ------ | ------ |
| UX-01 three-bucket end-to-end render | `cargo test -p tome --test cli_sync cleanup_renders_all_three_buckets_with_distinct_phrasing` | 1 passed | PASS |
| UX-02 dry-run does not prompt | `cargo test -p tome --test cli_migrate_library migrate_library_dry_run_does_not_prompt` | passed | PASS |
| UX-02 --no-input without --yes bails (byte-for-byte unchanged) | `cargo test -p tome --test cli_migrate_library migrate_library_no_input_without_yes_bails` | passed | PASS |
| UX-02 --yes skips prompt | `cargo test -p tome --test cli_migrate_library migrate_library_yes_skips_prompt` | passed | PASS |
| DOC-03 `tome sync --help` references cross-machine-sync.md | `cargo run -p tome -- sync --help` | stdout contains "see docs/src/cross-machine-sync.md" | PASS |
| UX-02 `tome migrate-library --help` advertises -y/--yes | `cargo run -p tome -- migrate-library --help` | stdout contains `-y, --yes` | PASS |
| Forbidden phrase absent from runtime code | `rg -n "no longer configured" crates/tome/src/cleanup.rs crates/tome/src/lib.rs` | 0 matches | PASS |
| Forbidden phrase absent from new docs | `rg -n "no longer configured" docs/src/architecture.md docs/src/cross-machine-sync.md` | 0 matches | PASS |

### Requirements Coverage

| Requirement | Source Plan | Description | Body Checkbox | Traceability Status | Evidence | Status |
| ----------- | ----------- | ----------- | ------------- | ------------------- | -------- | ------ |
| UX-01 | 16-01-cleanup-three-bucket-PLAN.md | Three-bucket cleanup partition with actionable hints | [x] (line 57) | **Pending** (line 170) | cleanup.rs three-bucket renderer + lib.rs::sync wiring + cli_sync integration test | SATISFIED in code; **traceability table out of date** |
| UX-02 | 16-02-migrate-confirm-summary-PLAN.md | Migration confirm gate + summary table | [x] (line 58) | **Pending** (line 171) | migration_v010 byte_size + render_plan_to + prompt_confirmation + cli_migrate_library tests | SATISFIED in code; **traceability table out of date** |
| DOC-01 | 16-03-architecture-doc-PLAN.md | architecture.md v0.10 rewrite | [x] (line 89) | **Pending** (line 172) | 4 new H2 sections + 6-step pipeline + alphabetised modules | SATISFIED in docs; **traceability table out of date** |
| DOC-02 | 16-04-changelog-PLAN.md | CHANGELOG v0.10 release notes | [x] (line 90) | Validated (line 173) | [Unreleased] 209 lines, migration walkthrough leads, 3 BREAKING entries, 28 issue links | SATISFIED |
| DOC-03 | 16-05-cross-machine-doc-PLAN.md | cross-machine-sync.md + SUMMARY + sync --help | [x] (line 91) | Validated (line 174) | 259 lines, 7 H2 sections, TOC wired, sync --help references the page | SATISFIED |

**Orphaned requirements:** None. All five requirements declared in plan frontmatter are mapped in REQUIREMENTS.md.

### Anti-Patterns Found

None. Inspected key files for stubs / TODOs / placeholders / hardcoded empty data:

- `cleanup.rs::render_cleanup_buckets` is a real renderer with locked phrases and per-bucket logic. Empty-bucket short-circuit is intentional (silent on no-op syncs).
- `migration_v010::prompt_confirmation` has three real arms (yes, no-input bail, interactive Confirm).
- `cmd_migrate_library` composes 5 real primitives, not a stub.
- Documentation pages contain real prose (not "Coming soon" / "Placeholder").

### Forbidden-Phrase Check

| Phrase | Files Scanned | Matches | Expected | Status |
| ------ | ------------- | ------- | -------- | ------ |
| `no longer configured` | `crates/tome/src/cleanup.rs`, `crates/tome/src/lib.rs` | 0 | 0 | PASS |
| `no longer configured` | `docs/src/architecture.md`, `docs/src/cross-machine-sync.md` | 0 | 0 | PASS |
| `no longer configured` (CHANGELOG `[Unreleased]` block) | `CHANGELOG.md` lines 8-215 | 0 (only appears in v0.7 historical entry, line 451) | 0 in v0.10 block | PASS |
| `tome adopt` (live command form) | `CHANGELOG.md`, `docs/src/architecture.md`, `docs/src/cross-machine-sync.md` | All 3 occurrences inside `Replaces the proposed` / `originally-proposed ... was folded into` / `tome adopt"` (stub-error reference) supersession sentences | Only supersession refs | PASS |
| `tome forget` (live command form) | same files | 1 occurrence inside `Replaces the proposed `tome forget` command` (CHANGELOG line 121) | Only supersession refs | PASS |
| `consolidated cache` | architecture.md | 0 | 0 | PASS (CHANGELOG line 209 is a META description of what was removed — accurate) |
| `auto-on-first-sync` | CHANGELOG.md `[Unreleased]` | 0 | 0 | PASS |
| `first-sync v0.10` | docs/src/cross-machine-sync.md | 0 | 0 | PASS |

### Human Verification Required

None. All success criteria are verifiable programmatically and the integration tests cover the runtime behaviour (3 UX-02 tests + 1 UX-01 end-to-end test all pass). Visual UX review of the rendered help output and TUI behaviour can happen during Phase 17 UAT but is not a Phase 16 gate.

## Gaps Summary

**One gap blocks Phase 16 from being marked fully `passed`:**

The Phase 16 work itself is complete and all five Success Criteria pass programmatic verification. Code, docs, integration tests, and CLI surfaces all line up with the plan. However, the **REQUIREMENTS.md traceability table** is out of date — UX-01, UX-02, and DOC-01 are still marked `Pending` even though the body checkboxes are `[x]` and the implementations have shipped. DOC-02 and DOC-03 are correctly marked `Validated`.

This appears to be an oversight in the plan-completion commits — Plans 16-01 / 16-02 / 16-03 marked their body requirement boxes `[x]` but did not update the traceability table rows. Plans 16-04 and 16-05 updated both. The fix is a 3-line edit: change `Pending` → `Validated` on lines 170, 171, 172 of `.planning/REQUIREMENTS.md`.

Once that edit lands, all five phase requirements will be both `[x]` in the body AND `Validated` in the traceability table, satisfying the verification objective's hard requirement.

---

_Verified: 2026-05-08T14:32:50Z_
_Verifier: Claude (gsd-verifier)_
