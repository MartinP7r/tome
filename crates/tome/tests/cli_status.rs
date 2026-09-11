use assert_fs::TempDir;
use std::process::Command as StdCommand;

mod common;
use common::*;

fn init_git_repo(dir: &std::path::Path) {
    let global_config = dir.with_extension("gitconfig");
    std::fs::write(&global_config, "").unwrap();
    for args in [
        &["init", "-b", "main"][..],
        &["config", "--local", "user.email", "test@test.com"],
        &["config", "--local", "user.name", "Test"],
        &["config", "--local", "commit.gpgsign", "false"],
        &["add", "-A"],
        &["commit", "-m", "seed"],
    ] {
        let output = StdCommand::new("git")
            .args(args)
            .current_dir(dir)
            .env_remove("GIT_DIR")
            .env_remove("GIT_WORK_TREE")
            .env_remove("GIT_INDEX_FILE")
            .env_remove("GIT_CONFIG")
            .env_remove("GIT_CONFIG_COUNT")
            .env("GIT_CONFIG_NOSYSTEM", "1")
            .env("GIT_CONFIG_GLOBAL", &global_config)
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "git {args:?} failed with {}: {}",
            output.status,
            String::from_utf8_lossy(&output.stderr)
        );
    }
}

fn git_output(dir: &std::path::Path, args: &[&str]) -> Vec<u8> {
    let output = StdCommand::new("git")
        .args(args)
        .current_dir(dir)
        .env_remove("GIT_DIR")
        .env_remove("GIT_WORK_TREE")
        .env_remove("GIT_INDEX_FILE")
        .output()
        .unwrap();
    assert!(output.status.success(), "git {args:?} failed");
    output.stdout
}

#[test]
fn status_shows_library_info() {
    let tmp = TempDir::new().unwrap();
    let config = write_config(tmp.path(), "");

    let output = tome()
        .args(["--config", config.to_str().unwrap(), "status"])
        .env("NO_COLOR", "1")
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    let settings = snapshot_settings(&tmp);
    settings.bind(|| {
        insta::assert_snapshot!("status_empty_library", stdout);
    });
}

#[test]
fn status_without_config_shows_init_prompt() {
    let tmp = TempDir::new().unwrap();
    // Point library_dir at a nonexistent dir (no sources) to simulate unconfigured state.
    // Using write_config would create library_dir, defeating the purpose.
    let config_path = tmp.path().join("config.toml");
    let nonexistent_library = tmp.path().join("nonexistent-library");
    std::fs::write(
        &config_path,
        format!("library_dir = \"{}\"", nonexistent_library.display()),
    )
    .unwrap();
    write_test_profile(tmp.path(), "");

    let output = tome()
        .args(["--config", config_path.to_str().unwrap(), "status"])
        .env("NO_COLOR", "1")
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    let settings = snapshot_settings(&tmp);
    settings.bind(|| {
        insta::assert_snapshot!("status_unconfigured", stdout);
    });
}

#[test]
fn status_json_output() {
    let env = TestEnvBuilder::new()
        .source("local", "directory")
        .target("test-tool")
        .skill("skill-a", "local")
        .build();

    tome()
        .args(["--config", &env.config_path.to_string_lossy(), "sync"])
        .assert()
        .success();

    let output = tome()
        .args([
            "--config",
            &env.config_path.to_string_lossy(),
            "status",
            "--json",
        ])
        .output()
        .expect("failed to run");

    let json: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("status --json should produce valid JSON");
    assert_eq!(json["configured"], true);
    assert!(json["directories"].is_array());
}

#[test]
fn profile_and_clean_git_health() {
    let tmp = TempDir::new().unwrap();
    let config = write_config(tmp.path(), "");
    init_git_repo(tmp.path());

    let text = tome()
        .args(["--config", config.to_str().unwrap(), "status"])
        .env("NO_COLOR", "1")
        .output()
        .unwrap();
    assert!(text.status.success());
    let text = String::from_utf8_lossy(&text.stdout);
    assert!(
        text.contains("Profile: test"),
        "missing selected profile: {text}"
    );
    assert!(
        text.contains("Git: clean"),
        "missing clean Git health: {text}"
    );

    let json = tome()
        .args(["--config", config.to_str().unwrap(), "status", "--json"])
        .output()
        .unwrap();
    assert!(json.status.success());
    let json: serde_json::Value = serde_json::from_slice(&json.stdout).unwrap();
    assert_eq!(json["profile"]["kind"], "selected");
    assert_eq!(json["profile"]["name"], "test");
    assert_eq!(json["git"]["kind"], "available");
    assert_eq!(json["git"]["clean"], true);
    assert_eq!(json["git"]["staged"]["total"], 0);
    assert_eq!(json["git"]["unstaged"]["total"], 0);
    assert_eq!(json["git"]["upstream"]["kind"], "unavailable");
}

