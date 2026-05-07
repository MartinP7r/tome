---
phase: 11-library-canonical-core
plan: 04
type: execute
wave: 3
depends_on:
  - 11-01
  - 11-02
files_modified:
  - crates/tome/src/migration_v010.rs
  - crates/tome/src/cli.rs
  - crates/tome/src/lib.rs
autonomous: true
requirements:
  - LIB-05
must_haves:
  truths:
    - "`tome migrate-library` exists as a CLI subcommand and prints a help string mentioning 'one-shot' and v0.10."
    - "`tome migrate-library --dry-run` enumerates v0.9-shape entries (qualifying per D-03) and renders a plan WITHOUT touching the filesystem."
    - "`tome migrate-library` (no flag) converts each qualifying symlink into a real directory copy of the source content, recording the original `content_hash` post-conversion to verify integrity."
    - "Entries qualify ONLY when ALL of: (a) the path at `library_dir/<name>` is a symlink, (b) `manifest[name].managed == true`, (c) `manifest.contains_key(name)` (per D-03)."
    - "Broken symlinks (target gone) are SKIPPED with a stderr warning AND PRESERVED in place — never deleted (per D-04). The library remains partially migrated; `tome sync` keeps refusing per D-02 until the user resolves manually."
    - "`tome migrate-library` exits non-zero on ANY skip (broken source) or failure (per D-05). Final summary uses SAFE-01 pattern: `⚠ N converted · K skipped (broken source) · M failed`."
    - "Re-running `tome migrate-library` after a partial run is idempotent (fresh detection scan picks up where it left off; D-06)."
    - "`tome sync` detects v0.9-shape entries (any qualifying entry per D-03) and refuses with a Conflict/Why/Suggestion error message naming `tome migrate-library` (per D-02)."
    - "User-created symlinks NOT in the manifest are NEVER touched by migrate-library (covered by D-03 condition (c))."
  artifacts:
    - path: "crates/tome/src/migration_v010.rs"
      provides: "Transitional v0.10 migration module — detection, plan/render/execute, SAFE-01 failure aggregation. Marked for v0.11+ removal."
      contains: "pub(crate) fn run_migrate_library"
      min_lines: 200
    - path: "crates/tome/src/cli.rs"
      provides: "MigrateLibrary subcommand variant with --dry-run flag"
      contains: "MigrateLibrary"
    - path: "crates/tome/src/lib.rs"
      provides: "MigrateLibrary command dispatch + sync v0.9-shape refuse-with-hint check"
      contains: "Command::MigrateLibrary"
  key_links:
    - from: "lib.rs::sync"
      to: "v0.9-shape detection (refuse-with-hint)"
      via: "isolated check before consolidate"
      pattern: "migration_v010::detect_v09_shape"
    - from: "lib.rs::run"
      to: "Command::MigrateLibrary dispatch"
      via: "match arm calling migration_v010::run_migrate_library"
      pattern: "Command::MigrateLibrary"
    - from: "migration_v010.rs::execute"
      to: "filesystem-only copy (no manifest mutation per D-06)"
      via: "remove symlink → copy_dir_recursive_resolving"
      pattern: "copy_dir_recursive"
---

<objective>
Ship `tome migrate-library` as a one-shot CLI command that converts v0.9-shape libraries
(managed skills as symlinks) to v0.10-shape (managed skills as real directory copies).

The new module `crates/tome/src/migration_v010.rs` is transitional — its module-level
doc comment marks it for removal in v0.11+ once all known users have migrated. The
file is the canonical home for everything migration does: detection (D-03), plan,
render, execute (with broken-symlink handling per D-04 and SAFE-01 failure aggregation
per D-05). Idempotent re-runs (D-06).

Also wire the v0.9-shape refuse-with-hint check into `lib.rs::sync` (D-02): `tome sync`
refuses to run on a v0.9-shape library and points the user at `tome migrate-library`.

Implements LIB-05 fully.

Wave 3 — depends on Plan 11-01 (manifest schema) and Plan 11-02 (consolidate_managed
new copy semantics, so post-migration re-sync produces the expected idempotent state).
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
@crates/tome/src/cli.rs
@crates/tome/src/lib.rs
@crates/tome/src/manifest.rs
@crates/tome/src/relocate.rs
@crates/tome/src/remove.rs
@crates/tome/src/paths.rs
@crates/tome/src/library.rs

<interfaces>
<!-- Pattern to follow: SAFE-01 failure aggregation from `crates/tome/src/remove.rs` -->
<!-- (See `FailureKind`, `RemoveFailure`, `FailureKind::ALL`, the const assert.) -->

<!-- Pattern to follow: plan/render/execute from `crates/tome/src/add.rs` and -->
<!-- `crates/tome/src/remove.rs`. Plan = compute, no I/O; render = print; execute = mutate. -->

<!-- Existing helper to reuse from `crates/tome/src/paths.rs`: -->
```rust
pub fn collapse_home(p: &Path) -> String  // for ~/-prefixed display
```

<!-- Existing helper to reuse from `crates/tome/src/manifest.rs` (post Plan 11-01): -->
```rust
pub fn hash_directory(dir: &Path) -> Result<ContentHash>
```

<!-- The new command's CLI variant signature (to be added to cli.rs): -->
```rust
/// Convert a v0.9-shape library (managed skills as symlinks) to v0.10 shape.
MigrateLibrary {
    /// Preview changes without modifying filesystem
    #[arg(long)]
    dry_run: bool,
}
```
</interfaces>
</context>

<tasks>

