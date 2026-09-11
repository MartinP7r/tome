use assert_cmd::Command;
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

fn shell_quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', "'\\''"))
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
fn legacy_layout_is_refused_with_migration_guidance() {
    let (tmp, config, settings) = fixture();
    let machine = tmp.path().join("machine.toml");
    std::fs::write(&config, format!("library_dir = \"{}\"\n\n[directories.source]\npath = \"{}\"\ntype = \"directory\"\nrole = \"source\"\n", tmp.path().join("library").display(), tmp.path().join("source").display())).unwrap();
    std::fs::write(&machine, "").unwrap();
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
        .failure()
        .stderr(predicates::str::contains("tome migrate profiles"));
}

#[test]
fn migrate_profiles_happy_path() {
    let (tmp, config, settings) = fixture();
    std::fs::remove_file(tmp.path().join("machines/work.toml")).unwrap();
    let legacy_machine = tmp.path().join(".config/tome/machine.toml");
    std::fs::create_dir_all(legacy_machine.parent().unwrap()).unwrap();
    std::fs::write(
        &config,
        format!(
            "library_dir = \"{}\"\n\n[directories.source]\npath = \"{}\"\ntype = \"directory\"\nrole = \"source\"\n",
            tmp.path().join("library").display(),
            tmp.path().join("source").display()
        ),
    )
    .unwrap();
    std::fs::write(&legacy_machine, "disabled = []\n").unwrap();

    let mut command = Command::new("script");
    if cfg!(target_os = "macos") {
        command.args([
            "-q",
            "/dev/null",
            cargo_bin_cmd!("tome").get_program().to_str().unwrap(),
            "--config",
            config.to_str().unwrap(),
            "--settings",
            settings.to_str().unwrap(),
            "migrate",
            "profiles",
        ]);
    } else {
        let invocation = format!(
            "{} --config {} --settings {} migrate profiles",
            shell_quote(cargo_bin_cmd!("tome").get_program().to_str().unwrap()),
            shell_quote(config.to_str().unwrap()),
            shell_quote(settings.to_str().unwrap()),
        );
        command.args(["-q", "-e", "-c", &invocation, "/dev/null"]);
    }

    command
        .env("HOME", tmp.path())
        .write_stdin("work\ny\n")
        .assert()
        .success();

    let migrated_pool = std::fs::read_to_string(&config).unwrap();
    assert!(migrated_pool.contains("library_dir"));
    assert!(!migrated_pool.contains("directories"));
    assert!(tmp.path().join("machines/work.toml").is_file());
    assert!(settings.is_file());
    assert!(!legacy_machine.exists());
}
