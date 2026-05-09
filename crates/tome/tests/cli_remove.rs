use assert_fs::TempDir;
use predicates::prelude::*;
use std::path::PathBuf;
use std::process::Command as StdCommand;

mod common;
use common::*;

/// Helper to create a remove-test environment where config is at `tome.toml`.
fn remove_test_env(tmp: &TempDir, directories_toml: &str) -> PathBuf {
    let library_dir = tmp.path().join("library");
    std::fs::create_dir_all(&library_dir).unwrap();
    let config_path = tmp.path().join("tome.toml");
    std::fs::write(
        &config_path,
        format!(
            "library_dir = \"{}\"\n{}",
            library_dir.display(),
            directories_toml,
        ),
    )
    .unwrap();
    config_path
}

#[test]
fn test_remove_nonexistent_directory() {
    let tmp = TempDir::new().unwrap();
    let skills_dir = tmp.path().join("skills");
    create_skill(&skills_dir, "my-skill");

    remove_test_env(
        &tmp,
        &format!(
            "[directories.local]\npath = \"{}\"\ntype = \"directory\"\nrole = \"source\"\n",
            skills_dir.display()
        ),
    );

    tome()
        .args([
            "--tome-home",
            tmp.path().to_str().unwrap(),
            "remove",
            "dir",
            "nonexistent",
            "--force",
        ])
        .env("NO_COLOR", "1")
        .assert()
        .failure()
        .stderr(predicate::str::contains("not found in config"));
}

#[test]
fn test_remove_local_directory() {
    let tmp = TempDir::new().unwrap();
    let skills_dir = tmp.path().join("skills");
    create_skill(&skills_dir, "my-skill");

    let target_dir = tmp.path().join("target");
    std::fs::create_dir_all(&target_dir).unwrap();

    remove_test_env(
        &tmp,
        &format!(
            "[directories.local]\npath = \"{}\"\ntype = \"directory\"\nrole = \"source\"\n\n[directories.test-target]\npath = \"{}\"\ntype = \"directory\"\nrole = \"target\"\n",
            skills_dir.display(),
            target_dir.display()
        ),
    );

    // First sync to populate library and targets
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

    let library_dir = tmp.path().join("library");
    assert!(library_dir.join("my-skill").exists());
    assert!(target_dir.join("my-skill").exists());

    // Remove the source directory
    tome()
        .args([
            "--tome-home",
            tmp.path().to_str().unwrap(),
            "remove",
            "dir",
            "local",
            "--force",
        ])
        .env("NO_COLOR", "1")
        .assert()
        .success();

    // Verify cleanup
    // v0.10 (LIB-04): library content for owned skills is preserved on
    // `tome remove`; the manifest entry transitions to Unowned. Distribution
    // symlinks ARE still removed (the user removed the source from config,
    // not the skill from the library).
    assert!(
        library_dir.join("my-skill").exists(),
        "library skill must be preserved as Unowned per LIB-04"
    );
    assert!(
        !target_dir.join("my-skill").exists(),
        "target symlink should be removed"
    );

    // Verify config no longer has the directory
    let config_content = std::fs::read_to_string(tmp.path().join("tome.toml")).unwrap();
    assert!(
        !config_content.contains("[directories.local]"),
        "config should no longer contain the removed directory"
    );
}

#[test]
fn test_remove_dry_run() {
    let tmp = TempDir::new().unwrap();
    let skills_dir = tmp.path().join("skills");
    create_skill(&skills_dir, "my-skill");

    let target_dir = tmp.path().join("target");
    std::fs::create_dir_all(&target_dir).unwrap();

    remove_test_env(
        &tmp,
        &format!(
            "[directories.local]\npath = \"{}\"\ntype = \"directory\"\nrole = \"source\"\n\n[directories.test-target]\npath = \"{}\"\ntype = \"directory\"\nrole = \"target\"\n",
            skills_dir.display(),
            target_dir.display()
        ),
    );

    // First sync
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

    // Remove with --dry-run
    let output = tome()
        .args([
            "--tome-home",
            tmp.path().to_str().unwrap(),
            "--dry-run",
            "remove",
            "dir",
            "local",
            "--force",
        ])
        .env("NO_COLOR", "1")
        .output()
        .unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("Dry run"),
        "should show dry run message, got: {stdout}"
    );

    // Verify nothing was actually removed
    let library_dir = tmp.path().join("library");
    assert!(
        library_dir.join("my-skill").exists(),
        "library skill should still exist after dry run"
    );
    let config_content = std::fs::read_to_string(tmp.path().join("tome.toml")).unwrap();
    assert!(
        config_content.contains("[directories.local]"),
        "config should still contain the directory after dry run"
    );
}

#[test]
fn test_remove_no_input_without_force_fails() {
    let tmp = TempDir::new().unwrap();
    let skills_dir = tmp.path().join("skills");
    create_skill(&skills_dir, "my-skill");

    remove_test_env(
        &tmp,
        &format!(
            "[directories.local]\npath = \"{}\"\ntype = \"directory\"\nrole = \"source\"\n",
            skills_dir.display()
        ),
    );

    tome()
        .args([
            "--tome-home",
            tmp.path().to_str().unwrap(),
            "--no-input",
            "remove",
            "dir",
            "local",
        ])
        .env("NO_COLOR", "1")
        .assert()
        .failure()
        .stderr(predicate::str::contains("use --force"));
}

