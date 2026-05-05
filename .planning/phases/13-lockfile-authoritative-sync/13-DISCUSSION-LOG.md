# Phase 13: Lockfile-authoritative sync - Discussion Log

> **Audit trail only.** Do not use as input to planning, research, or execution agents.
> Decisions are captured in CONTEXT.md — this log preserves the alternatives considered.

**Date:** 2026-05-05
**Phase:** 13-lockfile-authoritative-sync
**Areas discussed:** Drift classification & summary, Auto-install consent, Edit-in-library prompt, install.rs integration

---

## Drift Classification & Summary Output (RECON-01)

### Q1: What counts as 'Drift' in the Match/Drift/Vanished classification?

| Option | Description | Selected |
|--------|-------------|----------|
| Hash only | Drift = lockfile.content_hash differs from freshly-computed library content_hash. Version is display-only. Honors Phase 11 D-08 strictly. Avoids "stale version, same content" false positives. | ✓ |
| Hash OR version | Drift if either the hash differs OR the version string differs from lockfile. Matches RECON-01's literal wording. Surfaces version metadata bumps as drift even when content is identical. | |
| Hash + version sub-class | Three buckets: Match / ContentDrift / VersionDrift / Vanished. More information but more UX surface. | |

**User's choice:** Hash only (recommended)
**Notes:** Asked for clarification on "iff" jargon used in option description; rephrased to plain English ("Drift = ...") in the re-pose. Saved memory entry to avoid logic-shorthand in user-facing text.

---

### Q2: How should the per-class summary render after each sync?

| Option | Description | Selected |
|--------|-------------|----------|
| Single line | `✓ 12 match · ⚠ 2 drift · ⚠ 1 vanished` — one terse line, drift/vanished items expanded below if non-zero. | ✓ |
| Multi-line per-bucket header | Each non-zero bucket as its own block with header + per-skill bullets. More visual real-estate per sync. | |
| Tabled output (matching tome status) | Use `tabled` crate for a small status table per managed directory. Heavier per-sync. | |

**User's choice:** Single line (recommended)

---

### Q3: When every managed plugin is Match, what does the summary do?

| Option | Description | Selected |
|--------|-------------|----------|
| One-line confirmation | Print `✓ N plugins in sync` so the user sees positive confirmation that reconciliation ran. | ✓ |
| Stay silent | Print nothing in the all-match case (Cargo-style minimalism). | |
| Suppress unless --verbose | Default silent; with -v emit the confirmation line. | |

**User's choice:** One-line confirmation (recommended)

---

### Q4: For the single-line summary, what's the order of buckets when zero items exist?

| Option | Description | Selected |
|--------|-------------|----------|
| Always show all 3 | `✓ 12 match · ⚠ 0 drift · ⚠ 0 vanished` even when zero. Predictable; greppable across runs. | ✓ |
| Hide zero-count classes | All-match collapses; otherwise only non-zero buckets render. Reduces visual clutter. | |
| Always show match, hide zero drift/vanished | Middle ground. | |

**User's choice:** Always show all 3 (recommended)

---

### Q5: When drift items exist, how detailed should the per-skill expansion be?

| Option | Description | Selected |
|--------|-------------|----------|
| Per-skill diff line | `  • superpowers: 5.0.5 → 5.0.7` — names plus version annotation. | ✓ |
| Per-skill name only | `  • superpowers` per drift skill; no version. Tersest possible. | |
| Per-skill diff + content_hash short | Includes truncated hash for debug-friendliness. | |

**User's choice:** Per-skill diff line (recommended)

---

### Q6: How should vanished plugins surface?

| Option | Description | Selected |
|--------|-------------|----------|
| Per-skill stderr warning + summary count | `warning: plugin X vanished from marketplace Y; using preserved library copy` per item; summary line shows count. Mirrors RECON-04 wording. | ✓ |
| Aggregated stderr + summary count | One stderr block with bulleted list. | |
| Silent in stderr, only summary line | Skip stderr entirely. Cleanest output but vanish events worth surfacing. | |

**User's choice:** Per-skill stderr warning + summary count (recommended)

---

## Auto-install Consent State Machine (RECON-02)

### Q7: What's the persisted state shape in machine.toml?

| Option | Description | Selected |
|--------|-------------|----------|
| 3-state enum: Always | Ask | Never | `auto_install_plugins = "always"|"ask"|"never"`. Default unset = first-time prompt. Clear state machine. | ✓ |
| Simple bool | `auto_install_plugins = true|false`. Unset = ask on first sync. 'never' encoded as false (ambiguous). | |
| Two booleans: auto_install + never_ask | Two fields covering all 4 states. Probably overkill. | |

**User's choice:** 3-state enum (recommended)

---

### Q8: First-time prompt UX?

