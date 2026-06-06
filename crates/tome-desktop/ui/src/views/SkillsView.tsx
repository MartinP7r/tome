// SkillsView — VIEW-02 / NF-01 + UI-SPEC §"Per-view Design — Skills".
//
// List column (pinned SearchField + Sort/Group toolbar + virtualised list)
// alongside a detail column (placeholder in this plan — the real
// DetailHeader + MarkdownBody land in plans 26-03 / 26-04).
//
// **Virtualisation: React Aria native `<Virtualizer>` 1.18** (UI-SPEC OQ-1
// resolution, path A — zero extra dep, free a11y semantics; fixed 52px rows
// avoid measurement complexity). The TanStack-Virtual sub-clause of D-14 is
// superseded — see UI-SPEC §Revision Log entry for plan 26-02.
//
// **Fuzzy search: fuse.js JS-side.** RESEARCH §"Standard Stack — Fuzzy
// search" — per-keystroke Tauri command would blow the 60fps budget.

import { useEffect, useMemo, useRef, useState } from "react";
import {
  ListBox,
  ListBoxItem,
  ListLayout,
  Virtualizer,
} from "react-aria-components";
import { writeText } from "@tauri-apps/plugin-clipboard-manager";
import { commands } from "../bindings";
import type { DiscoveredSkill } from "../bindings";
import { DetailHeader } from "../components/DetailHeader";
import { MarkdownBody } from "../components/MarkdownBody";
import { PopupMenu, type PopupMenuItem } from "../components/PopupMenu";
import { SearchField, type SearchFieldHandle } from "../components/SearchField";
import { SectionHeader } from "../components/SectionHeader";
import {
  SkillContextMenu,
  type SkillContextAction,
} from "../components/SkillContextMenu";
import { SkillListRow } from "../components/SkillListRow";
import { useFuzzySearch } from "../hooks/useFuzzySearch";
import { useSkillActions } from "../hooks/useSkillActions";
import { useSkillDetail } from "../hooks/useSkillDetail";
import { useSkills } from "../hooks/useSkills";
import styles from "./SkillsView.module.css";

// Phase 27 plan 27-01b extracted the text-input-focused guard into
// `lib/textInputFocus.ts` so useMenuActions (which binds global ⌘R / ⌘.
// in 27-01b) can share the same abstain-when-typing logic. The Pitfall 9
// / T-26-07-01 rationale stays the same: a focused SearchField (or any
// text input) routes ⌘C to the OS Edit menu, and skill-scoped handlers
// must not collide.
import { isTextInputFocused } from "../lib/textInputFocus";

export type SortMode = "name" | "source" | "recent";
export type GroupMode = "none" | "source" | "role";

const SORT_ITEMS: PopupMenuItem[] = [
  { id: "name", label: "Name" },
  { id: "source", label: "Source" },
  { id: "recent", label: "Recent" },
];
const GROUP_ITEMS: PopupMenuItem[] = [
  { id: "none", label: "None" },
  { id: "source", label: "Source" },
  { id: "role", label: "Role" },
];

// React Aria's Virtualizer accepts a Layout *class* in its `layout` prop and
// constructs the instance internally. Passing the class — not an instance —
// is the documented usage pattern (react-aria.adobe.com/Virtualizer).
const LAYOUT_OPTIONS = { rowSize: 52, gap: 0, padding: 0 } as const;

/** Shared ListBoxItem renderer — used by both the Group=None flat list
 *  and each Group=Source / Group=Role group's per-group ListBox. */
function renderSkillRow(
  skill: DiscoveredSkill,
  setSelected: (name: string) => void,
) {
  return (
    <ListBoxItem
      id={skill.name}
      textValue={skill.name}
      aria-label={`${skill.name}, source ${skill.source_name}, ${skill.origin.kind === "managed" ? "managed" : "local"}`}
    >
      {({ isSelected }) => (
        <SkillContextMenuRow
          skill={skill}
          selected={isSelected}
          onSelect={() => setSelected(skill.name)}
        />
      )}
    </ListBoxItem>
  );
}

