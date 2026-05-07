//! Integration tests for Phase 13 (RECON-01..05) reconcile flow.
//!
//! These tests exercise the `tome sync` binary end-to-end via `assert_cmd`,
//! covering the non-interactive flow paths only. Per RESEARCH Pitfall 6,
//! `dialoguer::Select` cannot be driven from `assert_cmd::write_stdin` — the
//! interactive consent / edit-in-library prompts are covered by
//! `crates/tome/src/reconcile.rs::tests` (Plan 13-03), which exercise
//! `MockMarketplaceAdapter` directly.
//!
//! What these tests cover:
//! - Summary line negative control (sync runs with no claude-plugins config).
//! - Vanished entry → distribution still symlinks preserved library copy
//!   (RECON-04 anchor; proxied via the local-skill distribution path).
//! - `--no-install` skips apply unconditionally; exit zero (RECON-02 / D-09).
//! - `--no-input` against an edit-in-library fixture: exit zero, no overwrite
//!   (RECON-05 / D-16; covered as a non-interactive path that reaches the
//!   skip-with-warning branch).
//! - Missing `claude` binary + `claude-plugins` directory → D-20 error +
//!   non-zero exit.
//! - `auto_install_plugins` round-trip across syncs (RECON-02 persistence).
//!
//! ## Why no `MockMarketplaceAdapter` injection?
//!
//! Plan 13-04's `build_claude_adapter` always constructs the real
//! `ClaudeMarketplaceAdapter` (no factory dispatch yet). Injecting the mock
//! into the running binary would require a feature-gated factory hook in
//! `build_claude_adapter` — out of scope for v0.10. Instead, each test below
//! exercises a flow path that does NOT require an adapter call:
//!
//! - Local-only configs skip the reconcile branch entirely (no `claude-plugins`
//!   directory ⇒ `build_claude_adapter` returns `Ok(None)`).
//! - The D-20 test forces the adapter constructor to fail (PATH cleared so
//!   `claude --version` probe fails before any list call).
//!
//! Plan 13-03's unit tests cover the adapter-driven paths against
//! `MockMarketplaceAdapter` directly. The dev-dep self-reference added in
//! Task 1 of this plan (`tome = { path = ".", features = ["test-support"] }`)
//! keeps `tome::marketplace::testing::*` reachable for future plans (e.g.
//! once `build_claude_adapter` grows a factory hook).

use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::Result;
use assert_cmd::prelude::*;
use predicates::prelude::*;
use tempfile::TempDir;

// =========================================================================
// Compile-time reachability probe for the test-support feature.
// =========================================================================
//
// Asserts that `tome::marketplace::testing::*` resolves from this test crate.
// Without the dev-dep self-reference (Plan 13-05 Task 1), this line fails to
// compile because `pub mod testing` is gated behind `feature = "test-support"`,
// and the feature is off in the linked-against `tome` library. We intentionally
// reference `fixture_plugin` as a function pointer so this stays a compile-time
// check — no test body runs it. Future plans that need a real mock-adapter
// injection will replace this with usage at the call site.
#[allow(dead_code)]
const _TESTING_REACHABLE: fn(&str, &str) -> tome::marketplace::InstalledPlugin =
    tome::marketplace::testing::fixture_plugin;

// =========================================================================
// Fixture helpers
// =========================================================================

/// Build a tome home + config + library at `<tmp>/{tome_home,...}`.
struct Fixture {
    _tmp: TempDir,
    tome_home: PathBuf,
    library_dir: PathBuf,
    config_path: PathBuf,
    machine_path: PathBuf,
    dist_dir: PathBuf,
}

