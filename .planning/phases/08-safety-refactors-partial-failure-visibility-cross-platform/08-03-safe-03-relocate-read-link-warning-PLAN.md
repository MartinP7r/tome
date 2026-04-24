---
phase: 08-safety-refactors-partial-failure-visibility-cross-platform
plan: 03
type: execute
wave: 1
depends_on: []
files_modified:
  - crates/tome/src/relocate.rs
  - CHANGELOG.md
autonomous: true
requirements:
  - SAFE-03
must_haves:
  truths:
    - "User running `tome relocate` (or any command transiting the patched `fs::read_link(..).ok()` site) sees a stderr warning when a symlink cannot be read, with enough context (path + error) to diagnose — the command no longer silently records 'no provenance' on such failures"
    - "The warning string format mirrors PR #448's pattern verbatim (`warning: could not {verb} at {}: {e}`)"
  artifacts:
    - path: "crates/tome/src/relocate.rs"
      provides: "Explicit match replacing `read_link(&link_path).ok()` at line 93; on Err, eprintln warning + None"
      contains: "warning: could not read symlink at"
  key_links:
    - from: "crates/tome/src/relocate.rs::plan() managed-skill block (~lines 89-100)"
      to: "stderr via eprintln!(\"warning: could not read symlink at {}: {e}\", ...)"
      via: "match arm on Err(e) returning None"
      pattern: "warning: could not read symlink at"
---

<objective>
Replace the silent `std::fs::read_link(&link_path).ok()` drop at `crates/tome/src/relocate.rs:93` with an explicit `match` that emits an `eprintln!("warning: could not read symlink at {}: {e}", link_path.display())` on `Err` and returns `None`. Mirrors the canonical PR #448 fix at `lib.rs:687-693`. Covers SAFE-03 (#449).

Purpose: Today `relocate.rs::plan()`, when scanning managed skills for provenance, swallows `read_link` errors and silently records "no provenance" — users get no signal that the symlink read failed. This plan adds the diagnostic without behavioral change to the success path: the function still returns `None` on failure, the rest of the walk still completes, but the user sees a warning naming the bad path and the underlying error. Smallest plan in the phase: ~10 LoC production + ~20 LoC test.

Output: One-block edit at `relocate.rs:89-100`, one new unit test in the existing `#[cfg(test)] mod tests` block, one CHANGELOG bullet.
</objective>

<execution_context>
@$HOME/.claude/get-shit-done/workflows/execute-plan.md
@$HOME/.claude/get-shit-done/templates/summary.md
</execution_context>

<context>
@.planning/PROJECT.md
@.planning/ROADMAP.md
@.planning/STATE.md
@.planning/REQUIREMENTS.md
@.planning/phases/08-safety-refactors-partial-failure-visibility-cross-platform/08-CONTEXT.md
@.planning/phases/08-safety-refactors-partial-failure-visibility-cross-platform/08-RESEARCH.md
@crates/tome/src/relocate.rs
@crates/tome/src/lib.rs

**CRITICAL drift corrections from RESEARCH.md (override CONTEXT.md where they conflict):**
1. CONTEXT.md says SAFE-03 mirrors **PR #417**. RESEARCH.md confirms **#417 does not exist** as a PR — the canonical sibling-pattern fix landed in **PR #448** (commit `d6e9080`, closed issues #415, #417, #418). The verified canonical warning format lives at `crates/tome/src/lib.rs:687-693`. D-13's wording must match this shape verbatim: `warning: could not {verb} at {}: {e}` with anonymous `{e}` interpolation and lowercase `warning:` prefix.
2. Per D-13, do NOT gate on `!cli.quiet` — `relocate.rs::plan()` does not have a `cli` handle. A unified quiet-flag layer is post-v0.8 polish.
3. Per D-14, do NOT touch `theme.rs:115-117` `.ok()` (env parse fallback) or `git.rs:69` `let _ = rev` (unused-variable suppression) — those are deliberate.
4. Per D-20 + RESEARCH.md: `gag` is NOT a dev-dep. Default the test to observable side-effect (`source_path.is_none()`) rather than capturing stderr.
</context>

<tasks>

