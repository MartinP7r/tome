---
phase: 15-cli-hardening
plan: 05
subsystem: browse-tui
tags: [ratatui, test-backend, insta, snapshot-tests, machine-toml, smart-routing, status-message, mach-04, hard-12, hard-21]

# Dependency graph
requires:
  - phase: 15-cli-hardening
    provides: 15-01 cmd_<name> dispatch helpers (cmd_browse signature is the integration point for new MachinePrefs threading); per-domain tests/cli_*.rs split (cli_browse.rs already exists)
  - phase: 15-cli-hardening
    provides: 15-04 atomic-save preservation pattern in machine.rs (D-TILDE-2 fence preserves override paths verbatim); MachinePrefs::save unchanged contract
  - phase: 09-cross-machine-path-overrides
    provides: PORT-01..05 [directory_overrides.<name>] schema + DirectoryOverride type — referenced by HARD-21 unit tests
  - phase: 10-phase-8-review-tail
    provides: POLISH-02 StatusMessage enum (Success | Warning | Pending) — apply_toggle uses StatusMessage::Success(body) per D-BROWSE-3 step 4
  - phase: 14-unowned-library-lifecycle
    provides: D-C1 SkillRow.source_directory: Option<DirectoryName> — None falls through to Global toggle scope (Unowned skills have no parent dir)
provides:
  - "ratatui::backend::TestBackend + insta snapshot harness in crates/tome/tests/browse_snapshots.rs (13 tests covering status dashboard, skill list default/empty/filtered/grouped, detail pane managed/local/unowned, help overlay, light theme, post-toggle)"
  - "App::for_snapshot / enter_detail_mode_for_snapshot / enter_help_mode_for_snapshot / refilter_for_snapshot / execute_action_for_snapshot — feature-gated test-support fixture API"
  - "DetailAction::label(self, &SkillRow, &MachinePrefs) -> String — context-sensitive ACTION-MENU LABEL per D-BROWSE-2 (verb + scope, NEVER includes skill name)"
  - "DetailAction::fallback_label() -> &'static str — static fallback for legacy callers without prefs wired"
  - "ToggleScope { Global, PerDirBlocklist(DirectoryName), PerDirAllowlist(DirectoryName) } + ToggleScope::resolve(&row, &prefs) — D-BROWSE-1 smart-routing"
  - "current_toggle_action(&row, &prefs) -> DetailAction — D-BROWSE-3 step 3 selector for the action menu (returns Disable XOR Enable, never both)"
  - "App::with_machine_prefs(prefs, path) builder + machine_prefs/machine_path fields"
  - "App::apply_toggle(action) -> anyhow::Result<()> — full 4-step flow (mutate in-memory + atomic save + label flip + StatusMessage::Success body)"
  - "MachinePrefs accessors: directory_prefs() free fn + DirectoryPrefs::{disabled, enabled}_set()"
  - "MachinePrefs mutators: toggle_global_disabled / toggle_per_dir_blocklist / toggle_per_dir_allowlist (allowlist with inverted polarity)"
  - "browse module + machine module widened to pub under feature 'test-support' (production builds keep pub(crate) byte-for-byte)"
  - "SkillRow.source_directory: Option<DirectoryName> field for parent-directory tracking"
affects:
  - 15-06 (polish + older bugs): no overlap — 15-06 touches manifest.rs, wizard.rs, relocate.rs, reassign.rs, backup.rs; this plan touched browse/, machine.rs, lib.rs cmd_browse only
  - "v1.0 GUI Tauri IPC: feature-gated test-support widening keeps the surface OUT of production builds; v1.0 will need to decide whether to expose `App` / `MachinePrefs` to JS or wrap them in stable IPC types"

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "ratatui::backend::TestBackend + per-test full-buffer snapshot under insta — locks visual contract end-to-end (layout + theming + fuzzy highlighting + markdown preview interactions)"
    - "Feature-gated test-support widening (browse + machine modules go from pub(crate) → pub under feature = test-support) — same pattern as Phase 13's marketplace::testing"
    - "Context-sensitive enum label method (DetailAction::label takes &SkillRow + &MachinePrefs and returns String) — pattern reusable for future TUI actions whose label depends on world state"
    - "ToggleScope smart-routing enum with locality-principle resolution (per-dir blocklist > per-dir allowlist > global) — mirrors MachinePrefs::is_skill_allowed but for the *write* path"
    - "Twin-string D-BROWSE-2/D-BROWSE-3 contract: action-menu LABEL has NO skill name (verb + scope only); StatusMessage BODY has skill name (verb + skill + scope). Each is unit-tested with verbatim assertions to prevent the strings from drifting."

