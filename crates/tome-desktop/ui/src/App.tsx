// App shell — Phase 26 alpha cut.
//
// Per D-02 / VIEW-01, the app lands on the Status view. The Window /
// Titlebar / Sidebar / ContentPane shell is introduced by plan 26-02; until
// then App.tsx is a thin wrapper that renders <StatusView /> directly.

import { StatusView } from "./views/StatusView";

export default function App() {
  return <StatusView />;
}
