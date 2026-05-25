use assert_fs::TempDir;
use predicates::prelude::*;

mod common;
use common::*;

fn reassign_test_env(tmp: &TempDir) {
    let source_dir = tmp.path().join("source");
    std::fs::create_dir_all(&source_dir).unwrap();
    create_skill(&source_dir, "my-skill");

    let target_dir = tmp.path().join("target");
    std::fs::create_dir_all(&target_dir).unwrap();

    let library_dir = tmp.path().join("library");
    std::fs::create_dir_all(&library_dir).unwrap();

    let config_content = format!(
        // local-target uses `synced` role rather than `target` — Phase 14
        // D-A2 refuses reassign into target-only directories (a target-only
        // dir doesn't get rediscovered on next sync). `synced` participates
        // in both discovery and distribution, so reassign succeeds.
        "library_dir = \"{}\"\n\n[directories.local-source]\npath = \"{}\"\ntype = \"directory\"\nrole = \"source\"\n\n[directories.local-target]\npath = \"{}\"\ntype = \"directory\"\nrole = \"synced\"\n",
        library_dir.display(),
        source_dir.display(),
        target_dir.display()
    );

    let config_path = tmp.path().join("tome.toml");
    std::fs::write(&config_path, config_content).unwrap();

    // Sync to populate library and manifest
    tome()
        .args([
            "--tome-home",
            tmp.path().to_str().unwrap(),
            "sync",
            "--no-triage",
        ])
        .env("NO_COLOR", "1")
        .assert()
        .success();
}

#[test]
fn test_reassign_happy_path() {
    let tmp = TempDir::new().unwrap();
    reassign_test_env(&tmp);

    tome()
        .args([
            "--tome-home",
            tmp.path().to_str().unwrap(),
            "reassign",
            "my-skill",
            "--to",
            "local-target",
        ])
        .env("NO_COLOR", "1")
        .assert()
        .success()
        .stdout(predicate::str::contains("Reassigned"));
}

#[test]
fn test_reassign_nonexistent_skill() {
    let tmp = TempDir::new().unwrap();
    reassign_test_env(&tmp);

    tome()
        .args([
            "--tome-home",
            tmp.path().to_str().unwrap(),
            "reassign",
            "nonexistent-skill",
            "--to",
            "local-target",
        ])
        .env("NO_COLOR", "1")
        .assert()
        .failure()
        .stderr(predicate::str::contains("not found in library"));
}

#[test]
fn test_reassign_nonexistent_dir() {
    let tmp = TempDir::new().unwrap();
    reassign_test_env(&tmp);

    tome()
        .args([
            "--tome-home",
            tmp.path().to_str().unwrap(),
            "reassign",
            "my-skill",
            "--to",
            "nonexistent-dir",
        ])
        .env("NO_COLOR", "1")
        .assert()
        .failure()
        .stderr(predicate::str::contains("not found in config"));
}

#[test]
fn test_reassign_dry_run() {
    let tmp = TempDir::new().unwrap();
    reassign_test_env(&tmp);

    tome()
        .args([
            "--tome-home",
            tmp.path().to_str().unwrap(),
            "--dry-run",
            "reassign",
            "my-skill",
            "--to",
            "local-target",
        ])
        .env("NO_COLOR", "1")
        .assert()
        .success()
        .stdout(predicate::str::contains("Would"));
}

#[test]
fn test_fork_with_force() {
    let tmp = TempDir::new().unwrap();
    reassign_test_env(&tmp);

    tome()
        .args([
            "--tome-home",
            tmp.path().to_str().unwrap(),
            "fork",
            "my-skill",
            "--to",
            "local-target",
            "--force",
        ])
        .env("NO_COLOR", "1")
        .assert()
        .success()
        .stdout(predicate::str::contains("Forked"));

    // Verify files were actually copied to the target directory
    let target_skill = tmp.path().join("target").join("my-skill").join("SKILL.md");
    assert!(
        target_skill.exists(),
        "forked skill should exist in target directory"
    );
}

