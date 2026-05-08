use assert_fs::TempDir;
use predicates::prelude::*;

mod common;
use common::*;

#[test]
fn doctor_with_clean_state() {
    let tmp = TempDir::new().unwrap();
    let config = write_config(tmp.path(), "");

    let output = tome()
        .args(["--config", config.to_str().unwrap(), "doctor"])
        .env("NO_COLOR", "1")
        .output()
        .unwrap();
    assert!(output.status.success());

    let stdout = String::from_utf8_lossy(&output.stdout);
    let settings = snapshot_settings(&tmp);
    settings.bind(|| {
        insta::assert_snapshot!("doctor_clean", stdout);
    });
}

#[test]
fn doctor_detects_broken_symlinks() {
    use std::os::unix::fs as unix_fs;

    let tmp = TempDir::new().unwrap();
    let library = tmp.path().join("library");
    std::fs::create_dir_all(&library).unwrap();

    // Create a broken symlink in the library (legacy)
    unix_fs::symlink("/nonexistent/path", library.join("broken-skill")).unwrap();

    let config = write_config(tmp.path(), "");

    tome()
        .args(["--config", config.to_str().unwrap(), "--dry-run", "doctor"])
        .assert()
        .success()
        .stdout(predicate::str::contains("1 issue(s)"));
}

