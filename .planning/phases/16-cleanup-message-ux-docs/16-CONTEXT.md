# Phase 16: Cleanup-message UX + docs - Context

**Gathered:** 2026-05-08
**Status:** Ready for planning

<domain>
## Phase Boundary

v0.10's user-facing-surface polish before the rc cut. Three workstreams:

1. **Cleanup-message rewrite (UX-01)** — Rewrite `tome sync`'s cleanup output that
   originally triggered this milestone discussion. Today the cleanup pipeline is
   split across `cleanup_library` (`cleanup.rs:43`, two-bucket: Case 1
   unowned-transition + Case 2 missing-from-disk) and `cleanup_target`
   (`cleanup.rs:236`, silent removal of stale exclude-listed symlinks).
   This phase unifies them into a single user-facing surface with three buckets
   and per-entry actionable hints.

2. **Migration prompt confirmation (UX-02)** — Add a confirm-or-abort gate
   before any conversion runs in `tome migrate-library`. Today
   `cmd_migrate_library` calls `render_plan(&plan)` then `execute(&plan,
   dry_run)` directly with no human gate; users use `--dry-run` as the preview
   step. After this phase, non-dry-run runs MUST stop on a confirmation prompt
   and only proceed on explicit `y` (or `--yes` flag bypass).

3. **Documentation (DOC-01..03)** — Three docs land before the rc cut:
   `architecture.md` updated for the v0.10 model, `CHANGELOG.md` v0.10 entry
   with the three breaking changes called out, and a new
   `docs/src/cross-machine-sync.md` page documenting the dotfiles workflow
   end-to-end.

**In scope:**

- `cleanup.rs` rewrite — unified three-bucket output partition (UX-01)
- `migration_v010.rs::render_plan` + `cmd_migrate_library` — confirm gate +
  `tabled` summary + disk-estimate via `metadata().len()` walk (UX-02)
- `--yes` flag on `tome migrate-library` — mirrors Phase 14 D-B3 (`tome remove
  skill --yes`)
- `docs/src/architecture.md` — targeted rewrites + 4 new sections (DOC-01)
- `CHANGELOG.md` v0.10 entry — three breaking changes + migration step + vocab
  merge (DOC-02)
- `docs/src/cross-machine-sync.md` — new top-level page (DOC-03)
- `docs/src/SUMMARY.md` — link the new page
- `tome sync --help` long description — link to cross-machine-sync.md

**Out of scope** (handled by other phases or out of v0.10 entirely):

- Linux-runtime UAT for clipboard / `xdg-open` (carry-over from v0.8) → Phase 17
  (REL-03)
- Real-library migration smoke-test → Phase 17 (REL-04)
- In-flight PRs #484 / #504 landing → Phase 17 (REL-01)
- Issue triage pass → Phase 17 (REL-02)
- cargo-dist v0.10.0 release → Phase 17 (REL-05)
- Doctor-command remediation hints expansion (beyond what Phase 14 already
  delivered) — out of v0.10
- Localized docs (en/de/ja) — out of scope; deferred indefinitely
- Excalidraw diagram update — Claude's discretion (defer if the diagram still
  renders broadly accurate after the architecture.md text update)
- `--undo-migrate` or `tome migrate-library --revert` — explicitly NOT in scope
  per Phase 11 D-04 (broken-symlink preservation gives partial recovery; full
  undo is out of scope for v0.10)

**Vocabulary supersession (must surface in CONTEXT.md and DOC-02):**

REQUIREMENTS.md UX-02 says "First-sync v0.10 migration prompt (LIB-05) renders
a summary table". This wording is **superseded by Phase 11 D-01**: migration is
the one-shot `tome migrate-library` CLI command, **not** an auto-on-first-sync
prompt. The behaviour UX-02 describes (summary table, confirm-or-abort) is
delivered in full; the trigger is different. Phase 14 already set the
precedent for this kind of vocabulary supersession (D-API-1/-2 folded
`tome adopt`/`tome forget` into existing commands). DOC-02 (CHANGELOG.md) and
DOC-01 (architecture.md) must use the post-supersession vocabulary.

</domain>

<decisions>
## Implementation Decisions

### Cleanup three-bucket scope and layout (UX-01)

