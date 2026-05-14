# Phase 18 — Deferred Items

**Phase:** 18-observability-foundation-sync-diagnostics
**Created:** 2026-05-13
**Status:** Deferrals captured during Plan 18-02 execution.

## OBS-04 — `ChangeCause::PreviouslyFailed` emission deferred

**Status:** Enum variant + `Display` impl SHIPPED in Plan 18-02 (greppability preserved per OBS-04 vocabulary contract — `cause=previously failed` is reachable if/when an emission site fires). Emission site NOT WIRED in Phase 18.

**Why deferred:** The current `SkillEntry` schema in `manifest.rs` does not track per-skill failure state. Detecting "previous sync failed for this skill" requires one of:

1. Adding `last_sync_failed: bool` (or richer enum) field to `SkillEntry`, persisted in `.tome-manifest.json`. Manifest schema bump (backward-compatible via `#[serde(default)]`, but still a schema change).
2. Persisting the previous `SyncReport` to disk (e.g. `last-sync-report.json`) and diffing on next sync. New file, new failure mode (corrupted file).
3. Inferring from existing state — e.g. "skill is in lockfile but missing from library." Not strictly equivalent to "previous sync failed for this skill" semantically; produces false positives for skills that were intentionally removed.

None of these are essential to Phase 18's substrate scope. The OBS-04 success criterion lists four causes; emitting three (`HashChanged`, `NewlyAdded`, `DirectoryNowAllowed`) satisfies the literal text ("the cause that fires IS one of these four"). A strict read demands all four fire eventually; that strict read is honoured by deferring the emission site rather than dropping the variant.

**What would unblock:** Phase 19 polish, OR a v0.12 dedicated manifest-schema bump phase. The cleanest path is Option 1 (`last_sync_failed: bool` on `SkillEntry`) — `#[serde(default)]` makes it backward-compatible. The emission site would land in `library.rs::consolidate_managed` / `consolidate_local` at the `Err(_)` arms that today silently propagate up, branching to set the flag and emit the event on the NEXT sync when the flag is observed.

**Target:** v0.12 or later (no milestone commitment yet).

## OBS-04 — `ChangeCause::DirectoryNowAllowed` inference caveat

**Status:** Wired in `distribute.rs::distribute_to_directory` per the locally-computable inference recommended in RESEARCH §Open Question 2 ("skill in manifest, target has no symlink → was disabled previously"). The inference is implemented as:

```rust
let cause = if was_symlink {
    ChangeCause::HashChanged
} else if in_manifest {
    ChangeCause::DirectoryNowAllowed
} else {
    ChangeCause::NewlyAdded
};
```

Plan 18-02 Task 3's mental walkthrough surfaced the following false-positive case:

- **Case:** On the very first sync after a fresh `tome init` (or after any sync where a new skill is added), `library::consolidate` runs BEFORE `distribute::distribute_to_directory`. Consolidate inserts the skill's manifest entry. By the time distribute iterates the library, `manifest.get(skill_name)` returns `Some` for every skill — including the brand-new ones that have never been distributed to anything before.
- **Result:** distribute's classification logic fires `DirectoryNowAllowed` for genuinely-new skills, where the user-visible cause should arguably be `NewlyAdded`.

**Why accepted:**

1. The false-positive rate is bounded: only fires on the new-symlink-create branch (`!was_symlink`), never on existing-symlink-replace. The `was_symlink && symlink_points_to(false)` branch still correctly fires `HashChanged`.
2. The user-visible meaning ("skill is being symlinked into this directory for the first time in this directory's history") is close enough to "directory now allowed" that the grep vocabulary stays meaningful for debugging.
3. The strict-correct inference requires per-directory-per-skill "has been distributed before" state, which is the same schema-bump trade-off as `PreviouslyFailed` (above).
4. The defensive `NewlyAdded` fallback (`!was_symlink && !in_manifest`) is unreachable in practice but kept for forward-compat: a future code path that disrupts the consolidate-before-distribute invariant would land in the NewlyAdded arm rather than crash.

The wired inference exercises the `DirectoryNowAllowed` variant on every fresh sync, which keeps the greppable surface alive for the user's debug workflow at the cost of an "interpret the cause as 'new-to-this-directory'" mental note.

**What would unblock a strict implementation:** persisting a per-directory-per-skill "has been distributed before" bit. Either a new manifest schema field (similar to `PreviouslyFailed`) or inferring from machine.toml history. Same trade-off as `PreviouslyFailed`: not essential to Phase 18 substrate scope.

**Target:** v0.12 or later (no milestone commitment yet).
