// Relative-time formatter for the Status view's "Last sync" field (VIEW-01).
//
// The single display-only computation permitted by D-GUI-08: takes the
// RFC-3339 timestamp the Rust side ships in StatusReport.last_sync and turns
// it into a human-readable label. No domain logic, no rounding policy beyond
// the brackets listed below. Pure function — easy to unit-test, no deps.

/**
 * Render an RFC-3339 timestamp (or null) as a relative-time label.
 *
 * Brackets (all clamped to the user's local clock — no TZ awareness beyond
 * what `Date` provides):
 * - null → "Never"
 * - < 5s ago → "Just now"
 * - < 60s ago → "{N} seconds ago"
 * - < 60min ago → "{N} minutes ago"
 * - same calendar day → "Today at {h:mm AM/PM}"
 * - same calendar week → "{weekday} at {h:mm AM/PM}"
 * - older → the verbatim RFC-3339 string (the Status view shows the raw
 *   timestamp; D-13 wants HIG-aligned but not a full calendar widget).
 */
export function formatRelative(date: string | null): string {
  if (date === null) return "Never";

  const parsed = new Date(date);
  if (Number.isNaN(parsed.getTime())) {
    // Bad input — fall back to the raw string so the value is not silently
    // hidden. The Rust side controls this field, so this branch is defensive.
    return date;
  }

  const now = new Date();
  const diffMs = now.getTime() - parsed.getTime();
  const diffSec = Math.floor(diffMs / 1000);

  if (diffSec < 5) return "Just now";
  if (diffSec < 60) return `${diffSec} seconds ago`;

  const diffMin = Math.floor(diffSec / 60);
  if (diffMin < 60) return `${diffMin} minutes ago`;

  // Within today (same calendar day).
  const sameDay =
    parsed.getFullYear() === now.getFullYear() &&
    parsed.getMonth() === now.getMonth() &&
    parsed.getDate() === now.getDate();
  if (sameDay) return `Today at ${formatClock(parsed)}`;

  // Within the last 6 days — use weekday name.
  const dayDiff = Math.floor(diffMs / (24 * 60 * 60 * 1000));
  if (dayDiff < 7) {
    const weekday = parsed.toLocaleDateString(undefined, { weekday: "long" });
    return `${weekday} at ${formatClock(parsed)}`;
  }

  // Older — fall back to the raw RFC-3339 timestamp.
  return date;
}

function formatClock(d: Date): string {
  const h24 = d.getHours();
  const m = d.getMinutes();
  const period = h24 >= 12 ? "PM" : "AM";
  const h12 = h24 % 12 === 0 ? 12 : h24 % 12;
  const mm = m.toString().padStart(2, "0");
  return `${h12}:${mm} ${period}`;
}
