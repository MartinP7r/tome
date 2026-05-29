// Window — root container for the 3-column NavigationSplitView shell (D-01).
//
// Renders the Tauri unified titlebar (OS-owned traffic lights) above a body
// region that switches between a 2-column "single" layout (Status/Health —
// sidebar + content pane) and a 3-column "split" layout (Skills — sidebar +
// list column + detail column). Inherited by every Phase 26 view and by the
// Phases 27–31 views still to come.
//
// Layout intent matches UI-SPEC §Shell §Window:
//   grid-template-columns: 210px minmax(280px, 380px) 1fr   (split)
//   grid-template-columns: 210px 1fr                        (single)

import type { ReactNode } from "react";
import styles from "./Window.module.css";

export interface WindowProps {
  /** Single-pane (Status/Health) vs split-pane (Skills). */
  mode: "single" | "split";
  children: ReactNode;
}

export function Window({ mode, children }: WindowProps) {
  const bodyClass = `${styles.body} ${
    mode === "split" ? styles.split : styles.single
  }`;
  return (
    // `<main>` is the page-level landmark for the shell. The titlebar
    // owns `role="banner"` and the content pane owns its own `<main>` (we
    // use a generic `<div>` here to avoid duplicate landmarks).
    <div className={styles.window}>{splitChildren(children, bodyClass)}</div>
  );
}

/**
 * The `Window` accepts four conceptual slots — `<Titlebar />`, `<Sidebar />`,
 * and either one (single) or two (split) content pane children. Rather than
 * force callers to wrap the body children, this helper drops the first child
 * (the titlebar) at the top and groups everything else into the body grid.
 *
 * This keeps the JSX in `App.tsx` declarative:
 *
 *   <Window mode="split">
 *     <Titlebar ... />
 *     <Sidebar ... />
 *     <ContentPane ... />  {/* OR list+detail columns *\/}
 *   </Window>
 */
function splitChildren(children: ReactNode, bodyClass: string): ReactNode {
  const arr = Array.isArray(children) ? children : [children];
  const [titlebar, ...rest] = arr;
  return (
    <>
      {titlebar}
      <div className={bodyClass}>{rest}</div>
    </>
  );
}
