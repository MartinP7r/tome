# Phase 18: Observability foundation + sync diagnostics - Discussion Log

> **Audit trail only.** Do not use as input to planning, research, or execution agents.
> Decisions are captured in CONTEXT.md — this log preserves the alternatives considered.

**Date:** 2026-05-12
**Phase:** 18-observability-foundation-sync-diagnostics
**Areas discussed:** Substrate + migration shape, Output discipline & rendering, Span surface + change-cause carrier, Flag/env semantics + OBS-05 placement

---

## Substrate + migration shape

### Q1: How firm is the `tracing` lock when planning Phase 18?

| Option | Description | Selected |
|--------|-------------|----------|
| Tracing is locked | No checkpoint. Plan goes straight at migrating all ~97 call sites with tracing. Trust the prior decision. Simplest plan shape. | ✓ |
| Sample first, decide | Plan 1 instruments ONE module end-to-end with tracing. Diff review. If clean → continue; if gnarly → switch to log. Then sweep. | |
| Tracing locked, narrow scope | Tracing locked, no checkpoint. Migrate only the 2–3 modules OBS-03/04/05 needs. Defer rest to v0.12. Cost as scope question. | |

**User's choice:** Tracing is locked
**Notes:** User initially asked "what do you mean by cost-gate?" before answering. Explanation provided in plain English: a checkpoint moment where a sample of real call-sites with `tracing` reveals migration cost before committing to the full sweep. With concept clarified, user picked the strongest lock — no fallback gate exercised.

---

### Q2: How should the migration sweep be split into plans?

| Option | Description | Selected |
|--------|-------------|----------|
| Substrate + migration split | Plan A: subscriber wiring + 1–2 modules as proof. Plan B: migrate the rest. Two PRs; substrate review is small + focused. | ✓ |
| One plan, all 5 modules | Single plan: subscriber wiring + full migration + spans + change-cause + summary. Atomic landing; large diff. | |
| Substrate + per-feature plans | Plan A: substrate. Plan B: spans (OBS-03). Plan C: change-cause (OBS-04). Plan D: reconcile summary (OBS-05). 4 plans. | |

**User's choice:** Substrate + migration split
**Notes:** Two-plan split. Plan A's proof module recommended in CONTEXT.md as `reconcile.rs` (small, isolated, already has clean format_summary); planner's discretion to pick `lib.rs::sync` instead if end-to-end span validation matters more.

---

### Q3: Which tracing-ecosystem crates does the substrate pull in beyond `tracing` itself?

| Option | Description | Selected |
|--------|-------------|----------|
| tracing-subscriber (EnvFilter + fmt) | Mandatory for the subscriber. EnvFilter for TOME_LOG + flag mapping, fmt for human-readable output. | ✓ |
| tracing-error (anyhow context) | Integrates tracing spans into anyhow::Error backtraces. Useful for tome doctor. | ✓ |
| tracing-appender (file output) | Writes logs to a rotating file (e.g. ~/.tome/tome.log). Tauri IPC consumer (v1.0) may want this. | ✓ |
| Just tracing-subscriber | Bare minimum. Defer the other two to v1.0 Tauri prep. | |

**User's choice:** All three optional crates in
**Notes:** Multi-select. User picked all three optional crates beyond mandatory tracing+tracing-subscriber. This raised follow-up scope questions: do the deps mean Phase 18 wires file output / error layer, or just brings in the crates ready for later?

---

### Q3a (follow-up): tracing-appender — scope in Phase 18?

| Option | Description | Selected |
|--------|-------------|----------|
| Scaffold only | Add crate to Cargo.toml; no file sink in Phase 18. v1.0 wires it. | ✓ |
| Wire it now | Phase 18 wires a file sink. Needs path + rotation + env-override decisions. | |
| Wire it, minimal | Wire a file sink with hardcoded path (~/.tome/tome.log), no rotation, no env override. | |

