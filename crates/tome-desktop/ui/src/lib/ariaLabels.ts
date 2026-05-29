// Aria-label template helpers (UI-SPEC §VoiceOver labels).
//
// Pure string functions producing the verbatim labels the spec calls out for
// the DetailHeader action triplet and the Skills list row. Keeping these out
// of the components ensures the spec-fixed shapes can be assert-pinned in
// snapshot tests without re-rendering a component (plan 26-07 audit).
//
// Pin: these templates MUST match UI-SPEC §VoiceOver labels exactly. Any
// edit goes through the spec first.

/** `aria-label` for the detail-header "Open source folder" button. */
export function openSourceLabel(skillName: string): string {
  return `Open source folder for ${skillName} in Finder`;
}

/** `aria-label` for the detail-header "Copy path" button. */
export function copyPathLabel(skillName: string): string {
  return `Copy source path for ${skillName} to clipboard`;
}

/** `aria-label` for the detail-header "Disable on this machine" button. */
export function disableSkillLabel(skillName: string): string {
  return `Disable ${skillName} on this machine`;
}

/** `aria-label` for the detail-header "Enable on this machine" button. */
export function enableSkillLabel(skillName: string): string {
  return `Enable ${skillName} on this machine`;
}

/** Per-row VoiceOver label for a SkillListRow. Mirrors UI-SPEC §SkillListRow. */
export function skillRowLabel(args: {
  name: string;
  sourceName: string;
  managed: boolean;
  disabled?: boolean;
}): string {
  const { name, sourceName, managed, disabled = false } = args;
  const base = `${name}, source ${sourceName}, ${managed ? "managed" : "local"}`;
  return disabled ? `${base}, disabled on this machine` : base;
}

/** Announcement read by VoiceOver after a successful Disable click (D-06). */
export function disableSuccessAnnouncement(skillName: string): string {
  return `Disabled ${skillName} on this machine.`;
}

/** Announcement read after a successful Enable click. */
export function enableSuccessAnnouncement(skillName: string): string {
  return `Enabled ${skillName} on this machine.`;
}