#[cfg(unix)]
#[test]
fn remove_partial_failure_exits_nonzero_with_warning_marker() {
    use std::os::unix::fs::PermissionsExt;

    // Fixture: source dir with one skill, target dir (distribution) wired as
    // a target role in config. After sync, the target contains a symlink to
    // the library skill. We then chmod 0o000 the target directory so remove's
    // step-1 loop (distribution symlinks) cannot enumerate / unlink inside.
    let tmp = TempDir::new().unwrap();
    let skills_dir = tmp.path().join("skills");
    create_skill(&skills_dir, "my-skill");

    let target_dir = tmp.path().join("target");
    std::fs::create_dir_all(&target_dir).unwrap();

    remove_test_env(
        &tmp,
        &format!(
            "[directories.local]\npath = \"{}\"\ntype = \"directory\"\nrole = \"source\"\n\n[directories.test-target]\npath = \"{}\"\ntype = \"directory\"\nrole = \"target\"\n",
            skills_dir.display(),
            target_dir.display()
        ),
    );

    // Prime the library + target with a real symlink so the plan has
    // something to try to remove in step 1.
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

    assert!(target_dir.join("my-skill").exists());

    // Clamp the target dir to read+execute only: plan() can still read_dir
    // it to enumerate the symlinks, but execute()'s remove_file call needs
    // write permission on the parent dir and so hits EACCES — landing in
    // the partial-failure path rather than bailing from plan().
    std::fs::set_permissions(&target_dir, std::fs::Permissions::from_mode(0o500)).unwrap();

    let output = tome()
        .args([
            "--tome-home",
            tmp.path().to_str().unwrap(),
            "remove",
            "dir",
            "local",
            "--force",
        ])
        .env("NO_COLOR", "1")
        .output()
        .unwrap();

    // Restore permissions FIRST so TempDir::drop can clean up, BEFORE any
    // assertions (Pitfall 2 from 08-RESEARCH.md).
    std::fs::set_permissions(&target_dir, std::fs::Permissions::from_mode(0o755)).unwrap();

    assert!(
        !output.status.success(),
        "remove should fail on chmod 0o000 target, got status: {:?}",
        output.status,
    );

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("⚠"), "stderr missing ⚠ marker: {stderr}");
    assert!(
        stderr.contains("operations failed"),
        "stderr missing 'operations failed': {stderr}"
    );
    assert!(
        stderr.contains("remove completed with"),
        "stderr missing anyhow error 'remove completed with': {stderr}"
    );
    // I2/I3: user-facing message must mention retry path so they know
    // config/manifest entries survived for a retry attempt.
    assert!(
        stderr.contains("retained") || stderr.contains("retry"),
        "stderr missing retry guidance (I2/I3): {stderr}"
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    // TEST-01 / P1: success banner MUST NOT appear on partial failure.
    // The banner string is "✓ Removed directory" but the leading glyph may
    // be styled with ANSI codes; we assert on "Removed directory" (no glyph)
    // for robustness against console color rendering. NO_COLOR=1 is already
    // set above so the styled `✓` is a literal char, but defending against
    // both forms is defense-in-depth.
    assert!(
        !stdout.contains("Removed directory"),
        "stdout must NOT contain success banner on partial failure; got: {stdout}",
    );
    assert!(
        !stderr.contains("Removed directory"),
        "stderr must NOT contain success banner on partial failure (defense-in-depth); got: {stderr}",
    );
}