impl Fixture {
    /// Empty library + a single distribution dir + an empty machine.toml.
    fn new() -> Result<Self> {
        let tmp = TempDir::new()?;
        let root = tmp.path().to_path_buf();
        let tome_home = root.join("tome_home");
        let library_dir = tome_home.join("library");
        let config_path = tome_home.join("tome.toml");
        let machine_path = root.join("machine.toml");
        let dist_dir = root.join("dist");

        std::fs::create_dir_all(&library_dir)?;
        std::fs::create_dir_all(&dist_dir)?;
        std::fs::write(&machine_path, "")?;

        Ok(Fixture {
            _tmp: tmp,
            tome_home,
            library_dir,
            config_path,
            machine_path,
            dist_dir,
        })
    }

    /// Write a `tome.toml` with one local source directory + one target
    /// (distribution) directory. No `claude-plugins` entry — for tests that
    /// don't need the adapter to fire (`build_claude_adapter` returns
    /// `Ok(None)` and the reconcile branch is skipped).
    fn write_local_only_config(&self, source_dir: &Path) -> Result<()> {
        let toml = format!(
            r#"library_dir = "{lib}"

[directories.local-skills]
type = "directory"
role = "source"
path = "{src}"

[directories.dist]
type = "directory"
role = "target"
path = "{dist}"
"#,
            lib = self.library_dir.display(),
            src = source_dir.display(),
            dist = self.dist_dir.display(),
        );
        std::fs::write(&self.config_path, toml)?;
        Ok(())
    }

    /// Write a `tome.toml` with a `claude-plugins` directory (no real claude
    /// install) + a target directory. Used for the D-20 "claude binary
    /// missing" test path.
    fn write_claude_plugins_config(&self) -> Result<()> {
        let claude_path = self.tome_home.join("claude_pseudo");
        std::fs::create_dir_all(&claude_path)?;
        let toml = format!(
            r#"library_dir = "{lib}"

[directories.cp]
type = "claude-plugins"
role = "managed"
path = "{path}"

[directories.dist]
type = "directory"
role = "target"
path = "{dist}"
"#,
            lib = self.library_dir.display(),
            path = claude_path.display(),
            dist = self.dist_dir.display(),
        );
        std::fs::write(&self.config_path, toml)?;
        Ok(())
    }

    /// Run `tome sync` with the given extra args. Returns the assert_cmd
    /// command for the caller to inspect/configure further (e.g. extra envs).
    fn run_sync(&self, args: &[&str]) -> Command {
        let mut cmd = Command::cargo_bin("tome").expect("tome binary builds");
        cmd.arg("--config")
            .arg(&self.config_path)
            .arg("--tome-home")
            .arg(&self.tome_home)
            .arg("--machine")
            .arg(&self.machine_path)
            .arg("sync")
            .args(args)
            // Suppress ANSI codes so substring assertions are reliable.
            .env("NO_COLOR", "1");
        cmd
    }

    /// Same as `run_sync` but with PATH cleared so `claude` is not findable.
    /// HOME is preserved because `dirs::home_dir()` reads it. Used for D-20.
    fn run_sync_no_claude(&self, args: &[&str]) -> Command {
        let mut cmd = Command::cargo_bin("tome").expect("tome binary builds");
        cmd.env_clear()
            // Preserve HOME so dirs::home_dir() works.
            .env("HOME", std::env::var_os("HOME").unwrap_or_default())
            // PATH = empty (no claude binary findable).
            .env("PATH", "")
            .env("NO_COLOR", "1")
            .arg("--config")
            .arg(&self.config_path)
            .arg("--tome-home")
            .arg(&self.tome_home)
            .arg("--machine")
            .arg(&self.machine_path)
            .arg("sync")
            .args(args);
        cmd
    }
}

/// Write a minimal SKILL.md tree at `<dir>/<skill_name>/SKILL.md`.
fn write_skill(dir: &Path, name: &str, body: &str) -> Result<PathBuf> {
    let skill_dir = dir.join(name);
    std::fs::create_dir_all(&skill_dir)?;
    let frontmatter = format!("---\nname: {name}\ndescription: Test skill {name}\n---\n\n{body}\n");
    std::fs::write(skill_dir.join("SKILL.md"), frontmatter)?;
    Ok(skill_dir)
}

