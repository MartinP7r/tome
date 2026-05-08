//! Lockfile-authoritative reconciliation.
//!
//! Classifies every managed lockfile entry as Match / Drift / Vanished /
//! MissingFromMachine / Edited (D-01, D-14). Drives the drift-apply loop
//! through `MarketplaceAdapter::install`/`update` (D-22 partial-failure
//! invariant). Owns the `[Y/n/never]` consent prompt (D-07/08) and the
//! `[F/r/s]` edit-in-library prompt (D-15). Phase 13's RECON-01..05 entry
//! point.
//!
//! The single public entry point is [`reconcile_lockfile`]. Internally the
//! flow splits into pure helpers (`classify_lockfile`, `detect_edited`,
//! `apply_drift_and_missing`, `format_summary`) so each is independently
//! testable against `MockMarketplaceAdapter` from `crate::marketplace::testing`.
//!
//! This module mirrors `crate::update`'s plan/render/execute shape (RESEARCH
//! Pattern 1) so reviewers can map the two flows visually. Plan 13-04 wires
//! `reconcile_lockfile` into `lib.rs::sync` (replaces `reconcile_managed_plugins`).

use std::io::IsTerminal;
use std::path::Path;

use anyhow::{Context, Result};
use console::style;

use crate::config::DirectoryName;
use crate::discover::SkillName;
use crate::lockfile::{self, Lockfile};
use crate::machine::{self, AutoInstall, MachinePrefs};
use crate::manifest::{self, Manifest};
use crate::marketplace::{InstallFailure, InstallFailureKind, InstallOp, MarketplaceAdapter};
use crate::paths::TomePaths;

/// Classification of a single managed lockfile entry against the live
/// marketplace + library state. Drives the drift-apply loop (RECON-01).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReconcileClass {
    /// Lockfile content_hash equals freshly-computed library content_hash
    /// AND adapter reports the plugin is installed (D-01).
    Match,
    /// Lockfile content_hash differs from freshly-computed library hash
    /// (or library content is absent post-install). `version` strings are
    /// display-only (D-05) — drift basis is hash, not version (D-01).
    Drift {
        old_version: Option<String>,
        new_version: Option<String>,
    },
    /// `adapter.available(registry_id)` returned false. Library copy is
    /// preserved per LIB-04; downstream distribution still happens (D-06).
    Vanished { old_version: Option<String> },
    /// Lockfile entry exists but plugin is not in `adapter.list_installed()`
    /// AND `adapter.available()` returns true — first-machine bootstrap
    /// case (D-10).
    MissingFromMachine,
}

/// User's choice from the edit-in-library prompt (RECON-05 D-15).
///
/// Returned per `Edited` skill so the caller (`lib.rs::sync`) can apply the
/// manifest mutation — `reconcile_lockfile` only PROPOSES the choice; the
/// owner of the manifest applies it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EditDecision {
    /// D-13 in-place flip: `managed: true → false`, `source_name: Some →
    /// None`. Library content unchanged. Lockfile entry preserved with
    /// stale upstream metadata that downstream lockfile regeneration may
    /// clear.
    Fork,
    /// Discard local edits, restore from marketplace. Caller invokes
    /// `adapter.update(registry_id)` then re-hashes; reconcile flow's
    /// drift-apply path would normally produce the same result, so revert
    /// degenerates to "force a drift apply this run".
    Revert,
    /// Warn and don't touch — D-16 default for `--no-input`.
    Skip,
}

/// A managed lockfile entry's classification + the metadata needed to drive
/// downstream rendering / apply.
#[derive(Debug, Clone)]
pub struct Classified {
    pub name: SkillName,
    pub registry_id: String,
    pub source_name: DirectoryName,
    pub class: ReconcileClass,
}

/// Edit-in-library detection record (RECON-05). Separate from
/// `ReconcileClass` because the gate is different (manifest-side, not
/// lockfile-side) and the prompt is independent of `auto_install_plugins`.
#[derive(Debug, Clone)]
pub struct Edited {
    pub name: SkillName,
    pub old_source: DirectoryName,
    pub old_version: Option<String>,
}

/// Aggregate report returned to `lib.rs::sync` for stdout/stderr rendering
/// + exit-code decisions. The summary line `✓ N match · ⚠ N drift · ⚠ N
/// vanished` is computed from the counts here (D-02/D-04).
#[derive(Debug, Default)]
pub struct ReconcileReport {
    pub matches: usize,
    pub drift: Vec<Classified>,
    pub vanished: Vec<Classified>,
    pub missing: Vec<Classified>,
    pub edited: Vec<Edited>,
    pub install_failures: Vec<InstallFailure>,
    /// True when --no-install or `auto_install_plugins == Never`/Ask/None
    /// under non-interactive mode prevented the apply step from running.
    pub apply_skipped: bool,
    /// Per-edited-skill user choice. Same length as `edited` and in the
    /// same order. Populated by `handle_edited` based on prompt or
    /// non-interactive default. The caller (`lib.rs::sync`) applies the
    /// chosen action against the manifest.
    pub edit_decisions: Vec<EditDecision>,
}

/// Options threaded from `SyncOptions` (lib.rs) into reconcile.
//
// `verbose` is reserved for Plan 13-04's call site (verbose-mode tracing in
// reconcile internals will be added in a follow-up); kept as a public field
// because it is part of the contract from `SyncOptions`.
#[allow(dead_code)] // verbose field is reserved (read fields: dry_run, no_input, no_install, quiet)
#[derive(Debug, Clone, Copy)]
pub struct ReconcileOpts {
    pub dry_run: bool,
    pub no_input: bool,
    pub no_install: bool,
    pub quiet: bool,
    pub verbose: bool,
}

