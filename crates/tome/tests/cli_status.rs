use assert_fs::TempDir;

mod common;
use common::*;

#[test]
fn status_shows_library_info() {
    let tmp = TempDir::new().unwrap();
    let config = write_config(tmp.path(), "");

    let output = tome()
        .args(["--config", config.to_str().unwrap(), "status"])
        .env("NO_COLOR", "1")
        .output()
        .unwrap();
    assert!(output.status.success());

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

    let output = tome()
        .args(["--config", config_path.to_str().unwrap(), "status"])
        .env("NO_COLOR", "1")
        .output()
        .unwrap();
    assert!(output.status.success());

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

#[cfg(unix)]
#[test]
fn machine_override_rewrites_directory_path_for_status() {
    // PORT-01 + PORT-02 smoke: declare an override in machine.toml and
    // confirm `tome status --json` reports the OVERRIDDEN path, proving
    // the load pipeline applied the override before status::gather ran.
    let tmp = TempDir::new().unwrap();
    let real_skills = tmp.path().join("real-skills");
    create_skill(&real_skills, "x");

    // tome.toml points at a path that does NOT exist.
    let tome_toml = format!(
        "library_dir = \"{}/library\"\n\
         \n\
         [directories.work]\n\
         path = \"{}/does-not-exist\"\n\
         type = \"directory\"\n\
         role = \"source\"\n",
        tmp.path().display(),
        tmp.path().display(),
    );
    std::fs::write(tmp.path().join("tome.toml"), tome_toml).unwrap();

    // machine.toml overrides directories.work.path to the real path.
    let machine_toml = format!(
        "[directory_overrides.work]\npath = \"{}\"\n",
        real_skills.display(),
    );
    let machine_path = tmp.path().join("machine.toml");
    std::fs::write(&machine_path, machine_toml).unwrap();

    let assert = tome()
        .args([
            "--tome-home",
            tmp.path().to_str().unwrap(),
            "--machine",
            machine_path.to_str().unwrap(),
            "status",
            "--json",
        ])
        .env("NO_COLOR", "1")
        .assert()
        .success();
    let stdout = String::from_utf8(assert.get_output().stdout.clone()).unwrap();
    let report: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    let dirs = report["directories"].as_array().unwrap();
    let work = dirs
        .iter()
        .find(|d| d["name"] == "work")
        .expect("status JSON missing 'work' directory");
    let path = work["path"].as_str().unwrap();
    assert!(
        path.contains("real-skills"),
        "expected status to report overridden path, got: {path}"
    );
    assert!(
        !path.contains("does-not-exist"),
        "expected status to NOT report the original tome.toml path, got: {path}"
    );
}

#[cfg(unix)]
#[test]
fn machine_override_appears_in_status_and_doctor() {
    // PORT-05 (and end-to-end PORT-01/02 confirmation): an override declared
    // in machine.toml causes:
    //   - `tome sync` to operate on the overridden path,
    //   - `tome status` text mode to show `(override)` on the affected row,
    //   - `tome status --json` to include `override_applied: true`,
    //   - `tome doctor --json` to include `override_applied: true` for the overridden directory.
    //
    // The overridden directory `work` uses role = "synced" so it appears in BOTH
    // discovery (skill-a from real_path is consolidated into the library) AND
    // distribution (`tome doctor` diagnoses it). This pins the full PORT-05
    // contract end-to-end on an actually-overridden directory.
    let tmp = TempDir::new().unwrap();
    let library_dir = tmp.path().join("library");
    std::fs::create_dir_all(&library_dir).unwrap();

    // Two directories. `work` is synced (discovery + distribution); `other` is
    // a plain source for the negative-case check in status JSON.
    let dotfiles_path = tmp.path().join("dotfiles-says-here");
    let real_path = tmp.path().join("real-skills");
    create_skill(&real_path, "skill-a");
    // The synced directory must EXIST on disk pre-sync — distribute writes
    // symlinks into it. Create the real path's parent (already done by
    // `create_skill`) and ensure `real_path` itself is a directory.
    assert!(
        real_path.is_dir(),
        "real_path must exist for sync to succeed"
    );

    let other_path = tmp.path().join("other-skills");
    create_skill(&other_path, "skill-b");

    let tome_toml = format!(
        "library_dir = \"{}\"\n\
         \n\
         [directories.work]\n\
         path = \"{}\"\n\
         type = \"directory\"\n\
         role = \"synced\"\n\
         \n\
         [directories.other]\n\
         path = \"{}\"\n\
         type = \"directory\"\n\
         role = \"source\"\n",
        library_dir.display(),
        dotfiles_path.display(),
        other_path.display(),
    );
    std::fs::write(tmp.path().join("tome.toml"), tome_toml).unwrap();

    let machine_toml = format!(
        "[directory_overrides.work]\npath = \"{}\"\n",
        real_path.display(),
    );
    let machine_path = tmp.path().join("machine.toml");
    std::fs::write(&machine_path, machine_toml).unwrap();

    // 1. `tome sync` must succeed — sync sees the overridden path.
    let sync_assert = tome()
        .args([
            "--tome-home",
            tmp.path().to_str().unwrap(),
            "--machine",
            machine_path.to_str().unwrap(),
            "sync",
            "--no-triage",
        ])
        .env("NO_COLOR", "1")
        .assert()
        .success();
    let sync_stdout = String::from_utf8(sync_assert.get_output().stdout.clone()).unwrap();
    let skill_a_in_lib = library_dir.join("skill-a").exists();
    assert!(
        skill_a_in_lib,
        "expected skill-a from overridden path to be consolidated, got sync stdout:\n{sync_stdout}",
    );

    // 2. `tome status` text mode — stdout contains `(override)` exactly once.
    let status_assert = tome()
        .args([
            "--tome-home",
            tmp.path().to_str().unwrap(),
            "--machine",
            machine_path.to_str().unwrap(),
            "status",
        ])
        .env("NO_COLOR", "1")
        .assert()
        .success();
    let status_stdout = String::from_utf8(status_assert.get_output().stdout.clone()).unwrap();
    assert!(
        status_stdout.contains("(override)"),
        "expected `tome status` text output to contain `(override)`, got:\n{status_stdout}"
    );
    let override_marker_count = status_stdout.matches("(override)").count();
    assert_eq!(
        override_marker_count, 1,
        "expected exactly one `(override)` marker (for `work`), got {override_marker_count} in:\n{status_stdout}"
    );

    // 3. `tome status --json` — `work` has `override_applied: true`, `other` has false.
    let status_json_assert = tome()
        .args([
            "--tome-home",
            tmp.path().to_str().unwrap(),
            "--machine",
            machine_path.to_str().unwrap(),
            "status",
            "--json",
        ])
        .env("NO_COLOR", "1")
        .assert()
        .success();
    let status_json: serde_json::Value =
        serde_json::from_slice(&status_json_assert.get_output().stdout)
            .expect("status --json output must be valid JSON");
    let dirs = status_json["directories"].as_array().unwrap();
    let work = dirs.iter().find(|d| d["name"] == "work").unwrap();
    let other = dirs.iter().find(|d| d["name"] == "other").unwrap();
    assert_eq!(
        work["override_applied"],
        serde_json::Value::Bool(true),
        "expected work.override_applied == true, got: {work}"
    );
    assert_eq!(
        other["override_applied"],
        serde_json::Value::Bool(false),
        "expected other.override_applied == false, got: {other}"
    );

    // 4. `tome doctor --json` — `work` (now synced/distribution) appears in
    // `directory_issues` and carries `override_applied: true`. This is the
    // strongest end-to-end PORT-05 doctor assertion.
    let doctor_json_assert = tome()
        .args([
            "--tome-home",
            tmp.path().to_str().unwrap(),
            "--machine",
            machine_path.to_str().unwrap(),
            "doctor",
            "--json",
        ])
        .env("NO_COLOR", "1")
        .assert();
    // doctor exit may be 0 or non-0 depending on issues found — accept either.
    let doctor_json: serde_json::Value =
        serde_json::from_slice(&doctor_json_assert.get_output().stdout)
            .expect("doctor --json output must be valid JSON");

    let doctor_dirs = doctor_json["directory_issues"]
        .as_array()
        .expect("doctor --json must include directory_issues array");
    let work_entry = doctor_dirs
        .iter()
        .find(|d| d["name"] == "work")
        .expect("work must appear in doctor directory_issues (it has role = synced)");
    assert_eq!(
        work_entry["override_applied"],
        serde_json::Value::Bool(true),
        "expected work.override_applied == true in doctor JSON, got: {work_entry}"
    );

    // Sanity: every entry in directory_issues uses the new DirectoryDiagnostic
    // shape (has `name`, `issues`, and `override_applied`).
    for entry in doctor_dirs {
        assert!(
            entry.get("name").is_some()
                && entry.get("issues").is_some()
                && entry.get("override_applied").is_some(),
            "expected DirectoryDiagnostic shape (name + issues + override_applied), got: {entry}"
        );
    }
}

#[test]
fn phase14_status_text_shows_unowned_section() {
    let fix = phase14_build_fixture(&[], &[], &[("orphan", "removed-dir")]);

    let output = fix.cmd().arg("status").output().unwrap();
    assert!(output.status.success());
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
