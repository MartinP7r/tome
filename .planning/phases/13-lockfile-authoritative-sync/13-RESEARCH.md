# Phase 13: Lockfile-authoritative sync — Research

**Researched:** 2026-05-06
**Domain:** sync orchestration, marketplace adapter wiring, dialoguer prompt UX, partial-failure aggregation
**Confidence:** HIGH

## Summary

CONTEXT.md is unusually well-decided (22 locked decisions D-01..D-22). This research deliberately does not re-litigate those choices. Instead it surfaces six implementation-level unknowns the planner must resolve before writing tasks: (1) the precise `claude plugin update` argument shape, (2) `dialoguer` patterns for the 3-key `[Y/n/never]` and `[F/r/s]` prompts in a 0.12 codebase that does not have a single 3-way prompt today, (3) where the `auto_install_plugins` save fits in the existing two-call save chain inside `lib.rs::sync`, (4) `MockMarketplaceAdapter` exposure semantics for integration testing, (5) edge cases not covered by the 5 success criteria (notably `registry_id: None` skips, `--no-input` × `Ask`/`Unset` fallthrough, vanished + edited intersection), and (6) stdin-injection for the new prompts inside `assert_cmd` — which is *not* the same problem as the Confirm prompts that already gate behind `is_terminal()`.

All findings are HIGH confidence: every claim is verified against the actual code at the line numbers cited (read 2026-05-06), the locked Phase 12 contract in `marketplace.rs`, or the live dialoguer 0.12 source. No WebSearch involved — the questions are about *this* codebase, not the wider ecosystem.

**Primary recommendation:** Build `reconcile.rs` as a new module exposing exactly one entry point `reconcile_lockfile(lockfile, manifest, adapter, prefs, opts) -> Result<ReconcileReport>` — match the `update.rs::diff` + `present_changes` shape that the planner already expects per CONTEXT.md. Lift `MockMarketplaceAdapter` to `pub(crate)` testing surface so integration tests can construct one without re-implementing the trait. Save `machine.toml` *immediately* on consent change inside `reconcile_lockfile` rather than threading it through to the existing post-triage save site (the two saves are at different times for different reasons). Pass `registry_id` verbatim from the lockfile (qualified `axiom@axiom-marketplace`-shaped IDs) to `adapter.update()` — this is the format Phase 12 already records and the form `claude plugin update` accepts per Phase 12 D-08 unblocked findings. Default `--no-install` to a sentinel `bool` field on `SyncOptions` and treat `Ask`/`Unset` consent as `Skip` whenever the prompt cannot be shown (i.e. always under `--no-input` or non-TTY).

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions

**Drift classification & sync summary (RECON-01)**

- **D-01 (drift signal):** Hash-only. Drift means `lockfile.content_hash != freshly-computed library content_hash`. The `version` string is display-only.
- **D-02 (summary format):** Single line, terse: `✓ 12 match · ⚠ 2 drift · ⚠ 1 vanished`. Drift / vanished items expand below.
- **D-03 (all-match output):** Print `✓ N plugins in sync` so the user sees positive evidence.
- **D-04 (bucket visibility):** Always render all three buckets, even when zero.
- **D-05 (drift detail):** `  • <skill>: <old_version> → <new_version>` per item; `unknown` placeholder if missing.
- **D-06 (vanished UX):** Per-skill stderr warning + summary count. Read from `InstalledPlugin.errors[]` (zero extra subprocess calls).

**Auto-install consent state machine (RECON-02)**

- **D-07 (persisted state):** `machine.toml::auto_install_plugins` is `Option<AutoInstall>` with serde `lowercase` rename. None = unset = first-time prompt.
- **D-08 (first prompt):** 3-way `[Y/n/never]` with default `Y`. Y → `Always`. n → `Ask`. never → `Never`.
- **D-09 (--no-install scope):** Single-run override only. Doesn't touch persisted setting. Mirrors Cargo's `--frozen` / `--locked`.
- **D-10 (trigger condition):** Prompt fires on Drift OR Missing-from-machine. Vanished does NOT trigger consent.
- **D-11 (Ask state behavior):** Re-prompt every sync with full `[Y/n/never]` choice; user can escape to `never` anytime.
- **D-12 (doctor reports drift unconditionally):** Independent of consent state.

**Edit-in-library prompt (RECON-05)**

- **D-13 (fork semantic — lossy in-place flip):** `managed: true → false`, `source_name: Some → None`. No provenance history fields. Distinct from `tome fork <skill> --to <dir>`.
- **D-14 (Unowned bypass):** Detection gate is `managed == true && source_name.is_some() && hash mismatch`. Unowned never prompts.
- **D-15 (prompt content):** `[F/r/s]` with default `fork`. Show source + version being severed (read from lockfile).
- **D-16 (--no-input exit code):** Skip-with-warning, exit zero.

**`install.rs` integration boundary**

- **D-17 (`install.rs` fate):** Delete entirely. ClaudeMarketplaceAdapter becomes the single canonical reconciliation path.
- **D-18 (sync flow position):** Replaces `reconcile_managed_plugins` at line 978 in `lib.rs::sync`.
- **D-19 (per-skill prompt removed):** Legacy `Install N plugin(s)? [Y/N]` goes away.
- **D-20 (claude binary missing):** Hard error with actionable message. Planner: `which::which("claude")` at adapter construction.
- **D-21 (GitAdapter scope):** Git stays separate. `resolve_git_directories` keeps its current pre-discovery role. Match/Drift/Vanished applies to claude-plugins entries only.
- **D-22 (lockfile timing on partial failure):** Per-skill in-memory update; one `lockfile::save` after loop.

### Claude's Discretion

- Exact prompt copy text (within bounds shown in D-08 and D-15).
- Internal organization of the new reconciliation function (one helper vs split).
- Whether classification logic lives in `marketplace.rs`, new `reconcile.rs`, or inline in `lib.rs::sync`. CONTEXT.md recommends new `reconcile.rs`.
- Whether `AutoInstall` enum lives in `machine.rs` (recommended) or sub-module.
- Mock test surface: keep `MockMarketplaceAdapter` as `#[cfg(test)]`-only, or lift to `pub(crate) marketplace::testing` for integration-test reuse.
- Exact error wording for "claude binary not found" (D-20).

### Deferred Ideas (OUT OF SCOPE)

