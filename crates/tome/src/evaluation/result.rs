//! Versioned JSON result documents and privacy-aware evidence summaries.

use std::collections::{BTreeMap, BTreeSet};

use anyhow::{Context, Result, ensure};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};

use crate::validation::ContentHash;

use super::contract::{
    ActivationState, ExecutionPolicy, FailureKind, ModelPin, NormalizedEvent,
    ObservationCapability, RunnerIdentity, TerminalState, Variant,
};
use super::suite::ArtifactIdentity;

/// Current result-document schema version.
pub const RESULT_SCHEMA_VERSION: u32 = 1;

/// A completed or partial evaluation. Unknown top-level JSON fields are retained
/// so tools can read and round-trip documents extended by newer producers.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct EvaluationResult {
    pub schema_version: u32,
    pub suite_id: String,
    pub artifact: ArtifactIdentity,
    pub runs: Vec<RunRecord>,
    pub comparison: ComparisonResult,
    #[serde(default, flatten)]
    pub extensions: BTreeMap<String, Value>,
}

impl EvaluationResult {
    /// Encode a result as stable, human-readable JSON.
    pub fn to_json(&self) -> Result<String> {
        const RESERVED: [&str; 5] = [
            "schema_version",
            "suite_id",
            "artifact",
            "runs",
            "comparison",
        ];
        ensure!(
            self.extensions
                .keys()
                .all(|key| !RESERVED.contains(&key.as_str())),
            "result extension keys cannot shadow reserved schema fields"
        );
        serde_json::to_string_pretty(self).context("failed to serialize evaluation result")
    }
}

/// Decode a versioned result document. Unknown JSON fields remain in `extensions`.
pub fn parse_result_json(input: &str) -> Result<EvaluationResult> {
    serde_json::from_str(input).context("failed to parse evaluation result JSON")
}

/// Evidence retained for one case/variant/repetition.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RunRecord {
    pub case_id: String,
    pub variant: Variant,
    pub repetition: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fixture_hash: Option<ContentHash>,
    pub runner: RunnerIdentity,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub requested_model_pin: Option<ModelPin>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub observed_model_pin: Option<ModelPin>,
    pub requested_policy: ExecutionPolicy,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub observed_policy: Option<ExecutionPolicy>,
    pub terminal_state: TerminalState,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub failure: Option<FailureKind>,
    pub evidence: RunEvidence,
    pub graders: Vec<GraderResult>,
}

/// Persisted evidence. Raw text is omitted unless the suite opts into `FullTrace`.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct RunEvidence {
    #[serde(default)]
    pub capabilities: BTreeSet<ObservationCapability>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub final_text_sha256: Option<ContentHash>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub final_text: Option<String>,
    #[serde(default)]
    pub artifact_manifest: BTreeMap<String, ContentHash>,
    #[serde(default)]
    pub events: Vec<NormalizedEvent>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub activation: Option<ActivationState>,
}

/// Deterministic outcome for one grader.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct GraderResult {
    pub grader_id: String,
    pub status: GraderStatus,
    pub score_bearing: bool,
    pub weight: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
    #[serde(default)]
    pub evidence: Vec<EvidenceRef>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GraderStatus {
    Pass,
    Fail,
    NotComparable,
    Skipped,
}

/// A redacted pointer to evidence; it never embeds prompt or output excerpts.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum EvidenceRef {
    FinalTextHash { sha256: ContentHash },
    ArtifactPath { path: String },
    ArtifactHash { path: String, sha256: ContentHash },
    EventIndex { index: usize },
    Activation { state: ActivationState },
}

/// A score for one variant; basis points avoid floating-point drift.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct VariantScore {
    pub variant: Variant,
    pub passed_weight: u64,
    pub observed_weight: u64,
    pub total_weight: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub score_basis_points: Option<u32>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ComparisonStatus {
    Comparable,
    NotComparable,
}

/// Reasons a treatment/control delta was withheld.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NoDeltaReason {
    MissingControlRun,
    MissingTreatmentRun,
    TerminalRunFailure,
    MissingRequiredEvidence,
    FixtureMismatch,
    ModelPinMismatch,
    RunnerMismatch,
    PolicyMismatch,
    ScoreableGraderMismatch,
    NoScoreableGraders,
}

