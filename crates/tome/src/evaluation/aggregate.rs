//! Matched treatment/control scoring with explicit non-comparability reasons.

use std::collections::BTreeSet;

use super::contract::{TerminalState, Variant};
use super::result::{
    ComparisonResult, ComparisonStatus, GraderStatus, NoDeltaReason, RunRecord, VariantScore,
};
use super::suite::{EvaluationCase, EvaluationSuite};

/// Aggregate complete matched runs. Partial evidence remains on each run, but no
/// score delta is emitted unless every comparison precondition is satisfied.
pub fn aggregate(suite: &EvaluationSuite, runs: &[RunRecord]) -> ComparisonResult {
    let mut reasons = BTreeSet::new();
    let control = score_variant(suite, runs, Variant::Control);
    let treatment = score_variant(suite, runs, Variant::Treatment);

    for case in &suite.cases {
        for repetition in 0..suite.repetitions {
            let controls: Vec<_> = runs
                .iter()
                .filter(|run| {
                    run.case_id == case.id
                        && run.repetition == repetition
                        && run.variant == Variant::Control
                })
                .collect();
            let treatments: Vec<_> = runs
                .iter()
                .filter(|run| {
                    run.case_id == case.id
                        && run.repetition == repetition
                        && run.variant == Variant::Treatment
                })
                .collect();

            if controls.len() != 1 {
                reasons.insert(NoDeltaReason::MissingControlRun);
            }
            if treatments.len() != 1 {
                reasons.insert(NoDeltaReason::MissingTreatmentRun);
            }
            if let (Some(control_run), Some(treatment_run)) = (controls.first(), treatments.first())
            {
                compare_pair(case, control_run, treatment_run, &mut reasons);
            }
        }
    }

    let expected_runs = suite.cases.len() * suite.repetitions as usize * Variant::ALL.len();
    if runs.len() != expected_runs {
        reasons.insert(NoDeltaReason::MissingRequiredEvidence);
    }
    if control.total_weight == 0 && treatment.total_weight == 0 {
        reasons.insert(NoDeltaReason::NoScoreableGraders);
    }
    if control.total_weight > 0 && control.observed_weight != control.total_weight {
        reasons.insert(NoDeltaReason::MissingRequiredEvidence);
    }
    if treatment.total_weight > 0 && treatment.observed_weight != treatment.total_weight {
        reasons.insert(NoDeltaReason::MissingRequiredEvidence);
    }

    let status = if reasons.is_empty() {
        ComparisonStatus::Comparable
    } else {
        ComparisonStatus::NotComparable
    };
    let delta_basis_points = match (
        status,
        control.score_basis_points,
        treatment.score_basis_points,
    ) {
        (ComparisonStatus::Comparable, Some(control), Some(treatment)) => {
            Some(treatment as i32 - control as i32)
        }
        _ => None,
    };

    ComparisonResult {
        status,
        no_delta_reasons: reasons.into_iter().collect(),
        control,
        treatment,
        delta_basis_points,
    }
}

fn compare_pair(
    case: &EvaluationCase,
    control: &RunRecord,
    treatment: &RunRecord,
    reasons: &mut BTreeSet<NoDeltaReason>,
) {
    if control.terminal_state != TerminalState::Completed
        || treatment.terminal_state != TerminalState::Completed
    {
        reasons.insert(NoDeltaReason::TerminalRunFailure);
    }
    if control.fixture_hash != treatment.fixture_hash {
        reasons.insert(NoDeltaReason::FixtureMismatch);
    }
    if control.runner != treatment.runner {
        reasons.insert(NoDeltaReason::RunnerMismatch);
    }
    if control.requested_model_pin != treatment.requested_model_pin
        || control.observed_model_pin != treatment.observed_model_pin
        || control.requested_model_pin != control.observed_model_pin
        || treatment.requested_model_pin != treatment.observed_model_pin
    {
        reasons.insert(NoDeltaReason::ModelPinMismatch);
    }
    if control.requested_policy != treatment.requested_policy
        || control.observed_policy.as_ref() != Some(&control.requested_policy)
        || treatment.observed_policy.as_ref() != Some(&treatment.requested_policy)
        || control.observed_policy != treatment.observed_policy
    {
        reasons.insert(NoDeltaReason::PolicyMismatch);
    }

    let expected_control = expected_score_set(case, Variant::Control);
    let expected_treatment = expected_score_set(case, Variant::Treatment);
    let actual_control = actual_score_set(control);
    let actual_treatment = actual_score_set(treatment);
    let control_count = control
        .graders
        .iter()
        .filter(|grader| grader.score_bearing)
        .count();
    let treatment_count = treatment
        .graders
        .iter()
        .filter(|grader| grader.score_bearing)
        .count();
    if expected_control != actual_control
        || expected_treatment != actual_treatment
        || actual_control != actual_treatment
        || control_count != actual_control.len()
        || treatment_count != actual_treatment.len()
    {
        reasons.insert(NoDeltaReason::ScoreableGraderMismatch);
    }
    if has_uncomparable_score(control) || has_uncomparable_score(treatment) {
        reasons.insert(NoDeltaReason::MissingRequiredEvidence);
    }
}