/// Reconcile lockfile-recorded managed plugins against the live marketplace
/// + library. The single Phase 13 entry point.
///
/// Per CONTEXT.md D-18: replaces `reconcile_managed_plugins` at line 978 of
/// `lib.rs::sync`, runs BEFORE discovery, so the adapter installs missing
/// plugins → discovery sees the result → consolidate copies into library →
/// distribute symlinks unchanged.
///
/// ## Save chain (Pitfall 5)
///
/// - Consent change (`auto_install_plugins`) is persisted to `machine.toml`
///   via `machine::save` IMMEDIATELY after the prompt resolves, so a Ctrl-C
///   between consent and sync-completion preserves the user's choice.
/// - Lockfile is written to disk via `lockfile::save` ONCE at the end of
///   the apply loop (RESEARCH OQ-4 option a — D-22 literal reading). The
///   post-distribute `lockfile::save` in `lib.rs::sync` is a SECOND write
///   that converges on the same content (atomic temp+rename makes
///   double-write safe).
///
/// ## Returns
///
/// `Ok(ReconcileReport)` regardless of partial install failures — the
/// `install_failures` field carries them so `lib.rs::sync` can decide the
/// process exit code (RESEARCH OQ-6: caller does `anyhow::bail!` after
/// rendering).
#[allow(clippy::too_many_arguments)]
pub fn reconcile_lockfile(
    old_lockfile: Option<&Lockfile>,
    manifest: &Manifest,
    library_dir: &Path,
    adapter: &dyn MarketplaceAdapter,
    prefs: &mut MachinePrefs,
    machine_path: &Path,
    paths: &TomePaths,
    opts: ReconcileOpts,
) -> Result<ReconcileReport> {
    let Some(lockfile) = old_lockfile else {
        // First-run: nothing to reconcile against. lib.rs::sync prints the
        // "No previous lockfile" message itself.
        return Ok(ReconcileReport::default());
    };

    // 1. Classify every managed lockfile entry.
    let classified = classify_lockfile(lockfile, library_dir, adapter)?;

    // 2. Detect edit-in-library (manifest side, independent of consent).
    let edited = detect_edited(manifest, library_dir, lockfile)?;

    // 3. Build initial report from classification.
    let mut report = ReconcileReport::default();
    let mut drift_to_apply: Vec<Classified> = Vec::new();
    let mut missing_to_apply: Vec<Classified> = Vec::new();
    for c in classified {
        match c.class {
            ReconcileClass::Match => report.matches += 1,
            ReconcileClass::Drift { .. } => {
                drift_to_apply.push(c.clone());
                report.drift.push(c);
            }
            ReconcileClass::Vanished { .. } => report.vanished.push(c),
            ReconcileClass::MissingFromMachine => {
                missing_to_apply.push(c.clone());
                report.missing.push(c);
            }
        }
    }
    report.edited = edited;

    // #512: when a managed skill is BOTH classified as Drift AND detected
    // as Edited, route it exclusively through the edit-in-library prompt
    // (handle_edited at step 7). Without this filter the apply loop would
    // call adapter.update() BEFORE the prompt fires, and for managed skills
    // library/<skill> symlinks the source dir — `claude plugin update`
    // would overwrite the user's edits before fork/revert/skip is asked.
    // D-16 / RECON-05 require: never silently overwrite edited content.
    let edited_names: std::collections::HashSet<&SkillName> =
        report.edited.iter().map(|e| &e.name).collect();
    drift_to_apply.retain(|c| !edited_names.contains(&c.name));
    report.drift.retain(|c| !edited_names.contains(&c.name));

    // 4. Decide whether to apply drift+missing (consent + no_install gate).
    let needs_apply = !drift_to_apply.is_empty() || !missing_to_apply.is_empty();
    let consent = if needs_apply {
        resolve_consent(
            prefs,
            machine_path,
            &opts,
            drift_to_apply.len() + missing_to_apply.len(),
        )?
    } else {
        ConsentDecision::SkipNoWork
    };

    // 5. Apply if consent allows.
    let mut working_lockfile = clone_lockfile(lockfile);
    let apply_should_run = matches!(consent, ConsentDecision::Apply) && !opts.no_install;
    if apply_should_run {
        apply_drift_and_missing(
            &drift_to_apply,
            &missing_to_apply,
            adapter,
            library_dir,
            &mut working_lockfile,
            &mut report,
            opts.dry_run,
        )?;
    } else if needs_apply {
        report.apply_skipped = true;
    }

    // 6. Save lockfile once (D-22) — only when we actually mutated it.
    if !opts.dry_run && apply_should_run && working_lockfile != *lockfile {
        lockfile::save(&working_lockfile, paths.config_dir())
            .context("failed to save lockfile after reconcile drift apply")?;
    }

    // 7. Edit-in-library prompt (independent of consent — RECON-05 always
    //    asks before overwriting user content).
    if !report.edited.is_empty() {
        let edited_clone = report.edited.clone();
        handle_edited(&edited_clone, lockfile, &opts, &mut report)?;
        debug_assert_eq!(report.edit_decisions.len(), report.edited.len());
    }

    Ok(report)
}

/// Clone a `Lockfile` field-by-field. `Lockfile` does not derive `Clone`
/// (touching its derives is out of scope for Phase 13 — see HARD-06), but
/// `LockEntry` does, so this helper rebuilds the BTreeMap manually.
fn clone_lockfile(src: &Lockfile) -> Lockfile {
    let skills = src
        .skills
        .iter()
        .map(|(k, v)| (k.clone(), v.clone()))
        .collect();
    Lockfile {
        version: src.version,
        skills,
    }
}

/// Classify every managed lockfile entry against the live marketplace.
///
/// Per Pitfall 4: entries with `registry_id: None` (local skills tracked by
/// the lockfile) are SKIPPED — not classified at all. Per design, entries
/// with `source_name: None` (Unowned skills) are also skipped because they
/// have no upstream identity to reconcile against.
///
/// `plugin_id` is passed verbatim to adapter methods (RESEARCH OQ-3): the
/// lockfile-recorded qualified form (`axiom@axiom-marketplace`) is the
/// adapter's contract.
fn classify_lockfile(
    lockfile: &Lockfile,
    library_dir: &Path,
    adapter: &dyn MarketplaceAdapter,
) -> Result<Vec<Classified>> {
    let installed = adapter.list_installed()?;
    let installed_ids: std::collections::HashSet<&str> =
        installed.iter().map(|p| p.id.as_str()).collect();

    let mut out = Vec::new();
    for (name, entry) in lockfile.skills() {
        // Pitfall 4: skip entries with registry_id None (local skills in lockfile).
        let Some(registry_id) = entry.registry_id.as_ref() else {
            continue;
        };
        // Skip Unowned (no upstream identity).
        let Some(source_name) = entry.source_name.as_ref() else {
            continue;
        };

        let class = if !adapter.available(registry_id)? {
            ReconcileClass::Vanished {
                old_version: entry.version.clone(),
            }
        } else if !installed_ids.contains(registry_id.as_str()) {
            ReconcileClass::MissingFromMachine
        } else {
            // Compute live hash from library copy.
            let skill_dir = library_dir.join(name.as_str());
            let live_hash = manifest::hash_directory(&skill_dir)
                .with_context(|| format!("failed to hash library copy of {name}"))?;
            if live_hash == entry.content_hash {
                ReconcileClass::Match
            } else {
                let new_version = adapter.current_version(registry_id)?;
                ReconcileClass::Drift {
                    old_version: entry.version.clone(),
                    new_version,
                }
            }
        };

        out.push(Classified {
            name: name.clone(),
            registry_id: registry_id.clone(),
            source_name: source_name.clone(),
            class,
        });
    }
    Ok(out)
}

/// Detect skills that have been edited in the library after a managed sync
/// (RECON-05).
///
/// Gate per D-14: `managed=true && source_name=Some(_) && hash mismatch`.
/// Unowned skills (`source_name=None`) and local skills (`managed=false`)
/// are skipped — their content is user-canonical by design.
fn detect_edited(
    manifest: &Manifest,
    library_dir: &Path,
    lockfile: &Lockfile,
) -> Result<Vec<Edited>> {
    let mut out = Vec::new();
    for (name, entry) in manifest.iter() {
        // D-14 gate: managed && source_name.is_some() && hash mismatch.
        if !entry.managed {
            continue;
        }
        let Some(source_name) = entry.source_name.as_ref() else {
            continue;
        };
        let Some(lock_entry) = lockfile.skills().get(name.as_str()) else {
            continue;
        };

        let live_hash = manifest::hash_directory(&library_dir.join(name.as_str()))
            .with_context(|| format!("failed to hash library copy of {name}"))?;

        if live_hash != lock_entry.content_hash {
            out.push(Edited {
                name: name.clone(),
                old_source: source_name.clone(),
                old_version: lock_entry.version.clone(),
            });
        }
    }
    Ok(out)
}

/// Resolution of the consent state machine (RECON-02).
///
/// `Apply` runs the drift-apply loop. The three Skip variants document
/// *why* the apply step is suppressed; `apply_skipped == true` is set
/// in the report for any non-Apply outcome.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ConsentDecision {
    Apply,
    /// `Never`/`--no-install`/non-`Always` under `--no-input` → no install.
    SkipNoConsent,
    /// `Ask`/`None` consent + non-interactive (Pitfall 2).
    SkipNoInteractive,
    /// No drift / no missing — nothing to apply.
    SkipNoWork,
}

fn resolve_consent(
    prefs: &mut MachinePrefs,
    machine_path: &Path,
    opts: &ReconcileOpts,
    affected_count: usize,
) -> Result<ConsentDecision> {
    // --no-install: caller-side override, doesn't touch persisted state.
    if opts.no_install {
        return Ok(ConsentDecision::SkipNoConsent);
    }

    match prefs.auto_install_plugins {
        Some(AutoInstall::Always) => Ok(ConsentDecision::Apply),
        Some(AutoInstall::Never) => Ok(ConsentDecision::SkipNoConsent),
        Some(AutoInstall::Ask) | None => {
            // Non-interactive can't prompt — Pitfall 2.
            if opts.no_input || !std::io::stdin().is_terminal() {
                return Ok(ConsentDecision::SkipNoInteractive);
            }
            // Show prompt (D-08).
            let choice = prompt_consent(affected_count)?;
            apply_consent_decision(prefs, choice, machine_path)?;
            match choice {
                AutoInstall::Always => Ok(ConsentDecision::Apply),
                AutoInstall::Ask => Ok(ConsentDecision::Apply), // Y for this run
                AutoInstall::Never => Ok(ConsentDecision::SkipNoConsent),
            }
        }
    }
}