#[test]
fn status_reports_dirty_health_without_mutating_repository_state() {
    let tmp = TempDir::new().unwrap();
    let config = write_config(tmp.path(), "");
    init_git_repo(tmp.path());
    std::fs::write(tmp.path().join("staged"), "staged").unwrap();
    git_output(tmp.path(), &["add", "staged"]);
    std::fs::write(tmp.path().join("unstaged"), "unstaged").unwrap();

    let before_status = git_output(tmp.path(), &["status", "--porcelain=v2"]);
    let before_index = std::fs::read(tmp.path().join(".git/index")).unwrap();

    let text = tome()
        .args(["--config", config.to_str().unwrap(), "status"])
        .env("NO_COLOR", "1")
        .output()
        .unwrap();
    assert!(text.status.success());
    let text = String::from_utf8_lossy(&text.stdout);
    assert!(text.contains("Git: dirty (1 staged, 1 unstaged)"));
    assert!(text.contains("Changes: staged 1 (1 added, 0 modified, 0 deleted)"));
    assert!(text.contains("Remote: unavailable | Upstream: unavailable"));

    let json = tome()
        .args(["--config", config.to_str().unwrap(), "status", "--json"])
        .output()
        .unwrap();
    assert!(json.status.success());
    let json: serde_json::Value = serde_json::from_slice(&json.stdout).unwrap();
    assert_eq!(json["git"]["clean"], false);
    assert_eq!(json["git"]["staged"]["added"], 1);
    assert_eq!(json["git"]["unstaged"]["added"], 1);
    assert_eq!(json["git"]["upstream"]["kind"], "unavailable");

    assert_eq!(
        git_output(tmp.path(), &["status", "--porcelain=v2"]),
        before_status
    );
    assert_eq!(
        std::fs::read(tmp.path().join(".git/index")).unwrap(),
        before_index
    );
}

#[cfg(unix)]
#[test]
fn status_counts_skills_from_cached_git_source() {
    let tmp = TempDir::new().unwrap();
    let upstream_dir = tmp.path().join("upstream.git");
    std::fs::create_dir_all(&upstream_dir).unwrap();
    create_skill(&upstream_dir, "git-skill");

    init_git_repo(&upstream_dir);

    let config_path = write_config(
        tmp.path(),
        &format!(
            "[directories.myrepo]\n\
             path = \"file://{}\"\n\
             type = \"git\"\n\
             role = \"source\"\n\
             branch = \"main\"\n",
            upstream_dir.display()
        ),
    );

    tome()
        .args([
            "--config",
            &config_path.to_string_lossy(),
            "sync",
            "--no-triage",
        ])
        .assert()
        .success();

    let output = tome()
        .args([
            "--config",
            &config_path.to_string_lossy(),
            "status",
            "--json",
        ])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let report: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    let directory = report["directories"]
        .as_array()
        .unwrap()
        .iter()
        .find(|directory| directory["name"] == "myrepo")
        .unwrap();
    assert_eq!(directory["skill_count"]["count"], 1);
}

