---
phase: 15-cli-hardening
plan: 06
type: execute
wave: 2
depends_on: [15-01]
files_modified:
  - crates/tome/src/backup.rs
  - crates/tome/src/wizard.rs
  - crates/tome/src/relocate.rs
  - crates/tome/src/reassign.rs
  - crates/tome/src/manifest.rs
autonomous: true
requirements:
  - HARD-14
  - HARD-15
  - HARD-16
  - HARD-18
  - HARD-19
  - HARD-20
must_haves:
  truths:
    - "backup::tests::push_and_pull_roundtrip and diff_shows_changes do not flake (gpg signing disabled in test repos via local config)"
    - "wizard.rs diagnostic prints emit on stderr (eprintln!), not stdout — interactive prompts (dialoguer) stay on stdout"
    - "relocate.rs::provenance_from_link_result is renamed to warn_if_unreadable_symlink; all call sites updated"
    - "tome relocate cross-fs orphan-copy preservation surfaces a recovery hint (Conflict/Why/Suggestion shape)"
    - "tome reassign reads filesystem state once in plan(); execute() consumes the snapshot, never re-reads"
    - "manifest epoch-0 timestamp surfaces a warning rather than silently propagating as garbage data"
  artifacts:
    - path: "crates/tome/src/backup.rs"
      provides: "Per-test git config commit.gpgsign=false setup"
      contains: "commit.gpgsign"
    - path: "crates/tome/src/wizard.rs"
      provides: "Diagnostic prints converted to eprintln!"
    - path: "crates/tome/src/relocate.rs"
      provides: "warn_if_unreadable_symlink (renamed); cross-fs cleanup recovery hint"
      contains: "warn_if_unreadable_symlink"
    - path: "crates/tome/src/reassign.rs"
      provides: "ReassignPlan extended with PreReassignState snapshot"
      contains: "PreReassignState"
    - path: "crates/tome/src/manifest.rs"
      provides: "Epoch-0 timestamp warning"
  key_links:
    - from: "crates/tome/src/reassign.rs::ReassignPlan"
      to: "PreReassignState snapshot"
      via: "plan() captures, execute() consumes"
      pattern: "PreReassignState"
    - from: "crates/tome/src/relocate.rs"
      to: "Conflict/Why/Suggestion error template"
      via: "Phase 7 D-10 user-facing bail! shape"
      pattern: "Conflict|Why|Suggestion"
---

