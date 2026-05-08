use predicates::prelude::*;

mod common;
use common::*;

#[test]
fn eject_removes_symlinks_and_sync_restores() {
    let env = TestEnvBuilder::new()
        .source("local", "directory")
        .target("test-target")
        .skill("my-skill", "local")
        .build();

    // First sync to distribute
    env.cmd().arg("sync").assert().success();
    assert!(
        env.target_dir("test-target").join("my-skill").is_symlink(),
        "skill should be distributed after sync"
    );

    // Eject (non-interactive, stdin is not a terminal in tests so no prompt)
    env.cmd()
        .arg("eject")
        .assert()
        .success()
        .stdout(predicate::str::contains("Removed 1 symlink(s)"));

    assert!(
        !env.target_dir("test-target").join("my-skill").exists(),
        "symlink should be removed after eject"
    );
    assert!(
        env.library_dir().join("my-skill").is_dir(),
        "library should remain intact after eject"
    );

    // Sync again to restore
    env.cmd().arg("sync").assert().success();
    assert!(
        env.target_dir("test-target").join("my-skill").is_symlink(),
        "skill should be restored after re-sync"
    );
}

#[test]
fn eject_dry_run_does_not_remove() {
    let env = TestEnvBuilder::new()
        .source("local", "directory")
        .target("test-target")
        .skill("my-skill", "local")
        .build();

    // First sync to distribute
    env.cmd().arg("sync").assert().success();
    assert!(env.target_dir("test-target").join("my-skill").is_symlink());

    // Eject with dry-run
    env.cmd()
        .args(["--dry-run", "eject"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Dry run"));

    assert!(
        env.target_dir("test-target").join("my-skill").is_symlink(),
        "symlink should still exist after dry-run eject"
    );
}

#[test]
fn eject_nothing_to_eject() {
    let env = TestEnvBuilder::new()
        .source("local", "directory")
        .target("test-target")
        .skill("my-skill", "local")
        .build();

    // Don't sync — target is empty
    env.cmd()
        .arg("eject")
        .assert()
        .success()
        .stdout(predicate::str::contains("Nothing to eject"));
}
