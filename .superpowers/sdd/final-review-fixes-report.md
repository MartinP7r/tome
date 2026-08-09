# Final Whole-Branch Review Fixes Report

## Status

Both blocking findings are fixed and verified.

## RED Evidence

- `cargo test -p tome --test cli_add test_add_preserves_portable_paths_when_machine_override_is_configured -- --exact`
  - Failed because the explicit portable config remained unchanged after add; the override-aware config was passed to `cmd_add` and the save target was derived from `TomePaths` instead of the resolved `--config` file.
- `cargo test -p tome wizard::tests::detects_common_official_tome_repository_clone_urls -- --exact`
  - Failed for `https://github.com/MartinP7r/tome.git`, proving duplicate detection only accepted the byte-exact canonical HTTPS URL.
- `cargo test -p tome wizard::tests::does_not_treat_other_git_sources_as_official_tome_skills -- --exact`
  - Passed before the production change, establishing that forks, other hosts, and other subdirectories were already distinct and must remain so.

## GREEN Evidence

- Add dispatch now loads a fresh portable config without machine directory overrides and passes the resolved config file directly to `cmd_add`.
- The add integration regression runs both a local add and a Git add with a portable path plus machine override. It proves the original portable path remains, the override path is absent, both entries are saved, and an explicit non-default `--config` filename is honored.
- Init duplicate detection accepts the official repository over HTTPS with optional `.git` and trailing slash, SCP-style SSH, and `ssh://git@github.com` while requiring `type = "git"` and `subdir = "skills"`.
- Unit coverage proves equivalent sources suppress insertion and forks, other hosts, and other subdirectories do not.

## Files

- `crates/tome/src/lib.rs`
- `crates/tome/src/wizard.rs`
- `crates/tome/tests/cli_add.rs`
- `docs/superpowers/specs/2026-08-01-tome-agent-skill-design.md`
- `openspec/changes/ship-tome-agent-skill/design.md`
- `openspec/changes/ship-tome-agent-skill/specs/agent-skill-distribution/spec.md`
- `openspec/changes/ship-tome-agent-skill/tasks.md`

## Verification

- Focused add suite: 12 passed, 0 failed.
- Focused init suite: 20 passed, 0 failed.
- Focused wizard suite: 55 passed, 0 failed.
- `cargo fmt --all`: passed.
- `cargo clippy --all-targets -- -D warnings`: passed.
- `openspec validate ship-tome-agent-skill`: valid.
- `make ci`: passed, including 972 Tome library tests, all CLI integration suites, 36 desktop tests, watcher tests, doctests, formatting, Clippy, and typos.

## Signed Commit

- Implementation commit: `a13598ca5fd5933675be97ed4601028ba835e063`
- Signature: good SSH ED25519 signature, key fingerprint `SHA256:MXJbyWLxYTfzLsUF3xodH3QDV6s7ZKgCSFk0A81h6Ls`.

## Concerns

- No product or test concerns remain for these findings.
- Cargo reports the pre-existing future-incompatibility warning for `proc-macro-error2 v2.0.1`.
- Local Git signature output cannot map the valid signature to a principal because `~/.ssh/allowed_signers` is absent; cryptographic signature verification still reports `Good "git" signature`.
- All regression fixtures use temporary homes, config files, machine preferences, and source paths; actual user state was not read or mutated.