| Option | Description | Selected |
|--------|-------------|----------|
| [Y/n/never] 3-way | Per design doc OQ-3. Y = persist Always, n = persist Ask, never = persist Never. | ✓ |
| [Y/n] 2-way + follow-up if 'n' | Binary first prompt, follow-up "Always skip in future?" on n. More ceremony. | |
| [Y/n] 2-way only | Re-prompt every sync. No persistence. Simpler but annoying for dotfiles. | |

**User's choice:** [Y/n/never] 3-way (recommended)

---

### Q9: `--no-install` flag scope?

| Option | Description | Selected |
|--------|-------------|----------|
| Single-run override only | Skip install for THIS run; don't touch persisted setting. Mirrors Cargo's --frozen/--locked. | ✓ |
| Single-run + persist Never if not set | If unset, also writes never. Saves a second command for users wanting to opt out forever. | |
| Single-run + always persist Never | Always writes never. Surprising side effect. | |

**User's choice:** Single-run override only (recommended)

---

### Q10: What triggers the consent prompt?

| Option | Description | Selected |
|--------|-------------|----------|
| Drift OR missing-from-machine | Prompt fires on Drift OR when lockfile entry has no installed counterpart. Vanished doesn't trigger (nothing to install). Covers fresh-machine bootstrap + drift. | ✓ |
| Missing-from-machine only | Prompt only on missing; drift handled separately. More conservative. | |
| Drift only | Probably wrong — fresh-machine bootstrap is the primary case. | |

**User's choice:** Drift OR missing-from-machine (recommended)

---

### Q11: What happens when state is "ask" and drift detected?

| Option | Description | Selected |
|--------|-------------|----------|
| Prompt again with [Y/n/never] | Re-prompts every drift-detecting sync. ask = "don't decide for me". Always escapable to never. | ✓ |
| Prompt with [Y/n] only (no never) | Drop the never option after first decline. Removes mid-loop escape. | |
| Surface drift as warning, don't prompt | Warn each sync; user runs `tome sync --install` to accept. Removes prompt loop for flag model. | |

**User's choice:** Prompt again with [Y/n/never] (recommended)

---

### Q12: Should `tome doctor` flag drift even when `auto_install_plugins = "never"`?

| Option | Description | Selected |
|--------|-------------|----------|
| Yes, always surface drift in doctor | Doctor is diagnostic; drift is fact. Consent gates sync action, not reporting. | ✓ |
| No, respect consent in doctor too | Hides state but more respectful of "never". | |
| Respect consent in doctor unless --json | Different surfaces, different defaults. | |

**User's choice:** Yes, always surface drift in doctor (recommended)

---

## Edit-in-Library Prompt (RECON-05)

### Pre-question discussion: Fork target ambiguity & Unowned conceptual conflation

User asked two clarifying questions:
1. Does "promote in place" mean the file stays where it is? → Yes, only manifest metadata flips.
2. Does "Unowned" conflate multiple concepts (orphan / user-authored / forked)? → Yes; conceptually four sub-states with distinct provenance histories. Phase 11's schema treats them all as `source_name: None` with no history fields.

User chose **option B**: defer the schema extension (provenance history fields) to Phase 14; Phase 13's fork-in-place is **lossy** — drops upstream provenance metadata when the manifest entry flips to Unowned. Pre-Phase-14 forked entries will permanently have empty history (one-time UX gap, accepted).

This locked the original Q13: fork target = "promote in place to Unowned, lossy". The remaining edit-prompt questions follow.

---

### Q13: Should the edit-in-library prompt fire for Unowned skills?

| Option | Description | Selected |
|--------|-------------|----------|
| No — Unowned bypass the prompt | Detection gate requires `source_name.is_some()`. Unowned has no upstream; revert is meaningless. | ✓ |
| Yes — prompt with revert disabled | Show prompt with fork/skip only. Inconsistent UX. | |

**User's choice:** No — Unowned bypass the prompt (recommended)

---

### Q14: What does the fork-in-place prompt actually show?

| Option | Description | Selected |
|--------|-------------|----------|
| Show version + source being severed | `superpowers has local edits. Last upstream: claude-plugins @ 5.0.7.\n  fork = ...\n  revert = ...\n  skip = ...`. Reads lockfile at prompt time. | ✓ |
| Terse — just action labels | `superpowers has local edits. [fork/revert/skip]`. Less reading; less context. | |
| Show source name only, no version | Middle ground; skips version detail. | |

**User's choice:** Show version + source being severed (recommended)

---

### Q15: In `--no-input` mode, exit code when edits are detected and skipped?

| Option | Description | Selected |
|--------|-------------|----------|
| Zero | Exit 0. Mirrors today's install.rs::reconcile --no-input behavior. Edits are user-intentional. | ✓ |
| Non-zero (e.g. 2) | Forces CI/scripts to notice. Breaks routine syncs in CI for known edits. | |
| Zero by default, --strict-edits flag flips to non-zero | Covers both cases at cost of one more flag. | |

**User's choice:** Zero (recommended)

---

## install.rs Integration Boundary (RECON-01..05 wiring)

### Q16: What happens to `install.rs` when Phase 13 lands?

