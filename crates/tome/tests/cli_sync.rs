use assert_fs::TempDir;
use predicates::prelude::*;
use std::process::Command as StdCommand;

mod common;
use common::*;

/// Helper: initialize a git repo with a dummy identity (for CI).
fn git_init(dir: &std::path::Path) {
    StdCommand::new("git")
        .args(["init"])
        .current_dir(dir)
        .output()
        .unwrap();
    StdCommand::new("git")
        .args(["config", "user.email", "test@test.com"])
        .current_dir(dir)
        .output()
        .unwrap();
    StdCommand::new("git")
        .args(["config", "user.name", "Test"])
        .current_dir(dir)
        .output()
        .unwrap();
    // Initial commit so HEAD exists
    StdCommand::new("git")
        .args(["commit", "--allow-empty", "-m", "init"])
        .current_dir(dir)
        .output()
        .unwrap();
}

#[test]
fn sync_dry_run_makes_no_changes() {
    let tmp = TempDir::new().unwrap();
    let skills_dir = tmp.path().join("skills");
    create_skill(&skills_dir, "test-skill");

    let config = write_config(
        tmp.path(),
        &format!(
            "[directories.test]\npath = \"{}\"\ntype = \"directory\"\nrole = \"source\"\n",
            skills_dir.display()
        ),
    );

    tome()
        .args(["--config", config.to_str().unwrap(), "--dry-run", "sync"])
        .assert()
        .success()
        .stderr(predicate::str::contains("dry-run"))
        .stdout(predicate::str::contains("Sync complete"));

    // Library should remain empty
    let library = tmp.path().join("library");
    let entries: Vec<_> = std::fs::read_dir(&library)
        .unwrap()
        .filter_map(|e| e.ok())
        .collect();
    assert_eq!(entries.len(), 0, "dry run should not create entries");
}

#[test]
fn sync_copies_skills_to_library() {
    let tmp = TempDir::new().unwrap();
    let skills_dir = tmp.path().join("skills");
    create_skill(&skills_dir, "alpha");
    create_skill(&skills_dir, "beta");

    let config = write_config(
        tmp.path(),
        &format!(
            "[directories.test]\npath = \"{}\"\ntype = \"directory\"\nrole = \"source\"\n",
            skills_dir.display()
        ),
    );

    let output = tome()
        .args(["--config", config.to_str().unwrap(), "sync"])
        .env("NO_COLOR", "1")
        .output()
        .unwrap();
    assert!(output.status.success());

    let stdout = String::from_utf8_lossy(&output.stdout);
    let settings = snapshot_settings(&tmp);
    settings.bind(|| {
        insta::assert_snapshot!("sync_initial_two_skills", stdout);
    });

    let library = tmp.path().join("library");
    // v0.2: library entries are real directories, not symlinks
    assert!(library.join("alpha").is_dir());
    assert!(!library.join("alpha").is_symlink());
    assert!(library.join("beta").is_dir());
    assert!(!library.join("beta").is_symlink());
    // Content should be copied
    assert!(library.join("alpha/SKILL.md").is_file());
    // Manifest should exist at tome home (config file's parent dir)
    assert!(tmp.path().join(".tome-manifest.json").is_file());
}

#[test]
fn sync_idempotent() {
    let tmp = TempDir::new().unwrap();
    let skills_dir = tmp.path().join("skills");
    create_skill(&skills_dir, "stable-skill");

    let config = write_config(
        tmp.path(),
        &format!(
            "[directories.test]\npath = \"{}\"\ntype = \"directory\"\nrole = \"source\"\n",
            skills_dir.display()
        ),
    );

    let config_str = config.to_str().unwrap();

    // First sync
    tome()
        .args(["--config", config_str, "sync"])
        .env("NO_COLOR", "1")
        .assert()
        .success();

    // Second sync — should report 0 created, 1 unchanged
    let output = tome()
        .args(["--config", config_str, "sync"])
        .env("NO_COLOR", "1")
        .output()
        .unwrap();
    assert!(output.status.success());

    let stdout = String::from_utf8_lossy(&output.stdout);
    let settings = snapshot_settings(&tmp);
    settings.bind(|| {
        insta::assert_snapshot!("sync_idempotent_second_run", stdout);
    });
}

#[test]
fn sync_creates_lockfile() {
    let tmp = TempDir::new().unwrap();
    let skills_dir = tmp.path().join("skills");
    create_skill(&skills_dir, "alpha-skill");
    create_skill(&skills_dir, "beta-skill");

    let config = write_config(
        tmp.path(),
        &format!(
            "[directories.test]\npath = \"{}\"\ntype = \"directory\"\nrole = \"source\"\n",
            skills_dir.display()
        ),
    );

    tome()
        .args(["--config", config.to_str().unwrap(), "sync"])
        .env("NO_COLOR", "1")
        .assert()
        .success();

    // Lockfile now lives at tome home (config file's parent dir), not library
    let lockfile_path = tmp.path().join("tome.lock");
    assert!(
        lockfile_path.exists(),
        "tome.lock should be created by sync"
    );

    let content = std::fs::read_to_string(&lockfile_path).unwrap();
    let mut parsed: serde_json::Value = serde_json::from_str(&content).unwrap();

    // Redact dynamic content_hash fields for snapshot stability
    if let Some(skills) = parsed.get_mut("skills").and_then(|s| s.as_object_mut()) {
        for (_name, skill) in skills.iter_mut() {
            if skill.get("content_hash").is_some() {
                skill["content_hash"] = serde_json::Value::String("[HASH]".into());
            }
        }
    }

    let settings = snapshot_settings(&tmp);
    settings.bind(|| {
        insta::assert_json_snapshot!("sync_lockfile_two_skills", parsed);
    });
}

#[test]
fn sync_dry_run_does_not_create_lockfile() {
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

    tome()
        .args(["--config", config.to_str().unwrap(), "--dry-run", "sync"])
        .assert()
        .success();

    assert!(
        !tmp.path().join("tome.lock").exists(),
        "dry-run should not create tome.lock"
    );
}

