---
phase: 11-library-canonical-core
plan: 02
type: execute
wave: 2
depends_on:
  - 11-01
files_modified:
  - crates/tome/src/library.rs
autonomous: true
requirements:
  - LIB-01
  - LIB-02
must_haves:
  truths:
    - "After consolidating a managed skill, `library_dir/<skill>/` is a real directory containing a copy of the source content (verified by `is_dir() && !is_symlink()` and matching `content_hash`)."
    - "After `tome sync` finishes, `find <library_dir> -type l | wc -l` is 0 for managed skills (no symlinks remain in the library for managed entries)."
    - "Consolidating an already-up-to-date managed skill (manifest hash matches source) is a no-op (`unchanged += 1`, no I/O)."
    - "Consolidating a managed skill whose source content has changed re-copies (`updated += 1`)."
    - "When a v0.9-shape symlink is encountered at `library_dir/<skill>` for a managed entry that's in the manifest, `consolidate_managed` returns `result.skipped += 1` and a clear warning message — DO NOT auto-convert. (Migration is a separate one-shot CLI command per D-01; the refuse-with-hint check upstream in `lib.rs::sync` per D-02 prevents this case from ever reaching consolidate during normal sync.)"
    - "`classify_destination` enum branches still produce sensible output for each existing input shape."
  artifacts:
    - path: "crates/tome/src/library.rs"
      provides: "consolidate_managed rewritten as copy semantics"
      contains: "fn consolidate_managed"
  key_links:
    - from: "library.rs::consolidate_managed"
      to: "copy_dir_recursive (existing helper)"
      via: "filesystem copy instead of unix_fs::symlink"
      pattern: "copy_dir_recursive\\(&skill\\.path, dest\\)"
---

<objective>
Rewrite `consolidate_managed` from symlink-creation to recursive-copy semantics. After
this plan, both managed and local skills live as real directory copies in the library —
the library becomes the single source of truth (LIB-01).

Implements LIB-01 fully and LIB-02 (documentation update for `managed: bool` semantics
in `consolidate_managed`'s doc comment, plus moving the strategy-doc comment to reflect
the new uniform "copy" model).

Purpose: Wave 2, runs in parallel with Plan 11-03 (cleanup orphan transition). Both
depend on Plan 11-01's schema lift (call-sites use `SkillEntry::new` which is unchanged
in signature, but tests directly construct entries with `source_name: Some(...)`).

Output: `library.rs::consolidate_managed` writes a copy + records `content_hash`. The
existing local-skill tests in `consolidate_local` continue to work unchanged. Tests for
managed semantics rewritten to assert real dir, not symlink.
</objective>

<execution_context>
@$HOME/.claude/get-shit-done/workflows/execute-plan.md
@$HOME/.claude/get-shit-done/templates/summary.md
</execution_context>

<context>
@.planning/STATE.md
@.planning/ROADMAP.md
@.planning/REQUIREMENTS.md
@.planning/phases/11-library-canonical-core/11-CONTEXT.md
@CLAUDE.md
@crates/tome/src/library.rs
@crates/tome/src/manifest.rs
@crates/tome/src/discover.rs

<interfaces>
<!-- Existing helper that will be reused (already in library.rs at line ~296): -->
```rust
fn copy_dir_recursive(src: &Path, dst: &Path) -> Result<()>
```
This is the same recursive-copy helper used by `consolidate_local`. It walks the
source with `walkdir`, follows no symlinks (`follow_links(false)`), creates dirs +
copies files. Sufficient for managed-skill copy semantics in this plan.

<!-- Existing classify_destination — branches stay the same; only the action per branch changes for managed: -->
```rust
enum DestinationState { Symlink, Directory, Empty, Other }
fn classify_destination(dest: &Path) -> DestinationState
```

<!-- Existing record_in_manifest — unchanged; uses SkillEntry::new which is unchanged in signature post-Plan 11-01: -->
```rust
fn record_in_manifest(
    manifest: &mut Manifest,
    skill: &DiscoveredSkill,
    content_hash: crate::validation::ContentHash,
)
```
</interfaces>
</context>

