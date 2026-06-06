// TriagePanel — UI-SPEC §TriagePanel (Phase 27 plan 27-02 / SYNC-02).
//
// Sectioned list of pending lockfile-diff decisions. Three outer
// vertical sections (NEW / CHANGED / REMOVED — D-11), each grouped
// by source within the section. The middle column of SyncView's
// split-pane in-progress layout (UI-SPEC §"In-progress state").
//
// **Pitfall 1 — GridList, NOT ListBox.** TriageRow contains an inline
// `[✓ keep]` chip button (D-12); React Aria forbids interactive
// children inside ListBoxItem. We use GridList + GridListItem +
// GridListSection because GridList is explicitly designed for
// interactive children (drag handles, action buttons, etc.) and has
// the same single-selection keyboard navigation as ListBox.
//
// Default expansion (Claude's discretion per CONTEXT §"Claude's
// Discretion"): NEW expanded (most actionable, where bulk-actions
// live per D-13); CHANGED + REMOVED collapsed. Implemented via
// `<details>` wrapping each outer section — keyboard-toggleable via
// Enter/Space on the `<summary>` and discoverable to VoiceOver as a
// disclosure.
//
// Bulk actions (D-13) live ONLY on the NEW section: section-level
// `[Disable all new]` on the outer SectionHeader, source-group
// `[Disable all new from <source>]` on each inner SectionHeader.
// CHANGED and REMOVED outer headers render NO trailing buttons.

import { GridList, GridListItem } from "react-aria-components";
import type { Selection } from "react-aria-components";
import type {
  DirectoryName,
  LockfileDiff,
  SkillName,
  TriageEntry,
} from "../bindings";
import { Button } from "./Button";
import { SectionHeader } from "./SectionHeader";
import { TriageRow, type TriageDecision } from "./TriageRow";
import styles from "./TriagePanel.module.css";

/** Scope of a bulk action — section-level OR source-group inside a
 *  section. Both variants restrict to the NEW section per D-13. */
export type BulkScope =
  | { kind: "section"; section: "new" }
  | { kind: "source-group"; section: "new"; source: DirectoryName };

export interface TriagePanelProps {
  diff: LockfileDiff;
  decisions: ReadonlyMap<SkillName, TriageDecision>;
  onDecisionChange: (skill: SkillName, decision: TriageDecision) => void;
  selectedSkill: SkillName | null;
  onSelect: (skill: SkillName | null) => void;
  onBulkAction: (scope: BulkScope, decision: TriageDecision) => void;
  onApply: () => void;
}

/** Sort a string array alphabetically with the literal `"unowned"`
 *  bucket pinned to the end (UI-SPEC §TriagePanel "Source-group
 *  sorting: alphabetical with 'unowned' group last"). */
function sortSourcesUnownedLast(keys: string[]): string[] {
  return keys.slice().sort((a, b) => {
    if (a === "unowned" && b !== "unowned") return 1;
    if (b === "unowned" && a !== "unowned") return -1;
    return a.localeCompare(b);
  });
}

/** Group an entry array by source_name (or "unowned" when null),
 *  preserving the entry order within each group (the entries are
 *  already sorted alphabetically by skill name from the Rust side,
 *  so group iteration order is per-skill alphabetical). */
function groupBySource(
  entries: readonly TriageEntry[],
): Map<string, TriageEntry[]> {
  const map = new Map<string, TriageEntry[]>();
  for (const entry of entries) {
    const key = entry.source_name ?? "unowned";
    let group = map.get(key);
    if (group === undefined) {
      group = [];
      map.set(key, group);
    }
    group.push(entry);
  }
  return map;
}

/** Decision for an entry, falling back to "keep" when not in the
 *  map. The TriagePanel renders BOTH controlled (`decisions` populated)
 *  and seeded (`decisions.size === 0`) cases — the parent useSync hook
 *  seeds the map after diff load. */
function decisionFor(
  decisions: ReadonlyMap<SkillName, TriageDecision>,
  name: SkillName,
): TriageDecision {
  return decisions.get(name) ?? "keep";
}

/** Count of non-default decisions across Added + Changed (Removed is
 *  implicit per D-13). Drives the `[Apply N decisions]` button label
 *  and disabled state. */
function countPendingDecisions(
  diff: LockfileDiff,
  decisions: ReadonlyMap<SkillName, TriageDecision>,
): number {
  let n = 0;
  for (const entry of diff.added) {
    if (decisionFor(decisions, entry.name) !== "keep") n += 1;
  }
  for (const entry of diff.changed) {
    if (decisionFor(decisions, entry.name) !== "keep") n += 1;
  }
  return n;
}

export function TriagePanel({
  diff,
  decisions,
  onDecisionChange,
  selectedSkill,
  onSelect,
  onBulkAction,
  onApply,
}: TriagePanelProps) {
  const pending = countPendingDecisions(diff, decisions);

  return (
    <section
      className={styles.panel}
      aria-label="Triage decisions"
    >
      <OuterSection
        label="NEW"
        kind="new"
        entries={diff.added}
        decisions={decisions}
        onDecisionChange={onDecisionChange}
        selectedSkill={selectedSkill}
        onSelect={onSelect}
        onBulkAction={onBulkAction}
        defaultOpen
      />
      <OuterSection
        label="CHANGED"
        kind="changed"
        entries={diff.changed}
        decisions={decisions}
        onDecisionChange={onDecisionChange}
        selectedSkill={selectedSkill}
        onSelect={onSelect}
        onBulkAction={onBulkAction}
        defaultOpen={false}
      />
      <OuterSection
        label="REMOVED"
        kind="removed"
        entries={diff.removed}
        decisions={decisions}
        onDecisionChange={onDecisionChange}
        selectedSkill={selectedSkill}
        onSelect={onSelect}
        onBulkAction={onBulkAction}
        defaultOpen={false}
      />
      <div className={styles.applyRow}>
        <Button
          variant="primary"
          onPress={onApply}
          disabled={pending === 0}
          ariaLabel={`Apply ${pending} triage decisions, preview machine.toml diff`}
        >
          {`Apply ${pending} decisions`}
        </Button>
      </div>
    </section>
  );
}

