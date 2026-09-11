use assert_cmd::cargo::cargo_bin_cmd;
use assert_fs::TempDir;
use predicates::prelude::PredicateBooleanExt;

fn fixture() -> (TempDir, std::path::PathBuf, std::path::PathBuf) {
    let tmp = TempDir::new().unwrap();
    let config = tmp.path().join("tome.toml");
    let settings = tmp.path().join("settings.toml");
    std::fs::create_dir_all(tmp.path().join("machines")).unwrap();
    std::fs::write(
        &config,
        format!(
            "library_dir = \"{}\"\n",
            tmp.path().join("library").display()
        ),
    )
    .unwrap();
    std::fs::write(tmp.path().join("machines/work.toml"), "").unwrap();
    (tmp, config, settings)
}

#[test]
fn profile_selection() {
    let (_tmp, config, settings) = fixture();
    cargo_bin_cmd!("tome")
        .args([
            "--config",
            config.to_str().unwrap(),
            "--settings",
            settings.to_str().unwrap(),
            "profile",
            "select",
            "work",
        ])
        .assert()
        .success();
    cargo_bin_cmd!("tome")
        .args([
            "--config",
            config.to_str().unwrap(),
            "--settings",
            settings.to_str().unwrap(),
            "status",
        ])
        .assert()
        .success();
}

#[test]
fn sync_uses_the_selected_profile_routing_policy() {
    let (tmp, config, settings) = fixture();
    let source = tmp.path().join("source");
    let target = tmp.path().join("target");
    std::fs::create_dir_all(source.join("untagged")).unwrap();
    std::fs::write(source.join("untagged/SKILL.md"), "# untagged").unwrap();
    std::fs::write(
        tmp.path().join("machines/work.toml"),
        format!(
            "[directories.source]\npath = \"{}\"\nrole = \"source\"\n\n[directories.target]\npath = \"{}\"\nrole = \"target\"\n\n[routes.target]\ntags = [\"reference\"]\n",
            source.display(),
            target.display(),
        ),
    )
    .unwrap();
    std::fs::write(&settings, "profile = \"work\"\n").unwrap();

    cargo_bin_cmd!("tome")
        .args([
            "--config",
            config.to_str().unwrap(),
            "--settings",
            settings.to_str().unwrap(),
            "sync",
            "--no-input",
            "--no-install",
        ])
        .assert()
        .success();

    assert!(!target.join("untagged").exists());
}

#[test]
fn status_reports_selected_profile_and_project_destinations() {
    let (tmp, config, settings) = fixture();
    let profile_target = tmp.path().join("profile-target");
    let project_target = tmp.path().join("project-target");
    std::fs::write(
        tmp.path().join("machines/work.toml"),
        format!(
            "[directories.profile-target]\npath = \"{}\"\nrole = \"target\"\n",
            profile_target.display()
        ),
    )
    .unwrap();
    std::fs::write(&settings, "profile = \"work\"\n").unwrap();
    std::fs::write(
        tmp.path().join(".tome.toml"),
        format!(
            "[directories.project-target]\npath = \"{}\"\nrole = \"target\"\n",
            project_target.display()
        ),
    )
    .unwrap();

    let output = cargo_bin_cmd!("tome")
        .current_dir(tmp.path())
        .args([
            "--config",
            config.to_str().unwrap(),
            "--settings",
            settings.to_str().unwrap(),
            "status",
            "--json",
        ])
        .output()
        .unwrap();
    assert!(output.status.success(), "{output:?}");

    let report: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(report["profile"]["name"], "work");
    let names = report["directories"]
        .as_array()
        .unwrap()
        .iter()
        .map(|directory| directory["name"].as_str().unwrap())
        .collect::<Vec<_>>();
    assert!(names.contains(&"profile-target"));
    assert!(names.contains(&"project-target"));
}

#[test]
fn invalid_project_config_fails_status_without_profile_fallback() {
    let (tmp, config, settings) = fixture();
    std::fs::write(&settings, "profile = \"work\"\n").unwrap();
    std::fs::write(
        tmp.path().join(".tome.toml"),
        "[routes.missing]\ntags = [\"reference\"]\n",
    )
    .unwrap();

    cargo_bin_cmd!("tome")
        .current_dir(tmp.path())
        .args([
            "--config",
            config.to_str().unwrap(),
            "--settings",
            settings.to_str().unwrap(),
            "status",
        ])
        .assert()
        .failure()
        .stderr(predicates::str::contains("missing"));
}