<tasks>

<task type="auto" tdd="true">
  <name>Task 1: Rewrite `consolidate_managed` to use recursive copy instead of symlink</name>
  <files>crates/tome/src/library.rs</files>
  <read_first>
    - crates/tome/src/library.rs (the WHOLE file — current `consolidate_managed`, `consolidate_local`, `copy_dir_recursive`, `classify_destination`, `record_in_manifest`, all `mod tests`)
    - .planning/phases/11-library-canonical-core/11-CONTEXT.md (D-01, D-02 — migration is a separate CLI command, NOT auto-on-sync; D-08 — content_hash drives drift detection)
    - crates/tome/src/manifest.rs (post Plan 11-01 — confirm `SkillEntry::new` signature unchanged)
  </read_first>
  <behavior>
    - Test 1 (managed creates real dir): consolidating a Managed-origin DiscoveredSkill into an empty library produces `library_dir/<skill>/` as `is_dir() && !is_symlink()`, with all source content copied (e.g. `SKILL.md` content matches source).
    - Test 2 (managed idempotent on hash match): consolidating the same managed skill twice with the same source content yields `result.unchanged == 1` on the second call (manifest hash matches, no I/O).
    - Test 3 (managed updates on content change): modifying source content between two consolidate calls yields `result.updated == 1` on the second call, and the library copy reflects the new content.
    - Test 4 (managed dry-run does nothing): with `dry_run = true`, no library directory is created, but the returned in-memory manifest reflects the would-be state (`entry.managed == true`).
    - Test 5 (managed with v0.9 symlink at dest): if `library_dir/<skill>` is already a symlink AND the skill is in the manifest, `consolidate_managed` returns `result.skipped += 1` with a clear stderr warning (the migration is now a separate one-shot CLI command per D-01; consolidate refuses to auto-convert).
    - Test 6 (managed manifest entry records `managed: true`): manifest entry post-consolidate has `managed == true`, `source_path` matches `skill.path`, and `content_hash` matches `manifest::hash_directory(&skill.path)`.
    - Test 7 (managed force re-copies): with `force = true`, an unchanged-hash managed skill is re-copied (`result.updated == 1`, `result.unchanged == 0`).
    - Test 8 (managed strategy transition local->managed): an existing real-dir manifest entry with `managed: false` for skill X, then re-consolidating X with `Managed` origin: result.updated == 1, library dest is still a real directory (NOT a symlink), manifest entry has `managed: true`.
    - Test 9 (managed `find -type l` post-sync = 0): after consolidating a mix of managed and local skills, `walkdir::WalkDir::new(library_dir).into_iter().filter(|e| e.as_ref().map(|e| e.path_is_symlink()).unwrap_or(false)).count() == 0`.
  </behavior>
  <action>
