## 1. Contract foundations

- [ ] 1.1 Choose the canonical on-disk suite serialization and document its schema-versioning and fixture-directory rules.
- [ ] 1.2 Add Rust domain types for artifact identity, cases, execution policy, variants, normalized observations, grader outcomes, and result states.
- [ ] 1.3 Add parsing, validation, and round-trip tests for a minimal suite and invalid version/capability declarations.
- [ ] 1.4 Define the versioned result document and add serialization fixtures covering complete, partial, timed-out, permission-denied, and non-comparable runs.

## 2. Deterministic local evaluation core

- [ ] 2.1 Implement isolated temporary-workspace materialization that snapshots the selected artifact by content hash without mutating source, library, target, manifest, or lockfile state.
- [ ] 2.2 Implement normalized deterministic graders for final text, artifact paths/content, event occurrence/order, artifact allowlists, and activation/non-activation evidence.
- [ ] 2.3 Add focused tests proving missing runner capabilities yield `not_comparable`, rather than a silent pass or fail.
- [ ] 2.4 Implement repetition and aggregation, including treatment/control matching, weighted score calculation, and delta withholding for non-comparable variants.
- [ ] 2.5 Add fixture-driven tests for positive activation, negative routing, equal treatment/control outcomes, and a partial result that cannot produce delta.

## 3. Runner adapters

- [ ] 3.1 Define the adapter trait and capability declaration boundary, keeping raw runner events optional evidence rather than core grader input.
- [ ] 3.2 Implement a deterministic fake adapter for unit and integration tests.
- [ ] 3.3 Spike a `CodexExecAdapter` that materializes a scratch workspace, invokes `codex exec --json`, and normalizes supported JSONL observations.
- [ ] 3.4 Verify Codex adapter behavior for explicit trigger, implicit trigger, negative trigger, and no-skill control cases using only disposable fixtures.
- [ ] 3.5 Evaluate a Claude adapter separately: map only portable suite concepts to `claude plugin eval`, preserve the Tome result contract, and document unmappable semantics.

## 4. Optional semantic grading and user interfaces

- [ ] 4.1 Define a pluggable semantic-judge provider contract with explicit opt-in, pinned provider/model metadata, structured evidence, and usage/cost reporting.
- [ ] 4.2 Add a CLI surface that requires explicit runner and side-effect permissions and defaults to no remote model execution.
- [ ] 4.3 Produce a local human-readable report from the versioned result document and retain full traces/workspaces only by explicit policy.
- [ ] 4.4 Add CI guidance that pins runner/judge versions, fails deterministically on declared policy, and keeps partial/non-comparable results out of trend gates.

## 5. Release readiness

- [ ] 5.1 Run unit, integration, formatting, lint, and documentation checks for the selected implementation cut.
- [ ] 5.2 Review the feature against the no-mutation, isolation, cost/permission, and activation-does-not-inflate-delta requirements.
- [ ] 5.3 Update the issue, roadmap placement, and OpenSpec artifacts with the verified implementation scope and any deferred adapter work.