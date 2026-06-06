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

type SortMode = "name" | "source" | "recent";
type GroupMode = "none" | "source" | "role";

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

  // Group is a no-op render in this plan. Section-header rendering for
  // grouped mode is a follow-up — the toolbar is wired so the API contract
  // is in place; rendering grouped sections is small but adds Layout
  // complexity we'd rather verify against the 26-08 perf bench first.
  // (TODO 26-03+: implement grouped rendering once perf bench is green.)
  void group;

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
              {(skill) => (
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
              )}
            </ListBox>
          </Virtualizer>
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

function sortSkills(skills: DiscoveredSkill[], mode: SortMode): DiscoveredSkill[] {
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
    case "recent":
      // `DiscoveredSkill` does not carry a synced_at timestamp today (the
      // manifest does; this is a discovery-time projection). For alpha we
      // fall back to name order with this code comment as the flag — the
      // real "recent" sort wires through the manifest in a follow-up plan
      // once the GUI fetches manifest-shaped data alongside the list.
      out.sort((a, b) => a.name.localeCompare(b.name));
      return out;
  }
}