- **D-UX01-1 (bucket #3 = distribution-side):** The three buckets are:
  - **Bucket A (removed-from-config):** Manifest entries whose `source_name`
    points at a directory no longer in `config.directories`. Today's
    `cleanup_library` Case 1 (unowned-transition; LIB-04). Library content
    preserved; `manifest[skill].source_name` flips to `None`.
  - **Bucket B (missing-from-disk):** Manifest entries whose `source_name` IS
    still in `config.directories` but the source file vanished from disk.
    Today's `cleanup_library` Case 2. Library content removed (matches today's
    behaviour — a configured source dropping a file is intentional).
  - **Bucket C (now-in-exclude-list):** **NEW.** Library skills whose
    distribution symlinks were just removed because the user added them to
    `machine.toml::disabled` (global), `disabled_directories`, or per-directory
    `directories.<name>.disabled` (MACH-04). Library content **preserved**
    (LIB-04 invariant); only distribution symlinks change. Source: today's
    silent `cleanup_target` removal of stale exclude-listed symlinks
    (`cleanup.rs:236`), now surfaced.
- **D-UX01-2 (unified output):** The three buckets render as one user-facing
  section, even though the underlying logic is split across `cleanup_library`
  (Buckets A + B) and `cleanup_target` (Bucket C). Implementation needs a way
  to collect bucket entries from both functions before rendering. Two
  candidate shapes (plan to choose during implementation):
  1. New `CleanupSummary` struct accumulated across both functions, rendered
     once after both run.
  2. `cleanup_target` writes to a side-channel (`Vec<ExcludedSkill>`) that
     `cleanup_library`'s renderer drains.

  The planner picks; either is acceptable. The user-facing contract (three
  bucket headers in one block) is what matters.
- **D-UX01-3 (per-entry inline hints):** Each bucket renders as a colored
  bucket header line followed by per-skill lines with the actionable hint
  inline. Shape:

  ```
  3 skills no longer in any source (preserving as Unowned):
    foo (was: my-old-dir) — re-add my-old-dir, or run `tome reassign foo --to <dir>`
    bar (was: my-old-dir) — re-add my-old-dir, or run `tome reassign bar --to <dir>`
    baz (was: another-dir) — re-add another-dir, or run `tome reassign baz --to <dir>`

  1 skill missing from configured source on disk (removing from library):
    qux (from: my-current-dir) — restore the file, or run `tome remove skill qux`

  2 skills now in exclude list (distribution symlinks removed; library preserved):
    quux (excluded globally) — remove `quux` from `machine.toml::disabled` to re-distribute
    corge (excluded for: my-dir) — remove `corge` from `machine.toml::directories.my-dir.disabled` to re-distribute
  ```

  Rationale: per-skill hints are necessary because the actionable directory
  name varies per entry (Bucket A); the hints would be wrong as a single
  end-of-bucket footnote. Visual style follows today's `cleanup_library`
  Case-2 line pattern (`cleanup.rs:178`) extended to all three buckets.
- **D-UX01-4 (stderr discipline):** All cleanup output (bucket headers +
  per-skill lines + hints) goes to **stderr**. Matches Phase 15 HARD-15
  wizard-chrome-to-stderr precedent and today's `eprintln!` calls in
  `cleanup_library` (lines 110, 178). `stdout` stays reserved for
  machine-readable status output (e.g. JSON when `--json` is added later).
  Today's interactive Case-2 prompt uses `println!` (`cleanup.rs:146`); that
  prompt becomes a `dialoguer::Confirm` (which writes to stderr by default) so
  no migration is needed beyond auditing the call sites.

### Migration prompt confirmation + summary shape (UX-02)

