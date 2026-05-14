---
phase: 19-doctor-status-surface-bugfix-bundle
plan: 04
type: execute
wave: 2
depends_on: [01]
files_modified:
  - crates/tome/src/browse/app.rs
  - crates/tome/src/backup.rs
autonomous: true
requirements: [FIX-02]
requirements_addressed: [FIX-02]

must_haves:
  truths:
    - "browse::app::tests::copy_path_retry_helper_returns_within_bound has its upper bound relaxed from 600ms to 2000ms with a `FLAKE-FIX (#511 / HARD-14)` rooted-cause comment naming arboard contention + the rejected clock-injection alternative"
    - "The relaxed bound test still catches a real regression (10×-retry regression hits ~1100ms, well inside 2000ms but above 600ms baseline)"
    - "backup::tests::push_and_pull_roundtrip flake is investigated (it has NO timing assertions per RESEARCH) and addressed per its actual root-cause class — NOT necessarily a relaxed-bound treatment (D-FLAKE-2 permits re-opening)"
    - "Both tests pass 100 consecutive runs locally at --test-threads=8"
  artifacts:
    - path: "crates/tome/src/browse/app.rs"
      provides: "Relaxed-bound assertion (2000ms) + multi-line root-cause comment naming arboard contention class and the rejected clock-injection alternative (D-FLAKE-3)"
      contains: "FLAKE-FIX"
    - path: "crates/tome/src/backup.rs"
      provides: "Either (a) relaxed bound + root-cause comment if the flake mechanism is timing-based, or (b) retry wrapper / test isolation fix if the root cause is git-subprocess transients (planner re-opens per D-FLAKE-2 if mechanism differs)"
      contains: "FLAKE-FIX"
  key_links:
    - from: "browse::app::tests::copy_path_retry_helper_returns_within_bound"
      to: "arboard clipboard contention root cause"
      via: "Multi-line `// FLAKE-FIX (#511 / HARD-14):` comment above the relaxed assert"
      pattern: "FLAKE-FIX.*#511"
---

<objective>
Close GitHub #511 (browse copy-path timing flake) AND the HARD-14 carry-over (`backup::tests::push_and_pull_roundtrip`) per D-FLAKE-2's pairing. The browse test is straightforward: relax the upper bound from 600ms to 2000ms + add a root-cause comment naming arboard clipboard contention + the rejected clock-injection alternative (D-FLAKE-3 — out of scope for v0.11 polish). The backup test is the **ambiguous fix item** — RESEARCH.md flagged it has NO timing assertions, so the D-FLAKE-1 relaxed-bound pattern doesn't directly apply. D-FLAKE-2 explicitly permits re-opening the decision if investigation reveals a different root-cause class.

Purpose: Eliminate the two known intermittent flakes that block clean CI runs. The browse fix is a 5-LOC change; the backup fix needs a reproduce-first step before committing to a shape.
Output: Browse test bound relaxed + multi-line root-cause comment; backup test addressed per its actual root cause (either retry wrapper, test isolation marker, or relaxed bound if a hidden timing assertion is found).
</objective>

<execution_context>
@$HOME/.claude/get-shit-done/workflows/execute-plan.md
@$HOME/.claude/get-shit-done/templates/summary.md
</execution_context>

<context>
@.planning/ROADMAP.md
@.planning/REQUIREMENTS.md
@.planning/phases/19-doctor-status-surface-bugfix-bundle/19-CONTEXT.md
@.planning/phases/19-doctor-status-surface-bugfix-bundle/19-RESEARCH.md
@crates/tome/src/browse/app.rs
@crates/tome/src/backup.rs

<interfaces>
<!-- Code anchors from RESEARCH.md (exact line numbers; executor confirms by content). -->

