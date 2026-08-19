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