fn expected_score_set(case: &EvaluationCase, variant: Variant) -> BTreeSet<(String, u32)> {
    case.graders
        .iter()
        .filter(|grader| grader.score_bearing_for(variant))
        .map(|grader| (grader.id().to_string(), grader.weight()))
        .collect()
}

fn actual_score_set(run: &RunRecord) -> BTreeSet<(String, u32)> {
    run.graders
        .iter()
        .filter(|grader| grader.score_bearing)
        .map(|grader| (grader.grader_id.clone(), grader.weight))
        .collect()
}

fn has_uncomparable_score(run: &RunRecord) -> bool {
    run.graders.iter().any(|grader| {
        grader.score_bearing && !matches!(grader.status, GraderStatus::Pass | GraderStatus::Fail)
    })
}

fn score_variant(suite: &EvaluationSuite, runs: &[RunRecord], variant: Variant) -> VariantScore {
    let mut passed_weight = 0_u64;
    let mut observed_weight = 0_u64;
    let mut total_weight = 0_u64;
    for case in &suite.cases {
        for repetition in 0..suite.repetitions {
            let mut matching_runs = runs.iter().filter(|run| {
                run.case_id == case.id && run.repetition == repetition && run.variant == variant
            });
            let run = matching_runs.next();
            let has_duplicate_run = matching_runs.next().is_some();
            for expected in case
                .graders
                .iter()
                .filter(|grader| grader.score_bearing_for(variant))
            {
                let weight = u64::from(expected.weight());
                total_weight = total_weight.saturating_add(weight);
                if has_duplicate_run {
                    continue;
                }
                let Some(run) = run else {
                    continue;
                };
                let mut matching_graders = run.graders.iter().filter(|grader| {
                    grader.score_bearing
                        && grader.grader_id == expected.id()
                        && grader.weight == expected.weight()
                });
                let Some(grade) = matching_graders.next() else {
                    continue;
                };
                if matching_graders.next().is_some() {
                    continue;
                }
                match grade.status {
                    GraderStatus::Pass => {
                        observed_weight = observed_weight.saturating_add(weight);
                        passed_weight = passed_weight.saturating_add(weight);
                    }
                    GraderStatus::Fail => observed_weight = observed_weight.saturating_add(weight),
                    GraderStatus::NotComparable | GraderStatus::Skipped => {}
                }
            }
        }
    }
    let score_basis_points = (total_weight > 0 && observed_weight == total_weight)
        .then(|| ((u128::from(passed_weight) * 10_000) / u128::from(total_weight)) as u32);
    VariantScore {
        variant,
        passed_weight,
        observed_weight,
        total_weight,
        score_basis_points,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::evaluation::contract::{ActivationState, ExecutionPolicy, ModelPin, RunnerIdentity};
    use crate::evaluation::result::{EvidenceRef, GraderResult};
    use crate::evaluation::suite::{ArtifactIdentity, EvaluationCase, GraderSpec};
    use crate::validation::ContentHash;

    fn suite() -> EvaluationSuite {
        EvaluationSuite {
            schema_version: 1,
            suite_id: "suite".to_string(),
            artifact: ArtifactIdentity {
                name: "skill".to_string(),
                sha256: ContentHash::new("a".repeat(64)).unwrap(),
            },
            repetitions: 1,
            policy: ExecutionPolicy::default(),
            model_pin: Some(ModelPin {
                provider: "fake".to_string(),
                model: "deterministic".to_string(),
                version: Some("1".to_string()),
                prompt_version: Some("1".to_string()),
            }),
            cases: vec![EvaluationCase {
                id: "case".to_string(),
                prompt: "prompt".to_string(),
                fixture: None,
                tags: Vec::new(),
                graders: vec![GraderSpec::TextContains {
                    id: "answer".to_string(),
                    needle: "42".to_string(),
                    weight: 2,
                }],
            }],
        }
    }

    fn run(variant: Variant, status: GraderStatus) -> RunRecord {
        let suite = suite();
        RunRecord {
            case_id: "case".to_string(),
            variant,
            repetition: 0,
            fixture_hash: None,
            runner: RunnerIdentity {
                id: "fake".to_string(),
                version: Some("1".to_string()),
            },
            requested_model_pin: suite.model_pin.clone(),
            observed_model_pin: suite.model_pin.clone(),
            requested_policy: suite.policy.clone(),
            observed_policy: Some(suite.policy),
            terminal_state: TerminalState::Completed,
            failure: None,
            evidence: Default::default(),
            graders: vec![GraderResult {
                grader_id: "answer".to_string(),
                status,
                score_bearing: true,
                weight: 2,
                reason: None,
                evidence: vec![EvidenceRef::FinalTextHash {
                    sha256: ContentHash::new("b".repeat(64)).unwrap(),
                }],
            }],
        }
    }

    #[test]
    fn equal_control_and_treatment_scores_have_zero_delta() {
        let suite = suite();
        let comparison = aggregate(
            &suite,
            &[
                run(Variant::Control, GraderStatus::Pass),
                run(Variant::Treatment, GraderStatus::Pass),
            ],
        );
        assert_eq!(comparison.status, ComparisonStatus::Comparable);
        assert_eq!(comparison.delta_basis_points, Some(0));
    }

    #[test]
    fn failed_runs_withhold_delta() {
        let suite = suite();
        let mut failed = run(Variant::Treatment, GraderStatus::Fail);
        failed.terminal_state = TerminalState::TimedOut;
        let comparison = aggregate(&suite, &[run(Variant::Control, GraderStatus::Pass), failed]);
        assert_eq!(comparison.delta_basis_points, None);
        assert!(
            comparison
                .no_delta_reasons
                .contains(&NoDeltaReason::TerminalRunFailure)
        );
    }

    #[test]
    fn different_model_pins_withhold_delta() {
        let suite = suite();
        let mut treatment = run(Variant::Treatment, GraderStatus::Pass);
        treatment.observed_model_pin.as_mut().unwrap().version = Some("2".to_string());
        let comparison = aggregate(
            &suite,
            &[run(Variant::Control, GraderStatus::Pass), treatment],
        );
        assert_eq!(comparison.delta_basis_points, None);
        assert!(
            comparison
                .no_delta_reasons
                .contains(&NoDeltaReason::ModelPinMismatch)
        );
    }

    #[test]
    fn duplicate_score_rows_do_not_inflate_a_variant_score() {
        let suite = suite();
        let mut control = run(Variant::Control, GraderStatus::Pass);
        control.graders.push(control.graders[0].clone());
        let comparison = aggregate(
            &suite,
            &[control, run(Variant::Treatment, GraderStatus::Pass)],
        );
        assert_eq!(comparison.control.total_weight, 2);
        assert_eq!(comparison.control.score_basis_points, None);
        assert_eq!(comparison.delta_basis_points, None);
        assert!(
            comparison
                .no_delta_reasons
                .contains(&NoDeltaReason::ScoreableGraderMismatch)
        );
    }

    #[test]
    fn negative_routing_assertions_are_scoreable_in_both_variants() {
        let mut suite = suite();
        suite.cases[0].graders.push(GraderSpec::Activation {
            id: "not-routed".to_string(),
            treatment: Some(ActivationState::NotActivated),
            control: Some(ActivationState::NotActivated),
            weight: 10,
        });
        let mut control = run(Variant::Control, GraderStatus::Pass);
        let mut treatment = run(Variant::Treatment, GraderStatus::Pass);
        for run in [&mut control, &mut treatment] {
            run.graders.push(GraderResult {
                grader_id: "not-routed".to_string(),
                status: GraderStatus::Pass,
                score_bearing: true,
                weight: 10,
                reason: None,
                evidence: vec![EvidenceRef::Activation {
                    state: ActivationState::NotActivated,
                }],
            });
        }
        let comparison = aggregate(&suite, &[control, treatment]);
        assert_eq!(comparison.control.total_weight, 12);
        assert_eq!(comparison.treatment.total_weight, 12);
        assert_eq!(comparison.delta_basis_points, Some(0));
    }

    #[test]
    fn positive_activation_is_diagnostic_and_excluded_from_treatment_score() {
        let mut suite = suite();
        suite.cases[0].graders.push(GraderSpec::Activation {
            id: "activation".to_string(),
            treatment: Some(ActivationState::Activated),
            control: Some(ActivationState::NotActivated),
            weight: 10,
        });
        let mut control = run(Variant::Control, GraderStatus::Pass);
        let mut treatment = run(Variant::Treatment, GraderStatus::Pass);
        control.graders.push(GraderResult {
            grader_id: "activation".to_string(),
            status: GraderStatus::Pass,
            score_bearing: true,
            weight: 10,
            reason: None,
            evidence: vec![EvidenceRef::Activation {
                state: ActivationState::NotActivated,
            }],
        });
        treatment.graders.push(GraderResult {
            grader_id: "activation".to_string(),
            status: GraderStatus::Pass,
            score_bearing: false,
            weight: 10,
            reason: None,
            evidence: vec![EvidenceRef::Activation {
                state: ActivationState::Activated,
            }],
        });
        let comparison = aggregate(&suite, &[control, treatment]);
        assert_eq!(comparison.control.total_weight, 12);
        assert_eq!(comparison.treatment.total_weight, 2);
        assert_eq!(comparison.delta_basis_points, None);
        assert!(
            comparison
                .no_delta_reasons
                .contains(&NoDeltaReason::ScoreableGraderMismatch)
        );
    }
}