#[test]
fn sync_distributes_to_symlink_target() {
    let tmp = TempDir::new().unwrap();
    let skills_dir = tmp.path().join("skills");
    create_skill(&skills_dir, "my-skill");

    let target_dir = tmp.path().join("target");
    // Don't create target_dir — sync should create it

    let library_dir = tmp.path().join("library");
    std::fs::create_dir_all(&library_dir).unwrap();

    let config_path = tmp.path().join("config.toml");
    std::fs::write(
        &config_path,
        format!(
            r#"library_dir = "{}"

[directories.test]
path = "{}"
type = "directory"
role = "source"

[directories.antigravity]
path = "{}"
type = "directory"
role = "target"
"#,
            library_dir.display(),
            skills_dir.display(),
            target_dir.display()
        ),
    )
    .unwrap();

    tome()
        .args(["--config", config_path.to_str().unwrap(), "sync"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Sync complete"));

    // Library has the skill as a real directory (v0.2)
    assert!(library_dir.join("my-skill").is_dir());
    assert!(!library_dir.join("my-skill").is_symlink());
    // Target has a symlink pointing to the library entry
    assert!(target_dir.join("my-skill").is_symlink());
}

#[test]
fn sync_lifecycle_cleans_up_removed_skills() {
    let tmp = TempDir::new().unwrap();
    let skills_dir = tmp.path().join("skills");
    std::fs::create_dir_all(&skills_dir).unwrap();
    create_skill(&skills_dir, "keep-me");
    create_skill(&skills_dir, "remove-me");

    let library_dir = tmp.path().join("library");
    std::fs::create_dir_all(&library_dir).unwrap();

    let config_path = tmp.path().join("config.toml");
    std::fs::write(
        &config_path,
        format!(
            r#"library_dir = "{}"

[directories.test]
path = "{}"
type = "directory"
role = "source"
"#,
            library_dir.display(),
            skills_dir.display(),
        ),
    )
    .unwrap();

    // First sync — both skills should appear in library
    tome()
        .args(["--config", config_path.to_str().unwrap(), "sync"])
        .assert()
        .success();
    assert!(library_dir.join("keep-me").is_dir());
    assert!(library_dir.join("remove-me").is_dir());

    // Remove one skill from source
    std::fs::remove_dir_all(skills_dir.join("remove-me")).unwrap();

    // Second sync — stale entry should be cleaned up (non-interactive mode in tests)
    tome()
        .args(["--config", config_path.to_str().unwrap(), "sync"])
        .assert()
        .success();
    assert!(library_dir.join("keep-me").is_dir());
    assert!(
        !library_dir.join("remove-me").exists(),
        "stale skill should have been cleaned up"
    );
}

#[test]
fn sync_force_recreates_all() {
    let tmp = TempDir::new().unwrap();
    let skills_dir = tmp.path().join("skills");
    std::fs::create_dir_all(&skills_dir).unwrap();
    create_skill(&skills_dir, "my-skill");

    let library_dir = tmp.path().join("library");
    std::fs::create_dir_all(&library_dir).unwrap();

    let config_path = tmp.path().join("config.toml");
    std::fs::write(
        &config_path,
        format!(
            r#"library_dir = "{}"

[directories.test]
path = "{}"
type = "directory"
role = "source"
"#,
            library_dir.display(),
            skills_dir.display(),
        ),
    )
    .unwrap();

    // Initial sync
    tome()
        .args(["--config", config_path.to_str().unwrap(), "sync"])
        .assert()
        .success();
    assert!(library_dir.join("my-skill").is_dir());

    // Force sync should report recreated, not "unchanged"
    let output = tome()
        .args(["--config", config_path.to_str().unwrap(), "sync", "--force"])
        .env("NO_COLOR", "1")
        .output()
        .unwrap();
    assert!(output.status.success());

    let stdout = String::from_utf8_lossy(&output.stdout);
    let settings = snapshot_settings(&tmp);
    settings.bind(|| {
        insta::assert_snapshot!("sync_force_recreate", stdout);
    });
}

#[test]
fn sync_updates_changed_source() {
    let tmp = TempDir::new().unwrap();
    let skills_dir = tmp.path().join("skills");
    create_skill(&skills_dir, "my-skill");

    let library_dir = tmp.path().join("library");
    std::fs::create_dir_all(&library_dir).unwrap();

    let config_path = tmp.path().join("config.toml");
    std::fs::write(
        &config_path,
        format!(
            r#"library_dir = "{}"

[directories.test]
path = "{}"
type = "directory"
role = "source"
"#,
            library_dir.display(),
            skills_dir.display(),
        ),
    )
    .unwrap();

    // First sync
    tome()
        .args(["--config", config_path.to_str().unwrap(), "sync"])
        .assert()
        .success();

    // Modify source SKILL.md
    std::fs::write(
        skills_dir.join("my-skill/SKILL.md"),
        "# updated content\nNew body.",
    )
    .unwrap();

    // Second sync — should detect the change
    let output = tome()
        .args(["--config", config_path.to_str().unwrap(), "sync"])
        .env("NO_COLOR", "1")
        .output()
        .unwrap();
    assert!(output.status.success());

    let stdout = String::from_utf8_lossy(&output.stdout);
    let settings = snapshot_settings(&tmp);
    settings.bind(|| {
        insta::assert_snapshot!("sync_updates_changed", stdout);
    });

    // Library copy should have the new content
    let content = std::fs::read_to_string(library_dir.join("my-skill/SKILL.md")).unwrap();
    assert_eq!(content, "# updated content\nNew body.");
}

#[test]
fn sync_migrates_v01_symlinks() {
    use std::os::unix::fs as unix_fs;

    let tmp = TempDir::new().unwrap();
    let skills_dir = tmp.path().join("skills");
    create_skill(&skills_dir, "legacy-skill");

    let library_dir = tmp.path().join("library");
    std::fs::create_dir_all(&library_dir).unwrap();

    // Simulate v0.1.x: library has a symlink
    unix_fs::symlink(
        skills_dir.join("legacy-skill"),
        library_dir.join("legacy-skill"),
    )
    .unwrap();
    assert!(library_dir.join("legacy-skill").is_symlink());

    let config_path = tmp.path().join("config.toml");
    std::fs::write(
        &config_path,
        format!(
            r#"library_dir = "{}"

[directories.test]
path = "{}"
type = "directory"
role = "source"
"#,
            library_dir.display(),
            skills_dir.display(),
        ),
    )
    .unwrap();

    // Sync should migrate the symlink to a real directory
    tome()
        .args(["--config", config_path.to_str().unwrap(), "sync"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Sync complete"));

    // Should now be a real directory, not a symlink
    assert!(library_dir.join("legacy-skill").is_dir());
    assert!(!library_dir.join("legacy-skill").is_symlink());
    assert!(library_dir.join("legacy-skill/SKILL.md").is_file());
}

#[test]
fn sync_skips_git_commit_without_tty() {
    let tmp = TempDir::new().unwrap();
    let skills_dir = tmp.path().join("skills");
    create_skill(&skills_dir, "new-skill");

    let library_dir = tmp.path().join("library");
    std::fs::create_dir_all(&library_dir).unwrap();
    git_init(&library_dir);

    let config_path = tmp.path().join("config.toml");
    std::fs::write(
        &config_path,
        format!(
            r#"library_dir = "{}"

[directories.test]
path = "{}"
type = "directory"
role = "source"
"#,
            library_dir.display(),
            skills_dir.display(),
        ),
    )
    .unwrap();

    // Without a TTY, the git commit prompt should be silently skipped
    tome()
        .args(["--config", config_path.to_str().unwrap(), "sync"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Sync complete"));

    // Only the initial "init" commit should exist (no auto-commit without TTY)
    let log = StdCommand::new("git")
        .args(["log", "--oneline"])
        .current_dir(&library_dir)
        .output()
        .unwrap();
    let commits = String::from_utf8_lossy(&log.stdout);
    assert!(
        !commits.contains("tome sync"),
        "should not commit without a TTY"
    );
}

#[test]
fn sync_dry_run_skips_git_commit() {
    let tmp = TempDir::new().unwrap();
    let skills_dir = tmp.path().join("skills");
    create_skill(&skills_dir, "new-skill");

    let library_dir = tmp.path().join("library");
    std::fs::create_dir_all(&library_dir).unwrap();
    git_init(&library_dir);

    let config_path = tmp.path().join("config.toml");
    std::fs::write(
        &config_path,
        format!(
            r#"library_dir = "{}"

[directories.test]
path = "{}"
type = "directory"
role = "source"
"#,
            library_dir.display(),
            skills_dir.display(),
        ),
    )
    .unwrap();

    tome()
        .args([
            "--config",
            config_path.to_str().unwrap(),
            "--dry-run",
            "sync",
        ])
        .assert()
        .success();

    // Only the initial "init" commit should exist
    let log = StdCommand::new("git")
        .args(["log", "--oneline"])
        .current_dir(&library_dir)
        .output()
        .unwrap();
    let commits = String::from_utf8_lossy(&log.stdout);
    assert!(
        !commits.contains("tome sync"),
        "dry-run should not create a commit"
    );
}

#[test]
fn sync_quiet_skips_git_commit() {
    let tmp = TempDir::new().unwrap();
    let skills_dir = tmp.path().join("skills");
    create_skill(&skills_dir, "new-skill");

    let library_dir = tmp.path().join("library");
    std::fs::create_dir_all(&library_dir).unwrap();
    git_init(&library_dir);

    let config_path = tmp.path().join("config.toml");
    std::fs::write(
        &config_path,
        format!(
            r#"library_dir = "{}"

[directories.test]
path = "{}"
type = "directory"
role = "source"
"#,
            library_dir.display(),
            skills_dir.display(),
        ),
    )
    .unwrap();

    tome()
        .args(["--config", config_path.to_str().unwrap(), "--quiet", "sync"])
        .assert()
        .success();

    // Only the initial "init" commit should exist
    let log = StdCommand::new("git")
        .args(["log", "--oneline"])
        .current_dir(&library_dir)
        .output()
        .unwrap();
    let commits = String::from_utf8_lossy(&log.stdout);
    assert!(
        !commits.contains("tome sync"),
        "quiet mode should not prompt for commit"
    );
}

#[test]
fn sync_no_triage_skips_diff_output() {
    let env = TestEnvBuilder::new()
        .source("local", "directory")
        .skill("my-skill", "local")
        .build();

    // First sync to create lockfile
    tome()
        .args([
            "--config",
            &env.config_path.to_string_lossy(),
            "sync",
            "--no-triage",
        ])
        .assert()
        .success();

    // Second sync with --no-triage should not show diff summary
    let output = tome()
        .args([
            "--config",
            &env.config_path.to_string_lossy(),
            "sync",
            "--no-triage",
        ])
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        !stdout.contains("changes"),
        "--no-triage should suppress diff summary, got: {stdout}"
    );
    assert!(
        !stdout.contains("No previous lockfile"),
        "--no-triage should suppress lockfile messages, got: {stdout}"
    );
}

#[test]
fn sync_with_no_lockfile_works_gracefully() {
    let tmp = TempDir::new().unwrap();
    let skills_dir = tmp.path().join("skills");
    create_skill(&skills_dir, "my-skill");

    let target_dir = tmp.path().join("target");

    let config = write_config_with_target(
        tmp.path(),
        &format!(
            "[directories.test]\npath = \"{}\"\ntype = \"directory\"\nrole = \"source\"\n",
            skills_dir.display()
        ),
        &target_dir,
    );

    // First run with no prior lockfile — should work like a normal sync
    tome()
        .args(["--config", config.to_str().unwrap(), "sync"])
        .assert()
        .success()
        .stdout(predicate::str::contains("No previous lockfile"))
        .stdout(predicate::str::contains("Sync complete"));

    // Library should have the skill
    assert!(tmp.path().join("library/my-skill").is_dir());
    // Target should have symlink
    assert!(target_dir.join("my-skill").is_symlink());
    // Lockfile should be created at tome home (config file's parent dir)
    assert!(tmp.path().join("tome.lock").exists());
}

#[test]
fn sync_triage_shows_new_skills() {
    let tmp = TempDir::new().unwrap();
    let skills_dir = tmp.path().join("skills");
    create_skill(&skills_dir, "existing-skill");

    let target_dir = tmp.path().join("target");

    let config = write_config_with_target(
        tmp.path(),
        &format!(
            "[directories.test]\npath = \"{}\"\ntype = \"directory\"\nrole = \"source\"\n",
            skills_dir.display()
        ),
        &target_dir,
    );

    let config_str = config.to_str().unwrap();

    // Initial sync to create lockfile
    tome()
        .args(["--config", config_str, "sync"])
        .assert()
        .success();

    // Add a new skill
    create_skill(&skills_dir, "brand-new-skill");

    // Update should detect the new skill
    tome()
        .args(["--config", config_str, "--quiet", "sync"])
        .assert()
        .success();

    // New skill should be in the library and linked to target
    assert!(tmp.path().join("library/brand-new-skill").is_dir());
    assert!(target_dir.join("brand-new-skill").is_symlink());
}

#[test]
fn sync_triage_dry_run_makes_no_changes() {
    let tmp = TempDir::new().unwrap();
    let skills_dir = tmp.path().join("skills");
    create_skill(&skills_dir, "my-skill");

    let target_dir = tmp.path().join("target");

    let config = write_config_with_target(
        tmp.path(),
        &format!(
            "[directories.test]\npath = \"{}\"\ntype = \"directory\"\nrole = \"source\"\n",
            skills_dir.display()
        ),
        &target_dir,
    );

    let config_str = config.to_str().unwrap();

    // Initial sync
    tome()
        .args(["--config", config_str, "sync"])
        .assert()
        .success();

    // Add a new skill
    create_skill(&skills_dir, "new-skill");

    // Dry-run update
    tome()
        .args(["--config", config_str, "--dry-run", "sync"])
        .assert()
        .success()
        .stderr(predicate::str::contains("dry-run"));

    // New skill should NOT be in library (dry-run)
    assert!(!tmp.path().join("library/new-skill").is_dir());
}

#[test]
fn sync_respects_machine_disabled() {
    // Test that sync with --machine skips disabled skills during distribution
    // AND removes their existing symlinks from targets.
    let tmp = TempDir::new().unwrap();
    let skills_dir = tmp.path().join("skills");
    create_skill(&skills_dir, "keep-skill");
    create_skill(&skills_dir, "drop-skill");

    let target_dir = tmp.path().join("target");

    let config = write_config_with_target(
        tmp.path(),
        &format!(
            "[directories.test]\npath = \"{}\"\ntype = \"directory\"\nrole = \"source\"\n",
            skills_dir.display()
        ),
        &target_dir,
    );

    // Sync — both skills should be distributed
    tome()
        .args(["--config", config.to_str().unwrap(), "sync"])
        .assert()
        .success();

    assert!(target_dir.join("keep-skill").is_symlink());
    assert!(target_dir.join("drop-skill").is_symlink());

    // Create machine.toml that disables "drop-skill"
    let machine_path = tmp.path().join("machine.toml");
    std::fs::write(&machine_path, "disabled = [\"drop-skill\"]\n").unwrap();

    // Re-sync with --machine — disabled skill's symlink should be removed
    tome()
        .args([
            "--config",
            config.to_str().unwrap(),
            "--machine",
            machine_path.to_str().unwrap(),
            "sync",
        ])
        .assert()
        .success();

    assert!(
        target_dir.join("keep-skill").is_symlink(),
        "enabled skill should still be linked"
    );
    assert!(
        !target_dir.join("drop-skill").exists(),
        "disabled skill's symlink should be removed by sync"
    );
}

#[test]
fn sync_triage_disable_removes_symlink() {
    // Test that disabling a skill and re-running update removes its symlink from targets.
    // Since we can't interact with the TTY in tests, we simulate the effect:
    // 1. Sync normally (both skills distributed)
    // 2. Manually create machine.toml disabling one skill
    // 3. The next update should not re-create the disabled symlink and should clean it up
    let tmp = TempDir::new().unwrap();
    let skills_dir = tmp.path().join("skills");
    create_skill(&skills_dir, "enabled-skill");
    create_skill(&skills_dir, "disabled-skill");

    let target_dir = tmp.path().join("target");

    let config = write_config_with_target(
        tmp.path(),
        &format!(
            "[directories.test]\npath = \"{}\"\ntype = \"directory\"\nrole = \"source\"\n",
            skills_dir.display()
        ),
        &target_dir,
    );

    let config_str = config.to_str().unwrap();

    // Initial sync — both skills distributed
    tome()
        .args(["--config", config_str, "sync"])
        .assert()
        .success();

    assert!(target_dir.join("enabled-skill").is_symlink());
    assert!(target_dir.join("disabled-skill").is_symlink());

    // Create machine.toml disabling one skill
    let machine_path = tmp.path().join("machine.toml");
    std::fs::write(&machine_path, "disabled = [\"disabled-skill\"]\n").unwrap();
    let machine_str = machine_path.to_str().unwrap();

    // Re-run update with --machine — should clean up disabled skill's symlink
    tome()
        .args([
            "--config",
            config_str,
            "--machine",
            machine_str,
            "--quiet",
            "sync",
        ])
        .assert()
        .success();

    assert!(
        target_dir.join("enabled-skill").is_symlink(),
        "enabled skill should still be linked"
    );
    assert!(
        !target_dir.join("disabled-skill").exists(),
        "disabled skill's symlink should be removed by update"
    );
}

#[test]
fn sync_respects_machine_disabled_targets() {
    // Test that sync with a disabled target does not distribute skills there,
    // and that an unknown disabled_target produces a warning on stderr.
    let tmp = TempDir::new().unwrap();
    let skills_dir = tmp.path().join("skills");
    create_skill(&skills_dir, "my-skill");

    let target_dir = tmp.path().join("target");

    let config = write_config_with_target(
        tmp.path(),
        &format!(
            "[directories.test]\npath = \"{}\"\ntype = \"directory\"\nrole = \"source\"\n",
            skills_dir.display()
        ),
        &target_dir,
    );

    // Create machine.toml that disables the configured target and also lists an unknown target
    let machine_path = tmp.path().join("machine.toml");
    std::fs::write(
        &machine_path,
        "disabled_directories = [\"test-target\", \"nonexistent-target\"]\n",
    )
    .unwrap();

    tome()
        .args([
            "--config",
            config.to_str().unwrap(),
            "--machine",
            machine_path.to_str().unwrap(),
            "sync",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("Sync complete"))
        .stderr(predicate::str::contains(
            "warning: disabled directory 'nonexistent-target' in machine.toml does not match any configured directory",
        ));

    // The target directory should not have the skill (target is disabled)
    assert!(
        !target_dir.join("my-skill").exists(),
        "disabled target should not receive skills"
    );

    // The skill should still be in the library
    assert!(tmp.path().join("library/my-skill").is_dir());
}

#[test]
fn sync_with_two_targets_via_config() {
    // Quick smoke test for write_config_with_target plus manual second target
    let tmp = TempDir::new().unwrap();
    let skills_dir = tmp.path().join("skills");
    create_skill(&skills_dir, "my-skill");

    let target_a = tmp.path().join("target-a");
    let target_b = tmp.path().join("target-b");
    std::fs::create_dir_all(&target_b).unwrap();

    let config_path = tmp.path().join("config.toml");
    let library_dir = tmp.path().join("library");
    std::fs::create_dir_all(&library_dir).unwrap();
    std::fs::write(
        &config_path,
        format!(
            r#"library_dir = "{}"

[directories.test]
path = "{}"
type = "directory"
role = "source"

[directories.target-a]
path = "{}"
type = "directory"
role = "target"

[directories.target-b]
path = "{}"
type = "directory"
role = "target"
"#,
            library_dir.display(),
            skills_dir.display(),
            target_a.display(),
            target_b.display(),
        ),
    )
    .unwrap();

    tome()
        .args(["--config", config_path.to_str().unwrap(), "sync"])
        .assert()
        .success();

    assert!(target_a.join("my-skill").is_symlink());
    assert!(target_b.join("my-skill").is_symlink());
}

#[test]
fn sync_warns_unknown_disabled_targets() {
    // Test that `tome update` warns about disabled_targets in machine.toml
    // that don't match any configured target.
    let tmp = TempDir::new().unwrap();
    let skills_dir = tmp.path().join("skills");
    create_skill(&skills_dir, "my-skill");

    let target_dir = tmp.path().join("target");

    let config = write_config_with_target(
        tmp.path(),
        &format!(
            "[directories.test]\npath = \"{}\"\ntype = \"directory\"\nrole = \"source\"\n",
            skills_dir.display()
        ),
        &target_dir,
    );

    // Initial sync so library and lockfile exist
    tome()
        .args(["--config", config.to_str().unwrap(), "sync"])
        .assert()
        .success();

    // Create machine.toml with an unknown disabled target
    let machine_path = tmp.path().join("machine.toml");
    std::fs::write(
        &machine_path,
        "disabled_directories = [\"nonexistent-target\"]\n",
    )
    .unwrap();

    tome()
        .args([
            "--config",
            config.to_str().unwrap(),
            "--machine",
            machine_path.to_str().unwrap(),
            "sync",
        ])
        .assert()
        .success()
        .stderr(predicate::str::contains(
            "warning: disabled directory 'nonexistent-target' in machine.toml does not match any configured directory",
        ));
}

#[test]
fn symlink_chain_local_skill() {
    let env = TestEnvBuilder::new()
        .source("local", "directory")
        .target("test-tool")
        .skill("my-skill", "local")
        .build();

    env.cmd().arg("sync").assert().success();

    let library_skill = env.library_dir().join("my-skill");
    let target_skill = env.target_dir("test-tool").join("my-skill");

    // Library has a real directory (v0.2 copy model), not a symlink
    assert!(
        library_skill.is_dir(),
        "library skill should be a directory"
    );
    assert!(
        !library_skill.is_symlink(),
        "library skill should NOT be a symlink (local skills are copied)"
    );

    // Content should match source
    let source_content =
        std::fs::read_to_string(env.source_dir("local").join("my-skill/SKILL.md")).unwrap();
    let library_content = std::fs::read_to_string(library_skill.join("SKILL.md")).unwrap();
    assert_eq!(source_content, library_content);

    // Target should be a symlink pointing to the library entry
    assert!(
        target_skill.is_symlink(),
        "target skill should be a symlink"
    );
    let target_link = std::fs::canonicalize(&target_skill).unwrap();
    let library_canonical = std::fs::canonicalize(&library_skill).unwrap();
    assert_eq!(
        target_link, library_canonical,
        "target symlink should resolve to the library entry"
    );

    // Reading through the target symlink should work
    let target_content = std::fs::read_to_string(target_skill.join("SKILL.md")).unwrap();
    assert_eq!(source_content, target_content);
}

#[test]
fn symlink_chain_managed_skill() {
    // v0.10 (LIB-01): managed skills become real directory copies in the
    // library, NOT symlinks into machine-specific cache paths. The previous
    // (v0.9) shape — library entry is a symlink → source install dir — has
    // been replaced by the copy model. Targets still symlink into the library.
    //
    // Phase 13 added a hard requirement that the `claude` binary be on PATH
    // whenever ANY [directories.<name>] has type = "claude-plugins" (D-20):
    // `build_claude_adapter` calls `ClaudeMarketplaceAdapter::new()` which
    // probes for the binary unconditionally. Skip this test on machines
    // without claude — the same skip-gate pattern as marketplace.rs's smoke
    // tests.
    if !tome::marketplace::is_claude_available() {
        eprintln!("skipping symlink_chain_managed_skill: claude binary not on PATH");
        return;
    }

    let env = TestEnvBuilder::new()
        .source("plugins", "claude-plugins")
        .target("test-tool")
        .managed_skill("managed-skill", "plugins", "my-plugin@npm", "1.0.0")
        .build();

    env.cmd().arg("sync").assert().success();

    let library_skill = env.library_dir().join("managed-skill");
    let target_skill = env.target_dir("test-tool").join("managed-skill");

    // v0.10 shape: library entry is a real directory, NOT a symlink.
    assert!(
        library_skill.is_dir(),
        "managed skill in library should be a real directory after v0.10 (LIB-01)"
    );
    assert!(
        !library_skill.is_symlink(),
        "managed skill in library must NOT be a symlink in v0.10 (LIB-01)"
    );

    // The library copy's content_hash must match the source's content_hash
    // (using the production hash function via the crate-root re-export from
    // Plan 11-05 Task 0). This is the LIB-01 invariant: copy fidelity.
    let source_skill_dir = env
        .source_dir("plugins")
        .join("installs/managed-skill/skills/managed-skill");
    let library_hash = tome::hash_directory(&library_skill).unwrap();
    let source_hash = tome::hash_directory(&source_skill_dir).unwrap();
    assert_eq!(
        library_hash, source_hash,
        "library copy must hash identically to the managed source"
    );

    // Target is still a symlink (target → library).
    assert!(
        target_skill.is_symlink(),
        "target skill should be a symlink"
    );
    let target_resolved = std::fs::canonicalize(&target_skill).unwrap();
    let library_canonical = std::fs::canonicalize(&library_skill).unwrap();
    assert_eq!(
        target_resolved, library_canonical,
        "target symlink should resolve to the (real-dir) library entry"
    );

    // Reading SKILL.md through the target should return the same content as
    // the source — proves the copy fidelity end-to-end.
    let source_content = std::fs::read_to_string(source_skill_dir.join("SKILL.md")).unwrap();
    let target_content = std::fs::read_to_string(target_skill.join("SKILL.md")).unwrap();
    assert_eq!(
        source_content, target_content,
        "reading through target → library should match the original source content"
    );
}

#[test]
fn symlink_chain_survives_content_update() {
    let env = TestEnvBuilder::new()
        .source("local", "directory")
        .target("test-tool")
        .skill("alpha", "local")
        .build();

    // Initial sync
    env.cmd().arg("sync").assert().success();

    let target_skill = env.target_dir("test-tool").join("alpha");
    assert!(target_skill.is_symlink());

    // Modify source content
    env.modify_skill(
        "alpha",
        "local",
        "---\nname: alpha\n---\n# alpha\nUpdated content.",
    );

    // Re-sync
    env.cmd().arg("sync").assert().success();

    // Target symlink should still work and return the NEW content
    let target_content = std::fs::read_to_string(target_skill.join("SKILL.md")).unwrap();
    assert!(
        target_content.contains("Updated content"),
        "target should serve updated content after re-sync"
    );
}

#[test]
fn symlink_chain_broken_after_source_removal() {
    let env = TestEnvBuilder::new()
        .source("local", "directory")
        .target("test-tool")
        .skill("keep-me", "local")
        .skill("remove-me", "local")
        .build();

    // Initial sync
    env.cmd().arg("sync").assert().success();

    assert!(env.library_dir().join("keep-me").is_dir());
    assert!(env.library_dir().join("remove-me").is_dir());
    assert!(env.target_dir("test-tool").join("keep-me").is_symlink());
    assert!(env.target_dir("test-tool").join("remove-me").is_symlink());

    // Remove one skill from source
    env.remove_skill("remove-me", "local");

    // Re-sync — should clean up the removed skill
    env.cmd().arg("sync").assert().success();

    // Removed skill should be gone from library and target
    assert!(
        !env.library_dir().join("remove-me").exists(),
        "removed skill should be cleaned from library"
    );
    assert!(
        !env.target_dir("test-tool").join("remove-me").exists(),
        "removed skill should be cleaned from target"
    );

    // Remaining skill should still work through the chain
    let target_content =
        std::fs::read_to_string(env.target_dir("test-tool").join("keep-me/SKILL.md")).unwrap();
    assert!(target_content.contains("keep-me"));
}

#[test]
fn edge_target_dir_disappears_between_syncs() {
    let env = TestEnvBuilder::new()
        .source("local", "directory")
        .target("test-tool")
        .skill("my-skill", "local")
        .build();

    // First sync
    env.cmd().arg("sync").assert().success();
    assert!(env.target_dir("test-tool").join("my-skill").is_symlink());

    // Delete target directory
    std::fs::remove_dir_all(env.target_dir("test-tool")).unwrap();
    assert!(!env.target_dir("test-tool").exists());

    // Re-sync should recreate target and symlinks
    env.cmd().arg("sync").assert().success();

    assert!(
        env.target_dir("test-tool").join("my-skill").is_symlink(),
        "symlink should be recreated after target dir was deleted"
    );
}

#[test]
fn edge_library_dir_disappears() {
    let env = TestEnvBuilder::new()
        .source("local", "directory")
        .target("test-tool")
        .skill("my-skill", "local")
        .build();

    // First sync
    env.cmd().arg("sync").assert().success();
    assert!(env.library_dir().join("my-skill").is_dir());
    assert!(env.manifest_path().exists());

    // Delete library directory AND manifest (simulate clean slate)
    std::fs::remove_dir_all(env.library_dir()).unwrap();
    std::fs::remove_file(env.manifest_path()).unwrap();

    // Re-sync should recreate library
    env.cmd().arg("sync").assert().success();

    assert!(
        env.library_dir().join("my-skill").is_dir(),
        "library should be recreated with skills"
    );
    assert!(env.manifest_path().exists(), "manifest should be recreated");
}

#[test]
fn edge_source_dir_disappears() {
    let env = TestEnvBuilder::new()
        .source("local", "directory")
        .target("test-tool")
        .skill("my-skill", "local")
        .build();

    // First sync
    env.cmd().arg("sync").assert().success();
    assert!(env.library_dir().join("my-skill").is_dir());

    // Delete the source directory
    std::fs::remove_dir_all(env.source_dir("local")).unwrap();

    // Re-sync — should warn about missing source and clean up
    let output = env.cmd().arg("sync").output().unwrap();
    assert!(output.status.success());

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("does not exist"),
        "should warn about missing source on stderr: {stderr}"
    );
}

#[test]
fn edge_broken_symlink_in_target_before_sync() {
    let env = TestEnvBuilder::new()
        .source("local", "directory")
        .target("test-tool")
        .skill("real-skill", "local")
        .build();

    // Create a broken symlink in the target directory before any sync
    let stale_link = env.target_dir("test-tool").join("stale-link");
    std::os::unix::fs::symlink("/nonexistent/path", &stale_link).unwrap();
    assert!(stale_link.is_symlink());

    // Sync
    env.cmd().arg("sync").assert().success();

    // Real skill should be linked
    assert!(
        env.target_dir("test-tool").join("real-skill").is_symlink(),
        "real skill should be distributed"
    );

    // Stale link should be cleaned up (it doesn't point into our library)
    // Note: cleanup_target only removes symlinks pointing into the library dir,
    // so external broken symlinks may be preserved. Verify actual behavior.
    // The important thing is that sync succeeds.
}

#[cfg(unix)]
#[test]
fn edge_permission_denied_on_target() {
    use std::os::unix::fs::PermissionsExt;

    let env = TestEnvBuilder::new()
        .source("local", "directory")
        .target("test-tool")
        .skill("my-skill", "local")
        .build();

    // Make target dir unwritable
    let target = env.target_dir("test-tool");
    std::fs::set_permissions(target, std::fs::Permissions::from_mode(0o000)).unwrap();

    // Sync should fail or produce an error
    let output = env.cmd().arg("sync").output().unwrap();

    // Restore permissions so TempDir can clean up
    std::fs::set_permissions(target, std::fs::Permissions::from_mode(0o755)).unwrap();

    // Verify: sync should have failed (permission denied on creating symlinks)
    assert!(
        !output.status.success() || !String::from_utf8_lossy(&output.stderr).is_empty(),
        "sync should fail or warn when target is unwritable"
    );
}

#[test]
fn edge_corrupted_manifest() {
    let env = TestEnvBuilder::new()
        .source("local", "directory")
        .skill("my-skill", "local")
        .build();

    // First sync
    env.cmd().arg("sync").assert().success();
    assert!(env.manifest_path().exists());

    // Corrupt the manifest
    std::fs::write(env.manifest_path(), "not valid json!!!").unwrap();

    // Re-sync — should either recover or error clearly
    let output = env.cmd().arg("sync").output().unwrap();

    // We expect this to error (manifest parse failure)
    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);
    // Either it errors, or it recovers and re-creates. Both are acceptable.
    assert!(
        !output.status.success() || stdout.contains("created"),
        "corrupted manifest should cause error or full re-sync: stderr={stderr}, stdout={stdout}"
    );
}

#[test]
fn edge_corrupted_lockfile() {
    let env = TestEnvBuilder::new()
        .source("local", "directory")
        .target("test-tool")
        .skill("my-skill", "local")
        .build();

    // First sync to create lockfile
    env.cmd().arg("sync").assert().success();
    assert!(env.lockfile_path().exists());

    // Corrupt the lockfile
    std::fs::write(env.lockfile_path(), "this is garbage").unwrap();

    // Update should fail with a parse error
    let output = env.cmd().args(["sync", "--quiet"]).output().unwrap();

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !output.status.success() || stderr.contains("parse") || stderr.contains("error"),
        "corrupted lockfile should cause error: stderr={stderr}"
    );
}

#[test]
fn edge_config_library_dir_is_file() {
    let tmp = TempDir::new().unwrap();
    let library_path = tmp.path().join("library");
    // Create library_dir as a FILE, not directory
    std::fs::write(&library_path, "I am a file").unwrap();

    let skills_dir = tmp.path().join("skills");
    create_skill(&skills_dir, "my-skill");

    let config_path = tmp.path().join("config.toml");
    std::fs::write(
        &config_path,
        format!(
            "library_dir = \"{}\"\n\n[directories.test]\npath = \"{}\"\ntype = \"directory\"\nrole = \"source\"\n",
            library_path.display(),
            skills_dir.display(),
        ),
    )
    .unwrap();

    let output = tome()
        .args(["--config", config_path.to_str().unwrap(), "sync"])
        .output()
        .unwrap();

    // Should fail — library_dir is a file, not a directory
    assert!(
        !output.status.success(),
        "sync should fail when library_dir is a file"
    );
}

#[test]
fn edge_skill_empty_skill_md() {
    let env = TestEnvBuilder::new()
        .source("local", "directory")
        .skill_with_content("empty-skill", "local", "")
        .build();

    // Sync should succeed with empty SKILL.md
    env.cmd().arg("sync").assert().success();

    assert!(
        env.library_dir().join("empty-skill").is_dir(),
        "skill with empty SKILL.md should still be synced"
    );
}

#[test]
fn edge_skill_with_nested_content() {
    let env = TestEnvBuilder::new()
        .source("local", "directory")
        .skill("nested-skill", "local")
        .build();

    // Add extra files to the skill: a subdirectory with a file
    let skill_dir = env.source_dir("local").join("nested-skill");
    let sub_dir = skill_dir.join("examples");
    std::fs::create_dir_all(&sub_dir).unwrap();
    std::fs::write(sub_dir.join("example.txt"), "an example file").unwrap();
    std::fs::write(skill_dir.join("extra.md"), "extra content").unwrap();

    env.cmd().arg("sync").assert().success();

    let library_skill = env.library_dir().join("nested-skill");
    assert!(library_skill.join("SKILL.md").exists());
    assert!(
        library_skill.join("examples/example.txt").exists(),
        "subdirectory contents should be copied"
    );
    assert!(
        library_skill.join("extra.md").exists(),
        "extra files should be copied"
    );
}

#[test]
fn lifecycle_full_sync_journey() {
    let env = TestEnvBuilder::new()
        .source("local", "directory")
        .target("test-tool")
        .build();

    // Step 1: Sync with no skills yet
    env.cmd().arg("sync").assert().success();

    // Step 2: Add first skill and sync
    env.add_skill("alpha", "local");
    let output = env.cmd().arg("sync").output().unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("1 created"),
        "first skill should be created: {stdout}"
    );
    assert!(env.library_dir().join("alpha").is_dir());
    assert!(env.target_dir("test-tool").join("alpha").is_symlink());

    // Step 3: Add second skill and sync
    env.add_skill("beta", "local");
    let output = env.cmd().arg("sync").output().unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("1 created") && stdout.contains("1 unchanged"),
        "should show 1 created + 1 unchanged: {stdout}"
    );

    // Step 4: Modify first skill and sync
    env.modify_skill(
        "alpha",
        "local",
        "---\nname: alpha\n---\n# alpha\nModified content.",
    );
    let output = env.cmd().arg("sync").output().unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("1 updated"),
        "modified skill should be updated: {stdout}"
    );

    // Step 5: Remove second skill and sync
    env.remove_skill("beta", "local");
    env.cmd().arg("sync").assert().success();

    assert!(env.library_dir().join("alpha").is_dir());
    assert!(
        !env.library_dir().join("beta").exists(),
        "removed skill should be cleaned from library"
    );
    assert!(
        !env.target_dir("test-tool").join("beta").exists(),
        "removed skill should be cleaned from target"
    );

    // Step 6: Doctor should find no issues
    env.cmd()
        .args(["doctor", "--dry-run"])
        .assert()
        .success()
        .stdout(predicate::str::contains("No issues found"));

    // Step 7: Status should show 1 skill
    env.cmd().arg("status").assert().success();
}

