// textInputFocus — shared "is the user typing?" guard.
//
// Phase 26 plan 26-07 introduced this guard inline in SkillsView (the ⌘C
// skill-scoped handler had to abstain when the SearchField had focus so it
// didn't collide with the OS-routed Edit > Copy). Plan 27-01b extracts it
// because useMenuActions now binds two global keys (⌘R / ⌘.) and the same
// abstain-when-typing logic applies.
//
// React Aria nests the actual `<input>` inside a labelled `<div role="
// searchbox">`, so we accept the parent role too. Lives in `lib/` because
// it's pure DOM helper code — no React deps, no Tauri deps.

export function isTextInputFocused(): boolean {
  const el = document.activeElement;
  if (!(el instanceof HTMLElement)) return false;
  const tag = el.tagName;
  if (tag === "INPUT" || tag === "TEXTAREA") return true;
  if (el.isContentEditable) return true;
  const role = el.getAttribute("role");
  return role === "searchbox" || role === "textbox";
}
