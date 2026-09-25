## Why

Tome can validate skill structure and distribute skills, but it cannot show whether a skill changes agent behavior, improves a task outcome, or introduces regressions. Claude Code now bundles plugin evals and Codex documents a JSONL-based harness pattern; both demonstrate useful mechanics, but neither should become Tome's canonical dependency or format.

Tome needs a runner-independent evaluation contract so the same skill cases, scoring policy, and results can be used with different agent harnesses without mutating the canonical library or coupling the product to one vendor.

## What Changes

- Define a versioned, Tome-owned evaluation suite and result contract for evaluating a skill or skill set against realistic prompts.
- Define treatment/control execution semantics so a skill's contribution is reported separately from raw task success.
- Define normalized observations and deterministic graders that are independent of a particular agent's trace format.
- Define adapter boundaries for execution backends; Codex JSONL and Claude plugin eval are optional adapters, not core dependencies.
- Establish safety, reproducibility, and partial-result rules for isolated runs, fixtures, permissions, costs, timeouts, and artifact retention.
- Produce a design-only first cut: no new `tome` command, no automatic model calls, no CI gate, and no mutation of the canonical skill library.

## Capabilities

### New Capabilities

- `skill-evaluation-contract`: Defines the provider-neutral evaluation suite, observation, grading, treatment/control, and result semantics that a future Tome evaluation feature must implement.

### Modified Capabilities

- None.

## Impact

- New OpenSpec capability and design artifacts only in this change.
- Future implementation will likely add a Rust domain module, a versioned on-disk suite format, deterministic local graders, isolated workspace management, and runner adapters.
- No changes to `tome sync`, existing distribution behavior, lockfile/manifest semantics, or installed skill layout are proposed in this change.
- External runners such as `codex exec --json` and `claude plugin eval` remain optional, invoked only through explicit future adapter configuration and user-approved execution.