/// Aggregate result. A missing delta always has at least one explicit reason.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ComparisonResult {
    pub status: ComparisonStatus,
    #[serde(default)]
    pub no_delta_reasons: Vec<NoDeltaReason>,
    pub control: VariantScore,
    pub treatment: VariantScore,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub delta_basis_points: Option<i32>,
}

pub(crate) fn text_hash(text: &str) -> ContentHash {
    let digest = Sha256::digest(text.as_bytes());
    let hex = digest
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
    ContentHash::new(hex).expect("SHA-256 always produces a valid content hash")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::evaluation::contract::{NetworkPolicy, RetentionPolicy};

    fn result_with_states(states: &[TerminalState]) -> EvaluationResult {
        EvaluationResult {
            schema_version: RESULT_SCHEMA_VERSION,
            suite_id: "suite".to_string(),
            artifact: ArtifactIdentity {
                name: "skill".to_string(),
                sha256: ContentHash::new("a".repeat(64)).unwrap(),
            },
            runs: states
                .iter()
                .enumerate()
                .map(|(index, state)| RunRecord {
                    case_id: "case".to_string(),
                    variant: Variant::Control,
                    repetition: index as u32,
                    fixture_hash: None,
                    runner: RunnerIdentity {
                        id: "fake".to_string(),
                        version: Some("1".to_string()),
                    },
                    requested_model_pin: None,
                    observed_model_pin: None,
                    requested_policy: ExecutionPolicy::default(),
                    observed_policy: Some(ExecutionPolicy {
                        network: NetworkPolicy::Disabled,
                        ..ExecutionPolicy::default()
                    }),
                    terminal_state: *state,
                    failure: None,
                    evidence: RunEvidence::default(),
                    graders: Vec::new(),
                })
                .collect(),
            comparison: ComparisonResult {
                status: ComparisonStatus::NotComparable,
                no_delta_reasons: vec![NoDeltaReason::MissingTreatmentRun],
                control: VariantScore {
                    variant: Variant::Control,
                    passed_weight: 0,
                    observed_weight: 0,
                    total_weight: 0,
                    score_basis_points: None,
                },
                treatment: VariantScore {
                    variant: Variant::Treatment,
                    passed_weight: 0,
                    observed_weight: 0,
                    total_weight: 0,
                    score_basis_points: None,
                },
                delta_basis_points: None,
            },
            extensions: BTreeMap::new(),
        }
    }

    #[test]
    fn json_roundtrip_preserves_all_terminal_states() {
        let states = [
            TerminalState::Completed,
            TerminalState::TimedOut,
            TerminalState::PermissionDenied,
            TerminalState::RunnerError,
            TerminalState::CostLimited,
            TerminalState::Cancelled,
        ];
        let result = result_with_states(&states);
        let encoded = result.to_json().unwrap();
        let decoded = parse_result_json(&encoded).unwrap();
        assert_eq!(decoded, result);
        assert_eq!(
            decoded
                .runs
                .iter()
                .map(|run| run.terminal_state)
                .collect::<Vec<_>>(),
            states
        );
    }

    #[test]
    fn preserves_unknown_top_level_json_fields() {
        let mut result = result_with_states(&[]);
        result
            .extensions
            .insert("future_field".to_string(), serde_json::json!({"value": 7}));
        let decoded = parse_result_json(&result.to_json().unwrap()).unwrap();
        assert_eq!(decoded.extensions, result.extensions);
    }

    #[test]
    fn rejects_extension_keys_that_shadow_schema_fields() {
        let mut result = result_with_states(&[]);
        result
            .extensions
            .insert("runs".to_string(), serde_json::json!([]));
        assert!(
            result
                .to_json()
                .unwrap_err()
                .to_string()
                .contains("reserved")
        );
    }

    #[test]
    fn summary_retention_stores_hash_without_raw_text() {
        let text = "sample response";
        let evidence = RunEvidence {
            final_text_sha256: Some(text_hash(text)),
            final_text: None,
            ..RunEvidence::default()
        };
        let serialized = serde_json::to_string(&evidence).unwrap();
        assert!(serialized.contains("final_text_sha256"));
        assert!(!serialized.contains(text));
        assert_eq!(RetentionPolicy::default(), RetentionPolicy::SummaryOnly);
    }
}
