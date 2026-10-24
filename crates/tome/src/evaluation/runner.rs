//! Provider-neutral orchestration contracts. No process or model is invoked here.

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, ensure};

use crate::validation::ContentHash;

use super::aggregate::aggregate;
use super::contract::{
    ExecutionPolicy, FailureKind, ModelPin, ObservationCapability, RunObservation, RunnerIdentity,
    TerminalState, Variant,
};
use super::grader::grade_case;
use super::materialize::{hash_artifact_directory, materialize_run_workspace};
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

/// Supplies an isolated workspace for each run. Implementations own cleanup.
pub trait WorkspaceProvider {
    type Workspace: AsRef<Path>;

    fn create_workspace(&mut self, key: &RunKey) -> Result<Self::Workspace>;
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
    let actual_artifact_hash = hash_artifact_directory(artifact_source)
        .with_context(|| format!("failed to hash artifact '{}'", artifact_source.display()))?;
    ensure!(
        actual_artifact_hash == suite.artifact.sha256,
        "artifact hash does not match suite identity"
    );

    let mut seen_workspaces = BTreeSet::new();
    let mut runs = Vec::new();
    for case in &suite.cases {
        for variant in Variant::ALL {
            for repetition in 0..suite.repetitions {
                let key = RunKey {
                    case_id: case.id.clone(),
                    variant,
                    repetition,
                };
                let workspace = workspaces.create_workspace(&key)?;
                let workspace_path = workspace.as_ref();
                let canonical_workspace = validate_fresh_workspace(workspace_path)?;
                ensure!(
                    seen_workspaces.insert(canonical_workspace),
                    "workspace provider reused a workspace"
                );
                let materialized = materialize_run_workspace(
                    suite_root,
                    case,
                    variant,
                    artifact_source,
                    &suite.artifact.sha256,
                    workspace_path,
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
                let observation = match adapter.run(&request, workspace_path) {
                    Ok(observation)
                        if valid_observation(&observation, &suite.policy, case).is_ok() =>
                    {
                        observation
                    }
                    Ok(_) => invalid_observation(),
                    Err(_) => RunObservation::adapter_error(),
                };
                let grader_results =
                    grade_case(case, variant, adapter.capabilities(), &observation)?;
                runs.push(make_run_record(
                    case,
                    &request,
                    adapter.identity().clone(),
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

fn valid_observation(
    observation: &RunObservation,
    policy: &ExecutionPolicy,
    case: &EvaluationCase,
) -> Result<()> {
    if let Some(text) = &observation.final_text {
        ensure!(
            text.len() as u64 <= policy.max_output_bytes,
            "final text exceeds output limit"
        );
    }
    for path in observation.artifact_manifest.keys() {
        super::suite::validate_relative_path(path, "observed artifact path")?;
    }
    ensure!(
        observation.events.len() <= 100_000,
        "normalized event count exceeds limit"
    );
    for event in &observation.events {
        ensure!(
            !event.name.trim().is_empty() && event.name.len() <= 4096,
            "normalized event name is invalid"
        );
    }
    ensure!(!case.id.is_empty(), "case id cannot be empty");
    Ok(())
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
            artifact_manifest: observation.artifact_manifest,
            events: observation.events,
            activation: observation.activation,
        },
        graders,
    }
}