- Provenance history fields on `SkillEntry` (`previous_source`, `previous_provenance`) → Phase 14.
- `tome status` / `tome doctor` Unowned-set surfacing → Phase 14 (UNOWN-03).
- GitAdapter participation in unified Match/Drift/Vanished classification → not v0.10.
- `--strict-edits` flag for non-zero exit on edit-in-library skip → not v0.10.
- Per-plugin version pinning → blocked on upstream `claude plugin install --version`.
- Migration of `installed_plugins.json`-based reconciliation logic → D-17 deletes it outright.
- RECON-01 wording cleanup in REQUIREMENTS.md → tactical traceability flag, not blocking.
- Auto-snapshot before reconcile-apply → existing pre-sync auto-snapshot is sufficient.
- Doctor reporting Vanished separately → out of Phase 13 surface.
- Telemetry / metric for sync reconciliation outcomes → never (project policy).
</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| RECON-01 | Match / Drift / Vanished classification of every managed lockfile entry; per-class summary every sync | Existing `manifest::hash_directory` (line 213 — verified 2026-05-06) is the drift basis (D-01). Adapter cache in `marketplace.rs::ClaudeMarketplaceAdapter` (line 600) gives `list_installed()` + `available()` cheaply. D-02 summary format is fully specified. |
| RECON-02 | First-time-on-machine consent prompt persisted in `machine.toml::auto_install_plugins`; `--no-install` override | `MachinePrefs` (machine.rs:54) and `save()` (machine.rs:161) already follow atomic temp+rename. Adding `Option<AutoInstall>` is a one-field change with `#[serde(skip_serializing_if = "Option::is_none")]`. dialoguer 0.12 `Select::default(idx).interact() -> Result<usize>` provides the 3-way primitive. |
| RECON-03 | Drift apply: render diff, invoke `adapter.install`/`update`, re-discover, verify resulting `content_hash` | Adapter calls return `Result<()>` and auto-invalidate the cache on Ok per Phase 12 D-04. `manifest::hash_directory` re-computes hash post-install. `update.rs::present_changes` (lines 73-171) is the canonical plan/render/execute pattern to mirror. |
| RECON-04 | Vanished plugins emit per-skill stderr warning + summary count; distribution still works from preserved library copy | `InstalledPlugin.errors[]` (marketplace.rs:64) carries the marketplace "not found" signal — Phase 12 D-02 confirms zero extra subprocess calls. Library content already preserved per LIB-04 (Phase 11) so cleanup is unchanged in Phase 13. |
| RECON-05 | Edit-in-library detection: managed + hash mismatch → `[F/r/s]` prompt (default fork); `--no-input` skips with zero exit | `Manifest.iter()` (manifest.rs) gives access to `SkillEntry.managed` + `source_name`. `hash_directory` produces the comparison hash. `Select::default(0)` covers the `[F/r/s]` UI shape. Lockfile entry has `version` for the prompt copy ("Last upstream: \<source\> @ \<version\>") — read at prompt time (D-15) before in-place flip clears the link. |
</phase_requirements>

## Standard Stack

### Core (already in tree, no new deps)

| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| `dialoguer` | 0.12 (workspace) | `Select` for 3-way prompts; `Confirm` for binary | Already used in 12 sites across `wizard.rs`, `lib.rs`, `doctor.rs`. `Select::default(idx).interact()` is the locked pattern. |
| `anyhow` | 1 | `Result<T>` + `.with_context()` | Project-wide convention. |
| `serde` + `toml` | workspace | `AutoInstall` enum serde rename, `MachinePrefs` round-trip | Existing `MachinePrefs` already uses `#[serde(default, skip_serializing_if = "...")]`. Pattern mirrors. |
| `assert_cmd` | 2.2 | Integration tests via real binary | Workspace dep. **`Command::write_stdin()` exists** (verified at `~/.cargo/registry/src/.../assert_cmd-2.2.1/src/cmd.rs:88`) but see Pitfall 6 — it does not produce a real TTY. |

### Supporting (no new deps)

| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| `console` | 0.16 (workspace) | Coloured stderr (`style("⚠").yellow()`, `style("✓").green()`) | Already used by `marketplace.rs::format_install_failures`. Mirror exactly for the summary line. |
| `which` | NOT YET in deps | Probe `claude` on PATH at adapter construction | **Decision:** the existing `is_claude_available()` (marketplace.rs:541) already shells out `claude --version`. Reuse it instead of adding `which` — Phase 12 already chose this pattern and `ClaudeMarketplaceAdapter::new()` already calls it. Do NOT add `which` as a dep just for D-20. |

### Alternatives Considered

| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| `dialoguer::Select` | Hand-rolled `print!` + `stdin.read_line` then match `Y/n/never` | `Select` matches existing wizard shape (12 call sites). Hand-rolled would need `[F/r/s]` letter-key branching, which `Select` does NOT do natively (it shows a list with arrow keys). **See Open Question 1.** |
| New `reconcile.rs` module | Inline in `lib.rs::sync` | Inline = one less file but balloons sync to ~1100 LOC. CONTEXT.md (line 166) recommends `reconcile.rs`. Mirrors `update.rs` pattern. |
| `which` crate for D-20 | Reuse `is_claude_available()` (marketplace.rs:541) | Already shipped in Phase 12. Adding `which` for one call is dep churn. |

**Installation:** none — every dependency is already in the workspace.

**Version verification:**
```bash
# Verified 2026-05-06 from /Users/martin/dev/opensource/tome/Cargo.toml + ~/.cargo/registry
# dialoguer = "0.12" (workspace dep at Cargo.toml:27)
# assert_cmd = "2" → 2.2.1 resolved (verified in registry index)
# console workspace dep verified
```
No new Cargo.toml edits required for Phase 13.

## Architecture Patterns

### Recommended Module Layout

```
crates/tome/src/
├── reconcile.rs         # NEW: classification + drift-apply + edit-prompt
├── marketplace.rs       # UNCHANGED: trait + adapters (Phase 12)
├── machine.rs           # +AutoInstall enum, +auto_install_plugins field
├── cli.rs               # +Sync.no_install: bool
├── lib.rs               # SyncOptions.no_install, sync() calls reconcile_lockfile
└── install.rs           # DELETED entirely
```

### Pattern 1: Plan/Render/Execute (mirrors `update.rs`)

**What:** Phase 13's reconcile splits into three pure-ish steps so each is independently testable.

**When to use:** Always — D-22 partial-failure semantics demand the steps be separable so the in-memory lockfile can be mutated only on Ok adapter calls.

**Example (proposed shape; planner refines):**
```rust
// crates/tome/src/reconcile.rs

#[derive(Debug, Clone)]
pub enum ReconcileClass {
    Match,
    Drift {
        old_version: Option<String>,
        new_version: Option<String>,
    },
    Vanished {
        old_version: Option<String>,
    },
    /// Lockfile entry exists but plugin is not in adapter.list_installed() AND
    /// adapter.available() returns true — first-machine bootstrap case (D-10).
    MissingFromMachine,
    /// managed + source_name.is_some() + content_hash mismatch (D-14).
    Edited {
        old_source: DirectoryName,
        old_version: Option<String>,
    },
}

#[derive(Debug, Default)]
pub struct ReconcileReport {
    pub matches: usize,
    pub drift: Vec<(SkillName, Drift)>,
    pub vanished: Vec<(SkillName, Vanished)>,
    pub edited: Vec<(SkillName, Edited)>,
    pub missing: Vec<(SkillName, MissingFromMachine)>,
    pub install_failures: Vec<InstallFailure>,
    /// Set true when --no-install or `auto_install_plugins == Never` blocked apply.
    pub apply_skipped: bool,
}

pub fn reconcile_lockfile(
    old_lockfile: Option<&Lockfile>,
    library_dir: &Path,
    adapter: &dyn MarketplaceAdapter,
    prefs: &mut MachinePrefs,
    machine_path: &Path,         // for AutoInstall persistence
    opts: ReconcileOpts,         // dry_run, no_input, no_install, quiet, verbose
) -> Result<ReconcileReport>
```