export function SkillsView() {
  const { skills, warnings, err } = useSkills();
  const [query, setQuery] = useState("");
  const [sort, setSort] = useState<SortMode>("name");
  const [group, setGroup] = useState<GroupMode>("none");
  const [selected, setSelected] = useState<string | null>(null);
  const [removalAnnouncement, setRemovalAnnouncement] = useState<string>("");
  const searchRef = useRef<SearchFieldHandle>(null);

  // Plan 26-06 D-03 — when a watcher-driven refetch removes the currently
  // selected skill from the list (renamed/deleted externally), clear the
  // selection AND fire a one-time aria-live announcement so VoiceOver users
  // know why focus shifted. The selection state itself is preserved across
  // every other refresh — `useSkills` rebuilds `skills[]`; we leave
  // `selected` alone unless the skill is gone.
  useEffect(() => {
    if (!skills || selected === null) return;
    if (!skills.some((s) => s.name === selected)) {
      setSelected(null);
      setRemovalAnnouncement("Selected skill was removed.");
      // Clear after a beat so the message is fresh next time it fires.
      const t = window.setTimeout(() => setRemovalAnnouncement(""), 1000);
      return () => window.clearTimeout(t);
    }
  }, [skills, selected]);

  // Filter via fuse.js (display-only — D-GUI-08 §S-10 allowed exception for
  // sort/group/filter computations).
  const filtered = useFuzzySearch<DiscoveredSkill>(skills, query, {
    keys: ["name", "source_name"],
  });

  // Sort (display-only — D-GUI-08 §S-10 allowed exception).
  const sorted = useMemo(() => sortSkills(filtered, sort), [filtered, sort]);

  // Group buckets — Phase 27 plan 27-02b closes the Phase 26 VIEW-02
  // carryover. SectionHeader (level=2) renders OUTSIDE the virtualiser
  // between virtualised chunks; each group gets its own ListBox so
  // selection still threads through to the detail column. The typical
  // user has <10 groups (1–3 directories × 2 origin kinds) so the small
  // outer iteration is cheap and avoids heterogeneous-Virtualizer
  // complexity — see Phase 26 deferred-items.md note "(a) heterogeneous
  // Virtualizer item shape … OR (b) TanStack Virtual fallback" — we
  // took neither; instead we virtualise per-group, which keeps the 60
  // fps bench (NF-01) and yields a free heading rotor.
  const groups = useMemo(() => groupSkills(sorted, group), [sorted, group]);

  // ⌘F → focus the SearchField is now dispatched by the native macOS
  // menu (plan 26-07's View → Focus Search item) through the typed
  // `MenuAction::FocusSearch` event, handled in `useMenuActions`. The
  // duplicate document-level listener that lived here pre-26-07 was
  // removed to avoid a double-binding conflict with the menu
  // accelerator (Pitfall 9). Esc inside the SearchField is still
  // handled by React Aria SearchField's default behaviour (clears the
  // query — which we observe via the controlled `onChange`).

  // Look up the disabled flag for the selected row from the in-list skill
  // record — this drives both the context-menu label and the ⌘D shortcut
  // before the detail-pane fetch completes. The list rows don't carry
  // disabled state today (Phase 26's `DiscoveredSkill` shape pre-dates the
  // GUI's machine.toml read), so we lean on the detail-pane fetch via
  // `useSkillDetail` below for the authoritative flag.

  if (err) {
    return (
      <div className={styles.split}>
        <div className={styles.errorBanner}>
          <strong>[{err.code}]</strong> {err.message}
        </div>
      </div>
    );
  }

  return (
    <div className={styles.split}>
      {/* Hidden aria-live region — fires when a watcher refresh removes the
          selected skill (D-03 + UI-SPEC §Transient). visually-hidden via
          inline style to avoid pulling another CSS-Module slot. */}
      <div
        role="status"
        aria-live="polite"
        aria-atomic="true"
        style={{
          position: "absolute",
          width: 1,
          height: 1,
          padding: 0,
          overflow: "hidden",
          clip: "rect(0 0 0 0)",
          whiteSpace: "nowrap",
          border: 0,
        }}
      >
        {removalAnnouncement}
      </div>
      <div className={styles.listColumn}>
        <div className={styles.searchSlot}>
          <SearchField ref={searchRef} value={query} onChange={setQuery} />
        </div>
        <div className={styles.toolbar}>
          <PopupMenu
            label="Sort"
            ariaLabel="Sort skills"
            items={SORT_ITEMS}
            selectedId={sort}
            onChange={(id) => setSort(id as SortMode)}
          />
          <PopupMenu
            label="Group"
            ariaLabel="Group skills"
            items={GROUP_ITEMS}
            selectedId={group}
            onChange={(id) => setGroup(id as GroupMode)}
          />
        </div>
        {warnings.length > 0 && (
          <div className={styles.warningBanner}>
            {warnings.length} warning{warnings.length === 1 ? "" : "s"} during
            discovery
          </div>
        )}
        <div className={styles.list}>
          {group === "none" ? (
            // Group=None — Phase 26 back-compat path. One Virtualizer +
            // ListBox covering the whole filtered+sorted list. No
            // SectionHeader rendered (the toolbar selection is
            // semantically "flat").
            <Virtualizer layout={ListLayout} layoutOptions={LAYOUT_OPTIONS}>
              <ListBox
                aria-label="Skills"
                items={sorted}
                selectionMode="single"
                selectedKeys={selected ? new Set([selected]) : new Set()}
                onSelectionChange={(keys) => {
                  if (keys === "all") return;
                  const first = [...keys][0];
                  setSelected(typeof first === "string" ? first : null);
                }}
              >
                {(skill) => renderSkillRow(skill, setSelected)}
              </ListBox>
            </Virtualizer>
          ) : (
            // Group=Source / Group=Role — emit a SectionHeader (level=2)
            // before each group's virtualised ListBox. Selection state
            // is shared across groups (single `selected` key in the
            // parent state) so jumping rows across groups still drives
            // the detail column correctly.
            groups.map((g) => (
              <div key={g.groupKey} className={styles.groupSection}>
                <div className={styles.groupHeader}>
                  <SectionHeader
                    label={g.groupLabel}
                    count={g.entries.length}
                    level={2}
                  />
                </div>
                <Virtualizer
                  layout={ListLayout}
                  layoutOptions={LAYOUT_OPTIONS}
                >
                  <ListBox
                    aria-label={`Skills — ${g.groupLabel}`}
                    items={g.entries}
                    selectionMode="single"
                    selectedKeys={selected ? new Set([selected]) : new Set()}
                    onSelectionChange={(keys) => {
                      if (keys === "all") return;
                      const first = [...keys][0];
                      setSelected(typeof first === "string" ? first : null);
                    }}
                  >
                    {(skill) => renderSkillRow(skill, setSelected)}
                  </ListBox>
                </Virtualizer>
              </div>
            ))
          )}
        </div>
      </div>
      <div className={styles.detailColumn}>
        {selected ? (
          <DetailColumn name={selected} />
        ) : (
          <p className={styles.placeholder}>Select a skill to view details</p>
        )}
      </div>
    </div>
  );
}

