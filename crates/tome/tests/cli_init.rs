use assert_fs::TempDir;
use predicates::prelude::*;
use tome::config::{Config, DirectoryName, DirectoryRole, DirectoryType};

mod common;
use common::*;

/// Parse the wizard's dry-run TOML body from stdout. The wizard's chrome
/// (banner, dividers, "Generated config:" header) emits to stderr per
/// HARD-15; only the TOML body itself stays on stdout so a user can pipe
/// `tome init --dry-run > tome.toml`.
///
/// The `--dry-run` branch of the wizard runs `expand_tildes()` before emitting,
/// so the returned Config has absolute paths — tilde-relative comparisons do
/// NOT work; test callers must compare against expanded (TempDir-prefixed) paths.
fn parse_generated_config(stdout: &str) -> Config {
    let body = stdout.trim();
    assert!(
        !body.is_empty(),
        "dry-run stdout was empty — wizard never emitted the TOML body",
    );
    toml::from_str::<Config>(body)
        .unwrap_or_else(|e| panic!("generated TOML did not parse: {e}\n---\n{body}"))
}

/// Assert a Config round-trips: serialize, parse back, re-serialize, compare
/// bytes. Mirrors `Config::save_checked`'s round-trip guard.
fn assert_config_roundtrips(config: &Config) {
    let emitted = toml::to_string_pretty(config).expect("serialize Config");
    let reparsed: Config = toml::from_str(&emitted).expect("reparse Config");
    let reemitted = toml::to_string_pretty(&reparsed).expect("re-serialize Config");
    assert_eq!(
        emitted, reemitted,
        "Config round-trip mismatch — a field is not reversibly (de)serializable.\n\
         --- first emit ---\n{emitted}\n--- second emit ---\n{reemitted}",
    );
}

#[test]
fn init_with_no_input_and_dry_run_succeeds() {
    // Headless smoke: `tome init --no-input --dry-run` must run the wizard to
    // completion without any TTY and print the `Generated config:` chrome on
    // stderr (HARD-15) plus the TOML body itself on stdout. Richer assertions
    // on the emitted TOML live in `init_dry_run_no_input_empty_home` and
    // `init_dry_run_no_input_seeded_home`.
    let tmp = TempDir::new().unwrap();
    tome()
        .env("HOME", tmp.path())
        .env("NO_COLOR", "1")
        .args([
            "--tome-home",
            &tmp.path().display().to_string(),
            "--no-input",
            "--dry-run",
            "init",
        ])
        .assert()
        .success()
        .stderr(predicate::str::contains("Generated config:"))
        // The dry-run TOML body stays on stdout so it is pipe-safe.
        .stdout(predicate::str::contains("library_dir"));
}

#[test]
fn init_dry_run_no_input_empty_home() {
    // HOME has nothing under it → no known directories auto-discovered.
    // Wizard should still complete and print a valid, empty-directories Config.
    let tmp = TempDir::new().unwrap();
    let tome_home = tmp.path().join(".tome");

    let output = tome()
        .args(["init", "--dry-run", "--no-input"])
        .env("HOME", tmp.path())
        .env("TOME_HOME", &tome_home)
        .env("NO_COLOR", "1")
        .output()
        .unwrap();

    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    assert!(
        output.status.success(),
        "tome init --dry-run --no-input failed (empty HOME).\nstdout:\n{stdout}\nstderr:\n{stderr}",
    );

    let config = parse_generated_config(&stdout);

    assert!(
        config.directories().is_empty(),
        "expected empty directories on empty HOME, got: {:?}",
        config.directories().keys().collect::<Vec<_>>(),
    );

    assert_eq!(
        config.library_dir(),
        tmp.path().join(".tome/skills").as_path(),
        "library_dir should be <HOME>/.tome/skills after tilde expansion",
    );

    assert!(
        config.exclude().is_empty(),
        "expected empty exclude set, got: {:?}",
        config.exclude(),
    );

    config.validate().unwrap_or_else(|e| {
        panic!("generated config failed Config::validate(): {e:#}\nstdout:\n{stdout}")
    });

    assert_config_roundtrips(&config);
}