**`crates/tome/src/browse/app.rs:1782-1810`** — `copy_path_retry_helper_returns_within_bound` test:
```rust
#[test]
fn copy_path_retry_helper_returns_within_bound() {
    // ... happy path 5-500ms, retry path 100-600ms ...
    let elapsed = start.elapsed();
    assert!(elapsed < Duration::from_millis(600), "took {elapsed:?}");
}
```
Current bound: 600ms. New bound: 2000ms.

**`crates/tome/src/backup.rs:548-590`** — `push_and_pull_roundtrip` test. RESEARCH.md flagged: "No explicit timing assertions visible in the test. The flake mechanism is therefore NOT a timing bound — likely git-subprocess transient errors (filesystem timestamp resolution, network/file lock contention from parallel TempDir tests)." HARD-14 work in Phase 15 already added `setup_git_config(&repo_b)` (file line 548 in the test fixture).
</interfaces>
</context>

<tasks>

<task type="auto">
  <name>Task 1: Relax browse test bound 600ms → 2000ms + add multi-line root-cause comment</name>
  <files>crates/tome/src/browse/app.rs</files>
  <read_first>
    - crates/tome/src/browse/app.rs lines 1750-1830 (full context around the test — the existing inline comment at :1790-1795 documents the empirical 5-500ms happy path / 100-600ms retry path)
    - .planning/phases/19-doctor-status-surface-bugfix-bundle/19-CONTEXT.md (D-FLAKE-1, D-FLAKE-3)
    - .planning/phases/19-doctor-status-surface-bugfix-bundle/19-RESEARCH.md "FIX-02 (timing flake — closes #511 + HARD-14)" section (lines 486-517) — exact comment template
  </read_first>
  <action>
    Locate the `copy_path_retry_helper_returns_within_bound` test in `crates/tome/src/browse/app.rs` (search anchor: `fn copy_path_retry_helper_returns_within_bound` — RESEARCH-verified at line 1782, executor confirms).

    1. **Find the assert line** — currently `assert!(elapsed < Duration::from_millis(600), "took {elapsed:?}");` (or similar wording).

    2. **Replace `Duration::from_millis(600)` with `Duration::from_millis(2000)`** — leave the assert macro shape otherwise unchanged.

    3. **Insert the multi-line FLAKE-FIX comment immediately above the assert** (use the template from RESEARCH.md FIX-02 specifics verbatim):

       ```rust
       // FLAKE-FIX (#511 / HARD-14): bound relaxed from 600ms to 2000ms.
       // arboard clipboard contention under --test-threads=N can pause threads
       // ≫ 600ms regardless of helper performance — NSPasteboard / X11 clipboard
       // server / WinClipboard arbitration is opaque to user code. This assertion
       // guards against actual hangs (an unbounded retry `loop`), NOT perf
       // regressions. A 2000ms bound catches a 10×-retry regression while
       // tolerating realistic parallel-test contention.
       //
       // Deterministic clock injection (trait Clock in browse::app) was
       // considered but rejected for v0.11 scope (D-FLAKE-3). If this bound
       // flakes again post-fix, the abstraction can be introduced.
       assert!(elapsed < Duration::from_millis(2000), "took {elapsed:?}");
       ```

    4. **Do NOT** change any other line in the test (the existing empirical-breakdown comment at `:1790-1795` stays as-is — it documents the happy-path/retry-path timing model).

    5. **Run the test in isolation** to confirm it still passes locally:
       ```bash
       cargo test -p tome --lib browse::app::tests::copy_path_retry_helper_returns_within_bound
       ```

    6. **Optionally — run a 10-iteration stability check** (not a CI gate, just local sanity):
       ```bash
       for i in (seq 1 10); cargo test -p tome --lib browse::app::tests::copy_path_retry_helper_returns_within_bound --quiet; or break; end
       ```
       Should pass 10/10 consecutive runs locally.
  </action>
  <verify>
    <automated>cargo test -p tome --lib browse::app::tests::copy_path_retry_helper_returns_within_bound</automated>
  </verify>
  <acceptance_criteria>
    - `rg "Duration::from_millis\(2000\)" crates/tome/src/browse/app.rs` returns at least 1 match
    - `rg "Duration::from_millis\(600\)" crates/tome/src/browse/app.rs` returns 0 matches inside the `copy_path_retry_helper_returns_within_bound` test (other occurrences elsewhere in browse/app.rs are acceptable)
    - `rg "FLAKE-FIX \(#511 / HARD-14\)" crates/tome/src/browse/app.rs` returns 1 match
    - `rg "trait Clock" crates/tome/src/browse/app.rs` returns 0 matches in non-comment lines (D-FLAKE-3: clock injection is OUT OF SCOPE; mentioned only in the comment)
    - `cargo test -p tome --lib browse::app::tests::copy_path_retry_helper_returns_within_bound` exits 0
    - `cargo clippy --all-targets -- -D warnings` exits 0
  </acceptance_criteria>
  <done>Browse test bound is 2000ms; multi-line FLAKE-FIX comment names #511 + HARD-14 + arboard root cause + rejected clock-injection alternative; test passes locally.</done>
