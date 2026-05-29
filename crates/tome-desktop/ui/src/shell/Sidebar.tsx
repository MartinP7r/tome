// Sidebar — vibrancy/translucent nav rail (UI-SPEC §Shell §Sidebar + §NavItem).
//
// Three nav items (Status / Skills / Health) inside a React Aria ListBox so
// arrow-key navigation comes for free (NF-02). The Health item shows a red
// circle badge when `badgeCount > 0` (D-02). The footer reads the live skill
// count from `useStatus` so the user always sees an accurate total.
//
// Plan 26-05: the optional `badgeCount` prop is now backed by
// `useDoctorReport().report?.findings.length`, supplied by App.tsx. The
// badge clears at zero findings (D-12).

import { ListBox, ListBoxItem } from "react-aria-components";
import { useStatus } from "../hooks/useStatus";
import styles from "./Sidebar.module.css";

export type Section = "status" | "skills" | "health";

const SECTIONS: { id: Section; label: string }[] = [
  { id: "status", label: "Status" },
  { id: "skills", label: "Skills" },
  { id: "health", label: "Health" },
];

export interface SidebarProps {
  selected: Section;
  onChange: (section: Section) => void;
  /** Count surfaced next to the Health nav item (D-02). */
  badgeCount?: number;
}

export function Sidebar({ selected, onChange, badgeCount = 0 }: SidebarProps) {
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
          const showBadge = s.id === "health" && badgeCount > 0;
          const ariaLabel =
            s.id === "health" && badgeCount > 0
              ? `Health, Health section, ${badgeCount} health issues`
              : `${s.label}, ${s.label} section`;
          return (
            <ListBoxItem
              key={s.id}
              id={s.id}
              textValue={s.label}
              className={styles.navItem}
              aria-label={ariaLabel}
            >
              <span>{s.label}</span>
              {showBadge && (
                <span className={styles.badge} aria-hidden="true">
                  {badgeCount}
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

function isSection(value: string): value is Section {
  return value === "status" || value === "skills" || value === "health";
}
