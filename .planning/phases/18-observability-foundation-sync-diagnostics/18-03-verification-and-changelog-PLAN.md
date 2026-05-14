---
phase: 18-observability-foundation-sync-diagnostics
plan: 03
type: execute
wave: 3
depends_on:
  - 18-01-tracing-substrate-and-reconcile-proof
  - 18-02-migration-sweep-spans-cause-and-reconcile-line
files_modified:
  - CHANGELOG.md
  - crates/tome/tests/cli_sync.rs
files_modified_optional:
  - crates/tome/tests/cli_status.rs
  - crates/tome/tests/cli_doctor.rs
  - crates/tome/tests/cli_list.rs
autonomous: true
requirements:
  - OBS-01
  - OBS-02
  - OBS-03
  - OBS-04
  - OBS-05

must_haves:
  truths:
    - "`cargo run -p tome -- status` produces stdout BYTE-IDENTICAL to v0.10.0 (snapshot `cli_status__*.snap` files in `crates/tome/tests/snapshots/` pass without diff)."
    - "`cargo run -p tome -- init --dry-run --no-input` produces stdout BYTE-IDENTICAL to v0.10.0 (any `cli_init*.snap` files pass without diff; if no such snapshots exist, this truth is satisfied trivially)."
    - "`cargo run -p tome -- sync --verbose` against a configured environment emits at least 5 span CLOSE events (one each for `discover`, `reconcile`, `consolidate`, `distribute`, `cleanup`) with `time.busy=<value>` field present in each (RESEARCH §elapsed_ms FINDING — `time.busy` is the auto-emitted field name; \"elapsed_ms\" in the OBS-03 success criterion is conceptual)."
    - "`cargo run -p tome -- sync 2>&1` against a config that triggers reconcile emits exactly one line matching `^  reconcile: .+ match · .+ drift · .+ vanished · .+ missing-from-machine` in stdout, immediately above the per-bucket cleanup output."
    - "`cargo run -p tome -- sync --verbose 2>&1` against a config that triggers a re-emit shows at least one line containing `cause=hash changed` OR `cause=newly added` OR `cause=directory now allowed` (the three OBS-04 variants wired in Plan 18-02; `cause=previously failed` is deferred per 18-deferred-items.md)."
    - "`CHANGELOG.md` has a v0.11.0 (or [Unreleased] subsection if pre-release) entry describing the OBS-01..OBS-05 work: tracing substrate adopted, `TOME_LOG` env var introduced, per-step spans on `--verbose`, change-cause attribution at re-emit sites, reconcile classification line in `tome sync` summary, breaking-note for `--quiet` becoming a no-op when `TOME_LOG` is set (D-ENV-1 acknowledged trade-off)."
    - "An integration test in `crates/tome/tests/cli_sync.rs` (or equivalent) asserts that `cargo run -- sync --verbose` against a fixture configuration produces stderr containing the literal substring `discover` AND `time.busy` AND `cleanup` — pinning OBS-03 span emission regression-free."
  artifacts:
    - path: "CHANGELOG.md"
      provides: "v0.11.0 (or [Unreleased] subsection) entry listing OBS-01..OBS-05 changes with appropriate detail per the Keep-a-Changelog format used by prior milestones."
      contains: "tracing"
    - path: "crates/tome/tests/cli_sync.rs"
      provides: "A new integration test (e.g. `tome_sync_verbose_emits_per_step_spans`) using `assert_cmd` that asserts stderr contains the 5 step span names + `time.busy` field. Runs against a tempdir-fixture config."
      contains: "time.busy"
  key_links:
    - from: "crates/tome/tests/cli_sync.rs (new test)"
      to: "global tracing subscriber stderr output"
      via: "assert_cmd Command::cargo_bin(\"tome\").arg(\"sync\").arg(\"--verbose\").assert().stderr(predicates::str::contains(\"time.busy\"))"
      pattern: "predicates::str::contains\\(\"time\\.busy\"\\)"
    - from: "CHANGELOG.md v0.11 section"
      to: "v0.10 section (chronological order preserved per Keep-a-Changelog)"
      via: "## [0.11.0] - YYYY-MM-DD inserted above ## [0.10.0]"
      pattern: "## \\[0\\.11\\.0\\]"
---

<objective>
Verify Phase 18 lands without breaking the byte-identical-stdout commitment (success criterion 1) and without breaking OBS-03/OBS-04/OBS-05 emission contracts; document the v0.11 changes in `CHANGELOG.md`.

