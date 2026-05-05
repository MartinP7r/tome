# Phase 13: Lockfile-authoritative sync - Context

**Gathered:** 2026-05-05
**Status:** Ready for planning

<domain>
## Phase Boundary

`tome.lock` becomes the authoritative state for what's installed on every machine. `tome sync` reconciles drift via `ClaudeMarketplaceAdapter` (Phase 12), surfaces drift interactively, and never silently overwrites user content.

**In scope:**

- Match / Drift / Vanished classification of every managed lockfile entry on every sync (RECON-01).
- First-time-on-machine consent prompt for auto-installing missing/drifted plugins, persisted in `machine.toml` (RECON-02).
- Drift apply flow: render per-skill diff, invoke `adapter.install`/`adapter.update`, re-discover, verify resulting `content_hash` matches the freshly-recorded lockfile entry (RECON-03).
- Vanished plugin surfacing: per-skill stderr warning + summary count; downstream distribution still works from the preserved library copy (RECON-04).
- Edit-in-library detection (managed + content_hash mismatch) with **fork (default) / revert / skip** prompt; in `--no-input` mode, default to skip-with-warning, exit zero (RECON-05).
- Single-line per-class summary `✓ N match · ⚠ N drift · ⚠ N vanished` after every sync.
- Replacement of v0.9-era `crates/tome/src/install.rs` (and its `installed_plugins.json` parser + per-sync confirmation prompt) with the new adapter-based flow.

**Out of scope** (handled by other phases):

- `tome adopt` / `tome forget` lifecycle commands → Phase 14 (UNOWN-01, UNOWN-02).
- Surfacing Unowned skills in `tome status` / `tome doctor` → Phase 14 (UNOWN-03).
- Provenance history fields on `SkillEntry` (`previous_source`, `previous_provenance`) → deferred to Phase 14 if/when status/doctor needs them. Phase 13's fork-in-place is **lossy** (D-13).
- Cleanup-message UX rewrite (3-bucket partition for stale skills) → Phase 16 (UX-01).
- `GitAdapter` participation in the unified reconciliation grammar → not in v0.10. Git stays in `resolve_git_directories` as today (D-21).
- True version pinning per managed plugin → blocked on upstream Claude Code feature; design doc OQ-5.

</domain>

<decisions>
## Implementation Decisions

### Drift classification & sync summary (RECON-01)

- **D-01 (drift signal):** Hash-only. Drift means `lockfile.content_hash != freshly-computed library content_hash`. The `version` string from `claude plugin list --json` (or lockfile) is **display-only** — it annotates the diff line but never causes drift on its own. Honors Phase 11 D-08 strictly. Avoids false-positive drift on plugins where Claude Code bumps version metadata without changing skill content.

  RECON-01's wording in `.planning/REQUIREMENTS.md` says "version differs from lockfile or older" — this is a known wording carry-over from the design doc; the planner should flag in traceability that drift basis = hash, not version.

- **D-02 (summary format):** Single line, terse:
  ```
  ✓ 12 match · ⚠ 2 drift · ⚠ 1 vanished
  ```
  Drift / vanished items expand below the line as bulleted detail (D-05). Matches existing `tome sync` brevity.

- **D-03 (all-match output):** Print one-line confirmation `✓ N plugins in sync` so the user sees positive evidence reconciliation ran. Consistent with `tome sync`'s existing "No changes since last sync." line.

- **D-04 (bucket visibility):** Always render all three buckets in the summary line, even when zero. `✓ 12 match · ⚠ 0 drift · ⚠ 0 vanished` is acceptable. Predictable output is greppable across runs.

- **D-05 (drift detail):** When drift items exist, render `  • <skill>: <old_version> → <new_version>` per item below the summary line. Reads `version` from lockfile + adapter; non-fatal if either is missing (`unknown` placeholder).

- **D-06 (vanished UX):** Per-skill stderr warning **plus** the summary count. Each vanished plugin emits:
  ```
  warning: plugin <skill> vanished from marketplace <id>; using preserved library copy
  ```
  Phase 13 reads the signal directly from `InstalledPlugin.errors[]` per Phase 12 D-02 — zero extra subprocess calls.

