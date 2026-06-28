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

import { useState } from "react";
import { GridList, GridListItem } from "react-aria-components";
import type { Selection } from "react-aria-components";
import { commands } from "../bindings";
import type {
  DirectoryName,
  LockfileDiff,
  MachineTomlPreview,
  SkillName,
  TomeError,
  TriageDecision as TriageDecisionWire,
  TriageEntry,
} from "../bindings";
import { Button } from "./Button";
import { MachineTomlDiff } from "./MachineTomlDiff";
import { PreviewPopover } from "./PreviewPopover";
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
  /** Phase 27 plan 27-03 — invoked when the user clicks [Apply] inside the
   *  PreviewPopover AND the `applyMachineToml` command resolves successfully.
   *  The parent (useSync) clears decisions + dismisses the apply error. */
  onApplied: () => void;
}

/** Flatten the React-side decisions Map into the IPC wire shape. Skips
 *  Keep decisions because the Rust side treats them as no-ops at write
 *  time (a skill missing from the Vec is implicitly Keep). Sending only
 *  Disable entries keeps the boundary payload minimal — the Apply flow
 *  doesn't need to round-trip the user's explicit Keep choices. */
function buildDecisionsForIPC(
  decisions: ReadonlyMap<SkillName, TriageDecision>,
): TriageDecisionWire[] {
  const out: TriageDecisionWire[] = [];
  for (const [skill, decision] of decisions) {
    if (decision === "disable") {
      out.push({ skill, decision: "disable" });
    }
  }
  return out;
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
  onApplied,
}: TriagePanelProps) {
  const pending = countPendingDecisions(diff, decisions);
  // Phase 27 plan 27-03 — Apply flow state. The PreviewPopover renders
  // <MachineTomlDiff preview={previewResult} /> inside its body slot.
  // `previewResult` populates when the user clicks the trigger Button
  // (`onPress` fires `previewMachineToml` and stashes the result here).
  // `applyError` surfaces an inline disclosure if `applyMachineToml` rejects;
  // the popover stays open so the user can read it + retry.
  const [previewResult, setPreviewResult] = useState<MachineTomlPreview | null>(null);
  const [previewError, setPreviewError] = useState<TomeError | null>(null);
  const [applyError, setApplyError] = useState<TomeError | null>(null);

  /** Fired when the user presses the trigger Button (before the popover
   *  opens). Fetches the preview from the Rust side; the popover renders
   *  the result inside `<MachineTomlDiff />`. If the preview fetch fails,
   *  we still let the popover open and surface the error there (the user
   *  has something to look at instead of nothing happening). */
  const onPreviewPress = async () => {
    setPreviewResult(null);
    setPreviewError(null);
    const ipcDecisions = buildDecisionsForIPC(decisions);
    const res = await commands.previewMachineToml(ipcDecisions);
    if (res.status === "ok") {
      setPreviewResult(res.data);
    } else {
      setPreviewError(res.error);
    }
  };

  /** Fired when the user clicks [Apply] inside the popover. Commits the
   *  decisions via the canonical atomic write; on success, notify the
   *  parent so it can clear React-side triage state; on error, store the
   *  TomeError for the inline disclosure. */
  const onApply = async (): Promise<void> => {
    const ipcDecisions = buildDecisionsForIPC(decisions);
    const res = await commands.applyMachineToml(ipcDecisions);
    if (res.status === "ok") {
      // Clear local state + notify parent. The Phase-26 watcher will fire
      // MachinePrefsChanged for free and idle hooks (useSkills,
      // useDoctorReport) will refetch on their own.
      setPreviewResult(null);
      setPreviewError(null);
      setApplyError(null);
      onApplied();
    } else {
      // The popover already closed (Apply's onPress closes before
      // awaiting). We surface the error via the row below the trigger;
      // a future iteration could keep the popover open with a richer
      // inline error UI per the plan's D-11 pattern.
      throw res.error;
    }
  };

  return (
    <section className={styles.panel} aria-label="Triage decisions">
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
        {/* The trigger Button is owned by TriagePanel so the label can carry
         *  the pending count + the onPress can pre-fetch the preview before
         *  the popover opens. PreviewPopover treats it as a slot. */}
        <PreviewPopover
          trigger={
            <Button
              variant="primary"
              onPress={onPreviewPress}
              disabled={pending === 0}
              ariaLabel={`Apply ${pending} triage decisions, preview machine.toml diff`}
            >
              {`Apply ${pending} decisions`}
            </Button>
          }
          width={480}
          helperText="Applying writes ~/.config/tome/machine.toml. The CLI sees this change immediately."
          onApply={onApply}
          onError={setApplyError}
        >
          {previewError !== null ? (
            <div role="alert" className={styles.applyError}>
              <span>
                [{previewError.code}] {previewError.message}
              </span>
              {previewError.context.length > 0 && (
                <details>
                  <summary>Show context</summary>
                  <ul>
                    {previewError.context.map((c, i) => (
                      <li key={i}>{c}</li>
                    ))}
                  </ul>
                </details>
              )}
            </div>
          ) : previewResult !== null ? (
            <MachineTomlDiff preview={previewResult} />
          ) : (
            <p>Computing preview…</p>
          )}
        </PreviewPopover>
      </div>
      {applyError !== null && (
        <div role="alert" className={styles.applyError}>
          <span>
            [{applyError.code}] {applyError.message}
          </span>
          {applyError.context.length > 0 && (
            <details>
              <summary>Show context</summary>
              <ul>
                {applyError.context.map((c, i) => (
                  <li key={i}>{c}</li>
                ))}
              </ul>
            </details>
          )}
        </div>
      )}
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
