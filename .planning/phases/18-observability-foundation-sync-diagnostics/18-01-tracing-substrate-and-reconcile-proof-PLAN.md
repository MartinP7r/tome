---
phase: 18-observability-foundation-sync-diagnostics
plan: 01
type: execute
wave: 1
depends_on: []
files_modified:
  - Cargo.toml
  - crates/tome/Cargo.toml
  - crates/tome/src/lib.rs
  - crates/tome/src/main.rs
  - crates/tome/src/cli.rs
  - crates/tome/src/tracing_init.rs
  - crates/tome/src/reconcile.rs
autonomous: true
requirements:
  - OBS-01
  - OBS-02

must_haves:
  truths:
    - "Running `cargo build` against the workspace succeeds with the four new `tracing*` workspace dependencies declared and `crates/tome` depending on them."
    - "`tome --version` (or any subcommand) installs a global `tracing` subscriber before `tome::run(cli)` executes; subscriber-init failure prints a non-fatal warning to stderr and the command continues."
    - "Default verbosity emits `tracing::info!` events to stderr; `--verbose` raises filter to `debug`; `--quiet` lowers to `warn`; `TOME_LOG=tome::sync=debug,tome::reconcile=info` (or any other EnvFilter directive) replaces the flag-derived level entirely (per D-ENV-1)."
    - "Every `eprintln!(\"warning: ...\")`/`println!` site in `crates/tome/src/reconcile.rs` that is NOT the `format_summary`/`render_summary` stdout summary lines or the per-drift/per-missing apply-time stdout details has been replaced with `tracing::warn!`/`tracing::info!` per the call-site table in 18-RESEARCH.md §Output Channel Split (reconcile.rs)."
    - "`cargo test -p tome -- reconcile` continues to pass byte-for-byte (unit tests assert on state mutations, not on stderr text)."
    - "`cargo run -- status` and `cargo run -- init --dry-run --no-input` produce stdout byte-identical to v0.10.0 (no snapshot diffs in `cli_status__*.snap` / `cli_init*.snap` if such snapshots exist for `--dry-run --no-input`)."
  artifacts:
    - path: "Cargo.toml"
      provides: "Workspace dependency declarations for tracing/tracing-subscriber (wired) and tracing-error/tracing-appender (scaffolded per D-SUB-3)."
      contains: "tracing"
    - path: "crates/tome/Cargo.toml"
      provides: "Crate-level `.workspace = true` references for the four new deps."
      contains: "tracing.workspace"
    - path: "crates/tome/src/tracing_init.rs"
      provides: "New module exposing `pub fn install(level: LogLevel) -> anyhow::Result<()>` that wires the global subscriber (stderr writer, compact format, no target, FmtSpan::CLOSE, EnvFilter with TOME_LOG precedence)."
      contains: "pub fn install"
    - path: "crates/tome/src/cli.rs"
      provides: "`LogLevel::directive(self) -> &'static str` mapping Quiet→\"warn\", Normal→\"info\", Verbose→\"debug\" (D-ENV-3)."
      contains: "pub fn directive"
    - path: "crates/tome/src/main.rs"
      provides: "Subscriber install call site between `Cli::parse()` and `tome::run(cli)`, with non-fatal-on-failure stderr fallback."
      contains: "tracing_init::install"
    - path: "crates/tome/src/lib.rs"
      provides: "`pub mod tracing_init;` declaration in lib.rs's module list so `main.rs` can call `tome::tracing_init::install`."
      contains: "pub mod tracing_init"
  key_links:
    - from: "crates/tome/src/main.rs"
      to: "crates/tome/src/tracing_init.rs::install"
      via: "function call between Cli::parse() and tome::run(cli)"
      pattern: "tome::tracing_init::install\\(cli\\.log_level"
    - from: "crates/tome/src/tracing_init.rs::install"
      to: "tracing_subscriber EnvFilter + fmt subscriber installed globally"
      via: "fmt().with_writer(stderr).with_target(false).with_span_events(FmtSpan::CLOSE).compact().with_env_filter(filter).try_init()"
      pattern: "FmtSpan::CLOSE.*compact|EnvFilter::try_from_env"
    - from: "crates/tome/src/reconcile.rs (eprintln warning sites)"
      to: "global tracing subscriber"
      via: "tracing::warn! / tracing::info! macros"
      pattern: "tracing::warn!|tracing::info!"