#[cfg(unix)]
#[test]
fn remove_partial_failure_does_not_save_disk_state() {
    use std::os::unix::fs::PermissionsExt;

    // HOTFIX-02 / #461 H2: with the save chain reordered, a partial-failure
    // path must NOT mutate config / manifest / lockfile on disk. The user
    // retains a clean retry surface — no half-saved state. The reorder
    // guarantees the early-return ⚠ block fires BEFORE config.save /
    // manifest::save / lockfile::save can run, so on the failure path none
    // of those files are touched.
    let tmp = TempDir::new().unwrap();
    let skills_dir = tmp.path().join("skills");
    create_skill(&skills_dir, "my-skill");

    let target_dir = tmp.path().join("target");
    std::fs::create_dir_all(&target_dir).unwrap();

    remove_test_env(
        &tmp,
        &format!(
            "[directories.local]\npath = \"{}\"\ntype = \"directory\"\nrole = \"source\"\n\n[directories.test-target]\npath = \"{}\"\ntype = \"directory\"\nrole = \"target\"\n",
            skills_dir.display(),
            target_dir.display()
        ),
    );

    // Prime library + target so there's something to remove.
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

    // Snapshot pre-remove disk state. tome.lock may or may not exist
    // depending on what `tome sync --no-triage` writes for a non-git
    // config — tolerate either case (read returns empty Vec on missing,
    // and the byte-equality check still proves "missing-then-missing").
    let config_path = tmp.path().join("tome.toml");
    let manifest_path = tmp.path().join(".tome-manifest.json");
    let lockfile_path = tmp.path().join("tome.lock");

    let config_before = std::fs::read(&config_path).unwrap_or_default();
    let manifest_before = std::fs::read(&manifest_path).unwrap_or_default();
    let lockfile_before = std::fs::read(&lockfile_path).unwrap_or_default();

    // Trigger partial-failure: chmod 0o500 the target dir so step-1 unlink
    // hits EACCES — execute() lands in the partial-failure path with a
    // non-empty `failures` Vec.
    std::fs::set_permissions(&target_dir, std::fs::Permissions::from_mode(0o500)).unwrap();

    let output = tome()
        .args([
            "--tome-home",
            tmp.path().to_str().unwrap(),
            "remove",
            "dir",
            "local",
            "--force",
        ])
        .env("NO_COLOR", "1")
        .output()
        .unwrap();

    // Restore permissions BEFORE assertions so TempDir::drop can clean up.
    std::fs::set_permissions(&target_dir, std::fs::Permissions::from_mode(0o755)).unwrap();

    // 1. CLI exits non-zero.
    assert!(
        !output.status.success(),
        "remove should fail under chmod 0o500, got status: {:?}",
        output.status,
    );

    // 2. Stderr has the ⚠ block (proves the moved block fired BEFORE save).
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("⚠"), "missing ⚠ marker in stderr: {stderr}");
    assert!(
        stderr.contains("operations failed"),
        "missing 'operations failed' in stderr: {stderr}"
    );

    // 3. Disk state is unchanged byte-for-byte (the I2/I3 retention contract
    //    extended to disk: not just in-memory). If the reorder is reverted,
    //    config.save / manifest::save / lockfile::save run BEFORE the early
    //    return and these byte-equality assertions fail.
    let config_after = std::fs::read(&config_path).unwrap_or_default();
    let manifest_after = std::fs::read(&manifest_path).unwrap_or_default();
    let lockfile_after = std::fs::read(&lockfile_path).unwrap_or_default();
    assert_eq!(
        config_before, config_after,
        "tome.toml mutated on partial-failure path (HOTFIX-02 regression)"
    );
    assert_eq!(
        manifest_before, manifest_after,
        ".tome-manifest.json mutated on partial-failure path (HOTFIX-02 regression)"
    );
    assert_eq!(
        lockfile_before, lockfile_after,
        "tome.lock mutated on partial-failure path (HOTFIX-02 regression)"
    );
}

#[cfg(unix)]
#[test]
fn remove_retry_succeeds_after_failure_resolved() {
    use std::os::unix::fs::PermissionsExt;

    // TEST-02 / P2: end-to-end I2/I3 retention contract.
    //   1. Partial failure → config entry + manifest preserved (existing v0.8 contract)
    //   2. User fixes the underlying condition (chmod 0o755)
    //   3. Second `tome remove` succeeds, leaves NO leftover state
    //
    // Without this test, the retry path is only exercised by manual UAT.
    // A future refactor that mutates config/manifest on the failure path
    // (regressing #461 H2) would silently break retry — the second
    // `tome remove` would fail with "directory not found in config".

    let tmp = TempDir::new().unwrap();
    let skills_dir = tmp.path().join("skills");
    create_skill(&skills_dir, "my-skill");

    let target_dir = tmp.path().join("target");
    std::fs::create_dir_all(&target_dir).unwrap();

    remove_test_env(
        &tmp,
        &format!(
            "[directories.local]\npath = \"{}\"\ntype = \"directory\"\nrole = \"source\"\n\n[directories.test-target]\npath = \"{}\"\ntype = \"directory\"\nrole = \"target\"\n",
            skills_dir.display(),
            target_dir.display()
        ),
    );

    // Prime: sync to wire library + target symlink.
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
    assert!(
        target_dir.join("my-skill").exists(),
        "fixture: target symlink must exist after sync"
    );

    // Step 1 — partial failure: chmod 0o500 on target dir.
    std::fs::set_permissions(&target_dir, std::fs::Permissions::from_mode(0o500)).unwrap();

    let first = tome()
        .args([
            "--tome-home",
            tmp.path().to_str().unwrap(),
            "remove",
            "dir",
            "local",
            "--force",
        ])
        .env("NO_COLOR", "1")
        .output()
        .unwrap();
    assert!(
        !first.status.success(),
        "first remove must fail on chmod 0o500"
    );
    let first_stderr = String::from_utf8_lossy(&first.stderr);
    assert!(
        first_stderr.contains("⚠"),
        "first remove stderr missing ⚠ marker: {first_stderr}"
    );

    // Step 1.5 — assert config entry preserved (I2 retention).
    let config_after_fail = std::fs::read_to_string(tmp.path().join("tome.toml")).unwrap();
    assert!(
        config_after_fail.contains("[directories.local]"),
        "config entry for 'local' must be preserved on partial failure; got: {config_after_fail}"
    );

    // Step 2 — user fixes the underlying cause.
    std::fs::set_permissions(&target_dir, std::fs::Permissions::from_mode(0o755)).unwrap();

    // Step 3 — retry: second `tome remove` should succeed cleanly.
    let second = tome()
        .args([
            "--tome-home",
            tmp.path().to_str().unwrap(),
            "remove",
            "dir",
            "local",
            "--force",
        ])
        .env("NO_COLOR", "1")
        .output()
        .unwrap();
    assert!(
        second.status.success(),
        "retry remove must succeed after chmod restore; stderr: {}",
        String::from_utf8_lossy(&second.stderr)
    );
    let second_stdout = String::from_utf8_lossy(&second.stdout);
    assert!(
        second_stdout.contains("Removed directory"),
        "retry stdout must contain success banner; got: {second_stdout}"
    );

    // Step 4 — assert clean state per v0.10 LIB-04 / D-10 trigger 1:
    // - config entry is removed
    // - manifest entry is RETAINED (transitioned to Unowned with source_name omitted)
    // - library dir is RETAINED (preserved as Unowned content)
    let config_after_success = std::fs::read_to_string(tmp.path().join("tome.toml")).unwrap();
    assert!(
        !config_after_success.contains("[directories.local]"),
        "config entry for 'local' must be removed after retry success; got: {config_after_success}"
    );

    let manifest_path = tmp.path().join(".tome-manifest.json");
    assert!(
        manifest_path.exists(),
        "manifest must still exist after retry success"
    );
    let manifest = std::fs::read_to_string(&manifest_path).unwrap();
    assert!(
        manifest.contains("\"my-skill\""),
        "manifest must retain my-skill (Unowned) per LIB-04; got: {manifest}"
    );
    assert!(
        !manifest.contains("\"source_name\":\"local\""),
        "my-skill source_name must be transitioned away from 'local' (skip_serializing_if omits None); got: {manifest}"
    );

    let library_skill = tmp.path().join("library").join("my-skill");
    assert!(
        library_skill.exists(),
        "library dir for my-skill must be preserved as Unowned per LIB-04; missing at {}",
        library_skill.display()
    );
}

