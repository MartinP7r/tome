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
