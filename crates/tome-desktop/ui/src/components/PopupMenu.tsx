// PopupMenu — wraps React Aria MenuTrigger + Menu (UI-SPEC §Atoms §PopupMenu).
//
// Closed state: button with the currently-selected item's label + a
// chevron-down glyph. Open state: anchored Popover with a vertical Menu.
// Used for the Sort and Group toolbars in SkillsView.

import {
  Button,
  Menu,
  MenuItem,
  MenuTrigger,
  Popover,
} from "react-aria-components";
import styles from "./PopupMenu.module.css";

export interface PopupMenuItem {
  id: string;
  label: string;
}

export interface PopupMenuProps {
  /** Visible button-prefix text (e.g. "Sort"). The current value is appended. */
  label: string;
  ariaLabel: string;
  items: PopupMenuItem[];
  selectedId: string;
  onChange: (id: string) => void;
}

export function PopupMenu({
  label,
  ariaLabel,
  items,
  selectedId,
  onChange,
}: PopupMenuProps) {
  const selected = items.find((i) => i.id === selectedId);
  const displayLabel = selected ? `${label}: ${selected.label}` : label;

  return (
    <MenuTrigger>
      <Button className={styles.trigger} aria-label={ariaLabel}>
        <span>{displayLabel}</span>
        <ChevronDown />
      </Button>
      <Popover className={styles.popover}>
        <Menu
          className={styles.menu}
          aria-label={ariaLabel}
          selectionMode="single"
          selectedKeys={new Set([selectedId])}
          onAction={(key) => {
            if (typeof key === "string") onChange(key);
          }}
        >
          {items.map((i) => (
            <MenuItem key={i.id} id={i.id} className={styles.item}>
              {i.label}
            </MenuItem>
          ))}
        </Menu>
      </Popover>
    </MenuTrigger>
  );
}

function ChevronDown() {
  return (
    <svg
      className={styles.chevron}
      viewBox="0 0 10 10"
      fill="none"
      stroke="currentColor"
      strokeWidth="1.5"
      aria-hidden="true"
    >
      <polyline points="2,4 5,7 8,4" strokeLinecap="round" strokeLinejoin="round" />
    </svg>
  );
}