#[test]
fn init_dry_run_no_input_seeded_home() {
    // Seed HOME with one managed known dir and one synced known dir.
    // Wizard should auto-discover both, assign the expected type+role, and the
    // resulting Config should validate + round-trip.
    let tmp = TempDir::new().unwrap();
    let tome_home = tmp.path().join(".tome");

    std::fs::create_dir_all(tmp.path().join(".claude/plugins")).unwrap();
    std::fs::create_dir_all(tmp.path().join(".claude/skills")).unwrap();

    let output = tome()
        .args(["init", "--dry-run", "--no-input"])
        .env("HOME", tmp.path())
        .env("TOME_HOME", &tome_home)
        .env("NO_COLOR", "1")
        .output()
        .unwrap();

    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    assert!(
        output.status.success(),
        "tome init --dry-run --no-input failed (seeded HOME).\nstdout:\n{stdout}\nstderr:\n{stderr}",
    );

    let config = parse_generated_config(&stdout);

    assert_eq!(
        config.directories().len(),
        2,
        "expected exactly 2 directories (claude-plugins + claude-skills), got {}: {:?}",
        config.directories().len(),
        config.directories().keys().collect::<Vec<_>>(),
    );

    // claude-plugins entry: ClaudePlugins type, Managed role, expanded path.
    // role() is the accessor (field is pub(crate)); path + directory_type are pub.
    let plugins = config
        .directories()
        .get(&DirectoryName::new("claude-plugins").unwrap())
        .unwrap_or_else(|| {
            panic!(
                "missing claude-plugins entry; got: {:?}",
                config.directories().keys().collect::<Vec<_>>(),
            )
        });
    assert_eq!(plugins.directory_type, DirectoryType::ClaudePlugins);
    assert_eq!(plugins.role(), DirectoryRole::Managed);
    assert_eq!(
        plugins.path,
        tmp.path().join(".claude/plugins"),
        "claude-plugins path should be <HOME>/.claude/plugins after tilde expansion",
    );

    // claude-skills entry: Directory type, Synced role, expanded path.
    let skills = config
        .directories()
        .get(&DirectoryName::new("claude-skills").unwrap())
        .unwrap_or_else(|| {
            panic!(
                "missing claude-skills entry; got: {:?}",
                config.directories().keys().collect::<Vec<_>>(),
            )
        });
    assert_eq!(skills.directory_type, DirectoryType::Directory);
    assert_eq!(skills.role(), DirectoryRole::Synced);
    assert_eq!(
        skills.path,
        tmp.path().join(".claude/skills"),
        "claude-skills path should be <HOME>/.claude/skills after tilde expansion",
    );

    assert_eq!(
        config.library_dir(),
        tmp.path().join(".tome/skills").as_path(),
        "library_dir should be <HOME>/.tome/skills after tilde expansion",
    );

    config.validate().unwrap_or_else(|e| {
        panic!("generated config failed Config::validate(): {e:#}\nstdout:\n{stdout}")
    });

    assert_config_roundtrips(&config);
}

