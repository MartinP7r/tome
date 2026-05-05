# Phase 12: Marketplace adapter - Discussion Log

> **Audit trail only.** Do not use as input to planning, research, or execution agents.
> Decisions are captured in CONTEXT.md — this log preserves the alternatives considered.

**Date:** 2026-05-05
**Phase:** 12-marketplace-adapter
**Areas discussed:** Claude CLI non-interactivity, available() / list_installed() semantics, GitAdapter trait fit, InstallFailure shape

---

## Gray-area selection

| Area | Selected |
|------|----------|
| Claude CLI non-interactivity | ✓ |
| available() / list_installed() semantics | ✓ |
| GitAdapter trait fit | ✓ |
| InstallFailure shape | ✓ |

**User's choice:** All four selected.

---

## Empirical probes (run before answering)

User asked: "do we need to test any of these methods first?" → triggered three probes against `claude 2.1.126` before committing to a strategy.

### Probe 1 — stdin-closed install/update/failure

| Test | Result |
|------|--------|
| `claude plugin install axiom@axiom-marketplace </dev/null` (already installed) | Exit 0, prompt-free, immediate |
| `claude plugin update axiom </dev/null` | Exit 1, "Plugin axiom not found", prompt-free |
| `claude plugin install does-not-exist@nonexistent </dev/null` | Exit 1, clear stderr, prompt-free |

**Conclusion:** stdin-closed is sufficient for non-interactive guarantee. No env-var heuristics needed.

### Probe 2 — `--scope managed` for install

User-run: `claude plugin install --scope managed axiom@axiom-marketplace`
**Result:** `Invalid scope: managed. Must be one of: user, project, local.`
**Conclusion:** `--scope managed` not viable for install. Stick with default `user` scope.

### Probe 3 — `list --available --json` and `errors[]` field

| Finding |
|---------|
| `list --json` returns flat array of installed plugins |
| `list --available --json` returns object `{ "installed": [...], ... }` (different shape) |
| Installed entries include `errors: ["Plugin X not found in marketplace Y"]` when vanished |

**Conclusion:** `available()` parses `errors[]` from cached `list --json` snapshot — zero extra subprocess calls.

---

## Area: Claude CLI non-interactivity (subprocess invocation policy)

| Option | Description | Selected |
|--------|-------------|----------|
| Stdin=/dev/null + capture stderr | Stdin closed, env unchanged, capture stderr verbatim. Empirically validated. | ✓ |
| Stdin=/dev/null + CI=true belt-and-braces | Also set CI=true / NO_COLOR=1 defensively | |
| Stdin=/dev/null + version-pinned tested-against | Add `claude --version` log line for diagnostic correlation | |

**User's choice:** Stdin=/dev/null + capture stderr (Recommended)
**Notes:** Empirical evidence overwhelmingly supported the minimal policy. No need for defensive env-var or version-detection heuristics.

---

## Area: available() / list_installed() semantics — signal source

| Option | Description | Selected |
|--------|-------------|----------|
| Parse errors[] from list --json | Reuse the cached snapshot's `errors` field. Zero extra subprocess calls. | ✓ |
| Separate list --available --json query | Fetch full catalog per available() call (or once cached) | |
| Hybrid: errors[] then --available fallback | Cheap path first, fallback for plugins not in list | |

**User's choice:** Parse errors[] from list --json (Recommended)
**Notes:** Probe 3 confirmed the `errors[]` field carries the vanished signal natively. Phase 13 RECON-04 detection becomes essentially free.

---

## Area: available() / list_installed() — snapshot cache lifetime

| Option | Description | Selected |
|--------|-------------|----------|
| Internal cache + refresh() hook | RefCell snapshot, auto-invalidates on install/update Ok, public refresh() for explicit re-query | ✓ |
| Stateless adapter, caller manages snapshot | Always re-runs subprocess; trait params shift to `&InstalledPlugin` | |
| Cache once per instance, no auto-invalidate | Simplest; foot-gun if Phase 13 forgets to refresh | |

**User's choice:** Internal cache + refresh() hook (Recommended)
**Notes:** Auto-invalidate after install/update keeps the API foot-gun-free; explicit refresh() leaves a hook for callers that want forced freshness.

---

## Area: GitAdapter trait fit

| Option | Description | Selected |
|--------|-------------|----------|
| One adapter per git directory | URL-bound instance; `list_installed()` returns vec![one]; `current_version()` = HEAD SHA; `available()` = local clone exists | ✓ |
| One GitAdapter, batches all git URLs | Single instance handles all git URLs in batch | |
| Sub-trait: GitAdapter only implements install/update | Carve smaller trait; other methods error or no-op | |

**User's choice:** One adapter per git directory (Recommended)
**Notes:** Natural 1:1 with config entries. Trait surface stays uniform across both adapter implementations.

---

## Area: InstallFailure shape

| Option | Description | Selected |
|--------|-------------|----------|
| Marketplace-specific struct | `{ adapter_id, plugin_id, operation, kind, source }` with `InstallOp` and `InstallFailureKind` enums | ✓ |
| Mirror RemoveFailure exactly | `{ path, kind }` — slight forced fit (path may not exist at install time) | |
| Reuse RemoveFailure with new variants | Single failure type across remove and install; mixes concerns | |

**User's choice:** Marketplace-specific struct (Recommended)
**Notes:** Captures what's actually meaningful for marketplace ops. Compile-time `ALL` array exhaustiveness (POLISH-04) applies regardless.

---

## Wrap-up

User selected "Move to context" without exploring additional gray areas. Remaining details (mock adapter location, exact trait param types, install scope, `lib.rs::sync` wiring) became Claude's Discretion — captured in CONTEXT.md `<decisions>` and `Claude's Discretion` subsection.

## Claude's Discretion

- Exact rendering text of grouped failure summary (within SAFE-01 visual conventions)
- stderr-string → `InstallFailureKind` heuristic mapping
- `RefCell` vs `OnceCell` vs `Mutex` for cache (compiles with Send/Sync)
- Whether `GitAdapter::available()` actually probes network or trusts local-clone existence
- Missing-`claude`-binary detection mechanism
- `MockMarketplaceAdapter` constructor knobs
- Rendering helper location (`marketplace.rs` vs `lib.rs`)

## Deferred Ideas

- Subprocess timeout knob (deferred per D-03)
- `--scope project/local` support (deferred per D-09)
- `list --available --json` catalog query (not needed for Phase 12/13)
- Async trait variant
- Adapters for non-Claude marketplaces (npm, pip, OS package managers)
- Lifting `MockMarketplaceAdapter` to `pub(crate)` for integration test reuse (Phase 13 tactical decision)
