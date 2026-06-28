---
phase: 27-sync-triage-ui
plan: 02b
type: execute
wave: 3
depends_on:
  - 27-01b
files_modified:
  - crates/tome-desktop/ui/src/views/SkillsView.tsx
  - crates/tome-desktop/ui/src/views/SkillsView.module.css
  - crates/tome-desktop/tests/a11y/axe.spec.ts
  - .planning/REQUIREMENTS.md
autonomous: true
requirements:
  - SYNC-02
  - VIEW-02
tags:
  - react
  - skills-view
  - section-header
  - view-02-carryover
  - sort-recent
  - synced-at

must_haves:
  truths:
    - "SkillsView Sort=Recent uses synced_at: orders descending ISO-8601 lexicographic; nulls sort last; identical timestamps tiebreak alphabetically by name (Phase 26 deferred-items.md acceptance criteria)."
    - "SkillsView group-by Source renders SectionHeader at level=2 between groups with the source name uppercase + entry count; 'unowned' group sorts last."
    - "SkillsView group-by Role renders SectionHeader at level=2 between groups (MANAGED / LOCAL / UNOWNED) with entry counts."
    - "SkillsView keeps its existing ListBox primitive (no Pitfall 1 carry-over — Skills view rows have no inline buttons)."
    - "REQUIREMENTS.md VIEW-02 status flipped from partial to complete with a one-line traceability note pointing at 27-01a (synced_at plumbing) + 27-02 (SectionHeader level/trailing extension) + 27-02b (this plan's SkillsView wiring)."
  artifacts:
    - path: "crates/tome-desktop/ui/src/views/SkillsView.tsx"
      provides: "Sort=Recent comparator uses DiscoveredSkill.synced_at; group-by Source/Role emits SectionHeader between groups"
    - path: "crates/tome-desktop/ui/src/views/SkillsView.module.css"
      provides: "Group-separator spacing tokens between SectionHeader + virtualised group content"
    - path: ".planning/REQUIREMENTS.md"
      provides: "VIEW-02 status flipped to complete with traceability"
      contains: "VIEW-02"
  key_links:
    - from: "crates/tome-desktop/ui/src/views/SkillsView.tsx::sortComparator"
      to: "DiscoveredSkill.synced_at"
      via: "Recent sort uses synced_at descending with alphabetical name tiebreaker; nulls sort last"
      pattern: "synced_at"
    - from: "crates/tome-desktop/ui/src/views/SkillsView.tsx"
      to: "crates/tome-desktop/ui/src/components/SectionHeader.tsx"
      via: "renders SectionHeader (level=2) between groups when group-by is Source or Role"
      pattern: "SectionHeader"
---

<objective>
Close Phase 26 VIEW-02 carryovers in `SkillsView`: (1) wire the Sort=Recent comparator to the `synced_at` field 27-01a plumbed through `DiscoveredSkill`/`ListReport`/`bindings.ts`; (2) emit the `SectionHeader` primitive 27-02 extended (level=2) between groups when Group=Source or Group=Role is selected; (3) flip the `REQUIREMENTS.md` VIEW-02 status marker from `partial` to `complete` with a traceability note.

Per the revision split (warning W7), this plan was extracted from the original 27-02 to keep its file count under threshold. It runs in Wave 3 in parallel with 27-02 + 27-03; it has no dependency on 27-02's deliverables beyond the SectionHeader prop extension that 27-02 Task 1 ships — that part lands in `SectionHeader.tsx` which both plans coordinate around. Since 27-02b only depends on 27-01b (for synced_at being in `bindings.ts`) and the SectionHeader level/trailing prop extension (which 27-02 Task 1 ships), the planner records the ordering this way: 27-02 Task 1 runs first (it's the smallest unit of work that produces SectionHeader); 27-02b waits on 27-02 Task 1's `SectionHeader.tsx` change before consuming the level prop. To express this cleanly without forcing 27-02b into Wave 4, the executor MUST land 27-02 Task 1 before 27-02b's render-group-by task; the dependency is logical (both plans live in Wave 3 but the SectionHeader file is touched by 27-02 first).

Purpose: small, focused closure of the long-pending Phase 26 carryover; flips one REQUIREMENTS marker; unblocks the user-facing affordance ("Sort by Recent actually works now").

Output: SkillsView comparator + group-by rendering wired to the new field; REQUIREMENTS.md updated; axe scan extended.
</objective>

<execution_context>
@$HOME/.claude/get-shit-done/workflows/execute-plan.md
@$HOME/.claude/get-shit-done/templates/summary.md
</execution_context>

