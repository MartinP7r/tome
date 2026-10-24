# Skill evaluation core

Tome's evaluation core is a provider-neutral library foundation for comparing a skill-enabled run with a control run. It defines a versioned suite format, normalized run contracts, deterministic graders, matched aggregation, and evidence-preserving result JSON.

This is not a user-facing CLI or a model/provider integration. The core does not launch a model, start a subprocess, install a skill, or itself enforce an operating-system sandbox. Applications supply a `RunnerAdapter` and a `WorkspaceProvider`; adapters must enforce the requested execution policy (including timeout, network, and permission constraints) and report the observed policy. The built-in deterministic fake is intended for tests.

## Suite files

A suite is a `suite.toml` file with `schema_version = 1`. The artifact identity must match the exact artifact directory passed to evaluation. Use Tome's `hash_artifact_directory` function to obtain the `sha256` value; it is a deterministic directory hash, not a hash of an archive. The example below uses a placeholder digest and must be replaced with the real value before evaluation.

```toml
{{#include examples/evaluation-suite.toml}}
```

Use `load_suite_file(&path)` to read and validate a file, or `parse_suite_toml(&text)` when the TOML is already in memory. Parsing rejects unsupported schema versions, unknown fields, invalid identifiers, duplicate case/grader IDs, unsafe relative paths, invalid grader configuration, and suites that exceed the run-count limits.

Case `fixture` paths and grader artifact paths are portable relative paths. Absolute paths, `..`, empty components, backslashes, and drive-letter paths are rejected. Fixture paths are resolved under the suite directory.

Supported deterministic grader kinds:

- `text_contains` and `text_regex` inspect normalized final text.
- `artifact_exists`, `artifact_sha256`, and `artifact_allowlist` inspect the returned artifact manifest.
- `event_count` and `event_order` inspect normalized events.
- `activation` checks whether activation evidence is available and matches the declared state.

Every grader declares the observation capability it needs. Missing capabilities yield `not_comparable`, rather than silently treating absent evidence as a failure or a pass.

## Execution and isolation

`evaluate_suite` validates the suite and hashes the source artifact before creating a workspace or calling the adapter. Each case, control/treatment variant, and repetition receives a distinct empty workspace. The core copies fixtures into the workspace and copies the skill artifact only for treatment runs; the canonical artifact and suite inputs are not passed to the adapter as writable paths. Symlinks and hash mismatches are rejected during materialization.

The `ExecutionPolicy` records the requested timeout, output-size limit, network policy, fresh-workspace requirement, permissions, and result-retention policy. Defaults are a 120-second timeout request, a 1 MiB output limit, disabled network, fresh workspaces, no additional permissions, and summary-only result retention. These are contract values: an adapter is responsible for enforcing runtime limits and returning the observed policy. The core validates normalized observation bounds after the adapter returns, but cannot interrupt a non-cooperating provider.

`RunRequest` contains identifiers, prompts, hashes, the optional model pin, and policy; it contains no source-library path or credential field. `RunObservation` carries normalized evidence, not provider-specific raw payloads. Free-form adapter error text is discarded so it cannot accidentally persist prompts, paths, or credentials.

## Comparison and results

Control and treatment runs are aggregated as matched observations. A score delta is emitted only when the runs are comparable and have score-bearing evidence. Failures, missing capabilities, model-pin mismatches, and policy mismatches are retained as explicit no-delta reasons instead of being turned into a misleading numeric result.

`EvaluationResult::to_json()` emits the versioned result document; `parse_result_json()` validates a document when reading it back. By default, results retain the final-text SHA-256 and grader evidence, but omit the raw final response. `RetentionPolicy::FullTrace` explicitly opts into storing normalized final text; use it only when the data is appropriate to persist. Result extensions must not shadow reserved schema fields.

The library returns JSON as a string and does not choose a persistence location. The caller owns result-file creation, retention, and access control.

## Minimal library flow

```rust,no_run
use std::path::Path;
use tome::evaluation::{load_suite_file, EvaluationResult};

let suite = load_suite_file(Path::new("suite.toml"))?;
// Implement RunnerAdapter and WorkspaceProvider for the chosen environment.
// The application then passes those implementations and the verified artifact
// directory to evaluate_suite(...), and may persist result.to_json() itself.
# let _: EvaluationResult = todo!();
# Ok::<(), anyhow::Error>(())
```

The repository's fake adapter and fresh-workspace provider are enabled only for tests or the `test-support` feature. No production provider or evaluation command is included in this core.
