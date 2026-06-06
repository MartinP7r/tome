// SkillsView tests — Phase 27 plan 27-02b.
//
// Closes the Phase 26 VIEW-02 carryovers documented in
// `.planning/phases/26-read-only-views-alpha-cut/deferred-items.md`:
//
// 1. Sort=Recent comparator keys on `synced_at` (most-recent first;
//    nulls sort last; identical timestamps tiebreak alphabetically).
// 2. Group=Source and Group=Role emit `<SectionHeader level={2}>`
//    between groups (consumes the 27-02 SectionHeader extension).
// 3. Group=None preserves the Phase 26 flat-list rendering.
//
// Implementation lives in `../SkillsView.tsx`. The comparator and
// grouping helpers are exported for direct unit-testing — mirrors the
// 27-01a `join_synced_at_from_manifest` and 27-02 `lockfile_diff_projection`
// extraction pattern (pure fn factored out for testability).

import { render, screen } from "@testing-library/react";
import { describe, it, expect, vi } from "vitest";
import type { DiscoveredSkill } from "../../bindings";
import { groupSkills, sortSkills, SkillsView } from "../SkillsView";

// ---------------------------------------------------------------------------
// Test fixtures
// ---------------------------------------------------------------------------

function managedSkill(
  name: string,
  source: string,
  synced_at: string | null,
): DiscoveredSkill {
  return {
    name,
    path: `/fixture/${source}/${name}`,
    source_name: source,
    origin: {
      kind: "managed",
      provenance: {
        registry_id: "fixture@npm",
        version: "1.0.0",
        git_commit_sha: null,
      },
    },
    synced_at,
  };
}

function localSkill(
  name: string,
  source: string,
  synced_at: string | null,
): DiscoveredSkill {
  return {
    name,
    path: `/fixture/${source}/${name}`,
    source_name: source,
    origin: { kind: "local" },
    synced_at,
  };
}

// ---------------------------------------------------------------------------
// Sort=Recent comparator — VIEW-02 carryover #2 (synced_at)
// ---------------------------------------------------------------------------

describe("sortSkills(mode='recent')", () => {
  it("orders by synced_at descending (most-recent first)", () => {
    // Plan 27-02b acceptance shape: synced_at values
    // [2026-06-01T10:00:00Z, 2026-06-05T09:00:00Z, null, 2026-06-03T10:00:00Z]
    // produce the order [2026-06-05, 2026-06-03, 2026-06-01, null].
    const skills: DiscoveredSkill[] = [
      localSkill("a", "personal", "2026-06-01T10:00:00Z"),
      localSkill("b", "personal", "2026-06-05T09:00:00Z"),
      localSkill("c", "personal", null),
      localSkill("d", "personal", "2026-06-03T10:00:00Z"),
    ];
    const result = sortSkills(skills, "recent").map((s) => s.name);
    expect(result).toEqual(["b", "d", "a", "c"]);
  });

  it("places null synced_at last", () => {
    const skills: DiscoveredSkill[] = [
      localSkill("x", "personal", null),
      localSkill("y", "personal", "2026-06-05T09:00:00Z"),
      localSkill("z", "personal", null),
    ];
    const result = sortSkills(skills, "recent").map((s) => s.name);
    // Non-null first; nulls last in alphabetical name order.
    expect(result).toEqual(["y", "x", "z"]);
  });

  it("tiebreaks identical synced_at alphabetically by name", () => {
    const ts = "2026-06-05T09:00:00Z";
    const skills: DiscoveredSkill[] = [
      localSkill("zebra", "personal", ts),
      localSkill("alpha", "personal", ts),
      localSkill("mango", "personal", ts),
    ];
    const result = sortSkills(skills, "recent").map((s) => s.name);
    expect(result).toEqual(["alpha", "mango", "zebra"]);
  });

  it("tiebreaks multiple nulls alphabetically by name", () => {
    const skills: DiscoveredSkill[] = [
      localSkill("zebra", "personal", null),
      localSkill("alpha", "personal", null),
    ];
    const result = sortSkills(skills, "recent").map((s) => s.name);
    expect(result).toEqual(["alpha", "zebra"]);
  });

  it("does not mutate the input array", () => {
    const skills: DiscoveredSkill[] = [
      localSkill("b", "personal", "2026-06-01T10:00:00Z"),
      localSkill("a", "personal", "2026-06-05T09:00:00Z"),
    ];
    const snapshot = skills.map((s) => s.name);
    sortSkills(skills, "recent");
    expect(skills.map((s) => s.name)).toEqual(snapshot);
  });
});

// ---------------------------------------------------------------------------
// groupSkills — VIEW-02 carryover #1 (group-by section headers)
// ---------------------------------------------------------------------------

