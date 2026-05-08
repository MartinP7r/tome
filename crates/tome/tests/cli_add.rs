use assert_fs::TempDir;
use predicates::prelude::*;

mod common;
use common::*;

#[test]
fn test_add_happy_path() {
    let tmp = TempDir::new().unwrap();

    // Create minimal config
    let config_path = tmp.path().join("tome.toml");
    std::fs::write(&config_path, "").unwrap();
    std::fs::create_dir_all(tmp.path().join("library")).unwrap();

    tome()
        .args([
            "--tome-home",
            tmp.path().to_str().unwrap(),
            "add",
            "https://github.com/user/my-skills.git",
        ])
        .env("NO_COLOR", "1")
        .assert()
        .success()
        .stdout(predicate::str::contains("Added"));

    // Verify config was written
    let config_content = std::fs::read_to_string(&config_path).unwrap();
    assert!(
        config_content.contains("[directories.my-skills]"),
        "config should contain the new directory: {config_content}"
    );
    assert!(
        config_content.contains("type = \"git\""),
        "directory type should be git: {config_content}"
    );
}

#[test]
fn test_add_custom_name() {
    let tmp = TempDir::new().unwrap();
    let config_path = tmp.path().join("tome.toml");
    std::fs::write(&config_path, "").unwrap();
    std::fs::create_dir_all(tmp.path().join("library")).unwrap();

    tome()
        .args([
            "--tome-home",
            tmp.path().to_str().unwrap(),
            "add",
            "https://github.com/user/repo.git",
            "--name",
            "custom-name",
        ])
        .env("NO_COLOR", "1")
        .assert()
        .success()
        .stdout(predicate::str::contains("custom-name"));

    let config_content = std::fs::read_to_string(&config_path).unwrap();
    assert!(config_content.contains("[directories.custom-name]"));
}

#[test]
fn test_add_duplicate_name_fails() {
    let tmp = TempDir::new().unwrap();
    let config_path = tmp.path().join("tome.toml");
    std::fs::write(
        &config_path,
        "[directories.my-skills]\npath = \"https://github.com/user/my-skills.git\"\ntype = \"git\"\n",
    )
    .unwrap();
    std::fs::create_dir_all(tmp.path().join("library")).unwrap();

    tome()
        .args([
            "--tome-home",
            tmp.path().to_str().unwrap(),
            "add",
            "https://github.com/user/my-skills.git",
        ])
        .env("NO_COLOR", "1")
        .assert()
        .failure()
        .stderr(predicate::str::contains("already exists in config"));
}

#[test]
fn test_add_dry_run() {
    let tmp = TempDir::new().unwrap();
    let config_path = tmp.path().join("tome.toml");
    std::fs::write(&config_path, "").unwrap();
    std::fs::create_dir_all(tmp.path().join("library")).unwrap();

    tome()
        .args([
            "--tome-home",
            tmp.path().to_str().unwrap(),
            "--dry-run",
            "add",
            "https://github.com/user/my-skills.git",
        ])
        .env("NO_COLOR", "1")
        .assert()
        .success()
        .stdout(predicate::str::contains("Would"));

    // Config should be unchanged (empty)
    let config_content = std::fs::read_to_string(&config_path).unwrap();
    assert!(
        !config_content.contains("[directories"),
        "dry run should not modify config"
    );
}

#[test]
fn test_add_with_branch() {
    let tmp = TempDir::new().unwrap();
    let config_path = tmp.path().join("tome.toml");
    std::fs::write(&config_path, "").unwrap();
    std::fs::create_dir_all(tmp.path().join("library")).unwrap();

    tome()
        .args([
            "--tome-home",
            tmp.path().to_str().unwrap(),
            "add",
            "https://github.com/user/repo.git",
            "--branch",
            "develop",
        ])
        .env("NO_COLOR", "1")
        .assert()
        .success();

    let config_content = std::fs::read_to_string(&config_path).unwrap();
    assert!(
        config_content.contains("branch = \"develop\""),
        "config should contain branch: {config_content}"
    );
}

