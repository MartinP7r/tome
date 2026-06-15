---
phase: 26-read-only-views-alpha-cut
plan: 08
subsystem: perf-verification
tags: [perf, playwright, rust, fixture, ci, macos, NF-01]
status: complete
requires:
  - 26-02-SUMMARY  # SkillsView + Virtualizer + useSkills (the surface this bench measures)
  - 26-07-SUMMARY  # playwright + Vite alias pattern (this plan extends both)
provides:
  - "Cargo integration test `tests/perf/synthetic_skills.rs` — generates a deterministic 2000-skill tome library at `${PERF_FIXTURE_OUT}`"
  - "Playwright spec `tests/perf/60fps-search.spec.ts` — asserts p95 inter-frame delta < 18ms over 2s search-as-you-type"
  - "FPS sampler `tests/perf/fps-sampler.js` — window.__startFpsSampler(durationMs) + window.__fpsFrames[]"
  - "`PERF_TEST=1` Vite mode — alongside `A11Y_TEST=1` from 26-07; switches mock fixture from 3-skill to 2000-skill"
  - "GitHub Actions perf-bench workflow `.github/workflows/perf.yml` — macos-latest, narrow-path triggers, INDEPENDENT of `make ci` (CLAUDE.md constraint 11)"
  - "`26-PERF-REPORT.md` — methodology + OQ-1 evaluation + baseline runs"
  - "`tome::manifest` lifted from `pub(crate)` to `pub` — narrow surface (Manifest, SkillEntry, SkillOwnership, load, save) so the fixture can construct rows via the canonical serde shape"
affects:
  - crates/tome/src/lib.rs                                       # manifest module lifted to pub
  - crates/tome-desktop/Cargo.toml                               # rand 0.9 dev-dep + explicit [[test]] target
  - crates/tome-desktop/tests/perf/synthetic_skills.rs           # NEW — fixture generator
  - crates/tome-desktop/tests/perf/playwright.config.ts          # NEW — bench config
  - crates/tome-desktop/tests/perf/60fps-search.spec.ts          # NEW — the bench
  - crates/tome-desktop/tests/perf/fps-sampler.js                # NEW — rAF sampler
  - crates/tome-desktop/ui/package.json                          # dev:perf + test:perf scripts
  - crates/tome-desktop/ui/vite.config.ts                        # PERF_TEST=1 mode + define VITE_PERF_TEST
  - crates/tome-desktop/ui/src/__mocks__/tauri-api-core.ts       # listSkills branches between a11y/perf fixtures
  - crates/tome-desktop/ui/src/__mocks__/perf-skills.json        # NEW — 3-skill in-tree stub
  - .github/workflows/perf.yml                                   # NEW — macOS-only bench CI
  - .planning/phases/26-read-only-views-alpha-cut/26-PERF-REPORT.md  # NEW — perf report
  - .gitignore                                                   # 26-PERF-RUNS.tsv + perf test-results dirs
tech-stack:
  added:
    - "rand 0.9 (dev-dependency on tome-desktop) — seeded RNG for deterministic 2000-skill fixture body content"
  patterns:
    - "Perf-bench fixture pattern: Cargo integration test self-gates on PERF_FIXTURE_OUT env var so it skips silently during `cargo test --all` / `make ci`. Generates the fixture lazily on demand."
    - "Vite mode multiplexing: A11Y_TEST=1 and PERF_TEST=1 share the same Tauri mock modules; the listSkills arm branches on `import.meta.env.VITE_PERF_TEST` (pre-substituted via Vite's `define`) at build time. Zero runtime cost in production builds."
    - "FPS sampling via manual-start: `addInitScript` injects a sampler that exposes a `window.__startFpsSampler(durationMs)` global. The bench calls it immediately before `keyboard.type()` so the sampling window doesn't include first-paint cost (which would distort p95)."
    - "Boundary-case CI signal: when local-hardware runs land 0.1-0.4ms above threshold, leave the threshold unchanged. CI on dedicated hardware (Assumption A12) is the source of truth — the report documents both interpretations + decision path."
    - "Runs log separation: bench writes raw rows to gitignored `26-PERF-RUNS.tsv`, NOT appended to PERF-REPORT.md (which has prose sections after the table). CI uploads the TSV as artefact; human promotes curated rows into the report's `Baseline runs` table during review."
    - "GitHub Actions workflow path-trigger discipline: perf bench runs ONLY on PRs touching ui/, tests/perf/, tome-desktop crate, or core list/discover/manifest modules. No doc-only PR pays the bench tax."
