// SkillContextMenu — UI-SPEC §SkillListRow + §"Per-view Design — Skills"
// right-click contract (D-07).
//
// Right-click any list row → present the same three actions the DetailHeader
// surfaces: Open source folder, Copy path, Disable on this machine (label
// flips to Enable when the skill is currently disabled).
//
// Implementation note: React Aria's `MenuTrigger` doesn't ship a
// `contextmenu`-only trigger primitive yet — we bind an `onContextMenu`
// handler on the wrapper, suppress the native browser menu, and pop a Tauri
// `Menu` via `Menu` + `MenuTrigger` programmatically. The menu items use
// React Aria's `Menu` / `MenuItem` for keyboard semantics.

import { useState, type ReactNode } from "react";
import { Menu, MenuItem, MenuTrigger, Popover } from "react-aria-components";
import styles from "./SkillContextMenu.module.css";

export type SkillContextAction = "open" | "copy" | "toggle-disable";

export interface SkillContextMenuProps {
  /** Render the wrapped list row; right-click on it opens the menu. */
  children: ReactNode;
  /** Whether the skill is currently disabled (drives the toggle item label). */
  disabled: boolean;
  /** Dispatches the chosen action; the parent owns the handlers. */
  onAction: (action: SkillContextAction) => void;
}

export function SkillContextMenu({
  children,
  disabled,
  onAction,
}: SkillContextMenuProps) {
  // React Aria's controlled `<MenuTrigger isOpen onOpenChange>` pairs with our
  // own `onContextMenu` handler so the menu lives entirely off the right-click
  // event without forcing a separate trigger button.
  const [isOpen, setOpen] = useState(false);

  return (
    <MenuTrigger isOpen={isOpen} onOpenChange={setOpen}>
      <div
        className={styles.wrapper}
        onContextMenu={(e) => {
          e.preventDefault();
          setOpen(true);
        }}
      >
        {children}
      </div>
      <Popover className={styles.popover}>
        <Menu
          className={styles.menu}
          aria-label="Skill actions"
          onAction={(key) => {
            onAction(key as SkillContextAction);
            setOpen(false);
          }}
        >
          <MenuItem id="open" className={styles.item}>
            Open source folder
          </MenuItem>
          <MenuItem id="copy" className={styles.item}>
            Copy path
          </MenuItem>
          <MenuItem id="toggle-disable" className={styles.item}>
            {disabled ? "Enable on this machine" : "Disable on this machine"}
          </MenuItem>
        </Menu>
      </Popover>
    </MenuTrigger>
  );
}