/// Persist the user's consent decision IMMEDIATELY (Pitfall 5).
///
/// Setting + saving is factored out so the save-chain timing is independently
/// testable: a Ctrl-C between this call and sync-completion preserves the
/// user's choice on the next run.
pub(crate) fn apply_consent_decision(
    prefs: &mut MachinePrefs,
    choice: AutoInstall,
    machine_path: &Path,
) -> Result<()> {
    prefs.auto_install_plugins = Some(choice);
    machine::save(prefs, machine_path).context("failed to persist auto_install_plugins consent")?;
    Ok(())
}

/// Display the 3-way consent prompt via `dialoguer::Select` (OQ-1).
///
/// The literal `[Y/n/never]` line in CONTEXT.md D-08 is realised as an
/// arrow-key list; the option labels below describe the same three
/// outcomes and default to `Always` (the affirmative action).
fn prompt_consent(affected_count: usize) -> Result<AutoInstall> {
    let prompt = format!(
        "Tome detected {affected_count} missing or out-of-date managed plugins. \
         Install/update them now?"
    );
    let items = [
        "Yes (always — install on every sync)",
        "Yes (ask me again next time)",
        "No (never ask again on this machine)",
    ];
    let idx = dialoguer::Select::new()
        .with_prompt(prompt)
        .items(items)
        .default(0)
        .interact()
        .context("consent prompt failed")?;
    Ok(match idx {
        0 => AutoInstall::Always,
        1 => AutoInstall::Ask,
        2 => AutoInstall::Never,
        _ => unreachable!("Select::interact returns 0..items.len()"),
    })
}

/// Apply drift + missing in a single loop, honouring D-22's
/// partial-failure invariant: only successful adapter calls update the
/// in-memory `working_lockfile`; failed calls leave entries at their
/// previous values and append to `report.install_failures`.
///
/// The qualified `registry_id` is passed verbatim to adapter methods
/// (RESEARCH OQ-3 — that's the adapter's contract).
fn apply_drift_and_missing(
    drift: &[Classified],
    missing: &[Classified],
    adapter: &dyn MarketplaceAdapter,
    library_dir: &Path,
    working_lockfile: &mut Lockfile,
    report: &mut ReconcileReport,
    dry_run: bool,
) -> Result<()> {
    // Render diff lines (D-05) before applying so the user sees what's about
    // to happen — applies even when consent was Always (silently).
    for c in drift {
        if let ReconcileClass::Drift {
            old_version,
            new_version,
        } = &c.class
        {
            println!(
                "  • {}: {} → {}",
                c.name.as_str(),
                old_version.as_deref().unwrap_or("unknown"),
                new_version.as_deref().unwrap_or("unknown"),
            );
        }
    }
    for c in missing {
        println!("  • {} (missing — installing)", c.name.as_str());
    }

    if dry_run {
        return Ok(());
    }

    // Apply drift via update().
    for c in drift {
        match adapter.update(&c.registry_id) {
            Ok(()) => {
                if let Some(entry) = working_lockfile.skills.get_mut(c.name.as_str()) {
                    // #513: surface readback errors instead of swallowing.
                    // Previously `if let Ok(...)` silently kept the stale
                    // pre-update hash on re-hash failure, producing a fake-
                    // drift loop on every subsequent sync (Phase 13 D-01
                    // makes content_hash mismatch the drift trigger).
                    match manifest::hash_directory(&library_dir.join(c.name.as_str())) {
                        Ok(h) => entry.content_hash = h,
                        Err(e) => eprintln!(
                            "warning: post-update hash_directory({}) failed: {e:#} — \
                             leaving lockfile content_hash unchanged",
                            c.name.as_str()
                        ),
                    }
                    // #513: previously `.ok().flatten()` collapsed both
                    // Err(_) and Ok(None) into None, silently nulling the
                    // lockfile version after a successful apply.
                    match adapter.current_version(&c.registry_id) {
                        Ok(v) => entry.version = v,
                        Err(e) => eprintln!(
                            "warning: post-update current_version({}) failed: {e:#} — \
                             leaving lockfile version field unchanged",
                            c.registry_id
                        ),
                    }
                }
            }
            Err(e) => {
                report.install_failures.push(InstallFailure {
                    adapter_id: adapter.id().to_string(),
                    plugin_id: c.registry_id.clone(),
                    operation: InstallOp::Update,
                    kind: classify_install_error(&e),
                    source: e,
                });
                // D-22: leave working_lockfile entry untouched.
            }
        }
    }

    // Apply missing via install().
    for c in missing {
        match adapter.install(&c.registry_id) {
            Ok(()) => {
                if let Some(entry) = working_lockfile.skills.get_mut(c.name.as_str()) {
                    // #513: do NOT recompute content_hash here — for
                    // MissingFromMachine, the library copy doesn't exist
                    // yet (discover/consolidate run AFTER reconcile in
                    // lib.rs::sync). The post-distribute lockfile regen
                    // records the correct hash once consolidation has run.
                    match adapter.current_version(&c.registry_id) {
                        Ok(v) => entry.version = v,
                        Err(e) => eprintln!(
                            "warning: post-install current_version({}) failed: {e:#} — \
                             leaving lockfile version field unchanged",
                            c.registry_id
                        ),
                    }
                }
            }
            Err(e) => {
                report.install_failures.push(InstallFailure {
                    adapter_id: adapter.id().to_string(),
                    plugin_id: c.registry_id.clone(),
                    operation: InstallOp::Install,
                    kind: classify_install_error(&e),
                    source: e,
                });
            }
        }
    }

    Ok(())
}

/// Best-effort heuristic from anyhow chain text → `InstallFailureKind`.
///
/// Mirrors `marketplace::ClaudeMarketplaceAdapter`'s post-Phase 12
/// classifier for stderr text: precise enough for grouped rendering, falls
/// back to `Unknown` (which still surfaces the verbatim error chain via
/// `InstallFailure::source`).
fn classify_install_error(e: &anyhow::Error) -> InstallFailureKind {
    let msg = format!("{e:#}").to_lowercase();
    if msg.contains("not found") {
        InstallFailureKind::NotFound
    } else if msg.contains("permission") {
        InstallFailureKind::PermissionDenied
    } else if msg.contains("network") || msg.contains("timeout") || msg.contains("connect") {
        InstallFailureKind::NetworkError
    } else {
        InstallFailureKind::Unknown
    }
}

/// Handle edit-in-library detection (RECON-05).
///
/// Per D-16: `--no-input` or non-TTY → skip-with-warning, exit zero. The
/// fork/revert/skip prompt (D-13/D-15) is recorded into `report.edit_decisions`
/// so the caller (`lib.rs::sync`) can apply the manifest mutation; this
/// helper does NOT mutate the manifest itself (the manifest is owned at the
/// call site).
fn handle_edited(
    edited: &[Edited],
    lockfile: &Lockfile,
    opts: &ReconcileOpts,
    report: &mut ReconcileReport,
) -> Result<()> {
    let _ = lockfile; // reserved for richer prompt copy if needed later

    // D-16: --no-input or non-TTY → skip-with-warning per skill, exit zero.
    if opts.no_input || !std::io::stdin().is_terminal() {
        for e in edited {
            eprintln!(
                "warning: {} has local edits; skipping reconcile this sync (run interactively to fork/revert)",
                e.name.as_str()
            );
            report.edit_decisions.push(EditDecision::Skip);
        }
        return Ok(());
    }

    // Interactive [F/r/s] (D-15). Use dialoguer::Select per OQ-1.
    for e in edited {
        let prompt = format!(
            "{} has local edits. Last upstream: {} @ {}.",
            e.name.as_str(),
            e.old_source.as_str(),
            e.old_version.as_deref().unwrap_or("unknown"),
        );
        let items = [
            "fork — keep your edits, sever the upstream link (default)",
            "revert — discard your edits, restore upstream",
            "skip — warn and don't touch this entry this sync",
        ];
        let idx = dialoguer::Select::new()
            .with_prompt(prompt)
            .items(items)
            .default(0)
            .interact()
            .context("edit-in-library prompt failed")?;
        let decision = match idx {
            0 => EditDecision::Fork,
            1 => EditDecision::Revert,
            2 => EditDecision::Skip,
            _ => unreachable!("Select::interact returns 0..items.len()"),
        };
        report.edit_decisions.push(decision);
    }
    Ok(())
}