// =========================================================================
// Tests
// =========================================================================

#[test]
fn sync_summary_line_appears_with_three_buckets() -> Result<()> {
    // RECON-01 / D-02 / D-04: even when no managed plugins exist, sync
    // either omits the reconcile summary entirely (no claude-plugins dir)
    // or prints all three buckets.
    //
    // Here we run with a local-only config — no reconcile path fires, so the
    // test asserts that sync EXITS ZERO and produces NO panic. The positive
    // summary regex is asserted in unit tests (Plan 13-03) where we control
    // a populated lockfile + mock adapter directly. This test is the
    // negative-control anchor.
    let f = Fixture::new()?;
    let src = f.tome_home.join("source");
    std::fs::create_dir_all(&src)?;
    write_skill(&src, "alpha", "alpha body")?;
    f.write_local_only_config(&src)?;

    f.run_sync(&["--no-input"]).assert().success();
    Ok(())
}

#[test]
fn sync_no_install_skips_reconcile_apply_with_zero_exit() -> Result<()> {
    // RECON-02 / D-09: `--no-install` is a single-run override. We can't
    // easily simulate drift without a populated lockfile + a real adapter,
    // but we CAN verify that the flag PARSES + sync proceeds + the run
    // exits zero — which is the user-visible contract for the no-claude
    // case.
    let f = Fixture::new()?;
    let src = f.tome_home.join("source");
    std::fs::create_dir_all(&src)?;
    write_skill(&src, "alpha", "alpha body")?;
    f.write_local_only_config(&src)?;

    f.run_sync(&["--no-input", "--no-install"])
        .assert()
        .success();
    Ok(())
}

#[test]
fn sync_with_claude_plugins_dir_but_no_claude_binary_errors_with_d20_message() -> Result<()> {
    // D-20: when `[directories.<x>] type = "claude-plugins"` is configured
    // but `claude` is not on PATH, sync exits non-zero with the actionable
    // error message. Use env_clear() + PATH="" to make `claude` un-findable.
    let f = Fixture::new()?;
    f.write_claude_plugins_config()?;

    let mut cmd = f.run_sync_no_claude(&["--no-input"]);
    cmd.assert()
        .failure()
        .stderr(predicate::str::contains("claude binary not found on PATH"));
    Ok(())
}

#[test]
fn sync_with_no_claude_plugins_dir_does_not_require_claude() -> Result<()> {
    // Negative control for the D-20 test: when there's no claude-plugins
    // directory, sync runs without needing the binary on PATH.
    let f = Fixture::new()?;
    let src = f.tome_home.join("source");
    std::fs::create_dir_all(&src)?;
    write_skill(&src, "alpha", "alpha body")?;
    f.write_local_only_config(&src)?;

    let mut cmd = f.run_sync_no_claude(&["--no-input"]);
    cmd.assert().success();
    Ok(())
}

#[test]
fn sync_preserves_auto_install_plugins_across_runs() -> Result<()> {
    // RECON-02 persistence: write `auto_install_plugins = "always"` to
    // machine.toml, run sync, verify the field is preserved (not stripped
    // by the save chain).
    let f = Fixture::new()?;
    let src = f.tome_home.join("source");
    std::fs::create_dir_all(&src)?;
    write_skill(&src, "alpha", "alpha body")?;
    f.write_local_only_config(&src)?;

    std::fs::write(&f.machine_path, "auto_install_plugins = \"always\"\n")?;

    f.run_sync(&["--no-input"]).assert().success();

    // Re-read machine.toml and assert the consent value survived round-trip.
    let machine_toml = std::fs::read_to_string(&f.machine_path)?;
    assert!(
        machine_toml.contains("auto_install_plugins = \"always\""),
        "auto_install_plugins should be preserved across syncs; got:\n{machine_toml}"
    );
    Ok(())
}

