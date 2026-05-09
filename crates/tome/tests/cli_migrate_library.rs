use std::path::PathBuf;

mod common;

/// Synthetic v0.9 library fixture — exercises the migration boundary defenses.
///
/// Layout produced (per CONTEXT.md <specifics>):
///   tome_home/
///     tome.toml                  ← references local source dir
///     .tome-manifest.json        ← entries for managed + local + broken skills
///     skills/
///       p1/                      ← v0.9-shape symlink → plugin_cache/p1
///       p2/                      ← v0.9-shape symlink → plugin_cache/p2
///       l1/                      ← v0.10-shape real dir copy of local_source/l1
///       broken/                  ← v0.9-shape symlink to /nonexistent (D-04)
///       user-symlink/            ← user-created symlink, NOT in manifest (D-03)
///   plugin_cache/
///     p1/SKILL.md
///     p2/SKILL.md
///   local_source/
///     l1/SKILL.md
#[allow(dead_code)]
struct V09Fixture {
    _root: assert_fs::TempDir, // owns the temp dir; drop = cleanup
    tome_home: PathBuf,
    library_dir: PathBuf,
    plugin_cache: PathBuf,
    local_source: PathBuf,
    config_path: PathBuf,
    machine_path: PathBuf,
}

fn build_v09_fixture() -> V09Fixture {
    use std::os::unix::fs as unix_fs;
    let root = assert_fs::TempDir::new().unwrap();
    let tome_home = root.path().join("tome_home");
    let library_dir = tome_home.join("skills");
    let plugin_cache = root.path().join("plugin_cache");
    let local_source = root.path().join("local_source");
    std::fs::create_dir_all(&library_dir).unwrap();
    std::fs::create_dir_all(&plugin_cache).unwrap();
    std::fs::create_dir_all(&local_source).unwrap();

    // Plugin cache (acts as managed source — claude-plugins style).
    for n in &["p1", "p2"] {
        let d = plugin_cache.join(n);
        std::fs::create_dir_all(&d).unwrap();
        std::fs::write(d.join("SKILL.md"), format!("# {n}")).unwrap();
    }
    // Local source.
    let l1 = local_source.join("l1");
    std::fs::create_dir_all(&l1).unwrap();
    std::fs::write(l1.join("SKILL.md"), "# l1").unwrap();

    // v0.9-shape symlinks for managed skills.
    unix_fs::symlink(plugin_cache.join("p1"), library_dir.join("p1")).unwrap();
    unix_fs::symlink(plugin_cache.join("p2"), library_dir.join("p2")).unwrap();

    // v0.10-shape real-dir copy for local skill (already correct shape).
    let l1_lib = library_dir.join("l1");
    std::fs::create_dir_all(&l1_lib).unwrap();
    std::fs::write(l1_lib.join("SKILL.md"), "# l1").unwrap();

    // D-04: broken symlink (managed manifest entry, target gone).
    unix_fs::symlink("/nonexistent/target", library_dir.join("broken")).unwrap();

    // D-03 conservatism: user-created symlink NOT in manifest.
    let user_target = root.path().join("user_target");
    std::fs::create_dir_all(&user_target).unwrap();
    std::fs::write(user_target.join("SKILL.md"), "# user").unwrap();
    unix_fs::symlink(&user_target, library_dir.join("user-symlink")).unwrap();

    // Compute content_hashes using the production algorithm via the
    // crate-root re-export added in Plan 11-05 Task 0. This guarantees
    // byte-for-byte identity with `manifest::hash_directory` — no risk of
    // a duplicated SHA-256 helper drifting.
    let p1_hash = tome::hash_directory(&plugin_cache.join("p1")).unwrap();
    let p2_hash = tome::hash_directory(&plugin_cache.join("p2")).unwrap();
    let l1_hash = tome::hash_directory(&l1_lib).unwrap();
    let manifest_json = serde_json::json!({
        "skills": {
            "p1": {
                "source_path": plugin_cache.join("p1").to_string_lossy(),
                "source_name": "plugins",
                "content_hash": p1_hash.as_str(),
                "synced_at": "2024-01-01T00:00:00Z",
                "managed": true
            },
            "p2": {
                "source_path": plugin_cache.join("p2").to_string_lossy(),
                "source_name": "plugins",
                "content_hash": p2_hash.as_str(),
                "synced_at": "2024-01-01T00:00:00Z",
                "managed": true
            },
            "broken": {
                "source_path": "/nonexistent/target",
                "source_name": "plugins",
                "content_hash": "0".repeat(64),
                "synced_at": "2024-01-01T00:00:00Z",
                "managed": true
            },
            "l1": {
                "source_path": l1.to_string_lossy(),
                "source_name": "local",
                "content_hash": l1_hash.as_str(),
                "synced_at": "2024-01-01T00:00:00Z",
                "managed": false
            }
        }
    });
    std::fs::write(
        tome_home.join(".tome-manifest.json"),
        serde_json::to_string_pretty(&manifest_json).unwrap(),
    )
    .unwrap();

    // Minimal tome.toml. Declare a local source directory only — for sync's
    // refuse-with-hint test, only valid syntax + a library_dir is needed; the
    // managed entries already in the manifest are what trigger the v0.9 shape
    // detection in `lib.rs::sync` (it reads the manifest, not the config).
    let config_path = tome_home.join("tome.toml");
    let toml = format!(
        r#"library_dir = "{}"

[directories.local]
path = "{}"
type = "directory"
role = "source"
"#,
        library_dir.display(),
        local_source.display(),
    );
    std::fs::write(&config_path, toml).unwrap();

    let machine_path = root.path().join("machine.toml");
    std::fs::write(&machine_path, "").unwrap();

    V09Fixture {
        _root: root,
        tome_home,
        library_dir,
        plugin_cache,
        local_source,
        config_path,
        machine_path,
    }
}

