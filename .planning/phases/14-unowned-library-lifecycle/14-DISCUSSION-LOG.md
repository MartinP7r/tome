# Phase 14: Unowned-library lifecycle - Discussion Log

> **Audit trail only.** Do not use as input to planning, research, or execution agents.
> Decisions are captured in CONTEXT.md — this log preserves the alternatives considered.

**Date:** 2026-05-07
**Phase:** 14-unowned-library-lifecycle
**Areas discussed:** API merge (added mid-discussion), Adopt collision (became Reassign extension), Forget cleanup scope (became Remove skill), Last-known source provenance, Status/doctor rendering

---

## Initial Gray-Area Selection

**Areas presented:** Adopt collision behavior, Forget cleanup scope, Last-known source provenance, Status/doctor rendering.

**User selected:** All four areas.

---

## API Merge (raised mid-discussion by user)

User asked: "what's `tome adopt` for and can/should it not be merged into existing commands?"

After discussion of mechanics (adopt vs reassign do near-identical work; forget has no existing analogue), user confirmed `tome remove` exists today as a directory-scoped command. User then raised that subcommand-style `tome remove dir` / `tome remove skill` would be cleaner than overloading `<NAME>`.

### Decision: `tome adopt` fate

| Option | Description | Selected |
|--------|-------------|----------|
| Fold into `tome reassign` | Drop the Unowned refusal in `reassign.rs:58-63`. Reassign handles both Owned→Owned and Unowned→Owned. | ✓ |
| Keep as flat top-level command | Adopt stays separate; semantic verb distinction. | |
| Subcommand split: `tome reassign skill`, `tome adopt skill` | Mirror remove's dir/skill style across all skill-noun commands. | |
| Drop adopt entirely — manual recovery only | Re-add to tome.toml + sync. Fails for fork-in-place case. | |

**User's choice:** Fold into `tome reassign`.

**Notes:** Decision made after user confirmed the `tome remove` subcommand split direction. Folding adopt + restructuring remove together produces the smallest API surface.

### Decision: `tome forget` fate (implicit, captured during discussion)

