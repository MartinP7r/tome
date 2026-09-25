## ADDED Requirements

### Requirement: Versioned provider-neutral evaluation suite
Tome SHALL define a versioned evaluation suite contract that identifies the artifact under test by content hash and describes cases, execution policy, graders, and artifact-retention policy without requiring a vendor plugin manifest, vendor eval directory, or vendor trace format.

#### Scenario: Same suite targets two runners
- **WHEN** a suite identifies a skill artifact by immutable content hash and declares only provider-neutral cases and graders
- **THEN** a Codex adapter and a Claude adapter can each materialize their runner-specific inputs without changing the suite's case definitions

#### Scenario: Evaluation does not mutate a managed skill
- **WHEN** an evaluation runs for a skill from Tome's canonical library
- **THEN** the runner receives an isolated materialization or read-only reference and the source, library, target, manifest, and lockfile remain unchanged

### Requirement: Reproducible case execution policy
Each case SHALL define a natural-language prompt and MAY declare immutable fixtures, resource inputs, execution constraints, tags, and expected human-readable outcome. The suite execution policy SHALL define repetition count, variant selection, model/runner pins when applicable, time and cost ceilings, and allowed side-effect capabilities.

#### Scenario: Matched variants receive the same case inputs
- **WHEN** a case is evaluated with treatment and control variants
- **THEN** each comparable run uses the same prompt, fixture identity, repetition count, declared permissions, and execution limits except for availability of the artifact under test

#### Scenario: Unsafe setup is not implicit
- **WHEN** a case declares a fixture setup script or real external service requirement
- **THEN** the runner SHALL require an explicit execution-policy capability/approval before running it

### Requirement: Isolated execution and declared limitations
A runner adapter SHALL produce every run in a fresh declared workspace and report its isolation level, side-effect policy, runner identity/version, and any unavailable requested capabilities. A runner SHALL NOT claim a case comparison is valid when it cannot meet a policy-required isolation or capability condition.

#### Scenario: Missing event trace capability
- **WHEN** a case has a command-event grader and the selected runner does not expose normalized command events
- **THEN** the grader result is `not_comparable` with a capability explanation rather than a pass or fail

#### Scenario: External services are not started by default
- **WHEN** a case does not explicitly permit real external services
- **THEN** the runner SHALL NOT start a plugin hook, MCP server, or equivalent external process merely because the evaluated artifact declares one

### Requirement: Normalized run observations
Each run SHALL record a terminal execution state, duration, runner metadata, artifact identity, and any available final response and workspace artifact manifest. The normalized observation model SHALL support optional command events, tool events, file content snapshots, usage estimates, activation evidence, and sandbox/permission facts with declared provenance.

#### Scenario: Runner trace is normalized without exposing vendor events to graders
- **WHEN** a Codex adapter receives JSONL command events
- **THEN** it exposes normalized command observations for Tome graders while retaining adapter-specific raw data only as optional evidence

#### Scenario: Incomplete run preserves evidence
- **WHEN** a run reaches a time limit after creating files
- **THEN** the observation records `timed_out` and preserves the available artifact manifest and grader evidence without reporting the run as completed

### Requirement: Deterministic and optional semantic graders
Tome SHALL support deterministic graders over normalized observations, including final text, artifact paths/content, command or tool events, ordering, artifact allowlists, and activation evidence. Semantic judge graders SHALL be explicit optional providers and SHALL return a structured verdict plus evidence and provider/model identity.

#### Scenario: Deterministic artifact assertion
- **WHEN** a case requires a generated file at a declared path with matching content criteria
- **THEN** the deterministic grader evaluates the normalized workspace observation without a model call

#### Scenario: Semantic judge is not implicitly invoked
- **WHEN** a suite contains only deterministic graders
- **THEN** an evaluation SHALL NOT make an additional model call for grading

### Requirement: Treatment/control contribution measurement
The evaluation policy SHALL support treatment and control variants. Treatment makes the artifact under test available; control makes it unavailable. Tome SHALL compute a contribution delta only from comparable score-bearing graders and SHALL report per-variant scores independently.

#### Scenario: High success without contribution
- **WHEN** treatment and control variants both meet the same outcome graders with equal scores
- **THEN** the aggregate reports zero delta and does not claim the artifact caused the task success

#### Scenario: Non-comparable variants withhold delta
- **WHEN** a control run is missing required fixtures, uses a different pinned model, or cannot meet a required runner capability
- **THEN** the result marks the comparison non-comparable and omits delta rather than deriving one from unequal runs

### Requirement: Activation evidence does not inflate contribution
Activation evidence SHALL record whether the artifact was selected, invoked, or otherwise observed through runner-provided provenance. In treatment/control evaluation, a positive activation assertion SHALL default to unscored evidence because it cannot be satisfied by a control variant. A negative-routing assertion requiring no activation SHALL be scoreable in both variants.

#### Scenario: Positive activation remains diagnostic evidence
- **WHEN** treatment activates a skill and control cannot because the skill is absent
- **THEN** the result records the activation outcome but excludes it from both variants' comparative score

#### Scenario: Negative routing is comparable
- **WHEN** a case requires that an unrelated prompt not activate the skill
- **THEN** the no-activation assertion is scored in both treatment and control variants and can affect delta

### Requirement: Auditable aggregate and partial result semantics
Tome SHALL produce a versioned result document containing suite and artifact identity, normalized per-run observations, per-grader results, variant aggregates, retention locations, and estimated usage/cost when a runner exposes them. Terminal states and grader states SHALL distinguish ordinary failures from timeouts, cancellation, cost limits, permission denial, runner errors, skipped graders, and non-comparable evidence.

#### Scenario: Cost-limited suite is not charted as a regression
- **WHEN** a run is stopped by a configured cost ceiling or skips a required paid grader
- **THEN** the result is marked partial or non-comparable with the reason and SHALL NOT produce a treatment/control delta

#### Scenario: CI consumer reads forward-compatible result
- **WHEN** a CI consumer reads a result document from a later compatible minor schema revision
- **THEN** it can rely on declared schema version and stable aggregate/terminal-state fields while ignoring unrecognized optional fields
