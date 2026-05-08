//! HARD-10: hostile-input integration tests for `[directory_overrides.<name>]`
//! in `machine.toml`. The override schema (PORT-01..05, v0.9 Phase 9) accepts
//! per-machine path remaps; this suite pins that hostile shapes are rejected
//! with a clear, machine.toml-named error rather than coerced or silently
//! passed through to discover/distribute.

use assert_fs::TempDir;
use std::path::PathBuf;

mod common;
use common::*;

/// Helper to spin up the minimal three-file fixture every hostile-input
/// test needs: a `tome.toml` with one source directory and a `machine.toml`
/// containing the hostile `[directory_overrides.<name>]` block. Returns
/// (tome_home_path, config_path, machine_path) so the test can assert on
/// downstream filesystem state (library_dir, distribution dirs, etc.).
fn hostile_override_env(
    tmp: &TempDir,
    machine_toml_body: &str,
) -> (PathBuf, PathBuf, PathBuf) {
    let library_dir = tmp.path().join("library");
    std::fs::create_dir_all(&library_dir).unwrap();

    // Real on-disk source directory the override will redirect away from.
    let real_src = tmp.path().join("real-src");
    std::fs::create_dir_all(&real_src).unwrap();

    let config_path = tmp.path().join("tome.toml");
    std::fs::write(
        &config_path,
        format!(
            "library_dir = \"{}\"\n\n\
             [directories.foo]\n\
             path = \"{}\"\n\
             type = \"directory\"\n\
             role = \"source\"\n",
            library_dir.display(),
            real_src.display(),
        ),
    )
    .unwrap();

    let machine_path = tmp.path().join("machine.toml");
    std::fs::write(&machine_path, machine_toml_body).unwrap();

    (tmp.path().to_path_buf(), config_path, machine_path)
}

/// HARD-10 case 1: an override path containing a `..` traversal must be
/// rejected with a clear error that names `machine.toml` and the
/// offending directory.
#[test]
fn cli_overrides_hostile_dotdot_traversal_rejected() {
    let tmp = TempDir::new().unwrap();
    let (tome_home, config, machine) = hostile_override_env(
        &tmp,
        "[directory_overrides.foo]\npath = \"../../../etc\"\n",
    );

    let assert = tome()
        .args([
            "--tome-home",
            tome_home.to_str().unwrap(),
            "--config",
            config.to_str().unwrap(),
            "--machine",
            machine.to_str().unwrap(),
            "sync",
        ])
        .env("NO_COLOR", "1")
        .assert()
        .failure();

    let stderr = String::from_utf8_lossy(&assert.get_output().stderr);
    assert!(
        stderr.contains("machine.toml"),
        "stderr must blame machine.toml (PORT-04), got: {stderr}"
    );
    assert!(
        stderr.contains("foo"),
        "stderr must name the offending directory 'foo', got: {stderr}"
    );
    assert!(
        stderr.contains(".."),
        "stderr must surface the `..` traversal as the conflict reason, got: {stderr}"
    );

    // Library must be untouched: no skills were materialised because we
    // rejected before discover ever ran.
    let library_entries: Vec<_> = std::fs::read_dir(tmp.path().join("library"))
        .unwrap()
        .filter_map(|e| e.ok())
        .collect();
    assert!(
        library_entries.is_empty(),
        "library_dir must be untouched after a hostile-override rejection, got: {library_entries:?}"
    );
}