`tome forget` is folded into `tome remove skill <name>`, paired with `tome remove dir <name>` (renamed from today's `tome remove <name>`). Project policy "Backward compat: None" makes the breaking change acceptable.

---

## Area 1: Reassign extension (formerly "Adopt collision behavior")

Note: First three-question batch was rejected by user with request to clarify. Rejection led to the API-merge discussion above. Re-asked after merge decision.

### Different-content collision

| Option | Description | Selected |
|--------|-------------|----------|
| Refuse + suggest --force | Hash check in `plan`; refuse with error; new `--force` flag bypasses. | ✓ |
| Silent relink (today's behavior) | Don't add a content check. Drift resolves on next sync. | |
| Always overwrite library content into target | Library is canonical; overwrite target unconditionally. | |
| Confirm prompt (default no) | Interactive; --yes to skip. | |

**User's choice:** Refuse + suggest --force.

**Notes:** Hardens behaviour for both Owned→Owned and Unowned→Owned reassigns; closes a today-existing silent-discordance footgun.

### Target directory role restriction

| Option | Description | Selected |
|--------|-------------|----------|
| Discovery + mixed only | Reject target-only directories with a clear error. | ✓ |
| Any directory role (today's behavior) | Don't add a role check. | |
| Discovery only (strict) | Reject mixed dirs too. | |

**User's choice:** Discovery + mixed only.

**Notes:** Reassigning into a target-only dir leaves the skill stranded — nothing rediscovers it on next sync.

---

## Area 2: Remove skill cleanup scope (formerly "Forget cleanup scope")

### Cleanup scope (multi-select)

| Option | Description | Selected |
|--------|-------------|----------|
| Lockfile entry (`tome.lock`) | Remove matching `[[skills]]` block. Required for dotfiles cross-machine workflow. | ✓ |
| `machine.toml::disabled` entry | Remove if present. | ✓ |
| `machine.toml` per-directory `enabled`/`disabled` lists | Remove from each per-directory list. | ✓ |
| None — just the UNOWN-02 floor | Don't touch lockfile or machine.toml. | |

**User's choice:** All three additions selected (lockfile + machine.toml::disabled + per-directory lists).

**Notes:** Cross-machine workflow makes lockfile drift a real footgun; machine.toml hygiene is free at remove time.

### Owned-skill guard

| Option | Description | Selected |
|--------|-------------|----------|
| Refuse with hint (matches UNOWN-02) | No --force bypass. Hint directs to `tome remove dir` or filesystem delete. | ✓ |
| Refuse, but `--force` overrides | Risk: next sync re-discovers. | |
| Always allow | No guard; confusing semantics. | |

**User's choice:** Refuse with hint.

**Notes:** `--force` bypass would be misleading — source file would still be on disk; next sync would re-discover.

### Confirmation prompt default

| Option | Description | Selected |
|--------|-------------|----------|
| Default `n` | Safer default for destructive action. Matches `tome remove dir`. | ✓ |
| Default `y` | User already typed the command; default to proceed. | |
| No prompt by default; `--confirm` to opt in | Skip prompt entirely. | |

**User's choice:** Default `n`, `--yes` skips.

---

## Area 3: Last-known source provenance

### Source of "last-known source"

| Option | Description | Selected |
|--------|-------------|----------|
| Add `previous_source: Option<DirectoryName>` to SkillEntry | New optional field on SkillEntry + LockEntry. Written at all 3 transition sites. | ✓ |
| Use existing `source_path` field only | No schema change; render path string. | |
| Combine: previous_source if present, fall back to source_path | Add field AND show source_path supplementary. | |
| Show 'unknown' — don't carry provenance | Skip the last-known source UNOWN-03 part. | |

**User's choice:** Add `previous_source: Option<DirectoryName>`.

**Notes:** Closes the lossy gap from Phase 13 D-13. `#[serde(default, skip_serializing_if = "Option::is_none")]` keeps backward compat.

### Pre-Phase-14 entry fallback

| Option | Description | Selected |
|--------|-------------|----------|
| Fall back to `source_path` | Render path string when `previous_source` is None; collapse_home for tilde. | ✓ |
| Show '(source unknown)' literally | Honest about the lossy gap. | |
| Backfill on first sync after upgrade | Derive guess from source_path; persist. | |

**User's choice:** Fall back to `source_path`.

**Notes:** No backfill performed; one-time UX gap acknowledged. New transitions get clean provenance.

---

## Area 4: Status/doctor rendering

### Rendering shape

| Option | Description | Selected |
|--------|-------------|----------|
| Tabled, like Directories | NAME / LAST-KNOWN SOURCE / SYNCED columns; Style::blank() + bold header. | ✓ |
| Bulleted list | `Unowned skills (3):\n  • foo (last from: my-plugins)`. | |
| Grouped by transition cause | Group as 'From removed directories: ...' / 'From fork-in-place: ...'. | |

**User's choice:** Tabled.

**Notes:** Visual consistency with existing Directories table; reuses tabled::Table + Style::blank() pattern.

### Placement in `tome status`

| Option | Description | Selected |
|--------|-------------|----------|
| After Directories, before Health | Library → Directories → Unowned → Health reading order. | ✓ |
| Top, immediately after Library line | Maximum prominence; could feel alarmist. | |
| Bottom, after Health | Footer note; may get overlooked. | |

**User's choice:** After Directories, before Health.

**Notes:** Section omits cleanly when empty (no header, no blank line).

### Doctor severity treatment

| Option | Description | Selected |
|--------|-------------|----------|
| Informational only — don't count toward total_issues | Separate `unowned_skills` field on DoctorReport; doesn't affect exit code. | ✓ |
| Warning severity — count toward total_issues | Each unowned skill emits `DiagnosticIssue { severity: Warning }`. | |
| Mixed: count if `previous_source` is None only | Pre-Phase-14 lossy entries warn; clean entries informational. | |

**User's choice:** Informational only.

**Notes:** Unowned is intentional state (user removed a directory); conflating with actionable malfunctions would be noisy. Surface visibly in `tome doctor` output but don't change exit-code semantics.

---

## Claude's Discretion

The following implementation details were left to Claude/the planner per established codebase conventions:

- Exact wording of error messages (within Conflict / Why / Suggestion template).
- Exact prompt copy text (within bounds of D-B3).
- Internal organisation of `tome remove skill` (extend `remove.rs` vs new module — recommendation: extend).
- Whether `RemoveSkillFailureKind` is separate enum or generic kind parameter (recommendation: separate).
- Where `SkillSummary` lives (recommendation: new `summary.rs` or alongside SkillEntry in `manifest.rs`).
- `--force` × `--dry-run` interaction on reassign (recommendation: dry-run wins).
- Per-directory ordering of unowned skills in the table (recommendation: sorted by name ASC).

## Deferred Ideas

- Bulk unowned-set operations (`--all-unowned`, `--orphans-only`) → v0.11+ if needed.
- Per-cause grouping in unowned rendering → not in v0.10; data is there to derive later.
- `previous_source` backfill for pre-Phase-14 entries → not in v0.10; D-C2 fallback is sufficient.
- `tome reassign --force` interaction with library/source drift → pre-existing behaviour, not Phase-14 scope.
- REQUIREMENTS.md / ROADMAP.md / PROJECT.md text updates → planner picks up as part of phase plan or sibling doc PR.
