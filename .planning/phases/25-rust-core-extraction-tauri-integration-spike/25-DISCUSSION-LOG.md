# Phase 25: Rust core extraction + Tauri integration spike - Discussion Log

> **Audit trail only.** Do not use as input to planning, research, or execution agents.
> Decisions are captured in CONTEXT.md — this log preserves the alternatives considered.

**Date:** 2026-05-24
**Phase:** 25-rust-core-extraction-tauri-integration-spike
**Areas discussed:** Spike scope — frameworks, Structured-type home + specta gating, Progress-event abstraction, TomeError taxonomy granularity

---

## Spike scope — frameworks

### How many frameworks does the spike build?
| Option | Description | Selected |
|--------|-------------|----------|
| Build all 3 (R/S/Sv) | Same StatusReport view in React, Solid, AND Svelte; max decision confidence; ~3-4 days | ✓ |
| Build 2 (drop one upfront) | Pre-eliminate one on shortlist rationale; saves time but elimination is a guess | |
| Build 1 — pick by reading | Skip comparison; pick by docs + recommendations | |

**User's choice:** Build all 3.

### What does each spike render?
| Option | Description | Selected |
|--------|-------------|----------|
| Just StatusReport | Single-page dashboard, real `tome status` data, no virtualization/interactions | ✓ |
| Status + skill list (~50 skills) | Adds basic list; 2-3x more code per spike | |
| Status + list + one action | Adds a 2nd Tauri command click handler; risks Phase 26 scope creep | |

**User's choice:** Just StatusReport.

### Comparison criteria
| Option | Description | Selected |
|--------|-------------|----------|
| bindings.ts ergonomics | How natural specta TS types feel in each framework's idioms | ✓ |
| Bundle size + cold-start | Production bytes + initial render TTI | ✓ |
| Dev loop speed (HMR, errors) | Edit→see-change latency, error quality, type-check speed | ✓ |
| Ecosystem fit for v1.0 reqs | Virtualized lists, keyboard a11y, HIG components | ✓ |

**User's choice:** All four criteria (multi-select).

### Decision artifact
| Option | Description | Selected |
|--------|-------------|----------|
| Scoring table + ADR | 1-5 per criterion + ADR with rationale + invalidation conditions | ✓ |
| Decision in CONTEXT.md only | Chosen framework + 1 paragraph; lighter weight | |
| Just commit the winning crate | Surviving code IS the decision; relies on git history | |

**User's choice:** Scoring table + ADR (at `.planning/research/v1.0-frontend-framework-decision.md`; also update D-GUI-04).

---

## Structured-type home + specta gating

### Where types live
| Option | Description | Selected |
|--------|-------------|----------|
| Stay in crates/tome | Co-located in producing modules; tome-desktop imports directly; zero refactor | ✓ |
| Extract to crates/tome-core | New schema crate; cleaner but reshuffles ~20 type defs | |
| Wrapper types in tome-desktop | crates/tome untouched; doubles type surface | |

**User's choice:** Stay in crates/tome.

### Specta gating
| Option | Description | Selected |
|--------|-------------|----------|
| `bindings` feature flag | Optional feature; `cfg_attr` derive; CLI pays nothing | ✓ |
| Always-on specta derive | Simpler but CLI takes the dep forever | |
| Mirror types in tome-desktop | Strict isolation but duplicates defs (contradicts prior choice) | |

**User's choice:** `bindings` feature flag.

### bindings.ts generation + freshness
| Option | Description | Selected |
|--------|-------------|----------|
| Build-time at tauri-desktop compile | build.rs exports; committed; CI `git diff --exit-code` | ✓ |
| Dedicated codegen binary | `cargo run --bin gen-bindings`; decoupled from GUI build | |
| Generated at startup (not committed) | Gitignored; no freshness check; not PR-reviewable | |

**User's choice:** Build-time at compile, committed, CI diff gate.

### Owned/Unowned enum shape (#542)
| Option | Description | Selected |
|--------|-------------|----------|
| `Provenance` enum | `SkillProvenance { Owned { source }, Unowned { last_owner } }`; lifts last_directory_name | ✓ |
| Keep Option + separate Source enum | Less invasive; keeps Option-as-state-machine | |
| Tagged union via serde discriminator | Most flexible, most refactor | |

**User's choice:** `Provenance` enum.

---

## Progress-event abstraction

