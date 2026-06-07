// formatDuration — shared duration formatter for StageRow's per-stage
// duration display (UI-SPEC §StageRow §Duration format rule, Phase 27
// plan 27-04 / D-10).
//
// Three buckets:
//   - `< 1000 ms`         → "0.3s"   (1 decimal)
//   - `1000 ms .. 60 s`   → "8.2s"   (1 decimal)
//   - `>= 60 s`           → "1m 14s" (whole seconds)
//
// Numbers right-aligned for vertical scanning. Always returns a string
// — `0 ms` formats as "0.0s" (rather than "0ms") because the UI-SPEC's
// "always one of the three patterns" rule is the right call for the
// stepper's at-a-glance time column.

/** Format a duration in milliseconds per UI-SPEC §StageRow.
 *
 * Negative values are clamped to 0; non-finite (NaN, Infinity) returns
 * an empty string. The format is fixed even when the duration is very
 * small, so a stage that completes in 13ms renders "0.0s".
 */
export function formatDuration(ms: number): string {
  if (!Number.isFinite(ms)) return "";
  const value = Math.max(0, ms);
  if (value < 1000) {
    return `${(value / 1000).toFixed(1)}s`;
  }
  if (value < 60_000) {
    return `${(value / 1000).toFixed(1)}s`;
  }
  const seconds = Math.floor(value / 1000);
  const minutes = Math.floor(seconds / 60);
  const remainderSeconds = seconds % 60;
  return `${minutes}m ${remainderSeconds}s`;
}