The function signature takes `Option<&Lockfile>` because no-prior-lockfile is a real first-run state (lockfile.rs:201 returns `None`). When the lockfile is `None`, all 5 classes are empty by definition (nothing to reconcile against) — return `ReconcileReport::default()` and let `sync()` print the "No previous lockfile — performing initial sync." message it already prints today (lib.rs:1074).

### Pattern 2: 3-key Keystroke Prompt (NEW pattern — see Open Question 1)

**What:** A prompt that accepts a single keystroke from a small set with a default on Enter.

**When to use:** D-08 `[Y/n/never]` and D-15 `[F/r/s]`.

**Why this is novel:** No existing tome prompt uses this shape. `dialoguer::Confirm` is 2-way; `dialoguer::Select` shows a navigated list. The ergonomic shape the user picked in CONTEXT.md (`[F/r/s]`, `[Y/n/never]`) implies single-keystroke + Enter, *not* arrow-navigated selection.

**Two viable implementations:**

**Option A — `dialoguer::Select` with 3 items** (proven pattern, used 12× already):
```rust
let options = ["Yes (always install)", "Yes (ask next time)", "No (never ask)"];
let idx = Select::new()
    .with_prompt("Tome detected N missing or out-of-date managed plugins. Install/update them now?")
    .items(&options)
    .default(0)
    .interact()?;
let consent = match idx {
    0 => AutoInstall::Always,
    1 => AutoInstall::Ask,
    2 => AutoInstall::Never,
    _ => unreachable!(),
};
```
*Pros:* zero new code, identical to the 12 wizard call sites. *Cons:* the user-facing UX shows arrow-key navigation, not the literal `[Y/n/never]` line CONTEXT.md prompt copy implies.

**Option B — Hand-rolled `print! + stdin.read_line` + match-first-char:**
```rust
print!("[Y/n/never] ");
io::stdout().flush()?;
let mut buf = String::new();
io::stdin().read_line(&mut buf)?;
let consent = match buf.trim().to_ascii_lowercase().as_str() {
    "" | "y" | "yes" => AutoInstall::Always,
    "n" | "no" => AutoInstall::Ask,
    "never" => AutoInstall::Never,
    other => { /* re-prompt or bail */ }
};
```
*Pros:* literal match for the prompt copy in D-08/D-15. Easy to test via `assert_cmd::write_stdin`. *Cons:* new pattern in this codebase; needs explicit re-prompt loop on invalid input; no nice arrow-key affordance.

**Recommendation for planner:** **Use Option A.** Reasons: (1) consistent with the 12 existing `Select` call sites — UX consistency wins; (2) the prompt copy in D-08/D-15 is "within these bounds" (CONTEXT.md "Claude's Discretion" bullet 1) — switching the visual to a Select list is within the planner's discretion; (3) Select is already proven to work under non-TTY by *failing fast* (returns `Err`) rather than hanging — giving us a clean signal to take the `--no-input`/non-TTY default. **Document this choice in the plan** so the diff to D-08's literal `[Y/n/never]` is explicit and traceable.

### Pattern 3: Per-skill Lockfile Update (D-22 partial-failure)

**What:** Mutate the in-memory `Lockfile` per successful adapter call; write to disk once at the end.

**When to use:** Always — partial failure must leave Ok entries advanced and Err entries at previous lockfile state.

**Example:**
```rust
// Build a working copy from the input lockfile (or empty)
let mut working_lockfile = old_lockfile.cloned().unwrap_or_else(|| Lockfile {
    version: 1,
    skills: BTreeMap::new(),
});

for (name, drift) in &report.drift {
    if dry_run { continue; }
    match adapter.update(&drift.registry_id) {
        Ok(()) => {
            // Re-discover by re-hashing the library entry post-update
            let new_hash = manifest::hash_directory(&library_dir.join(name.as_str()))?;
            // RECON-03: verify the freshly-recorded content_hash matches what the adapter produced
            // (note: spec says "re-discover" which here means re-hash the library copy AFTER
            //  consolidate runs in the next sync iteration. In Phase 13 we re-hash IN THIS LOOP
            //  because consolidate hasn't run yet — see Open Question 4.)
            if let Some(entry) = working_lockfile.skills.get_mut(name) {
                entry.content_hash = new_hash;
                entry.version = adapter.current_version(&drift.registry_id)?;
            }
        }
        Err(e) => {
            report.install_failures.push(InstallFailure { /* ... */ });
            // Leave working_lockfile entry untouched — D-22 invariant.
        }
    }
}

// Single atomic save after the loop.
if !dry_run && working_lockfile != *old_lockfile.unwrap_or(&Lockfile::default()) {
    lockfile::save(&working_lockfile, paths.config_dir())?;
}
```

**Note on the timing:** `lib.rs::sync` *already* generates a fresh lockfile post-consolidate (line 1054 — `pre_cleanup_lockfile = lockfile::generate(&manifest, &skills)`). Phase 13's `reconcile` runs *before* consolidate (D-18 says "same slot as reconcile_managed_plugins"), so the working lockfile in `reconcile_lockfile` is for the adapter-side update accounting only. Consolidate's downstream `lockfile::generate` will pick up the freshly-installed library content and produce the actual on-disk lockfile. **See Open Question 4 for the exact ordering question.**

### Anti-Patterns to Avoid

