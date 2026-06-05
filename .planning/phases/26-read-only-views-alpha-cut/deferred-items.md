# Deferred items — phase 26-read-only-views-alpha-cut

## 2026-05-29 — VIEW-02 group-by toolbar is a no-op (deferred to Phase 27)

The Skills view's Group PopupMenu renders **None / Source / Role** options and persists the
selected state, but the consumer treats every value as flat — no section headers are emitted
between groups. The user can flip the menu but the list never visually reflows.

**Why deferred, not fixed in 26-02:** the alpha cut prioritised the load-bearing 60fps virtualised
list + JS-side fuzzy search (NF-01 budget). Group-by visual rendering requires either:
(a) a heterogeneous Virtualizer item shape with section-header rows the React Aria native
`<Virtualizer>` understands as non-selectable, OR (b) a TanStack Virtual fallback with manual
section-row sizing. Both touch the post-26-08 perf-bench decision path. Phase 27's sync/triage UI
is the natural home — its sectioned "pending changes" surface needs the same section-header
abstraction, so build it once for both.

**Status update for `REQUIREMENTS.md` VIEW-02:** mark as `partial` until Phase 27.

**Acceptance for closure (Phase 27):** picking `Group = Source` or `Group = Role` produces visible
`SectionHeader` rows between the appropriate skill spans, accessible to VoiceOver as a heading
landmark, with totals (e.g. `Claude Code (12)`).

## 2026-05-29 — VIEW-02 "Recent" sort silently falls back to alphabetical name (deferred to Phase 27)

The Skills view Sort PopupMenu offers **Name / Source / Recent** with **Name** as the default.
**Recent** is wired into the toolbar state but the sort comparator falls back to alphabetical
name because `DiscoveredSkill` currently has no `synced_at` (or `last_seen`) field — the manifest
stores a per-skill SHA + provenance but not a "first seen" / "last updated" timestamp the GUI can
sort against.

**Why deferred, not fixed in 26-02:** adding `synced_at: Option<DateTime>` to `DiscoveredSkill`
is a cross-crate change that touches the discovery layer, the manifest serialisation, and the
specta bindings. Phase 27 is already extending the manifest with sync provenance for its "what
changed" surface — adding the timestamp once during that work is cleaner than two passes.

**Status update for `REQUIREMENTS.md` VIEW-02:** mark as `partial` until Phase 27.

**Acceptance for closure (Phase 27):** picking `Sort = Recent` produces a stable ordering keyed
on the manifest's per-skill `synced_at` (most-recent first), with a documented tiebreaker
(suggestion: alphabetical name) for skills that share a timestamp.

## 2026-05-29 — plan 26-08 fmt drift in unrelated files [RESOLVED 2026-05-29]

Resolved by post-verifier `style(phase-26): cargo fmt phase-26 surfaces` commit (orchestrator-applied
during the gap-1 fix loop after the verifier surfaced it as a BLOCKER). `cargo fmt --all -- --check`
now passes on the phase branch. Original entry retained below for traceability — note that the
verifier traced the drift back to phase-26 commits (`17e022d`, `fcf3bba`), not to pre-phase landings
as the original entry assumed; the formatting tweaks were small enough that the executor's
clippy-only verification missed them.

---

Cargo fmt surfaced formatting drift in `crates/tome/src/doctor.rs`, `crates/tome/src/skill.rs`,
and `crates/tome-desktop/src/commands.rs` (originally believed to be unrelated to plan 26-08's
surface; later traced to plans 26-03 and 26-05 commits). Per executor scope-boundary rules these
were NOT auto-fixed during execution — they were logged here and resolved post-verification:

- `crates/tome/src/doctor.rs:1486` — `FindingId::LibraryStaleManifest { skill: name.clone() }` should split across multiple lines.
- `crates/tome/src/doctor.rs:1529` — `FindingId::LibraryBrokenSymlink { path: path.clone() }` should collapse onto one line.
- `crates/tome/src/doctor.rs:1569` — `FindingId::UnparsableFrontmatter { skill: name.clone() }` should split across multiple lines.
- `crates/tome/src/doctor.rs:3690` — assertion line should collapse onto one line.
- `crates/tome/src/skill.rs:183` — `Ok(json) =>` arm should collapse onto one line.
- `crates/tome-desktop/src/commands.rs:13-15` — `use tome::SkillName;` should sort before `use tome::TomePaths;` alphabetically.