/// Render the summary line + drift detail + vanished warnings into a String.
///
/// Exposed as `format_summary` returning a `String` so tests can assert
/// against substrings; thin wrapper [`render_summary`] does the actual
/// printing.
///
/// Per D-04: always renders all three buckets (`✓ N match · ⚠ N drift · ⚠ N
/// vanished`) — predictable output is greppable across runs. Per D-03: when
/// drift+vanished are zero AND matches > 0, prepends a positive `✓ N plugins
/// in sync` line.
pub fn format_summary(report: &ReconcileReport) -> String {
    let total = report.matches + report.drift.len() + report.vanished.len();
    if total == 0 {
        return String::new();
    }
    let mut out = String::new();
    if report.drift.is_empty() && report.vanished.is_empty() && report.matches > 0 {
        // D-03: all-match positive evidence.
        out.push_str(&format!(
            "{} {} plugins in sync\n",
            style("✓").green(),
            report.matches
        ));
    }
    // D-02 + D-04: always render all three buckets.
    out.push_str(&format!(
        "{} {} match · {} {} drift · {} {} vanished\n",
        style("✓").green(),
        report.matches,
        style("⚠").yellow(),
        report.drift.len(),
        style("⚠").yellow(),
        report.vanished.len(),
    ));
    // D-05: per-drift detail line.
    for c in &report.drift {
        if let ReconcileClass::Drift {
            old_version,
            new_version,
        } = &c.class
        {
            out.push_str(&format!(
                "  • {}: {} → {}\n",
                c.name.as_str(),
                old_version.as_deref().unwrap_or("unknown"),
                new_version.as_deref().unwrap_or("unknown"),
            ));
        }
    }
    // D-06: per-vanished warning. The caller decides stdout vs stderr; we
    // bake the verbatim "using preserved library copy" text per the design.
    for c in &report.vanished {
        out.push_str(&format!(
            "warning: plugin {} vanished from marketplace {}; using preserved library copy\n",
            c.name.as_str(),
            c.source_name.as_str(),
        ));
    }
    out
}

