---
phase: 15-cli-hardening
plan: 05
type: execute
wave: 2
depends_on:
  - 15-01
files_modified:
  - crates/tome/src/browse/app.rs
  - crates/tome/src/browse/ui.rs
  - crates/tome/src/browse/mod.rs
  - crates/tome/src/machine.rs
  - crates/tome/tests/browse_snapshots/mod.rs
  - crates/tome/Cargo.toml
autonomous: true
requirements:
  - HARD-12
  - HARD-21
must_haves:
  truths:
    - "browse/ui.rs has ratatui TestBackend + insta snapshot tests covering status dashboard, skill list, detail pane, help overlay, empty state, search-filter state, and theme variants"
    - "DetailAction::Disable and DetailAction::Enable are wired up — no #[allow(dead_code)] at browse/app.rs:168"
    - "Toggle smart-routes per D-BROWSE-1: per-directory disabled blocklist OR per-directory enabled allowlist OR global MachinePrefs.disabled, never both"
    - "DetailAction::label() returns the ACTION MENU LABEL (verb + scope, NO skill name) per D-BROWSE-2; status-message body is a SEPARATE string (verb + skill + scope) per D-BROWSE-3 step 4"
    - "After toggle, all 4 D-BROWSE-3 steps fire: (1) MachinePrefs mutated in-memory, (2) machine.toml saved atomically, (3) row's action label flips Disable ↔ Enable on next render, (4) StatusMessage::Success surfaces with scope-explicit body"
  artifacts:
    - path: "crates/tome/src/browse/app.rs"
      provides: "Wired DetailAction::{Disable, Enable} with smart-routing toggle"
      contains: "DetailAction::Disable"
    - path: "crates/tome/tests/browse_snapshots/mod.rs"
      provides: "ratatui TestBackend + insta snapshot tests"
    - path: "crates/tome/Cargo.toml"
      provides: "ratatui TestBackend feature flag (if not already enabled)"
  key_links:
    - from: "crates/tome/src/browse/app.rs::DetailAction"
      to: "crates/tome/src/machine.rs::MachinePrefs"
      via: "smart-routing apply_toggle"
      pattern: "directories.*\\.(disabled|enabled)"
    - from: "crates/tome/src/browse/app.rs"
      to: "crates/tome/src/browse/ui.rs::StatusMessage"
      via: "POLISH-02 Success variant"
      pattern: "StatusMessage::Success"
---

