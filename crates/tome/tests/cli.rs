use assert_cmd::{Command, cargo_bin_cmd};
use assert_fs::TempDir;
use predicates::prelude::*;
use std::process::Command as StdCommand;

fn tome() -> Command {
    cargo_bin_cmd!("tome")
}

fn write_config(dir: &std::path::Path, sources_toml: &str) -> std::path::PathBuf {
    let config_path = dir.join("config.toml");
    let library_dir = dir.join("library");
    std::fs::create_dir_all(&library_dir).unwrap();
    std::fs::write(
        &config_path,
        format!(
            "library_dir = \"{}\"\n{}",
            library_dir.display(),
            sources_toml
        ),
    )
    .unwrap();
    config_path
}

fn write_config_with_target(
    dir: &std::path::Path,
    sources_toml: &str,
    target_dir: &std::path::Path,
) -> std::path::PathBuf {
    let config_path = dir.join("config.toml");
    let library_dir = dir.join("library");
    std::fs::create_dir_all(&library_dir).unwrap();
    std::fs::write(
        &config_path,
        format!(
            "library_dir = \"{}\"\n{}\n[targets.test-target]\nenabled = true\nmethod = \"symlink\"\nskills_dir = \"{}\"\n",
            library_dir.display(),
            sources_toml,
            target_dir.display()
        ),
    )
    .unwrap();
    config_path
}

fn create_skill(dir: &std::path::Path, name: &str) {
    let skill_dir = dir.join(name);
    std::fs::create_dir_all(&skill_dir).unwrap();
    std::fs::write(
        skill_dir.join("SKILL.md"),
        format!("---\nname: {name}\n---\n# {name}\nA test skill."),
    )
    .unwrap();
}

// -- Help & version --

#[test]
fn help_shows_usage() {
    tome()
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "Sync AI coding skills across tools",
        ));
}

#[test]
fn version_shows_version() {
    tome()
        .arg("--version")
        .assert()
        .success()
        .stdout(predicate::str::contains(env!("CARGO_PKG_VERSION")));
}

// -- List --

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
            "[[sources]]\nname = \"test\"\npath = \"{}\"\ntype = \"directory\"\n",
            skills_dir.display()
        ),
    );

    tome()
        .args(["--config", config.to_str().unwrap(), "list"])
        .assert()
        .success()
        .stdout(
            predicate::str::contains("my-skill")
                .and(predicate::str::contains("other-skill"))
                .and(predicate::str::contains("2 skill(s) total")),
        );
}

// -- Sync --

#[test]
fn sync_dry_run_makes_no_changes() {
    let tmp = TempDir::new().unwrap();
    let skills_dir = tmp.path().join("skills");
    create_skill(&skills_dir, "test-skill");

    let config = write_config(
        tmp.path(),
        &format!(
            "[[sources]]\nname = \"test\"\npath = \"{}\"\ntype = \"directory\"\n",
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
            "[[sources]]\nname = \"test\"\npath = \"{}\"\ntype = \"directory\"\n",
            skills_dir.display()
        ),
    );

    tome()
        .args(["--config", config.to_str().unwrap(), "sync"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Sync complete"));

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
            "[[sources]]\nname = \"test\"\npath = \"{}\"\ntype = \"directory\"\n",
            skills_dir.display()
        ),
    );

    let config_str = config.to_str().unwrap();

    // First sync
    tome()
        .args(["--config", config_str, "sync"])
        .assert()
        .success();

    // Second sync — should report 0 created, 1 unchanged
    tome()
        .args(["--config", config_str, "sync"])
        .assert()
        .success()
        .stdout(predicate::str::contains("0 created").and(predicate::str::contains("1 unchanged")));
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
            "[[sources]]\nname = \"test\"\npath = \"{}\"\ntype = \"directory\"\n",
            skills_dir.display()
        ),
    );

    tome()
        .args(["--config", config.to_str().unwrap(), "sync"])
        .assert()
        .success();

    // Lockfile now lives at tome home (config file's parent dir), not library
    let lockfile_path = tmp.path().join("tome.lock");
    assert!(
        lockfile_path.exists(),
        "tome.lock should be created by sync"
    );

    let content = std::fs::read_to_string(&lockfile_path).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&content).unwrap();
    assert_eq!(parsed["version"], 1);
    assert!(parsed["skills"]["alpha-skill"].is_object());
    assert!(parsed["skills"]["beta-skill"].is_object());
    assert_eq!(parsed["skills"]["alpha-skill"]["source_name"], "test");
    assert!(
        !parsed["skills"]["alpha-skill"]["content_hash"]
            .as_str()
            .unwrap()
            .is_empty(),
        "content_hash should be present and non-empty"
    );
}

