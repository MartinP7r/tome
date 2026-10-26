//! Provider-neutral skill evaluation contracts and deterministic evidence handling.

pub mod aggregate;
pub mod contract;
pub mod grader;
pub mod materialize;
pub mod result;
pub mod runner;
pub mod suite;

#[cfg(any(test, feature = "test-support"))]
pub mod fake;

pub use aggregate::aggregate;
pub use contract::{
    ActivationState, EventKind, ExecutionPolicy, FailureKind, IsolationMode, ModelPin,
    NetworkPolicy, NormalizedEvent, ObservationCapability, RetentionPolicy, RunObservation,
    RunnerIdentity, TerminalState, Variant,
};
pub use grader::grade_case;
pub use materialize::{MaterializedWorkspace, hash_artifact_directory, materialize_run_workspace};
pub use result::{
    ArtifactEvidence, ComparisonResult, ComparisonStatus, EvaluationResult, EventEvidence,
    EvidenceRef, GraderResult, GraderStatus, NoDeltaReason, RunEvidence, RunRecord, VariantScore,
    parse_result_json,
};
pub use runner::{RunKey, RunRequest, RunnerAdapter, WorkspaceProvider, evaluate_suite};
pub use suite::{
    ArtifactIdentity, EvaluationCase, EvaluationSuite, EventSelector, GraderSpec, load_suite_file,
    parse_suite_toml, validate_relative_path,
};
