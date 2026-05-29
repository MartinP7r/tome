---
phase: 26-read-only-views-alpha-cut
plan: 08
subsystem: perf-verification
tags: [perf, playwright, rust, fixture, NF-01]
status: in-progress
requires:
  - 26-02-SUMMARY  # SkillsView + Virtualizer + useSkills (the surface this bench measures)
  - 26-07-SUMMARY  # playwright + Vite alias pattern (this plan extends both)
provides: []  # Will be filled in once all three tasks land + SUMMARY finalises.
affects:
  - crates/tome-desktop/Cargo.toml             # Task 1 — rand 0.9 dev-dep + [[test]] target
  - crates/tome-desktop/tests/perf/synthetic_skills.rs  # Task 1 — fixture generator
  - crates/tome/src/lib.rs                     # Task 1 — manifest module lifted to pub
tech-stack:
  added:
    - "rand 0.9 (dev-dependency on tome-desktop) — seeded fixture RNG"
metrics:
  duration: in-progress
  started: "2026-05-29T11:14:31Z"
---

# Phase 26 Plan 08: Alpha-cut perf-bench harness — Summary (in progress)

The closing-out plan for the read-only-views alpha cut: a Rust-generated 2000-skill synthetic fixture + a Playwright FPS bench that asserts NF-01 (search-as-you-type sustains 60fps on a real Skills view) + a macOS-only GitHub Actions workflow that runs it on PRs touching `ui/` or `tests/perf/`.

## Progress

- [x] **Task 1** — Rust synthetic 2000-skill fixture generator
- [x] **Task 2** — Playwright FPS sampler + 60fps-search.spec.ts + Tauri mock extension
- [ ] **Task 3** — `.github/workflows/perf.yml` (macOS-only CI)

## Task commits

| Task | Hash | Subject |
|---|---|---|
| 1 | `8c021da` | `test(26-08): synthetic 2000-skill fixture generator (NF-01 setup)` |
| 1+ | `746da6d` | `docs(26-08): start SUMMARY (Task 1 of 3 complete)` |
| 2 | `b534a0d` | `test(26-08): Playwright 60fps perf bench + Tauri mock extension (NF-01)` |

## What ships in Task 1

A Cargo integration test at `crates/tome-desktop/tests/perf/synthetic_skills.rs` that, when invoked with `PERF_FIXTURE_OUT=<path>`, materialises a deterministic 2000-skill tome library at that path:

- `<path>/library/skill-NNNN/SKILL.md` — 2000 directories with random-length lorem-ipsum bodies (100–5000 chars, seeded RNG).
- `<path>/.tome-manifest.json` — built through the canonical `tome::manifest` API so any future drift in `SkillEntry`'s serde shape breaks the fixture at write time, not at `tome::manifest::load` time downstream. Round-trip-verified by the test itself.
- `<path>/tome.toml` — one `[directories.synthetic]` entry pointing at `<path>/library`.
- `<path>/perf-skills.json` — a flat array of `DiscoveredSkill`-shaped objects (the wire shape the Tauri `list_skills` command emits). Task 2's Vite mock will read this at build time when `PERF_TEST=1`.

The test gates on `PERF_FIXTURE_OUT` and prints a skip message when the env var is unset. That keeps it out of `cargo test --all` / `make ci` (CLAUDE.md constraint 11 — perf benches must NOT run as part of the standard test matrix).

### Two cross-cutting visibility tweaks

- **`tome::manifest` lifted from `pub(crate)` to `pub`** so the fixture can construct `Manifest` + `SkillEntry` rows through the canonical public API instead of hand-crafting JSON. The narrow surface (`Manifest`, `SkillEntry`, `SkillOwnership`, `load`, `save`) matches the precedent set in plan 26-02 (lifting `tome::list`). `MANIFEST_FILENAME` stays `pub(crate)`.
- **Explicit `[[test]]` target declaration** in `crates/tome-desktop/Cargo.toml` because Cargo's auto-discovery doesn't traverse `tests/perf/`. Keeping the file under `tests/perf/` (rather than `tests/`) groups it with the Playwright spec + sampler that will land in Task 2.

### Local verification (Task 1)

```text
PERF_FIXTURE_OUT=/tmp/tome-perf-fixture \\
  cargo test -p tome-desktop --test synthetic_skills -- --nocapture
→ test setup_perf_fixture ... ok
→ 2000 skill dirs, 2000 SKILL.md, 244K perf-skills.json, 652K manifest
cargo clippy -p tome --all-targets -- -D warnings → clean
cargo clippy -p tome-desktop --tests -- -D warnings → clean
cargo test -p tome --lib → 909 passed (no regressions from the pub lift)
unset PERF_FIXTURE_OUT && cargo test -p tome-desktop --test synthetic_skills → skips silently
```

## Deviations from plan (so far)

### Auto-fixed during execution

**1. [Rule 3 - Blocking] `tome::manifest` was `pub(crate)`**
- **Found during:** Task 1 (test compile).
- **Fix:** Lifted module to `pub` (precedent: plan 26-02 lifted `tome::list`). Narrow surface, documented in a leading comment.
- **Commit:** `8c021da`.