---

<objective>
Land the `tracing` substrate (OBS-01 substrate side + OBS-02 full) and exercise it end-to-end through `reconcile.rs` as the proof module per locked decision D-SUB-2 (Plan A = substrate + 1 proof module).

This plan adds four workspace dependencies (`tracing`, `tracing-subscriber` wired; `tracing-error`, `tracing-appender` scaffolded per D-SUB-3), creates `crates/tome/src/tracing_init.rs` with a single `install(LogLevel)` entry point, wires `LogLevel::directive` per D-ENV-3, calls `install` from `main.rs` between `Cli::parse()` and `tome::run(cli)`, and migrates the four `eprintln!("warning: ...")` sites in `reconcile.rs` (lines 540, 551, 584, 644 per 18-RESEARCH.md §Output Channel Split) to `tracing::warn!`. The stdout `println!` lines in `reconcile.rs` (drift diff lines 512, missing-installing line 521, plus `format_summary`/`render_summary`) STAY ON STDOUT — they are user-facing summary output per RESEARCH §Output Channel Split, not log-like chatter.

Purpose: deliver the substrate that Plan 18-02 sweeps the remaining 4 modules onto. Reconcile is small (6 sites), isolated, and already has a `format_summary`/`render_summary` separation — making it the locked proof choice per D-SUB-2 (Plan A's recommended module).

Output: four crates in Cargo.toml; `tracing_init.rs` module; subscriber installation in `main.rs`; `LogLevel::directive`; reconcile.rs `eprintln!("warning: ...")` → `tracing::warn!` migration. The `reconcile::render_summary` call site at `lib.rs:1557` is NOT touched here — its relocation to `render_sync_report` is Plan 18-02's job (D-ENV-4). The reconcile `format_summary`/`render_summary` functions themselves stay unchanged in this plan.
</objective>

<execution_context>
@$HOME/.claude/get-shit-done/workflows/execute-plan.md
@$HOME/.claude/get-shit-done/templates/summary.md
</execution_context>

<context>
@.planning/PROJECT.md
@.planning/ROADMAP.md
@.planning/STATE.md
@.planning/REQUIREMENTS.md
@.planning/phases/18-observability-foundation-sync-diagnostics/18-CONTEXT.md
@.planning/phases/18-observability-foundation-sync-diagnostics/18-RESEARCH.md

@Cargo.toml
@crates/tome/Cargo.toml
@crates/tome/src/main.rs
@crates/tome/src/lib.rs
@crates/tome/src/cli.rs
@crates/tome/src/reconcile.rs

<interfaces>
<!-- Key types and call sites the executor needs. Extracted from the codebase. -->

From crates/tome/src/cli.rs (existing — DO NOT redesign):

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum LogLevel {
    Quiet,
    #[default]
    Normal,
    Verbose,
}

impl LogLevel {
    pub const ALL: [Self; 3] = [Self::Quiet, Self::Normal, Self::Verbose];
    pub fn is_verbose(self) -> bool { /* ... */ }
    pub fn is_quiet(self) -> bool { /* ... */ }
}

// The exhaustive-match sentinel + const_assert!(LogLevel::ALL.len() == 3) MUST
// continue to compile after `directive()` is added.

impl Cli {
    pub fn log_level(&self) -> LogLevel { /* existing */ }
}
```

From crates/tome/src/main.rs (existing — TARGET FOR EDIT):

```rust
let cli = tome::cli::Cli::parse();

match tome::run(cli) {
    Ok(()) => ExitCode::SUCCESS,
    Err(e) => {
        if let Some(lint_failed) = e.downcast_ref::<tome::LintFailed>() {
            eprintln!("error: {lint_failed}");
            return ExitCode::FAILURE;
        }
        // ... typed-error downcasts stay on raw eprintln! per D-OUT-1 carve-out ...
    }
}
```

From crates/tome/src/reconcile.rs (call sites to migrate — verified line numbers
from 18-RESEARCH.md §Output Channel Split):

- Line 540: `eprintln!("warning: post-update hash_directory(...) failed: ...")`  → `tracing::warn!`
- Line 551: `eprintln!("warning: post-update current_version(...) failed: ...")` → `tracing::warn!`
- Line 584: `eprintln!("warning: post-install current_version(...) failed: ...")` → `tracing::warn!`
- Line 644: `eprintln!("warning: {} has local edits; skipping reconcile this sync ...")` → `tracing::warn!`

Lines 512 / 521 (apply-time per-drift and per-missing stdout `println!`) STAY AS-IS — they are
user-facing summary output, not log-like chatter.

Lines 745–752 (`render_summary` printing `format_summary` to stdout) STAY AS-IS in this plan.
The CALL SITE at `lib.rs:1557` invoking `reconcile::render_summary(&report, quiet)` is
NOT touched here — it is removed in Plan 18-02 per D-ENV-4. This plan leaves the function intact.
</interfaces>

</context>

<tasks>

<task type="auto" tdd="false">
  <name>Task 1: Add tracing crates to workspace + crate manifests; add LogLevel::directive</name>
  <files>Cargo.toml, crates/tome/Cargo.toml, crates/tome/src/cli.rs</files>
  <read_first>
    - Cargo.toml (current workspace.dependencies block — find alphabetical insertion point after `tabled` and before `terminal_size`)
    - crates/tome/Cargo.toml (current [dependencies] block — find alphabetical insertion point after `tabled.workspace = true`)
    - crates/tome/src/cli.rs (existing LogLevel enum at lines 14-54; preserve ALL/exhaustive-match/const_assert)
    - .planning/phases/18-observability-foundation-sync-diagnostics/18-RESEARCH.md §Standard Stack (lines 192-265) — pinned versions + feature flags
    - .planning/phases/18-observability-foundation-sync-diagnostics/18-CONTEXT.md D-SUB-3 — the four-crate scaffolded set rationale
  </read_first>
  <behavior>
    - LogLevel::Quiet.directive() returns "warn"
    - LogLevel::Normal.directive() returns "info"
    - LogLevel::Verbose.directive() returns "debug"
    - `cargo build -p tome` succeeds after the dependency additions (lockfile additive only — RESEARCH §Standard Stack confirms zero existing tracing* entries)
    - LogLevel::ALL still has length 3; existing const_assert and exhaustive-match sentinel continue to compile
  </behavior>
  <action>
    Step 1 — Add to root `Cargo.toml` `[workspace.dependencies]` block, alphabetically inserted AFTER `tabled = { version = "0.20", features = ["ansi"] }` (line 32) and BEFORE `terminal_size = "0.4"` (line 33), the four lines:

    ```toml
    tracing = "0.1"
    tracing-subscriber = { version = "0.3", features = ["env-filter", "fmt"] }
    tracing-error = { version = "0.2", default-features = false }
    tracing-appender = "0.2"
    ```

    Verified versions on crates.io 2026-05-12 (RESEARCH §Standard Stack version verification table): `tracing` 0.1.44, `tracing-subscriber` 0.3.23, `tracing-error` 0.2.1, `tracing-appender` 0.2.5. Caret pin to the minor (`"0.1"` / `"0.3"` / `"0.2"`) matches the rest of the workspace deps. MSRV 1.65 for all four is well below tome's 1.85 MSRV.

    `tracing-subscriber`'s `["env-filter", "fmt"]` features are MANDATORY: `env-filter` pulls in directive parsing (RESEARCH); `fmt` is on by default but explicit listing is documentation-grade safety. `tracing-error` uses `default-features = false` per D-SUB-3 scaffold-only stance (skip the lazy `Backtrace` capture cost — RESEARCH Open Question 5).

    Step 2 — Add to `crates/tome/Cargo.toml` `[dependencies]` block, alphabetically inserted AFTER `tabled.workspace = true` (line 27) and BEFORE `terminal_size.workspace = true` (line 28), the four lines:

    ```toml
    tracing.workspace = true
    tracing-subscriber.workspace = true
    tracing-error.workspace = true
    tracing-appender.workspace = true
    ```

    Step 3 — In `crates/tome/src/cli.rs`, INSIDE the existing `impl LogLevel` block (currently spans lines 24-39), AFTER the existing `is_quiet` method, add:

    ```rust
    /// Map verbosity to a `tracing_subscriber::EnvFilter` directive string.
    /// Single source of truth for the flag → tracing level translation per
    /// D-ENV-3 (Phase 18). Called by `tracing_init::install` when `TOME_LOG`
    /// is unset.
    pub fn directive(self) -> &'static str {
        match self {
            Self::Quiet => "warn",
            Self::Normal => "info",
            Self::Verbose => "debug",
        }
    }
    ```

    DO NOT modify `LogLevel::ALL`, the `_log_level_exhaustiveness` function, or the `const _: () = assert!(...)`. They must continue to compile unchanged (POLISH-04 pattern preserved).

    Step 4 — Add a unit test inside the existing `#[cfg(test)] mod tests` block in `crates/tome/src/cli.rs` (e.g., after `log_level_default_trait_impl_is_normal`):

    ```rust
    #[test]
    fn log_level_directive_maps_three_levels() {
        assert_eq!(LogLevel::Quiet.directive(), "warn");
        assert_eq!(LogLevel::Normal.directive(), "info");
        assert_eq!(LogLevel::Verbose.directive(), "debug");
    }
    ```

    Do NOT run `cargo fetch` or `cargo update` as a separate step; the next task's `cargo build` resolves new deps on first invocation.
  </action>
  <verify>
    <automated>cargo build -p tome 2>&amp;1 | tail -5</automated>
  </verify>
  <acceptance_criteria>
    - `rg "^tracing = \"0\\.1\"" Cargo.toml` returns exactly 1 match
    - `rg "^tracing-subscriber = " Cargo.toml | rg "features = \\[\"env-filter\", \"fmt\"\\]"` returns 1 match
    - `rg "^tracing-error = " Cargo.toml | rg "default-features = false"` returns 1 match
    - `rg "^tracing-appender = \"0\\.2\"" Cargo.toml` returns 1 match
    - `rg "^tracing\\.workspace = true" crates/tome/Cargo.toml` returns 1 match
    - `rg "^tracing-subscriber\\.workspace = true" crates/tome/Cargo.toml` returns 1 match
    - `rg "^tracing-error\\.workspace = true" crates/tome/Cargo.toml` returns 1 match
    - `rg "^tracing-appender\\.workspace = true" crates/tome/Cargo.toml` returns 1 match
    - `rg "pub fn directive\\(self\\) -> &'static str" crates/tome/src/cli.rs` returns 1 match
    - `cargo build -p tome` exits 0
    - `cargo test -p tome --lib cli::tests::log_level_directive_maps_three_levels` exits 0
    - `cargo test -p tome --lib cli::tests::log_level_all_array_lists_every_variant` still exits 0 (POLISH-04 sentinel preserved)
  </acceptance_criteria>
  <done>Four `tracing*` workspace deps declared and resolved; `LogLevel::directive` exists and returns the locked three-string mapping; `cargo build -p tome` succeeds.</done>
