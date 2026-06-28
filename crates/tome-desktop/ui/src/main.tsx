import React from "react";
import ReactDOM from "react-dom/client";
import App from "./App";
import { SyncProvider } from "./hooks/useSync";
import "./styles.css";

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode>
    {/* Phase 27 plan 27-01b — single shared sync state machine. App
     *  (Sidebar spinner / badge), useMenuActions (global ⌘R), and
     *  SyncView all consume `useSync()` from this provider so they
     *  observe the SAME in-flight run + the SAME syncProgress event
     *  listener. */}
    <SyncProvider>
      <App />
    </SyncProvider>
  </React.StrictMode>,
);