#[test]
fn lib_rs_remove_handler_prints_success_banner_before_regen_warnings() {
    // TEST-04 / P4 regression: pin the source-order in lib.rs Command::Remove
    // happy-path. The success banner `println!("Removed directory ...")` MUST
    // appear earlier in the file than the `for w in &regen_warnings ... eprintln!`
    // loop. If a future refactor reorders these, this test fails.
    //
    // ANCHORING: lib.rs contains three `for w in &regen_warnings` loops —
    // one each in Remove, Reassign, Fork handlers. Without anchoring to
    // `Command::Remove` first, a future reorder of Reassign or Fork (or
    // a new handler inserted above Remove with its own regen-warnings
    // loop) could create a false-positive failure unrelated to Remove.
    // We anchor all subsequent searches to `region_start` to keep the
    // test focused on the Remove handler contract.
    //
    // We assert at the source level (file byte-position) rather than at the
    // process-output level because stdout vs stderr ordering is determined
    // by terminal interleaving, not by Rust flush order — assert_cmd captures
    // them as separate streams and gives us no temporal ordering signal.

    let lib_rs_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/lib.rs");
    let lib_rs = std::fs::read_to_string(&lib_rs_path)
        .unwrap_or_else(|e| panic!("lib.rs must exist at {}: {e}", lib_rs_path.display()));

    let region_start = lib_rs
        .find("Command::Remove")
        .expect("lib.rs must contain `Command::Remove` handler");

    let banner_offset = lib_rs[region_start..]
        .find("Removed directory")
        .expect("✓ Removed directory banner must appear inside Command::Remove region");
    let banner_idx = region_start + banner_offset;

    let warnings_offset = lib_rs[region_start..]
        .find("for w in &regen_warnings")
        .expect("regen_warnings loop must appear inside Command::Remove region");
    let warnings_idx = region_start + warnings_offset;

    assert!(
        banner_idx < warnings_idx,
        "TEST-04 option a: `Removed directory` banner (byte {}) MUST precede `for w in &regen_warnings` loop (byte {}) inside the Command::Remove handler region (starts at byte {})",
        banner_idx,
        warnings_idx,
        region_start,
    );
}

