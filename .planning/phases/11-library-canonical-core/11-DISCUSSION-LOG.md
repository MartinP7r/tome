# Phase 11: Library-canonical core - Discussion Log

> **Audit trail only.** Do not use as input to planning, research, or execution agents.
> Decisions are captured in CONTEXT.md — this log preserves the alternatives considered.

**Date:** 2026-05-03
**Phase:** 11-library-canonical-core
**Areas discussed:** Migration prompt UX, Migration atomicity & recovery, Source-removal → Unowned transition, Manifest schema compatibility

---

## Migration prompt UX

### Q1: What does the migration prompt actually show? (LIB-05)

| Option | Description | Selected |
|---|---|---|
| Count + total + per-skill list | Header line + per-skill list (alphabetical), matches v0.8 SAFE-01 partial-failure summary style | ✓ |
| Count + total only | Header only, no per-skill list — lighter for users with 200+ skill libraries | |
| tabled table per skill | `tabled::Table` with columns NAME / FROM / TO / SIZE | |

**User's choice:** Count + total + per-skill list.
**Notes:** Initial choice. Decision was later subsumed by D-MIG-1 (the entire migration was reframed as a one-shot CLI command instead of a sync-time prompt). Output style for `tome migrate-library` defers to SAFE-01 conventions.

---

### Q2 (rejected): How is the disk-delta total computed?

**User rejected this question.** Their reasoning:

> "ask again. after you explain what the prompt is for and whether it is only for migration. if only for migration it should be easy to remove again afterwards."

**Reformulation:** I explained the prompt's purpose, the one-time-per-machine nature, and the "transitional code, deletable in v0.11" plan. Then re-asked.

**Subsequent direction:** User clarified "i'd prefer a one-shot cli command that we remove in 0.11", which collapsed the entire migration UX area: disk-delta calculation became moot, abort semantics became moot (no consent flag needed), and the original 4-question outline collapsed to 2 substantive questions about the new CLI-command shape (Q3 + Q4 below).

---

### Q3: If `tome sync` runs on a machine with an un-migrated v0.9-shape library, what does it do?

| Option | Description | Selected |
|---|---|---|
| Refuse with hint | Print "library is in v0.9 shape. Run `tome migrate-library` first." and exit non-zero | ✓ |
| Auto-convert silently | Convert symlinks to copies as part of normal consolidate phase | |
| Warn but proceed (skip the entries) | Print stderr warning, skip entries, continue syncing local skills + lockfile | |

**User's choice:** Refuse with hint.
**Notes:** Captured as D-MIG-2. Aligns with explicit user-driven migration; transitional code stays isolated to two clearly-marked spots (the migrate-library command + this one detection check) — both deletable in v0.11.

---

### Q4: What identifies a "v0.9-shape entry" that `tome migrate-library` should convert? (LIB-05 detection precision)

| Option | Description | Selected |
|---|---|---|
| Manifest-anchored: symlink + manifest.managed=true | Convert only library entries where path is symlink AND `manifest[name].managed == true` AND `manifest.contains_key(name)` | ✓ |
| Symlink + manifest presence | Convert any symlink keyed in manifest, regardless of `managed` flag | |
| Symlink only (any reason) | Convert any symlink in `library_dir/`, no manifest cross-check | |

**User's choice:** Manifest-anchored.
**Notes:** Captured as D-MIG-3. Maximally conservative — never touches user-created symlinks tome didn't put there.

---

### Q5: How does `tome migrate-library` handle a broken symlink (target deleted) during conversion?

| Option | Description | Selected |
|---|---|---|
| Mark as Unowned (LIB-04 territory) | Set `manifest[name].source_name = None`, remove the broken symlink, leave manifest entry as placeholder | |
| Skip with warning, leave broken | Print stderr warning, leave the broken symlink in place, continue migrating others | ✓ |
| Abort migration entirely | First broken symlink → abort, prompt user to fix manually | |

**User's choice:** Skip with warning + preserve symlink.
**Notes:** Captured as D-MIG-4. User explicitly said: "I don't want any meta-info to get lost. e.g. the directory of the original resources" — the symlink target string carries metadata about where the original source lived. Preserving the symlink keeps that metadata available for later recovery.

---

### Q6 (clarifying side-discussion): Are there any backup mechanisms for non-managed sources?

**User asked, not me.** Triggered a substantive design clarification — the user pushed for "one source of truth": the library is canonical for everything, sources are update channels for managed-only, drift detection on managed via content_hash. This reframed the v0.10 model more precisely than the design doc had stated.

**Sub-decisions captured:**

#### Q6a: For local-skill source-side duplicates, what should v0.10 do?

| Option | Description | Selected |
|---|---|---|
| Skip — manual cleanup | v0.10 doesn't touch source-side duplicates; user `rm -rf` the orphans manually | ✓ |
| Add LIB-06: tome migrate-library --consolidate-local | Opt-in flag deletes source-side duplicates that match library content_hash | |
| Defer to v0.11 | Add a v0.11 milestone item for local-source consolidation tooling | |

**User's choice:** Skip — manual cleanup.
**Notes:** Captured as D-07. v0.10's new architecture (LIB-04) prevents recurrence; existing 15 `personal-skills` orphans are a one-time hand-fix.

#### Q6b: Switch managed-source drift detection from version-based to content_hash-based?