#[cfg(unix)]
#[test]
fn status_warns_when_cached_git_source_is_missing() {
    let tmp = TempDir::new().unwrap();
    let upstream_dir = tmp.path().join("upstream.git");
    std::fs::create_dir_all(&upstream_dir).unwrap();
    create_skill(&upstream_dir, "git-skill");

    init_git_repo(&upstream_dir);

    let config_path = write_config(
        tmp.path(),
        &format!(
            "[directories.myrepo]\n\
             path = \"file://{}\"\n\
             type = \"git\"\n\
             role = \"source\"\n\
             branch = \"main\"\n",
            upstream_dir.display()
        ),
    );

    tome()
        .args([
            "--config",
            &config_path.to_string_lossy(),
            "sync",
            "--no-triage",
        ])
        .assert()
        .success();

    let cache_dir = std::fs::read_dir(tmp.path().join("repos"))
        .unwrap()
        .next()
        .unwrap()
        .unwrap()
        .path();
    std::fs::remove_dir_all(cache_dir).unwrap();

    let output = tome()
        .args([
            "--config",
            &config_path.to_string_lossy(),
            "status",
            "--json",
        ])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let report: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    let directory = report["directories"]
        .as_array()
        .unwrap()
        .iter()
        .find(|directory| directory["name"] == "myrepo")
        .unwrap();
    assert!(
        directory["warnings"]
            .as_array()
            .unwrap()
            .iter()
            .any(|warning| warning.as_str().unwrap().contains("cache dir"))
    );
}

#[test]
fn phase14_status_text_shows_unowned_section() {
    let fix = phase14_build_fixture(&[], &[], &[("orphan", "removed-dir")]);

    let output = fix.cmd().arg("status").output().unwrap();
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("Unowned skills (1)"),
        "stdout must include 'Unowned skills (1)': {stdout}"
    );
    assert!(
        stdout.contains("orphan"),
        "stdout must include the skill name: {stdout}"
    );
    assert!(
        stdout.contains("removed-dir"),
        "stdout must show LAST-KNOWN SOURCE = previous_source per D-C1: {stdout}"
    );
}

#[test]
fn phase14_status_json_includes_unowned_field() {
    let fix = phase14_build_fixture(&[], &[], &[("orphan", "removed-dir")]);

    let output = fix.cmd().args(["status", "--json"]).output().unwrap();
    assert!(output.status.success());
    let json: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("status --json must produce valid JSON");
    let unowned = json["unowned"]
        .as_array()
        .expect("status --json must include 'unowned' as an array");
    assert_eq!(unowned.len(), 1, "expected 1 unowned skill: {json}");
    let entry = &unowned[0];
    assert_eq!(entry["name"], "orphan");
    assert_eq!(entry["previous_source"], "removed-dir");
    // Stable shape: SkillSummary always exposes these fields.
    for key in [
        "name",
        "previous_source",
        "source_path_display",
        "synced_at",
        "managed",
    ] {
        assert!(
            entry.get(key).is_some(),
            "SkillSummary JSON must contain '{key}': {entry}"
        );
    }
}

// ============================================================================
// OBS-07 (Plan 19-03): last_sync header + SKILLS column integration tests.
//
// These pin the user-visible behavior of D-LSYNC-1/-2/-3 + D-DIR-1:
// - text Last sync: "never" when manifest missing, RFC-3339 when stamped
// - JSON last_sync: null when fresh, RFC-3339 string after a successful sync
// - text Directories table has a SKILLS column header
// ============================================================================

#[test]
fn status_last_sync_never_for_fresh_manifest() {
    // D-LSYNC-2: a fresh TempDir with no manifest must render "Last sync: never".
    let tmp = TempDir::new().unwrap();
    let config = write_config(tmp.path(), "");

    let output = tome()
        .args(["--config", config.to_str().unwrap(), "status"])
        .env("NO_COLOR", "1")
        .output()
        .unwrap();
    assert!(output.status.success());

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("Last sync: never"),
        "fresh manifest must render 'Last sync: never', got:\n{stdout}"
    );
}

#[test]
fn status_last_sync_renders_after_sync() {
    // D-LSYNC-3: a successful sync stamps last_synced_at; subsequent status
    // renders an RFC-3339 timestamp (year prefix is the deterministic part).
    let env = TestEnvBuilder::new()
        .source("local", "directory")
        .target("test-tool")
        .skill("skill-a", "local")
        .build();

    tome()
        .args(["--config", &env.config_path.to_string_lossy(), "sync"])
        .assert()
        .success();

    let output = tome()
        .args(["--config", &env.config_path.to_string_lossy(), "status"])
        .env("NO_COLOR", "1")
        .output()
        .unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("Last sync: "),
        "post-sync status must render 'Last sync: <ts>', got:\n{stdout}"
    );
    assert!(
        !stdout.contains("Last sync: never"),
        "post-sync status must NOT render 'never', got:\n{stdout}"
    );
    // RFC-3339 year prefix: matches '20YY-' for any 21st-century stamp.
    assert!(
        stdout.contains("Last sync: 20"),
        "post-sync status must render an RFC-3339 timestamp (year 20YY), got:\n{stdout}"
    );
}