#[test]
fn migrate_library_converts_managed_symlinks_to_real_dirs() {
    let fix = build_v09_fixture();

    let output = assert_cmd::Command::cargo_bin("tome")
        .unwrap()
        .args([
            "migrate-library",
            "--config",
            fix.config_path.to_str().unwrap(),
            "--tome-home",
            fix.tome_home.to_str().unwrap(),
            "--machine",
            fix.machine_path.to_str().unwrap(),
        ])
        .env("NO_COLOR", "1")
        .output()
        .unwrap();
    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);

    // p1 and p2: managed symlinks should now be real directories with copied content.
    for n in &["p1", "p2"] {
        let dest = fix.library_dir.join(n);
        assert!(
            dest.is_dir(),
            "{n} must be a real directory after migration"
        );
        assert!(
            !dest.is_symlink(),
            "{n} must NOT be a symlink after migration"
        );
        assert!(dest.join("SKILL.md").is_file(), "{n}/SKILL.md must exist");
        let content = std::fs::read_to_string(dest.join("SKILL.md")).unwrap();
        assert_eq!(
            content,
            format!("# {n}"),
            "content for {n} must match source"
        );
    }

    // l1: local skill, was already real-dir — UNCHANGED.
    let l1 = fix.library_dir.join("l1");
    assert!(l1.is_dir() && !l1.is_symlink());
    assert_eq!(
        std::fs::read_to_string(l1.join("SKILL.md")).unwrap(),
        "# l1"
    );

    // broken: D-04 — symlink preserved, NOT deleted.
    let broken = fix.library_dir.join("broken");
    assert!(
        broken.is_symlink(),
        "broken symlink must be preserved per D-04, got: {stdout}\n{stderr}"
    );
    // D-04 stderr warning surfaced.
    assert!(
        stderr.contains("broken") && stderr.contains("unreachable"),
        "stderr must mention broken-source skip, got: {stderr}"
    );

    // user-symlink: D-03 conservatism — NOT in manifest, must be untouched.
    let user_sym = fix.library_dir.join("user-symlink");
    assert!(
        user_sym.is_symlink(),
        "user-created symlink (NOT in manifest) must be preserved per D-03"
    );

    // D-05: exit code non-zero because of the broken-symlink skip.
    assert!(
        !output.status.success(),
        "must exit non-zero on broken-symlink skip per D-05"
    );

    // SAFE-01 banner format check.
    let combined = format!("{stdout}{stderr}");
    assert!(
        combined.contains("converted") && combined.contains("skipped"),
        "output must include SAFE-01 summary banner, got: {combined}"
    );

    // Silence dead-code warnings on unused fixture fields.
    let _ = (&fix.plugin_cache, &fix.local_source);
}

#[test]
fn migrate_library_dry_run_makes_no_changes() {
    let fix = build_v09_fixture();

    // Snapshot library state pre-run.
    let p1_was_symlink = fix.library_dir.join("p1").is_symlink();
    let p2_was_symlink = fix.library_dir.join("p2").is_symlink();
    assert!(p1_was_symlink && p2_was_symlink, "fixture sanity");

    let output = assert_cmd::Command::cargo_bin("tome")
        .unwrap()
        .args([
            "migrate-library",
            "--dry-run",
            "--config",
            fix.config_path.to_str().unwrap(),
            "--tome-home",
            fix.tome_home.to_str().unwrap(),
            "--machine",
            fix.machine_path.to_str().unwrap(),
        ])
        .env("NO_COLOR", "1")
        .output()
        .unwrap();

    // Filesystem unchanged.
    assert!(
        fix.library_dir.join("p1").is_symlink(),
        "dry-run must not convert p1"
    );
    assert!(
        fix.library_dir.join("p2").is_symlink(),
        "dry-run must not convert p2"
    );
    assert!(
        fix.library_dir.join("broken").is_symlink(),
        "dry-run must not touch broken"
    );

    // Output should mention dry-run.
    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        combined.contains("dry-run"),
        "output must mention dry-run, got: {combined}"
    );
}

