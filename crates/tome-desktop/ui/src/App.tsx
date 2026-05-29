// App shell — Phase 26 alpha cut.
//
// The 3-column NavigationSplitView shell (D-01): Window wraps a Titlebar +
// Sidebar + the active view. Lands on Status (D-02). ⌘1 / ⌘2 / ⌘3 jump
// between Status / Skills / Health (UI-SPEC §Keyboard Map). The Skills
// view's right side hosts a list column + detail column instead of a single
// ContentPane, so we branch on `view === 'skills'` before rendering.
//
// Plan 26-05 wires the Sidebar Health badge to `useDoctorReport`'s live
// `findings.length` and replaces the HealthPlaceholder with the real
// HealthView.

import { useEffect } from "react";
import { ContentPane } from "./shell/ContentPane";
import { Sidebar } from "./shell/Sidebar";
import { Titlebar, type SectionLabel } from "./shell/Titlebar";
import { Window } from "./shell/Window";
import { useDoctorReport } from "./hooks/useDoctorReport";
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

/** ⌘1 / ⌘2 / ⌘3 → setView. Bound at the document level so the shortcuts
 *  work regardless of which child has focus. NF-02 / UI-SPEC §Keyboard Map. */
function useGlobalShortcuts() {
  useEffect(() => {
    const handler = (event: KeyboardEvent) => {
      if (!event.metaKey || event.ctrlKey || event.altKey) return;
      if (event.key === "1") {
        event.preventDefault();
        setView("status");
      } else if (event.key === "2") {
        event.preventDefault();
        setView("skills");
      } else if (event.key === "3") {
        event.preventDefault();
        setView("health");
      }
    };
    window.addEventListener("keydown", handler);
    return () => window.removeEventListener("keydown", handler);
  }, []);
}

export default function App() {
  const { view } = useRouter();
  useGlobalShortcuts();
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
