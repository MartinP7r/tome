//! Deterministic grading over normalized observations only.

use std::collections::BTreeSet;

use anyhow::Result;
use regex::Regex;

use super::contract::{ObservationCapability, RunObservation, TerminalState, Variant};
use super::result::{EvidenceRef, GraderResult, GraderStatus, text_hash};
use super::suite::{EvaluationCase, GraderSpec};

/// Grade one case without invoking a provider or interpreting free-form traces.
pub fn grade_case(
    case: &EvaluationCase,
    variant: Variant,
    adapter_capabilities: &BTreeSet<ObservationCapability>,
    observation: &RunObservation,
) -> Result<Vec<GraderResult>> {
    case.graders
        .iter()
        .map(|grader| grade_one(grader, variant, adapter_capabilities, observation))
        .collect()
}

fn grade_one(
    grader: &GraderSpec,
    variant: Variant,
    adapter_capabilities: &BTreeSet<ObservationCapability>,
    observation: &RunObservation,
) -> Result<GraderResult> {
    let score_bearing = grader.score_bearing_for(variant);
    let weight = grader.weight();
    let mut result = GraderResult {
        grader_id: grader.id().to_string(),
        status: GraderStatus::NotComparable,
        score_bearing,
        weight,
        reason: None,
        evidence: Vec::new(),
    };

    if matches!(
        grader,
        GraderSpec::Activation {
            treatment: None,
            control: None,
            ..
        }
    ) {
        result.status = GraderStatus::Skipped;
        result.reason = Some("no activation expectation declared".to_string());
        return Ok(result);
    }

    if !matches!(observation.terminal_state, TerminalState::Completed) {
        result.reason = Some("run did not complete".to_string());
        return Ok(result);
    }

    let required = grader.required_capability();
    if !adapter_capabilities.contains(&required) || !observation.capabilities.contains(&required) {
        result.reason = Some(format!(
            "required observation capability {required:?} is unavailable"
        ));
        return Ok(result);
    }

    let (status, reason, evidence) = match grader {
        GraderSpec::TextContains { needle, .. } => match observation.final_text.as_deref() {
            Some(text) => {
                let hash = text_hash(text);
                let matched = text.contains(needle);
                (
                    if matched {
                        GraderStatus::Pass
                    } else {
                        GraderStatus::Fail
                    },
                    (!matched)
                        .then(|| "final text did not contain the expected literal".to_string()),
                    vec![EvidenceRef::FinalTextHash { sha256: hash }],
                )
            }
            None => return not_comparable(result, "final-text capability had no final text"),
        },
        GraderSpec::TextRegex { pattern, .. } => match observation.final_text.as_deref() {
            Some(text) => {
                let hash = text_hash(text);
                let matched = Regex::new(pattern)?.is_match(text);
                (
                    if matched {
                        GraderStatus::Pass
                    } else {
                        GraderStatus::Fail
                    },
                    (!matched).then(|| "final text did not match the declared pattern".to_string()),
                    vec![EvidenceRef::FinalTextHash { sha256: hash }],
                )
            }
            None => return not_comparable(result, "final-text capability had no final text"),
        },
        GraderSpec::ArtifactExists {
            path,
            expected_present,
            ..
        } => {
            let present = observation.artifact_manifest.contains_key(path);
            let matched = present == *expected_present;
            let evidence = present
                .then(|| EvidenceRef::ArtifactPath { path: path.clone() })
                .into_iter()
                .collect();
            (
                if matched {
                    GraderStatus::Pass
                } else {
                    GraderStatus::Fail
                },
                (!matched).then(|| "artifact presence did not match expectation".to_string()),
                evidence,
            )
        }
        GraderSpec::ArtifactSha256 {
            path,
            expected_sha256,
            ..
        } => match observation.artifact_manifest.get(path) {
            Some(actual) => (
                if actual == expected_sha256 {
                    GraderStatus::Pass
                } else {
                    GraderStatus::Fail
                },
                (actual != expected_sha256)
                    .then(|| "artifact hash did not match expectation".to_string()),
                vec![EvidenceRef::ArtifactHash {
                    path: path.clone(),
                    sha256: actual.clone(),
                }],
            ),
            None => (
                GraderStatus::Fail,
                Some("expected artifact path was absent".to_string()),
                Vec::new(),
            ),
        },
        GraderSpec::EventCount {
            event_kind,
            name,
            expected_count,
            ..
        } => {
            let indices: Vec<_> = observation
                .events
                .iter()
                .enumerate()
                .filter_map(|(index, event)| {
                    (event.kind == *event_kind && event.name == *name).then_some(index)
                })
                .collect();
            let matched = indices.len() == *expected_count;
            (
                if matched {
                    GraderStatus::Pass
                } else {
                    GraderStatus::Fail
                },
                (!matched).then(|| "event count did not match expectation".to_string()),
                indices
                    .into_iter()
                    .map(|index| EvidenceRef::EventIndex { index })
                    .collect(),
            )
        }
        GraderSpec::EventOrder { sequence, .. } => {
            let mut cursor = 0;
            let mut indices = Vec::with_capacity(sequence.len());
            for selector in sequence {
                let found = observation.events[cursor..]
                    .iter()
                    .position(|event| {
                        event.kind == selector.event_kind && event.name == selector.name
                    })
                    .map(|offset| cursor + offset);
                match found {
                    Some(index) => {
                        indices.push(index);
                        cursor = index + 1;
                    }
                    None => break,
                }
            }
            let matched = indices.len() == sequence.len();
            (
                if matched {
                    GraderStatus::Pass
                } else {
                    GraderStatus::Fail
                },
                (!matched).then(|| "required event sequence was not observed in order".to_string()),
                indices
                    .into_iter()
                    .map(|index| EvidenceRef::EventIndex { index })
                    .collect(),
            )
        }
        GraderSpec::ArtifactAllowlist { allowed_paths, .. } => {
            let unexpected: Vec<_> = observation
                .artifact_manifest
                .keys()
                .filter(|path| !allowed_paths.contains(path))
                .cloned()
                .collect();
            (
                if unexpected.is_empty() {
                    GraderStatus::Pass
                } else {
                    GraderStatus::Fail
                },
                (!unexpected.is_empty())
                    .then(|| "artifact manifest contains a non-allowlisted path".to_string()),
                unexpected
                    .into_iter()
                    .map(|path| EvidenceRef::ArtifactPath { path })
                    .collect(),
            )
        }
        GraderSpec::Activation {
            treatment, control, ..
        } => {
            let expected = match variant {
                Variant::Treatment => treatment,
                Variant::Control => control,
            };
            let Some(expected) = expected else {
                result.status = GraderStatus::Skipped;
                result.reason =
                    Some("no activation expectation declared for this variant".to_string());
                return Ok(result);
            };
            let Some(actual) = observation.activation else {
                return not_comparable(result, "activation capability had no activation evidence");
            };
            let matched = actual == *expected;
            (
                if matched {
                    GraderStatus::Pass
                } else {
                    GraderStatus::Fail
                },
                (!matched).then(|| "activation state did not match expectation".to_string()),
                vec![EvidenceRef::Activation { state: actual }],
            )
        }
    };

    result.status = status;
    result.reason = reason;
    result.evidence = evidence;
    Ok(result)
}