</task>

<task type="auto">
  <name>Task 2: Reproduce-first then fix backup::tests::push_and_pull_roundtrip per its actual root-cause class</name>
  <files>crates/tome/src/backup.rs</files>
  <read_first>
    - crates/tome/src/backup.rs lines 540-610 (the full push_and_pull_roundtrip test + setup_git_config helper from HARD-14)
    - .planning/phases/19-doctor-status-surface-bugfix-bundle/19-CONTEXT.md (D-FLAKE-2 — permits re-opening if root cause differs from D-FLAKE-1's relaxed-bound treatment)
    - .planning/phases/19-doctor-status-surface-bugfix-bundle/19-RESEARCH.md "FIX-02 sibling: backup::tests::push_and_pull_roundtrip (D-FLAKE-2)" section (lines 519-529)
  </read_first>
  <action>
    **CRITICAL — Reproduce-first step (per RESEARCH.md flag: "This is the most ambiguous FIX item"):**

    Before applying any fix, run the test 20-50 consecutive times under `--test-threads=8` to attempt to reproduce the flake locally:

    ```bash
    for i in (seq 1 50)
        cargo test -p tome backup::tests::push_and_pull_roundtrip -- --test-threads=8 --quiet
        if test $status -ne 0
            echo "FLAKE REPRODUCED on iteration $i"
            break
        end
    end
    ```

    Three possible outcomes:

    **Outcome A — Flake reproduces with a clear timing assertion failure (NOT expected per RESEARCH):**
    Apply D-FLAKE-1 treatment (relaxed bound + named-root-cause comment) following the same template as Task 1's browse fix. Comment shape:
    ```rust
    // FLAKE-FIX (HARD-14 carry-over): <bound> relaxed to <new value>.
    // <root cause class>. This assertion guards against actual hangs, NOT
    // perf regressions.
    ```

    **Outcome B — Flake reproduces but in a `git push`/`git pull` subprocess error (most likely per RESEARCH):**
    Add a retry wrapper around the failing git subprocess call. The pattern: retry 3 times with 50ms exponential backoff on transient failures. Example shape:
    ```rust
    fn git_with_retry(args: &[&str], cwd: &Path, retries: u8) -> Result<()> {
        let mut last_err = None;
        for attempt in 0..=retries {
            let status = Command::new("git").args(args).current_dir(cwd).status()?;
            if status.success() {
                return Ok(());
            }
            last_err = Some(anyhow!("git {} failed (attempt {})", args.join(" "), attempt + 1));
            std::thread::sleep(Duration::from_millis(50 * (1 << attempt)));
        }
        Err(last_err.unwrap())
    }
    ```
    Add a `// FLAKE-FIX (HARD-14):` comment above explaining the retry rationale (git push/pull contention under parallel TempDir tests; file-lock arbitration on .git/refs).

    Apply the retry wrapper to the SPECIFIC subprocess call that flakes — do NOT wholesale-wrap every git call in the test (risk: hiding a real failure).

    **Outcome C — Flake does NOT reproduce locally (also possible per RESEARCH "cannot easily reproduce the under-load failure"):**
    Add a defensive `// FLAKE-WATCH (HARD-14):` comment above the test documenting:
    - The flake history (HARD-14 added `setup_git_config(&repo_b)` to disable git signing in test fixtures)
    - That the flake could not be reproduced locally during Phase 19 (date + machine class)
    - That if it recurs, the next mitigation step is a retry wrapper around `git push`/`git pull` calls
    Do NOT change test logic if the flake doesn't reproduce. Closing #511 via the browse fix in Task 1 + this defensive comment is acceptable per D-FLAKE-2 ("If investigation reveals a different root cause class, planner re-opens this decision").

    **Document the decision in the task summary:**
    Record which outcome (A/B/C) the executor encountered + the chosen mitigation in `19-04-SUMMARY.md`. This is the most ambiguous FIX item; downstream verification needs to know which path was taken.

    **Regression test:** The existing test IS the regression test. No new test needed for FIX-02 per RESEARCH.md ("the existing test IS the regression test").
  </action>
  <verify>
    <automated>cargo test -p tome backup::tests::push_and_pull_roundtrip -- --test-threads=8</automated>
  </verify>
  <acceptance_criteria>
    - One of these patterns must exist in `crates/tome/src/backup.rs`:
      (a) `rg "FLAKE-FIX \(HARD-14" crates/tome/src/backup.rs` returns 1 match (Outcome A or B applied a real fix), OR
      (b) `rg "FLAKE-WATCH \(HARD-14" crates/tome/src/backup.rs` returns 1 match (Outcome C — defensive comment only, no test logic change)
    - `cargo test -p tome backup::tests::push_and_pull_roundtrip -- --test-threads=8` exits 0
    - `cargo clippy --all-targets -- -D warnings` exits 0
    - `.planning/phases/19-doctor-status-surface-bugfix-bundle/19-04-SUMMARY.md` (or a clearly marked subsection in this plan's summary) explicitly records which outcome (A/B/C) was encountered and the mitigation chosen
  </acceptance_criteria>
  <done>backup test addressed per its actual root-cause class; outcome documented in summary; clippy clean.</done>
</task>

</tasks>

<verification>
- `cargo test -p tome --lib browse::app::tests::copy_path_retry_helper_returns_within_bound` — passes
- `cargo test -p tome backup::tests::push_and_pull_roundtrip -- --test-threads=8` — passes
- Optional 50-iteration stability run (Fish loop) — passes on majority of iterations (if Outcome C, document any non-reproducible iterations in summary)
- `cargo clippy --all-targets -- -D warnings` — clean
- `cargo fmt -- --check` — clean
</verification>

<success_criteria>
- FIX-02: Both flaky tests pass cleanly at --test-threads=8; browse bound relaxed to 2000ms with multi-line root-cause comment
- D-FLAKE-2 honored: backup test is investigated and addressed per its actual root cause (not blindly relaxed-bound if no timing assertion exists)
- D-FLAKE-3 honored: clock injection (`trait Clock`) is NOT introduced — only mentioned in the rejected-alternative comment
- ROADMAP Success Criterion 3 first bullet: `copy_path_retry_helper_returns_within_bound` passes 100 consecutive runs at `--test-threads=8` (verified via local loop or CI run)
</success_criteria>

<output>
After completion, create `.planning/phases/19-doctor-status-surface-bugfix-bundle/19-04-SUMMARY.md` documenting:
- Outcome A/B/C encountered for the backup test (per RESEARCH ambiguity)
- Mitigation chosen and rationale
- Local stability-run results (X/50 passing iterations) for both tests
- Whether clock injection was tempted (and rejected per D-FLAKE-3)
</output>