describe("groupSkills(mode='none')", () => {
  it("returns a single group with no label when group=none", () => {
    const skills: DiscoveredSkill[] = [
      localSkill("a", "personal", null),
      localSkill("b", "personal", null),
    ];
    const groups = groupSkills(skills, "none");
    expect(groups).toHaveLength(1);
    expect(groups[0]?.groupKey).toBe("");
    expect(groups[0]?.groupLabel).toBe("");
    expect(groups[0]?.entries.map((s) => s.name)).toEqual(["a", "b"]);
  });
});

describe("groupSkills(mode='source')", () => {
  it("groups by source_name and sorts groups alphabetically with UNOWNED last", () => {
    const skills: DiscoveredSkill[] = [
      localSkill("a1", "personal", null),
      localSkill("b1", "plugins", null),
      localSkill("a2", "personal", null),
      // source_name === "" is the unowned marker — the projection upstream
      // sets it to "" / "unowned"; we treat empty source as unowned to be
      // resilient.
      { ...localSkill("c1", "", null), source_name: "" },
    ];
    const groups = groupSkills(skills, "source");
    expect(groups.map((g) => g.groupKey)).toEqual([
      "personal",
      "plugins",
      "unowned",
    ]);
    // Group labels are uppercase per UI-SPEC.
    expect(groups.map((g) => g.groupLabel)).toEqual([
      "PERSONAL",
      "PLUGINS",
      "UNOWNED",
    ]);
    expect(groups[0]?.entries.map((s) => s.name)).toEqual(["a1", "a2"]);
    expect(groups[1]?.entries.map((s) => s.name)).toEqual(["b1"]);
    expect(groups[2]?.entries.map((s) => s.name)).toEqual(["c1"]);
  });

  it("places UNOWNED group last even when alphabetically earlier", () => {
    const skills: DiscoveredSkill[] = [
      // alphabetic order would be "unowned" < "zzz"; UNOWNED must override.
      { ...localSkill("u1", "", null), source_name: "" },
      localSkill("z1", "zzz", null),
    ];
    const groups = groupSkills(skills, "source");
    expect(groups.map((g) => g.groupKey)).toEqual(["zzz", "unowned"]);
  });
});

describe("groupSkills(mode='role')", () => {
  it("groups MANAGED / LOCAL / UNOWNED with counts", () => {
    const skills: DiscoveredSkill[] = [
      managedSkill("m1", "plugins", null),
      localSkill("l1", "personal", null),
      managedSkill("m2", "plugins", null),
      localSkill("l2", "personal", null),
    ];
    const groups = groupSkills(skills, "role");
    expect(groups.map((g) => g.groupLabel)).toEqual(["MANAGED", "LOCAL"]);
    expect(groups.find((g) => g.groupKey === "managed")?.entries.length).toBe(
      2,
    );
    expect(groups.find((g) => g.groupKey === "local")?.entries.length).toBe(2);
  });
});

// ---------------------------------------------------------------------------
// Render-level smoke test — confirms wiring + Group=None back-compat
// ---------------------------------------------------------------------------
//
// Driving the PopupMenu open from a vitest jsdom environment is flaky with
// React Aria (the Popover renders to a portal and menu-item activation needs
// keyboard-event sequences `@testing-library/user-event` provides, which is
// not part of the repo's vitest deps). The helper-level tests above
// (`groupSkills` / `sortSkills`) cover the grouping + sort comparator
// contract directly; the user-facing render path is exercised end-to-end by
// the axe-core playwright scan in `tests/a11y/axe.spec.ts` (which uses a
// real browser-driving Playwright runtime to toggle the Group menu).

// Mock the bindings so the SkillsView mounts without IPC.
vi.mock("../../bindings", () => ({
  commands: {
    listSkills: () =>
      Promise.resolve({
        status: "ok" as const,
        data: {
          skills: [
            managedSkill("axiom-build", "plugins", "2026-06-05T09:00:00Z"),
            localSkill("rust-helper", "personal", "2026-06-03T10:00:00Z"),
            localSkill("deprecated-skill", "personal", null),
          ],
          warnings: [],
        },
      }),
    getSkillDetail: () =>
      Promise.resolve({ status: "ok" as const, data: null }),
  },
  events: {
    manifestChanged: { listen: () => Promise.resolve(() => undefined) },
    lockfileChanged: { listen: () => Promise.resolve(() => undefined) },
    libraryChanged: { listen: () => Promise.resolve(() => undefined) },
    machinePrefsChanged: { listen: () => Promise.resolve(() => undefined) },
  },
}));

describe("SkillsView — render smoke", () => {
  it("mounts and renders the Skills listbox", async () => {
    render(<SkillsView />);
    await screen.findByRole("listbox", { name: "Skills" });
  });

  it("does not render SectionHeader when Group=None (Phase 26 back-compat)", async () => {
    render(<SkillsView />);
    await screen.findByRole("listbox", { name: "Skills" });
    // The default Group state is "none"; no level-2 headings should
    // appear inside the SkillsView body — preserves the Phase 26
    // flat-list rendering contract.
    expect(screen.queryAllByRole("heading", { level: 2 })).toHaveLength(0);
  });
});
