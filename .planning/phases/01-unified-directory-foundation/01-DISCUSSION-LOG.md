# Phase 1: Unified Directory Foundation - Discussion Log

> **Audit trail only.** Do not use as input to planning, research, or execution agents.
> Decisions are captured in CONTEXT.md — this log preserves the alternatives considered.

**Date:** 2026-04-12
**Phase:** 01-unified-directory-foundation
**Areas discussed:** Old config error UX, Role defaulting logic, Migration path, Test strategy

---

## Old Config Error UX

| Option | Description | Selected |
|--------|-------------|----------|
| Serde error + hint | Let toml::from_str fail naturally. Catch error, check raw TOML for old keys, append migration hint. | :heavy_check_mark: |
| Pre-parse validation | Scan raw TOML for old-format keys before deserializing. Bail with dedicated migration error. | |
| deny_unknown_fields only | Add #[serde(deny_unknown_fields)] to Config. Default serde error, no custom hint. | |

**User's choice:** Serde error + hint
**Notes:** None

### Follow-up: deny_unknown_fields as belt-and-suspenders

| Option | Description | Selected |
|--------|-------------|----------|
| Yes, deny_unknown_fields too | Catches typos and future format drift. Hint logic handles old-format specifically. | :heavy_check_mark: |
| No, hint logic is enough | Keep Config struct lenient for forward compat. | |

**User's choice:** Yes, deny_unknown_fields too
**Notes:** None

---

## Role Defaulting Logic

### Wizard role explanation approach

| Option | Description | Selected |
|--------|-------------|----------|
| Inline descriptions in summary | Auto-assign roles, show summary with plain-english description next to each role. | :heavy_check_mark: |
| Legend block before summary | Print role legend once, then compact summary table. | |
| Both — legend + inline hints | Legend block AND inline hints per row. | |

**User's choice:** Inline descriptions in summary
**Notes:** User emphasized that role names (managed, synced, target) are jargon and need plain-english explanations for new users. This applies to all user-facing role displays, not just the wizard.

### Role edit interaction

| Option | Description | Selected |
|--------|-------------|----------|
| Select menu with descriptions | dialoguer Select showing each role with one-line description, filtered to valid roles per type. | :heavy_check_mark: |
| Free text with validation | User types role name, rejects invalid with error. | |
| You decide | Claude picks based on existing wizard patterns. | |

**User's choice:** Select menu with descriptions
**Notes:** None

---

## Migration Path

| Option | Description | Selected |
|--------|-------------|----------|
| CHANGELOG section only | Breaking Changes section with before/after config example. | :heavy_check_mark: |
| Standalone MIGRATION.md | Dedicated migration guide in docs/. | |
| Both — CHANGELOG + MIGRATION.md | CHANGELOG summary + detailed guide. | |

**User's choice:** CHANGELOG section only
**Notes:** None

### Follow-up: No config behavior

| Option | Description | Selected |
|--------|-------------|----------|
| Error with init hint | Print "no config found, run tome init" and exit. Current behavior. | :heavy_check_mark: |
| Auto-launch wizard | Automatically start init wizard if no config exists. | |
| You decide | Claude picks based on current codebase behavior. | |

**User's choice:** Error with init hint
**Notes:** Same as current behavior.

---

## Test Strategy

| Option | Description | Selected |
|--------|-------------|----------|
| Rewrite tests in lockstep | Convert each module's tests as it's converted. Integration tests last. | :heavy_check_mark: |
| Delete old tests, write fresh | Remove all tests, get code compiling, write new tests from scratch. | |
| Integration-first | Get binary compiling, write integration tests first, backfill unit tests. | |

**User's choice:** Rewrite tests in lockstep
**Notes:** None

### Follow-up: Snapshot handling

| Option | Description | Selected |
|--------|-------------|----------|
| Regenerate fresh | Delete old snapshots, run tests, accept new via cargo insta review. | :heavy_check_mark: |
| Update inline | Review each snapshot diff individually. | |
| You decide | Claude picks pragmatic approach per snapshot. | |

**User's choice:** Regenerate fresh
**Notes:** None

---

## Claude's Discretion

- Exact wording of role descriptions
- Internal module organization for DirectoryName
- Whether to keep TargetName as type alias during transition

## Deferred Ideas

None — discussion stayed within phase scope.
