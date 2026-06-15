// DetailHeader — UI-SPEC §Per-view Design §Skills, Component Contract §
// "DetailHeader". The compact metadata header that sits above the (future)
// MarkdownBody (plan 26-04).
//
// Three rows top-to-bottom (UI-SPEC §DetailHeader):
//   1. Skill name (--text-title 22px / 600) + trailing badges
//      (Badge--managed, Badge--disabled when applicable).
//   2. Metadata grid: SOURCE / HASH / SYNC. Mono path + truncated hash +
//      relative time. Labels --text-small 12px / 600 uppercase.
//   3. Action buttons row: [Open source folder] [Copy path] [Disable on
//      this machine] — primary is the lone Phase-26 mutation (D-06); the
//      other two are secondary.
//
// The Copy button label flips to "Copied" for 2s after a successful click
// (D-07). The Disable button flips to "Enable on this machine" + Badge--
// disabled appears when `detail.disabled` is true.

import type { SkillDetail } from "../bindings";
import {
  copyPathLabel,
  disableSkillLabel,
  enableSkillLabel,
  openSourceLabel,
} from "../lib/ariaLabels";
import { formatRelative } from "../lib/relativeTime";
import { Badge } from "./Badge";
import { Button } from "./Button";
import styles from "./DetailHeader.module.css";

export type CopyState = "idle" | "copied";

export interface DetailHeaderProps {
  detail: SkillDetail;
  onOpenSource: () => void;
  onCopyPath: () => void;
  onDisableToggle: () => void;
  /** "copied" briefly after a successful copy; the parent owns the timer. */
  copyState: CopyState;
}

/** Middle-truncate a path so the start and end stay visible. */
function truncatePath(p: string, max = 56): string {
  if (p.length <= max) return p;
  const head = Math.ceil((max - 1) / 2);
  const tail = Math.floor((max - 1) / 2);
  return `${p.slice(0, head)}…${p.slice(p.length - tail)}`;
}

/** `sha256:abc12345…` — first 8 hex chars + ellipsis. */
function truncateHash(h: string): string {
  return `sha256:${h.slice(0, 8)}…`;
}

export function DetailHeader({
  detail,
  onOpenSource,
  onCopyPath,
  onDisableToggle,
  copyState,
}: DetailHeaderProps) {
  const skillName = detail.name;
  const sourcePath = detail.source_path;
  const hash = detail.content_hash;
  const lastSync = detail.last_sync;
  const managed = detail.managed;
  const disabled = detail.disabled;

  const copyLabel = copyState === "copied" ? "Copied" : "Copy path";
  const disableLabel = disabled ? "Enable on this machine" : "Disable on this machine";
  const disableAria = disabled
    ? enableSkillLabel(skillName)
    : disableSkillLabel(skillName);

  return (
    <header
      className={styles.header}
      aria-label={`${skillName} details`}
    >
      {/* Row 1: name + badges */}
      <div className={styles.titleRow}>
        <h2 className={styles.title}>{skillName}</h2>
        <div className={styles.badges}>
          {managed && <Badge subtype="managed">Managed</Badge>}
          {disabled && <Badge subtype="disabled">Disabled</Badge>}
        </div>
      </div>

      {/* Row 2: metadata grid */}
      <dl className={styles.metaGrid}>
        <div className={styles.metaCell}>
          <dt className={styles.metaLabel}>SOURCE</dt>
          <dd className={styles.metaValueMono} title={sourcePath}>
            {truncatePath(sourcePath)}
          </dd>
        </div>
        <div className={styles.metaCell}>
          <dt className={styles.metaLabel}>HASH</dt>
          <dd className={styles.metaValueMono} title={hash}>
            {truncateHash(hash)}
          </dd>
        </div>
        <div className={styles.metaCell}>
          <dt className={styles.metaLabel}>SYNC</dt>
          <dd className={styles.metaValue}>{formatRelative(lastSync)}</dd>
        </div>
      </dl>

      {/* Row 3: action buttons */}
      <div className={styles.actions}>
        <Button
          variant="secondary"
          onPress={onOpenSource}
          ariaLabel={openSourceLabel(skillName)}
        >
          Open source folder
        </Button>
        <Button
          variant="secondary"
          onPress={onCopyPath}
          ariaLabel={copyPathLabel(skillName)}
        >
          {copyLabel}
        </Button>
        <Button
          variant="primary"
          onPress={onDisableToggle}
          ariaLabel={disableAria}
        >
          {disableLabel}
        </Button>
      </div>
    </header>
  );
}