#[test]
fn sync_dry_run_does_not_create_lockfile() {
    let tmp = TempDir::new().unwrap();
    let skills_dir = tmp.path().join("skills");
    create_skill(&skills_dir, "my-skill");

    let config = write_config(
        tmp.path(),
        &format!(
            "[[sources]]\nname = \"test\"\npath = \"{}\"\ntype = \"directory\"\n",
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

// -- Status --

#[test]
fn status_shows_library_info() {
    let tmp = TempDir::new().unwrap();
    let config = write_config(tmp.path(), "");

    tome()
        .args(["--config", config.to_str().unwrap(), "status"])
        .assert()
        .success()
        .stdout(
            predicate::str::contains("Library:")
                .and(predicate::str::contains("Sources:"))
                .and(predicate::str::contains("Targets:")),
        );
}

// -- Config --

#[test]
fn config_path_prints_default_path() {
    tome()
        .args(["config", "--path"])
        .assert()
        .success()
        .stdout(predicate::str::contains("tome.toml"));
}

// -- Sync with targets --

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

[[sources]]
name = "test"
path = "{}"
type = "directory"

[targets.antigravity]
enabled = true
method = "symlink"
skills_dir = "{}"
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

[[sources]]
name = "test"
path = "{}"
type = "directory"
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

[[sources]]
name = "test"
path = "{}"
type = "directory"
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
    tome()
        .args(["--config", config_path.to_str().unwrap(), "sync", "--force"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Sync complete"))
        .stdout(predicate::str::contains("updated").or(predicate::str::contains("created")));
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

[[sources]]
name = "test"
path = "{}"
type = "directory"
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
    tome()
        .args(["--config", config_path.to_str().unwrap(), "sync"])
        .assert()
        .success()
        .stdout(predicate::str::contains("1 updated"));

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

[[sources]]
name = "test"
path = "{}"
type = "directory"
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

// -- Doctor --

#[test]
fn doctor_with_clean_state() {
    let tmp = TempDir::new().unwrap();
    let config = write_config(tmp.path(), "");

    tome()
        .args(["--config", config.to_str().unwrap(), "doctor"])
        .assert()
        .success()
        .stdout(predicate::str::contains("No issues found"));
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

// -- Pre-init state (unconfigured) --

#[test]
fn status_without_config_shows_init_prompt() {
    let tmp = TempDir::new().unwrap();
    // Point library_dir at a nonexistent dir (no sources) to simulate unconfigured state.
    // Using write_config would create library_dir, defeating the purpose.
    let config_path = tmp.path().join("config.toml");
    let nonexistent_library = tmp.path().join("nonexistent-library");
    std::fs::write(
        &config_path,
        format!("library_dir = \"{}\"", nonexistent_library.display()),
    )
    .unwrap();

    tome()
        .args(["--config", config_path.to_str().unwrap(), "status"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Not configured yet"))
        .stdout(predicate::str::contains("tome init"));
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

// -- Git commit on sync --

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

[[sources]]
name = "test"
path = "{}"
type = "directory"
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

[[sources]]
name = "test"
path = "{}"
type = "directory"
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
fn list_json_outputs_valid_json() {
    let tmp = TempDir::new().unwrap();
    let skills_dir = tmp.path().join("skills");
    create_skill(&skills_dir, "alpha-skill");
    create_skill(&skills_dir, "beta-skill");

    let config = write_config(
        tmp.path(),
        &format!(
            "[[sources]]\nname = \"test-src\"\npath = \"{}\"\ntype = \"directory\"\n",
            skills_dir.display()
        ),
    );

    let output = tome()
        .args(["--config", config.to_str().unwrap(), "list", "--json"])
        .output()
        .unwrap();
    assert!(output.status.success());

    let parsed: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("stdout should be valid JSON");
    let arr = parsed.as_array().expect("should be a JSON array");
    assert_eq!(arr.len(), 2);

    // Each entry should have name, source, and path fields
    for entry in arr {
        assert!(entry.get("name").is_some(), "missing 'name' field");
        assert!(entry.get("source").is_some(), "missing 'source' field");
        assert!(entry.get("path").is_some(), "missing 'path' field");
    }

    // Check that our source name appears
    assert!(arr.iter().any(|e| e["source"] == "test-src"));
    // Check both skill names are present
    let names: Vec<&str> = arr.iter().map(|e| e["name"].as_str().unwrap()).collect();
    assert!(names.contains(&"alpha-skill"));
    assert!(names.contains(&"beta-skill"));
}

#[test]
fn list_json_with_quiet_still_outputs_json() {
    let tmp = TempDir::new().unwrap();
    let skills_dir = tmp.path().join("skills");
    create_skill(&skills_dir, "my-skill");

    let config = write_config(
        tmp.path(),
        &format!(
            "[[sources]]\nname = \"test\"\npath = \"{}\"\ntype = \"directory\"\n",
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
        .output()
        .unwrap();
    assert!(output.status.success());

    let parsed: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("stdout should be valid JSON");
    let arr = parsed.as_array().expect("should be a JSON array");
    assert_eq!(arr.len(), 0);
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

[[sources]]
name = "test"
path = "{}"
type = "directory"
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

// -- Update command --

#[test]
fn update_with_no_lockfile_works_gracefully() {
    let tmp = TempDir::new().unwrap();
    let skills_dir = tmp.path().join("skills");
    create_skill(&skills_dir, "my-skill");

    let target_dir = tmp.path().join("target");

    let config = write_config_with_target(
        tmp.path(),
        &format!(
            "[[sources]]\nname = \"test\"\npath = \"{}\"\ntype = \"directory\"\n",
            skills_dir.display()
        ),
        &target_dir,
    );

    // First run with no prior lockfile — should work like a normal sync
    tome()
        .args(["--config", config.to_str().unwrap(), "update"])
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
fn update_shows_new_skills() {
    let tmp = TempDir::new().unwrap();
    let skills_dir = tmp.path().join("skills");
    create_skill(&skills_dir, "existing-skill");

    let target_dir = tmp.path().join("target");

    let config = write_config_with_target(
        tmp.path(),
        &format!(
            "[[sources]]\nname = \"test\"\npath = \"{}\"\ntype = \"directory\"\n",
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
        .args(["--config", config_str, "--quiet", "update"])
        .assert()
        .success();

    // New skill should be in the library and linked to target
    assert!(tmp.path().join("library/brand-new-skill").is_dir());
    assert!(target_dir.join("brand-new-skill").is_symlink());
}

#[test]
fn update_dry_run_makes_no_changes() {
    let tmp = TempDir::new().unwrap();
    let skills_dir = tmp.path().join("skills");
    create_skill(&skills_dir, "my-skill");

    let target_dir = tmp.path().join("target");

    let config = write_config_with_target(
        tmp.path(),
        &format!(
            "[[sources]]\nname = \"test\"\npath = \"{}\"\ntype = \"directory\"\n",
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
        .args(["--config", config_str, "--dry-run", "update"])
        .assert()
        .success()
        .stderr(predicate::str::contains("dry-run"));

    // New skill should NOT be in library (dry-run)
    assert!(!tmp.path().join("library/new-skill").is_dir());
}

// -- Sync with machine prefs --

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
            "[[sources]]\nname = \"test\"\npath = \"{}\"\ntype = \"directory\"\n",
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
fn update_disable_removes_symlink() {
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
            "[[sources]]\nname = \"test\"\npath = \"{}\"\ntype = \"directory\"\n",
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
            "update",
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
            "[[sources]]\nname = \"test\"\npath = \"{}\"\ntype = \"directory\"\n",
            skills_dir.display()
        ),
        &target_dir,
    );

    // Create machine.toml that disables the configured target and also lists an unknown target
    let machine_path = tmp.path().join("machine.toml");
    std::fs::write(
        &machine_path,
        "disabled_targets = [\"test-target\", \"nonexistent-target\"]\n",
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
            "warning: disabled target 'nonexistent-target' in machine.toml does not match any configured target",
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
fn update_warns_unknown_disabled_targets() {
    // Test that `tome update` warns about disabled_targets in machine.toml
    // that don't match any configured target.
    let tmp = TempDir::new().unwrap();
    let skills_dir = tmp.path().join("skills");
    create_skill(&skills_dir, "my-skill");

    let target_dir = tmp.path().join("target");

    let config = write_config_with_target(
        tmp.path(),
        &format!(
            "[[sources]]\nname = \"test\"\npath = \"{}\"\ntype = \"directory\"\n",
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
        "disabled_targets = [\"nonexistent-target\"]\n",
    )
    .unwrap();

    tome()
        .args([
            "--config",
            config.to_str().unwrap(),
            "--machine",
            machine_path.to_str().unwrap(),
            "update",
        ])
        .assert()
        .success()
        .stderr(predicate::str::contains(
            "warning: disabled target 'nonexistent-target' in machine.toml does not match any configured target",
        ));
}
