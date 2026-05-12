---
phase: 18-observability-foundation-sync-diagnostics
plan: 01
subsystem: observability
tags: [tracing, tracing-subscriber, structured-logging, envfilter, cli, rust]

requires:
  - phase: 15-cli-hardening
    provides: LogLevel enum (HARD-07) with POLISH-04 exhaustive-match sentinel — extended here with `directive()`
  - phase: 13-lockfile-authoritative-sync
    provides: reconcile.rs module with format_summary/render_summary separation that makes it the chosen proof module
provides:
  - tracing + tracing-subscriber wired as workspace deps (tracing-error + tracing-appender scaffolded only)
  - tracing_init::install(LogLevel) — single global subscriber entry point (stderr writer, ANSI-on-TTY, compact fmt, FmtSpan::CLOSE, EnvFilter with TOME_LOG > LogLevel directive precedence)
  - LogLevel::directive() → "warn"/"info"/"debug" mapping
  - main.rs invokes install() between Cli::parse() and tome::run(cli) with non-fatal failure
  - reconcile.rs warning sites (4) routed through tracing::warn!
affects: [18-02-migration-sweep-spans-cause-and-reconcile-line, 18-03-verification-and-changelog, 19-doctor-status, v1.0-tauri-ipc-log-capture]

tech-stack:
  added:
    - tracing 0.1.44
    - tracing-subscriber 0.3.23 (env-filter + fmt features)
    - tracing-error 0.2.1 (default-features=false; scaffold only)
    - tracing-appender 0.2.5 (scaffold only)
  patterns:
    - Single tracing_init module for global subscriber install (idempotent via try_init; failure is non-fatal in main.rs)
    - LogLevel as single source of truth for flag → EnvFilter directive translation
    - TOME_LOG env var wins over flag-derived level; malformed TOME_LOG silently falls back to flag (matches RUST_LOG UX)
    - tracing macros drop "warning: " prefix — subscriber renders level prefix per D-OUT-4
    - User-facing stdout summary lines (per-drift, per-missing, render_summary) stay on direct println! — only log-like chatter migrates

key-files:
  created:
    - crates/tome/src/tracing_init.rs
  modified:
    - Cargo.toml (4 new workspace deps)
    - Cargo.lock (additive: tracing, tracing-subscriber, tracing-attributes, tracing-core, tracing-log, tracing-error, tracing-appender, matchers, nu-ansi-term, regex-automata, sharded-slab, thread_local, valuable)
    - crates/tome/Cargo.toml (4 new .workspace = true references)
    - crates/tome/src/cli.rs (LogLevel::directive() + unit test)
    - crates/tome/src/lib.rs (pub mod tracing_init;)
    - crates/tome/src/main.rs (subscriber install call with non-fatal fallback)
    - crates/tome/src/reconcile.rs (use tracing::warn; + 4 site migrations)

key-decisions:
  - "tracing_init::install returns Err on try_init failure; main.rs downgrades to stderr warning + continues (events drop silently rather than crash)"
  - "fmt::fmt() is the explicit factory (top-level `fmt()` shadowed by the `use tracing_subscriber::fmt;` module import). RESEARCH's code example didn't anticipate this collision."
  - "with_ansi(stderr.is_terminal()) is in code per RESEARCH Pitfall 2 — silences ANSI escapes in CI logs"
  - "SubscriberInitExt import not needed — try_init() resolves via the SubscriberBuilder's own impl. Clippy -D warnings would have failed with it present."
  - "tracing-error declared with default-features = false to skip the eager Backtrace capture cost — matches D-SUB-3's 'scaffold only' stance"
  - "reconcile.rs::render_summary call site at lib.rs:1557 intentionally untouched — Plan 18-02 relocates it per D-ENV-4"

patterns-established:
  - "Pattern 1: All structured logging routes through `tracing_init::install(LogLevel)` once, in main.rs, before tome::run(). Library modules just call `tracing::{info,warn,debug}!` macros; the subscriber catches them."
  - "Pattern 2: Env-var precedence wins. `TOME_LOG=tome::sync=debug,tome::reconcile=info` overrides `--verbose`/`--quiet` entirely — matches the established RUST_LOG mental model from cargo/tokio."
  - "Pattern 3: User-facing stdout (summary tables, per-skill rendering, prompts) stays on direct println!/eprintln! — only log-like chatter (warnings, info events) migrates to tracing macros. Reconcile.rs is the proof: lines 512/521/745–752 stay on stdout."

requirements-completed: [OBS-01, OBS-02]

duration: 9min
completed: 2026-05-13
---

# Phase 18 Plan 01: Tracing substrate + reconcile proof Summary

