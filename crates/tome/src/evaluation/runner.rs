//! Provider-neutral orchestration contracts. No process or model is invoked here.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, ensure};

use crate::validation::ContentHash;

use super::aggregate::aggregate;
use super::contract::{
    ExecutionPolicy, FailureKind, MAX_ARTIFACT_MANIFEST_ENTRIES, MAX_EVENT_NAME_BYTES,
    MAX_EVENTS_PER_RUN, MAX_RESULT_EVIDENCE_BYTES, MAX_RESULT_EVIDENCE_REFS, ModelPin,
    ObservationCapability, RunObservation, RunnerIdentity, TerminalState, Variant,
    validate_model_pin, validate_safe_token,
};
use super::grader::grade_case;
use super::materialize::{
    hash_artifact_directory, hash_fixture_directory, materialize_run_workspace,
    validate_workspace_candidate,
};
use super::result::{EvaluationResult, RESULT_SCHEMA_VERSION, RunEvidence, RunRecord, text_hash};
use super::suite::{EvaluationCase, EvaluationSuite};

/// Stable identity for one case/variant/repetition.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct RunKey {
    pub case_id: String,
    pub variant: Variant,
    pub repetition: u32,
}

/// Request passed to a neutral adapter. It contains no source-library path or credentials.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RunRequest {
    pub suite_id: String,
    pub case_id: String,
    pub prompt: String,
    pub variant: Variant,
    pub repetition: u32,
    pub artifact_hash: ContentHash,
    pub fixture_hash: Option<ContentHash>,
    pub model_pin: Option<ModelPin>,
    pub policy: ExecutionPolicy,
}

impl RunRequest {
    pub fn key(&self) -> RunKey {
        RunKey {
            case_id: self.case_id.clone(),
            variant: self.variant,
            repetition: self.repetition,
        }
    }
}

/// Supplies a candidate workspace path. Implementations must not create it; the core validates
/// disjointness and creates the directory before materializing inputs.
pub trait WorkspaceProvider {
    fn workspace_path(&mut self, key: &RunKey) -> Result<PathBuf>;
}

/// Minimal contract for future providers. This core itself launches no process.
pub trait RunnerAdapter {
    fn identity(&self) -> &RunnerIdentity;
    fn capabilities(&self) -> &BTreeSet<ObservationCapability>;
    fn run(&mut self, request: &RunRequest, workspace: &Path) -> Result<RunObservation>;
}