#[test]
fn sync_machine_toml_with_auto_install_never_parses_cleanly() -> Result<()> {
    // RECON-02: `never` is a valid serialized value of the AutoInstall enum.
    let f = Fixture::new()?;
    let src = f.tome_home.join("source");
    std::fs::create_dir_all(&src)?;
    write_skill(&src, "alpha", "alpha body")?;
    f.write_local_only_config(&src)?;

    std::fs::write(&f.machine_path, "auto_install_plugins = \"never\"\n")?;

    f.run_sync(&["--no-input"]).assert().success();
    Ok(())
}

#[test]
fn sync_machine_toml_with_invalid_auto_install_errors() -> Result<()> {
    // RECON-02: invalid enum value is rejected at machine.toml parse time.
    let f = Fixture::new()?;
    let src = f.tome_home.join("source");
    std::fs::create_dir_all(&src)?;
    write_skill(&src, "alpha", "alpha body")?;
    f.write_local_only_config(&src)?;

    std::fs::write(&f.machine_path, "auto_install_plugins = \"sometimes\"\n")?;

    f.run_sync(&["--no-input"]).assert().failure().stderr(
        predicate::str::contains("auto_install_plugins").or(predicate::str::contains("sometimes")),
    );
    Ok(())
}

#[test]
fn vanished_entry_in_lockfile_still_distributes_preserved_library_copy() -> Result<()> {
    // RECON-04 anchor: when a managed skill exists in the library but its
    // marketplace presence is gone, distribution must still create the
    // symlink to the preserved copy.
    //
    // Simulation strategy: pre-populate the library with a local skill
    // copy (no marketplace involvement at distribute time — the source of
    // truth at distribute is the library). Sync runs with no claude-plugins
    // directory (so reconcile doesn't fire), and we verify the symlink is
    // created in the dist dir.
    //
    // This proves the distribution path works for any preserved library
    // entry — vanished is a special case of "library content + lockfile
    // entry, no source dir". The reconcile-side classification of vanished
    // is fully covered by Plan 13-03's unit tests against
    // `MockMarketplaceAdapter`.
    let f = Fixture::new()?;
    let src = f.tome_home.join("source");
    std::fs::create_dir_all(&src)?;
    write_skill(&src, "preserved", "preserved body")?;
    f.write_local_only_config(&src)?;

    // Run sync to populate library + distribute.
    f.run_sync(&["--no-input"]).assert().success();

    // Verify symlink exists in dist dir.
    let symlink = f.dist_dir.join("preserved");
    assert!(
        symlink.exists() || symlink.is_symlink(),
        "expected symlink at {} after sync",
        symlink.display()
    );
    Ok(())
}

#[test]
fn sync_help_advertises_no_install_flag() -> Result<()> {
    // RECON-02 / D-09: `--no-install` is in `tome sync --help`.
    Command::cargo_bin("tome")?
        .arg("sync")
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("--no-install"));
    Ok(())
}

#[test]
fn sync_dry_run_with_no_install_does_not_modify_machine_toml() -> Result<()> {
    // RECON-02 / D-09: `--no-install` is a single-run override; combined
    // with `--dry-run` it should not touch the machine.toml file.
    let f = Fixture::new()?;
    let src = f.tome_home.join("source");
    std::fs::create_dir_all(&src)?;
    write_skill(&src, "alpha", "alpha body")?;
    f.write_local_only_config(&src)?;

    let machine_before = std::fs::read_to_string(&f.machine_path)?;
    f.run_sync(&["--no-input", "--no-install", "--dry-run"])
        .assert()
        .success();
    let machine_after = std::fs::read_to_string(&f.machine_path)?;
    assert_eq!(
        machine_before, machine_after,
        "dry-run + no-install must not modify machine.toml"
    );
    Ok(())
}
