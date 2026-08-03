use assert_fs::TempDir;
use predicates::prelude::*;
use tome::config::{Config, DirectoryName, DirectoryRole, DirectoryType, expand_tilde};

mod common;
use common::*;

#[test]
fn test_add_managed_local_directory_preserves_portable_path() {
    let tmp = TempDir::new().unwrap();
    let tome_home = tmp.path().join("tome-home");
    std::fs::create_dir_all(tmp.path().join(".pfw/skills")).unwrap();

    tome()
        .env("HOME", tmp.path())
        .env("TOME_HOME", &tome_home)
        .env("NO_COLOR", "1")
        .args(["add", "~/.pfw/skills", "--role", "managed"])
        .assert()
        .success();

    let config_path = tome_home.join("tome.toml");
    let config = Config::load(&config_path).unwrap();
    let entry = config
        .directories()
        .get(&DirectoryName::new("skills").unwrap())
        .unwrap();
    assert_eq!(entry.directory_type, DirectoryType::Directory);
    assert_eq!(entry.role(), DirectoryRole::Managed);
    assert_eq!(
        entry.path,
        expand_tilde(std::path::Path::new("~/.pfw/skills")).unwrap()
    );

    let raw = std::fs::read_to_string(config_path).unwrap();
    assert!(raw.contains("path = \"~/.pfw/skills\""), "{raw}");
}

#[test]
fn test_add_dot_relative_directory_is_anchored_across_working_directories() {
    let tmp = TempDir::new().unwrap();
    let home_input = tmp.path().join("home");
    std::fs::create_dir_all(&home_input).unwrap();
    // Avoid macOS's /var -> /private/var alias so HOME and process CWD agree.
    let home = home_input.canonicalize().unwrap();
    let add_cwd = home.join("project");
    let source = add_cwd.join("team-skills");
    let later_cwd = home.join("elsewhere");
    let tome_home = home.join("tome-home");
    let other_source = home.join("other-skills");
    std::fs::create_dir_all(&source).unwrap();
    std::fs::create_dir_all(&later_cwd).unwrap();
    std::fs::create_dir_all(&other_source).unwrap();

    tome()
        .current_dir(&add_cwd)
        .env("HOME", &home)
        .env("TOME_HOME", &tome_home)
        .env("NO_COLOR", "1")
        .args(["add", "./team-skills"])
        .assert()
        .success()
        .stdout(predicate::str::contains(source.display().to_string()));

    let config_path = tome_home.join("tome.toml");
    let raw = std::fs::read_to_string(&config_path).unwrap();
    assert!(
        raw.contains("path = \"~/project/team-skills\""),
        "anchored under-HOME path should use normal portable serialization: {raw}"
    );

    tome()
        .current_dir(&later_cwd)
        .env("HOME", &home)
        .env("TOME_HOME", &tome_home)
        .env("NO_COLOR", "1")
        .args(["add", "~/other-skills", "--role", "source"])
        .assert()
        .success();

    let raw = std::fs::read_to_string(&config_path).unwrap();
    assert!(
        raw.contains("path = \"~/project/team-skills\""),
        "subsequent checked save must retain the stable portable path: {raw}"
    );

    tome()
        .current_dir(&later_cwd)
        .env("HOME", &home)
        .env("TOME_HOME", &tome_home)
        .env("NO_COLOR", "1")
        .arg("config")
        .assert()
        .success()
        .stdout(predicate::str::contains(source.display().to_string()));
}

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