| Option | Description | Selected |
|---|---|---|
| Yes — content_hash is the truth | Replace version-based Match/Drift with content_hash. Version becomes display-only | ✓ |
| Both — hash for accuracy, version for display | content_hash authoritative; version preserved for diff display | |
| Keep version-based as drafted | Don't change RECON-01 | |

**User's choice:** content_hash is the truth.
**Notes:** Captured as D-08 (cross-phase decision). RECON-01 in REQUIREMENTS.md will need rewording. Flag for Phase 13 planner.

---

### Q7: Exit code + summary shape on partial migration failure (LIB-05 atomicity)

| Option | Description | Selected |
|---|---|---|
| Non-zero on ANY failure (skipped or errored) | Both broken-symlink skips AND IO/permission errors trigger exit ≠ 0 | ✓ |
| Non-zero only on errors; skipped is OK | Broken-symlink skips exit 0 with warning; only errors trigger non-zero | |
| Always exit 0 if any conversion succeeded | Treat partial as success; only stderr | |

**User's choice:** Non-zero on ANY failure.
**Notes:** Captured as D-MIG-5. Strict; matches SAFE-01 / `tome remove` semantics. Migration is "done" or "not done."

---

## Source-removal → Unowned transition

### Q8: Which stale-skill cases transition to Unowned (preserve library copy)? (LIB-04 scope)

| Option | Description | Selected |
|---|---|---|
| Case 1 only (config removed) | Source removed from `tome.toml` → library content preserved. Case 2 keeps today's behavior | ✓ |
| Both cases (preserve unless user explicit) | Both Case 1 AND Case 2 → Unowned. Library copy only deleted via explicit `tome forget` | |
| Case 2 prompts; Case 1 silent transition | Case 1 silent transition; Case 2 prompts during sync | |

**User's choice:** Case 1 only.
**Notes:** Captured as D-09. Matches LIB-04 wording. A configured source removing a file is intentional; tome respects that.

### Q9: Where in code does the transition happen? (LIB-04 trigger point)

| Option | Description | Selected |
|---|---|---|
| Hybrid: tome remove sets it; cleanup is safety net | `tome remove` explicit + cleanup phase detects orphans from manual config edits | ✓ |
| Cleanup phase only | Only cleanup phase transitions; manifest stays inconsistent between `tome remove` and next sync | |
| tome remove only | Only `tome remove` sets it; manual config edits leave stale entries | |

**User's choice:** Hybrid.
**Notes:** Captured as D-10. Two simple checks; both deletable in one place each in the future.

### Q10: When `tome sync` runs, are Unowned skills distributed to targets? (LIB-04 distribution semantics)

| Option | Description | Selected |
|---|---|---|
| Yes — distributed normally | Unowned skills get symlinked into targets like any other library entry | ✓ |
| No — skipped during distribute | Unowned skills stay in library but are skipped during distribute. User runs `tome adopt` to resume | |
| Yes, but warn on sync | Distribute as today + stderr warning each sync | |

**User's choice:** Yes — distributed normally.
**Notes:** Captured as D-11. Library is canonical → its content gets symlinked into targets. User opts out via `tome forget` or `machine.toml::disabled`.

---

## Manifest schema compatibility

### Q11: How should `SkillEntry::new` handle the Unowned case? (LIB-03 constructor shape)

| Option | Description | Selected |
|---|---|---|
| Two constructors | `SkillEntry::new(...)` for owned + `SkillEntry::new_unowned(...)` for Unowned | ✓ |
| Single constructor with Option param | `SkillEntry::new(source_name: Option<DirectoryName>, ...)` everywhere | |
| Builder pattern | `SkillEntry::builder().source_name(name).build()` | |

**User's choice:** Two constructors.
**Notes:** Captured as D-13. Avoids forcing every owned-entry call-site to wrap in `Some(...)`. Schema choice (Option<DirectoryName> + serde defaults) was Claude's recommendation, accepted implicitly via the focus on constructor shape — captured as D-12.

---

## Claude's Discretion

Items where the user did NOT pick from a multiple-choice but where convention or prior decisions answer the question:

- Output text of `tome migrate-library` (within SAFE-01 visual conventions)
- Output text of `tome sync`'s "library is in v0.9 shape" error (within Conflict / Why / Suggestion template per Phase 7 D-10)
- `--dry-run` semantics (project convention: every destructive command has one)
- Internal organization of `migration_v010.rs` (e.g., whether to expose helpers via `pub(crate)`)
- Whether to add a fresh `MigrationFailureKind` enum or reuse `RemoveFailure` shape (recommendation: fresh, per POLISH-04 compile-time-enforcement pattern)
- Constructor body for `SkillEntry::new_unowned` (likely shares an inner helper with `new`)

---

## Deferred Ideas

Mentioned during discussion; captured in CONTEXT.md `<deferred>` section:

- `tome adopt` / `tome forget` (Phase 14)
- Surfacing Unowned in `tome status` / `tome doctor` (Phase 14, UNOWN-03)
- Cleanup-message UX rewrite (Phase 16, UX-01)
- Drift detection implementation (Phase 13, RECON-01..05)
- Local-skill source-side cleanup tooling (deferred past v0.10)
- Edit-in-library detection (Phase 13, RECON-05)
- Re-derivation of broken-source-target metadata for status surfacing (Phase 14)
- RECON-01 wording update for content_hash basis (Phase 13 discuss-phase)