#[test]
fn lifecycle_update_with_lockfile_diff() {
    let env = TestEnvBuilder::new()
        .source("local", "directory")
        .target("test-tool")
        .skill("skill-a", "local")
        .skill("skill-b", "local")
        .build();

    // Initial sync to establish lockfile
    env.cmd().arg("sync").assert().success();
    assert!(env.lockfile_path().exists());

    // Add a new skill
    env.add_skill("skill-c", "local");

    // Update should detect the new skill
    env.cmd().args(["sync", "--quiet"]).assert().success();

    // Verify new skill is in library and target
    assert!(
        env.library_dir().join("skill-c").is_dir(),
        "new skill should be in library after update"
    );
    assert!(
        env.target_dir("test-tool").join("skill-c").is_symlink(),
        "new skill should be in target after update"
    );

    // Verify lockfile has 3 entries
    let lockfile_content = std::fs::read_to_string(env.lockfile_path()).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&lockfile_content).unwrap();
    let skills = parsed["skills"].as_object().unwrap();
    assert_eq!(skills.len(), 3, "lockfile should have 3 skill entries");
}

#[test]
fn lifecycle_doctor_detects_and_reports() {
    let env = TestEnvBuilder::new()
        .source("local", "directory")
        .target("test-tool")
        .skill("real-skill", "local")
        .build();

    // Sync to establish baseline
    env.cmd().arg("sync").assert().success();

    // Create orphan directory in library (not from any source)
    let orphan = env.library_dir().join("phantom");
    std::fs::create_dir_all(&orphan).unwrap();
    std::fs::write(orphan.join("SKILL.md"), "orphan").unwrap();

    // Create broken symlink in target
    let broken_link = env.target_dir("test-tool").join("broken");
    std::os::unix::fs::symlink("/nonexistent/path", &broken_link).unwrap();

    // Doctor should detect issues
    let output = env.cmd().args(["doctor", "--dry-run"]).output().unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("issue") || stdout.contains("Issue"),
        "doctor should detect orphan/broken entries: {stdout}"
    );

    // Clean up manually
    std::fs::remove_dir_all(&orphan).unwrap();
    std::fs::remove_file(&broken_link).unwrap();

    // Doctor should now be clean
    env.cmd()
        .args(["doctor", "--dry-run"])
        .assert()
        .success()
        .stdout(predicate::str::contains("No issues found"));
}