key-files:
  created:
    - "crates/tome/tests/browse_snapshots.rs (13 ratatui TestBackend snapshot tests, 120x40)"
    - "crates/tome/tests/snapshots/browse_snapshots__snapshot_status_dashboard_default.snap"
    - "crates/tome/tests/snapshots/browse_snapshots__snapshot_skill_list_default.snap"
    - "crates/tome/tests/snapshots/browse_snapshots__snapshot_skill_list_empty.snap"
    - "crates/tome/tests/snapshots/browse_snapshots__snapshot_skill_list_filtered.snap"
    - "crates/tome/tests/snapshots/browse_snapshots__snapshot_skill_list_grouped_by_source.snap"
    - "crates/tome/tests/snapshots/browse_snapshots__snapshot_detail_pane_managed_skill.snap"
    - "crates/tome/tests/snapshots/browse_snapshots__snapshot_detail_pane_local_skill.snap"
    - "crates/tome/tests/snapshots/browse_snapshots__snapshot_detail_pane_unowned_skill.snap"
    - "crates/tome/tests/snapshots/browse_snapshots__snapshot_help_overlay_default.snap"
    - "crates/tome/tests/snapshots/browse_snapshots__snapshot_theme_light_status_dashboard.snap"
    - "crates/tome/tests/snapshots/browse_snapshots__snapshot_theme_light_skill_list.snap"
    - "crates/tome/tests/snapshots/browse_snapshots__snapshot_theme_light_filtered.snap"
    - "crates/tome/tests/snapshots/browse_snapshots__snapshot_detail_pane_after_disable_toggle.snap (HARD-21 post-toggle fixture)"
  modified:
    - "crates/tome/src/lib.rs — browse + machine modules widened to pub under feature 'test-support'; cmd_browse signature gains machine_prefs + machine_path; dispatch loads MachinePrefs before invoking browse"
    - "crates/tome/src/browse/mod.rs — app + theme + ui submodules widened to pub under feature 'test-support'; browse() signature gains machine_prefs + machine_path; SkillRow.source_directory wired from DiscoveredSkill.source_name"
    - "crates/tome/src/browse/app.rs — DetailAction::label refactored to context-sensitive String; fallback_label added; ToggleScope enum + resolve(); current_toggle_action(); SkillRow.source_directory field; App.machine_prefs/machine_path fields; with_machine_prefs builder; apply_toggle() 4-step flow; refresh_detail_actions(); selected_skill_row() helper; for_snapshot / enter_detail_mode_for_snapshot / enter_help_mode_for_snapshot / refilter_for_snapshot / execute_action_for_snapshot test-support API; #[allow(dead_code)] dropped from DetailAction; 19 new HARD-21 unit tests (smart-routing + label + status body + 4-step flow + MACH-04 + no-dead-code)"
    - "crates/tome/src/browse/ui.rs — render_detail Actions list calls action.label(row, prefs) when both are available; falls back to action.fallback_label() otherwise"
    - "crates/tome/src/browse/fuzzy.rs — SkillRow test fixtures gain source_directory: None"
    - "crates/tome/src/machine.rs — directory_prefs() free fn + DirectoryPrefs::{disabled, enabled}_set() accessors; toggle_global_disabled / toggle_per_dir_blocklist / toggle_per_dir_allowlist mutators (allowlist with inverted polarity)"

