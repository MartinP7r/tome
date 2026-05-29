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
- [ ] **Task 2** — Playwright FPS sampler + 60fps-search.spec.ts + Tauri mock extension
- [ ] **Task 3** — `.github/workflows/perf.yml` (macOS-only CI)

## Task 1 commit

| Hash | Subject |
|---|---|
| `8c021da` | `test(26-08): synthetic 2000-skill fixture generator (NF-01 setup)` |

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

---

(Tasks 2 + 3 to follow — this SUMMARY is updated incrementally per the connection-resilience rule.)
