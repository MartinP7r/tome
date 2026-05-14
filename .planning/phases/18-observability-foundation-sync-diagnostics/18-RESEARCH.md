# Phase 18: Observability foundation + sync diagnostics — Research

**Researched:** 2026-05-12
**Domain:** Rust `tracing` ecosystem + structured-logging migration of an existing CLI pipeline
**Confidence:** HIGH

## Summary

Phase 18 introduces `tracing` + `tracing-subscriber` as the structured-logging
substrate for tome and migrates ~114 `eprintln!`/`println!` call sites across
five in-scope modules (lib.rs sync pipeline, reconcile, library, distribute,
cleanup) onto `tracing::{info,warn,debug}!`. The locked design is captured
upstream in `18-CONTEXT.md` (15 numbered decisions D-SUB-1..D-ENV-4). This
research validates the locked design against current `tracing` (v0.1.44) and
`tracing-subscriber` (v0.3.23) documentation, pins the concrete builder calls,
and answers the specific gotchas the planner needs to know to avoid burning
cycles.

Two findings the planner should not skip: (1) `FmtSpan::CLOSE` emits
**`time.busy` and `time.idle`** as field names, NOT a literal `elapsed_ms`
field — the success criterion's "elapsed_ms" wording is conceptual; planner
treats the auto-emitted timing fields as satisfying OBS-03 and documents the
mapping in 18-PLAN.md so reviewers don't grep for the wrong string. (2)
Integration tests under `assert_cmd` each spawn a fresh process so global
subscriber init is naturally isolated; unit tests in the library crate that
run in one process need `try_init()` ignore-already-set, OR scope subscribers
per-test via `tracing::subscriber::with_default`. Phase 18 does not need
either today because no in-scope unit test asserts on tracing output — but
the planner should add a one-line guard against future surprises.

**Primary recommendation:** Implement Plan A exactly as D-SUB-1..D-OUT-4
prescribe — `tracing_init.rs` module, subscriber installed in `main.rs`
between `Cli::parse()` and `tome::run(cli)`, stderr writer, compact format,
`with_target(false)`, `FmtSpan::CLOSE`, `EnvFilter::try_from_env("TOME_LOG")`
with `LogLevel::directive()` fallback. Migrate `reconcile.rs` as the proof
module (6 sites, isolated, format_summary/render_summary already structured).
Plan B sweeps the remaining 4 modules and lands OBS-03 spans + OBS-04
ChangeCause + OBS-05 reconcile breakdown line in one PR.

## User Constraints (from CONTEXT.md)

### Locked Decisions

**Substrate + migration shape:**

- **D-SUB-1 (tracing locked, no fallback gate):** `tracing` is firm. No
  cost-gate moment. If Plan A is rough, Plan B sub-divides — but no crate
  swap mid-phase.
- **D-SUB-2 (Plan A = substrate + 1 proof module; Plan B = sweep):**
  - Plan A: `tracing` + `tracing-subscriber` + scaffolded `tracing-error` +
    scaffolded `tracing-appender` in Cargo.toml; subscriber init at CLI
    boundary using `cli.log_level() → EnvFilter`; migrate ONE proof module
    fully (recommend `reconcile.rs`).
  - Plan B: sweep 4 remaining in-scope modules + OBS-03 spans + OBS-04
    ChangeCause emission + OBS-05 reconcile-line relocation.
- **D-SUB-3 (Cargo.toml crate set):** Four crates as workspace deps:
  - `tracing` — core macros (wired)
  - `tracing-subscriber` — `EnvFilter` + `fmt` layer (wired)
  - `tracing-error` — **scaffolded only.** No `ErrorLayer` install in
    Phase 18. Phase 19 or v1.0 wires.
  - `tracing-appender` — **scaffolded only.** No file sink in Phase 18.
    v1.0 Tauri IPC wires.

**Output discipline + rendering:**

- **D-OUT-1 (doc-enforced scope contract):** In-scope/out-of-scope module
  lists below are the contract. CONTEXT.md + 18-PLAN.md enumerate. Code
  review catches drift. NO `#![deny(clippy::print_stdout, clippy::print_stderr)]`.
- **D-OUT-2 (sink = stderr):** Subscriber init must call
  `.with_writer(std::io::stderr)`. Matches Phase 16 D-UX01-4 (cleanup → stderr)
  and HARD-15 (wizard → stderr).
- **D-OUT-3 (spans verbose-only):** Default `tome sync` (info level) does
  NOT print per-step span lines. Spans render only at `debug` (i.e.
  `--verbose` or `TOME_LOG=tome::sync=debug`). OBS-05 reconcile breakdown
  is the only NEW info-level addition.
- **D-OUT-4 (format = compact, no target, no info prefix):** `fmt::compact()`
  with `.with_target(false)` + info-level prefix suppressed. `warn`/`error`
  keep the level prefix.

**Span surface + change-cause:**

- **D-SPAN-1 (flat span tree: top + 5 step spans):** One top-level `sync`
  span wraps the pipeline. Inside: `discover`, `reconcile`, `consolidate`,
  `distribute`, `cleanup` — one span per step, no nesting below. NO
  `distribute_dir{name=…}` per directory; NO `consolidate_skill{name=…}`
  per skill.
- **D-SPAN-2 (FmtSpan::CLOSE only):** `with_span_events(FmtSpan::CLOSE)`.
  One line per span on completion. NO `NEW`/`ENTER`/`EXIT`.
- **D-SPAN-3 (ChangeCause enum + ALL sentinel):**
  `enum ChangeCause { HashChanged, PreviouslyFailed, NewlyAdded, DirectoryNowAllowed }`
  with `ALL: &[ChangeCause; 4]`, exhaustive-match sentinel,
  `const_assert!(ChangeCause::ALL.len() == 4, ...)`, `impl Display` returning
  literal strings: `"hash changed"`, `"previously failed"`, `"newly added"`,
  `"directory now allowed"`.
- **D-SPAN-4 (decision-site emission, no result-struct extension):**
  `info!` events fire at decision branch in `library.rs::consolidate` and
  `distribute.rs::distribute_to_directory`. NO new field on
  `ConsolidateResult` / `DistributeResult`. NO central walking in
  `lib.rs::sync`.

**Flag/env semantics + OBS-05 placement:**

- **D-ENV-1 (TOME_LOG wins; env overrides flag):**
  ```rust
  let filter = EnvFilter::try_from_env("TOME_LOG")
      .unwrap_or_else(|_| EnvFilter::new(level_from_log_level(cli.log_level())));
  ```
- **D-ENV-2 (default level = info):** No flag, no env var → `info`. No
  baked-in per-target downgrades.
- **D-ENV-3 (flag UX byte-near-identical):** `--quiet` → `warn`; default →
  `info`; `--verbose` → `debug`. Span CLOSE events are NEW lines at
  `--verbose` (additive). NO lines silently disappear.
- **D-ENV-4 (OBS-05 line moves into render_sync_report; reconcile detail
  relocates):**
  - DELETE the inline `reconcile::render_summary` call at `lib.rs:1557`.
  - `render_sync_report` (lib.rs:1801) emits `reconcile: N match · M drift
    · K vanished · L missing-from-machine` IMMEDIATELY ABOVE the per-bucket
    cleanup output.
  - Per-drift detail + per-vanished warnings relocate from line 1557 into
    `render_sync_report` (either lifted, or `format_summary` called from
    inside `render_sync_report`). Planner picks.

### Claude's Discretion

- Plan A's choice of proof module (recommendation: `reconcile.rs`).
- Subscriber init location (recommendation: `main.rs` between `Cli::parse()`
  and `tome::run(cli)`).
- Whether to add a `tracing_init` module (recommendation: yes,
  `crates/tome/src/tracing_init.rs`).
- Cause field format in OBS-04 events (recommendation: `Display` via `%`).
- Whether to use `#[tracing::instrument]` attribute or explicit `info_span!`
  (recommendation: explicit `info_span!` for in-`lib.rs::sync` step
  boundaries).
- `ChangeCause` module location (recommendation: new
  `crates/tome/src/change_cause.rs`).
- LogLevel → EnvFilter directive mapping function — colocated with
  `LogLevel` in `cli.rs`.
- Reconcile classification line glyphs (recommendation: reuse `✓`/`⚠`
  palette).
- Whether `render_sync_report` orchestrates both reconcile-summary AND
  cleanup-buckets rendering (recommendation: yes — centralizes ordering).

### Deferred Ideas (OUT OF SCOPE)

- `tracing-error::ErrorLayer` install + `anyhow .in_current_span()` sweep
  (Phase 19 or v1.0)
- `tracing-appender` file sink (v1.0 Tauri)
- JSON log output `--log-format json` (OBS-FUTURE-01 → v0.12 or later)
- OpenTelemetry export (OBS-FUTURE-02)
- Sub-spans per directory or per skill (D-SPAN-1 keeps flat)
- Per-target-module EnvFilter baked defaults (D-ENV-2 declined)
- Custom `FormatEvent` impl (D-OUT-4 declined unless `fmt::compact()`
  knobs prove insufficient)