key-decisions:
  - "Feature-gated test-support widening for browse + machine modules (NOT pub-by-default): the v1.0 Tauri IPC surface should not bind to App / MachinePrefs by accident. The integration snapshot tests in tests/browse_snapshots.rs need to construct App + MachinePrefs, but production builds keep the existing pub(crate) visibility byte-for-byte. Same pattern as Phase 13's marketplace::testing."
  - "DetailAction::label refactored to take (self, &SkillRow, &MachinePrefs) and return String, NOT a sibling label_with_scope() method. The plan suggested either path; the chosen one keeps the public surface to a single label method (no risk of callers using the wrong one). Existing callers — only browse/ui.rs:render_detail — were updated to thread row + prefs through. A `fallback_label() -> &'static str` covers legacy paths (no row selected, prefs not wired) so the fixture-based unit tests don't need to construct full prefs to exercise the action menu."
  - "App.machine_prefs and App.machine_path are Option<...>, NOT mandatory. This keeps backward compatibility with the v0.6+ unit-test surface (which constructs App via App::new(rows) without prefs); apply_toggle returns an Err with a clear message if either is None. Production cmd_browse always passes Some(...) via with_machine_prefs."
  - "SkillRow.source_directory is Option<DirectoryName> rather than a string: matches Phase 14 D-C1 (Unowned skills have None) and prevents accidental string-comparison bugs against `tome.toml::directories.<name>` keys. The browse module pipes DiscoveredSkill.source_name (already a DirectoryName) directly into the field."
  - "ToggleScope.resolve uses *most-specific-wins* logic (per-dir blocklist > per-dir allowlist > global). This mirrors MachinePrefs::is_skill_allowed (the *read* path) so the read/write semantics stay symmetric. Empty `disabled` BTreeSet treated as 'no per-dir list' (falls through to allowlist or global) so the resolver doesn't pin a directory to blocklist scope just because the field defaulted to empty."
  - "MACH-04 invariant preserved by construction: the toggle methods on MachinePrefs only ever touch ONE of `disabled` / `enabled` per directory. apply_toggle dispatches through ToggleScope to pick which mutator runs; the mutators themselves cannot violate the invariant because they each only touch their own field. Regression test toggle_never_sets_both_disabled_and_enabled validates after a Disable+Enable round-trip via prefs.validate()."
  - "Snapshot terminal size fixed at 120x40 with right-trimming of trailing spaces per row. Width fits typical `~/.tome/library/<name>` paths without column wrapping; height fits the help overlay (18 rows) plus surrounding chrome. Right-trim avoids baking a 120-wide whitespace rectangle into every snapshot — the line break is implicit. Documented at the top of browse_snapshots.rs so future contributors don't churn the size."
  - "for_snapshot constructs an App with a stable preview body (filesystem-independent) so snapshots don't flake based on whether the test runner actually has the SKILL.md files referenced in the fixtures. Theme is injected (NOT Theme::detect()) to avoid $COLORFGBG env-var dependency. refilter_for_snapshot re-stamps the preview body after refilter() runs (refilter calls refresh_preview which would touch the filesystem)."
  - "Post-toggle snapshot test (snapshot_detail_pane_after_disable_toggle) routes through execute_action_for_snapshot to mirror the production keyflow exactly — apply_toggle runs, then refresh_detail_actions flips slot 2 from Disable to Enable, then the next render shows both the new label AND the StatusMessage::Success body. The snapshot locks both visual signals in one frame."

metrics:
  duration_estimate: ~1.5h
  completed_at: 2026-05-08
  tests_added: 33  # 12 initial snapshots + 1 post-toggle snapshot + 19 unit tests + 1 import-compile sentinel
  tests_total_browse: 76  # was 56 pre-plan
  files_created: 14  # tests/browse_snapshots.rs + 13 .snap fixtures
  files_modified: 6  # lib.rs, browse/{mod,app,ui,fuzzy}.rs, machine.rs

issues_closed:
  - "#498 (HARD-12: ratatui TestBackend + insta snapshots for browse/ui.rs)"
  - "#447 (HARD-21: browse Disable/Enable wired per D-BROWSE-1..3)"
---

# Phase 15 Plan 05: Browse UI Snapshot Coverage + Disable/Enable Wiring Summary

ratatui `TestBackend` + `insta` snapshots lock the visual regression contract for `browse/ui.rs` across 13 canonical scenes (status dashboard, skill list default/empty/filtered/grouped, detail pane managed/local/unowned, help overlay, light theme, post-toggle), and `DetailAction::{Disable, Enable}` are wired end-to-end with smart-routing per `D-BROWSE-1` (most-specific list wins), context-sensitive action-menu labels per `D-BROWSE-2` (verb + scope, NEVER skill name), and the 4-step toggle flow per `D-BROWSE-3` (mutate in-memory + atomic save + label flip + `StatusMessage::Success` body — verb + skill + scope, distinct from the label).

## Coverage Map