1. **Replace the entire `consolidate_managed` function body** (currently spans lines ~129-202 in the current file). The new function should mirror `consolidate_local`'s copy semantics but record `managed: true` in the manifest. Specific replacement:

   ```rust
   /// Consolidate a managed skill: copy the source directory into the library.
   ///
   /// Per LIB-01 (v0.10), managed skills are stored as real directory copies in
   /// the library — not symlinks. The `managed: true` flag in the manifest entry
   /// is the "update channel" indicator (per LIB-02): managed = upstream sync
   /// (e.g. claude plugin install/update) feeds updates into the library; local
   /// = library is canonical and never auto-overwritten.
   ///
   /// Idempotency: when the source's content_hash matches the manifest entry,
   /// this is a no-op (`result.unchanged += 1`).
   ///
   /// v0.9-shape detection: if a symlink already exists at `dest` AND the skill
   /// is in the manifest, this function refuses to auto-convert and returns
   /// `result.skipped += 1` with a stderr warning. The user must run
   /// `tome migrate-library` to convert the v0.9-shape library to v0.10-shape
   /// (per D-01). Normally this branch is unreachable because `lib.rs::sync`
   /// performs an isolated v0.9-shape detection check before consolidate (per
   /// D-02) and refuses with a hint.
   fn consolidate_managed(
       skill: &DiscoveredSkill,
       dest: &Path,
       manifest: &mut Manifest,
       result: &mut ConsolidateResult,
       dry_run: bool,
       force: bool,
   ) -> Result<()> {
       let content_hash = manifest::hash_directory(&skill.path)?;

       match classify_destination(dest) {
           DestinationState::Symlink => {
               // v0.9-shape (managed-as-symlink) — refuse to auto-convert.
               // The user must run `tome migrate-library` (per D-01).
               // Normally `lib.rs::sync` blocks this path entirely (per D-02);
               // this branch defends the boundary in case sync's gate is bypassed
               // (e.g. direct call to consolidate from a test or future helper).
               eprintln!(
                   "warning: {} is a v0.9-shape symlink for managed skill — \
                    run `tome migrate-library` to convert to v0.10 shape, skipping",
                   dest.display()
               );
               result.skipped += 1;
           }
           DestinationState::Directory => {
               if let Some(entry) = manifest.get(skill.name.as_str()) {
                   if entry.content_hash == content_hash && !force {
                       // Hash matches — possibly flip managed flag if it changed
                       // (e.g. local→managed strategy transition where content
                       // happens to be identical).
                       if !entry.managed {
                           record_in_manifest(manifest, skill, content_hash.clone());
                           result.updated += 1;
                       } else {
                           result.unchanged += 1;
                       }
                       return Ok(());
                   }
                   // Content changed or force — re-copy.
                   if !dry_run {
                       std::fs::remove_dir_all(dest).with_context(|| {
                           format!("failed to remove old managed skill dir {}", dest.display())
                       })?;
                       copy_dir_recursive(&skill.path, dest)?;
                   }
                   record_in_manifest(manifest, skill, content_hash.clone());
                   result.updated += 1;
               } else {
                   // Real dir exists but not in manifest — user-created collision, skip.
                   eprintln!(
                       "warning: {} exists but is not in the manifest, skipping",
                       dest.display()
                   );
                   result.skipped += 1;
               }
           }
           DestinationState::Empty => {
               // New managed skill — copy from source.
               if !dry_run {
                   copy_dir_recursive(&skill.path, dest)?;
               }
               record_in_manifest(manifest, skill, content_hash.clone());
               result.created += 1;
           }
           DestinationState::Other => {
               eprintln!(
                   "warning: {} exists but is not in the manifest, skipping",
                   dest.display()
               );
               result.skipped += 1;
           }
       }

       Ok(())
   }
   ```

2. **Update the module-level doc comment at the top of `library.rs`** (lines ~1-10). Replace:
   ```rust
   //! Consolidate discovered skills into the library directory.
   //!
   //! Two consolidation strategies based on source type:
   //! - **Managed** (ClaudePlugins): symlink in library → source dir (package manager owns the files)
   //! - **Local** (Directory): copy into library (library is the canonical home)
   //!
   //! Idempotent — unchanged skills are skipped. Handles strategy transitions when a skill's
   //! source type changes between syncs.
   ```
   with:
   ```rust
   //! Consolidate discovered skills into the library directory.
   //!
   //! Per LIB-01 (v0.10), all library entries are real directory copies — both
   //! managed and local skills. The `managed: bool` flag on manifest entries is
   //! the **update channel** indicator (LIB-02):
   //! - **Managed**: upstream sync (e.g. `claude plugin install/update`) feeds
   //!   updates into the library on every `tome sync`.
   //! - **Local**: the library is canonical; the source-on-disk pattern feeds
   //!   the library only when the source content changes.
   //!
   //! Idempotent — unchanged skills (matching content_hash) are skipped. Handles
   //! strategy transitions when a skill's `managed` flag flips between syncs.
   ```