#[cfg(unix)]
#[test]
fn remove_failure_summary_wording() {
    use std::os::unix::fs::PermissionsExt;

    // HOTFIX-03 / #461 H3: the leading line of the partial-failure summary
    // must end the colon-introduced clause with `after resolving:` (which
    // introduces the per-kind listing), NOT with `Run `tome doctor`:` (which
    // falsely promised tome doctor output).
    let tmp = TempDir::new().unwrap();
    let skills_dir = tmp.path().join("skills");
    create_skill(&skills_dir, "my-skill");

    let target_dir = tmp.path().join("target");
    std::fs::create_dir_all(&target_dir).unwrap();

    remove_test_env(
        &tmp,
        &format!(
            "[directories.local]\npath = \"{}\"\ntype = \"directory\"\nrole = \"source\"\n\n[directories.test-target]\npath = \"{}\"\ntype = \"directory\"\nrole = \"target\"\n",
            skills_dir.display(),
            target_dir.display()
        ),
    );

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

    std::fs::set_permissions(&target_dir, std::fs::Permissions::from_mode(0o500)).unwrap();

    let output = tome()
        .args([
            "--tome-home",
            tmp.path().to_str().unwrap(),
            "remove",
            "dir",
            "local",
            "--force",
        ])
        .env("NO_COLOR", "1")
        .output()
        .unwrap();

    std::fs::set_permissions(&target_dir, std::fs::Permissions::from_mode(0o755)).unwrap();

    assert!(
        !output.status.success(),
        "remove should fail under chmod 0o500"
    );

    let stderr = String::from_utf8_lossy(&output.stderr);

    // The new wording is present.
    assert!(
        stderr.contains("after resolving:"),
        "stderr missing reworded fragment 'after resolving:': {stderr}"
    );

    // The doctor hint is still surfaced (we kept the call-to-action inline).
    assert!(
        stderr.contains("tome doctor"),
        "stderr missing 'tome doctor' hint: {stderr}"
    );

    // The misleading old wording is gone. With NO_COLOR=1 the styled
    // `tome doctor` is wrapped in backticks but unstyled, so this literal
    // pattern matches reliably.
    assert!(
        !stderr.contains("addressing these. Run `tome doctor`:"),
        "stderr still contains old misleading wording 'addressing these. Run `tome doctor`:': {stderr}"
    );
}

#[cfg(unix)]
#[test]
fn remove_preserves_git_lockfile_entries() {
    // HOTFIX-01 / #461 H1: the regenerated lockfile after `tome remove` must
    // NOT silently drop git-source-name entries. Before the fix, the handler
    // passed an empty BTreeMap to discover_all, which `continue`'d for every
    // git-type directory — wiping their entries from the regenerated lockfile.
    //
    // Fixture: a "real" local git repo (file:// URL) holding one skill plus
    // a separate directory-type "local" source holding another skill. We run
    // `tome sync` to populate the manifest + lockfile from both sources, then
    // run `tome remove local` and assert the regenerated lockfile still
    // contains a `source_name = "myrepo"` entry.
    let tmp = TempDir::new().unwrap();

    // Set up the directory-type "local" source with one skill.
    let skills_dir = tmp.path().join("skills");
    create_skill(&skills_dir, "local-skill");

    // Set up a real local git repo to act as the "myrepo" git directory.
    // Using a file:// URL means `git clone` and `git fetch` work without
    // network access — sync's resolve_git_directories can clone it normally.
    let upstream_dir = tmp.path().join("upstream-myrepo.git");
    std::fs::create_dir_all(&upstream_dir).unwrap();
    create_skill(&upstream_dir, "git-skill");
    // Initialize the upstream as a real git repo so `git clone` accepts it.
    let git_init = |dir: &std::path::Path, args: &[&str]| {
        StdCommand::new("git")
            .args(args)
            .current_dir(dir)
            .env_remove("GIT_DIR")
            .env_remove("GIT_WORK_TREE")
            .env_remove("GIT_INDEX_FILE")
            .output()
            .unwrap();
    };
    // `git init -b main` so the initial branch name is stable across host configs.
    git_init(&upstream_dir, &["init", "-b", "main"]);
    git_init(&upstream_dir, &["config", "user.email", "test@test.com"]);
    git_init(&upstream_dir, &["config", "user.name", "Test"]);
    git_init(&upstream_dir, &["add", "-A"]);
    git_init(&upstream_dir, &["commit", "-m", "seed"]);

    let dummy_url = format!("file://{}", upstream_dir.display());

    // Config: one Directory + one Git directory.
    remove_test_env(
        &tmp,
        &format!(
            "[directories.local]\n\
             path = \"{}\"\n\
             type = \"directory\"\n\
             role = \"source\"\n\
             \n\
             [directories.myrepo]\n\
             path = \"{}\"\n\
             type = \"git\"\n\
             role = \"source\"\n\
             branch = \"main\"\n",
            skills_dir.display(),
            dummy_url,
        ),
    );

    // Sync to populate manifest and lockfile. `--no-triage` avoids the
    // interactive lockfile diff step on an initial sync.
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

    // Sanity: post-sync lockfile must contain a myrepo entry whose
    // `git_commit_sha` is populated. The bug doesn't drop the entry by
    // source_name (that comes from the manifest, which `tome remove` of an
    // unrelated directory does not touch), but it DOES wipe `git_commit_sha`
    // because `discover_all` skips git directories when resolved_paths is
    // empty — so `lockfile::generate` falls back to `(None, None, None)` for
    // provenance. We assert on `git_commit_sha` to actually exercise the bug.
    let lockfile_path = tmp.path().join("tome.lock");
    let lockfile_before: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&lockfile_path).unwrap()).unwrap();
    let myrepo_sha_before: Option<String> = lockfile_before["skills"]
        .as_object()
        .unwrap()
        .values()
        .find(|v| v["source_name"].as_str() == Some("myrepo"))
        .and_then(|v| v["git_commit_sha"].as_str().map(|s| s.to_string()));
    assert!(
        myrepo_sha_before.is_some(),
        "precondition: post-sync lockfile must contain a myrepo entry with \
         git_commit_sha set, got: {lockfile_before}"
    );

    // Now remove the OTHER (directory-type) directory. The regenerated
    // lockfile MUST keep the myrepo entries' provenance — pre-fix the
    // git_commit_sha was silently wiped (skill missing from discover →
    // lockfile::generate falls back to None).
    tome()
        .args([
            "--tome-home",
            tmp.path().to_str().unwrap(),
            "remove",
            "dir",
            "local",
            "--force",
        ])
        .env("NO_COLOR", "1")
        .assert()
        .success();

    let lockfile_after: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&lockfile_path).unwrap()).unwrap();
    let myrepo_sha_after: Option<String> = lockfile_after["skills"]
        .as_object()
        .unwrap()
        .values()
        .find(|v| v["source_name"].as_str() == Some("myrepo"))
        .and_then(|v| v["git_commit_sha"].as_str().map(|s| s.to_string()));
    assert_eq!(
        myrepo_sha_after, myrepo_sha_before,
        "REGRESSION (#461 H1): lockfile after `tome remove local` lost myrepo \
         git_commit_sha provenance — git-sourced skills were silently dropped \
         during regen. Before: {myrepo_sha_before:?}, After: {myrepo_sha_after:?}, \
         full lockfile: {lockfile_after}"
    );
}