/* ------------------------------------------------------------------ */
/*  Detail column — DetailHeader + (future MarkdownBody from 26-04).  */
/* ------------------------------------------------------------------ */

function DetailColumn({ name }: { name: string }) {
  const { detail, err, refetch } = useSkillDetail(name);
  const actions = useSkillActions({
    name,
    disabled: detail?.disabled ?? false,
    refetch,
  });

  // Phase 26 plan 26-07 HIG + Pitfall 9 audit (26-A11Y-AUDIT.md §"Keyboard
  // shortcut audit results"):
  //
  // * **⌘C** — Edit menu's Predefined Copy already handles text-input copy.
  //   When the focus is on a text field (the SearchField is the only one
  //   in Skills view today) the OS routes ⌘C to that control automatically.
  //   We gate the skill-scoped ⌘C handler on `activeElement` so the
  //   Predefined item wins on text inputs (Pitfall 9, T-26-07-01) and our
  //   "copy source path" only fires when the list/detail has focus.
  // * **⌘O** — bare ⌘O is the macOS HIG convention for "Open…" dialog.
  //   Rebound to **⌘⇧O** ("Open source folder in Finder") so the
  //   convention stays available for a future Phase 27+ Open dialog.
  // * **⌘D** — bare ⌘D is "Don't Save" / "Duplicate" / "Bookmarks" in
  //   most macOS apps. Removed entirely: the "Disable on this machine"
  //   button in the DetailHeader is one click away and the action is
  //   deliberate; promoting it to a keyboard shortcut overlaps too many
  //   conventions to keep safe. (UI-SPEC §Keyboard Map amended in the
  //   same commit.)
  useEffect(() => {
    if (!detail) return;
    const handler = (event: KeyboardEvent) => {
      if (!event.metaKey || event.ctrlKey || event.altKey) return;
      // ⌘C — skill-scoped copy, but only when no text input owns focus.
      // The Predefined Edit > Copy item handles the input case.
      if (!event.shiftKey && event.key === "c" && !isTextInputFocused()) {
        event.preventDefault();
        void actions.onCopyPath();
        return;
      }
      // ⌘⇧O — rebound from bare ⌘O after the macOS-HIG conflict audit.
      if (event.shiftKey && event.key.toLowerCase() === "o") {
        event.preventDefault();
        void actions.onOpenSource();
        return;
      }
      // ⌘D removed by 26-07 audit — no keyboard shortcut for Disable.
    };
    window.addEventListener("keydown", handler);
    return () => window.removeEventListener("keydown", handler);
  }, [detail, actions.onCopyPath, actions.onOpenSource]);

  const surfacedErr = err ?? actions.err;

  return (
    <>
      {/* Visually-hidden aria-live region for D-06 announcements. */}
      <div
        role="status"
        aria-live="polite"
        aria-atomic="true"
        style={{
          position: "absolute",
          width: 1,
          height: 1,
          padding: 0,
          overflow: "hidden",
          clip: "rect(0 0 0 0)",
          whiteSpace: "nowrap",
          border: 0,
        }}
      >
        {actions.announcement}
      </div>
      {surfacedErr && (
        <div className={styles.errorBanner}>
          <strong>[{surfacedErr.code}]</strong> {surfacedErr.message}
        </div>
      )}
      {detail ? (
        <>
          <DetailHeader
            detail={detail}
            onOpenSource={actions.onOpenSource}
            onCopyPath={actions.onCopyPath}
            onDisableToggle={actions.onDisableToggle}
            copyState={actions.copyState}
          />
          <MarkdownBody body={detail.body} skillName={detail.name} />
        </>
      ) : surfacedErr ? null : (
        <p className={styles.placeholder}>Loading…</p>
      )}
    </>
  );
}

