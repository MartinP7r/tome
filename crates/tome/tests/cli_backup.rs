use predicates::prelude::*;

mod common;
use common::*;

/// HARD-14 (closes #500): the `tome backup init` / `snapshot` paths run
/// real `git commit` subprocesses. On developer machines with global
/// `commit.gpgsign=true`, those commits inherit the signing requirement
/// and fail the integration tests when the gpg agent refuses (or the
/// test process can't authenticate). Pointing the subprocess at empty
/// fallback git config files isolates it from the user's global config.
fn isolate_git_config(cmd: &mut assert_cmd::Command, tmp: &std::path::Path) {
    cmd.env("GIT_CONFIG_GLOBAL", tmp.join(".gitconfig-empty"))
        .env("GIT_CONFIG_SYSTEM", tmp.join(".gitconfig-empty-system"))
        .env_remove("GIT_CONFIG_NOSYSTEM");
}

#[test]
fn backup_init_and_snapshot() {
    let env = TestEnvBuilder::new()
        .source("local", "directory")
        .skill("my-skill", "local")
        .build();

    // Sync first to populate the library
    let mut cmd = tome();
    isolate_git_config(&mut cmd, env.tome_home());
    cmd.args(["--config", &env.config_path.to_string_lossy(), "sync"])
        .assert()
        .success();

    // Init backup (commits existing library content)
    let mut cmd = tome();
    isolate_git_config(&mut cmd, env.tome_home());
    cmd.args([
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
    let mut cmd = tome();
    isolate_git_config(&mut cmd, env.tome_home());
    cmd.args([
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
    let mut cmd = tome();
    isolate_git_config(&mut cmd, env.tome_home());
    cmd.args(["--config", &env.config_path.to_string_lossy(), "sync"])
        .assert()
        .success();

    // Init backup
    let mut cmd = tome();
    isolate_git_config(&mut cmd, env.tome_home());
    cmd.args([
        "--config",
        &env.config_path.to_string_lossy(),
        "backup",
        "init",
    ])
    .assert()
    .success();

    // Add a file and create a snapshot
    std::fs::write(env.library_dir.join("extra.txt"), "new content").unwrap();
    let mut cmd = tome();
    isolate_git_config(&mut cmd, env.tome_home());
    cmd.args([
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
    let mut cmd = tome();
    isolate_git_config(&mut cmd, env.tome_home());
    cmd.args([
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
