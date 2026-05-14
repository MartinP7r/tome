# Phase 18: Observability foundation + sync diagnostics - Context

**Gathered:** 2026-05-12
**Status:** Ready for planning

<domain>
## Phase Boundary

v0.11's structured-logging substrate plus the first wave of `tome sync`
diagnostic surface. Two workstreams running in two plans:

1. **Tracing substrate (Plan A — substrate + proof module)** — Wire
   `tracing` + `tracing-subscriber` into `main.rs` / `lib.rs::run` as the
   application logging substrate. Map the existing `LogLevel` enum (HARD-07)
   into a `tracing_subscriber::EnvFilter`-shaped subscriber. Migrate ONE
   in-scope module end-to-end so the substrate is exercised before the
   sweep. `tracing-error` and `tracing-appender` enter `Cargo.toml` as
   scaffolded deps (no wiring) so v1.0's Tauri IPC layer can adopt them
   without a second `Cargo.toml` PR.

2. **Sync diagnostic surface (Plan B — migration sweep + features)** —
   Sweep the remaining 4 in-scope modules
   (`sync`/`consolidate`/`distribute`/`cleanup`/`discover`, plus the
   remaining `lib.rs::sync` chatter that Plan A didn't touch). Layer on:
   per-pipeline-step spans with `elapsed_ms` (OBS-03), change-cause
   attribution at re-emit sites (OBS-04), and the reconcile classification
   breakdown line in `render_sync_report` (OBS-05). The reconcile detail
   relocates from mid-pipeline (today's inline `reconcile::render_summary`
   at `lib.rs:1557`) into the final summary block at `lib.rs:1801`,
   immediately above the 3-bucket cleanup output (Phase 16 D-UX01-1..-4).

**In scope** (modules getting `eprintln!`/`println!` → `tracing::*!`
migrated):

- `crates/tome/src/lib.rs` — `sync` pipeline chatter (97 sites — heaviest)
- `crates/tome/src/reconcile.rs` — 6 sites; the natural Plan A proof
  candidate (small, isolated, already has a structured render flow)
- `crates/tome/src/library.rs` (consolidate) — 6 sites; OBS-04
  `ChangeCause` emission sites for re-emits
- `crates/tome/src/distribute.rs` — 3 sites; OBS-04 `ChangeCause`
  emission sites for re-emits
- `crates/tome/src/cleanup.rs` — 2 sites
- `crates/tome/src/discover.rs` — discovery chatter (warning aggregation
  via `Vec<String>` already; mostly threads through `lib.rs::sync`)

**Out of scope** (modules keeping raw stdout — drift-prevention contract,
honoured by doc per D-OUT-1):

- `crates/tome/src/wizard.rs` — `dialoguer` interactive prompts (and
  HARD-15 already routed wizard chrome to stderr; not log-like)
- `crates/tome/src/browse/*` — TUI; `ratatui` owns the screen
- `crates/tome/src/status.rs` — `tabled` summary tables (ceremonial)
- `crates/tome/src/doctor.rs` — `tabled` summary + JSON renderer.
  Phase 19's OBS-06 categorization work touches this module; tracing
  migration for doctor's *diagnostic* output (not its summary table)
  belongs there.
- `crates/tome/src/lint.rs` — frontmatter validation output (ceremonial,
  consumed by editors)
- `crates/tome/src/main.rs` — top-level error printer (typed downcasts +
  `eprintln!`). Stays raw so failures during subscriber init still print.
- All `tabled::Table` render call sites everywhere

**Out of scope** (features explicitly NOT in v0.11):

- JSON-formatted log output (`--log-format json` / `TOME_LOG_FORMAT=json`) —
  OBS-FUTURE-01; deferred until a real machine consumer exists
- OpenTelemetry export (`tracing-opentelemetry`) — OBS-FUTURE-02
- File-sink wiring of `tracing-appender` (D-SUB-3 scaffold-only)
- `tracing-error::ErrorLayer` install + `anyhow` `.in_current_span()`
  sweep (D-SUB-3 scaffold-only) — Phase 19 or v1.0 wires when there's
  a consumer
- Sub-spans per directory (in distribute) or per skill (in consolidate)
  — D-SPAN-1 keeps the tree flat (top + 5 step spans only)
- Per-target-module EnvFilter baked defaults — D-ENV-2 picked plain
  `info` global; no module-scoped tuning at compile time
- Custom `FormatEvent` impl — D-OUT-4 picked `fmt::compact()` with two
  knobs (`with_target(false)` + info-level prefix suppression). No hand-
  rolled formatter.
- Migration of out-of-scope modules listed above

</domain>

<decisions>
## Implementation Decisions

### Substrate + migration shape

- **D-SUB-1 (tracing locked, no fallback gate):** PROJECT.md's `tracing`-as-
  default decision is firm. No cost-gate moment in Phase 18 planning. The
  `log` fallback named in PROJECT.md ("if adoption cost shows up") is NOT
  exercised; if real cost surfaces during execution it becomes a deferred
  v0.12 question, not a Plan A reversal. Rationale: substrate is shipped
  in two plans (D-SUB-2); Plan A's proof module is the de-facto cost check.
  If Plan A is rough, the planner can split the Plan B sweep into per-
  module sub-plans rather than swap crates mid-phase.
- **D-SUB-2 (Plan A = substrate + 1 proof module; Plan B = sweep):**
  Two plans, not one mega-plan and not four feature-plans. Plan A:
  (a) `tracing` + `tracing-subscriber` enter `Cargo.toml`, `tracing-error`
  + `tracing-appender` enter as scaffolded deps; (b) subscriber init at
  CLI boundary using `cli.log_level()` → `EnvFilter`; (c) migrate ONE
  in-scope module fully (recommend `reconcile.rs` — small, isolated,
  already has a structured render flow ready to convert; planner's
  discretion if `lib.rs::sync` reads better as proof). Plan B: sweep the
  remaining 4 in-scope modules + OBS-03 spans + OBS-04 ChangeCause
  emission + OBS-05 reconcile-line relocation. Rationale: substrate
  review stays small + focused; the sweep parallelizes module-by-module
  if needed; landing Plan A first means Plan B can rebase on a known-
  good substrate.
- **D-SUB-3 (Cargo.toml crate set):** Add four crates as workspace deps:
  - `tracing` — core macros (`info!`, `warn!`, `debug!`, `info_span!`,
    `instrument`)
  - `tracing-subscriber` — `EnvFilter` + `fmt` layer (wired)
  - `tracing-error` — **scaffolded only.** Dep present; `ErrorLayer`
    NOT installed in Phase 18. Phase 19 OBS-06 may wire it for `tome
    doctor` span-context-on-errors; otherwise it lands in v1.0 prep.
    Rationale: v0.11 lands the full crate set so v1.0 doesn't need a
    second `Cargo.toml` PR.
  - `tracing-appender` — **scaffolded only.** Dep present; no file sink
    in Phase 18. v1.0's Tauri IPC layer wires when log capture becomes
    a real consumer. Rationale: same as `tracing-error` — single
    `Cargo.toml` PR.

### Output discipline + rendering

- **D-OUT-1 (doc-enforced scope contract):** The in-scope/out-of-scope
  module lists in `<domain>` above are the contract. CONTEXT.md +
  18-PLAN.md enumerate them. Code review catches drift. NO
  `#![deny(clippy::print_stdout, clippy::print_stderr)]` per-module
  attributes. Rationale: per-module attributes add noise and rely on
  clippy lint stability across Rust versions; reviewer discipline +
  explicit doc list has worked for prior milestones (D-UX01-4, HARD-15).
- **D-OUT-2 (sink = stderr):** Subscriber init must call
  `.with_writer(std::io::stderr)`. Matches Unix convention (stderr =
  diagnostic chatter, stdout = program output), Phase 16 D-UX01-4 (3-bucket
  cleanup → stderr), HARD-15 (wizard chrome → stderr), and the dominant
  shape of existing `eprintln!` sites. `stdout` is reserved for machine-
  readable output (current `tabled` tables, future JSON streams in
  OBS-FUTURE-01). Practical note: this is a one-line subscriber-builder
  call; cost is zero.
- **D-OUT-3 (spans verbose-only):** Default `tome sync` (info level) does
  NOT print per-step span lines. Spans render only at `debug` (i.e.
  `--verbose` or `TOME_LOG=tome::sync=debug`). Matches the OBS-03 success
  criterion wording "visible in `--verbose` text output and reachable via
  `TOME_LOG=tome::sync=debug`" verbatim. Rationale: default output stays
  calm; users opt in to per-step timing. OBS-05 reconcile breakdown is
  the only NEW info-level addition to the default `tome sync` summary;
  per-step timing stays behind the verbose gate.
- **D-OUT-4 (format = compact, no target, no info prefix):** Subscriber
  `fmt::compact()` with `.with_target(false)` (drops the module name —
  users don't care that it came from `tome::sync`) and info-level prefix
  suppressed. `warn`/`error` keep the level prefix (the user needs that
  visual differentiation). Rationale: info-level lines stay byte-close-
  to-today's `eprintln!` output; users don't see `INFO sync:` prefixes
  on every line. Pragmatic middle ground between "use the default" and
  "write a custom `FormatEvent`." If the suppression turns out to need
  format customization beyond what `fmt::compact()` builder methods can
  do, planner promotes to a hand-rolled `FormatEvent` impl as part of
  Plan A (deferred-items track this).

### Span surface + change-cause

- **D-SPAN-1 (flat span tree: top + 5 step spans):** One top-level `sync`
  span wraps the entire pipeline. Inside: `discover`, `reconcile`,
  `consolidate`, `distribute`, `cleanup` — one span per step, no nesting
  below. Matches OBS-03 success criterion verbatim ("one span per
  pipeline step (`discover`, `reconcile`, `consolidate`, `distribute`,
  `cleanup`) with an `elapsed_ms` field on span close"). NO
  `distribute_dir{name=…}` sub-span per directory; NO `consolidate_skill
  {name=…}` per skill. Rationale: O(1) span events per sync, regardless
  of library size; matches OBS-03 wording; per-directory/per-skill drill-
  down stays a deferred follow-up if a real diagnostic need surfaces.
- **D-SPAN-2 (FmtSpan::CLOSE only):** Configure `with_span_events
  (FmtSpan::CLOSE)`. One line per span on completion with `elapsed_ms`.
  NO `NEW`/`ENTER`/`EXIT` events. Rationale: matches OBS-03 success
  criterion "`elapsed_ms` field on span close"; minimal output volume;
  no entry/exit chatter from `#[instrument]` attribute boundaries.
  Trade-off: long-running spans (e.g., a 10s consolidate) are silent
  until close — acceptable for v0.11; if `tome sync` ever gets a
  progress bar overlay this gets revisited.
- **D-SPAN-3 (ChangeCause enum + ALL sentinel):** New
  `enum ChangeCause { HashChanged, PreviouslyFailed, NewlyAdded,
  DirectoryNowAllowed }` with:
  - `ChangeCause::ALL: &[ChangeCause; 4]` constant
  - Exhaustive-match sentinel function (mirrors `FailureKind::ALL`,
    `LogLevel::ALL`, `MigrationFailureKind::ALL` precedent) so a new
    variant added without updating `ALL` is a compile error
  - `const_assert!(ChangeCause::ALL.len() == 4, ...)` enforces array
    length matches variant count
  - `impl Display for ChangeCause` returning literal user-facing
    strings: `"hash changed"`, `"previously failed"`, `"newly added"`,
    `"directory now allowed"` (matches OBS-04 vocabulary verbatim)

  Rationale: typed enum + ALL discipline is the codebase's settled
  pattern for finite-vocabulary fields (POLISH-04 / FailureKind /
  LogLevel / MigrationFailureKind). Refactor-safe, greppable, and the
  compile-time exhaustiveness catches "added a fifth cause without
  surfacing it."
- **D-SPAN-4 (decision-site emission, no result-struct extension):**
  `info!` events fire at the decision branch inside `library.rs::
  consolidate` and `distribute.rs::distribute_to_directory` — wherever
  the code decides to re-copy/re-symlink a skill. The emission shape:
  ```rust
  tracing::info!(
      skill = %name,
      directory = %dir,
      cause = %cause,  // ChangeCause via Display
      "re-emitted",
  );
  ```
  NO new field on `ConsolidateResult` / `DistributeResult`. NO
  `lib.rs::sync` walking results to centrally emit. Rationale: cause is
  locally obvious in code review (sits next to the branch that picks
  it); zero plumbing across struct boundaries; consistent with the
  existing `result.updated += 1` style of incrementing counters at the
  decision site.

### Flag/env semantics + OBS-05 placement

- **D-ENV-1 (TOME_LOG wins; env overrides flag):** Subscriber init
  composes the `EnvFilter` as:
  ```rust
  let filter = EnvFilter::try_from_env("TOME_LOG")
      .unwrap_or_else(|_| EnvFilter::new(level_from_log_level(cli.log_level())));
  ```
  When `TOME_LOG` is set, it fully replaces the flag-derived level —
  `--quiet TOME_LOG=tome::sync=debug` shows `tome::sync` at debug,
  everything else at the default for an `EnvFilter` with no explicit
  default (typically `error`, but the directive language controls).
  When `TOME_LOG` is unset, the flag-derived level applies globally.
  Rationale: matches the cargo / tokio / `RUST_LOG` mental model users
  bring from other Rust CLIs; advanced users get full control via
  EnvFilter directive syntax. Trade-off (acknowledged in release notes):
  `--quiet` becomes a no-op when `TOME_LOG` is set; not surprising for
  users familiar with `RUST_LOG`-style precedence.
- **D-ENV-2 (default level = info):** No flag, no env var → `info`.
  Matches OBS-02 success criterion verbatim. Today's `eprintln!`
  chatter at info level remains visible after migration. NO baked-in
  per-target downgrades (e.g. `tome::backup=warn` defaults). If a
  module turns out to be too chatty at info, the response is to
  downgrade those specific call sites to `debug` in code, NOT to bake
  a global EnvFilter default that surprises `TOME_LOG` users.
- **D-ENV-3 (flag UX byte-near-identical):** Flag mapping:
  `--quiet` → `warn`; default → `info`; `--verbose` → `debug`. After
  migration, the lines that existing users see at each verbosity level
  still print, modulo reformatting per D-OUT-4 (no `LEVEL target:`
  prefix at info). Span CLOSE events (D-SPAN-2) are NEW lines at
  `--verbose` and are an additive change. NO lines silently disappear
  in the migration. If a line genuinely is noise (e.g., currently-info
  output that should be debug), demote it to `debug!` at the call site
  and document the demotion in 18-PLAN.md. Rationale: the migration is
  meant to be substrate-replacement, not output-redesign — the scope-
  discipline framing PROJECT.md set out at milestone start.
- **D-ENV-4 (OBS-05 line moves into render_sync_report; reconcile detail
  relocates):** Today:
  - `reconcile::render_summary` fires inline at `lib.rs:1557` and prints
    `✓ N match · ⚠ N drift · ⚠ N vanished` plus per-drift detail lines
    and per-vanished warnings.
  - `cleanup::render_cleanup_buckets` fires at `lib.rs:1768` and prints
    the 3-bucket cleanup output.
  - `render_sync_report` fires at `lib.rs:1801` and prints the final
    summary block.

  After OBS-05:
  - Delete the inline `reconcile::render_summary` call at
    `lib.rs:1557`.
  - `render_sync_report` (lib.rs:1801) emits the reconcile classification
    line `reconcile: N match · M drift · K vanished · L missing-from-
    machine` IMMEDIATELY ABOVE the per-bucket cleanup output (which
    means the cleanup output call also moves into `render_sync_report`,
    or at least the ordering between them is owned by `render_sync_
    report`'s caller).
  - The MissingFromMachine count is NEW in the classification line —
    today's `format_summary` only renders match/drift/vanished. The
    count comes from `ReconcileReport::missing: Vec<Classified>`
    (already populated; zero new computation).
  - The per-drift detail lines and per-vanished warnings need to
    relocate too. Two options for the planner: (a) lift them into
    `render_sync_report` immediately after the classification line;
    (b) keep them on `format_summary` and emit `format_summary` from
    `render_sync_report` rather than inline at line 1557. Planner
    picks; either honours the success criterion.

  Rationale: success-criterion wording explicitly says "immediately
  above the existing per-bucket cleanup summary," which is the final
  summary block, not the mid-pipeline emission site. Single user-
  facing "final summary block" reads cleaner than today's split.

### Claude's Discretion

The following are implementation details that follow established
patterns; no user input needed:

- Exact `TOME_LOG` directive examples in `--help` text (the OBS-02
  success criterion suggests `TOME_LOG=tome::sync=debug,tome::reconcile
  =info` as a documentation example — planner picks the literal string).
- Plan A's choice of proof module — `reconcile.rs` is recommended
  (small, isolated, format_summary already structured) but `lib.rs::sync`
  is acceptable if the planner reads it as the more end-to-end-validating
  choice. NOT both; one proof module is the bar.
- Subscriber init location — `main.rs` between `Cli::parse()` and
  `tome::run(cli)`, OR the first line of `tome::run`. The first variant
  means `main.rs`'s typed-error downcasts (`LintFailed`,
  `MigrationPartialOrFailed`) print AFTER tracing is installed; the
  second variant means subscriber init errors are unprintable. Planner
  picks; recommendation is `main.rs` so subscriber-init errors can fall
  back to `eprintln!`.
- Subscriber init's return type — `tracing_subscriber::Registry`'s
  builder pattern; whether to return a `WorkerGuard` (for tracing-
  appender if it were wired) is moot per D-SUB-3 scaffold-only; no
  guard threading.
- Whether to add a `tracing_init` module (`crates/tome/src/tracing_
  init.rs`) or inline the builder in `main.rs` / `lib.rs::run`.
  Recommendation: small module for testability — `pub fn install
  (level: LogLevel) -> Result<()>` that future `tracing-error` /
  `tracing-appender` wiring can extend.
- Format of `cause = …` field in OBS-04 events — `Display` (via `%`)
  vs `Debug` (via `?`). Recommend `Display` (user-facing strings, no
  surrounding `ChangeCause::HashChanged` enum noise).
- Whether to use `#[tracing::instrument(skip_all, fields(elapsed_ms))]`
  attribute or explicit `info_span!("step_name").entered()` blocks for
  the 5 step spans. Either honours D-SPAN-1/D-SPAN-2; recommendation is
  the explicit `info_span!` form for the in-`lib.rs::sync` step
  boundaries because the steps don't map 1:1 to function boundaries.
- LogLevel → EnvFilter directive mapping function — recommend `fn
  level_directive(l: LogLevel) -> &'static str { match l { Quiet =>
  "warn", Normal => "info", Verbose => "debug" } }` colocated with
  `LogLevel` in `cli.rs`.
- ChangeCause module location — recommend new
  `crates/tome/src/change_cause.rs` (small enum file) over inline in
  `library.rs` so `distribute.rs` doesn't have to re-export from
  another module.
- `MissingFromMachine` count field — recommend `report.missing.len()`
  inline in `render_sync_report`; no new helper.
- Reconcile classification line color/glyph — recommend reusing
  today's `format_summary` palette (`style("✓").green()`, `style("⚠").
  yellow()`) for visual consistency.
- Whether `render_sync_report` becomes the orchestrator that calls
  both reconcile-summary rendering AND cleanup-buckets rendering, OR
  remains a thin "additional summary" function with the call ordering
  owned by `lib.rs::sync`. Recommend the first (centralizes the
  ordering contract); flag if it forces awkward parameter threading.
- 18-PLAN.md anchor pattern — recommend the same Plan A / Plan B
  structure used in Phases 11-16 (named plans, success-criteria-anchored
  task lists, traceability to OBS-IDs).

### Folded Todos

(None — `gsd-tools.cjs todo match-phase 18` returned 0 matches.)

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### Design + planning context (this milestone)

- `.planning/REQUIREMENTS.md` — OBS-01..05 are this phase's requirements
  (lines 13-19). OBS-FUTURE-01/02 + the "Out of Scope (v0.11)" table
  (lines 44-54) define negative scope. The "scope discipline:
  'instrument existing output' — not 'redesign output'" framing (line 13)
  is the single most important constraint.
- `.planning/ROADMAP.md` — Phase 18 section (lines 203-213). Success
  criteria 1-5 are the testable targets.
- `.planning/PROJECT.md` — Milestone goal (line 147), tracing-as-default
  decision (line 151, 159), HARD-07 LogLevel substrate note (Phase 15
  archive section), scope-discipline framing (lines 159-160). Backward-
  compat policy (line 161): "Flag/env-var behavior changes will be
  release-noted but not gated on a migration shim."
- `.planning/STATE.md` — `tracing` adoption-cost blocker note (line 93,
  ~50+ call sites) and output-discipline-boundary note (line 94,
  enumerate-in-scope-vs-out-of-scope-to-avoid-drift) are the two open
  questions Phase 18 closes via D-OUT-1 and D-SUB-2.

### Prior phase context (decisions to honour)

- `.planning/phases/15-cli-hardening/15-CONTEXT.md` —
  - HARD-07: `LogLevel { Quiet, Normal, Verbose }` enum + `ALL` array
    + exhaustive-match sentinel. OBS-02 extends this enum (D-ENV-1
    bridges it to EnvFilter); does NOT replace it.
  - HARD-15: `wizard.rs` chrome routed to stderr — same discipline
    D-OUT-2 inherits.
  - HARD-22: `Config::save_checked` + `paths::unexpand_tilde` — not
    directly relevant but the precedent for `paths.rs` helper additions
    if Plan B needs one.
- `.planning/phases/16-cleanup-message-ux-docs/16-CONTEXT.md` —
  - D-UX01-4: stderr discipline for cleanup output. D-OUT-2 mirrors.
  - The 3-bucket cleanup render call at `lib.rs:1768` is the anchor
    point for D-ENV-4's OBS-05 placement ("immediately above").
- `.planning/phases/13-lockfile-authoritative-sync/13-CONTEXT.md` —
  Reconcile classification model (Match/Drift/Vanished/
  MissingFromMachine). `ReconcileReport::missing` field already exists
  (no new computation; D-ENV-4 just surfaces the count).
- `.planning/phases/10-…` (no dedicated CONTEXT.md; archived in
  PROJECT.md Key Decisions) — POLISH-04 `FailureKind::ALL` compile-
  enforced via exhaustive-match sentinel. D-SPAN-3 mirrors this
  pattern for `ChangeCause::ALL`.

### Codebase modules being changed in Phase 18

**In-scope (tracing-migrated):**

- `crates/tome/src/main.rs` — Subscriber install between `Cli::parse()`
  and `tome::run(cli)` per Claude's-discretion recommendation. Typed-
  error downcasts (`LintFailed`, `MigrationPartialOrFailed`) keep their
  raw `eprintln!` (this is the error-printer site, not chatter).
- `crates/tome/src/lib.rs` — `sync` pipeline (line 1464 onwards). 97
  `eprintln!`/`println!` sites in the file overall; the in-sync ones
  migrate to `tracing::*!`. `render_sync_report` (line 1801) gains the
  OBS-05 reconcile classification line + drift/vanished detail
  relocation per D-ENV-4. Per-step `info_span!` blocks wrap each step
  per D-SPAN-1/D-SPAN-2.
- `crates/tome/src/reconcile.rs` — Plan A proof candidate. 6 sites.
  `render_summary` / `format_summary` still useful (callable from
  `render_sync_report`) but the inline-at-line-1557 call site goes
  away per D-ENV-4.
- `crates/tome/src/library.rs` — `consolidate` re-emit branches emit
  `ChangeCause` events per D-SPAN-3 / D-SPAN-4.
- `crates/tome/src/distribute.rs` — `distribute_to_directory` re-emit
  branches emit `ChangeCause` events per D-SPAN-3 / D-SPAN-4.
- `crates/tome/src/cleanup.rs` — `cleanup_library` / `cleanup_target`
  diagnostic chatter (2 sites). The user-facing 3-bucket render
  (`render_cleanup_buckets`) stays as direct stderr write — that's
  ceremonial summary output, not log-like. Diagnostic warnings around
  it migrate.
- `crates/tome/src/discover.rs` — Warning aggregation already threads
  through `Vec<String>` to `lib.rs::sync`; the user-facing print site
  in `lib.rs::sync` (currently `for w in &warnings { eprintln!
  ("warning: {}", w); }`, line ~1603) migrates to `warn!`.
- `crates/tome/src/cli.rs` — `LogLevel` (lines 6-54) gains
  `pub fn directive(self) -> &'static str` per Claude's-discretion
  recommendation. `ALL` array discipline preserved.

**New module (Plan A):**

- `crates/tome/src/tracing_init.rs` (recommended) — `pub fn install
  (level: LogLevel) -> Result<()>` with subscriber builder, stderr
  writer, compact format, target-hidden + info-level-suppressed.
  Reads `TOME_LOG` via `EnvFilter::try_from_env` per D-ENV-1.

**New module (Plan B):**

- `crates/tome/src/change_cause.rs` (recommended) — `enum ChangeCause`
  per D-SPAN-3 with `ALL` array + exhaustive-match sentinel + `impl
  Display`.

**Out of scope (keep raw stdout/stderr; doc-enforced per D-OUT-1):**

- `crates/tome/src/wizard.rs`
- `crates/tome/src/browse/*`
- `crates/tome/src/status.rs`
- `crates/tome/src/doctor.rs` (Phase 19 OBS-06 territory)
- `crates/tome/src/lint.rs`
- All `tabled::Table` render sites in the codebase

### Implementation precedent (existing patterns to mirror)

- `crates/tome/src/cli.rs:14-54` — `LogLevel` + `ALL` exhaustive-match
  sentinel + `const_assert!` length check. `ChangeCause` (D-SPAN-3)
  mirrors this exactly.
- `crates/tome/src/remove.rs::FailureKind::ALL`,
  `crates/tome/src/migration_v010.rs:53-76::MigrationFailureKind::ALL`
  — POLISH-04 pattern. Same shape for ChangeCause.
- `crates/tome/src/reconcile.rs:683-753::format_summary` / `render_
  summary` — separation of `format_*` (returns String, testable) +
  `render_*` (writes to stderr/stdout). Plan B's `render_sync_report`
  expansion may want a similar `format_classification_line` helper
  for testability.
- `crates/tome/src/cleanup.rs::render_cleanup_buckets` — Phase 16
  3-bucket rendering precedent. Writes via `std::io::Write` trait
  on a generic writer so tests can pass a `Vec<u8>` buffer. OBS-05's
  classification line render can adopt the same `Write`-generic shape.
- `crates/tome/src/marketplace.rs::render_install_failures` — SAFE-01-
  shaped grouped renderer; precedent for warn-block formatting.

### Specs / ADRs

- No formal ADRs in this codebase. The decision log is `PROJECT.md`'s
  Key Decisions table + per-phase CONTEXT.md files.

### External docs (consult during planning)

- `tracing` crate documentation (`mcp__context7__resolve-library-id`
  + `query-docs` for `tracing`, `tracing-subscriber`) — for subscriber
  builder method names, `EnvFilter` directive syntax, `FmtSpan`
  variants.
- `tracing-error` README — to confirm `ErrorLayer` install shape (for
  the deferred wiring task; not used in Plan A/B).
- `tracing-appender` README — to confirm `RollingFileAppender` + non-
  blocking writer shape (for the deferred wiring task; not used in
  Plan A/B).

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets

- **`LogLevel` enum + `ALL` exhaustive-match sentinel + `const_assert!`
  length check** — `crates/tome/src/cli.rs:14-54`. Direct reuse: add
  `pub fn directive(self) -> &'static str` returning `"warn"` /
  `"info"` / `"debug"`. `ChangeCause` (D-SPAN-3) clones this whole
  pattern.
- **`MigrationFailureKind::ALL` + exhaustive sentinel** —
  `crates/tome/src/migration_v010.rs:53-76`. The exact template
  `ChangeCause` follows.
- **`reconcile::format_summary` / `render_summary`** —
  `crates/tome/src/reconcile.rs:683-753`. Format/render split is the
  testability pattern for any new summary helper Plan B introduces.
- **`cleanup::render_cleanup_buckets`** — writes via `std::io::Write`
  trait on a generic writer. Same shape for OBS-05's classification
  line.
- **`ReconcileReport::missing: Vec<Classified>`** —
  `crates/tome/src/reconcile.rs:105`. Field already populated by
  `classify_lockfile`. D-ENV-4 just reads `.len()` in `render_sync_
  report` — zero new computation.
- **`paths::collapse_home`, `paths::unexpand_tilde`** — existing
  helpers; not directly needed but available if Plan A/B emits path
  fields in tracing events.

### Established Patterns

- **stderr discipline for diagnostics** — D-UX01-4 (cleanup),
  HARD-15 (wizard), `lib.rs::sync` eprintln! sites. D-OUT-2 inherits.
- **Exhaustive-match sentinel + `ALL` array** for finite enums —
  HARD-07 / POLISH-04 / MigrationFailureKind / FailureKind. D-SPAN-3
  applies to `ChangeCause`.
- **`format_*` returns `String`; `render_*` writes to writer** —
  reconcile + cleanup precedent. Any new summary helper Plan B adds
  follows this split for testability.
- **Decision-site counter incrementation** — `result.updated += 1` in
  `library.rs::consolidate`. D-SPAN-4 emits ChangeCause events at the
  same decision branches; no new struct fields.
- **Plan/render/execute triad** — not directly applicable to Phase 18
  (no destructive commands introduced) but the planner familiarity
  with the pattern carries over for any new ceremony.

### Integration Points

- **`main.rs` between `Cli::parse()` and `tome::run(cli)`** —
  subscriber install lands here (Claude's-discretion recommendation).
  Today's flow:
  ```rust
  let cli = tome::cli::Cli::parse();
  match tome::run(cli) { ... }
  ```
  After Plan A:
  ```rust
  let cli = tome::cli::Cli::parse();
  tome::tracing_init::install(cli.log_level())
      .unwrap_or_else(|e| eprintln!("warning: tracing init failed: {e}"));
  match tome::run(cli) { ... }
  ```
- **`lib.rs::sync` per-step boundaries** — currently each step is
  bracketed by `let sp = show_progress.then(|| spinner(...))` +
  `if verbose { eprintln!("{}", style(...).dim()); }` (e.g. lines
  1582-1594 for "Resolving git sources..."). Plan B wraps each step
  in `info_span!("step_name").entered()`; the spinner stays
  (orthogonal — it's a TTY UX widget, not log-like). The
  `if verbose { eprintln!(...); }` "doing X..." lines migrate to
  `debug!` per D-OUT-3 (spans verbose-only) — they become
  redundant once the span CLOSE event prints the same step name
  with `elapsed_ms`. Planner deletes the redundant `eprintln!` and
  trusts the span event.
- **`render_sync_report` at `lib.rs:1801`** — call site receives the
  OBS-05 classification line + drift/vanished detail relocation per
  D-ENV-4. Today's signature is `render_sync_report(&report:
  &SyncReport)`. Plan B extends the input — likely a 2nd parameter
  `report: &SyncReport, reconcile_report: Option<&ReconcileReport>`
  so the unowned `if let Some(claude_adapter) = build_claude_adapter
  (config)?` branch threads the report through. Alternative: store
  the reconcile report on `SyncReport` itself (cleaner; more
  refactor).
- **Inline `reconcile::render_summary` call at `lib.rs:1557`** —
  deleted per D-ENV-4. Caveat: `reconcile_install_failures` rendering
  at line 1568 stays where it is (that's `marketplace::render_
  install_failures`, a different code path; not part of OBS-05).
- **`tests/cli_*.rs` snapshot tests** — currently many integration
  tests `assert_cmd::output()` and check stderr content. The format-
  reformatting per D-OUT-4 (no `LEVEL target:` prefix at info)
  preserves byte-near-identical content, but any test that asserts
  on exact `eprintln!`-shaped output may need an update. Plan B
  enumerates which tests need re-snapshotting; the planner samples
  before committing to the format choice.

</code_context>

<specifics>
## Specific Ideas

- **`ChangeCause` user-facing strings (D-SPAN-3) must match OBS-04
  vocabulary verbatim:** `"hash changed"`, `"previously failed"`,
  `"newly added"`, `"directory now allowed"`. These are the literal
  strings users grep for (`grep "cause=hash changed" sync.log`).
  Changes to these strings would silently break user filtering.
- **OBS-05 classification line format must match the success-criterion
  shape:** `reconcile: N match · M drift · K vanished · L missing-
  from-machine`. Today's `format_summary` uses `✓` and `⚠` glyphs;
  D-ENV-4 keeps those visual elements. Open: does the new line use
  the same glyph palette, or text-only? Recommend: glyphs to match
  visual continuity with the existing format_summary output that
  drift/vanished detail still uses.
- **`TOME_LOG` documentation in `--help` text** — `tome --help` and
  `tome sync --help` long_about must explain (a) default level, (b)
  flag mapping, (c) `TOME_LOG` directive syntax with one realistic
  example, (d) precedence (TOME_LOG wins). Recommend the example
  `TOME_LOG=tome::sync=debug,tome::reconcile=info` to mirror the
  OBS-02 success criterion's example string verbatim.
- **Plan A proof-module recommendation = `reconcile.rs`** — it's
  small (6 sites), already has a clean `format_summary` /
  `render_summary` separation, and its OBS-05 work in Plan B
  benefits from the substrate being battle-tested in `reconcile.rs`
  first. `lib.rs::sync` is the alternative if the planner wants
  end-to-end span validation in Plan A; but `lib.rs::sync` is the
  heaviest migration, which defeats the "small proof" framing.
- **The format reformatting (D-OUT-4) WILL change snapshot tests.**
  Plan B's task list must enumerate test files touched. Today's
  `assert_cmd::Command::cargo_bin("tome").assert().stderr(predicates::
  str::contains("..."))` calls survive (containment, not equality),
  but any `insta::assert_snapshot!` over stderr that includes the
  current `eprintln!` chatter will diff. Planner samples
  `tests/cli_*.rs` early.
- **Default `tome sync` output should look like v0.10.0 with TWO
  additions:** (a) reconcile classification line in the final
  summary block (OBS-05), (b) ChangeCause events on re-emit (OBS-04
  — only fires when a skill actually re-copies/re-symlinks; idle
  syncs show no new lines). All other info-level chatter stays
  byte-near-identical.

</specifics>

<deferred>
## Deferred Ideas

- **`tracing-error::ErrorLayer` install + `anyhow` `.in_current_
  span()` sweep** — Deps scaffolded per D-SUB-3; wiring deferred to
  Phase 19 (OBS-06 doctor categorization may use it) or v1.0 (Tauri
  IPC error context). No Phase 18 work.
- **`tracing-appender` file sink** — Dep scaffolded per D-SUB-3;
  wiring deferred to v1.0. Open questions for whoever wires it: file
  path (`$XDG_STATE_HOME/tome/tome.log`? `~/.tome/logs/tome.log`?),
  rotation policy (daily? size-capped?), `TOME_LOG_FILE` env override.
- **JSON-formatted log output (OBS-FUTURE-01)** — `--log-format json`
  / `TOME_LOG_FORMAT=json`. Tracing-subscriber `fmt::format::Json`
  layer covers this; flag-level integration deferred until a real
  machine consumer exists (likely v1.0 Tauri IPC).
- **OpenTelemetry export (OBS-FUTURE-02)** — `tracing-opentelemetry`.
  Not justified at single-user scale.
- **Sub-spans per directory (in distribute) or per skill (in
  consolidate)** — D-SPAN-1 keeps the tree flat. If a future
  diagnostic need surfaces (e.g., "which directory is slowest?"), add
  a `distribute_dir{name=…}` sub-span. Track in
  `18-deferred-items.md` if the planner notes a use case during
  execution.
- **Per-target-module EnvFilter baked defaults** (e.g.
  `tome::backup=warn` default) — D-ENV-2 declined. If a module turns
  out to be too chatty at info, demote those specific call sites to
  `debug!` rather than bake a global filter.
- **Custom `FormatEvent` impl** — D-OUT-4 picked `fmt::compact()` with
  two knobs. If those knobs don't cover what the planner needs (e.g.,
  the suppression of info-level prefix turns out to require deeper
  customization), promote to a custom `FormatEvent` impl as part of
  Plan A. Track here only if deferred entirely.
- **Migration of out-of-scope modules** (wizard / browse / status /
  doctor table renderers / list table / lint) — Permanent v0.11
  deferral per D-OUT-1. These modules render ceremonial output to
  stdout or interactive output via dialoguer/ratatui; they're not
  log-like. Phase 19 OBS-06 may pull `doctor.rs`'s diagnostic-warning
  surface (not the table) into tracing for OBS-06 categorization.
- **Snapshot test re-baselining for format changes** — Tracked in
  Plan B's task list once the planner enumerates affected tests.
  If the planner discovers the volume is large enough to warrant a
  separate plan, escalate to a Plan C; flag in `18-deferred-items.md`.
- **Spinner / progress UX overlap** — `lib.rs::sync` uses `indicatif::
  ProgressBar` spinners per step (line 1582 etc.). Spinners stay
  orthogonal to tracing (TTY UX widget, not log-like). If a future
  effort unifies them, that's a v0.12+ polish; out of scope here.
- **`MissingFromMachine` rename / vocabulary cleanup** — The variant
  name is already settled (D-01 in Phase 13). D-ENV-4 surfaces the
  count under the user-facing label `missing-from-machine` (kebab-
  case for the classification line; CamelCase enum stays internal).

### Reviewed Todos (not folded)

(None — `gsd-tools.cjs todo match-phase 18` returned 0 matches.)

</deferred>

---

*Phase: 18-observability-foundation-sync-diagnostics*
*Context gathered: 2026-05-12*
