//! Versioned JSON result documents and privacy-aware evidence summaries.

use std::collections::{BTreeMap, BTreeSet};
use std::io::{self, Write};

use anyhow::{Context, Result, ensure};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};

use crate::validation::{ContentHash, validate_identifier};

use super::contract::{
    ActivationState, EventKind, ExecutionPolicy, FailureKind, MAX_ARTIFACT_MANIFEST_ENTRIES,
    MAX_ARTIFACT_PATH_BYTES, MAX_EVENT_NAME_BYTES, MAX_EVENTS_PER_RUN, MAX_GRADER_EVIDENCE_REFS,
    MAX_GRADERS_PER_CASE, MAX_RESULT_EVIDENCE_REFS, MAX_RESULT_JSON_BYTES, ModelPin,
    ObservationCapability, RunnerIdentity, TerminalState, Variant, validate_model_pin,
    validate_safe_token,
};
use super::suite::ArtifactIdentity;

/// Current result-document schema version.
pub const RESULT_SCHEMA_VERSION: u32 = 2;

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
        self.validate()?;
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
        let mut writer = LimitedBuffer::new(MAX_RESULT_JSON_BYTES);
        serde_json::to_writer_pretty(&mut writer, self)
            .context("failed to serialize evaluation result")?;
        String::from_utf8(writer.bytes).context("evaluation result JSON was not UTF-8")
    }

    fn validate(&self) -> Result<()> {
        ensure!(
            self.schema_version == RESULT_SCHEMA_VERSION,
            "unsupported evaluation result schema version {}",
            self.schema_version
        );
        validate_identifier(&self.suite_id, "suite id")?;
        validate_identifier(&self.artifact.name, "artifact name")?;
        ensure!(
            self.runs.len() <= super::suite::MAX_RUNS,
            "result run count exceeds limit"
        );

        let mut run_keys = BTreeSet::new();
        let mut contains_failure = false;
        let mut evidence_reference_count = 0usize;
        for run in &self.runs {
            validate_identifier(&run.case_id, "result case id")?;
            validate_safe_token(&run.runner.id, "runner id", 128, false)?;
            if let Some(version) = &run.runner.version {
                validate_safe_token(version, "runner version", 64, false)?;
            }
            if let Some(pin) = &run.requested_model_pin {
                validate_model_pin(pin)?;
            }
            if let Some(pin) = &run.observed_model_pin {
                validate_model_pin(pin)?;
            }
            run.requested_policy.validate()?;
            if let Some(policy) = &run.observed_policy {
                policy.validate()?;
            }
            ensure!(
                run.terminal_state != TerminalState::Completed || run.failure.is_none(),
                "completed run cannot report a failure"
            );
            contains_failure |=
                run.failure.is_some() || run.terminal_state != TerminalState::Completed;
            ensure!(
                run_keys.insert((run.case_id.clone(), run.variant, run.repetition)),
                "result contains a duplicate run key"
            );
            ensure!(
                run.evidence.events.len() <= MAX_EVENTS_PER_RUN,
                "result event count exceeds limit"
            );
            ensure!(
                run.evidence.artifact_manifest.len() <= MAX_ARTIFACT_MANIFEST_ENTRIES,
                "result artifact manifest count exceeds limit"
            );
            ensure!(
                run.graders.len() <= MAX_GRADERS_PER_CASE,
                "result grader count exceeds limit"
            );
            if let Some(text) = &run.evidence.final_text {
                ensure!(
                    run.requested_policy.retention == super::contract::RetentionPolicy::FullTrace,
                    "summary-retention result contains raw final text"
                );
                ensure!(
                    text.len() as u64 <= run.requested_policy.max_output_bytes,
                    "result final text exceeds the requested output limit"
                );
                ensure!(
                    run.evidence.final_text_sha256.as_ref() == Some(&text_hash(text)),
                    "result final text hash does not match retained text"
                );
            }
            for event in &run.evidence.events {
                if let Some(name) = &event.name {
                    ensure!(
                        run.requested_policy.retention
                            == super::contract::RetentionPolicy::FullTrace,
                        "summary-retention result contains a raw event name"
                    );
                    validate_safe_token(name, "retained event name", MAX_EVENT_NAME_BYTES, false)?;
                    ensure!(
                        event.name_sha256 == text_hash(name),
                        "retained event-name hash does not match its value"
                    );
                } else {
                    ensure!(
                        run.requested_policy.retention
                            != super::contract::RetentionPolicy::FullTrace,
                        "full-trace result omits a retained event name"
                    );
                }
            }
            for artifact in &run.evidence.artifact_manifest {
                if let Some(path) = &artifact.path {
                    ensure!(
                        run.requested_policy.retention
                            == super::contract::RetentionPolicy::FullTrace,
                        "summary-retention result contains a raw artifact path"
                    );
                    super::suite::validate_relative_path(path, "retained artifact path")?;
                    ensure!(
                        path.len() <= MAX_ARTIFACT_PATH_BYTES,
                        "retained artifact path exceeds limit"
                    );
                    ensure!(
                        artifact.path_sha256 == text_hash(path),
                        "retained artifact path hash does not match its value"
                    );
                } else {
                    ensure!(
                        run.requested_policy.retention
                            != super::contract::RetentionPolicy::FullTrace,
                        "full-trace result omits a retained artifact path"
                    );
                }
            }
            for grader in &run.graders {
                validate_safe_token(&grader.grader_id, "grader id", 128, false)?;
                ensure!(
                    grader.evidence.len() <= MAX_GRADER_EVIDENCE_REFS,
                    "grader evidence count exceeds limit"
                );
                evidence_reference_count =
                    evidence_reference_count.saturating_add(grader.evidence.len());
                ensure!(
                    evidence_reference_count <= MAX_RESULT_EVIDENCE_REFS,
                    "result evidence-reference count exceeds limit"
                );
            }
        }
        validate_comparison(&self.comparison, contains_failure)
    }
}