- **Anti-pattern: Re-using `install.rs::reconcile`.** D-17 says delete entirely. Don't try to refactor it — it has a totally different shape (file-based JSON parser, `[Y/N]` confirm) than Phase 13 wants.
- **Anti-pattern: Threading the new save through to the existing `machine::save` call at lib.rs:1062.** That save is for `update::present_changes` triage — it persists "skill X disabled this sync". The Phase 13 save is for "consent picked: Always". **Different events, different times. Save twice when both fire.** See Pitfall 5.
- **Anti-pattern: Calling `adapter.list_installed()` more than once per sync.** ClaudeMarketplaceAdapter caches per Phase 12 D-04, but the cache is internal to the adapter instance. If two reconcile sub-functions take `&dyn MarketplaceAdapter`, both can call `list_installed()` cheaply because the cache amortizes the second call — verified at marketplace.rs:732 (`self.cache.borrow().clone()`).
- **Anti-pattern: Bailing on the first install failure.** Phase 12 ADP-04 + Phase 8 SAFE-01 prescribe partial-succeed with grouped failure summary. Mirror `format_install_failures` (marketplace.rs:278) verbatim.

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| 3-way prompt | Hand-rolled char-matching loop with re-prompt | `dialoguer::Select::default(0).interact()` | 12 existing call sites; well-understood failure mode under non-TTY (returns Err, not hangs). |
| Atomic file write | Hand-rolled tempfile + rename | `lockfile::save` (lockfile.rs:214) and `machine::save` (machine.rs:161) | Already temp+rename with cleanup-on-failure (lockfile.rs:226, machine.rs:174). |
| SHA-256 directory hash | Hand-rolled walker + hasher | `manifest::hash_directory` (manifest.rs:213) | Deterministic, sorted-by-relpath, used by consolidate already. Re-export at lib.rs:75 for tests. |
| Subprocess wrapper around `claude` | New shell-out helper | `marketplace::ClaudeMarketplaceAdapter` (marketplace.rs:600) | Phase 12 ships it with cache + auto-invalidation. |
| Failure aggregation rendering | New formatter | `marketplace::format_install_failures` + `render_install_failures` (marketplace.rs:278, 317) | Phase 12 ships it (currently `dead_code`-allowed for Phase 13's first non-test caller). |
| TTY detection | Hand-rolled isatty | `std::io::IsTerminal` via `std::io::stdin().is_terminal()` | Already used in update.rs:78 and install.rs:130. |
| Vanished signal probing | Extra `claude plugin list` call | `InstalledPlugin.errors[]` substring `"not found in marketplace"` | Phase 12 D-02 says zero extra subprocess calls; signal is already in the cached snapshot (marketplace.rs:738-758). |

**Key insight:** Phase 12 deliberately shipped every primitive Phase 13 needs as `#[allow(dead_code)]` because Phase 13 is the first non-test caller. Plan tasks should systematically *remove* those `dead_code` attrs as wiring lands — they're a checklist of "what hasn't been wired up yet."

## Runtime State Inventory

This is a **code-restructuring + new-module** phase, not a rename or migration. The only on-disk state changes are:

| Category | Items Found | Action Required |
|----------|-------------|------------------|
| Stored data | `tome.lock` schema unchanged (LockEntry already has all needed fields per lockfile.rs:32 — `source_name`, `content_hash`, `registry_id`, `version`, `git_commit_sha`); `machine.toml` gains optional `auto_install_plugins` field (additive, backward-compat via `#[serde(default, skip_serializing_if = "Option::is_none")]`); `.tome-manifest.json` schema unchanged. | Code edit only — no migration. Old `machine.toml` files parse cleanly because field is `Option`. |
| Live service config | None — `installed_plugins.json` (Claude Code's own state file) was *read* by `install.rs` but is no longer touched. Claude Code continues writing it; tome stops reading it. | Document in CHANGELOG (Phase 16 DOC-02). |
| OS-registered state | None — no Task Scheduler / launchd / pm2 state. | None. |
| Secrets/env vars | None — no env var renames or new secret reads. | None. |
| Build artifacts | `crates/tome/src/install.rs` deletion — Cargo will rebuild without it cleanly. | None — `cargo clean` not required; `mod install` removed from lib.rs:36 will fail compile until install.rs is gone, which is the correct invariant. |

## Common Pitfalls

### Pitfall 1: `claude plugin update` qualifier-vs-bare-id ambiguity (Phase 12 carry-over)
**What goes wrong:** `claude plugin update axiom` (bare id) exits 1 with `Plugin "axiom" not found`. Calling with the qualified form (`claude plugin update axiom@axiom-marketplace`) was not empirically verified in Phase 12 (CONTEXT.md line 234: "planner should verify whether `update` needs the `@marketplace` qualifier").
**Why it happens:** Claude Code's `update` subcommand requires unambiguous resolution; multi-marketplace setups would otherwise be ambiguous. The `install` subcommand always takes qualified IDs.
**How to avoid:** **Always pass `LockEntry.registry_id` verbatim to `adapter.update()`.** The `registry_id` field already records the qualified form per Phase 12 D-08 (`marketplace.rs:40-42` documents `"axiom@axiom-marketplace"` shape). The planner does NOT need to extract or re-qualify — verbatim pass-through is correct.
**Warning signs:** Test failure with stderr `Plugin "X" not found`. **Verify in a Phase 13 task:** add a smoke test that exercises `adapter.update("nonexistent@nonexistent-marketplace")` and asserts the error path *does* match the qualified form (mirroring `smoke_claude_install_nonexistent_returns_err` at marketplace.rs:1529). If the bare-id form is needed, document it as a Phase 13 finding so Phase 12 docs can be updated.

### Pitfall 2: `auto_install_plugins == Ask` under `--no-input`
**What goes wrong:** User picked `n` (Ask) on machine A. Machine A then runs `tome sync --no-input` for CI. The sync hits drift; the consent state says "ask"; there's no TTY to ask.
**Why it happens:** Ask is a deferred decision — the consent says "I haven't decided yet, prompt me again." Non-interactive contexts can't resolve this.
**How to avoid:** **Treat `Ask`/`None` consent under `--no-input` or non-TTY as `Skip` (warn + don't apply).** Specifics line in CONTEXT.md confirms: "For `--no-input` + drift detected + `auto_install_plugins = ask`: treat `ask` as `never` for this run (can't prompt non-interactively). Surface drift as warning. Don't persist any state change. Same shape for `auto_install_plugins = unset`."
**Warning signs:** Sync hangs in CI (means dialoguer was reached without TTY) or persists state from a non-interactive run (means the decision was committed without consent).

### Pitfall 3: Vanished + Edited intersection
**What goes wrong:** A managed skill (a) is no longer obtainable from the marketplace (vanished per `errors[]`) AND (b) the user has edited it locally (hash mismatch).
**Why it happens:** Both states can coexist — vanished means upstream is gone; edited means local is changed.
**How to avoid:** **Edited takes precedence over Vanished.** If the user has edited, prompt `[F/r/s]` per RECON-05. The `revert` option is degraded (no upstream to revert to) — the prompt should detect this and either (a) drop `revert` from the choices for vanished entries or (b) emit `revert` choice but on selection bail with "cannot revert: plugin vanished from marketplace; pick fork or skip." **Decision for planner:** drop the `revert` option entirely when the entry is also vanished — keeps the prompt small.
**Warning signs:** `revert` chosen + adapter.install errors with NotFound. Make this an enforced precondition, not a runtime failure.

### Pitfall 4: `LockEntry.registry_id == None` interleaved with managed `source_name`
**What goes wrong:** A skill is in the lockfile with `source_name = Some("claude-plugins")` but `registry_id = None`. This shouldn't happen via consolidate (it always populates registry_id for managed skills via `lockfile::generate` at lockfile.rs:66-76 reading `SkillProvenance`), but lockfiles can be hand-edited or come from `--force` operations.
**Why it happens:** Edge case; possible after `tome reassign` or `tome fork` to a managed-shaped directory.
**How to avoid:** **Skip such entries from Match/Drift/Vanished classification** — they have no marketplace identity, so the adapter can't reason about them. Emit a verbose-mode info: `info: skill <name> claims source 'claude-plugins' but has no registry_id; skipping reconciliation`. **Confirmed in Specifics line:** "Lockfile entries that have `registry_id: None`: these are local skills that ended up in the lockfile via `lockfile::generate`. They have no marketplace identity. Phase 13's reconciliation skips them."
**Warning signs:** A test in `cli_sync.rs` that constructs a hand-rolled lockfile with `source_name=Some, registry_id=None` and expects classification to skip it.

### Pitfall 5: Save-chain ordering — `auto_install_plugins` persistence vs `update::present_changes` save
**What goes wrong:** `lib.rs::sync` already calls `machine::save` at line 1062 *after* `update::present_changes` flips `disabled` for newly-disabled-via-triage skills. If Phase 13's `reconcile_lockfile` ALSO saves `machine.toml` to persist the `Always`/`Never` consent, two saves can race or duplicate.
**Why it happens:** The two saves serve different purposes at different sync stages: reconcile is *before* discovery (line 978 today); triage is *after* consolidate (line 1060). The two events are temporally separated.
**How to avoid:** **Save twice, in-order, atomically each time.** Each save uses temp+rename so a partial write isn't possible; the second save overwrites the first cleanly. Reconcile saves consent immediately after the prompt (so a Ctrl-C between consent and sync-completion still persists the user's "Always" choice — the user did decide). Triage saves disabled-skills at its current site. **Both saves go to the same `machine_path` from `SyncOptions`.**
**Warning signs:** A test that sets consent to `Always`, kills sync mid-run before discovery, and verifies on next run that the `[Y/n/never]` prompt does NOT re-fire.

### Pitfall 6: `assert_cmd` + `dialoguer::Select` is non-TTY by default
**What goes wrong:** Integration test with `Command::cargo_bin("tome").write_stdin("0\n")` does not actually feed input to a Select prompt — `dialoguer::Select::interact()` checks for a terminal and errors out (`Err`) when it doesn't see one, *before* reading stdin.
**Why it happens:** dialoguer 0.12 uses raw-mode terminal interaction. `assert_cmd` runs the binary as a subprocess with piped stdin — no PTY.
**How to avoid:** **Don't try to drive `Select` prompts from `assert_cmd` integration tests.** Two viable strategies for Phase 13's tests:
1. **Unit-test the consent state machine in isolation** (in `reconcile.rs::tests` or `machine.rs::tests`) — construct `MachinePrefs` directly, exercise the decision logic without invoking dialoguer.
2. **Drive integration tests through the `--no-input` path only** — exercise that the non-interactive defaults take the right branches (apply silently when consent=Always, skip-with-warning when Ask/Unset/Never).
3. **For genuine end-to-end TTY tests:** use a PTY library like `expectrl` or `rexpect` — but neither is in the workspace; adding them is dep churn for a marginal test gain. **Recommendation: skip TTY tests entirely, rely on unit tests for the prompt logic.**
**Warning signs:** Integration test hangs or `Err: not a terminal`-style stderr.

### Pitfall 7: `claude` CLI behavior change between machines
**What goes wrong:** Machine A has Claude Code 2.1.128 (the version Phase 12 was empirically verified against, marketplace.rs:457). Machine B has 2.2.x with different stderr wording or JSON shape.
**Why it happens:** Claude Code is a third-party tool; tome doesn't pin its version.
**How to avoid:** **Tolerate unknown stderr by falling through to `InstallFailureKind::Unknown`** (already done — marketplace.rs:524). **Tolerate unknown JSON fields** (already done — `parse_claude_plugin_list_json` at marketplace.rs:478 deliberately omits `#[serde(deny_unknown_fields)]`). **Surface the verbatim stderr in the grouped failure summary** so the user can see what actually happened.
**Warning signs:** New `unknown` failures dominating the grouped summary on a fresh Claude Code install. Document in CHANGELOG that Phase 13 is verified against claude 2.1.128 specifically.

## Code Examples

### Adding `AutoInstall` enum to `machine.rs`
```rust
// crates/tome/src/machine.rs (additions)

/// 3-state auto-install consent persisted in machine.toml (RECON-02 D-07).
///
/// `None` (field absent / unset) means "first-time prompt" — distinguished
/// from `Some(Ask)` which means "user picked 'n' last time, ask again."
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AutoInstall {
    Always,
    Ask,
    Never,
}

// In MachinePrefs, add:
#[serde(default, skip_serializing_if = "Option::is_none")]
pub(crate) auto_install_plugins: Option<AutoInstall>,
```
*Source pattern:* mirrors `DirectoryOverride` schema (machine.rs:27) with `#[serde(deny_unknown_fields)]` already used for forward-compat.

### Adding `--no-install` to `Sync` command
```rust
// crates/tome/src/cli.rs Command::Sync (around line 90)
Sync {
    #[arg(short, long)]
    force: bool,
    #[arg(long)]
    no_triage: bool,
    /// Skip auto-install/update of missing or drifted plugins this run.
    /// Doesn't change the persisted `auto_install_plugins` setting in
    /// `machine.toml`. Mirrors Cargo's `--frozen` / `--locked`.
    #[arg(long)]
    no_install: bool,
},
```

### Wiring `no_install` through `SyncOptions`
```rust
// crates/tome/src/lib.rs (around line 760, struct definition)
struct SyncOptions<'a> {
    dry_run: bool,
    force: bool,
    no_triage: bool,
    no_input: bool,
    no_install: bool,  // NEW
    verbose: bool,
    quiet: bool,
    machine_path: &'a Path,
    machine_prefs: &'a machine::MachinePrefs,
}

// crates/tome/src/lib.rs Command::Sync dispatch (around line 352)
Command::Sync { force, no_triage, no_install } => sync(
    &config,
    &paths,
    SyncOptions {
        dry_run: cli.dry_run,
        force,
        no_triage: no_triage || cli.no_input,
        no_input: cli.no_input,
        no_install,                           // NEW
        verbose: cli.verbose,
        quiet: cli.quiet,
        machine_path: &machine_path,
        machine_prefs: &machine_prefs,
    },
)?,
```

### Replacing `reconcile_managed_plugins` call site
```rust
// crates/tome/src/lib.rs::sync (around line 977-979 today)
//
// Today (DELETE):
//     if !dry_run {
//         reconcile_managed_plugins(&old_lockfile, config, quiet, no_input)?;
//     }
//
// Phase 13 (REPLACE WITH):
{
    let mut adapters = build_adapter_map(config)?;        // see below
    if let Some(claude_adapter) = adapters.get("claude-plugins") {
        let mut machine_prefs_for_recon = machine_prefs.clone();
        let report = reconcile::reconcile_lockfile(
            old_lockfile.as_ref(),
            paths.library_dir(),
            claude_adapter.as_ref(),
            &mut machine_prefs_for_recon,
            machine_path,
            reconcile::ReconcileOpts {
                dry_run,
                no_input,
                no_install,
                quiet,
                verbose,
            },
        )?;
        report.render_summary(quiet);  // D-02..D-06
        if !report.install_failures.is_empty() {
            marketplace::render_install_failures(&report.install_failures);
            // D-22 + ADP-04: lockfile only updates Ok entries; failures don't
            // bail the sync. Distribution from the preserved library copy
            // continues. Exit code is non-zero per ADP-04.
        }
        // Update machine_prefs in place if reconcile mutated it (consent change)
        if machine_prefs_for_recon != *machine_prefs {
            machine::save(&machine_prefs_for_recon, machine_path)?;
            // Note: the existing post-triage machine::save at line 1062 saves
            // `machine_prefs` (the outer binding); reconcile's mutation must
            // be merged back. See Pitfall 5.
        }
    }
}
```
*Caveat:* the exact `machine_prefs` ownership pattern needs planner refinement — the current `sync()` already clones `machine_prefs` once at line 970. Extending the same lifetime through reconcile is straightforward; the snippet above is illustrative.

### Adapter-map dispatcher (Phase 12 D-11 — new helper, planner shape)
```rust
// crates/tome/src/reconcile.rs OR inline in lib.rs

fn build_adapter_map(config: &Config) -> Result<BTreeMap<String, Box<dyn MarketplaceAdapter>>> {
    let mut map: BTreeMap<String, Box<dyn MarketplaceAdapter>> = BTreeMap::new();
    let needs_claude = config
        .directories()
        .values()
        .any(|d| d.directory_type == DirectoryType::ClaudePlugins);
    if needs_claude {
        // D-20: hard error if claude binary missing.
        let adapter = marketplace::ClaudeMarketplaceAdapter::new()
            .context("claude CLI required for [directories.<name>] entries with type = \"claude-plugins\"")?;
        map.insert("claude-plugins".to_string(), Box::new(adapter));
    }
    // GitAdapter NOT inserted per D-21 — git stays in resolve_git_directories.
    Ok(map)
}
```

### Edit-in-library detection (RECON-05)
```rust
// crates/tome/src/reconcile.rs

fn detect_edited(
    manifest: &Manifest,
    library_dir: &Path,
    old_lockfile: &Lockfile,
) -> Result<Vec<(SkillName, EditedDetail)>> {
    let mut edited = Vec::new();
    for (name, entry) in manifest.iter() {
        // D-14: gate is managed=true AND source_name=Some AND hash mismatch.
        if !entry.managed { continue; }
        let Some(ref source_name) = entry.source_name else { continue; };

        let lock_entry = match old_lockfile.skills.get(name) {
            Some(e) => e,
            None => continue,  // not in lockfile yet; not "edited" against anything
        };

        let live_hash = manifest::hash_directory(&library_dir.join(name.as_str()))?;
        if live_hash != lock_entry.content_hash {
            edited.push((name.clone(), EditedDetail {
                old_source: source_name.clone(),
                old_version: lock_entry.version.clone(),
            }));
        }
    }
    Ok(edited)
}
```

### Fork-in-place flip (D-13 lossy)
```rust
fn apply_fork_in_place(
    manifest: &mut Manifest,
    name: &SkillName,
) -> Result<()> {
    // D-13: managed: true → false; source_name: Some → None.
    // Provenance history dropped — accepted UX gap.
    let entry = manifest.get_mut(name)
        .with_context(|| format!("manifest entry missing for {name}"))?;
    entry.managed = false;
    entry.source_name = None;
    Ok(())
}
```
*Caveat:* `Manifest` API may not expose `get_mut` today — verify `crate::manifest` mutation surface and add a method if needed (HARD-06 will eventually tighten manifest visibility but Phase 13 lands before HARD-06).

## Lifting `MockMarketplaceAdapter` (Open Question 5 — answered)

**Recommendation:** **Lift to `pub(crate) marketplace::testing` module.**

**Rationale:**
- Phase 13 needs integration tests in `tests/cli.rs` (or `tests/cli_sync.rs` per HARD-13) that drive `tome sync` end-to-end with a synthetic adapter — the real `ClaudeMarketplaceAdapter` requires `claude` on PATH and shells out to a real CLI.
- Today `MockMarketplaceAdapter` is `#[cfg(test)] pub(super)` in marketplace.rs:777. Integration tests in `tests/cli.rs` cannot see it (different crate).
- **Cleanest lift:** wrap in `pub(crate) mod testing { ... }` *gated by `#[cfg(any(test, feature = "test-support"))]`* — but that adds a feature flag.
- **Simpler lift:** put the mock in `pub(crate) mod testing` *without a feature flag* — exposed in production builds but only constructed by `#[cfg(test)]` callers. The mock has zero runtime cost when not constructed (no static state).
- **Even simpler:** keep the mock in `marketplace.rs` and add a `pub(crate)` constructor + a `pub(crate) trait` re-export, but this leaks test surface into production module docs.

**Decision for planner:** the simplest path that's not gross is **add a new `crates/tome/src/marketplace_testing.rs` file gated by `#[cfg(any(test, debug_assertions))]`** — but actually, the cleanest path is just to **move the integration tests that need a mock into the same crate as a `#[cfg(test)]` integration test** by making `tests/cli.rs` a `#[path]`-included module — but tome's tests are already external.

**Final recommendation:** **add a `pub(crate) mod testing` inside `marketplace.rs`, no cfg gate.** Justification: `MockMarketplaceAdapter` is a small (~50 LOC) struct with zero runtime cost when not constructed; gating it adds build complexity with no real benefit; integration tests in `tests/cli.rs` can construct it via `tome::marketplace::testing::MockMarketplaceAdapter` if `marketplace` is made `pub` (it's currently `pub(crate)` at lib.rs:42). **Alternative if `marketplace` must stay `pub(crate)`:** re-export the mock through a new `pub(crate) mod test_support` at the crate root, gated by `#[cfg(test)]` — `tests/cli.rs` is technically a separate crate and can't see `pub(crate)` items, so this requires `pub` exposure regardless. **Per HARD-06 future direction (tighten visibility), the planner should pick the option that minimizes future churn — likely a `#[cfg(feature = "test-support")]` feature flag or accept the mock surface as part of the public API for v0.10.**

**Honest gap:** I could not produce a one-line "this is the right way" answer here. The choice depends on whether v0.10 wants the test-support surface to be public API or feature-gated. **Planner must decide between three options and document the choice in PLAN-13-XX:**
1. Make `marketplace` module `pub` and add `pub mod testing` inside it (simplest; test-support is public API).
2. Add `[features] test-support = []` and gate the mock module on it (cleanest separation; small Cargo.toml churn).
3. Keep `MockMarketplaceAdapter` at `#[cfg(test)]` in marketplace.rs and write Phase 13 integration tests that don't use a mock (test against a real `claude` binary — flaky in CI without `claude` installed).

Recommendation: **option 2 (feature-gated)**. It's the v1.0-friendly path.

## Save-Chain Ordering (Open Question 3 — answered)

The existing `lib.rs::sync` save chain (verified at lib.rs:1062 and elsewhere):

| Stage | Save call | Triggered by | What's persisted |
|-------|-----------|--------------|------------------|
| Today: post-triage | `machine::save(&machine_prefs, machine_path)` (lib.rs:1062) | `update::present_changes` returned non-empty `newly_disabled` list | Updates to `MachinePrefs.disabled` |

Phase 13 adds:

| Stage | Save call | Triggered by | What's persisted |
|-------|-----------|--------------|------------------|
| **NEW: post-consent** | `machine::save(&machine_prefs, machine_path)` inside `reconcile_lockfile` after the `[Y/n/never]` prompt | User picked `Always`/`Ask`/`Never` | Updates to `MachinePrefs.auto_install_plugins` |
| **NEW: post-edit-fork** | `machine::save(...)` if a fork-in-place was applied | User picked `fork` in `[F/r/s]` prompt | (None — fork mutates *manifest*, not machine prefs. **No machine save needed for fork.**) |

Manifest save chain (existing, unchanged by Phase 13):
| Stage | Save call | What's persisted |
|-------|-----------|------------------|
| Post-cleanup | `manifest::save(&manifest, paths.config_dir())` (lib.rs around line 1100+ — verify exact line) | All manifest mutations including the fork-in-place flip |

Lockfile save chain:
| Stage | Save call | What's persisted |
|-------|-----------|------------------|
| Post-distribute | `lockfile::save(&lockfile, paths.config_dir())` (existing, near end of `sync`) | Generated from final manifest + skills |

**Phase 13 ordering (D-22):** Per-skill in-memory updates to a *working* `Lockfile` happen inside `reconcile_lockfile`, but the actual on-disk `lockfile::save` is the *existing* post-distribute one — Phase 13 doesn't write the lockfile inside reconcile, because consolidate hasn't run yet, so the manifest doesn't reflect the new content yet. **The "lockfile written once at end of loop" in D-22 means the working copy is mutated once-per-Ok-call; the disk write is the existing end-of-sync write.**

**Wait — clarification needed.** D-22 says "After the loop, write the lockfile to disk once (atomic temp+rename per existing `lockfile::save` pattern)." This *could* mean either (a) the working copy is written immediately after the reconcile loop (a new save call), or (b) the working copy is merged into the post-distribute lockfile generation. **See Open Question 4** — this is the single most important ordering question for the planner.

## State of the Art

| Old Approach (v0.9-shape) | Current Approach (v0.10 Phase 13) | When Changed | Impact |
|--------------------------|------------------------------------|--------------|--------|
| `installed_plugins.json` parsing in `install.rs::parse_installed_registry_ids` | `claude plugin list --json` via `ClaudeMarketplaceAdapter::list_installed` | Phase 13 | tome no longer reads Claude Code's internal state file. Forward-compat with future Claude versions. |
| Per-sync `Install N plugin(s)? [Y/N]` confirmation | One-time `[Y/n/never]` consent persisted in `machine.toml` | Phase 13 (D-19) | Library-as-dotfiles bootstrap is one-keystroke. Re-prompt only on consent change. |
| Drift = "version differs or older" (per RECON-01 wording) | Drift = `content_hash` mismatch | Phase 11 D-08 + Phase 13 D-01 | Eliminates false-positive drift on plugins where Claude bumps version metadata without changing content. |
| Bail-on-first-failure | Partial-succeed + grouped summary (SAFE-01 / ADP-04) | Phase 8 + Phase 12 | Network blip on plugin 9 of 10 doesn't block the sync. |

**Deprecated/outdated (do not propagate):**
- `install.rs` (entire file) — D-17 deletion target.
- `installed_plugins.json` v1 format parser (install.rs:225-227) — already inert; v2 is the format.
- The `Install N plugin(s)? [Y/N]` `Confirm` at install.rs:131 — replaced by `[Y/n/never]` Select.

## Open Questions

1. **3-key prompt UX: literal `[Y/n/never]` or `Select` 3-item list?**
   - What we know: dialoguer 0.12 `Select` is proven (12 sites); `Confirm` is 2-way only.
   - What's unclear: whether the planner / user wants the prompt copy in D-08/D-15 to be literal (hand-rolled) or visualized as Select.
   - Recommendation: **Use `Select`** for consistency. Document the choice in PLAN. Honors "Claude's Discretion" bullet 1.

2. **`MockMarketplaceAdapter` exposure: `pub(crate)` lift, feature flag, or no integration mock?**
   - What we know: today the mock is `#[cfg(test)] pub(super)` in marketplace.rs:777. `tests/cli.rs` is a separate crate and cannot see `pub(crate)` items.
   - What's unclear: whether v0.10 wants `marketplace::testing` as public API or feature-gated.
   - Recommendation: **Feature flag `test-support`** gating a new `pub mod testing` inside `marketplace.rs`. Phase 13 integration tests enable the feature in `[dev-dependencies]`.

3. **`claude plugin update` exact ID format — qualified or bare?**
   - What we know: Phase 12 D-08 records qualified IDs (`axiom@axiom-marketplace`). `install` accepts qualified. `update axiom` (bare) fails per Phase 12 RESEARCH probe 1b.
   - What's unclear: whether `update axiom@axiom-marketplace` (qualified) succeeds.
   - Recommendation: **Pass `LockEntry.registry_id` verbatim and add a smoke test** that exercises a known-installed plugin's update path against the qualified form. If it fails, document and treat as Phase 13 finding.

4. **D-22 "write the lockfile to disk once" — new save inside reconcile, or merge into existing post-distribute save?**
   - What we know: today `lib.rs::sync` writes the lockfile *once* at the very end via `lockfile::save` after consolidate + distribute. That save is generated from `manifest` + `skills` via `lockfile::generate`. Reconcile runs *before* discovery (D-18).
   - What's unclear: whether reconcile's working-copy lockfile mutations should (a) be written to disk immediately by reconcile (causing two saves total per sync), or (b) be threaded to influence the *generated* lockfile at the existing save site.
   - Recommendation: **Option (a) — reconcile saves once at end of its loop.** Reasons: (1) D-22's literal text "lockfile only updates entries that succeeded; failed entries stay at previous lockfile state" implies the disk lockfile is the durable "what succeeded" record, written at reconcile-time so a crash mid-sync preserves the partial-success state. (2) The post-distribute generated lockfile will naturally regenerate the same content_hashes from the freshly-installed library + manifest, so the two writes converge. (3) Atomic temp+rename means double-write is safe. **Planner should explicitly call out this decision in the plan and verify it doesn't conflict with consolidate's post-discovery lockfile generation.**

5. **`Lockfile.skills` mutation API — does `LockEntry` need a builder or is `skills.get_mut().content_hash = ...` ok?**
   - What we know: `LockEntry` has all fields `pub` (lockfile.rs:32-53). HARD-06 plans to tighten this to `pub(crate)` with accessors but is post-Phase-13.
   - What's unclear: whether reconcile should mutate fields directly or go through a constructor.
   - Recommendation: **Direct field mutation is fine for Phase 13** — HARD-06 will refactor uniformly later.

6. **Sync exit code semantics on partial install failure**
   - What we know: ADP-04 says "exits non-zero on partial install failure but library distribution still completes." `tome sync` today exits 0 on success.
   - What's unclear: where the exit code transition happens — does `reconcile_lockfile` return an error, or does `sync` track a flag?
   - Recommendation: **`reconcile_lockfile` returns `Ok(ReconcileReport)` regardless** (because we want consolidate/distribute to still run from the library copy per RECON-04). Then `sync` (or `run`) checks `report.install_failures.is_empty()` and either bails with `anyhow::bail!` at the end of sync, or returns a sentinel error type that `main.rs` maps to exit 1. **The cleanest fit with HARD-04** (which adds a downcastable `LintFailed` error) is to define `pub(crate) struct ReconcileFailed;` and downcast in `main.rs` similarly. Phase 13 may not want that infra yet — alternative: at the end of `sync`, `if !install_failures.is_empty() { anyhow::bail!("{N} plugin install failures (see above)") }` — exit 1 via existing main.rs error path.

## Environment Availability

| Dependency | Required By | Available | Version | Fallback |
|------------|------------|-----------|---------|----------|
| Rust toolchain (stable, edition 2024) | Compile | ✓ | 1.85.0+ (per CLAUDE.md) | — |
| `cargo` | Build / test | ✓ | bundled with rustup | — |
| `claude` CLI on PATH | Smoke tests of `ClaudeMarketplaceAdapter` (existing tests in marketplace.rs:1501-1537 already gate on `is_claude_available`) | ✓ on Martin's machines | claude 2.1.128 (Phase 12 verification target) | Smoke tests print `SKIP` and return; no fallback for production use — D-20 is a hard error if user has `[directories.claude-plugins]` configured without claude installed. |
| `git` CLI | `GitAdapter` (Phase 12) — but Phase 13 doesn't call `GitAdapter` (D-21) | ✓ | system git | — |
| `dialoguer` runtime PTY | Interactive prompts | ✓ on user-facing machines | 0.12 | Phase 13 honors `--no-input` and TTY detection per Pitfall 6. |

**Missing dependencies with no fallback:** None blocking Phase 13's code-only work.

**Missing dependencies with fallback:** `claude` on CI is the only one — handled by existing smoke-test gating pattern at marketplace.rs:1501.

## Validation Architecture

> Skipped: `workflow.nyquist_validation` is `false` in `.planning/config.json` (verified 2026-05-06).

## Sources

### Primary (HIGH confidence — read directly from the working tree on 2026-05-06)

- `/Users/martin/dev/opensource/tome/.planning/phases/13-lockfile-authoritative-sync/13-CONTEXT.md` — full 22 D-XX decision set.
- `/Users/martin/dev/opensource/tome/.planning/REQUIREMENTS.md` lines 32-39 — RECON-01..05 verbatim.
- `/Users/martin/dev/opensource/tome/.planning/research/v0.10-library-canonical-design.md` lines 165-220 — OQ-3 / OQ-4 / OQ-6 resolution rationale.
- `/Users/martin/dev/opensource/tome/.planning/phases/12-marketplace-adapter/12-CONTEXT.md` line 234 — explicit handoff: "planner should verify whether `update` needs the `@marketplace` qualifier."
- `/Users/martin/dev/opensource/tome/crates/tome/src/marketplace.rs` (1538 lines) — full Phase 12 adapter surface with `#[allow(dead_code)]` enumerating Phase 13's wire-up checklist.
- `/Users/martin/dev/opensource/tome/crates/tome/src/install.rs` (313 lines) — full delete target.
- `/Users/martin/dev/opensource/tome/crates/tome/src/lib.rs` lines 760-779 (SyncOptions), 912-1100 (sync function head), 1616-1640 (reconcile_managed_plugins).
- `/Users/martin/dev/opensource/tome/crates/tome/src/lockfile.rs` lines 22-94 (Lockfile + LockEntry + generate), 213-231 (save).
- `/Users/martin/dev/opensource/tome/crates/tome/src/manifest.rs` lines 209-255 (hash_directory).
- `/Users/martin/dev/opensource/tome/crates/tome/src/machine.rs` (598 lines) — full schema + atomic save pattern.
- `/Users/martin/dev/opensource/tome/crates/tome/src/cli.rs` lines 86-97 (Sync command shape).
- `/Users/martin/dev/opensource/tome/crates/tome/src/update.rs` (346 lines) — full plan/render/execute pattern Phase 13 mirrors.
- `/Users/martin/dev/opensource/tome/crates/tome/src/wizard.rs` lines 170-360 — 12 `Select::new()` call sites verifying the locked pattern.
- `/Users/martin/dev/opensource/tome/Cargo.toml` (workspace) — dialoguer 0.12, assert_cmd 2.
- `~/.cargo/registry/src/.../dialoguer-0.12.0/src/prompts/select.rs` lines 68 (default), 122 (with_prompt), 143 (interact), 174 (interact_opt).
- `~/.cargo/registry/src/.../assert_cmd-2.2.1/src/cmd.rs` line 88 — `pub fn write_stdin<S>(&mut self, buffer: S) -> &mut Self`.

### Secondary (MEDIUM confidence — inference from cited primaries)

- `claude plugin update` qualified-vs-bare ID — Phase 12 RESEARCH probe 1b documented bare-id failure; qualified path *not* empirically verified. Recommendation includes a Phase 13 smoke test to close.
- `auto_install_plugins == Ask` re-prompt under `--no-input` — inferred from Specifics line in CONTEXT.md (treated as `Skip`); not verified end-to-end.

### Tertiary (LOW confidence)

- None. All claims trace to either the working tree or locked Phase 12 contract.

## Metadata

**Confidence breakdown:**
- Standard stack: **HIGH** — no new deps; every primitive verified at line numbers in the working tree.
- Architecture: **HIGH** — D-18 + Phase 12 D-11 fully specify the dispatch shape; the only design freedom is module organization within `reconcile.rs`.
- Pitfalls: **HIGH** — pitfalls 1, 2, 4, 7 are documented in CONTEXT.md/Phase 12 docs verbatim; pitfalls 3, 5, 6 are derived from code inspection; pitfall 6 is verified against assert_cmd source.
- Edge cases / open questions: **MEDIUM** — 6 questions surfaced; recommendations given but not empirically verified (especially #3 and #4).

**Research date:** 2026-05-06
**Valid until:** ~2026-06-05 (30 days — phase is internal-refactor work; only external dependency is `claude` CLI behavior which Phase 12 already handles forward-compat).
