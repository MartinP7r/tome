---
phase: 11-library-canonical-core
plan: 03
type: execute
wave: 2
depends_on:
  - 11-01
files_modified:
  - crates/tome/src/cleanup.rs
  - crates/tome/src/remove.rs
autonomous: true
requirements:
  - LIB-04
must_haves:
  truths:
    - "Removing a `[directories.*]` entry from `tome.toml` and running `tome sync` preserves all library content originally sourced from that directory; manifest entries' `source_name` becomes `None` (Unowned), library directories remain on disk with content_hash unchanged."
    - "`tome remove <dir>` explicitly transitions all manifest entries owned by `<dir>` to `source_name = None` BEFORE removing the directory entry from config (per D-10 trigger 1)."
    - "The cleanup phase during `tome sync` detects orphan manifest entries (entries whose `source_name == Some(name)` where `name` is no longer a key in `config.directories`) and transitions them to `source_name = None` instead of deleting (per D-10 trigger 2)."
    - "Case 2 (file deleted from disk while source still configured) keeps today's behavior — the library copy is deleted on next sync (D-09)."
    - "After source removal, the next `tome sync` does NOT delete library content; the library directory for the now-Unowned skill remains on disk with its original `content_hash`."
  artifacts:
    - path: "crates/tome/src/cleanup.rs"
      provides: "cleanup_library partitions stale entries into Case 1 (transition) and Case 2 (delete) per D-09/D-10"
      contains: "fn cleanup_library"
    - path: "crates/tome/src/remove.rs"
      provides: "tome remove explicitly transitions owned entries to Unowned before config removal"
      contains: "fn execute"
  key_links:
    - from: "remove.rs::execute"
      to: "manifest entries → source_name = None"
      via: "explicit transition before manifest.remove"
      pattern: "source_name = None"
    - from: "cleanup.rs::cleanup_library"
      to: "config.directories.contains_key check"
      via: "Case 1 detection during stale-entry processing"
      pattern: "config\\.directories\\.contains_key"
---

<objective>
Implement source-removal → Unowned transition (LIB-04) at the two trigger points
specified in CONTEXT.md D-10:
1. **`tome remove <dir>`** — explicit trigger: before removing the directory entry from
   config, transition all manifest entries owned by `<dir>` to `source_name = None`.
   Library content is preserved (no library_path delete).
2. **Cleanup phase during `tome sync`** — implicit/safety-net trigger: detect orphans
   (manifest entries whose `source_name = Some(name)` where `name` isn't a key in
   `config.directories`) and transition them. The user manually editing `tome.toml`
   outside `tome remove` lands here.

D-09 scope: Case 1 only. Case 2 (source still configured but file vanished from disk)
keeps today's "delete on next sync" behavior.

Wave 2, runs in parallel with Plan 11-02. Both depend on Plan 11-01's schema lift.

Output: `cleanup.rs::cleanup_library` partitions stale candidates by D-09 case, then
either transitions (Case 1) or deletes (Case 2). `remove.rs::execute` adds the
explicit Unowned transition step before config removal.
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
@crates/tome/src/cleanup.rs
@crates/tome/src/remove.rs
@crates/tome/src/manifest.rs
@crates/tome/src/config.rs

<interfaces>
<!-- Existing cleanup_library signature — UNCHANGED in this plan: -->
```rust
pub fn cleanup_library(
    library_dir: &Path,
    discovered_names: &HashSet<String>,
    manifest: &mut Manifest,
    dry_run: bool,
    quiet: bool,
    no_input: bool,
) -> Result<CleanupResult>
```

<!-- Today's stale-entry detection (lines ~42-46): -->
```rust
let stale: Vec<SkillName> = manifest
    .keys()
    .filter(|name| !discovered_names.contains(name.as_str()))
    .cloned()
    .collect();
```

<!-- After this plan, cleanup_library MUST also accept `config: &Config` (or its
     directories map) so it can distinguish Case 1 from Case 2. The signature changes —
     callers in lib.rs::sync need updating. See Task 1 below. -->

<!-- Existing remove::execute body (lines ~297-392) — adds Unowned transition before
     config removal. -->
```rust
pub(crate) fn execute(
    plan: &RemovePlan,
    config: &mut Config,
    manifest: &mut Manifest,
    dry_run: bool,
) -> Result<RemoveResult>
```
</interfaces>
</context>

<tasks>

