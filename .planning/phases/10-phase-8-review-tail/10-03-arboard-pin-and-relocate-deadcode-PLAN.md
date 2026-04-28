---
phase: 10-phase-8-review-tail
plan: 03
type: execute
wave: 1
depends_on: []
files_modified:
  - Cargo.toml
  - crates/tome/src/relocate.rs
autonomous: true
requirements: [POLISH-06, TEST-05]
issue: "https://github.com/MartinP7r/tome/issues/463 + https://github.com/MartinP7r/tome/issues/462"

must_haves:
  truths:
    - "`arboard` is pinned to a patch-version range (`>=3.6, <3.7`, matching the current Cargo.lock-resolved 3.6.1) in the workspace `Cargo.toml`, with a code comment documenting the bump-review policy: \"review CHANGELOG.md for new arboard::Error variants before bumping; the browse module match-arms must remain exhaustive.\""
    - "`SkillMoveEntry.source_path` is REMOVED from `crates/tome/src/relocate.rs` (POLISH-05 / TEST-05 option a — remove). The `#[allow(dead_code)]` attribute is also gone."
    - "Removing `source_path` does NOT regress the SAFE-03 warning surface: `provenance_from_link_result` is still invoked from the `plan()` body for managed skills (the symlink-read warning fires), but its return value is now logged inline and discarded instead of being stored on the SkillMoveEntry. The SAFE-03 unit test `provenance_from_link_result_warns_and_returns_none_on_err` must still pass."
    - "Three pre-existing tests in `relocate.rs::tests` that ASSERT on `SkillMoveEntry.source_path` (lines 582, 804, 918–924 as of the time of writing) are surgically updated: the SAFE-03 corrupt-symlink integration test stops asserting on the removed field; the standalone `provenance_from_link_result_warns_and_returns_none_on_err` (line 940) becomes the sole regression guard for the SAFE-03 stderr-warning contract."
  artifacts:
    - path: "Cargo.toml"
      provides: "Workspace `[workspace.dependencies]` arboard line is pinned to a patch range (e.g., `\">=3.6, <3.7\"` for current Cargo.lock 3.6.1) instead of `\"3\"`. A `#` comment above the line explains the bump-review policy."
      contains: "arboard"
    - path: "crates/tome/src/relocate.rs"
      provides: "`SkillMoveEntry` no longer has a `source_path: Option<PathBuf>` field. `#[allow(dead_code)]` annotation is removed. `provenance_from_link_result` is retained — called for its stderr-warning side effect with `let _ = ...` discarding the return. Three test-side assertions on `source_path` (lines 582, 804, 918–924) are deleted or replaced."
      contains: "SkillMoveEntry"
  key_links:
    - from: "Cargo.toml [workspace.dependencies]"
      to: "arboard patch-version pin + comment"
      via: "version range narrows from `\"3\"` to `\">=3.6, <3.7\"` (matching current Cargo.lock-resolved 3.6.1)"
      pattern: "arboard.*=.*\">=3"
    - from: "crates/tome/src/relocate.rs SkillMoveEntry"
      to: "no source_path field"
      via: "field declaration removed"
      pattern: "SkillMoveEntry"
    - from: "crates/tome/src/relocate.rs"
      to: "no #[allow(dead_code)]"
      via: "annotation removed"
      pattern: "allow\\(dead_code\\)"
---

<objective>
Pin `arboard` to a patch-version range with a documented bump-review policy in the workspace `Cargo.toml`, and remove the dead `SkillMoveEntry.source_path` field from `crates/tome/src/relocate.rs` along with its `#[allow(dead_code)]` attribute.

This is the cleanup bundle — two small, independent items that don't fit the TUI or Remove buckets. Both are blast-radius-of-1: one Cargo.toml edit, one Rust struct-field deletion plus three corresponding test-assertion deletions.

**Closes:** POLISH-06 (D6, arboard drift hygiene), TEST-05 (P5, dead `source_path` field).

