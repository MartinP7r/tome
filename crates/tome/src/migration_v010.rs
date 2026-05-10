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

use crate::manifest::Manifest;
use crate::paths::collapse_home;

// -- Byte-size helpers (D-UX02-3 / D-UX02-4) --

/// Walk `source` and sum `metadata().len()` for every regular file.
///
/// Uses `WalkDir::follow_links(false)` per D-UX02-4 to avoid double-counting
/// nested symlinked subdirectories. Errors during walk or metadata are
/// silently treated as zero contribution — this is a UX size estimate, not a
/// correctness-critical measurement. Saturating arithmetic guards against
/// pathological accumulation overflow on enormous libraries.
fn walk_byte_size(source: &Path) -> u64 {
    let mut total: u64 = 0;
    for entry in walkdir::WalkDir::new(source)
        .follow_links(false)
        .into_iter()
        .flatten()
    {
        if entry.file_type().is_file()
            && let Ok(meta) = entry.metadata()
        {
            total = total.saturating_add(meta.len());
        }
    }
    total
}

/// Render a byte count in the largest sensible binary unit (B / KB / MB /
/// GB / TB). Inline helper rather than the `humansize` crate per CONTEXT.md
/// `<decisions>` "Claude's Discretion" — minimises dep growth for ~10 LOC.
fn humanize_bytes(bytes: u64) -> String {
    const UNITS: &[&str] = &["B", "KB", "MB", "GB", "TB"];
    let mut value = bytes as f64;
    let mut unit_idx = 0;
    while value >= 1024.0 && unit_idx < UNITS.len() - 1 {
        value /= 1024.0;
        unit_idx += 1;
    }
    if unit_idx == 0 {
        format!("{} {}", bytes, UNITS[0])
    } else {
        format!("{:.1} {}", value, UNITS[unit_idx])
    }
}

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
    /// Sum of `metadata().len()` for every regular file under the resolved
    /// source. `Some(bytes)` when `source_reachable`; `None` when broken.
    /// Walks with `follow_links(false)` per D-UX02-4 to avoid double-counting
    /// nested symlinked subdirs. Populated by `plan()`; consumed by
    /// `render_plan_to` for the disk-estimate summary line + per-skill SIZE
    /// column.
    pub byte_size: Option<u64>,
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

/// Migration command failure marker (HARD-04 sibling).
///
/// Bubbled through `anyhow::Result` from `cmd_migrate_library` when the
/// migration result is partial-or-failed (D-05). Pinned with a typed
/// error so `main.rs` can downcast and exit 1 instead of the library
/// calling `process::exit(1)` directly.
#[derive(Debug)]
pub struct MigrationPartialOrFailed {
    pub skipped_broken_source: usize,
    pub failed: usize,
}

impl std::fmt::Display for MigrationPartialOrFailed {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "migrate-library finished with skips or failures (skipped: {}, failed: {})",
            self.skipped_broken_source, self.failed,
        )
    }
}