key-files:
  created:
    - crates/tome-desktop/tests/perf/synthetic_skills.rs
    - crates/tome-desktop/tests/perf/60fps-search.spec.ts
    - crates/tome-desktop/tests/perf/fps-sampler.js
    - crates/tome-desktop/tests/perf/playwright.config.ts
    - crates/tome-desktop/ui/src/__mocks__/perf-skills.json
    - .github/workflows/perf.yml
    - .planning/phases/26-read-only-views-alpha-cut/26-PERF-REPORT.md
  modified:
    - crates/tome/src/lib.rs
    - crates/tome-desktop/Cargo.toml
    - crates/tome-desktop/ui/package.json
    - crates/tome-desktop/ui/vite.config.ts
    - crates/tome-desktop/ui/src/__mocks__/tauri-api-core.ts
    - .gitignore
    - Cargo.lock
decisions:
  - "Lift `tome::manifest` from `pub(crate)` to `pub` so the perf fixture can construct Manifest + SkillEntry rows via the canonical serde shape — no hand-written JSON, no drift risk. Precedent: plan 26-02 lifted `tome::list` for the same reason."
  - "Declare `[[test]] name = synthetic_skills, path = tests/perf/synthetic_skills.rs` in `crates/tome-desktop/Cargo.toml` because Cargo's auto-discovery doesn't traverse `tests/perf/`. Keeps the file in its planned location (alongside the Playwright spec) without giving up `cargo test --test synthetic_skills` invocation."
  - "Self-gate the fixture test on `PERF_FIXTURE_OUT` env var (skip silently when unset) so it never runs as part of `make ci` (CLAUDE.md constraint 11 — perf benches are explicitly excluded from the standard test matrix)."
  - "Use plain `.js` (not `.ts`) for the FPS sampler — `addInitScript({ path: ... })` reads the file verbatim into the page with no transpilation; TS-only syntax fails silently. Globals are typed in the spec file via `declare global`."
  - "Manual sampler start (`window.__startFpsSampler(2000)` called right before `keyboard.type()`) — auto-sampling from page load would include first-paint + initial-render cost and distort the median/p95."
  - "Use Playwright's `getByRole('searchbox', { name: 'Search skills' })` rather than a CSS attribute selector — the React Aria `<AriaSearchField>` puts the `aria-label` on the OUTER wrapper while the inner `<input type=\"search\">` carries the implicit `role=\"searchbox\"`, so the combined CSS selector cannot match either single DOM node."
  - "Write per-run rows to a gitignored `26-PERF-RUNS.tsv` rather than appending to `26-PERF-REPORT.md` — `fs.appendFileSync` only writes to file end, which would scramble the report's prose sections. CI uploads the TSV as an artefact; humans promote curated rows into the report's table."
  - "Ship an in-tree 3-skill stub `perf-skills.json` so unrelated UI builds + the a11y gate keep resolving the mock's JSON import. The bench harness copies the real 2000-skill fixture over the stub before `npm run dev:perf`."
  - "Vite `define` substitutes `import.meta.env.VITE_PERF_TEST` at build time → the mock module's listSkills branch is a compile-time constant; production builds have zero perf-mode overhead."
  - "Perf workflow runs only on PRs touching `ui/`, `tests/perf/`, `tome-desktop/src/`, or core `list.rs`/`discover.rs`/`manifest.rs` — narrow path trigger, doc-only PRs pay no bench tax. `workflow_dispatch` covers the manual case."
  - "macos-latest only — D-GUI-06 ships the GUI macOS-only; Assumption A12 calibrates the perf budget to dedicated Apple Silicon. Linux runners would test irrelevant render paths."
  - "Do NOT silently raise the p95 threshold despite the local-hardware boundary failure (p95 18.10-18.40ms vs 18ms target). The plan's executor instructions are explicit: surface the failure, document the OQ-1 decision path (path-A vs path-B/TanStack), don't auto-rewrite SkillsView. CI on dedicated macos-latest is the source of truth."