<context>
@.planning/PROJECT.md
@.planning/ROADMAP.md
@.planning/REQUIREMENTS.md
@.planning/phases/27-sync-triage-ui/27-CONTEXT.md
@.planning/phases/27-sync-triage-ui/27-RESEARCH.md
@.planning/phases/27-sync-triage-ui/27-PATTERNS.md
@.planning/phases/27-sync-triage-ui/27-01a-PLAN.md
@.planning/phases/27-sync-triage-ui/27-02-PLAN.md
@.planning/phases/26-read-only-views-alpha-cut/deferred-items.md
@crates/tome-desktop/ui/src/views/SkillsView.tsx
@crates/tome-desktop/ui/src/components/SectionHeader.tsx
@crates/tome-desktop/ui/src/lib/relativeTime.ts

<interfaces>
From crates/tome-desktop/ui/src/bindings.ts (post-27-01b):
- DiscoveredSkill record now includes `synced_at: string | null` (ISO-8601)

From crates/tome-desktop/ui/src/components/SectionHeader.tsx (post-27-02 Task 1):
- interface SectionHeaderProps { label: string; count: number; level?: 2 | 3; trailing?: ReactNode }
</interfaces>
</context>

<tasks>

<task type="auto" tdd="true">
  <name>Task 1: Sort=Recent comparator uses synced_at; group-by Source/Role emits SectionHeader between groups; REQUIREMENTS.md VIEW-02 status flipped</name>
  <files>
    crates/tome-desktop/ui/src/views/SkillsView.tsx,
    crates/tome-desktop/ui/src/views/SkillsView.module.css,
    crates/tome-desktop/tests/a11y/axe.spec.ts,
    .planning/REQUIREMENTS.md
  </files>
  <read_first>
    crates/tome-desktop/ui/src/views/SkillsView.tsx (full file — current group-by + sort implementation; identify the comparator memo + the grouping pipeline),
    crates/tome-desktop/ui/src/views/SkillsView.module.css (existing styles),
    crates/tome-desktop/ui/src/components/SectionHeader.tsx (after 27-02 Task 1's level extension),
    crates/tome-desktop/ui/src/lib/relativeTime.ts (formatRelative — already used; carryforward),
    crates/tome-desktop/ui/src/bindings.ts (post-27-01b — DiscoveredSkill.synced_at must be present),
    .planning/phases/26-read-only-views-alpha-cut/deferred-items.md (VIEW-02 acceptance criteria),
    .planning/phases/27-sync-triage-ui/27-CONTEXT.md §"Phase 26 carryovers folded in",
    .planning/REQUIREMENTS.md §VIEW (locate VIEW-02 row to flip status)
  </read_first>
  <behavior>
    - Test (Sort=Recent): given fixture skills with synced_at values `["2026-06-01T10:00:00Z", "2026-06-05T09:00:00Z", null, "2026-06-03T10:00:00Z"]`, the Recent sort produces the ordered sequence `[2026-06-05, 2026-06-03, 2026-06-01, null]` (most-recent first; null sorts last). Two skills with identical synced_at sort alphabetically by name.
    - Test (group-by Source): selecting Group=Source renders SectionHeader at level 2 with the source name uppercase and the group count between groups; the inner list of skills is the group's content. VoiceOver heading rotor announces each group header.
    - Test (group-by Role): selecting Group=Role renders SectionHeaders for each role (MANAGED, LOCAL, UNOWNED) with counts.
    - Test (group-by None): selecting Group=None renders no SectionHeader (back-compat with Phase 26 behavior).
    - Test (a11y): Skills view group-by rendering passes axe-core scan with no nested-interactive violations (SkillsView keeps its existing ListBox; the GridList vs ListBox decision in TriagePanel does NOT carry over because SkillsView rows have no inline buttons).
    - Test (REQUIREMENTS update): grep for `VIEW-02` in REQUIREMENTS.md confirms the status marker now reads `complete`.
  </behavior>
  <action>
    1. In `SkillsView.tsx`, locate the existing sort comparator memo and update the "recent" branch to a comparator that handles nulls last + alphabetical tiebreaker:
       ```ts
       case "recent": {
         const aSynced = a.synced_at;
         const bSynced = b.synced_at;
         if (aSynced === bSynced) return a.name.localeCompare(b.name);
         if (aSynced === null) return 1;   // nulls last
         if (bSynced === null) return -1;
         return bSynced.localeCompare(aSynced); // ISO-8601 descending
       }
       ```
       Place the helper inline in the comparator switch; do not add a new file.
    2. Locate the group-by rendering. If group-by is currently a no-op (just renders the flat list regardless of the menu selection — a VIEW-02 carryover), implement it now. Build a `useMemo` that maps the sorted array into `Array<{ groupKey: string; groupLabel: string; entries: DiscoveredSkill[] }>` based on groupBy:
       - "none": single group with empty label (omit the SectionHeader; just render the existing flat list).
       - "source": group by `source_name` (unowned key "unowned", label "UNOWNED"); sort groups alphabetically with "UNOWNED" last.
       - "role": group by `origin.kind` (managed/local) plus the unowned case; label uppercase ("MANAGED" / "LOCAL" / "UNOWNED").
       Render: for each group, emit `<SectionHeader label={groupLabel} count={entries.length} level={2} />` (only when `groupKey` is not empty) followed by the existing ListBox/Virtualizer rendering of that group's entries. SectionHeader sits OUTSIDE the virtualiser between virtualised chunks (the small group count makes this acceptable; typical user has fewer than 10 groups).
    3. Update `SkillsView.module.css` with any group-separator spacing needed (margin-top on the group container; bottom-padding to keep group rhythm). Reuse Phase 26 spacing tokens.
    4. Flip `.planning/REQUIREMENTS.md` VIEW-02 status from `partial` to `complete` (or whatever the current marker reads — confirm the exact line). Add a one-line traceability note: `(closed in Phase 27 via 27-01a synced_at plumbing + 27-02 SectionHeader level extension + 27-02b SkillsView wiring)`.
    5. Extend `crates/tome-desktop/tests/a11y/axe.spec.ts` Skills-view block (or add a new sibling block "Skills view group-by a11y") that navigates to Skills, selects Group=Source via the existing menu, runs `await new AxeBuilder({ page }).analyze()`, asserts `results.violations === []`.
  </action>
  <verify>
    <automated>cd crates/tome-desktop/ui &amp;&amp; npm run typecheck &amp;&amp; npm run test -- --run SkillsView &amp;&amp; cd ../.. &amp;&amp; npx playwright test tests/a11y/axe.spec.ts -g "Skills view" &amp;&amp; rg -q "VIEW-02.*complete" /Users/martin/dev/opensource/tome/.planning/REQUIREMENTS.md</automated>
  </verify>
  <done>
    `SkillsView` Sort=Recent uses `synced_at` (descending, null last, alphabetical tiebreaker); group-by Source / Role renders SectionHeader between groups; axe-core scan clean; REQUIREMENTS.md VIEW-02 marker flipped from partial to complete with traceability note. Phase 26 VIEW-02 carryovers fully closed.
  </done>
</task>

</tasks>

<threat_model>
## Trust Boundaries

| Boundary | Description |
|----------|-------------|
| ListReport payload → SkillsView | DiscoveredSkill records cross the boundary with `synced_at`; already crossed in 27-01a; consumed here as a sort key |

## STRIDE Threat Register

| Threat ID | Category | Component | Disposition | Mitigation Plan |
|-----------|----------|-----------|-------------|-----------------|
| T-27-02b-01 | Information Disclosure | synced_at timestamp leaks usage info to webview | accept | Same user owns both sides; not PII. |
| T-27-02b-02 | Tampering | Comparator handling of null/undefined edge cases produces inconsistent ordering | mitigate | Behavior tests in Task 1 pin the null-last + alphabetical-tiebreaker contract; comparator handles all four shape combinations explicitly. |
| T-27-02b-SC | Tampering | npm/cargo package installs | accept | This plan adds ZERO new external packages. |
</threat_model>

<verification>
- `cd crates/tome-desktop/ui && npm run typecheck && npm run test -- --run SkillsView` — Sort/group-by tests pass.
- `npx playwright test tests/a11y/axe.spec.ts -g "Skills view"` — axe scan clean.
- `rg "VIEW-02.*complete" .planning/REQUIREMENTS.md` matches the flipped marker.
- Manual smoke: launch `cargo tauri dev`, navigate to Skills view, click Sort=Recent, observe skills ordered by synced_at (most-recent first); click Group=Source, observe SectionHeader rows between source groups.
</verification>

<success_criteria>
- ROADMAP Phase 27 SC#6 met: VIEW-02 closure — SectionHeader wired into SkillsView group-by Source/Role; Recent sort uses synced_at; REQUIREMENTS.md flipped to complete.
- `bindings.ts` unchanged from 27-01b (this plan adds no new commands or events).
- All existing tests in `crates/tome-desktop/ui` continue to pass.
</success_criteria>

<output>
Create `.planning/phases/27-sync-triage-ui/27-02b-SUMMARY.md` when done.
</output>