<objective>
Land the polish + older-bugs cluster: backup test flake fix (HARD-14, closes #500); wizard.rs eprintln! discipline (HARD-15, closes #501); relocate.rs function rename for side-effect intent (HARD-16, closes #502); cross-fs cleanup recovery hint (HARD-18, closes #416); reassign read-once filesystem state (HARD-19, closes #430); manifest epoch-0 timestamp warning (HARD-20, closes #433).

Purpose: Clear the older-bug backlog (#416, #430, #433, #447, #457 — note #447, #457 are addressed in 15-05/15-02 respectively) and the v0.9-review polish items (#500-#502) in a single sweep. Each item is module-local and can land independently.
Output: 6 small, independent fixes spread across backup.rs, wizard.rs, relocate.rs, reassign.rs, manifest.rs.
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

@crates/tome/src/backup.rs
@crates/tome/src/wizard.rs
@crates/tome/src/relocate.rs
@crates/tome/src/reassign.rs
@crates/tome/src/manifest.rs

<interfaces>
Existing surfaces this plan modifies (each independently):

From crates/tome/src/backup.rs:
  Tests `push_and_pull_roundtrip` and `diff_shows_changes` create temporary git repos.
  Without gpg signing disabled per-repo, signing config inherited from user's global
  ~/.gitconfig may force commits to require a key — flake source.

From crates/tome/src/wizard.rs (1,819 LOC):
  Mix of interactive prompts (dialoguer — stays on stdout) and diagnostic prints
  (println! — should be eprintln!). HARD-15 mechanically converts the diagnostic class.

From crates/tome/src/relocate.rs (1,012 LOC):
  pub fn provenance_from_link_result(...)  ← line ~? — read first; HARD-16 rename target
  Cross-fs orphan-copy preservation logic — HARD-18 adds recovery hint following
  Phase 7 D-10 Conflict / Why / Suggestion template.

From crates/tome/src/reassign.rs (719 LOC):
  pub struct ReassignPlan { ... }
  pub fn plan(...) -> ReassignPlan
  pub fn execute(plan: &ReassignPlan, ...) -> Result<...>
  HARD-19: extend ReassignPlan with PreReassignState snapshot; execute consumes it.

From crates/tome/src/manifest.rs (667 LOC):
  SkillEntry has a sync timestamp field. Epoch-0 (SystemTime::UNIX_EPOCH) is the
  default for un-initialised entries. HARD-20: surface as warning, not silent
  garbage.

Phase 7 D-10 Conflict/Why/Suggestion error template (used by HARD-18):

  Conflict: <what is wrong>
  Why: <why this is wrong>
  Suggestion: <what to do about it>

Pattern in other modules (e.g. config/validate.rs Cases A/B/C, migration_v010.rs).
</interfaces>
</context>

<tasks>

<task type="auto">
  <name>Task 1: HARD-14 (backup test flake fix) + HARD-15 (wizard eprintln!) + HARD-16 (relocate rename)</name>
  <files>crates/tome/src/backup.rs, crates/tome/src/wizard.rs, crates/tome/src/relocate.rs</files>
  <read_first>
    - crates/tome/src/backup.rs (push_and_pull_roundtrip + diff_shows_changes test fns; setup helper that creates temp git repos)
    - crates/tome/src/wizard.rs (1,819 LOC — every println! call site; identify which are dialoguer-driven prompts vs diagnostic)
    - crates/tome/src/relocate.rs (find provenance_from_link_result definition + every call site)
    - .planning/phases/15-cli-hardening/15-CONTEXT.md section "HARD-14 backup flake fix scope" "HARD-15" "HARD-16"
    - .planning/REQUIREMENTS.md sections "HARD-14" "HARD-15" "HARD-16"
  </read_first>
  <action>
    Three small, independent mechanical fixes in one task.

    **Step A: HARD-14 — disable git signing in backup test repos.**

    In crates/tome/src/backup.rs, find the test setup helper that runs `git init` for the test repos (likely a fn like `setup_test_repo()` or inline in `push_and_pull_roundtrip` and `diff_shows_changes`).

    After `git init`, add per-repo config flags to disable signing:

    ```rust
    Command::new("git").args(["init", repo_path]).output()?;
    Command::new("git")
        .args(["config", "--local", "commit.gpgsign", "false"])
        .current_dir(repo_path)
        .output()?;
    Command::new("git")
        .args(["config", "--local", "tag.gpgsign", "false"])
        .current_dir(repo_path)
        .output()?;
    Command::new("git")
        .args(["config", "--local", "user.email", "test@example.com"])
        .current_dir(repo_path)
        .output()?;
    Command::new("git")
        .args(["config", "--local", "user.name", "Test"])
        .current_dir(repo_path)
        .output()?;
    ```

    The user.email + user.name flags are belt-and-braces — without them, git on a clean CI may also fail (signing is often the symptom; identity is sometimes the cause). Keep them for robustness. Per CONTEXT.md "Claude's Discretion": "Whether this lives as a per-test setup helper or a shared common/ test-helper is the planner's call. Scope is just backup::tests; other modules don't run real git commands." — **per-test setup helper is fine**.

    Run the test ≥20 times in a tight loop to verify flake is gone:
    ```
    for i in {1..20}; do cargo test -p tome backup::tests::push_and_pull_roundtrip || break; done
    ```

    **Step B: HARD-15 — wizard.rs println! → eprintln! for diagnostics.**

    In crates/tome/src/wizard.rs, find every `println!` call:

    ```bash
    grep -n "println!" crates/tome/src/wizard.rs
    ```

    For each call site, judge:
    - **Interactive prompt or user-facing question**: KEEP on stdout (stays println! OR is already routed through dialoguer — leave dialoguer alone since it owns its own output).
    - **Diagnostic / status / warning / "what we found" message**: CONVERT to eprintln!.

    Diagnostic patterns to convert (look for these phrases as hints):
    - "Found existing config..."
    - "Detecting..."
    - "Skipping..."
    - "Using..."
    - "Configured..."
    - Any "warning:" prefix
    - Status updates between dialoguer prompts

    User-facing prompt patterns to KEEP on stdout (or leave to dialoguer):
    - "Welcome to..."
    - "Choose..."
    - Anything that's the literal prompt text immediately preceding a dialoguer call (those are stdout because the user is reading them at a TTY).

    Per-call judgement; this is mechanical but requires reading the surrounding context. Document the rule used at the top of the function (or in a brief comment) so future readers don't accidentally regress.

    Run wizard tests:
    ```
    cargo test -p tome wizard::tests
    cargo test -p tome --test cli_init   # post-15-01
    ```

    **Step C: HARD-16 — rename provenance_from_link_result → warn_if_unreadable_symlink.**

    In crates/tome/src/relocate.rs, find the function definition:

    ```rust
    pub fn provenance_from_link_result(...) -> ...
    ```

    Rename the function (and any internal helper if applicable):

    ```rust
    pub fn warn_if_unreadable_symlink(...) -> ...
    ```

    The new name reflects the side-effect intent (the function emits a warning) per CONTEXT.md "side-effect intent is in the function name".

    Update all callers across the crate:
    ```bash
    rg "provenance_from_link_result" crates/tome/src
    ```

    Each caller flips to the new name. Pure rename — no behaviour change.

    No new tests required — existing tests should pass byte-for-byte after the rename. If any test asserts on the function name (unlikely), update those references.
  </action>
  <verify>
    <automated>cargo test -p tome backup::tests; cargo test -p tome wizard::tests; cargo test -p tome relocate::tests; cargo build -p tome; cargo clippy --all-targets -- -D warnings; if grep -E "provenance_from_link_result" crates/tome/src; then exit 1; fi</automated>
  </verify>
  <acceptance_criteria>
    - backup.rs test setup includes `git config --local commit.gpgsign false`: `grep -E "commit\\.gpgsign" crates/tome/src/backup.rs` returns ≥1 match.
    - 20 consecutive runs of `cargo test -p tome backup::tests::push_and_pull_roundtrip` pass: verify locally with the loop in Step A.
    - `grep -c "eprintln!" crates/tome/src/wizard.rs` post-change returns a higher count than `grep -c "eprintln!"` pre-change (delta ≥ 5; verify by reading pre-change baseline before mutation).
    - Diagnostic-class println! call sites in wizard.rs are now eprintln!: spot-check ≥3 specific call sites previously identified during the audit.
    - `grep -E "fn provenance_from_link_result" crates/tome/src/relocate.rs` returns NOTHING.
    - `grep -E "fn warn_if_unreadable_symlink" crates/tome/src/relocate.rs` returns 1 match.
    - `rg "provenance_from_link_result" crates/tome/src` returns NOTHING (all callers updated).
    - `cargo build -p tome` exits 0; `cargo clippy --all-targets -- -D warnings` exits 0.
    - All existing wizard, backup, relocate tests pass: `cargo test -p tome` baseline preserved.
  </acceptance_criteria>
  <done>
    backup tests no longer flake (gpg signing disabled per-repo); wizard.rs diagnostic prints on stderr; provenance_from_link_result is renamed to warn_if_unreadable_symlink with all callers updated.
  </done>
</task>

<task type="auto" tdd="true">
  <name>Task 2: HARD-18 (relocate cross-fs hint) + HARD-19 (reassign read-once)</name>
  <files>crates/tome/src/relocate.rs, crates/tome/src/reassign.rs</files>
  <read_first>
    - crates/tome/src/relocate.rs (cross-fs orphan-copy preservation logic — find the branch where rename-fails-cross-fs falls back to copy+leave-original)
    - crates/tome/src/reassign.rs (719 LOC — current ReassignPlan struct + plan() + execute() shapes; identify which fs reads happen in execute() that could move to plan())
    - .planning/phases/15-cli-hardening/15-CONTEXT.md section "HARD-18" "HARD-19 reassign read-once mechanism" (Claude's Discretion: extend ReassignPlan with PreReassignState struct)
    - .planning/REQUIREMENTS.md sections "HARD-18" "HARD-19"
    - .planning/phases/07-wizard-ux-greenfield-brownfield-legacy/07-CONTEXT.md (Phase 7 D-10 Conflict/Why/Suggestion error template — applied to HARD-18)
  </read_first>
  <behavior>
    HARD-18 cross-fs cleanup recovery hint:
    - Test: simulate a cross-fs relocate (target on different filesystem) where orphan-copy fallback kicks in. The user-facing output includes a Conflict/Why/Suggestion block that names the orphan path the user can manually clean up.
    - Test: same-fs relocate (no orphan-copy needed) does NOT emit the hint.

    HARD-19 reassign read-once:
    - Test: reassign::plan() captures filesystem state into ReassignPlan.pre_state; execute() consumes the snapshot.
    - Test: a manual filesystem mutation between plan() and execute() does NOT change execute's behaviour (it operates on the snapshot, not the live state).
    - Test: PreReassignState round-trips serialisation if it's part of the plan-render output (only if the existing ReassignPlan serialises; otherwise this is a structural-only change).
    - Test: existing reassign tests still pass byte-for-byte (this is a refactor, not a behaviour change).
  </behavior>
  <action>
    **Step A: HARD-18 — cross-fs cleanup recovery hint in relocate.rs.**

    Find the branch in relocate.rs where the orphan-copy preservation kicks in. The current code likely:
    1. Tries fs::rename(old_lib, new_lib).
    2. On EXDEV (cross-device link), falls back to copy(old_lib, new_lib) + leave old_lib intact.
    3. Logs that the original path was preserved.

    The current logging is insufficient — the user doesn't see a recovery action. Replace with a Conflict/Why/Suggestion block:

    ```rust
    eprintln!(
        "Conflict: relocate could not move the library across filesystems.\n\
         Why: source ({old_path}) and target ({new_path}) are on different filesystems;\n      cross-fs rename is not atomic, so the original copy was preserved as a safety measure.\n\
         Suggestion: the new library is at {new_path}. Once you've verified it works (run \\\n             tome status, tome doctor), manually remove the old copy with: rm -rf {old_path}\n",
        old_path = old_lib.display(),
        new_path = new_lib.display(),
    );
    ```

    (Adapt phrasing to match Phase 7 D-10 template prevailing tone — read other Conflict/Why/Suggestion sites in the codebase for style.)

    Per Phase 7 D-10: the three sections ("Conflict", "Why", "Suggestion") MUST appear as labelled prefixes, in that order. Existing prior-art sites: search `grep -rE "^Conflict:" crates/tome/src` and match the closest spelling/format.

    Add a regression test in relocate.rs::tests:

    - `cross_fs_relocate_emits_recovery_hint`: simulate cross-fs (mock or skip-with-cfg if real cross-fs setup is too heavy in CI). Assert stderr contains "Conflict:", "Why:", "Suggestion:", and the orphan path.
    - `same_fs_relocate_no_hint`: regression — same-fs path does NOT emit the hint.

    If real cross-fs simulation is impractical in CI, refactor the hint emission into a pure formatter `fn cross_fs_recovery_hint(old: &Path, new: &Path) -> String` and unit-test the formatter directly.

    **Step B: HARD-19 — reassign read-once snapshot.**

    In crates/tome/src/reassign.rs:

    1. Define a new struct `PreReassignState` capturing what reassign::plan() reads from disk that execute() currently re-reads. Read the plan/execute split first to identify duplicated reads. Likely candidates:
       - Manifest entry for the skill (manifest.skills().get(name))
       - Library directory state (does target dir exist? content_hash of source dir?)
       - Source directory existence

       ```rust
       #[derive(Debug, Clone)]
       pub struct PreReassignState {
           /// Manifest entry as read at plan() time.
           pub manifest_entry_at_plan: Option<SkillEntry>,
           /// Whether target library directory existed at plan() time.
           pub target_existed_at_plan: bool,
           /// Source content hash at plan() time (used by D-A1 collision check).
           pub source_hash_at_plan: Option<ContentHash>,
           /// Target content hash at plan() time, if target existed (D-A1).
           pub target_hash_at_plan: Option<ContentHash>,
           // ... whatever else the live reads need
       }
       ```

    2. Extend ReassignPlan to carry it:
       ```rust
       pub struct ReassignPlan {
           // ... existing fields
           pub pre_state: PreReassignState,
       }
       ```

    3. In `plan()`, populate `pre_state` once. Anywhere `plan()` already reads from disk, also persist the read into `pre_state`.

    4. In `execute()`, replace every `fs::read(...)` / `manifest.skills().get(name)` call that re-reads the same data with `plan.pre_state.<field>`. Per CONTEXT.md: "execute consumes the snapshot rather than re-reading."

    5. Atomic-mutation reads (e.g. acquiring a file lock, the actual write of the new manifest entry, the actual rename of the directory) STAY in execute() — those are mutations, not reads. The optimisation is for reads only.

    6. Verify behaviour parity: existing reassign tests should pass byte-for-byte. The plan/execute drift bug (#430) is closed because no read can race between plan() and execute() — the snapshot is authoritative.

    Add unit tests in reassign.rs::tests:

    - `pre_state_captured_at_plan_time`: call plan(), assert PreReassignState has the expected fields populated.
    - `execute_consumes_pre_state_not_live`: call plan(), mutate the live manifest between plan and execute, call execute, assert execute used the snapshot (NOT the mutation).
    - All existing reassign tests still pass — this is a behaviour-preserving refactor.

    Phase 14's reassign test fixtures (D-API-1, D-A1, D-A2) should pass without modification.
  </action>
  <verify>
    <automated>cargo test -p tome relocate::tests; cargo test -p tome reassign::tests; cargo build -p tome; cargo clippy --all-targets -- -D warnings</automated>
  </verify>
  <acceptance_criteria>
    - `grep -E "(Conflict|Why|Suggestion):" crates/tome/src/relocate.rs` returns ≥3 matches (the three labels).
    - At least 2 new relocate tests: `cross_fs_relocate_emits_recovery_hint` + `same_fs_relocate_no_hint` (or analogous names).
    - `grep -E "pub struct PreReassignState" crates/tome/src/reassign.rs` returns 1 match.
    - `grep -E "pre_state: PreReassignState" crates/tome/src/reassign.rs` returns at least 1 match (field on ReassignPlan).
    - `grep -E "plan\\.pre_state\\." crates/tome/src/reassign.rs` returns ≥3 matches (execute consumes snapshot in ≥3 places that previously re-read).
    - At least 2 new reassign tests: `pre_state_captured_at_plan_time` + `execute_consumes_pre_state_not_live`.
    - All existing reassign tests pass: `cargo test -p tome reassign::tests` exits 0.
    - `cargo build -p tome` exits 0; `cargo clippy --all-targets -- -D warnings` exits 0.
  </acceptance_criteria>
  <done>
    relocate cross-fs cleanup emits Conflict/Why/Suggestion hint with orphan path; reassign::plan() captures filesystem state into PreReassignState, execute() consumes the snapshot, no plan/execute drift possible.
  </done>
</task>

<task type="auto" tdd="true">
  <name>Task 3: HARD-20 manifest epoch-0 timestamp warning</name>
  <files>crates/tome/src/manifest.rs</files>
  <read_first>
    - crates/tome/src/manifest.rs (SkillEntry struct — find the sync-timestamp field; find the load/deserialise path; check if there's a load-time validator)
    - .planning/phases/15-cli-hardening/15-CONTEXT.md section "HARD-20" (Claude's Discretion: in SkillEntry::new or load-time validator)
    - .planning/REQUIREMENTS.md section "HARD-20"
  </read_first>
  <behavior>
    Manifest epoch-0 timestamp warning:
    - Test: load a manifest with a SkillEntry whose sync-timestamp is SystemTime::UNIX_EPOCH → loads successfully, but emits a stderr warning naming the skill.
    - Test: load a manifest with a normal (non-epoch) timestamp → no warning.
    - Test: the warning is emitted exactly once per load (not once per access).
    - Test: epoch-0 entries do NOT poison downstream features — they remain loadable and the rest of the manifest functions normally. The warning is informational.
  </behavior>
  <action>
    Read crates/tome/src/manifest.rs and identify:
    1. The SkillEntry timestamp field (likely `synced_at: SystemTime` or `pub last_synced: SystemTime` — check exact name).
    2. The Manifest load path (probably `Manifest::load(path)` or via serde Deserialize).

    The garbage-data risk per CONTEXT.md HARD-20 is that an epoch-0 timestamp silently flows into diff comparisons or display output, where it appears as `1970-01-01T00:00:00` and is meaningless. The fix is to surface it as a warning at load time.

    Implementation strategy A (load-time validator, recommended):

    Modify `Manifest::load`:

    ```rust
    pub fn load(path: &Path) -> anyhow::Result<Self> {
        let raw = fs::read_to_string(path)?;
        let manifest: Manifest = serde_json::from_str(&raw)?;
        // HARD-20: emit warning for epoch-0 timestamps (silent garbage data)
        for (name, entry) in &manifest.skills {
            if entry.synced_at == SystemTime::UNIX_EPOCH {
                eprintln!(
                    "warning: manifest entry for {name} has UNIX_EPOCH sync-timestamp; \
                     this likely indicates a partial-save or migration artifact. \
                     Run `tome sync` to refresh, or `tome doctor` for full diagnosis."
                );
            }
        }
        Ok(manifest)
    }
    ```

    Adapt to actual code shape (synced_at field name; whether load uses serde_json or another path).

    Implementation strategy B (SkillEntry::new validator):

    If timestamps flow through SkillEntry::new at construction sites where the caller chooses the timestamp, validate there. But CONTEXT.md notes "load-time validator" — strategy A is preferred.

    Per CONTEXT.md "Likely in SkillEntry::new or load-time validator": pick load-time validator unless reading manifest.rs reveals a better integration point.

    Add unit tests in manifest.rs::tests covering all 4 cases in `<behavior>`. Use a captured stderr buffer (e.g. `gag` crate, OR fork the warning into a pure-formatter helper `fn epoch_warning_for(name: &SkillName) -> String` and unit-test the formatter directly without stderr capture).

    Per the existing test style in tome (e.g. POLISH-02 tests), the pure-formatter approach is conventional — extract `fn epoch_zero_warning(name: &SkillName) -> Option<String>` returning Some(text) for epoch-0, None otherwise. Then `Manifest::load` calls the formatter and eprints if Some. Unit tests assert on the formatter's return value directly.
  </action>
  <verify>
    <automated>cargo test -p tome manifest::tests; cargo build -p tome; cargo clippy --all-targets -- -D warnings</automated>
  </verify>
  <acceptance_criteria>
    - `grep -E "UNIX_EPOCH" crates/tome/src/manifest.rs` returns ≥1 match in production code (load-path validator).
    - Pure formatter exists: `grep -E "fn epoch.*warning|fn epoch_zero" crates/tome/src/manifest.rs` returns ≥1 match (or analogous helper with descriptive name).
    - At least 4 new tests covering epoch-0 detection + non-epoch no-warning + once-per-load + non-poison: `grep -cE "fn .*epoch" crates/tome/src/manifest.rs` returns ≥4.
    - The warning text names the affected skill: assertion in tests like `assert!(warning.contains(skill_name))`.
    - Loading a manifest with epoch-0 entries does NOT fail (still returns Ok): regression test loads a fixture and asserts entry is in the loaded manifest.
    - `cargo build -p tome` exits 0; `cargo clippy --all-targets -- -D warnings` exits 0.
  </acceptance_criteria>
  <done>
    Manifest::load emits a warning for SkillEntry timestamps at SystemTime::UNIX_EPOCH; pure formatter is unit-tested; entries remain loadable; warning surfaces once per load. Closes #433.
  </done>
</task>

</tasks>

<verification>
- `cargo build -p tome` exits 0
- `cargo clippy --all-targets -- -D warnings` exits 0
- `cargo test -p tome` passes; new tests added (target: ≥10 new tests across 6 sub-items)
- `cargo test -p tome backup::tests::push_and_pull_roundtrip` passes 20 consecutive runs (HARD-14 done)
- `rg "println!" crates/tome/src/wizard.rs` returns only interactive-prompt sites; diagnostic prints converted to eprintln! (HARD-15 done)
- `rg "provenance_from_link_result" crates/tome/src` returns 0 results; `warn_if_unreadable_symlink` exists (HARD-16 done)
- relocate.rs cross-fs branch emits Conflict/Why/Suggestion hint (HARD-18 done)
- ReassignPlan has PreReassignState; execute() consumes snapshot (HARD-19 done)
- Manifest::load emits epoch-0 warning when applicable (HARD-20 done)
</verification>

<success_criteria>
- HARD-14: backup test gpg-signing flake fixed via per-repo `git config --local commit.gpgsign false` (closes #500)
- HARD-15: wizard.rs diagnostic prints converted to eprintln! (interactive prompts stay on stdout) (closes #501)
- HARD-16: relocate.rs::provenance_from_link_result renamed to warn_if_unreadable_symlink (closes #502)
- HARD-18: tome relocate cross-fs cleanup emits Conflict/Why/Suggestion recovery hint (closes #416)
- HARD-19: tome reassign reads filesystem state once via PreReassignState; execute() consumes snapshot (closes #430)
- HARD-20: manifest epoch-0 timestamp surfaces stderr warning at load time (closes #433)
- Test count grows by ≥10 (relocate cross-fs, reassign read-once, manifest epoch-0)
</success_criteria>

<output>
After completion, create `.planning/phases/15-cli-hardening/15-06-SUMMARY.md` recording:
- backup test git-config setup snippet
- Number of wizard.rs println! → eprintln! conversions
- relocate.rs new function name confirmed
- relocate.rs cross-fs hint text (verbatim)
- ReassignPlan.pre_state shape
- Manifest::load epoch-0 warning text
- Issues closed: #500 (HARD-14), #501 (HARD-15), #502 (HARD-16), #416 (HARD-18), #430 (HARD-19), #433 (HARD-20)
</output>
