# Milestones

## v0.10 Library-canonical Model + Cross-Machine Plugin Reconciliation (Shipped: 2026-05-12)

**Phases completed:** 7 phases, 33 plans, 61 tasks

**Key accomplishments:**

- `SkillEntry.source_name` and `LockEntry.source_name` widened to `Option<DirectoryName>` (serde default + skip_serializing_if) so the v0.10 Unowned state is representable end-to-end; old manifests/lockfiles parse unchanged via serde's natural Option handling.
- `consolidate_managed` rewritten from symlink-creation to recursive copy — both managed and local skills now live as real directory copies in the library (LIB-01); the `managed: bool` flag becomes the LIB-02 "update channel" indicator with the v0.9-shape symlink case explicitly skipped (D-02 boundary defense) so the user must opt in to migration via `tome migrate-library`.
- `tome remove` transitions owned manifest entries to `source_name = None` and preserves library content; `cleanup_library` adds the same safety-net transition for users who manually edit `tome.toml` outside `tome remove` — D-10 hybrid triggers for LIB-04.
- `tome migrate-library` is a one-shot CLI command that converts v0.9-shape libraries (managed skills as symlinks) to v0.10-shape (real directory copies) with manifest-anchored detection (D-03), broken-symlink preservation (D-04), SAFE-01 failure aggregation (D-05), and idempotent re-runs (D-06); `tome sync` refuses to run on a v0.9-shape library and points the user at the new command (D-02).
- Five end-to-end CLI tests anchor the v0.10 library-canonical-core success criteria at the binary surface — `tome migrate-library` (happy path + dry-run + boundary defenses for D-03/D-04/D-05), `tome sync` v0.9-shape refuse-with-hint (D-02), source-removal Unowned preservation (LIB-04 / D-09 Case 1 / D-10 trigger 2), and post-migration idempotent sync — all reusing the production `tome::hash_directory` via a new crate-root re-export so synthetic-fixture hashes are byte-for-byte identical to production hashes.
- MarketplaceAdapter trait + InstalledPlugin data type + MockMarketplaceAdapter test double — object-safe contract that Plans 12-02..12-04 will implement and Phase 13's sync flow will dispatch through.
- InstallFailure / InstallOp / InstallFailureKind types with POLISH-04 compile-time `ALL` exhaustiveness, plus a pure-formatter + eprint!-wrapper renderer pair that mirrors the SAFE-01 grouped failure summary from Phase 8 — Phase 13 collects `Vec<InstallFailure>` and calls `render_install_failures` for zero rendering work.
- GitAdapter implements MarketplaceAdapter as a thin shim over crate::git — every trait method delegates verbatim to the existing v0.6 helpers, anchored by 9 unit tests and the D-05a byte-for-byte regression contract on tests/cli.rs (141 tests passing, same as baseline).
- Production `ClaudeMarketplaceAdapter` ships with a pure JSON parser, pure heuristic stderr classifier, RefCell-backed snapshot cache, subprocess invocations using stdin = /dev/null per D-01, the D-02 zero-extra-subprocess-call vanished signal via the cached `errors[]` field, and 21 new unit + smoke tests anchoring every trait method.
- AutoInstall 3-state enum + auto_install_plugins field on MachinePrefs + --no-install CLI flag plumbed through SyncOptions, backward-compatible with existing machine.toml files
- `tome::marketplace::testing::MockMarketplaceAdapter` lifted from `#[cfg(test)] pub(super)` into a feature-gated `pub mod testing`, reachable from external test crates when `--features test-support` is on; production builds stay mock-free.
- `pub fn reconcile_lockfile` + ReconcileClass + ReconcileReport + 7 internal helpers + 25 unit tests live in `crates/tome/src/reconcile.rs`. Owns Phase 13's classification + drift apply + consent prompts + edit-detection. Plan 13-04 wires the consumer.
- `tome sync` now drives reconcile::reconcile_lockfile through ClaudeMarketplaceAdapter; legacy install.rs deleted (312 LOC); D-13 fork-in-place flip applied at the manifest call site; sync exits non-zero on partial install failures.
- Two Rule 1 fixes (plan spec bugs), both resolved automatically:
- Adds `previous_source: Option<DirectoryName>` to SkillEntry + LockEntry, captured at all three Owned→Unowned transition sites (cleanup orphan, `tome remove dir`, fork-in-place), closing the Phase 13 D-13 lossy-fork-in-place gap.
- Shared SkillSummary projection type wired into lib.rs, ready for 14-06 (status) and 14-07 (doctor) to consume in Wave 3 without struct-shape coordination.
- Replaced `tome remove <name>` with nested subcommand `tome remove dir|skill <name>` per D-API-2 and added `--force` to `tome reassign` per D-A1 — public CLI surface for plans 14-04 and 14-05 is now stable.
- `tome reassign` now accepts Unowned skills and refuses target-only roles + different-content collisions (UNOWN-01 delivered via merged-verb API per D-API-1)
- `tome remove skill <name>` cleans manifest + library + distribution symlinks + lockfile + machine.toml memberships in one atomic-save flow, refusing Owned skills with an actionable hint and aggregating partial failures via the new RemoveSkillFailureKind enum.
- `tome status` now surfaces the Unowned set: tabled section (NAME | LAST-KNOWN SOURCE | SYNCED) between Directories and Health when N > 0; JSON always exposes `unowned: [SkillSummary]` for stable shape; D-C2 fallback when previous_source is missing.
- `tome doctor` surfaces the Unowned set as an informational tabled section parallel to issue checks, with stable JSON shape, while preserving the D-D3 contract that Unowned never affects exit code or `total_issues`.
- REQUIREMENTS.md / ROADMAP.md / PROJECT.md / CHANGELOG.md updated to reflect the D-API-1/-2 merge (`tome adopt` → `tome reassign`, `tome forget` → `tome remove skill`), v0.10 [Unreleased] entry calls out the BREAKING `tome remove <name>` → `tome remove dir <name>` restructure, and 10 phase14_-prefixed integration tests in tests/cli.rs anchor UNOWN-01..03 success criteria to the real `tome` binary via assert_cmd.
- 1. [Rule 3 - Blocking] `clippy::too_many_arguments` on `cmd_add`
- Split 3,122-LOC config.rs into four-file `config/` module with Config::save_checked locked to mod.rs (S3); added paths::unexpand_tilde so save_checked auto-rewrites under-$HOME paths to ~/-shape and a checked-in tome.toml stays portable across machines.
- 1. [Rule 3 - Blocking] clippy::derivable_impls on LogLevel default impl
- LintFailed/MigrationPartialOrFailed downcast through anyhow replaces in-library process::exit(1); all four save() impls now atomic with regression coverage; distribute warns-and-skips foreign symlinks (D-DIST-1) and doctor surfaces them as typed ForeignSymlink Warning (D-DIST-2); [directory_overrides] hostile inputs (`..`, NUL, loops, duplicates) rejected with machine.toml-named errors; tome remove dir end-to-end coverage for git + claude-plugins.
- 1. [Rule 1 - Bug] `SkillName::new(s)` failed to compile from `&&str`
- Cleared the older-bug backlog (#416, #430, #433) and the v0.9-review polish items (#500-#502) in a single sweep: backup gpg-signing flake fix, wizard chrome routed to stderr, relocate function rename, cross-fs recovery hint, reassign read-once snapshot, manifest epoch-0 warning. 11 new tests, 0 regressions.
- `tome sync` cleanup output rewritten as three named buckets — removed-from-config + missing-from-disk + now-in-exclude-list — each with per-skill inline actionable hints; library content preservation invariants (LIB-04) intact; all 13 baseline cleanup tests still pass; new integration test pins all three buckets render against a real binary fixture.
- docs/src/architecture.md rewritten end-to-end for the v0.10 library-canonical model: managed-as-real-dir-copy mechanic, lockfile-authoritative reconciliation with Match/Drift/Vanished classification, MarketplaceAdapter trait shape, and Unowned skill lifecycle with the D-API-1/-2 vocab merge fully honoured.
- `[Unreleased]` rewritten as the v0.10 release notes draft — 22 lines → 209 lines — with the migration walkthrough leading and three breaking changes called out explicitly. All forbidden phrases absent (`tome adopt` / `tome forget` only appear in supersession sentences, "no longer configured" gone, "auto-on-first-sync" gone). All 22 HARD-cluster issue links + 5 older-bug links + #459 epic link present.
- Created docs/src/cross-machine-sync.md (259 lines) documenting the library-as-dotfiles workflow end-to-end with two walkthroughs (Machine A source-of-truth, Machine B fresh machine) plus five reference sections (tome.lock, auto_install_plugins consent, directory_overrides, missing-claude error, v0.9 library migration). Page is reachable via mdbook TOC AND `tome sync --help` long-about, with an in-prose cross-link from architecture.md's Library-canonical model section.

---

## v0.9 Cross-Machine Config Portability & Polish (Shipped: 2026-04-29)

**Phases completed:** 2 phases (9 + 10), 6 plans, ~26 tasks

**Key accomplishments:**

- **Cross-machine config portability** (#458) — `[directory_overrides.<name>]` schema in `machine.toml` lets a single `tome.toml` checked into dotfiles work across machines with different filesystem layouts. Override application happens at config load time (after tilde expansion, before `Config::validate`) so every downstream command (`sync`, `status`, `doctor`, `lockfile::generate`) operates on the merged result.
- **Override surfacing** (#458) — Typo'd override target names produce a stderr `warning:` line without aborting load; override-induced validation failures surface a distinct error class naming `machine.toml` (not `tome.toml`); `tome status` and `tome doctor` mark overridden directories with `(override)` in text output and `override_applied: bool` in JSON.
- **Bare-slug `tome add` expansion** (PR #471, included in v0.9.0) — `tome add planetscale/database-skills` now expands to `https://github.com/planetscale/database-skills` so users can paste org/repo tokens directly without ceremony.
- **TUI polish** (#463 D1-D3) — `tome browse open` paints "Opening: <path>..." before blocking on `xdg-open`/`open` via closure-callback redraw threading; `StatusMessage` redesigned as `Success | Warning | Pending` enum with `body()`/`glyph()`/`severity()` accessors and `pub(super)` visibility; `ClipboardOccupied` auto-retries once with 100ms backoff before surfacing a warning.
- **Type-design polish** (#463 D4-D6) — `FailureKind::ALL` compile-enforced via exhaustive-match sentinel + `const _: () = { assert!(...len() == 4); };`; `RemoveFailure::new` gains `debug_assert!(path.is_absolute(), ...)` invariant; `arboard` pinned to `>=3.6, <3.7` with bump-review-on-bump comment in `Cargo.toml`; dead `SkillMoveEntry.source_path` field removed from `relocate.rs`.
- **Test coverage closing the v0.8 review tail** (#462 P1-P5) — `status_message_from_open_result` helper extracted from ViewSource match with all three arms (Ok+success, Ok+non-zero exit, Err) unit-tested via synthetic `ExitStatus`; `regen_warnings` deferred until after the success banner with source-byte regression test anchored to `Command::Remove` region; partial-failure success-banner-absence assertion + retry-after-fix end-to-end test pinning the I2/I3 retention contract.
- **Test footprint:** 526 unit + 136 integration = 662 total tests at v0.9.0 (was 514 + 130 = 644 at v0.8.1; +18 new tests from v0.9 phases plus +4 `tome add` slug tests bundled in).

---

## v0.8 Wizard UX & Safety Hardening (Shipped: 2026-04-27)

**Phases completed:** 2 phases, 7 plans, 26 tasks

**Key accomplishments:**

- `tome init` now prints `resolved tome_home: <path> (from <source>)` before any Step 1 wizard prompts so users can Ctrl-C before destructive writes — foundation for WUX-01 greenfield gating.
- `tome init` on a greenfield machine now prompts for `tome_home` location (default `~/.tome`, custom with path validation) and offers to persist a custom choice to `~/.config/tome/config.toml` — closing the silent-default footgun and fixing the latent `default_config_path()` save-path bug at wizard.rs:310.
- `tome init` on a brownfield machine (existing `tome.toml` at the resolved `tome_home`) now shows a summary and offers 4 choices (use existing / edit / reinitialize-with-backup / cancel) — the dotfiles-sync workflow that triggered the v0.8 milestone is safe: `--no-input` defaults to "use existing" and never overwrites a valid config. `Option<&Config>` prefill threads through every wizard helper so "edit" preserves custom directories that aren't in `KNOWN_DIRECTORIES` (Pitfall 2 fix).
- `tome remove` now aggregates partial-cleanup failures into a typed `Vec<RemoveFailure>`, prints a grouped `⚠ K operations failed` summary to stderr, and exits non-zero — closing #413 where the command silently reported success while filesystem artifacts leaked.
- `tome browse` `open` (ViewSource) and `copy path` (CopyPath) actions now work on Linux via `xdg-open` + `arboard` (replacing the macOS-only `open` + `sh -c … | pbcopy` invocation which was also a command-injection vector), and both success (`✓`) and failure (`⚠`) outcomes appear in the TUI status bar in place of the keybind line until the next keypress — closing #414.
- Replaced silent `std::fs::read_link(..).ok()` drop at `relocate.rs:93` with an explicit match that emits a stderr warning on `Err` in the canonical PR #448 format, plus a regression test engineering the failure via `chmod 0o000`.

---

## v0.7 Wizard Hardening (Shipped: 2026-04-22)

**Phases completed:** 3 phases, 9 plans, 8 tasks

**Key accomplishments:**

- All four Config::validate() bail! bodies rewritten to the D-10 Conflict+Why+Suggestion template with DirectoryRole::description() used for every role mention
- Config::validate() now rejects every path relation where library_dir overlaps a distribution directory — equality, nesting either direction — using lexical, tilde-aware, trailing-separator-normalized comparison
- Config::save_checked enforces expand → validate → TOML round-trip → write; wizard save + dry-run now share the same pipeline so invalid configs never reach disk
- Two `assert_cmd` integration tests drive `tome init --dry-run --no-input` end-to-end against empty and seeded TempDir HOMEs, proving the generated Config validates and round-trips through TOML byte-equal.
- In-scope correctness fix to `Config::validate()`.
- Migrated `wizard::show_directory_summary` from manual `println!` column formatting to `tabled::Table` with `Style::rounded()` borders, `PriorityMax::right()` truncation, and an 80-column non-TTY fallback.
- Closed the v0.7 doc half of WHARD-08: PROJECT.md now explicitly marks WIZ-01–05 as shipped-in-v0.6 and hardened-in-v0.7 (Phases 4+5), stale "Known Gaps (deferred from v0.6)" subsection removed, footer dated 2026-04-21, CHANGELOG cites WHARD-07 + WHARD-08 under [Unreleased].

---

## v0.6 Unified Directory Model (Shipped: 2026-04-16)

**Phases completed:** 3 phases, 11 plans, 19 tasks

**Key accomplishments:**

- Unified directory type system (DirectoryName/Type/Role/Config) replacing Source/TargetName/TargetConfig with deny_unknown_fields, migration hint, validation, and convenience iterators
- Four pipeline modules (discover, distribute) rewritten for unified directory model with manifest-based circular prevention replacing shares_tool_root()
- Unified directory terminology in manifest, lockfile, machine prefs, status, and doctor -- disabled_directories replaces disabled_targets, DirectoryStatus replaces SourceStatus/TargetStatus
- Self-contained git.rs module with clone/update/SHA-reading plus subdir config field and repos_dir path method
- RED:
- Git directory clone/update wired as pre-discovery sync step with per-directory skill filtering in distribution
- `tome remove` command with full source cleanup: symlinks, library dirs, manifest entries, config save, and lockfile regeneration
- Three new CLI commands (add, reassign, fork) for git repo registration and skill provenance management
- Terminal-adaptive theming, fuzzy match highlighting, scrollbar, markdown preview rendering, and help overlay for the browse TUI

---