- **D-UX02-1 (confirm default-no):** `tome migrate-library` (non-dry-run) prompts
  before any conversion using `dialoguer::Confirm::default(false)`. User must
  press `y` to migrate. Matches Phase 14 D-B3 (`tome remove skill`
  confirmation default-no), today's cleanup Case-2 prompt
  (`cleanup.rs:168`), and the pre-existing render_plan warning ("tome does not
  snapshot your library before migrating; commit your library directory to
  git ... BEFORE proceeding"). Safest — never silently mutates a real library
  on accidental Enter.
- **D-UX02-2 (`--yes` flag bypass):** Add `--yes` / `-y` flag on `tome
  migrate-library`. Mirrors Phase 14 D-B3. Behaviour:
  - With `--yes`: skip the prompt, proceed.
  - Under `--no-input` without `--yes`: bail with a clear message ("`tome
    migrate-library` is destructive; pass `--yes` to confirm in non-interactive
    mode"). Mirror Phase 7 D-10 Conflict/Why/Suggestion shape.
  - Under `--no-input` with `--yes`: skip the prompt, proceed (CI-friendly).
  - `--dry-run` always skips the prompt (no destructive action ever runs).
- **D-UX02-3 (summary format = inline summary line + tabled rounded):** Above
  the per-skill list, render a bold inline summary line:
  `Will convert 62 symlinks → real directories (~30.4 MB additional disk).`
  Then a `tabled::Table` with `Style::rounded()` and columns:

  | SKILL | SOURCE | SIZE | STATUS |

  Where SOURCE is `paths::collapse_home(raw_link_target)` (matches today's
  render_plan), SIZE is the human-readable byte count of the resolved source,
  STATUS is ✓ (convert) or ⚠ (skip-broken). One ceremonial table for a
  ceremonial one-shot command. Matches WHARD-07 wizard-summary precedent.
  Truncation rules: show all entries by default (62 skills is reasonable);
  Claude's discretion if a future user has 500+ skills.
- **D-UX02-4 (disk estimate via metadata().len() walk):** During
  `migration_v010::plan()`, walk each `MigrationEntry`'s `resolved_source`
  using `walkdir` + `metadata().len()` summed per file. Add a `byte_size:
  Option<u64>` field to `MigrationEntry` (Some on `source_reachable: true`,
  None on broken). Total goes into the summary line; per-skill values into the
  table SIZE column. Estimated cost: ~20 ms/skill × 62 skills ≈ 1.2s on a real
  library — acceptable for a one-shot ceremonial command. Estimate is byte-
  accurate (not block-rounded like `du -h`); display via a small humanize
  helper (existing `console` crate has formatters; if not, write a tiny one
  in `paths.rs` or a new `human_bytes.rs`).

### Architecture doc scope (DOC-01)

- **D-DOC01-1 (targeted rewrites + 4 new sections):** Keep the existing
  `architecture.md` skeleton (Sync Pipeline, Other Modules, Key Patterns,
  Testing, CI). Approach:
  - **Rewrite** the 3-4 paragraphs containing v0.9 framing:
    - "Consolidate" paragraph (line 16) — drop the symlink/copy split; describe
      managed-as-copy + `consolidate_managed` rewrite per LIB-01. Reference the
      new `marketplace.rs` adapter dispatcher for managed-source updates.
    - "Two-tier model" paragraph (line 43) — rewrite as
      "Discovery → Library → Distribution"; library is canonical (real-dir
      copies for both managed and local).
    - "Distribute" paragraph (line 17) — drop the "library skills" framing;
      reframe as "library entries (Owned + Unowned) get symlinked into
      `synced` / `target` directories". Cross-link to the new "Unowned
      lifecycle" section.
    - Modules list — touch entries for `library.rs`, `cleanup.rs`,
      `manifest.rs`, `lockfile.rs` (schema changes), `remove.rs`, `reassign.rs`
      (subcommand split, `--force`, Unowned input). Add new entries for
      `marketplace.rs`, `reconcile.rs`, `migration_v010.rs`, `summary.rs`.
  - **Add 4 new sections** (insert between "Key Patterns" and "Testing"):
    1. **Library-canonical model** — Managed-as-copy mechanic; how
       `consolidate_managed` writes a real-dir copy via
       `walkdir::WalkDir::follow_links(true)`. Why this matters
       (cross-machine, version-churn resilience, vanished-plugin fallback).
       Reference Phase 11 D-08 (drift basis = content_hash, not version).
    2. **Lockfile-authoritative reconciliation** — Match/Drift/Vanished
       classification per `ReconcileClass`; `auto_install_plugins` consent
       prompt; edit-in-library 3-way prompt (fork/revert/skip). Reference
       Phase 13 D-13 lossy-fork-in-place gap and the `previous_source`
       breadcrumb that closes it.
    3. **Marketplace adapter trait** — `MarketplaceAdapter` six-method shape
       (`id`, `current_version`, `install`, `update`, `list_installed`,
       `available`); `ClaudeMarketplaceAdapter` (subprocess + `RefCell`
       cache); `GitAdapter` (thin shim). Mention upstream constraint
       (`claude plugin install` doesn't accept `--version`).
    4. **Unowned lifecycle** — `SkillEntry.source_name: Option<DirectoryName>`
       schema; `previous_source` breadcrumb; transitions
       (cleanup orphan, `tome remove dir`, fork-in-place); CLI vocab
       (`tome reassign --to <dir>` for re-anchor, `tome remove skill <name>`
       for delete; **D-API-1/-2 vocab merge** explicitly called out — no
       `tome adopt`/`forget` shipped); status/doctor surfacing.
  - **Update** — the existing Excalidraw link (line 3) probably needs a v0.10
    refresh. **Claude's discretion**: defer to a follow-up issue if the diagram
    still renders broadly accurate after text updates; flag in
    deferred-items.md if so. The two-tier flow → three-tier flow shift may
    warrant a fresh diagram, but it's not blocking for the rc cut.
  - **Net change estimate:** ~150-200 new lines, ~40 lines edited. Existing
    file is ~75 lines; final ~225-275 lines.

### Cross-machine doc placement (DOC-03)

- **D-DOC03-1 (standalone page):** New top-level `docs/src/cross-machine-sync.md`.
  Listed in `docs/src/SUMMARY.md` between "Configuration" and "Architecture"
  (so the reading order is: introduction → commands → configuration →
  cross-machine-sync → architecture → ... → development-workflow). Matches
  PROJECT.md's framing of library-as-dotfiles as the **core value** — the page
  is the user-facing manifestation of that value, worth being discoverable.
- **D-DOC03-2 (page structure = walkthrough first, reference second):**
  Document opens with two numbered walkthroughs:
  1. **Machine A (the source-of-truth machine):** `tome init` → curate the
     library → commit `~/.tome/` to dotfiles → push.
  2. **Machine B (a fresh machine):** install tome → clone dotfiles → run
     `tome sync` → first-time `auto_install_plugins` consent prompt → done.

  Below the walkthroughs, reference sections cover:
  - **`tome.lock` semantics** — Cargo.lock-shaped; what it pins; why it's the
    authoritative state on Machine B.
  - **`auto_install_plugins` values** — `Yes` (auto-install on every sync),
    `Never` (warn-only on drift), `Prompt` (default for unset; ask once and
    persist) — reference RECON-02. Document `--no-install` global override.
  - **`directory_overrides` for path remapping** — `[directory_overrides.<name>]`
    in `machine.toml` for per-machine path adjustments (PORT-01..05). Common
    case: `~/.claude/plugins/cache` differs between macOS and Linux.
  - **What happens when `claude` is missing on Machine B** — ADP-02 error
    behaviour; how to install Claude Code; `tome sync` partial-failure exit
    code semantics (RECON-04 vanished + ADP-04 install failure).
  - **Migrating a v0.9 library on Machine B** — When dotfiles include a
    pre-v0.10 library, `tome sync` refuses with a hint pointing at `tome
    migrate-library`. Walk the user through the migration flow (matches the
    UX-02 confirmation gate landed in this phase).
- **D-DOC03-3 (linking strategy):**
  - Linked from `tome sync --help` long description. Per the `clap` long-form
    `#[command(long_about = ...)]` shape (or whatever `cli.rs` uses today).
    Link target depends on deploy: if mdbook is published at a stable URL,
    use that; otherwise use a relative `docs/src/cross-machine-sync.md` path
    that works for users running `--help` inside a clone. **Claude's
    discretion** to pick based on what the existing `--help` strings do
    (likely no published-URL convention yet; relative path is fine).
  - Linked from architecture.md's new "Library-canonical model" section.
  - Listed in `docs/src/SUMMARY.md`.

### Claude's Discretion

The following are implementation details not worth user input; they follow
established codebase conventions:

- Exact wording of cleanup hint strings (within Conflict/Why/Suggestion shape;
  the planner sketches in D-UX01-3 are illustrative, not literal contract).
- Bucket ordering in cleanup output (recommend A → B → C since A is the
  Unowned-transition story line that v0.10 specifically addresses; planner can
  reorder if a different ordering reads better).
- Color / glyph choices per bucket. Recommend reusing
  `console::style(...).yellow().bold()` for headers (matches today's Case-2),
  with `dim` for source-attribution parenthesis. Bucket C may use a different
  shade if it improves scanability.
- Exact text of bucket header lines (number agreement, plurals).
- `CleanupSummary` struct shape vs. side-channel `Vec<ExcludedSkill>` — both
  acceptable per D-UX01-2; planner picks.
- Migration table column widths and overflow behaviour. `tabled`
  `PriorityMax::right()` truncation precedent from WHARD-07 is available.
- Truncation policy when migration plan has >N skills (recommend show-all up
  to 100, then truncate with "...and N more"; defer to planner).
- `--yes` short form `-y` (recommend yes; matches Phase 14 D-B3 and
  conventional Unix CLI ergonomics).
- `byte_size` field name in `MigrationEntry` (or alternative); whether to
  serialize (probably not — purely transient).
- Helper for human-readable byte size — recommend `humansize` crate (~10 LOC,
  pure-Rust) if not already pulled in transitively, else inline a small `fn
  humanize_bytes(u64) -> String` in `paths.rs` or `migration_v010.rs`.
- CHANGELOG.md tone — recommend the existing CHANGELOG.md house style (it's
  already structured around milestone sections with sub-headers per feature
  category). The three breaking changes call out:
  1. **Library shape conversion required** — pre-v0.10 libraries must run
     `tome migrate-library` once. Followed by a confirmation prompt; `--dry-run`
     for preview.
  2. **Plugin updates require `tome sync`** — Claude Code plugin updates no
     longer auto-propagate via symlink (former cache → library symlink path
     no longer exists). Users opt into upstream changes via `tome sync`.
  3. **`tome remove <name>` → `tome remove dir <name>`** — clap subcommand
     split per Phase 14 D-API-2. New `tome remove skill <name>` for Unowned
     skill deletion.
  Plus a clear migration-step paragraph at the top of the v0.10 section
  walking users through `tome migrate-library --dry-run` → review →
  `tome migrate-library`.
- Excalidraw diagram update — defer to a follow-up issue if the existing
  diagram still represents the broad two-tier flow accurately after text
  updates. Track in `16-deferred-items.md` if deferred.
- Whether to update `roadmap.md` (`docs/src/roadmap.md`) — likely yes (drop
  v0.10 from "in progress"); planner's discretion based on what the existing
  doc looks like.
- JSON shape of cleanup output if `--json` is added later — explicitly
  out-of-scope for v0.10; flag as a v0.11 follow-up if planner notices a
  small surface-area change makes future JSON cheaper.

### Folded Todos

(None — `gsd-tools.cjs todo match-phase 16` returned 0 matches.)

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### Design + planning context (this milestone)

- `.planning/research/v0.10-library-canonical-design.md` — v0.10 design
  exploration; the "no longer configured" UX question and library-as-dotfiles
  realization that motivated this milestone surface here.
- `.planning/REQUIREMENTS.md` — UX-01, UX-02, DOC-01, DOC-02, DOC-03 are this
  phase's requirements. Phase 14 vocabulary supersession note (lines 181-186)
  applies to DOC-01/DOC-02. Phase 11 D-01 supersedes UX-02's "first-sync
  prompt" wording (see <domain> above).
- `.planning/ROADMAP.md` — Phase 16 section: goal, success criteria,
  dependencies (depends on Phases 13/14/15 — confirmed shipped).
- `.planning/PROJECT.md` — Library-as-dotfiles is the **core value**; key
  decisions D-LIB-01..05 cover the v0.10 model shifts that DOC-01 must
  document.

### Prior phase context (decisions to honour)

- `.planning/phases/11-library-canonical-core/11-CONTEXT.md` — D-01
  (`tome migrate-library` is the migration mechanism, NOT auto-on-first-sync —
  supersedes UX-02 wording); D-04 (broken-symlink preservation; UX-02 confirm
  flow must respect this); D-08 (drift basis is content_hash, not version —
  DOC-01 reconciliation section must say this); D-09/D-10 (cleanup Case 1/2
  semantics — UX-01 Buckets A+B preserve this).
- `.planning/phases/14-unowned-library-lifecycle/14-CONTEXT.md` — D-API-1/-2
  (vocab merge: `tome reassign --to`, `tome remove skill`; NO
  `tome adopt`/`tome forget`); D-B3 (`--yes` flag pattern for destructive
  commands; UX-02 D-UX02-2 mirrors this); D-D3 (unowned set excluded from
  total_issues — DOC-01 unowned section must call out).
- `.planning/phases/13-lockfile-authoritative-sync/13-CONTEXT.md` — Reconcile
  classification model (Match/Drift/Vanished); `auto_install_plugins`
  consent values; `--no-install` global flag; DOC-01 reconciliation section
  + DOC-03 reference sections both consume this.
- `.planning/phases/15-cli-hardening/15-CONTEXT.md` — HARD-15 wizard chrome to
  stderr (UX-01 D-UX01-4 follows the same discipline);
  `Config::save_checked` + `paths::unexpand_tilde` (HARD-22; DOC-01 module
  list update + DOC-03 cross-machine config example); HARD-21 browse
  Disable/Enable wiring (DOC-01 module list update); `tabled::Style::rounded`
  + `PriorityMax::right` truncation precedent.

### Codebase modules being changed in Phase 16

- `crates/tome/src/cleanup.rs` — Three-bucket partition rewrite (UX-01
  D-UX01-1..-4). `cleanup_library` (line 43) and `cleanup_target` (line 236)
  both touched; new `CleanupSummary` (or side-channel) coordinates output.
- `crates/tome/src/migration_v010.rs` — Confirm gate (UX-02 D-UX02-1),
  `--yes` flag wiring (D-UX02-2), tabled summary table (D-UX02-3),
  `metadata().len()` walk (D-UX02-4). `MigrationEntry.byte_size:
  Option<u64>` field added; `render_plan` rewritten to return / emit the
  summary line + table; `cmd_migrate_library` (in `lib.rs`) routes the
  prompt + flag.
- `crates/tome/src/cli.rs` (or wherever `Command::MigrateLibrary` lives) —
  Add `yes: bool` arg with `#[arg(long, short = 'y')]` (Phase 14 D-B3
  pattern).
- `crates/tome/src/lib.rs::cmd_migrate_library` — Wire `--yes` through;
  invoke prompt unless `dry_run` or `yes`; bail under `--no-input` without
  `--yes`.
- `docs/src/architecture.md` — Targeted rewrites + 4 new sections (DOC-01
  D-DOC01-1).
- `docs/src/cross-machine-sync.md` — **NEW.** Walkthrough + reference
  (DOC-03 D-DOC03-1..-3).
- `docs/src/SUMMARY.md` — Add cross-machine-sync.md entry.
- `CHANGELOG.md` — v0.10 entry with three breaking-change call-outs +
  migration step (DOC-02). Use the existing house style.
- `tome sync --help` (long description in `cli.rs`) — Link to
  cross-machine-sync.md.

### Implementation precedent (existing patterns to mirror)

- `crates/tome/src/wizard.rs` — `tabled::Style::rounded()` + ceremonial
  summary precedent (WHARD-07). Reuse for migration plan table (D-UX02-3).
- `crates/tome/src/remove.rs::FailureKind::ALL` — POLISH-04 compile-time
  exhaustiveness pattern. Cleanup output may not need this directly, but the
  planner may want it for any new enum (e.g. `CleanupBucket`).
- `crates/tome/src/remove.rs` — `--yes` flag pattern (Phase 14 D-B3 — `tome
  remove skill --yes`). UX-02 mirrors structurally.
- `crates/tome/src/cleanup.rs:178` — `eprintln!("warning: skill ...")` line
  shape — D-UX01-3 hints follow the same line-per-skill conventions.
- `crates/tome/src/paths.rs::collapse_home` — Existing tilde-collapsing
  helper for path display. Migration table SOURCE column should use it.
- `crates/tome/src/browse/markdown.rs` (precedent for prose rendering, if
  any). Likely not used in this phase but available.

### Specs / ADRs

- No formal ADRs in this codebase. The decision log is `PROJECT.md` Key
  Decisions table + per-phase CONTEXT.md files.

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets

- **`tabled::Style::rounded()` + `PriorityMax::right()` truncation pattern**
  — From `wizard.rs` (WHARD-07). Reuse for the migration plan summary table.
- **`dialoguer::Confirm::new().with_prompt(...).default(false).interact_opt()`**
  — Pattern from `cleanup.rs:166-169`. Mirror for `tome migrate-library`
  confirm gate.
- **`paths::collapse_home(path)`** — Already used by
  `migration_v010::render_plan` (line 258) for `~/`-style display. Reuse
  in the new tabled SOURCE column.
- **`console::style(...).yellow().bold()` + `console::style(...).dim()`** —
  Used for cleanup output today (`cleanup.rs:148-160`). Consistent across all
  three buckets in D-UX01-3.
- **`MigrationFailureKind::ALL` + exhaustive-match sentinel** — POLISH-04
  pattern from `migration_v010.rs:53-76`. Available if the planner adds a
  new enum (e.g. `CleanupBucket` if implemented as enum-driven dispatch).
- **`walkdir::WalkDir::new(...).follow_links(true)`** — Existing pattern in
  `migration_v010::copy_dir_recursive_resolving`. Use the same shape for the
  `metadata().len()` walk (D-UX02-4) — though the disk-estimate walk should
  use `follow_links(false)` to avoid double-counting if symlinked subdirs
  exist.
- **`migration_v010::MigrationEntry`** — Already structured with
  `library_path`, `raw_link_target`, `source_reachable`. Adding `byte_size:
  Option<u64>` is a one-field extension.
- **Phase 14 `--yes` flag wiring** — `Command::Remove { kind: RemoveKind {
  Skill { yes, .. }, .. }, .. }` pattern (Phase 14 D-API-2). Mirror for
  `Command::MigrateLibrary { yes: bool, dry_run: bool, .. }`.

### Established Patterns

- **stderr discipline** — Wizard chrome (HARD-15), cleanup messages
  (`cleanup.rs:110, 178`), and migration warnings (`migration_v010.rs:273`)
  all use `eprintln!`. UX-01 D-UX01-4 honours this; the today's `println!`
  for the interactive Case-2 prompt header (`cleanup.rs:146`) drops away
  when `dialoguer::Confirm` (which writes to stderr) takes over.
- **Conflict / Why / Suggestion error template** — Phase 7 D-10. Apply to
  `--no-input` without `--yes` bail message (UX-02 D-UX02-2):
  ```
  error: tome migrate-library is destructive (converts symlinks to real copies).
    Why: --no-input mode skips the confirmation prompt; --yes is required to
         confirm.
    Suggestion: re-run with `--yes` to proceed, or remove `--no-input` for the
                interactive prompt.
  ```
- **`tabled` ceremonial summary** — WHARD-07 precedent. One-shot commands
  with weighty output use `Style::rounded()`; repeated-use commands (`tome
  status`) use `Style::blank()`. Migration plan summary is one-shot →
  rounded.
- **Plan / render / execute** — `migration_v010` already follows this. The
  confirm gate slots in **between render_plan and execute** in
  `cmd_migrate_library`. No structural change beyond a `pub fn
  prompt_confirmation(yes: bool, no_input: bool) -> Result<bool>` helper.

### Integration Points

- **`lib.rs::cmd_migrate_library`** — Per Phase 15 HARD-02 decomposition,
  this is now a top-level helper. Confirm-gate wiring lands here:
  ```rust
  pub(crate) fn cmd_migrate_library(
      paths: &TomePaths,
      dry_run: bool,
      yes: bool,
      no_input: bool,
  ) -> Result<()> {
      let manifest = manifest::load(&paths.library_dir)?;
      let plan = migration_v010::plan(&paths.library_dir, &manifest)?;
      migration_v010::render_plan(&plan);  // emits summary line + tabled table
      if !dry_run {
          if !migration_v010::prompt_confirmation(yes, no_input)? {
              return Ok(());  // user said no; clean exit code 0
          }
      }
      let result = migration_v010::execute(&plan, dry_run)?;
      // existing partial-or-failed bail logic...
  }
  ```
- **`SyncReport` / cleanup invocation** — `cleanup_library` and
  `cleanup_target` are called from `lib.rs::sync`. Coordinating their output
  for the unified three-bucket render means either:
  - Threading a `&mut CleanupSummary` through both calls, or
  - Returning `CleanupResult` shapes from both that the caller drains
    before rendering.
  The planner picks; either honors D-UX01-2.
- **`docs/src/SUMMARY.md`** — mdbook table-of-contents. Single-line entries;
  existing pages match the pattern. `cross-machine-sync.md` slots in between
  Configuration and Architecture per D-DOC03-1.

</code_context>

<specifics>
## Specific Ideas

- The "no longer configured" cleanup wording was the **literal trigger** for
  this entire v0.10 milestone discussion. UX-01's three-bucket partition is
  the user-facing closure of that thread; the wording **must not silently
  reuse "no longer configured"** in any of the three buckets — that phrase is
  exactly the conflated message we're rewriting away from. The illustrative
  examples in D-UX01-3 use specific bucket-distinct language ("no longer in
  any source", "missing from configured source on disk", "now in exclude
  list").
- The migration prompt should feel like a serious one-shot ceremony, not a
  speed bump. The pre-existing render_plan warning ("tome does not snapshot
  your library before migrating; commit your library directory to git ...")
  is the right tone — a calm reminder that this is irreversible. The
  confirmation prompt sits below that warning naturally.
- Cross-machine sync page tone — direct, walkthrough-style, no "production
  guidance" boilerplate. Two numbered walks (Machine A / Machine B), then
  reference. Reader can skim the walk and dive into reference when something
  surprises them.
- The CHANGELOG.md v0.10 section should lead with the migration step (one
  paragraph, three commands: `tome migrate-library --dry-run`, review,
  `tome migrate-library`). Most users encountering the changelog will be
  upgrading; that's the most important paragraph for them.

</specifics>

<deferred>
## Deferred Ideas

- **JSON shape for cleanup output** — When `--json` arrives (post-v0.10),
  the three-bucket partition translates to a `cleanup: { removed_from_config:
  [], missing_from_disk: [], now_excluded: [] }` shape. Out of scope for
  v0.10; flag if the planner notices a small structural change makes future
  JSON cheaper.
- **Excalidraw architecture diagram refresh** — The two-tier flow (sync →
  library → distribution) is visually stable for v0.10, but the marketplace
  adapter dispatcher and unowned lifecycle aren't depicted. Defer to a
  v0.11+ doc-polish pass; track in `16-deferred-items.md` if deferred at
  plan time.
- **`tome migrate-library --revert` / undo flag** — Out of scope per Phase 11
  D-04. Broken-symlink preservation gives a partial recovery story; full undo
  requires snapshotting the v0.9 symlink shape, which is too much new
  machinery for a transitional command. Track as v0.11+ if anyone hits a
  case the existing recovery doesn't handle.
- **Localized docs (en/de/ja)** — Out of scope; defer indefinitely. tome
  is a single-developer tool; localization isn't a milestone candidate.
- **Doctor command remediation hints expansion** — Phase 14 already wired
  unowned-section surfacing in `tome doctor`; further remediation suggestions
  (e.g. "run `tome reassign foo --to <dir>` to re-anchor") are a Phase 17 or
  v0.11+ polish. Flag if cleanup hints (D-UX01-3) make the doctor surface
  feel under-documented by comparison.
- **`humansize` crate vs. inline helper** — Either acceptable; if `humansize`
  isn't already in the dep tree, the inline 10-LOC helper is the lighter
  choice. Decide at plan time.
- **Truncation policy for migration plan tables with hundreds of skills** —
  Show-all up to ~100 is the recommendation; truncate with "...and N more"
  beyond that. Concrete threshold can be Claude's discretion.
- **CHANGELOG.md prior-milestone shape audit** — If the existing house
  style doesn't already separate breaking/non-breaking, this phase shouldn't
  be the first to do so. Match what's there. Flag as v0.11+ if the existing
  shape is awkward.

### Reviewed Todos (not folded)

(None — `gsd-tools.cjs todo match-phase 16` returned 0 matches.)

</deferred>

---

*Phase: 16-cleanup-message-ux-docs*
*Context gathered: 2026-05-08*