impl std::error::Error for MigrationPartialOrFailed {}

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

        // D-UX02-4: walk the resolved source to estimate disk impact for the
        // confirmation prompt summary. Walking `library_path` (a symlink in
        // v0.9 shape) follows through to the real source content; we want
        // `follow_links(false)` on the *walk* so nested symlinked subdirs
        // aren't double-counted, but the top-level symlink IS resolved by
        // `WalkDir::new()` itself so the walk still reaches real content.
        let byte_size = if source_reachable {
            Some(walk_byte_size(&library_path))
        } else {
            None
        };

        entries.push(MigrationEntry {
            skill_name: skill_name.as_str().to_string(),
            library_path,
            raw_link_target: raw_target,
            source_reachable,
            byte_size,
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

/// Render the migration plan into `w`. Per UX-02 D-UX02-3 the output is:
///
/// 1. Bold "v0.9 → v0.10 library migration plan" header.
/// 2. Bold inline summary line:
///    `Will convert N symlinks → real directories (~X.Y MB additional disk).`
/// 3. Optional broken-symlink warning line.
/// 4. `tabled::Style::rounded()` four-column table:
///    `SKILL | SOURCE | SIZE | STATUS`.
/// 5. Closing note about non-reversibility.
///
/// Empty-plan path emits the existing already-in-v0.10-shape message.
pub(crate) fn render_plan_to(
    plan: &MigrationPlan,
    w: &mut impl std::io::Write,
) -> std::io::Result<()> {
    writeln!(w, "{}", style("v0.9 → v0.10 library migration plan").bold())?;
    writeln!(w)?;
    if plan.entries.is_empty() {
        writeln!(
            w,
            "  {} no v0.9-shape entries detected — library is already in v0.10 shape.",
            style("✓").green()
        )?;
        return Ok(());
    }

    let convertible = plan.entries.iter().filter(|e| e.source_reachable).count();
    let broken = plan.entries.len() - convertible;
    let total_bytes: u64 = plan
        .entries
        .iter()
        .filter(|e| e.source_reachable)
        .filter_map(|e| e.byte_size)
        .sum();

    // D-UX02-3 bold summary line. Locks the wording cited by DOC-02.
    writeln!(
        w,
        "  {}",
        style(format!(
            "Will convert {} symlink{} → real director{} (~{} additional disk).",
            convertible,
            if convertible == 1 { "" } else { "s" },
            if convertible == 1 { "y" } else { "ies" },
            humanize_bytes(total_bytes),
        ))
        .bold()
    )?;
    if broken > 0 {
        writeln!(
            w,
            "  {} {} broken symlink{} will be SKIPPED and preserved (manual fix required).",
            style("⚠").yellow(),
            style(broken).bold(),
            if broken == 1 { "" } else { "s" }
        )?;
    }
    writeln!(w)?;

    // D-UX02-3 four-column tabled summary; Style::rounded() per WHARD-07.
    use tabled::{Table, settings::Style};
    #[derive(tabled::Tabled)]
    struct Row {
        #[tabled(rename = "SKILL")]
        skill: String,
        #[tabled(rename = "SOURCE")]
        source: String,
        #[tabled(rename = "SIZE")]
        size: String,
        #[tabled(rename = "STATUS")]
        status: String,
    }
    let rows: Vec<Row> = plan
        .entries
        .iter()
        .map(|e| Row {
            skill: e.skill_name.clone(),
            source: collapse_home(&e.raw_link_target),
            size: e
                .byte_size
                .map(humanize_bytes)
                .unwrap_or_else(|| "—".into()),
            status: if e.source_reachable {
                "✓".into()
            } else {
                "⚠".into()
            },
        })
        .collect();
    let mut t = Table::new(rows);
    t.with(Style::rounded());
    writeln!(w, "{t}")?;
    writeln!(w)?;
    writeln!(
        w,
        "  Note: tome does not snapshot your library before migrating. Commit your"
    )?;
    writeln!(
        w,
        "  library directory to git (or back it up some other way) BEFORE proceeding."
    )?;
    writeln!(
        w,
        "  This conversion is one-way — there is no path back to v0.9 shape."
    )?;
    Ok(())
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

        // 1. Resolve the symlink target into an owned PathBuf so we can copy
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

        // 2. Remove the symlink (NOT the target — remove_file unlinks the
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

        // 3. Recursive copy from resolved source → library_path. Resolves
        //    nested symlinks (follow_links(true)) so the library is fully
        //    materialized with no symlink content.
        //
        // No post-copy hash check: copy_dir_recursive_resolving returning
        // Ok(()) already implies every file copied successfully (each
        // std::fs::copy is checked individually). A pre-vs-post hash
        // comparison would also be incorrect: hash_directory uses
        // WalkDir::follow_links(false) which treats nested symlinks as
        // opaque entries, while copy_dir_recursive_resolving uses
        // follow_links(true) which materializes them — so a source with
        // any nested directory symlink would always compare unequal even
        // on a perfectly correct copy. (See #515 for the false-failure
        // mode this caused before.)
        if let Err(e) = copy_dir_recursive_resolving(&resolved_source, &entry.library_path) {
            eprintln!("error: copy failed for '{}': {e:#}", entry.skill_name);
            result.failed += 1;
            result.failures.push(MigrationFailure::new(
                MigrationFailureKind::IoError,
                entry.library_path.clone(),
                None,
            ));
            continue;
        }

        result.converted += 1;
    }

    Ok(result)
}

/// Recursive copy that RESOLVES symlinks (follows them) — opposite of
/// `relocate.rs::copy_library` (which preserves them). For migration we
/// want a fully-materialized library with no symlinks.
fn copy_dir_recursive_resolving(src: &Path, dst: &Path) -> Result<()> {
    std::fs::create_dir_all(dst).with_context(|| format!("failed to create {}", dst.display()))?;
    for entry in walkdir::WalkDir::new(src).follow_links(true).into_iter() {
        let entry = entry.with_context(|| format!("failed to walk source {}", src.display()))?;
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

/// Confirm-or-abort gate before destructive migration (UX-02 D-UX02-1/-2).
///
/// Three-arm behaviour matrix:
/// - `yes=true`               → `Ok(true)` — skip prompt, proceed
///   (CI-friendly; mirrors Phase 14 D-B3 `tome remove skill --yes`).
/// - `yes=false, no_input=true`  → `Err(_)` — refuses to silently mutate
///   under `--no-input`. Phase 7 D-10 Conflict/Why/Suggestion shape.
/// - `yes=false, no_input=false` → `dialoguer::Confirm::default(false)`;
///   pressing anything other than `y` aborts cleanly (no mutation).
///
/// The interactive arm is intentionally not unit-tested here (dialoguer
/// requires a TTY); the abort-leaves-library-untouched invariant is
/// covered by the `cli_migrate_library` integration tests landing in
/// Task 3.
pub(crate) fn prompt_confirmation(yes: bool, no_input: bool) -> Result<bool> {
    if yes {
        return Ok(true);
    }
    if no_input {
        anyhow::bail!(
            "tome migrate-library is destructive (converts symlinks to real copies).\n  \
             Why: --no-input mode skips the confirmation prompt; --yes is required to confirm.\n  \
             Suggestion: re-run with `--yes` to proceed, or remove `--no-input` for the interactive prompt."
        );
    }
    let confirmed = dialoguer::Confirm::new()
        .with_prompt("Proceed with migration?")
        .default(false)
        .interact_opt()?;
    Ok(confirmed.unwrap_or(false))
}

/// Render the SAFE-01 grouped failure summary + final ✓/⚠ banner into `w`.
/// Per HARD-15 stderr discipline, production callers pass an
/// `io::stderr().lock()` writer.
pub(crate) fn render_result_to(
    result: &MigrationResult,
    w: &mut impl std::io::Write,
) -> std::io::Result<()> {
    writeln!(w)?;
    let banner = format!(
        "⚠ {} converted · {} skipped (broken source) · {} failed",
        result.converted, result.skipped_broken_source, result.failed,
    );
    if result.is_partial_or_failed() {
        writeln!(w, "{}", style(&banner).yellow().bold())?;
    } else {
        writeln!(
            w,
            "{}",
            style(format!(
                "✓ {} skill{} migrated to v0.10 shape",
                result.converted,
                if result.converted == 1 { "" } else { "s" }
            ))
            .green()
            .bold()
        )?;
    }

    if result.failures.is_empty() {
        return Ok(());
    }

    // Group by kind in `MigrationFailureKind::ALL` order (POLISH-04 pattern).
    for kind in MigrationFailureKind::ALL.iter().copied() {
        let group: Vec<&MigrationFailure> =
            result.failures.iter().filter(|f| f.kind == kind).collect();
        if group.is_empty() {
            continue;
        }
        writeln!(w)?;
        writeln!(
            w,
            "  {} ({}):",
            style(kind.label()).yellow().bold(),
            group.len()
        )?;
        for f in group {
            match &f.error {
                Some(e) => writeln!(w, "    {} ({e})", collapse_home(&f.path))?,
                None => writeln!(w, "    {}", collapse_home(&f.path))?,
            }
        }
    }
    Ok(())
}

// `run_migrate_library` was deleted in Plan 16-02 Task 3 — `cmd_migrate_library`
// now drives the plan / render_plan / prompt_confirmation / execute /
// render_result flow directly so the UX-02 confirm gate slots in between
// render_plan and execute. There is one canonical entry point for the
// migration flow; this module exposes its primitives and lib.rs composes them.

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::DirectoryName;
    use crate::discover::SkillName;
    use crate::manifest::{self, Manifest, SkillEntry};
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

    fn add_managed_entry(manifest: &mut Manifest, library: &Path, source: &Path, name: &str) {
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
        assert!(
            p.entries.is_empty(),
            "user-created symlink must NOT be in plan"
        );
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
        assert!(
            library.join("broken").is_symlink(),
            "broken symlink must be preserved"
        );
        // Manifest unchanged (D-06).
        assert!(manifest.contains_key("broken"));
        assert!(manifest.get("broken").unwrap().managed);
    }

    #[test]
    fn execute_succeeds_with_nested_directory_symlink_in_source() {
        // Regression for #515: pre-fix, the migration's pre/post hash check
        // used WalkDir::follow_links(false) for pre_hash but follow_links(true)
        // (via copy_dir_recursive_resolving) for post_hash. A source containing
        // any nested directory symlink would always hash unequal between the
        // two walks, producing a false IoError on a perfectly correct copy.
        let (_tmp, library, source, mut manifest) = setup_fixture();

        // Build a managed source dir whose SKILL.md sits next to a nested
        // directory symlink (e.g. plugin caches use these for shared assets).
        let src = make_managed_source(&source, "with-nested-symlink", "# main");
        let shared_target = source.join("shared-assets");
        std::fs::create_dir_all(&shared_target).unwrap();
        std::fs::write(shared_target.join("data.txt"), "shared").unwrap();
        unix_fs::symlink(&shared_target, src.join("shared")).unwrap();
        add_managed_entry(&mut manifest, &library, &src, "with-nested-symlink");

        let p = plan(&library, &manifest).unwrap();
        let result = execute(&p, false).unwrap();

        assert_eq!(
            result.converted, 1,
            "nested-symlink source must convert cleanly"
        );
        assert_eq!(result.failed, 0, "must not record a false IoError");
        assert!(!result.is_partial_or_failed());

        // Post-conversion library is materialized real dir with the nested
        // symlink dereferenced into a real directory copy.
        let dest = library.join("with-nested-symlink");
        assert!(dest.is_dir() && !dest.is_symlink());
        assert!(dest.join("shared").is_dir());
        assert_eq!(
            std::fs::read_to_string(dest.join("shared").join("data.txt")).unwrap(),
            "shared"
        );
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
        assert!(
            !r2.is_partial_or_failed(),
            "idempotent re-run must succeed cleanly"
        );
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
            [
                MigrationFailureKind::BrokenSource,
                MigrationFailureKind::IoError
            ]
        );
    }

    #[test]
    fn migration_failure_kind_labels() {
        assert_eq!(MigrationFailureKind::BrokenSource.label(), "Broken source");
        assert_eq!(MigrationFailureKind::IoError.label(), "I/O errors");
    }

    // -- UX-02 / Plan 16-02 Task 1 — byte_size + render_plan_to --

    #[test]
    fn plan_populates_byte_size_for_reachable_sources() {
        // D-UX02-4: each reachable entry's byte_size is Some(>= file content sum).
        let (_tmp, library, source, mut manifest) = setup_fixture();

        let s1 = source.join("p1");
        std::fs::create_dir_all(&s1).unwrap();
        // 1024-byte SKILL.md (single file).
        std::fs::write(s1.join("SKILL.md"), "x".repeat(1024)).unwrap();
        add_managed_entry(&mut manifest, &library, &s1, "p1");

        let s2 = source.join("p2");
        std::fs::create_dir_all(&s2).unwrap();
        // 1024-byte SKILL.md + 2048-byte data.txt = 3072 bytes minimum.
        std::fs::write(s2.join("SKILL.md"), "y".repeat(1024)).unwrap();
        std::fs::write(s2.join("data.txt"), "z".repeat(2048)).unwrap();
        add_managed_entry(&mut manifest, &library, &s2, "p2");

        let p = plan(&library, &manifest).unwrap();
        let by_name: std::collections::HashMap<&str, &MigrationEntry> = p
            .entries
            .iter()
            .map(|e| (e.skill_name.as_str(), e))
            .collect();

        let p1 = by_name.get("p1").expect("p1 entry");
        let p2 = by_name.get("p2").expect("p2 entry");
        assert!(
            p1.byte_size.is_some(),
            "reachable source must have Some byte_size"
        );
        assert!(
            p1.byte_size.unwrap() >= 1024,
            "p1 byte_size must include the 1024-byte SKILL.md, got {:?}",
            p1.byte_size
        );
        assert!(p2.byte_size.is_some());
        assert!(
            p2.byte_size.unwrap() >= 3072,
            "p2 byte_size must include SKILL.md + data.txt = >= 3072, got {:?}",
            p2.byte_size
        );
    }

    #[test]
    fn plan_byte_size_is_none_for_broken_source() {
        // D-UX02-4: broken symlinks have byte_size = None (no walk possible).
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
        assert_eq!(p.entries.len(), 1);
        assert!(!p.entries[0].source_reachable);
        assert!(
            p.entries[0].byte_size.is_none(),
            "broken sources must have byte_size = None, got {:?}",
            p.entries[0].byte_size
        );
    }

    #[test]
    fn render_plan_to_writer_emits_summary_line_with_total_size() {
        // D-UX02-3: writer-output contains the bold "Will convert N symlink"
        // wording and at least one humanize_bytes unit token.
        let (_tmp, library, source, mut manifest) = setup_fixture();
        let src = make_managed_source(&source, "p1", "# p1");
        add_managed_entry(&mut manifest, &library, &src, "p1");

        let p = plan(&library, &manifest).unwrap();
        let mut buf = Vec::new();
        render_plan_to(&p, &mut buf).unwrap();
        let out = String::from_utf8(buf).unwrap();

        assert!(
            out.contains("Will convert 1 symlink"),
            "summary line missing convert wording, got: {out}"
        );
        // At least one size unit token must appear (default total may be < 1KB
        // → "B"; larger sources promote to KB/MB/etc).
        let has_unit = ["B", "KB", "MB", "GB", "TB"].iter().any(|u| {
            out.contains(&format!("{u} additional disk")) || out.contains(&format!(" {u} "))
        });
        assert!(has_unit, "summary line missing size unit token, got: {out}");
    }

    #[test]
    fn render_plan_table_has_four_column_headers() {
        // D-UX02-3: tabled table emits all four expected column headers.
        let (_tmp, library, source, mut manifest) = setup_fixture();
        let src = make_managed_source(&source, "p1", "# p1");
        add_managed_entry(&mut manifest, &library, &src, "p1");

        let p = plan(&library, &manifest).unwrap();
        let mut buf = Vec::new();
        render_plan_to(&p, &mut buf).unwrap();
        let out = String::from_utf8(buf).unwrap();

        for header in ["SKILL", "SOURCE", "SIZE", "STATUS"] {
            assert!(
                out.contains(header),
                "table missing required column header `{header}`, got: {out}"
            );
        }
    }

    // -- UX-02 / Plan 16-02 Task 2 — prompt_confirmation --

    #[test]
    fn prompt_confirmation_returns_true_when_yes_flag_set() {
        // yes=true short-circuits before any dialoguer interaction.
        let r = prompt_confirmation(true, false).unwrap();
        assert!(r, "yes=true must return Ok(true) without prompting");
    }

    #[test]
    fn prompt_confirmation_returns_true_when_yes_and_no_input_set() {
        // yes=true wins over no_input=true (CI-friendly path).
        let r = prompt_confirmation(true, true).unwrap();
        assert!(r);
    }

    #[test]
    fn prompt_confirmation_bails_when_no_input_without_yes() {
        // Phase 7 D-10 Conflict/Why/Suggestion bail.
        let err = prompt_confirmation(false, true).unwrap_err();
        let msg = format!("{err:#}");
        assert!(
            msg.contains("destructive"),
            "bail message must mention 'destructive', got: {msg}"
        );
        assert!(
            msg.contains("--yes"),
            "bail message must mention '--yes', got: {msg}"
        );
        assert!(
            msg.contains("--no-input"),
            "bail message must mention '--no-input', got: {msg}"
        );
    }

    #[test]
    fn humanize_bytes_unit_promotion() {
        assert_eq!(humanize_bytes(0), "0 B");
        assert_eq!(humanize_bytes(512), "512 B");
        // 1024 -> 1.0 KB (one decimal); 1536 -> 1.5 KB.
        assert_eq!(humanize_bytes(1024), "1.0 KB");
        assert_eq!(humanize_bytes(1536), "1.5 KB");
        // 1 MB exactly.
        assert_eq!(humanize_bytes(1024 * 1024), "1.0 MB");
        // ~30 MB (matches the canonical UX-02 example).
        let thirty_mb = 30 * 1024 * 1024 + (1024 * 410); // ~30.4 MB
        let s = humanize_bytes(thirty_mb);
        assert!(s.starts_with("30.") && s.ends_with(" MB"), "got: {s}");
    }
}