#[test]
fn init_no_input_writes_config_and_reloads() {
    // End-to-end save path: `tome init --no-input` (no --dry-run) runs the wizard
    // → assemble_config → save_checked → writes tome.toml → post-init sync().
    // A future regression in save_checked, post-init sync, or the no_input save
    // branch in wizard::run would slip past the dry-run tests above.
    //
    // Invariants:
    //   - exit 0
    //   - $TOME_HOME/tome.toml exists after the run
    //   - Config::load on the written file yields a valid Config
    //   - directories()/library_dir()/exclude() match the headless defaults
    //     on empty HOME (include-all, ~/.tome/skills expanded, empty exclude)
    //   - written file round-trips byte-equal through toml::{to_string_pretty, from_str}
    let tmp = TempDir::new().unwrap();
    let tome_home = tmp.path().join(".tome");
    let config_path = tome_home.join("tome.toml");

    let output = tome()
        .args(["init", "--no-input"])
        .env("HOME", tmp.path())
        .env("TOME_HOME", &tome_home)
        .env("NO_COLOR", "1")
        .output()
        .unwrap();

    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    assert!(
        output.status.success(),
        "tome init --no-input failed.\nstdout:\n{stdout}\nstderr:\n{stderr}",
    );

    assert!(
        config_path.exists(),
        "expected tome.toml at {} after `tome init --no-input`, but nothing was written",
        config_path.display(),
    );

    // HARD-22 / D-TILDE-1 (Plan 15-02): `save_checked` now preserves `~/...`
    // shape on disk so a checked-in `tome.toml` is portable across machines.
    // The wizard's headless default is `~/.tome/skills`; on-disk it stays
    // `~/.tome/skills`. We assert the on-disk shape directly here, then
    // load + validate + check exclude/directories below.
    let on_disk = std::fs::read_to_string(&config_path).unwrap_or_else(|e| {
        panic!(
            "failed to read written config at {}: {e}",
            config_path.display()
        )
    });
    assert!(
        on_disk.contains("library_dir = \"~/.tome/skills\""),
        "expected ~-shape preserved in saved file (D-TILDE-1), got:\n{on_disk}",
    );

    // Load reparses the on-disk file. `Config::load` calls `expand_tildes()`,
    // which uses `dirs::home_dir()` of the *test process* (NOT the subprocess
    // we ran with a fresh HOME env). On macOS/Linux that resolves to the real
    // user's home, so the loaded library_dir reflects the test process's HOME.
    // Skip the absolute-path check; assert structure invariants instead.
    let loaded = Config::load(&config_path).unwrap_or_else(|e| {
        panic!(
            "Config::load on wizard-written file failed: {e:#}\nfile:\n{}",
            std::fs::read_to_string(&config_path).unwrap_or_default()
        )
    });

    loaded
        .validate()
        .unwrap_or_else(|e| panic!("reloaded config failed Config::validate(): {e:#}"));

    // Empty HOME → no auto-discovered known dirs.
    assert!(
        loaded.directories().is_empty(),
        "expected empty directories on empty HOME, got: {:?}",
        loaded.directories().keys().collect::<Vec<_>>(),
    );

    // After load, library_dir is tilde-expanded against the test-process HOME
    // (dirs::home_dir() does not honor a child process's $HOME). The loaded
    // path must end with `.tome/skills` and be absolute.
    let loaded_lib = loaded.library_dir();
    assert!(
        loaded_lib.is_absolute(),
        "library_dir should be expanded to an absolute path on load, got: {}",
        loaded_lib.display(),
    );
    assert!(
        loaded_lib.ends_with(".tome/skills"),
        "library_dir should end with .tome/skills, got: {}",
        loaded_lib.display(),
    );

    assert!(loaded.exclude().is_empty(), "expected empty exclude set");

    // Round-trip parity on the written file.
    assert_config_roundtrips(&loaded);
}

#[test]
fn init_prints_resolved_tome_home_with_default_source() {
    // No TOME_HOME set, HOME has no ~/.config/tome/config.toml → Default source.
    let tmp = TempDir::new().unwrap();

    let output = tome()
        .args(["init", "--dry-run", "--no-input"])
        .env("HOME", tmp.path())
        .env_remove("TOME_HOME")
        .env("NO_COLOR", "1")
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "tome init failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    // HARD-15: wizard chrome (the resolved tome_home line) emits on stderr.
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("resolved tome_home:"),
        "stderr missing resolved tome_home line:\n{stderr}"
    );
    assert!(
        stderr.contains("(from default)"),
        "stderr missing '(from default)' source label:\n{stderr}"
    );
}

#[test]
fn init_prints_resolved_tome_home_with_env_source() {
    let tmp = TempDir::new().unwrap();
    let tome_home = tmp.path().join(".tome");

    let output = tome()
        .args(["init", "--dry-run", "--no-input"])
        .env("HOME", tmp.path())
        .env("TOME_HOME", &tome_home)
        .env("NO_COLOR", "1")
        .output()
        .unwrap();
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(output.status.success(), "stderr: {stderr}");
    // HARD-15: wizard chrome (the resolved tome_home line) emits on stderr.
    assert!(
        stderr.contains("(from TOME_HOME env)"),
        "stderr missing '(from TOME_HOME env)' label:\n{stderr}"
    );
    assert!(
        stderr.contains(tome_home.display().to_string().as_str()),
        "stderr missing TOME_HOME path:\n{stderr}"
    );
}

