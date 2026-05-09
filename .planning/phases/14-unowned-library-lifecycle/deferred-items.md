# Phase 14 Deferred Items

Items discovered during Phase 14 execution that are intentionally deferred to
later plans within this phase or follow-up phases.

## `#[allow(dead_code)]` retained on `SkillEntry::new_unowned`

**Found during:** Plan 14-01 (this plan).

**Plan instruction (Task 1):** "Drop the `#[allow(dead_code)]` attribute and
its 4-line doc-comment justification — Phase 14 has callers (this plan +
14-04 + 14-05 indirectly)."

**Why retained (Rule 3 deviation):** Plan 14-01's Task 2 captures
`previous_source` via the `.take()` pattern at the three transition sites; it
does NOT add a production caller of `SkillEntry::new_unowned`. The other Wave
1 plan (14-02 — `SkillSummary`) only consumes `previous_source` field reads.
The actual production callers of `new_unowned` arrive in:

- **Plan 14-04** (reassign-unowned-input) — when `tome reassign` re-anchors
  an Unowned skill, the Unowned-input path may construct a fresh entry.
- **Plan 14-05** (remove-skill) — `tome remove skill <name>` flows that
  preserve manifest state mid-operation.

CI runs `cargo clippy --all-targets -- -D warnings` and rejects unused public
items in the binary library. Without the allow, CI fails. Same precedent set
in Phase 11 (see `.planning/phases/11-library-canonical-core/deferred-items.md`).

**Updated comment** points the reader at the resolution plans:

```
// dead_code allow: Phase 14 Plan 14-01 widens the signature with
// `previous_source`. Production callers arrive in Plans 14-04 (reassign
// re-anchor flow) and 14-05 (remove-skill plan/render/execute). Drop
// this attr when those plans land. Tracked in deferred-items.md.
```

**Owner:** Plan 14-04 OR 14-05 — whichever lands first as a real production
caller drops the attribute.

**Status:** All 14-01 tests + full lib suite + integration suite pass.
`cargo clippy --all-targets -p tome -- -D warnings` exits 0. `cargo fmt -p
tome -- --check` exits 0.