#[test]
fn test_add_expands_bare_github_slug() {
    // `tome add owner/repo` should expand to https://github.com/owner/repo so
    // a later `tome sync` can clone it. Without expansion, git would
    // interpret the bare slug as a local path and fail.
    let tmp = TempDir::new().unwrap();
    let config_path = tmp.path().join("tome.toml");
    std::fs::write(&config_path, "").unwrap();
    std::fs::create_dir_all(tmp.path().join("library")).unwrap();

    tome()
        .args([
            "--tome-home",
            tmp.path().to_str().unwrap(),
            "add",
            "planetscale/database-skills",
        ])
        .env("NO_COLOR", "1")
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "https://github.com/planetscale/database-skills",
        ));

    let config_content = std::fs::read_to_string(&config_path).unwrap();
    assert!(
        config_content.contains("path = \"https://github.com/planetscale/database-skills\""),
        "config should store the expanded URL: {config_content}"
    );
    assert!(
        config_content.contains("[directories.database-skills]"),
        "directory should be named after the repo segment of the slug: {config_content}"
    );
}

#[test]
fn test_add_dry_run_shows_expanded_slug() {
    // Dry-run with a bare slug must (a) print the expanded URL so the
    // user can confirm the rewrite, and (b) leave the config on disk
    // untouched — same contract as `test_add_dry_run` but for the slug
    // path, since slug expansion is a separate code branch.
    let tmp = TempDir::new().unwrap();
    let config_path = tmp.path().join("tome.toml");
    std::fs::write(&config_path, "").unwrap();
    std::fs::create_dir_all(tmp.path().join("library")).unwrap();

    tome()
        .args([
            "--tome-home",
            tmp.path().to_str().unwrap(),
            "--dry-run",
            "add",
            "planetscale/database-skills",
        ])
        .env("NO_COLOR", "1")
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "https://github.com/planetscale/database-skills",
        ));

    let config_content = std::fs::read_to_string(&config_path).unwrap();
    assert!(
        !config_content.contains("[directories"),
        "dry run should not modify config (slug path): {config_content}"
    );
}

#[test]
fn test_add_bare_slug_with_name_override() {
    // `--name` skips extract_repo_name, but the slug still has to
    // expand. This test pins the order: normalize_url runs before the
    // name-or-extract decision, so the stored path is the expanded URL
    // regardless of where the directory name comes from.
    let tmp = TempDir::new().unwrap();
    let config_path = tmp.path().join("tome.toml");
    std::fs::write(&config_path, "").unwrap();
    std::fs::create_dir_all(tmp.path().join("library")).unwrap();

    tome()
        .args([
            "--tome-home",
            tmp.path().to_str().unwrap(),
            "add",
            "planetscale/database-skills",
            "--name",
            "ps-db",
        ])
        .env("NO_COLOR", "1")
        .assert()
        .success();

    let config_content = std::fs::read_to_string(&config_path).unwrap();
    assert!(
        config_content.contains("[directories.ps-db]"),
        "user-supplied --name must win: {config_content}"
    );
    assert!(
        config_content.contains("path = \"https://github.com/planetscale/database-skills\""),
        "slug must still be expanded when --name is set: {config_content}"
    );
}

#[test]
fn test_add_bare_slug_with_branch_flag() {
    // The slug flow must coexist with --branch (and by extension --tag,
    // --rev). Stored config should have both the expanded URL AND the
    // branch field, written into the same directory section.
    let tmp = TempDir::new().unwrap();
    let config_path = tmp.path().join("tome.toml");
    std::fs::write(&config_path, "").unwrap();
    std::fs::create_dir_all(tmp.path().join("library")).unwrap();

    tome()
        .args([
            "--tome-home",
            tmp.path().to_str().unwrap(),
            "add",
            "planetscale/database-skills",
            "--branch",
            "main",
        ])
        .env("NO_COLOR", "1")
        .assert()
        .success();

    let config_content = std::fs::read_to_string(&config_path).unwrap();
    assert!(
        config_content.contains("path = \"https://github.com/planetscale/database-skills\""),
        "expanded URL not in config: {config_content}"
    );
    assert!(
        config_content.contains("branch = \"main\""),
        "branch field not in config: {config_content}"
    );
}