/// Evaluate a suite with an injected adapter and workspace provider.
/// Artifact identity is checked before any adapter invocation; each run gets
/// copied inputs in a fresh workspace, never the canonical source path.
pub fn evaluate_suite<W, R>(
    suite_root: &Path,
    artifact_source: &Path,
    suite: &EvaluationSuite,
    workspaces: &mut W,
    adapter: &mut R,
) -> Result<EvaluationResult>
where
    W: WorkspaceProvider,
    R: RunnerAdapter,
{
    suite.validate()?;
    let runner_identity = adapter.identity().clone();
    validate_runner_identity(&runner_identity)?;
    let actual_artifact_hash = hash_artifact_directory(artifact_source)
        .with_context(|| format!("failed to hash artifact '{}'", artifact_source.display()))?;
    ensure!(
        actual_artifact_hash == suite.artifact.sha256,
        "artifact hash does not match suite identity"
    );

    let mut fixture_hashes = BTreeMap::new();
    for case in &suite.cases {
        if let Some(relative) = &case.fixture {
            fixture_hashes.insert(
                case.id.clone(),
                hash_fixture_directory(suite_root, relative)?,
            );
        }
    }

    let mut seen_workspaces = BTreeSet::new();
    let mut total_evidence_bytes = 0_u64;
    let mut total_grader_evidence_refs = 0_usize;
    let mut runs = Vec::new();
    for case in &suite.cases {
        for variant in Variant::ALL {
            for repetition in 0..suite.repetitions {
                let key = RunKey {
                    case_id: case.id.clone(),
                    variant,
                    repetition,
                };
                let candidate = workspaces.workspace_path(&key)?;
                let canonical_workspace =
                    validate_workspace_candidate(&candidate, suite_root, artifact_source)?;
                ensure!(
                    seen_workspaces.insert(canonical_workspace.clone()),
                    "workspace provider reused a workspace"
                );
                let parent = canonical_workspace
                    .parent()
                    .context("workspace path has no parent directory")?;
                fs::create_dir_all(parent).with_context(|| {
                    format!("failed to create workspace parent '{}'", parent.display())
                })?;
                fs::create_dir(&canonical_workspace).with_context(|| {
                    format!(
                        "failed to create fresh workspace '{}'",
                        canonical_workspace.display()
                    )
                })?;
                ensure!(
                    validate_fresh_workspace(&canonical_workspace)? == canonical_workspace,
                    "workspace path changed after validation"
                );

                let expected_fixture_hash = fixture_hashes.get(&case.id);
                let materialized = materialize_run_workspace(
                    suite_root,
                    case,
                    variant,
                    artifact_source,
                    &suite.artifact.sha256,
                    expected_fixture_hash,
                    &canonical_workspace,
                )?;
                let request = RunRequest {
                    suite_id: suite.suite_id.clone(),
                    case_id: case.id.clone(),
                    prompt: case.prompt.clone(),
                    variant,
                    repetition,
                    artifact_hash: materialized.artifact_hash.clone(),
                    fixture_hash: materialized.fixture_hash.clone(),
                    model_pin: suite.model_pin.clone(),
                    policy: suite.policy.clone(),
                };
                let observation = match adapter.run(&request, &canonical_workspace) {
                    Ok(observation) => match valid_observation(&observation, &suite.policy, case) {
                        Ok(observation_bytes) => {
                            let next_total = total_evidence_bytes
                                .checked_add(observation_bytes)
                                .filter(|total| *total <= MAX_RESULT_EVIDENCE_BYTES);
                            if let Some(next_total) = next_total {
                                total_evidence_bytes = next_total;
                                observation
                            } else {
                                invalid_observation()
                            }
                        }
                        Err(_) => invalid_observation(),
                    },
                    Err(_) => RunObservation::adapter_error(),
                };
                let mut grader_results =
                    grade_case(case, variant, adapter.capabilities(), &observation)?;
                for grader in &mut grader_results {
                    let available =
                        MAX_RESULT_EVIDENCE_REFS.saturating_sub(total_grader_evidence_refs);
                    if grader.evidence.len() > available {
                        grader.omitted_evidence_references += grader.evidence.len() - available;
                        grader.evidence.truncate(available);
                    }
                    total_grader_evidence_refs += grader.evidence.len();
                }
                runs.push(make_run_record(
                    case,
                    &request,
                    runner_identity.clone(),
                    observation,
                    grader_results,
                    suite.policy.retention,
                ));
            }
        }
    }

    let comparison = aggregate(suite, &runs);
    Ok(EvaluationResult {
        schema_version: RESULT_SCHEMA_VERSION,
        suite_id: suite.suite_id.clone(),
        artifact: suite.artifact.clone(),
        runs,
        comparison,
        extensions: Default::default(),
    })
}

fn validate_fresh_workspace(path: &Path) -> Result<PathBuf> {
    let metadata = fs::symlink_metadata(path)
        .with_context(|| format!("workspace '{}' does not exist", path.display()))?;
    ensure!(
        metadata.is_dir() && !metadata.file_type().is_symlink(),
        "workspace must be a real directory"
    );
    ensure!(
        fs::read_dir(path)?.next().is_none(),
        "workspace must be empty"
    );
    path.canonicalize()
        .with_context(|| format!("failed to resolve workspace '{}'", path.display()))
}

fn validate_runner_identity(identity: &RunnerIdentity) -> Result<()> {
    validate_safe_token(&identity.id, "runner id", 128, false)?;
    if let Some(version) = &identity.version {
        validate_safe_token(version, "runner version", 64, false)?;
    }
    Ok(())
}