/* ------------------------------------------------------------------ */
/*  Row wrapper — adds the right-click context menu (D-07).           */
/* ------------------------------------------------------------------ */

interface SkillContextMenuRowProps {
  skill: DiscoveredSkill;
  selected: boolean;
  onSelect: () => void;
}

function SkillContextMenuRow({
  skill,
  selected,
  onSelect,
}: SkillContextMenuRowProps) {
  // Right-click contextual actions (D-07). To avoid issuing a per-row
  // `getSkillDetail` fetch (would be N + 1 IPC calls on mount), we do NOT
  // know the row's disabled flag here — the context menu uses neutral
  // labels ("Toggle disabled on this machine"), and the actual toggle
  // resolves the current state via a fresh fetch on click. The DetailHeader
  // continues to show the precise Disable / Enable label (driven by the
  // selected row's `useSkillDetail` result in `DetailColumn`).
  const handleAction = async (action: SkillContextAction) => {
    // Select the row first so the detail pane mirrors the user's intent.
    onSelect();
    if (action === "open") {
      const res = await commands.openSourceFolder(skill.name);
      // Errors are swallowed at the row level — DetailColumn's surfaced
      // error banner is the canonical surface. Right-click errors aren't
      // common enough to warrant a per-row banner.
      void res;
    } else if (action === "copy") {
      const res = await commands.copyPath(skill.name);
      if (res.status === "ok") {
        try {
          await writeText(res.data);
        } catch {
          /* see comment above — silenced */
        }
      }
    } else if (action === "toggle-disable") {
      const detailRes = await commands.getSkillDetail(skill.name);
      if (detailRes.status === "ok") {
        await commands.setSkillDisabled(skill.name, !detailRes.data.disabled);
      }
    }
  };

  return (
    <SkillContextMenu
      // The label flips based on a per-click fetch, not pre-loaded state.
      // We render "Disable" by default; the actual toggle inspects current
      // state at click time.
      disabled={false}
      onAction={handleAction}
    >
      <SkillListRow skill={skill} selected={selected} />
    </SkillContextMenu>
  );
}

/**
 * Sort `skills` by `mode` — pure helper, returns a new array (does not
 * mutate the input). Exported so unit tests can pin the comparator
 * contract directly (mirrors the 27-01a `join_synced_at_from_manifest`
 * + 27-02 `lockfile_diff_projection` extraction pattern: pure fn
 * factored out, tests exercise it without spinning the full view).
 *
 * Sort=Recent (Phase 27 plan 27-02b closes the Phase 26 VIEW-02
 * carryover): keys on `DiscoveredSkill.synced_at` descending
 * (most-recent first) since the manifest stores RFC-3339 strings —
 * ISO-8601 lexicographic comparison is the same as chronological for
 * fixed-width zoned timestamps. Null synced_at sorts last. Identical
 * timestamps tiebreak alphabetically by name so the order is stable
 * across renders.
 */
