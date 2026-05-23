# Requirements: tome v0.11 — Polish + Observability

**Defined:** 2026-05-12
**Source:** Milestone discussion (no formal research; polish-heavy milestone against existing codebase)
**Predecessor:** [`milestones/v0.10-REQUIREMENTS.md`](milestones/v0.10-REQUIREMENTS.md) (SHIPPED 2026-05-11)

## v0.11 Requirements

Requirements for the v0.11 release. Grouped by category; each will map to one or more roadmap phases. Phase numbering continues from v0.10 → starts at Phase 18.

### Observability (OBS)

Adopt structured logging across the codebase so `tome sync`/`doctor`/`status` give clearer signal. Lays groundwork for v1.0 GUI's IPC + log-capture needs. Scope discipline: "instrument existing output" — not "redesign output."

- [x] **OBS-01**: Adopt `tracing` + `tracing-subscriber` crates. Replace internal `eprintln!`/`println!` chatter with `tracing::{info,warn,debug}!` calls. Wizard prompts, TUI browse output, and user-facing summary tables (status/list/doctor) stay as direct stdout — instrument only the *log-like* output (sync progress, cleanup actions, diagnostic warnings).
- [x] **OBS-02**: Wire `--verbose`/`--quiet` global flags + `TOME_LOG` env var to `tracing_subscriber::EnvFilter`. Default level `info`; `--verbose` → `debug`; `--quiet` → `warn`. Existing `LogLevel` enum (HARD-07) wraps the subscriber configuration; behavior preserved for users who only use the flags.
- [x] **OBS-03**: `tome sync` emits per-pipeline-step spans (discover, reconcile, consolidate, distribute, cleanup) with elapsed-ms attached. Visible in `--verbose` text output and reachable via `TOME_LOG=tome::sync=debug`.
- [x] **OBS-04**: Change-cause attribution — when consolidate or distribute re-emits a skill, the reason ("hash changed", "previously failed", "newly added", "directory now allowed") is logged at `info!` for user visibility.
- [x] **OBS-05**: Reconcile classification breakdown surfaced in `tome sync` summary — show counts of Match / Drift / Vanished / MissingFromMachine in the final summary block (not only the consolidated outcome).
- [x] **OBS-06**: `tome doctor` richer surface — categorize issues (Library / Directory / Config / Foreign-symlink) with per-category counts in text output; JSON shape gains `category` field per issue. Implementation overlaps with FIX-01 (#530); single change closes both.
- [x] **OBS-07**: `tome status` richer surface — surface per-directory skill counts, override status (already present in v0.9), and a "last sync" timestamp; JSON shape parity with text output.

### Bugfixes (FIX)

The v0.10-surfaced bug bundle and the older wizard-polish backlog. Each requirement closes one or more existing GitHub issues; full mapping in Traceability.

- [x] **FIX-01**: `tome doctor` "auto-fixable" count and prompt exclude items with no auto-repair available — the current "N auto-fixable issues" then "(no auto-repair available)" UX is a contradiction. Closes [#530](https://github.com/MartinP7r/tome/issues/530).
- [x] **FIX-02**: `browse::app::tests::copy_path_retry_helper_returns_within_bound` timing flake fixed — diagnose root cause (timing-based assertion under parallel contention) and replace with deterministic clock injection or relaxed bound with explicit comment. Closes [#511](https://github.com/MartinP7r/tome/issues/511).
- [x] **FIX-03**: `tome doctor` "N managed symlink(s) tracked in git" check removed or rewritten — v0.10 made managed skills real directory copies, so the check is stale (false-positive count post-migration). Closes [#532](https://github.com/MartinP7r/tome/issues/532).
- [x] **FIX-04**: Wizard summary table column misalignment — ANSI bold escapes are miscounted as visible width by `tabled` in interactive TTY mode. Use ANSI-aware width measurement (or strip ANSI before measuring). Closes [#454](https://github.com/MartinP7r/tome/issues/454).
- [x] **FIX-05**: Wizard `configure_library` and library-default derivation follow the resolved `tome_home` instead of hardcoding `~/.tome/skills` — when a user customizes `tome_home`, the library default should follow. Single fix closes both linked issues. Closes [#453](https://github.com/MartinP7r/tome/issues/453) + [#456](https://github.com/MartinP7r/tome/issues/456).
- [x] **FIX-06**: `make release` automatically stamps the release date in `CHANGELOG.md` — replaces `[Unreleased]` with `[X.Y.Z] - YYYY-MM-DD` during the version-bump PR. Closes [#533](https://github.com/MartinP7r/tome/issues/533).

## Future Requirements

Deferred from v0.11 scoping; surface as candidates for v0.12 / v1.0 / v2.

- **OBS-FUTURE-01**: JSON-by-default streaming log output (`--log-format json` or `TOME_LOG_FORMAT=json`) for machine consumers / future Tauri IPC. Tracing subscriber already supports this; flag-level integration deferred until a real consumer exists.
- **OBS-FUTURE-02**: OpenTelemetry export (`tracing-opentelemetry`) — not justified at single-user scale; surface again if multi-machine telemetry becomes a need.
- **FIX-FUTURE-01**: `Lockfile::version` silently accepts unknown values — future-compat time bomb ([#426](https://github.com/MartinP7r/tome/issues/426)). Small fix but not anchored to a v0.11 user-visible outcome; defer to v0.12 or fold into v1.0 prep.
- **FIX-FUTURE-02**: Selective items from Phase 11/12/13 review followup bundles ([#517](https://github.com/MartinP7r/tome/issues/517), [#518](https://github.com/MartinP7r/tome/issues/518), [#519](https://github.com/MartinP7r/tome/issues/519)) — bundle-level decisions, evaluate per-item during v1.0 prep.
- **WATCH-FUTURE-01**: File watcher for auto-sync ([#59](https://github.com/MartinP7r/tome/issues/59)) — carried from v0.10. Orthogonal product direction; v1.x or v2.

## Out of Scope (v0.11)

| Item | Reason |
|------|--------|
| Output redesign (new table layouts, JSON-by-default streams, OpenTelemetry export) | Observability scope is "instrument existing output" only. Output-shape changes belong in v1.0 GUI prep or a later polish milestone. |
| Tauri Desktop GUI | Deferred to v1.0 — drafted in `milestones/v1.0-{REQUIREMENTS,ROADMAP}.md`. v0.11 lays the logging substrate that v1.0's IPC + log capture will consume. |
| Linux runtime UAT (clipboard + xdg-open carry-over from v0.8) | Sixth consecutive milestone without Linux hardware. Formally deferred to **v1.0** where the Tauri build forces Linux access. Written rationale in `08-HUMAN-UAT.md` frontmatter. |
| Rust → TypeScript rewrite | Explored in untracked `docs/migration/rust-to-typescript-feature-inventory.md` (snapshot 2026-04-27 @ v0.8.2). Parked. v1.0 continues on Rust + Tauri. |
| Backward-compat shim for log flag changes | Per project policy (Backward compat: None). Flag changes will be release-noted; users adapt at the v0.11 boundary. |
| New marketplace adapters (npm, generic URL) | Carried from v0.10 future requirements. Trait shape designed for extensibility; ADP-FUTURE-01/02 defer indefinitely until a real consumer exists. |

## Traceability

Filled by `gsd-roadmapper` 2026-05-12. 13 requirements mapped across 2 phases (18–19), 100% coverage.

| Requirement | Phase | GitHub Issue | Status |
|-------------|-------|--------------|--------|
| OBS-01 | Phase 18 | — | Done |
| OBS-02 | Phase 18 | — | Done |
| OBS-03 | Phase 18 | — | Done |
| OBS-04 | Phase 18 | — | Done |
| OBS-05 | Phase 18 | — | Done |
| OBS-06 | Phase 19 | — | Done |
| OBS-07 | Phase 19 | — | Done |
| FIX-01 | Phase 19 | [#530](https://github.com/MartinP7r/tome/issues/530) | Done |
| FIX-02 | Phase 19 | [#511](https://github.com/MartinP7r/tome/issues/511) | Done |
| FIX-03 | Phase 19 | [#532](https://github.com/MartinP7r/tome/issues/532) | Done |
| FIX-04 | Phase 19 | [#454](https://github.com/MartinP7r/tome/issues/454) | Done |
| FIX-05 | Phase 19 | [#453](https://github.com/MartinP7r/tome/issues/453) + [#456](https://github.com/MartinP7r/tome/issues/456) | Done |
| FIX-06 | Phase 19 | [#533](https://github.com/MartinP7r/tome/issues/533) | Done |

**Coverage:**
- v0.11 requirements: 13 total (7 OBS + 6 FIX)
- Mapped to phases: 13 / 13 ✓