<objective>
Add ratatui TestBackend + insta snapshot test coverage for `browse/ui.rs` (HARD-12, closes #498) and wire the stubbed `DetailAction::{Disable, Enable}` actions per D-BROWSE-1/-2/-3 smart-routing rules (HARD-21, closes #447).

Purpose: Lock visual regressions on the browse UI; deliver the long-stubbed Disable/Enable toggle that respects MACH-04 mutual-exclusion and surfaces scope-explicit StatusMessage feedback.
Output: New `tests/browse_snapshots/` insta corpus; wired DetailAction handlers in `browse/app.rs`; context-sensitive `DetailAction::label()` (action-menu label, NO skill name); StatusMessage::Success body (verb + skill + scope) on toggle.
</objective>

<execution_context>
@$HOME/.claude/get-shit-done/workflows/execute-plan.md
@$HOME/.claude/get-shit-done/templates/summary.md
</execution_context>

<context>
@.planning/PROJECT.md
@.planning/ROADMAP.md
@.planning/STATE.md
@.planning/REQUIREMENTS.md
@.planning/phases/15-cli-hardening/15-CONTEXT.md
@.planning/phases/15-cli-hardening/15-01-cli-decomposition-PLAN.md

@crates/tome/src/browse/app.rs
@crates/tome/src/browse/ui.rs
@crates/tome/src/browse/mod.rs
@crates/tome/src/machine.rs
@crates/tome/Cargo.toml

<interfaces>
Existing browse module surfaces (lines + signatures from 15-CONTEXT.md):

From crates/tome/src/browse/app.rs:
  // line 168: #[allow(dead_code)] on DetailAction::Disable / DetailAction::Enable variants
  // lines 178-186: DetailAction::label() returns &'static str (HARD-21 D-BROWSE-2 makes this context-sensitive)
  // lines 337-342: render-time comment "show Disable if enabled, Enable if disabled — never both"
  // line 167-186: DetailAction enum and its impl
  pub enum DetailAction { CopyPath, Disable, Enable, ... }
  impl DetailAction {
      pub fn label(&self) -> &'static str { ... }  // HARD-21 D-BROWSE-2 target — context-sensitive
  }

From crates/tome/src/machine.rs:
  pub struct DirectoryEntry {
      pub disabled: Option<BTreeSet<SkillName>>,  // blocklist (mutually exclusive with enabled per MACH-04)
      pub enabled: Option<BTreeSet<SkillName>>,   // allowlist (mutually exclusive with disabled)
  }
  pub struct MachinePrefs {
      pub disabled: BTreeSet<SkillName>,            // global blocklist
      pub directories: BTreeMap<DirectoryName, DirectoryEntry>,
  }
  // Existing API: pub fn disable_skill, pub fn is_disabled (global toggle)
  // Per-directory mutators already exist on DirectoryEntry per CONTEXT.md "Reusable assets".

From crates/tome/src/browse/ui.rs:
  // POLISH-02: StatusMessage enum
  pub enum StatusMessage { Success { body, glyph, severity }, Warning { ... }, Pending { ... } }

D-BROWSE-2 ACTION-MENU LABEL shapes (verbatim from CONTEXT.md lines 196-204) — what `DetailAction::label()` returns:
  Global toggle:           "Disable on this machine"   /  "Enable on this machine"
  Per-directory blocklist: "Disable for <dir-name>"    /  "Enable for <dir-name>"
  Per-directory allowlist: "Disable for <dir-name>"    /  "Enable for <dir-name>"  (label same; semantics differ)
  → Skill name does NOT appear in the action-menu label.

D-BROWSE-3 STATUS-MESSAGE BODY shapes (verbatim from CONTEXT.md line 230 + step 4) — what `StatusMessage::Success { body, .. }` carries:
  Global toggle:           "Disabled <skill> on this machine"   /  "Enabled <skill> on this machine"
  Per-directory:           "Disabled <skill> for <dir-name>"    /  "Enabled <skill> for <dir-name>"
  → Skill name DOES appear in the status-message body.

D-BROWSE-3 toggle flow (4 explicit steps, verbatim from CONTEXT.md lines 222-231):
  1. Mutate MachinePrefs in-memory.
  2. Save machine.toml atomically (existing temp+rename).
  3. Re-render the row's action label so it flips Disable <-> Enable immediately.
  4. Surface StatusMessage::Success with scope-explicit body in status bar; auto-fades per POLISH-02.

D-BROWSE-1 smart-routing logic (verbatim from CONTEXT.md):
  - If parent directory has disabled blocklist set → toggle that list (insert/remove)
  - Else if parent directory has enabled allowlist set → toggle allowlist (remove/insert; INVERTED polarity)
  - Else → toggle global MachinePrefs.disabled
</interfaces>
</context>

<tasks>

<task type="auto" tdd="true">
  <name>Task 1: HARD-12 ratatui TestBackend + insta snapshot tests for browse/ui.rs</name>
  <files>crates/tome/tests/browse_snapshots/mod.rs, crates/tome/Cargo.toml</files>
  <read_first>
    - crates/tome/src/browse/ui.rs (537 LOC — every render fn that produces a top-level frame: status dashboard, skill list, detail pane, help overlay)
    - crates/tome/src/browse/app.rs (App state struct used by render fns; SkillRow shape)
    - crates/tome/src/browse/theme.rs (light/dark theme variants — snapshot both)
    - crates/tome/src/browse/fuzzy.rs (search-filter state — feed a non-empty filter into a snapshot)
    - crates/tome/Cargo.toml (insta + ratatui — verify TestBackend feature is available; insta is already a dev-dep per the project tech stack)
    - .planning/phases/15-cli-hardening/15-CONTEXT.md section "HARD-12 browse snapshot scope" (Claude's Discretion: status dashboard, skill list, detail pane, help overlay, empty state, search-filter state, theme variants)
    - .planning/REQUIREMENTS.md section "HARD-12"
  </read_first>
  <behavior>
    Snapshot tests cover at minimum these scenarios:
    - status_dashboard_default: status row at the top of the browse view in a default (no-search, no-error, dark theme) state
    - skill_list_default: skill list with 3-5 fixture entries, dark theme, no filter
    - skill_list_empty: skill list with zero skills (empty-state messaging)
    - skill_list_filtered: skill list with a fuzzy filter active (e.g. "fo" with 2 matching skills)
    - detail_pane_managed_skill: detail pane for a managed skill (synced provenance)
    - detail_pane_local_skill: detail pane for a local skill
    - detail_pane_unowned_skill: detail pane for an unowned skill (Phase 14 D-C1 previous_source)
    - help_overlay_default: help overlay with all key bindings
    - theme_light_status_dashboard: status dashboard in light theme
    - theme_light_skill_list: skill list in light theme

    Each snapshot rendered into a ratatui::backend::TestBackend with fixed terminal dimensions (e.g. 80x40). Captured via insta::assert_snapshot!(buf_to_string(&backend)).
  </behavior>
  <action>
    **Step A: Confirm dev-dependencies.**

    Read `crates/tome/Cargo.toml` to confirm:
    - `insta` is a dev-dep (it is — listed in tech stack with json+filters features)
    - `ratatui` exposes the `TestBackend` (since 0.30, this is available without a feature flag — confirm by reading ratatui's actual prelude or running cargo doc).

    If TestBackend is feature-gated, add to Cargo.toml [dev-dependencies]:
    ```
    ratatui = { version = "0.30", features = ["test-backend"] }   # or similar — match actual feature name
    ```

    **Step B: Create test infrastructure.**

    New file: `crates/tome/tests/browse_snapshots/mod.rs` (or top-level `crates/tome/tests/browse_snapshots.rs` — pick consistent with the rest of tests/). Per CONTEXT.md `<canonical_refs>` "Codebase modules": "New `tests/browse_snapshots/` directory."

    Convention: insta snapshots land in a `snapshots/` sibling subdirectory next to the test file. Insta auto-discovers.

    Skeleton:

    ```rust
    use ratatui::Terminal;
    use ratatui::backend::TestBackend;
    use tome::browse::{render_browse_frame, App, ...};  // import the actual render entry point

    fn render_to_string(app: &App, theme: Theme, w: u16, h: u16) -> String {
        let backend = TestBackend::new(w, h);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal.draw(|f| render_browse_frame(f, app, &theme)).unwrap();
        let buf = terminal.backend().buffer();
        buf_to_string(buf)
    }

    fn buf_to_string(buf: &ratatui::buffer::Buffer) -> String {
        let mut s = String::new();
        for y in 0..buf.area.height {
            for x in 0..buf.area.width {
                s.push(buf.get(x, y).symbol().chars().next().unwrap_or(' '));
            }
            s.push('\n');
        }
        s
    }

    #[test]
    fn snapshot_status_dashboard_default() {
        let app = App::for_snapshot(/* fixture: 5 skills, no filter */);
        let out = render_to_string(&app, Theme::dark(), 80, 40);
        insta::assert_snapshot!(out);
    }

    // ... etc per <behavior>
    ```

    **Step C: Add a test-only `App::for_snapshot` constructor (or equivalent fixture factory).**

    The browse module's normal entry point reads from a real manifest + machine.toml + library. For snapshot tests, expose a `pub(crate)` (or `#[cfg(any(test, feature = "test-support"))]`) fixture builder that constructs an App directly from in-memory data:

    ```rust
    impl App {
        #[cfg(any(test, feature = "test-support"))]
        pub fn for_snapshot(skills: Vec<SkillRow>, filter: Option<String>) -> Self { ... }
    }
    ```

    Phase 13 used the `test-support` feature to widen `marketplace::testing` — same precedent applies here. Cargo.toml may already have `[features] test-support = []`. If not, add it (Plan 13-02 SUMMARY confirms it exists). The new fixture builder gates on `cfg(any(test, feature = "test-support"))`.

    **Step D: Write all snapshot tests.**

    Per `<behavior>` list. Each snapshot lands in `tests/browse_snapshots/snapshots/` (insta default). On first run, insta auto-creates `.snap.new` files that the executor reviews via `cargo insta review` or by visual inspection of the snapshot output. Once accepted, the `.snap` files commit alongside the test source.

    Run tests with `INSTA_FORCE_PASS=1` on first run to generate snapshots, then review and accept. Alternatively, use `cargo insta test --accept` if cargo-insta is installed.

    Test naming: prefix all with `snapshot_` so they group under `cargo test snapshot_`.

    Theme variants: write fixtures for both `Theme::dark()` and `Theme::light()`. CONTEXT.md explicitly lists "theme variants (light + dark)" as in-scope.
  </action>
  <verify>
    <automated>cargo test -p tome --test browse_snapshots 2>&amp;1 | tee /tmp/15-05-task1.log; cargo clippy --all-targets -- -D warnings</automated>
  </verify>
  <acceptance_criteria>
    - File `crates/tome/tests/browse_snapshots/mod.rs` (or `crates/tome/tests/browse_snapshots.rs`) exists.
    - Directory `crates/tome/tests/browse_snapshots/snapshots/` exists with `.snap` files committed (one per test).
    - At least 10 snapshot tests exist matching pattern `snapshot_*`: `grep -cE "fn snapshot_" crates/tome/tests/browse_snapshots/*.rs` returns ≥10.
    - Both themes covered: at least one `theme_light_*` and one default-dark snapshot exist.
    - Empty state covered: a `snapshot_skill_list_empty` (or similar) test exists.
    - Search-filter state covered: a `snapshot_skill_list_filtered` test exists.
    - All snapshots pass without diff: `cargo test -p tome --test browse_snapshots` exits 0.
    - `cargo clippy --all-targets -- -D warnings` exits 0.
    - `App::for_snapshot` (or equivalent fixture factory) is gated on `#[cfg(any(test, feature = "test-support"))]` to avoid leaking into v1.0 GUI surface.
  </acceptance_criteria>
  <done>
    `tests/browse_snapshots/` has insta snapshot coverage for the browse UI's primary states (status dashboard, skill list with default/empty/filtered, detail pane variants, help overlay, both themes). All snapshots pass.
  </done>
</task>

<task type="auto" tdd="true">
  <name>Task 2: HARD-21 wire DetailAction::{Disable, Enable} per D-BROWSE-1/-2/-3</name>
  <files>crates/tome/src/browse/app.rs, crates/tome/src/browse/ui.rs, crates/tome/src/browse/mod.rs, crates/tome/src/machine.rs</files>
  <read_first>
    - crates/tome/src/browse/app.rs lines 167-186 (DetailAction enum + label impl); lines 337-342 (render-time comment "show Disable if enabled, Enable if disabled — never both")
    - crates/tome/src/browse/ui.rs (StatusMessage enum — POLISH-02 Success variant)
    - crates/tome/src/machine.rs (MachinePrefs::disable_skill, is_disabled — global; DirectoryEntry per-directory mutators)
    - .planning/phases/15-cli-hardening/15-CONTEXT.md section "HARD-21 browse Disable/Enable wiring" "D-BROWSE-1" "D-BROWSE-2" "D-BROWSE-3" (lines 196-231 — D-BROWSE-2 and D-BROWSE-3 define TWO DISTINCT STRINGS — the action-menu label and the status-message body)
    - .planning/REQUIREMENTS.md section "HARD-21"
    - .planning/phases/10-phase-8-review-tail/10-CONTEXT.md (POLISH-02 StatusMessage enum spec)
  </read_first>
  <behavior>
    **CRITICAL: D-BROWSE-2 (action label) and D-BROWSE-3 step 4 (status-message body) are TWO DISTINCT STRINGS** — do not conflate them. The action-menu label is what `DetailAction::label()` returns; the status-message body is what `StatusMessage::Success { body, .. }` carries. Skill name appears in the body but NOT in the label.

    Smart-routing toggle (D-BROWSE-1):
    - Test: skill foo's parent directory has `[directories.bar.disabled = ["baz"]]` set; pressing Disable on foo (in dir bar) → inserts "foo" into directories.bar.disabled blocklist; machine.toml on disk reflects the addition.
    - Test: same setup, pressing Enable on foo (already in blocklist) → removes "foo" from the blocklist.
    - Test: skill foo's parent directory has `[directories.bar.enabled = ["foo"]]` set (allowlist); pressing Disable on foo → REMOVES "foo" from the allowlist (inverted polarity); MACH-04 invariant preserved (disabled stays None).
    - Test: skill foo's parent directory has neither disabled nor enabled set; pressing Disable on foo → inserts "foo" into global MachinePrefs.disabled.
    - Test: pressing the inverse undoes the action exactly (toggle is reversible).

    Action-menu LABEL — `DetailAction::label(&row, &prefs)` (D-BROWSE-2; verb + scope, NO skill name):
    - Test: row in global-toggle scope (Disable variant)             → `DetailAction::Disable.label(&row, &prefs) == "Disable on this machine"`
    - Test: row in global-toggle scope (Enable variant)              → `DetailAction::Enable.label(&row, &prefs) == "Enable on this machine"`
    - Test: row in per-directory blocklist scope, dir = "my-dir"     → `DetailAction::Disable.label(&row, &prefs) == "Disable for my-dir"`
    - Test: row in per-directory allowlist scope, dir = "my-dir"     → `DetailAction::Disable.label(&row, &prefs) == "Disable for my-dir"` (same label as blocklist; semantics differ)
    - Test (negative): label NEVER contains the skill name. Search the returned String for `row.name`; assert absent.

    Status-message BODY — `StatusMessage::Success { body, .. }` after toggle (D-BROWSE-3 step 4; verb + skill + scope):
    - Test: global-toggle Disable on skill "foo"            → `status_msg.body == "Disabled foo on this machine"`
    - Test: global-toggle Enable on skill "foo"             → `status_msg.body == "Enabled foo on this machine"`
    - Test: per-directory Disable on skill "foo", dir "my-dir" → `status_msg.body == "Disabled foo for my-dir"`
    - Test: per-directory Enable on skill "foo", dir "my-dir"  → `status_msg.body == "Enabled foo for my-dir"`

    D-BROWSE-3 4-step toggle flow (one explicit acceptance criterion per step):
    - Step 1 (in-memory mutation): test asserts `prefs.is_disabled("foo")` (or per-directory equivalent) flips after `apply_toggle`.
    - Step 2 (atomic save): test asserts `machine.toml` on disk reflects the toggle after save (load + re-read; existing temp+rename round-trip).
    - Step 3 (label flip): test asserts `DetailAction::label(&row, &prefs)` for the SAME row+action returns "Disable …" before toggle and "Enable …" after toggle (or vice-versa).
    - Step 4 (status surface): test asserts `app.status_message` is `Some(StatusMessage::Success { .. })` with body matching the verbatim shapes above.

    Atomic save + flip (D-BROWSE-3):
    - Test: NO confirmation prompt — single keystroke applies (no intermediate state).

    No remaining #[allow(dead_code)]:
    - Test: `grep -E "#\[allow\(dead_code\)\]" crates/tome/src/browse/app.rs` does NOT return a match adjacent to DetailAction (line 168 specifically).
  </behavior>
  <action>
    **Step A: Drop #[allow(dead_code)] at browse/app.rs:168.**

    Read line 168. The attribute sits on (or above) DetailAction::Disable and DetailAction::Enable. Once these variants are wired (step B), the attribute can be removed entirely. If only some sub-elements need it removed, surgically remove only those — do not blanket-remove if other variants still legitimately need it.

    **Step B: Implement smart-routing toggle (D-BROWSE-1).**

    Add a method to App (browse/app.rs):

    ```rust
    impl App {
        /// Apply the user's Disable/Enable keystroke to the currently-selected skill.
        /// D-BROWSE-1 smart-routing:
        ///   1. If parent dir has disabled blocklist set -> toggle blocklist
        ///   2. Else if parent dir has enabled allowlist set -> toggle allowlist (inverted)
        ///   3. Else -> toggle global MachinePrefs.disabled
        /// MACH-04 invariant preserved (only one of disabled/enabled is set per directory).
        pub(crate) fn apply_toggle(&mut self, action: DetailAction) -> anyhow::Result<()> {
            let row = self.selected_skill().ok_or_else(|| anyhow::anyhow!("no skill selected"))?;
            let dir_name = row.source_directory.clone();   // None for unowned (Phase 14 D-C1) — falls into global branch
            let skill_name = row.name.clone();
            let was_disable = matches!(action, DetailAction::Disable);

            // D-BROWSE-3 step 1: decide scope.
            let scope = if let Some(dir) = dir_name.as_ref().and_then(|d| self.machine_prefs.directories.get(d)) {
                if dir.disabled.is_some() { ToggleScope::PerDirBlocklist(dir_name.clone().unwrap()) }
                else if dir.enabled.is_some() { ToggleScope::PerDirAllowlist(dir_name.clone().unwrap()) }
                else { ToggleScope::Global }
            } else {
                ToggleScope::Global
            };

            // D-BROWSE-3 step 1: mutate MachinePrefs in-memory.
            match scope {
                ToggleScope::Global => {
                    if was_disable {
                        self.machine_prefs.disabled.insert(skill_name.clone());
                    } else {
                        self.machine_prefs.disabled.remove(&skill_name);
                    }
                }
                ToggleScope::PerDirBlocklist(ref dir) => {
                    let entry = self.machine_prefs.directories.get_mut(dir).unwrap();
                    let blocklist = entry.disabled.as_mut().unwrap();
                    if was_disable {
                        blocklist.insert(skill_name.clone());
                    } else {
                        blocklist.remove(&skill_name);
                    }
                }
                ToggleScope::PerDirAllowlist(ref dir) => {
                    let entry = self.machine_prefs.directories.get_mut(dir).unwrap();
                    let allowlist = entry.enabled.as_mut().unwrap();
                    if was_disable {
                        allowlist.remove(&skill_name);   // inverted polarity
                    } else {
                        allowlist.insert(skill_name.clone());
                    }
                }
            }

            // D-BROWSE-3 step 2: atomic save.
            self.machine_prefs.save(&self.paths.machine_toml)?;

            // D-BROWSE-3 step 3: label flip is render-driven — DetailAction::label(&row, &prefs)
            //   re-evaluates next render with the now-mutated prefs, so it flips automatically.

            // D-BROWSE-3 step 4: surface scope-explicit StatusMessage with VERB + SKILL + SCOPE.
            //   NOTE: the BODY differs from the action-menu LABEL (which has NO skill name).
            //   Body shapes are verbatim from CONTEXT.md line 230 + step 4:
            //     "Disabled <skill> on this machine"  /  "Enabled <skill> on this machine"
            //     "Disabled <skill> for <dir>"        /  "Enabled <skill> for <dir>"
            let verb_past = if was_disable { "Disabled" } else { "Enabled" };
            let body = match &scope {
                ToggleScope::Global =>
                    format!("{} {} on this machine", verb_past, skill_name),
                ToggleScope::PerDirBlocklist(d) | ToggleScope::PerDirAllowlist(d) =>
                    format!("{} {} for {}", verb_past, skill_name, d),
            };
            self.status_message = Some(StatusMessage::Success { body, ... });

            Ok(())
        }
    }

    enum ToggleScope {
        Global,
        PerDirBlocklist(DirectoryName),
        PerDirAllowlist(DirectoryName),
    }
    ```

    Adapt to actual codebase conventions (struct field access, method names) — the snippet above is illustrative.

    **Step C: Make DetailAction::label() context-sensitive (D-BROWSE-2) — ACTION-MENU LABEL ONLY.**

    Current shape (browse/app.rs lines 178-186):
    ```rust
    impl DetailAction {
        pub fn label(&self) -> &'static str {
            match self {
                DetailAction::Disable => "Disable",
                DetailAction::Enable => "Enable",
                DetailAction::CopyPath => "Copy path",
                ...
            }
        }
    }
    ```

    Refactor to take a `&SkillRow` parameter and the App's MachinePrefs context, returning `String`. **The label is the ACTION-MENU label per D-BROWSE-2: VERB + SCOPE only — NO skill name.** (The skill name appears in the StatusMessage body in Step B, NOT here.)

    ```rust
    impl DetailAction {
        /// Action-menu label per D-BROWSE-2. Verb + scope only — no skill name.
        /// Distinct from the StatusMessage body (which DOES include the skill name)
        /// produced inside App::apply_toggle.
        pub fn label(&self, row: &SkillRow, prefs: &MachinePrefs) -> String {
            match self {
                DetailAction::Disable | DetailAction::Enable => {
                    let verb = if matches!(self, DetailAction::Disable) { "Disable" } else { "Enable" };
                    let scope_str = match scope_of(row, prefs) {
                        ToggleScope::Global => "on this machine".to_string(),
                        ToggleScope::PerDirBlocklist(d) | ToggleScope::PerDirAllowlist(d) => format!("for {}", d),
                    };
                    // VERB + SCOPE — no skill name (D-BROWSE-2 verbatim).
                    format!("{} {}", verb, scope_str)
                }
                DetailAction::CopyPath => "Copy path".to_string(),
                ...
            }
        }
    }
    ```

    **Verify the resulting strings match D-BROWSE-2 verbatim (no skill name):**
    - Global: `"Disable on this machine"` / `"Enable on this machine"`
    - Per-directory: `"Disable for <dir>"` / `"Enable for <dir>"`

    Update every caller of `.label()` to pass the row + prefs context.

    Step C alternative: rather than threading row+prefs through `.label()`, keep label as `&'static str` and add a sibling `pub fn label_with_scope(&self, ...) -> String`. Pick whichever minimises caller churn. CONTEXT.md notes: "Mechanically a small refactor."

    **Step D: Wire keystroke → apply_toggle in the event loop.**

    In browse/app.rs (or wherever the keypress handler is), the existing keybinding for Disable/Enable currently does nothing (since #[allow(dead_code)]). Wire it to call `App::apply_toggle(DetailAction::Disable)` (or Enable based on current state). Per D-BROWSE-3: "single keystroke applies; no confirmation prompt".

    **Step E: Tests.**

    Unit tests in browse/app.rs::tests covering all `<behavior>` cases. Use the same `App::for_snapshot` (or sibling test fixture factory) introduced in Task 1. Mock or in-memory MachinePrefs for setup:

    Smart-routing tests:
    - `apply_toggle_global_when_no_per_dir_list`
    - `apply_toggle_per_dir_blocklist`
    - `apply_toggle_per_dir_allowlist_inverted_polarity`
    - `apply_toggle_undo_via_inverse`

    Action-menu LABEL tests (D-BROWSE-2; verb + scope, NO skill name):
    - `label_global_scope_disable` — asserts `DetailAction::Disable.label(&row, &prefs) == "Disable on this machine"`
    - `label_global_scope_enable` — asserts `DetailAction::Enable.label(&row, &prefs) == "Enable on this machine"`
    - `label_per_dir_blocklist` — asserts `DetailAction::Disable.label(&row, &prefs) == "Disable for my-dir"`
    - `label_per_dir_allowlist` — asserts `DetailAction::Disable.label(&row, &prefs) == "Disable for my-dir"`
    - `label_does_not_contain_skill_name` — negative assertion (skill name absent from label)

    Status-message BODY tests (D-BROWSE-3 step 4; verb + skill + scope):
    - `apply_toggle_status_message_global_disable` — asserts `status_msg.body == "Disabled foo on this machine"`
    - `apply_toggle_status_message_global_enable` — asserts `status_msg.body == "Enabled foo on this machine"`
    - `apply_toggle_status_message_per_dir_disable` — asserts `status_msg.body == "Disabled foo for my-dir"`
    - `apply_toggle_status_message_per_dir_enable` — asserts `status_msg.body == "Enabled foo for my-dir"`

    D-BROWSE-3 4-step flow tests (one per step):
    - `apply_toggle_step1_mutates_in_memory` — assert in-memory MachinePrefs reflects the toggle (e.g. `prefs.is_disabled("foo")` flipped).
    - `apply_toggle_step2_atomic_save_round_trip` — assert machine.toml on disk round-trips the mutation (load + assert).
    - `apply_toggle_step3_label_flips` — assert `DetailAction::label(&row, &prefs)` for the row flips from "Disable …" to "Enable …" across the toggle (or vice-versa).
    - `apply_toggle_step4_surfaces_success_status` — assert `app.status_message == Some(StatusMessage::Success { .. })` with the body matching the verbatim shape from S1 (e.g. `"Disabled foo on this machine"`).

    Add a snapshot test variant in tests/browse_snapshots/ for the post-toggle state showing the label flipped + StatusMessage rendered.
  </action>
  <verify>
    <automated>cargo test -p tome browse::app::tests; cargo test -p tome browse::ui::tests; cargo test -p tome --test browse_snapshots; cargo clippy --all-targets -- -D warnings</automated>
  </verify>
  <acceptance_criteria>
    - `grep -E "#\\[allow\\(dead_code\\)\\]" crates/tome/src/browse/app.rs` does NOT return a match within ±2 lines of `DetailAction::Disable` or `DetailAction::Enable`.
    - `grep -E "fn apply_toggle" crates/tome/src/browse/app.rs` returns at least one match.
    - `grep -E "ToggleScope::(Global|PerDirBlocklist|PerDirAllowlist)" crates/tome/src/browse/app.rs` returns at least 6 matches (3 declarations + 3+ pattern matches).
    - `grep -E "StatusMessage::Success" crates/tome/src/browse/app.rs` returns at least one match (after toggle).
    - DetailAction::label() is context-sensitive: returns String OR takes additional context param. Verify via `grep -E "fn label\\(" crates/tome/src/browse/app.rs`.

    **S1 — DISTINCT label vs body assertions (action-menu LABEL has NO skill name; status BODY DOES):**
    - Action-menu label assertions in unit tests (D-BROWSE-2):
      - `assert_eq!(DetailAction::Disable.label(&row, &prefs), "Disable on this machine")` (global toggle case)
      - `assert_eq!(DetailAction::Enable.label(&row, &prefs), "Enable on this machine")` (global toggle case)
      - `assert_eq!(DetailAction::Disable.label(&row, &prefs), "Disable for my-dir")` (per-directory case)
      - `assert_eq!(DetailAction::Enable.label(&row, &prefs), "Enable for my-dir")` (per-directory case)
      - At least one negative assertion: `assert!(!label.contains(&row.name.to_string()))` — label MUST NOT contain skill name.
    - Status-message body assertions in unit tests (D-BROWSE-3 step 4):
      - `assert_eq!(status_msg.body, "Disabled foo on this machine")` (global Disable)
      - `assert_eq!(status_msg.body, "Enabled foo on this machine")` (global Enable)
      - `assert_eq!(status_msg.body, "Disabled foo for my-dir")` (per-directory Disable)
      - `assert_eq!(status_msg.body, "Enabled foo for my-dir")` (per-directory Enable)

    **S6 — D-BROWSE-3 4-step flow assertions (one explicit acceptance criterion per step):**
    - Step 1 (in-memory mutation): test `apply_toggle_step1_mutates_in_memory` exists and asserts `prefs.is_disabled("foo")` (or per-directory equivalent) flips after toggle.
    - Step 2 (atomic save): test `apply_toggle_step2_atomic_save_round_trip` exists and asserts `machine.toml` on disk reflects the toggle after save (load + re-read assertion).
    - Step 3 (label flip): test `apply_toggle_step3_label_flips` exists and asserts `DetailAction::label(&row, &prefs)` flips from "Disable …" to "Enable …" (or vice-versa) across the toggle.
    - Step 4 (status surface): test `apply_toggle_step4_surfaces_success_status` exists and asserts `app.status_message` is `Some(StatusMessage::Success { .. })` with body matching the S1 verbatim shape.
    - Verify all 4 step tests exist: `grep -cE "fn apply_toggle_step[1-4]_" crates/tome/src/browse/app.rs` returns ≥4.

    - At least 13 new unit tests in browse/app.rs::tests covering the `<behavior>` matrix (4 smart-routing + 5 label + 4 status-body + 4 step-flow, with overlap allowed).
    - At least one snapshot test post-toggle in tests/browse_snapshots/.
    - `cargo clippy --all-targets -- -D warnings` exits 0.
    - MACH-04 invariant preserved: at least one regression test asserts that toggling never sets BOTH disabled and enabled on the same DirectoryEntry.
  </acceptance_criteria>
  <done>
    DetailAction::Disable and DetailAction::Enable are wired with smart-routing per D-BROWSE-1. Action-menu labels are context-sensitive per D-BROWSE-2 (verb + scope, NO skill name) with verbatim text shapes. Status-message bodies per D-BROWSE-3 step 4 (verb + skill + scope) are distinct strings. All 4 D-BROWSE-3 steps fire on toggle (in-memory mutation, atomic save, label flip, success status). MACH-04 invariant preserved. All `#[allow(dead_code)]` removed from the wired path.
  </done>
</task>

</tasks>

<verification>
- `cargo build -p tome` exits 0
- `cargo clippy --all-targets -- -D warnings` exits 0
- `cargo test -p tome --tests` passes; new tests added (target: ≥23 new tests across snapshot + apply_toggle + label + 4-step flow)
- `tests/browse_snapshots/` exists with insta snapshots covering 10+ scenarios
- `#[allow(dead_code)]` is gone from DetailAction::Disable / DetailAction::Enable in browse/app.rs
- Pressing Disable/Enable in browse mutates the correct list per D-BROWSE-1, with MACH-04 preserved
- Action-menu LABEL text matches D-BROWSE-2 verbatim shapes (verb + scope, NO skill name)
- Status-message BODY text matches D-BROWSE-3 step 4 verbatim shapes (verb + skill + scope)
- All 4 D-BROWSE-3 steps have explicit per-step assertions
- machine.toml round-trips through atomic save after every toggle
</verification>

<success_criteria>
- HARD-12: ratatui TestBackend + insta snapshots cover status dashboard, skill list (default/empty/filtered), detail pane (managed/local/unowned), help overlay, both themes (closes #498)
- HARD-21: DetailAction::{Disable, Enable} wired per D-BROWSE-1 (smart-routing) / D-BROWSE-2 (action-menu LABEL — verb + scope, no skill name) / D-BROWSE-3 (instant + atomic save + 4-step flow + StatusMessage body — verb + skill + scope); MACH-04 invariant preserved (closes #447)
- Test count grows by ≥23 (10 snapshot + 4 smart-routing + 5 label + 4 status-body + 4 step-flow tests, with overlap allowed)
</success_criteria>

<output>
After completion, create `.planning/phases/15-cli-hardening/15-05-SUMMARY.md` recording:
- List of snapshot test files created in tests/browse_snapshots/
- DetailAction::label() new signature
- ToggleScope enum variants used
- Action-menu LABEL text shapes (verbatim — D-BROWSE-2; verb + scope, NO skill name)
- Status-message BODY text shapes (verbatim — D-BROWSE-3 step 4; verb + skill + scope)
- Confirmation that all 4 D-BROWSE-3 steps have per-step test assertions
- Issues closed: #498 (HARD-12), #447 (HARD-21)
</output>