3. **Remove the now-unused `create_symlink` helper and `unix_fs` import.** The helper (lines ~49-52) is no longer used after this rewrite — only `consolidate_local`'s v0.1 migration path used it indirectly (via test fixtures). Verify no other callers remain in this file before deleting:
   - Run `rg "create_symlink|unix_fs::symlink" crates/tome/src/library.rs` and confirm only the `create_symlink` definition itself and the existing `mod tests` `use std::os::unix::fs as unix_fs;` import inside test scopes match.
   - Delete the `create_symlink` function (the 4-line helper at lines ~49-52).
   - Delete the top-of-file `use std::os::unix::fs as unix_fs;` import (line 11).
   - Keep the test-scoped `use std::os::unix::fs as unix_fs;` imports inside `mod tests` (those are still needed for fixture symlinks that simulate v0.9-shape libraries).

4. **Rewrite the existing `consolidate_symlinks_managed_skill` test and friends.** Search for all tests in `mod tests` containing `assert!(dest.is_symlink()` for managed-origin skills — there are several. Update each to assert the new copy semantics:

   - `consolidate_symlinks_managed_skill` → rename to `consolidate_managed_creates_real_dir` and update assertions:
     - `assert!(dest.is_dir())` (still true)
     - `assert!(!dest.is_symlink(), "managed skill should be a real directory copy in v0.10");`
     - `assert!(dest.join("SKILL.md").is_file())`
     - Manifest entry assertions unchanged (`entry.managed == true`).

   - `consolidate_managed_idempotent` — keep, but the test should now assert the second call yields `unchanged == 1` for a hash-matching managed skill. The existing test already does this; just verify it still works after rewrite.

   - `consolidate_managed_path_changed` — currently asserts the symlink target changed. Rewrite to assert the COPIED content changed: after re-consolidate with a different source, the library dest has the second source's content (e.g. write different SKILL.md content in source2 and assert the library copy reflects it).

   - `consolidate_strategy_transition_local_to_managed` — currently ends with `assert!(dest.is_symlink())`. Rewrite to:
     - `assert!(dest.is_dir())`
     - `assert!(!dest.is_symlink(), "managed skill should remain a real directory in v0.10");`
     - `assert!(manifest.get("my-skill").unwrap().managed)` (managed flag flipped).

   - `consolidate_strategy_transition_managed_to_local` — first half of the test (creating the initial managed entry) currently sets up a symlink. Rewrite the setup to consolidate the managed skill first (now produces a real dir copy), then re-consolidate with Local origin. Verify `dest.is_dir() && !dest.is_symlink()` both before and after the transition; the `managed` flag flips from `true` to `false`.

   - `consolidate_managed_dry_run_no_symlink_created` → rename to `consolidate_managed_dry_run_no_dir_created`. Replace the assertion `assert!(!dest.is_symlink(), "dry-run should not create symlink");` with `assert!(!dest.exists(), "dry-run should not create the directory");` (already there) and remove the symlink-specific assertion.

   - `consolidate_managed_force_recreates_symlink` → rename to `consolidate_managed_force_recopies`. Replace `assert!(dest.is_symlink(), "should still be a symlink after force");` with `assert!(dest.is_dir() && !dest.is_symlink(), "should be a real directory after force");`.

   - `consolidate_managed_repairs_stale_directory` — this test simulates a real directory existing where a symlink was expected. In v0.10 this is the EXPECTED state (managed = real dir), so the test no longer represents "stale". Rewrite to:
     - Setup: consolidate normally (creates real dir copy).
     - Modify the library dir's content directly (e.g. write a different `SKILL.md`) so it diverges from the source hash.
     - Re-consolidate. Assert `result.updated == 1` (content_hash mismatch triggers re-copy from source).
     - Assert the library copy now has the source's content again, not the locally-modified content.
     - Rename the test to `consolidate_managed_recopies_when_content_diverges`.

   - `consolidate_managed_replaces_local_dir_with_symlink` — this test asserts the OLD behavior (replacing a real dir with a symlink). Rewrite to assert the NEW behavior:
     - Same setup (real dir in library, manifest entry with `managed: false`).
     - Call `consolidate_managed` directly.
     - Assert `dest.is_dir() && !dest.is_symlink()` (still a real dir, content was re-copied from the managed source).
     - Assert `manifest.get("skill-a").unwrap().managed == true` (managed flag flipped).
     - Assert the library copy contains the managed source's `SKILL.md` content (not the original local content).
     - Rename the test to `consolidate_managed_replaces_local_dir_with_managed_copy`.