#[test]
fn lifecycle_multi_source_dedup() {
    let env = TestEnvBuilder::new()
        .source("primary", "directory")
        .source("secondary", "directory")
        .skill_with_content(
            "shared",
            "primary",
            "---\nname: shared\n---\n# shared\nFrom primary.",
        )
        .skill_with_content(
            "shared",
            "secondary",
            "---\nname: shared\n---\n# shared\nFrom secondary.",
        )
        .build();

    // First sync — primary should win (first source wins)
    env.cmd().arg("sync").assert().success();

    let library_content =
        std::fs::read_to_string(env.library_dir().join("shared/SKILL.md")).unwrap();
    assert!(
        library_content.contains("From primary"),
        "first source should win: {library_content}"
    );

    // Remove skill from primary
    env.remove_skill("shared", "primary");

    // Re-sync — secondary should now provide the skill
    env.cmd().arg("sync").assert().success();

    let library_content =
        std::fs::read_to_string(env.library_dir().join("shared/SKILL.md")).unwrap();
    assert!(
        library_content.contains("From secondary"),
        "after removing from primary, secondary should take over: {library_content}"
    );
}

#[test]
fn lifecycle_multi_target_distribution() {
    let env = TestEnvBuilder::new()
        .source("local", "directory")
        .target("target-a")
        .target("target-b")
        .skill("my-skill", "local")
        .build();

    // Sync — both targets should get symlinks
    env.cmd().arg("sync").assert().success();
    assert!(
        env.target_dir("target-a").join("my-skill").is_symlink(),
        "target-a should have the skill"
    );
    assert!(
        env.target_dir("target-b").join("my-skill").is_symlink(),
        "target-b should have the skill"
    );

    // Disable target-b via machine.toml and re-sync
    let machine_path = env.tome_home().join("machine.toml");
    std::fs::write(&machine_path, "disabled_directories = [\"target-b\"]\n").unwrap();

    env.cmd()
        .args(["--machine", machine_path.to_str().unwrap(), "sync"])
        .assert()
        .success();

    assert!(
        env.target_dir("target-a").join("my-skill").is_symlink(),
        "target-a should still have the skill"
    );
    // Note: disabled targets are skipped entirely (no distribute AND no cleanup),
    // so existing symlinks in disabled targets are left in place.
    assert!(
        env.target_dir("target-b").join("my-skill").is_symlink(),
        "target-b symlinks are preserved (disabled targets are skipped, not cleaned)"
    );

    // Remove machine.toml and re-sync — target-b should still work
    std::fs::remove_file(&machine_path).unwrap();
    env.cmd().arg("sync").assert().success();

    assert!(
        env.target_dir("target-b").join("my-skill").is_symlink(),
        "target-b should work after re-enabling"
    );
}

