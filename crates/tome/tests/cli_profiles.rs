use assert_cmd::cargo::cargo_bin_cmd;
use assert_fs::TempDir;

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
fn legacy_directory_overrides_remain_compatible_with_a_warning() {
    let (tmp, config, settings) = fixture();
    let source = tmp.path().join("legacy-source");
    let target = tmp.path().join("legacy-target");
    let machine = tmp.path().join("machine.toml");
    std::fs::write(
        &config,
        format!(
            "library_dir = \"{}\"\n\n[directories.source]\npath = \"{}\"\ntype = \"directory\"\nrole = \"source\"\n",
            tmp.path().join("library").display(),
            source.display(),
        ),
    )
    .unwrap();
    std::fs::write(
        &machine,
        format!(
            "[directory_overrides.source]\npath = \"{}\"\n",
            target.display()
        ),
    )
    .unwrap();

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
        .success()
        .stderr(predicates::str::contains(
            "directory_overrides is deprecated",
        ));
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
