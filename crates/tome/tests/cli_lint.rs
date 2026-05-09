use assert_fs::TempDir;
use predicates::prelude::*;

mod common;
use common::*;

#[test]
fn lint_clean_library() {
    let env = TestEnvBuilder::new()
        .source("local", "directory")
        .skill_with_content(
            "good-skill",
            "local",
            "---\nname: good-skill\ndescription: A valid skill\n---\n# Good Skill",
        )
        .build();

    // First sync to populate the library
    env.cmd().arg("sync").assert().success();

    env.cmd()
        .arg("lint")
        .assert()
        .success()
        .stdout(predicate::str::contains("0 error(s)"));
}

#[test]
fn lint_reports_errors() {
    let env = TestEnvBuilder::new()
        .source("local", "directory")
        .skill_with_content("my-skill", "local", "---\nname: wrong-name\n---\n# Wrong")
        .build();

    // Sync to populate library
    env.cmd().arg("sync").assert().success();

    env.cmd()
        .arg("lint")
        .assert()
        .failure() // exit code 1 because of errors
        .stdout(predicate::str::contains("does not match directory"));
}

#[test]
fn lint_json_output() {
    let env = TestEnvBuilder::new()
        .source("local", "directory")
        .skill_with_content(
            "test-skill",
            "local",
            "---\nname: test-skill\ndescription: Valid skill\n---\n# Test",
        )
        .build();

    // Sync to populate library
    env.cmd().arg("sync").assert().success();

    env.cmd()
        .args(["lint", "--format", "json"])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"skills_checked\": 1"));
}

#[test]
fn lint_single_skill_path() {
    let tmp = TempDir::new().unwrap();
    let skill = tmp.path().join("my-skill");
    std::fs::create_dir_all(&skill).unwrap();
    std::fs::write(
        skill.join("SKILL.md"),
        "---\nname: my-skill\ndescription: Test\n---\n# Test",
    )
    .unwrap();

    tome()
        .env("TOME_HOME", tmp.path())
        .args(["lint", &skill.to_string_lossy()])
        .assert()
        .success()
        .stdout(predicate::str::contains("0 error(s)"));
}

#[test]
fn lint_single_skill_path_with_errors() {
    let tmp = TempDir::new().unwrap();
    let skill = tmp.path().join("bad-skill");
    std::fs::create_dir_all(&skill).unwrap();
    std::fs::write(skill.join("SKILL.md"), "# No frontmatter").unwrap();

    tome()
        .env("TOME_HOME", tmp.path())
        .args(["lint", &skill.to_string_lossy()])
        .assert()
        .failure()
        .stdout(predicate::str::contains("no frontmatter"));
}

/// HARD-04: lint failure must exit with code 1 via the LintFailed downcast
/// in main.rs (not via process::exit inside lib.rs). Pins the binary-level
/// exit-code contract end-to-end.
#[test]
fn lint_failure_exit_code_via_lint_failed_downcast() {
    let tmp = TempDir::new().unwrap();
    let skill = tmp.path().join("bad-skill");
    std::fs::create_dir_all(&skill).unwrap();
    // Frontmatter present but name mismatch -> emits a Severity::Error
    // through the new LintFailed bubble-up path.
    std::fs::write(
        skill.join("SKILL.md"),
        "---\nname: definitely-not-bad-skill\ndescription: x\n---\n# x",
    )
    .unwrap();

    let assert = tome()
        .env("TOME_HOME", tmp.path())
        .args(["lint", &skill.to_string_lossy()])
        .assert()
        .failure()
        .code(1);

    let output = assert.get_output();
    let stderr = String::from_utf8_lossy(&output.stderr);
    // main.rs renders LintFailed via Display -> "lint failed: N violation(s)".
    assert!(
        stderr.contains("lint failed:") && stderr.contains("violation(s)"),
        "stderr should carry the LintFailed Display output, got: {stderr}"
    );
}