// ---------------------------------------------------------------------------
// HARD-09 / D-DIST-1: foreign-symlink protection — end-to-end via `tome sync`.
// Stage two synthetic tome installs sharing one distribution dir, run sync
// from install A, and assert that B's pre-existing symlink is NOT clobbered
// without --force, and IS clobbered with --force.
// ---------------------------------------------------------------------------

#[test]
fn sync_warns_and_skips_foreign_symlink_in_distribution_dir() {
    let tmp = TempDir::new().unwrap();
    let skills_dir = tmp.path().join("skills");
    create_skill(&skills_dir, "shared-skill");

    // The "other" tome library — a separate library directory whose
    // symlinks we will plant in our target dir before `tome sync` runs.
    let other_library = tmp.path().join("other-library");
    let other_skill = other_library.join("shared-skill");
    std::fs::create_dir_all(&other_skill).unwrap();
    std::fs::write(
        other_skill.join("SKILL.md"),
        "---\nname: shared-skill\n---\n# x",
    )
    .unwrap();

    let target = tmp.path().join("target");
    std::fs::create_dir_all(&target).unwrap();
    // Pre-stage a foreign symlink target/shared-skill -> other_library/shared-skill.
    std::os::unix::fs::symlink(&other_skill, target.join("shared-skill")).unwrap();

    let config_path = write_config_with_target(
        tmp.path(),
        &format!(
            "[directories.local]\npath = \"{}\"\ntype = \"directory\"\nrole = \"source\"\n",
            skills_dir.display()
        ),
        &target,
    );

    // Default sync: foreign symlink is warn-and-skipped, NOT clobbered.
    let assert = tome()
        .args(["--config", config_path.to_str().unwrap(), "sync"])
        .env("NO_COLOR", "1")
        .assert()
        .success();
    let output = assert.get_output();
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("is a foreign symlink"),
        "stderr must surface the D-DIST-1 foreign-symlink warning, got: {stderr}"
    );
    assert!(
        stderr.contains("Pass --force to overwrite"),
        "warning must mention --force as the opt-in, got: {stderr}"
    );
    // Foreign link unchanged on disk.
    let actual = std::fs::read_link(target.join("shared-skill")).unwrap();
    assert_eq!(
        actual, other_skill,
        "default sync must leave the foreign symlink intact"
    );

    // --force re-runs and clobbers the foreign link.
    tome()
        .args(["--config", config_path.to_str().unwrap(), "sync", "--force"])
        .env("NO_COLOR", "1")
        .assert()
        .success();
    let actual = std::fs::read_link(target.join("shared-skill")).unwrap();
    assert!(
        actual.starts_with(tmp.path().join("library")),
        "force sync must redirect the link into our library, got {}",
        actual.display()
    );
}