| Requirement | Surface | Test signal |
|-------------|---------|-------------|
| **HARD-12** | `crates/tome/tests/browse_snapshots.rs` (13 snapshots, 120x40 TestBackend) | All 13 pass deterministically; `cargo clippy --all-targets -- -D warnings` clean |
| **HARD-21 D-BROWSE-1** | `ToggleScope::resolve` in `app.rs` | 4 smart-routing tests (`apply_toggle_global_when_no_per_dir_list`, `apply_toggle_per_dir_blocklist`, `apply_toggle_per_dir_allowlist_inverted_polarity`, `apply_toggle_undo_via_inverse`) |
| **HARD-21 D-BROWSE-2** | `DetailAction::label(&row, &prefs) -> String` | 5 label assertion tests including negative `label_does_not_contain_skill_name` |
| **HARD-21 D-BROWSE-3 step 1** | `apply_toggle` mutates in-memory | `apply_toggle_step1_mutates_in_memory` |
| **HARD-21 D-BROWSE-3 step 2** | `machine::save` round-trip | `apply_toggle_step2_atomic_save_round_trip` |
| **HARD-21 D-BROWSE-3 step 3** | `current_toggle_action` selector + `refresh_detail_actions` | `apply_toggle_step3_label_flips` |
| **HARD-21 D-BROWSE-3 step 4** | `StatusMessage::Success` body shapes | 4 status-body tests + `apply_toggle_step4_surfaces_success_status` |
| **MACH-04 invariant** | `MachinePrefs::validate` after toggle round-trip | `toggle_never_sets_both_disabled_and_enabled` |
| **No-dead-code attr** | Source-text scan around `pub enum DetailAction` | `no_dead_code_attr_above_detail_action` |

## Action-menu LABEL shapes (D-BROWSE-2 — verbatim, NO skill name)

```
Global toggle:           "Disable on this machine"   /  "Enable on this machine"
Per-directory blocklist: "Disable for <dir-name>"    /  "Enable for <dir-name>"
Per-directory allowlist: "Disable for <dir-name>"    /  "Enable for <dir-name>"
```

The skill name is **deliberately absent** from the label. The label tells the user *what kind* of mutation will happen (and to what scope); the skill name is implicit from the row they're looking at. Negative test `label_does_not_contain_skill_name` enforces this.

## Status-message BODY shapes (D-BROWSE-3 step 4 — verbatim, includes skill name)

```
Global toggle:           "Disabled <skill> on this machine"   /  "Enabled <skill> on this machine"
Per-directory:           "Disabled <skill> for <dir-name>"    /  "Enabled <skill> for <dir-name>"
```

The body **does** include the skill name — the user sees confirmation of *which skill* was toggled, *what verb* was applied, and *to which scope*. Composed via `format!("Disabled {row_name} on this machine")` etc. inside `apply_toggle`.

## D-BROWSE-3 4-step flow (one assertion per step)

| Step | Action | Test |
|-----:|--------|------|
| 1 | Mutate `MachinePrefs` in-memory | `apply_toggle_step1_mutates_in_memory` — asserts `prefs.is_disabled("foo")` flips |
| 2 | Atomic save to `machine.toml` | `apply_toggle_step2_atomic_save_round_trip` — load + re-read + re-assert |
| 3 | Re-render the row's action label | `apply_toggle_step3_label_flips` — `current_toggle_action(&row, &prefs)` flips Disable → Enable; label flips "Disable on this machine" → "Enable on this machine" |
| 4 | Surface `StatusMessage::Success` | `apply_toggle_step4_surfaces_success_status` — asserts `status_message == Some(Success(_))` with the verbatim body shape |

## ToggleScope resolution (D-BROWSE-1)

```rust
pub enum ToggleScope {
    Global,
    PerDirBlocklist(DirectoryName),
    PerDirAllowlist(DirectoryName),
}

impl ToggleScope {
    pub fn resolve(row: &SkillRow, prefs: &MachinePrefs) -> Self {
        // 1. Unowned skills (source_directory == None) → Global.
        // 2. Parent directory has non-empty `disabled` blocklist → PerDirBlocklist.
        // 3. Parent directory has `enabled` allowlist set → PerDirAllowlist (inverted polarity).
        // 4. Otherwise → Global.
    }
}
```

**Locality principle:** most-specific list wins. Mirrors the *read*-path semantics in `MachinePrefs::is_skill_allowed` so read and write stay symmetric.

## Self-Check: PASSED