| Option | Description | Selected |
|--------|-------------|----------|
| Delete entirely | ClaudeMarketplaceAdapter is canonical. Missing claude binary → adapter surfaces clear error. Simpler architecture. | ✓ |
| Keep as fallback when claude missing | Falls back to installed_plugins.json reading. Adds branch to maintain. | |
| Keep alongside, run both during transition | Run both paths, cross-check in tests. Doubles work. | |

**User's choice:** Delete entirely (recommended)

---

### Q17: Where in `lib.rs::sync()` does the new adapter-based reconciliation slot in?

| Option | Description | Selected |
|--------|-------------|----------|
| Replace `reconcile_managed_plugins` (before discovery) | Same position as today's call. Adapter installs first → discover sees result. Minimal flow disturbance. | ✓ |
| After consolidate, before cleanup | Reconcile against freshly-consolidated library. Two-pass; cleaner conceptually. | |
| After distribute (post-sync) | Reconciliation as separate post-step. Probably wrong — drift-apply requires another sync. | |

**User's choice:** Replace `reconcile_managed_plugins` (before discovery) (recommended)

---

### Q18: What happens to today's per-plugin install confirmation prompt (`Install N plugin(s)? [Y/N]`)?

| Option | Description | Selected |
|--------|-------------|----------|
| Replaced by auto_install_plugins consent | New 3-state consent governs install. No mid-sync re-prompt. User decides once. | ✓ |
| Kept as 'Are you sure?' over the consent | Belt-and-suspenders; contradicts consent's purpose. | |
| Kept, scoped to first sync only | Confusing transition. | |

**User's choice:** Replaced by auto_install_plugins consent (recommended)

---

### Q19: When `claude` binary isn't on PATH but `[directories.claude-plugins]` is configured?

| Option | Description | Selected |
|--------|-------------|----------|
| Hard error with actionable message | "claude not found on PATH. Install Claude Code, or remove [directories.claude-plugins] from tome.toml." Sync exits non-zero. | ✓ |
| Skip claude reconciliation, continue sync | Warn and proceed. Hides config-machine mismatch. | |
| Hard error unless --skip-claude-check flag | Default hard error; flag escapes for unusual cases. Adds another flag. | |

**User's choice:** Hard error with actionable message (recommended)

---

### Q20: Does GitAdapter participate in the same reconciliation flow?

| Option | Description | Selected |
|--------|-------------|----------|
| Keep git separate | `resolve_git_directories` handles git as today. GitAdapter exists for trait-shape symmetry. Match/Drift/Vanished is claude-only in v0.10. Honors Phase 12 D-05a regression contract. | ✓ |
| Unify under adapter-based reconciliation | Both go through same dispatch. Architectural symmetry but disrupts existing git tests. Risky. | |
| Unify but Git's available() always Ok(true) | Pulls git into same reconciliation grammar. Probably right long-term but expands Phase 13 scope. | |

**User's choice:** Keep git separate (recommended)

---

### Q21: When are lockfile entries updated relative to install/update operations during sync?

| Option | Description | Selected |
|--------|-------------|----------|
| Per-skill update after each successful adapter call | Failed entries stay at previous lockfile state. Mirrors design doc OQ-4 exactly. | ✓ |
| Batch — update only on full success | All-or-nothing semantics. Surprising on partial failures. | |
| Per-skill, but write lockfile after each call | More IO; only matters in unusual crash scenarios. | |

**User's choice:** Per-skill update after each successful adapter call (recommended)

---

## Claude's Discretion

Areas where the user explicitly deferred to Claude / planner judgement (per CONTEXT.md):

- Exact prompt copy text within bounds shown in D-08 and D-15
- Internal organization of the new reconciliation function (single function vs split helpers)
- Module location of Match/Drift/Vanished classification logic (recommendation: new `reconcile.rs`)
- `AutoInstall` enum location in machine.rs vs sub-module (recommendation: top-level)
- Mock test surface lift from `#[cfg(test)]` to `pub(crate) testing` (tactical)
- Exact "claude binary not found" error wording within Conflict / Why / Suggestion template

## Deferred Ideas

Surfaced during discussion but belong elsewhere (per CONTEXT.md `<deferred>` section):

- Provenance history fields (`previous_source`, `previous_provenance`) on `SkillEntry` → Phase 14
- `tome status` / `tome doctor` Unowned-set surfacing → Phase 14 (UNOWN-03)
- GitAdapter participation in unified Match/Drift/Vanished classification → out of v0.10
- `--strict-edits` flag for non-zero exit on edit-in-library skip → not v0.10
- Per-plugin version pinning in `tome.toml` → blocked on upstream Claude CLI
- `installed_plugins.json` fallback adapter → not added; D-17 deletes outright
- RECON-01 wording cleanup in REQUIREMENTS.md → traceability flag for planner
- Auto-snapshot before reconcile-apply → not added
- Doctor reporting Vanished separately → Phase 14 / status surface area
- Telemetry / metrics for sync outcomes → never (project policy)