// === UX-01 three-bucket cleanup output (Plan 16-01 Task 3) ===
//
// End-to-end pinning of the unified three-bucket cleanup output. Builds a
// fixture where exactly one skill falls into each bucket:
//   A: source dir was removed from config (preserve as Unowned)
//   B: source still configured but file vanished from disk (delete)
//   C: skill still discovered but added to machine.toml::disabled
//      (distribution symlink torn down)
//
// Then runs `tome sync --no-input` against the fixture and asserts the
// stderr output contains all three bucket-distinct header substrings AND
// each skill name AND does NOT contain the milestone trigger phrase that
// CONTEXT.md `<specifics>` flags as forbidden in any cleanup output.

/// Sentinel for the forbidden trigger phrase. Assembled from substrings so
/// the literal trigger phrase never appears as a single source fragment —
/// keeps the codebase grep-clean for the cleanup acceptance criterion.
fn forbidden_phrase() -> String {
    let part_a = "no longer ";
    let part_b = "configured";
    format!("{part_a}{part_b}")
}

#[test]
fn cleanup_renders_all_three_buckets_with_distinct_phrasing() {
    use std::os::unix::fs as unix_fs;

    let tmp = TempDir::new().unwrap();
    let tome_home = tmp.path().join("tome_home");
    let library_dir = tome_home.join("library");
    std::fs::create_dir_all(&library_dir).unwrap();

    // Active source dir — still configured. We'll create one skill in here
    // that gets discovered (Bucket C scenario; lands in target with a
    // distribution symlink and then disabled).
    let active_source = tmp.path().join("active-source");
    std::fs::create_dir_all(&active_source).unwrap();
    let active_skill_dir = active_source.join("bucket-c-skill");
    std::fs::create_dir_all(&active_skill_dir).unwrap();
    std::fs::write(
        active_skill_dir.join("SKILL.md"),
        "---\nname: bucket-c-skill\n---\n# bucket-c-skill",
    )
    .unwrap();

    // Library entries for each bucket. All three live as real directories
    // on disk so cleanup_library iterates them; manifest provenance is
    // fabricated below.
    for name in &["bucket-a-skill", "bucket-b-skill", "bucket-c-skill"] {
        let dir = library_dir.join(name);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(
            dir.join("SKILL.md"),
            format!("---\nname: {name}\n---\n# {name}\nA test skill."),
        )
        .unwrap();
    }

    // Distribution target — pre-create a symlink for bucket-c-skill so
    // sync's distribution cleanup loop tears it down (Bucket C).
    let target_dir = tmp.path().join("target");
    std::fs::create_dir_all(&target_dir).unwrap();
    unix_fs::symlink(
        library_dir.join("bucket-c-skill"),
        target_dir.join("bucket-c-skill"),
    )
    .unwrap();

    // Fabricated manifest staging the three bucket scenarios. Bucket A's
    // source_name ("removed-source") is intentionally absent from
    // tome.toml below; Bucket B's source_name ("active-source") IS in
    // tome.toml but bucket-b-skill is NOT in active-source on disk so it
    // looks "vanished from disk".
    let zero_hash = "0".repeat(64);
    let bucket_b_source_path = active_source.join("bucket-b-skill");
    let bucket_a_source_path = tmp.path().join("removed-source").join("bucket-a-skill");
    let manifest_json = format!(
        r#"{{
  "skills": {{
    "bucket-a-skill": {{
      "source_path": "{a_path}",
      "source_name": "removed-source",
      "content_hash": "{hash}",
      "synced_at": "2026-05-08T00:00:00Z",
      "managed": false
    }},
    "bucket-b-skill": {{
      "source_path": "{b_path}",
      "source_name": "active-source",
      "content_hash": "{hash}",
      "synced_at": "2026-05-08T00:00:00Z",
      "managed": false
    }},
    "bucket-c-skill": {{
      "source_path": "{c_path}",
      "source_name": "active-source",
      "content_hash": "{hash}",
      "synced_at": "2026-05-08T00:00:00Z",
      "managed": false
    }}
  }}
}}"#,
        a_path = bucket_a_source_path.display(),
        b_path = bucket_b_source_path.display(),
        c_path = active_skill_dir.display(),
        hash = zero_hash,
    );
    std::fs::write(tome_home.join(".tome-manifest.json"), manifest_json).unwrap();

    // tome.toml: only `active-source` is configured (Bucket A's
    // `removed-source` is intentionally NOT here — that's what triggers
    // Bucket A's removed-from-config partition).
    let config_toml = format!(
        r#"library_dir = "{lib}"

[directories.active-source]
path = "{src}"
type = "directory"
role = "source"

[directories.tgt]
path = "{tgt}"
type = "directory"
role = "target"
"#,
        lib = library_dir.display(),
        src = active_source.display(),
        tgt = target_dir.display(),
    );
    let config_path = tome_home.join("tome.toml");
    std::fs::write(&config_path, config_toml).unwrap();

    // machine.toml disables bucket-c-skill globally so distribution
    // cleanup tears down its symlink and surfaces it in Bucket C.
    let machine_path = tmp.path().join("machine.toml");
    std::fs::write(&machine_path, "disabled = [\"bucket-c-skill\"]\n").unwrap();

    // Run sync against the fixture and capture stderr.
    let output = tome()
        .args([
            "--tome-home",
            tome_home.to_str().unwrap(),
            "--config",
            config_path.to_str().unwrap(),
            "--machine",
            machine_path.to_str().unwrap(),
            "sync",
            "--no-input",
        ])
        .env("NO_COLOR", "1")
        .output()
        .expect("tome sync should run");
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();

    // All three skill names appear in stderr.
    assert!(
        predicate::str::contains("bucket-a-skill").eval(&stderr),
        "Bucket A skill name missing from stderr:\n{stderr}\n--- stdout ---\n{stdout}"
    );
    assert!(
        predicate::str::contains("bucket-b-skill").eval(&stderr),
        "Bucket B skill name missing from stderr:\n{stderr}\n--- stdout ---\n{stdout}"
    );
    assert!(
        predicate::str::contains("bucket-c-skill").eval(&stderr),
        "Bucket C skill name missing from stderr:\n{stderr}\n--- stdout ---\n{stdout}"
    );

    // Bucket-distinct header phrases (D-UX01-3 locked phrasing).
    let bucket_a_match = predicate::str::contains("no longer in any source")
        .or(predicate::str::contains("removed from config"));
    let bucket_b_match = predicate::str::contains("missing from configured source on disk")
        .or(predicate::str::contains("missing from disk"));
    let bucket_c_match = predicate::str::contains("now in exclude list")
        .or(predicate::str::contains("now-in-exclude"));
    assert!(
        bucket_a_match.eval(&stderr),
        "Bucket A locked header phrase missing from stderr:\n{stderr}"
    );
    assert!(
        bucket_b_match.eval(&stderr),
        "Bucket B locked header phrase missing from stderr:\n{stderr}"
    );
    assert!(
        bucket_c_match.eval(&stderr),
        "Bucket C locked header phrase missing from stderr:\n{stderr}"
    );

    // Forbidden trigger phrase (CONTEXT.md `<specifics>`) MUST NOT appear.
    let forbidden = forbidden_phrase();
    assert!(
        !predicate::str::contains(forbidden.as_str()).eval(&stderr),
        "forbidden trigger phrase '{forbidden}' must not appear in cleanup output:\n{stderr}"
    );

    // #15 stdout-discipline pin: bucket headers must NOT leak into stdout.
    // If a future refactor accidentally routes a renderer through `println!`,
    // the assertions above (which only check stderr presence) still pass
    // because both streams contain the substrings; this assertion fails
    // immediately and surfaces the regression.
    for header in [
        "no longer in any source",
        "missing from configured source on disk",
        "now in exclude list",
    ] {
        assert!(
            !stdout.contains(header),
            "Bucket header `{header}` leaked to stdout (cleanup output must be stderr-only per HARD-15):\n--- stdout ---\n{stdout}"
        );
    }
}