#[test]
fn init_prints_resolved_tome_home_with_flag_source() {
    let tmp = TempDir::new().unwrap();
    let custom = tmp.path().join("custom-home");

    let output = tome()
        .args([
            "init",
            "--dry-run",
            "--no-input",
            "--tome-home",
            custom.to_str().unwrap(),
        ])
        .env("HOME", tmp.path())
        .env_remove("TOME_HOME")
        .env("NO_COLOR", "1")
        .output()
        .unwrap();
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(output.status.success(), "stderr: {stderr}");
    // HARD-15: wizard chrome (the resolved tome_home line) emits on stderr.
    assert!(
        stderr.contains("(from --tome-home flag)"),
        "stderr missing '--tome-home flag' label:\n{stderr}"
    );
}

#[test]
fn init_resolved_tome_home_line_precedes_step_prompts() {
    let tmp = TempDir::new().unwrap();
    let tome_home = tmp.path().join(".tome");

    let output = tome()
        .args(["init", "--dry-run", "--no-input"])
        .env("HOME", tmp.path())
        .env("TOME_HOME", &tome_home)
        .env("NO_COLOR", "1")
        .output()
        .unwrap();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    assert!(output.status.success(), "stderr: {stderr}");
    // HARD-15: wizard chrome (the resolved tome_home line and the Step 1
    // header) both emit on stderr.

    let resolved_idx = stderr
        .find("resolved tome_home:")
        .expect("missing info line");
    let step1_idx = stderr.find("Step 1").expect("missing Step 1 prompt header");
    assert!(
        resolved_idx < step1_idx,
        "resolved tome_home line must come BEFORE Step 1.\n\
         resolved_idx={resolved_idx}, step1_idx={step1_idx}\nstderr:\n{stderr}"
    );
}

#[test]
fn init_legacy_detected_no_input_leaves_file() {
    let tmp = TempDir::new().unwrap();
    let tome_home = tmp.path().join(".tome");
    let xdg_dir = tmp.path().join(".config/tome");
    let xdg_file = xdg_dir.join("config.toml");
    std::fs::create_dir_all(&xdg_dir).unwrap();
    let legacy_content = "[[sources]]\nname = \"old\"\npath = \"/tmp\"\ntype = \"directory\"\n";
    std::fs::write(&xdg_file, legacy_content).unwrap();

    let output = tome()
        .args(["init", "--dry-run", "--no-input"])
        .env("HOME", tmp.path())
        .env("TOME_HOME", &tome_home)
        .env("NO_COLOR", "1")
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "tome init failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stderr = String::from_utf8_lossy(&output.stderr);

    // HARD-15: warning AND skip note both emit on stderr (handle_legacy_cleanup
    // is wizard chrome).
    assert!(
        stderr.contains("Legacy pre-v0.6 config detected"),
        "stderr missing legacy warning:\n{stderr}"
    );
    assert!(
        stderr.contains("skipped legacy cleanup"),
        "stderr missing skipped-cleanup note:\n{stderr}"
    );

    // File must be byte-identical after the run.
    let after = std::fs::read_to_string(&xdg_file).unwrap();
    assert_eq!(after, legacy_content, "legacy file should be unchanged");
}

#[test]
fn init_legacy_with_only_tome_home_not_flagged() {
    let tmp = TempDir::new().unwrap();
    let tome_home = tmp.path().join(".tome");
    let xdg_dir = tmp.path().join(".config/tome");
    std::fs::create_dir_all(&xdg_dir).unwrap();
    // v0.6+ shape — should NOT trigger legacy warning.
    std::fs::write(xdg_dir.join("config.toml"), "tome_home = \"~/somewhere\"\n").unwrap();

    let output = tome()
        .args(["init", "--dry-run", "--no-input"])
        .env("HOME", tmp.path())
        .env("TOME_HOME", &tome_home)
        .env("NO_COLOR", "1")
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stderr = String::from_utf8_lossy(&output.stderr);
    // HARD-15: legacy warning and skip-note are both stderr-emitted.
    assert!(
        !stderr.contains("Legacy pre-v0.6 config detected"),
        "v0.6+-only XDG file should NOT trigger legacy warning. stderr:\n{stderr}"
    );
    assert!(
        !stderr.contains("skipped legacy cleanup"),
        "v0.6+-only XDG file should NOT trigger skip-note. stderr:\n{stderr}"
    );
}