5. **Update `consolidate_managed_skips_non_manifest_dir_collision`** — this test pre-creates a real dir at the library path with no manifest entry. Today this hits the `Directory` branch in the new code and returns `skipped += 1` with the warning "exists but is not in the manifest". The test logic should still pass; verify the assertions match the new branch behavior.

6. **Update `gitignore_lists_managed_skills` test.** With managed skills now being real-dir copies (and committed to the library-as-dotfiles git repo), the gitignore behavior may need rethinking. **For Phase 11, do NOT change `generate_gitignore`'s behavior** — leave it as-is (still gitignores managed skill dir names). This is intentional: even though they're real dirs now, they're recreated/updated by `tome sync` on every machine, so gitignoring them avoids cross-machine churn. The existing test just verifies the listing, so it should continue to pass without modification.

   Note this in a code comment above `generate_gitignore`:
   ```rust
   /// Generate or update `.gitignore` in the library directory.
   ///
   /// Managed skill entries are gitignored — even though v0.10 stores them as
   /// real directory copies (LIB-01), they are regenerated by `tome sync` on
   /// every machine via the marketplace adapter (Phase 12+ work). Gitignoring
   /// them avoids cross-machine churn in the dotfiles repo. Local (canonical)
   /// skill entries and `.tome-manifest.json` are tracked.
   ```

7. **Add a new test** `consolidate_refuses_v09_shape_managed_symlink` to assert the D-02 boundary defense:
   ```rust
   #[test]
   fn consolidate_refuses_v09_shape_managed_symlink() {
       use std::os::unix::fs as unix_fs;
       let source = TempDir::new().unwrap();
       let library = TempDir::new().unwrap();
       let skill = make_managed_skill(source.path(), "plugin-skill");

       // Pre-create a v0.9-shape symlink + manifest entry simulating an
       // un-migrated library that bypassed the lib.rs::sync gate.
       unix_fs::symlink(&skill.path, library.path().join("plugin-skill")).unwrap();
       let mut manifest = Manifest::default();
       manifest.insert(
           skill.name.clone(),
           SkillEntry::new(
               skill.path.clone(),
               skill.source_name.clone(),
               manifest::hash_directory(&skill.path).unwrap(),
               true,
           ),
       );
       manifest::save(&manifest, library.path()).unwrap();

       let (result, _) = consolidate(
           std::slice::from_ref(&skill),
           &TomePaths::new(library.path().to_path_buf(), library.path().to_path_buf()).unwrap(),
           false,
           false,
       )
       .unwrap();

       // consolidate_managed must NOT auto-convert — that's migration's job.
       assert_eq!(result.skipped, 1);
       assert_eq!(result.created, 0);
       assert_eq!(result.updated, 0);
       assert!(
           library.path().join("plugin-skill").is_symlink(),
           "v0.9 symlink must be preserved (skipped, not auto-converted)"
       );
   }
   ```