fn not_comparable(mut result: GraderResult, reason: &str) -> Result<GraderResult> {
    result.status = GraderStatus::NotComparable;
    result.reason = Some(reason.to_string());
    Ok(result)
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use crate::validation::ContentHash;

    use super::*;
    use crate::evaluation::contract::{ActivationState, EventKind, NormalizedEvent};
    use crate::evaluation::suite::EventSelector;

    fn caps(values: &[ObservationCapability]) -> BTreeSet<ObservationCapability> {
        values.iter().copied().collect()
    }

    fn observation(capability: ObservationCapability) -> RunObservation {
        let mut result = RunObservation::completed();
        result.capabilities.insert(capability);
        result
    }

    fn grader(kind: GraderSpec) -> EvaluationCase {
        EvaluationCase {
            id: "case".to_string(),
            prompt: "prompt".to_string(),
            fixture: None,
            tags: Vec::new(),
            graders: vec![kind],
        }
    }

    #[test]
    fn grades_final_text_literal_and_regex_deterministically() {
        let mut obs = observation(ObservationCapability::FinalText);
        obs.final_text = Some("the answer is 42".to_string());
        let specs = [
            GraderSpec::TextContains {
                id: "literal".to_string(),
                needle: "42".to_string(),
                weight: 1,
            },
            GraderSpec::TextRegex {
                id: "regex".to_string(),
                pattern: r"answer.*42".to_string(),
                weight: 1,
            },
        ];
        for spec in specs {
            let case = grader(spec);
            let graded = grade_case(
                &case,
                Variant::Treatment,
                &caps(&[ObservationCapability::FinalText]),
                &obs,
            )
            .unwrap();
            assert_eq!(graded[0].status, GraderStatus::Pass);
            assert!(matches!(
                graded[0].evidence[0],
                EvidenceRef::FinalTextHash { .. }
            ));
        }
    }

    #[test]
    fn missing_required_capability_is_not_comparable() {
        let case = grader(GraderSpec::TextContains {
            id: "text".to_string(),
            needle: "42".to_string(),
            weight: 1,
        });
        let graded = grade_case(
            &case,
            Variant::Treatment,
            &BTreeSet::new(),
            &RunObservation::completed(),
        )
        .unwrap();
        assert_eq!(graded[0].status, GraderStatus::NotComparable);
        assert!(graded[0].reason.as_deref().unwrap().contains("capability"));
    }

    #[test]
    fn artifact_and_event_graders_use_normalized_evidence() {
        let mut obs = observation(ObservationCapability::ArtifactManifest);
        obs.capabilities.insert(ObservationCapability::Events);
        obs.artifact_manifest.insert(
            "out/report.txt".to_string(),
            ContentHash::new("b".repeat(64)).unwrap(),
        );
        obs.events = vec![
            NormalizedEvent {
                kind: EventKind::ToolCall,
                name: "search".to_string(),
                succeeded: Some(true),
            },
            NormalizedEvent {
                kind: EventKind::FileWrite,
                name: "write-report".to_string(),
                succeeded: Some(true),
            },
        ];
        let graders = vec![
            GraderSpec::ArtifactExists {
                id: "exists".to_string(),
                path: "out/report.txt".to_string(),
                expected_present: true,
                weight: 1,
            },
            GraderSpec::ArtifactSha256 {
                id: "hash".to_string(),
                path: "out/report.txt".to_string(),
                expected_sha256: ContentHash::new("b".repeat(64)).unwrap(),
                weight: 1,
            },
            GraderSpec::EventCount {
                id: "search".to_string(),
                event_kind: EventKind::ToolCall,
                name: "search".to_string(),
                expected_count: 1,
                weight: 1,
            },
            GraderSpec::EventOrder {
                id: "order".to_string(),
                sequence: vec![
                    EventSelector {
                        event_kind: EventKind::ToolCall,
                        name: "search".to_string(),
                    },
                    EventSelector {
                        event_kind: EventKind::FileWrite,
                        name: "write-report".to_string(),
                    },
                ],
                weight: 1,
            },
            GraderSpec::ArtifactAllowlist {
                id: "allowlist".to_string(),
                allowed_paths: vec!["out/report.txt".to_string()],
                weight: 1,
            },
        ];
        let case = EvaluationCase {
            id: "case".to_string(),
            prompt: "prompt".to_string(),
            fixture: None,
            tags: Vec::new(),
            graders,
        };
        let graded = grade_case(
            &case,
            Variant::Treatment,
            &caps(&[
                ObservationCapability::ArtifactManifest,
                ObservationCapability::Events,
            ]),
            &obs,
        )
        .unwrap();
        assert!(
            graded
                .iter()
                .all(|result| result.status == GraderStatus::Pass)
        );
    }

    #[test]
    fn positive_activation_is_diagnostic_and_negative_activation_is_scored() {
        let case = EvaluationCase {
            id: "routing".to_string(),
            prompt: "prompt".to_string(),
            fixture: None,
            tags: Vec::new(),
            graders: vec![GraderSpec::Activation {
                id: "activation".to_string(),
                treatment: Some(ActivationState::Activated),
                control: Some(ActivationState::NotActivated),
                weight: 10,
            }],
        };
        let mut obs = observation(ObservationCapability::Activation);
        obs.activation = Some(ActivationState::Activated);
        let positive = grade_case(
            &case,
            Variant::Treatment,
            &caps(&[ObservationCapability::Activation]),
            &obs,
        )
        .unwrap();
        assert_eq!(positive[0].status, GraderStatus::Pass);
        assert!(!positive[0].score_bearing);

        obs.activation = Some(ActivationState::NotActivated);
        let negative = grade_case(
            &case,
            Variant::Control,
            &caps(&[ObservationCapability::Activation]),
            &obs,
        )
        .unwrap();
        assert_eq!(negative[0].status, GraderStatus::Pass);
        assert!(negative[0].score_bearing);
    }

    #[test]
    fn artifact_allowlist_rejects_unexpected_output() {
        let mut obs = observation(ObservationCapability::ArtifactManifest);
        obs.artifact_manifest.insert(
            "unexpected.txt".to_string(),
            ContentHash::new("c".repeat(64)).unwrap(),
        );
        let case = grader(GraderSpec::ArtifactAllowlist {
            id: "allow".to_string(),
            allowed_paths: vec!["expected.txt".to_string()],
            weight: 1,
        });
        let graded = grade_case(
            &case,
            Variant::Treatment,
            &caps(&[ObservationCapability::ArtifactManifest]),
            &obs,
        )
        .unwrap();
        assert_eq!(graded[0].status, GraderStatus::Fail);
        assert_eq!(
            graded[0].evidence,
            vec![EvidenceRef::ArtifactPath {
                path: "unexpected.txt".to_string()
            }]
        );
    }
}
