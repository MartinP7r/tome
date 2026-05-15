# Architecture

> **[System Diagram (Excalidraw)](https://excalidraw.com/#json=5-pjpDsna4Way3lfGW5km,p0bQwpcJEl6do68RrnKAgw)** — interactive diagram showing the discovery → library → distribution flow. The diagram pre-dates v0.10 and does not depict the marketplace adapter dispatcher or unowned lifecycle; the broad three-tier shape is still accurate. Refresh deferred to a follow-up.

Rust workspace (edition 2024) with a single crate producing one binary.

## `crates/tome` — CLI (`tome`)

The main binary. All domain logic lives here as a library (`lib.rs` re-exports all modules) with a thin `main.rs` that parses CLI args and calls `tome::run()`.

### Sync Pipeline

The core flow that `tome sync` and `tome init` both invoke (`lib.rs::sync`):

1. **Reconcile** (`reconcile.rs`) — Lockfile-authoritative drift detection for managed skills, run first against the previously-saved manifest and `tome.lock`. Each managed skill is classified as Match / Drift / Vanished; Drift entries optionally apply via the marketplace adapter (subject to `auto_install_plugins` consent and the `--no-install` override). Edit-in-library detection raises a fork/revert/skip prompt when a managed skill's library content_hash diverges from its lockfile entry (RECON-05). See [Lockfile-authoritative reconciliation](#lockfile-authoritative-reconciliation) for details.
2. **Discover** (`discover.rs`) — Walk every directory whose role is `managed`, `synced`, or `source` looking for `*/SKILL.md`. Three directory types: `ClaudePlugins` (reads `installed_plugins.json`), `Directory` (flat walkdir scan), and `Git` (shallow-clones into `~/.tome/repos/<sha256>/` and then scans the clone). First directory wins on name conflicts; the `exclude` list is applied.
3. **Consolidate** (`library.rs`) — Both managed AND local skills are stored as real-directory copies in the library — the library is the canonical home for every skill (LIB-01 / LIB-02). Managed skills (Claude plugins, git clones) get a recursive copy from the source on first sync; subsequent updates flow through the marketplace adapter (`reconcile.rs`) in step 1. Local skills (`directory`/`synced` sources) are copied as before. The `managed: bool` flag now means *update channel* — managed = upstream feeds updates into the library; local = library is canonical and edited in-place. It no longer means "stored as a symlink". A manifest (`.tome-manifest.json`) tracks SHA-256 content hashes for idempotent updates: unchanged skills are skipped, changed skills are re-copied. Sync against a v0.9-shape library (managed = symlink) is refused with a hint pointing at `tome migrate-library`.
4. **Distribute** (`distribute.rs`) — Push library entries (Owned and Unowned alike) to every directory whose role is `synced` or `target` via symlinks. The library is canonical, so distribution is a downhill operation; Unowned skills (manifest `source_name = None`) still get distributed because library content is preserved per LIB-04. Skills disabled in `machine.toml` (globally or per-directory) are skipped, as are directories on the `disabled_directories` list. `distribute` refuses to clobber pre-existing symlinks pointing outside the current library (HARD-09 foreign-symlink protection).
5. **Cleanup** (`cleanup.rs`) — Stale-candidate skills are partitioned into three buckets per UX-01: **removed-from-config** (the source directory was removed from `tome.toml` — manifest entry transitions to Unowned, library content preserved per LIB-04), **missing-from-disk** (the source directory still exists but the file vanished — library copy removed), **now-in-exclude-list** (skill added to `machine.toml::disabled` or per-directory blocklist/allowlist — distribution symlinks removed, library copy preserved). Each bucket renders with a header and per-skill resolution hint to stderr. Broken symlinks in distribution directories are also cleaned, with HARD-09 origin verification before any removal.
6. **Lockfile** (`lockfile.rs`) — Generate `tome.lock` capturing a reproducible snapshot of the library state for diffing on the next sync. Each entry now carries `previous_source` so cross-machine forks-in-place stay traceable to the directory that originally owned the skill.

### Other Modules

Listed roughly in alphabetical order:

- `add.rs` — `tome add` command (plan/render/execute pattern). Registers a directory in `tome.toml`.
- `backup.rs` — Git-backed snapshot/restore/diff for the library. The pre-restore safety snapshot is the only recovery path if a restore was accidental, so `restore` aborts if the snapshot fails (#415).
- `browse/` — TUI browser (`tome browse`): `app.rs` (state + key handling), `ui.rs` (ratatui rendering), `theme.rs` (adaptive dark/light), `fuzzy.rs` (nucleo-matcher), `markdown.rs` (preview rendering). The status bar uses a `StatusMessage { Success | Warning | Pending }` enum (POLISH-02) so glyph + colorization stay consistent. Disable/Enable actions are wired to `MachinePrefs` via the smart-routing scope resolver (HARD-21 D-BROWSE-1: per-directory blocklist > per-directory allowlist > global).
- `cleanup.rs` — Three-bucket cleanup output (UX-01). `cleanup_library` emits Buckets A (removed-from-config — Owned-to-Unowned transition per LIB-04) and B (missing-from-disk — library entry removed); `lib.rs::cleanup_disabled_from_target` emits Bucket C (now-in-exclude-list — distribution symlinks removed, library content preserved). Bucket C entries are collected into a sibling `Vec<ExcludedSkill>` and rendered alongside A+B by `cleanup::render_cleanup_buckets` (called from `lib.rs::sync`) for a single user-facing surface. All output goes to stderr (D-UX01-4). Cleanup no longer auto-deletes orphaned skills (LIB-04); orphan transitions are the unowned-lifecycle entry point.
- `config/` — TOML config at `~/.tome/tome.toml`, split into `mod.rs` (load/save), `types.rs` (`DirectoryName`, `DirectoryType` = `ClaudePlugins`/`Directory`/`Git`, `DirectoryRole` = `Managed`/`Synced`/`Source`/`Target`, `DirectoryConfig`), `overrides.rs` (`apply_machine_overrides` merges `[directory_overrides.<name>]` from `machine.toml` after tilde expansion and before validation, PORT-01..04), and `validate.rs`. `Config::save_checked` round-trips `~/`-shaped paths via `paths::unexpand_tilde` so dotfile-committed configs stay portable (HARD-22).
- `discover.rs` — Skill discovery from all configured directories. `ScanMode::{Local, ManagedNoProvenance, ManagedWith}` replaces the v0.9 `Option<Option<SkillProvenance>>` (HARD-05).
- `distribute.rs` — Distribution to `synced` / `target` directories via Unix symlinks. HARD-09 foreign-symlink detection uses a 2x2 canonicalize-vs-lexical-prefix matrix to handle macOS `/var → /private/var`-style middle symlinks without false positives.
- `doctor.rs` — Diagnoses library issues (orphan directories, missing manifest entries, broken legacy symlinks, missing directory paths) and surfaces the unowned set in a NAME / LAST-KNOWN SOURCE / SYNCED tabled section. Per Phase 14 D-D3, the unowned set is informational and does NOT contribute to `total_issues()`. Annotates `(override)` for paths sourced from `machine.toml` (PORT-05). v0.11 adds issue categorization (`IssueCategory` = Library / Directory / Config / Foreign-symlink, OBS-06) with per-category counts in the text summary and `summary.by_category` + `summary.auto_fixable_by_category` maps in JSON output. Auto-repair dispatch uses typed `RepairKind` enum discrimination (POLISH-04 sentinel pattern); adding a new repair without a handler fails to compile. The pre-v0.11 "N auto-fixable issues / (no auto-repair available)" contradiction is closed (FIX-01 / #530).
- `eject.rs` — Remove all of tome's distribution symlinks (reversible via `tome sync`).
- `git.rs` — Git clone / pull for `type = "git"` directories. Shallow clones to `~/.tome/repos/<sha256>/`, with `branch`/`tag`/`rev` ref pinning and SHA captured in the lockfile.
- `install.rs` — Shell completion installation. (The v0.9 reconcile-managed-plugins logic that used to live here moved to `reconcile.rs` in Phase 13.)
- `library.rs` — `consolidate()` — copies both managed and local skills as real directories into the library (LIB-01 / LIB-02). `consolidate_managed` performs a recursive `walkdir::WalkDir::follow_links(true)` copy on first sync and on every reconcile-driven update; `consolidate_local` mirrors the same content_hash-flag-flip path. Refuses to operate on v0.9-shape (managed = symlink) entries.
- `lint.rs` — Validates SKILL.md frontmatter; downcastable `LintFailed` error mapped to exit code 1 by `main.rs` (HARD-04).
- `lockfile.rs` — Generates and loads `tome.lock` files. Each `LockEntry` carries `name`, `content_hash`, `source_name: Option<DirectoryName>` (None = Unowned), `previous_source: Option<DirectoryName>` (Phase 14 D-C1 cross-machine breadcrumb), `version`, `registry_id`, and `git_commit_sha`. Top-level fields are `pub(crate)` with read-accessors (HARD-06). The lockfile is now authoritative for managed-skill drift detection (RECON-01..05) — `reconcile.rs` reads it on every sync. Atomic temp+rename writes.
- `machine.rs` — Per-machine preferences (`~/.config/tome/machine.toml`). Tracks `disabled` skill set, `disabled_directories` set, per-directory `disabled`/`enabled` skill filtering (`DirectoryPrefs`, MACH-04), `[directory_overrides.<name>]` path remapping (PORT-01), and `auto_install_plugins: AutoInstall { Always, Ask, Never }` (RECON-02). Hostile-input rejection in `apply_machine_overrides` covers `..` traversal, NUL bytes, broken/looping symlinks, and duplicate target paths. Atomic temp+rename writes.
- `manifest.rs` — Library manifest (`.tome-manifest.json`). Each `SkillEntry` records `source_name: Option<DirectoryName>` (None = Unowned per LIB-03) and `previous_source: Option<DirectoryName>` (the last directory that owned the entry — Phase 14 D-C1, also closes Phase 13 D-13 fork-in-place lossy-trace gap). Twin-constructor pattern: `SkillEntry::new` for owned entries, `SkillEntry::new_unowned` for entries materialised directly into the unowned state. Provides `hash_directory()` for deterministic SHA-256 of directory contents. v0.11 adds `last_synced_at: Option<String>` (RFC-3339) at the manifest level, stamped inside the `!dry_run` guard immediately before `manifest::save` so `tome status` can surface the last-sync timestamp (OBS-07 / D-LSYNC-3). Schema lift is additive — pre-v0.11 manifests deserialize cleanly with `last_synced_at: None`. Atomic temp+rename writes.
- `marketplace.rs` — `MarketplaceAdapter` trait (six methods: `id`, `current_version`, `install`, `update`, `list_installed`, `available`) plus `ClaudeMarketplaceAdapter` (subprocess to `claude plugin install/update`, parses `claude plugin list --json`, `RefCell` cache that auto-invalidates on `Ok` install/update) and `GitAdapter` (thin shim over `git.rs`). Failure aggregation via `InstallFailure` / `InstallOp` / `InstallFailureKind` mirrors the `RemoveFailure` pattern; `InstallFailureKind::ALL` plus a const-fn drift guard pin compile-time exhaustiveness (POLISH-04). Test mock `MockMarketplaceAdapter` lives in `marketplace::testing` behind the `test-support` feature.
- `migration_v010.rs` — One-shot `tome migrate-library` command for converting v0.9-shape libraries (managed = symlink) to v0.10 shape (managed = real-directory copy). Idempotent; broken symlinks preserved per Phase 11 D-04. Confirm-or-abort gate via `dialoguer::Confirm::default(false)` with `--yes`/`-y` bypass (UX-02 / Phase 14 D-B3); `--no-input` without `--yes` bails with a Conflict/Why/Suggestion message. Migration plan summary uses `tabled::Style::rounded()` with NAME / SOURCE / SIZE / STATUS columns; per-skill disk size is computed via `walkdir` + `metadata().len()` (`follow_links(false)`). Slated for removal in v0.11+ once all known users have migrated.
- `paths.rs` — `TomePaths` struct bundling `tome_home`/`library_dir`/`config_dir` to prevent parameter swaps. `expand_tilde` / `unexpand_tilde` round-trip pair (HARD-22). Symlink path utilities: resolves relative symlink targets to absolute paths and checks whether a symlink points to a given destination. `collapse_home` for display.
- `reassign.rs` — `tome reassign <skill> --to <dir>` command. Plan/render/execute. Phase 14 D-API-1: accepts Unowned input (re-anchors `source_name: None` to a configured directory). The `--force` flag bypasses D-A1 different-content collision detection; D-A2 refuses target-only directory roles. Re-anchor clears `previous_source` (Phase 14 D-C1). HARD-19 plan/execute filesystem snapshot eliminates drift between phases. The originally-proposed `tome adopt` verb was folded into this command (vocabulary supersession; see [Unowned lifecycle](#unowned-lifecycle)).
- `reconcile.rs` — `tome sync` reconciliation core. Classifies each managed skill in the lockfile as Match / Drift / Vanished (`ReconcileClass`); resolves `auto_install_plugins` consent; renders per-skill diff before applying installs/updates; verifies post-install content_hash; surfaces edit-in-library detection with the fork/revert/skip 3-way prompt (RECON-01..05). Drift detection is content_hash-based, not version-based (Phase 11 D-08).
- `relocate.rs` — Move the skill library to a new path with full safety guarantees: detects cross-filesystem moves with a Phase 7 D-10 Conflict/Why/Suggestion recovery hint (HARD-18), re-anchors all distribution symlinks, calls `warn_if_unreadable_symlink` (intent-first naming per HARD-16) on unreadable managed-skill symlinks instead of silently dropping provenance.
- `remove.rs` — `tome remove dir <name>` and `tome remove skill <name>` (Phase 14 D-API-2 subcommand split). `tome remove dir` transitions every owned manifest entry to Unowned and preserves library content (Phase 11 D-10). `tome remove skill` deletes an Unowned skill entry, its library directory, downstream distribution symlinks, lockfile entry, and `machine.toml` memberships (D-B1); refuses Owned skills with a hint to use `tome remove dir` first (D-B2). Confirms via interactive prompt unless `--yes`/`-y` (D-B3). Failure aggregation via `Vec<RemoveFailure>` + `FailureKind::ALL` POLISH-04 sentinel. The originally-proposed `tome forget` verb was folded into the `skill` subcommand (vocabulary supersession; see [Unowned lifecycle](#unowned-lifecycle)).
- `status.rs` — Read-only summary of library, directories (with type/role + override annotations), and health. Renders an Unowned skills section (NAME / LAST-KNOWN SOURCE / SYNCED) when any entries have `source_name = None` (UNOWN-03). v0.11 adds a top-line `Last sync: <RFC-3339>` (or `never`) line and a `SKILLS` column on the Directories table (OBS-07); JSON shape gains top-level `last_sync: Option<String>` and per-directory `skill_count`. Single-pass directory scan for efficiency.
- `summary.rs` — `SkillSummary` shared type (NAME / LAST-KNOWN SOURCE / SYNCED columns) consumed by `status.rs` and `doctor.rs` Unowned sections. JSON-stable (`previous_source` serialises explicit `null` rather than being skipped).
- `update.rs` — Lockfile diffing and interactive triage logic, invoked by `tome sync` to surface added/changed/removed skills and offer to disable unwanted new skills. (Per-managed-skill version reconciliation moved to `reconcile.rs` in Phase 13; this module retains only the pre-cleanup user-presented diff.)
- `wizard.rs` — Interactive `tome init` setup using `dialoguer` (MultiSelect, Input, Confirm, Select). Uses the merged `KNOWN_DIRECTORIES` registry (WIZ-01, hardened in v0.7) to auto-discover common tool locations (`~/.claude/plugins/cache`, `~/.claude/skills`, `~/.codex/skills`, `~/.gemini/antigravity/skills`, etc.). Detects pre-v0.6 legacy configs and offers cleanup (WUX-03). All diagnostic chrome routes to stderr (HARD-15).

## Key Patterns

- **Library-canonical model**: Discovery directories →(consolidate)→ Library →(distribute)→ Distribution directories. The library is the source of truth and every skill (managed or local) lives there as a real directory copy. Managed skills use a marketplace adapter (`reconcile.rs` + `marketplace.rs`) to pull updates from upstream; local skills are edited in-place in the library. Distribution always uses Unix symlinks (`std::os::unix::fs::symlink`) pointing into the library. Unix-only. See [Library-canonical model](#library-canonical-model) for the full mechanics.
- **Directories are data-driven**: `config::directories` is a `BTreeMap<DirectoryName, DirectoryConfig>` — any tool can be added as a directory with a role without code changes. The wizard's `KNOWN_DIRECTORIES` registry is used purely for auto-discovery convenience.
- **Roles, not "sources vs targets"**: A directory can be `managed` (read-only source), `source` (discovery only), `target` (distribution only), or `synced` (both — same dir is read AND written, e.g. `~/.claude/skills`). The pipeline asks each directory's role what to do with it; there is no separate "sources" vs "targets" config.
- **`dry_run` threading**: Most operations accept a `dry_run: bool` that skips filesystem writes but still counts what *would* change. Results report the same counts either way.
- **Atomic writes**: `manifest.json`, `tome.lock`, and `machine.toml` are always written via temp file + rename. The temp file is in the same directory as the target so the rename is atomic on POSIX.
- **Plan/render/execute**: `add`, `remove`, `reassign`, `relocate`, `eject` build an explicit plan, render it for the user, and only then execute. Dry-run is free; tests can assert plan structure without touching the filesystem.
- **Newtypes at boundaries**: `SkillName`, `DirectoryName`, `ContentHash`, `TomePaths` validate at construction so downstream code doesn't have to. The shared `validate_identifier` rejects empty names, path separators, `.`, and `..`.
- **Error handling**: `anyhow` for the application; `.with_context()` adds path context to every fs error. Missing sources/paths produce stderr warnings rather than hard errors. Symlink operations always verify the link points into the library before deleting.
- **Per-machine portability**: The portable `tome.toml` describes the abstract topology; `machine.toml` provides path overrides (`[directory_overrides.<name>]`) and machine-local opt-outs. Override application happens at config load, before validation, so all downstream code sees post-override paths.

## Library-canonical model

The library at `~/.tome/library/` is the single source of truth for every
skill on the machine — managed AND local. Both kinds are stored as real
directory copies; neither is a symlink into a marketplace cache.

**What changed in v0.10 (vs. v0.9):** Pre-v0.10, managed skills (Claude
plugins, git clones) lived in the library as symlinks pointing back at the
package manager's cache. Updating a plugin updated the library transparently
via the symlink. The trade-off was that removing a source erased every skill
it provided, and shipping the library across machines was impossible because
the symlink targets weren't portable. v0.10 inverts the relationship:
`consolidate_managed` performs a recursive copy on first sync (and on every
update afterward via the marketplace adapter), so the library on disk is
fully self-contained.

**Why this matters:**

- **Cross-machine portability** — `~/.tome/` can be committed to dotfiles
  and cloned onto a fresh machine. Pair with `tome.lock` for exact-version
  reproducibility (see [Lockfile-authoritative reconciliation](#lockfile-authoritative-reconciliation) below).
  See [Cross-machine sync](cross-machine-sync.md) for the end-to-end
  walkthrough (Machine A source-of-truth → Machine B fresh-machine
  bootstrap).
- **Source removal preserves content** — removing a `[directories.*]` entry
  from `tome.toml` (or running `tome remove dir <name>`) no longer erases
  the skills it provided. Their manifest entries transition to Unowned
  (`source_name: None`); library content stays in place (LIB-04).
- **Vanished plugins stay usable** — if a marketplace removes a plugin,
  `tome sync` warns but keeps using the preserved library copy.
- **Drift basis is `content_hash`, not version** (Phase 11 D-08). Reconcile
  compares `content_hash(library/<skill>)` against the lockfile entry; the
  version string is display-only in diff output (e.g.
  `plugin X: 5.0.5 → 5.0.7`). Because the upstream `claude plugin install`
  command doesn't accept `--version`, true version pinning is upstream
  future work.

**Migration path:** v0.9-shape libraries (managed = symlink) need to run
`tome migrate-library` once. Detection: `library_dir/<name>.is_symlink() &&
manifest[name].managed == true && manifest.contains_key(name)` (Phase 11
D-03). The command shows a summary table (skill count + per-skill disk
estimate via `walkdir` + `metadata().len()`), prompts for confirmation, then
converts symlinks to real copies. Broken symlinks are preserved in place per
Phase 11 D-04. Idempotent on re-run; `tome sync` refuses to operate on
v0.9-shape libraries until migration runs. The migration is a one-shot CLI
command, NOT auto-on-first-sync (Phase 11 D-01 supersedes the original
UX-02 wording).

## Lockfile-authoritative reconciliation

`tome.lock` is the cross-machine state contract. Cargo.lock-shaped: each
managed skill records `(name, version, content_hash, source_name,
previous_source, registry_id, git_commit_sha)`. On every `tome sync`,
`reconcile.rs::reconcile_lockfile` classifies each managed skill into one
of four buckets:

- **Match** — `content_hash(library/<skill>) == lockfile.content_hash`.
  Nothing to do.
- **Drift** — `content_hash` differs OR the lockfile expects a version the
  adapter no longer provides. Renders a per-skill diff
  (`plugin X: 5.0.5 → 5.0.7`); applies the install/update via the
  marketplace adapter; verifies the resulting library `content_hash`
  matches the lockfile entry.
- **Vanished** — `adapter.available()` returns false (the marketplace no
  longer offers the plugin). Stderr warning, distribution continues from
  the preserved library copy (RECON-04).
- **Edited-in-library** — `managed: true` and the library `content_hash`
  diverges from the lockfile in a way that looks like an in-place user
  edit rather than upstream drift. See "Edit-in-library detection" below.

**Auto-install consent (RECON-02)** — the first sync on a machine with a
non-empty drift set prompts: `Auto-install missing plugins on every sync?
[Y/n/never]`. The choice persists in `machine.toml::auto_install_plugins`
as one of `Always | Ask | Never`. The `--no-install` global flag overrides
the persisted choice for the current invocation (mirrors Cargo's
`--frozen` / `--locked`).

**Edit-in-library detection (RECON-05)** — when a managed skill's library
content_hash diverges from the lockfile, the user is prompted with three
choices: **fork** (default — promote to local via the existing `tome fork`
machinery), **revert** (overwrite from marketplace), **skip** (warn and
don't touch this entry this sync). In `--no-input` mode the default is
**skip with warning** so edited content is never silently overwritten.

**`previous_source` breadcrumb** — when a managed skill forks in-place
(Drift → fork), the manifest entry records the old `source_name` in
`previous_source` before flipping to local. This closes the Phase 13 D-13
"lossy fork-in-place" gap; `tome status` and `tome doctor` show the
last-known source so the user can re-anchor cleanly later via
`tome reassign <skill> --to <dir>`.

**Partial-failure surfacing (ADP-04)** — adapter `install`/`update` errors
aggregate into `Vec<InstallFailure>` and render as a grouped `⚠ N install
operations failed` summary (matches the v0.8 SAFE-01 `RemoveFailure`
pattern). Library distribution still completes for skills whose adapter
calls succeeded; sync exits non-zero on partial failure.

## Marketplace adapter trait

`marketplace.rs` defines the `MarketplaceAdapter` trait that isolates
install / update / availability logic per marketplace:

```rust
pub trait MarketplaceAdapter {
    fn id(&self) -> &str;
    fn current_version(&self, plugin_id: &str) -> Result<Option<String>>;
    fn install(&self, plugin_id: &str) -> Result<()>;
    fn update(&self, plugin_id: &str) -> Result<()>;
    fn list_installed(&self) -> Result<Vec<InstalledPlugin>>;
    fn available(&self, plugin_id: &str) -> Result<bool>;
}
```

v0.10 ships two production adapters and a feature-gated test mock:

- **`ClaudeMarketplaceAdapter`** (ADP-02) — Shells out to
  `claude plugin install`, `claude plugin update`, `claude plugin list
  --json`. Caches the parsed `list` output in
  `RefCell<Option<Vec<InstalledPlugin>>>`; the cache auto-invalidates on
  `Ok` install/update calls. Missing `claude` on PATH surfaces as a clear
  actionable error message naming the binary. Upstream constraint:
  `claude plugin install` doesn't accept `--version`, so the adapter
  installs "latest" only; the lockfile records the actual installed
  version and surfaces drift on subsequent syncs.
- **`GitAdapter`** (ADP-03) — Thin shim over `crates/tome/src/git.rs`;
  behavior for existing git directories is byte-for-byte unchanged from
  v0.9.
- **`MockMarketplaceAdapter`** — Lives in `marketplace::testing` behind
  the `test-support` feature. Used by integration tests to inject
  deterministic install/update/availability behavior without invoking real
  subprocesses.

Failure aggregation: `InstallFailure` + `InstallOp` + `InstallFailureKind`
+ a `Kind::ALL` exhaustive sentinel mirror the `remove.rs::FailureKind`
pattern (POLISH-04). Adding a new failure kind without updating `ALL` is a
compile error.

## Unowned lifecycle

A skill's manifest entry has `source_name: Option<DirectoryName>` (LIB-03).
`Some(<dir>)` = Owned (the directory in `tome.toml` provides the source);
`None` = Unowned (the library copy is canonical with no upstream source).

**Transitions to Unowned:**

- **Cleanup orphan** — when a `[directories.*]` entry is removed from
  `tome.toml` (manually edited or via `tome remove dir <name>`), every
  manifest entry whose `source_name` pointed at the removed directory
  transitions to `source_name: None` on the next `tome sync`. Library
  content preserved (LIB-04). This is Bucket A in the cleanup output —
  see [Sync Pipeline](#sync-pipeline) step 5.
- **`tome remove dir <name>`** — explicitly transitions all manifest
  entries owned by `<name>` to Unowned and preserves library content
  (Phase 11 D-10).
- **Fork-in-place** (managed → local during reconcile drift) — `source_name`
  stays the same value (it points at the now-local directory), but
  `previous_source` records the original managed `source_name` so the user
  can re-anchor.

Whenever the manifest transitions an entry to Unowned, `previous_source`
captures the old `source_name` value (Phase 14 D-C1). `tome status` and
`tome doctor` use this value to render the LAST-KNOWN SOURCE column.

**CLI verbs** (Phase 14 D-API-1 / D-API-2 vocabulary merge):

- **Re-anchor** — `tome reassign <skill> --to <dir>`. Accepts Unowned
  input. The originally-proposed `tome adopt` was folded into existing
  `tome reassign` machinery; the work is identical (copy content into
  `<dir>`, update manifest `source_name`).
- **Delete** — `tome remove skill <name>`. Confirmation prompt defaults
  to no; `--yes`/`-y` skips. Cleans manifest entry + library directory +
  distribution symlinks + lockfile entry + `machine.toml` memberships
  (D-B1). Refuses on Owned skills with a hint to run `tome remove dir`
  first (D-B2).

**Surfacing** — `tome status` and `tome doctor` show an
`Unowned skills (N):` section with NAME / LAST-KNOWN SOURCE / SYNCED
columns. JSON output includes the new `unowned` (status) /
`unowned_skills` (doctor) field. Per Phase 14 D-D3, the unowned set is
informational and does NOT contribute to `tome doctor`'s `total_issues()`;
exit code is unaffected.

## Observability (v0.11)

Sync/reconcile/consolidate/distribute/cleanup chatter routes through
`tracing::{info,warn,debug}!` (OBS-01). Wizard prompts (`dialoguer`), TUI
browse output, and user-facing summary tables (`tome status`/`list`/`doctor`
tables, `tome sync` final summary) stay on direct stdout — `tracing` is for
log-like output only.

The `LogLevel` enum (HARD-07) maps to a `tracing_subscriber::EnvFilter`
(OBS-02):

- Default level: `info`.
- `--verbose` raises to `debug`.
- `--quiet` lowers to `warn`.
- `TOME_LOG=tome::sync=debug,tome::reconcile=info` (or any `EnvFilter`
  string) overrides the flag-derived level (D-ENV-1).

`tome sync --verbose` emits one span per pipeline step (`discover`,
`reconcile`, `consolidate`, `distribute`, `cleanup`) with an `elapsed_ms`
field on span close (OBS-03). Spans nest under a top-level `sync` span so
a single run produces a hierarchical trace.

When `consolidate` / `distribute` re-emits a skill, the `cause` field on
the `info!` event names *why* — one of `hash changed`, `previously
failed`, `newly added`, or `directory now allowed` (OBS-04). The final
sync summary block includes a reconcile classification line
(`reconcile: N match · M drift · K vanished · L missing-from-machine`,
OBS-05) above the cleanup buckets.

`PreviouslyFailed` and `DirectoryNowAllowed` causes are documented but
deferred to a future schema bump — the substrate is in place, the emit
sites need a per-skill failure-history field on `SkillEntry`.

## Testing

Unit tests are co-located with each module (`#[cfg(test)] mod tests`). Integration tests live in `crates/tome/tests/` and exercise the binary via `assert_cmd` — post-HARD-13 (v0.10) the original `cli.rs` was split into per-domain files (`cli_sync.rs`, `cli_doctor.rs`, `cli_status.rs`, `cli_init.rs`, `cli_make_release.rs`, etc.) with shared helpers under `tests/common/`. Snapshot tests use `insta` (filtered for tmpdir paths). Tests use `tempfile::TempDir` and `assert_fs::TempDir` for filesystem isolation — no cleanup needed.

## CI

GitHub Actions runs on both `ubuntu-latest` and `macos-latest`: fmt check, clippy with `-D warnings`, tests, and release build.
