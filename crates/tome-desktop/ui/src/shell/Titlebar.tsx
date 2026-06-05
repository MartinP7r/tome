// Titlebar — unified native macOS titlebar (UI-SPEC §Shell §Titlebar).
//
// Tauri 2's `titleBarStyle: "Overlay"` + `hiddenTitle: true` puts the OS
// traffic lights on top of the window content; this component fills the
// 44px-tall slot with a centred section label. No JS code owns the traffic
// lights — they're rendered by macOS.

import styles from "./Titlebar.module.css";

export type SectionLabel = "Status" | "Skills" | "Health";

export interface TitlebarProps {
  section: SectionLabel;
}

export function Titlebar({ section }: TitlebarProps) {
  return (
    <header
      className={styles.titlebar}
      role="banner"
      aria-label={`tome ${section}`}
    >
      <span className={styles.title}>tome — {section}</span>
    </header>
  );
}
