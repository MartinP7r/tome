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

## Milestone: v0.9 — Cross-Machine Config Portability & Polish

**Shipped:** 2026-04-29 (v0.9.0)
**Phases:** 2 (9 + 10) | **Plans:** 6 | **Tasks:** ~26

### What Was Built

- **Cross-machine portability** (#458) — `[directory_overrides.<name>]` schema in `machine.toml`, `Config::apply_machine_overrides` between `expand_tildes()` and `validate()`, threaded through Sync/Status/Doctor/Init via `Config::load_with_overrides`. Typo guard (stderr `warning:`), distinct `machine.toml` error class for override-induced validation failures, `(override)` annotation in `tome status`/`tome doctor` text + `override_applied: true` in JSON.
- **TUI polish** (#463 D1-D3) — `StatusMessage` collapsed to `Success | Warning | Pending` enum with `body()`/`glyph()`/`severity()` accessors and `pub(super)` visibility; UI formats `"{glyph} {body}"` at render time. "Opening: <path>..." renders before `xdg-open`/`open` blocks via closure-callback redraw threading from `run_loop` → `handle_key` → `handle_view_source`. `ClipboardOccupied` auto-retries once with 100ms backoff.
- **Type-design polish** (#463 D4-D6) — `FailureKind::ALL` compile-enforced via exhaustive-match sentinel + const-len assert; `RemoveFailure::new` `debug_assert!(path.is_absolute())`; `arboard` patch-pinned with bump-review comment; dead `SkillMoveEntry.source_path` field removed.
- **Test coverage** (#462 P1-P5) — `status_message_from_open_result` 3-arm tests via synthetic `ExitStatus`; banner-absence assertion; retry-after-fix end-to-end pinning the I2/I3 retention contract; `regen_warnings` deferred until after success banner with source-byte regression test anchored to `Command::Remove` region.
- **Bonus:** Bare-slug `tome add planetscale/database-skills` expansion (PR #471, bundled in v0.9.0).

### What Worked

- **Forward-planning v1.0 in parallel.** While v0.9 phases were executing, the v1.0 Tauri GUI artifacts (`milestones/v1.0-{REQUIREMENTS,ROADMAP}.md`) were drafted independently. Reference made it from forward-plan to "ready-to-ratify" status by the time v0.9 closed. Saves a future planning session's fresh-start cost.
- **Wave 1 parallel execution in Phase 10.** Three plans, three modules, zero file overlap, ~32 min wall time vs ~80 min sequential. The planner's deliberate file-disjointness during plan boundary decisions (browse vs remove + lib + tests/cli.rs vs Cargo.toml + relocate) is what made true-parallel safe.
- **Plan-checker iteration 2 catches.** Twice this milestone the plan-checker caught issues the planner missed — Phase 9's `DirectoryConfig` struct-literal cascade (~38 sites compile-fail without explicit guidance) and Phase 10's `SkillMoveEntry.source_path` test assertions (3 tests compile-fail when the field is removed). Both would have surfaced as executor surprises mid-task; iter-2 review was the right place to catch them.
- **Memory-driven discipline pays off.** Three direct-to-main / stacked-PR / empty-squash incidents during Phase 9 setup — all caught either by the permission system or by the now-canonical pre-commit `git status -sb` check. Zero such incidents in Phase 10 setup or execution. The discipline is paying its way.

### What Was Inefficient

- **Phase 9 setup chain (3 PRs deep before plans landed on main).** `b8d7ac8` direct-to-main → #474 revert → #475 stacked-PR misroute → #476 empty squash → #477 clean cherry-pick. Five PRs to land what should have been one. Each recovery was correct *for the bug it knew about*, but each had a fresh GitHub-sequencing footgun the previous one didn't anticipate. The cumulative memory entry now has 3 layers of process-bug coverage; future sessions should skip the chain entirely.
- **`gsd-tools milestone complete` produced bogus stats.** Counted "11 phases / 36 plans / 78 tasks" (cumulative across the project's entire history, not v0.9-specific) and listed accomplishments from every prior phase including `RED:` as a literal line extracted from a TDD-style summary header. Required manual MILESTONES.md rewrite. Worth filing upstream.
- **v0.9-ROADMAP.md archive captured pre-collapse snapshot** (same v0.7/v0.8 issue) — needed manual patch to flip 🚧 → ✅ and update active-section header. Pattern is consistent across milestones; could be fixed by ordering: collapse current ROADMAP first, *then* let `milestone complete` archive it.

### Patterns Established

- **Forward-plan adjacent milestones during current execution.** `milestones/v1.0-{REQUIREMENTS,ROADMAP}.md` was drafted while v0.9 was still in flight. When v0.9 closed, the next milestone wasn't a blank slate. Worth carrying as a discipline: "if a future milestone has obvious shape, draft it speculatively while waiting on the current one's verification cycles."
- **Plan-check "removal" work is high-leverage.** Removing a struct field reverberates beyond the constructor sites — any code that READS the field also breaks. Plan-checker-iter-2 caught this twice (Phase 9 struct-literal cascade, Phase 10 source_path assertions). Could be turned into a planner heuristic: "when removing a struct field, grep for ALL references (constructions AND reads/asserts) and enumerate them in the plan."
- **Parallel-wave plan partitioning.** When a phase has ≥3 plans, optimize the file-disjointness of the partition, not just the logical grouping. Phase 10 Wave 1 nailed this — three plans across three modules with zero overlap, true-parallel safe.

### Key Lessons

1. **Verify the actual file landed, not just the merge.** PR #476 squash-merged successfully but produced an empty commit (`d22b3d8`) — the UI showed the right diff, the merge succeeded, but the destination tree was unchanged. After ANY recovery PR, `git ls-tree HEAD <expected_path>` is the only honest check. Added to memory.
2. **Branch-discipline doesn't just save direct-to-main commits — it saves planning-doc inconsistency too.** All v0.9 setup-chain incidents started with the same root cause: a subagent's `git checkout main` invisible to the orchestrator. The pre-commit `git status -sb` check is now the single most-load-bearing discipline this project has.
3. **Three milestone-closure runs deep, the workflow's `gsd-tools milestone complete` still produces wrong stats.** Don't trust the auto-generated MILESTONES.md entry — always rewrite it manually with milestone-scoped accomplishments + correct phase/plan/task counts.

### Cost Observations

- Model mix: orchestrator + planner on Opus, executors on Opus, verifier + plan-checker on Sonnet — typical balanced profile
- v0.9 wall time: ~3 days (Phase 9 over 1 day + Phase 10 in <1 day + setup-chain overhead)
- Phase 10 was the smoothest execution this entire milestone — proves the discipline accumulated through Phases 7/8/8.1/9 pays off when nothing breaks the chain

## Cross-Milestone Trends

| Metric | v0.6 | v0.8 | v0.9 |
|--------|------|------|------|
| Phases | 3 | 3 (7, 8, 8.1) | 2 (9, 10) |
| Plans | 11 | 10 | 6 |
| Tasks | ~19 | ~26 | ~26 |
| Timeline | 2 days | ~5 days incl. hotfix | ~3 days |
| Known gaps | 5 (WIZ-01–05) | 2 (Linux UAT carry-over) | 2 (Linux UAT carry-over still) |
| Critical bugs found in review | 3 | 3 (#461 H1/H2/H3) | 0 |
| Hotfix release | — | v0.8.1 (3 fixes) | — |
| Plan-checker iter-2 catches | n/a | n/a | 2 (struct-literal cascade, source_path assertions) |
| Process incidents (direct-to-main / stacked-PR / empty-squash) | 0 | 0 | 3 (Phase 9 setup chain) |

**Recurring pattern:** Post-merge review consistently catches issues that phase verification misses — both as a quality gate. v0.6 had a 3-issue PR review; v0.8 had a 3-issue post-merge re-review. v0.9 had ZERO post-merge findings — possibly because the v0.8 review tail (#462 + #463) was cleared in-milestone via Phase 10, depriving the next post-merge review of low-hanging items.

**New pattern:** Plan-checker iteration 2 ("removal-work miss" — fields removed without enumerating consumers) is now a recurring catch. Worth turning into a planner-skill heuristic.

**Discipline trend:** Process incidents went 0 → 0 → 3 → 0 (across v0.6, v0.7 unrecorded, v0.8, v0.9). The v0.9 spike was concentrated in Phase 9 setup; Phase 10 had zero. The branch-discipline memory entry is now load-bearing — every future session inherits it.
