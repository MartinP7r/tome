// SkillListRow — middle-column row in the Skills view (UI-SPEC §Molecules §SkillListRow).
//
// Two-line: primary name + secondary "source · managed|local". Fixed 52px
// height (UI-SPEC anchor) — both lines clip with ellipsis to honour Pitfall 8
// from RESEARCH §"Pattern 8".
//
// Rendered INSIDE a `<ListBoxItem>` parent (the parent owns selection state +
// React Aria's data-attributes). This component is a presentation-only child
// — it reads `data-selected="true"` off its parent via the `[data-selected]`
// CSS selector on the row. To keep the JSX small, we render a single `<div>`
// here and let the parent ListBoxItem own the row's interactive semantics
// (Virtualizer + ListBox automatically render the data attributes on the
// ListBoxItem; our CSS targets a `.row` class on the inner div, so we use
// CSS variables / parent data-attributes for the selected/focus styles).

import type { DiscoveredSkill } from "../bindings";
import { Badge } from "./Badge";
import styles from "./SkillListRow.module.css";

export interface SkillListRowProps {
  skill: DiscoveredSkill;
  disabled?: boolean;
  /** When true, the row paints accent fill + 600 weight (UI-SPEC §SkillListRow).
   *  Forwarded from the parent ListBoxItem render-state. */
  selected?: boolean;
}

export function SkillListRow({ skill, disabled = false, selected = false }: SkillListRowProps) {
  const managed = skill.origin.kind === "managed";
  const sourceDisplay = skill.source_name;
  const secondary = `${sourceDisplay} · ${managed ? "managed" : "local"}`;
  return (
    <div className={styles.row} data-selected={selected ? "true" : undefined}>
      <div className={styles.text}>
        <span className={styles.primary}>{skill.name}</span>
        <span className={styles.secondary}>{secondary}</span>
      </div>
      {disabled && (
        <span className={styles.trailing}>
          <Badge subtype="disabled">Disabled</Badge>
        </span>
      )}
    </div>
  );
}