#[test]
fn doctor_without_config_shows_init_prompt() {
    let tmp = TempDir::new().unwrap();
    let config_path = tmp.path().join("config.toml");
    let nonexistent_library = tmp.path().join("nonexistent-library");
    std::fs::write(
        &config_path,
        format!("library_dir = \"{}\"", nonexistent_library.display()),
    )
    .unwrap();

    tome()
        .args([
            "--config",
            config_path.to_str().unwrap(),
            "--dry-run",
            "doctor",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("Not configured yet"))
        .stdout(predicate::str::contains("tome init"));
}

#[test]
fn doctor_with_no_input_skips_repair() {
    let env = TestEnvBuilder::new()
        .source("local", "directory")
        .skill("skill-a", "local")
        .build();

    // Doctor with --no-input should not hang on prompts
    tome()
        .args([
            "--config",
            &env.config_path.to_string_lossy(),
            "--no-input",
            "doctor",
        ])
        .assert()
        .success();
}

#[test]
fn doctor_json_output() {
    let env = TestEnvBuilder::new()
        .source("local", "directory")
        .skill("skill-a", "local")
        .build();

    let output = tome()
        .args([
            "--config",
            &env.config_path.to_string_lossy(),
            "doctor",
            "--json",
        ])
        .output()
        .expect("failed to run");

    let json: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("doctor --json should produce valid JSON");
    assert_eq!(json["configured"], true);
    assert!(json["library_issues"].is_array());
}

#[cfg(unix)]
#[test]
fn machine_override_unknown_target_warns_and_continues() {
    // PORT-03: an override targeting a directory name not present in tome.toml
    // produces a stderr `warning:` line (typo guard) without aborting load.
    let tmp = TempDir::new().unwrap();
    let real_skills = tmp.path().join("real-skills");
    create_skill(&real_skills, "x");

    let tome_toml = format!(
        "library_dir = \"{}/library\"\n\
         \n\
         [directories.work]\n\
         path = \"{}\"\n\
         type = \"directory\"\n\
         role = \"source\"\n",
        tmp.path().display(),
        real_skills.display(),
    );
    std::fs::write(tmp.path().join("tome.toml"), tome_toml).unwrap();

    // Override target `claud` is a typo — does not match any configured
    // directory. The typo guard fires for any unknown name, regardless of
    // whether a similarly-named directory exists.
    let machine_toml = format!(
        "[directory_overrides.claud]\npath = \"{}/elsewhere\"\n",
        tmp.path().display(),
    );
    let machine_path = tmp.path().join("machine.toml");
    std::fs::write(&machine_path, machine_toml).unwrap();

    let assert = tome()
        .args([
            "--tome-home",
            tmp.path().to_str().unwrap(),
            "--machine",
            machine_path.to_str().unwrap(),
            "status",
        ])
        .env("NO_COLOR", "1")
        .assert()
        .success(); // does NOT abort, only warns
    let stderr = String::from_utf8(assert.get_output().stderr.clone()).unwrap();
    assert!(
        stderr.contains("warning:") && stderr.contains("claud") && stderr.contains("machine.toml"),
        "expected stderr warning naming 'claud' and 'machine.toml', got:\n{stderr}"
    );
}

#[cfg(unix)]
#[test]
fn machine_override_validation_failure_blames_machine_toml() {
    // PORT-04: validation failures triggered by an override surface as a
    // distinct error class that names machine.toml (not tome.toml) as the
    // file to edit.
    let tmp = TempDir::new().unwrap();
    let library_dir = tmp.path().join("library");
    std::fs::create_dir_all(&library_dir).unwrap();

    // tome.toml is valid: library_dir and directories.work.path are disjoint.
    let work_dir = tmp.path().join("work-skills");
    std::fs::create_dir_all(&work_dir).unwrap();
    let tome_toml = format!(
        "library_dir = \"{}\"\n\
         \n\
         [directories.work]\n\
         path = \"{}\"\n\
         type = \"directory\"\n\
         role = \"synced\"\n",
        library_dir.display(),
        work_dir.display(),
    );
    std::fs::write(tmp.path().join("tome.toml"), tome_toml).unwrap();

    // machine.toml override forces directories.work.path == library_dir.
    // After apply_machine_overrides, validate() will fail with the existing
    // "library_dir overlaps distribution directory 'work'" error.
    let machine_toml = format!(
        "[directory_overrides.work]\npath = \"{}\"\n",
        library_dir.display(),
    );
    let machine_path = tmp.path().join("machine.toml");
    std::fs::write(&machine_path, machine_toml).unwrap();

    let assert = tome()
        .args([
            "--tome-home",
            tmp.path().to_str().unwrap(),
            "--machine",
            machine_path.to_str().unwrap(),
            "status",
        ])
        .env("NO_COLOR", "1")
        .assert()
        .failure();
    let stderr = String::from_utf8(assert.get_output().stderr.clone()).unwrap();

    // Wrapped error MUST mention machine.toml (so the user knows where to look).
    assert!(
        stderr.contains("machine.toml"),
        "expected stderr to name machine.toml, got:\n{stderr}"
    );
    // And include the original validate() error text (preserved inside the wrapper).
    assert!(
        stderr.contains("library_dir") && stderr.contains("overlaps"),
        "expected wrapped error to preserve the original validate() text, got:\n{stderr}"
    );
    // And reference the override-induced classification.
    assert!(
        stderr.contains("override-induced") || stderr.contains("directory_overrides"),
        "expected wrapped error to identify itself as override-induced, got:\n{stderr}"
    );
    // Negative assertion (the discriminator): MUST NOT direct user to edit tome.toml.
    assert!(
        !stderr.contains("edit tome.toml") && !stderr.contains("Edit tome.toml"),
        "wrapped error must NOT direct the user to edit tome.toml, got:\n{stderr}"
    );
}

#[test]
fn phase14_doctor_informational_unowned_does_not_affect_exit_code() {
    let fix = phase14_build_fixture(
        &[],
        &[],
        &[("orphan-a", "removed-1"), ("orphan-b", "removed-2")],
    );

    let output = fix.cmd().arg("doctor").output().unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);

    // Exit code 0 — Unowned alone never escalates doctor.
    assert!(
        output.status.success(),
        "doctor must exit 0 when only Unowned skills are present (D-D3). stdout: {stdout}"
    );
    assert!(
        stdout.contains("Unowned skills (2)"),
        "doctor stdout must include 'Unowned skills (2)': {stdout}"
    );
    assert!(
        stdout.contains("No issues found"),
        "doctor must report 'No issues found' since unowned doesn't count (D-D3): {stdout}"
    );

    // JSON: total_issues derivation excludes unowned_skills.
    let json_output = fix.cmd().args(["doctor", "--json"]).output().unwrap();
    let json: serde_json::Value =
        serde_json::from_slice(&json_output.stdout).expect("doctor --json must produce valid JSON");
    assert_eq!(
        json["unowned_skills"].as_array().map(|a| a.len()),
        Some(2),
        "doctor --json must include 'unowned_skills' with 2 entries: {json}"
    );
    assert!(
        json["library_issues"]
            .as_array()
            .map(|a| a.is_empty())
            .unwrap_or(false),
        "library_issues must be empty: {json}"
    );
}