This plan is small by design — Plans 18-01 and 18-02 did the substrate and migration work. Plan 18-03 is the regression sweep + CHANGELOG entry. Two tasks:

1. **Verification regression test** — Add an `assert_cmd`-based integration test in `crates/tome/tests/cli_sync.rs` that pins OBS-03 span emission (asserts on stderr containing `discover` and `time.busy` for a fixture config). Snapshot re-baseline accept (or reject) the OBS-05 reconcile classification line additions in `cli_sync__*.snap` if any are outstanding from Plan 18-02. Verify the byte-identical-stdout commitment by running `cargo test -p tome --test cli_status` and `cargo test -p tome --test cli_init` (if `cli_init.rs` exists) — these MUST exit 0 with no snapshot diffs (Plan 18-01 + 18-02 did NOT touch status/init code paths).

2. **CHANGELOG entry** — Add a v0.11.0 entry (or `[Unreleased]` subsection if pre-release timing) to `CHANGELOG.md` documenting OBS-01..OBS-05 with appropriate user-facing detail. Mirror the v0.10 entry's Keep-a-Changelog format. Include the documented breaking trade-off from D-ENV-1: `--quiet` becomes a no-op when `TOME_LOG` is set.

Output: integration test in `cli_sync.rs`, CHANGELOG entry. No source-code changes beyond test-only files (plus the doc file).
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
@.planning/phases/18-observability-foundation-sync-diagnostics/18-01-tracing-substrate-and-reconcile-proof-PLAN.md
@.planning/phases/18-observability-foundation-sync-diagnostics/18-02-migration-sweep-spans-cause-and-reconcile-line-PLAN.md
@.planning/phases/18-observability-foundation-sync-diagnostics/18-01-SUMMARY.md
@.planning/phases/18-observability-foundation-sync-diagnostics/18-02-SUMMARY.md
@.planning/phases/18-observability-foundation-sync-diagnostics/18-deferred-items.md

@CHANGELOG.md
@crates/tome/tests/cli_sync.rs

<interfaces>
<!-- Key types and call sites the executor needs. Extracted from the codebase. -->

From CHANGELOG.md (current top section):

```
## [Unreleased]

## [0.10.0] - 2026-05-11
...
```

Phase 18 ships before the v0.11 cut, so Phase 18's entry goes UNDER `## [Unreleased]` (with a subsection like `### Added` / `### Changed`) OR creates a new `## [0.11.0] - {date}` heading. Decision: keep entries in `## [Unreleased]` for now — the actual v0.11 release date is set by Phase 19 when v0.11 cuts. Phase 18 is mid-milestone work.

From crates/tome/tests/cli_sync.rs (existing test patterns):

```rust
use assert_cmd::Command;
use predicates::prelude::*;
use assert_fs::prelude::*;

// Example existing test pattern (search the file for `Command::cargo_bin`):
#[test]
fn sync_initial_two_skills_passes_snapshot() {
    let tmp = assert_fs::TempDir::new().unwrap();
    // ... setup tempdir + config + skill fixtures ...
    let output = Command::cargo_bin("tome")
        .unwrap()
        .arg("--tome-home").arg(tmp.path())
        .arg("sync")
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    insta::assert_snapshot!("sync_initial_two_skills", stdout);
}
```

