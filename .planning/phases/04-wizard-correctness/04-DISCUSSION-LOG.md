# Phase 4: Wizard Correctness - Discussion Log

> **Audit trail only.** Do not use as input to planning, research, or execution agents.
> Decisions are captured in CONTEXT.md — this log preserves the alternatives considered.

**Date:** 2026-04-19
**Phase:** 04-wizard-correctness
**Areas discussed:** Validation location, Overlap semantics, Wizard failure UX, Error message style

---

## Area Selection

**Question:** Which areas do you want to discuss for Phase 4: Wizard Correctness?

| Option | Description | Selected |
|--------|-------------|----------|
| Validation location | Where new path checks live (Config::validate vs wizard-only vs hybrid); whether TOML round-trip is needed. | ✓ |
| Overlap semantics | Which overlap relations to reject; path canonicalization strategy. | ✓ |
| Wizard failure UX | What happens when validation fails at save time (hard exit vs retry). | ✓ |
| Error message style | How actionable error messages should be; whether to upgrade existing errors. | ✓ |

**User's choice:** All four selected.

---

## Validation Location

### Q1: Where should the new path-overlap/circularity checks live?

| Option | Description | Selected |
|--------|-------------|----------|
| In Config::validate() | Load and save share the same rules (symmetry). Hand-edited bad configs fail to load. Introduces I/O into validate(). | ✓ |
| Wizard-only pre-save check | New function `wizard::validate_for_save(&config)`. Config::validate() stays pure. Hand-edited bad configs reach sync silently. | |
| Hybrid: core + extension | Config::validate() keeps existing pure checks; new Config::validate_paths() for filesystem-aware checks. Two-method API. | |

**User's choice:** In Config::validate()
**Notes:** Simplicity and load-symmetry win. User chose the "one rule, everywhere" approach even though it extends `validate()`'s responsibility.

### Q2: How should path-overlap checks handle paths that don't exist yet?

| Option | Description | Selected |
|--------|-------------|----------|
| Lexical comparison only | Compare expanded paths as strings after tilde expansion + normalization. No canonicalization. Misses symlink-based overlaps. | ✓ |
| Canonicalize when possible | Use Path::canonicalize() when paths exist; fall back to lexical. Partial-canonicalization can produce confusing results. | |
| Require existence + canonicalize | Validation fails if paths don't exist. Strictest. Blocks first-time setup flow. | |

**User's choice:** Lexical comparison only
**Notes:** Keeps validate() I/O-free for new checks (existing library_dir is_dir check stays, but no NEW I/O). Supports first-time setup where paths may not exist yet. Explicitly accepts symlink-based overlap miss as a trade-off.

### Q3: Beyond validate(), do we also need a TOML round-trip check before save?

| Option | Description | Selected |
|--------|-------------|----------|
| No — validate() is enough | Trust in-memory Config if validate() passes. Simpler. Misses serde-level drift. | |
| Yes — defense in depth | Serialize → parse → validate again → compare for equality. Bulletproof. Extra CPU per save, PartialEq required. | ✓ |
| Round-trip in tests only | validate() in prod, round-trip as a test assertion (WHARD-05). No prod overhead. Won't catch post-release regressions. | |

**User's choice:** Yes — defense in depth
**Notes:** Wizard save path only. Catches serde-level bugs (e.g. load-bearing field accidentally marked `#[serde(skip_serializing_if)]`) that validate() wouldn't see.

---

## Overlap Semantics

### Q1: Which overlap relations should validate() reject?

| Option | Description | Selected |
|--------|-------------|----------|
| All three (A + B + C) | Reject exact match, library-inside-distro, AND distro-inside-library. Most conservative. | ✓ |
| Only A + B | Reject exact match and library-inside-distro (literal WHARD-02/03 reading). Allow distro-inside-library. | |
| A + B + C with C as warning | Hard error on A/B, warning-only on C. Needs separate warning API surface. | |

**User's choice:** All three (A + B + C)
**Notes:** No ambiguity; all dangerous configs blocked. Cleaner predicate for implementation: `is_overlap(a, b) = equal(a, b) || contains(a, b) || contains(b, a)`.

### Q2: Should distribution-directory-only overlap (two distros nested) also be rejected?

