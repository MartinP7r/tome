use predicates::prelude::*;

mod common;
use common::*;

#[test]
fn eject_leaves_copy_deployments_for_future_remove_slice() {
    let env = TestEnvBuilder::new()
        .source("local", "directory")
        .target("test-target")
        .skill("my-skill", "local")
        .build();

    // First sync to distribute a create-only copy.
    env.cmd().arg("sync").assert().success();
    assert!(
        env.target_dir("test-target").join("my-skill").is_dir(),
        "skill should be copied after sync"
    );

    // Eject remains scoped to legacy symlinks in this slice. Copy removal is
    // explicitly out of scope for MCO-152.
    env.cmd()
        .arg("eject")
        .assert()
        .success()
        .stdout(predicate::str::contains("Nothing to eject"));

    assert!(
        env.target_dir("test-target").join("my-skill").is_dir(),
        "copy deployment should be preserved after eject"
    );
    assert!(
        env.library_dir().join("my-skill").is_dir(),
        "library should remain intact after eject"
    );
}

#[test]
fn eject_dry_run_leaves_copy_deployment() {
    let env = TestEnvBuilder::new()
        .source("local", "directory")
        .target("test-target")
        .skill("my-skill", "local")
        .build();

    // First sync to distribute a create-only copy.
    env.cmd().arg("sync").assert().success();
    assert!(env.target_dir("test-target").join("my-skill").is_dir());

    // Eject with dry-run still has no symlink work to perform.
    env.cmd()
        .args(["--dry-run", "eject"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Nothing to eject"));

    assert!(
        env.target_dir("test-target").join("my-skill").is_dir(),
        "copy deployment should still exist after dry-run eject"
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