interface OuterSectionProps {
  label: string;
  kind: "new" | "changed" | "removed";
  entries: readonly TriageEntry[];
  decisions: ReadonlyMap<SkillName, TriageDecision>;
  onDecisionChange: (skill: SkillName, decision: TriageDecision) => void;
  selectedSkill: SkillName | null;
  onSelect: (skill: SkillName | null) => void;
  onBulkAction: (scope: BulkScope, decision: TriageDecision) => void;
  defaultOpen: boolean;
}

function OuterSection({
  label,
  kind,
  entries,
  decisions,
  onDecisionChange,
  selectedSkill,
  onSelect,
  onBulkAction,
  defaultOpen,
}: OuterSectionProps) {
  const grouped = groupBySource(entries);
  const groupKeys = sortSourcesUnownedLast([...grouped.keys()]);

  // Bulk-action button rendered ONLY on the NEW section (D-13).
  const sectionTrailing =
    kind === "new" && entries.length > 0 ? (
      <Button
        variant="secondary"
        onPress={() =>
          onBulkAction({ kind: "section", section: "new" }, "disable")
        }
        ariaLabel="Disable all new skills"
      >
        Disable all new
      </Button>
    ) : undefined;

  // The bulk-action button must NOT live inside the <summary> element
  // — axe's `nested-interactive` rule (WCAG 4.1.2) forbids interactive
  // children inside interactive elements (including <summary>). We
  // render the SectionHeader inside <summary> (just label+count) and
  // emit the bulk button OUTSIDE the summary, anchored to the right of
  // the outer-section block via CSS positioning.
  return (
    <details open={defaultOpen} className={styles.outerSection}>
      <summary className={styles.summary}>
        <SectionHeader
          label={label}
          count={entries.length}
          level={2}
        />
      </summary>
      {sectionTrailing !== undefined && (
        <div className={styles.bulkActionRow}>{sectionTrailing}</div>
      )}
      {groupKeys.map((sourceKey) => {
        const groupEntries = grouped.get(sourceKey) ?? [];

        const groupTrailing =
          kind === "new" && sourceKey !== "unowned" && groupEntries.length > 0 ? (
            <Button
              variant="secondary"
              onPress={() =>
                onBulkAction(
                  {
                    kind: "source-group",
                    section: "new",
                    source: sourceKey,
                  },
                  "disable",
                )
              }
              ariaLabel={`Disable all new skills from ${sourceKey}`}
            >
              {`Disable all new from ${sourceKey}`}
            </Button>
          ) : undefined;

        return (
          <div key={sourceKey} className={styles.sourceGroup}>
            <SectionHeader
              label={sourceKey.toUpperCase()}
              count={groupEntries.length}
              level={3}
              trailing={groupTrailing}
            />
            <SourceGroupGridList
              entries={groupEntries}
              decisions={decisions}
              onDecisionChange={onDecisionChange}
              selectedSkill={selectedSkill}
              onSelect={onSelect}
            />
          </div>
        );
      })}
    </details>
  );
}

interface SourceGroupGridListProps {
  entries: readonly TriageEntry[];
  decisions: ReadonlyMap<SkillName, TriageDecision>;
  onDecisionChange: (skill: SkillName, decision: TriageDecision) => void;
  selectedSkill: SkillName | null;
  onSelect: (skill: SkillName | null) => void;
}

function SourceGroupGridList({
  entries,
  decisions,
  onDecisionChange,
  selectedSkill,
  onSelect,
}: SourceGroupGridListProps) {
  // GridList selection — single, controlled. `selectedSkill === null`
  // becomes an empty Set (no row selected). React Aria's `Selection`
  // type accepts either "all" or a Set of keys.
  const selectedKeys: Selection = new Set(
    selectedSkill !== null ? [selectedSkill] : [],
  );

  return (
    <GridList
      aria-label="Triage decisions"
      selectionMode="single"
      selectedKeys={selectedKeys}
      onSelectionChange={(keys: Selection) => {
        // Single selection — pull the first (only) key.
        if (keys === "all") return;
        const first = keys.values().next();
        if (first.done) {
          onSelect(null);
        } else {
          onSelect(first.value as SkillName);
        }
      }}
      className={styles.gridList}
    >
      {entries.map((entry) => (
        <GridListItem
          key={entry.name}
          id={entry.name}
          textValue={entry.name}
        >
          <TriageRow
            name={entry.name}
            changeKind={entry.change_kind}
            sourceName={entry.source_name}
            origin={entry.origin}
            syncedAt={entry.synced_at}
            decision={decisionFor(decisions, entry.name)}
            onDecisionToggle={() =>
              onDecisionChange(
                entry.name,
                decisionFor(decisions, entry.name) === "keep"
                  ? "disable"
                  : "keep",
              )
            }
            isSelected={selectedSkill === entry.name}
          />
        </GridListItem>
      ))}
    </GridList>
  );
}
