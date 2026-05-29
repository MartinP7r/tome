// SeverityIcon — atom (UI-SPEC §Atoms §SeverityIcon).
//
// 16×16 SVG glyph rendered at the start of every `FindingRow`. Two states:
//   - warning: yellow ⚠ for auto-fixable findings (--danger fill).
//   - blocked: grey ⛔ for non-fixable findings (--label-secondary fill).
//
// The icon is purely decorative — VoiceOver reads severity from the parent
// `FindingRow`'s `aria-label` ("Warning finding: …" / "Blocked finding: …")
// so the SVG is `aria-hidden="true"`. This matches the StatusDot pattern
// established in plan 26-01.

export type Severity = "warning" | "blocked";

export interface SeverityIconProps {
  severity: Severity;
}

export function SeverityIcon({ severity }: SeverityIconProps) {
  if (severity === "warning") {
    return (
      <svg
        width="16"
        height="16"
        viewBox="0 0 16 16"
        fill="none"
        aria-hidden="true"
      >
        {/* Triangle outline + exclamation mark, sized for the FindingRow
         *  16×16 atom. Stroke + fill use --danger so the glyph reads as a
         *  hazard. */}
        <path
          d="M8 1.5 L15 14 L1 14 Z"
          stroke="var(--danger)"
          strokeWidth="1.4"
          strokeLinejoin="round"
          fill="var(--danger)"
          fillOpacity="0.12"
        />
        <line
          x1="8"
          y1="6"
          x2="8"
          y2="10"
          stroke="var(--danger)"
          strokeWidth="1.6"
          strokeLinecap="round"
        />
        <circle cx="8" cy="12" r="0.9" fill="var(--danger)" />
      </svg>
    );
  }
  return (
    <svg
      width="16"
      height="16"
      viewBox="0 0 16 16"
      fill="none"
      aria-hidden="true"
    >
      {/* Solid circle with a diagonal slash for the "blocked / requires
       *  manual action" glyph. Uses --label-secondary so it reads as
       *  informational rather than alarming. */}
      <circle
        cx="8"
        cy="8"
        r="6.5"
        stroke="var(--label-secondary)"
        strokeWidth="1.4"
        fill="var(--label-secondary)"
        fillOpacity="0.12"
      />
      <line
        x1="3.6"
        y1="3.6"
        x2="12.4"
        y2="12.4"
        stroke="var(--label-secondary)"
        strokeWidth="1.6"
        strokeLinecap="round"
      />
    </svg>
  );
}