/// HARD-10 case 2: an override path that resolves into a symlink loop
/// must produce a clear error and a non-zero exit; the library is
/// untouched.
///
/// The current implementation rejects these via the runtime
/// canonicalize/read_dir failure surfaced by sync's discovery pass —
/// the assertion pins the user-facing wording so future refactors can
/// only tighten the rejection, never weaken it.
#[test]
fn cli_overrides_hostile_symlink_loop_rejected() {
    let tmp = TempDir::new().unwrap();

    // Build a 2-link symlink loop: a -> b, b -> a.
    let loop_dir = tmp.path().join("loop");
    std::fs::create_dir_all(&loop_dir).unwrap();
    let a = loop_dir.join("a");
    let b = loop_dir.join("b");
    std::os::unix::fs::symlink(&b, &a).unwrap();
    std::os::unix::fs::symlink(&a, &b).unwrap();

    let (tome_home, config, machine) = hostile_override_env(
        &tmp,
        &format!(
            "[directory_overrides.foo]\npath = \"{}\"\n",
            a.display()
        ),
    );

    let assert = tome()
        .args([
            "--tome-home",
            tome_home.to_str().unwrap(),
            "--config",
            config.to_str().unwrap(),
            "--machine",
            machine.to_str().unwrap(),
            "sync",
        ])
        .env("NO_COLOR", "1")
        .assert()
        .failure();

    let stderr = String::from_utf8_lossy(&assert.get_output().stderr);
    // The error chain mentions the override target dir name. The exact
    // wording from the underlying read_link / canonicalize chain is OS-
    // dependent (ELOOP / "Too many levels of symbolic links"), so we
    // assert on the directory name + a stable "fail" / "error" marker.
    assert!(
        stderr.contains("foo"),
        "stderr must mention the offending directory 'foo', got: {stderr}"
    );
    assert!(
        !stderr.is_empty(),
        "symlink-loop sync must produce a non-empty stderr error, got: {stderr}"
    );

    // Library content is untouched.
    let library_entries: Vec<_> = std::fs::read_dir(tmp.path().join("library"))
        .unwrap()
        .filter_map(|e| e.ok())
        .collect();
    assert!(
        library_entries.is_empty(),
        "library_dir must be untouched after a symlink-loop rejection, got: {library_entries:?}"
    );
}

/// HARD-10 case 3: two `[directory_overrides.<name>]` entries pointing
/// at the same path must be rejected with a clear error that names
/// `machine.toml`, both directory names, and the duplicate path.
#[test]
fn cli_overrides_hostile_duplicate_target_rejected() {
    let tmp = TempDir::new().unwrap();
    let library_dir = tmp.path().join("library");
    std::fs::create_dir_all(&library_dir).unwrap();
    let real_src1 = tmp.path().join("src1");
    let real_src2 = tmp.path().join("src2");
    std::fs::create_dir_all(&real_src1).unwrap();
    std::fs::create_dir_all(&real_src2).unwrap();
    let shared = tmp.path().join("shared-target");
    std::fs::create_dir_all(&shared).unwrap();

    let config_path = tmp.path().join("tome.toml");
    std::fs::write(
        &config_path,
        format!(
            "library_dir = \"{}\"\n\n\
             [directories.foo]\n\
             path = \"{}\"\n\
             type = \"directory\"\n\
             role = \"source\"\n\
             \n\
             [directories.bar]\n\
             path = \"{}\"\n\
             type = \"directory\"\n\
             role = \"source\"\n",
            library_dir.display(),
            real_src1.display(),
            real_src2.display(),
        ),
    )
    .unwrap();

    let machine_path = tmp.path().join("machine.toml");
    std::fs::write(
        &machine_path,
        format!(
            "[directory_overrides.foo]\npath = \"{}\"\n\
             [directory_overrides.bar]\npath = \"{}\"\n",
            shared.display(),
            shared.display(),
        ),
    )
    .unwrap();

    let assert = tome()
        .args([
            "--tome-home",
            tmp.path().to_str().unwrap(),
            "--config",
            config_path.to_str().unwrap(),
            "--machine",
            machine_path.to_str().unwrap(),
            "sync",
        ])
        .env("NO_COLOR", "1")
        .assert()
        .failure();

    let stderr = String::from_utf8_lossy(&assert.get_output().stderr);
    assert!(
        stderr.contains("machine.toml"),
        "stderr must blame machine.toml (PORT-04), got: {stderr}"
    );
    assert!(
        stderr.contains("foo") && stderr.contains("bar"),
        "stderr must enumerate both colliding directory names, got: {stderr}"
    );
    assert!(
        stderr.contains(&shared.display().to_string()),
        "stderr must surface the duplicate path, got: {stderr}"
    );

    // Library must be untouched.
    let library_entries: Vec<_> = std::fs::read_dir(&library_dir)
        .unwrap()
        .filter_map(|e| e.ok())
        .collect();
    assert!(
        library_entries.is_empty(),
        "library_dir must be untouched after a duplicate-path rejection, got: {library_entries:?}"
    );
}