### Abstraction shape
| Option | Description | Selected |
|--------|-------------|----------|
| ProgressSink trait | `&dyn ProgressSink`; domain stays sync; CLI=indicatif, GUI=tauri emit | ✓ |
| tokio mpsc channel return | Idiomatic for Tauri but forces async-colouring + tokio dep | |
| Closure callback | Simpler than trait but duplicated closure setup per call site | |

**User's choice:** ProgressSink trait.

### ProgressEvent shape
| Option | Description | Selected |
|--------|-------------|----------|
| Per-op variants | Type-safe, semantically rich; GUI pattern-matches | ✓ |
| Generic Phase/Percent/Message | Simpler but loses semantic richness | |
| JSON Value payload | Max flexibility, loses type safety | |

**User's choice:** Per-op variants.

### Trait home + CLI default
| Option | Description | Selected |
|--------|-------------|----------|
| Trait in tome::progress, Indicatif in lib.rs | New progress.rs; IndicatifSink+NullSink; tome-desktop has TauriEventSink | ✓ |
| Trait in tome, sink in cli::progress | More structure now | |
| Trait+sink in tome-desktop, CLI keeps indicatif inline | Defers contract to Phase 27 | |

**User's choice:** Trait in tome::progress, Indicatif impl in lib.rs.

### Cancellation
| Option | Description | Selected |
|--------|-------------|----------|
| Per-op `&CancellationToken` arg | Explicit, testable; CLI passes never-tripped token | ✓ |
| Cancel via ProgressSink return value | `ControlFlow` from emit; conflates observe + control | |
| Defer to Phase 27 — sketch only | TODO comments at boundaries; risk stub mismatch | |

**User's choice:** Per-op `&CancellationToken` arg (threaded now; real behavior lands SYNC-04/Phase 27).

---

## TomeError taxonomy granularity

### Typing boundary
| Option | Description | Selected |
|--------|-------------|----------|
| Classify at IPC boundary | Domain keeps anyhow; tome-desktop wraps into TomeError at the edge | ✓ |
| Typed errors at domain boundaries only | Public fns return TomeError; ~14 fns; medium refactor | |
| Typed errors all the way down | thiserror through all 27 modules; massive refactor, regression risk | |

**User's choice:** Classify at IPC boundary.

### Classification reliability
| Option | Description | Selected |
|--------|-------------|----------|
| Typed sentinel errors via downcast | `DomainErrorKind` attached at GUI-relevant sites; `downcast_ref` at boundary | ✓ |
| io::ErrorKind + heuristics only | Free Permission/NotFound; coarse fallback to Internal | |
| Sentinel at boundary call sites only | Per-command mapping; duplicates classification knowledge | |

**User's choice:** Typed sentinel errors via downcast.

### ErrorCode granularity
| Option | Description | Selected |
|--------|-------------|----------|
| Coarse ~6 categories | Validation, NotFound, Permission, Conflict, Git, Io, Internal | ✓ |
| Medium ~10-12 with sub-context | PathOverlap, LockfileStale, RemoteAuthFailed, etc. | |
| Start coarse, grow per phase | 6 now; later phases add codes their UI needs | |

**User's choice:** Coarse ~6 categories (with additive-growth discipline).

### Error payload
| Option | Description | Selected |
|--------|-------------|----------|
| code + message + context chain | `Vec<String>` flattened anyhow chain; GUI shows details on disclosure | ✓ |
| code + message only | Single flattened string; loses headline-vs-details split | |
| code + message + optional structured detail | Per-code payload; richest but couples to failure data | |

**User's choice:** code + message + context chain.

---

## Claude's Discretion

- `lib.rs::run` decomposition mechanics (cmd_* inline vs `presenters/` module) — left to planning.
- Exact `SyncStage` members, `ProgressEvent` variant fields, and which sites get `DomainErrorKind` sentinels — planner's call within the locked decisions.

## Deferred Ideas

- `lib.rs::run` decomposition mechanics (Phase 25 planning detail, not a future phase).
- Tauri 2.x minor-version pin policy (Q2) — default to pinning `2.M.N`.
- CI matrix shape for tome-desktop builds + bindings-freshness gate placement.
- Per-code structured `ErrorDetail` payloads — added when a later phase's UI needs them.
- Q3 (lint surfacing), Q4 (tray icon), Q6 (Sparkle vs tauri-plugin-updater), Q7 (telemetry) — later phases.
