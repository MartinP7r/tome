## Context

Tome currently discovers, validates, consolidates, and distributes `SKILL.md` packages. `tome lint` validates static structure, but no Tome-owned model exists for proving that a skill activates under realistic prompts, changes agent behavior, or improves the outcome compared with no skill.

Claude Code's `plugin eval` provides a native harness for plugin packaging: isolated child sessions, repeated runs, deterministic and model-judge graders, a no-plugin baseline, and JSON/HTML results. Codex documents the equivalent pattern through `codex exec --json`, leaving suite execution, trace parsing, grading, repetition, and reporting to the caller. These are useful execution backends, but their plugin layouts, trace formats, permissions, and output formats are not suitable as Tome's source of truth.

The initial design is intentionally architecture-only. It establishes an on-disk contract and domain boundaries before a CLI command, model runner, or CI integration exists.

## Goals / Non-Goals

**Goals:**

- Define one versioned suite format that represents skill evaluation independently of an agent vendor.
- Separate the artifact under test, the execution backend, normalized observations, grading, and aggregation.
- Preserve the important distinction between task correctness and marginal skill value through treatment/control runs.
- Make deterministic checks first-class and keep semantic judging explicitly optional.
- Preserve enough evidence to audit a score without treating partial, timed-out, or permission-constrained runs as comparable success/failure data.
- Allow a later runner adapter to target Codex, Claude, Hermes, OpenCode, or a deterministic fake runner without changing cases or result consumers.

**Non-Goals:**

- Implement `tome eval`, a runner adapter, automatic model calls, report rendering, or CI gating.
- Define a universal cross-agent tool vocabulary or require every runner to expose every observation.
- Mutate a source, library, target, plugin manifest, or checked-in skill to make it evaluable.
- Claim that a passing behavioral suite is a safety/security assessment of a skill, hook, MCP server, or external tool.
- Define a hosted evaluation service, credentials model, or price accounting across providers.

## Decisions

### D1 — Tome owns the suite and result contracts

A suite SHALL be TOML/YAML/JSON-serializable data stored beside test fixtures, not a Claude plugin `evals/` directory or a Codex-only script. It identifies an artifact by immutable content hash plus an optional source/provenance reference. A result records the suite schema version, artifact hash, runner identity/version, execution policy, observations, grader results, and aggregate.

**Rationale:** skill files and traces can evolve independently by vendor. Hashing the evaluated artifact makes an outcome attributable to the exact library content rather than a name that can be overwritten later.

**Alternatives considered:** adopt Claude's files directly (couples every skill to plugin packaging); parse Codex JSONL as the suite format (couples the contract to one runner); persist only a final score (cannot explain or reproduce a regression).

### D2 — The treatment/control pair is an execution policy, not a grader

A case describes a task. The policy selects one or more variants: at minimum `treatment` (artifact available) and optional `control` (artifact absent). The same prompt, fixtures, model configuration, permissions, and run count apply to every comparable variant. The aggregate reports each variant's score and `delta = treatment - control` only when the observations are comparable.

**Rationale:** a skill can have a high task-success score while adding no value because the base agent succeeds without it. Keeping the baseline outside grader semantics prevents activation-only evidence from manufacturing a delta.

**Alternatives considered:** no baseline (cannot measure contribution); baseline as a special `llm` grader (conflates execution and scoring); always require a baseline (unnecessarily doubles cost for fast local iteration).

### D3 — Normalized observations use capabilities, not a forced event schema

A runner produces a `RunObservation` with mandatory outcome state, final response when available, workspace artifact manifest, duration, and runner metadata. Optional capabilities expose normalized command events, tool events, file content snapshots, usage estimates, and sandbox/permission facts. Graders declare the observation capabilities they require; a run that lacks them is `not_comparable` for that grader rather than silently failing.

**Rationale:** Claude can expose `Skill` tool calls, Codex emits JSONL command events, and a deterministic runner may expose only files. A lowest-common-denominator trace would throw away valuable evidence, while a universal taxonomy would leak vendor details into core semantics.

**Alternatives considered:** require a full event trace from all adapters (excludes viable runners); treat absent observations as failed assertions (creates false regressions); preserve opaque adapter blobs only (makes portable graders impossible).

### D4 — Deterministic graders are core; semantic judges are a pluggable boundary

