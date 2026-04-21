# Phase 4: Wizard Correctness - Context

**Gathered:** 2026-04-19
**Status:** Ready for planning

<domain>
## Phase Boundary

Close the correctness gaps between the shipped wizard (v0.6) and the original WIZ-01–05 intent. The wizard must refuse to save a config that would fail at sync time. Three concrete gaps:

1. Wizard today calls `config.save()` without running `Config::validate()` — invalid type/role combos produced by the role-editing loop or custom-directory flow can reach disk.
2. `Config::validate()` does not check for path overlap between `library_dir` and distribution (Synced/Target) directories — a configuration that would self-loop at distribute time.
3. `Config::validate()` does not check whether `library_dir` is a subdirectory of a synced directory — circular-symlink risk when `distribute` pushes library contents into a directory that contains the library.

This phase does NOT include: wizard test coverage (Phase 5), display polish via `tabled` (Phase 6), registry expansion for new tools (deferred to v2 requirements), nor ground-up wizard rewrite (out of scope — hardening only).

</domain>

<decisions>
## Implementation Decisions

### Validation Location & Strength

- **D-01:** New path-overlap / circularity checks go in `Config::validate()` — load-symmetric. A hand-edited `tome.toml` with overlapping paths will fail to load. One rule, enforced everywhere (wizard save AND `Config::load()`).
- **D-02:** Path comparison is **lexical only** — after tilde expansion and `PathBuf` normalization. No `Path::canonicalize()`. This keeps `validate()` I/O-free for the new checks and supports first-time setup where `library_dir` may not exist yet. Trade-off explicitly accepted: symlink-based overlaps (where two lexically-different paths resolve to the same physical location) will not be detected.
- **D-03:** The wizard's save path also runs a **TOML round-trip check** (defense in depth): serialize `Config` to TOML, parse back, compare for equality. Catches serde-level regressions that `validate()` cannot see (e.g., a load-bearing field accidentally marked `#[serde(skip_serializing_if)]`). This lives in the wizard save path only — not in `Config::load()`.

### Overlap Semantics