metrics:
  duration: "~29 min (single-session, no interruptions)"
  started: "2026-05-29T11:14:31Z"
  completed: "2026-05-29T11:43:39Z"
  loc: "+1084 / -15 across 16 files (incl. 4 commits + 2 SUMMARY checkpoints)"
  tasks_completed: "3 (all type=auto; Tasks 1+2 tdd=true)"
  commits: 6
---

# Phase 26 Plan 08: Alpha-cut perf-bench harness — Summary

A 2000-skill synthetic fixture + a Playwright `requestAnimationFrame` bench + a macOS-only GitHub Actions workflow that runs it on PRs touching the Skills view's render path. Closes out the read-only-views alpha cut and stamps NF-01 on the budget side: the React Aria native `<Virtualizer>` either confirms 60fps under real load or surfaces a concrete OQ-1 fallback decision rather than papering over the budget.

## What ships

### NF-01 — three-layer perf-bench harness

| Layer | File | Role |
|---|---|---|
| Rust fixture generator | `crates/tome-desktop/tests/perf/synthetic_skills.rs` | Cargo integration test that writes 2000 deterministic-seeded SKILL.md fixtures + a real `.tome-manifest.json` + a `tome.toml` + a `perf-skills.json` projection at `${PERF_FIXTURE_OUT}`. Self-gates on the env var so `cargo test --all` skips silently. |
| Playwright bench | `crates/tome-desktop/tests/perf/60fps-search.spec.ts` + `fps-sampler.js` + `playwright.config.ts` | Navigates to the Skills view (mocked Tauri returns the 2000-skill fixture), focuses the SearchField, kicks off a 2-second `requestAnimationFrame` sampling window, types `"tdd"` at 15ms inter-keystroke delay, asserts p95 inter-frame delta < 18ms (~55fps p95). |
| GitHub Actions workflow | `.github/workflows/perf.yml` | macOS-only (Apple Silicon, Assumption A12). Narrow-path triggers (`ui/`, `tests/perf/`, `tome-desktop/src/`, plus the core `list.rs`/`discover.rs`/`manifest.rs`). Uploads `26-PERF-REPORT.md` + `26-PERF-RUNS.tsv` as artefacts on every run. INDEPENDENT of `ci.yml` — CLAUDE.md constraint 11 honored. |

### Architecture choices that the harness encodes

- **Vite mode multiplexing.** `A11Y_TEST=1` (plan 26-07) and `PERF_TEST=1` (this plan) share the same Tauri mock modules. The new entry — `define: { "import.meta.env.VITE_PERF_TEST": JSON.stringify(...) }` — pre-substitutes the env var at build time, so the mock's `commands.listSkills` arm is a **compile-time constant**: zero runtime cost in production builds.
- **Manual FPS sampler start.** The plan's draft sampler auto-started on page load, which would have included first-paint + initial-list-render cost in the percentile math. The shipped sampler exposes `window.__startFpsSampler(durationMs)` so the bench scopes the window to **search-as-you-type only**.
- **`.js`, not `.ts`, for the sampler.** `addInitScript({ path: ... })` reads the file verbatim into the page — no TypeScript transpilation. TS syntax (`declare global`, `export {}`) fails silently. Globals are typed in the spec file via `declare global` so `page.evaluate(() => window.__startFpsSampler(...))` still type-checks.
- **Searchbox selector via `getByRole`, not CSS.** React Aria's `<AriaSearchField>` puts the `aria-label="Search skills"` on the OUTER wrapper while the inner `<input type="search">` carries the implicit `role="searchbox"`. A combined CSS attribute selector cannot match either single DOM node. `page.getByRole("searchbox", { name: "Search skills" })` walks the accessible-name tree, so the inner input inherits the wrapper's label.
- **Runs log split from the report.** `fs.appendFileSync` only writes to file end, which would scramble `26-PERF-REPORT.md`'s prose sections. The shipped bench writes to a separate gitignored `26-PERF-RUNS.tsv`; CI uploads it as an artefact; humans promote curated rows into the report's `Baseline runs` table during review.
- **In-tree stub `perf-skills.json`.** A 3-skill placeholder keeps unrelated UI builds + the a11y gate happy. The bench harness copies the real 2000-skill fixture from `${PERF_FIXTURE_OUT}` over the stub before `npm run dev:perf`. CI is ephemeral; local users can `git checkout` the stub after a run.