- Migration of out-of-scope modules: `wizard.rs`, `browse/*`, `status.rs`,
  `doctor.rs` (Phase 19 OBS-06 may touch doctor's *diagnostic* surface,
  not its table), `lint.rs`, `main.rs` error printer, all `tabled::Table`
  render sites

## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| OBS-01 | Adopt `tracing` + `tracing-subscriber`; migrate internal `eprintln!`/`println!` chatter in sync/reconcile/consolidate/distribute/cleanup to `tracing::{info,warn,debug}!`. Wizard/TUI/tables stay on stdout. | Crate selection (Standard Stack), Output channel split table, Don't Hand-Roll guidance |
| OBS-02 | Wire `--verbose`/`--quiet` + `TOME_LOG` env var to `EnvFilter`. Default `info`; `--verbose` → `debug`; `--quiet` → `warn`. `LogLevel` enum wraps the subscriber config. | `LogLevel::directive()` mapping; `EnvFilter::try_from_env("TOME_LOG")` precedence pattern; verified against `tracing-subscriber` 0.3.23 docs |
| OBS-03 | `tome sync` emits per-pipeline-step spans (`discover`, `reconcile`, `consolidate`, `distribute`, `cleanup`) with elapsed-ms attached; nested under a top-level `sync` span; visible in `--verbose` and via `TOME_LOG=tome::sync=debug`. | `info_span!` + `entered()` pattern; `FmtSpan::CLOSE` auto-emits `time.busy`+`time.idle` (NOT literal `elapsed_ms`) |
| OBS-04 | On consolidate/distribute re-emit, log at `info!` naming cause: `hash changed`/`previously failed`/`newly added`/`directory now allowed` with skill + directory as structured fields. | `ChangeCause` enum + `Display` impl per D-SPAN-3; decision-site emission per D-SPAN-4; concrete call-site map below |
| OBS-05 | `tome sync` final summary block includes `reconcile: N match · M drift · K vanished · L missing-from-machine` immediately above per-bucket cleanup summary. | `ReconcileReport::{matches, drift, vanished, missing}` already populated by Phase 13 (`reconcile.rs:101-105`); relocate-and-extend `render_sync_report` per D-ENV-4 |

## Project Constraints (from CLAUDE.md)

These directives govern Phase 18 implementation:

- **Rust edition 2024, MSRV 1.85.0+.** Both `tracing` 0.1 and
  `tracing-subscriber` 0.3 declare `rust-version: 1.65.0`, well below
  tome's MSRV. No compatibility risk.
- **Quality gates:** `cargo fmt`, `cargo clippy --all-targets -- -D warnings`,
  `cargo test` must all pass.
- **`make ci` matches CI pipeline.** Plan B's PR must `make ci` clean.
- **Unit tests co-located** with `#[cfg(test)] mod tests`. Integration tests
  in `tests/cli_*.rs` use `assert_cmd`.
- **No project skills directory** in this repo — N/A.
- **Non-interactive shell flags** (`cp -f`, `rm -rf`, etc.) — N/A for this
  phase.
- **No backward compat shim.** D-ENV-1's `--quiet` no-op-when-`TOME_LOG`-set
  semantics will be release-noted in CHANGELOG.md (DOC-02 precedent).
- **Don't push to remote** until session work is complete (CLAUDE.md
  Session Completion). Phase 18 is two plans → likely two PRs.
- **GSD workflow enforcement** active. All file edits go through `/gsd:*`
  commands.

## Standard Stack

### Core (Plan A wires)

| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| `tracing` | `0.1` (currently `0.1.44`) | Span + structured-event macros (`info!`, `warn!`, `debug!`, `info_span!`, `instrument`). | De facto Rust observability substrate; tokio-rs project; powers virtually every modern async Rust app. PROJECT.md locked it as default. |
| `tracing-subscriber` | `0.3` (currently `0.3.23`) | Subscriber implementation: `EnvFilter` directive parsing + `fmt` text formatter. | The sibling crate that turns `tracing` events into actual output. Needed for ANY use of `tracing` in a CLI. |

### Scaffolded only (Plan A adds to Cargo.toml; no wiring)

| Library | Version | Purpose | When Wired |
|---------|---------|---------|------------|
| `tracing-error` | `0.2` (currently `0.2.1`) | `ErrorLayer` adds tracing-span context to `anyhow::Error` backtraces. | Phase 19 (possibly OBS-06 doctor categorization) or v1.0 prep. |
| `tracing-appender` | `0.2` (currently `0.2.5`) | Non-blocking writer + rolling file appender for log file sinks. | v1.0 (Tauri IPC log capture). |

### Feature flags required