- **D-04:** `validate()` rejects all three relations between `library_dir` and every distribution (Synced or Target) directory:
  - **Case A** — exact path equality
  - **Case B** — `library_dir` is a descendant of the distribution directory (WHARD-03's "library inside synced")
  - **Case C** — the distribution directory is a descendant of `library_dir`
- **D-05:** Distribution-directory-to-distribution-directory overlap (two distro dirs nested) is **out of scope** for this phase. WHARD-02/03 scope is strictly `library_dir` vs distribution pairs.
- **D-06:** Prefix-matching uses trailing-separator normalization to avoid false positives (e.g., `/foo/bar` does NOT contain `/foo/barbaz`).
- **D-07:** Tilde expansion runs before comparison. Wizard save order matches load order: `expand_tildes()` → `validate()` → serialize → save.

### Wizard Failure UX

- **D-08:** On validation failure at the wizard's save step: **hard error + exit**. No retry loop. `wizard::run()` returns `Err(...)` with the validation error context; user re-runs `tome init`. Simple surface area; acceptable cost given how rare validation failure will be once the common first-time flow is correct.
- **D-09:** Non-interactive / `--no-input` / no-TTY mode behaves identically to interactive mode on validation failure: hard error. No interactive recovery attempted.

### Error Message Style

- **D-10:** New validation errors follow a **Conflict + Why + Suggestion** template:
  - **Conflict:** which fields/directories collide, with paths
  - **Why:** plain-english explanation of the consequence (circular symlinks, self-loop, etc.)
  - **Suggestion:** a concrete alternative (e.g., `hint: choose a library_dir outside any distribution directory, such as '~/.tome/skills'`)
- **D-11:** Role names in error messages MUST include the plain-english parenthetical per Phase 1 D-05 (e.g., "Synced (skills discovered here AND distributed here)"). Applies to every new error and to upgrades of existing errors.
- **D-12:** Existing `validate()` errors (Managed-only-for-ClaudePlugins, Git-fields-only-for-Git, etc.) are **upgraded to the new Conflict+Why+Suggestion template** in the same phase. Scope decision: one consistent voice across `validate()`, even at the cost of snapshot-test churn and slight scope growth beyond WHARD-01..03.

### Claude's Discretion

- Exact wording of error messages (as long as they follow the Conflict + Why + Suggestion template and preserve the D-11 role parenthetical)
- Internal layout of new validation helpers (e.g. whether overlap detection is a free function or method on `Config`; whether trailing-separator normalization is a helper in `paths.rs` or inline)
- Whether `Config::validate()` remains a single method or splits into `validate_structure()` + `validate_paths()` — both called from `load()` and the wizard
- Whether the TOML round-trip helper lives in `config.rs` (e.g. `Config::save_checked`) or in `wizard.rs`
- Whether `PartialEq` is derived on `Config` or the round-trip check compares TOML strings directly after canonicalizing

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### Requirements & Roadmap
- `.planning/REQUIREMENTS.md` — WHARD-01 through WHARD-03 definitions and Phase 4 traceability.
- `.planning/ROADMAP.md` §"Phase 4: Wizard Correctness" — four success criteria that must be TRUE after this phase.
- `.planning/PROJECT.md` — Key Decisions table, constraints (Unix-only, single user, hard break OK).

### Prior Phase Context (decisions carried forward)
- `.planning/phases/01-unified-directory-foundation/01-CONTEXT.md` — Phase 1 decisions. Especially D-04/D-05 (plain-english role parenthetical) and D-06 (role picker filtered by type).
- `.planning/phases/02-git-sources-selection/02-CONTEXT.md` — Phase 2 decisions. D-13/D-14 (destructive-command UX patterns) inform how wizard errors surface.
- `.planning/phases/03-import-reassignment-browse-polish/03-CONTEXT.md` — Phase 3 decisions. Confirms `tome add` is the canonical way to register a git skill repo; Phase 4 validation applies uniformly to entries created this way.

### Key Source Files
- `crates/tome/src/config.rs` — `Config`, `DirectoryConfig`, `DirectoryType`, `DirectoryRole`, `Config::validate()`, `Config::load()`, `Config::save()`, `expand_tildes()`. **Primary site for D-01, D-04, D-06, D-10, D-12 changes.**
- `crates/tome/src/config.rs:331` — existing `Config::validate()` body. Extension point.
- `crates/tome/src/config.rs:274` — existing `Config::load()`; already calls `expand_tildes()` then `validate()`. Wizard save must match this order (D-07).
- `crates/tome/src/wizard.rs` — `wizard::run()` entry point at `wizard.rs:126`. Save path at `wizard.rs:312-334` currently calls `config.save()` directly; must insert `validate()` + TOML round-trip ahead of `save()`.
- `crates/tome/src/paths.rs` — `expand_tilde()`, `collapse_home_path()`. Potential home for trailing-separator normalization helper (D-06).
- `crates/tome/src/validation.rs` — `validate_identifier()`. Not the home for path overlap checks (those belong in `config.rs`), but the shape of existing validation errors is a reference.
- `crates/tome/src/lib.rs` — `sync()` orchestrator; `Config::load()` happens here. No changes needed, but the phase verifies that a bad config never reaches `sync()`.

### Existing Tests (update in Phase 5, reference now)
- `crates/tome/src/config.rs:799+` — existing `#[cfg(test)] mod tests` with `validate_rejects_*` cases. Pattern for new overlap tests.
- `crates/tome/tests/cli.rs` — integration tests with `TestEnvBuilder`. Phase 5 will add wizard-specific cases; this phase just ensures existing tests still pass.

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets
- `Config::validate()` — existing structure (iterate directories, `anyhow::bail!` on first failure). New checks slot in as additional iteration after existing ones.
- `Config::load()` path — calls `expand_tildes()` then `validate()`. The save path must mirror this, with the round-trip check inserted.
- `Config::distribution_dirs()` iterator at `config.rs:389` — filters directories by `role().is_distribution()`. Exactly the set we need to compare `library_dir` against. No new iterator needed.
- `expand_tilde()` in `paths.rs` — already used in wizard and config. Reuse for expanding `library_dir` and distribution paths in overlap checks.
- `DirectoryRole::description()` at `config.rs:156` — returns the plain-english parenthetical form (e.g., "Synced (skills discovered here AND distributed here)"). Use this verbatim in new error messages per D-11.
- `#[cfg(test)] mod tests` pattern at `config.rs:799` — existing `validate_rejects_managed_with_directory_type`, `validate_rejects_target_with_git_type`, etc. New tests follow this structure.

### Established Patterns
- `anyhow::bail!("directory '{name}': ...")` — existing error format. D-10 upgrades the message body but keeps the `bail!` call shape.
- `Config::load()` mutates then validates: `expand_tildes()` is in-place on `&mut self`, `validate()` takes `&self`. Wizard must follow: expand → validate → round-trip → save.
- `#[serde(transparent)]` + custom `Deserialize` for validated newtypes (see `DirectoryName` at `config.rs:79`). Not directly touched, but a reference for how strict-on-deserialize validation is layered.
- Fail-fast via `bail!` — `validate()` returns on the first error. Keep this; don't switch to error-collection here (out of scope, and a hard-exit UX means the user fixes one thing at a time anyway).

### Integration Points
- `crates/tome/src/wizard.rs::run()` save block at `wizard.rs:312-334` — single insertion point for both the `validate()` call and the TOML round-trip check. Keep the dry-run preview branch (line 306-311) unchanged except that it should also validate, so `tome init --dry-run` reports the same errors a real save would.
- `crates/tome/src/config.rs::Config::validate()` — the function being extended. All new checks go here, no new public functions required.
- `crates/tome/src/lib.rs::sync()` — inherits the upgrade via `Config::load()`. No direct changes.
- `crates/tome/src/add.rs` — writes git directory entries via `Config::save()`. After Phase 4, if `tome add` is ever used with a URL or `--name` that collides with an existing entry, or if an entry is hand-edited to bad state, the next `Config::load()` will fail with the new error messages.

</code_context>

<specifics>
## Specific Ideas

- Error template "Conflict + Why + Suggestion" is a concrete rule for code review: every new `bail!` in `validate()` must provide all three pieces.
- TOML round-trip check is a useful regression guard even though `validate()` now covers the same ground — it catches a different failure class (serde drift rather than semantic misconfig).
- The wizard's `--dry-run` preview (lines 306-311 in `wizard.rs`) must also run `validate()` + round-trip. If the user sees "Generated config:" output, they should trust that the same config would save cleanly.
- Nothing in this phase touches the TUI (`browse/`), the sync pipeline proper, or `tome add/remove/reassign/fork`. The blast radius is `config.rs` + `wizard.rs` + targeted tests.

</specifics>

<deferred>
## Deferred Ideas

- **In-wizard retry loop on validation failure** — considered and rejected for this phase (D-08). If post-v0.7 user feedback shows this hard-exit UX is painful, revisit as a separate enhancement.
- **Canonicalization-based overlap detection** (resolving symlinks before comparison) — rejected for D-02 to keep `validate()` I/O-free. If symlink-based overlaps ever bite a user, add an opt-in `Config::validate_canonicalized()` later.
- **Collect-all-errors `validate()` API** — rejected. Fail-fast remains the pattern. An error-collection mode would be useful for a "batch check before a big migration" workflow, but that's not this phase.
- **Distro-distro overlap detection** — D-05 explicitly out of scope. If this turns out to be a real foot-gun, schedule a dedicated phase.

</deferred>

---

*Phase: 04-wizard-correctness*
*Context gathered: 2026-04-19*