The new test follows the same shape but asserts on STDERR for span emission (stdout-only snapshots stay clean — that's the byte-identical-stdout commitment).
</interfaces>

</context>

<tasks>

<task type="auto" tdd="true">
  <name>Task 1: Add cli_sync integration test asserting on span CLOSE emission in stderr; verify byte-identical stdout for status + init</name>
  <files>crates/tome/tests/cli_sync.rs</files>
  <read_first>
    - crates/tome/tests/cli_sync.rs (full file — adopt the existing tempdir + skill-fixture pattern; do NOT invent a new setup helper)
    - crates/tome/tests/snapshots/ (verify cli_status__status_unconfigured.snap and cli_status__status_empty_library.snap still match — they are stdout-only and Plan 18 did NOT touch the status code path)
    - .planning/phases/18-observability-foundation-sync-diagnostics/18-RESEARCH.md §Span Shape (the 5 expected span names) and §elapsed_ms FINDING (the `time.busy=` auto-emitted field)
    - .planning/phases/18-observability-foundation-sync-diagnostics/18-RESEARCH.md §Common Pitfalls Pitfall 2 (CI ANSI) — `with_ansi(stderr.is_terminal())` means tests get ANSI off, so plain-string assertions work
    - .planning/phases/18-observability-foundation-sync-diagnostics/18-RESEARCH.md §Test Isolation Strategy (assert_cmd subprocess isolation)
  </read_first>
  <behavior>
    - New test `sync_verbose_emits_step_spans_on_stderr` in `cli_sync.rs` runs `tome sync --verbose` against a fixture and asserts stderr contains the literal substrings `discover` AND `consolidate` AND `cleanup` AND `time.busy` (the 3 step names are guaranteed-emit-able on any minimal config; reconcile + distribute may or may not fire depending on fixture)
    - The new test PASSES when run (`cargo test -p tome --test cli_sync sync_verbose_emits_step_spans_on_stderr` exits 0)
    - Existing `cli_sync__*.snap` snapshots may have been re-baselined in Plan 18-02 (acceptable — adding the OBS-05 reconcile line is intentional new content); status / list / doctor / browse snapshots are UNCHANGED (success criterion 1)
    - The test does NOT call `tracing_init::install` (per RESEARCH §Test Isolation — subprocesses are naturally isolated)
  </behavior>
  <action>
    Step 1 — Read `crates/tome/tests/cli_sync.rs` to find an existing test that sets up a minimal sync-able fixture (tempdir + skill fixture + config). Identify the helper functions or inline setup used. Reuse them in the new test — do NOT introduce a new test-fixture pattern.

    Step 2 — Add the new test at the END of `crates/tome/tests/cli_sync.rs` (after the last existing test):

    ```rust
    /// Phase 18 OBS-03 regression test: `tome sync --verbose` MUST emit per-step
    /// `tracing` spans on stderr, one for each of the 5 pipeline steps. The
    /// `time.busy=` field is auto-emitted by `FmtSpan::CLOSE` (the `elapsed_ms`
    /// wording in the OBS-03 success criterion is conceptual; the literal
    /// emitted field is `time.busy` per RESEARCH §elapsed_ms FINDING and
    /// `tracing-subscriber` 0.3 documentation).
    ///
    /// This test pins the span emission so a future migration of the subscriber
    /// (e.g., to a custom FormatEvent impl) doesn't silently drop the timing
    /// data the OBS-03 contract relies on.
    ///
    /// Note on which spans are guaranteed: `discover` and `consolidate` and
    /// `cleanup` always fire for any sync that completes; `reconcile` only
    /// fires when a Claude adapter is configured; `distribute` only fires
    /// when at least one distribution directory is configured. This test
    /// uses a minimal fixture and asserts only on the three guaranteed
    /// span names.
    #[test]
    fn sync_verbose_emits_step_spans_on_stderr() {
        // {Reuse the existing minimal sync-able fixture pattern from the file.
        //  This block must be filled in by reading the existing tests and
        //  copying their tempdir + skill fixture + tome.toml setup. The exact
        //  code depends on the helpers present.}

        let tmp = /* tempdir setup matching existing tests */;
        // ... fixture setup ...

        let output = assert_cmd::Command::cargo_bin("tome")
            .unwrap()
            .arg("--tome-home").arg(tmp.path())
            .arg("--verbose")
            .arg("sync")
            .arg("--dry-run")  // dry-run to avoid mutating fixture state
            .output()
            .expect("tome sync --verbose --dry-run must execute");

        let stderr = String::from_utf8_lossy(&output.stderr);

        // Assertions — substring containment (no regex parsing required;
        // tracing's compact format prints the span name as a token):
        assert!(
            stderr.contains("discover"),
            "stderr must contain 'discover' span name (OBS-03). stderr was:\n{stderr}"
        );
        assert!(
            stderr.contains("consolidate"),
            "stderr must contain 'consolidate' span name (OBS-03). stderr was:\n{stderr}"
        );
        assert!(
            stderr.contains("cleanup"),
            "stderr must contain 'cleanup' span name (OBS-03). stderr was:\n{stderr}"
        );
        assert!(
            stderr.contains("time.busy"),
            "stderr must contain 'time.busy' timing field (RESEARCH §elapsed_ms FINDING). stderr was:\n{stderr}"
        );
    }
    ```

    Replace the `/* tempdir setup matching existing tests */` placeholder with the actual setup code, copied from an existing test (e.g., `sync_initial_two_skills_passes_snapshot` or similar). Do NOT invent a new fixture pattern; reuse what exists.

    Step 3 — Run the test in isolation: `cargo test -p tome --test cli_sync sync_verbose_emits_step_spans_on_stderr`. Iterate on the fixture setup until it passes. Use `cargo test -- --nocapture` if needed to see actual stderr output during debugging.

    Step 4 — Verify byte-identical stdout for status (success criterion 1 anchor) — run:
    ```
    cargo test -p tome --test cli_status
    cargo test -p tome --test cli_list
    cargo test -p tome --test cli_doctor
    ```

    All MUST exit 0 with no snapshot diffs. If any diff, investigate — Plan 18-01 / 18-02 did NOT touch these code paths, so diffs indicate either:
    - A side-effect leak (e.g., tracing init writing to stdout instead of stderr) — investigate and fix
    - A test ordering issue (assert_cmd spawns subprocesses; this should not happen, but RESEARCH §Test Isolation Pitfall 1 names the failure mode)

    Step 5 — If `crates/tome/tests/cli_init.rs` exists (verify with `fd cli_init crates/tome/tests/`), run `cargo test -p tome --test cli_init` and confirm no snapshot diffs for `--dry-run --no-input` flows. If no such test file exists, this verification anchor is satisfied trivially.
  </action>
  <verify>
    <automated>cargo test -p tome --test cli_sync sync_verbose_emits_step_spans_on_stderr 2>&amp;1 | tail -3</automated>
  </verify>
  <acceptance_criteria>
    - `rg "fn sync_verbose_emits_step_spans_on_stderr" crates/tome/tests/cli_sync.rs` returns 1 match
    - `rg "time\\.busy" crates/tome/tests/cli_sync.rs` returns ≥ 1 match
    - `cargo test -p tome --test cli_sync sync_verbose_emits_step_spans_on_stderr` exits 0
    - `cargo test -p tome --test cli_status` exits 0 with zero snapshot diffs (success criterion 1 — status stdout byte-identical to v0.10.0)
    - `cargo test -p tome --test cli_list` exits 0 with zero snapshot diffs (list stdout byte-identical)
    - `cargo test -p tome --test cli_doctor` exits 0 with zero snapshot diffs (doctor stdout byte-identical)
    - `cargo clippy --all-targets -- -D warnings` exits 0
    - `cargo fmt -- --check` exits 0
  </acceptance_criteria>
  <done>New `cli_sync.rs` integration test pins OBS-03 span emission via stderr substring assertions; status/list/doctor snapshot tests remain green confirming byte-identical stdout commitment from success criterion 1.</done>
</task>

<task type="auto" tdd="false">
  <name>Task 2: Add Phase 18 entry to CHANGELOG.md under [Unreleased]</name>
  <files>CHANGELOG.md</files>
  <read_first>
    - CHANGELOG.md lines 1-15 (current [Unreleased] section + format of v0.10.0 entry below it for Keep-a-Changelog convention)
    - CHANGELOG.md lines 10-218 (sample of the v0.10.0 entry — match its formatting precedent: subsections like `### Added`, `### Changed`, `### Fixed`)
    - .planning/REQUIREMENTS.md lines 11-22 (OBS-01..OBS-05 verbatim descriptions — use as source of truth for the changelog wording)
    - .planning/phases/18-observability-foundation-sync-diagnostics/18-CONTEXT.md D-ENV-1 (the `--quiet` no-op trade-off — release-noted acknowledgment)
    - .planning/phases/18-observability-foundation-sync-diagnostics/18-deferred-items.md (PreviouslyFailed deferral — mention in CHANGELOG as a known-deferred item)
  </read_first>
  <behavior>
    - CHANGELOG.md `[Unreleased]` section contains subsections describing Phase 18's work
    - Entry mentions: `tracing` adoption, `TOME_LOG` env var, per-step spans, change-cause attribution, reconcile classification line
    - Entry mentions the D-ENV-1 trade-off (`--quiet` becomes no-op when `TOME_LOG` set)
    - Entry mentions PreviouslyFailed cause deferral
    - Format matches Keep-a-Changelog convention used by prior v0.10 entry
  </behavior>
  <action>
    In `CHANGELOG.md`, under the `## [Unreleased]` heading (currently empty after the v0.10.0 ship), add the following subsections. Place them AFTER the `## [Unreleased]` line and BEFORE the `## [0.10.0] - 2026-05-11` line.

    ```markdown
    ### Added

    - **OBS-01 / OBS-02 — Structured logging substrate (`tracing`).** Adopted `tracing` + `tracing-subscriber` as the application logging substrate. Internal `eprintln!`/`println!` chatter in the sync, reconcile, consolidate, distribute, and cleanup paths now routes through `tracing::{info,warn,debug}!`. Wizard prompts, TUI browse output, and user-facing summary tables (`tome status`/`list`/`doctor` tables, `tome sync` final summary block) remain on direct stdout — output discipline unchanged for byte-identical stdout in `tome status` and `tome init --dry-run`.
    - **OBS-02 — `TOME_LOG` environment variable.** New `TOME_LOG` env var configures the subscriber filter using `tracing_subscriber::EnvFilter` directive syntax. Examples: `TOME_LOG=debug` (verbose globally), `TOME_LOG=tome::sync=debug,tome::reconcile=info` (verbose specifically in sync), `TOME_LOG=warn,tome::library=debug` (warn globally except verbose consolidate). When `TOME_LOG` is set, it fully replaces the flag-derived level.
    - **OBS-03 — Per-pipeline-step spans with timing.** `tome sync --verbose` (or `TOME_LOG=tome::sync=debug`) now emits one `tracing` span per pipeline step (`discover`, `reconcile`, `consolidate`, `distribute`, `cleanup`) nested under a top-level `sync` span. Each span records `time.busy` / `time.idle` timing fields on close — useful for diagnosing slow phases.
    - **OBS-04 — Change-cause attribution.** When `consolidate` or `distribute` re-emits a skill, the log line names the cause at `info!` level via a typed `ChangeCause` field. Three of four causes wire up in this release: `hash changed`, `newly added`, `directory now allowed`. A user running `tome sync --verbose` can grep stderr for `cause=` to see exactly why each re-emit happened.
    - **OBS-05 — Reconcile classification breakdown.** The `tome sync` final summary block now includes a per-classification reconcile line: `reconcile: ✓ N match · ⚠ M drift · ⚠ K vanished · ⚠ L missing-from-machine` immediately above the per-bucket cleanup summary. Counts come from the existing `ReconcileReport` populated since v0.10's Phase 13; no new computation.

    ### Changed

    - **Logging output now routes to stderr by default** for all migrated diagnostic chatter. Stdout remains reserved for user-facing summary tables and version output (consistent with Unix convention; matches `tome sync`'s cleanup-bucket output discipline from v0.10's Phase 16).
    - **`--quiet` and `--verbose` flags map to subscriber levels** through the new `LogLevel::directive` accessor: `--quiet` → `warn`, default → `info`, `--verbose` → `debug`. Behavior preserved for users who only use the flags; no lines silently disappear.

    ### Deferred (tracked in `.planning/phases/18-observability-foundation-sync-diagnostics/18-deferred-items.md`)

    - **`ChangeCause::PreviouslyFailed` cause not emitted.** The enum variant + `Display` impl ship in this release (so the grep vocabulary `cause=previously failed` is reachable), but the emission site requires a manifest-schema bump to track per-skill last-sync failure state. Deferred to v0.12 or a later polish phase.

    ### Trade-offs (release-noted; no migration shim)

    - `--quiet` becomes a no-op when `TOME_LOG` is set in the environment. Matches the `RUST_LOG` precedence mental model users bring from cargo/tokio. Per the project's documented policy (Backward compat: None), this is not gated on a shim.
    - The OBS-03 timing field is named `time.busy` (auto-emitted by `tracing-subscriber`'s `FmtSpan::CLOSE` event), NOT `elapsed_ms`. The OBS-03 success-criterion wording said "elapsed_ms" conceptually; `time.busy` is the literal field name — grep accordingly.
    - `tracing-error` and `tracing-appender` enter `Cargo.toml` as scaffolded deps with no runtime wiring. They light up in a future phase (Phase 19 OBS-06 may wire `tracing-error::ErrorLayer` for `tome doctor`; v1.0 Tauri IPC wires `tracing-appender` for log file capture).
    ```

    DO NOT touch the `## [0.10.0] - 2026-05-11` section below — those entries are historical and frozen.

    DO NOT create a `## [0.11.0]` heading yet — the v0.11 release date is set by Phase 19 when the milestone cuts. Phase 18 entries live under `## [Unreleased]` until that point. Phase 19 will rename the heading when the release ships.
  </action>
  <verify>
    <automated>rg -n "tracing" CHANGELOG.md | head -5 &amp;&amp; rg -c "OBS-0" CHANGELOG.md</automated>
  </verify>
  <acceptance_criteria>
    - `rg "## \\[Unreleased\\]" CHANGELOG.md` returns 1 match
    - `rg "OBS-01" CHANGELOG.md` returns ≥ 1 match
    - `rg "OBS-02" CHANGELOG.md` returns ≥ 1 match
    - `rg "OBS-03" CHANGELOG.md` returns ≥ 1 match
    - `rg "OBS-04" CHANGELOG.md` returns ≥ 1 match
    - `rg "OBS-05" CHANGELOG.md` returns ≥ 1 match
    - `rg "TOME_LOG" CHANGELOG.md` returns ≥ 1 match
    - `rg "PreviouslyFailed" CHANGELOG.md` returns ≥ 1 match (the deferred cause is called out)
    - `rg "time\\.busy" CHANGELOG.md` returns ≥ 1 match (the `elapsed_ms` mapping note)
    - `rg "## \\[0\\.10\\.0\\]" CHANGELOG.md` returns 1 match (the prior section header is preserved, not duplicated or renamed)
  </acceptance_criteria>
  <done>CHANGELOG.md `[Unreleased]` section documents Phase 18's OBS-01..OBS-05 deliverables, the D-ENV-1 `--quiet`-vs-`TOME_LOG` trade-off, the `time.busy`-vs-`elapsed_ms` naming clarification, and the PreviouslyFailed deferral; v0.10.0 section unchanged.</done>
</task>

</tasks>

<verification>
After both tasks land:

1. **Full quality gate:**
   ```
   cargo fmt -- --check
   cargo clippy --all-targets -- -D warnings
   cargo test -p tome
   ```
   All three exit 0.

2. **Success criterion 1 anchor — byte-identical stdout:**
   ```
   cargo test -p tome --test cli_status
   cargo test -p tome --test cli_list
   cargo test -p tome --test cli_doctor
   ```
   All exit 0, NO snapshot diffs.

3. **OBS-03 regression test passes:**
   ```
   cargo test -p tome --test cli_sync sync_verbose_emits_step_spans_on_stderr
   ```
   Exits 0.

4. **End-to-end smoke:**
   ```
   cargo run -p tome -- sync --verbose 2>&1 | rg "(discover|reconcile|consolidate|distribute|cleanup)" | rg "(close|time\.busy)"
   ```
   Returns ≥ 3 lines (the 3 always-firing step spans + their close events).

5. **CHANGELOG entry verifiable:**
   ```
   rg "OBS-0[1-5]" CHANGELOG.md | wc -l
   ```
   Returns ≥ 5 (one per OBS requirement).
</verification>

<success_criteria>
- Plan 18-03 verification test pins OBS-03 span emission for future regression-catching
- `cli_status`, `cli_list`, `cli_doctor` snapshot tests remain green (success criterion 1 — byte-identical stdout for these commands)
- `CHANGELOG.md` `[Unreleased]` section describes the five OBS-* changes plus the two release-noted trade-offs and the PreviouslyFailed deferral
- `cargo fmt`, `cargo clippy --all-targets -- -D warnings`, `cargo test -p tome` all pass
- Phase 18 work is complete and ready for `/gsd:verify-work` + commit + push
</success_criteria>

<output>
After completion, create `.planning/phases/18-observability-foundation-sync-diagnostics/18-03-SUMMARY.md` summarizing:

- The new integration test name + which step spans it asserts on
- Whether status/list/doctor snapshots needed any re-baselining (expected: NONE; if any did, that's a bug to fix not accept)
- CHANGELOG entry placement (under `[Unreleased]` — left for Phase 19 to rename to `[0.11.0] - YYYY-MM-DD` at release time)
- Notes for the `/gsd:verify-work` step: which goal-backward truths to prioritize verifying first (recommendation: byte-identical stdout for status, then OBS-05 reconcile line presence, then OBS-04 cause grep, then OBS-03 time.busy grep, then OBS-01/02 substrate sanity via `--help`).
</output>