**Decisions pinned:**
- POLISH-06 option: **(a) patch-version pin** (`">=3.6, <3.7"` style, NOT a `cfg(test)` enum-growth canary). Pin is simpler, more obvious, and the bump-review comment captures the same "audit when this changes" intent without test-runtime overhead.
- TEST-05 option: **(a) REMOVE the field** (NOT wire it into `copy_library`/`recreate_target_symlinks`). Wiring it would mean using the symlink target to validate the recreated managed-skill symlink — but `recreate_target_symlinks` already only re-points symlinks that lived in DISTRIBUTION dirs (`recreate_target_symlinks` operates on `plan.targets`, not `plan.skills`). The library-side managed symlinks are recreated implicitly when `copy_library` walks the source tree and copies symlinks via `read_link` + `os::unix::fs::symlink` (lines 419–424). The `source_path` would be redundant with that read. So: delete the field.

Purpose: Remove future-bump silent-breakage hazard for `arboard::Error` variant additions; eliminate `#[allow(dead_code)]` from `relocate.rs` so the file passes future "no-allow-dead-code" lints cleanly.

Output: 2-line Cargo.toml edit, ~10-line struct/import cleanup in relocate.rs, three test-assertion deletions.
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

@Cargo.toml
@crates/tome/src/relocate.rs
@crates/tome/src/browse/app.rs

<interfaces>
<!-- Key types and contracts the executor needs. Extracted from codebase. -->

Current `Cargo.toml` (lines 13–15):
```toml
[workspace.dependencies]
anyhow = "1"
arboard = { version = "3", default-features = false, features = ["wayland-data-control"] }
```

After POLISH-06 (option a — patch pin, current Cargo.lock resolves to 3.6.1):
```toml
[workspace.dependencies]
anyhow = "1"
# Pin arboard to a patch-version range. Bump-review policy: when
# bumping the upper bound (e.g., `<3.7` → `<3.8`), review CHANGELOG.md
# for new `arboard::Error` variants. The match-arms in
# `crates/tome/src/browse/app.rs::execute_action(CopyPath)` and the
# `try_clipboard_set_text_with_retry` helper must remain exhaustive.
# Adding a new variant unobserved is a silent UX regression — the
# fall-through `format!("Could not copy: {other}")` branch hides the
# semantic of the new error from the user (POLISH-06 / #463 D6).
arboard = { version = ">=3.6, <3.7", default-features = false, features = ["wayland-data-control"] }
```