<task type="auto">
  <name>Task 1: Replace `.ok()` with explicit match + eprintln warning at relocate.rs:93 (D-13)</name>
  <files>crates/tome/src/relocate.rs</files>
  <read_first>
    - .planning/phases/08-safety-refactors-partial-failure-visibility-cross-platform/08-CONTEXT.md (D-13, D-14)
    - .planning/phases/08-safety-refactors-partial-failure-visibility-cross-platform/08-RESEARCH.md (Pattern 2 — eprintln warning; "Canonical eprintln warning format" code block from PR #448)
    - crates/tome/src/lib.rs (lines 685-697 — read the canonical PR #448 warning format BEFORE editing relocate.rs to confirm the exact string shape)
    - crates/tome/src/relocate.rs (lines 89-100 — the managed-skill block; verify `is_symlink()` wrapping is preserved)
  </read_first>
  <action>
    In `crates/tome/src/relocate.rs::plan()`, locate the managed-skill provenance block at lines 89-100. Current shape (paraphrased):

    ```rust
    let source_path = if entry.managed {
        let link_path = old_library_dir.join(name.as_str());
        if link_path.is_symlink() {
            let raw_target = std::fs::read_link(&link_path).ok();
            raw_target.map(|t| resolve_symlink_target(&link_path, &t))
        } else {
            None
        }
    } else {
        None
    };
    ```

    Replace the `read_link(&link_path).ok()` chain with an explicit match. The exact target shape (from RESEARCH.md Pattern 2, mirroring `lib.rs:687-693` verbatim):

    ```rust
    let source_path = if entry.managed {
        let link_path = old_library_dir.join(name.as_str());
        if link_path.is_symlink() {
            match std::fs::read_link(&link_path) {
                Ok(raw_target) => Some(resolve_symlink_target(&link_path, &raw_target)),
                Err(e) => {
                    eprintln!(
                        "warning: could not read symlink at {}: {e}",
                        link_path.display()
                    );
                    None
                }
            }
        } else {
            None
        }
    } else {
        None
    };
    ```

    Format-string contract (must match PR #448 exactly):
    - Lowercase `warning:` prefix (no `Warning:` / `WARNING:`).
    - Anonymous `{e}` interpolation (NOT `{}` with positional `e`) — matches `lib.rs:687-693`.
    - `link_path.display()` (NOT `link_path.to_string_lossy()`) — matches the canonical pattern.
    - No `!cli.quiet` gate per D-13 (the lib.rs reference DOES have one because `quiet` is in scope there; here it isn't, and adding plumbing is out of scope).

    Behavior preservation:
    - Success path: `Some(resolve_symlink_target(&link_path, &raw_target))` — identical to before, just no longer wrapped in `Option::map`.
    - `Err` path: returns `None` (so the rest of `plan()` continues unchanged), AND emits the warning.
    - `else if !is_symlink()`: still returns `None` silently (no warning) — that branch is by design for non-symlink entries.
    - `else if !entry.managed`: still returns `None` silently — also by design.

    Do NOT touch any other `.ok()` site in the file. Do NOT touch `theme.rs` or `git.rs` per D-14.
  </action>
  <verify>
    <automated>cargo build -p tome 2>&1 | tail -5 && cargo clippy -p tome --all-targets -- -D warnings 2>&1 | tail -5</automated>
  </verify>
  <acceptance_criteria>
    - `grep -q 'warning: could not read symlink at' crates/tome/src/relocate.rs`
    - `grep -q 'link_path.display()' crates/tome/src/relocate.rs`
    - `grep -q 'match std::fs::read_link(&link_path)' crates/tome/src/relocate.rs`
    - `! grep -q 'std::fs::read_link(&link_path).ok()' crates/tome/src/relocate.rs` (silent .ok() drop is gone)
    - `! grep -q '!cli.quiet' crates/tome/src/relocate.rs` (no quiet gate added — D-13)
    - `cargo build -p tome 2>&1 | grep -q 'error\[' && exit 1 || exit 0`
    - `cargo clippy -p tome --all-targets -- -D warnings 2>&1 | grep -qE 'warning\[|error\[' && exit 1 || exit 0`
  </acceptance_criteria>
  <done>The `.ok()` chain at relocate.rs:93 is replaced by an explicit match; on Err the warning is emitted to stderr in the canonical PR #448 format; on Ok behavior is identical; no quiet gate; clippy clean.</done>
</task>

<task type="auto" tdd="true">
  <name>Task 2: Unit test — corrupted managed-skill symlink triggers warning path; plan still succeeds (D-20)</name>
  <files>crates/tome/src/relocate.rs</files>
  <behavior>
    - Given a managed skill whose expected library symlink path exists AND `is_symlink()` is true BUT `read_link` returns `Err` (engineered via `chmod 0o000` on the parent directory), `plan()` must:
      - Still return `Ok(plan)` (overall plan succeeds).
      - The plan entry corresponding to the corrupted skill must have `source_path: None` (observable side-effect of the warning path).
    - Per D-20 + RESEARCH.md: do NOT capture stderr (gag is not a dev-dep). Assert via the observable side-effect (`source_path.is_none()`) instead.
  </behavior>
  <read_first>
    - .planning/phases/08-safety-refactors-partial-failure-visibility-cross-platform/08-CONTEXT.md (D-20)
    - .planning/phases/08-safety-refactors-partial-failure-visibility-cross-platform/08-RESEARCH.md (Pitfall 3 — is_symlink + read_link race; how to engineer an Err from read_link via chmod 0o000 on the parent dir; Pitfall 2 — restore permissions before TempDir::drop)
    - crates/tome/src/relocate.rs (existing `#[cfg(test)] mod tests { ... }` block starting around line 477; existing fixture helpers)
  </read_first>
  <action>
    Add a unit test `read_link_failure_records_no_provenance` to the existing `#[cfg(test)] mod tests { ... }` block in `crates/tome/src/relocate.rs`. Use `#[cfg(unix)]` gating (the chmod trick is Unix-only).

    Strategy (per RESEARCH.md Pitfall 3):
    1. `use std::os::unix::fs::PermissionsExt;` and `use std::os::unix::fs::symlink;`
    2. Create a `TempDir`. Build the existing test scaffold for a managed-skill manifest entry — clone the shape of nearby tests in the `#[cfg(test)] mod tests` block (around lines 477-818) that already construct `LibraryManifest` + managed entries.
    3. Inside the old library dir, create a real symlink for the managed skill via `std::os::unix::fs::symlink(&some_target, &link_path)`. Confirm `link_path.is_symlink()` returns true.
    4. `chmod 0o000` the **parent directory** (i.e., `old_library_dir`) so `read_link` returns `Err(EACCES)`. Save original mode via `std::fs::metadata(...).unwrap().permissions().mode()` so it can be restored.
    5. Call `plan(...)`, capture the result.
    6. **Restore permissions FIRST**: `std::fs::set_permissions(&old_library_dir, std::fs::Permissions::from_mode(0o755)).unwrap();` — BEFORE assertions (Pitfall 2).
    7. Assert (after restore):
       - `let plan = result.expect("plan should succeed even when one symlink read fails");`
       - Find the entry for the corrupted skill in the returned plan structure (use whatever lookup pattern the existing tests use — by skill name).
       - `assert!(entry.source_path.is_none(), "source_path must be None when read_link errors");`

    Do NOT capture stderr. Do NOT add `gag` as a dev-dep. The warning's existence is verified at code-review level + by the existence of the format-string in source (Task 1's grep acceptance) — the unit test only needs to verify the observable side-effect that `source_path` stays `None`.

    If the chmod-on-parent trick proves unreliable in the existing test fixture (e.g., test helper recreates dirs), the alternative engineerable failure is: after creating the symlink, delete the symlink file and create a regular file with the same name — but `is_symlink()` would then return false and the test wouldn't reach the new match arm. Stick with the chmod-on-parent approach.
  </action>
  <verify>
    <automated>cargo test -p tome --lib relocate::tests::read_link_failure_records_no_provenance 2>&1 | tail -15</automated>
  </verify>
  <acceptance_criteria>
    - `grep -q 'read_link_failure_records_no_provenance' crates/tome/src/relocate.rs`
    - `grep -q 'Permissions::from_mode(0o000)' crates/tome/src/relocate.rs` (chmod trick used)
    - `grep -q 'Permissions::from_mode(0o755)' crates/tome/src/relocate.rs` (permissions restored)
    - `grep -q 'source_path.is_none()' crates/tome/src/relocate.rs` (observable side-effect assertion)
    - `cargo test -p tome --lib relocate::tests::read_link_failure_records_no_provenance 2>&1 | grep -q 'test result: ok. 1 passed'`
    - `! grep -q 'use gag' crates/tome/src/relocate.rs` (no gag dev-dep introduced)
    - `! grep -q '^gag' Cargo.toml` (no gag in workspace deps)
  </acceptance_criteria>
  <done>Unit test engineers a `read_link` Err via chmod 0o000 on the symlink's parent dir; restores permissions before assertions; asserts `plan()` still succeeds AND the affected entry's `source_path` is `None`. No stderr capture; no gag dep.</done>
</task>

<task type="auto">
  <name>Task 3: CHANGELOG entry under v0.8 unreleased (SAFE-03 / #449)</name>
  <files>CHANGELOG.md</files>
  <read_first>
    - CHANGELOG.md (top ~50 lines)
  </read_first>
  <action>
    In `CHANGELOG.md`, under v0.8 unreleased `### Fixed`, add:

    ```
    - `tome relocate` now emits a stderr warning (`warning: could not read symlink at <path>: <error>`) when a managed-skill symlink cannot be read, instead of silently recording the entry as having no provenance. Mirrors the eprintln-warning pattern shipped in PR #448. ([#449](https://github.com/MartinP7r/tome/issues/449))
    ```

    Do NOT bump the version number in `Cargo.toml`.
  </action>
  <verify>
    <automated>grep -q 'tome relocate.*could not read symlink' CHANGELOG.md && grep -q '#449' CHANGELOG.md</automated>
  </verify>
  <acceptance_criteria>
    - `grep -q '#449' CHANGELOG.md`
    - `grep -q 'could not read symlink' CHANGELOG.md`
    - `grep -q 'PR #448' CHANGELOG.md` (references the canonical pattern source)
    - `git diff Cargo.toml 2>&1 | grep -q '^+version' && exit 1 || exit 0` (no version bump)
  </acceptance_criteria>
  <done>CHANGELOG.md has a bullet for SAFE-03 referencing #449 and PR #448 as the pattern source; no Cargo.toml version bump.</done>
</task>

</tasks>

<verification>
- `cargo fmt -- --check` passes
- `cargo clippy --all-targets -- -D warnings` passes
- `cargo test` passes (the new `read_link_failure_records_no_provenance` unit test green)
- The warning string format matches `lib.rs:687-693` exactly: lowercase `warning:`, anonymous `{e}`, `path.display()` — verified via the grep acceptance in Task 1
- No quiet-flag plumbing added (D-13)
- `theme.rs:115-117` and `git.rs:69` untouched (D-14) — confirmed by `git diff --stat` showing only `relocate.rs` + `CHANGELOG.md` changed in this plan
- No `gag` dev-dep introduced (D-20 + RESEARCH.md)
</verification>

<success_criteria>
- SAFE-03 requirement satisfied: `tome relocate` surfaces `read_link` failures via stderr warning while preserving the `None` fallback so the rest of the walk completes.
- Format string mirrors PR #448 exactly (the canonical sibling-pattern reference).
- One new unit test (`read_link_failure_records_no_provenance`); no stderr capture; no new dev-deps.
- CHANGELOG bullet present; Cargo.toml untouched.
- Files touched in this plan: `crates/tome/src/relocate.rs`, `CHANGELOG.md`. Nothing else.
</success_criteria>

<output>
After completion, create `.planning/phases/08-safety-refactors-partial-failure-visibility-cross-platform/08-03-safe-03-relocate-read-link-warning-SUMMARY.md` capturing:
- The single-site match replacement at relocate.rs:89-100
- The exact warning format used (verify match with PR #448's `lib.rs:687-693`)
- Unit test name + pass status
- CHANGELOG bullet
- Confirm: theme.rs and git.rs untouched (D-14); no gag dep; no quiet-gate
- Note: PR #449 references "PR #417" but the actual sibling-pattern fix is PR #448 (RESEARCH.md correction); record this so future readers don't chase a phantom PR
</output>