**User's choice:** Scaffold only
**Notes:** Defers the file-sink + path-config questions to v1.0 where Tauri IPC actually consumes it.

---

### Q3b (follow-up): tracing-error — scope in Phase 18?

| Option | Description | Selected |
|--------|-------------|----------|
| Scaffold only | Add crate; do NOT install ErrorLayer in Phase 18. Phase 19 / v1.0 wires it. | ✓ |
| Wire it now | Install ErrorLayer + sweep anyhow sites with .in_current_span(). | |
| Wire layer only | Install ErrorLayer in subscriber; don't sweep anyhow sites. Errors raised inside spans pick up context automatically. | |

**User's choice:** Scaffold only
**Notes:** Same posture as tracing-appender. Defers anyhow-integration sweep to a phase where a consumer (Phase 19 OBS-06 doctor categorization, or v1.0) actually surfaces the value.

---

## Output discipline & rendering

### Q4: How should the in-scope module list be enforced?

| Option | Description | Selected |
|--------|-------------|----------|
| Doc-enforced | List in CONTEXT.md + 18-PLAN.md. Code review catches drift. | ✓ |
| Lint-enforced (deny in module) | #![deny(clippy::print_stdout, clippy::print_stderr)] on in-scope modules. CI catches drift. | |
| Doc + clippy on in-scope | Doc list AND per-module attribute. Belt-and-suspenders. | |

**User's choice:** Doc-enforced
**Notes:** Reviewer discipline approach. Avoids per-module attribute noise + clippy-lint-name-stability concerns.

---

### Q5: Where does tracing output go by default?

| Option | Description | Selected |
|--------|-------------|----------|
| stderr | Unix convention; matches D-UX01-4 + HARD-15. | ✓ |
| stdout | Default tracing-subscriber behavior; matches existing println! calls in lib.rs. | |
| Split by level | (Effectively the same as 'stderr' — explicit contract statement.) | |

**User's choice:** stderr (after asking "what's the standard recommendation?")
**Notes:** User asked for the standard recommendation. Explanation: long-standing Unix convention puts diagnostic logs on stderr, machine-readable output on stdout; cargo/rustc/git all do this; tracing-subscriber defaults to stdout because it's library-first, but every Rust CLI overrides it. Plus codebase precedent (D-UX01-4, HARD-15). User accepted the recommendation.

---

### Q6: When are step spans visible to the user?

| Option | Description | Selected |
|--------|-------------|----------|
| Verbose-only (debug) | Default tome sync stays terse. --verbose shows per-step elapsed-ms + nested span path. | ✓ |
| Step spans at info, sub-spans at debug | Default shows one line per top-level step. Nested spans at --verbose. ~5 extra lines per sync. | |
| All spans at debug | Spans never emit at info. Matches OBS-03 'visible in --verbose' wording exactly. | |

**User's choice:** Verbose-only (debug)
**Notes:** Default sync output stays calm. OBS-05 reconcile classification is the only new info-level addition; per-step timing is opt-in via --verbose.

---

### Q7: Format style for the subscriber's text output?

| Option | Description | Selected |
|--------|-------------|----------|
| compact, no target, no level at info | fmt::compact() with .with_target(false) + info-level prefix suppressed. Info lines look like today's eprintln! output. | ✓ |
| compact, default options | fmt::compact() with everything on. Slightly noisier than current eprintln! density. | |
| pretty (multi-line) | fmt::pretty(). Multi-line with span fields. Out of place for CLI tool default. | |
| Custom FormatEvent | Hand-rolled formatter for full visual control. | |

**User's choice:** compact, no target, no level at info (after asking "what's the standard recommendation?")
**Notes:** User asked for the standard recommendation. Explanation: pretty() is server-log-ish; custom FormatEvent is more code; compact() with target+info-level suppression is the pragmatic middle ground that most CLI projects land on. Byte-close-to-identical to today's eprintln! output at info level.

---

## Span surface + change-cause carrier (OBS-03, OBS-04)

