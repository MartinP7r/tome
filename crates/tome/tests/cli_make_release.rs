//! Regression test for FIX-06 (#533): `make release` stamps the release
//! date in CHANGELOG.md by replacing `## [Unreleased]` with
//! `## [X.Y.Z] - YYYY-MM-DD`. This test exercises the exact `sed` command
//! the Makefile recipe runs.
//!
//! Cross-platform note: the original tests used `sed -i ''` (BSD-only form)
//! which broke CI on Linux ("can't read s/...: No such file or directory"
//! — GNU sed interprets the empty `''` as a filename). The Makefile and
//! these tests both now use `sed -i.bak ... && rm -f file.bak`, which
//! works on both BSD (macOS) and GNU (Linux) sed.

use std::fs;
use std::path::Path;
use std::process::Command;
use tempfile::TempDir;

/// Returns today's UTC date as `YYYY-MM-DD` via `date -u +%Y-%m-%d` —
/// the same command the Makefile recipe uses. Portable across BSD `date`
/// (macOS) and GNU `date` (Linux).
fn today_utc() -> String {
    let out = Command::new("date")
        .args(["-u", "+%Y-%m-%d"])
        .output()
        .expect("invoke `date`");
    String::from_utf8(out.stdout)
        .expect("date output is UTF-8")
        .trim()
        .to_string()
}

/// Runs `sed -i.bak <expr> <file>` and deletes the resulting `.bak` file.
/// `-i.bak` is the BSD/GNU-portable form of in-place edit; the explicit
/// suffix is required by BSD and accepted by GNU.
fn sed_in_place(expr: &str, file: &Path) {
    let status = Command::new("sed")
        .args(["-i.bak", expr, file.to_str().unwrap()])
        .status()
        .unwrap();
    assert!(status.success(), "sed exited non-zero");
    let bak = file.with_extension(format!(
        "{}.bak",
        file.extension().unwrap().to_str().unwrap()
    ));
    let _ = fs::remove_file(&bak);
}

#[test]
fn make_release_sed_replaces_unreleased_section() {
    let tmp = TempDir::new().unwrap();
    let changelog = tmp.path().join("CHANGELOG.md");
    fs::write(
        &changelog,
        "## [Unreleased]\n\n### Added\n- foo\n\n## [0.10.0] - 2026-05-11\n",
    )
    .unwrap();

    let date = today_utc();
    let sed_expr = format!("s/^## \\[Unreleased\\]/## [0.99.0] - {date}/");
    sed_in_place(&sed_expr, &changelog);

    let content = fs::read_to_string(&changelog).unwrap();
    let expected_line = format!("## [0.99.0] - {date}");
    assert!(
        content.contains(&expected_line),
        "sed did not replace [Unreleased]; got:\n{content}"
    );
    assert!(
        !content.contains("## [Unreleased]"),
        "[Unreleased] line still present after sed; got:\n{content}"
    );
}

#[test]
fn make_release_sed_is_idempotent() {
    let tmp = TempDir::new().unwrap();
    let changelog = tmp.path().join("CHANGELOG.md");
    fs::write(&changelog, "## [Unreleased]\n\n### Added\n- foo\n").unwrap();

    let date = today_utc();
    let sed_expr = format!("s/^## \\[Unreleased\\]/## [0.99.0] - {date}/");
    sed_in_place(&sed_expr, &changelog);
    let first_pass = fs::read_to_string(&changelog).unwrap();

    // Second pass with a DIFFERENT version — must NOT find [Unreleased]
    // to replace (it's already gone), so the file is byte-identical.
    let sed_expr2 = format!("s/^## \\[Unreleased\\]/## [1.0.0] - {date}/");
    sed_in_place(&sed_expr2, &changelog);
    let second_pass = fs::read_to_string(&changelog).unwrap();
    assert_eq!(
        first_pass, second_pass,
        "sed must be idempotent (no [Unreleased] left after first run)"
    );
}

#[test]
fn make_release_sed_silent_noop_when_no_unreleased_section() {
    let tmp = TempDir::new().unwrap();
    let changelog = tmp.path().join("CHANGELOG.md");
    let original = "## [0.10.0] - 2026-05-11\n\n### Added\n- foo\n";
    fs::write(&changelog, original).unwrap();

    let date = today_utc();
    let sed_expr = format!("s/^## \\[Unreleased\\]/## [0.99.0] - {date}/");
    sed_in_place(&sed_expr, &changelog);

    let content = fs::read_to_string(&changelog).unwrap();
    assert_eq!(
        content, original,
        "file must be unchanged when no [Unreleased] is present"
    );
}