#[test]
fn phase14_remove_skill_full_cleanup() {
    let fix = phase14_build_fixture(
        &[("local-target", "synced")],
        &[],
        &[("orphan-foo", "removed-dir")],
    );

    // Stage a distribution symlink pointing at the library skill.
    let library_skill = fix.library_dir.join("orphan-foo");
    let target = fix.target_dir.clone().unwrap();
    let dist_link = target.join("orphan-foo");
    std::os::unix::fs::symlink(&library_skill, &dist_link).unwrap();
    assert!(dist_link.is_symlink());

    // Stage a lockfile entry for the skill.
    let lockfile_path = fix.tome_home.join("tome.lock");
    let lockfile_json = serde_json::json!({
        "version": 1,
        "skills": {
            "orphan-foo": {
                "previous_source": "removed-dir",
                "content_hash": "a".repeat(64),
            }
        }
    });
    std::fs::write(
        &lockfile_path,
        serde_json::to_string_pretty(&lockfile_json).unwrap(),
    )
    .unwrap();

    // Stage machine.toml `disabled` membership.
    std::fs::write(&fix.machine_path, "disabled = [\"orphan-foo\"]\n").unwrap();

    fix.cmd()
        .args(["remove", "skill", "orphan-foo", "--yes"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Forgot skill 'orphan-foo'"));

    // Library directory removed.
    assert!(
        !library_skill.exists(),
        "library/orphan-foo must be removed after `remove skill`"
    );

    // Distribution symlink removed.
    assert!(
        !dist_link.exists() && !dist_link.is_symlink(),
        "distribution symlink must be removed"
    );

    // Manifest entry removed.
    let manifest = fix.manifest_value();
    assert!(
        manifest["skills"].get("orphan-foo").is_none(),
        "manifest entry must be removed: {manifest}"
    );

    // Lockfile entry removed.
    let lockfile_after: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&lockfile_path).unwrap()).unwrap();
    assert!(
        lockfile_after["skills"].get("orphan-foo").is_none(),
        "lockfile entry must be removed: {lockfile_after}"
    );

    // machine.toml disabled membership removed.
    let machine_after = std::fs::read_to_string(&fix.machine_path).unwrap();
    assert!(
        !machine_after.contains("orphan-foo"),
        "machine.toml disabled-set membership must be removed: {machine_after}"
    );
}

#[test]
fn phase14_remove_skill_refuses_owned() {
    let fix = phase14_build_fixture(&[("active-dir", "synced")], &[("kept", "active-dir")], &[]);

    let assert = fix
        .cmd()
        .args(["remove", "skill", "kept", "--yes"])
        .assert()
        .failure();
    let output = assert.get_output();
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("is owned by directory"),
        "stderr must contain 'is owned by directory': {stderr}"
    );
    assert!(
        stderr.contains("tome remove dir"),
        "stderr must hint at `tome remove dir`: {stderr}"
    );

    // Manifest entry preserved (no destructive changes on owned-skill refusal).
    let manifest = fix.manifest_value();
    assert!(
        manifest["skills"].get("kept").is_some(),
        "manifest entry for owned skill must be preserved on refusal: {manifest}"
    );
}