#[test]
fn init_greenfield_no_legacy_warning() {
    let tmp = TempDir::new().unwrap();
    let tome_home = tmp.path().join(".tome");

    let output = tome()
        .args(["init", "--dry-run", "--no-input"])
        .env("HOME", tmp.path())
        .env("TOME_HOME", &tome_home)
        .env("NO_COLOR", "1")
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stderr = String::from_utf8_lossy(&output.stderr);
    // HARD-15: legacy warning is stderr-emitted.
    assert!(
        !stderr.contains("Legacy pre-v0.6 config detected"),
        "greenfield run should NOT show legacy warning. stderr:\n{stderr}"
    );
}

#[test]
fn init_greenfield_no_input_skips_step_0_prompt() {
    // TomeHomeSource::Default + --no-input → Step 0 prompt must be skipped.
    let tmp = TempDir::new().unwrap();

    let output = tome()
        .args(["init", "--dry-run", "--no-input"])
        .env("HOME", tmp.path())
        .env_remove("TOME_HOME")
        .env("NO_COLOR", "1")
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "tome init failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stderr = String::from_utf8_lossy(&output.stderr);
    // HARD-15: Step header and resolved tome_home line both emit on stderr.
    assert!(
        !stderr.contains("Step 0:"),
        "--no-input must skip Step 0 prompt, but stderr contains it:\n{stderr}"
    );
    // WUX-04 info line still prints (informational, not a prompt)
    assert!(
        stderr.contains("resolved tome_home:"),
        "resolved tome_home line must still appear in --no-input mode:\n{stderr}"
    );
}

#[test]
fn init_greenfield_no_input_does_not_write_xdg() {
    // --no-input must NOT write to ~/.config/tome/config.toml even under greenfield.
    // (07-RESEARCH.md § "Integration with no_input" — "Skip" row for WUX-05.)
    let tmp = TempDir::new().unwrap();

    let output = tome()
        .args(["init", "--dry-run", "--no-input"])
        .env("HOME", tmp.path())
        .env_remove("TOME_HOME")
        .env("NO_COLOR", "1")
        .output()
        .unwrap();
    assert!(output.status.success());

    let xdg = tmp.path().join(".config/tome/config.toml");
    assert!(
        !xdg.exists(),
        "--no-input must not write XDG config, but {} exists",
        xdg.display()
    );
}

#[test]
fn init_with_flag_source_skips_step_0() {
    // TomeHomeSource::CliTomeHome (from --tome-home flag) → Step 0 MUST be skipped
    // even without --no-input, because the user already indicated a choice.
    // We test via --no-input to keep the test headless; the key assertion is on
    // the "Step 0:" header absence.
    let tmp = TempDir::new().unwrap();
    let custom = tmp.path().join("custom-home");

    let output = tome()
        .args([
            "init",
            "--dry-run",
            "--no-input",
            "--tome-home",
            custom.to_str().unwrap(),
        ])
        .env("HOME", tmp.path())
        .env_remove("TOME_HOME")
        .env("NO_COLOR", "1")
        .output()
        .unwrap();
    assert!(output.status.success());

    let stderr = String::from_utf8_lossy(&output.stderr);
    // HARD-15: Step header and tome_home label both emit on stderr.
    assert!(
        !stderr.contains("Step 0:"),
        "--tome-home flag (CliTomeHome source) must skip Step 0:\n{stderr}"
    );
    assert!(
        stderr.contains("(from --tome-home flag)"),
        "source label should confirm flag branch:\n{stderr}"
    );
}

#[test]
fn init_derived_library_default_under_custom_tome_home() {
    // When tome_home = <tmp>/custom-tome (non-default), library default should
    // derive as <tmp>/custom-tome/skills (NOT ~/.tome/skills). Tests the
    // Pitfall 1 fix.
    let tmp = TempDir::new().unwrap();
    let custom = tmp.path().join("custom-tome");

    let output = tome()
        .args([
            "init",
            "--dry-run",
            "--no-input",
            "--tome-home",
            custom.to_str().unwrap(),
        ])
        .env("HOME", tmp.path())
        .env_remove("TOME_HOME")
        .env("NO_COLOR", "1")
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let config = parse_generated_config(&stdout);
    // library_dir after tilde expansion should be under the custom tome_home,
    // NOT under tmp/.tome/skills.
    assert_eq!(
        config.library_dir(),
        custom.join("skills"),
        "library default should derive from --tome-home, got {:?}",
        config.library_dir()
    );
}