#[test]
fn sync_refuses_on_v09_shape_library_with_hint() {
    let fix = build_v09_fixture();

    let output = assert_cmd::Command::cargo_bin("tome")
        .unwrap()
        .args([
            "sync",
            "--no-input",
            "--config",
            fix.config_path.to_str().unwrap(),
            "--tome-home",
            fix.tome_home.to_str().unwrap(),
            "--machine",
            fix.machine_path.to_str().unwrap(),
        ])
        .env("NO_COLOR", "1")
        .output()
        .unwrap();
    let stderr = String::from_utf8_lossy(&output.stderr);

    // D-02: sync must refuse with a Conflict/Why/Suggestion error.
    assert!(
        !output.status.success(),
        "sync must exit non-zero on v0.9-shape library"
    );
    assert!(
        stderr.contains("v0.9 shape"),
        "stderr must mention 'v0.9 shape': {stderr}"
    );
    assert!(
        stderr.contains("tome migrate-library"),
        "stderr must point at `tome migrate-library`: {stderr}"
    );

    // Library must NOT have been modified by the refused sync.
    assert!(
        fix.library_dir.join("p1").is_symlink(),
        "refused sync must not modify library"
    );
    assert!(fix.library_dir.join("p2").is_symlink());
}

#[test]
fn sync_succeeds_after_migrate_library() {
    let fix = build_v09_fixture();

    // Remove the broken symlink first so migrate-library exits cleanly
    // (otherwise the broken-symlink D-04 path would block this test from
    // reaching the post-migration sync).
    std::fs::remove_file(fix.library_dir.join("broken")).unwrap();

    // Drop the broken manifest entry too — otherwise sync's v0.9-shape
    // detection would still fire (`broken` would still be in the manifest
    // with managed=true and no library entry).
    let manifest_path = fix.tome_home.join(".tome-manifest.json");
    let mut manifest: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&manifest_path).unwrap()).unwrap();
    manifest["skills"].as_object_mut().unwrap().remove("broken");
    std::fs::write(
        &manifest_path,
        serde_json::to_string_pretty(&manifest).unwrap(),
    )
    .unwrap();

    // Step 1: migrate-library.
    let migrate = assert_cmd::Command::cargo_bin("tome")
        .unwrap()
        .args([
            "migrate-library",
            "--config",
            fix.config_path.to_str().unwrap(),
            "--tome-home",
            fix.tome_home.to_str().unwrap(),
            "--machine",
            fix.machine_path.to_str().unwrap(),
        ])
        .env("NO_COLOR", "1")
        .output()
        .unwrap();
    assert!(
        migrate.status.success(),
        "migrate-library must succeed cleanly when no broken symlinks remain.\nstdout: {}\nstderr: {}",
        String::from_utf8_lossy(&migrate.stdout),
        String::from_utf8_lossy(&migrate.stderr),
    );

    // Step 2: sync. Should NOT refuse anymore (no v0.9-shape symlinks left).
    let sync = assert_cmd::Command::cargo_bin("tome")
        .unwrap()
        .args([
            "sync",
            "--no-input",
            "--config",
            fix.config_path.to_str().unwrap(),
            "--tome-home",
            fix.tome_home.to_str().unwrap(),
            "--machine",
            fix.machine_path.to_str().unwrap(),
        ])
        .env("NO_COLOR", "1")
        .output()
        .unwrap();

    let sync_stderr = String::from_utf8_lossy(&sync.stderr);
    assert!(
        !sync_stderr.contains("v0.9 shape"),
        "sync after migrate must NOT refuse with v0.9 hint, got: {sync_stderr}"
    );
    // We don't assert sync.status.success() because the synthetic fixture
    // doesn't have a real claude-plugins source so the managed entries
    // become orphans — but that's a Phase 13 concern. The KEY assertion
    // is that the v0.9 refuse-with-hint check no longer fires.
}

