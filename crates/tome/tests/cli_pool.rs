mod common;

use assert_cmd::{Command, cargo_bin_cmd};
use predicates::prelude::*;
use tempfile::TempDir;

fn git(dir: &std::path::Path, args: &[&str]) {
    let output = std::process::Command::new("git")
        .args(args)
        .current_dir(dir)
        .env_remove("GIT_DIR")
        .env_remove("GIT_WORK_TREE")
        .env_remove("GIT_INDEX_FILE")
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "git {} failed: {}",
        args.join(" "),
        String::from_utf8_lossy(&output.stderr)
    );
}

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
    std::fs::write(
        &config,
        format!("library_dir = \"{}\"\n", library.display()),
    )
    .unwrap();
    std::fs::create_dir_all(root.join("machines")).unwrap();
    std::fs::write(
        root.join("machines/profile-a.toml"),
        format!(
            "[directories.source]\npath = \"{}\"\ntype = \"directory\"\nrole = \"source\"\n",
            source.display()
        ),
    )
    .unwrap();
    std::fs::write(
        root.join("machines/profile-b.toml"),
        format!(
            "[directories.target]\npath = \"{}\"\ntype = \"directory\"\nrole = \"target\"\n",
            target.display()
        ),
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
    std::fs::write(
        &config,
        format!("library_dir = \"{}\"\n", library.display()),
    )
    .unwrap();
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

#[test]
fn pool_remove_excludes_before_cleanup_and_restore_allows_import() {
    let tmp = TempDir::new().unwrap();
    let root = tmp.path();
    let library = root.join("library");
    let source = root.join("source");
    std::fs::create_dir_all(&library).unwrap();
    common::create_skill(&source, "removed-skill");
    let config = root.join("tome.toml");
    std::fs::write(
        &config,
        format!("library_dir = \"{}\"\n", library.display()),
    )
    .unwrap();
    std::fs::create_dir_all(root.join("machines")).unwrap();
    std::fs::write(
        root.join("machines/test.toml"),
        format!(
            "[directories.source]\npath = \"{}\"\ntype = \"directory\"\nrole = \"source\"\n",
            source.display()
        ),
    )
    .unwrap();
    let settings = root.join("settings.toml");
    std::fs::write(&settings, "profile = \"test\"\ngit_sync = \"never\"\n").unwrap();
    run(&config, &settings).assert().success();

    let mut remove = cargo_bin_cmd!("tome");
    remove.args([
        "--config",
        config.to_str().unwrap(),
        "--settings",
        settings.to_str().unwrap(),
        "--no-input",
        "remove",
        "pool",
        "removed-skill",
        "--yes",
    ]);
    remove.assert().success();
    assert!(!library.join("removed-skill").exists());
    assert!(
        std::fs::read_to_string(&config)
            .unwrap()
            .contains("removed-skill")
    );

    run(&config, &settings).assert().success();
    assert!(!library.join("removed-skill").exists());

    let mut restore = cargo_bin_cmd!("tome");
    restore.args([
        "--config",
        config.to_str().unwrap(),
        "--settings",
        settings.to_str().unwrap(),
        "--no-input",
        "pool",
        "restore",
        "removed-skill",
    ]);
    restore.assert().success();
    run(&config, &settings).assert().success();
    assert!(library.join("removed-skill").exists());
}

#[test]
fn git_policy_matrix_uses_pulled_profile_in_the_same_sync() {
    let tmp = TempDir::new().unwrap();
    let root = tmp.path();
    let remote = root.join("remote.git");
    let seed = root.join("seed");
    let library = root.join("library");
    let source_a = root.join("source-a");
    let source_b = root.join("source-b");
    std::fs::create_dir_all(&library).unwrap();
    common::create_skill(&source_a, "from-a");
    common::create_skill(&source_b, "from-b");
    git(root, &["init", "--bare", remote.to_str().unwrap()]);
    git(
        root,
        &["clone", remote.to_str().unwrap(), seed.to_str().unwrap()],
    );
    git(&seed, &["config", "user.email", "test@example.com"]);
    git(&seed, &["config", "user.name", "Test User"]);
    std::fs::create_dir_all(seed.join("machines")).unwrap();
    std::fs::write(
        seed.join("tome.toml"),
        format!("library_dir = \"{}\"\n", library.display()),
    )
    .unwrap();
    std::fs::write(
        seed.join("machines/test.toml"),
        format!(
            "[directories.source]\npath = \"{}\"\ntype = \"directory\"\nrole = \"source\"\n",
            source_a.display()
        ),
    )
    .unwrap();
    git(&seed, &["add", "tome.toml", "machines/test.toml"]);
    git(&seed, &["commit", "-m", "initial pool profile"]);
    git(&seed, &["push", "-u", "origin", "HEAD"]);

    let local = root.join("local");
    let writer = root.join("writer");
    git(
        root,
        &["clone", remote.to_str().unwrap(), local.to_str().unwrap()],
    );
    git(
        root,
        &["clone", remote.to_str().unwrap(), writer.to_str().unwrap()],
    );
    git(&writer, &["config", "user.email", "test@example.com"]);
    git(&writer, &["config", "user.name", "Test User"]);
    std::fs::write(
        writer.join("machines/test.toml"),
        format!(
            "[directories.source]\npath = \"{}\"\ntype = \"directory\"\nrole = \"source\"\n",
            source_b.display()
        ),
    )
    .unwrap();
    git(&writer, &["add", "machines/test.toml"]);
    git(
        &writer,
        &["commit", "-m", "switch selected profile topology"],
    );
    git(&writer, &["push"]);

    let settings = root.join("settings.toml");
    std::fs::write(&settings, "profile = \"test\"\ngit_sync = \"never\"\n").unwrap();
    let mut command = cargo_bin_cmd!("tome");
    command.args([
        "--config",
        local.join("tome.toml").to_str().unwrap(),
        "--settings",
        settings.to_str().unwrap(),
        "--tome-home",
        local.to_str().unwrap(),
        "--no-input",
        "sync",
        "--git-sync",
        "always",
    ]);
    command.assert().success();
    assert!(library.join("from-b/SKILL.md").is_file());
    assert!(!library.join("from-a").exists());
}

#[test]
fn git_owned_path_staging_commits_only_pool_content() {
    let tmp = TempDir::new().unwrap();
    let root = tmp.path();
    let remote = root.join("remote.git");
    let local = root.join("local");
    let source = root.join("source");
    common::create_skill(&source, "managed-skill");
    git(root, &["init", "--bare", remote.to_str().unwrap()]);
    git(
        root,
        &["clone", remote.to_str().unwrap(), local.to_str().unwrap()],
    );
    git(&local, &["config", "user.email", "test@example.com"]);
    git(&local, &["config", "user.name", "Test User"]);
    std::fs::create_dir_all(local.join("machines")).unwrap();
    std::fs::write(
        local.join("tome.toml"),
        format!("library_dir = \"{}\"\n", local.join("skills").display()),
    )
    .unwrap();
    std::fs::write(
        local.join("machines/test.toml"),
        format!(
            "[directories.source]\npath = \"{}\"\ntype = \"directory\"\nrole = \"source\"\n",
            source.display()
        ),
    )
    .unwrap();
    std::fs::write(local.join("agent-notes"), "do not commit me").unwrap();
    std::fs::create_dir_all(local.join("repos/cache")).unwrap();
    std::fs::write(local.join("repos/cache/untracked"), "cache stays local").unwrap();
    git(&local, &["add", "tome.toml", "machines/test.toml"]);
    git(&local, &["commit", "-m", "initial pool config"]);
    git(&local, &["push", "-u", "origin", "HEAD"]);

    let settings = root.join("settings.toml");
    std::fs::write(&settings, "profile = \"test\"\ngit_sync = \"always\"\n").unwrap();
    let mut command = cargo_bin_cmd!("tome");
    command.args([
        "--config",
        local.join("tome.toml").to_str().unwrap(),
        "--settings",
        settings.to_str().unwrap(),
        "--tome-home",
        local.to_str().unwrap(),
        "--no-input",
        "sync",
    ]);
    command.assert().success();

    let changed = std::process::Command::new("git")
        .args(["show", "--format=", "--name-only", "HEAD"])
        .current_dir(&local)
        .output()
        .unwrap();
    let changed = String::from_utf8(changed.stdout).unwrap();
    assert!(changed.lines().any(|path| path == "tome.lock"));
    assert!(changed.lines().all(|path| {
        path == "tome.lock"
            || path == "tome.toml"
            || path == "machines/test.toml"
            || path.starts_with("skills/")
    }));
    assert_eq!(
        std::fs::read_to_string(local.join("agent-notes")).unwrap(),
        "do not commit me"
    );
    assert_eq!(
        std::fs::read_to_string(local.join("repos/cache/untracked")).unwrap(),
        "cache stays local"
    );

    std::fs::write(local.join("staged-unrelated"), "keep staged").unwrap();
    git(&local, &["add", "staged-unrelated"]);
    let mut blocked = cargo_bin_cmd!("tome");
    blocked.args([
        "--config",
        local.join("tome.toml").to_str().unwrap(),
        "--settings",
        settings.to_str().unwrap(),
        "--tome-home",
        local.to_str().unwrap(),
        "--no-input",
        "sync",
    ]);
    blocked.assert().success();
    let staged = std::process::Command::new("git")
        .args(["diff", "--cached", "--name-only"])
        .current_dir(&local)
        .output()
        .unwrap();
    assert_eq!(
        String::from_utf8(staged.stdout).unwrap(),
        "staged-unrelated\n"
    );
}