| Option | Description | Selected |
|--------|-------------|----------|
| No — out of scope | WHARD-02/03 only cover library_dir vs distribution. Two nested distribution dirs = separate concern. | ✓ |
| Yes — same function | validate() also checks distro-pair nesting. Comprehensive but scope creep. | |

**User's choice:** No — out of scope
**Notes:** Keeps Phase 4 scoped to WHARD-02/03 as literally specified. If this becomes a real foot-gun, a dedicated phase can handle it.

---

## Wizard Failure UX

### Q1: When validate() fails at the wizard's save step, what should happen?

| Option | Description | Selected |
|--------|-------------|----------|
| Hard error + exit | Print validation error, return Err from wizard::run(), user re-runs tome init. Minimal code; 10+ answers lost on a typo. | ✓ |
| Library-path retry loop | Re-prompt only for library_dir on overlap failure (common case); hard-exit on other errors. Targeted UX fix. | |
| Full retry loop | Offer Select: re-edit library, re-edit a directory, or quit. Best UX; more code. | |

**User's choice:** Hard error + exit
**Notes:** Pragmatic choice. Phase 4 is correctness, not polish. Minimal surface area. User accepts re-run cost in exchange for simpler implementation. If post-release feedback shows this is painful, retry UX can be added later.

### Q2: How should non-interactive mode (--no-input / no TTY) handle validation failure?

| Option | Description | Selected |
|--------|-------------|----------|
| Always hard error | Any validate() failure returns Err immediately in non-TTY mode. | ✓ |
| Same as interactive | Retry loop with default values. Could infinite-loop. | |

**User's choice:** Always hard error
**Notes:** Predictable for CI/tests. Consistent with interactive mode's hard-exit choice.

---

## Error Message Style

### Q1: How actionable should new validation error messages be?

| Option | Description | Selected |
|--------|-------------|----------|
| Name the conflict only | `library_dir '~/x' overlaps with directory 'y' at '~/x'`. Concise, technical. | |
| Conflict + why | Adds explanation of why it's a problem (circular symlinks etc.). Includes D-05 role parenthetical. | |
| Conflict + why + suggestion | Adds concrete alternative (e.g., 'try ~/.tome/skills'). Most helpful. | ✓ |

**User's choice:** Conflict + why + suggestion
**Notes:** With hard-exit UX, the error is the user's only debugging surface. Making errors maximally actionable reduces the re-run-guess-again cycle.

### Q2: Consistency — should we upgrade existing validate() errors too?

| Option | Description | Selected |
|--------|-------------|----------|
| Only upgrade the new ones | New overlap errors use new style; existing errors stay as-is. Inconsistent voice. | |
| Upgrade all validate() errors | Rewrite all existing messages to match new template. Consistent. Snapshot test churn. | ✓ |

**User's choice:** Upgrade all validate() errors
**Notes:** One consistent voice across validate(). Worth the scope expansion beyond WHARD-01..03 literal text.

---

## Closing Check

**Question:** Any gray areas remain unclear, or ready for context?
**User's choice:** Ready for context
**Notes:** All four selected areas covered to user's satisfaction. No additional gray areas surfaced.

---

## Claude's Discretion

- Exact wording of error messages (as long as Conflict + Why + Suggestion template and D-11 role parenthetical are preserved)
- Internal layout of new validation helpers (free function vs method; path-normalization helper location)
- Whether to split Config::validate() into multiple methods
- Where the TOML round-trip helper lives (config.rs vs wizard.rs)
- Whether PartialEq on Config is the right round-trip comparison mechanism vs comparing TOML strings

## Deferred Ideas

- In-wizard retry loop on validation failure (post-v0.7 consideration)
- Canonicalization-based overlap detection (opt-in future method)
- Collect-all-errors validate() API
- Distro-distro overlap detection (dedicated phase if needed)

## Mid-Workflow User Question

**User asked:** "do we already have a way to track standard git repo skills? E.g. can I register a new skill just with the repo URL?"
**Answer given:** Yes — `tome add <url>` exists (shipped v0.6 Phase 3). Registers a git skill repo from URL alone; extracts repo name or uses `--name`; supports `--branch`/`--tag`/`--rev`; config-only (no sync). Works with HTTPS and SSH URLs. Implementation at `crates/tome/src/add.rs`, CLI at `crates/tome/src/cli.rs:56-58`.