struct LimitedBuffer {
    bytes: Vec<u8>,
    limit: usize,
}

impl LimitedBuffer {
    fn new(limit: usize) -> Self {
        Self {
            bytes: Vec::new(),
            limit,
        }
    }
}

impl Write for LimitedBuffer {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        if self.bytes.len().saturating_add(bytes.len()) > self.limit {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "evaluation result exceeds JSON size limit",
            ));
        }
        self.bytes.extend_from_slice(bytes);
        Ok(bytes.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

fn validate_comparison(comparison: &ComparisonResult, contains_failure: bool) -> Result<()> {
    ensure!(
        comparison.control.variant == Variant::Control,
        "result control score has the wrong variant"
    );
    ensure!(
        comparison.treatment.variant == Variant::Treatment,
        "result treatment score has the wrong variant"
    );
    for score in [&comparison.control, &comparison.treatment] {
        ensure!(
            score.passed_weight <= score.observed_weight,
            "score passed weight exceeds observed weight"
        );
        ensure!(
            score.observed_weight <= score.total_weight,
            "score observed weight exceeds total weight"
        );
        if let Some(points) = score.score_basis_points {
            ensure!(points <= 10_000, "score exceeds 10000 basis points");
        }
        let expected_points =
            if score.total_weight > 0 && score.observed_weight == score.total_weight {
                Some(
                    ((u128::from(score.passed_weight) * 10_000) / u128::from(score.total_weight))
                        as u32,
                )
            } else {
                None
            };
        ensure!(
            score.score_basis_points == expected_points,
            "score basis points do not match the reported weights"
        );
    }
    match comparison.status {
        ComparisonStatus::Comparable => {
            ensure!(
                comparison.delta_basis_points.is_some(),
                "comparable result must include a delta"
            );
            ensure!(
                comparison.no_delta_reasons.is_empty(),
                "comparable result cannot include no-delta reasons"
            );
            ensure!(
                !contains_failure,
                "failed runs cannot produce a comparable result"
            );
            ensure!(
                comparison.control.total_weight > 0
                    && comparison.treatment.total_weight > 0
                    && comparison.control.observed_weight == comparison.control.total_weight
                    && comparison.treatment.observed_weight == comparison.treatment.total_weight,
                "comparable result requires complete score-bearing evidence"
            );
            let control_points = comparison
                .control
                .score_basis_points
                .context("comparable result requires a control score")?;
            let treatment_points = comparison
                .treatment
                .score_basis_points
                .context("comparable result requires a treatment score")?;
            ensure!(
                comparison.delta_basis_points
                    == Some(treatment_points as i32 - control_points as i32),
                "comparison delta does not equal treatment minus control score"
            );
        }
        ComparisonStatus::NotComparable => {
            ensure!(
                comparison.delta_basis_points.is_none(),
                "non-comparable result cannot include a delta"
            );
            ensure!(
                !comparison.no_delta_reasons.is_empty(),
                "non-comparable result requires a reason"
            );
        }
    }
    Ok(())
}

/// Decode a versioned result document. Unknown JSON fields remain in `extensions`.
pub fn parse_result_json(input: &str) -> Result<EvaluationResult> {
    ensure!(
        input.len() <= MAX_RESULT_JSON_BYTES,
        "evaluation result exceeds JSON size limit"
    );
    let result: EvaluationResult =
        serde_json::from_str(input).context("failed to parse evaluation result JSON")?;
    result.validate()?;
    Ok(result)
}

/// Persisted output-artifact evidence. Summary retention stores only the path digest.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ArtifactEvidence {
    pub path_sha256: ContentHash,
    pub sha256: ContentHash,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
}