#[test]
fn test_fork_no_input_without_force_fails() {
    let tmp = TempDir::new().unwrap();
    reassign_test_env(&tmp);

    tome()
        .args([
            "--tome-home",
            tmp.path().to_str().unwrap(),
            "--no-input",
            "fork",
            "my-skill",
            "--to",
            "local-target",
        ])
        .env("NO_COLOR", "1")
        .assert()
        .failure()
        .stderr(predicate::str::contains("use --force"));
}

#[test]
fn test_fork_dry_run() {
    let tmp = TempDir::new().unwrap();
    reassign_test_env(&tmp);

    tome()
        .args([
            "--tome-home",
            tmp.path().to_str().unwrap(),
            "--dry-run",
            "fork",
            "my-skill",
            "--to",
            "local-target",
            "--force",
        ])
        .env("NO_COLOR", "1")
        .assert()
        .success()
        .stdout(predicate::str::contains("Would"));

    // Target may already have a symlink from sync, but the fork dry run
    // should not have created a regular (non-symlink) directory
    let target_skill = tmp.path().join("target").join("my-skill");
    if target_skill.exists() {
        assert!(
            target_skill.is_symlink(),
            "dry run should not create a real directory copy in target"
        );
    }
}

#[test]
fn phase14_reassign_unowned_input_succeeds() {
    let fix = phase14_build_fixture(
        &[("local-target", "synced")],
        &[],
        &[("orphan-foo", "removed-dir")],
    );

    fix.cmd()
        .args(["reassign", "orphan-foo", "--to", "local-target"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Reassigned"));

    // Manifest: ownership flips from Unowned to Owned("local-target") on
    // re-anchor (#542 SkillOwnership enum on-disk shape; no flat source_name
    // / previous_source keys). The Owned variant carries no breadcrumb, so
    // the D-C1 "previous_source cleared" semantic is structural.
    let manifest = fix.manifest_value();
    let entry = &manifest["skills"]["orphan-foo"];
    assert_eq!(
        entry["ownership"]["kind"].as_str(),
        Some("owned"),
        "ownership.kind must be owned after re-anchor: {entry}"
    );
    assert_eq!(
        entry["ownership"]["source"].as_str(),
        Some("local-target"),
        "ownership.source must be local-target after re-anchor: {entry}"
    );
    assert!(
        entry.get("source_name").is_none() && entry.get("previous_source").is_none(),
        "Owned entry must not carry legacy flat source_name/previous_source keys: {entry}"
    );

    // Skill content materialised in the target directory on disk.
    let target_skill_md = fix.target_dir.unwrap().join("orphan-foo").join("SKILL.md");
    assert!(
        target_skill_md.exists(),
        "skill content must be copied to target dir on re-anchor"
    );
}

#[test]
fn phase14_reassign_into_target_only_role_rejected() {
    let fix = phase14_build_fixture(
        &[("claude-target", "target")],
        &[],
        &[("orphan-foo", "removed-dir")],
    );

    fix.cmd()
        .args(["reassign", "orphan-foo", "--to", "claude-target"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("target-only"));
}

#[test]
fn phase14_reassign_force_bypasses_different_content_collision() {
    let fix = phase14_build_fixture(
        &[("local-target", "synced")],
        &[],
        &[("orphan-foo", "removed-dir")],
    );

    // Pre-populate target dir with a DIFFERENT-content version of the skill
    // so plan() hits the D-A1 hash mismatch path.
    let target = fix.target_dir.clone().unwrap();
    let collision = target.join("orphan-foo");
    std::fs::create_dir_all(&collision).unwrap();
    std::fs::write(
        collision.join("SKILL.md"),
        "---\nname: orphan-foo\n---\n# orphan-foo\nDIFFERENT content at target\n",
    )
    .unwrap();

    // Without --force: refused, exits non-zero, error names "different content".
    fix.cmd()
        .args(["reassign", "orphan-foo", "--to", "local-target"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("different content"));

    // With --force: succeeds; target contents now match the library copy.
    fix.cmd()
        .args(["reassign", "orphan-foo", "--to", "local-target", "--force"])
        .assert()
        .success();

    let target_body = std::fs::read_to_string(collision.join("SKILL.md")).unwrap();
    assert!(
        target_body.contains("unowned skill orphan-foo"),
        "with --force, target content must match the library copy: {target_body}"
    );
}