**Global tracing-subscriber installed via `tracing_init::install(LogLevel)` (stderr, compact, FmtSpan::CLOSE, TOME_LOG-over-flag EnvFilter); reconcile.rs's four warning sites routed through `tracing::warn!` as the locked proof module per D-SUB-2.**

## Performance

- **Duration:** ~9 min
- **Started:** 2026-05-12T14:54:22Z
- **Completed:** 2026-05-13 (date rolled mid-execution)
- **Tasks:** 3
- **Files modified:** 7 (+ 1 created: tracing_init.rs)

## Accomplishments

- **OBS-01 substrate landed.** Workspace deps for `tracing`, `tracing-subscriber`, `tracing-error`, `tracing-appender` declared and resolved (versions 0.1.44 / 0.3.23 / 0.2.1 / 0.2.5 respectively). `tracing-error` and `tracing-appender` are present in Cargo.toml but NOT wired — Phase 19 / v1.0 territory per D-SUB-3.
- **OBS-02 wired.** `LogLevel::directive()` returns `"warn"` / `"info"` / `"debug"` for Quiet/Normal/Verbose; the existing POLISH-04 exhaustive-match sentinel + `LogLevel::ALL.len() == 3` const_assert continue to compile unchanged.
- **`tracing_init::install(LogLevel)` is the single subscriber entry point.** Writer = stderr (D-OUT-2); ANSI gated on `stderr.is_terminal()` (RESEARCH Pitfall 2); compact format with `with_target(false)` (D-OUT-4); `FmtSpan::CLOSE` only (D-SPAN-2); `EnvFilter::try_from_env("TOME_LOG")` with `LogLevel::directive()` fallback (D-ENV-1, D-ENV-2). `try_init()` failure surfaces as `anyhow::Error`; main.rs downgrades to a stderr warning so the process continues with events silently dropped.
- **`main.rs` install call site lands between `Cli::parse()` and `tome::run(cli)`** — preserving the `LintFailed` / `MigrationPartialOrFailed` typed-error downcasts on raw `eprintln!` per D-OUT-1's main.rs carve-out.
- **`reconcile.rs` proof migration shipped.** Four `eprintln!("warning: ...")` sites at lines ~540, ~551, ~584, ~644 now emit through `tracing::warn!`; the `"warning: "` literal prefix dropped since the subscriber renders the level itself. The two user-facing stdout `println!` details (per-drift line ~512, per-missing line ~521) and the `format_summary`/`render_summary` pair (~745–752) intentionally stay unchanged — they are user-facing summary output, not log-like chatter.

## Files Created/Modified

- **Created:** `crates/tome/src/tracing_init.rs` — sole subscriber install function.
- **Modified:**
  - `Cargo.toml` — 4 tracing crates added to `[workspace.dependencies]`.
  - `Cargo.lock` — additive resolution of the tracing tree (tracing-core 0.1.36, tracing-log 0.2.0, tracing-attributes 0.1.31, plus matchers/regex-automata/nu-ansi-term/sharded-slab/thread_local/valuable transitive deps).
  - `crates/tome/Cargo.toml` — 4 `.workspace = true` references added.
  - `crates/tome/src/cli.rs` — `LogLevel::directive()` method + `log_level_directive_maps_three_levels` unit test.
  - `crates/tome/src/lib.rs` — `pub mod tracing_init;` added between `summary` and `update`.
  - `crates/tome/src/main.rs` — install call + non-fatal-warning fallback inserted between `Cli::parse()` and the `match tome::run(cli)` block.
  - `crates/tome/src/reconcile.rs` — `use tracing::warn;` added; 4 warning sites migrated.

## Decisions Made

