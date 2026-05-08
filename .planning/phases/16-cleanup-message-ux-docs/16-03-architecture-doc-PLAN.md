---
phase: 16-cleanup-message-ux-docs
plan: 03
type: execute
wave: 2
depends_on:
  - 16-01
  - 16-02
files_modified:
  - docs/src/architecture.md
autonomous: true
requirements:
  - DOC-01

must_haves:
  truths:
    - "`docs/src/architecture.md` describes the v0.10 library-canonical model: managed AND local skills are stored as real-directory copies in the library; managed = update channel, not symlink-storage."
    - "Four new sections — Library-canonical model, Lockfile-authoritative reconciliation, Marketplace adapter trait, Unowned lifecycle — appear between the existing Key Patterns and Testing sections."
    - "The pre-Phase-11 framing — \"managed skills are symlinked from library → source\" / \"library is a consolidated cache\" — is removed wherever it conflicts with v0.10 semantics."
    - "Phase 14 D-API-1/-2 vocabulary merge is honored: the doc references `tome reassign --to <dir>` for re-anchor and `tome remove skill <name>` for delete; the strings `tome adopt` and `tome forget` are NOT used as commands (only acceptable as historical context with explicit supersession note)."
    - "Modules list is updated with new entries for `marketplace.rs`, `reconcile.rs`, `migration_v010.rs`, `summary.rs`, and the existing entries for `library.rs`, `cleanup.rs`, `manifest.rs`, `lockfile.rs`, `remove.rs`, `reassign.rs` reflect Phase 11/13/14/15 schema and behavior changes."
    - "Net length lands in the 225-275 line range per CONTEXT.md D-DOC01-1 estimate (existing ~75 lines + ~150-200 added + ~40 edited)."
  artifacts:
    - path: "docs/src/architecture.md"
      provides: "v0.10-current architecture documentation: library-canonical model + reconciliation + adapter trait + unowned lifecycle"
      min_lines: 200
      contains: "Library-canonical model"
  key_links:
    - from: "architecture.md Modules list"
      to: "crates/tome/src/marketplace.rs, reconcile.rs, migration_v010.rs, summary.rs"
      via: "module entries describing each new module's purpose"
      pattern: "marketplace\\.rs|reconcile\\.rs|migration_v010\\.rs|summary\\.rs"
    - from: "architecture.md Library-canonical model section"
      to: "Phase 11 D-08 (drift basis = content_hash, not version)"
      via: "explicit callout that drift is content-hash-based"
      pattern: "content.hash|content_hash"
    - from: "architecture.md Unowned lifecycle section"
      to: "tome reassign --to / tome remove skill (D-API-1/-2 vocab)"
      via: "command references"
      pattern: "tome reassign --to|tome remove skill"
---

<objective>
Update `docs/src/architecture.md` to describe the v0.10 architecture per DOC-01 + CONTEXT.md D-DOC01-1. Today's doc (60 lines) is structured around the v0.6 two-tier model with managed-as-symlink framing. After this plan: targeted rewrites to four existing paragraphs (Consolidate, Distribute, Two-tier model, Modules list) + four NEW sections inserted between Key Patterns and Testing — Library-canonical model, Lockfile-authoritative reconciliation, Marketplace adapter trait, Unowned lifecycle.

CRITICAL vocabulary constraints (NON-NEGOTIABLE):
- D-API-1 / D-API-2 vocab merge: NO `tome adopt`, NO `tome forget` as live commands. Use `tome reassign <skill> --to <dir>` (re-anchor) and `tome remove skill <name>` (delete). Mention the supersession only in a brief Phase-history footnote if needed.
- Phase 11 D-01 supersession of UX-02 wording: migration is the one-shot `tome migrate-library` CLI command, NOT auto-on-first-sync.
- Phase 11 D-08 drift basis: content_hash, not version. Version is display-only in diff output.

Purpose: closes DOC-01. Anyone reading architecture.md after this plan understands the v0.10 model without needing to chase decision logs in `.planning/`.