### Auto-install consent state machine (RECON-02)

- **D-07 (persisted state):** `machine.toml::auto_install_plugins` is a 3-state enum (serialized as a string field):
  ```toml
  auto_install_plugins = "always" | "ask" | "never"
  ```
  Unset = treat as first-time prompt. Use a dedicated enum in `machine.rs`:
  ```rust
  #[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
  #[serde(rename_all = "lowercase")]
  pub enum AutoInstall { Always, Ask, Never }
  ```
  Field is `Option<AutoInstall>` (None = unset = first-time prompt). Serialize via `#[serde(skip_serializing_if = "Option::is_none")]`.

- **D-08 (first prompt):** 3-way `[Y/n/never]` with default `Y`. Map:
  - `Y` (or empty) → persist `Always`. Drift gets auto-applied silently with a per-skill diff summary.
  - `n` → persist `Ask`. Re-prompt every sync that detects drift.
  - `never` → persist `Never`. Drift surfaces as warning; no install/update.

  Prompt copy (planner can refine within these bounds):
  ```
  Tome detected N missing or out-of-date managed plugins. Install/update them now?
    Y       — yes, and always do this on future syncs (recommended)
    n       — yes for this sync; ask me again next time
    never   — no, and don't ask again on this machine
  [Y/n/never]
  ```

- **D-09 (--no-install scope):** Single-run override only. `--no-install` flag on `tome sync` skips install/update calls for THIS run. Doesn't touch the persisted `auto_install_plugins` setting. Mirrors Cargo's `--frozen` / `--locked` model. Clean separation between transient and durable state.

- **D-10 (trigger condition):** Prompt fires when **Drift OR missing-from-machine**. Specifically:
  - **Drift**: lockfile entry exists, plugin installed locally, content_hash differs.
  - **Missing-from-machine**: lockfile entry exists, plugin not in `adapter.list_installed()` output AND `adapter.available()` returns true (i.e. installable).
  - **Vanished** (`available()` returns false) does NOT trigger install consent — there's nothing to install. Vanished only emits warnings (D-06).

- **D-11 (Ask state behavior):** When `auto_install_plugins = "ask"` (user picked `n` previously), every sync that detects drift OR missing re-prompts with the full `[Y/n/never]` choice. The user always has an escape to `never` mid-prompt-loop. Honors design doc OQ-3.

- **D-12 (doctor reports drift unconditionally):** `tome doctor` always reports drift as a warning regardless of consent state. Consent only controls whether `tome sync` ACTS on drift, not whether tome SURFACES it. Doctor is the diagnostic tool; drift is a fact.

### Edit-in-library prompt (RECON-05)

- **D-13 (fork semantic — lossy in-place flip):** When the user picks "fork", Phase 13 flips manifest metadata in place:
  - `managed: true → false`
  - `source_name: Some(<dir>) → None`
  - Library content stays at `library_dir/<skill>/` (already edited; that's why we're prompting).

  No new schema. **Provenance history (e.g. `previous_source`, `previous_provenance`) is dropped on the floor.** Phase 14 may retroactively add history fields when building UNOWN-03 status/doctor surfacing; pre-Phase-14 forked entries will permanently have empty history. Accepted one-time UX gap.

  This is **distinct from today's `tome fork <skill> --to <dir>`**, which copies the skill out of the library into a separate Source directory. Phase 13's fork-in-place leaves the library entry as the canonical home. Both flavors of "fork" coexist; the prompt's "fork" choice is the in-place variant.

- **D-14 (Unowned bypass):** Detection gate is:
  ```
  manifest[name].managed == true
   && manifest[name].source_name.is_some()
   && content_hash(library/<name>) != lockfile.content_hash
  ```
  Unowned skills (`source_name = None`) never trigger the prompt — there's no upstream to revert to and the entry is already user-canonical.