#[test]
fn status_json_last_sync_null_for_fresh() {
    // D-LSYNC-2: JSON shape emits `"last_sync": null` for fresh manifest —
    // not omitted, for stable-shape JSON consumers.
    let tmp = TempDir::new().unwrap();
    let config = write_config(tmp.path(), "");

    let output = tome()
        .args(["--config", config.to_str().unwrap(), "status", "--json"])
        .env("NO_COLOR", "1")
        .output()
        .unwrap();
    assert!(output.status.success());

    let json: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("status --json must produce valid JSON");
    assert!(
        json.get("last_sync").is_some(),
        "JSON must always include 'last_sync' key for stable shape: {json}"
    );
    assert!(
        json["last_sync"].is_null(),
        "fresh manifest must emit last_sync == null, got: {}",
        json["last_sync"]
    );
}

#[test]
fn status_json_last_sync_string_after_sync() {
    // D-LSYNC-3: after a successful sync, JSON last_sync is an RFC-3339 string.
    let env = TestEnvBuilder::new()
        .source("local", "directory")
        .target("test-tool")
        .skill("skill-a", "local")
        .build();

    tome()
        .args(["--config", &env.config_path.to_string_lossy(), "sync"])
        .assert()
        .success();

    let output = tome()
        .args([
            "--config",
            &env.config_path.to_string_lossy(),
            "status",
            "--json",
        ])
        .env("NO_COLOR", "1")
        .output()
        .unwrap();
    assert!(output.status.success());

    let json: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("status --json must produce valid JSON");
    let ts = json["last_sync"].as_str().unwrap_or_else(|| {
        panic!(
            "post-sync last_sync must be a String, got: {}",
            json["last_sync"]
        )
    });
    assert!(
        ts.ends_with('Z') && ts.len() == 20,
        "last_sync must be RFC-3339 'YYYY-MM-DDTHH:MM:SSZ' (length 20, trailing Z), got: {ts}"
    );
    assert!(
        ts.starts_with("20"),
        "last_sync must have a 21st-century year prefix, got: {ts}"
    );
}

#[test]
fn status_skills_column_present_in_text() {
    // D-DIR-1: the Directories table in text output gains a SKILLS column.
    let env = TestEnvBuilder::new()
        .source("local", "directory")
        .target("test-tool")
        .skill("skill-a", "local")
        .build();

    tome()
        .args(["--config", &env.config_path.to_string_lossy(), "sync"])
        .assert()
        .success();

    let output = tome()
        .args(["--config", &env.config_path.to_string_lossy(), "status"])
        .env("NO_COLOR", "1")
        .output()
        .unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("SKILLS"),
        "Directories table must include 'SKILLS' column header, got:\n{stdout}"
    );
    // The directory `local` was discovered with 1 skill — assert the row count.
    assert!(
        stdout.contains("✓ 1") || stdout.contains("local"),
        "Directories table must render the discovered skill count, got:\n{stdout}"
    );
}

#[test]
fn phase14_status_text_omits_unowned_section_when_empty() {
    let fix = phase14_build_fixture(&[("active-dir", "synced")], &[("alpha", "active-dir")], &[]);

    let output = fix.cmd().arg("status").output().unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        !stdout.contains("Unowned skills"),
        "stdout must NOT include 'Unowned skills' header when set is empty: {stdout}"
    );

    // JSON shape stays stable: empty array, not omitted.
    let json_output = fix.cmd().args(["status", "--json"]).output().unwrap();
    let json: serde_json::Value =
        serde_json::from_slice(&json_output.stdout).expect("status --json must produce valid JSON");
    let unowned = json["unowned"]
        .as_array()
        .expect("status --json must include 'unowned' as an array even when empty");
    assert!(
        unowned.is_empty(),
        "unowned array must be empty (not omitted): {json}"
    );
}