**2. [Rule 3 - Blocking] Cargo can't auto-discover tests under `tests/perf/`**
- **Found during:** Task 1 (first `cargo test --test synthetic_skills` invocation reports "no test target named `synthetic_skills`").
- **Fix:** Explicit `[[test]] name = "synthetic_skills" path = "tests/perf/synthetic_skills.rs"` in `crates/tome-desktop/Cargo.toml`. Keeps the file in its planned location (alongside the upcoming Playwright spec) while making the target discoverable.
- **Commit:** `8c021da`.

**3. [Logged as deferred, NOT auto-fixed] `cargo fmt --all -- --check` surfaced pre-existing drift**
- **Found during:** Task 1 (`cargo fmt` after edits).
- **Files affected (pre-existing, not my edits):** `crates/tome/src/doctor.rs` (4 spots), `crates/tome/src/skill.rs` (1 spot), `crates/tome-desktop/src/commands.rs` (import order).
- **Why deferred:** Out of scope per executor scope-boundary rules. These were on `main` before this plan started.
- **Action:** Logged to `.planning/phases/26-read-only-views-alpha-cut/deferred-items.md` for a follow-up cleanup.

## What ships in Task 2

A Playwright spec at `crates/tome-desktop/tests/perf/60fps-search.spec.ts` that loads the SkillsView against the 2000-skill perf fixture, types a 3-character query at 15ms inter-keystroke delay, samples `requestAnimationFrame` deltas for 2 seconds, and asserts p95 inter-frame delta < 18ms.

Architecture mirrors plan 26-07's a11y gate (path A from 26-RESEARCH.md):