Output: a rewritten `docs/src/architecture.md` (~225-275 lines).
</objective>

<execution_context>
@$HOME/.claude/get-shit-done/workflows/execute-plan.md
@$HOME/.claude/get-shit-done/templates/summary.md
</execution_context>

<context>
@.planning/PROJECT.md
@.planning/ROADMAP.md
@.planning/REQUIREMENTS.md
@.planning/phases/16-cleanup-message-ux-docs/16-CONTEXT.md

@docs/src/architecture.md

<interfaces>
Existing v0.9 framing in `docs/src/architecture.md` that must be REWRITTEN (verbatim quotes from current file):

Line 16 (Consolidate paragraph):
> **Consolidate** (`library.rs`) — Two strategies depending on directory role: **managed** skills (Claude plugins, git clones) are symlinked from library → source dir so the package manager continues to own the bytes; **local** skills (`directory`/`synced` sources) are copied into the library (the library is the canonical home).

Line 17 (Distribute paragraph):
> **Distribute** (`distribute.rs`) — Push library skills to every directory whose role is `synced` or `target` via symlinks. Skills disabled in `machine.toml` (globally or per-directory) are skipped, as are directories on the `disabled_directories` list.

Line 43 (Two-tier model paragraph):
> **Two-tier model**: Discovery directories →(consolidate)→ Library →(distribute)→ Distribution directories. The library is the source of truth. Managed skills (Claude plugins, git clones) are symlinked from library → source dir; local skills (`directory`/`synced` sources) are copied into the library.

Existing modules (~lines 22-39) — these must be UPDATED:
- `library.rs` line 16 — reframe to "managed AND local skills both copied as real dirs"
- `cleanup.rs` (implicit in Sync Pipeline step 4 line 18) — reframe as "no longer auto-deletes orphans; transitions to Unowned per LIB-04 + three-bucket UX per UX-01"
- `manifest.rs` line 29 — add `source_name: Option<DirectoryName>`, `previous_source: Option<DirectoryName>` schema lift (LIB-03 + Phase 14)
- `lockfile.rs` line 30 — add `previous_source` field; mention that lockfile is now authoritative for managed skill version per RECON-01..05
- `remove.rs` line 25 — note the Phase 14 D-API-2 subcommand split: `tome remove dir <name>` + `tome remove skill <name>`
- `reassign.rs` line 25 — note Phase 14 D-API-1: accepts Unowned input; `--force` flag for D-A1 collision check
- `update.rs` line 32 — note that `reconcile.rs` is now the primary triage path; update.rs may be slimmed or deleted entirely (verify by reading current crates/tome/src/lib.rs to see whether update.rs survives Phase 13)

NEW modules to ADD to the Modules list:
- `marketplace.rs` — `MarketplaceAdapter` trait + `ClaudeMarketplaceAdapter` + `GitAdapter` + `InstallFailure`/`InstallOp`/`InstallFailureKind`
- `reconcile.rs` — Match/Drift/Vanished classification, `auto_install_plugins` consent, edit-in-library 3-way prompt
- `migration_v010.rs` — One-shot v0.9→v0.10 library migration; deleted in v0.11+
- `summary.rs` — `SkillSummary` shared type for status + doctor unowned sections (Phase 14 14-02)

Phase 11 D-08 callout (must appear in Lockfile-authoritative reconciliation section):
"Drift detection is content-hash-based, not version-based. The version string is display-only in the diff output (e.g., `plugin X: 5.0.5 → 5.0.7`); a content-hash mismatch is what triggers the reconcile prompt. Because Claude CLI doesn't accept `--version` on `claude plugin install`, true version pinning is upstream future work."

Phase 14 D-D3 callout (Unowned lifecycle section):
"The unowned set is informational — `tome doctor`'s `total_issues()` is unaffected by it; exit code does not change."

