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

/** Heuristic to derive `tome_home` from `library_dir` for the TOME HOME row.
 *
 * `StatusReport` does NOT (yet) carry `tome_home` — the canonical Rust path
 * is `paths::tome_home()`. The UI-SPEC asks for a TOME HOME row, so until
 * a future StatusReport extension surfaces it, derive it from the library
 * dir's parent (which is the conventional `~/.tome/` layout). This is a
 * display-only fallback; D-GUI-08 still holds because no business logic
 * depends on it.
 */
function deriveTomeHome(libraryDir: string): string {
  // strip a trailing /library segment if present.
  const stripped = libraryDir.replace(/\/library\/?$/, "");
  return stripped === libraryDir ? libraryDir : stripped;
}

export function StatusView() {
  const { status, err, updatedAt } = useStatus();

  if (err) {
    // Error banner — matches Phase 25 App.tsx shape.
    return (
      <div className="app">
        <h1>tome</h1>
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
      </div>
    );
  }

  if (!status) {
    return <div className="app">Loading…</div>;
  }

  const tomeHome = deriveTomeHome(status.library_dir);
  const showUpdatedPill =
    updatedAt !== null && Date.now() - updatedAt < 2000;
  const lockfileOk = status.lockfile.kind === "in_sync";

  return (
    <div className="app">
      <h1>Status</h1>

      <section>
        <KeyValueRow label="TOME HOME" value={tomeHome} mono />
        <KeyValueRow
          label="LIBRARY"
          value={status.library_dir}
          mono
          trailing={<span>{formatSkillCount(status)}</span>}
        />
        <KeyValueRow
          label="LAST SYNC"
          value={formatRelative(status.last_sync)}
          trailing={showUpdatedPill ? <Pill variant="updated">Updated</Pill> : null}
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
    </div>
  );
}
