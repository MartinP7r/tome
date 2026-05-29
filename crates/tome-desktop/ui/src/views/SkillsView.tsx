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
import type { DiscoveredSkill } from "../bindings";
import { PopupMenu, type PopupMenuItem } from "../components/PopupMenu";
import { SearchField, type SearchFieldHandle } from "../components/SearchField";
import { SkillListRow } from "../components/SkillListRow";
import { useFuzzySearch } from "../hooks/useFuzzySearch";
import { useSkills } from "../hooks/useSkills";
import styles from "./SkillsView.module.css";

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
  const searchRef = useRef<SearchFieldHandle>(null);

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

  // ⌘F → focus the SearchField (UI-SPEC §Keyboard Map). Scoped to the
  // Skills view; ⌘1/⌘2/⌘3 stay global. Esc inside the SearchField is
  // handled by React Aria SearchField's default behaviour (clears the
  // query — which we observe via the controlled `onChange`).
  useEffect(() => {
    const handler = (event: KeyboardEvent) => {
      if (event.metaKey && event.key === "f") {
        event.preventDefault();
        searchRef.current?.focus();
      }
    };
    window.addEventListener("keydown", handler);
    return () => window.removeEventListener("keydown", handler);
  }, []);

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
                    <SkillListRow skill={skill} selected={isSelected} />
                  )}
                </ListBoxItem>
              )}
            </ListBox>
          </Virtualizer>
        </div>
      </div>
      <div className={styles.detailColumn}>
        {selected ? (
          <p className={styles.placeholder}>
            Detail pane ships in 26-03
          </p>
        ) : (
          <p className={styles.placeholder}>Select a skill to view details</p>
        )}
      </div>
    </div>
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