<task type="auto" tdd="true">
  <name>Task 1: Add Case 1/Case 2 partition to `cleanup_library` (orphan transition vs disk-delete)</name>
  <files>crates/tome/src/cleanup.rs</files>
  <read_first>
    - crates/tome/src/cleanup.rs (the WHOLE file — current `cleanup_library`, all `mod tests`)
    - crates/tome/src/config.rs (Config::directories method/field — confirm it's `BTreeMap<DirectoryName, DirectoryConfig>`)
    - crates/tome/src/lib.rs (line ~1060 — current `cleanup::cleanup_library` call site, need to add `&config` arg)
    - .planning/phases/11-library-canonical-core/11-CONTEXT.md (D-09, D-10, D-11)
  </read_first>
  <behavior>
    - Test 1 (Case 1 — source removed from config, transition to Unowned): a manifest entry with `source_name = Some(DirectoryName::new("removed-dir")?)` for skill X. `discovered_names` does NOT contain X (it wasn't discovered because its source dir is gone). `config.directories` does NOT contain "removed-dir". Library has a real directory at `library_dir/X`. After cleanup_library: manifest entry for X has `source_name = None` (Unowned); library dir at `library_dir/X` still exists with original content; `result.removed_from_library == 0` (nothing removed); `result.transitioned_to_unowned == 1` (new counter, see action step 2).
    - Test 2 (Case 2 — source configured but file vanished, delete): manifest entry with `source_name = Some(DirectoryName::new("active-dir")?)` for skill Y. `discovered_names` does NOT contain Y. `config.directories` DOES contain "active-dir" (the directory is still in config; the user just deleted Y from it). After cleanup_library: manifest entry for Y is removed; library dir at `library_dir/Y` is deleted; `result.removed_from_library == 1`.
    - Test 3 (Unowned skill stays Unowned): a manifest entry with `source_name = None` and `library_dir/Z` exists. `discovered_names` does NOT contain Z (Unowned skills don't go through discover). After cleanup_library: manifest entry preserved (still Unowned), library dir preserved, `result.removed_from_library == 0`, `result.transitioned_to_unowned == 0` (already Unowned).
    - Test 4 (mixed case): one Case 1 + one Case 2 in the same cleanup. Assert both behaviors execute correctly.
    - Test 5 (dry-run preserves both kinds): in dry-run mode, neither the manifest mutation (Case 1 transition) nor the library deletion (Case 2) happens, but `result.removed_from_library` and `result.transitioned_to_unowned` reflect the would-be counts.
    - Test 6 (broken-symlink branch unchanged): the existing "remove broken symlinks" branch (lines ~131-152) continues to work as today. Test `cleanup_removes_broken_legacy_symlinks` still passes.
  </behavior>
  <action>
1. **Extend `CleanupResult` with a new counter** for visibility:
   ```rust
   #[derive(Debug, Default)]
   pub struct CleanupResult {
       pub removed_from_library: usize,
       /// Skills transitioned from owned → Unowned (Case 1 of LIB-04 / D-09).
       /// Library content for these skills is preserved on disk.
       pub transitioned_to_unowned: usize,
   }
   ```

2. **Change `cleanup_library` signature to accept `config: &Config`.** Replace the existing function signature with:
   ```rust
   pub fn cleanup_library(
       library_dir: &Path,
       discovered_names: &HashSet<String>,
       manifest: &mut Manifest,
       config: &crate::config::Config,
       dry_run: bool,
       quiet: bool,
       no_input: bool,
   ) -> Result<CleanupResult>
   ```

3. **Rewrite the stale-entry processing branch** to partition by D-09 case. Replace lines ~42-128 (everything from `let stale: Vec<SkillName> = ...` to the end of `for name in skills_to_remove { ... }`) with:

   ```rust
   // Stale candidates = manifest entries whose skill names weren't discovered.
   // We split into D-09 cases:
   //   Case 1: source removed from config → transition to Unowned (preserve library)
   //   Case 2: source still configured, file vanished from disk → delete (today's behavior)
   //
   // Already-Unowned entries (source_name == None) are filtered out of the
   // stale set entirely; they have no source to compare against and are
   // preserved by definition (LIB-04). They were skipped from discover too.
   let stale: Vec<SkillName> = manifest
       .keys()
       .filter(|name| !discovered_names.contains(name.as_str()))
       .filter(|name| {
           // Skip already-Unowned entries — they're preserved by definition.
           manifest
               .get(name.as_str())
               .map(|e| e.source_name.is_some())
               .unwrap_or(false)
       })
       .cloned()
       .collect();

   // Partition stale entries into Case 1 (transition) and Case 2 (delete).
   let mut case1_unowned_transition: Vec<SkillName> = Vec::new();
   let mut case2_delete: Vec<SkillName> = Vec::new();
   for name in &stale {
       let entry = manifest.get(name.as_str()).expect("stale name from manifest");
       // SAFETY: we already filtered out None-source_name entries above.
       let source = entry.source_name.as_ref().expect("filter-guard ensures Some");
       if config.directories().contains_key(source) {
           // Source dir is still configured → file vanished from disk → Case 2.
           case2_delete.push(name.clone());
       } else {
           // Source dir is gone from config → preserve library, transition → Case 1.
           case1_unowned_transition.push(name.clone());
       }
   }

   // --- Case 1: transition to Unowned (preserve library content) ---
   for name in &case1_unowned_transition {
       if !quiet {
           let prev_source = manifest
               .get(name.as_str())
               .and_then(|e| e.source_name.as_ref())
               .map(|d| d.as_str().to_string())
               .unwrap_or_else(|| "unknown".to_string());
           eprintln!(
               "info: skill '{name}' (from '{prev_source}') no longer in any source — preserving as Unowned"
           );
       }
       if !dry_run {
           if let Some(entry) = manifest.skills_get_mut(name.as_str()) {
               entry.source_name = None;
           }
       }
       result.transitioned_to_unowned += 1;
   }

   // --- Case 2: delete (today's behavior) ---
   // Group by source for messaging (matches today's UX) and apply the
   // existing interactive/non-interactive decision logic.
   let mut case2_by_source: std::collections::BTreeMap<String, Vec<SkillName>> =
       std::collections::BTreeMap::new();
   for name in &case2_delete {
       let source = manifest
           .get(name.as_str())
           .and_then(|e| e.source_name.as_ref())
           .map(|d| d.as_str().to_string())
           .unwrap_or_else(|| "unknown".to_string());
       case2_by_source.entry(source).or_default().push(name.clone());
   }

   let skills_to_remove: Vec<SkillName> = if interactive && !case2_delete.is_empty() {
       println!(
           "{}",
           console::style(format!(
               "{} skill(s) missing from configured sources:",
               case2_delete.len()
           ))
           .yellow()
           .bold()
       );
       for (source, names) in &case2_by_source {
           println!(
               "  {} (from '{}'):",
               console::style(format!("{} skill(s)", names.len())).dim(),
               source
           );
           for name in names {
               println!("    {}", name);
           }
       }
       println!();
       let confirmed = dialoguer::Confirm::new()
           .with_prompt("Delete these skills from library?")
           .default(false)
           .interact_opt()?;
       if confirmed == Some(true) {
           case2_delete.clone()
       } else {
           Vec::new()
       }
   } else if !case2_delete.is_empty() {
       for (source, names) in &case2_by_source {
           for name in names {
               eprintln!(
                   "warning: skill '{name}' (from '{source}') missing from source on disk, removing from library"
               );
           }
       }
       case2_delete.clone()
   } else {
       Vec::new()
   };

   for name in skills_to_remove {
       let entry_path = library_dir.join(name.as_str());

       if !dry_run {
           if entry_path.is_symlink() {
               std::fs::remove_file(&entry_path).with_context(|| {
                   format!("failed to remove managed symlink {}", entry_path.display())
               })?;
           } else if entry_path.is_dir() {
               std::fs::remove_dir_all(&entry_path).with_context(|| {
                   format!("failed to remove stale skill dir {}", entry_path.display())
               })?;
           }
           manifest.remove(name.as_str());
       }
       result.removed_from_library += 1;
   }
   ```

4. **Add the `skills_get_mut` accessor to `Manifest`** if it doesn't already exist. Check first: `rg "fn skills_get_mut|fn get_mut" crates/tome/src/manifest.rs`. If neither exists, add to `impl Manifest` in `crates/tome/src/manifest.rs`:
   ```rust
       /// Mutable access to a skill entry by name.
       pub(crate) fn skills_get_mut(&mut self, name: &str) -> Option<&mut SkillEntry> {
           self.skills.get_mut(name)
       }
   ```
   (Only add this if no equivalent `get_mut` accessor exists. Use `pub(crate)` to keep surface minimal.)

5. **Update the call site in `lib.rs`** (around line ~1060):
   - Find the existing call:
     ```rust
     let cleanup_result = cleanup::cleanup_library(
         paths.library_dir(),
         &discovered_names,
         &mut manifest,
         dry_run,
         quiet,
         no_input,
     )?;
     ```
   - Replace with:
     ```rust
     let cleanup_result = cleanup::cleanup_library(
         paths.library_dir(),
         &discovered_names,
         &mut manifest,
         config,
         dry_run,
         quiet,
         no_input,
     )?;
     ```

6. **Update existing tests in `cleanup.rs::tests`** to pass a `Config` argument. Each test fixture currently does NOT construct a `Config`. Add a helper at the top of the test module:
   ```rust
   fn empty_config() -> crate::config::Config {
       crate::config::Config::default()
   }
   fn config_with_dir(name: &str) -> crate::config::Config {
       use crate::config::{Config, DirectoryConfig, DirectoryName, DirectoryRole, DirectoryType};
       use std::collections::BTreeMap;
       let mut directories = BTreeMap::new();
       directories.insert(
           DirectoryName::new(name).unwrap(),
           DirectoryConfig {
               path: std::path::PathBuf::from("/tmp/source"),
               directory_type: DirectoryType::Directory,
               role: Some(DirectoryRole::Source),
               git_ref: None,
               subdir: None,
               override_applied: false,
           },
       );
       Config { directories, ..Default::default() }
   }
   ```

   Then update every `cleanup_library(...)` call in tests to pass either `&empty_config()` or `&config_with_dir("test")` as the new 4th arg, BEFORE `dry_run`.

   Specifically:
   - `cleanup_removes_stale_manifest_entries` — pass `&empty_config()`. The test's manifest entry has `source_name = "test"`, but config has no "test" dir → Case 1 (transition). UPDATE the assertions: `result.removed_from_library == 0`, `result.transitioned_to_unowned == 1`, `library.path().join("old-skill").exists()` (TRUE — preserved), `manifest.get("old-skill").unwrap().source_name == None` (transitioned to Unowned).
     **Rename the test to `cleanup_transitions_orphaned_to_unowned_when_source_removed_from_config`.**
   - `cleanup_preserves_current_skills` — pass `&config_with_dir("test")`. No change to assertions; `keep-me` is in `discovered_names` so it's not stale.
   - `cleanup_dry_run_preserves_stale` — pass `&empty_config()`. UPDATE assertions: `result.removed_from_library == 0`, `result.transitioned_to_unowned == 1` (would-be transition), library still exists (preserved as Unowned even outside dry-run), manifest entry still has `source_name = Some(...)` (because dry-run skipped the mutation).
     **Rename to `cleanup_dry_run_does_not_mutate_manifest_for_unowned_transition`.**
   - `cleanup_removes_broken_legacy_symlinks` — pass `&empty_config()`. No change to broken-symlink branch behavior; this test should still pass.
   - `cleanup_dry_run_preserves_managed_symlink` — pass `&empty_config()`. The current behavior (broken-symlink detection) is unchanged.
   - `cleanup_removes_managed_symlink` — this test simulates a managed-skill symlink in the library that's no longer discovered. The manifest entry has `source_name = "plugins"` and config has no "plugins" dir → Case 1. UPDATE assertions: `result.removed_from_library == 0`, `result.transitioned_to_unowned == 1`, the symlink IS preserved (library content preserved), manifest entry's `source_name == None`.
     **NOTE:** the symlink existence here is a v0.9-shape artifact. In v0.10 these would be real dirs (per Plan 11-02). The test still represents a valid scenario (legacy machine that never ran migrate-library, but still has cleanup running). The behavior should preserve the entry the same way.
     **Rename to `cleanup_transitions_managed_symlink_to_unowned_when_source_removed`.**

7. **Add new tests** for the partition behavior:
   ```rust
   #[test]
   fn cleanup_case2_deletes_when_source_still_configured() {
       let library = TempDir::new().unwrap();
       let skill_dir = library.path().join("vanished");
       std::fs::create_dir_all(&skill_dir).unwrap();

       let mut manifest = Manifest::default();
       manifest.insert(
           crate::discover::SkillName::new("vanished").unwrap(),
           crate::manifest::SkillEntry::new(
               std::path::PathBuf::from("/tmp/source/vanished"),
               crate::config::DirectoryName::new("active-source").unwrap(),
               crate::validation::test_hash("h"),
               false,
           ),
       );

       // Config STILL has "active-source" — file vanished from source disk → Case 2.
       let config = config_with_dir("active-source");
       let discovered: HashSet<String> = HashSet::new();
       let result = cleanup_library(
           library.path(), &discovered, &mut manifest, &config, false, false, true,
       ).unwrap();

       assert_eq!(result.removed_from_library, 1, "Case 2 must delete");
       assert_eq!(result.transitioned_to_unowned, 0, "Case 2 must NOT transition");
       assert!(!library.path().join("vanished").exists(), "Case 2 must remove library dir");
       assert!(!manifest.contains_key("vanished"));
   }

   #[test]
   fn cleanup_already_unowned_entry_is_preserved_and_not_counted() {
       let library = TempDir::new().unwrap();
       let skill_dir = library.path().join("orphan");
       std::fs::create_dir_all(&skill_dir).unwrap();

       let mut manifest = Manifest::default();
       manifest.insert(
           crate::discover::SkillName::new("orphan").unwrap(),
           crate::manifest::SkillEntry::new_unowned(
               std::path::PathBuf::from("/tmp/orphan"),
               crate::validation::test_hash("h"),
               false,
           ),
       );

       let config = empty_config();
       let discovered: HashSet<String> = HashSet::new();
       let result = cleanup_library(
           library.path(), &discovered, &mut manifest, &config, false, false, true,
       ).unwrap();

       assert_eq!(result.removed_from_library, 0);
       assert_eq!(result.transitioned_to_unowned, 0, "already-Unowned must not be counted");
       assert!(library.path().join("orphan").is_dir(), "Unowned library content preserved");
       assert!(manifest.contains_key("orphan"));
       assert!(manifest.get("orphan").unwrap().source_name.is_none());
   }

   #[test]
   fn cleanup_case1_and_case2_in_same_run() {
       let library = TempDir::new().unwrap();
       std::fs::create_dir_all(library.path().join("orphan-c1")).unwrap();
       std::fs::create_dir_all(library.path().join("vanished-c2")).unwrap();

       let mut manifest = Manifest::default();
       manifest.insert(
           crate::discover::SkillName::new("orphan-c1").unwrap(),
           crate::manifest::SkillEntry::new(
               std::path::PathBuf::from("/tmp/removed-source/orphan-c1"),
               crate::config::DirectoryName::new("removed-source").unwrap(),
               crate::validation::test_hash("h1"),
               false,
           ),
       );
       manifest.insert(
           crate::discover::SkillName::new("vanished-c2").unwrap(),
           crate::manifest::SkillEntry::new(
               std::path::PathBuf::from("/tmp/active-source/vanished-c2"),
               crate::config::DirectoryName::new("active-source").unwrap(),
               crate::validation::test_hash("h2"),
               false,
           ),
       );

       // Config has "active-source" but NOT "removed-source".
       let config = config_with_dir("active-source");
       let discovered: HashSet<String> = HashSet::new();
       let result = cleanup_library(
           library.path(), &discovered, &mut manifest, &config, false, false, true,
       ).unwrap();

       assert_eq!(result.removed_from_library, 1);
       assert_eq!(result.transitioned_to_unowned, 1);
       assert!(library.path().join("orphan-c1").exists(), "C1 preserved");
       assert!(!library.path().join("vanished-c2").exists(), "C2 deleted");
       assert_eq!(manifest.get("orphan-c1").unwrap().source_name, None);
       assert!(!manifest.contains_key("vanished-c2"));
   }
   ```
  </action>
  <verify>
    <automated>cargo test --package tome --lib cleanup::tests</automated>
  </verify>
  <acceptance_criteria>
    - `rg -n "fn cleanup_library" crates/tome/src/cleanup.rs` returns 1 match with new signature including `config: &crate::config::Config`
    - `rg -n "transitioned_to_unowned" crates/tome/src/cleanup.rs` returns at least 3 matches (struct field, increment site, test assertions)
    - `rg -n "config\\.directories\\(\\)\\.contains_key" crates/tome/src/cleanup.rs` returns at least 1 match (Case 1 vs Case 2 dispatch)
    - `rg -n "cleanup_case2_deletes_when_source_still_configured|cleanup_already_unowned_entry_is_preserved_and_not_counted|cleanup_case1_and_case2_in_same_run|cleanup_transitions_orphaned_to_unowned_when_source_removed_from_config" crates/tome/src/cleanup.rs` returns 4 matches
    - `rg -n "cleanup::cleanup_library\\(" crates/tome/src/lib.rs` shows the call passes `config` as 4th arg
    - `cargo test --package tome --lib cleanup::tests` exits 0
    - `cargo build --package tome` exits 0
  </acceptance_criteria>
  <done>cleanup_library partitions stale candidates into Case 1 (transition to Unowned, preserve library) and Case 2 (today's delete behavior); already-Unowned entries are preserved untouched; the lib.rs call site is updated; new and renamed tests cover all four scenarios (Case 1, Case 2, already-Unowned, mixed).</done>
</task>

<task type="auto" tdd="true">
  <name>Task 2: Add explicit Unowned transition to `tome remove <dir>` execute path (D-10 trigger 1)</name>
  <files>crates/tome/src/remove.rs</files>
  <read_first>
    - crates/tome/src/remove.rs (the WHOLE file — current `execute`, `RemovePlan`, `RemoveResult`, `mod tests`)
    - crates/tome/src/manifest.rs (post Plan 11-01 — confirm `source_name: Option<DirectoryName>` and `skills_get_mut` accessor)
    - .planning/phases/11-library-canonical-core/11-CONTEXT.md (D-10 trigger 1, D-11 distribution semantics)
  </read_first>
  <behavior>
    - Test 1 (transition then preserve library): a manifest with skill X owned by directory D (source_name = Some(D)). `tome remove D` execute path: after success, manifest entry for X has `source_name = None`, library entry at `library_dir/X` is preserved (NOT deleted), config no longer has D, distribution symlinks from D are removed.
    - Test 2 (multiple owned skills): three skills owned by D. After `tome remove D`: all three transition to source_name = None; all three library entries preserved; config no longer has D.
    - Test 3 (mixed: Unowned + owned-by-D + owned-by-other): manifest has skill X owned by D, skill Y owned by other-dir, skill Z already Unowned. `tome remove D`: only X transitions; Y and Z untouched.
    - Test 4 (dry-run preserves all state): in dry-run, no manifest mutation, no config mutation, no library deletion.
    - Test 5 (partial failure on dist-symlink + still transitions): if a distribution symlink removal fails (FailureKind::DistributionSymlink), the existing partial-failure semantics retain config + manifest (today's behavior). The Unowned transition does NOT happen on partial failure either — config retention pairs with manifest retention so re-running `tome remove D` works.
  </behavior>
  <action>
1. **Modify `RemovePlan`** to carry the planned-skill list explicitly (already does via `skills: Vec<String>`) and DROP the `library_paths` removal step from execute. Today's `execute` removes library_paths in step 2 — for v0.10, library content stays. The `library_paths` field on `RemovePlan` becomes informational only (used by `render_plan` to tell the user what's preserved vs removed).

   Update the doc comment at the top of `remove.rs`:
   ```rust
   //! Remove a directory entry from config and clean up most artifacts.
   //!
   //! Cleanup order (v0.10, per LIB-04 / D-10 trigger 1):
   //! 1. Remove symlinks from distribution directories
   //! 2. Transition manifest entries owned by this directory to Unowned
   //!    (`source_name = None`) — library content is preserved on disk
   //! 3. Remove cached git repo (if git-type directory)
   //! 4. Remove directory entry from config
   //! 5. Regenerate lockfile
   //!
   //! Note: library directories for skills owned by the removed directory are
   //! NOT deleted (LIB-04). The user can later re-anchor them with
   //! `tome adopt <skill> <new-dir>` or delete them with `tome forget <skill>`
   //! (Phase 14 commands).
   ```

2. **Update `RemovePlan` doc comment** for the `library_paths` field:
   ```rust
       /// Library directories for these skills (preserved per LIB-04 v0.10 — these
       /// are reported by render_plan as "kept as Unowned" but NOT deleted by execute).
       pub library_paths: Vec<PathBuf>,
   ```

3. **Update `RemoveResult`** to drop the now-meaningless `library_entries_removed` counter and add a transition counter. Replace:
   ```rust
   pub(crate) struct RemoveResult {
       pub symlinks_removed: usize,
       pub library_entries_removed: usize,
       pub git_cache_removed: bool,
       pub failures: Vec<RemoveFailure>,
   }
   ```
   with:
   ```rust
   pub(crate) struct RemoveResult {
       pub symlinks_removed: usize,
       /// Manifest entries transitioned to Unowned (`source_name = None`) per
       /// LIB-04 / D-10 trigger 1. Library content for these skills is preserved.
       pub library_entries_transitioned_to_unowned: usize,
       pub git_cache_removed: bool,
       pub failures: Vec<RemoveFailure>,
   }
   ```

4. **Drop the LibraryDir and LibrarySymlink variants** from `FailureKind` since v0.10 `tome remove` no longer touches the library files. Replace the enum, ALL constant, label() match, _ensure_failure_kind_all_exhaustive const fn, and the `const _: () = { assert!(FailureKind::ALL.len() == 4); };` to be `len() == 2`:
   ```rust
   #[derive(Debug, Clone, Copy, PartialEq, Eq)]
   pub(crate) enum FailureKind {
       /// Distribution-dir symlink removal — emitted when `remove_file` fails
       /// while iterating `plan.symlinks_to_remove`.
       DistributionSymlink,
       /// Git repo cache removal — emitted when `remove_dir_all` fails on the
       /// plan's `git_cache_path`.
       GitCache,
   }

   impl FailureKind {
       pub(crate) const ALL: [FailureKind; 2] = [
           FailureKind::DistributionSymlink,
           FailureKind::GitCache,
       ];

       pub(crate) fn label(self) -> &'static str {
           match self {
               FailureKind::DistributionSymlink => "Distribution symlinks",
               FailureKind::GitCache => "Git cache",
           }
       }
   }

   #[allow(dead_code)]
   const fn _ensure_failure_kind_all_exhaustive(k: FailureKind) -> usize {
       match k {
           FailureKind::DistributionSymlink => 0,
           FailureKind::GitCache => 1,
       }
   }

   const _: () = {
       assert!(FailureKind::ALL.len() == 2);
   };
   ```

5. **Rewrite `execute`** to: (a) keep step 1 (distribution symlink removal), (b) replace step 2 (library deletion) with the Unowned transition, (c) keep step 4 (git cache removal), (d) keep step 5 (config + manifest update on full success — but manifest now keeps the entries with source_name=None, doesn't remove them).

   Replace the entire body of `execute` (lines ~297-392):
   ```rust
   pub(crate) fn execute(
       plan: &RemovePlan,
       config: &mut Config,
       manifest: &mut Manifest,
       dry_run: bool,
   ) -> Result<RemoveResult> {
       let mut symlinks_removed = 0;
       let mut library_entries_transitioned_to_unowned = 0;
       let mut git_cache_removed = false;
       let mut failures: Vec<RemoveFailure> = Vec::new();

       // 1. Remove symlinks from distribution directories.
       for symlink in &plan.symlinks_to_remove {
           if dry_run {
               symlinks_removed += 1;
           } else {
               match std::fs::remove_file(symlink) {
                   Ok(_) => symlinks_removed += 1,
                   Err(e) => failures.push(RemoveFailure::new(
                       FailureKind::DistributionSymlink,
                       symlink.clone(),
                       e,
                   )),
               }
           }
       }

       // 2. Remove cached git repo (if applicable).
       if let Some(cache_path) = &plan.git_cache_path {
           if dry_run {
               git_cache_removed = true;
           } else {
               match std::fs::remove_dir_all(cache_path) {
                   Ok(_) => git_cache_removed = true,
                   Err(e) => failures.push(RemoveFailure::new(
                       FailureKind::GitCache,
                       cache_path.clone(),
                       e,
                   )),
               }
           }
       }

       // 3. On full success: transition manifest entries owned by this directory
       //    to Unowned (source_name = None) AND remove the directory entry from
       //    config (LIB-04 / D-10 trigger 1). Library content is preserved on disk.
       //
       //    On partial failure: preserve config + manifest entries unchanged so
       //    `tome remove <name>` can be re-run after addressing the underlying
       //    cause (matches Phase 8 SAFE-01 retention semantics).
       if !dry_run && failures.is_empty() {
           for skill_name in &plan.skills {
               if let Some(entry) = manifest.skills_get_mut(skill_name) {
                   entry.source_name = None;
                   library_entries_transitioned_to_unowned += 1;
               }
           }
           config.directories.remove(&plan.directory_name);
       } else if dry_run {
           // In dry-run, count what WOULD transition but don't mutate.
           library_entries_transitioned_to_unowned = plan.skills.len();
       }

       Ok(RemoveResult {
           symlinks_removed,
           library_entries_transitioned_to_unowned,
           git_cache_removed,
           failures,
       })
   }
   ```

6. **Update `render_plan`** to tell the user library content is preserved. Replace the `library_paths` rendering block:
   ```rust
       if !plan.library_paths.is_empty() {
           println!(
               "  Library content preserved as {} (run `tome forget <skill>` later to delete): {}",
               style("Unowned").yellow(),
               style(plan.library_paths.len()).bold()
           );
       }
   ```

7. **Update existing tests in `remove::tests`** for the new behavior:
   - `execute_removes_artifacts` — UPDATE assertions:
     - `result.symlinks_removed == 1` (unchanged)
     - `result.library_entries_transitioned_to_unowned == 1` (replaces `library_entries_removed`)
     - `assert!(!config.directories.contains_key(&DirectoryName::new("test-source").unwrap()))` (config entry removed)
     - `assert!(!manifest.is_empty(), "manifest entry retained as Unowned");`
     - `assert_eq!(manifest.get("my-skill").unwrap().source_name, None, "transitioned to Unowned");`
     - `assert!(_tmp.path().join("library").join("my-skill").exists(), "library content preserved per LIB-04");`
     **Rename to `execute_transitions_to_unowned_and_preserves_library`**.

   - `partial_failure_aggregates_symlink_error` — the LibraryDir failure variant no longer exists; this test currently asserts only the DistributionSymlink failure (which still works). UPDATE assertions:
     - `result.library_entries_transitioned_to_unowned == 0` (no transition on partial failure)
     - `assert_eq!(manifest.get("my-skill").unwrap().source_name, Some(DirectoryName::new("test-source").unwrap()), "transition NOT applied on partial failure");`

   - `partial_failure_aggregates_multiple_kinds` — currently uses `FailureKind::LibraryDir` to test multi-variant aggregation. The LibraryDir variant is removed; rewrite this test to aggregate `DistributionSymlink + GitCache` instead. Use a git-type directory in the fixture and engineer a permission-denial on the cache dir parent.
     Alternative: simplify by removing the multi-variant aspect — the single-variant `partial_failure_aggregates_symlink_error` already covers the aggregation pattern. Decision: replace this test with a `failure_kind_label_coverage` that asserts every variant's `.label()` returns its expected string (covers `DistributionSymlink` and `GitCache`) plus `failure_kind_all_pinned_size_two` that asserts `FailureKind::ALL.len() == 2`. Simpler and covers the same compile-enforcement boundary.

   - `failure_kind_all_length_matches_variant_count` — UPDATE the literal `4` to `2`; the membership checks change to only `DistributionSymlink` and `GitCache`.

   - `failure_kind_all_ordering_pinned` — UPDATE the literal array to `[FailureKind::DistributionSymlink, FailureKind::GitCache]`.

   - `execute_dry_run_preserves_state` — UPDATE assertions:
     - `result.symlinks_removed == 1`, `result.library_entries_transitioned_to_unowned == 1` (would-be count)
     - `assert!(config.directories.contains_key(...), "dry-run preserves config");`
     - `assert_eq!(manifest.get("my-skill").unwrap().source_name, Some(DirectoryName::new("test-source").unwrap()), "dry-run does not mutate manifest");`

   - `remove_failure_new_relative_path_panics_in_debug` and `remove_failure_new_absolute_path_succeeds` — unchanged behavior, unchanged tests. Verify they still compile after the FailureKind refactor (only the `kind:` argument changes if a removed variant was used, but these tests use `DistributionSymlink` which still exists).

8. **Add a new test** for the multi-skill transition:
   ```rust
   #[test]
   fn execute_transitions_multiple_owned_skills_to_unowned() {
       let (_tmp, mut config, paths, mut manifest) = make_test_setup();
       // Add two more skills owned by test-source.
       for n in &["skill-2", "skill-3"] {
           manifest.insert(
               SkillName::new(n).unwrap(),
               SkillEntry::new(
                   _tmp.path().join("source").join(n),
                   DirectoryName::new("test-source").unwrap(),
                   test_hash(),
                   false,
               ),
           );
       }
       let p = plan("test-source", &config, &paths, &manifest).unwrap();
       let result = execute(&p, &mut config, &mut manifest, false).unwrap();

       assert_eq!(result.library_entries_transitioned_to_unowned, 3);
       for n in &["my-skill", "skill-2", "skill-3"] {
           assert_eq!(
               manifest.get(n).unwrap().source_name, None,
               "skill {n} should transition to Unowned"
           );
       }
       assert!(failures_is_empty(&result.failures));
   }

   fn failures_is_empty(f: &Vec<RemoveFailure>) -> bool { f.is_empty() }
   ```

9. **Update `lib.rs::run()`** Command::Remove dispatch (around line ~396-430). The current code probably reads `result.library_entries_removed` and `result.failures`. Search and replace:
   - `result.library_entries_removed` → `result.library_entries_transitioned_to_unowned`
   - Any user-facing message about "library entries removed" should now read "library entries kept as Unowned" or similar (per `paths::collapse_home` style).
   - The grouped-failure summary section iterating `FailureKind::ALL` continues to work — just with 2 variants instead of 4.

   Specifically, find the lib.rs Command::Remove branch and update the rendering:
   ```rust
   // Find lines containing `library_entries_removed` in lib.rs and replace.
   ```
   Run `rg "library_entries_removed" crates/tome/src/lib.rs` to find affected lines and update them.

10. **Run `make ci`** at the end to confirm the `lib.rs` integration still builds and passes.
  </action>
  <verify>
    <automated>cargo test --package tome --lib remove::tests</automated>
  </verify>
  <acceptance_criteria>
    - `rg -n "library_entries_transitioned_to_unowned" crates/tome/src/remove.rs` returns at least 3 matches (struct field, increment, test assertions)
    - `rg -n "fn execute" crates/tome/src/remove.rs` returns 1 match
    - `rg -n "FailureKind::ALL.len\\(\\) == 2" crates/tome/src/remove.rs` returns 1 match (the const assert)
    - `rg -n "FailureKind::LibraryDir|FailureKind::LibrarySymlink" crates/tome/src/remove.rs` returns 0 matches (variants removed)
    - `rg -n "entry\\.source_name = None" crates/tome/src/remove.rs` returns at least 1 match (the transition site)
    - `rg -n "execute_transitions_to_unowned_and_preserves_library|execute_transitions_multiple_owned_skills_to_unowned" crates/tome/src/remove.rs` returns 2 matches
    - `rg -n "library_entries_removed" crates/tome/src/lib.rs` returns 0 matches (all updated)
    - `cargo test --package tome --lib remove::tests` exits 0
    - `cargo build --package tome` exits 0
    - `make ci` exits 0
  </acceptance_criteria>
  <done>`tome remove` transitions owned manifest entries to Unowned, preserves library content, and removes the directory from config; `RemoveResult` exposes a `library_entries_transitioned_to_unowned` counter; `FailureKind` is reduced to 2 variants since library files are no longer touched; render_plan tells the user library content is "kept as Unowned"; lib.rs call site updated; partial-failure semantics unchanged (config + manifest preserved on partial failure).</done>
</task>

</tasks>

<verification>
- `cargo test --package tome --lib cleanup::tests remove::tests` exits 0
- `cargo build --package tome` exits 0
- LIB-04 truth verified by Task 1 Test 1 and Task 2 Test 1
- D-10 trigger 1 verified by Task 2 Test 1
- D-10 trigger 2 verified by Task 1 Test 1
- D-09 case partition verified by Task 1 Tests 1, 2, 4
- `make ci` exits 0
</verification>

<success_criteria>
- LIB-04 fully addressed at both triggers (D-10): `tome remove` explicitly transitions, cleanup phase implicitly transitions for manually-edited tome.toml.
- D-09 case partition: Case 1 preserves library, Case 2 keeps today's delete behavior.
- D-11 distribution semantics preserved: Unowned skills aren't filtered out at distribute time (no changes to `distribute.rs`); they continue to flow into target dirs as today.
- Phase 14 (UNOWN-01/02) commands `tome adopt` and `tome forget` will build on this — they are NOT in scope for this plan.
- The migrate-library command (Plan 11-04) does not touch this code; the two pieces of work are orthogonal.
</success_criteria>

<output>
After completion, create `.planning/phases/11-library-canonical-core/11-03-SUMMARY.md`
documenting: cleanup_library partition logic, the new `transitioned_to_unowned`
counter, `tome remove` execute flow change, FailureKind reduction, lib.rs call site
updates, and a note pointing Phase 14 to the now-reachable Unowned manifest state.
</output>
