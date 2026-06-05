// HealthView — VIEW-05 (UI-SPEC §Per-view Design — Health).
//
// Three shapes:
//   1. error from `commands.getDoctorReport()` → ErrorBanner (reusing the
//      Phase 25 styles.css `.error-banner` class StatusView already uses).
//   2. zero findings → all-clear state: centred ✓ glyph + "Everything looks
//      healthy" + sub-line. Per UI-SPEC §All-clear health (D-12).
//   3. one or more findings → partition into AUTO-FIXABLE + NEEDS ATTENTION
//      via `repair_kind != null` and render SectionHeader + FindingRow per
//      group.
//
// `onApplyFix` is the Apply-button click handler: dispatches
// `commands.doctorRepairOne(id)`, throws on error so the PreviewPopover →
// FindingRow inline-error chain catches it, calls `refetch()` on success so
// the repaired row drops within ~100ms (without waiting for the watcher
// round-trip).
//
// Per-item only — NO bulk Fix-all button (D-10).

import type { DoctorFinding, FindingId } from "../bindings";
import { commands } from "../bindings";
import { FindingRow } from "../components/FindingRow";
import { SectionHeader } from "../components/SectionHeader";
import { useDoctorReport } from "../hooks/useDoctorReport";
import styles from "./HealthView.module.css";

export function HealthView() {
  const { report, err, refetch } = useDoctorReport();

  if (err) {
    return (
      <div className="error-banner">
        <strong>[{err.code}]</strong> {err.message}
        {err.context.length > 0 && (
          <ul>
            {err.context.map((c, i) => (
              <li key={i}>{c}</li>
            ))}
          </ul>
        )}
      </div>
    );
  }

  if (!report) {
    return <div>Loading…</div>;
  }

  // All-clear: per UI-SPEC §All-clear health (D-12). Centred checkmark,
  // heading, sub-line. role=status so VoiceOver announces it cleanly.
  if (report.findings.length === 0) {
    return (
      <section
        className={styles.allClear}
        role="status"
        aria-label="Library health"
      >
        <svg
          width="32"
          height="32"
          viewBox="0 0 32 32"
          fill="none"
          aria-hidden="true"
          className={styles.checkmark}
        >
          <circle
            cx="16"
            cy="16"
            r="14"
            stroke="var(--success)"
            strokeWidth="2"
            fill="var(--success)"
            fillOpacity="0.12"
          />
          <polyline
            points="10,16 14,20 22,12"
            fill="none"
            stroke="var(--success)"
            strokeWidth="2.5"
            strokeLinecap="round"
            strokeLinejoin="round"
          />
        </svg>
        <h2 className={styles.allClearHeading}>Everything looks healthy</h2>
        <p className={styles.allClearSub}>
          No findings. The library, distribution targets, and manifest are in
          sync.
        </p>
      </section>
    );
  }

  const autoFixable = report.findings.filter(
    (f: DoctorFinding) => f.repair_kind != null,
  );
  const manual = report.findings.filter(
    (f: DoctorFinding) => f.repair_kind == null,
  );

  const onApplyFix = async (id: FindingId): Promise<void> => {
    const res = await commands.doctorRepairOne(id);
    if (res.status === "error") {
      // Propagate to the PreviewPopover → FindingRow inline-error sink.
      throw res.error;
    }
    // Successful fix — refetch immediately for instant UI feedback. The
    // watcher (plan 26-06) will refetch a second time when its debounced
    // event fires; both are idempotent.
    await refetch();
  };

  return (
    <>
      {autoFixable.length > 0 && (
        <>
          <SectionHeader label="AUTO-FIXABLE" count={autoFixable.length} />
          <div>
            {autoFixable.map((f: DoctorFinding) => (
              <FindingRow
                key={`af-${findingIdKey(f.id)}`}
                finding={f}
                onApplyFix={onApplyFix}
              />
            ))}
          </div>
        </>
      )}
      {manual.length > 0 && (
        <>
          <SectionHeader label="NEEDS ATTENTION" count={manual.length} />
          <div>
            {manual.map((f: DoctorFinding) => (
              <FindingRow
                key={`m-${findingIdKey(f.id)}`}
                finding={f}
                onApplyFix={onApplyFix}
              />
            ))}
          </div>
        </>
      )}
    </>
  );
}

/** Stable React key derived from the tagged-union FindingId. */
function findingIdKey(id: FindingId): string {
  switch (id.kind) {
    case "library_stale_manifest":
      return `${id.kind}:${id.skill}`;
    case "library_broken_symlink":
      return `${id.kind}:${id.path}`;
    case "target_stale_symlink":
      return `${id.kind}:${id.directory}:${id.path}`;
    case "target_real_dir_to_symlink":
      return `${id.kind}:${id.directory}:${id.path}`;
    case "unparsable_frontmatter":
      return `${id.kind}:${id.skill}`;
    case "diverging_target":
      return `${id.kind}:${id.directory}:${id.path}`;
  }
}