### Q8: How nested should the span tree be (visible at --verbose)?

| Option | Description | Selected |
|--------|-------------|----------|
| Top + 5 step spans only | sync (top) → discover/reconcile/consolidate/distribute/cleanup. No sub-spans. Matches OBS-03 verbatim. | ✓ |
| Step + per-directory in distribute | Add distribute_dir{name=…} per directory. Other steps single-span. | |
| Step + per-directory + per-skill | Full drill-down. O(N) span events. | |

**User's choice:** Top + 5 step spans only
**Notes:** Flat tree. Matches OBS-03 success criterion. Per-directory / per-skill sub-spans deferred to future diagnostic need (tracked in CONTEXT.md deferred section).

---

### Q9: When does the elapsed_ms field fire on a span?

| Option | Description | Selected |
|--------|-------------|----------|
| Close only (recommended) | FmtSpan::CLOSE — one line per span on completion. Matches OBS-03 wording 'elapsed-ms attached on span close'. | ✓ |
| New + close | FmtSpan::NEW \| CLOSE — entry + close lines. Live progress; 2x line volume. | |
| Active span events (entry/exit pairs) | FmtSpan::FULL. Noisy; rarely what a CLI wants. | |

**User's choice:** Close only (recommended)
**Notes:** One line per step on close. Minimal output volume.

---

### Q10: How is the change-cause carried in the OBS-04 info! event?

| Option | Description | Selected |
|--------|-------------|----------|
| Typed enum + ALL sentinel (recommended) | enum ChangeCause { … } with POLISH-04 ALL array + exhaustive-match sentinel + impl Display. Refactor-safe. | ✓ |
| Free-form &'static str | const strings + free-form cause=…. Typo-prone; no compile-time check. | |
| Owned String, no const | Plain "hash changed".to_string() at each site. Worst option. | |

**User's choice:** Typed enum + ALL sentinel (recommended)
**Notes:** Mirrors HARD-07 / POLISH-04 / MigrationFailureKind discipline. ChangeCause module recommended at crates/tome/src/change_cause.rs.

---

### Q11: Where is the change-cause computed?

| Option | Description | Selected |
|--------|-------------|----------|
| At the decision site (recommended) | library.rs::consolidate and distribute.rs::distribute_to_directory emit info! directly at the re-emit branch. No new struct fields. | ✓ |
| Threaded via result struct | Extend ConsolidateResult / DistributeResult with cause field; lib.rs::sync emits centrally. | |
| Hybrid | Decision site computes; helper wraps the info! macro. | |

**User's choice:** At the decision site (recommended)
**Notes:** Local clarity. No plumbing across struct boundaries. Mirrors result.updated += 1 increment-at-decision-site precedent.

---

## Flag/env semantics + OBS-05 placement

### Q12: When BOTH --verbose/--quiet and TOME_LOG are set, which wins?

| Option | Description | Selected |
|--------|-------------|----------|
| TOME_LOG wins (recommended) | Env fully overrides flag. Matches cargo/tokio RUST_LOG mental model. --quiet becomes no-op when TOME_LOG is set. | ✓ |
| Flag wins | Flag is authoritative; TOME_LOG ignored when --verbose/--quiet is passed. | |
| Layered (flag is base, env adds directives) | Flag sets global default; TOME_LOG directives layer on top per-target. Most flexible. | |

**User's choice:** TOME_LOG wins (recommended)
**Notes:** Matches RUST_LOG-style precedence. Acknowledged in release notes: --quiet is a no-op when TOME_LOG is set.

---

### Q13: Where does the OBS-05 reconcile classification line render?

| Option | Description | Selected |
|--------|-------------|----------|
| Move into render_sync_report (recommended) | Delete inline reconcile::render_summary at lib.rs:1557. render_sync_report emits the classification line immediately above 3-bucket cleanup output. Drift/vanished detail relocates too. | ✓ |
| Keep inline + duplicate in summary | Keep render_summary mid-pipeline AND add a line to render_sync_report. Users see numbers twice. | |
| Expand render_summary in place | Add MissingFromMachine to existing inline line. Mid-pipeline placement contradicts OBS-05 wording. | |

