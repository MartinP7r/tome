use assert_fs::TempDir;
use predicates::prelude::*;

mod common;
use common::*;

#[test]
fn help_shows_usage() {
    tome()
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "Sync AI coding skills across tools",
        ));
}

#[test]
fn version_shows_version() {
    tome()
        .arg("--version")
        .assert()
        .success()
        .stdout(predicate::str::contains(env!("CARGO_PKG_VERSION")));
}

#[test]
fn version_subcommand_shows_version() {
    tome()
        .arg("version")
        .assert()
        .success()
        .stdout(predicate::str::contains(env!("CARGO_PKG_VERSION")));
}

#[test]
fn short_version_flag_shows_version() {
    tome()
        .arg("-V")
        .assert()
        .success()
        .stdout(predicate::str::contains(env!("CARGO_PKG_VERSION")));
}

#[test]
fn verbose_short_flag_still_works() {
    // Ensure -v is still --verbose and doesn't conflict with -V
    tome()
        .args(["-v", "status", "--config", "/nonexistent/path.toml"])
        .assert()
        // This may fail because no config, but should NOT be interpreted as version
        .stderr(predicate::str::contains("version").not());
}

#[test]
fn completions_fish_installs_to_file() {
    let home = TempDir::new().unwrap();
    let xdg_config = home.path().join(".config");
    tome()
        .env("HOME", home.path())
        .env("TOME_HOME", home.path().join(".tome"))
        .env("XDG_CONFIG_HOME", &xdg_config)
        .args(["completions", "fish"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Installed fish completions"));
    let completions_file = xdg_config.join("fish/completions/tome.fish");
    assert!(completions_file.exists());
    let content = std::fs::read_to_string(&completions_file).unwrap();
    assert!(content.contains("complete -c tome"));
}

#[test]
fn completions_bash_installs_to_file() {
    let home = TempDir::new().unwrap();
    let xdg_data = home.path().join(".local/share");
    tome()
        .env("HOME", home.path())
        .env("TOME_HOME", home.path().join(".tome"))
        .env("XDG_DATA_HOME", &xdg_data)
        .args(["completions", "bash"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Installed bash completions"));
    let completions_file = xdg_data.join("bash-completion/completions/tome");
    assert!(completions_file.exists());
    let content = std::fs::read_to_string(&completions_file).unwrap();
    assert!(content.contains("tome"));
}

#[test]
fn completions_zsh_installs_to_file() {
    let home = TempDir::new().unwrap();
    tome()
        .env("HOME", home.path())
        .env("TOME_HOME", home.path().join(".tome"))
        .args(["completions", "zsh"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Installed zsh completions"));
    let completions_file = home.path().join(".zfunc/_tome");
    assert!(completions_file.exists());
    let content = std::fs::read_to_string(&completions_file).unwrap();
    assert!(content.contains("#compdef tome"));
}

#[test]
fn completions_invalid_shell_fails() {
    tome().args(["completions", "invalid"]).assert().failure();
}

#[test]
fn completions_powershell_errors_with_instructions() {
    let tmp = TempDir::new().unwrap();
    tome()
        .env("TOME_HOME", tmp.path())
        .args(["completions", "powershell"])
        .assert()
        .failure()
        .stderr(predicate::str::contains(
            "Automatic installation not supported",
        ))
        .stderr(predicate::str::contains("--print"));
}

#[test]
fn completions_print_outputs_to_stdout() {
    let tmp = TempDir::new().unwrap();
    tome()
        .env("TOME_HOME", tmp.path())
        .args(["completions", "fish", "--print"])
        .assert()
        .success()
        .stdout(predicate::str::contains("complete -c tome"));
}

#[test]
fn exit_code_2_for_invalid_args() {
    // Clap returns exit code 2 for usage errors (invalid flags)
    tome().arg("--nonexistent-flag").assert().code(2);
}

#[test]
fn exit_code_1_for_runtime_errors() {
    // Runtime errors (missing config) return exit code 1
    tome()
        .args(["--config", "/nonexistent/path.toml", "status"])
        .assert()
        .code(1);
}

#[test]
fn no_input_flag_skips_all_prompts() {
    let env = TestEnvBuilder::new()
        .source("local", "directory")
        .target("test-tool")
        .skill("skill-a", "local")
        .build();

    // Sync with --no-input should succeed without hanging on prompts
    tome()
        .args([
            "--config",
            &env.config_path.to_string_lossy(),
            "--no-input",
            "sync",
        ])
        .assert()
        .success();

    // Verify skill was distributed to default target
    let target_dir = &env.target_dirs[0].1;
    assert!(target_dir.join("skill-a").is_symlink());
}

#[test]
fn no_color_env_suppresses_ansi_escapes() {
    let env = TestEnvBuilder::new()
        .source("local", "directory")
        .skill("skill-a", "local")
        .build();

    // Sync first to populate library
    tome()
        .args(["--config", &env.config_path.to_string_lossy(), "sync"])
        .assert()
        .success();

    // Run status with NO_COLOR=1 — output must not contain ANSI escape sequences
    let output = tome()
        .args(["--config", &env.config_path.to_string_lossy(), "status"])
        .env("NO_COLOR", "1")
        .output()
        .expect("failed to run tome status");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !stdout.contains("\x1b["),
        "stdout should not contain ANSI escapes with NO_COLOR=1, got: {stdout}"
    );
    assert!(
        !stderr.contains("\x1b["),
        "stderr should not contain ANSI escapes with NO_COLOR=1, got: {stderr}"
    );
}
