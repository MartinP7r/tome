// StatusView — VIEW-01 (UI-SPEC §Per-view Design — Status).
//
// Renders every StatusReport field via KeyValueRow + DirectoryTable atoms.
// Lands the app on Status per D-02. No client-side business logic beyond the
// relative-time formatter on `last_sync` (D-GUI-08). Event-driven refresh
// lands in plan 26-06 — this plan only ships the read-once render.

import type { LockfileState, StatusReport_Serialize } from "../bindings";
import { DirectoryTable } from "../components/DirectoryTable";
import { KeyValueRow } from "../components/KeyValueRow";
import { Pill } from "../components/Pill";
import { StatusDot } from "../components/StatusDot";
import { useStatus } from "../hooks/useStatus";
import { formatRelative } from "../lib/relativeTime";

/** Status — Lockfile copy per UI-SPEC §Copywriting. */
function lockfileLabel(state: LockfileState): string {
  switch (state.kind) {
    case "in_sync":
      return "In sync";
    case "out_of_sync":
      // The Status view's MACHINE/LOCKFILE rows reflect the bracketed
      // copywriting from UI-SPEC §Copywriting. drift_count is shown so the
      // user can grok "how out of sync".
      return `Out of sync (${state.drift_count} drift)`;
    case "missing":
      return "Never";
  }
}

function formatSkillCount(report: StatusReport_Serialize): string {
  const { library_count } = report;
  if (library_count.error) return library_count.error;
  if (library_count.count === null) return "—";
  return `${library_count.count} skills`;
}

export function StatusView() {
  const { status, err, updatedAt } = useStatus();

  if (err) {
    // Error banner — matches Phase 25 App.tsx shape. Rendered as a fragment so
    // the surrounding ContentPane owns the outer page chrome (plan 26-02
    // Task 1 wired the shell — see App.tsx).
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

  if (!status) {
    return <div>Loading…</div>;
  }

  const showUpdatedPill =
    updatedAt !== null && Date.now() - updatedAt < 2000;
  const lockfileOk = status.lockfile.kind === "in_sync";

  return (
    <>
      <section>
        <KeyValueRow
          label="TOME DATA FOLDER"
          value={status.tome_home}
          description="Portable Tome data; machine settings live in ~/.config/tome."
          mono
        />
        <KeyValueRow
          label="LIBRARY"
          value={status.library_dir}
          mono
          trailing={<span>{formatSkillCount(status)}</span>}
        />
        <KeyValueRow
          label="LAST SYNC"
          value={formatRelative(status.last_sync)}
          trailing={
            showUpdatedPill ? (
              // `key={updatedAt}` force-remounts the Pill on every
              // watcher-driven refetch (plan 26-06) so the CSS fade
              // animation restarts cleanly even when refetches arrive
              // in rapid succession.
              <Pill key={updatedAt} variant="updated">
                Updated
              </Pill>
            ) : null
          }
        />
        <KeyValueRow
          label="LOCKFILE"
          value={lockfileLabel(status.lockfile)}
          trailing={<StatusDot ok={lockfileOk} />}
        />
        <KeyValueRow
          label="MACHINE"
          value={`${status.machine_prefs_summary.disabled_count} skills disabled`}
        />
      </section>

      <section>
        <h2>Directories ({status.directories.length})</h2>
        <DirectoryTable directories={status.directories} />
      </section>
    </>
  );
}