- **`fps-sampler.js`** is plain JS (Playwright's `addInitScript({ path: ... })` reads it verbatim into the page — no TS transpilation). Exposes `window.__startFpsSampler(durationMs)` so the bench resets the frame buffer and starts a fresh window right before typing. Manual start prevents first-paint cost from skewing the p95.
- **`60fps-search.spec.ts`** navigates via the Sidebar (`role="option"`), waits for the React Aria ListBox to render, focuses the SearchField (via `getByRole("searchbox", { name: "Search skills" })` — the aria-label is on the wrapper, the searchbox role is on the inner input; CSS attribute selectors can't see both at once), starts the sampler, types `"tdd"`, waits 2.5s, reads `window.__fpsFrames`, computes p50/p95/max, appends a row to `26-PERF-RUNS.tsv`, asserts `p95 < 18`.
- **`playwright.config.ts`** runs Vite via the new `npm run dev:perf` script (sets `PERF_TEST=1`), uses workers=1 + no retries (FPS noise must surface), and inherits the `PW_USE_SYSTEM_CHROME=1` system-Chrome fallback added in 26-07.
- **`vite.config.ts`** adds `PERF_TEST=1` alongside the existing `A11Y_TEST=1`; both share the same Tauri mock modules. The new `define` entry pre-substitutes `import.meta.env.VITE_PERF_TEST` at build time so the mock's `commands.listSkills` arm branches between the 3-skill a11y fixture and the 2000-skill perf fixture without runtime cost.
- **`tauri-api-core.ts`** extended: imports a JSON sibling `perf-skills.json` and selects via `VITE_PERF_TEST`. A 3-skill in-tree stub keeps the import resolving in unrelated builds; the bench harness copies the real 2000-skill `${PERF_FIXTURE_OUT}/perf-skills.json` over it before runs.
- **`26-PERF-REPORT.md`** seeded with methodology, OQ-1 status, baseline runs, and the boundary-case OQ-1 evaluation (see below).
- **`.gitignore`** covers the rolling `26-PERF-RUNS.tsv` log + the per-run Playwright artefact dirs.

### Local verification (Task 2)

```text
cd crates/tome-desktop/ui && npx tsc --noEmit                    → clean
cd crates/tome-desktop/ui && npm test                            → 5/5 vitest
cd crates/tome-desktop/ui && A11Y_TEST=1 npm run build           → clean (a11y mode still works)
cd crates/tome-desktop/ui && PERF_TEST=1 npm run build           → clean (perf mode bundles)
cargo clippy --all-targets -- -D warnings                        → clean
4× PERF_TEST=1 npx playwright test --config=...                  → all 4 ran end-to-end
```

### First-run perf baseline (LOCAL HARDWARE, NOT CI)

Four runs on an Apple Silicon M-series Mac in headed dev mode (Vite watcher + browser dev-tools live):

| Run | Samples | p50 (ms) | p95 (ms) | max (ms) | Dropped (>18ms) |
|---|---|---|---|---|---|
| 1 | 121 | 16.70 | 18.20 | 31.73 | 9 |
| 2 | 121 | 16.70 | 18.10 | 21.59 | 9 |
| 3 | 121 | 16.70 | 18.10 | 18.60 | 7 |
| 4 | 121 | 16.70 | 18.40 | 18.70 | 13 |

**Result: boundary-case FAIL.** p50 is exactly 60fps (16.70ms — vsync 60Hz); p95 lands 0.1–0.4ms above the 18ms target across all four runs. This is the kind of result the plan explicitly anticipated — see PLAN.md 26-08 §"Hardware" wording around "55fps p95".

### OQ-1 evaluation — the bench's first verdict

26-PERF-REPORT.md §"OQ-1 evaluation" lays out the two interpretations:

1. **The threshold is tuned a hair too tight for the React Aria path under local-hardware noise.** The plan's own commentary calls strict 60fps unrealistic; 18ms was chosen as a slack-bearing approximation. The empirical local result puts us 0.1–0.4ms past that approximation on a contended laptop — well inside measurement noise. CI on macos-latest dedicated Apple Silicon (Assumption A12) is the source of truth.
2. **Measurable steady-state regression vs path B (TanStack Virtual).** The fallback documented in 26-RESEARCH.md §"Standard Stack — Virtualisation" is a non-trivial SkillsView refactor — out of scope for plan 26-08 per executor instructions, but on the table if CI numbers also show the boundary failure.

**Decision in this plan:** the 18ms target in `60fps-search.spec.ts` is **unchanged**. CI will fail on PR if the result lands above it — that failure surface is the signal the plan asked for. SkillsView is NOT refactored to TanStack here; that decision goes to the next milestone's planning phase if CI confirms a real regression.

## Deviations from plan (Task 2)

### Auto-fixed during execution

**4. [Rule 3 - Blocking] CSS attribute selector `[role="searchbox"][aria-label="Search skills"]` doesn't match**
- **Found during:** Task 2 (first Playwright run — `page.focus` timed out at 30s).
- **Investigation:** React Aria's `<AriaSearchField>` puts the `aria-label="Search skills"` on the OUTER wrapper while the inner `<input type="search">` carries the implicit `role="searchbox"`. The two live on different DOM nodes, so the combined CSS selector can't match either.
- **Fix:** Switched to `page.getByRole("searchbox", { name: "Search skills" }).focus()` — `getByRole`'s accessible-name tree walks parent labels into child inputs, so the inner input inherits the wrapper's label and resolves correctly. Same pattern as the 26-07 a11y spec.
- **Files modified:** `crates/tome-desktop/tests/perf/60fps-search.spec.ts`.
- **Commit:** `b534a0d`.

**5. [Rule 3 - Blocking] `addInitScript({ path: "fps-sampler.ts" })` failed because Playwright runs the file verbatim in the page — no TS transpilation**
- **Found during:** Task 2 (second Playwright run — `__startFpsSampler is not a function`).
- **Issue:** TS-specific syntax (`declare global`, `export {}`) don't parse as plain JavaScript in the browser. The file ran but the side effects that should have set the window globals didn't fire.
- **Fix:** Renamed `fps-sampler.ts` → `fps-sampler.js` (plain JS, no TS-specific syntax). Type declarations for the window globals moved to the top of `60fps-search.spec.ts` so the spec's `page.evaluate(() => window.__startFpsSampler(...))` calls still type-check.
- **Files modified:** `crates/tome-desktop/tests/perf/fps-sampler.js` (created), `crates/tome-desktop/tests/perf/60fps-search.spec.ts`.
- **Commit:** `b534a0d`.

**6. [Rule 2 - Critical] `fs.appendFileSync` on 26-PERF-REPORT.md lands rows AFTER later prose sections**
- **Found during:** Task 2 (after the first run, the appended baseline row landed after the new "OQ-1 evaluation" section, breaking the report's structure).
- **Issue:** Markdown reports have a structure; appending to file end ignores it. The original plan's `fs.appendFileSync` approach in §Task 2(g) assumes the report is rows-only forever.
- **Fix:** Bench now writes to a separate `26-PERF-RUNS.tsv` log (gitignored). The TSV captures raw run history; the human (or a future CI step) promotes representative rows into 26-PERF-REPORT.md's "Baseline runs" table during review. CI uploads the TSV as a workflow artefact.
- **Files modified:** `crates/tome-desktop/tests/perf/60fps-search.spec.ts`, `.gitignore`, `.planning/phases/26-read-only-views-alpha-cut/26-PERF-REPORT.md`.
- **Commit:** `b534a0d`.

### Deliberately NOT auto-fixed

**7. [Rule 4 boundary] p95 = 18.10–18.40ms on local hardware — boundary-case FAIL**
- **Found during:** Task 2 (four local runs, all FAIL by 0.1–0.4ms).
- **Why deferred:** Plan's executor instructions explicitly say: "Don't auto-rewrite SkillsView; that's a separate decision." TanStack Virtual swap is the path-B fallback per 26-RESEARCH.md but it's a substantial refactor. CI on macos-latest (Task 3 next) is the source-of-truth measurement; local laptop with Vite watcher live is contention-noise-prone.
- **Resolution:** 26-PERF-REPORT.md §"OQ-1 evaluation" documents both interpretations + the decision path forward. The 18ms target in the spec is unchanged — CI will surface the truth on first PR run.

(Task 3 to follow — CI workflow wiring.)