- `tracing-subscriber` MUST be enabled with features `["env-filter", "fmt"]`.
  - `env-filter` pulls in `matchers` + `regex-automata` for directive parsing.
  - `fmt` is on by default; explicit listing is documentation-grade safety.
  - The default feature set also includes `ansi`, `tracing-log`, `smallvec`,
    `std`. These are fine to keep for tome (we want ANSI on TTY, no harm
    in `tracing-log` even though tome doesn't use the `log` facade).
- `tracing` keeps defaults (`std` + `attributes`). The `attributes` feature
  pulls in `tracing-attributes` which provides `#[tracing::instrument]` — even
  if Plan B prefers explicit `info_span!`, keep `attributes` on so the
  attribute is available for future ergonomic refactors.
- `tracing-error` MUST be added with `default-features = false` initially
  (scaffold-only per D-SUB-3). The default `traced-error` feature is harmless
  but adds a (small) `Backtrace` capture cost.
- `tracing-appender` defaults are fine; no file sink runs.

### Alternatives Considered

| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| `tracing` | `log` + `env_logger` | Simpler, no spans. PROJECT.md explicitly chose `tracing` for v1.0 Tauri IPC needs. Locked via D-SUB-1. |
| `tracing-subscriber` `fmt` | `tracing-bunyan-formatter` / `tracing-tree` / `tracing-forest` | All add structure/visual hierarchy. Out of scope for a substrate-replacement phase. |
| `EnvFilter` | `Targets` filter (compile-time enumerated) | `Targets` requires baking the module list. `EnvFilter` is the standard `RUST_LOG`-style UX users already know. |

**Installation (recommended Cargo.toml diff):**

```toml
# Add to root Cargo.toml [workspace.dependencies] (alphabetical insertion
# point right after `tabled = ...`, before `terminal_size = ...`):

tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter", "fmt"] }
tracing-error = { version = "0.2", default-features = false }
tracing-appender = "0.2"

# Add to crates/tome/Cargo.toml [dependencies] (alphabetical, after
# `tabled.workspace = true`):

tracing.workspace = true
tracing-subscriber.workspace = true
tracing-error.workspace = true
tracing-appender.workspace = true
```

**Version verification (2026-05-12 against crates.io):**

| Crate | Currently published | MSRV | Verified via |
|-------|---------------------|------|--------------|
| `tracing` | 0.1.44 | 1.65.0 | `cargo info tracing` |
| `tracing-subscriber` | 0.3.23 | 1.65.0 | `cargo info tracing-subscriber` |
| `tracing-error` | 0.2.1 | 1.63.0 | `cargo info tracing-error` |
| `tracing-appender` | 0.2.5 | 1.63.0 | `cargo info tracing-appender` |

Pin to caret ranges (`"0.1"`, `"0.3"`, `"0.2"`, `"0.2"`) per the rest of
tome's workspace deps. These are stable, mature crates owned by `tokio-rs`
and follow `0.x.y` semver-compatible patch bumps.

**Cargo.lock impact:** Currently zero `tracing*` entries (`grep "^name = \"tracing\"" Cargo.lock` returns nothing). Plan A's lockfile diff is purely additive.

## Subscriber Initialization

### Location

**Recommendation:** `crates/tome/src/main.rs`, between `Cli::parse()` and
`tome::run(cli)`. This is consistent with the Claude's-discretion
recommendation in CONTEXT.md, AND it preserves the typed-error printers
(`LintFailed`, `MigrationPartialOrFailed`) — which are intentionally raw
`eprintln!` per D-OUT-1's "main.rs error printer stays raw" carve-out.

```rust
// main.rs — after ctrlc handler, before tome::run(cli):

let cli = tome::cli::Cli::parse();

// Install tracing subscriber. Failure is non-fatal — we fall back to
// no-subscriber (events drop silently) and warn on stderr. The match
// on tome::run(cli) below still uses raw eprintln! for typed errors
// (D-OUT-1 carve-out for main.rs error printer).
if let Err(e) = tome::tracing_init::install(cli.log_level()) {
    eprintln!("warning: tracing init failed: {e:#} — continuing without structured logging");
}

match tome::run(cli) { ... }
```

### `tracing_init.rs` module (new, Plan A)

```rust
//! Tracing subscriber installation. Single entry point: `install(LogLevel)`.
//!
//! Wires `tracing-subscriber` per Phase 18 D-OUT-2..D-OUT-4 + D-ENV-1..D-ENV-2:
//! - Writer: stderr (D-OUT-2)
//! - Format: compact, no target, info-level prefix suppressed (D-OUT-4)
//! - Span events: CLOSE only (D-SPAN-2)
//! - Filter: TOME_LOG env wins; falls back to LogLevel-derived directive (D-ENV-1)
//! - Default level: info (D-ENV-2)

use anyhow::Result;
use tracing_subscriber::{
    EnvFilter,
    fmt::{self, format::FmtSpan},
    util::SubscriberInitExt,
};

use crate::cli::LogLevel;

/// Install the global tracing subscriber. Idempotent in spirit — uses
/// `try_init` so repeated calls inside the same process (e.g., re-entrant
/// integration test harnesses) silently no-op rather than panic.
pub fn install(level: LogLevel) -> Result<()> {
    let filter = EnvFilter::try_from_env("TOME_LOG")
        .unwrap_or_else(|_| EnvFilter::new(level.directive()));

    fmt()
        .with_writer(std::io::stderr)
        .with_target(false)
        .with_span_events(FmtSpan::CLOSE)
        .compact()
        .with_env_filter(filter)
        .try_init()
        .map_err(|e| anyhow::anyhow!("tracing subscriber init failed: {e}"))?;

    Ok(())
}
```

### Idempotency strategy

`tracing_subscriber::fmt::SubscriberBuilder::try_init()` returns
`Result<(), TryInitError>`; the error variant fires when a global subscriber
is already installed. Per the docs:

> "This method returns an error if a global default subscriber has already
> been set, or if a `log` logger has already been set (when the
> 'tracing-log' feature is enabled)."

Three scenarios:

1. **Production binary (one `main` invocation):** `try_init` succeeds on
   first call. No second call exists. Map error → stderr warning + continue.
2. **Integration tests via `assert_cmd`:** Each test spawns a fresh `tome`
   subprocess. `try_init` is the first call in each subprocess; no conflict
   possible. **Naturally isolated** — no special handling needed.
3. **Unit tests in `crates/tome/src/`:** All run in one binary process. If
   any unit test calls `tracing_init::install(...)`, subsequent calls in the
   same process binary will hit `try_init`'s already-set error. Plan A
   currently does NOT call `install` from unit tests (the proof migration
   in `reconcile.rs` exercises `tracing::info!` macros, which fire events
   into whatever subscriber happens to be installed — `None` is fine; events
   silently drop). **No mitigation needed today.** See "Test Isolation"
   below for the recommended guard.

The `tracing-log` feature is enabled by default in `tracing-subscriber`'s
default features. tome doesn't use the `log` crate facade anywhere
(`rg "^use log::|::log::" crates/` returns zero hits), so the
"`log` logger already set" branch of `try_init` is unreachable.

## LogLevel → EnvFilter Mapping (Concrete Code)

Add to `crates/tome/src/cli.rs::LogLevel`:

```rust
impl LogLevel {
    // ... existing methods ...

    /// Map verbosity to an EnvFilter directive string. Single source of
    /// truth for the flag → tracing level translation per D-ENV-3.
    /// Called by `tracing_init::install` when `TOME_LOG` is unset.
    pub fn directive(self) -> &'static str {
        match self {
            Self::Quiet => "warn",
            Self::Normal => "info",
            Self::Verbose => "debug",
        }
    }
}
```

The exhaustive-match keeps `LogLevel::ALL` discipline intact (compile-time
exhaustiveness is enforced by the existing `_log_level_exhaustiveness`
sentinel + `LogLevel::ALL.len() == 3` const_assert).

**Precedence pattern (D-ENV-1):**

```rust
let filter = EnvFilter::try_from_env("TOME_LOG")
    .unwrap_or_else(|_| EnvFilter::new(level.directive()));
```

`EnvFilter::try_from_env` returns `Result<Self, FromEnvError>` and fails
when the env var is **unset OR contains invalid directives**. The
`unwrap_or_else` branch covers both: an unset `TOME_LOG` and a malformed
`TOME_LOG=garbage` both fall back to the flag-derived level. On malformed
input we silently fall back rather than emit a warning — this matches
`RUST_LOG`'s established UX in cargo/tokio.

**Trade-off acknowledged:** A user setting `TOME_LOG=` (empty string) gets
the flag fallback. A user setting `TOME_LOG=info,garbage` ALSO gets the
flag fallback (the whole string is invalid even though `info` alone parses).
If the planner wants to be stricter, they can call
`EnvFilter::try_new(std::env::var("TOME_LOG").unwrap_or_default())` and
emit a warning on parse failure — but the cost is yet-another stderr line
on a feature most users won't touch. Recommend the lossless fall-back.

**`TOME_LOG` directive examples** to put in `--help` long_about:

- `TOME_LOG=debug` — verbose globally
- `TOME_LOG=tome::sync=debug,tome::reconcile=info` — verbose specifically
  in sync; info elsewhere. (This is the literal example from the OBS-02
  success criterion.)
- `TOME_LOG=warn,tome::library=debug` — warn globally except verbose
  consolidate.

## Output Channel Split (Full Call-Site Classification)

Total in-scope sites: **114 across 5 modules** (per `rg -c "eprintln!|println!"`).

Recommendation column key:
- `tracing::info!` — log-like; default visible at info; goes to stderr.
- `tracing::warn!` — warning chatter; visible at all levels except quiet+TOME_LOG=error.
- `tracing::debug!` — verbose-only chatter.
- **`STDOUT (keep)`** — user-facing summary/table/prompt; DO NOT touch.
- **`STDERR (keep)`** — ceremonial-but-not-log-like (e.g., interactive
  pre-prompt context, dry-run banner).

### `reconcile.rs` (6 sites — Plan A proof module)

| Line | Current | Recommendation | Notes |
|------|---------|----------------|-------|
| 512 | `println!("  • {}: {} → {}", ...)` (drift diff line) | **STDOUT (keep)** | Per-drift detail rendered before apply. Part of `apply_drift_and_missing`; user-facing. Stays direct `println!`. |
| 521 | `println!("  • {} (missing — installing)", ...)` | **STDOUT (keep)** | Same as line 512 — per-missing apply-time detail. |
| 540 | `eprintln!("warning: post-update hash_directory(...) failed: ...")` | `tracing::warn!` | True warning, hash failure mid-apply. |
| 551 | `eprintln!("warning: post-update current_version(...) failed: ...")` | `tracing::warn!` | True warning, version readback failure. |
| 584 | `eprintln!("warning: post-install current_version(...) failed: ...")` | `tracing::warn!` | True warning, post-install readback failure. |
| 644 | `eprintln!("warning: {} has local edits; skipping reconcile this sync ...")` | `tracing::warn!` | True warning, edit-in-library skip. |
| 745–752 | `render_summary` prints `format_summary` to stdout via `print!` | **STDOUT (keep) — but caller relocates per D-ENV-4** | The function itself stays printing to stdout; the **call site** at `lib.rs:1557` is deleted and the function is invoked from inside `render_sync_report` per OBS-05. |

**Note:** `reconcile.rs` `format_summary` returns a `String` (the testable
seam). `render_summary` prints it. Both stay on stdout because they are
user-facing summary output, not log-like chatter. The OBS-05 relocation
moves WHEN it prints (into the final summary block), not WHERE (still stdout).

### `library.rs` (6 sites — consolidate logic)

| Line | Current | Recommendation | Notes |
|------|---------|----------------|-------|
| 161 | `eprintln!("warning: {} is a v0.9-shape symlink for managed skill ...")` | `tracing::warn!` | Defensive warning; matches v0.10 migration guidance. |
| 193 | `eprintln!("warning: {} exists but is not in the manifest, skipping")` | `tracing::warn!` | Skip-with-warning collision. |
| 209 | `eprintln!("warning: {} exists but is not in the manifest, skipping")` | `tracing::warn!` | Same shape as line 193. |
| 263 | `eprintln!("warning: v0.1 symlink target for '{}' is gone, copying from current source")` | `tracing::warn!` | Legacy v0.1.x migration warning. |
| 301 | `eprintln!("warning: {} exists but is not in the manifest, skipping")` | `tracing::warn!` | Same shape as line 193. |
| 352 | `eprintln!("warning: skipping symlink inside skill dir: {}")` | `tracing::warn!` | Inside copy_dir_recursive; rare but valid warning. |
| **NEW** | (none today) | `tracing::info!(skill=%name, directory=%dir, cause=%cause, "re-emitted")` | **OBS-04 emission site** — `result.updated += 1` branches in `consolidate_managed` (line 190) and `consolidate_local` (line 271, 298). See "Cause Attribution" below. |

### `distribute.rs` (3 sites)

| Line | Current | Recommendation | Notes |
|------|---------|----------------|-------|
| 100 | `eprintln!("warning: failed to remove legacy symlink {}: {}")` | `tracing::warn!` | I/O warning in circular-symlink-cleanup branch. |
| 132 | `eprintln!("warning: {} is a foreign symlink ...; skipping. Pass --force ...")` | `tracing::warn!` | HARD-09 foreign-symlink protection. Multi-line; tracing's compact format flattens to one line — acceptable (the hint still names `--force`). |
| 147 | `eprintln!("warning: {} exists in target and is not a symlink, skipping")` | `tracing::warn!` | Foreign-file warning. |
| **NEW** | (none today) | `tracing::info!(skill=%name, directory=%dir, cause=%cause, "re-emitted")` | **OBS-04 emission site** — the `result.changed += 1` branch on line 164. See "Cause Attribution" below. |

### `cleanup.rs` (2 sites)

| Line | Current | Recommendation | Notes |
|------|---------|----------------|-------|
| 30 | doc-comment reference to `eprintln!` discipline | N/A | Doc-only; update comment to reflect tracing in Plan B. |
| 479 | `eprintln!("warning: could not canonicalize library path {} ...")` | `tracing::warn!` | Canonicalize fallback warning. |
| 135–238 | `render_cleanup_buckets` writes via `&mut impl Write` | **STDERR (keep)** | The renderer takes a writer; `lib.rs:1768` routes that writer to `std::io::stderr().lock()`. Stays direct write. User-facing ceremonial output, not log-like. |
| 244–278 | `render_distribution_cleanup_failures` writes via `&mut impl Write` | **STDERR (keep)** | Same shape — ceremonial. |

### `discover.rs` (warning surface, not direct emission)

`discover.rs` aggregates warnings into a `Vec<String>` that the caller
drains. The **emission site** is in `lib.rs::sync` at line 1604:

```rust
for w in &warnings {
    eprintln!("warning: {}", w);  // → tracing::warn!("{}", w);
}
```

The aggregation pattern stays (clean separation of "produce warnings" from
"render them"); only the print site migrates. No source line changes inside
`discover.rs` itself for Phase 18.

### `lib.rs` (97 sites — the sweep)

Full enumeration is the planner's job in Plan B; the heuristic is:

**`tracing::info!`** — info-level chatter at default verbosity:
- 1499 `println!("  {} Pulled changes from remote", ...)` → `info!`
- 1515 `eprintln!("info: pre-sync snapshot created")` → `info!`
- 1832 `println!("  {} Pushed to remote", ...)` → `info!`

**`tracing::warn!`** — all `eprintln!("warning: ...")` sites in sync flow:
- 1506, 1519, 1576, 1808, 1835, 2046, 2195, 2385, 2386 — straight conversion.
- 1604 (the discover warnings loop) per above.

**`tracing::debug!`** — verbose-only "doing X..." status lines:
- 1584, 1594, 1616, 1638, 1684, 1704, 1717 — these are gated by `if verbose
  { eprintln!(...) }` today; D-OUT-3 says spans handle the same need at
  `debug` level, so these become **either** `debug!` lines **or** they're
  deleted entirely because the span CLOSE event already prints the step
  name + timing. Recommend deletion (trust the span) for the seven
  "Resolving/Discovering/Consolidating/etc..." lines and conversion to
  `debug!` for the "Found N skills" and "Skipping directory '...'" lines
  that carry payload the span doesn't.

**STDOUT (keep)** — user-facing summary lines:
- 189 `println!("tome {}", ...)` — version output.
- 258, 262, 267, 1499 (the `Pulled changes` line in `tome backup`-style
  commands stays as info — but the literal output stays on stdout for
  consistency with cleanup buckets? **Decision needed.** Recommend
  re-check by the planner against output discipline contract — the
  pull/push lines are arguably ceremonial; they could stay `println!` on
  stdout or move to `info!` on stderr. D-OUT-2's contract is "diagnostic
  → stderr"; pulled-from-remote is information about the sync, not log-
  like → stays `println!`).
- 1610, 1651, 1655, 1662, 1665, 2003–2032 — `render_sync_report` body —
  **STDOUT (keep)**. This IS the user-facing summary table per OBS-01 carve-out.
- 1832, 2072, 2081, 2118–2120 — list/status table output — **STDOUT (keep)**.
- 2323, 2325, 2340, 2365, 2368, 2379 — completions / config / git-init
  prints — **STDOUT (keep)** (ceremonial command output).

**STDERR (keep)** — wizard chrome / pre-flow context:
- 215, 216, 656, 672, 674, 793, 808, 810, 894, 968, 998 — wizard
  flows (per HARD-15 these are intentionally stderr; they're around
  dialoguer prompts). `wizard.rs` is OUT OF SCOPE per D-OUT-1, but several
  wizard call sites live inside `lib.rs::run`'s init dispatch. Treat them
  as the equivalent of wizard chrome and keep them on `eprintln!`.

**Planner action (Plan B Task 1):** produce the verbatim line-by-line
table for all 97 sites with the column ["migrate to tracing::X" | "keep as
println!" | "keep as eprintln!"]. The heuristic above resolves ~80% of
sites; the planner audits the remaining ~20% individually.

### Out-of-scope modules (do NOT touch in Phase 18)

Per D-OUT-1 (doc-enforced):

- `wizard.rs` — dialoguer prompts + interactive chrome.
- `browse/*` — ratatui TUI owns the screen.
- `status.rs` — `tabled` summary tables (ceremonial).
- `doctor.rs` — `tabled` + JSON renderer (Phase 19 OBS-06 may touch its
  diagnostic surface separately).
- `lint.rs` — frontmatter validation output (consumed by editors).
- `main.rs` typed-error downcasts (lines 27–35) — these stay raw because
  they print AFTER tracing-init failure paths.

## Span Shape (Concrete Code)

Per D-SPAN-1, the tree is one top span + 5 step spans. Per D-SPAN-2,
events fire on CLOSE only.

### Top-level + 5 step spans

```rust
// lib.rs::sync — opening of the function:

use tracing::info_span;

let _sync_span = info_span!("sync", dry_run = dry_run, force = force).entered();

// ... pre-sync setup (lockfile load, machine prefs, etc.) ...

// 0. Resolve git directories
let _git_span = info_span!("discover").entered();  // NOTE: "discover" step
// wraps both git-resolve AND discover_all per D-SPAN-1's 5-step shape.
// Planner picks whether git-resolve is inside the discover span or a
// separate (un-spanned) prelude. Recommend: inside discover span.
let resolved_git_paths = resolve_git_directories(...);
// ...
let skills = discover::discover_all(...)?;
drop(_git_span);  // close span → fires CLOSE event with time.busy/time.idle.

// 1. Reconcile
{
    let _span = info_span!("reconcile").entered();
    let report = reconcile::reconcile_lockfile(...)?;
    // ... apply edit decisions, etc.
}

// 2. Consolidate
{
    let _span = info_span!("consolidate").entered();
    let (consolidate_result, mut manifest) = library::consolidate(...)?;
}

// 3. Distribute
{
    let _span = info_span!("distribute").entered();
    for (name, dir_config) in config.distribution_dirs() {
        // ... per-directory distribute loop ...
    }
}

// 4. Cleanup
{
    let _span = info_span!("cleanup").entered();
    let cleanup_result = cleanup::cleanup_library(...)?;
    // ... distribution-cleanup loop, render buckets, etc.
}
```

**Naming note:** OBS-03 success criterion lists 5 step names. Reconcile
today RUNS BEFORE discover/consolidate/distribute/cleanup in `lib.rs::sync`
(see line 1538-1571 — reconcile is the first pipeline step). The phase
description's order `(discover, reconcile, consolidate, distribute,
cleanup)` is the user-facing mental model, not the call order. The span
NAMES match the success criterion; the chronological order in the trace
output reflects actual execution. Planner verifies the success criterion
is satisfied by name presence, not call ordering.

### `elapsed_ms` field — IMPORTANT FINDING

The success criterion text says "with an `elapsed_ms` field on span close".
**The tracing-subscriber 0.3 builder does NOT emit a field literally named
`elapsed_ms`.** With `FmtSpan::CLOSE` + the default fmt formatter:

> "An event will be synthesized when a span closes. If timestamps are
> enabled for this formatter, the generated event will contain fields
> with the span's _busy time_ (the total time for which it was entered)
> and _idle time_ (the total time that the span existed but was not
> entered)."
> — `tracing-subscriber` 0.3.23 docs (verified 2026-05-12)

The auto-emitted field names are **`time.busy`** and **`time.idle`**, with
human-readable values like `time.busy=12.3ms time.idle=4.5ms` in the
default formatter's output. The `time.busy` field is what tome cares about
(actual work time); `time.idle` is near-zero for sync's synchronous code.

**Three options for the planner:**

1. **Accept the auto-emitted fields.** Document in 18-PLAN.md: "the OBS-03
   `elapsed_ms` requirement is satisfied by `time.busy` (auto-emitted by
   `FmtSpan::CLOSE`); see `tracing-subscriber` docs." Zero extra code.
   **RECOMMENDED.**

2. **Manually record `elapsed_ms` as a field.** Capture `Instant::now()`
   at span entry; record the delta on span exit. Adds boilerplate at every
   step. Loses the `time.idle` distinction (not useful here, but doctrinally
   correct).

3. **Custom `FormatEvent` impl that renames `time.busy` → `elapsed_ms`.**
   D-OUT-4 explicitly defers this (custom FormatEvent declined unless
   compact knobs don't suffice). Most invasive option.

**Recommendation: option 1.** Document the field-name mapping in the plan
so reviewers don't grep for the wrong string. The OBS-03 acceptance test
should look for `time.busy=` (or a regex that captures the auto-emitted
timing), not `elapsed_ms=`.

### Why `info_span!` + `.entered()` over `#[tracing::instrument]`?

D-SPAN-1 / Claude's discretion recommends explicit `info_span!`. Reasons:

- The 5 step boundaries don't map 1:1 to function boundaries. `discover`
  wraps git-resolve + discover_all; `cleanup` wraps `cleanup_library` +
  `cleanup_disabled_from_target` + `render_cleanup_buckets` +
  `render_distribution_cleanup_failures`. `#[instrument]` requires a
  function to attach to.
- Existing `if verbose { eprintln!("Resolving git sources..."); }` lines
  delete cleanly when the span CLOSE event prints `discover` with timing
  (D-ENV-3: no info-level line disappears, but verbose-level "step starting"
  text becomes redundant once the span CLOSE event provides the same info).
- Span lifetime is the `let _span = info_span!(...).entered();` lexical
  scope — readable, no annotation magic.

## Cause Attribution (OBS-04 — Enum + Emission Sites)

### `change_cause.rs` (new module, Plan B Task 1)

```rust
//! `ChangeCause` — typed reason a skill was re-emitted by consolidate or
//! distribute. OBS-04 (Phase 18) success criterion locks the four
//! user-facing strings:
//!
//! - "hash changed"            — content_hash mismatch
//! - "previously failed"       — last sync recorded a failure for this skill
//! - "newly added"             — first time tome sees the skill
//! - "directory now allowed"   — was disabled, now enabled in machine.toml
//!
//! Greppability matters: `grep "cause=hash changed" ~/.tome/sync.log` is
//! the user's debugging workflow. Renaming any string is a BREAKING change.

use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChangeCause {
    HashChanged,
    PreviouslyFailed,
    NewlyAdded,
    DirectoryNowAllowed,
}

impl ChangeCause {
    /// POLISH-04 exhaustiveness sentinel — compile fails if a new variant
    /// is added without updating ChangeCause::ALL.
    pub const ALL: [Self; 4] = [
        Self::HashChanged,
        Self::PreviouslyFailed,
        Self::NewlyAdded,
        Self::DirectoryNowAllowed,
    ];
}

#[allow(dead_code)]
fn _change_cause_exhaustiveness(c: ChangeCause) {
    match c {
        ChangeCause::HashChanged => {}
        ChangeCause::PreviouslyFailed => {}
        ChangeCause::NewlyAdded => {}
        ChangeCause::DirectoryNowAllowed => {}
    }
}

const _: () = assert!(
    ChangeCause::ALL.len() == 4,
    "ChangeCause::ALL must list every variant",
);

impl fmt::Display for ChangeCause {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            Self::HashChanged => "hash changed",
            Self::PreviouslyFailed => "previously failed",
            Self::NewlyAdded => "newly added",
            Self::DirectoryNowAllowed => "directory now allowed",
        };
        f.write_str(s)
    }
}
```

This mirrors `LogLevel` (cli.rs:14-54), `FailureKind` (remove.rs), and
`MigrationFailureKind` (migration_v010.rs:53-76) verbatim. No new pattern.

### Where each variant fires (decision branches per D-SPAN-4)

`library.rs::consolidate` (and its helpers `consolidate_managed` /
`consolidate_local`):

| Variant | Branch | Local state available? |
|---------|--------|-------------------------|
| `NewlyAdded` | `DestinationState::Empty` branch (line 200, `result.created += 1`) AND `else` of the "not in manifest" branch (line 308, `result.created += 1`). | Yes — `manifest.get(name).is_none()`. |
| `HashChanged` | `Content changed or force` branch (line 182-190, `result.updated += 1`) AND its symmetric line 287-298. | Yes — explicit `entry.content_hash != content_hash` comparison just above. |
| `DirectoryNowAllowed` | NEW path: needs new tracking. The current consolidate loop doesn't know whether a skill was previously disabled-then-enabled; that signal lives in `machine_prefs` diff between two syncs (NOT in manifest). **OPEN — see Open Questions below.** |
| `PreviouslyFailed` | NEW path: needs new tracking. There is no per-skill "last sync failed" field in the manifest today. **OPEN — see Open Questions below.** |

`distribute.rs::distribute_to_directory`:

| Variant | Branch | Local state available? |
|---------|--------|-------------------------|
| `NewlyAdded` | `target_link.is_symlink()` false branch flowing into the symlink-create site (line 155-164, `result.changed += 1`), when the target previously had no symlink. | Yes — `target_link.is_symlink()` is false. |
| `HashChanged` | Same `result.changed += 1` site, but when an existing symlink was stale (line 110-145, "Update stale link" branch). | Yes — `symlink_points_to(...)` was false but the link existed. |
| `DirectoryNowAllowed` | The `disabled` filter check at line 80-83 used to fail; this sync it passed. **OPEN — same tracking problem.** |
| `PreviouslyFailed` | NEW path — no per-distribution-failure record exists. **OPEN.** |

### Emission shape (locked by D-SPAN-4)

```rust
// At the decision branch — e.g., inside consolidate_managed's
// "content changed or force" arm:

let cause = ChangeCause::HashChanged;
tracing::info!(
    skill = %name,
    directory = %dir_name,  // adapted to the call site's local variable
    cause = %cause,         // % = Display (user-facing string, no enum noise)
    "re-emitted",
);
```

The literal log line shape (per the compact formatter, no target, info
prefix suppressed per D-OUT-4):

```
  re-emitted skill=foo directory=local-skills cause=hash changed
```

`grep "cause=" sync-output.txt` returns every re-emit reason.

### Open Questions on cause variants

**`PreviouslyFailed`** — the OBS-04 vocabulary includes this string, but
the current manifest schema (`SkillEntry` in manifest.rs) does NOT track
per-skill failure state. Three options:

1. **Add `last_sync_failed: bool` to `SkillEntry`.** Persisted in
   `.tome-manifest.json`. Schema bump (additive — `#[serde(default)]`
   keeps backward compat). Set by any branch that catches an error in
   consolidate/distribute. Next sync reads it and emits the cause. **Largest
   scope.**
2. **Detect at runtime via `result.skipped > 0` from the previous sync.**
   Needs access to the previous `SyncReport`, which sync currently does
   not persist anywhere. Would need a `LastSyncReport` file. **Scope creep.**
3. **Demote `PreviouslyFailed` to a deferred cause variant.** Land
   `ChangeCause::{HashChanged, NewlyAdded, DirectoryNowAllowed}` in Phase 18
   and document in 18-deferred-items.md that `PreviouslyFailed` requires
   manifest schema extension (Phase 19 OBS-06 polish or v0.12). Define the
   enum variant + Display impl now, but `_phantom` it (no emission site).

**`DirectoryNowAllowed`** — similar problem. Today the loop just skips
disabled skills (`if !machine_prefs.is_skill_allowed(...) { continue; }`).
The signal "was disabled last sync, now enabled" requires comparing two
machine_prefs snapshots. Options:

1. **Diff machine_prefs from previous sync.** Needs persisted state.
2. **Detect via "distribution exists in library but no symlink in target"
   semantics.** Existing skill (in manifest), allowed now, but no symlink
   → was disabled last time. Possibly inferable from current state alone.
3. **Defer like `PreviouslyFailed`.**

**Recommendation to planner:** Land all 4 variants on the enum + Display
impl (preserves OBS-04 grep vocabulary). Wire emission sites for
`HashChanged` and `NewlyAdded` immediately (full local state available).
For `DirectoryNowAllowed`, try option 2 (infer from "in manifest, allowed
now, target has no symlink") in distribute — that single signal is locally
computable. For `PreviouslyFailed`, **defer with a tracked
`18-deferred-items.md` entry** unless the planner finds a cheap signal.
The OBS-04 success criterion says "one of [4 strings]" — leaving one
variant unfired is a defensible read of the spec ("the cause that fires
IS one of these four"); a stricter read demands all four fire eventually.
Plan B's task list must call this out explicitly so /gsd:verify-work can
adjudicate.

## Reconcile Breakdown Line (OBS-05)

### `ReconcileReport` field availability

`reconcile.rs:100-116`:

```rust
#[derive(Debug, Default)]
pub struct ReconcileReport {
    pub matches: usize,
    pub drift: Vec<Classified>,
    pub vanished: Vec<Classified>,
    pub missing: Vec<Classified>,
    pub edited: Vec<Edited>,
    pub install_failures: Vec<InstallFailure>,
    pub apply_skipped: bool,
    pub edit_decisions: Vec<EditDecision>,
}
```

All four counts are `Vec::len()` / `usize` reads:

- `match` count: `report.matches`
- `drift` count: `report.drift.len()`
- `vanished` count: `report.vanished.len()`
- `missing-from-machine` count: `report.missing.len()`

No new computation, no new field on `ReconcileReport`. D-ENV-4 confirms.

### Where the line renders

Today's flow (lines 1539-1568 of `lib.rs::sync`):

```rust
let report = reconcile::reconcile_lockfile(...)?;
if !quiet {
    reconcile::render_summary(&report, quiet);  // <-- DELETE this call
}
// apply edit decisions; render install failures; etc.
```

After OBS-05 (per D-ENV-4):

```rust
let report = reconcile::reconcile_lockfile(...)?;
// reconcile::render_summary call deleted here.
// The report is THREADED into render_sync_report at the end.

// ... sync continues, reconcile output is no longer printed mid-pipeline ...
```

And the final summary block at `lib.rs:1801`:

```rust
let report = SyncReport {
    consolidate: consolidate_result,
    distributions: distribute_results,
    cleanup: cleanup_result,
    removed_from_targets,
    reconcile: reconcile_report,  // NEW: thread the reconcile report through
};

if !quiet {
    render_sync_report(&report);
}
```

### `render_sync_report` extension (concrete)

The current implementation (lines 2002-2037) renders:
1. "Sync complete" header
2. Library / distribution counters
3. Cleaned-from-library line (if > 0)
4. Cleaned-from-targets line (if > 0)

After OBS-05 + D-ENV-4 orchestration recommendation:

```rust
fn render_sync_report(report: &SyncReport) {
    println!("{}", style("Sync complete").green().bold());
    println!("  Library: ...");
    for dr in &report.distributions { println!("  {}: ...", ...); }
    if report.cleanup.removed_from_library > 0 { ... }
    if report.removed_from_targets > 0 { ... }

    // OBS-05: reconcile classification line (D-ENV-4) — IMMEDIATELY ABOVE
    // the cleanup buckets per the success criterion.
    if let Some(rr) = &report.reconcile {
        println!(
            "  reconcile: {} {} match · {} {} drift · {} {} vanished · {} {} missing-from-machine",
            style("✓").green(),
            rr.matches,
            style("⚠").yellow(),
            rr.drift.len(),
            style("⚠").yellow(),
            rr.vanished.len(),
            style("⚠").yellow(),
            rr.missing.len(),
        );

        // Then the relocated drift/vanished detail (was at lib.rs:1557 via
        // format_summary). Two options per CONTEXT.md D-ENV-4:
        // (a) lift the drift-detail loop here inline, OR
        // (b) call reconcile::format_summary(rr) and print the result —
        //     simpler, preserves the format/render split, but format_summary
        //     today includes the classification line we already wrote
        //     (collision). Recommend writing a new helper:
        //     `reconcile::format_classification_detail(rr) -> String` that
        //     returns ONLY the per-drift `• X: 1.0 → 2.0` lines + per-vanished
        //     warning lines, NO classification line.
    }

    // Cleanup buckets called from here (orchestration recommendation):
    // render_cleanup_buckets(stderr, &report.cleanup.bucket_a, ...);
    // ... but the existing call is at lib.rs:1768, on stderr. Two options:
    // (1) keep the lib.rs:1768 call and just precede it with the
    //     classification line printed inside render_sync_report;
    //     simpler diff but split ownership.
    // (2) move the lib.rs:1768 stderr writes into render_sync_report.
    //     Cleaner ownership, larger diff. Claude's discretion recommends (2)
    //     for "centralizes ordering contract." Planner picks.
}
```

**Planner note on the orchestration recommendation:** Claude's discretion
recommends `render_sync_report` orchestrate BOTH the reconcile-summary AND
cleanup-buckets rendering (centralizes ordering). The straightforward way
is option (2) above. The downside is that `render_sync_report` then must
accept the `stderr` writer for cleanup-buckets — its signature widens.
Recommend the planner accept the widening; it's a one-time refactor that
makes the "what gets printed first?" question reviewable in one function.

**Caveat (not just Claude noise):** The OBS-05 success criterion says
"included in the final summary block (printed at `info` level — visible by
default)." If `render_sync_report` keeps using `println!` (stdout), then
the line is on stdout, not at info level. **This is fine** per CONTEXT.md
D-OUT-1: the user-facing summary tables stay on stdout via direct
`println!`. The success criterion's "at info level — visible by default"
phrasing in this context means "visible at the default verbosity," not
"emitted via `tracing::info!`." Plan B should document this explicitly so
a future reader doesn't try to migrate `render_sync_report` itself to
`tracing::info!`. The summary stays stdout `println!`.

## Test Isolation Strategy

### Integration tests (`tests/cli_*.rs` — `assert_cmd`)

Each `Command::cargo_bin("tome").assert()` invocation **spawns a fresh
subprocess.** Global subscriber state is per-process; new process = new
subscriber state. **Naturally isolated.** No special handling required.

**Snapshot-test impact:** Existing snapshot tests like
`tests/cli_sync.rs` use `insta::assert_snapshot!("...", stdout)` — they
capture **stdout only**. Per the migration plan, all `tracing::*!` output
goes to **stderr** (D-OUT-2). Stdout snapshots are therefore byte-near-
identical to today's output. **Estimated stdout snapshot diffs: zero**
for sync/status/list/init flows.

**Stderr containment assertions** like
`Command::cargo_bin("tome").assert().stderr(predicates::str::contains("..."))`
WILL likely still pass because tracing's compact format preserves the
warning/info text mostly verbatim — but the planner should re-snapshot or
re-run each test in `tests/cli_*.rs` that has a `.stderr(...)` assertion
after the migration to catch any wrinkle.

The handful of stderr containment assertions to audit:

```bash
rg -l "\.stderr\(" crates/tome/tests/cli_*.rs
```

Plan B Task X should enumerate the affected tests upfront.

### Unit tests (in-process)

The bulk of in-scope module unit tests (reconcile.rs `mod tests`,
library.rs `mod tests`, cleanup.rs `mod tests`, distribute.rs `mod tests`)
assert on **state changes** (manifest mutations, return-value fields, etc.)
not on captured tracing output. They will continue to pass without any
tracing-subscriber installed — emitted events drop into the void.

**One existing test guards content of `cleanup.rs` SOURCE for a forbidden
phrase** (`cleanup_module_source_does_not_contain_forbidden_phrase` at
line 1252). It runs `include_str!("cleanup.rs")` and asserts the trigger
phrase from Phase 16 D-UX01-3 is absent. The migration replaces
`eprintln!` with `tracing::warn!` but keeps the string content; this test
continues to pass byte-for-byte.

### Recommended guard pattern (if unit tests later capture tracing output)

```rust
// Pattern for any future test that wants to assert on tracing output:
use tracing::subscriber::with_default;
use tracing_subscriber::fmt;

#[test]
fn my_test_capturing_tracing() {
    let (sub, handle) = /* build a test subscriber writing to Vec<u8> */;
    with_default(sub, || {
        // Code under test — events here go to `sub`.
    });
    // Assert against handle's captured output.
}
```

`with_default` is **thread-local scoped** and does NOT conflict with a
global subscriber. Phase 18 does not need this pattern today, but the
planner can include a one-line note in 18-PLAN.md so a future test author
doesn't try to call `tracing_init::install` in test setup.

### Pitfall: `cargo test` running in parallel

`cargo test` defaults to multi-threaded execution. If one test thread
calls `tracing_init::install`, all OTHER test threads in the same process
share that global subscriber. This is fine for tests that don't assert
on output. The danger is "I installed a subscriber writing to my test's
Vec<u8>, but another test's events also landed there."

**Mitigation:** If any future test ever needs to install a subscriber,
use `with_default` (scoped) NOT `try_init` (global). For Phase 18, no
test installs a subscriber, so the danger is theoretical.

## Project Constraints (from CLAUDE.md — Re-stated for Quick Reference)

- **Quality gates:** `make ci` (= fmt-check + clippy `-D warnings` + tests)
  MUST pass before commit. Plan A's and Plan B's PRs both run CI.
- **Rust edition 2024, MSRV 1.85.0.** All four new crates support 1.65+.
- **Strict clippy.** Adding 4 new dependencies adds zero clippy warnings
  in tome's own code; verify after crate addition.
- **Non-interactive shell forms** for any cleanup operations (`cp -f`, `rm -rf`).
- **GSD workflow enforced.** Every edit goes through `/gsd:*`.
- **Push before session end** is mandatory.

## Runtime State Inventory

> Skipped — Phase 18 is purely an in-process code change. No databases,
> no live services, no OS-registered state, no secrets/env-var renames,
> no build artifacts impacted by the migration. Cargo.lock IS regenerated
> on first `cargo build` after Cargo.toml diff — that's the entire
> external footprint. Documented as "none in any category."

## Common Pitfalls

### Pitfall 1: Subscriber double-init in tests

**What goes wrong:** A unit test calls `tracing_init::install(...)`. A
second unit test in the same binary calls the same. The second call's
`try_init` returns `Err(TryInitError)`; tome's `install` propagates that
as `anyhow::Error`. Test panics on `.unwrap()` or asserts on success.

**Why it happens:** `tracing` subscribers are process-global. `cargo test`
runs all unit tests in one binary per crate.

**How to avoid:** Don't call `install` from unit tests. Use
`with_default(subscriber, || { ... })` for any test that needs a custom
subscriber. Currently Phase 18 has no such tests.

**Warning signs:** Any future test that does
`tracing_init::install(LogLevel::Normal).unwrap()` in setup.

### Pitfall 2: ANSI escape codes in CI output

**What goes wrong:** `tracing-subscriber`'s default `fmt` has ANSI on
(via the `ansi` feature, which is in default features). On a CI runner
without a TTY, ANSI escapes appear as raw `\u{1b}[...m` in log capture,
making the log unreadable.

**Why it happens:** The default ANSI behavior is unconditional; it doesn't
auto-detect TTY like `console`/`indicatif` do.

**How to avoid:** Add `.with_ansi(std::io::stderr().is_terminal())` to
the subscriber builder. (See `IsTerminal` already imported in lib.rs.)
This matches tome's existing TTY-aware UX in `console::style` callsites.

**Warning signs:** GitHub Actions log shows `[2m[33mwarning:[0m` instead of
plain `warning:`.

```rust
// In tracing_init::install — recommended addition:
use std::io::IsTerminal;

fmt()
    .with_writer(std::io::stderr)
    .with_ansi(std::io::stderr().is_terminal())  // <-- ADD
    .with_target(false)
    // ...
```

### Pitfall 3: stdout vs. stderr mismatch on the OBS-05 line

**What goes wrong:** Reader sees "info level" in OBS-05's success criterion
and migrates `render_sync_report`'s `println!` to `tracing::info!`. Result:
the summary block lives on stderr (D-OUT-2 routes tracing to stderr). User
piping `tome sync > out.log` no longer captures the summary.

**Why it happens:** Conflation of "visible at default verbosity" with
"emitted via tracing::info!".

**How to avoid:** Document explicitly in 18-PLAN.md: `render_sync_report`
stays `println!`. The OBS-05 line is `println!` on stdout. The "at info
level" phrasing in the success criterion refers to the equivalent
verbosity, not the emission channel.

**Warning signs:** PR review comment "shouldn't render_sync_report use
`tracing::info!`?" Answer: no, per D-OUT-1 user-facing summary tables stay
on stdout.

### Pitfall 4: Span lifetime + `?` early returns

**What goes wrong:** A `?` early-return inside a step span body skips the
explicit `drop(_span)`. The span still drops correctly (RAII), but if the
planner writes:

```rust
let _span = info_span!("consolidate").entered();
let result = library::consolidate(&skills, paths, dry_run, force)?;  // early return!
// ... do more work ...
drop(_span);
```

The span drops on early-return — fine. But if the planner uses
`.in_scope(|| { ... })` inside which a `?` returns from the closure not
from `sync`, the closure's error type bubbles up correctly only with
explicit Result-returning closures.

**Why it happens:** Standard Rust ownership; not a tracing bug, but a
common stumble.

**How to avoid:** Prefer `let _span = info_span!(...).entered();` blocks
delimited by `{}` so RAII matches lexical scope. Avoid `.in_scope()`
inside step blocks unless explicitly handling closure-vs-outer Result.

**Warning signs:** Compiler error "expected `()`, found `Result<...>`"
inside an `.in_scope()` closure.

### Pitfall 5: `time.busy` vs. `elapsed_ms` literal mismatch

**What goes wrong:** Planner writes a verification test:
`assert!(output.contains("elapsed_ms="))`. Test fails because
`FmtSpan::CLOSE` emits `time.busy=` not `elapsed_ms=`.

**Why it happens:** Success criterion text uses `elapsed_ms` conceptually;
actual emitted field is `time.busy`.

**How to avoid:** See "Span Shape" / "elapsed_ms field" section above.
Document field-name mapping in 18-PLAN.md. Acceptance regex:
`r"time\.busy=\d+(\.\d+)?(ns|µs|ms|s)"`.

**Warning signs:** OBS-03 verification step says "grep elapsed_ms".

### Pitfall 6: `console::style(...)` color codes inside `tracing::info!`

**What goes wrong:** Migrating a current `eprintln!("warning: {}",
console::style(msg).yellow())` site to `tracing::warn!("{}",
console::style(msg).yellow())`. The yellow code embeds in the log message;
when ANSI is off (CI, log-to-file), the user sees `\u{1b}[33m...\u{1b}[0m`
surrounding the message.

**Why it happens:** `console::style` emits ANSI unconditionally (or per
its own TTY detection, which doesn't see tracing's writer).

**How to avoid:** When migrating, drop the `console::style` wrapper from
the log message string and rely on tracing's level-based coloring (warn =
yellow in tracing's default ANSI). The tracing formatter auto-colors by
level when ANSI is on. Saves a layer of confusion. Apply only to migrated
lines, NOT to render-to-stdout user-facing strings (those keep
`console::style` because the renderers control their own ANSI gate).

**Warning signs:** Log line in CI looks like `^[[33mwarning: foo^[[0m`
even though `with_ansi(false)` was set.

### Pitfall 7: `EnvFilter::try_from_env` failing on empty `TOME_LOG=`

**What goes wrong:** User sets `TOME_LOG=` (intent: "no override").
`try_from_env` doesn't strip empty values; it tries to parse and fails
the "no directives provided" check. Falls back to flag-derived level.
Acceptable, but surprising if the user expected `TOME_LOG=""` to be a
no-op.

**How to avoid:** Document in `--help`: `TOME_LOG=""` is treated as unset.
Or: explicitly check `std::env::var("TOME_LOG").ok().filter(|s|
!s.is_empty())` before passing to `EnvFilter`. Recommend doing nothing
(matches `RUST_LOG=""` behavior in cargo).

## Code Examples (Verified Patterns)

### Subscriber init (complete reference)

```rust
// crates/tome/src/tracing_init.rs

use std::io::IsTerminal;

use anyhow::{Context, Result};
use tracing_subscriber::{
    EnvFilter,
    fmt::{self, format::FmtSpan},
    util::SubscriberInitExt,
};

use crate::cli::LogLevel;

pub fn install(level: LogLevel) -> Result<()> {
    let filter = EnvFilter::try_from_env("TOME_LOG")
        .unwrap_or_else(|_| EnvFilter::new(level.directive()));

    fmt()
        .with_writer(std::io::stderr)
        .with_ansi(std::io::stderr().is_terminal())
        .with_target(false)
        .with_span_events(FmtSpan::CLOSE)
        .compact()
        .with_env_filter(filter)
        .try_init()
        .map_err(|e| anyhow::anyhow!("tracing subscriber init failed: {e}"))?;

    Ok(())
}
```

### `info_span!` step block (lib.rs::sync)

```rust
{
    let _span = tracing::info_span!("reconcile").entered();
    let report = reconcile::reconcile_lockfile(...)?;
    // ... handle report ...
}  // span drops here, emits CLOSE event with time.busy/time.idle.
```

### OBS-04 emission (library.rs::consolidate_local, after content-changed branch)

```rust
// Decision branch: content changed or force re-copy.
if !dry_run {
    if dest.is_dir() {
        std::fs::remove_dir_all(dest)?;
    }
    copy_dir_recursive(&skill.path, dest)?;
}
record_in_manifest(manifest, skill, content_hash.clone());
result.updated += 1;

// OBS-04 emission — decision-site, no struct field.
tracing::info!(
    skill = %skill.name,
    directory = %skill.source_name,
    cause = %ChangeCause::HashChanged,
    "re-emitted",
);
```

### OBS-05 line in render_sync_report

```rust
if let Some(rr) = &report.reconcile {
    println!(
        "  reconcile: {} {} match · {} {} drift · {} {} vanished · {} {} missing-from-machine",
        style("✓").green(), rr.matches,
        style("⚠").yellow(), rr.drift.len(),
        style("⚠").yellow(), rr.vanished.len(),
        style("⚠").yellow(), rr.missing.len(),
    );
}
```

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| `log` + `env_logger` crate | `tracing` + `tracing-subscriber` | ~2019 onwards; tracing 1.0 stable on `tokio-rs` | Spans, structured fields, async-aware contexts. Industry default for Rust CLIs + servers. |
| Manual `Instant::now()` timing logs | `info_span!` + `FmtSpan::CLOSE` | Built-in since tracing-subscriber 0.2 | Auto-emitted `time.busy`/`time.idle` on span close. Zero boilerplate. |
| Free-form `cause: &str` log fields | Typed enum + `impl Display` (POLISH-04 pattern) | Tome-internal convention (HARD-07 / FailureKind / MigrationFailureKind / LogLevel) | Refactor-safe; exhaustive-match guards against silent string drift. |

**Deprecated/outdated:**

- `tracing-subscriber` `pretty()` formatter is server-log-ish (multi-line);
  not a fit for CLI default. `compact()` is the CLI consensus.
- `env_logger` is fine for `log`-based projects but doesn't compose with
  `tracing`'s span model.

## Open Questions

1. **Where does `PreviouslyFailed` get its signal?**
   - What we know: no `last_sync_failed` field on `SkillEntry` today.
   - What's unclear: cheapest way to capture the signal without a manifest
     schema bump.
   - Recommendation: defer the emission site to Phase 19 polish or v0.12.
     Land the enum variant + Display impl in Phase 18 (preserves the OBS-04
     grep vocabulary). Document in `18-deferred-items.md`. The OBS-04
     success criterion's wording "one of [4 strings]" is satisfiable with
     3 wired and 1 enum-defined-but-never-fired.

2. **Where does `DirectoryNowAllowed` get its signal?**
   - What we know: machine_prefs filtering happens at line 80-83 of
     distribute.rs (`machine_prefs.is_skill_allowed(...)`).
   - What's unclear: how to detect "was excluded last sync, now included"
     from in-process state alone.
   - Recommendation: in distribute.rs at the "was excluded, target has no
     symlink yet" detection point, emit `DirectoryNowAllowed`. Locally
     computable from existing state (`target_link.is_symlink()` false +
     skill is in manifest + skill is now allowed). Verify the inference
     during plan implementation; if any branch contradicts, defer with
     `PreviouslyFailed`.

3. **Should `render_sync_report` orchestrate cleanup-bucket rendering?**
   - What we know: D-OUT-2 routes cleanup buckets to stderr;
     `render_sync_report` today writes to stdout.
   - What's unclear: whether the orchestration recommendation (Claude's
     discretion) is worth the signature widening + stderr-writer threading.
   - Recommendation: Plan B Task X — Spike: try the centralized
     orchestration; if it forces awkward writer threading, fall back to
     keeping the cleanup-bucket call at `lib.rs:1768` and the new OBS-05
     line at the top of `render_sync_report`. The success criterion only
     requires order (reconcile line above cleanup buckets), not common
     ownership.

4. **Is there a `time.busy` field on `FmtSpan::CLOSE` when fmt's
   `with_timer` is off?**
   - What we know: docs say "If timestamps are enabled for this formatter,
     the generated event will contain fields with the span's busy time..."
   - What's unclear: does `fmt::compact()` enable the system timer by
     default? Empirically yes (compact format includes a timestamp prefix
     by default), so `time.busy`/`time.idle` SHOULD fire.
   - Recommendation: write a quick Plan A throwaway test that runs `tome
     sync --verbose` against an empty config and `grep "time.busy"` the
     captured stderr to confirm. If absent, add `.with_timer(...)` or set
     a timer explicitly. Low-effort confidence boost before Plan B
     commits to the span shape.

5. **Does `tracing-error` add unwanted weight if it's only scaffolded?**
   - What we know: `tracing-error` 0.2.1 has `default-features = ["traced-
     error"]` which captures `std::backtrace::Backtrace` lazily.
   - What's unclear: adding the crate without using `ErrorLayer` means the
     `Backtrace` capture is dormant. Confirm zero runtime cost.
   - Recommendation: add with `default-features = false` to be safe per
     D-SUB-3 scaffold-only stance. Minor — affects code-size + zero
     runtime — but matches the "scaffold but DON'T wire" decision.

## Environment Availability

> Skipped — Phase 18 is purely an in-process code change. The only external
> dependencies are `cargo` (present, MSRV 1.85.0) and the four new crates
> from crates.io (resolved on first `cargo build`).

## Sources

### Primary (HIGH confidence)

- `tracing-subscriber` 0.3.23 docs — `fmt::SubscriberBuilder` —
  https://docs.rs/tracing-subscriber/0.3.23/tracing_subscriber/fmt/struct.SubscriberBuilder.html
  (verified `with_writer`, `compact`, `with_target`, `with_span_events`)
- `tracing-subscriber` 0.3.23 docs — `EnvFilter` —
  https://docs.rs/tracing-subscriber/0.3.23/tracing_subscriber/filter/struct.EnvFilter.html
  (verified `try_from_env`, `try_new`, `new`, default-when-unset behavior)
- `tracing-subscriber` 0.3.23 docs — `SubscriberInitExt::try_init` —
  https://docs.rs/tracing-subscriber/0.3.23/tracing_subscriber/util/trait.SubscriberInitExt.html
  (verified Result return + already-set error)
- Tome internal: `crates/tome/src/cli.rs:14-54` — existing `LogLevel` enum
  + `ALL` exhaustiveness sentinel + `const_assert!`
- Tome internal: `crates/tome/src/reconcile.rs:100-116` —
  `ReconcileReport` struct with `matches`, `drift`, `vanished`, `missing`
  fields already populated
- Tome internal: `crates/tome/src/lib.rs:1539-1568, 1801, 2002-2037` —
  current reconcile rendering + sync_report shape
- `cargo info tracing` / `tracing-subscriber` / `tracing-error` /
  `tracing-appender` 2026-05-12 — current versions + MSRV

### Secondary (MEDIUM confidence)

- `tracing-subscriber` `FmtSpan::CLOSE` field naming (`time.busy`/
  `time.idle`) — confirmed via two independent sources (docs.rs
  documentation excerpt + WebSearch result). Specific format string
  (`time.busy=12.3ms`) is not formally documented; behavior matches
  community examples.
- Test isolation pattern (`with_default` vs `try_init`) —
  https://users.rust-lang.org/t/how-to-test-tracing-setup/119713 +
  https://github.com/tokio-rs/tracing/discussions/2736

### Tertiary (LOW confidence — flag for validation during Plan A)

- Exact ANSI behavior of `with_ansi(bool)` interacting with `compact()` —
  Plan A's proof migration should verify a single warn message renders
  cleanly in both `TERM=xterm-256color` and `TERM=dumb` (CI) environments.
- Whether `fmt::compact()` enables a timestamp prefix by default — assumed
  yes; if no, `time.busy`/`time.idle` may not emit. Open Question #4
  resolves this with a quick spike in Plan A.

## Metadata

**Confidence breakdown:**

- Standard stack (crate selection + versions): **HIGH** — Context7-equivalent
  doc verification + cargo info confirmation.
- Subscriber init shape: **HIGH** — Exact builder methods verified against
  0.3.23 docs.
- LogLevel → EnvFilter mapping: **HIGH** — Pattern verified, precedence
  semantics confirmed.
- Output channel split (call-site table): **HIGH** for the 17 sites in
  reconcile/library/distribute/cleanup; **MEDIUM** for the 97 lib.rs sites
  (heuristic-driven enumeration — planner audits individually in Plan B).
- Span shape (info_span!, FmtSpan::CLOSE): **HIGH** for the shape;
  **MEDIUM** for the literal `time.busy` field name verification (one
  empirical check recommended in Plan A).
- Cause attribution: **HIGH** for `HashChanged` + `NewlyAdded` (locally
  computable); **LOW** for `PreviouslyFailed` + `DirectoryNowAllowed`
  (flagged as Open Questions).
- Reconcile breakdown: **HIGH** — `ReconcileReport` fields confirmed
  present.
- Test isolation: **HIGH** for integration (process-isolated); **HIGH** for
  current unit-test impact (zero — no tracing-output assertions).
- Pitfalls: **HIGH** — all enumerated pitfalls are concrete and verified.

**Research date:** 2026-05-12
**Valid until:** 2026-06-12 (30 days; the tracing crate family is mature
and stable; only an unlikely 0.x major bump invalidates findings)
