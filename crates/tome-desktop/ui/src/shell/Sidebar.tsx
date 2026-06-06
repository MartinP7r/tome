// Sidebar — vibrancy/translucent nav rail (UI-SPEC §Shell §Sidebar + §NavItem).
//
// Four nav items as of Phase 27 plan 27-01b (Status / Skills / Sync / Health)
// inside a React Aria ListBox so arrow-key navigation comes for free (NF-02).
//
// Per UI-SPEC §Sidebar updated NavItem (plan 27-01b):
//   - Sync row has a spinner slot — when a sync is in flight the row icon
//     swaps to a small system-spinner-styled SVG; otherwise the slot is
//     empty (the row label sits in default position).
//   - Sync row has a dual-meaning badge slot — "pending" (managed-blue fill)
//     for unresolved triage decisions [populated in plan 27-02] and
//     "failures" (danger fill) for partial-failure counts [populated in plan
//     27-05]. The plan-27-01b SyncView skeleton passes `{ kind: "none" }`
//     until those later plans wire the real counts.
//   - Health row keeps its danger-fill badge (D-12).
//
// Plan 26-05: the optional `badgeCount` prop is backed by
// `useDoctorReport().report?.findings.length` (App.tsx). The badge clears at
// zero findings.

import { ListBox, ListBoxItem } from "react-aria-components";
import { useStatus } from "../hooks/useStatus";
import styles from "./Sidebar.module.css";

export type Section = "status" | "skills" | "sync" | "health";

const SECTIONS: { id: Section; label: string }[] = [
  { id: "status", label: "Status" },
  { id: "skills", label: "Skills" },
  { id: "sync", label: "Sync" },
  { id: "health", label: "Health" },
];

/** Dual-meaning Sync badge state. `kind: "none"` renders nothing. */
export type SyncBadge =
  | { kind: "none"; count?: 0 }
  | { kind: "pending"; count: number }
  | { kind: "failures"; count: number };

export interface SidebarProps {
  selected: Section;
  onChange: (section: Section) => void;
  /** Count surfaced next to the Health nav item (D-02). */
  badgeCount?: number;
  /** True while `useSync().isRunning` — swaps the Sync row icon to a
   *  small spinner. Plan 27-01b leaves the icon slot empty when this is
   *  false; the row label still aligns the same way. */
  syncInProgress?: boolean;
  /** Phase 27 dual-meaning badge slot. `pending` → blue fill (UI-SPEC
   *  managed accent); `failures` → red fill (danger). Plan 27-01b passes
   *  `{ kind: "none" }` until 27-02 / 27-05 wire the real counts. */
  syncBadge?: SyncBadge;
}

export function Sidebar({
  selected,
  onChange,
  badgeCount = 0,
  syncInProgress = false,
  syncBadge = { kind: "none" },
}: SidebarProps) {
  const { status } = useStatus();
  const skillCount =
    status?.library_count.count ?? // CountOrError.count (Option<usize>)
    null;
  const footerText =
    skillCount === null ? "tome" : `tome · ${skillCount} skills`;

  return (
    <aside className={styles.sidebar} aria-label="Sections">
      <div className={styles.caption}>LIBRARY</div>
      <ListBox
        className={styles.nav}
        aria-label="Sections"
        selectionMode="single"
        disallowEmptySelection
        selectedKeys={new Set([selected])}
        onSelectionChange={(keys) => {
          // Single-selection ListBox gives us a `Set<Key>` with at most
          // one element. Narrow to the literal Section union.
          const first = [...keys][0];
          if (typeof first === "string" && isSection(first)) {
            onChange(first);
          }
        }}
      >
        {SECTIONS.map((s) => {
          const showHealthBadge = s.id === "health" && badgeCount > 0;
          const showSyncBadge =
            s.id === "sync" && syncBadge.kind !== "none" && syncBadge.count > 0;
          const ariaLabel = computeAriaLabel(s, {
            healthBadge: badgeCount,
            syncInProgress,
            syncBadge,
          });
          return (
            <ListBoxItem
              key={s.id}
              id={s.id}
              textValue={s.label}
              className={styles.navItem}
              aria-label={ariaLabel}
            >
              {s.id === "sync" && syncInProgress && (
                <SpinnerIcon className={styles.syncSpinner} />
              )}
              <span>{s.label}</span>
              {showHealthBadge && (
                <span className={styles.badge} aria-hidden="true">
                  {badgeCount}
                </span>
              )}
              {showSyncBadge && (
                <span
                  className={
                    syncBadge.kind === "failures"
                      ? styles.badge
                      : styles.badgePending
                  }
                  aria-hidden="true"
                >
                  {syncBadge.count}
                </span>
              )}
            </ListBoxItem>
          );
        })}
      </ListBox>
      <div className={styles.footer}>{footerText}</div>
    </aside>
  );
}

/** UI-SPEC §VoiceOver labels — 4 entries for the Sync NavItem (one per
 *  badge state × spinner state). */
function computeAriaLabel(
  s: { id: Section; label: string },
  ctx: {
    healthBadge: number;
    syncInProgress: boolean;
    syncBadge: SyncBadge;
  },
): string {
  if (s.id === "health" && ctx.healthBadge > 0) {
    return `Health, Health section, ${ctx.healthBadge} health issues`;
  }
  if (s.id === "sync") {
    if (ctx.syncInProgress) {
      return "Sync, Sync section, sync in progress";
    }
    if (ctx.syncBadge.kind === "pending" && ctx.syncBadge.count > 0) {
      return `Sync, Sync section, ${ctx.syncBadge.count} pending decisions`;
    }
    if (ctx.syncBadge.kind === "failures" && ctx.syncBadge.count > 0) {
      return `Sync, Sync section, ${ctx.syncBadge.count} failures`;
    }
    return "Sync, Sync section";
  }
  return `${s.label}, ${s.label} section`;
}

/** Small system-spinner SVG icon — HIG-aligned, rotates via CSS
 *  `animation: spin 1s linear infinite`. Plan 27-01b deliberately uses an
 *  inline SVG (not a `<Spinner>` component import) so the Sidebar stays
 *  the only consumer of this primitive — the real shared spinner lands
 *  alongside the StageStepper in 27-04. */
function SpinnerIcon({ className }: { className?: string }) {
  return (
    <svg
      className={className}
      width="12"
      height="12"
      viewBox="0 0 24 24"
      fill="none"
      aria-hidden="true"
    >
      <circle
        cx="12"
        cy="12"
        r="10"
        stroke="currentColor"
        strokeWidth="2"
        strokeOpacity="0.25"
      />
      <path
        d="M22 12a10 10 0 0 1-10 10"
        stroke="currentColor"
        strokeWidth="2"
        strokeLinecap="round"
      />
    </svg>
  );
}

function isSection(value: string): value is Section {
  return (
    value === "status" ||
    value === "skills" ||
    value === "sync" ||
    value === "health"
  );
}