</task>

<task type="auto" tdd="false">
  <name>Task 2: Create crates/tome/src/tracing_init.rs and wire it into main.rs</name>
  <files>crates/tome/src/tracing_init.rs, crates/tome/src/lib.rs, crates/tome/src/main.rs</files>
  <read_first>
    - crates/tome/src/main.rs (current entry — install lands between `Cli::parse()` and `tome::run(cli)`, AFTER the ctrlc handler)
    - crates/tome/src/lib.rs lines 1-100 (module declaration list — `pub mod tracing_init;` lands here alphabetically)
    - .planning/phases/18-observability-foundation-sync-diagnostics/18-RESEARCH.md §Subscriber Initialization (lines 267-364) + §Code Examples / Subscriber init (lines 1222-1254) — full builder code
    - .planning/phases/18-observability-foundation-sync-diagnostics/18-RESEARCH.md §Common Pitfalls Pitfall 2 (ANSI in CI) — `with_ansi(std::io::stderr().is_terminal())` is REQUIRED
    - .planning/phases/18-observability-foundation-sync-diagnostics/18-CONTEXT.md D-OUT-2/D-OUT-3/D-OUT-4 (sink + format decisions) and D-ENV-1/D-ENV-2 (TOME_LOG precedence)
  </read_first>
  <behavior>
    - Module compiles standalone (`pub mod tracing_init;` in lib.rs)
    - `install(LogLevel::Normal)` succeeds first time it is called in a process
    - Repeated `install` calls within the same process return `Err` from `try_init` and the caller's wrapper (main.rs) emits a stderr warning + continues — they do not panic or abort
    - Subscriber writes to stderr (not stdout)
    - Subscriber's `FmtSpan::CLOSE` is enabled so spans emit `time.busy`/`time.idle` on close (per RESEARCH §elapsed_ms FINDING — these are the auto-emitted timing field names; "elapsed_ms" in the success criterion is conceptual)
    - When `TOME_LOG` env var is unset, filter level derives from `LogLevel::directive()`
    - When `TOME_LOG` env var is set to a valid `EnvFilter` directive, it overrides the flag-derived level entirely (D-ENV-1)
    - When `TOME_LOG` is malformed, the subscriber falls back to flag-derived level silently (per RESEARCH §LogLevel → EnvFilter Mapping)
  </behavior>
  <action>
    Step 1 — Create new file `crates/tome/src/tracing_init.rs` with exact content (per RESEARCH §Code Examples / Subscriber init lines 1222-1254 + Pitfall 2 ANSI gate):

    ```rust
    //! Tracing subscriber installation. Single entry point: `install(LogLevel)`.
    //!
    //! Wires `tracing-subscriber` per Phase 18 decisions:
    //! - Writer: stderr (D-OUT-2 — matches Unix convention + Phase 16 D-UX01-4)
    //! - ANSI: gated on stderr.is_terminal() (RESEARCH Pitfall 2 — CI safety)
    //! - Format: compact, no target, info-level prefix suppressed (D-OUT-4)
    //! - Span events: CLOSE only — auto-emits `time.busy`/`time.idle` fields
    //!   (D-SPAN-2; "elapsed_ms" in OBS-03 success criterion is conceptual —
    //!   auto-emitted timing fields satisfy it; see 18-RESEARCH.md §elapsed_ms
    //!   FINDING and Pitfall 5 for grep regex)
    //! - Filter: TOME_LOG env wins; falls back to LogLevel-derived directive
    //!   (D-ENV-1)
    //! - Default level: info (D-ENV-2)
    //!
    //! Idempotency: `try_init` returns `Err` if a global subscriber is already
    //! installed. We propagate that as `anyhow::Error`; the caller in `main.rs`
    //! emits a stderr warning and continues — events drop silently rather than
    //! the process aborting.

    use std::io::IsTerminal;

    use anyhow::Result;
    use tracing_subscriber::{
        EnvFilter,
        fmt::{self, format::FmtSpan},
        util::SubscriberInitExt,
    };

    use crate::cli::LogLevel;

    /// Install the global tracing subscriber. Idempotent in spirit — repeated
    /// calls inside the same process return an `Err` that the caller may
    /// downgrade to a non-fatal warning.
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

    Step 2 — In `crates/tome/src/lib.rs`, locate the `pub mod` declarations near the top of the file (search for `pub mod ` to find the alphabetically-ordered list). Add the line:

    ```rust
    pub mod tracing_init;
    ```

    Insert it alphabetically — between whichever existing modules sort before `tracing_init` and after (e.g., between `pub mod status;` and `pub mod update;` if present; consult the existing alphabetical order).

    Step 3 — In `crates/tome/src/main.rs`, after the ctrlc handler block (current lines 11-16) and BEFORE the `match tome::run(cli)` (current line 20), insert:

    ```rust
    // Install tracing subscriber per Phase 18 OBS-01/OBS-02. Failure is
    // non-fatal — we fall back to no-subscriber (events drop silently) and
    // warn on stderr. The typed-error downcasts below stay on raw eprintln!
    // per D-OUT-1's "main.rs error printer stays raw" carve-out — they must
    // print even if subscriber init failed.
    if let Err(e) = tome::tracing_init::install(cli.log_level()) {
        eprintln!("warning: tracing init failed: {e:#} — continuing without structured logging");
    }
    ```

    DO NOT touch the typed-error downcast match arms (`LintFailed`, `MigrationPartialOrFailed`) — they intentionally stay as raw `eprintln!` per D-OUT-1.
  </action>
  <verify>
    <automated>cargo build -p tome 2>&amp;1 | tail -5 &amp;&amp; cargo run -p tome -- --help 2>&amp;1 | tail -3</automated>
  </verify>
  <acceptance_criteria>
    - `crates/tome/src/tracing_init.rs` exists
    - `rg "pub fn install\\(level: LogLevel\\) -> Result<\\(\\)>" crates/tome/src/tracing_init.rs` returns 1 match
    - `rg "with_writer\\(std::io::stderr\\)" crates/tome/src/tracing_init.rs` returns 1 match
    - `rg "FmtSpan::CLOSE" crates/tome/src/tracing_init.rs` returns 1 match
    - `rg "EnvFilter::try_from_env\\(\"TOME_LOG\"\\)" crates/tome/src/tracing_init.rs` returns 1 match
    - `rg "with_ansi\\(std::io::stderr\\(\\)\\.is_terminal\\(\\)\\)" crates/tome/src/tracing_init.rs` returns 1 match
    - `rg "pub mod tracing_init;" crates/tome/src/lib.rs` returns 1 match
    - `rg "tome::tracing_init::install\\(cli\\.log_level\\(\\)\\)" crates/tome/src/main.rs` returns 1 match
    - `cargo build -p tome` exits 0
    - `cargo run -p tome -- --help` exits 0 and stdout is unchanged from v0.10 help text (subscriber is now installed for the command; help still prints normally to stdout)
    - `TOME_LOG=tome=debug cargo run -p tome -- --version 2>&1 | head -1` succeeds (verifies env-var path doesn't crash)
  </acceptance_criteria>
  <done>`tracing_init.rs` module exists with the locked subscriber configuration; `main.rs` calls `install` before `tome::run`; `cargo build` succeeds; `tome --help` and `tome --version` still work.</done>
</task>

<task type="auto" tdd="false">
  <name>Task 3: Migrate reconcile.rs warning sites to tracing::warn! (proof module per D-SUB-2)</name>
  <files>crates/tome/src/reconcile.rs</files>
  <read_first>
    - crates/tome/src/reconcile.rs (target file — verify lines 540, 551, 584, 644 match RESEARCH's call-site table)
    - .planning/phases/18-observability-foundation-sync-diagnostics/18-RESEARCH.md §Output Channel Split / reconcile.rs (lines 432-449) — the line-by-line recommendation table
    - .planning/phases/18-observability-foundation-sync-diagnostics/18-RESEARCH.md §Common Pitfalls Pitfall 6 (console::style inside tracing macros) — strip wrappers in migrated lines
    - .planning/phases/18-observability-foundation-sync-diagnostics/18-CONTEXT.md D-ENV-3 (no line silently disappears; reformatting OK)
  </read_first>
  <behavior>
    - Each warning that was previously `eprintln!("warning: ...")` now routes through `tracing::warn!`
    - The four `println!` summary/detail sites in reconcile.rs (lines 512, 521, 745-752 `render_summary`) stay as `println!` — STDOUT remains the channel for user-facing summary output
    - When subscriber is installed at warn or info or debug level, the four migrated warnings still appear on stderr
    - When subscriber is installed at error level (e.g., via TOME_LOG=error), the warnings are suppressed (expected; matches RUST_LOG mental model)
    - Existing `cargo test -p tome --lib reconcile::tests` continues to pass byte-for-byte (unit tests assert on state mutations, not stderr text)
  </behavior>
  <action>
    Step 1 — At the top of `crates/tome/src/reconcile.rs` (in the existing `use` block), add:

    ```rust
    use tracing::warn;
    ```

    Insert alphabetically among existing `use` lines.

    Step 2 — Migrate each of the four warning sites. The conversion pattern is:

    BEFORE:
    ```rust
    eprintln!("warning: post-update hash_directory({}) failed: {e}", ...);
    ```

    AFTER:
    ```rust
    warn!("post-update hash_directory({}) failed: {e}", ...);
    ```

    Note: drop the literal `"warning: "` prefix — `tracing::warn!` events render with the level prefix when ANSI/format dictates (per D-OUT-4: warn keeps level prefix). Per RESEARCH §Pitfall 6: if any of these sites wraps an argument in `console::style(...).yellow()` strip the wrapper — tracing handles level coloring.

    The four sites per RESEARCH §Output Channel Split / reconcile.rs:

    | Approximate line | Original | New |
    |------------------|----------|-----|
    | ~540 | `eprintln!("warning: post-update hash_directory(...) failed: ..."` | `warn!("post-update hash_directory(...) failed: ..."` |
    | ~551 | `eprintln!("warning: post-update current_version(...) failed: ..."` | `warn!("post-update current_version(...) failed: ..."` |
    | ~584 | `eprintln!("warning: post-install current_version(...) failed: ..."` | `warn!("post-install current_version(...) failed: ..."` |
    | ~644 | `eprintln!("warning: {} has local edits; skipping reconcile this sync ..."` | `warn!("{} has local edits; skipping reconcile this sync ..."` |

    Use Read to confirm exact current line content before editing; line numbers may have shifted slightly since RESEARCH was written. The signature `rg -n 'eprintln!\\("warning:' crates/tome/src/reconcile.rs` will enumerate the survivors.

    Step 3 — DO NOT touch:
    - Lines 512 and 521 (`println!` per-drift/per-missing detail lines — RESEARCH explicitly says STDOUT (keep))
    - Lines 745-752 (`render_summary` printing `format_summary` to stdout — RESEARCH explicitly says STDOUT (keep))
    - The CALL SITE at `lib.rs:1557` invoking `reconcile::render_summary` — that relocation is Plan 18-02's job per D-ENV-4. Plan 18-01 leaves it untouched.
    - The `format_summary` function body — it returns a String, stays unchanged.

    Step 4 — Run `cargo test -p tome --lib -- reconcile::tests` to confirm no regression. Run `cargo clippy --all-targets -- -D warnings -p tome` and resolve any lints (most likely zero new ones; `warn!` import is used).
  </action>
  <verify>
    <automated>rg -n 'eprintln!\\("warning:' crates/tome/src/reconcile.rs | wc -l | tr -d ' '</automated>
  </verify>
  <acceptance_criteria>
    - `rg -n 'eprintln!\\("warning:' crates/tome/src/reconcile.rs | wc -l` returns 0
    - `rg "use tracing::warn;" crates/tome/src/reconcile.rs` returns 1 match
    - `rg -c "warn!\\(" crates/tome/src/reconcile.rs` returns at least 4
    - `rg "println!" crates/tome/src/reconcile.rs | wc -l` returns ≥ 2 (the surviving drift/missing stdout details — STDOUT keep per RESEARCH)
    - `rg "reconcile::render_summary" crates/tome/src/lib.rs` returns at least 1 match (the call site at `lib.rs:1557` is NOT touched by this plan — Plan 18-02 removes it)
    - `cargo test -p tome --lib -- reconcile::tests` exits 0
    - `cargo clippy --all-targets -- -D warnings` exits 0
    - `cargo fmt -- --check` exits 0
  </acceptance_criteria>
  <done>Four `eprintln!("warning: ...")` sites in reconcile.rs migrated to `tracing::warn!`; all stdout summary lines preserved; reconcile unit tests pass; clippy/fmt clean.</done>
</task>

</tasks>

<verification>
After all 3 tasks land:

1. **Build + tests:**
   ```
   cargo fmt -- --check
   cargo clippy --all-targets -- -D warnings
   cargo test -p tome
   ```
   All three exit 0.

2. **Substrate functional check (OBS-01 + OBS-02):**
   ```
   cargo run -p tome -- --version
   TOME_LOG=debug cargo run -p tome -- --version
   cargo run -p tome -- --verbose --version
   cargo run -p tome -- --quiet --version
   ```
   All four exit 0. None panics on subscriber init.

3. **Snapshot drift check (success criterion 1 — byte-identical stdout):**
   ```
   cargo test -p tome --test cli_status
   cargo test -p tome --test cli_sync
   cargo test -p tome --test cli_list
   cargo test -p tome --test cli_doctor
   ```
   All four exit 0 with no `insta` snapshot diffs. (Plan 18-01 does not touch the in-scope sync pipeline of `lib.rs`, so existing sync snapshots should already match; this is a regression check that the subscriber install in `main.rs` does not leak any output to stdout.)

4. **Reconcile proof migration check (OBS-01 proof):**
   ```
   rg -n 'eprintln!\("warning:' crates/tome/src/reconcile.rs
   ```
   Empty output — all four warning sites are now `tracing::warn!`.
</verification>

<success_criteria>
- `tracing`, `tracing-subscriber`, `tracing-error`, `tracing-appender` are workspace + crate dependencies (OBS-01 substrate)
- `LogLevel::directive` exists and maps Quiet→"warn", Normal→"info", Verbose→"debug" (OBS-02)
- `tracing_init::install` installs the global subscriber per D-OUT-2/D-OUT-4 + D-SPAN-2 + D-ENV-1/D-ENV-2 + RESEARCH Pitfall 2 (ANSI gate)
- `main.rs` invokes `tracing_init::install` between `Cli::parse()` and `tome::run(cli)`; subscriber-init failure is non-fatal
- `crates/tome/src/reconcile.rs` has zero `eprintln!("warning: ...")` sites (4 migrated to `tracing::warn!`); zero `println!` removed (stdout summary preserved per RESEARCH)
- `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test -p tome` all pass
- Existing snapshot tests for `tome status`, `tome list`, `tome sync`, `tome init --dry-run --no-input` continue to pass byte-for-byte (stdout-only snapshots; tracing writes to stderr)
</success_criteria>

<output>
After completion, create `.planning/phases/18-observability-foundation-sync-diagnostics/18-01-SUMMARY.md` summarizing:

- Which modules + files were touched
- Versions of the four `tracing*` crates resolved into Cargo.lock
- Confirmation that `lib.rs::sync` pipeline was NOT touched (Plan 18-02's territory)
- Any reformatting deviations from RESEARCH (e.g., if `cause = %cause` field syntax needed adjustment for downstream Plan 18-02)
- Notes for Plan 18-02 (e.g., "verify `tome sync --verbose` against an empty config emits `time.busy=` on span close — RESEARCH Open Question 4 spike was deferred" if not run)
</output>