export function sortSkills(
  skills: DiscoveredSkill[],
  mode: SortMode,
): DiscoveredSkill[] {
  const out = [...skills];
  switch (mode) {
    case "name":
      out.sort((a, b) => a.name.localeCompare(b.name));
      return out;
    case "source":
      out.sort((a, b) => {
        const s = a.source_name.localeCompare(b.source_name);
        return s !== 0 ? s : a.name.localeCompare(b.name);
      });
      return out;
    case "recent": {
      // Plan 27-02b: keys on synced_at (D-16, 27-01a-plumbed). Null last,
      // alphabetical name tiebreaker. The bindings.ts type declares
      // `synced_at?: string | null` — both `undefined` (field omitted)
      // and `null` are treated as "no timestamp" and sort last.
      out.sort((a, b) => {
        const aSynced = a.synced_at ?? null;
        const bSynced = b.synced_at ?? null;
        if (aSynced === bSynced) return a.name.localeCompare(b.name);
        if (aSynced === null) return 1; // nulls last
        if (bSynced === null) return -1;
        return bSynced.localeCompare(aSynced); // ISO-8601 descending
      });
      return out;
    }
  }
}

/** A single render group produced by `groupSkills`. */
export interface SkillGroup {
  /** Stable identity for React keys + tests. Empty string for the
   *  "no grouping" case (groupMode = "none"). */
  groupKey: string;
  /** Display label for the SectionHeader (uppercase, e.g. "MANAGED"
   *  or "PERSONAL"). Empty string when groupMode = "none". */
  groupLabel: string;
  /** Entries that belong to this group. Preserves the input ordering
   *  (i.e. the sort applied upstream) within each group. */
  entries: DiscoveredSkill[];
}

const UNOWNED_KEY = "unowned";
const UNOWNED_LABEL = "UNOWNED";

/**
 * Bucket `skills` into render groups based on `mode` — pure helper.
 *
 * - "none" → a single group with no label (caller suppresses the
 *   SectionHeader render). Preserves the Phase 26 flat-list contract.
 * - "source" → groups by `source_name`. Empty or missing source_name
 *   maps to the "unowned" group (label "UNOWNED"). Groups are sorted
 *   alphabetically with UNOWNED forced last.
 * - "role" → groups by `origin.kind` ("managed" / "local"). The order
 *   is fixed: MANAGED → LOCAL → UNOWNED. (UNOWNED appears only if a
 *   skill has no origin shape, which `discover_all` doesn't produce
 *   today; the slot is included for parity with the source-mode
 *   bucket so the section-header rotor reads consistently across
 *   group modes.)
 *
 * Empty groups are omitted from the result so the render layer
 * doesn't emit headers with `(0)` counts.
 */
export function groupSkills(
  skills: DiscoveredSkill[],
  mode: GroupMode,
): SkillGroup[] {
  if (mode === "none") {
    return [{ groupKey: "", groupLabel: "", entries: [...skills] }];
  }

  if (mode === "source") {
    const buckets = new Map<string, DiscoveredSkill[]>();
    for (const s of skills) {
      const key = s.source_name && s.source_name.length > 0
        ? s.source_name
        : UNOWNED_KEY;
      const list = buckets.get(key) ?? [];
      list.push(s);
      buckets.set(key, list);
    }
    const keys = [...buckets.keys()].sort((a, b) => {
      // UNOWNED forced last regardless of alphabetical order.
      if (a === UNOWNED_KEY && b !== UNOWNED_KEY) return 1;
      if (b === UNOWNED_KEY && a !== UNOWNED_KEY) return -1;
      return a.localeCompare(b);
    });
    return keys.map((key) => ({
      groupKey: key,
      groupLabel: key === UNOWNED_KEY ? UNOWNED_LABEL : key.toUpperCase(),
      entries: buckets.get(key) ?? [],
    }));
  }

  // mode === "role"
  const managed: DiscoveredSkill[] = [];
  const local: DiscoveredSkill[] = [];
  const unowned: DiscoveredSkill[] = [];
  for (const s of skills) {
    if (s.origin.kind === "managed") managed.push(s);
    else if (s.origin.kind === "local") local.push(s);
    else unowned.push(s);
  }
  const out: SkillGroup[] = [];
  if (managed.length > 0) {
    out.push({ groupKey: "managed", groupLabel: "MANAGED", entries: managed });
  }
  if (local.length > 0) {
    out.push({ groupKey: "local", groupLabel: "LOCAL", entries: local });
  }
  if (unowned.length > 0) {
    out.push({
      groupKey: UNOWNED_KEY,
      groupLabel: UNOWNED_LABEL,
      entries: unowned,
    });
  }
  return out;
}
