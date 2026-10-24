use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

use tempfile::TempDir;
use tome::evaluation::fake::{DeterministicFakeAdapter, FreshWorkspaceProvider};
use tome::evaluation::{
    ArtifactIdentity, EvaluationCase, EvaluationSuite, ExecutionPolicy, GraderSpec, ModelPin,
    ObservationCapability, RunKey, RunObservation, RunnerIdentity, Variant, evaluate_suite,
    hash_artifact_directory,
};

fn create_inputs(root: &Path) -> (std::path::PathBuf, std::path::PathBuf, tome::ContentHash) {
    let artifact = root.join("skill");
    let suite_root = root.join("suite");
    fs::create_dir_all(&artifact).unwrap();
    fs::create_dir_all(suite_root.join("fixtures/case")).unwrap();
    fs::write(artifact.join("SKILL.md"), "fixture skill").unwrap();
    fs::write(suite_root.join("fixtures/case/input.txt"), "test only").unwrap();
    let hash = hash_artifact_directory(&artifact).unwrap();
    (artifact, suite_root, hash)
}

fn suite(hash: tome::ContentHash) -> EvaluationSuite {
    EvaluationSuite {
        schema_version: 1,
        suite_id: "integration-suite".to_string(),
        artifact: ArtifactIdentity {
            name: "fixture-skill".to_string(),
            sha256: hash,
        },
        repetitions: 2,
        policy: ExecutionPolicy::default(),
        model_pin: Some(ModelPin {
            provider: "fake".to_string(),
            model: "deterministic".to_string(),
            version: Some("1".to_string()),
            prompt_version: Some("p1".to_string()),
        }),
        cases: vec![EvaluationCase {
            id: "case".to_string(),
            prompt: "answer the fixture question".to_string(),
            fixture: Some("fixtures/case".to_string()),
            tags: vec!["fixture".to_string()],
            graders: vec![GraderSpec::TextContains {
                id: "answer".to_string(),
                needle: "42".to_string(),
                weight: 1,
            }],
        }],
    }
}

fn adapter_with_pair(
    capabilities: impl IntoIterator<Item = ObservationCapability>,
    suite: &EvaluationSuite,
    treatment_model: Option<ModelPin>,
) -> DeterministicFakeAdapter {
    let mut adapter = DeterministicFakeAdapter::new(
        RunnerIdentity {
            id: "fake-adapter".to_string(),
            version: Some("1".to_string()),
        },
        capabilities,
    );
    for repetition in 0..suite.repetitions {
        for variant in Variant::ALL {
            let mut observation = RunObservation::completed();
            observation.capabilities = [ObservationCapability::FinalText].into_iter().collect();
            observation.final_text = Some("answer is 42".to_string());
            observation.model_pin = if variant == Variant::Treatment {
                treatment_model.clone().or_else(|| suite.model_pin.clone())
            } else {
                suite.model_pin.clone()
            };
            adapter.insert_observation(
                RunKey {
                    case_id: "case".to_string(),
                    variant,
                    repetition,
                },
                observation,
            );
        }
    }
    adapter
}

#[test]
fn fake_adapter_runs_isolated_copies_and_serializes_only_summary_by_default() {
    let root = TempDir::new().unwrap();
    let (artifact, suite_root, hash) = create_inputs(root.path());
    let suite = suite(hash.clone());
    let mut adapter = adapter_with_pair([ObservationCapability::FinalText], &suite, None);
    adapter.set_write_marker(true);
    let workspace_root = root.path().join("workspaces");
    let mut workspaces = FreshWorkspaceProvider::new(workspace_root.clone());

    let result = evaluate_suite(
        &suite_root,
        &artifact,
        &suite,
        &mut workspaces,
        &mut adapter,
    )
    .unwrap();

    assert_eq!(adapter.invocations().len(), 4);
    assert_eq!(result.runs.len(), 4);
    assert_eq!(result.comparison.delta_basis_points, Some(0));
    assert_eq!(result.comparison.control.score_basis_points, Some(10_000));
    assert_eq!(result.comparison.treatment.score_basis_points, Some(10_000));
    assert!(
        result
            .runs
            .iter()
            .all(|run| run.evidence.final_text.is_none())
    );
    assert!(
        result
            .runs
            .iter()
            .all(|run| run.evidence.final_text_sha256.is_some())
    );
    let json = result.to_json().unwrap();
    assert!(!json.contains("answer is 42"));
    assert!(json.contains("final_text_sha256"));
    assert_eq!(hash_artifact_directory(&artifact).unwrap(), hash);
    assert!(!artifact.join("fake-adapter-marker").exists());

    let workspaces: Vec<_> = fs::read_dir(&workspace_root)
        .unwrap()
        .map(Result::unwrap)
        .collect();
    assert_eq!(workspaces.len(), 4);
    for entry in workspaces {
        let path = entry.path();
        assert!(path.join("fake-adapter-marker").is_file());
        assert!(path.join("fixtures/input.txt").is_file());
        if entry.file_name().to_string_lossy().contains("treatment") {
            assert!(path.join("artifact/SKILL.md").is_file());
        } else {
            assert!(!path.join("artifact").exists());
        }
    }
}

#[test]
fn artifact_hash_mismatch_prevents_workspace_creation_and_adapter_invocation() {
    let root = TempDir::new().unwrap();
    let (artifact, suite_root, _) = create_inputs(root.path());
    let wrong_hash = tome::ContentHash::new("f".repeat(64)).unwrap();
    let suite = suite(wrong_hash);
    let mut adapter = adapter_with_pair([ObservationCapability::FinalText], &suite, None);
    let workspace_root = root.path().join("workspaces");
    let mut workspaces = FreshWorkspaceProvider::new(workspace_root.clone());

    assert!(
        evaluate_suite(
            &suite_root,
            &artifact,
            &suite,
            &mut workspaces,
            &mut adapter
        )
        .is_err()
    );
    assert!(adapter.invocations().is_empty());
    assert!(!workspace_root.exists());
}

#[test]
fn missing_observation_capability_is_not_comparable() {
    let root = TempDir::new().unwrap();
    let (artifact, suite_root, hash) = create_inputs(root.path());
    let suite = suite(hash);
    let mut adapter = adapter_with_pair(BTreeSet::new(), &suite, None);
    let mut workspaces = FreshWorkspaceProvider::new(root.path().join("workspaces"));

    let result = evaluate_suite(
        &suite_root,
        &artifact,
        &suite,
        &mut workspaces,
        &mut adapter,
    )
    .unwrap();

    assert!(
        result
            .runs
            .iter()
            .all(|run| { run.graders[0].status == tome::evaluation::GraderStatus::NotComparable })
    );
    assert_eq!(result.comparison.delta_basis_points, None);
    assert!(
        result
            .comparison
            .no_delta_reasons
            .contains(&tome::evaluation::NoDeltaReason::MissingRequiredEvidence)
    );
}
