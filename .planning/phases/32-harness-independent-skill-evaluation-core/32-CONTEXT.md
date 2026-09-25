# Phase 32: Harness-independent skill-evaluation core — Context

**Gathered:** 2026-09-25
**Status:** Ready for execution
**Issue:** [#604](https://github.com/MartinP7r/tome/issues/604)
**OpenSpec:** `harness-independent-skill-evals`

## Phase boundary

Build Tome's first executable, provider-neutral evaluation core. It evaluates supplied normalized observations with deterministic graders and can drive a deterministic fake adapter in isolated temporary directories. It is deliberately a library-only foundation: no `tome eval` command, provider subprocess, model call, semantic judge, CI gate, or canonical-library mutation.

The on-disk contract is:

- author-authored suite: `evals/<suite-id>/suite.toml` plus `fixtures/<case-id>/...`;
- produced result: versioned JSON;
- fixture paths are relative to the suite directory and must not escape it;
- artifact identity is the existing `ContentHash`, so every result names the exact evaluated content.

## Locked decisions

1. **TOML suites; JSON results.** TOML matches Tome configuration and is review-friendly. JSON results support nested evidence and forward-compatible consumers.
2. **No new dependencies.** Reuse `serde`, `toml`, `serde_json`, `sha2`, and `tempfile`.
3. **Core does not invoke an agent.** The first runner is a deterministic fake adapter that returns supplied observations. Codex and Claude remain later adapters.
4. **Missing evidence is non-comparable.** A grader that needs unavailable command/tool/file evidence returns `not_comparable`, never a false pass/fail.
5. **Activation is diagnostic by default.** Positive activation may be recorded but never contributes to treatment/control score or delta. Negative-routing assertions can be comparative.
6. **Delta requires matched variants.** Different repetitions, model pin, fixture identity, limits, isolation, score-bearing grader set, or incomplete required run yields an explicit no-delta reason.
7. **Run materialization is isolated.** Each fake run gets a fresh temporary workspace. Treatment receives a copied, hash-verified artifact; control does not. Fixture and artifact sources remain unmodified.

## Deferred

- CLI/reporting/retention UX and CI gating.
- Codex, Claude, Hermes, OpenCode, and raw-trace adapters.
- Semantic judges, provider credentials, cost accounting, and model calls.
- Fixture setup scripts, hooks, MCP, external services, and user permission UX.
- Multi-skill/routing installation semantics and a universal vendor-event taxonomy.

## Verification contract

The phase is complete only when the core's unit/integration tests prove suite validation, result round-trips, deterministic graders, diagnostic-only positive activation, scoreable negative routing, withheld delta for incomplete or incompatible variants, and no mutation of source artifact/fixture/library state. Required final gates: `make ci`, `openspec validate harness-independent-skill-evals --strict`, and `git diff --check`.