Core graders SHALL cover assertions over declared observations: text/regex, files/paths/content hash, command or tool event occurrence/order, artifact allowlists, and explicit activation/non-activation. A semantic judge is represented as an optional grader provider that returns a structured verdict and evidence; it is never implicitly enabled by a suite.

**Rationale:** deterministic checks are reproducible, cheap, and explanatory. Semantic quality is sometimes essential, but it is paid, non-deterministic, and provider-dependent.

**Alternatives considered:** model-judge everything (costly and unstable); omit semantic grading entirely (cannot evaluate many qualitative conventions); hard-code a Codex or Claude judge (breaks harness independence).

### D5 — Activation is evidence, not value

An activation grader records whether the target skill/artifact was selected or its declared behavior was observed. In a treatment/control comparison it defaults to unscored evidence, because a control run cannot activate an absent artifact. A negative-routing grader explicitly requiring no activation is comparable and can be scored in both variants.

**Rationale:** this preserves Claude's important anti-inflation behavior while generalizing it beyond a specific `Skill` tool trace.

**Alternatives considered:** count activation in all scores (inflates treatment delta); discard activation data (loses diagnosis of routing failures); infer activation only from output (unreliable and adapter-specific).

### D6 — Isolation and side-effect policy belong to the runner contract

Every run SHALL declare an isolation level and side-effect policy. The initial policy target is a fresh workspace per run, immutable copied/linked input artifact, no automatic real MCP services/hooks/scaffold scripts, and least-privilege tools. Any runner that cannot provide a declared control or isolation requirement MUST surface that limitation in the observation and mark affected comparisons non-comparable.

**Rationale:** reproducibility and safe defaults are product properties, not a convenience of a particular agent CLI. An agent sandbox does not contain externally started hooks or services.

**Alternatives considered:** trust runner defaults (inconsistent and unsafe); make all runners fully containerized (too heavy for a first contract); forbid all tools/services forever (prevents realistic future evaluations).

### D7 — Results model partiality and comparability explicitly

A run has one terminal state: `completed`, `failed`, `timed_out`, `cancelled`, `cost_limited`, `permission_denied`, or `runner_error`. Every grader separately records `pass`, `fail`, `skipped`, or `not_comparable`. An aggregate is eligible for comparison only when all policy-required variants and score-bearing graders completed comparably; otherwise it carries an explicit reason and SHALL not yield a delta.

**Rationale:** a model-rate-limit error, skipped paid judge, or missing trace capability must not look like an ordinary behavior regression.

**Alternatives considered:** use one boolean result (loses diagnosis); average available runs opportunistically (biases comparisons); discard all partial data (loses useful debugging evidence).

## Risks / Trade-offs

- **[IR becomes a second agent-skill standard]** → Keep v1 limited to evaluation concepts and adapter-neutral primitives; do not encode plugin installation, routing internals, or vendor configuration.
- **[Artifacts expose source data or prompts]** → Make retention configurable and default to summaries plus hashes; require explicit opt-in for full traces/workspaces.
- **[Control run is not genuinely equivalent]** → Record model, policy, fixture hash, runner version, and capabilities per variant; withhold delta when declared comparability is violated.
- **[Runner adapters make different activation claims]** → Define activation evidence provenance and confidence in the normalized observation; never infer it as a scoring requirement by default.
- **[Judge score variability makes trends noisy]** → Keep judges opt-in, pin provider/model/prompt revision in observations, and report deterministic and judge contributions separately.
- **[Fixture setup executes untrusted code]** → Treat fixture setup as a separately approved runner capability, not a passive suite property; do not enable it by default.

## Migration Plan

No existing on-disk format, command, or persisted state changes in this design-only change. A future implementation SHALL introduce the suite format behind an explicit new command and preserve all current `tome sync`, library, lockfile, and target behavior. Removing the future feature is a matter of deleting its optional suite/artifact directory; no migration of installed skills is required.

## Open Questions

- Which canonical serialization offers the clearest authoring and stable diff behavior: TOML, YAML, or a split manifest-plus-case-directory layout?
- Should a suite evaluate a single skill only in the first implementation, or can an artifact be an ordered skill set with a declared routing scope?
- What deterministic file-diff policy best protects host repositories while supporting fixture-driven code tasks?
- What minimum observation capability set must a runner provide before Tome accepts it for treatment/control comparisons?
- Should isolated execution first use temporary directories, git worktrees, or an explicit runner-selected workspace strategy?
- How should user-owned credential requirements be represented without recording secrets or encouraging unattended paid runs?