#[test]
fn profile_create_list_and_select_are_validated() {
    let (_tmp, config, settings) = fixture();

    cargo_bin_cmd!("tome")
        .args([
            "--config",
            config.to_str().unwrap(),
            "--settings",
            settings.to_str().unwrap(),
            "profile",
            "create",
            "personal",
        ])
        .assert()
        .success();
    cargo_bin_cmd!("tome")
        .args([
            "--config",
            config.to_str().unwrap(),
            "--settings",
            settings.to_str().unwrap(),
            "profile",
            "list",
        ])
        .assert()
        .success()
        .stdout(predicates::str::contains("personal"));
    cargo_bin_cmd!("tome")
        .args([
            "--config",
            config.to_str().unwrap(),
            "--settings",
            settings.to_str().unwrap(),
            "profile",
            "create",
            "personal",
        ])
        .assert()
        .failure()
        .stderr(predicates::str::contains("already exists"));
    cargo_bin_cmd!("tome")
        .args([
            "--config",
            config.to_str().unwrap(),
            "--settings",
            settings.to_str().unwrap(),
            "profile",
            "create",
            "../unsafe",
        ])
        .assert()
        .failure()
        .stderr(predicates::str::contains("directory name"));
}

#[test]
fn released_cli_rejects_machine_flag() {
    let (tmp, config, settings) = fixture();
    std::fs::write(&settings, "profile = \"work\"\n").unwrap();
    let machine = tmp.path().join("machine.toml");
    std::fs::write(&machine, "disabled = [\"anything\"]\n").unwrap();

    cargo_bin_cmd!("tome")
        .args([
            "--config",
            config.to_str().unwrap(),
            "--settings",
            settings.to_str().unwrap(),
            "--machine",
            machine.to_str().unwrap(),
            "status",
        ])
        .assert()
        .failure()
        .stderr(predicates::str::contains("unexpected argument '--machine'"));
}

#[test]
fn status_requires_explicit_profile_selection() {
    let (_tmp, config, settings) = fixture();
    cargo_bin_cmd!("tome")
        .args([
            "--config",
            config.to_str().unwrap(),
            "--settings",
            settings.to_str().unwrap(),
            "status",
        ])
        .assert()
        .failure()
        .stderr(predicates::str::contains("profile select"));
}

#[test]
fn empty_profile_bypass_environment_variable_has_no_effect() {
    let (_tmp, config, settings) = fixture();
    std::fs::write(&settings, "profile = \"missing\"\n").unwrap();

    cargo_bin_cmd!("tome")
        .env("TOME_TEST_ALLOW_EMPTY_PROFILE", "1")
        .args([
            "--config",
            config.to_str().unwrap(),
            "--settings",
            settings.to_str().unwrap(),
            "status",
        ])
        .assert()
        .failure()
        .stderr(predicates::str::contains("does not exist"));
}

#[test]
fn legacy_layout_names_required_layered_files_without_migration_guidance() {
    let (tmp, config, settings) = fixture();
    let machine = tmp.path().join("machine.toml");
    std::fs::write(&config, format!("library_dir = \"{}\"\n\n[directories.source]\npath = \"{}\"\ntype = \"directory\"\nrole = \"source\"\n", tmp.path().join("library").display(), tmp.path().join("source").display())).unwrap();
    std::fs::write(&machine, "").unwrap();
    cargo_bin_cmd!("tome")
        .args([
            "--config",
            config.to_str().unwrap(),
            "--settings",
            settings.to_str().unwrap(),
            "status",
        ])
        .assert()
        .failure()
        .stderr(predicates::str::contains("machines/<profile>.toml"))
        .stderr(predicates::str::contains("settings.toml"))
        .stderr(predicates::str::contains("tome migrate profiles").not());
}

#[test]
fn pool_git_source_is_not_rejected_as_a_legacy_layout() {
    let (tmp, config, settings) = fixture();
    std::fs::write(
        &config,
        format!(
            "library_dir = \"{}\"\n\n[directories.pool]\npath = \"https://example.test/skills.git\"\ntype = \"git\"\nrole = \"source\"\n",
            tmp.path().join("library").display(),
        ),
    )
    .unwrap();
    std::fs::write(&settings, "profile = \"work\"\n").unwrap();

    cargo_bin_cmd!("tome")
        .args([
            "--config",
            config.to_str().unwrap(),
            "--settings",
            settings.to_str().unwrap(),
            "status",
        ])
        .assert()
        .success();
}