8. **Add a new test** `consolidate_post_sync_no_symlinks_in_library` to anchor the LIB-01 must-have:
   ```rust
   #[test]
   fn consolidate_post_sync_no_symlinks_in_library() {
       let source = TempDir::new().unwrap();
       let library = TempDir::new().unwrap();
       let local = make_skill(source.path(), "local-skill");
       let managed = make_managed_skill(source.path(), "managed-skill");

       consolidate(
           &[local, managed],
           &TomePaths::new(library.path().to_path_buf(), library.path().to_path_buf()).unwrap(),
           false,
           false,
       )
       .unwrap();

       let symlink_count = walkdir::WalkDir::new(library.path())
           .into_iter()
           .filter_map(Result::ok)
           .filter(|e| e.path_is_symlink())
           .count();
       assert_eq!(
           symlink_count, 0,
           "LIB-01: library must contain zero symlinks after sync (managed and local are both real-dir copies)"
       );
   }
   ```
  </action>
  <verify>
    <automated>cargo test --package tome --lib library::tests</automated>
  </verify>
  <acceptance_criteria>
    - `rg -n "fn consolidate_managed" crates/tome/src/library.rs` returns 1 match (one definition only)
    - `rg -n "unix_fs::symlink" crates/tome/src/library.rs` matches ONLY inside `mod tests` (no top-of-file import; no usage in `consolidate_managed`). Confirm with: `rg -n "unix_fs::symlink" crates/tome/src/library.rs | grep -v "mod tests"` returns no matches outside test scope.
    - `rg -n "fn create_symlink" crates/tome/src/library.rs` returns 0 matches (helper deleted)
    - `rg -n "use std::os::unix::fs as unix_fs;" crates/tome/src/library.rs` returns 0 matches at the top-of-file scope (only inside test scope, which is fine)
    - `rg -n "copy_dir_recursive\\(&skill\\.path, dest\\)" crates/tome/src/library.rs` returns at least 2 matches (one in `consolidate_managed::Empty` branch, one in `consolidate_managed::Directory` re-copy branch)
    - `rg -n "consolidate_managed_creates_real_dir|consolidate_post_sync_no_symlinks_in_library|consolidate_refuses_v09_shape_managed_symlink|consolidate_managed_recopies_when_content_diverges|consolidate_managed_replaces_local_dir_with_managed_copy" crates/tome/src/library.rs` returns 5 matches
    - `cargo test --package tome --lib library::tests` exits 0
    - `cargo test --package tome --lib library::tests::consolidate_post_sync_no_symlinks_in_library` exits 0
    - `cargo test --package tome --lib library::tests::consolidate_managed_creates_real_dir` exits 0
  </acceptance_criteria>
  <done>`consolidate_managed` produces real directory copies; tests assert `is_dir() && !is_symlink()`; the v0.9-shape symlink case is refused (not auto-converted); module doc comment reflects LIB-02 "update channel" semantics; the `find -type l` invariant is unit-tested; no obsolete `create_symlink` helper or top-level `unix_fs` import remains.</done>
</task>

</tasks>

<verification>
- `cargo test --package tome --lib library::tests` exits 0 (all rewritten tests pass)
- `cargo build --package tome` exits 0
- LIB-01 truth verified by `consolidate_post_sync_no_symlinks_in_library` test
- LIB-02 documentation verified by reading library.rs module doc comment
- `make ci` exits 0
</verification>

<success_criteria>
- LIB-01 fully addressed: managed skills consolidate to real directory copies; library has zero symlinks for managed entries.
- LIB-02 fully addressed: `managed: bool` semantic shift to "update channel" documented in `consolidate_managed`'s doc comment, the module-level doc comment, and the `SkillEntry.managed` field doc (already done in Plan 11-01).
- Strategy transition tests cover both directions (local→managed, managed→local) and assert real-dir-throughout.
- The v0.9-shape boundary defense is in place; migration (Plan 11-04) handles the conversion.
</success_criteria>

<output>
After completion, create `.planning/phases/11-library-canonical-core/11-02-SUMMARY.md`
documenting: rewritten consolidate_managed, deleted helpers, test rewrites, the
v0.9-shape boundary-defense behavior, and downstream impacts (Plan 11-04 migration
will exercise the v0.9 → v0.10 conversion path).
</output>
