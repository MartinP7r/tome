use predicates::prelude::*;

mod common;
use common::*;

#[test]
fn backup_init_and_snapshot() {
    let env = TestEnvBuilder::new()
        .source("local", "directory")
        .skill("my-skill", "local")
        .build();

    // Sync first to populate the library
    tome()
        .args(["--config", &env.config_path.to_string_lossy(), "sync"])
        .assert()
        .success();

    // Init backup (commits existing library content)
    tome()
        .args([
            "--config",
            &env.config_path.to_string_lossy(),
            "backup",
            "init",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("Initialized backup repo"));

    // Add a new file to the library so there's something to snapshot
    std::fs::write(env.library_dir.join("extra.txt"), "new content").unwrap();

    // Snapshot
    tome()
        .args([
            "--config",
            &env.config_path.to_string_lossy(),
            "backup",
            "snapshot",
            "-m",
            "test snapshot",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("Snapshot created"));
}

#[test]
fn backup_list_shows_history() {
    let env = TestEnvBuilder::new()
        .source("local", "directory")
        .skill("skill-a", "local")
        .build();

    // Sync to populate library
    tome()
        .args(["--config", &env.config_path.to_string_lossy(), "sync"])
        .assert()
        .success();

    // Init backup
    tome()
        .args([
            "--config",
            &env.config_path.to_string_lossy(),
            "backup",
            "init",
        ])
        .assert()
        .success();

    // Add a file and create a snapshot
    std::fs::write(env.library_dir.join("extra.txt"), "new content").unwrap();
    tome()
        .args([
            "--config",
            &env.config_path.to_string_lossy(),
            "backup",
            "snapshot",
            "-m",
            "first snapshot",
        ])
        .assert()
        .success();

    // List should show both the initial backup and the snapshot
    tome()
        .args([
            "--config",
            &env.config_path.to_string_lossy(),
            "backup",
            "list",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("first snapshot"))
        .stdout(predicate::str::contains("Initial tome backup"));
}