fn valid_observation(
    observation: &RunObservation,
    policy: &ExecutionPolicy,
    case: &EvaluationCase,
) -> Result<u64> {
    ensure!(
        observation.terminal_state != TerminalState::Completed || observation.failure.is_none(),
        "completed observation cannot report a failure"
    );
    if let Some(text) = &observation.final_text {
        ensure!(
            text.len() as u64 <= policy.max_output_bytes,
            "final text exceeds output limit"
        );
    }
    if let Some(pin) = &observation.model_pin {
        validate_model_pin(pin)?;
    }
    if let Some(observed_policy) = &observation.execution_policy {
        observed_policy.validate()?;
    }
    ensure!(
        observation.artifact_manifest.len() <= MAX_ARTIFACT_MANIFEST_ENTRIES,
        "artifact manifest entry count exceeds limit"
    );
    ensure!(
        observation.events.len() <= MAX_EVENTS_PER_RUN,
        "normalized event count exceeds limit"
    );

    let mut output_bytes = observation
        .final_text
        .as_ref()
        .map_or(0_u64, |text| text.len() as u64);
    for path in observation.artifact_manifest.keys() {
        super::suite::validate_relative_path(path, "observed artifact path")?;
        output_bytes = add_output_bytes(output_bytes, path.len() as u64 + 72)?;
    }
    for event in &observation.events {
        validate_safe_token(
            &event.name,
            "normalized event name",
            MAX_EVENT_NAME_BYTES,
            false,
        )?;
        output_bytes = add_output_bytes(output_bytes, event.name.len() as u64 + 8)?;
    }
    ensure!(!case.id.is_empty(), "case id cannot be empty");
    ensure!(
        output_bytes <= policy.max_output_bytes,
        "normalized observation exceeds output limit"
    );
    Ok(output_bytes)
}

fn add_output_bytes(total: u64, additional: u64) -> Result<u64> {
    total
        .checked_add(additional)
        .context("normalized observation size overflowed")
}

fn invalid_observation() -> RunObservation {
    RunObservation {
        terminal_state: TerminalState::RunnerError,
        failure: Some(FailureKind::InvalidObservation),
        ..RunObservation::completed()
    }
}

fn make_run_record(
    case: &EvaluationCase,
    request: &RunRequest,
    runner: RunnerIdentity,
    observation: RunObservation,
    graders: Vec<super::result::GraderResult>,
    retention: super::contract::RetentionPolicy,
) -> RunRecord {
    let retain_raw_details = retention == super::contract::RetentionPolicy::FullTrace;
    let artifact_manifest = observation
        .artifact_manifest
        .into_iter()
        .map(|(path, sha256)| super::result::ArtifactEvidence {
            path_sha256: text_hash(&path),
            sha256,
            path: retain_raw_details.then_some(path),
        })
        .collect();
    let events = observation
        .events
        .into_iter()
        .map(|event| super::result::EventEvidence {
            kind: event.kind,
            name_sha256: text_hash(&event.name),
            name: retain_raw_details.then_some(event.name),
            succeeded: event.succeeded,
        })
        .collect();
    let final_text_sha256 = observation.final_text.as_deref().map(text_hash);
    let final_text = match (retention, observation.final_text) {
        (super::contract::RetentionPolicy::FullTrace, text) => text,
        (super::contract::RetentionPolicy::SummaryOnly, _) => None,
    };
    RunRecord {
        case_id: case.id.clone(),
        variant: request.variant,
        repetition: request.repetition,
        fixture_hash: request.fixture_hash.clone(),
        runner,
        requested_model_pin: request.model_pin.clone(),
        observed_model_pin: observation.model_pin,
        requested_policy: request.policy.clone(),
        observed_policy: observation.execution_policy,
        terminal_state: observation.terminal_state,
        failure: observation.failure,
        evidence: RunEvidence {
            capabilities: observation.capabilities,
            final_text_sha256,
            final_text,
            artifact_manifest,
            events,
            activation: observation.activation,
        },
        graders,
    }
}
