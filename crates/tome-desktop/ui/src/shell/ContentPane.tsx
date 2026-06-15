// ContentPane — right pane wrapper for Status/Health (UI-SPEC §Shell §ContentPane).
//
// Fixed header (view title + optional trailing meta slot — e.g. the
// transient "Updated" pill in Status) above a scrolling body. The Skills
// view bypasses this component because its right-column layout differs
// (list column + detail column).

import type { ReactNode } from "react";
import styles from "./ContentPane.module.css";

export interface ContentPaneProps {
  title: string;
  trailing?: ReactNode;
  children: ReactNode;
}

export function ContentPane({ title, trailing, children }: ContentPaneProps) {
  return (
    <main className={styles.pane} aria-label={title}>
      <div className={styles.header}>
        <h1 className={styles.title}>{title}</h1>
        {trailing && <div className={styles.trailing}>{trailing}</div>}
      </div>
      <div className={styles.body}>{children}</div>
    </main>
  );
}
