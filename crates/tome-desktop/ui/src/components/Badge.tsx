// Badge — atom (UI-SPEC §Atoms — Badge).
//
// Subtype → token mapping table. Weight per subtype:
//   - managed / disabled → 600 (emphasis: provenance + state)
//   - everything else → 400 (informational tints)

import type { ReactNode } from "react";
import styles from "./Badge.module.css";

export type BadgeSubtype =
  | "role-discovery"
  | "role-distribution"
  | "role-managed"
  | "type-claude-plugins"
  | "type-git"
  | "type-directory"
  | "managed"
  | "disabled"
  | "override";

export interface BadgeProps {
  subtype: BadgeSubtype;
  children: ReactNode;
}

const SUBTYPE_CLASS: Record<BadgeSubtype, string> = {
  "role-discovery": styles.roleDiscovery,
  "role-distribution": styles.roleDistribution,
  "role-managed": styles.roleManaged,
  "type-claude-plugins": styles.typeClaudePlugins,
  "type-git": styles.typeGit,
  "type-directory": styles.typeDirectory,
  managed: styles.managed,
  disabled: styles.disabled,
  override: styles.override,
};

const WEIGHT_600 = new Set<BadgeSubtype>(["managed", "disabled"]);

export function Badge({ subtype, children }: BadgeProps) {
  const classes = [styles.badge, SUBTYPE_CLASS[subtype]];
  if (WEIGHT_600.has(subtype)) classes.push(styles.weight600);
  return <span className={classes.join(" ")}>{children}</span>;
}
