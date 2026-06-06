// App shell — Phase 26 alpha cut + Phase 27 plan 27-01b Sync surface.
//
// The 3-column NavigationSplitView shell (D-01): Window wraps a Titlebar +
// Sidebar + the active view. Lands on Status (D-02). ⌘1 / ⌘2 / ⌘3 / ⌘4 jump
// between Status / Skills / Sync / Health and ⌘F focuses the SearchField —
// all five now dispatched by the **native macOS menu bar**
// (plan 26-07, `crates/tome-desktop/src/menu.rs`) → typed `MenuAction`
// Tauri event → `useMenuActions` hook. Plan 27-01b re-anchored ⌘3 from
// Health to Sync and moved Health to ⌘4 (Pitfall 7).
//
// Plan 27-01b also lifts `useSync` to the App root so the Sidebar's
// spinner + dual-meaning badge slot can subscribe to the in-flight sync
// state without each consumer re-binding the SyncProgress listener.

import { ContentPane } from "./shell/ContentPane";
import { Sidebar } from "./shell/Sidebar";
import { Titlebar, type SectionLabel } from "./shell/Titlebar";
import { Window } from "./shell/Window";
import { useDoctorReport } from "./hooks/useDoctorReport";
import { useMenuActions } from "./hooks/useMenuActions";
import { useSync } from "./hooks/useSync";
import { useRouter, setView, type View } from "./stores/router";
import { HealthView } from "./views/HealthView";
import { SkillsView } from "./views/SkillsView";
import { StatusView } from "./views/StatusView";
import { SyncView } from "./views/SyncView";

function sectionLabel(view: View): SectionLabel {
  switch (view) {
    case "status":
      return "Status";
    case "skills":
      return "Skills";
    case "sync":
      return "Sync";
    case "health":
      return "Health";
  }
}

export default function App() {
  const { view } = useRouter();
  // Plan 26-07 / NF-03 — subscribe to the native menu's `MenuAction`
  // event for the lifetime of the app. Replaces the per-key
  // document-level listener that lived here pre-26-07.
  useMenuActions();
  // Plan 26-05 / D-02 / D-12: live doctor-finding count drives the Sidebar
  // Health badge. Subscribes to manifest + library + lockfile events; the
  // badge clears at zero findings.
  const { report: doctorReport } = useDoctorReport();
  const badgeCount = doctorReport?.findings.length ?? 0;

  // Plan 27-01b — Sidebar Sync NavItem spinner + dual-meaning badge slot.
  // The badge counts are stubs until 27-02 (pendingDecisions) and 27-05
  // (failureCount) wire the real values; the spinner already reflects
  // the live `isRunning` flag.
  const sync = useSync();
  const syncBadge = sync.failureCount > 0
    ? { kind: "failures" as const, count: sync.failureCount }
    : sync.pendingDecisions > 0
      ? { kind: "pending" as const, count: sync.pendingDecisions }
      : { kind: "none" as const };

  const isSkills = view === "skills";

  return (
    <Window mode={isSkills ? "split" : "single"}>
      <Titlebar section={sectionLabel(view)} />
      <Sidebar
        selected={view}
        onChange={setView}
        badgeCount={badgeCount}
        syncInProgress={sync.isRunning}
        syncBadge={syncBadge}
      />
      {isSkills ? (
        <SkillsView />
      ) : view === "status" ? (
        <ContentPane title="Status">
          <StatusView />
        </ContentPane>
      ) : view === "sync" ? (
        <ContentPane title="Sync">
          <SyncView />
        </ContentPane>
      ) : (
        <ContentPane title="Health">
          <HealthView />
        </ContentPane>
      )}
    </Window>
  );
}