#[test]
fn phase14_remove_skill_no_input_without_yes_bails() {
    let fix = phase14_build_fixture(&[], &[], &[("orphan-foo", "removed-dir")]);

    fix.cmd()
        .args(["--no-input", "remove", "skill", "orphan-foo"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("requires confirmation"));

    // Manifest entry preserved.
    let manifest = fix.manifest_value();
    assert!(
        manifest["skills"].get("orphan-foo").is_some(),
        "manifest entry must be preserved when bail occurred: {manifest}"
    );
}

// ---------------------------------------------------------------------------
// HARD-11: end-to-end coverage for `tome remove dir <name>` across the two
// directory types whose cleanup is non-trivial.
//
// Both tests:
//   1. Drive the binary via assert_cmd::Command::cargo_bin (post-Phase-14
//      D-API-2 `tome remove dir <name>` shape).
//   2. Verify the directory is gone from `tome.toml`.
//   3. Verify type-specific side effects (git cache cleanup, claude-
//      plugins state).
//   4. Verify the LIB-04 / Phase 11 D-10 invariant: skills sourced from
//      the removed directory transition to Unowned (`source_name = None`)
//      with `previous_source` capturing the prior owner; the library
//      directory itself is preserved.
// ---------------------------------------------------------------------------

#[cfg(unix)]
#[test]
fn tome_remove_dir_cleans_git_cache() {
    let tmp = TempDir::new().unwrap();

    // Build a real local git repo to act as the upstream. Using a
    // `file://` URL keeps the test offline.
    let upstream_dir = tmp.path().join("upstream-test-git.git");
    std::fs::create_dir_all(&upstream_dir).unwrap();
    create_skill(&upstream_dir, "git-skill");
    let git_init = |dir: &std::path::Path, args: &[&str]| {
        StdCommand::new("git")
            .args(args)
            .current_dir(dir)
            .env_remove("GIT_DIR")
            .env_remove("GIT_WORK_TREE")
            .env_remove("GIT_INDEX_FILE")
            .output()
            .unwrap();
    };
    git_init(&upstream_dir, &["init", "-b", "main"]);
    git_init(&upstream_dir, &["config", "user.email", "test@test.com"]);
    git_init(&upstream_dir, &["config", "user.name", "Test"]);
    git_init(&upstream_dir, &["add", "-A"]);
    git_init(&upstream_dir, &["commit", "-m", "seed"]);

    let url = format!("file://{}", upstream_dir.display());

    remove_test_env(
        &tmp,
        &format!(
            "[directories.test-git]\n\
             path = \"{}\"\n\
             type = \"git\"\n\
             role = \"source\"\n\
             branch = \"main\"\n",
            url,
        ),
    );

    // Sync to populate library + git cache (~/.tome/repos/<sha>/).
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

    // Sanity: the git cache directory exists and contains at least one
    // entry. Use repos_dir convention: <tome_home>/repos/<sha256(url)>/.
    let repos_dir = tmp.path().join("repos");
    let repos_entries: Vec<_> = std::fs::read_dir(&repos_dir)
        .expect("repos_dir must exist after a git-source sync")
        .filter_map(|e| e.ok())
        .collect();
    assert_eq!(
        repos_entries.len(),
        1,
        "expected one git-cache entry post-sync, got: {repos_entries:?}"
    );
    let cache_dir = repos_entries[0].path();
    assert!(cache_dir.is_dir());

    // Sanity: library_dir contains the discovered skill before removal.
    let library_dir = tmp.path().join("library");
    assert!(
        library_dir.join("git-skill").exists(),
        "library_dir must contain git-skill before remove"
    );

    // Confirm manifest before remove records the directory as the source.
    let manifest_before: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(tmp.path().join(".tome-manifest.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(
        manifest_before["skills"]["git-skill"]["source_name"].as_str(),
        Some("test-git"),
        "precondition: manifest must record source_name = test-git pre-remove, got: {manifest_before}"
    );

    // -- Run: `tome remove dir test-git --force` (non-interactive).
    tome()
        .args([
            "--tome-home",
            tmp.path().to_str().unwrap(),
            "remove",
            "dir",
            "test-git",
            "--force",
        ])
        .env("NO_COLOR", "1")
        .assert()
        .success();

    // -- Verify: tome.toml no longer has the directory entry.
    let config_after = std::fs::read_to_string(tmp.path().join("tome.toml")).unwrap();
    assert!(
        !config_after.contains("[directories.test-git]"),
        "config must no longer contain test-git directory: {config_after}"
    );

    // -- Verify: git cache cleaned.
    assert!(
        !cache_dir.exists(),
        "git cache dir {} must be removed by `tome remove dir`",
        cache_dir.display()
    );

    // -- Verify: library content for the skill is PRESERVED (LIB-04).
    assert!(
        library_dir.join("git-skill").exists(),
        "library skill must be preserved as Unowned per LIB-04"
    );

    // -- Verify: manifest entry transitioned to Unowned (source_name = None,
    //    previous_source captures the prior owner).
    let manifest_after: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(tmp.path().join(".tome-manifest.json")).unwrap(),
    )
    .unwrap();
    assert!(
        manifest_after["skills"]["git-skill"]
            .get("source_name")
            .is_none()
            || manifest_after["skills"]["git-skill"]["source_name"].is_null(),
        "manifest entry for git-skill must transition to Unowned (source_name omitted/null), got: {manifest_after}"
    );
    assert_eq!(
        manifest_after["skills"]["git-skill"]["previous_source"].as_str(),
        Some("test-git"),
        "manifest entry must record previous_source = test-git per Phase 14 D-C1, got: {manifest_after}"
    );
}

/// Build a synthetic claude-plugins directory (v2 `installed_plugins.json`
/// shape with one plugin install dir containing one skill). Returns the
/// source root the caller registers as `[directories.<name>]
/// type = "claude-plugins"`.
fn build_synthetic_claude_plugins(parent: &std::path::Path, plugin: &str, skill: &str) -> PathBuf {
    let cp_root = parent.join("cp-root");
    std::fs::create_dir_all(&cp_root).unwrap();

    let install_dir = cp_root.join("installs").join(plugin);
    let skills_subdir = install_dir.join("skills").join(skill);
    std::fs::create_dir_all(&skills_subdir).unwrap();
    std::fs::write(
        skills_subdir.join("SKILL.md"),
        format!("---\nname: {skill}\n---\n# {skill}\nA managed skill."),
    )
    .unwrap();

    let json = serde_json::json!({
        "version": 2,
        "plugins": {
            "marketplace/foo": [
                {
                    "installPath": install_dir.display().to_string(),
                    "version": "1.0.0",
                }
            ]
        }
    });
    std::fs::write(
        cp_root.join("installed_plugins.json"),
        serde_json::to_string_pretty(&json).unwrap(),
    )
    .unwrap();

    cp_root
}

#[cfg(unix)]
#[test]
fn tome_remove_dir_cleans_claude_plugins() {
    let tmp = TempDir::new().unwrap();

    // Build a synthetic claude-plugins source dir (v2 installed_plugins.json
    // shape) with one managed skill `managed-foo` under plugin
    // `marketplace/foo`. ClaudePlugins discovery picks up the plugin's
    // `skills/` subdir and consolidates the skill into the library.
    let cp_root = build_synthetic_claude_plugins(tmp.path(), "managed-foo", "managed-foo");
    let target_dir = tmp.path().join("target");
    std::fs::create_dir_all(&target_dir).unwrap();

    remove_test_env(
        &tmp,
        &format!(
            "[directories.test-cp]\n\
             path = \"{}\"\n\
             type = \"claude-plugins\"\n\
             \n\
             [directories.dist]\n\
             path = \"{}\"\n\
             type = \"directory\"\n\
             role = \"target\"\n",
            cp_root.display(),
            target_dir.display(),
        ),
    );

    // Sync to populate library + manifest + distribution symlinks.
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

    let library_dir = tmp.path().join("library");
    assert!(
        library_dir.join("managed-foo").exists(),
        "library skill must exist post-sync"
    );

    let dist_link = target_dir.join("managed-foo");
    assert!(
        dist_link.is_symlink(),
        "distribution symlink must exist post-sync: {}",
        dist_link.display()
    );

    let manifest_before: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(tmp.path().join(".tome-manifest.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(
        manifest_before["skills"]["managed-foo"]["source_name"].as_str(),
        Some("test-cp"),
        "precondition: manifest must record source_name = test-cp pre-remove, got: {manifest_before}"
    );

    // -- Run: `tome remove dir test-cp --force` (non-interactive).
    tome()
        .args([
            "--tome-home",
            tmp.path().to_str().unwrap(),
            "remove",
            "dir",
            "test-cp",
            "--force",
        ])
        .env("NO_COLOR", "1")
        .assert()
        .success();

    // -- Verify: tome.toml no longer has the directory entry.
    let config_after = std::fs::read_to_string(tmp.path().join("tome.toml")).unwrap();
    assert!(
        !config_after.contains("[directories.test-cp]"),
        "config must no longer contain test-cp directory: {config_after}"
    );

    // -- Verify: distribution symlinks pointing at the removed source's library
    //    entries are removed.
    assert!(
        !dist_link.exists() && !dist_link.is_symlink(),
        "distribution symlink at {} must be removed post-`remove dir`",
        dist_link.display()
    );

    // -- Verify: library content is PRESERVED (LIB-04).
    assert!(
        library_dir.join("managed-foo").exists(),
        "library skill must be preserved as Unowned per LIB-04"
    );

    // -- Verify: manifest transitioned to Unowned + records previous_source.
    let manifest_after: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(tmp.path().join(".tome-manifest.json")).unwrap(),
    )
    .unwrap();
    assert!(
        manifest_after["skills"]["managed-foo"]
            .get("source_name")
            .is_none()
            || manifest_after["skills"]["managed-foo"]["source_name"].is_null(),
        "manifest must transition managed-foo to Unowned, got: {manifest_after}"
    );
    assert_eq!(
        manifest_after["skills"]["managed-foo"]["previous_source"].as_str(),
        Some("test-cp"),
        "manifest must record previous_source = test-cp per Phase 14 D-C1, got: {manifest_after}"
    );
}