- **D-15 (prompt content):** Show the source + version being severed. Read directly from the lockfile at prompt time (the data is there even though we won't persist it post-flip). Prompt copy (planner can refine within these bounds):
  ```
  <skill> has local edits. Last upstream: <source_name> @ <version>.
    fork    — keep your edits, sever the upstream link (default)
    revert  — discard your edits, restore <source_name> @ <version>
    skip    — warn and don't touch this entry this sync
  [F/r/s]
  ```
  Default = `fork` (preserves user content; safest outcome).

- **D-16 (--no-input exit code):** Skip-with-warning, exit **zero**. Edits are user-intentional; tome's job is to not overwrite them, not to fail the sync. Mirrors today's `install.rs::reconcile` --no-input behavior.

### `install.rs` integration boundary (RECON-01..05 wiring)

- **D-17 (`install.rs` fate):** Delete entirely in Phase 13. `ClaudeMarketplaceAdapter` becomes the single canonical reconciliation path. Removes `crates/tome/src/install.rs`, `installed_plugins.json` parsing, and the per-sync `Install N plugin(s)?` confirmation prompt. Reduces surface area; aligns with HARD cluster goal of trimming legacy.

- **D-18 (sync flow position):** New adapter-based reconciliation **replaces `reconcile_managed_plugins`** at the same slot in `lib.rs::sync` (line 978 today, "before discovery"). Adapter installs missing plugins and updates drifted plugins → discovery sees the result → consolidate copies into library → distribute symlinks unchanged. Minimal flow disturbance.

- **D-19 (per-skill prompt removed):** The legacy per-sync `Install N plugin(s)? [Y/N]` confirmation goes away. The 3-state `auto_install_plugins` consent (D-07/D-08) governs install behavior — user makes the decision once (or per-sync via `Ask`), not via two layered prompts.

- **D-20 (claude binary missing):** Hard error. When `[directories.claude-plugins]` is configured but `claude` isn't on PATH, `tome sync` exits non-zero with an actionable message:
  ```
  error: claude binary not found on PATH.
  Install Claude Code, or remove [directories.claude-plugins] from tome.toml.
  ```
  No JSON-file fallback. Planner: `which::which("claude")` at adapter construction (also covered by Phase 12 Claude's-Discretion bullet).

- **D-21 (GitAdapter scope):** Git stays separate from Phase 13's reconciliation flow. `resolve_git_directories` keeps its current pre-discovery role (line 994 in `lib.rs::sync`). `GitAdapter` exists for trait-shape parity (Phase 12) but Phase 13 does NOT call it. Honors Phase 12 D-05a (byte-for-byte regression contract for git).

  Match/Drift/Vanished classification therefore applies to **claude-plugins type entries only** in v0.10. The summary line counts only managed (claude) plugins. Git directories don't appear in the count.

- **D-22 (lockfile timing on partial failure):** Per-skill in-memory update after each successful adapter call; failed calls leave the corresponding entry at its previous lockfile value. After the loop, write the lockfile to disk once (atomic temp+rename per existing `lockfile::save` pattern). Matches design doc OQ-4: "lockfile only updates entries that succeeded; failed entries stay at previous lockfile state."

### Carried forward from prior phases (locked, do not re-decide)

- **Phase 11 D-08:** Drift basis = `content_hash`, not `version`. → Honored by D-01.
- **Phase 11 LIB-04:** Source removal → Unowned transition. Phase 13's edit-in-library fork semantic uses the same Unowned state.
- **Phase 12 D-08:** `MarketplaceAdapter` trait surface (six methods) is locked. Phase 13 calls these methods; doesn't change the trait.
- **Phase 12 D-11:** Adapter dispatch by `DirectoryType`. Phase 13 owns the dispatcher (`Box<dyn MarketplaceAdapter>` per directory).
- **Phase 12 D-02:** Vanished signal from cached `errors[]` field. → Honored by D-06 (zero extra subprocess calls).
- **Phase 12 D-04:** ClaudeMarketplaceAdapter snapshot cache auto-invalidates on Ok install/update. Phase 13 doesn't manually invalidate.
- **Phase 12 D-09:** Install scope = `user` default. Phase 13 doesn't pass `--scope`.
- **Phase 12 ADP-04:** `InstallFailure` + `format_install_failures` + `render_install_failures` already shipped in `marketplace.rs`. Phase 13 calls `render_install_failures(&failures)` after the reconcile loop.
- **Phase 8 SAFE-01:** Grouped failure summary pattern. → Already absorbed via ADP-04 above.

### Claude's Discretion

- Exact prompt copy text (within the bounds shown in D-08 and D-15). Planner can refine wording for clarity; the structure (option labels, default markers) is fixed.
- Internal organization of the new reconciliation function (e.g. `reconcile_lockfile(lockfile, adapter, prefs, ...) -> Result<ReconcileReport>` vs split into smaller helpers). Planner picks based on testability needs.
- Whether the Match/Drift/Vanished classification logic lives in `marketplace.rs`, a new `reconcile.rs` module, or inline in `lib.rs::sync`. Recommendation: new `reconcile.rs` for testability (mirrors the `update.rs` pattern).
- Whether `AutoInstall` enum lives in `machine.rs` (alongside `MachinePrefs`) or a sub-module. Recommendation: top-level in `machine.rs`, near `DirectoryOverride`.
- Mock test surface: whether `MockMarketplaceAdapter` (today `#[cfg(test)]` per Phase 12 D-10) gets lifted to `pub(crate) marketplace::testing` for Phase 13's integration tests, or stays unit-test-only. Tactical Phase 13 decision.
- Exact error wording for "claude binary not found" (D-20) — within the Conflict / Why / Suggestion template.

### Folded Todos

(None — `gsd-tools.cjs todo match-phase 13` returned 0 matches.)

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### v0.10 design + planning

- `.planning/research/v0.10-library-canonical-design.md` — full v0.10 design exploration. OQ-3 (auto-install consent), OQ-4 (partial-failure handling), OQ-6 (lockfile divergence) are the specific resolutions Phase 13 inherits.
- `.planning/REQUIREMENTS.md` §"Lockfile-authoritative reconciliation (RECON)" — RECON-01..05 verbatim. **Note for traceability:** RECON-01's "version differs" wording is superseded by D-01 (content_hash is the drift signal). Planner should flag this in the traceability check.
- `.planning/ROADMAP.md` §"Phase 13: Lockfile-authoritative sync" — success criteria 1–5 are the verification anchors.
- `.planning/PROJECT.md` §"Current Milestone: v0.10" — milestone-level decisions and behavior-change documentation requirements.

### Phase 11 (predecessor — manifest/lockfile contract)

- `.planning/phases/11-library-canonical-core/11-CONTEXT.md` — D-08 drift basis (content_hash, not version). LIB-03/04 lock the `Option<DirectoryName>` schema Phase 13 reads when classifying Unowned skills.
- `.planning/phases/11-library-canonical-core/11-VERIFICATION.md` — confirms manifest schema state Phase 13 builds on.

### Phase 12 (predecessor — adapter trait + adapters shipped)

- `.planning/phases/12-marketplace-adapter/12-CONTEXT.md` — D-01..D-11. Phase 13 calls the adapters Phase 12 shipped; trait surface is locked. Specifically: D-02 (vanished signal), D-04 (auto-invalidate cache), D-09 (default scope), D-11 (dispatch by DirectoryType).
- `crates/tome/src/marketplace.rs` — concrete adapters + `InstallFailure` + `render_install_failures`. Phase 13 wires these into `lib.rs::sync`.

### Existing code to delete or modify

- `crates/tome/src/install.rs` — **DELETE in Phase 13** per D-17. Today's `installed_plugins.json` parser, `find_missing`, `reconcile`, `find_installed_plugins_json`. ClaudeMarketplaceAdapter replaces all of it.
- `crates/tome/src/lib.rs::sync` (line 913) — replace `reconcile_managed_plugins` call (line 978) with new adapter-based reconciliation per D-18.
- `crates/tome/src/lib.rs::reconcile_managed_plugins` (line 1617) — DELETE per D-17/D-18.
- `crates/tome/src/machine.rs` — add `auto_install_plugins: Option<AutoInstall>` field + `AutoInstall` enum per D-07. Atomic save pattern unchanged.
- `crates/tome/src/cli.rs::Command::Sync` — add `#[arg(long)] no_install: bool` per D-09.
- `crates/tome/src/lib.rs::SyncOptions` — propagate `no_install` to the new reconciliation function.

### New code shipped in this phase (provisional layout — planner decides)

- `crates/tome/src/reconcile.rs` (new module) — Match/Drift/Vanished classification, prompt orchestration, drift-apply loop, summary rendering. Imports `marketplace::*`. Exposes `reconcile_lockfile(...) -> Result<ReconcileReport>` to `lib.rs::sync`.
- New types likely in `reconcile.rs`:
  - `enum ReconcileClass { Match, Drift { old_version: Option<String>, new_version: Option<String> }, Vanished }`
  - `struct ReconcileReport { ... summary counts + per-skill details }`
- New types in `machine.rs`:
  - `enum AutoInstall { Always, Ask, Never }`

### Patterns to follow (no behavior change to these modules; prior art)

- `crates/tome/src/update.rs` — existing `diff` + `present_changes` pattern for lockfile diffing. Phase 13's drift-apply prompt follows the same plan/render/execute shape.
- `crates/tome/src/remove.rs::FailureKind` + `format_install_failures` — SAFE-01 grouped-failure rendering. Already wired into `marketplace.rs`; Phase 13 just calls `render_install_failures(&failures)`.
- `crates/tome/src/wizard.rs` — `dialoguer::Select` patterns for 3-way prompts (the [Y/n/never] dialog uses `dialoguer::Confirm` won't fit; planner picks `Select` or custom 3-key prompt).
- `crates/tome/src/manifest.rs::save` / `lockfile.rs::save` — atomic temp+rename pattern. Phase 13's lockfile write at end of reconcile loop (D-22) uses `lockfile::save`.

### Tests to write (not exhaustive — research/planner can extend)

- Unit: `reconcile.rs::tests` — Match/Drift/Vanished classification given mock adapter + synthetic lockfile/manifest.
- Unit: `reconcile.rs::tests` — drift-apply loop with partial failure (some installs Ok, some Err); verify lockfile entries updated only for Ok cases (D-22).
- Unit: `reconcile.rs::tests` — edit-in-library detection gate exhaustively (managed=true/false × source_name=Some/None × hash match/mismatch).
- Unit: `machine.rs::tests` — `AutoInstall` round-trip via TOML; `Option<AutoInstall>` skip-on-None.
- Integration (`tests/cli.rs` or `tests/cli_sync.rs` per HARD-13 split): full sync flow with `MockMarketplaceAdapter` injected — fresh-machine bootstrap (Always/Ask/Never paths), drift-apply success + failure, vanished plugin distribution from preserved library copy (RECON-04 anchor), edit-in-library prompt in interactive + `--no-input` modes.

### Adjacent issues (won't fix in Phase 13, but be aware)

- **HARD-13** (Phase 15): `tests/cli.rs` split into per-domain files. New Phase 13 integration tests should land in `tests/cli_sync.rs` (or whatever shape Phase 15 settles on) to minimize churn at split time.
- **HARD-07** (Phase 15): `(verbose, quiet)` → `LogLevel` enum. Phase 13's reconcile output uses today's `quiet`/`verbose` bools.
- **HARD-02** (Phase 15): `lib.rs::run` decomposition. Phase 13's new reconciliation in `sync` follows current pattern; Phase 15 may move it into `cmd_sync` helper.

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable assets

- **`crates/tome/src/marketplace.rs::ClaudeMarketplaceAdapter`** — already implements `list_installed`, `install`, `update`, `current_version`, `available` per Phase 12. Internal `RefCell` cache auto-invalidates on Ok install/update (D-04). Phase 13 just calls these methods.
- **`crates/tome/src/marketplace.rs::render_install_failures`** — SAFE-01-shaped grouped renderer for `Vec<InstallFailure>`. Already shipped; Phase 13 calls it after the reconcile loop.
- **`crates/tome/src/lockfile.rs::Lockfile`, `LockEntry`** — `LockEntry.source_name: Option<DirectoryName>` already shipped (Phase 11 D-14). `content_hash`, `version`, `registry_id`, `git_commit_sha` are the fields Phase 13 reads for drift classification.
- **`crates/tome/src/manifest.rs::hash_directory`** — deterministic SHA-256 directory hash. Phase 13 calls this post-install/update to verify the new library content matches what the adapter produced.
- **`crates/tome/src/update.rs::diff` + `present_changes`** — existing lockfile diffing for skill-level changes. Pattern to mirror in `reconcile.rs::present_drift`.
- **`dialoguer::Select`** — for 3-way `[Y/n/never]` prompt (D-08). `Confirm` is 2-way only.

### Established patterns

- **`anyhow::Result + .with_context()`** everywhere; Phase 13 mirrors.
- **Plan / render / execute** for any flow that mutates filesystem state: drift-apply uses this shape (`plan_reconcile` → `render_drift_summary` → `apply_reconcile`). Dry-run is free.
- **Atomic temp+rename** for any on-disk writes: `lockfile::save` at end of reconcile loop, `machine::save` after persisting consent (D-07/D-08).
- **`#[cfg(test)] mod tests`** — co-located unit tests in each module.
- **Mock adapter for unit tests** — Phase 12 already ships `MockMarketplaceAdapter`. Phase 13 may need to lift it to `pub(crate) testing` if integration tests want it (D-10 in Phase 12 CONTEXT).

### Integration points

- **`crates/tome/src/lib.rs::sync` (line 913)** — entry point for the reconciliation flow. Replace `reconcile_managed_plugins(...)` call (line 978) with `reconcile::reconcile_lockfile(...)` per D-18.
- **`crates/tome/src/lib.rs::SyncOptions`** — add `no_install: bool` field per D-09. Plumb through from CLI.
- **`crates/tome/src/cli.rs::Command::Sync`** — add `#[arg(long)] no_install: bool` flag per D-09.
- **`crates/tome/src/machine.rs::MachinePrefs`** — add `auto_install_plugins: Option<AutoInstall>` field per D-07. Plumb the new prompt + persistence through the existing save chain.
- **`crates/tome/src/cleanup.rs`** — unchanged in Phase 13. Vanished plugins are preserved per LIB-04 (Phase 11) and surface as warnings (D-06); cleanup phase's "stale = transition to Unowned" branch already handles them.

### Constraints from existing architecture

- **Unix-only** (per project policy). All adapter ops are subprocess + filesystem; matches.
- **No tokio / async**. ClaudeMarketplaceAdapter is synchronous; Phase 13 stays synchronous.
- **Edition 2024 / strict clippy** (`-D warnings`). New `AutoInstall` enum + `ReconcileClass` need full `Debug` + `Clone` + appropriate serde derives.
- **`installed_plugins.json` will no longer be read by tome after Phase 13.** Claude Code itself still maintains it; tome just doesn't depend on it. `find_installed_plugins_json` (in `install.rs`) goes away with the file deletion.

</code_context>

<specifics>
## Specific Ideas

- **Empirical check from Phase 12 discussion:** `claude plugin update <plugin>` exits 1 with `Plugin "<plugin>" not found` when given a bare ID (no `@marketplace` qualifier). Phase 12 left the exact `update()` ID format as a "Specifics — to verify" item; Phase 13's reconciliation flow will exercise the actual contract. Planner: pass through whatever ID format the lockfile records (i.e. the `registry_id` field, which is the qualified form like `superpowers@claude-plugins-official`).

- **Auto-install consent prompt timing:** the prompt happens AFTER the Match/Drift/Vanished classification produces a non-empty Drift+Missing list, BEFORE the apply step. Don't prompt before classification (would prompt every sync regardless of state).

- **Lockfile entries that have `registry_id: None`:** these are local skills that ended up in the lockfile via `lockfile::generate`. They have no marketplace identity. Phase 13's reconciliation **skips** them (they're not managed; they don't participate in Match/Drift/Vanished).

- **Edit-in-library detection runs on every sync, regardless of `auto_install_plugins` state.** Even with `auto_install_plugins = always`, edited managed skills get the prompt — never silently overwrite user content. The consent governs install/update; the edit prompt is a separate safety layer.

- **For `--no-input` + drift detected + `auto_install_plugins = always`:** apply silently. This is the dotfiles-on-Machine-B happy path. No prompts, just per-skill diff lines + a "applied N updates" summary.

- **For `--no-input` + drift detected + `auto_install_plugins = ask`:** treat `ask` as `never` for this run (can't prompt non-interactively). Surface drift as warning. Don't persist any state change. Same shape for `auto_install_plugins = unset`.

- **Recommended file layout** (Claude's Discretion confirmed): new `crates/tome/src/reconcile.rs` module exports `reconcile_lockfile(...)`, `ReconcileClass`, `ReconcileReport`. Tests co-located. `marketplace.rs` stays adapter-only.

</specifics>

<deferred>
## Deferred Ideas

These came up during discussion but belong in other phases or out of v0.10 entirely:

- **Provenance history fields on `SkillEntry`** (`previous_source: Option<DirectoryName>`, `previous_provenance: Option<SkillProvenance>`) — defer to Phase 14. UNOWN-03 (status/doctor surfacing) is the natural consumer; Phase 14 owns the schema lift if it adds these. Phase 13's lossy fork (D-13) accepts a one-time UX gap on already-forked entries.

- **`tome status` / `tome doctor` Unowned-set surfacing** — Phase 14 (UNOWN-03). Phase 13 makes Unowned reachable via fork-in-place; Phase 14 wires the user-facing display.

- **GitAdapter participation in unified Match/Drift/Vanished classification** — out of v0.10. Git stays in `resolve_git_directories` per D-21. If a future milestone wants symmetric reconciliation across adapters, that's a Phase 14+ revisit (and the right time to relax Phase 12 D-05a's byte-for-byte regression contract).

- **`--strict-edits` flag** for non-zero exit on edit-in-library skip in `--no-input` — not added in v0.10. D-16 picks zero-exit; if a CI use case surfaces, add the flag in HARD or a v0.11 follow-up.

- **Per-plugin version pinning in `tome.toml::[directories.claude-plugins.pins]`** — design doc OQ-5: blocked on upstream `claude plugin install --version`. Lockfile-record-only is the v0.10 reproducibility level.

- **Migration of `installed_plugins.json`-based reconciliation logic to a separate fallback adapter** — D-17 deletes it outright. If a future user lacks `claude` but still uses the library on the same machine, they remove `[directories.claude-plugins]` from `tome.toml` (per D-20 error message). No fallback path.

- **RECON-01 wording cleanup in REQUIREMENTS.md** — D-01 supersedes the literal "version differs" phrasing. Tactical fix: planner flags this in traceability; cleanup commit updates REQUIREMENTS.md. Not blocking Phase 13 work.

- **Auto-snapshot before reconcile-apply** — Phase 13 doesn't add an extra snapshot beyond the existing pre-sync auto-snapshot (`config.backup.auto_snapshot`). Drift-apply is reversible-ish (re-run `tome sync` against the original lockfile or `git checkout tome.lock`); a dedicated snapshot is overkill.

- **Doctor reporting Vanished separately** — out of Phase 13's surface area. Doctor today reports symlink integrity + path overlaps; adding a "managed plugin marketplace status" check is Phase 14 / status-surface-area territory.

- **Telemetry / metric for sync reconciliation outcomes** — never. Project policy: no telemetry.

### Reviewed Todos (not folded)

(None — `gsd-tools.cjs todo match-phase 13` returned 0 matches.)

</deferred>

---

*Phase: 13-lockfile-authoritative-sync*
*Context gathered: 2026-05-05*