#[test]
fn init_brownfield_no_input_keeps_existing() {
    let tmp = TempDir::new().unwrap();
    let tome_home = tmp.path().join(".tome");
    std::fs::create_dir_all(&tome_home).unwrap();
    let config_path = tome_home.join("tome.toml");
    let seed = "library_dir = \"~/.tome/skills\"\n[directories]\n";
    std::fs::write(&config_path, seed).unwrap();

    let output = tome()
        .args(["init", "--no-input"]) // NOT --dry-run — we want the actual no-op path
        .env("HOME", tmp.path())
        .env("TOME_HOME", &tome_home)
        .env("NO_COLOR", "1")
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "tome init should succeed; stderr:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stderr = String::from_utf8_lossy(&output.stderr);
    // HARD-15: brownfield summary is wizard chrome and emits on stderr.
    assert!(
        stderr.contains("Existing config detected"),
        "stderr missing brownfield summary:\n{stderr}"
    );

    // File must be byte-identical
    let after = std::fs::read_to_string(&config_path).unwrap();
    assert_eq!(
        after, seed,
        "brownfield --no-input must not modify existing config"
    );

    // No sync side-effect: library dir should not be created
    let library = tmp.path().join(".tome/skills");
    assert!(
        !library.exists(),
        "use-existing path must not run post-init sync (library dir present at {})",
        library.display()
    );
}

#[test]
fn init_brownfield_invalid_config_no_input_cancels() {
    let tmp = TempDir::new().unwrap();
    let tome_home = tmp.path().join(".tome");
    std::fs::create_dir_all(&tome_home).unwrap();
    let config_path = tome_home.join("tome.toml");
    let seed = "this is [[[ not valid toml";
    std::fs::write(&config_path, seed).unwrap();

    let output = tome()
        .args(["init", "--no-input"])
        .env("HOME", tmp.path())
        .env("TOME_HOME", &tome_home)
        .env("NO_COLOR", "1")
        .output()
        .unwrap();
    // Must exit 0 (clean cancel), not an error
    assert!(
        output.status.success(),
        "invalid-config no-input path should cancel cleanly (exit 0); stderr:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stderr = String::from_utf8_lossy(&output.stderr);
    // HARD-15: invalid-config marker is wizard chrome on stderr.
    assert!(
        stderr.contains("invalid:") || stderr.contains("cancelled"),
        "stderr should indicate invalid config or cancellation:\n{stderr}"
    );

    let after = std::fs::read_to_string(&config_path).unwrap();
    assert_eq!(
        after, seed,
        "invalid-config no-input must not modify the file"
    );
}

#[test]
fn init_brownfield_with_legacy_runs_both_cleanups() {
    let tmp = TempDir::new().unwrap();
    let tome_home = tmp.path().join(".tome");
    std::fs::create_dir_all(&tome_home).unwrap();
    let config_path = tome_home.join("tome.toml");
    let brownfield_seed = "library_dir = \"~/.tome/skills\"\n[directories]\n";
    std::fs::write(&config_path, brownfield_seed).unwrap();

    let xdg_dir = tmp.path().join(".config/tome");
    let xdg_file = xdg_dir.join("config.toml");
    std::fs::create_dir_all(&xdg_dir).unwrap();
    let legacy_seed = "[[sources]]\nname = \"old\"\npath = \"/tmp\"\ntype = \"directory\"\n";
    std::fs::write(&xdg_file, legacy_seed).unwrap();

    let output = tome()
        .args(["init", "--no-input"])
        .env("HOME", tmp.path())
        .env("TOME_HOME", &tome_home)
        .env("NO_COLOR", "1")
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stderr = String::from_utf8_lossy(&output.stderr);
    // HARD-15: both cleanup chrome lines emit on stderr.
    assert!(
        stderr.contains("Legacy pre-v0.6 config detected"),
        "stderr missing legacy warning:\n{stderr}"
    );
    assert!(
        stderr.contains("Existing config detected"),
        "stderr missing brownfield summary:\n{stderr}"
    );

    // Both files unchanged under --no-input
    assert_eq!(
        std::fs::read_to_string(&config_path).unwrap(),
        brownfield_seed
    );
    assert_eq!(std::fs::read_to_string(&xdg_file).unwrap(), legacy_seed);
}