/// Persisted normalized event evidence. Summary retention stores only the name digest.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EventEvidence {
    pub kind: EventKind,
    pub name_sha256: ContentHash,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub succeeded: Option<bool>,
}

/// One case/variant/repetition result and its retained evidence.
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

/// Evidence retained for one case/variant/repetition.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct RunEvidence {
    #[serde(default)]
    pub capabilities: BTreeSet<ObservationCapability>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub final_text_sha256: Option<ContentHash>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub final_text: Option<String>,
    #[serde(default)]
    pub artifact_manifest: Vec<ArtifactEvidence>,
    #[serde(default)]
    pub events: Vec<EventEvidence>,
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
    /// Evidence references omitted by the retention limits.
    #[serde(default, skip_serializing_if = "is_zero")]
    pub omitted_evidence_references: usize,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
    #[serde(default)]
    pub evidence: Vec<EvidenceRef>,
}

fn is_zero(value: &usize) -> bool {
    *value == 0
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
    FinalTextHash {
        sha256: ContentHash,
    },
    ArtifactPath {
        path_sha256: ContentHash,
    },
    ArtifactHash {
        path_sha256: ContentHash,
        sha256: ContentHash,
    },
    EventIndex {
        index: usize,
    },
    Activation {
        state: ActivationState,
    },
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

    fn result_with_full_trace_evidence() -> EvaluationResult {
        let mut result = result_with_states(&[TerminalState::Completed]);
        let full_trace_policy = ExecutionPolicy {
            retention: RetentionPolicy::FullTrace,
            ..ExecutionPolicy::default()
        };
        let run = &mut result.runs[0];
        run.requested_policy = full_trace_policy.clone();
        run.observed_policy = Some(full_trace_policy);
        let event_name = "private-event".to_string();
        run.evidence.events.push(EventEvidence {
            kind: EventKind::ToolCall,
            name_sha256: text_hash(&event_name),
            name: Some(event_name),
            succeeded: Some(true),
        });
        let artifact_path = "private/output.txt".to_string();
        run.evidence.artifact_manifest.push(ArtifactEvidence {
            path_sha256: text_hash(&artifact_path),
            sha256: ContentHash::new("b".repeat(64)).unwrap(),
            path: Some(artifact_path),
        });
        result
    }

    fn set_summary_retention(result: &mut EvaluationResult) {
        result.runs[0].requested_policy.retention = RetentionPolicy::SummaryOnly;
        result.runs[0].observed_policy.as_mut().unwrap().retention = RetentionPolicy::SummaryOnly;
    }

    fn set_summary_retention_json(value: &mut Value) {
        value["runs"][0]["requested_policy"]["retention"] = Value::from("summary_only");
        value["runs"][0]["observed_policy"]["retention"] = Value::from("summary_only");
    }

    fn comparable_result() -> EvaluationResult {
        let mut result = result_with_states(&[]);
        result.comparison.status = ComparisonStatus::Comparable;
        result.comparison.no_delta_reasons.clear();
        result.comparison.control = VariantScore {
            variant: Variant::Control,
            passed_weight: 1,
            observed_weight: 2,
            total_weight: 2,
            score_basis_points: Some(5_000),
        };
        result.comparison.treatment = VariantScore {
            variant: Variant::Treatment,
            passed_weight: 2,
            observed_weight: 2,
            total_weight: 2,
            score_basis_points: Some(10_000),
        };
        result.comparison.delta_basis_points = Some(5_000);
        result
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
    fn rejects_unsupported_result_schema_versions() {
        let result = result_with_states(&[]);
        let encoded: Value = serde_json::from_str(&result.to_json().unwrap()).unwrap();
        for version in [1, RESULT_SCHEMA_VERSION + 1] {
            let mut value = encoded.clone();
            value["schema_version"] = Value::from(version);
            assert!(
                parse_result_json(&value.to_string())
                    .unwrap_err()
                    .to_string()
                    .contains("unsupported")
            );
        }
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
    fn rejects_raw_event_names_under_summary_retention_on_write_and_read() {
        let mut result = result_with_full_trace_evidence();
        result.runs[0].evidence.artifact_manifest.clear();
        set_summary_retention(&mut result);
        assert!(
            result
                .to_json()
                .unwrap_err()
                .to_string()
                .contains("raw event name")
        );

        let mut value: Value =
            serde_json::from_str(&result_with_full_trace_evidence().to_json().unwrap()).unwrap();
        value["runs"][0]["evidence"]["artifact_manifest"] = serde_json::json!([]);
        set_summary_retention_json(&mut value);
        assert!(
            parse_result_json(&value.to_string())
                .unwrap_err()
                .to_string()
                .contains("raw event name")
        );
    }

    #[test]
    fn rejects_raw_artifact_paths_under_summary_retention_on_write_and_read() {
        let mut result = result_with_full_trace_evidence();
        result.runs[0].evidence.events.clear();
        set_summary_retention(&mut result);
        assert!(
            result
                .to_json()
                .unwrap_err()
                .to_string()
                .contains("raw artifact path")
        );

        let mut value: Value =
            serde_json::from_str(&result_with_full_trace_evidence().to_json().unwrap()).unwrap();
        value["runs"][0]["evidence"]["events"] = serde_json::json!([]);
        set_summary_retention_json(&mut value);
        assert!(
            parse_result_json(&value.to_string())
                .unwrap_err()
                .to_string()
                .contains("raw artifact path")
        );
    }

    #[test]
    fn rejects_comparison_deltas_that_do_not_match_validated_scores() {
        let mut valid = comparable_result();
        let encoded: Value = serde_json::from_str(&valid.to_json().unwrap()).unwrap();
        let mut invalid_json = encoded;
        invalid_json["comparison"]["delta_basis_points"] = Value::from(0);
        assert!(parse_result_json(&invalid_json.to_string()).is_err());

        valid.comparison.delta_basis_points = Some(0);
        assert!(valid.to_json().is_err());
    }

    #[test]
    fn rejects_incomplete_or_inconsistent_comparable_scores() {
        let mut missing_score = comparable_result();
        missing_score.comparison.control.observed_weight = 0;
        missing_score.comparison.control.passed_weight = 0;
        missing_score.comparison.control.score_basis_points = None;
        assert!(missing_score.to_json().is_err());

        let mut inconsistent_points = comparable_result();
        inconsistent_points.comparison.control.score_basis_points = Some(4_999);
        assert!(inconsistent_points.to_json().is_err());
    }

    #[test]
    fn partial_noncomparable_scores_have_no_basis_points() {
        let mut result = result_with_states(&[]);
        result.comparison.control.total_weight = 2;
        result.comparison.control.observed_weight = 1;
        result.comparison.control.passed_weight = 1;
        assert!(result.to_json().is_ok());
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
