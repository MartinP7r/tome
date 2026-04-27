# Retrospective

## Milestone: v0.6 — Unified Directory Model

**Shipped:** 2026-04-16
**Phases:** 3 | **Plans:** 11 | **Tasks:** ~19

### What Was Built
- Unified `[directories.*]` config model replacing separate sources/targets
- Pipeline rewrite (discover, distribute) with manifest-based circular prevention
- Git source cloning with shallow fetch, ref pinning, and SHA recording in lockfile
- Per-directory skill filtering (`enabled`/`disabled` in machine.toml)
- CLI commands: `tome add`, `tome remove`, `tome reassign`, `tome fork`
- Browse TUI: adaptive theming, fuzzy match highlighting, scrollbar, markdown preview, help overlay

### What Worked
- **Parallel worktree execution**: Both Phase 3 plans ran simultaneously in isolated git worktrees, completing in ~10 min each instead of ~20 sequential
- **Plan/render/execute pattern**: Introduced in `remove.rs`, reused identically for `reassign.rs` — clean separation of planning, display, and filesystem mutation
- **Per-phase verification**: Each phase verified independently (9/9, 13/13, 9/9 must-haves) catching issues before they compounded
- **PR review as quality gate**: Post-execution review caught 18 issues including 3 critical (silent no-op, fail-open confirmation, bypass safety)

### What Was Inefficient
- **Wizard rewrite descoped silently**: Phase 1 planned 5 plans but only 3 executed. WIZ-01–05 were never addressed and the ROADMAP showed "3/5 plans" without clear indication the remaining 2 were deliberately skipped
- **REQUIREMENTS.md drift**: BROWSE-01–04 were shipped but checkboxes never updated; traceability table had 9 "Pending" entries for shipped work. Caught only at milestone close
- **Planning doc merge conflicts**: Both parallel agents updated STATE.md and ROADMAP.md, requiring manual conflict resolution every time. These files should be updated only by the orchestrator

### Patterns Established
- `plan/render/execute` for destructive CLI commands with `--force`, `--dry-run`, and TTY detection
- Git subprocess env clearing: every `Command::new("git")` chains `.env_remove` for isolation
- Theme computed once at app init (not per render frame) for TUI performance
- Derived styles via methods (not stored fields) to prevent theme consistency bugs
- Non-interactive mode fails safe (requires `--force`) for all destructive operations

### Key Lessons
- **Parallel agents need disjoint write sets**: Code files merged cleanly; planning docs didn't. Future phases should either serialize planning doc updates or have the orchestrator own them
- **Verification checkboxes must auto-update**: The gap between "code ships" and "requirement marked done" is a manual step that gets missed. Consider automating traceability updates in the verification step
- **PR review finds what verification misses**: Phase verification checks "does the code do X?" but not "does the code handle edge Y safely?" — both gates are valuable

## Milestone: v0.8 — Wizard UX & Safety Hardening

**Shipped:** 2026-04-27 (v0.8.0 on 2026-04-26 + v0.8.1 hotfix on 2026-04-27)
**Phases:** 3 (7, 8, 8.1) | **Plans:** 10 | **Tasks:** ~26

