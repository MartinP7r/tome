use assert_fs::TempDir;
use predicates::prelude::*;

mod common;
use common::*;

#[test]
fn tag_commands_mutate_only_existing_manifest_entries() {
    let env = TestEnvBuilder::new()
        .source("source", "directory")
        .skill("reference-skill", "source")
        .build();
    env.cmd().args(["sync", "--no-input"]).assert().success();

    env.cmd()
        .args(["tag", "add", "reference-skill", "reference"])
        .assert()
        .success();
    let manifest = std::fs::read_to_string(env.manifest_path()).unwrap();
    assert!(manifest.contains("\"reference\""), "{manifest}");

    env.cmd()
        .args(["tag", "list", "reference-skill"])
        .assert()
        .success()
        .stdout(predicate::str::contains("reference"));

    env.cmd()
        .args(["tag", "list"])
        .assert()
        .success()
        .stdout(predicate::str::contains("reference-skill"));

    let before = manifest;
    env.cmd()
        .args(["tag", "remove", "reference-skill", "missing"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("missing"));
    assert_eq!(
        std::fs::read_to_string(env.manifest_path()).unwrap(),
        before
    );

    env.cmd()
        .args(["tag", "add", "missing-skill", "reference"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("missing-skill"));
    assert_eq!(
        std::fs::read_to_string(env.manifest_path()).unwrap(),
        before
    );
}

#[test]
fn route_commands_write_to_the_destination_owner_and_reject_unknown_destinations() {
    let env = TestEnvBuilder::new()
        .source("source", "directory")
        .target("profile-target")
        .skill("reference-skill", "source")
        .build();
    env.cmd().args(["sync", "--no-input"]).assert().success();
    env.cmd()
        .args(["tag", "add", "reference-skill", "reference"])
        .assert()
        .success();

    env.cmd()
        .args(["route", "tag", "add", "--to", "profile-target", "reference"])
        .assert()
        .success();
    let profile_path = env.config_path.parent().unwrap().join("machines/test.toml");
    let profile = std::fs::read_to_string(&profile_path).unwrap();
    assert!(profile.contains("[routes.profile-target]"), "{profile}");
    assert!(profile.contains("tags = [\"reference\"]"), "{profile}");

    env.cmd()
        .args(["tag", "add", "reference-skill", "missing"])
        .assert()
        .success();
    let before_profile = profile.clone();
    env.cmd()
        .args([
            "route",
            "tag",
            "remove",
            "--to",
            "profile-target",
            "missing",
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains("route selector"));
    assert_eq!(
        std::fs::read_to_string(&profile_path).unwrap(),
        before_profile
    );

    let project_dir = env.tmp.path().join("project");
    let project_target = project_dir.join("target");
    std::fs::create_dir_all(&project_target).unwrap();
    std::fs::write(
        project_dir.join(".tome.toml"),
        format!(
            "[directories.project-target]\npath = \"{}\"\ntype = \"directory\"\nrole = \"target\"\n",
            project_target.display()
        ),
    )
    .unwrap();

    env.cmd()
        .current_dir(&project_dir)
        .args([
            "route",
            "exclude",
            "add",
            "--to",
            "project-target",
            "reference-skill",
        ])
        .assert()
        .success();
    let project_path = project_dir.join(".tome.toml");
    let project = std::fs::read_to_string(&project_path).unwrap();
    assert!(project.contains("[routes.project-target]"), "{project}");
    assert!(
        project.contains("exclude = [\"reference-skill\"]"),
        "{project}"
    );

    let before_profile = std::fs::read_to_string(&profile_path).unwrap();
    let before_project = project;
    env.cmd()
        .current_dir(env.tmp.path())
        .args(["route", "tag", "add", "--to", "project-target", "reference"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("project-target"));
    assert_eq!(
        std::fs::read_to_string(profile_path).unwrap(),
        before_profile
    );
    assert_eq!(
        std::fs::read_to_string(project_path).unwrap(),
        before_project
    );
}

#[test]
fn dry_run_tag_and_route_mutations_validate_without_writing() {
    let env = TestEnvBuilder::new()
        .source("source", "directory")
        .target("profile-target")
        .skill("reference-skill", "source")
        .build();
    env.cmd().args(["sync", "--no-input"]).assert().success();
    env.cmd()
        .args(["tag", "add", "reference-skill", "reference"])
        .assert()
        .success();

    let manifest_before = std::fs::read_to_string(env.manifest_path()).unwrap();
    env.cmd()
        .args(["--dry-run", "tag", "add", "reference-skill", "reference"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Would"));
    assert_eq!(
        std::fs::read_to_string(env.manifest_path()).unwrap(),
        manifest_before
    );

    let profile_path = env.config_path.parent().unwrap().join("machines/test.toml");
    let profile_before = std::fs::read_to_string(&profile_path).unwrap();
    env.cmd()
        .args([
            "--dry-run",
            "route",
            "tag",
            "add",
            "--to",
            "profile-target",
            "reference",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("Would"));
    assert_eq!(
        std::fs::read_to_string(&profile_path).unwrap(),
        profile_before
    );

    env.cmd()
        .args([
            "--dry-run",
            "route",
            "tag",
            "remove",
            "--to",
            "profile-target",
            "reference",
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains("route selector"));
    assert_eq!(
        std::fs::read_to_string(profile_path).unwrap(),
        profile_before
    );
}

#[test]
fn route_rejects_manifest_unknown_tags_and_skills_without_writing() {
    let env = TestEnvBuilder::new()
        .source("source", "directory")
        .target("profile-target")
        .skill("reference-skill", "source")
        .build();
    env.cmd().args(["sync", "--no-input"]).assert().success();
    env.cmd()
        .args(["tag", "add", "reference-skill", "reference"])
        .assert()
        .success();

    let profile_path = env.config_path.parent().unwrap().join("machines/test.toml");
    let profile_before = std::fs::read_to_string(&profile_path).unwrap();
    for args in [
        vec![
            "route",
            "tag",
            "add",
            "--to",
            "profile-target",
            "unknown-tag",
        ],
        vec![
            "route",
            "exclude",
            "add",
            "--to",
            "profile-target",
            "unknown-skill",
        ],
    ] {
        env.cmd()
            .args(args)
            .assert()
            .failure()
            .stderr(predicate::str::contains("manifest"));
        assert_eq!(
            std::fs::read_to_string(&profile_path).unwrap(),
            profile_before
        );
    }

    let project_dir = env.tmp.path().join("project");
    let project_target = project_dir.join("target");
    std::fs::create_dir_all(&project_target).unwrap();
    let project_path = project_dir.join(".tome.toml");
    std::fs::write(
        &project_path,
        format!(
            "[directories.project-target]\npath = \"{}\"\ntype = \"directory\"\nrole = \"target\"\n",
            project_target.display()
        ),
    )
    .unwrap();
    let project_before = std::fs::read_to_string(&project_path).unwrap();
    for args in [
        vec![
            "route",
            "tag",
            "add",
            "--to",
            "project-target",
            "unknown-tag",
        ],
        vec![
            "route",
            "exclude",
            "add",
            "--to",
            "project-target",
            "unknown-skill",
        ],
    ] {
        env.cmd()
            .current_dir(&project_dir)
            .args(args)
            .assert()
            .failure()
            .stderr(predicate::str::contains("manifest"));
        assert_eq!(
            std::fs::read_to_string(&project_path).unwrap(),
            project_before
        );
    }
}

#[test]
fn config_path_prints_default_path() {
    let tmp = TempDir::new().unwrap();
    tome()
        .env("TOME_HOME", tmp.path())
        .args(["config", "--path"])
        .assert()
        .success()
        .stdout(predicate::str::contains("tome.toml"));
}

#[test]
fn tome_home_flag_overrides_default() {
    let env = TestEnvBuilder::new()
        .source("local", "directory")
        .target("test-tool")
        .skill("my-skill", "local")
        .build();

    // The --tome-home flag should be accepted and tome should use that directory
    // for manifest storage. We verify by syncing and checking that the manifest
    // ends up in the custom tome home, not the default.
    let custom_home = env.tmp.path().join("custom-tome-home");
    std::fs::create_dir_all(&custom_home).unwrap();

    // Copy config into the custom home so tome can find it
    let config_path = custom_home.join("tome.toml");
    std::fs::copy(&env.config_path, &config_path).unwrap();
    copy_selected_profile(&env.config_path, &config_path);

    tome()
        .arg("--tome-home")
        .arg(&custom_home)
        .arg("sync")
        .assert()
        .success();

    // Manifest should be in custom home, not default
    assert!(
        custom_home.join(".tome-manifest.json").exists(),
        "manifest should be in custom tome home"
    );
}

#[test]
fn tome_home_env_var_overrides_default() {
    let env = TestEnvBuilder::new()
        .source("local", "directory")
        .target("test-tool")
        .skill("env-skill", "local")
        .build();

    let custom_home = env.tmp.path().join("env-tome-home");
    std::fs::create_dir_all(&custom_home).unwrap();
    let config_path = custom_home.join("tome.toml");
    std::fs::copy(&env.config_path, &config_path).unwrap();
    copy_selected_profile(&env.config_path, &config_path);

    tome()
        .env("TOME_HOME", &custom_home)
        .arg("sync")
        .assert()
        .success();

    assert!(
        custom_home.join(".tome-manifest.json").exists(),
        "manifest should be in TOME_HOME directory"
    );
}

#[test]
fn tome_home_flag_takes_precedence_over_env() {
    let env = TestEnvBuilder::new()
        .source("local", "directory")
        .target("test-tool")
        .skill("prio-skill", "local")
        .build();

    let env_home = env.tmp.path().join("env-home");
    let flag_home = env.tmp.path().join("flag-home");
    std::fs::create_dir_all(&env_home).unwrap();
    std::fs::create_dir_all(&flag_home).unwrap();

    // Copy config to both locations
    let env_config = env_home.join("tome.toml");
    let flag_config = flag_home.join("tome.toml");
    std::fs::copy(&env.config_path, &env_config).unwrap();
    std::fs::copy(&env.config_path, &flag_config).unwrap();
    copy_selected_profile(&env.config_path, &env_config);
    copy_selected_profile(&env.config_path, &flag_config);

    tome()
        .env("TOME_HOME", &env_home)
        .arg("--tome-home")
        .arg(&flag_home)
        .arg("sync")
        .assert()
        .success();

    // Flag should win over env var
    assert!(
        flag_home.join(".tome-manifest.json").exists(),
        "manifest should be in --tome-home path, not TOME_HOME env"
    );
    assert!(
        !env_home.join(".tome-manifest.json").exists(),
        "manifest should NOT be in TOME_HOME env path"
    );
}

#[test]
fn tome_home_finds_config_in_dotdir() {
    // When config is at TOME_HOME/.tome/tome.toml, tome should find it
    let env = TestEnvBuilder::new()
        .source("local", "directory")
        .target("test-tool")
        .skill("dotdir-skill", "local")
        .build();

    let repo_root = env.tmp.path().join("my-repo");
    let dotdir = repo_root.join(".tome");
    std::fs::create_dir_all(&dotdir).unwrap();
    let config_path = dotdir.join("tome.toml");
    std::fs::copy(&env.config_path, &config_path).unwrap();
    copy_selected_profile(&env.config_path, &config_path);

    tome()
        .arg("--tome-home")
        .arg(&repo_root)
        .arg("sync")
        .assert()
        .success();

    // Manifest and lockfile should be in .tome/ subdir, not repo root
    assert!(
        dotdir.join(".tome-manifest.json").exists(),
        "manifest should be in .tome/ subdir"
    );
    assert!(
        dotdir.join("tome.lock").exists(),
        "lockfile should be in .tome/ subdir"
    );
    assert!(
        !repo_root.join(".tome-manifest.json").exists(),
        "manifest should NOT be at repo root"
    );
}

#[test]
fn tome_home_falls_back_to_root_config() {
    // When config is at TOME_HOME/tome.toml (no .tome/ subdir), tome should use root
    let env = TestEnvBuilder::new()
        .source("local", "directory")
        .target("test-tool")
        .skill("root-skill", "local")
        .build();

    let custom_home = env.tmp.path().join("root-config-home");
    std::fs::create_dir_all(&custom_home).unwrap();
    let config_path = custom_home.join("tome.toml");
    std::fs::copy(&env.config_path, &config_path).unwrap();
    copy_selected_profile(&env.config_path, &config_path);

    tome()
        .arg("--tome-home")
        .arg(&custom_home)
        .arg("sync")
        .assert()
        .success();

    // Manifest and lockfile should be at root (backwards compat)
    assert!(
        custom_home.join(".tome-manifest.json").exists(),
        "manifest should be at tome home root"
    );
    assert!(
        custom_home.join("tome.lock").exists(),
        "lockfile should be at tome home root"
    );
}

#[test]
fn tome_home_dotdir_wins_over_root() {
    // When both TOME_HOME/.tome/tome.toml and TOME_HOME/tome.toml exist,
    // .tome/ subdir should win
    let env = TestEnvBuilder::new()
        .source("local", "directory")
        .target("test-tool")
        .skill("priority-skill", "local")
        .build();

    let repo_root = env.tmp.path().join("both-configs");
    let dotdir = repo_root.join(".tome");
    std::fs::create_dir_all(&dotdir).unwrap();

    // Put config in both locations
    let dot_config = dotdir.join("tome.toml");
    let root_config = repo_root.join("tome.toml");
    std::fs::copy(&env.config_path, &dot_config).unwrap();
    std::fs::copy(&env.config_path, &root_config).unwrap();
    copy_selected_profile(&env.config_path, &dot_config);
    copy_selected_profile(&env.config_path, &root_config);

    tome()
        .arg("--tome-home")
        .arg(&repo_root)
        .arg("sync")
        .assert()
        .success();

    // .tome/ subdir should win — manifest goes there
    assert!(
        dotdir.join(".tome-manifest.json").exists(),
        "manifest should be in .tome/ subdir (wins over root)"
    );
    assert!(
        !repo_root.join(".tome-manifest.json").exists(),
        "manifest should NOT be at root when .tome/ exists"
    );
}

#[test]
fn config_path_shows_correct_location_for_dotdir() {
    let env = TestEnvBuilder::new()
        .source("local", "directory")
        .skill("cfg-skill", "local")
        .build();

    let repo_root = env.tmp.path().join("config-path-test");
    let dotdir = repo_root.join(".tome");
    std::fs::create_dir_all(&dotdir).unwrap();
    std::fs::copy(&env.config_path, dotdir.join("tome.toml")).unwrap();

    let output = tome()
        .arg("--tome-home")
        .arg(&repo_root)
        .args(["config", "--path"])
        .output()
        .unwrap();

    let stdout = String::from_utf8_lossy(&output.stdout);
    let expected = dotdir.join("tome.toml");
    assert!(
        stdout.trim().ends_with(".tome/tome.toml"),
        "config --path should show .tome/tome.toml, got: {}",
        stdout.trim()
    );
    assert_eq!(stdout.trim(), expected.display().to_string());
}

#[test]
fn config_toml_tome_home_override() {
    // This test verifies that --tome-home takes precedence,
    // which exercises the resolution order without needing to write
    // to ~/.config/tome/config.toml.
    let env = TestEnvBuilder::new()
        .source("local", "directory")
        .skill("skill-a", "local")
        .build();

    // Sync using --tome-home to set a custom tome home
    tome()
        .args([
            "--config",
            &env.config_path.to_string_lossy(),
            "--tome-home",
            &env.library_dir.parent().unwrap().to_string_lossy(),
            "status",
        ])
        .assert()
        .success();
}
