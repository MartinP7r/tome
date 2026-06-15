// DirectoryTable — Status view section (UI-SPEC §Molecules — DirectoryTable).
//
// Columns: NAME (primary + secondary path line) / ROLE (Badge) / TYPE (Badge).
// No row interaction in Phase 26 — read-only.

import type { DirectoryStatus_Serialize, DirectoryRole } from "../bindings";
import { Badge, type BadgeSubtype } from "./Badge";
import styles from "./DirectoryTable.module.css";

export interface DirectoryTableProps {
  directories: ReadonlyArray<DirectoryStatus_Serialize>;
}

/** Map `DirectoryRole` → role badge subtype.
 *
 * `synced` and `source` both expose skills to discovery; `target` is
 * distribution-only; `managed` gets its own subtype to surface the
 * package-manager-owned semantic.
 */
function roleSubtype(role: DirectoryRole): BadgeSubtype {
  switch (role) {
    case "managed":
      return "role-managed";
    case "synced":
    case "source":
      return "role-discovery";
    case "target":
      return "role-distribution";
  }
}

/** Map `directory_type` string → type badge subtype.
 *
 * The Rust side ships `claude-plugins` / `git` / `directory` (string).
 * Anything outside the known set falls back to the neutral `type-directory`
 * styling so a future variant doesn't render unstyled.
 */
function typeSubtype(directoryType: string): BadgeSubtype {
  switch (directoryType) {
    case "claude-plugins":
      return "type-claude-plugins";
    case "git":
      return "type-git";
    case "directory":
      return "type-directory";
    default:
      return "type-directory";
  }
}

export function DirectoryTable({ directories }: DirectoryTableProps) {
  if (directories.length === 0) {
    return <div className={styles.empty}>(none configured)</div>;
  }

  return (
    <div className={styles.wrapper}>
      <table className={styles.table}>
        <thead>
          <tr>
            <th scope="col" className={styles.th}>
              NAME
            </th>
            <th scope="col" className={styles.th}>
              ROLE
            </th>
            <th scope="col" className={styles.th}>
              TYPE
            </th>
          </tr>
        </thead>
        <tbody>
          {directories.map((d) => (
            <tr key={d.name} className={styles.tr}>
              <td className={styles.td}>
                <div className={styles.name}>{d.name}</div>
                <div className={styles.path}>{d.path}</div>
              </td>
              <td className={styles.td}>
                <Badge subtype={roleSubtype(d.role)}>{d.role}</Badge>
              </td>
              <td className={styles.td}>
                <Badge subtype={typeSubtype(d.directory_type)}>
                  {d.directory_type}
                </Badge>
              </td>
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  );
}
