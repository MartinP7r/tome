use assert_fs::TempDir;
use predicates::prelude::*;

mod common;
use common::*;

#[test]
fn list_with_no_sources_shows_message() {
    let tmp = TempDir::new().unwrap();
    let config = write_config(tmp.path(), "");

    tome()
        .args(["--config", config.to_str().unwrap(), "list"])
        .assert()
        .success()
        .stdout(predicate::str::contains("No skills found"));
}

#[test]
fn list_shows_discovered_skills() {
    let tmp = TempDir::new().unwrap();
    let skills_dir = tmp.path().join("skills");
    create_skill(&skills_dir, "my-skill");
    create_skill(&skills_dir, "other-skill");

    let config = write_config(
        tmp.path(),
        &format!(
            "[directories.test]\npath = \"{}\"\ntype = \"directory\"\nrole = \"source\"\n",
            skills_dir.display()
        ),
    );

    let output = tome()
        .args(["--config", config.to_str().unwrap(), "list"])
        .env("NO_COLOR", "1")
        .output()
        .unwrap();
    assert!(output.status.success());

    let stdout = String::from_utf8_lossy(&output.stdout);
    let settings = snapshot_settings(&tmp);
    settings.bind(|| {
        insta::assert_snapshot!("list_table_two_skills", stdout);
    });
}

#[test]
fn list_json_outputs_valid_json() {
    let tmp = TempDir::new().unwrap();
    let skills_dir = tmp.path().join("skills");
    create_skill(&skills_dir, "alpha-skill");
    create_skill(&skills_dir, "beta-skill");

    let config = write_config(
        tmp.path(),
        &format!(
            "[directories.test-src]\npath = \"{}\"\ntype = \"directory\"\nrole = \"source\"\n",
            skills_dir.display()
        ),
    );

    let output = tome()
        .args(["--config", config.to_str().unwrap(), "list", "--json"])
        .env("NO_COLOR", "1")
        .output()
        .unwrap();
    assert!(output.status.success());

    let parsed: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("stdout should be valid JSON");
    let arr = parsed.as_array().expect("should be a JSON array");
    assert_eq!(arr.len(), 2);

    // Redact dynamic path fields for snapshot stability
    let mut redacted = parsed.clone();
    for entry in redacted.as_array_mut().unwrap() {
        entry["path"] = serde_json::Value::String("[TMPDIR]".into());
    }

    let settings = snapshot_settings(&tmp);
    settings.bind(|| {
        insta::assert_json_snapshot!("list_json_two_skills", redacted);
    });
}

#[test]
fn list_json_with_quiet_still_outputs_json() {
    let tmp = TempDir::new().unwrap();
    let skills_dir = tmp.path().join("skills");
    create_skill(&skills_dir, "my-skill");

    let config = write_config(
        tmp.path(),
        &format!(
            "[directories.test]\npath = \"{}\"\ntype = \"directory\"\nrole = \"source\"\n",
            skills_dir.display()
        ),
    );

    let output = tome()
        .args([
            "--config",
            config.to_str().unwrap(),
            "--quiet",
            "list",
            "--json",
        ])
        .output()
        .unwrap();
    assert!(output.status.success());

    let parsed: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("--json should override --quiet");
    let arr = parsed.as_array().expect("should be a JSON array");
    assert_eq!(arr.len(), 1);
}

#[test]
fn list_json_with_no_skills_outputs_empty_array() {
    let tmp = TempDir::new().unwrap();
    let config = write_config(tmp.path(), "");

    let output = tome()
        .args(["--config", config.to_str().unwrap(), "list", "--json"])
        .env("NO_COLOR", "1")
        .output()
        .unwrap();
    assert!(output.status.success());

    let parsed: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("stdout should be valid JSON");

    let settings = snapshot_settings(&tmp);
    settings.bind(|| {
        insta::assert_json_snapshot!("list_json_empty", parsed);
    });
}
