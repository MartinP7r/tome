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

## Cross-Milestone Trends

| Metric | v0.6 |
|--------|------|
| Phases | 3 |
| Plans | 11 |
| Tasks | ~19 |
| Timeline | 2 days |
| Known gaps | 5 (WIZ-01–05) |
| Critical bugs found in review | 3 |