/// Print the summary string to stdout. Suppressed under `--quiet`.
pub fn render_summary(report: &ReconcileReport, quiet: bool) {
    if quiet {
        return;
    }
    let s = format_summary(report);
    if !s.is_empty() {
        print!("{s}");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::DirectoryName;
    use crate::lockfile::LockEntry;
    use crate::manifest::SkillEntry;
    use crate::marketplace::testing::{MockMarketplaceAdapter, fixture_plugin};
    use crate::validation::ContentHash;
    use std::collections::{BTreeMap, HashSet};
    use std::path::{Path, PathBuf};
    use tempfile::TempDir;

    // ---------- Fixture helpers ----------

    /// Placeholder ContentHash used when the test doesn't care about hash
    /// identity (e.g. the `Vanished` path skips hash comparison entirely).
    fn placeholder_hash() -> ContentHash {
        ContentHash::new("a".repeat(64)).unwrap()
    }

    /// Create a SKILL.md tree under `library_dir/<name>/` with the given
    /// body and return its content hash.
    fn make_skill_dir(library_dir: &Path, name: &str, body: &str) -> ContentHash {
        let dir = library_dir.join(name);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(
            dir.join("SKILL.md"),
            format!("---\nname: {name}\n---\n\n{body}\n"),
        )
        .unwrap();
        manifest::hash_directory(&dir).unwrap()
    }

    fn lock_entry(
        source: &str,
        hash: ContentHash,
        registry_id: Option<&str>,
        version: Option<&str>,
    ) -> LockEntry {
        LockEntry {
            source_name: Some(DirectoryName::new(source).unwrap()),
            previous_source: None,
            content_hash: hash,
            registry_id: registry_id.map(|s| s.to_string()),
            version: version.map(|s| s.to_string()),
            git_commit_sha: None,
        }
    }

    fn lockfile_with(entries: Vec<(&str, LockEntry)>) -> Lockfile {
        let mut skills = BTreeMap::new();
        for (name, entry) in entries {
            skills.insert(SkillName::new(name).unwrap(), entry);
        }
        Lockfile { version: 1, skills }
    }

    fn empty_mock(id: &str) -> MockMarketplaceAdapter {
        MockMarketplaceAdapter {
            id: id.to_string(),
            installed: Vec::new(),
            available: HashSet::new(),
            fail_install: HashSet::new(),
            fail_update: HashSet::new(),
        }
    }

    fn default_opts() -> ReconcileOpts {
        ReconcileOpts {
            dry_run: false,
            no_input: true,
            no_install: false,
            quiet: true,
            verbose: false,
        }
    }

    fn paths_with_lib(tmp: &Path) -> (TomePaths, PathBuf) {
        let library_dir = tmp.join("library");
        std::fs::create_dir_all(&library_dir).unwrap();
        let paths = TomePaths::new(tmp.to_path_buf(), library_dir.clone()).unwrap();
        (paths, library_dir)
    }

    // ---------- Classification tests (RECON-01) ----------

    #[test]
    fn classify_match_when_hash_and_id_agree() {
        let tmp = TempDir::new().unwrap();
        let (_paths, lib) = paths_with_lib(tmp.path());
        let hash = make_skill_dir(&lib, "alpha", "hello");

        let mut mock = empty_mock("mp");
        mock.installed.push(fixture_plugin("alpha@mp", "5.0.5"));
        mock.available.insert("alpha@mp".to_string());

        let lockfile = lockfile_with(vec![(
            "alpha",
            lock_entry("claude-plugins", hash, Some("alpha@mp"), Some("5.0.5")),
        )]);

        let result = classify_lockfile(&lockfile, &lib, &mock).unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].class, ReconcileClass::Match);
    }

    #[test]
    fn classify_drift_when_hash_differs() {
        let tmp = TempDir::new().unwrap();
        let (_paths, lib) = paths_with_lib(tmp.path());
        let _live_hash = make_skill_dir(&lib, "alpha", "new content");
        let stale_hash = ContentHash::new("b".repeat(64)).unwrap();

        let mut mock = empty_mock("mp");
        mock.installed.push(fixture_plugin("alpha@mp", "5.0.7"));
        mock.available.insert("alpha@mp".to_string());

        let lockfile = lockfile_with(vec![(
            "alpha",
            lock_entry(
                "claude-plugins",
                stale_hash,
                Some("alpha@mp"),
                Some("5.0.5"),
            ),
        )]);

        let result = classify_lockfile(&lockfile, &lib, &mock).unwrap();
        assert_eq!(result.len(), 1);
        match &result[0].class {
            ReconcileClass::Drift {
                old_version,
                new_version,
            } => {
                assert_eq!(old_version.as_deref(), Some("5.0.5"));
                assert_eq!(new_version.as_deref(), Some("5.0.7"));
            }
            other => panic!("expected Drift, got {other:?}"),
        }
    }

    #[test]
    fn classify_vanished_when_adapter_unavailable() {
        let tmp = TempDir::new().unwrap();
        let (_paths, lib) = paths_with_lib(tmp.path());
        // Library copy intentionally absent — Vanished is decided BEFORE hashing.
        let _ = lib;

        // Mock has NO entry for "beta@mp" in `available` set → available() returns false.
        let mock = empty_mock("mp");

        let lockfile = lockfile_with(vec![(
            "beta",
            lock_entry(
                "claude-plugins",
                placeholder_hash(),
                Some("beta@mp"),
                Some("3.0.0"),
            ),
        )]);

        let result = classify_lockfile(&lockfile, &lib, &mock).unwrap();
        assert_eq!(result.len(), 1);
        assert!(matches!(
            result[0].class,
            ReconcileClass::Vanished {
                old_version: Some(ref v)
            } if v == "3.0.0"
        ));
    }

    #[test]
    fn classify_missing_when_lockfile_entry_not_in_adapter_list() {
        let tmp = TempDir::new().unwrap();
        let (_paths, lib) = paths_with_lib(tmp.path());

        let mut mock = empty_mock("mp");
        // available() returns true but list_installed() does NOT include gamma@mp.
        mock.available.insert("gamma@mp".to_string());

        let lockfile = lockfile_with(vec![(
            "gamma",
            lock_entry(
                "claude-plugins",
                placeholder_hash(),
                Some("gamma@mp"),
                Some("1.0.0"),
            ),
        )]);

        let result = classify_lockfile(&lockfile, &lib, &mock).unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].class, ReconcileClass::MissingFromMachine);
    }

    #[test]
    fn classify_skips_local_skills_with_no_registry_id() {
        let tmp = TempDir::new().unwrap();
        let (_paths, lib) = paths_with_lib(tmp.path());

        let mock = empty_mock("mp");
        let lockfile = lockfile_with(vec![(
            "local-only",
            lock_entry("standalone", placeholder_hash(), None, None),
        )]);

        let result = classify_lockfile(&lockfile, &lib, &mock).unwrap();
        assert!(
            result.is_empty(),
            "local skills must be omitted from classification, got {result:?}"
        );
    }

    #[test]
    fn classify_skips_unowned_skills() {
        let tmp = TempDir::new().unwrap();
        let (_paths, lib) = paths_with_lib(tmp.path());

        let mock = empty_mock("mp");
        let entry = LockEntry {
            source_name: None, // Unowned
            previous_source: None,
            content_hash: placeholder_hash(),
            registry_id: Some("orphan@mp".to_string()),
            version: Some("1.0.0".to_string()),
            git_commit_sha: None,
        };
        let lockfile = lockfile_with(vec![("orphan", entry)]);

        let result = classify_lockfile(&lockfile, &lib, &mock).unwrap();
        assert!(
            result.is_empty(),
            "Unowned skills must be omitted from classification, got {result:?}"
        );
    }

    // ---------- Edit-in-library detection tests (RECON-05) ----------

    #[test]
    fn detect_edited_managed_with_hash_mismatch() {
        let tmp = TempDir::new().unwrap();
        let (_paths, lib) = paths_with_lib(tmp.path());
        let _live_hash = make_skill_dir(&lib, "edited-skill", "user added content");
        let stale_hash = ContentHash::new("c".repeat(64)).unwrap();

        let mut manifest = Manifest::default();
        manifest.insert(
            SkillName::new("edited-skill").unwrap(),
            SkillEntry::new(
                PathBuf::from("/tmp/edited-skill"),
                DirectoryName::new("claude-plugins").unwrap(),
                stale_hash.clone(),
                true, // managed
            ),
        );

        let lockfile = lockfile_with(vec![(
            "edited-skill",
            lock_entry(
                "claude-plugins",
                stale_hash,
                Some("edited-skill@mp"),
                Some("1.0.0"),
            ),
        )]);

        let result = detect_edited(&manifest, &lib, &lockfile).unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].name.as_str(), "edited-skill");
        assert_eq!(result[0].old_source.as_str(), "claude-plugins");
        assert_eq!(result[0].old_version.as_deref(), Some("1.0.0"));
    }

    #[test]
    fn detect_edited_skips_unmanaged() {
        let tmp = TempDir::new().unwrap();
        let (_paths, lib) = paths_with_lib(tmp.path());
        let _live_hash = make_skill_dir(&lib, "local-skill", "body");
        let stale_hash = ContentHash::new("d".repeat(64)).unwrap();

        let mut manifest = Manifest::default();
        manifest.insert(
            SkillName::new("local-skill").unwrap(),
            SkillEntry::new(
                PathBuf::from("/tmp/local-skill"),
                DirectoryName::new("standalone").unwrap(),
                stale_hash.clone(),
                false, // NOT managed
            ),
        );

        let lockfile = lockfile_with(vec![(
            "local-skill",
            lock_entry("standalone", stale_hash, None, None),
        )]);

        let result = detect_edited(&manifest, &lib, &lockfile).unwrap();
        assert!(
            result.is_empty(),
            "non-managed skills must not be edit-classified, got {result:?}"
        );
    }

    #[test]
    fn detect_edited_skips_unowned_managed() {
        let tmp = TempDir::new().unwrap();
        let (_paths, lib) = paths_with_lib(tmp.path());
        let _live_hash = make_skill_dir(&lib, "orphan", "body");
        let stale_hash = ContentHash::new("e".repeat(64)).unwrap();

        let mut manifest = Manifest::default();
        manifest.insert(
            SkillName::new("orphan").unwrap(),
            SkillEntry::new_unowned(PathBuf::from("/tmp/orphan"), stale_hash.clone(), true, None),
        );

        let lockfile = lockfile_with(vec![(
            "orphan",
            lock_entry("standalone", stale_hash, Some("orphan@mp"), Some("1.0.0")),
        )]);

        let result = detect_edited(&manifest, &lib, &lockfile).unwrap();
        assert!(
            result.is_empty(),
            "Unowned managed entries must not be edit-classified (D-14), got {result:?}"
        );
    }

    #[test]
    fn detect_edited_skips_when_hash_matches() {
        let tmp = TempDir::new().unwrap();
        let (_paths, lib) = paths_with_lib(tmp.path());
        let live_hash = make_skill_dir(&lib, "stable-skill", "unchanged");

        let mut manifest = Manifest::default();
        manifest.insert(
            SkillName::new("stable-skill").unwrap(),
            SkillEntry::new(
                PathBuf::from("/tmp/stable-skill"),
                DirectoryName::new("claude-plugins").unwrap(),
                live_hash.clone(),
                true,
            ),
        );

        let lockfile = lockfile_with(vec![(
            "stable-skill",
            lock_entry(
                "claude-plugins",
                live_hash,
                Some("stable-skill@mp"),
                Some("1.0.0"),
            ),
        )]);

        let result = detect_edited(&manifest, &lib, &lockfile).unwrap();
        assert!(
            result.is_empty(),
            "matching hash must not yield edit detection, got {result:?}"
        );
    }

    // ---------- Drift apply tests (RECON-03 + D-22) ----------

    #[test]
    fn apply_drift_succeeds_updates_working_lockfile() {
        let tmp = TempDir::new().unwrap();
        let (_paths, lib) = paths_with_lib(tmp.path());
        let new_a = make_skill_dir(&lib, "alpha", "new alpha");
        let new_b = make_skill_dir(&lib, "beta", "new beta");

        let stale_a = ContentHash::new("1".repeat(64)).unwrap();
        let stale_b = ContentHash::new("2".repeat(64)).unwrap();

        let mut mock = empty_mock("mp");
        mock.installed.push(fixture_plugin("alpha@mp", "2.0.0"));
        mock.installed.push(fixture_plugin("beta@mp", "3.0.0"));

        let drift = vec![
            Classified {
                name: SkillName::new("alpha").unwrap(),
                registry_id: "alpha@mp".to_string(),
                source_name: DirectoryName::new("claude-plugins").unwrap(),
                class: ReconcileClass::Drift {
                    old_version: Some("1.0.0".to_string()),
                    new_version: Some("2.0.0".to_string()),
                },
            },
            Classified {
                name: SkillName::new("beta").unwrap(),
                registry_id: "beta@mp".to_string(),
                source_name: DirectoryName::new("claude-plugins").unwrap(),
                class: ReconcileClass::Drift {
                    old_version: Some("2.5.0".to_string()),
                    new_version: Some("3.0.0".to_string()),
                },
            },
        ];

        let mut working = lockfile_with(vec![
            (
                "alpha",
                lock_entry("claude-plugins", stale_a, Some("alpha@mp"), Some("1.0.0")),
            ),
            (
                "beta",
                lock_entry("claude-plugins", stale_b, Some("beta@mp"), Some("2.5.0")),
            ),
        ]);
        let mut report = ReconcileReport::default();

        apply_drift_and_missing(&drift, &[], &mock, &lib, &mut working, &mut report, false)
            .unwrap();

        assert!(report.install_failures.is_empty());
        assert_eq!(working.skills["alpha"].content_hash, new_a);
        assert_eq!(working.skills["beta"].content_hash, new_b);
        assert_eq!(working.skills["alpha"].version.as_deref(), Some("2.0.0"));
    }

    #[test]
    fn apply_drift_partial_failure_only_updates_ok_entries() {
        let tmp = TempDir::new().unwrap();
        let (_paths, lib) = paths_with_lib(tmp.path());
        let new_a = make_skill_dir(&lib, "alpha", "new alpha");
        let _new_b = make_skill_dir(&lib, "beta", "new beta");

        let stale_a = ContentHash::new("1".repeat(64)).unwrap();
        let stale_b = ContentHash::new("2".repeat(64)).unwrap();

        let mut mock = empty_mock("mp");
        mock.installed.push(fixture_plugin("alpha@mp", "2.0.0"));
        mock.installed.push(fixture_plugin("beta@mp", "3.0.0"));
        mock.fail_update.insert("beta@mp".to_string()); // beta fails

        let drift = vec![
            Classified {
                name: SkillName::new("alpha").unwrap(),
                registry_id: "alpha@mp".to_string(),
                source_name: DirectoryName::new("claude-plugins").unwrap(),
                class: ReconcileClass::Drift {
                    old_version: Some("1.0.0".to_string()),
                    new_version: Some("2.0.0".to_string()),
                },
            },
            Classified {
                name: SkillName::new("beta").unwrap(),
                registry_id: "beta@mp".to_string(),
                source_name: DirectoryName::new("claude-plugins").unwrap(),
                class: ReconcileClass::Drift {
                    old_version: Some("2.5.0".to_string()),
                    new_version: Some("3.0.0".to_string()),
                },
            },
        ];

        let mut working = lockfile_with(vec![
            (
                "alpha",
                lock_entry("claude-plugins", stale_a, Some("alpha@mp"), Some("1.0.0")),
            ),
            (
                "beta",
                lock_entry(
                    "claude-plugins",
                    stale_b.clone(),
                    Some("beta@mp"),
                    Some("2.5.0"),
                ),
            ),
        ]);
        let mut report = ReconcileReport::default();

        apply_drift_and_missing(&drift, &[], &mock, &lib, &mut working, &mut report, false)
            .unwrap();

        assert_eq!(report.install_failures.len(), 1);
        assert_eq!(report.install_failures[0].plugin_id, "beta@mp");
        // alpha updated to new hash
        assert_eq!(working.skills["alpha"].content_hash, new_a);
        // beta stays at stale hash (D-22 partial-failure invariant)
        assert_eq!(working.skills["beta"].content_hash, stale_b);
        assert_eq!(working.skills["beta"].version.as_deref(), Some("2.5.0"));
    }

    #[test]
    fn apply_drift_skipped_when_no_install_flag() {
        let tmp = TempDir::new().unwrap();
        let (paths, lib) = paths_with_lib(tmp.path());
        let stale = ContentHash::new("1".repeat(64)).unwrap();
        let _ = make_skill_dir(&lib, "alpha", "body");

        let mut mock = empty_mock("mp");
        mock.installed.push(fixture_plugin("alpha@mp", "1.0.0"));
        mock.available.insert("alpha@mp".to_string());

        let lockfile = lockfile_with(vec![(
            "alpha",
            lock_entry("claude-plugins", stale, Some("alpha@mp"), Some("0.9.0")),
        )]);

        let mut prefs = MachinePrefs {
            auto_install_plugins: Some(AutoInstall::Always),
            ..Default::default()
        };
        let machine_path = tmp.path().join("machine.toml");

        let opts = ReconcileOpts {
            no_install: true, // <-- skip apply
            ..default_opts()
        };

        let report = reconcile_lockfile(
            Some(&lockfile),
            &Manifest::default(),
            &lib,
            &mock,
            &mut prefs,
            &machine_path,
            &paths,
            opts,
        )
        .unwrap();

        assert!(
            report.apply_skipped,
            "expected apply_skipped=true under --no-install"
        );
        assert!(report.install_failures.is_empty());
    }

    #[test]
    fn apply_drift_skipped_when_consent_never() {
        let tmp = TempDir::new().unwrap();
        let (paths, lib) = paths_with_lib(tmp.path());
        let stale = ContentHash::new("1".repeat(64)).unwrap();
        let _ = make_skill_dir(&lib, "alpha", "body");

        let mut mock = empty_mock("mp");
        mock.installed.push(fixture_plugin("alpha@mp", "1.0.0"));
        mock.available.insert("alpha@mp".to_string());

        let lockfile = lockfile_with(vec![(
            "alpha",
            lock_entry("claude-plugins", stale, Some("alpha@mp"), Some("0.9.0")),
        )]);

        let mut prefs = MachinePrefs {
            auto_install_plugins: Some(AutoInstall::Never),
            ..Default::default()
        };
        let machine_path = tmp.path().join("machine.toml");

        let report = reconcile_lockfile(
            Some(&lockfile),
            &Manifest::default(),
            &lib,
            &mock,
            &mut prefs,
            &machine_path,
            &paths,
            default_opts(),
        )
        .unwrap();

        assert!(
            report.apply_skipped,
            "expected apply_skipped=true under Never consent"
        );
        assert!(report.install_failures.is_empty());
    }

    // ---------- #512: Edit + Drift overlap regression (D-16 / RECON-05) ----------

    #[test]
    fn edited_skill_with_drift_is_steered_to_prompt_not_apply_loop() {
        // Regression for #512 / D-16: when a managed skill is BOTH classified
        // as Drift AND detected as Edited with auto_install_plugins = Always,
        // the prior implementation called adapter.update() before the
        // edit-in-library prompt could fire — for managed skills the library
        // symlinks the source dir, so claude plugin update would overwrite
        // the user's edits. RECON-05 / D-16: never silently overwrite edited
        // content. The fix steers the skill to handle_edited exclusively.
        use crate::manifest::SkillEntry;
        let tmp = TempDir::new().unwrap();
        let (paths, lib) = paths_with_lib(tmp.path());

        // The user edited library/alpha — its on-disk content_hash differs
        // from the value recorded in the lockfile. Both classify_lockfile
        // and detect_edited compare live_hash vs lock_entry.content_hash,
        // so both will flag this skill.
        let _live_hash = make_skill_dir(&lib, "alpha", "user edited body");
        let stale_lockfile_hash = ContentHash::new("1".repeat(64)).unwrap();

        // Manifest: alpha is managed + owned (source_name=Some). detect_edited
        // requires both flags AND a hash mismatch with the lockfile.
        let mut manifest = Manifest::default();
        manifest.insert(
            SkillName::new("alpha").unwrap(),
            SkillEntry::new(
                lib.join("alpha"),
                DirectoryName::new("claude-plugins").unwrap(),
                stale_lockfile_hash.clone(),
                true,
            ),
        );

        // Lockfile records the OLD (now-stale) content_hash.
        let lockfile = lockfile_with(vec![(
            "alpha",
            lock_entry(
                "claude-plugins",
                stale_lockfile_hash.clone(),
                Some("alpha@mp"),
                Some("1.0.0"),
            ),
        )]);

        // Mock adapter: alpha is installed, available, with a NEWER version
        // (so classify_lockfile would otherwise put it in Drift).
        let mut mock = empty_mock("mp");
        mock.installed.push(fixture_plugin("alpha@mp", "2.0.0"));
        mock.available.insert("alpha@mp".to_string());

        // Always consent + no_input → the prior bug would call adapter.update()
        // for alpha. With the fix, the skill is filtered out of drift_to_apply
        // and routed through handle_edited (which under no_input defaults to
        // Skip per D-16).
        let mut prefs = MachinePrefs {
            auto_install_plugins: Some(AutoInstall::Always),
            ..Default::default()
        };
        let machine_path = tmp.path().join("machine.toml");

        let report = reconcile_lockfile(
            Some(&lockfile),
            &manifest,
            &lib,
            &mock,
            &mut prefs,
            &machine_path,
            &paths,
            default_opts(),
        )
        .unwrap();

        // The skill must surface as Edited (handled by the prompt).
        assert_eq!(
            report.edited.len(),
            1,
            "edited skill must surface in report.edited"
        );
        assert_eq!(report.edited[0].name.as_str(), "alpha");

        // The skill must NOT be in report.drift — it was filtered out so the
        // prompt is the only path that touches it.
        assert!(
            report.drift.is_empty(),
            "edited skill must be filtered out of drift (would otherwise drive adapter.update)"
        );

        // No install failures — apply loop never ran for alpha.
        assert!(report.install_failures.is_empty());

        // Library content unchanged (the mock update wouldn't actually mutate
        // anything, but this is the user-visible safety property under D-16).
        let body = std::fs::read_to_string(lib.join("alpha").join("SKILL.md")).unwrap();
        assert!(body.contains("user edited body"));
    }

    // ---------- Consent state machine tests (RECON-02 + D-07/08/11) ----------

    #[test]
    fn consent_skip_when_no_input_and_unset() {
        let tmp = TempDir::new().unwrap();
        let (paths, lib) = paths_with_lib(tmp.path());
        let stale = ContentHash::new("1".repeat(64)).unwrap();
        let _ = make_skill_dir(&lib, "alpha", "body");

        let mut mock = empty_mock("mp");
        mock.installed.push(fixture_plugin("alpha@mp", "1.0.0"));
        mock.available.insert("alpha@mp".to_string());

        let lockfile = lockfile_with(vec![(
            "alpha",
            lock_entry("claude-plugins", stale, Some("alpha@mp"), Some("0.9.0")),
        )]);

        // None consent + no_input
        let mut prefs = MachinePrefs::default();
        let machine_path = tmp.path().join("machine.toml");
        let opts = ReconcileOpts {
            no_input: true,
            ..default_opts()
        };

        let report = reconcile_lockfile(
            Some(&lockfile),
            &Manifest::default(),
            &lib,
            &mock,
            &mut prefs,
            &machine_path,
            &paths,
            opts,
        )
        .unwrap();

        assert!(report.apply_skipped);
        // Pitfall 2: consent must NOT be persisted when non-interactive.
        assert!(prefs.auto_install_plugins.is_none());
        assert!(!machine_path.exists(), "machine.toml must not be touched");
    }

    #[test]
    fn consent_skip_when_no_input_and_ask() {
        let tmp = TempDir::new().unwrap();
        let (paths, lib) = paths_with_lib(tmp.path());
        let stale = ContentHash::new("1".repeat(64)).unwrap();
        let _ = make_skill_dir(&lib, "alpha", "body");

        let mut mock = empty_mock("mp");
        mock.installed.push(fixture_plugin("alpha@mp", "1.0.0"));
        mock.available.insert("alpha@mp".to_string());

        let lockfile = lockfile_with(vec![(
            "alpha",
            lock_entry("claude-plugins", stale, Some("alpha@mp"), Some("0.9.0")),
        )]);

        let mut prefs = MachinePrefs {
            auto_install_plugins: Some(AutoInstall::Ask),
            ..Default::default()
        };
        let machine_path = tmp.path().join("machine.toml");
        let opts = ReconcileOpts {
            no_input: true,
            ..default_opts()
        };

        let report = reconcile_lockfile(
            Some(&lockfile),
            &Manifest::default(),
            &lib,
            &mock,
            &mut prefs,
            &machine_path,
            &paths,
            opts,
        )
        .unwrap();

        assert!(report.apply_skipped);
        // Pitfall 2: Ask must NOT be promoted to Never just because we're
        // non-interactive. The persisted Ask survives untouched.
        assert_eq!(prefs.auto_install_plugins, Some(AutoInstall::Ask));
    }

    #[test]
    fn consent_apply_when_always() {
        let tmp = TempDir::new().unwrap();
        let (paths, lib) = paths_with_lib(tmp.path());
        let _ = make_skill_dir(&lib, "alpha", "new content");
        let stale = ContentHash::new("1".repeat(64)).unwrap();

        let mut mock = empty_mock("mp");
        mock.installed.push(fixture_plugin("alpha@mp", "2.0.0"));
        mock.available.insert("alpha@mp".to_string());

        let lockfile = lockfile_with(vec![(
            "alpha",
            lock_entry("claude-plugins", stale, Some("alpha@mp"), Some("1.0.0")),
        )]);

        let mut prefs = MachinePrefs {
            auto_install_plugins: Some(AutoInstall::Always),
            ..Default::default()
        };
        let machine_path = tmp.path().join("machine.toml");

        let report = reconcile_lockfile(
            Some(&lockfile),
            &Manifest::default(),
            &lib,
            &mock,
            &mut prefs,
            &machine_path,
            &paths,
            ReconcileOpts {
                no_input: true, // even with no_input, Always still applies
                ..default_opts()
            },
        )
        .unwrap();

        assert!(!report.apply_skipped);
        assert_eq!(report.drift.len(), 1);
        assert!(report.install_failures.is_empty());
    }

    // ---------- Summary rendering tests (RECON-01 + D-02/03/04) ----------

    fn fake_drift(name: &str, old: &str, new: &str) -> Classified {
        Classified {
            name: SkillName::new(name).unwrap(),
            registry_id: format!("{name}@mp"),
            source_name: DirectoryName::new("claude-plugins").unwrap(),
            class: ReconcileClass::Drift {
                old_version: Some(old.to_string()),
                new_version: Some(new.to_string()),
            },
        }
    }

    fn fake_vanished(name: &str, source: &str) -> Classified {
        Classified {
            name: SkillName::new(name).unwrap(),
            registry_id: format!("{name}@mp"),
            source_name: DirectoryName::new(source).unwrap(),
            class: ReconcileClass::Vanished {
                old_version: Some("1.0.0".to_string()),
            },
        }
    }

    #[test]
    fn render_summary_all_three_buckets_present() {
        let report = ReconcileReport {
            matches: 12,
            drift: vec![
                fake_drift("alpha", "5.0.5", "5.0.7"),
                fake_drift("beta", "2.0.0", "2.0.1"),
            ],
            vanished: vec![fake_vanished("ghost", "claude-plugins")],
            ..Default::default()
        };
        let out = format_summary(&report);
        // console::style ANSI reset codes can break literal regex checks;
        // strip ANSI for the assertion. We keep the assertion against the
        // semantic content (numbers + "match · " · " drift · " · " vanished").
        let stripped = strip_ansi(&out);
        assert!(
            stripped.contains("12 match · "),
            "missing match count: {stripped}"
        );
        assert!(
            stripped.contains(" 2 drift · "),
            "missing drift count: {stripped}"
        );
        assert!(
            stripped.contains(" 1 vanished"),
            "missing vanished count: {stripped}"
        );
    }

    #[test]
    fn render_summary_zero_buckets_still_print() {
        let report = ReconcileReport {
            matches: 5,
            drift: vec![],
            vanished: vec![],
            ..Default::default()
        };
        let out = format_summary(&report);
        let stripped = strip_ansi(&out);
        assert!(
            stripped.contains(" 0 drift · "),
            "missing zero drift bucket: {stripped}"
        );
        assert!(
            stripped.contains(" 0 vanished"),
            "missing zero vanished bucket: {stripped}"
        );
    }

    #[test]
    fn render_summary_all_match_prints_in_sync() {
        let report = ReconcileReport {
            matches: 7,
            ..Default::default()
        };
        let out = format_summary(&report);
        let stripped = strip_ansi(&out);
        assert!(
            stripped.contains("7 plugins in sync"),
            "missing in-sync line: {stripped}"
        );
    }

    #[test]
    fn render_drift_detail_lines() {
        let report = ReconcileReport {
            matches: 0,
            drift: vec![fake_drift("alpha", "5.0.5", "5.0.7")],
            ..Default::default()
        };
        let out = format_summary(&report);
        let stripped = strip_ansi(&out);
        assert!(
            stripped.contains("• alpha: 5.0.5 → 5.0.7"),
            "missing drift detail line: {stripped}"
        );
    }

    #[test]
    fn render_vanished_warning_per_skill() {
        let report = ReconcileReport {
            matches: 0,
            vanished: vec![fake_vanished("ghost", "claude-plugins")],
            ..Default::default()
        };
        let out = format_summary(&report);
        let stripped = strip_ansi(&out);
        assert!(
            stripped.contains(
                "warning: plugin ghost vanished from marketplace claude-plugins; using preserved library copy"
            ),
            "missing or wrong vanished warning: {stripped}"
        );
    }

    /// Strip ANSI escape sequences (CSI ... letter) for substring assertions.
    /// `console::style` may or may not emit them depending on TTY detection in
    /// the test runner; tests must work either way.
    fn strip_ansi(s: &str) -> String {
        let mut out = String::with_capacity(s.len());
        let mut chars = s.chars().peekable();
        while let Some(c) = chars.next() {
            if c == '\x1b' {
                // Skip ESC + '[' and everything up to a letter (CSI sequence).
                if let Some(&'[') = chars.peek() {
                    chars.next();
                    for inner in chars.by_ref() {
                        if inner.is_ascii_alphabetic() {
                            break;
                        }
                    }
                    continue;
                }
            }
            out.push(c);
        }
        out
    }

    // ---------- Save-chain tests (Pitfall 5) ----------

    #[test]
    fn consent_change_persists_immediately() {
        // Pitfall 5: setting + saving consent must happen as a unit so a
        // Ctrl-C between the prompt and sync-completion preserves the
        // user's choice. Tested via the `apply_consent_decision` helper
        // (factored out for exactly this reason).
        let tmp = TempDir::new().unwrap();
        let machine_path = tmp.path().join("machine.toml");

        let mut prefs = MachinePrefs::default();
        // No machine.toml exists yet.
        assert!(!machine_path.exists());

        apply_consent_decision(&mut prefs, AutoInstall::Always, &machine_path).unwrap();

        // BEFORE returning, machine.toml MUST be on disk with the choice.
        assert!(machine_path.exists());
        let loaded = machine::load(&machine_path).unwrap();
        assert_eq!(loaded.auto_install_plugins, Some(AutoInstall::Always));
    }

    // ---------- Lockfile save tests (D-22 + RESEARCH OQ-4 option a) ----------

    #[test]
    fn reconcile_writes_lockfile_when_drift_applied_ok() {
        let tmp = TempDir::new().unwrap();
        let (paths, lib) = paths_with_lib(tmp.path());
        let new_hash = make_skill_dir(&lib, "alpha", "new content");
        let stale_hash = ContentHash::new("1".repeat(64)).unwrap();

        let mut mock = empty_mock("mp");
        mock.installed.push(fixture_plugin("alpha@mp", "2.0.0"));
        mock.available.insert("alpha@mp".to_string());

        let lockfile = lockfile_with(vec![(
            "alpha",
            lock_entry(
                "claude-plugins",
                stale_hash.clone(),
                Some("alpha@mp"),
                Some("1.0.0"),
            ),
        )]);
        // Persist initial lockfile so we can confirm write happened.
        lockfile::save(&lockfile, paths.config_dir()).unwrap();

        let mut prefs = MachinePrefs {
            auto_install_plugins: Some(AutoInstall::Always),
            ..Default::default()
        };
        let machine_path = tmp.path().join("machine.toml");

        let _report = reconcile_lockfile(
            Some(&lockfile),
            &Manifest::default(),
            &lib,
            &mock,
            &mut prefs,
            &machine_path,
            &paths,
            default_opts(),
        )
        .unwrap();

        let on_disk = lockfile::load(paths.config_dir()).unwrap().unwrap();
        assert_eq!(
            on_disk.skills["alpha"].content_hash, new_hash,
            "lockfile on disk should reflect new (post-update) hash"
        );
        assert_ne!(on_disk.skills["alpha"].content_hash, stale_hash);
    }

    #[test]
    fn reconcile_dry_run_does_not_write_lockfile_or_machine_toml() {
        let tmp = TempDir::new().unwrap();
        let (paths, lib) = paths_with_lib(tmp.path());
        let _ = make_skill_dir(&lib, "alpha", "new content");
        let stale_hash = ContentHash::new("1".repeat(64)).unwrap();

        let mut mock = empty_mock("mp");
        mock.installed.push(fixture_plugin("alpha@mp", "2.0.0"));
        mock.available.insert("alpha@mp".to_string());

        let lockfile = lockfile_with(vec![(
            "alpha",
            lock_entry(
                "claude-plugins",
                stale_hash,
                Some("alpha@mp"),
                Some("1.0.0"),
            ),
        )]);
        // Persist initial lockfile, capture bytes for byte-equal assertion.
        lockfile::save(&lockfile, paths.config_dir()).unwrap();
        let lockfile_bytes_before = std::fs::read(paths.config_dir().join("tome.lock")).unwrap();

        let mut prefs = MachinePrefs {
            auto_install_plugins: Some(AutoInstall::Always),
            ..Default::default()
        };
        let machine_path = tmp.path().join("machine.toml");
        // machine.toml does not exist — confirm dry_run leaves it absent.
        assert!(!machine_path.exists());

        let _report = reconcile_lockfile(
            Some(&lockfile),
            &Manifest::default(),
            &lib,
            &mock,
            &mut prefs,
            &machine_path,
            &paths,
            ReconcileOpts {
                dry_run: true,
                ..default_opts()
            },
        )
        .unwrap();

        let lockfile_bytes_after = std::fs::read(paths.config_dir().join("tome.lock")).unwrap();
        assert_eq!(
            lockfile_bytes_before, lockfile_bytes_after,
            "dry_run must NOT write lockfile"
        );
        assert!(
            !machine_path.exists(),
            "dry_run must NOT create machine.toml"
        );
    }

    // ---------- Edit-decision tests (Plan 13-04 Task 1) ----------

    #[test]
    fn handle_edited_no_input_returns_all_skip() {
        // Build a 2-entry edited Vec, run handle_edited under no_input=true,
        // assert decisions are [Skip, Skip]. No interactive prompt fires.
        let opts = ReconcileOpts {
            no_input: true,
            ..default_opts()
        };
        let edited = vec![
            Edited {
                name: SkillName::new("alpha").unwrap(),
                old_source: DirectoryName::new("claude-plugins").unwrap(),
                old_version: Some("1.0.0".to_string()),
            },
            Edited {
                name: SkillName::new("beta").unwrap(),
                old_source: DirectoryName::new("claude-plugins").unwrap(),
                old_version: Some("2.0.0".to_string()),
            },
        ];
        let lockfile = lockfile_with(vec![]);
        let mut report = ReconcileReport::default();

        handle_edited(&edited, &lockfile, &opts, &mut report).unwrap();

        assert_eq!(
            report.edit_decisions,
            vec![EditDecision::Skip, EditDecision::Skip],
            "no_input must yield Skip per edited entry, got {:?}",
            report.edit_decisions
        );
    }

    #[test]
    fn edit_decision_serialization_compile_check() {
        // Compile-time only: prove every variant is constructible from outside
        // `mod tests` (i.e. the enum is `pub`). If the enum is removed or
        // privatised this test fails to compile.
        let _f = EditDecision::Fork;
        let _r = EditDecision::Revert;
        let _s = EditDecision::Skip;
        // Sanity: distinct variants compare unequal.
        assert_ne!(EditDecision::Fork, EditDecision::Revert);
        assert_ne!(EditDecision::Revert, EditDecision::Skip);
        assert_ne!(EditDecision::Fork, EditDecision::Skip);
    }

    #[test]
    fn report_default_edit_decisions_empty() {
        let report = ReconcileReport::default();
        assert!(
            report.edit_decisions.is_empty(),
            "default ReconcileReport must have empty edit_decisions, got {:?}",
            report.edit_decisions
        );
    }
}