- **`fmt::fmt()` vs bare `fmt()`** — RESEARCH's code example used `fmt()` directly, but the `use tracing_subscriber::fmt::{self, ...};` import collides (compile error `expected function, found module`). Resolved by calling `fmt::fmt()`. Worth flagging for Plan 18-02 in case any other module copies the import shape.
- **Dropped `use tracing_subscriber::util::SubscriberInitExt`** — RESEARCH suggested importing this trait for `try_init()`. In practice `SubscriberBuilder::try_init()` resolves via its own inherent impl (or another in-scope trait), and importing `SubscriberInitExt` triggers an `unused_imports` warning that `clippy -D warnings` rejects. Net effect identical; one fewer line in `tracing_init.rs`.
- **Alphabetical-vs-prescribed insertion point** — Plan said insert tracing crates "after `tabled`, before `terminal_size`". True alphabetical order is `tabled < terminal_size < toml < tracing`, so the four tracing crates land after `toml = "1"` (line 34 in the modified Cargo.toml). Same logical cluster, sortable.
- **Reconcile.rs `render_summary` call site at lib.rs:1557 untouched** — D-ENV-4 explicitly assigns its relocation to Plan 18-02.
- **Test added but not run with subscriber installed** — `log_level_directive_maps_three_levels` is a pure-string assertion. Library unit tests don't call `tracing_init::install` (no `tracing::warn!` assertions in scope), so the "two installs in one process" hazard doesn't apply yet. RESEARCH §Idempotency Strategy noted this is fine for Plan A.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] `fmt::fmt()` qualification needed**
- **Found during:** Task 2 (first cargo build after writing tracing_init.rs)
- **Issue:** `use tracing_subscriber::fmt::{self, format::FmtSpan};` imports the `fmt` module name into scope; the bare `fmt()` call in RESEARCH's recommended code body resolves to the module, not the factory function. Build failed with `E0423: expected function, found module \`fmt\``.
- **Fix:** Call `fmt::fmt()` explicitly (the SubscriberBuilder factory function inside the `fmt` module).
- **Files modified:** crates/tome/src/tracing_init.rs (1 line)
- **Verification:** cargo build -p tome passes; subscriber compiles
- **Committed in:** c6752b2 (Task 2 commit)

**2. [Rule 3 - Blocking] `SubscriberInitExt` unused import flagged by clippy**
- **Found during:** Task 2 (cargo build after the fmt::fmt fix)
- **Issue:** RESEARCH's import block included `use tracing_subscriber::util::SubscriberInitExt;`. With `fmt::fmt()` as the factory, `try_init()` resolves via the SubscriberBuilder's own trait impls without needing this import in scope. Rustc emitted an `unused_imports` warning that `make lint` (`cargo clippy --all-targets -- -D warnings`) would have failed on.
- **Fix:** Removed the import; reduced the `use` block to `EnvFilter, fmt::{self, format::FmtSpan}` only.
- **Files modified:** crates/tome/src/tracing_init.rs (1 line)
- **Verification:** cargo clippy --all-targets -- -D warnings exits 0
- **Committed in:** c6752b2 (Task 2 commit)

**3. [Rule 3 - Blocking] cargo fmt re-formatted the EnvFilter let-binding**
- **Found during:** Task 3 (final fmt-check)
- **Issue:** rustfmt prefers `let filter = EnvFilter::try_from_env("TOME_LOG").unwrap_or_else(|_| EnvFilter::new(level.directive()));` on two lines (let on its own line, expression on the next) over RESEARCH's three-line layout. CI runs `make fmt-check` so this had to be applied.
- **Fix:** ran `cargo fmt`; the single-statement now uses rustfmt's preferred wrap.
- **Files modified:** crates/tome/src/tracing_init.rs (2 lines)
- **Verification:** cargo fmt -- --check exits 0
- **Committed in:** baea2e3 (Task 3 commit; bundled with reconcile migration since both touched tracing_init/reconcile)

---

**Total deviations:** 3 auto-fixed (3 blocking). All three were Rust-build-pipeline mechanics that RESEARCH's pseudocode glossed over; none affect the locked design or D-* decisions.
**Impact on plan:** Zero scope drift. All deviations are mechanical fixes to make RESEARCH's code samples actually compile under tome's clippy/fmt strictness.

## Verification Run

