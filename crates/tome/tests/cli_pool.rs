mod common;

use assert_cmd::{Command, cargo_bin_cmd};
use predicates::prelude::*;
use tempfile::TempDir;

fn run(config: &std::path::Path, settings: &std::path::Path) -> Command {
    let mut command = cargo_bin_cmd!("tome");
    command.args([
        "--config",
        config.to_str().unwrap(),
        "--settings",
        settings.to_str().unwrap(),
        "--no-input",
        "sync",
    ]);
    command
}

#[test]
fn cross_profile_pool_preserves_and_distributes() {
    let tmp = TempDir::new().unwrap();
    let root = tmp.path();
    let library = root.join("library");
    let source = root.join("source");
    let target = root.join("target");
    std::fs::create_dir_all(&library).unwrap();
    std::fs::create_dir_all(&target).unwrap();
    common::create_skill(&source, "shared-skill");
    let config = root.join("tome.toml");
    std::fs::write(&config, format!("library_dir = \"{}\"\n", library.display())).unwrap();
    std::fs::create_dir_all(root.join("machines")).unwrap();
    std::fs::write(
        root.join("machines/profile-a.toml"),
        format!("[directories.source]\npath = \"{}\"\ntype = \"directory\"\nrole = \"source\"\n", source.display()),
    )
    .unwrap();
    std::fs::write(
        root.join("machines/profile-b.toml"),
        format!("[directories.target]\npath = \"{}\"\ntype = \"directory\"\nrole = \"target\"\n", target.display()),
    )
    .unwrap();
    let settings = root.join("settings.toml");
    std::fs::write(&settings, "profile = \"profile-a\"\ngit_sync = \"never\"\n").unwrap();

    run(&config, &settings).assert().success();
    assert!(library.join("shared-skill/SKILL.md").is_file());
    assert!(root.join("tome.lock").is_file());

    std::fs::write(&settings, "profile = \"profile-b\"\ngit_sync = \"never\"\n").unwrap();
    run(&config, &settings).assert().success();
    assert!(target.join("shared-skill").is_symlink());
}

#[test]
fn conflicting_candidates_do_not_mutate_pool() {
    let tmp = TempDir::new().unwrap();
    let root = tmp.path();
    let library = root.join("library");
    let left = root.join("left");
    let right = root.join("right");
    std::fs::create_dir_all(&library).unwrap();
    common::create_skill(&left, "same");
    common::create_skill(&right, "same");
    std::fs::write(right.join("same/SKILL.md"), "different").unwrap();
    let config = root.join("tome.toml");
    std::fs::write(&config, format!("library_dir = \"{}\"\n", library.display())).unwrap();
    std::fs::create_dir_all(root.join("machines")).unwrap();
    std::fs::write(
        root.join("machines/test.toml"),
        format!("[directories.left]\npath = \"{}\"\ntype = \"directory\"\nrole = \"source\"\n\n[directories.right]\npath = \"{}\"\ntype = \"directory\"\nrole = \"source\"\n", left.display(), right.display()),
    ).unwrap();
    let settings = root.join("settings.toml");
    std::fs::write(&settings, "profile = \"test\"\ngit_sync = \"never\"\n").unwrap();
    run(&config, &settings)
        .assert()
        .failure()
        .stderr(predicate::str::contains("pool reconciliation halted"));
    assert!(!library.join("same").exists());
    assert!(!root.join("tome.lock").exists());
}