## Task commits

| Task | Hash | Subject |
|------|------|---|
| 1 | `8c021da` | `test(26-08): synthetic 2000-skill fixture generator (NF-01 setup)` |
| 1+ | `746da6d` | `docs(26-08): start SUMMARY (Task 1 of 3 complete)` |
| 2 | `b534a0d` | `test(26-08): Playwright 60fps perf bench + Tauri mock extension (NF-01)` |
| 2+ | `85287d4` | `docs(26-08): SUMMARY checkpoint after Task 2` |
| 3 | `ac8559c` | `ci(26-08): macOS-only perf bench workflow (NF-01)` |

(A final `docs(26-08): finalize SUMMARY` commit follows the SUMMARY rewrite below.)

## First-run perf baseline

Four local runs on an Apple Silicon M-series Mac in headed dev mode (Vite watcher + browser dev-tools live):

| Run | Verdict | Samples | p50 (ms) | p95 (ms) | max (ms) | Dropped (>18ms) |
|---|---|---|---|---|---|---|
| 1 | FAIL | 121 | 16.70 | 18.20 | 31.73 | 9 |
| 2 | FAIL | 121 | 16.70 | 18.10 | 21.59 | 9 |
| 3 | FAIL | 121 | 16.70 | 18.10 | 18.60 | 7 |
| 4 | FAIL | 121 | 16.70 | 18.40 | 18.70 | 13 |

**Boundary-case failure pattern.** p50 is exactly 60fps (16.70ms — vsync 60Hz); p95 lands 0.1–0.4ms above the 18ms target across all four runs. p50 + p95 + max all tell a consistent story: **the steady-state per-frame work is well within budget; the tail behaviour during search-as-you-type is right at the edge.**

### OQ-1 evaluation (full text in `26-PERF-REPORT.md`)

**This is NOT a clear failure of the React Aria native `<Virtualizer>`.** The virtualisation is working — p50 is locked at 60fps. The boundary is in the **keystroke-driven filter + re-render cycle**: each character typed triggers fuse.js filtering + React reconciliation, costing one or two ~one-vsync stalls. The TanStack-Virtual swap (path B) would change the scrolling-window math but would NOT eliminate the keystroke-driven cost.

**Two interpretations:**

1. **Threshold tuned a hair too tight for the React Aria path under local-hardware noise.** Plan's commentary explicitly calls 60fps over 1s unrealistic; 18ms was the slack-bearing approximation. Local Mac with Vite watcher + dev tools live is contention-noise-prone. A 19ms (~52fps p95) threshold would land all four runs as PASS. **CI on dedicated macos-latest (Apple Silicon, Assumption A12) is the source of truth.**
2. **Measurable steady-state regression vs path B (TanStack Virtual).** The documented fallback in 26-RESEARCH.md §"Standard Stack — Virtualisation". It's a non-trivial SkillsView refactor — **out of scope for plan 26-08 per executor instructions, but on the table if CI numbers also show the boundary failure.**

**Decision encoded in the harness:** the 18ms target is unchanged. CI will fail on PR if the result lands above it — that failure surface IS the signal the plan asked for. SkillsView is NOT refactored to TanStack here; that decision goes to the next milestone's planning phase if CI confirms a real regression.

## Verification

| Gate | Result |
|---|---|
| `cargo clippy --all-targets -- -D warnings` | clean |
| `cargo test -p tome --lib` | 909 passed (no regression from the manifest pub lift) |
| `PERF_FIXTURE_OUT=/tmp/tome-perf-fixture cargo test -p tome-desktop --test synthetic_skills -- --nocapture` | 2000 skill dirs + 244K perf-skills.json + 652K manifest written |
| `unset PERF_FIXTURE_OUT && cargo test -p tome-desktop --test synthetic_skills` | skips silently (correct gate) |
| `cd crates/tome-desktop/ui && npx tsc --noEmit` | clean |
| `cd crates/tome-desktop/ui && npm test` | 5/5 vitest |
| `cd crates/tome-desktop/ui && A11Y_TEST=1 npm run build` | clean (a11y mode unaffected) |
| `cd crates/tome-desktop/ui && PERF_TEST=1 npm run build` | clean (perf mode bundles) |
| `cd crates/tome-desktop/ui && PERF_TEST=1 npm run test:perf` | 4× ran end-to-end, p95 = 18.10–18.40ms (boundary FAIL — see OQ-1 evaluation) |
| `actionlint .github/workflows/perf.yml` | clean (workflow well-formed) |

