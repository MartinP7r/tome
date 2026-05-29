// useFuzzySearch — JS-side fuzzy filter wrapping fuse.js.
//
// **Tradeoff (Assumption A3 in RESEARCH).** fuse.js produces a different
// ranking from the CLI's `nucleo-matcher` — both are "close-enough" fuzzy
// rankers but neither is bit-for-bit identical. The CLI uses nucleo because
// it's small and Rust-native; the GUI uses fuse.js because it's small,
// JS-native, and avoids a per-keystroke Tauri command round-trip (~6-30% of
// the 60fps budget per RESEARCH §"Standard Stack — Fuzzy search"). Ranking
// parity is a known-deferred concern; if beta feedback complains we can
// either (a) ship a thin nucleo-port to JS or (b) accept the divergence
// and document it. For alpha: "close enough".

import { useMemo } from "react";
import Fuse from "fuse.js";

export interface FuzzyOptions<T> {
  /** Properties to match against (e.g. `["name", "source_name"]`). */
  keys: (keyof T & string)[];
  /** Fuse threshold (0 = exact, 1 = match anything). Default 0.4 — slightly
   *  more lenient than fuse's default 0.6 for skill-name typo tolerance. */
  threshold?: number;
}

export function useFuzzySearch<T>(
  items: T[] | null,
  query: string,
  options: FuzzyOptions<T>,
): T[] {
  const fuse = useMemo(() => {
    if (!items) return null;
    return new Fuse<T>(items, {
      keys: options.keys,
      threshold: options.threshold ?? 0.4,
      // Score ascending — best match first. ignoreLocation lets matches
      // anywhere in the string score equally; without it, fuse prefers
      // matches at the start, which over-weights name prefixes.
      ignoreLocation: true,
    });
    // We deliberately rebuild on identity change of `items` — the typical
    // case is a single mount fetch, so the cost is bounded. If 26-06 wires
    // event-driven refetches and we see jank, switch to a key-equality
    // comparison.
  }, [items, options.keys, options.threshold]);

  return useMemo(() => {
    if (!items) return [];
    if (query === "") return items;
    if (!fuse) return [];
    return fuse.search(query).map((r) => r.item);
  }, [items, query, fuse]);
}
