// App shell — Phase 26 alpha cut.
//
// The 3-column NavigationSplitView shell (D-01): Window wraps a Titlebar +
// Sidebar + the active view. Lands on Status (D-02). ⌘1 / ⌘2 / ⌘3 jump
// between Status / Skills / Health and ⌘F focuses the SearchField —
// all four now dispatched by the **native macOS menu bar**
// (plan 26-07, `crates/tome-desktop/src/menu.rs`) → typed `MenuAction`
// Tauri event → `useMenuActions` hook. The previous document-level
// keydown listener was removed in plan 26-07 to avoid a double-binding
// conflict with the menu's registered accelerators (Pitfall 9). The
// Skills view's right side hosts a list column + detail column instead
// of a single ContentPane, so we branch on `view === 'skills'` before
// rendering.
//
// Plan 26-05 wires the Sidebar Health badge to `useDoctorReport`'s live
// `findings.length` and replaces the HealthPlaceholder with the real
// HealthView.

import { ContentPane } from "./shell/ContentPane";
import { Sidebar } from "./shell/Sidebar";
import { Titlebar, type SectionLabel } from "./shell/Titlebar";
import { Window } from "./shell/Window";
import { useDoctorReport } from "./hooks/useDoctorReport";
import { useMenuActions } from "./hooks/useMenuActions";
import { useRouter, setView, type View } from "./stores/router";
import { HealthView } from "./views/HealthView";
import { SkillsView } from "./views/SkillsView";
import { StatusView } from "./views/StatusView";

function sectionLabel(view: View): SectionLabel {
  switch (view) {
    case "status":
      return "Status";
    case "skills":
      return "Skills";
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

  const isSkills = view === "skills";

  return (
    <Window mode={isSkills ? "split" : "single"}>
      <Titlebar section={sectionLabel(view)} />
      <Sidebar selected={view} onChange={setView} badgeCount={badgeCount} />
      {isSkills ? (
        <SkillsView />
      ) : view === "status" ? (
        <ContentPane title="Status">
          <StatusView />
        </ContentPane>
      ) : (
        <ContentPane title="Health">
          <HealthView />
        </ContentPane>
      )}
    </Window>
  );
}