Excalidraw diagram (line 3) — KEEP per CONTEXT.md `<decisions>`:
> **[System Diagram (Excalidraw)](https://excalidraw.com/#json=5-pjpDsna4Way3lfGW5km,p0bQwpcJEl6do68RrnKAgw)** — interactive diagram showing the two-tier discovery → library → distribution flow.

Per CONTEXT.md `<decisions>` "Claude's Discretion": defer Excalidraw diagram refresh to a follow-up issue if the existing diagram still represents the broad two-tier flow accurately. Track in `16-deferred-items.md` if deferred at plan time.
</interfaces>
</context>

<tasks>

<task type="auto">
  <name>Task 1: Rewrite four existing paragraphs + Modules list for v0.10 framing</name>
  <files>docs/src/architecture.md</files>
  <read_first>
    - docs/src/architecture.md (entire file — 60 lines; locate verbatim text from `<interfaces>` above)
    - .planning/phases/16-cleanup-message-ux-docs/16-CONTEXT.md (D-DOC01-1 — full rewrite/add scope)
    - crates/tome/src/lib.rs (sync pipeline step ordering for the rewritten Sync Pipeline section)
    - crates/tome/src/library.rs (current consolidate logic — verify the wording matches reality)
    - crates/tome/src/marketplace.rs (the trait + ClaudeMarketplaceAdapter + GitAdapter — for the new module entry; cross-check method names)
    - crates/tome/src/reconcile.rs (ReconcileClass + ReconcileReport — for the new module entry)
    - crates/tome/src/manifest.rs (current SkillEntry shape with source_name + previous_source)
  </read_first>
  <action>
    **Step 1: Rewrite the Sync Pipeline Consolidate paragraph (line 16).** Replace verbatim text with:
    ```
    **Consolidate** (`library.rs`) — Both managed AND local skills are stored as real-directory copies in the library — the library is the canonical home for every skill (LIB-01 / LIB-02). Managed skills (Claude plugins, git clones) get a recursive copy from the source on first sync; subsequent syncs use the marketplace adapter (`reconcile.rs`) to pull updates. Local skills (`directory`/`synced` sources) are copied as today. The `managed: bool` flag now means "update channel" (managed = upstream feeds updates; local = library is canonical), NOT "stored as a symlink". A manifest (`.tome-manifest.json`) tracks SHA-256 content hashes for idempotent updates.
    ```

    **Step 2: Rewrite the Distribute paragraph (line 17).** Replace with:
    ```
    **Distribute** (`distribute.rs`) — Push library entries (Owned and Unowned alike) to every directory whose role is `synced` or `target` via symlinks. The library is canonical, so distribution is a downhill operation; Unowned skills (manifest `source_name = None`) still get distributed because library content is preserved per LIB-04. Skills disabled in `machine.toml` (globally or per-directory) are skipped; directories on `disabled_directories` are skipped. `distribute` refuses to clobber pre-existing symlinks pointing outside the current library (HARD-09 foreign-symlink protection).
    ```

    **Step 3: Rewrite the Cleanup pipeline step (line 18) to reflect Phase 16 UX-01:**
    ```
    **Cleanup** (`cleanup.rs`) — Stale-candidate skills are partitioned into three buckets per UX-01: **removed-from-config** (source dir was removed from `tome.toml` — manifest entry transitions to Unowned, library content preserved per LIB-04), **missing-from-disk** (source dir still configured but file vanished — library copy removed), **now-in-exclude-list** (skill added to `machine.toml::disabled` — distribution symlinks removed, library copy preserved). Each bucket renders with a header + per-skill resolution hint to stderr. Broken symlinks in distribution directories are also cleaned per HARD-09 origin check.
    ```

    **Step 4: Rewrite the Two-tier model bullet under Key Patterns (line 43).** Replace with:
    ```
    **Library-canonical model**: Discovery directories →(consolidate)→ Library →(distribute)→ Distribution directories. The library is the source of truth and every skill (managed or local) lives there as a real directory copy. Managed skills use a marketplace adapter (`reconcile.rs` + `marketplace.rs`) to pull updates from upstream; local skills are edited in-place in the library. Distribution always uses Unix symlinks (`std::os::unix::fs::symlink`) pointing into the library. Unix-only.
    ```

    **Step 5: Update the Modules list (lines 22-39) per `<interfaces>` above.** For each module that already has an entry, edit the entry. For each new module (`marketplace.rs`, `reconcile.rs`, `migration_v010.rs`, `summary.rs`), insert a new bullet in alphabetical position. Concrete additions:

    - `marketplace.rs` — `MarketplaceAdapter` trait (six methods: `id`, `current_version`, `install`, `update`, `list_installed`, `available`) + `ClaudeMarketplaceAdapter` (subprocess to `claude plugin install/update/list --json`, `RefCell` cache) + `GitAdapter` (thin shim over `git.rs`). `InstallFailure`/`InstallOp`/`InstallFailureKind` for partial-failure aggregation (mirrors `RemoveFailure`).
    - `reconcile.rs` — `tome sync` reconciliation core: classifies each managed skill in the lockfile as Match/Drift/Vanished (`ReconcileClass`); resolves `auto_install_plugins` consent; renders per-skill diff before applying installs/updates; verifies post-install content_hash; surfaces edit-in-library detection with the fork/revert/skip 3-way prompt (RECON-01..05).
    - `migration_v010.rs` — One-shot `tome migrate-library` command for converting v0.9-shape libraries (managed = symlink) to v0.10 shape (managed = real-dir copy). Idempotent; broken symlinks preserved per D-04. Slated for removal in v0.11+ once all known users have migrated.
    - `summary.rs` — `SkillSummary` shared type (NAME / LAST-KNOWN SOURCE / SYNCED columns) consumed by `status.rs` and `doctor.rs` Unowned sections (Phase 14).
    - Update `manifest.rs` entry to mention `source_name: Option<DirectoryName>` (None = Unowned) and `previous_source: Option<DirectoryName>` (closes Phase 13 D-13 fork-in-place gap).
    - Update `remove.rs` entry to mention the Phase 14 D-API-2 subcommand split: `tome remove dir <name>` (formerly `tome remove <name>`) and the new `tome remove skill <name>`.
    - Update `reassign.rs` entry to mention D-API-1: accepts Unowned input + the `--force` flag.

    **Step 6: Verify post-edit forbidden phrases.** Run `rg -n 'tome adopt\|tome forget\|consolidated cache' docs/src/architecture.md` — both should produce zero matches (or one match each ONLY in a clearly-marked supersession footnote with text like "v0.10 D-API-1/-2 superseded the originally-proposed `tome adopt`/`tome forget` verbs").
  </action>
  <verify>
    <automated>cd /Users/martin/dev/opensource/tome &amp;&amp; cargo build -p tome &amp;&amp; rg -c 'managed|library' docs/src/architecture.md</automated>
  </verify>
  <acceptance_criteria>
    - `rg -n 'symlinked from library → source' docs/src/architecture.md` outputs zero matches (v0.9 framing removed)
    - `rg -n 'consolidated cache' docs/src/architecture.md` outputs zero matches (v0.9 framing removed)
    - `rg -n 'real.directory copies|real-directory copies' docs/src/architecture.md` outputs at least one match (v0.10 framing present)
    - `rg -n 'marketplace\.rs|reconcile\.rs|migration_v010\.rs|summary\.rs' docs/src/architecture.md` outputs at least four matches (one per new module)
    - `rg -n 'source_name: Option' docs/src/architecture.md` outputs at least one match (LIB-03 schema lift mentioned)
    - `rg -n 'tome remove skill|tome remove dir' docs/src/architecture.md` outputs at least one match each (D-API-2 vocab present)
    - `rg -n 'tome reassign.* --to' docs/src/architecture.md` outputs at least one match (D-API-1 vocab present)
    - `rg -n 'tome adopt' docs/src/architecture.md` outputs zero matches OR exactly one match in a "superseded by" supersession footnote
    - `rg -n 'tome forget' docs/src/architecture.md` outputs zero matches OR exactly one match in a "superseded by" supersession footnote
  </acceptance_criteria>
  <done>
    Existing four paragraphs are rewritten with v0.10 framing; Modules list is updated for Phase 11/13/14/15 changes and adds entries for `marketplace.rs`, `reconcile.rs`, `migration_v010.rs`, `summary.rs`. Forbidden v0.9 phrases removed; D-API-1/-2 vocab in place.
  </done>
</task>

<task type="auto">
  <name>Task 2: Add four new sections — Library-canonical model, Lockfile-authoritative reconciliation, Marketplace adapter trait, Unowned lifecycle</name>
  <files>docs/src/architecture.md</files>
  <read_first>
    - docs/src/architecture.md (after Task 1 — locate the spot between Key Patterns and Testing where the four sections will be inserted)
    - .planning/phases/16-cleanup-message-ux-docs/16-CONTEXT.md (D-DOC01-1 — section content scope; Phase 11 D-01, D-04, D-08 callouts)
    - .planning/PROJECT.md (Key Decisions table — D-LIB-01..05 for the Library-canonical model section content)
    - crates/tome/src/marketplace.rs (the trait — confirm method names + signatures for the trait section)
    - crates/tome/src/reconcile.rs (ReconcileClass variants + flow — for the reconciliation section)
    - crates/tome/src/manifest.rs (SkillEntry::new_unowned constructor + previous_source semantics — for the Unowned lifecycle section)
  </read_first>
  <action>
    **Step 1: Locate the insertion point.** Find the line `## Testing` in `docs/src/architecture.md` (after Task 1 it should be around line 65-75 depending on Task 1's edits). Insert the four new sections IMMEDIATELY before `## Testing`.

    **Step 2: Insert section 1 — `## Library-canonical model`.** Content (~40-50 lines):
    ```
    ## Library-canonical model

    The library at `~/.tome/library/` is the single source of truth for every
    skill on the machine — managed AND local. Both kinds are stored as real
    directory copies; neither is a symlink into a marketplace cache.

    **What changed in v0.10 (vs. v0.9):** Pre-v0.10, managed skills (Claude
    plugins, git clones) lived in the library as symlinks pointing back at the
    package manager's cache. Updating a plugin updated the library transparently
    via the symlink. The trade-off was that removing a source erased every
    skill it provided, and shipping the library across machines was impossible
    because the symlink targets weren't portable. v0.10 inverts the relationship:
    `consolidate_managed` performs a recursive copy on first sync (and on every
    update afterward via the marketplace adapter), so the library on disk is
    fully self-contained.

    **Why this matters:**
    - **Cross-machine portability** — `~/.tome/` can be committed to dotfiles
      and cloned onto a fresh machine. Pair with `tome.lock` for exact-version
      reproducibility (see Lockfile-authoritative reconciliation below).
    - **Source removal preserves content** — removing a `[directories.*]` entry
      from `tome.toml` (or running `tome remove dir <name>`) no longer erases the
      skills it provided. Their manifest entries transition to Unowned
      (`source_name: None`); library content stays in place (LIB-04).
    - **Vanished plugins stay usable** — if a marketplace removes a plugin,
      `tome sync` warns but keeps using the preserved library copy.
    - **Drift basis is content_hash, not version** (Phase 11 D-08). Reconcile
      compares `content_hash(library/<skill>)` against the lockfile entry; the
      version string is display-only in diff output (e.g.
      `plugin X: 5.0.5 → 5.0.7`). Because the upstream `claude plugin install`
      command doesn't accept `--version`, true version pinning is future work.

    **Migration path:** v0.9-shape libraries (managed = symlink) need to run
    `tome migrate-library` once. Detection: `library_dir/<name>.is_symlink() &&
    manifest[name].managed == true && manifest.contains_key(name)` (Phase 11
    D-03). The command shows a summary table (skill count + disk estimate),
    prompts for confirmation, then converts symlinks to real copies. Broken
    symlinks are preserved in place per Phase 11 D-04. Idempotent on re-run;
    `tome sync` refuses to operate on v0.9-shape libraries until migration runs.
    The migration is a one-shot CLI command, NOT auto-on-first-sync (Phase 11
    D-01).
    ```

    **Step 3: Insert section 2 — `## Lockfile-authoritative reconciliation`.** Content (~40-50 lines):
    ```
    ## Lockfile-authoritative reconciliation

    `tome.lock` is the cross-machine state contract. Cargo.lock-shaped: each
    managed skill records `(name, version, content_hash, source_name,
    previous_source, registry_id, git_commit_sha)`. On every `tome sync`,
    `reconcile.rs::reconcile_lockfile` classifies each managed skill into one
    of three buckets:

    - **Match** — `content_hash(library/<skill>) == lockfile.content_hash`.
      Nothing to do.
    - **Drift** — content_hash differs OR the lockfile expects a version the
      adapter no longer provides. Renders a per-skill diff
      (`plugin X: 5.0.5 → 5.0.7`); applies the install/update via the
      marketplace adapter; verifies the resulting library content_hash matches
      the lockfile entry.
    - **Vanished** — `adapter.available()` returns false (the marketplace no
      longer offers the plugin). Stderr warning, distribution continues from
      the preserved library copy.

    **Auto-install consent** — the first sync on a machine with a non-empty
    drift set prompts: "Auto-install missing plugins on every sync? [Y/n/never]".
    The choice persists in `machine.toml::auto_install_plugins`. The
    `--no-install` global flag overrides the persisted choice for the current
    invocation (mirrors Cargo's `--frozen` / `--locked`).

    **Edit-in-library detection** — when `managed: true` and
    `content_hash(library/<skill>) != lockfile.content_hash`, the user is
    prompted with three choices: **fork** (default — promote to local via
    `tome fork` machinery), **revert** (overwrite from marketplace), **skip**
    (warn and don't touch this entry this sync). In `--no-input` mode the
    default is **skip with warning** so edited content is never silently
    overwritten.

    **`previous_source` breadcrumb** — when a managed skill forks in-place
    (Drift → fork), the manifest entry records the old `source_name` in
    `previous_source` before flipping to local. This closes the Phase 13 D-13
    "lossy fork-in-place" gap; `tome status` and `tome doctor` show the
    last-known source so the user can re-anchor cleanly later via
    `tome reassign <skill> --to <dir>`.

    **Partial-failure surfacing** — adapter `install`/`update` errors aggregate
    into `Vec<InstallFailure>` and render as a grouped `⚠ N install operations
    failed` summary (matches the v0.8 SAFE-01 `RemoveFailure` pattern). Library
    distribution still completes for skills whose adapter calls succeeded; sync
    exits non-zero on partial failure.
    ```

    **Step 4: Insert section 3 — `## Marketplace adapter trait`.** Content (~30-40 lines):
    ```
    ## Marketplace adapter trait

    `marketplace.rs` defines the `MarketplaceAdapter` trait that isolates
    install / update / availability logic per marketplace:

    ```rust
    pub trait MarketplaceAdapter {
        fn id(&self) -> &str;
        fn current_version(&self, plugin: &str) -> anyhow::Result<Option<String>>;
        fn install(&mut self, plugin: &str) -> anyhow::Result<()>;
        fn update(&mut self, plugin: &str) -> anyhow::Result<()>;
        fn list_installed(&mut self) -> anyhow::Result<Vec<InstalledPlugin>>;
        fn available(&self, plugin: &str) -> anyhow::Result<bool>;
    }
    ```

    v0.10 ships two production adapters and a feature-gated test mock:

    - **`ClaudeMarketplaceAdapter`** — Shells out to `claude plugin install`,
      `claude plugin update`, `claude plugin list --json`. Caches the parsed
      `list` output in a `RefCell<Option<Vec<InstalledPlugin>>>`; the cache
      auto-invalidates on `Ok` install / update calls. Missing `claude` on
      PATH surfaces as a clear actionable error message naming the binary.
      Upstream constraint: `claude plugin install` doesn't accept `--version`,
      so the adapter installs "latest" only; lockfile records the actual
      installed version and surfaces drift on subsequent syncs.
    - **`GitAdapter`** — Thin shim over `crates/tome/src/git.rs`; behavior for
      existing git directories is byte-for-byte unchanged from v0.9.
    - **`MockMarketplaceAdapter`** — Lives in `marketplace::testing` behind the
      `test-support` feature. Used by integration tests to inject deterministic
      install/update/availability behavior without invoking real subprocesses.

    Failure aggregation: `InstallFailure` / `InstallOp` / `InstallFailureKind`
    + a `Kind::ALL` exhaustive sentinel mirror the `remove.rs::FailureKind`
    pattern. Adding a new failure kind without updating `ALL` is a compile
    error.
    ```

    **Step 5: Insert section 4 — `## Unowned lifecycle`.** Content (~25-35 lines):
    ```
    ## Unowned lifecycle

    A skill's manifest entry has `source_name: Option<DirectoryName>` (LIB-03).
    `Some(<dir>)` = Owned (the directory in `tome.toml` provides the source);
    `None` = Unowned (the library copy is canonical with no upstream source).

    **Transitions to Unowned:**
    - **Cleanup orphan** — when a `[directories.*]` entry is removed from
      `tome.toml` (manually or via `tome remove dir <name>`), every manifest
      entry whose `source_name` pointed at the removed directory transitions
      to `source_name: None` on the next `tome sync`. Library content
      preserved (LIB-04).
    - **`tome remove dir <name>`** — explicitly transitions all manifest
      entries owned by `<name>` to Unowned and preserves library content
      (Phase 11 D-10).
    - **Fork-in-place** (managed → local during reconcile drift) — `source_name`
      stays the same value (it's a local directory now), but `previous_source`
      records the original managed `source_name` so the user can re-anchor.

    **Whenever the manifest transitions an entry to Unowned, `previous_source`
    captures the old `source_name` value (Phase 14 D-C1). `tome status` and
    `tome doctor` use this value to render the LAST-KNOWN SOURCE column.

    **CLI verbs** (Phase 14 D-API-1 / D-API-2 vocab merge):
    - **Re-anchor** — `tome reassign <skill> --to <dir>`. Accepts Unowned
      input. The originally-proposed `tome adopt` was folded into existing
      `tome reassign` machinery; the work is identical (copy content into
      `<dir>`, update manifest `source_name`).
    - **Delete** — `tome remove skill <name>`. Confirmation prompt defaults to
      no; `--yes`/`-y` skips. Cleans manifest entry + library directory +
      distribution symlinks + lockfile entry + `machine.toml` memberships
      (D-B1). Refuses on Owned skills with a hint to run `tome remove dir`
      first (D-B2).

    **Surfacing** — `tome status` and `tome doctor` show an `Unowned skills (N):`
    section with NAME / LAST-KNOWN SOURCE / SYNCED columns. JSON output
    includes the new `unowned` (status) / `unowned_skills` (doctor) field.
    Per Phase 14 D-D3, the unowned set is informational and does NOT
    contribute to `tome doctor`'s `total_issues()`; exit code is unaffected.
    ```

    **Step 6: Verify section presence and ordering.** After Step 5, the file should look like:
    ```
    # Architecture
    ...
    ## `crates/tome` — CLI (`tome`)
    ...
    ### Sync Pipeline
    ...
    ### Other Modules
    ...
    ## Key Patterns
    ...
    ## Library-canonical model       <- NEW
    ## Lockfile-authoritative reconciliation   <- NEW
    ## Marketplace adapter trait     <- NEW
    ## Unowned lifecycle             <- NEW
    ## Testing
    ## CI
    ```

    **Step 7: Verify the file builds via mdbook.** Run `cd docs && mdbook build` if mdbook is on PATH (it's used in the project's `make book` target — verify via `rg mdbook /Users/martin/dev/opensource/tome/Makefile`). If the build succeeds without warnings about broken links or invalid markdown, the doc is ready.
  </action>
  <verify>
    <automated>rg -c "^## " /Users/martin/dev/opensource/tome/docs/src/architecture.md &amp;&amp; wc -l /Users/martin/dev/opensource/tome/docs/src/architecture.md</automated>
  </verify>
  <acceptance_criteria>
    - `rg -n '^## Library-canonical model$' docs/src/architecture.md` outputs one match
    - `rg -n '^## Lockfile-authoritative reconciliation$' docs/src/architecture.md` outputs one match
    - `rg -n '^## Marketplace adapter trait$' docs/src/architecture.md` outputs one match
    - `rg -n '^## Unowned lifecycle$' docs/src/architecture.md` outputs one match
    - `rg -n 'content.hash' docs/src/architecture.md` outputs at least one match (Phase 11 D-08 callout present)
    - `rg -n 'tome migrate-library' docs/src/architecture.md` outputs at least one match (Phase 11 D-01 vocab present)
    - `rg -n 'auto_install_plugins' docs/src/architecture.md` outputs at least one match (RECON-02 surfaced)
    - `rg -n 'previous_source' docs/src/architecture.md` outputs at least one match (Phase 14 D-C1 surfaced)
    - `wc -l docs/src/architecture.md` reports a line count between 200 and 300 inclusive (CONTEXT.md target ~225-275)
    - `rg -n 'tome adopt' docs/src/architecture.md` outputs zero matches OR matches only inside a "superseded by" footnote (D-API-1/-2 vocab merge honored)
    - `rg -n 'tome forget' docs/src/architecture.md` outputs zero matches OR matches only inside a "superseded by" footnote
    - `rg -n 'no longer configured' docs/src/architecture.md` outputs zero matches (forbidden v0.9 trigger phrase)
  </acceptance_criteria>
  <done>
    Four new sections inserted between Key Patterns and Testing, each ~25-50 lines, covering library-canonical model + reconciliation + adapter trait + unowned lifecycle. The doc is now ~225-275 lines total and describes the v0.10 architecture in user-facing prose without requiring readers to chase decision logs.
  </done>
</task>

</tasks>

<verification>
- `rg -c '^## ' docs/src/architecture.md` shows ≥7 H2 sections (Sync Pipeline, Other Modules, Key Patterns, Library-canonical model, Lockfile-authoritative reconciliation, Marketplace adapter trait, Unowned lifecycle, Testing, CI)
- File length is between 200 and 300 lines
- All forbidden v0.9 phrases removed: "consolidated cache", "symlinked from library → source", "tome adopt" (as a command), "tome forget" (as a command), "no longer configured"
- All locked v0.10 vocabulary present: `tome migrate-library`, `tome reassign --to`, `tome remove skill`, `tome remove dir`, `auto_install_plugins`, `previous_source`, `content_hash`
- mdbook (if on PATH via `make book`) builds without errors
</verification>

<success_criteria>
- DOC-01 satisfied: `architecture.md` describes the v0.10 model accurately and completely
- D-DOC01-1 four-section addition complete (Library-canonical model, Lockfile-authoritative reconciliation, Marketplace adapter trait, Unowned lifecycle)
- D-API-1 / D-API-2 vocabulary merge honored (no `tome adopt` or `tome forget` as live commands)
- Phase 11 D-01 supersession honored (migration is `tome migrate-library`, not auto-on-first-sync)
- Phase 11 D-08 drift basis honored (content_hash, not version)
- File builds via mdbook without warnings
</success_criteria>

<output>
After completion, create `.planning/phases/16-cleanup-message-ux-docs/16-03-SUMMARY.md` documenting:
- Final line count of architecture.md
- Whether the Excalidraw diagram link was kept as-is or flagged for refresh in `16-deferred-items.md`
- Any v0.9 phrases that proved stickier than expected and how they were rephrased
- Any module entries that were touched beyond the planned set (e.g. browse module if Phase 15 HARD-21 needed surface area)
</output>
