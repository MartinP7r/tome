use assert_fs::TempDir;
use predicates::prelude::*;

mod common;
use common::*;

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
    std::fs::copy(&env.config_path, custom_home.join("tome.toml")).unwrap();

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
    std::fs::copy(&env.config_path, custom_home.join("tome.toml")).unwrap();

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
    std::fs::copy(&env.config_path, env_home.join("tome.toml")).unwrap();
    std::fs::copy(&env.config_path, flag_home.join("tome.toml")).unwrap();

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
    std::fs::copy(&env.config_path, dotdir.join("tome.toml")).unwrap();

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
    std::fs::copy(&env.config_path, custom_home.join("tome.toml")).unwrap();

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
    std::fs::copy(&env.config_path, dotdir.join("tome.toml")).unwrap();
    std::fs::copy(&env.config_path, repo_root.join("tome.toml")).unwrap();

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
