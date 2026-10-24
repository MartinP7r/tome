use std::fs;
use std::io::Write;
use std::path::Path;

use tempfile::TempDir;

use super::*;
use crate::evaluation::suite::GraderSpec;

fn write(path: &Path, contents: &[u8]) {
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::File::create(path).unwrap().write_all(contents).unwrap();
}

fn case(fixture: Option<&str>) -> EvaluationCase {
    EvaluationCase {
        id: "case".to_string(),
        prompt: "prompt".to_string(),
        fixture: fixture.map(str::to_string),
        tags: Vec::new(),
        graders: vec![GraderSpec::TextContains {
            id: "text".to_string(),
            needle: "ok".to_string(),
            weight: 1,
        }],
    }
}

#[test]
fn copies_treatment_and_fixtures_without_changing_sources() {
    let root = TempDir::new().unwrap();
    let artifact = root.path().join("source-skill");
    let suite = root.path().join("suite");
    fs::create_dir_all(&artifact).unwrap();
    write(&artifact.join("SKILL.md"), b"source skill");
    write(&suite.join("fixtures/case/input.txt"), b"fixture");
    let expected = hash_directory(&artifact).unwrap();
    let fixture_hash = hash_directory(&suite.join("fixtures/case")).unwrap();
    let treatment = TempDir::new().unwrap();
    let control = TempDir::new().unwrap();
    let treatment_paths = materialize_run_workspace(
        &suite,
        &case(Some("fixtures/case")),
        Variant::Treatment,
        &artifact,
        &expected,
        treatment.path(),
    )
    .unwrap();
    let control_paths = materialize_run_workspace(
        &suite,
        &case(Some("fixtures/case")),
        Variant::Control,
        &artifact,
        &expected,
        control.path(),
    )
    .unwrap();
    assert!(
        treatment_paths
            .artifact_path
            .as_ref()
            .unwrap()
            .join("SKILL.md")
            .is_file()
    );
    assert!(control_paths.artifact_path.is_none());
    assert_eq!(treatment_paths.fixture_hash, Some(fixture_hash));
    assert_eq!(control_paths.fixture_hash, treatment_paths.fixture_hash);
    write(
        &treatment_paths.artifact_path.unwrap().join("SKILL.md"),
        b"mutated copy",
    );
    assert_eq!(hash_directory(&artifact).unwrap(), expected);
    assert_eq!(
        fs::read(artifact.join("SKILL.md")).unwrap(),
        b"source skill"
    );
}

#[test]
fn refuses_hash_mismatch_before_writing_anything() {
    let root = TempDir::new().unwrap();
    let artifact = root.path().join("skill");
    fs::create_dir_all(&artifact).unwrap();
    write(&artifact.join("SKILL.md"), b"actual");
    let workspace = TempDir::new().unwrap();
    let wrong = ContentHash::new("a".repeat(64)).unwrap();
    assert!(
        materialize_run_workspace(
            root.path(),
            &case(None),
            Variant::Treatment,
            &artifact,
            &wrong,
            workspace.path(),
        )
        .is_err()
    );
    assert!(fs::read_dir(workspace.path()).unwrap().next().is_none());
}

#[cfg(unix)]
#[test]
fn rejects_symlinked_source_files_and_fixtures() {
    use std::os::unix::fs::symlink;

    let root = TempDir::new().unwrap();
    let artifact = root.path().join("skill");
    fs::create_dir_all(&artifact).unwrap();
    write(&artifact.join("SKILL.md"), b"skill");
    let outside = root.path().join("outside");
    fs::create_dir_all(&outside).unwrap();
    write(&outside.join("input.txt"), b"outside");
    let expected = hash_directory(&artifact).unwrap();

    symlink(outside.join("input.txt"), artifact.join("linked.txt")).unwrap();
    let artifact_workspace = TempDir::new().unwrap();
    assert!(
        materialize_run_workspace(
            root.path(),
            &case(None),
            Variant::Treatment,
            &artifact,
            &expected,
            artifact_workspace.path(),
        )
        .is_err()
    );
    assert!(
        fs::read_dir(artifact_workspace.path())
            .unwrap()
            .next()
            .is_none()
    );
    fs::remove_file(artifact.join("linked.txt")).unwrap();

    let suite = root.path().join("suite");
    fs::create_dir_all(&suite).unwrap();
    symlink(&outside, suite.join("fixture-link")).unwrap();
    let fixture_workspace = TempDir::new().unwrap();
    assert!(
        materialize_run_workspace(
            &suite,
            &case(Some("fixture-link")),
            Variant::Control,
            &artifact,
            &expected,
            fixture_workspace.path(),
        )
        .is_err()
    );
    assert!(
        fs::read_dir(fixture_workspace.path())
            .unwrap()
            .next()
            .is_none()
    );
}
