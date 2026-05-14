//! Regression test for FIX-06 (#533): `make release` stamps the release
//! date in CHANGELOG.md by replacing `## [Unreleased]` with
//! `## [X.Y.Z] - YYYY-MM-DD`. This test exercises the exact `sed` command
//! the Makefile recipe runs.

use std::fs;
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
    let status = Command::new("sed")
        .args(["-i", "", &sed_expr, changelog.to_str().unwrap()])
        .status()
        .unwrap();
    assert!(status.success(), "sed exited non-zero");

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
    Command::new("sed")
        .args(["-i", "", &sed_expr, changelog.to_str().unwrap()])
        .status()
        .unwrap();
    let first_pass = fs::read_to_string(&changelog).unwrap();

    // Second pass with a DIFFERENT version — must NOT find [Unreleased]
    // to replace (it's already gone), so the file is byte-identical.
    let sed_expr2 = format!("s/^## \\[Unreleased\\]/## [1.0.0] - {date}/");
    let status2 = Command::new("sed")
        .args(["-i", "", &sed_expr2, changelog.to_str().unwrap()])
        .status()
        .unwrap();
    assert!(status2.success(), "second sed exited non-zero");
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
    let status = Command::new("sed")
        .args(["-i", "", &sed_expr, changelog.to_str().unwrap()])
        .status()
        .unwrap();
    assert!(status.success(), "sed exited non-zero on no-match input");

    let content = fs::read_to_string(&changelog).unwrap();
    assert_eq!(
        content, original,
        "file must be unchanged when no [Unreleased] is present"
    );
}