**Pin range selection:** check the current `Cargo.lock` for the resolved arboard version. Cargo.lock currently resolves to `3.6.1` — pin `">=3.6, <3.7"`. If a future arboard 3.7.x lands and Cargo.lock has been bumped before this plan executes, widen accordingly to `">=3.7, <3.8"`. The range MUST allow the current resolved version (so `cargo build` doesn't break) but block minor bumps. Use:
```bash
rg "^name = \"arboard\"" Cargo.lock -A 1 | head -5
```

**Why range vs `~3.6`:** `~3.6` is equivalent to `>=3.6, <3.7` (the patch range we want). Either form works; use the explicit `>=N.M, <N.(M+1)` form because it makes the bump-review intent literally readable in the comment.

Current `crates/tome/src/relocate.rs::SkillMoveEntry` (lines 32–39):
```rust
#[derive(Debug)]
pub(crate) struct SkillMoveEntry {
    pub name: SkillName,
    pub is_managed: bool,
    /// For managed skills, the original symlink target (external source path).
    #[allow(dead_code)]
    pub source_path: Option<PathBuf>,
}
```

After TEST-05 (option a — remove):
```rust
#[derive(Debug)]
pub(crate) struct SkillMoveEntry {
    pub name: SkillName,
    pub is_managed: bool,
}
```

The `plan()` body currently constructs `SkillMoveEntry { name, is_managed, source_path }`. After removal, drop `source_path` from the construction (line ~115).

The `provenance_from_link_result` helper (lines 316–327) is currently called from the `plan()` body to populate `source_path`. After `source_path` is removed, the helper has no consumer:
- **Option α:** drop the helper too. Cleaner; one fewer dead function.
- **Option β:** keep the helper but call it for its SIDE EFFECT (the stderr warning on Err) and discard the return value with `let _ =`.

The SAFE-03 contract (#449) requires that an unreadable symlink produces a stderr warning during `relocate plan`. If we drop the helper entirely, the warning is gone and SAFE-03 regresses.

**Decision: option β.** Keep `provenance_from_link_result` (the warning is SAFE-03's whole point); discard its return value. Update its doc comment to reflect that the return value is no longer used in the relocate path.

Updated plan() body in relocate.rs (~lines 88–117):
```rust
for (name, entry) in manifest.iter() {
    if entry.managed {
        // SAFE-03 / #449: surface read_link errors as stderr warnings during
        // relocate planning so the user can diagnose unreadable symlinks
        // before commit. The provenance return value is no longer stored
        // on SkillMoveEntry (POLISH-05 / TEST-05 option a) — we call this
        // for its side effect only.
        let link_path = old_library_dir.join(name.as_str());
        if link_path.is_symlink() {
            let _ = provenance_from_link_result(std::fs::read_link(&link_path), &link_path);
        }
    }

    skills.push(SkillMoveEntry {
        name: name.clone(),
        is_managed: entry.managed,
    });
}
```

The `let _ = ...` makes the discard explicit; clippy is happy.

**Imports check:** `use std::path::PathBuf` may now have fewer consumers in relocate.rs after the field removal. Run `cargo clippy` and trim if it warns about unused imports. (`PathBuf` is likely still used elsewhere — `RelocatePlan.old_library_dir: PathBuf`, etc.)

**Pre-existing tests that ASSERT on `source_path`** (verified by reading `crates/tome/src/relocate.rs`):

```text
582: assert!(p.skills[0].source_path.is_none());
804: assert!(managed.source_path.is_some());
918–924: assert!(corrupt.source_path.is_none(), "source_path must be None ...", corrupt.source_path);
```

These three sites do NOT construct `SkillMoveEntry` literally — they assert on the field of an entry that `plan()` already built. After the field is removed, all three call sites compile-fail with `error[E0609]: no field 'source_path' on type 'SkillMoveEntry'`. Task 2 / Step 5 below specifies the surgical fix for each site.

The standalone `provenance_from_link_result_warns_and_returns_none_on_err` test (around line 940) drives `provenance_from_link_result` directly with a synthetic `io::Error` and does NOT touch `SkillMoveEntry`. It continues to verify the SAFE-03 stderr-warning contract and is the actual regression guard once the integration-style assertion at lines 918–924 is gone.
</interfaces>
</context>

<tasks>

<task type="auto">
  <name>Task 1: Pin `arboard` to a patch-version range with bump-review comment (POLISH-06)</name>
  <files>Cargo.toml</files>
  <read_first>
    - Cargo.toml lines 13–16 (workspace.dependencies arboard line)
    - Cargo.lock (resolved arboard version — determines the lower bound of the pin range)
  </read_first>
  <action>
**Step 1 — Determine current resolved arboard version**:

```bash
rg "^name = \"arboard\"" -A 1 Cargo.lock | head -10
```

Look for the line `version = "3.X.Y"`. Note the `3.X` major.minor — that's the lower bound. The upper bound is `3.(X+1)` exclusive. As of the time of writing, the resolved version is `3.6.1`, so the pin is `">=3.6, <3.7"`. Use whatever Cargo.lock currently shows.

**Step 2 — Edit `Cargo.toml`**. Replace line 15:

```toml
arboard = { version = "3", default-features = false, features = ["wayland-data-control"] }
```

With (substituting `3.X` for the resolved minor — `3.6` for current Cargo.lock 3.6.1):

```toml
# Pin arboard to a patch-version range. Bump-review policy: when bumping
# the upper bound, review CHANGELOG.md for new `arboard::Error` variants.
# The match-arms in `crates/tome/src/browse/app.rs::execute_action`
# (CopyPath arm) and `try_clipboard_set_text_with_retry` must remain
# exhaustive — a new variant unobserved is a silent UX regression because
# the fall-through `Could not copy: {other}` branch hides the semantic
# of the new error from the user. Closes #463 / POLISH-06 (D6).
arboard = { version = ">=3.6, <3.7", default-features = false, features = ["wayland-data-control"] }
```

**Step 3 — Verify the pin doesn't break the build**:

```bash
cargo build -p tome 2>&1 | tail -10
```

If it fails because the resolved version is outside the new range, widen the range to include the resolved version. The pin should match what's already in `Cargo.lock`.

**Step 4 — Verify no `arboard` 4.x or other major bumps slip through**:

```bash
cargo update -p arboard --dry-run 2>&1 | head -20
```

Should show "no updates available" or only patch updates within the pinned range.

**Step 5 — Verify `crates/tome/Cargo.toml` line 16** still uses `arboard = { workspace = true }` — no change needed there. The pin lives in the workspace manifest only.
  </action>
  <verify>
    <automated>cd /Users/martin/dev/opensource/tome && cargo build -p tome 2>&1 | tail -5 && cargo clippy -p tome --all-targets -- -D warnings 2>&1 | tail -5</automated>
  </verify>
  <acceptance_criteria>
    - `rg -n "^arboard\\s*=\\s*\\{" Cargo.toml` returns exactly 1 match.
    - `rg -n "arboard.*\">=3" Cargo.toml` returns exactly 1 match (pin uses an explicit `>=N.M` lower bound).
    - `rg -n "arboard.*<3" Cargo.toml` returns exactly 1 match (pin uses an explicit `<N.(M+1)` upper bound).
    - `rg -n "Bump-review policy" Cargo.toml` returns exactly 1 match (comment present).
    - `rg -n "match-arms.*exhaustive|exhaustive.*match-arms" Cargo.toml` returns at least 1 match (comment names the consumer site).
    - `rg -n "POLISH-06|#463" Cargo.toml` returns at least 1 match (traceability).
    - `cargo build -p tome` is clean.
    - `cargo clippy -p tome --all-targets -- -D warnings` is clean.
    - `Cargo.lock`'s arboard version is unchanged from before the edit (the pin must not force a downgrade or upgrade — it should accept the currently-resolved version).
  </acceptance_criteria>
  <done>
    `arboard` workspace dependency is pinned to a patch-version range with a multi-line comment documenting the bump-review policy and citing POLISH-06 / #463 D6. Build and clippy clean. Cargo.lock unchanged.
  </done>
</task>

<task type="auto">
  <name>Task 2: Remove dead `SkillMoveEntry.source_path` field + `#[allow(dead_code)]` (TEST-05)</name>
  <files>crates/tome/src/relocate.rs</files>
  <read_first>
    - crates/tome/src/relocate.rs lines 30–40 (SkillMoveEntry struct decl)
    - crates/tome/src/relocate.rs lines 85–120 (plan() body — source_path construction)
    - crates/tome/src/relocate.rs lines 305–328 (provenance_from_link_result — keep, update doc)
    - crates/tome/src/relocate.rs lines 575–585 (first plan-level test asserting source_path.is_none() at line 582)
    - crates/tome/src/relocate.rs lines 795–810 (execute_preserves_managed_symlinks — asserts source_path.is_some() at line 804)
    - crates/tome/src/relocate.rs lines 905–945 (managed_symlink_unreadable_records_no_provenance + provenance_from_link_result_warns_and_returns_none_on_err)
  </read_first>
  <action>
**Step 1 — Remove the `source_path` field from `SkillMoveEntry`** (lines 32–39 in `crates/tome/src/relocate.rs`):

Replace:
```rust
/// A single skill that will be moved.
#[derive(Debug)]
pub(crate) struct SkillMoveEntry {
    pub name: SkillName,
    pub is_managed: bool,
    /// For managed skills, the original symlink target (external source path).
    #[allow(dead_code)]
    pub source_path: Option<PathBuf>,
}
```

With:
```rust
/// A single skill that will be moved during a `tome relocate`.
#[derive(Debug)]
pub(crate) struct SkillMoveEntry {
    pub name: SkillName,
    /// True for managed skills (library symlink → external source dir).
    /// Used by `copy_library` to preserve the symlink shape during the
    /// move (lines ~419–424).
    pub is_managed: bool,
}
```

**Step 2 — Update `plan()` body** (lines 85–117). Replace the current block:

```rust
let source_path = if entry.managed {
    let link_path = old_library_dir.join(name.as_str());
    if link_path.is_symlink() {
        provenance_from_link_result(std::fs::read_link(&link_path), &link_path)
    } else {
        None
    }
} else {
    None
};

skills.push(SkillMoveEntry {
    name: name.clone(),
    is_managed: entry.managed,
    source_path,
});
```

With:

```rust
// SAFE-03 / #449: managed-skill symlinks are checked here so an unreadable
// symlink produces a stderr warning during plan() instead of silently
// disappearing during execute(). The provenance Option<PathBuf> is no
// longer stored on SkillMoveEntry (TEST-05 / POLISH-05 option a — removed
// as dead code). We call provenance_from_link_result for its stderr
// side effect only and discard the return value.
if entry.managed {
    let link_path = old_library_dir.join(name.as_str());
    if link_path.is_symlink() {
        let _ = provenance_from_link_result(std::fs::read_link(&link_path), &link_path);
    }
}

skills.push(SkillMoveEntry {
    name: name.clone(),
    is_managed: entry.managed,
});
```

**Step 3 — Update `provenance_from_link_result` doc comment** (lines 305–315) to reflect the new "called for side effect only" usage:

```rust
/// Translate a `read_link` result into a provenance `Option<PathBuf>`, warning
/// on Err instead of silently dropping (SAFE-03 / #449).
///
/// **Note (TEST-05 / POLISH-05 option a):** the return value is no longer
/// consumed by `relocate::plan()` — `SkillMoveEntry.source_path` was removed
/// as dead code. The function is retained because its primary purpose is
/// the stderr WARNING on the Err arm; the `Option<PathBuf>` return shape is
/// kept for testability (the SAFE-03 unit test asserts `None` on the Err
/// path). Future consumers (e.g., a debug tool that needs provenance) can
/// use the return value; current callers `let _ = ...` it.
fn provenance_from_link_result(raw: std::io::Result<PathBuf>, link_path: &Path) -> Option<PathBuf> {
    match raw {
        Ok(raw_target) => Some(resolve_symlink_target(link_path, &raw_target)),
        Err(e) => {
            eprintln!(
                "warning: could not read symlink at {}: {e}",
                link_path.display()
            );
            None
        }
    }
}
```

The function body is UNCHANGED — only the doc comment is updated.

**Step 4 — Check `mod tests` for literal `SkillMoveEntry { ... }` constructions** (line 519 onwards):

```bash
rg -n "SkillMoveEntry\\s*\\{" crates/tome/src/relocate.rs
```

If matches exist with `source_path: ...` set explicitly in a struct literal, remove the field from those literal constructions. Based on a current scan of the file, NO test constructs `SkillMoveEntry` literally — tests drive `plan()` and let it build entries. So this step is expected to be a no-op grep that confirms there's nothing to fix here. Proceed to Step 5.

**Step 5 — Delete the existing `source_path` ASSERTIONS in `relocate.rs::tests`.**

Three pre-existing tests assert on `SkillMoveEntry.source_path` (NOT construct it). After Step 1 removes the field, all three compile-fail with `error[E0609]: no field 'source_path' on type 'SkillMoveEntry'`. Each must be fixed by deleting (or replacing) the assertion line(s). Locations are stable as of the time of writing:

- **Line 582** (in the first plan-level test, around the `assert_eq!(p.skills.len(), 1)` block at lines 579–582): delete the line
  ```rust
  assert!(p.skills[0].source_path.is_none());
  ```
  No replacement needed — the surrounding `assert!(!p.skills[0].is_managed);` at line 581 already covers the unmanaged-classification intent. (For an unmanaged skill, `source_path` was always `None`; the assertion was redundant with the `is_managed` check.)

- **Line 804** (in `execute_preserves_managed_symlinks`): delete the line
  ```rust
  assert!(managed.source_path.is_some());
  ```
  Replace with `assert!(managed.is_managed);` to keep an assertion at that spot. (Preserves the test's "we found the managed entry" verification — the surrounding `find` filters by name only; the `is_managed` assertion confirms the entry was correctly classified by `plan()`.)

- **Lines 918–924** (in `managed_symlink_unreadable_records_no_provenance` — the SAFE-03 corrupt-symlink integration test): delete the entire block
  ```rust
  assert!(
      corrupt.source_path.is_none(),
      "source_path must be None when the symlink cannot be read (either via the new \
       read_link Err arm or the outer is_symlink-false branch — both uphold SAFE-03's \
       contract); got {:?}",
      corrupt.source_path
  );
  ```
  No replacement needed — the SAFE-03 stderr-warning contract is preserved by the standalone `provenance_from_link_result_warns_and_returns_none_on_err` test (around line 940), which directly tests `provenance_from_link_result` with a synthetic `io::Error`. That test does NOT touch `SkillMoveEntry` and continues to verify the warning side-effect — making it the actual SAFE-03 regression guard. The deleted block was a redundant integration-style assertion on `source_path` and is no longer meaningful once that field is gone.

  The surrounding `assert!(corrupt.is_managed, "corrupt-skill manifest entry is managed");` block (lines 914–917) STAYS — it still pins that the manifest entry was correctly classified as managed even when the symlink could not be read.

**Run `cargo build -p tome --tests` after each deletion** to confirm the build is clean. Then run:

```bash
cargo test -p tome relocate::tests
```

The `provenance_from_link_result_warns_and_returns_none_on_err` test (line ~940) MUST still pass — the function's return shape is unchanged. The `managed_symlink_unreadable_records_no_provenance` test continues to pass because its remaining assertions (existence of the entry in `plan.skills`, `is_managed == true`) do not touch the removed field.

**Step 6 — Verify no other consumer of `source_path` exists** in the crate:

```bash
rg -n "source_path" crates/tome/src/
```

After Steps 1–5, this should return 0 matches in `relocate.rs`. Pre-existing matches in OTHER files (unlikely — they would refer to a different `source_path` field, not the one on `SkillMoveEntry`) need their own treatment if found; document them in the SUMMARY as out-of-scope or fix inline.

**Step 7 — Verify `#[allow(dead_code)]` is gone from `relocate.rs`**:

```bash
rg -n "#\\[allow\\(dead_code\\)\\]" crates/tome/src/relocate.rs
```

Should return 0 matches. (The `RemovePlan::is_empty` `#[allow(dead_code)]` at `crates/tome/src/remove.rs:36` is a separate item not in scope for this task — leave it alone.)

Run: `cargo build -p tome && cargo test -p tome relocate::tests && cargo clippy -p tome --all-targets -- -D warnings`
  </action>
  <verify>
    <automated>cd /Users/martin/dev/opensource/tome && cargo build -p tome --tests 2>&1 | tail -5 && cargo test -p tome relocate::tests 2>&1 | tail -10 && cargo clippy -p tome --all-targets -- -D warnings 2>&1 | tail -5</automated>
  </verify>
  <acceptance_criteria>
    - `rg -c "source_path" crates/tome/src/relocate.rs` returns 0 (field declaration AND all three assertion references gone — Steps 1, 2, and 5 combined).
    - `rg -n "#\\[allow\\(dead_code\\)\\]" crates/tome/src/relocate.rs` returns 0 matches.
    - `rg -n "pub source_path" crates/tome/src/relocate.rs` returns 0 matches (defense-in-depth).
    - `rg -n "pub struct SkillMoveEntry" crates/tome/src/relocate.rs` returns exactly 1 match.
    - `rg -n "let _ = provenance_from_link_result" crates/tome/src/relocate.rs` returns at least 1 match (the side-effect-only call).
    - `rg -n "TEST-05.*option a|POLISH-05.*option a" crates/tome/src/relocate.rs` returns at least 1 match (decision documented in code).
    - `cargo build -p tome --tests` is clean (no "no field `source_path` on type `SkillMoveEntry`" errors anywhere — proves the test-side assertions were removed in Step 5).
    - `cargo test -p tome relocate::tests` passes (all existing tests, including SAFE-03 unit test `provenance_from_link_result_warns_and_returns_none_on_err`).
    - `cargo clippy -p tome --all-targets -- -D warnings` is clean.
  </acceptance_criteria>
  <done>
    `SkillMoveEntry` no longer has a `source_path` field; `#[allow(dead_code)]` is gone from `relocate.rs`. `provenance_from_link_result` is retained (called for SAFE-03 stderr warning side effect; return value discarded with `let _`). The three pre-existing tests asserting on `source_path` (lines 582, 804, 918–924) are surgically updated. Existing SAFE-03 unit test still passes. Build + clippy clean.
  </done>
</task>

</tasks>

<verification>
- `cargo build -p tome --tests` — clean (after both tasks; proves test-side assertions on `source_path` were removed).
- `cargo test -p tome relocate::tests` — all pre-existing tests pass (SAFE-03 unit test included).
- `cargo clippy -p tome --all-targets -- -D warnings` — clean.
- `make ci` — clean.
- `rg -n "arboard.*\">=3" Cargo.toml` — exactly 1 match.
- `rg -n "Bump-review policy" Cargo.toml` — exactly 1 match.
- `rg -c "source_path" crates/tome/src/relocate.rs` — 0.
- `rg -n "#\\[allow\\(dead_code\\)\\]" crates/tome/src/relocate.rs` — 0 matches.
- Cargo.lock arboard version unchanged from pre-edit.
</verification>

<success_criteria>
- `arboard` is pinned to a patch-version range in workspace `Cargo.toml` with a multi-line comment documenting the bump-review policy and citing POLISH-06 / #463 D6 (POLISH-06 option a).
- `SkillMoveEntry.source_path` is removed; `#[allow(dead_code)]` is gone from `relocate.rs` (TEST-05 option a).
- The three pre-existing test-side assertions on `source_path` are surgically updated (lines 582, 804, 918–924).
- SAFE-03 stderr warning surface is preserved: `provenance_from_link_result` is still invoked from `plan()` (for its side effect), and the standalone unit test `provenance_from_link_result_warns_and_returns_none_on_err` continues to pass as the SAFE-03 regression guard.
- Build + clippy clean. Cargo.lock unchanged.
</success_criteria>

<output>
After completion, create `.planning/phases/10-phase-8-review-tail/10-03-SUMMARY.md` recording:
- POLISH-06 option chosen: (a) patch-version pin with bump-review comment.
- TEST-05 option chosen: (a) REMOVE the field.
- Resolved `arboard` version at edit time (e.g., `3.6.1`) and the resulting pin range (e.g., `>=3.6, <3.7`).
- Whether `provenance_from_link_result` was kept (option β: side-effect-only call, return value discarded) or removed.
- Test-assertion deletions: line numbers actually edited (may shift slightly from 582/804/918 if the file has been edited between plan write and execution).
- One-line confirmation: POLISH-06 + TEST-05 closed.
</output>