**User's choice:** Move into render_sync_report (recommended)
**Notes:** Final-summary-block placement matches OBS-05 success criterion verbatim. Drift/vanished detail relocates with it. Planner picks whether to lift detail lines into render_sync_report directly or call format_summary from render_sync_report.

---

### Q14: What is the default log level when no flag and no env var are set?

| Option | Description | Selected |
|--------|-------------|----------|
| info (recommended) | Matches OBS-02 success criterion verbatim. Today's eprintln! chatter at info remains visible. | ✓ |
| warn | Default is calm; only warnings + errors print. BREAKS today's chatter behavior. | |
| info with target-scoped overrides | Default info globally; specific noisy modules downgraded to warn via baked EnvFilter. | |

**User's choice:** info (recommended)
**Notes:** Behavior-preservation. If a specific call site is too chatty at info, demote it to debug! in code rather than bake global filter.

---

### Q15: What happens to existing --verbose/--quiet semantics for users who don't touch TOME_LOG?

| Option | Description | Selected |
|--------|-------------|----------|
| Byte-near-identical (recommended) | Without TOME_LOG: --quiet→warn, default→info, --verbose→debug. Lines that exist today still print, modulo D-OUT-4 reformatting. | ✓ |
| Behavior-preserving but reordered | Same level mapping; span events at --verbose are NEW lines (additive). No lines silently disappear. | |
| Allow regressions for cleaner default | Some current chatter may drop to debug. 'Behavior preserved' no longer literally true. | |

**User's choice:** Byte-near-identical (recommended)
**Notes:** Scope-discipline anchor: this migration is substrate-replacement, not output-redesign. New span CLOSE events at --verbose are additive; no info-level lines silently disappear. If a line should be debug rather than info, demote it explicitly in code and document.

---

## Claude's Discretion

Captured in CONTEXT.md `<decisions>` § Claude's Discretion. Highlights:

- Plan A's choice of proof module (recommendation: `reconcile.rs`)
- Subscriber init location (recommendation: `main.rs` between `Cli::parse()` and `tome::run(cli)`)
- Whether to add a `tracing_init` module (recommendation: yes — `crates/tome/src/tracing_init.rs`)
- Cause field format in OBS-04 events (recommendation: `Display` via `%`)
- Whether to use `#[tracing::instrument]` attribute or explicit `info_span!` (recommendation: explicit `info_span!` for in-`lib.rs::sync` step boundaries)
- `ChangeCause` module location (recommendation: new `crates/tome/src/change_cause.rs`)
- LogLevel → EnvFilter directive mapping function colocated with `LogLevel` in `cli.rs`
- Reconcile classification line glyphs (recommendation: reuse today's `✓`/`⚠` palette)
- Whether `render_sync_report` orchestrates both reconcile-summary AND cleanup-buckets rendering (recommendation: yes — centralizes ordering)

## Deferred Ideas

Captured in CONTEXT.md `<deferred>` section. Highlights:

- `tracing-error::ErrorLayer` install + anyhow sweep — deferred to Phase 19 / v1.0
- `tracing-appender` file sink — deferred to v1.0 (Tauri IPC consumer)
- JSON-formatted log output (OBS-FUTURE-01) — deferred
- OpenTelemetry export (OBS-FUTURE-02) — deferred
- Sub-spans per directory / per skill — deferred to future diagnostic need
- Per-target-module EnvFilter baked defaults — declined
- Custom FormatEvent impl — deferred to "if `fmt::compact()` knobs don't suffice"
- Migration of out-of-scope modules (wizard / browse / status / doctor table / list / lint) — permanent v0.11 deferral
- Spinner / progress UX unification with tracing — v0.12+ polish