## Deviations from plan

### Auto-fixed during execution

**1. [Rule 3 - Blocking] `tome::manifest` was `pub(crate)`**
- **Found during:** Task 1 (test compile).
- **Issue:** The fixture wanted to construct `Manifest` + `SkillEntry` rows via the canonical serde API but the module wasn't reachable from `tome-desktop`'s test target.
- **Fix:** Lifted to `pub`. Narrow surface (`Manifest`, `SkillEntry`, `SkillOwnership`, `load`, `save`, `hash_directory`). Precedent: plan 26-02 lifted `tome::list` for the same reason.
- **Commit:** `8c021da`.

**2. [Rule 3 - Blocking] Cargo can't auto-discover tests under `tests/perf/`**
- **Found during:** Task 1 (first `cargo test --test synthetic_skills` reports "no test target").
- **Fix:** Explicit `[[test]] name = "synthetic_skills" path = "tests/perf/synthetic_skills.rs"` in `crates/tome-desktop/Cargo.toml`. Keeps the file in its planned location (alongside the Playwright spec).
- **Commit:** `8c021da`.

**3. [Logged as deferred, NOT auto-fixed] `cargo fmt --all -- --check` surfaced pre-existing drift**
- **Found during:** Task 1.
- **Files affected (pre-existing, not my edits):** `crates/tome/src/doctor.rs` (4 spots), `crates/tome/src/skill.rs` (1 spot), `crates/tome-desktop/src/commands.rs` (import order).
- **Why deferred:** Out of scope per executor scope-boundary rules. Logged to `deferred-items.md` for a follow-up cleanup.

**4. [Rule 3 - Blocking] CSS attribute selector `[role="searchbox"][aria-label="..."]` doesn't match**
- **Found during:** Task 2 (first Playwright run — `page.focus` timed out at 30s).
- **Issue:** The two attributes live on different DOM nodes in React Aria's `<AriaSearchField>` (wrapper + inner input).
- **Fix:** Switched to `page.getByRole("searchbox", { name: "Search skills" })` — same pattern as the 26-07 a11y spec; accessible-name tree resolves the inner input correctly.
- **Commit:** `b534a0d`.

**5. [Rule 3 - Blocking] `addInitScript({ path: "fps-sampler.ts" })` fails — TS doesn't transpile**
- **Found during:** Task 2 (`window.__startFpsSampler is not a function`).
- **Issue:** Playwright runs the script verbatim in the page. TS syntax (`declare global`, `export {}`) doesn't parse as plain JS.
- **Fix:** Renamed `fps-sampler.ts` → `fps-sampler.js` (plain JS, no TS syntax). Type declarations for the window globals moved to the top of `60fps-search.spec.ts`.
- **Commit:** `b534a0d`.

**6. [Rule 2 - Critical] `fs.appendFileSync` on `26-PERF-REPORT.md` lands rows AFTER later prose sections**
- **Found during:** Task 2 (after the first run, the appended baseline row landed after the new "OQ-1 evaluation" section).
- **Issue:** Markdown reports have a structure; appending to file end ignores it.
- **Fix:** Bench writes to a separate `26-PERF-RUNS.tsv` log (gitignored). CI uploads the TSV; humans promote curated rows into the report's `Baseline runs` table during review.
- **Commit:** `b534a0d`.

### Deliberately NOT auto-fixed

**7. [Rule 4 boundary] p95 = 18.10–18.40ms on local hardware — boundary-case FAIL**
- **Found during:** Task 2 (four local runs, all FAIL by 0.1–0.4ms).
- **Why deferred:** Plan's executor instructions are explicit: "Don't auto-rewrite SkillsView; that's a separate decision." TanStack Virtual swap is the path-B fallback but a substantial refactor. CI on macos-latest (Task 3 now landed) is the source-of-truth measurement; local laptop with Vite watcher live is contention-noise-prone.
- **Resolution:** `26-PERF-REPORT.md` §"OQ-1 evaluation" documents both interpretations + the decision path. The 18ms target in the spec is unchanged — CI will surface the truth on first PR run.

## Threat model — disposition recap