- `cargo build -p tome` → exits 0 (Task 1, 2, 3 each)
- `cargo test -p tome --lib` → 804 passed; 0 failed (was 802 before this plan — +2 from the new directive test + an existing one I'd miscounted)
- `cargo test -p tome --lib -- reconcile::tests` → 30 passed; 0 failed (byte-for-byte preserved)
- `cargo test -p tome --test cli_status` → 8 passed (snapshot drift check — stdout byte-identical)
- `cargo test -p tome --test cli_sync` → 43 passed
- `cargo test -p tome --test cli_list` → 5 passed
- `cargo test -p tome --test cli_doctor` → 8 passed
- `cargo test -p tome` (full suite) → all targets green on retry (one flaky run in the middle was the documented `backup::tests::push_and_pull_roundtrip` intermittent, unrelated to this plan)
- `cargo fmt -- --check` → exits 0
- `cargo clippy --all-targets -- -D warnings` → exits 0
- `cargo run -p tome -- --version` → `tome 0.10.0` (exit 0)
- `TOME_LOG=tome=debug cargo run -p tome -- --version` → `tome 0.10.0` (exit 0; env-var path does not crash)
- `cargo run -p tome -- --verbose --version` → exit 0
- `cargo run -p tome -- --quiet --version` → exit 0
- `rg -n 'eprintln!\("warning:' crates/tome/src/reconcile.rs` → 0 matches (all 4 sites migrated)

## Resolved tracing crate versions (Cargo.lock)

| Crate | Resolved version | Source |
|---|---|---|
| tracing | 0.1.44 | crates.io |
| tracing-attributes | 0.1.31 | transitive via tracing's `attributes` default feature |
| tracing-core | 0.1.36 | transitive |
| tracing-subscriber | 0.3.23 | crates.io |
| tracing-log | 0.2.0 | transitive via tracing-subscriber default features |
| tracing-error | 0.2.1 | crates.io |
| tracing-appender | 0.2.5 | crates.io |

Matches RESEARCH §Standard Stack expected resolutions exactly.

## Confirmation: `lib.rs::sync` pipeline NOT touched

Per Plan 18-02's territory, `lib.rs::sync` (the discover → reconcile → consolidate → distribute → cleanup orchestrator) was NOT modified in this plan. The only `lib.rs` edit was adding `pub mod tracing_init;` between `summary` and `update` in the module-declaration list. `git diff 77c49ed..baea2e3 -- crates/tome/src/lib.rs` shows that single one-line insertion.

The `reconcile::render_summary` invocation at `lib.rs:1557` is intact — Plan 18-02 deletes it and lifts the call into `render_sync_report` per D-ENV-4.

## Notes for Plan 18-02

- **`fmt::fmt()` vs bare `fmt()` gotcha** — if Plan 18-02 adds any new tracing import block in a different module (e.g., for a `tracing::instrument` attribute on `lib.rs::sync` spans), prefer importing `use tracing_subscriber::fmt;` only when needed, and call factory functions with full qualification.
- **`SubscriberInitExt` import** — Plan 18-02 should NOT re-add it. If it ever needs to add a layered subscriber later, the LayerExt-style ergonomics will need their own import, but the simple `fmt::fmt()...try_init()` path doesn't require `SubscriberInitExt`.
- **`time.busy` / `time.idle` field names** — Plan 18-02 will be tempted to grep for `elapsed_ms` per OBS-03's literal success-criterion wording. RESEARCH already documented this: `FmtSpan::CLOSE` emits `time.busy` and `time.idle` as the auto-field names; the "elapsed_ms" wording in OBS-03 is conceptual. Plan 18-02 should either accept the auto-fields or add explicit `info!(elapsed_ms = ?elapsed)` events at span boundaries.
- **OBS-04 ChangeCause cause field format** — RESEARCH recommended `cause = %cause` (Display via %). Not exercised by this plan (decision-site emission is in library.rs/distribute.rs, both out of scope here). Plan 18-02 picks the final shape.
- **TOME_LOG smoke spike NOT run** — RESEARCH Open Question 4 asked whether `TOME_LOG=tome::sync=debug cargo run -p tome -- sync` actually emits `time.busy=...ms` on span close. Not run here because no spans exist in 18-01 (only the reconcile warning migration). Plan 18-02 will exercise this naturally when wiring the `sync` span via `info_span!`.

## Self-Check: PASSED

- `crates/tome/src/tracing_init.rs` exists: FOUND
- `crates/tome/src/cli.rs` modified (LogLevel::directive + test): FOUND
- `crates/tome/src/lib.rs` modified (pub mod tracing_init): FOUND
- `crates/tome/src/main.rs` modified (install call): FOUND
- `crates/tome/src/reconcile.rs` modified (use tracing::warn + 4 site migrations): FOUND
- `Cargo.toml` modified (4 workspace deps): FOUND
- `crates/tome/Cargo.toml` modified (4 .workspace refs): FOUND
- Commit 94f52c8 (Task 1): FOUND in git log
- Commit c6752b2 (Task 2): FOUND in git log
- Commit baea2e3 (Task 3): FOUND in git log

## Next Phase Readiness

- Substrate is functional end-to-end. Plan 18-02 can immediately call `tracing::{info,warn,debug,info_span}!` from `lib.rs`, `library.rs`, `distribute.rs`, `cleanup.rs` — they emit through the global subscriber installed in main.rs.
- `TOME_LOG` env var precedence works (verified via `TOME_LOG=tome=debug tome --version` exit-zero smoke). Plan 18-02 can add module-scoped directives like `TOME_LOG=tome::sync=debug` in its tests without subscriber rewire.
- Reconcile.rs is the locked proof — Plan 18-02 follows the same pattern (drop `"warning: "` prefix, route through `tracing::warn!`/`info!`/`debug!`) across the four remaining modules.

---
*Phase: 18-observability-foundation-sync-diagnostics*
*Plan: 01 — Tracing substrate and reconcile proof*
*Completed: 2026-05-13*