- `crates/tome/tests/browse_snapshots.rs` — FOUND
- 14 `.snap` files in `crates/tome/tests/snapshots/` — FOUND
- 13 `fn snapshot_*` tests defined — FOUND (≥10 acceptance bar met)
- 19 HARD-21 unit tests in `browse::app::tests` — FOUND (≥13 acceptance bar met)
- `#[allow(dead_code)]` removed from `DetailAction` in `browse/app.rs` — VERIFIED via `no_dead_code_attr_above_detail_action` test
- `cargo test -p tome --test browse_snapshots` exits 0 (13 passed)
- `cargo test -p tome --lib browse::` exits 0 (76 passed, was 56 pre-plan)
- `cargo clippy --all-targets -- -D warnings` exits 0
- `cargo fmt --check` exits 0
- Commit `6944d1f` (Task 1 — HARD-12) — FOUND
- Commit `f4ce3a7` (Task 2 — HARD-21) — FOUND

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] `SkillName::new(s)` failed to compile from `&&str`**

- **Found during:** Task 2 unit-test compilation
- **Issue:** `SkillName::new` takes `impl Into<String>`. Passing the iterator binding `s: &&str` directly tripped the inference (`String: From<&&str>` doesn't exist).
- **Fix:** Dereference once — `SkillName::new(*s)` — to bridge to `&str`, which does have an `Into<String>` via `ToString`.
- **Files modified:** `crates/tome/src/browse/app.rs` (2 `seed_*` test helpers)
- **Commit:** `f4ce3a7`

**2. [Rule 1 - Bug] clippy `doc_lazy_continuation` and `doc_overindented_list_items` warnings**

- **Found during:** Task 2 final clippy run
- **Issue:** Doc-comment markdown formatting tripped two clippy lints in the new `DetailAction::label` and `execute_action_for_snapshot` doc strings.
- **Fix:** Reformatted the offending paragraphs (added blank lines around list items; replaced `+` symbol with "plus" word in one case).
- **Files modified:** `crates/tome/src/browse/app.rs`
- **Commit:** `f4ce3a7`

### Auto-added Missing Critical Functionality

**3. [Rule 2 - Missing] DetailAction::fallback_label() for prefs-less callers**

- **Found during:** Task 2 — refactoring `DetailAction::label` to take `(&row, &prefs)` broke `ui::render_detail` for the case where `app.machine_prefs == None` (legacy unit-test fixtures, transient empty-row state).
- **Issue:** The plan didn't specify a fallback for the no-prefs case, but blowing up via `unwrap()` in the renderer would be a correctness bug.
- **Fix:** Added `DetailAction::fallback_label() -> &'static str` returning the pre-HARD-21 static labels. `ui::render_detail` calls it when either no row is selected or no prefs are wired.
- **Files modified:** `crates/tome/src/browse/app.rs`, `crates/tome/src/browse/ui.rs`
- **Commit:** `f4ce3a7`

**4. [Rule 2 - Missing] Widened `machine` module visibility under test-support feature**

- **Found during:** Task 2 — the post-toggle snapshot test in `tests/browse_snapshots.rs` needed to construct a `MachinePrefs` to wire into the App.
- **Issue:** `machine` was `pub(crate)`, unreachable from integration tests.
- **Fix:** Same `cfg(any(test, feature = "test-support"))` widening already applied to `browse`. Production builds keep `pub(crate)` byte-for-byte.
- **Files modified:** `crates/tome/src/lib.rs`
- **Commit:** `f4ce3a7`

### Auth gates: NONE

No interactive auth required during this plan.

### Architectural changes: NONE

No Rule-4 architectural decisions taken. The widening of `browse` and `machine` modules under `test-support` is a non-breaking visibility change (production users see the same surface).

## Pre-existing flakes (NOT in scope)

`backup::tests::snapshot_creates_commit` and `git::tests::read_head_sha_returns_40_char_hex` flake intermittently in parallel runs (pass in isolation). Both are folded into HARD-14 in plan 15-06 — out of scope for this plan per the strict beta-cut policy (D-PLAN-2).

## Issues closed

- **#498** (HARD-12: ratatui TestBackend + insta snapshots for `browse/ui.rs`) — covered by 13 snapshot tests in `tests/browse_snapshots.rs`
- **#447** (HARD-21: browse Disable/Enable wired per D-BROWSE-1..3) — covered by `App::apply_toggle` + 19 unit tests + 1 post-toggle snapshot

## Commits

- `6944d1f` — `test(15-05): HARD-12 ratatui TestBackend + insta snapshots for browse/ui.rs`
- `f4ce3a7` — `feat(15-05): HARD-21 wire DetailAction::{Disable,Enable} per D-BROWSE-1..3`