#[test]
fn sync_preserves_library_when_source_removed_from_config() {
    // LIB-04 / D-09 Case 1 / D-10 trigger 2: user edits tome.toml outside
    // `tome remove` to drop a source directory; the next `tome sync` cleanup
    // phase must transition the orphaned manifest entries to Unowned and
    // preserve their library content (NOT delete).
    //
    // Note on config shape: `lib.rs::sync` has a CFG-06 safety guard that
    // returns early ("no directories configured") if `config.directories`
    // is empty — this would skip cleanup entirely. To exercise the cleanup
    // path we keep ONE source in config (`other`, with no skills) and remove
    // the one that owned the orphan (`local`). Manifest entry for `alpha`
    // still references `local`, which is no longer in `config.directories` —
    // the exact D-09 Case 1 trigger.
    let root = assert_fs::TempDir::new().unwrap();
    let tome_home = root.path().join("tome_home");
    let library_dir = tome_home.join("skills");
    let local_source = root.path().join("local_source");
    let other_source = root.path().join("other_source");
    std::fs::create_dir_all(&library_dir).unwrap();
    std::fs::create_dir_all(&local_source).unwrap();
    std::fs::create_dir_all(&other_source).unwrap();

    // Create a real skill in `other` so sync's `skills.is_empty()` early-exit
    // doesn't fire (cleanup only runs after discover finds at least one skill).
    let other_skill = other_source.join("beta");
    std::fs::create_dir_all(&other_skill).unwrap();
    std::fs::write(
        other_skill.join("SKILL.md"),
        "---\nname: beta\n---\n# beta\nA filler skill so sync proceeds past discover.",
    )
    .unwrap();

    // Create a local skill in source and pre-populate library + manifest.
    let src = local_source.join("alpha");
    std::fs::create_dir_all(&src).unwrap();
    std::fs::write(src.join("SKILL.md"), "# alpha").unwrap();

    let lib_alpha = library_dir.join("alpha");
    std::fs::create_dir_all(&lib_alpha).unwrap();
    std::fs::write(lib_alpha.join("SKILL.md"), "# alpha").unwrap();

    // Use the production hash function via the crate-root re-export (Task 0).
    let alpha_hash = tome::hash_directory(&lib_alpha).unwrap();
    let manifest_json = serde_json::json!({
        "skills": {
            "alpha": {
                "source_path": src.to_string_lossy(),
                "source_name": "local",
                "content_hash": alpha_hash.as_str(),
                "synced_at": "2024-01-01T00:00:00Z",
                "managed": false
            }
        }
    });
    std::fs::write(
        tome_home.join(".tome-manifest.json"),
        serde_json::to_string_pretty(&manifest_json).unwrap(),
    )
    .unwrap();

    // Final config: drops `[directories.local]`, keeps `[directories.other]`.
    // The `local` entry was the previous owner of `alpha`; with `local` gone,
    // the cleanup phase classifies `alpha` as a Case 1 orphan (source no
    // longer in config) → transition to Unowned + preserve library content.
    let config_path = tome_home.join("tome.toml");
    let machine_path = root.path().join("machine.toml");
    std::fs::write(&machine_path, "").unwrap();
    let config_without_source = format!(
        r#"library_dir = "{}"

[directories.other]
path = "{}"
type = "directory"
role = "source"
"#,
        library_dir.display(),
        other_source.display(),
    );
    std::fs::write(&config_path, &config_without_source).unwrap();

    // Step 2: run sync. Cleanup phase should detect the orphan (alpha's
    // source_name "local" is no longer in config.directories) and
    // transition it to Unowned — preserving the library content per LIB-04.
    let sync = assert_cmd::Command::cargo_bin("tome")
        .unwrap()
        .args([
            "sync",
            "--no-input",
            "--config",
            config_path.to_str().unwrap(),
            "--tome-home",
            tome_home.to_str().unwrap(),
            "--machine",
            machine_path.to_str().unwrap(),
        ])
        .env("NO_COLOR", "1")
        .output()
        .unwrap();
    let sync_stderr = String::from_utf8_lossy(&sync.stderr);
    let sync_stdout = String::from_utf8_lossy(&sync.stdout);

    // The library directory MUST still exist with the same content.
    assert!(
        library_dir.join("alpha").is_dir(),
        "LIB-04: library content must be preserved on source removal.\nstdout: {sync_stdout}\nstderr: {sync_stderr}"
    );
    let preserved = std::fs::read_to_string(library_dir.join("alpha/SKILL.md")).unwrap();
    assert_eq!(preserved, "# alpha", "library content must be unchanged");

    // The manifest entry must have transitioned to Unowned (source_name omitted/null).
    let manifest_after: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(tome_home.join(".tome-manifest.json")).unwrap(),
    )
    .unwrap();
    let alpha_entry = &manifest_after["skills"]["alpha"];
    assert!(
        alpha_entry
            .get("source_name")
            .map(|v| v.is_null())
            .unwrap_or(true),
        "manifest entry's source_name must be omitted or null after source removal: {alpha_entry}"
    );
    // content_hash unchanged.
    assert_eq!(
        alpha_entry["content_hash"].as_str().unwrap(),
        alpha_hash.as_str(),
        "content_hash must remain unchanged across the Case 1 transition"
    );
}