<task type="auto">
  <name>Task 1: Create new module `migration_v010.rs` with detection, plan, render, execute</name>
  <files>crates/tome/src/migration_v010.rs</files>
  <read_first>
    - crates/tome/src/remove.rs (FailureKind, RemoveFailure, the `const _: () = { assert!(...) };` compile-time-enforcement pattern, plan/render/execute structure — model for migration_v010)
    - crates/tome/src/relocate.rs lines 432-480 (`copy_library` recursive copy iteration pattern; migration's version RESOLVES symlinks instead of preserving them)
    - crates/tome/src/manifest.rs (post Plan 11-01 — confirm `Manifest::iter`, `hash_directory`)
    - crates/tome/src/paths.rs (`collapse_home`)
    - crates/tome/src/library.rs (`copy_dir_recursive` private helper — pattern reference for migration's copy)
    - .planning/phases/11-library-canonical-core/11-CONTEXT.md (D-01 through D-06 verbatim)
  </read_first>
  <action>
1. **Create the new file `crates/tome/src/migration_v010.rs`** with the following structure (single file, ~250 LOC, including unit tests):

   ```rust
   //! v0.9 → v0.10 library migration (transitional, one-shot).
   //!
   //! This module is **transitional**: it exists to convert v0.9-shape libraries
   //! (managed skills stored as symlinks per the pre-LIB-01 model) into v0.10-shape
   //! libraries (managed skills stored as real directory copies, per LIB-01).
   //! It is invoked exclusively by the one-shot `tome migrate-library` CLI command
   //! (per CONTEXT.md D-01) and is **slated for removal in v0.11+** once all known
   //! users have migrated. File a v0.11 follow-up issue at v0.10 ship time.
   //!
   //! Detection (D-03): a `library_dir/<name>` qualifies for migration ONLY when ALL of:
   //!   (a) the path is a symlink, AND
   //!   (b) `manifest[name].managed == true`, AND
   //!   (c) `manifest.contains_key(name)`.
   //! Never touches user-created symlinks tome didn't put there.
   //!
   //! Broken-symlink handling (D-04): broken symlinks (target gone) are SKIPPED
   //! with a stderr warning AND PRESERVED in place. The symlink target string
   //! carries metadata about where the original source lived; preserving it gives
   //! the user a chance to manually recover. Library stays partially-migrated;
   //! `tome sync` keeps refusing per D-02 until resolved.
   //!
   //! Exit code (D-05): non-zero on ANY failure (broken-symlink skip OR I/O error).
   //! Re-running is idempotent (D-06): the manifest is not mutated by migration —
   //! `source_path`, `content_hash`, `managed: true` all stay correct after the
   //! filesystem-only conversion. Detection re-runs from scratch each invocation.

   use std::path::{Path, PathBuf};

   use anyhow::{Context, Result};
   use console::style;

   use crate::manifest::{self, Manifest};
   use crate::paths::{TomePaths, collapse_home};

   // -- Failure aggregation (SAFE-01 pattern from Phase 8 / remove.rs::FailureKind) --

   /// Kinds of migration failure that can be aggregated and reported as a group.
   ///
   /// Variants are ordered for the user-facing grouped output (matches the
   /// SAFE-01 pattern in `remove.rs`). Adding a new variant requires updating
   /// `MigrationFailureKind::ALL` AND the exhaustive `_ensure_*` const fn AND
   /// the `len() ==` assertion below — symmetric to remove.rs (POLISH-04).
   #[derive(Debug, Clone, Copy, PartialEq, Eq)]
   pub(crate) enum MigrationFailureKind {
       /// Broken symlink (target gone) — SKIPPED, symlink preserved per D-04.
       /// Exits with non-zero code per D-05; re-run after resolving manually.
       BrokenSource,
       /// I/O failure during copy or symlink replacement (permission, ENOSPC, etc.).
       IoError,
   }

   impl MigrationFailureKind {
       pub(crate) const ALL: [MigrationFailureKind; 2] = [
           MigrationFailureKind::BrokenSource,
           MigrationFailureKind::IoError,
       ];

       pub(crate) fn label(self) -> &'static str {
           match self {
               MigrationFailureKind::BrokenSource => "Broken source",
               MigrationFailureKind::IoError => "I/O errors",
           }
       }
   }

   #[allow(dead_code)]
   const fn _ensure_failure_kind_all_exhaustive(k: MigrationFailureKind) -> usize {
       match k {
           MigrationFailureKind::BrokenSource => 0,
           MigrationFailureKind::IoError => 1,
       }
   }

   const _: () = {
       assert!(MigrationFailureKind::ALL.len() == 2);
   };

   #[derive(Debug)]
   pub(crate) struct MigrationFailure {
       pub kind: MigrationFailureKind,
       /// Symlink path inside the library (always absolute).
       pub path: PathBuf,
       pub error: Option<std::io::Error>,
   }

   impl MigrationFailure {
       pub(crate) fn new(
           kind: MigrationFailureKind,
           path: PathBuf,
           error: Option<std::io::Error>,
       ) -> Self {
           debug_assert!(
               path.is_absolute(),
               "MigrationFailure::path must be absolute, got: {}",
               path.display()
           );
           MigrationFailure { kind, path, error }
       }
   }

   // -- Plan / Render / Execute --

   /// A single qualifying entry detected for migration.
   #[derive(Debug, Clone)]
   pub(crate) struct MigrationEntry {
       pub skill_name: String,
       /// Path inside the library (always absolute).
       pub library_path: PathBuf,
       /// Resolved symlink target (the source content path). May be broken.
       pub raw_link_target: PathBuf,
       /// Whether the resolved target exists on disk (false = broken symlink).
       pub source_reachable: bool,
   }

   #[derive(Debug, Default)]
   pub(crate) struct MigrationPlan {
       pub entries: Vec<MigrationEntry>,
   }

   #[derive(Debug, Default)]
   pub(crate) struct MigrationResult {
       pub converted: usize,
       /// Skipped per D-04 (broken source) — counts toward non-zero exit per D-05.
       pub skipped_broken_source: usize,
       /// I/O failures during conversion.
       pub failed: usize,
       pub failures: Vec<MigrationFailure>,
   }

   impl MigrationResult {
       /// Per D-05: any skip OR failure means non-zero exit.
       pub(crate) fn is_partial_or_failed(&self) -> bool {
           self.skipped_broken_source > 0 || self.failed > 0
       }
   }

   /// Detection (D-03): a `library_dir/<name>` qualifies for migration ONLY when ALL of:
   ///   (a) the path is a symlink, AND
   ///   (b) `manifest[name].managed == true`, AND
   ///   (c) `manifest.contains_key(name)`.
   ///
   /// Note: condition (c) is structurally redundant with iterating manifest entries
   /// (we only see names that ARE in the manifest), but the call-site that checks
   /// for v0.9-shape from `lib.rs::sync` walks the library_dir directly — the
   /// check there must enforce (c) explicitly. See `detect_v09_shape` below.
   pub(crate) fn plan(library_dir: &Path, manifest: &Manifest) -> Result<MigrationPlan> {
       let mut entries = Vec::new();

       for (skill_name, skill_entry) in manifest.iter() {
           if !skill_entry.managed {
               continue;
           }
           let library_path = library_dir.join(skill_name.as_str());
           if !library_path.is_symlink() {
               continue;
           }
           // Read the symlink target (raw, not canonicalized — preserve user-visible path).
           let raw_target = std::fs::read_link(&library_path).with_context(|| {
               format!(
                   "failed to read symlink target for managed skill '{skill_name}' at {}",
                   library_path.display()
               )
           })?;
           // is_dir() on the symlink path resolves the link and checks the target;
           // false means either the target is gone OR isn't a directory.
           let source_reachable = library_path.is_dir();

           entries.push(MigrationEntry {
               skill_name: skill_name.as_str().to_string(),
               library_path,
               raw_link_target: raw_target,
               source_reachable,
           });
       }

       Ok(MigrationPlan { entries })
   }

   /// Quick check used by `lib.rs::sync` to refuse with a hint (D-02).
   /// Returns true if ANY qualifying v0.9-shape entry exists. Cheap walk;
   /// no I/O beyond `is_symlink` checks per manifest entry.
   pub(crate) fn detect_v09_shape(library_dir: &Path, manifest: &Manifest) -> bool {
       for (skill_name, skill_entry) in manifest.iter() {
           if !skill_entry.managed {
               continue;
           }
           let library_path = library_dir.join(skill_name.as_str());
           if library_path.is_symlink() {
               return true;
           }
       }
       false
   }

   pub(crate) fn render_plan(plan: &MigrationPlan) {
       println!(
           "{}",
           style("v0.9 → v0.10 library migration plan").bold()
       );
       println!();
       if plan.entries.is_empty() {
           println!(
               "  {} no v0.9-shape entries detected — library is already in v0.10 shape.",
               style("✓").green()
           );
           return;
       }

       let convertable = plan.entries.iter().filter(|e| e.source_reachable).count();
       let broken = plan.entries.len() - convertable;

       println!(
           "  Will convert {} symlink{} → real directory cop{}.",
           style(convertable).bold(),
           if convertable == 1 { "" } else { "s" },
           if convertable == 1 { "y" } else { "ies" }
       );
       if broken > 0 {
           println!(
               "  {} {} broken symlink{} will be SKIPPED and preserved (manual fix required).",
               style("⚠").yellow(),
               style(broken).bold(),
               if broken == 1 { "" } else { "s" }
           );
       }
       println!();
       for entry in &plan.entries {
           let marker = if entry.source_reachable {
               style("✓").green().to_string()
           } else {
               style("⚠").yellow().to_string()
           };
           println!(
               "  {} {} → {}",
               marker,
               style(&entry.skill_name).cyan(),
               collapse_home(&entry.raw_link_target)
           );
       }
       println!();
       println!(
           "  Note: tome does not snapshot your library before migrating. Commit your"
       );
       println!(
           "  library directory to git (or back it up some other way) BEFORE proceeding."
       );
       println!(
           "  This conversion is one-way — there is no path back to v0.9 shape."
       );
   }

   pub(crate) fn execute(plan: &MigrationPlan, dry_run: bool) -> Result<MigrationResult> {
       let mut result = MigrationResult::default();

       for entry in &plan.entries {
           if !entry.source_reachable {
               // D-04: skip with stderr warning, preserve the broken symlink.
               eprintln!(
                   "warning: skipping '{}' — symlink target {} is unreachable; preserving symlink in place",
                   entry.skill_name,
                   collapse_home(&entry.raw_link_target)
               );
               result.skipped_broken_source += 1;
               result.failures.push(MigrationFailure::new(
                   MigrationFailureKind::BrokenSource,
                   entry.library_path.clone(),
                   None,
               ));
               continue;
           }

           if dry_run {
               // No I/O — just count.
               result.converted += 1;
               continue;
           }

           // 1. Capture source content_hash before conversion (used to verify post-copy).
           //    NOTE: Add an inline comment AT THIS SITE in the produced code reading
           //    exactly:
           //        // hash_directory works on a symlink path: WalkDir follows the symlink root
           //        // even when follow_links is false, so this hashes the source content correctly.
           //    This documents the implicit assumption the executor relies on (the
           //    `library_path` is still a symlink at this point and walkdir's
           //    follow-symlink-root default is what makes hashing the *source*
           //    content possible without a manual `read_link` resolve.)
           let pre_hash = match manifest::hash_directory(&entry.library_path) {
               Ok(h) => h,
               Err(e) => {
                   eprintln!(
                       "error: could not hash source for '{}': {e}",
                       entry.skill_name
                   );
                   result.failed += 1;
                   result.failures.push(MigrationFailure::new(
                       MigrationFailureKind::IoError,
                       entry.library_path.clone(),
                       None,
                   ));
                   continue;
               }
           };

           // 2. Resolve the symlink target into an owned PathBuf so we can copy
           //    from it after removing the symlink. is_dir() already confirmed
           //    reachability above; canonicalize for safety against TOCTOU on a
           //    relative-target symlink whose CWD interpretation differs from
           //    library_path's parent.
           let resolved_source = match std::fs::canonicalize(&entry.library_path) {
               Ok(p) => p,
               Err(e) => {
                   eprintln!(
                       "error: could not canonicalize source for '{}': {e}",
                       entry.skill_name
                   );
                   result.failed += 1;
                   result.failures.push(MigrationFailure::new(
                       MigrationFailureKind::IoError,
                       entry.library_path.clone(),
                       Some(e),
                   ));
                   continue;
               }
           };

           // 3. Remove the symlink (NOT the target — remove_file unlinks the
           //    symlink itself even if the target is a directory).
           if let Err(e) = std::fs::remove_file(&entry.library_path) {
               eprintln!(
                   "error: could not remove symlink for '{}': {e}",
                   entry.skill_name
               );
               result.failed += 1;
               result.failures.push(MigrationFailure::new(
                   MigrationFailureKind::IoError,
                   entry.library_path.clone(),
                   Some(e),
               ));
               continue;
           }

           // 4. Recursive copy from resolved source → library_path. Resolves
           //    nested symlinks (follow_links(true)) so the library is fully
           //    materialized with no symlink content.
           if let Err(e) = copy_dir_recursive_resolving(&resolved_source, &entry.library_path) {
               eprintln!(
                   "error: copy failed for '{}': {e:#}",
                   entry.skill_name
               );
               result.failed += 1;
               result.failures.push(MigrationFailure::new(
                   MigrationFailureKind::IoError,
                   entry.library_path.clone(),
                   None,
               ));
               continue;
           }

           // 5. Verify content_hash after copy matches pre-copy hash. If not,
           //    flag IoError but don't roll back — the user can re-run.
           match manifest::hash_directory(&entry.library_path) {
               Ok(post_hash) if post_hash == pre_hash => {
                   result.converted += 1;
               }
               Ok(post_hash) => {
                   eprintln!(
                       "error: content_hash mismatch for '{}' after copy (expected {pre_hash}, got {post_hash})",
                       entry.skill_name
                   );
                   result.failed += 1;
                   result.failures.push(MigrationFailure::new(
                       MigrationFailureKind::IoError,
                       entry.library_path.clone(),
                       None,
                   ));
               }
               Err(e) => {
                   eprintln!(
                       "error: could not hash post-copy library for '{}': {e}",
                       entry.skill_name
                   );
                   result.failed += 1;
                   result.failures.push(MigrationFailure::new(
                       MigrationFailureKind::IoError,
                       entry.library_path.clone(),
                       None,
                   ));
               }
           }
       }

       Ok(result)
   }

   /// Recursive copy that RESOLVES symlinks (follows them) — opposite of
   /// `relocate.rs::copy_library` (which preserves them). For migration we
   /// want a fully-materialized library with no symlinks.
   fn copy_dir_recursive_resolving(src: &Path, dst: &Path) -> Result<()> {
       std::fs::create_dir_all(dst)
           .with_context(|| format!("failed to create {}", dst.display()))?;
       for entry in walkdir::WalkDir::new(src).follow_links(true).into_iter() {
           let entry = entry
               .with_context(|| format!("failed to walk source {}", src.display()))?;
           let rel = entry.path().strip_prefix(src).with_context(|| {
               format!(
                   "BUG: WalkDir yielded path {} not under root {}",
                   entry.path().display(),
                   src.display()
               )
           })?;
           let target = dst.join(rel);
           if entry.file_type().is_dir() {
               std::fs::create_dir_all(&target)
                   .with_context(|| format!("failed to create {}", target.display()))?;
           } else if entry.file_type().is_file() {
               if let Some(parent) = target.parent() {
                   std::fs::create_dir_all(parent)
                       .with_context(|| format!("failed to create parent {}", parent.display()))?;
               }
               std::fs::copy(entry.path(), &target).with_context(|| {
                   format!(
                       "failed to copy {} -> {}",
                       entry.path().display(),
                       target.display()
                   )
               })?;
           }
           // entry.file_type().is_symlink() can't fire because follow_links(true)
           // resolves before yielding — the entry's file_type() reflects the target.
       }
       Ok(())
   }

   /// Render the SAFE-01 grouped failure summary + final ✓/⚠ banner.
   fn render_result(result: &MigrationResult) {
       println!();
       let banner = format!(
           "⚠ {} converted · {} skipped (broken source) · {} failed",
           result.converted, result.skipped_broken_source, result.failed,
       );
       if result.is_partial_or_failed() {
           println!("{}", style(&banner).yellow().bold());
       } else {
           println!(
               "{}",
               style(format!(
                   "✓ {} skill{} migrated to v0.10 shape",
                   result.converted,
                   if result.converted == 1 { "" } else { "s" }
               ))
               .green()
               .bold()
           );
       }

       if result.failures.is_empty() {
           return;
       }

       // Group by kind in `MigrationFailureKind::ALL` order (POLISH-04 pattern).
       for kind in MigrationFailureKind::ALL.iter().copied() {
           let group: Vec<&MigrationFailure> =
               result.failures.iter().filter(|f| f.kind == kind).collect();
           if group.is_empty() {
               continue;
           }
           println!();
           println!(
               "  {} ({}):",
               style(kind.label()).yellow().bold(),
               group.len()
           );
           for f in group {
               match &f.error {
                   Some(e) => println!("    {} ({e})", collapse_home(&f.path)),
                   None => println!("    {}", collapse_home(&f.path)),
               }
           }
       }
   }

   /// Top-level entry: run the full plan/render/execute flow.
   ///
   /// Per D-05, returns Ok(MigrationResult) regardless of partial failure;
   /// the caller in `lib.rs` interprets `is_partial_or_failed()` and exits
   /// with code 1 on partial. Hard errors (unparseable manifest, etc.)
   /// surface as Err.
   pub(crate) fn run_migrate_library(paths: &TomePaths, dry_run: bool) -> Result<MigrationResult> {
       if dry_run {
           eprintln!(
               "{}",
               style("[dry-run] No changes will be made").yellow().bold()
           );
       }

       let manifest = manifest::load(paths.config_dir())?;
       let plan = plan(paths.library_dir(), &manifest)?;
       render_plan(&plan);

       let result = execute(&plan, dry_run)?;
       render_result(&result);
       Ok(result)
   }

   #[cfg(test)]
   mod tests {
       use super::*;
       use crate::config::DirectoryName;
       use crate::discover::SkillName;
       use crate::manifest::{Manifest, SkillEntry};
       use crate::validation::test_hash;
       use std::os::unix::fs as unix_fs;
       use tempfile::TempDir;

       fn setup_fixture() -> (TempDir, PathBuf, PathBuf, Manifest) {
           let tmp = TempDir::new().unwrap();
           let library = tmp.path().join("library");
           let source = tmp.path().join("source");
           std::fs::create_dir_all(&library).unwrap();
           std::fs::create_dir_all(&source).unwrap();
           let manifest = Manifest::default();
           (tmp, library, source, manifest)
       }

       fn make_managed_source(source_root: &Path, name: &str, body: &str) -> PathBuf {
           let dir = source_root.join(name);
           std::fs::create_dir_all(&dir).unwrap();
           std::fs::write(dir.join("SKILL.md"), body).unwrap();
           dir
       }

       fn add_managed_entry(
           manifest: &mut Manifest,
           library: &Path,
           source: &Path,
           name: &str,
       ) {
           // Create the v0.9-shape symlink in library.
           unix_fs::symlink(source, library.join(name)).unwrap();
           let hash = manifest::hash_directory(source).unwrap();
           manifest.insert(
               SkillName::new(name).unwrap(),
               SkillEntry::new(
                   source.to_path_buf(),
                   DirectoryName::new("plugins").unwrap(),
                   hash,
                   true,
               ),
           );
       }

       #[test]
       fn plan_detects_managed_symlinks_in_manifest() {
           let (_tmp, library, source, mut manifest) = setup_fixture();
           let src1 = make_managed_source(&source, "p1", "# p1");
           let src2 = make_managed_source(&source, "p2", "# p2");
           add_managed_entry(&mut manifest, &library, &src1, "p1");
           add_managed_entry(&mut manifest, &library, &src2, "p2");

           let p = plan(&library, &manifest).unwrap();
           assert_eq!(p.entries.len(), 2);
           assert!(p.entries.iter().all(|e| e.source_reachable));
       }

       #[test]
       fn plan_skips_user_created_symlink_not_in_manifest() {
           // D-03 condition (c): never touch user-created symlinks.
           let (_tmp, library, source, manifest) = setup_fixture();
           let user_target = make_managed_source(&source, "user", "# user");
           unix_fs::symlink(&user_target, library.join("user-symlink")).unwrap();

           let p = plan(&library, &manifest).unwrap();
           assert!(p.entries.is_empty(), "user-created symlink must NOT be in plan");
       }

       #[test]
       fn plan_skips_local_skills_not_managed() {
           // D-03 condition (b): only managed entries qualify.
           let (_tmp, library, source, mut manifest) = setup_fixture();
           let src = make_managed_source(&source, "local", "# local");
           unix_fs::symlink(&src, library.join("local")).unwrap();
           let hash = manifest::hash_directory(&src).unwrap();
           manifest.insert(
               SkillName::new("local").unwrap(),
               SkillEntry::new(
                   src,
                   DirectoryName::new("dir").unwrap(),
                   hash,
                   false, // <-- managed: false
               ),
           );

           let p = plan(&library, &manifest).unwrap();
           assert!(p.entries.is_empty(), "non-managed entries must NOT qualify");
       }

       #[test]
       fn plan_handles_broken_symlink() {
           let (_tmp, library, _source, mut manifest) = setup_fixture();
           // Create a managed symlink whose target is gone.
           unix_fs::symlink("/nonexistent/path", library.join("broken")).unwrap();
           manifest.insert(
               SkillName::new("broken").unwrap(),
               SkillEntry::new(
                   PathBuf::from("/nonexistent/path"),
                   DirectoryName::new("plugins").unwrap(),
                   test_hash("broken"),
                   true,
               ),
           );

           let p = plan(&library, &manifest).unwrap();
           assert_eq!(p.entries.len(), 1);
           assert!(!p.entries[0].source_reachable);
       }

       #[test]
       fn execute_converts_managed_symlink_to_real_dir() {
           let (_tmp, library, source, mut manifest) = setup_fixture();
           let src = make_managed_source(&source, "p1", "# p1 content");
           add_managed_entry(&mut manifest, &library, &src, "p1");

           let p = plan(&library, &manifest).unwrap();
           let result = execute(&p, false).unwrap();
           assert_eq!(result.converted, 1);
           assert_eq!(result.skipped_broken_source, 0);
           assert_eq!(result.failed, 0);

           let dest = library.join("p1");
           assert!(dest.is_dir(), "should be a real directory");
           assert!(!dest.is_symlink(), "should NOT be a symlink");
           let content = std::fs::read_to_string(dest.join("SKILL.md")).unwrap();
           assert_eq!(content, "# p1 content");
       }

       #[test]
       fn execute_preserves_broken_symlink_d04() {
           let (_tmp, library, _source, mut manifest) = setup_fixture();
           unix_fs::symlink("/nonexistent/path", library.join("broken")).unwrap();
           manifest.insert(
               SkillName::new("broken").unwrap(),
               SkillEntry::new(
                   PathBuf::from("/nonexistent/path"),
                   DirectoryName::new("plugins").unwrap(),
                   test_hash("broken"),
                   true,
               ),
           );

           let p = plan(&library, &manifest).unwrap();
           let result = execute(&p, false).unwrap();
           assert_eq!(result.converted, 0);
           assert_eq!(result.skipped_broken_source, 1);
           assert!(result.is_partial_or_failed(), "D-05 non-zero exit on skip");

           // D-04: broken symlink preserved on disk.
           assert!(library.join("broken").is_symlink(), "broken symlink must be preserved");
           // Manifest unchanged (D-06).
           assert!(manifest.contains_key("broken"));
           assert!(manifest.get("broken").unwrap().managed);
       }

       #[test]
       fn execute_dry_run_changes_nothing() {
           let (_tmp, library, source, mut manifest) = setup_fixture();
           let src = make_managed_source(&source, "p1", "# p1");
           add_managed_entry(&mut manifest, &library, &src, "p1");

           let p = plan(&library, &manifest).unwrap();
           let result = execute(&p, true).unwrap();
           assert_eq!(result.converted, 1);

           // No filesystem mutation — symlink still in place.
           assert!(library.join("p1").is_symlink(), "dry-run must not convert");
       }

       #[test]
       fn execute_idempotent_on_re_run() {
           let (_tmp, library, source, mut manifest) = setup_fixture();
           let src = make_managed_source(&source, "p1", "# p1");
           add_managed_entry(&mut manifest, &library, &src, "p1");

           // First run.
           let p = plan(&library, &manifest).unwrap();
           let r1 = execute(&p, false).unwrap();
           assert_eq!(r1.converted, 1);

           // Second run — fresh detection finds nothing (no more symlinks).
           let p2 = plan(&library, &manifest).unwrap();
           assert!(p2.entries.is_empty(), "re-run plan must be empty");
           let r2 = execute(&p2, false).unwrap();
           assert_eq!(r2.converted, 0);
           assert_eq!(r2.skipped_broken_source, 0);
           assert_eq!(r2.failed, 0);
           assert!(!r2.is_partial_or_failed(), "idempotent re-run must succeed cleanly");
       }

       #[test]
       fn detect_v09_shape_returns_true_when_managed_symlink_present() {
           let (_tmp, library, source, mut manifest) = setup_fixture();
           let src = make_managed_source(&source, "p1", "# p1");
           add_managed_entry(&mut manifest, &library, &src, "p1");
           assert!(detect_v09_shape(&library, &manifest));
       }

       #[test]
       fn detect_v09_shape_returns_false_when_library_empty() {
           let (_tmp, library, _source, manifest) = setup_fixture();
           assert!(!detect_v09_shape(&library, &manifest));
       }

       #[test]
       fn detect_v09_shape_returns_false_when_managed_already_real_dir() {
           let (_tmp, library, source, mut manifest) = setup_fixture();
           let src = make_managed_source(&source, "p1", "# p1");
           // Real dir copy in library (v0.10 shape) + managed manifest entry.
           crate::manifest::hash_directory(&src).unwrap();
           let dst = library.join("p1");
           std::fs::create_dir_all(&dst).unwrap();
           std::fs::write(dst.join("SKILL.md"), "# p1").unwrap();
           let hash = manifest::hash_directory(&dst).unwrap();
           manifest.insert(
               SkillName::new("p1").unwrap(),
               SkillEntry::new(src, DirectoryName::new("plugins").unwrap(), hash, true),
           );
           assert!(!detect_v09_shape(&library, &manifest));
       }

       #[test]
       fn migration_failure_kind_all_pinned() {
           assert_eq!(MigrationFailureKind::ALL.len(), 2);
           assert_eq!(
               MigrationFailureKind::ALL,
               [MigrationFailureKind::BrokenSource, MigrationFailureKind::IoError]
           );
       }

       #[test]
       fn migration_failure_kind_labels() {
           assert_eq!(MigrationFailureKind::BrokenSource.label(), "Broken source");
           assert_eq!(MigrationFailureKind::IoError.label(), "I/O errors");
       }
   }
   ```

2. **Wire the module into `lib.rs::pub(crate) mod` declarations.** In `crates/tome/src/lib.rs`, add the line:
   ```rust
   pub(crate) mod migration_v010;
   ```
   Place it alphabetically between `manifest` and `paths` (the existing module list is alphabetical).

3. **Add the `pre_hash` site inline comment in `execute()`.** When you write the `execute()` body, ensure that immediately above the `let pre_hash = match manifest::hash_directory(&entry.library_path) { ... }` line you place the following comment exactly:
   ```rust
   // hash_directory works on a symlink path: WalkDir follows the symlink root
   // even when follow_links is false, so this hashes the source content correctly.
   ```
   Rationale: at this point `library_path` is still a v0.9-shape symlink. `hash_directory` calls `WalkDir::new(library_path).follow_links(false)`, but walkdir follows the **symlink root** by default even when `follow_links(false)` (only nested symlinks are skipped). So hashing succeeds against the source content the symlink resolves to. The comment makes this implicit assumption explicit so a future reader doesn't get confused or "fix" it into a `read_link + hash_directory(target)` two-step.
  </action>
  <verify>
    <automated>cargo test --package tome --lib migration_v010::tests</automated>
  </verify>
  <acceptance_criteria>
    - `crates/tome/src/migration_v010.rs` exists and contains module-level doc comment with substring "transitional" AND "v0.11+" AND "D-01"
    - `rg -n "pub\\(crate\\) fn run_migrate_library" crates/tome/src/migration_v010.rs` returns 1 match
    - `rg -n "pub\\(crate\\) fn detect_v09_shape" crates/tome/src/migration_v010.rs` returns 1 match
    - `rg -n "pub\\(crate\\) fn plan|pub\\(crate\\) fn execute|pub\\(crate\\) fn render_plan" crates/tome/src/migration_v010.rs` returns 3 matches
    - `rg -n "MigrationFailureKind::ALL" crates/tome/src/migration_v010.rs` returns at least 2 matches (definition + iteration in render_result)
    - `rg -n "assert!\\(MigrationFailureKind::ALL\\.len\\(\\) == 2\\)" crates/tome/src/migration_v010.rs` returns 1 match (compile-time check)
    - `rg -n "fn copy_dir_recursive_resolving" crates/tome/src/migration_v010.rs` returns 1 match
    - `rg -n "follow_links\\(true\\)" crates/tome/src/migration_v010.rs` returns 1 match (resolves symlinks during copy, opposite of relocate.rs)
    - `rg -n "pub\\(crate\\) mod migration_v010;" crates/tome/src/lib.rs` returns 1 match
    - `rg -n "WalkDir follows the symlink root" crates/tome/src/migration_v010.rs` returns 1 match (the inline comment above the `pre_hash` site documenting the implicit assumption)
    - execute() body in migration_v010.rs contains the comment "WalkDir follows the symlink root"
    - `cargo test --package tome --lib migration_v010::tests` exits 0 (all 11 unit tests pass)
    - `cargo build --package tome` exits 0
  </acceptance_criteria>
  <done>migration_v010.rs created with detection (D-03), plan/render/execute (D-04 broken-symlink preserve, D-05 SAFE-01 failure aggregation, D-06 idempotency); module wired into lib.rs; the `pre_hash` site has an inline comment documenting why `hash_directory` works on a symlink path (walkdir follows the symlink root); comprehensive unit tests covering plan, detection variations, conversion, broken-symlink preservation, dry-run, idempotent re-run, and the compile-time MigrationFailureKind::ALL guard.</done>
</task>

<task type="auto">
  <name>Task 2: Wire `Command::MigrateLibrary` into CLI and add v0.9-shape refuse-with-hint check to `lib.rs::sync`</name>
  <files>crates/tome/src/cli.rs, crates/tome/src/lib.rs</files>
  <read_first>
    - crates/tome/src/cli.rs (existing Command enum, after_help convention)
    - crates/tome/src/lib.rs (run() command dispatch, sync() body — specifically the section between consolidate and the rest of sync)
    - .planning/phases/11-library-canonical-core/11-CONTEXT.md (D-01, D-02, D-05)
    - crates/tome/src/migration_v010.rs (Task 1 output — `run_migrate_library`, `detect_v09_shape`)
  </read_first>
  <action>
1. **Add `Command::MigrateLibrary` to `cli.rs`.** Find the `Command` enum (around line 56). Add this variant in alphabetical position (between `Lint` and `Init`/`Reassign`/`Relocate` — placement doesn't affect behavior, but match existing alphabetical or logical ordering. Place it just after `Lint` and before `Browse` since Init is far above):

   ```rust
       /// One-shot migration: convert a v0.9-shape library (managed skills as
       /// symlinks) to v0.10 shape (real directory copies). Run once after
       /// upgrading from v0.9.x. Idempotent on re-run.
       ///
       /// Commit your library (or back it up) BEFORE running — there is no
       /// path back to v0.9 shape.
       #[command(
           after_help = "Examples:\n  tome migrate-library --dry-run\n  tome migrate-library\n\nThis is a one-shot command for migrating from tome v0.9.x to v0.10. \
                          On v0.10 fresh installs it has nothing to do."
       )]
       MigrateLibrary {
           /// Preview changes without modifying filesystem
           #[arg(long)]
           dry_run: bool,
       },
   ```

2. **Add the dispatch in `lib.rs::run()`** Command match. Find the existing arm pattern (e.g. `Command::Eject => { ... }`) and add a new arm:

   ```rust
           Command::MigrateLibrary { dry_run } => {
               // Per D-05: any skip or failure means non-zero exit. The
               // run_migrate_library helper returns Ok(result) on partial;
               // we interpret here and `process::exit(1)` on partial-or-failed.
               let result = migration_v010::run_migrate_library(&paths, dry_run || cli.dry_run)?;
               if result.is_partial_or_failed() {
                   std::process::exit(1);
               }
           }
   ```

   Place it adjacent to other one-shot maintenance commands (e.g. near `Command::Eject` or `Command::Relocate`). Order doesn't affect runtime; pick a logical spot.

3. **Add the v0.9-shape refuse-with-hint check to `lib.rs::sync`** (D-02). Locate the line in `sync()` where consolidate is invoked:

   ```rust
       // 2. Consolidate into library (copy)
       let sp = show_progress.then(|| spinner("Consolidating to library..."));
       if verbose {
           eprintln!("{}", style("Consolidating to library...").dim());
       }
       let (consolidate_result, mut manifest) = library::consolidate(&skills, paths, dry_run, force)?;
   ```

   IMMEDIATELY BEFORE `library::consolidate`, add the check. The manifest must be loaded for detection (consolidate loads it internally; we load it here too for the check):

   ```rust
       // v0.10 D-02: refuse to sync against a v0.9-shape library. Detection is an
       // isolated check; the entire migration_v010 module deletes cleanly with
       // this check in v0.11+.
       {
           let manifest_for_detection = manifest::load(paths.config_dir())?;
           if migration_v010::detect_v09_shape(paths.library_dir(), &manifest_for_detection) {
               anyhow::bail!(
                   "library is in v0.9 shape (one or more managed skills are stored as symlinks).\n\
                    \n\
                    Why: v0.10 stores managed skills as real directory copies (LIB-01).\n\
                    Run `tome migrate-library` to convert the library, then re-run this command.\n\
                    Pass `--dry-run` first to preview changes without touching the filesystem."
               );
           }
       }

       // 2. Consolidate into library (copy)
       let sp = show_progress.then(|| spinner("Consolidating to library..."));
       ...
   ```

   This message follows the Conflict / Why / Suggestion template from Phase 7 D-10 (existing convention in this codebase — search `crates/tome/src/config.rs` for examples if needed, but the structure is: error line, then "Why:" line, then suggested action line).

4. **Confirm dispatch reaches every required surface.** Run `cargo build --package tome` and confirm clean compile. Run `cargo run -- migrate-library --help` (in a way that builds): the help text should appear with the after_help block including "one-shot".

5. **Quick smoke test using the binary.** This is a verification-only step; no test harness needed:
   ```bash
   cargo build --package tome
   ./target/debug/tome migrate-library --help | grep -i "one-shot"
   ./target/debug/tome migrate-library --dry-run --help  # validates --dry-run flag exists
   ```
  </action>
  <verify>
    <automated>cargo build --package tome && ./target/debug/tome migrate-library --help | grep -q "one-shot"</automated>
  </verify>
  <acceptance_criteria>
    - `rg -n "MigrateLibrary" crates/tome/src/cli.rs` returns at least 2 matches (variant declaration + after_help string)
    - `rg -n "Command::MigrateLibrary" crates/tome/src/lib.rs` returns 1 match (dispatch arm)
    - `rg -n "migration_v010::run_migrate_library" crates/tome/src/lib.rs` returns 1 match
    - `rg -n "migration_v010::detect_v09_shape" crates/tome/src/lib.rs` returns 1 match (in sync's refuse-with-hint check)
    - `rg -n "library is in v0.9 shape" crates/tome/src/lib.rs` returns 1 match
    - `cargo build --package tome` exits 0
    - `./target/debug/tome migrate-library --help` exits 0 AND output contains the substring "one-shot"
    - `./target/debug/tome migrate-library --help` output contains "--dry-run"
  </acceptance_criteria>
  <done>`tome migrate-library` is a real CLI command with `--dry-run`; `lib.rs::sync` refuses to run on v0.9-shape libraries with a Conflict/Why/Suggestion error pointing at the new command; partial-or-failed migration exits non-zero per D-05.</done>
</task>

</tasks>

<verification>
- `cargo test --package tome --lib migration_v010::tests` exits 0 (all unit tests including detection variations, broken-symlink preservation, and idempotent re-run)
- `cargo build --package tome` exits 0
- `./target/debug/tome migrate-library --help | grep -q "one-shot"` exits 0
- `make ci` exits 0
</verification>

<success_criteria>
- LIB-05 fully addressed: one-shot CLI command with detection per D-03, broken-symlink handling per D-04, SAFE-01 failure aggregation per D-05, idempotent re-run per D-06.
- D-02 refuse-with-hint wired into sync: any v0.9-shape detection triggers the Conflict / Why / Suggestion error.
- D-01 module structure: `migration_v010.rs` is a clean self-contained file with module-level "remove in v0.11+" doc comment.
- The integration tests in Plan 11-05 will exercise this end-to-end with a synthetic v0.9 library fixture.
</success_criteria>

<output>
After completion, create `.planning/phases/11-library-canonical-core/11-04-SUMMARY.md`
documenting: new migration_v010 module surface, the v0.9 detection rules (D-03 ALL conditions),
broken-symlink preservation rationale (D-04), SAFE-01 failure aggregation (D-05 — ⚠ N converted ·
K skipped · M failed banner), the lib.rs::sync refuse-with-hint integration point, and a
v0.11 follow-up reminder ("delete migration_v010.rs and the sync v0.9-shape check").
</output>
</content>
</invoke>