| ID | Threat | Disposition delivered |
|----|---|---|
| T-26-08-01 | DoS — synthetic fixture exhausts CI disk | **mitigated** — fixture is ~10MB on disk (2000 skill dirs avg 5KB, 244KB JSON projection, 652KB manifest). Well within macos-latest GitHub Actions disk budget. |
| T-26-08-02 | InformationDisclosure — perf report uploaded as PR artefact | **accepted** — only timing numbers + ISO timestamps; no user data. Public on PR artefacts. |
| T-26-08-03 | Tampering — perf-mode mock ships to production | **mitigated** — Vite alias gated by `process.env.PERF_TEST === "1"` AND `import.meta.env.VITE_PERF_TEST === "1"`. Production builds set neither → no aliases → mock module is tree-shaken. |
| T-26-08-04 | Tampering — flaky FPS measurements cause false positives/negatives | **mitigated** — p95 (not max) is the assertion; 2s sampling window; `workers: 1` + `retries: 0` in Playwright config eliminates parallel-execution noise. If CI also shows flakes, raise the threshold to 19ms (52fps p95) in a documented follow-up — but don't silently change it. |
| T-26-08-SC | Tampering — npm/cargo installs (rand, playwright) | **mitigated** — `playwright` covered by 26-07 Task 0; `rand` 0.9 is a widely-used Rust standard-library-adjacent crate (rust-random org, MIT OR Apache-2.0). |

## Known Stubs

The in-tree `crates/tome-desktop/ui/src/__mocks__/perf-skills.json` is a 3-skill **stub** that ships in git. It is NOT a stub in the production render path — the mock module that imports it is itself gated behind the Vite `PERF_TEST=1` alias, so production builds never reach it. During perf-bench runs the bench harness copies the real 2000-skill fixture over the stub. Documented inline in the mock module + at the top of the file.

## TDD Gate Compliance

Tasks 1 + 2 are marked `tdd="true"` in the plan. The shipped tests are self-checking integration tests (Task 1's fixture generator round-trips through `tome::manifest::load`; Task 2's bench asserts on a measurable threshold). The RED → GREEN cycle is degenerate because the test IS the implementation in both cases — but a meaningful "would have failed without the implementation" is captured: Task 1's `assert_eq!(written, 2000)` would fail on any premature return; Task 2's `expect(frames.length).toBeGreaterThan(30)` catches a non-firing sampler.

The `test(...)` commit subjects (`test(26-08): ...`) carry the TDD intent through `git log` so future reviewers can trace the gate sequence.

## Threat Flags

None. No new network endpoints, auth paths, file access patterns, or schema changes were introduced. The bench writes to a tempdir (`/tmp/tome-perf-fixture`) under an env var the caller sets; the perf workflow runs only on macos-latest GitHub-hosted runners (no self-hosted runner introduction).

---
*Phase: 26-read-only-views-alpha-cut*
*Plan: 08*
*Completed: 2026-05-29*

## Self-Check: PASSED

Files asserted to exist (created):

```
crates/tome-desktop/tests/perf/synthetic_skills.rs              — FOUND
crates/tome-desktop/tests/perf/60fps-search.spec.ts             — FOUND
crates/tome-desktop/tests/perf/fps-sampler.js                   — FOUND
crates/tome-desktop/tests/perf/playwright.config.ts             — FOUND
crates/tome-desktop/ui/src/__mocks__/perf-skills.json           — FOUND (555B stub)
.github/workflows/perf.yml                                      — FOUND
.planning/phases/26-read-only-views-alpha-cut/26-PERF-REPORT.md — FOUND
.planning/phases/26-read-only-views-alpha-cut/26-08-SUMMARY.md  — FOUND
.planning/phases/26-read-only-views-alpha-cut/deferred-items.md — FOUND
```

Commits asserted to exist:

```
8c021da — test(26-08): synthetic 2000-skill fixture generator (NF-01 setup)  — FOUND
746da6d — docs(26-08): start SUMMARY (Task 1 of 3 complete)                  — FOUND
b534a0d — test(26-08): Playwright 60fps perf bench + Tauri mock extension    — FOUND
85287d4 — docs(26-08): SUMMARY checkpoint after Task 2                       — FOUND
ac8559c — ci(26-08): macOS-only perf bench workflow (NF-01)                  — FOUND
```