### What Was Built
- Wizard handles greenfield, brownfield, and legacy machine states (WUX-01..05) — `tome init` no longer silently overwrites valid configs or ignores legacy `~/.config/tome/config.toml` files
- `tome remove` aggregates partial-cleanup failures into a typed `Vec<RemoveFailure>` and surfaces them via grouped stderr summary + non-zero exit (SAFE-01 / #413)
- `tome browse` `open` and `copy path` actions work on Linux via `xdg-open` + `arboard` — replaces the macOS-only `sh -c | pbcopy` invocation that was also a command-injection vector (SAFE-02 / #414)
- `relocate.rs` `read_link` failures surface as stderr warnings instead of silent `.ok()` drops (SAFE-03 / #449)
- v0.8.1 hotfix (Phase 8.1): offline `resolved_paths_from_lockfile_cache` helper restores git-skill provenance after Remove/Reassign/Fork; `Command::Remove` save chain reordered so partial-failure ⚠ block surfaces before save errors; failure-summary wording reworded (HOTFIX-01..03 / #461)

### What Worked
- **Hotfix discipline held**: Phase 8.1 was scoped tightly to 3 plans / 3 waves / 3 sequential commits per task. No scope creep into "while we're in there" cleanup. v0.8.1 shipped exactly what #461 captured.
- **Byte-for-byte snapshot tests for "this code path doesn't write to disk"**: HOTFIX-02's integration test reads `tome.toml` / `.tome-manifest.json` / `tome.lock` bytes pre/post and `assert_eq!` — the right shape for proving non-mutation, beats logical assertions that miss timestamp/whitespace re-emit.
- **Manual test-revert sanity check**: Each hotfix integration test was manually verified to FAIL when the production fix is reverted. Confirms tests actually exercise the bug, not just structurally pass.
- **Decimal phase numbering**: Inserting Phase 8.1 as a hotfix between Phase 8 and Phase 9 worked cleanly — preserves linear roadmap reading while making the patch-release relationship explicit.
- **Auto-applied rustfmt as separate commit**: Wave 3's executor committed `style(08.1-03): rustfmt wrap` separately from the test commit. Clean diff, behavior unchanged, no `--no-verify` shenanigans.

### What Was Inefficient
- **Branch-drift via stray-branch creation**: Two of three Wave 3 executor agent calls created and silently switched to a stray `gsd/phase-01-unified-directory-foundation` branch and committed phase artifacts there instead of the active phase branch. Recovered both times via `git merge --ff-only` + `git branch -d`, but the orchestrator had to detect and clean up. Root cause: `gsd-tools commit` infers branch policy from STATE.md, and post-`phase complete` STATE.md cleared the active phase, defaulting to phase 01. Worth filing upstream.
- **HOTFIX-01/02/03 not in REQUIREMENTS.md**: By design (project decision in STATE.md), but the verifier and `requirements mark-complete` paths kept hitting `not_found` and surfacing it as a question. Could be smoother — either a formal "hotfix without REQ" workflow or skip-flagging.
- **`/gsd:complete-milestone` workflow vs project tag policy**: The workflow's `git_tag` step conflicts with this project's `make release` ownership of tagging. Resolved by skipping the step, but the workflow doesn't have a config flag for this — had to recognize the conflict via stored memory and pause for user confirmation.
- **`typos` CLI not in dev-env path**: `make ci` includes `typos` but it wasn't installed locally. Each wave that ran `make ci` had to install it first. Worth a one-time `cargo install typos-cli` + Makefile bootstrap target.
- **Linux runtime UAT items deferred indefinitely**: 2 items in `08-HUMAN-UAT.md` (clipboard / xdg-open runtime) accepted as carry-over for two consecutive milestone closures. Either accept-as-carry-over with explicit closure semantics or actually run the test.

### Patterns Established
- **Lockfile-as-cache for offline-only operations**: When destructive commands need data that `sync()` would normally fetch online, read the previous lockfile + check on-disk artifacts. Better than empty default (silent drop) or running the online resolver (network from local commands).
- **Save-chain ordering as user-facing contract**: When `Result<T>` flows through multiple `?` gates with side-effecting println before/after, the order of those println relative to `?` *is* the contract. Failure-state messaging must come before any `?` that could short-circuit it.
- **`chmod 0o500` (not `0o000`) for partial-failure fixtures**: `0o000` makes `read_dir` itself fail, so `plan()` bails before `execute()` runs the partial-failure loop. `0o500` lets enumeration succeed but blocks `unlink`, hitting the actual code path under test.

### Key Lessons
1. **Empty-default is the laziest bug pattern**: `BTreeMap::new()` passed where `discover_all` expects resolved paths is syntactically valid but semantically a stand-in saying "I didn't do the work." HOTFIX-01 lived through Phase 8 review because no integration test crossed the destructive-command + git-source seam end-to-end. New rule: if a function takes a "context" map populated by a sibling caller, write at least one test exercising every caller with a populated map.
2. **Phase complete ≠ milestone complete**: v0.8.0 shipped, then post-merge re-review surfaced 3 findings worth a patch release. Build the planning model so a "v0.X.0 ships → review → v0.X.1 hotfix → milestone closes" flow is normal and not exceptional.
3. **Verifier deviations matter**: Wave 1's executor surfaced a worthwhile deviation (asserting on `git_commit_sha` instead of `source_name` per plan) because the bug actually lives in the SHA fallback, not the source-name field. Plans describe intent; the executor reasoning about the bug catches plan-error cases.
4. **Spawned agents can switch branches silently**: Always verify branch state after each agent return when running parallel/sequential executors. Spot-check via `git branch --show-current` plus reflog if drift is suspected.

### Cost Observations
- Model mix: orchestrator on Sonnet 4.6, executors on Opus, verifier on Sonnet — typical balanced profile
- Phase 8.1 wall time: ~16 min for 3 sequential waves + verification (vs ~30 min projected for 3 plans)
- Integration test revert-and-rerun sanity checks added ~3 min per plan but caught 0 false-positives this milestone — still worth it as a discipline

## Cross-Milestone Trends

| Metric | v0.6 | v0.8 |
|--------|------|------|
| Phases | 3 | 3 (7, 8, 8.1) |
| Plans | 11 | 10 |
| Tasks | ~19 | ~26 |
| Timeline | 2 days | ~5 days incl. hotfix |
| Known gaps | 5 (WIZ-01–05) | 2 (Linux UAT carry-over) |
| Critical bugs found in review | 3 | 3 (#461 H1/H2/H3) |
| Hotfix release | — | v0.8.1 (3 fixes) |

**Recurring pattern:** Post-merge review consistently catches issues that phase verification misses — both as a quality gate. v0.6 had a 3-issue PR review; v0.8 had a 3-issue post-merge re-review. Worth formalizing as a phase exit criterion.
