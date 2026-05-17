# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.12.1] - 2026-05-17

### Notes

Version-bump-only patch. **No code delta from v0.12.0.** Cargo.toml +
Cargo.lock workspace-version line are the only changes. Tagged to
resolve a release-flow state where Homebrew had already propagated
v0.12.0 before the local `make release` invocation completed; v0.12.1
gave the publisher a clean tag to ship from.

## [0.12.0] - 2026-05-17

Pre-v1.0 review polish — bundles 15 of 16 findings from a whole-codebase
audit (5 specialist review agents, scope: entire `crates/tome/src/`)
plus a dependabot dependency bump. The one held finding (Owned/Unowned
enum migration) is tracked in issue #542 for v1.0 Phase 10 absorption.

### Breaking

- **`tome status --json` `role` field shape change.**
  `DirectoryStatus.role` is now the typed `DirectoryRole` enum
  (serializes as `"managed"` / `"synced"` / `"source"` / `"target"`)
  instead of a human-readable description string. The previous prose
  description is now in a new `role_description` field. JSON consumers
  reading `role` as a description string need to switch to
  `role_description` or branch on the enum. Per the project's
  `Backward compat: None` policy, no shim is provided. (PR #541,
  Important #10.)

### Fixed

- **Critical — `apply_edit_decisions` manifest reload race.** Refactored
  `apply_edit_decisions` to take `&mut Manifest`; `sync()` now owns the
  manifest variable end-to-end through reconcile. Eliminates the
  pre-refactor double-disk-touch pattern (separate load → mutate → save
  followed by a separate consolidate load) that risked silently losing
  Fork mutations under future `consolidate` refactors. (PR #541,
  Critical #1.)
- **Critical — `Lockfile` missing `Clone` derive.** Derived `Clone`;
  deleted the brittle manual `clone_lockfile` helper in `reconcile.rs`
  that would have silently dropped any new field added to `Lockfile`.
  (PR #541, Critical #2.)
- **Critical — `ClaudeMarketplaceAdapter::list_installed` silent
  cache-miss.** Added `tracing::warn!` for the
  "`populate_cache()? Ok` but cache is `None`" invariant-violation
  case so reconcile-silently-skipping-every-managed-update is at least
  diagnosable in `--verbose` / `TOME_LOG=warn` traces. (PR #541,
  Critical #4.)
- **Critical — Migration render errors silently dropped.** The
  `tome migrate-library` flow's `let _ = render_plan_to(...)` and
  `render_result_to(...)` discarded I/O errors, allowing a user to land
  in the migration confirmation prompt having seen no plan. Now logged
  at `warn!` so a broken stderr is diagnosable. (PR #541, Critical #6.)
- **Non-interactive Case 2 cleanup deletes now warn before action.**
  In non-TTY / `--quiet` / `--no-input` mode, library entries whose
  source file vanished from disk were auto-removed with no log line
  before the deletion. CI scenarios where an NFS mount dropped would
  silently wipe every skill from that source. Added a `tracing::warn!`
  listing every skill name + count before the deletion proceeds.
  (PR #541, Important #9.)
- **Cleanup render errors silently dropped.** Same pattern as Critical
  #6 — `render_cleanup_buckets` and `render_distribution_cleanup_failures`
  I/O errors are the only user notification for stale-skill action,
  and they were discarded via `let _ = ...`. Now routed through
  `tracing::warn!`. (PR #541, Important #12.)
- **Doctor JSON empty-string fallback.** Replaced
  `.ok().unwrap_or_default()` on `IssueCategory` serialization with
  `.expect("IssueCategory serializes to a JSON string")` so a future
  `Serialize` break is surfaced as a programming error instead of
  emitting corrupt `"": <count>` keys to `tome doctor --json`.
  (PR #541, Important #13.)

### Added

- **`apply_edit_decisions` Revert + Skip + dry_run regression tests.**
  Pins the v0.10 deferred `Revert` stub semantic — emits a warning,
  mutates nothing — with byte-level (`serde_json::to_string` equality)
  + field-level assertions. Closes a data-loss-class test gap: a future
  "completion" of the revert path that accidentally added library-
  overwrite logic would now fail this test first. (PR #541,
  Critical #5.)
- **`CleanupResult` public accessors.** Added `removed_from_library()`,
  `transitioned_to_unowned()`, `bucket_a_removed_from_config()`, and
  `bucket_b_missing_from_disk()` matching the `Lockfile::skills()`
  pattern. Lets v1.0 GUI consumers read bucket counts without mutating
  the internal vec state. (PR #541, Important #11.)
- **`MarketplaceAdapter` trait docstring with `# Example`.** Worked
  example showing how to implement a new adapter (npm adapter shape);
  documents cache-invariant rules, sealing-status, and known
  implementations. (PR #541, Important #16.)
- **`SyncReport` + `ReconcileReport` lifecycle docstrings.** Document
  field ownership, the `edited` / `edit_decisions` parallel-vec
  invariant (with forward-reference to the #519 single-Vec follow-up),
  and the OBS-05 summary line origin. (PR #541, Important #16.)
- **Doctor auto-fix integration test.**
  `doctor_dry_run_reports_issues_without_filesystem_mutation` pins
  the `--dry-run` + `--no-input` filesystem-invariants at the CLI
  boundary (previously only unit-level coverage existed). (PR #541,
  Important #14.)

### Changed

- **Stderr discipline: per-skill drift/missing diff lines moved from
  stdout to stderr.** The two `println!` calls in
  `apply_drift_and_missing` were the only stdout writers in the entire
  sync pipeline. Now `eprintln!` matches the cleanup buckets / dry-run
  banner / warnings convention. (PR #541, Important #7.)
- **Stale `#[allow(dead_code)]` suppressions cleanup.** Stripped 13 of
  15 suppressions in `marketplace.rs` (Phase 13 shipped; the "drop
  when Phase 13 wires" comments were stale). The remaining two
  (`classify_claude_install_stderr`, `build_install_failure`) carry
  updated comments referencing #518 follow-up. (PR #541, Important #8.)
- **Comment drift fixes in 5 files** — pre-v0.10 "managed = symlink"
  wording updated to v0.10 library-canonical real-dir-copy semantics
  in `discover.rs`, `distribute.rs`, `status.rs`, `relocate.rs`, and
  the crate-root `lib.rs` pipeline doc. (PR #541, Important #15.)
- **Dependency bumps**: `clap_complete` 4.6.4 → 4.6.5,
  `assert_cmd` patch bump. (PR #543.)

## [0.11.1] - 2026-05-15

### Fixed

- **`make release` recipe + FIX-06 regression tests** — Cross-platform `sed`.
  The Makefile's `release` recipe and the three `cli_make_release.rs`
  regression tests both used `sed -i ''` (BSD-only syntax) which fails on
  GNU sed (Linux) with `can't read s/...: No such file or directory` —
  GNU sed treats the empty `''` as the filename rather than a no-backup
  sentinel. Switched both to `sed -i.bak <expr> <file> && rm -f file.bak`,
  which works on BSD (macOS) and GNU (Linux). `make release` is now
  Linux-portable; the v0.11.0 release was cut from macOS where the bug
  didn't surface. (PR #537.)

### Changed

- **CLI help text — "sources and targets" → "directories".** Three
  command doc-comments (`tome init`, `tome status`, `tome list`) still
  used the pre-v0.6 mental model in their `--help` output. Updated to
  the v0.6 unified directory vocabulary. `tome status` description now
  also mentions the v0.11 last-sync line. (PR #537.)
- **README + docs/src sweep for v0.10/v0.11 drift.** The README
  Configuration example used the pre-v0.6 `[[sources]]` / `[targets.*]`
  schema (which has been failing to parse since v0.6 shipped on
  2026-04-16) — replaced with `[directories.<name>]` + role schema. The
  "managed = symlink" claim was corrected to v0.10's library-canonical
  model. Six missing commands added to the command table.
  `docs/src/architecture.md` gained a v0.11 Observability section
  documenting the `tracing` substrate + `TOME_LOG` envvar + per-step
  spans. `docs/src/test-setup.md` got corrected counts
  (214 unit / 32 integration → ~825 / ~197) and HARD-13 `cli_*.rs`
  split documentation. Closes [#446](https://github.com/MartinP7r/tome/issues/446).
  (PR #537.)
- **Planning artifacts synced post-shipment.** Phase 19 closure metadata
  brought back into alignment with the shipped v0.11.0 surface
  (`ROADMAP.md` milestone status, `STATE.md`, `19-07-SUMMARY.md`).
  (PR #537.)

### Removed

- **Unused `tracing-error` and `tracing-appender` workspace deps.** Both
  were added speculatively during Phase 18 (observability foundation)
  for features that never shipped (file-log rotation and `SpanTrace`
  error-spanning). `cargo machete` flagged them; verified unused via
  source grep. Their transitive deps (`crossbeam-channel`, `symlink`)
  also drop out of `Cargo.lock`. If/when those features are added back,
  the deps can come back. (PRs #537 + #538.)

## [0.11.0] - 2026-05-14

### Added

- **OBS-01 / OBS-02 — Structured logging substrate (`tracing`).** Adopted
  `tracing` + `tracing-subscriber` as the application logging substrate.
  Internal `eprintln!` / `println!` chatter in the sync, reconcile, consolidate,
  distribute, and cleanup paths now routes through `tracing::{info,warn,debug}!`.
  Wizard prompts, TUI browse output, and user-facing summary tables
  (`tome status` / `list` / `doctor` tables, the `tome sync` final summary
  block) remain on direct stdout — output discipline unchanged for byte-identical
  stdout in `tome status` and `tome init --dry-run`. (Phase 18 plans 18-01,
  18-02.)
- **OBS-02 — `TOME_LOG` environment variable.** New `TOME_LOG` env var
  configures the subscriber filter using `tracing_subscriber::EnvFilter`
  directive syntax. Examples:

  ```bash
  TOME_LOG=debug tome sync                                # verbose globally
  TOME_LOG=tome::sync=debug,tome::reconcile=info tome sync # scoped to sync
  TOME_LOG=warn,tome::library=debug tome sync             # warn globally, debug consolidate
  ```

  When `TOME_LOG` is set, it fully replaces the flag-derived level. Malformed
  directives silently fall back to the flag-derived level (matches the
  `RUST_LOG` UX users bring from cargo / tokio).
- **OBS-03 — Per-pipeline-step spans with timing.** `tome sync --verbose`
  (or `TOME_LOG=tome::sync=debug`) now emits one `tracing` span per pipeline
  step (`discover`, `reconcile`, `consolidate`, `distribute`, `cleanup`)
  nested under a top-level `sync` span. Each span records `time.busy` and
  `time.idle` timing fields on close — useful for diagnosing slow phases.
  Note: the literal field name is `time.busy` (auto-emitted by
  `FmtSpan::CLOSE`), NOT `elapsed_ms`. The OBS-03 success-criterion wording
  said "elapsed_ms" conceptually; grep accordingly.
- **OBS-04 — Change-cause attribution.** When `consolidate` or `distribute`
  re-emits a skill, the log line names the cause at `info!` level via a typed
  `ChangeCause` field. Three of four causes wire up in this release:
  `cause=hash changed`, `cause=newly added`, `cause=directory now allowed`.
  A user running `tome sync --verbose` can grep stderr for `cause=` to see
  exactly why each re-emit happened.
- **OBS-05 — Reconcile classification breakdown.** The `tome sync` final
  summary block now includes a per-classification reconcile line:

  ```
    reconcile: ✓ N match · ⚠ M drift · ⚠ K vanished · ⚠ L missing-from-machine
  ```

  immediately above the per-bucket cleanup summary. Counts come from the
  existing `ReconcileReport` populated since v0.10's Phase 13; no new
  computation. Only emits when reconcile actually fires (i.e. when at least
  one Claude adapter directory is configured).
- **OBS-06 — `tome doctor` issue categorization.** Each `DiagnosticIssue`
  carries an `IssueCategory` (`Library` / `Directory` / `Config` /
  `ForeignSymlink`) derived at construction from the `DoctorReport` field +
  `DiagnosticIssueKind` (`ForeignSymlink` promotes regardless of source
  field). Text summary line now includes a per-category auto-fixable
  breakdown (e.g. `(3 auto-fixable: Library 2, Foreign-symlink 1)`). JSON
  shape gains a `category` field per issue plus `summary.by_category` and
  `summary.auto_fixable_by_category` maps. Replaces the message-substring
  classification anti-pattern with typed `RepairKind` discrimination; built
  on the POLISH-04 enum-exhaustiveness sentinel pattern so adding a variant
  without a dispatcher arm fails to compile. (Phase 19 plan 19-01.)
- **OBS-07 — `tome status` richer surface.** Top-line `Last sync: <RFC-3339
  timestamp>` (or `Last sync: never` when no manifest exists). Per-directory
  `SKILLS` column in the Directories table (the existing `(override)`
  annotation from PORT-05 is preserved). JSON shape gains a top-level
  `last_sync: Option<String>` field. Manifest schema additive lift —
  pre-v0.11 manifests deserialize cleanly with `last_synced_at: None`. The
  stamp fires after distribute + cleanup succeed and before `lockfile.save`
  (D-LSYNC-3). (Phase 19 plan 19-03.)

### Changed

- **Logging output now routes to stderr by default** for all migrated diagnostic
  chatter. Stdout remains reserved for user-facing summary tables and version
  output (consistent with Unix convention; matches `tome sync`'s cleanup-bucket
  output discipline from v0.10's Phase 16).
- **`--quiet` and `--verbose` flags map to subscriber levels** through the new
  `LogLevel::directive` accessor: `--quiet` → `warn`, default → `info`,
  `--verbose` → `debug`. Behavior preserved for users who only use the flags;
  no lines silently disappear. Warnings that were previously gated on
  `if !quiet { eprintln!(\"warning: ...\") }` now always fire — the global
  subscriber's `EnvFilter` is the single discipline point. If any of these
  warnings turn out to be noise that should be silent under `--quiet`, a
  future PR can demote individual ones to `debug!`.

### Deferred (tracked in `.planning/phases/18-observability-foundation-sync-diagnostics/18-deferred-items.md`)

- **`ChangeCause::PreviouslyFailed` cause not emitted.** The enum variant +
  `Display` impl ship in this release (so the grep vocabulary `cause=previously
  failed` is reachable if/when an emission site fires), but the emission site
  requires a manifest-schema bump to track per-skill last-sync failure state.
  Deferred to v0.12 or a later polish phase.
- **`ChangeCause::DirectoryNowAllowed` inference false positive on fresh
  skills.** `consolidate` inserts the manifest entry BEFORE `distribute`
  iterates, so on the very first sync after a fresh `tome init` every new
  skill emits `cause=directory now allowed` where the strict-correct cause
  would be `cause=newly added`. Accepted for v0.11 (false-positive rate is
  bounded; user-visible meaning is close enough). Strict fix requires
  per-directory-per-skill "has been distributed before" state — same
  schema-bump trade-off as `PreviouslyFailed`. Deferred to v0.12 or later.

### Fixed

- **FIX-01 / [#530](https://github.com/MartinP7r/tome/issues/530)** —
  `tome doctor` no longer prints "N auto-fixable issues" followed by
  "(no auto-repair available)". The dispatcher uses typed `RepairKind`
  discrimination (`RemoveStaleManifestEntry`, `RemoveBrokenLibrarySymlink`,
  `RemoveStaleTargetSymlink`) instead of message substring matching. When
  `auto_fixable_count == 0`, the global "Apply N auto-fixable repairs?
  [Y/n]" prompt is skipped entirely. Adding a `RepairKind` variant without
  a dispatcher handler now fails to compile (POLISH-04 sentinel + exhaustive
  match). (Phase 19 plan 19-01.)
- **FIX-02 / [#511](https://github.com/MartinP7r/tome/issues/511) +
  HARD-14 carry-over** — Browse copy-path timing flake. Upper bound on
  `browse::app::tests::copy_path_retry_helper_returns_within_bound` relaxed
  from 600ms to 2000ms with a multi-line `FLAKE-FIX` comment naming
  `arboard` clipboard contention under `--test-threads=N` as the root
  cause (and the rejected clock-injection alternative as out-of-scope for
  v0.11 polish). 100/100 consecutive runs pass locally. The paired backup
  test (`backup::tests::push_and_pull_roundtrip`) did not reproduce on M1
  macOS (50/50 isolated + 10/10 module + 5/5 lib-suite runs all clean); a
  defensive `FLAKE-WATCH` comment ships in place describing flake history
  and a retry-wrapper mitigation pattern for future-phase pickup if it
  recurs in CI. (Phase 19 plan 19-04.)
- **FIX-03 / [#532](https://github.com/MartinP7r/tome/issues/532)** —
  `tome doctor` no longer reports `N managed symlink(s) tracked in git`
  warnings on v0.10-shape libraries. The stale check was deleted wholesale
  (v0.10 made managed skills real directory copies, so the original
  concern no longer applies): the `check_library` emit block, the
  `tracked_managed_symlinks` helper, and the interactive git-tracked
  render/confirm path were all removed. A regression test
  (`doctor_clean_v010_library_emits_no_tracked_in_git_warning`) pins the
  clean-library behavior. (Phase 19 plan 19-01.)
- **FIX-04 / [#454](https://github.com/MartinP7r/tome/issues/454)** —
  Wizard summary table column alignment under ANSI-bold styled headers.
  Reproduce-first investigation confirmed the existing
  `tabled = { features = ["ansi"] }` feature (added in commit `0803afb`,
  April 2026) already produces aligned output — divider positions are
  byte-identical at `[0, 17, 34, 59, 79]` even with ANSI escapes around
  header cells. No `strip-ansi-escapes` dep was added. A snapshot test
  (`wizard::tests::show_directory_summary_aligns_header_with_body_under_ansi`)
  ships as a regression guard — it fails immediately if anyone drops the
  `tabled[ansi]` feature. (Phase 19 plan 19-05 — administrative-close
  path 2B.)
- **FIX-05 / [#453](https://github.com/MartinP7r/tome/issues/453) +
  [#456](https://github.com/MartinP7r/tome/issues/456)** — Wizard library
  default follows `tome_home`. The implementation at `wizard.rs:637` was
  already correct (derived `<resolved_tome_home>/skills` from the resolved
  `TOME_HOME`); the bug filed was actually a test-coverage gap. Two new
  pinning integration tests in `cli_init.rs` lock the behavior: positive
  (library default == `<TOME_HOME>/skills`) + negative (no fallback to
  `~/.tome/skills` when `TOME_HOME` is customized). Sensitivity verified —
  replacing `wizard.rs:637` with a hardcoded path makes both tests fail.
  (Phase 19 plan 19-06.)
- **FIX-06 / [#533](https://github.com/MartinP7r/tome/issues/533)** —
  `make release VERSION=X.Y.Z` now automatically replaces `## [Unreleased]`
  with `## [X.Y.Z] - YYYY-MM-DD` in `CHANGELOG.md` during the version-bump
  commit. Inline `sed -i ''` in the Makefile recipe, style-matched with
  the existing Cargo.toml version-bump sed line; portable across BSD
  (macOS) and GNU (Linux) sed. Idempotent — silent no-op if no
  `[Unreleased]` section is present. `CHANGELOG.md` is staged alongside
  `Cargo.toml` + `Cargo.lock` in the version-bump commit. Three regression
  tests in `cli_make_release.rs` pin sed substitution + idempotency +
  silent-noop. (Phase 19 plan 19-02.)

### Trade-offs (release-noted; no migration shim)

- `--quiet` becomes a no-op when `TOME_LOG` is set in the environment. Matches
  the `RUST_LOG` precedence mental model users bring from cargo / tokio. Per
  the project's documented policy (Backward compat: None), this is not gated
  on a shim.
- The OBS-03 timing field is named `time.busy` (auto-emitted by
  `tracing-subscriber`'s `FmtSpan::CLOSE` event), NOT `elapsed_ms`. The OBS-03
  success-criterion wording said "elapsed_ms" conceptually; `time.busy` is the
  literal field name — grep accordingly.
- `tracing-error` and `tracing-appender` enter `Cargo.toml` as scaffolded deps
  with no runtime wiring. They light up in a future phase (Phase 19's OBS-06
  may wire `tracing-error::ErrorLayer` for `tome doctor`; v1.0 Tauri IPC wires
  `tracing-appender` for log-file capture).

## [0.10.0] - 2026-05-11

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

The dry-run and the live run both render a bold summary line — `Will
convert N symlink(s) → real director{y|ies} (~X.Y UNIT additional disk).`
— followed by a `tabled` plan with SKILL / SOURCE / SIZE / STATUS columns
before any conversion happens. The live run prompts via
`dialoguer::Confirm` defaulting to no — pressing anything other than `y`
aborts cleanly with no filesystem mutation. Aborted runs leave the library
byte-for-byte unchanged. The conversion is one-way — there is no
`--undo-migrate`. Commit your library directory to git (or back it up some
other way) before running.

`tome migrate-library --no-input` (without `--yes`) bails with a
Conflict/Why/Suggestion error pointing at `--yes`; `--dry-run` always
skips the prompt. Broken managed symlinks (target gone) are SKIPPED and
preserved in place so you can recover manually; idempotent on re-run.

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
  configured" wording — the trigger phrase for the v0.10 milestone
  discussion — is gone. (UX-01)
- `MarketplaceAdapter` trait isolates marketplace-specific install /
  update / availability logic. Two production adapters:
  `ClaudeMarketplaceAdapter` (subprocess to `claude plugin install / update
  / list --json`, with a `RefCell<Option<Vec<InstalledPlugin>>>` cache that
  auto-invalidates on `Ok` install / update calls) and `GitAdapter` (thin
  shim over `git.rs`). Adapter `install` / `update` failures aggregate
  into `Vec<InstallFailure>` and surface as a grouped `⚠ N install
  operations failed` summary (mirrors v0.8 SAFE-01 `RemoveFailure`
  pattern). (ADP-01..04)
- `tome.lock`-authoritative `tome sync`. Reconciles every managed skill
  against the lockfile and classifies as Match / Drift / Vanished
  (`reconcile.rs::ReconcileClass`). Per-class summary on every sync
  (`✓ N match · ⚠ N drift · ⚠ N vanished`). On Drift, applies installs /
  updates via the marketplace adapter (subject to consent) and verifies
  the resulting `content_hash` against the lockfile. Edit-in-library
  detection prompts fork / revert / skip (default fork interactively,
  default skip with warning under `--no-input`). Drift basis is
  `content_hash`, not version (Phase 11 D-08). (RECON-01..05)
- `auto_install_plugins` per-machine consent flow. First sync with non-
  empty drift prompts `Auto-install missing plugins on every sync?
  [Y/n/never]`; choice persists in `machine.toml::auto_install_plugins`.
  Global flag `--no-install` overrides the persisted choice for the
  current invocation (mirrors Cargo's `--frozen` / `--locked`). (RECON-02)
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
- The literal stub error in `reassign.rs` pointing at "Phase 14 /
  `tome adopt`" is deleted; Unowned input is now accepted directly.

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

## [0.9.0] - 2026-04-29

The **v0.9 Cross-Machine Path Overrides** milestone. Adds a per-machine path-remapping layer in `machine.toml` so the same `tome.toml` can ship in dotfiles across machines with divergent on-disk layouts. Bundles a Phase-8 review-tail pass that hardens the v0.8 `tome browse` partial-failure UX and lifts a `StatusMessage` enum.

### Added

- `[directory_overrides.<name>]` section in `machine.toml` for per-machine path remapping. Each override entry can supply a `path` that replaces the value in `tome.toml`, allowing the same shared config to work across machines whose home layouts differ. Override application happens at config load (after tilde expansion, before `Config::validate`), so all downstream code sees the canonical post-override paths. Unknown override directory names emit a typo-target stderr warning instead of silently being ignored. Override-induced validation failures are wrapped with a distinct error attributing them to `machine.toml` rather than `tome.toml`. (PORT-01..04, [#458](https://github.com/MartinP7r/tome/issues/458))
- `(override)` annotation in `tome status` and `tome doctor` text and JSON output for any directory whose path was rewritten by a `machine.toml` override, so the user can tell at a glance which paths come from the portable config and which come from the machine-local layer. (PORT-05)
- TUI status-bar `Pending` state for in-progress actions: `Opening: <path>...` appears in `tome browse` before the `xdg-open`/`open` syscall returns, replacing the prior "no feedback" gap. Pre-block status messages drain pending TTY events to avoid stale keypresses interleaving with the `Success`/`Warning` outcome banner. (POLISH-01)
- `ClipboardOccupied` errors in `tome browse copy path` now auto-retry once with a 100 ms backoff before surfacing the warning, so most transient X11/Wayland data-control collisions become invisible to the user. (POLISH-03)
- Test additions: success-banner-absence assertion on `tome remove` partial-failure (TEST-01); end-to-end retry-after-fix coverage (TEST-02); `status_message_from_open_result` 3-arm unit tests (TEST-03); `regen_warnings` deferred-emit ordering (TEST-04).

### Changed

- `StatusMessage` redesigned as a `Success | Warning | Pending` enum with `body`, `glyph`, and `severity` accessors. Old code that built status text by string concatenation is gone; all callers funnel through the enum so glyph + colorization stay consistent. (POLISH-02, [#463](https://github.com/MartinP7r/tome/issues/463))
- `FailureKind::ALL` is now compile-time-enforced via an exhaustive-match sentinel: adding a new `FailureKind` variant without updating `ALL` is a compile error. Eliminates the silent-drop class of bugs where a new failure kind would be omitted from the partial-failure summary. (POLISH-04)
- `RemoveFailure::new` adds a `debug_assert!(path.is_absolute())` invariant. Catches the "relative path leaked into a removal failure record" class of bug in debug builds before it reaches the user-facing summary. (POLISH-05)
- `arboard` is now patch-pinned to `>=3.6, <3.7` with an in-line bump-review policy (Cargo.toml). The match arms in `browse/app.rs::execute_action` and `try_clipboard_set_text_with_retry` must remain exhaustive — a new `arboard::Error` variant unobserved is a silent UX regression because the fall-through branch hides the semantic. The pin forces a manual review on minor bumps. (POLISH-06)

### Fixed

- `tome relocate` now emits a stderr warning (`warning: could not read symlink at <path>: <error>`) when a managed-skill symlink cannot be read, instead of silently recording the entry as having no provenance. Mirrors the eprintln-warning pattern shipped in PR #448. ([#449](https://github.com/MartinP7r/tome/issues/449))

### Internal

- Dead `SkillMoveEntry.source_path` field removed; `tome relocate` no longer carries the unused field through its move plan. (TEST-05)

## [0.8.2] - 2026-04-27

### Added

- `tome add <owner>/<repo>` now expands a bare GitHub slug to `https://github.com/<owner>/<repo>` so users can paste an `org/repo` token directly from the address bar without ceremony. URLs (anything containing `://` or starting with `git@`) are left untouched, and the heuristic refuses paths with relative segments (`./foo`, `../bar`) or invalid characters (spaces, etc.) so a typo never confidently rewrites to the wrong clone target. Example: `tome add planetscale/database-skills` is now equivalent to `tome add https://github.com/planetscale/database-skills`. ([#471](https://github.com/MartinP7r/tome/pull/471))

## [0.8.1] - 2026-04-26

The **v0.8.1 hotfix** for the v0.8.0 release. Fixes a lockfile regen + save chain ordering issue surfaced immediately after the v0.8.0 cut. ([#468](https://github.com/MartinP7r/tome/pull/468))

### Fixed

- `tome sync` save-chain ordering: lockfile regeneration runs after manifest persist so a partial-failure mid-chain cannot leave the lockfile pointing at a manifest entry that was never written. Distinct error wording on lockfile-vs-manifest failures so the user can tell which step blew up. (HOTFIX-01/02/03)

## [0.8.0] - 2026-04-26

The **v0.8 Safety Refactors** milestone — partial-failure visibility, cross-platform `tome browse`, and surfaced warnings. Closes the longstanding gap where `tome remove` and `tome browse` actions could fail silently. ([#460](https://github.com/MartinP7r/tome/pull/460))

### Fixed

- `tome remove` now aggregates partial-cleanup failures and exits non-zero with a distinct `⚠ N operations failed` summary grouped by failure kind (distribution symlinks, library entries, library symlinks, git cache). The success banner (`✓ Removed directory ...`) is suppressed entirely when failures occur, so it cannot hide a `⚠` warning that scrolled off-screen. On partial failure the directory's config entry AND its manifest entries are preserved so the user can re-run `tome remove <name>` after addressing the underlying cause (typically permission fixes) — previously the config was unconditionally dropped, leaving orphaned filesystem artifacts with no programmatic recovery path. Previously the command reported success while filesystem artifacts leaked. (SAFE-01, [#413](https://github.com/MartinP7r/tome/issues/413))
- `tome browse` actions `open` (ViewSource) and `copy path` (CopyPath) now work on Linux — `open` dispatches to `xdg-open` and `copy path` uses the `arboard` crate with both X11 and Wayland (`wayland-data-control`) backends enabled. `open` now uses `.status()` instead of `.spawn()` so a non-zero exit from `xdg-open` (no MIME handler, no DISPLAY) surfaces as `⚠ xdg-open exited N for: <path>` instead of the previous silent `✓ Opened` lie. Clipboard failures surface targeted hints: `⚠ Clipboard unavailable (headless or unsupported session)` for `ClipboardNotSupported`, `⚠ Clipboard busy (another app is holding it); try again` for `ClipboardOccupied`. Both success (`✓`) and failure (`⚠`) outcomes appear in the TUI status bar in place of the keybind line until the next keypress, replacing the prior macOS-only silent-drop behavior. The `sh -c "echo -n ${path} | pbcopy"` invocation is removed (eliminates a command-injection vector). (SAFE-02, [#414](https://github.com/MartinP7r/tome/issues/414))
- Wizard summary table now aligns correctly in interactive terminals. Previously, the bold ANSI escape codes wrapping header cells (e.g. `\x1b[1mNAME\x1b[0m`) were counted as visible characters by `tabled 0.20`'s default width calculation, inflating header cell widths by 8 columns and misaligning the column dividers with the body rows. Enabled tabled's `ansi` feature so escape sequences are correctly excluded from width measurement.

## [0.7.0] - 2026-04-23

The **v0.7 Wizard Hardening** milestone. The Phase 4–6 code shipped to users interim as v0.6.2 (on 2026-04-17); this release is the formal milestone cut and bundles the post-milestone safety patches on top. Users upgrading from v0.6.2 get the safety patches; users on v0.6.1 or earlier get the full wizard hardening surface.

### Changed

- Migrated `tome init` directory summary table to `tabled` with `Style::rounded()` borders and terminal-width-aware truncation via `Width::truncate(..).priority(PriorityMax::right())`. Long paths (including git-repo clones under `~/.tome/repos/<sha>/`) now render cleanly on narrow terminals without breaking column alignment. (WHARD-07)
- Marked WIZ-01 through WIZ-05 as validated / hardened in `PROJECT.md`; removed the stale "Known Gaps (deferred from v0.6)" entry. Phases 4 + 5 closed the correctness gaps (validation, overlap detection, test coverage) and this release reflects that in the project docs. (WHARD-08)
- `Config::validate()` now rejects invalid type/role combinations, library-vs-distribution path overlaps (Cases A/B/C), and circular library paths before save. All four validation bail sites use the Conflict/Why/Suggestion error template. (WHARD-01/02/03)
- `Config::save_checked` enforces expand → validate → TOML round-trip → atomic write. The wizard save path and `--dry-run` branch share this pipeline, so invalid configs can no longer reach disk.
- Removed deprecated internal APIs: `DistributeResult.target_name` alias and `SyncReport.warnings` field (no external impact — `DistributeResult` is not serialized to JSON).

### Added

- `--no-input` flag now supported on `tome init` (previously bailed). Runs the wizard non-interactively using sensible defaults — required for CI smoke tests and headless provisioning.
- Integration tests for `tome init --dry-run --no-input` on empty and seeded `HOME` directories, asserting the generated config validates and round-trips through TOML byte-equal.
- Table-driven `(DirectoryType × DirectoryRole)` matrix test — exhaustive 12-combo coverage verifying `valid_roles()` ↔ `Config::validate()` agreement. (WHARD-04/05/06)
- Regression test for `tome backup restore` bail-on-failure: `restore_bails_when_pre_snapshot_fails` guards against future simplification of the safety-snapshot propagation.

### Fixed

- `tome backup restore` now aborts if the pre-restore safety snapshot fails, instead of silently proceeding with the destructive `git checkout`. The safety snapshot is the user's only recovery path if a restore was accidental. (#415)
- Warn on git cache HEAD-sha read failure instead of silently recording `git_commit_sha: null` in the lockfile (false "no provenance" claim). (#417)
- Warn on SKILL.md read failure post-scan instead of silently dropping frontmatter metadata. `tome browse` no longer hides affected skills' descriptions. (#418)

### Docs

- Enable `mdbook-mermaid` preprocessor on GitHub Pages — all 10 existing mermaid diagrams across `introduction.md`, `test-setup.md`, and `tool-landscape.md` now render correctly (previously shown as raw code blocks). (#450)
- Refresh introduction diagram to reflect the v0.6+ unified directory model with current tool names (Codex, Antigravity, Cursor) and type + role annotations.
- Archive v0.7 milestone planning artifacts to `.planning/milestones/v0.7-ROADMAP.md` and `.planning/milestones/v0.7-REQUIREMENTS.md`.

### Internal

- `/pr-review-toolkit` whole-codebase review produced 36 prioritized findings; 5 shipped in this release (P0 safety fixes + 2 dead-code cleanups). 30 remaining findings filed as issues for v0.8 scoping.

## [0.6.0] - 2026-04-16

The **v0.6 Unified Directory Model** milestone. Phases 1–3 shipped (config type system + git sources + browse TUI polish). Interim patch releases v0.6.1 and v0.6.2 followed without formal CHANGELOG entries; v0.6.2 also carried the v0.7 Wizard Hardening code surface ahead of its formal v0.7.0 release.

### Breaking Changes

The `[[sources]]` and `[targets.*]` config sections have been replaced by a single `[directories.*]` section. tome will refuse to parse old-format config files and show a migration hint.

**Before (v0.5):**
```toml
library_dir = "~/.tome/skills"

[[sources]]
name = "my-skills"
path = "~/skills"
type = "directory"

[[sources]]
name = "plugins"
path = "~/.claude/plugins/cache"
type = "claude-plugins"

[targets.claude]
enabled = true
method = "symlink"
skills_dir = "~/.claude/skills"
```

**After (v0.6):**
```toml
library_dir = "~/.tome/skills"

[directories.my-skills]
path = "~/skills"
type = "directory"
role = "source"

[directories.plugins]
path = "~/.claude/plugins/cache"
type = "claude-plugins"

[directories.claude]
path = "~/.claude/skills"
type = "directory"
role = "target"
```

**Key changes:**
- Each directory has a `type` (directory, claude-plugins, git) and a `role` (source, target, synced, managed)
- `role` defaults based on type: `directory` → source, `claude-plugins` → managed
- `synced` role = both source and target (discovered here AND distributed here)
- `enabled`/`method`/`skills_dir` fields removed — `path` is the only location field
- `disabled_targets` in `machine.toml` renamed to `disabled_directories`

### Added

- Git source type: `type = "git"` directories clone a remote repo into `~/.tome/repos/<sha256>/` with shallow clone, ref pinning (branch / tag / rev), and offline fallback to cached state.
- Per-directory skill selection via `machine.toml`: `[directories.<name>]` blocks accept `enabled = [...]` allowlist OR `disabled = [...]` blocklist (mutually exclusive).
- `tome add <url>` — register a git skill repo from URL.
- `tome remove <name>` — remove a directory from config with full cleanup (symlinks, library entries, manifest, lockfile).
- `tome reassign` / `tome fork` — change a skill's provenance between directories.
- Browse TUI polish: adaptive dark/light theming, fuzzy-match highlighting, scrollbar, markdown preview rendering, and help overlay.

## [0.5.4] - 2026-04-10

### Added
- **Config-based tool root detection** (#390): `shares_tool_root` now derives tool roots from configured source paths instead of hardcoded directory names. No code changes needed when new tools are added.
- **`--json` for status and doctor** (#374): `tome status --json` and `tome doctor --json` produce structured JSON. Uses `CountOrError` type for clean API shape (`{"count": 254}` not `{"Ok": 254}`)
- **Graceful Ctrl-C** (#373): Signal handler prints "interrupted — run `tome sync` to resume" and exits with code 130. Sync pipeline is crash-safe by design.
- **Frontmatter parsed during discovery** (#393): `DiscoveredSkill` now carries parsed `SkillFrontmatter`. Parse errors reported as warnings instead of silently swallowed.
- **XDG config for tome_home** (#369): `~/.config/tome/config.toml` with `tome_home` field as alternative to `TOME_HOME` env var. Resolution: CLI flag → env var → config file → default.

### Fixed
- **Lockfile write failure is now an error** (#394): Previously demoted to warning; now blocks sync with actionable message
- **Safer managed skill skip default**: When path canonicalization fails for managed skills, default to skip (preventing circular symlinks) instead of distribute
- **Tighter tool root matching**: Parent directory check guards against matching paths that only share the home directory

### Closed
- **#370** Merge lockfile/manifest — not planned; separation is by design (internal tracking vs portable snapshot)
- **#391** Extract TOOL_DIRS — superseded by #390

### Deferred
- **#362** Init consolidation — moved to v0.6 (unified directory model)

## [0.5.3] - 2026-04-09

### Added
- **`--no-input` global flag** (#376): Suppresses all interactive prompts (cleanup, triage, install, doctor). Implies `--no-triage` for sync. Errors on `tome init`.
- **Managed skill counts in sync output** (#389): Per-target output shows `skipped (managed)` count for skills not distributed to their own tool
- **Group triage output by source** (#380): Changes grouped under source headers with `+`/`~`/`-` indicators instead of flat list
- **Batch stale cleanup prompt** (#382): Shows all stale skills grouped by previous source, confirms once instead of per-skill
- **Keybinding hints on triage** (#381): "(space to toggle, enter to confirm)" on MultiSelect prompt
- **Subcommand help examples** (#378): Every subcommand `--help` includes usage examples
- **Updated docs** (#368): README command table and `docs/src/commands.md` updated with all commands and new flags (`--no-input`, `--tome-home`)

### Fixed
- **NO_COLOR support verified** (#371): `console` crate already respects `NO_COLOR` env var; added integration test
- **Semantic exit codes verified** (#375): clap returns exit code 2 for invalid arguments; runtime errors return 1; added integration tests
- **`--no-input` threaded through doctor** — `tome doctor --no-input` skips repair prompt
- **Legacy symlink removal warns on failure** instead of silently discarding errors
- **Plugin reconciliation runs with `--no-input`** — users get info message about missing plugins instead of silent skip

## [0.5.2] - 2026-04-05

### Fixed
- **Legacy managed symlink cleanup** during sync: removes stale symlinks from targets on first run after v0.5.1 upgrade

## [0.5.1] - 2026-04-05

### Fixed
- **Default `library_dir` from TOME_HOME** (#383): defaults to `TOME_HOME/skills` instead of hardcoded `~/.tome/skills`
- **Skip managed skills to own tool** (#385): managed plugin skills no longer distributed to their source tool's skills directory, preventing duplicates

## [0.5.0] - 2026-03-28

### Added
- **Auto-install missing managed plugins** during `tome sync` (#347, #355): detects plugins in the lockfile that aren't installed locally, prompts to install via `claude plugin install`
- **Remote sync** integrated into `tome sync` (#349, #353): pull from remote before sync, push after commit. Fast-forward-only merges with actionable error on divergence
- **Remote setup wizard** in `tome backup init`: offers to add a remote URL, verifies connectivity, pushes initial commit
- `--no-triage` flag for `tome sync` to skip interactive prompts (for CI/scripts)
- `tome version` subcommand and `-V` short flag (#298)
- `typos-cli`, `cargo-machete`, and `cargo-semver-checks` CI checks (#297)
- `TomePaths` struct bundles `tome_home` and `library_dir` to prevent parameter swaps (#287)
- `TargetName` newtype for type-safe target identifiers (#285)
- Log disabled target skips during sync (#284)
- Warn on unknown `disabled_targets` entries in `machine.toml` (#281)
- Validate parent directory in `resolve_tome_home()` (#280)
- Test to verify `tome_home` / `library_dir` separation (#279)

### Changed
- **BREAKING: `tome update` removed** (#352): functionality merged into `tome sync`, which now includes lockfile diffing and interactive triage
- **Git repo root moved** from `~/.tome/skills/` to `~/.tome/` (#348, #350): backup repo now tracks skills, `tome.toml`, `tome.lock`, and future config
- Restructured tome home directory to `~/.tome/` (#271)

### Fixed
- Corrected `tome config` help text from "Show or edit" to "Show" (#296)
- Added missing `.unwrap()` on `TomePaths::new()` in `repair_library` test
- Fix stale path references after `~/.tome/` restructure (#283, #282)
- Suppress noisy `canonicalize` warnings in dry-run mode (#266)

## [0.3.3] - 2026-03-15

### Changed
- Removed MCP server and MCP distribution method (#263)
- Updated milestone naming from v0.4/v0.4.x to v0.4.1/v0.4.2 (#264)

### Fixed
- Pass `--head` to `gh pr create` in release target (#256)

## [0.3.2] - 2026-03-15

### Added
- Vercel Skills comparison research doc (#254)

### Fixed
- Allow Zlib license in cargo-deny config (#251)

## [0.3.1] - 2026-03-14

### Added
- `tome browse` interactive skill browser (#249)
- Audit known targets/sources against current platform docs (#248)

### Fixed
- Suppress noisy `installed_plugins.json` parent dir warning (#247)

## [0.3.0] - 2026-03-13

### Added
- **Per-machine preferences**: `~/.config/tome/machine.toml` with `disabled` list — skills stay in library but are skipped during distribution
- **`tome update` command**: loads lockfile, diffs against current state, presents added/changed/removed skills interactively, offers to disable unwanted new skills
- **`tome.lock` lockfile**: reproducible library snapshots with provenance metadata
- **Connector architecture**: `BTreeMap<String, TargetConfig>` replaces hardcoded Targets struct — any tool can be added as a target without code changes
- **KnownTarget registry**: wizard auto-discovers common tool locations for target configuration
- `--json` flag for `tome list`, structured warning collection, data struct extraction
- **Two-tier consolidation**: managed skills (ClaudePlugins) are symlinked, local skills (Directory) are copied into the library
- **Content hashing**: SHA-256 manifest (`.tome-manifest.json`) for idempotent sync — unchanged skills are skipped
- **`.gitignore` generation** for library directory to support git-friendly skill tracking
- `--machine` global CLI flag to override machine preferences path

### Changed
- `Config::exclude` changed from `Vec<String>` to `BTreeSet<SkillName>` for type safety
- `count_entries` now skips hidden directories

### Fixed
- Atomic lockfile and machine prefs saves (temp+rename) to prevent corruption on crash
- `sync` now cleans up disabled skill symlinks from targets (previously only `update` did this)
- MCP server now filters out disabled skills from machine preferences
- `offer_git_commit` scopes `git add` to tome-managed paths instead of `git add .`
- `cleanup_disabled_from_target` verifies symlinks point into the library before removing
- `count_health_issues` no longer double-counts broken managed symlinks
- Managed skill consolidation repairs stale directory state instead of silently skipping
- Various security and correctness fixes (MCP path validation, doctor repair, config validation)
- Sync lifecycle and `--force` integration test coverage

## [0.2.0] - 2026-02-25

### Added
- **Library copies**: library is the source of truth for local skills — `tome sync` copies from sources into the library instead of symlinking
- **Git init** offered during wizard for library directory
- **Git commit** offered after sync when library is a git repo with changes

### Changed
- Consolidation model: sources → (copy) → library → (symlink) → targets (previously sources → (symlink) → library → (symlink) → targets)

### Fixed
- Skip distribution to targets where skills already originate (prevents circular symlinks)
- MCP `read_skill` path validation (symlink escape vulnerability)
- Doctor repair checks `target.enabled` before operating
- Config validation errors on nonexistent parent directory
- Wizard surfaces discovery errors instead of silently swallowing
- Various hardening (canonicalization, error propagation, `expect()` removal)

## [0.1.4] - 2026-02-25

### Added
- Progress spinners during sync pipeline stages (discover, consolidate, distribute, cleanup)
- Table-formatted output for `tome list` using `tabled`
- Dry-run banner (`[dry-run] No changes will be made`) when running with `--dry-run`
- Verbose output mode showing per-stage details during sync

### Fixed
- Error handling and silent failure bugs across discover, distribute, and MCP modules
- Symlink escape vulnerability in MCP `read_skill` tool
- Non-object `mcpServers` now returns a clear error instead of panicking

## [0.1.3] - 2026-02-24

### Added
- Graceful handling of pre-init state in `tome status` and `tome doctor`
- `status` shows a helpful "run `tome init`" message when unconfigured
- `doctor` shows init prompt instead of erroring when no config exists

### Changed
- Updated GitHub Actions checkout from v4 to v6
- Dependabot config now ignores cargo-dist-managed workflows

## [0.1.2] - 2026-02-22

### Fixed
- Exclude `tome-mcp` binary from Homebrew installer (only `tome` is needed)
- Updated dependencies

## [0.1.1] - 2026-02-20

### Added
- README badges (crates.io, CI, license)
- Mascot image in README

## [0.1.0] - 2026-02-19

### Added
- Initial release
- **Sync pipeline**: discover → consolidate → distribute → cleanup
- **Discovery**: `ClaudePlugins` (reads `installed_plugins.json` v1 and v2) and `Directory` source types
- **Library**: symlink-based consolidation with idempotent create/update/skip
- **Distribution**: symlink targets (Antigravity) and MCP targets (Codex, OpenClaw)
- **Cleanup**: removes broken symlinks from library and stale links from targets
- **Interactive wizard** (`tome init`): auto-discovers known source locations, configures targets
- **Doctor** (`tome doctor`): diagnoses broken symlinks and missing sources, optional repair
- **Status** (`tome status`): read-only summary of library, sources, targets, and health
- **MCP server** (`tome serve` / `tome-mcp`): exposes `list_skills` and `read_skill` tools over stdio
- **Config**: TOML at `~/.config/tome/config.toml` with tilde expansion
- `--dry-run`, `--quiet`, `--verbose` global flags
- `tome list` / `tome ls` for listing discovered skills
- `tome config --path` for printing config location
- CI on Ubuntu and macOS (fmt, clippy, test, release build)
- cargo-dist release workflow for cross-platform binaries

[Unreleased]: https://github.com/MartinP7r/tome/compare/v0.7.0...HEAD
[0.7.0]: https://github.com/MartinP7r/tome/compare/v0.6.2...v0.7.0
[0.6.0]: https://github.com/MartinP7r/tome/compare/v0.5.4...v0.6.0
[0.3.3]: https://github.com/MartinP7r/tome/compare/v0.3.2...v0.3.3
[0.3.2]: https://github.com/MartinP7r/tome/compare/v0.3.1...v0.3.2
[0.3.1]: https://github.com/MartinP7r/tome/compare/v0.3.0...v0.3.1
[0.3.0]: https://github.com/MartinP7r/tome/compare/v0.2.0...v0.3.0
[0.2.0]: https://github.com/MartinP7r/tome/compare/v0.1.4...v0.2.0
[0.1.4]: https://github.com/MartinP7r/tome/compare/v0.1.3...v0.1.4
[0.1.3]: https://github.com/MartinP7r/tome/compare/v0.1.2...v0.1.3
[0.1.2]: https://github.com/MartinP7r/tome/compare/v0.1.1...v0.1.2
[0.1.1]: https://github.com/MartinP7r/tome/compare/v0.1.0...v0.1.1
[0.1.0]: https://github.com/MartinP7r/tome/releases/tag/v0.1.0
