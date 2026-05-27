# Requirements: tome v1.0 — tome Desktop (Tauri GUI)

**Defined:** 2026-04-28 (drafted as `milestones/v1.0-REQUIREMENTS.md`)
**Ratified:** 2026-05-23 (promoted to active `REQUIREMENTS.md` via `/gsd-new-milestone` after v0.16 shipped)
**Status:** Active milestone — Phases 25–31
**Scope anchor:** Epic to be filed (`tome Desktop — Tauri 2 GUI`)
**Core Value:** Every AI coding tool on a developer's machine shares the same skill library without manual copying or per-tool configuration. v1.0 makes that library *visible* — directories, skills, sync state, and conflicts are observed and managed from a desktop app rather than a terminal.

## Milestone context

v0.6–v0.16 hardened the CLI in 11 minor releases: unified directory model (v0.6), wizard UX hardening (v0.7), cross-platform safety (v0.8), cross-machine config portability (v0.9), library-canonical model + marketplace adapter (v0.10), observability foundation (v0.11), pre-v1.0 review polish (v0.12), `tome add` UX (v0.13), type+role UX + claim-orphan (v0.14), generic managed source directory (v0.15), doctor diagnostics expansion (v0.16). v1.0 is the inflection point where tome stops being CLI-only.

**Why now:**

- Mental model is mature. Directories/library/distribution/lockfile have been stable since v0.6 — the GUI can model them rather than chase them.
- The CLI surface is already structured around plan/render/execute (remove, reassign, fork). Adding a GUI render layer is incremental.
- Sync triage and the wizard are the two remaining "TUI-shaped" surfaces. Both are better as proper GUI flows.

**Why Tauri 2 (not Electron):**

The Rust core already exists. Electron + napi-rs would impose an N-API shim, ABI rebuilds, native-module signing, and ~150 MB Node runtime — all in service of letting JS call Rust. Tauri 2 calls Rust directly via `#[tauri::command]`, ships the OS webview (~3–10 MB bundle), has built-in code-signed auto-update, and reuses the same Developer ID + notarization flow we already need for the CLI. Decision recorded as **D-GUI-01** below.

## v1 Requirements

Requirements for the v1.0 release. Grouped by category; each maps to a roadmap phase.

### Core architecture (CORE)

The Rust crate must be reshaped to support both the existing CLI and the new GUI without duplicating domain logic. This is the foundation; every other GUI requirement depends on it.

- [x] **CORE-01**: Domain operations (`sync`, `status::gather`, `list::collect`, `lint::lint_library`/`lint_skill`, `doctor::diagnose`, `remove::plan`, `reassign::plan` (covers fork via `is_fork: true` — no separate `fork.rs`), `relocate::plan`, `eject::plan`, `backup::*`) return structured Rust types — not formatted strings — and are callable from any front-end. Existing `lib.rs::run` is decomposed into a thin CLI wrapper that calls these and formats output; the GUI calls them directly.
- [x] **CORE-02**: A new crate `crates/tome-desktop` is added to the workspace alongside `crates/tome` and depends on it as a path dependency. The Tauri app lives in this crate. The CLI continues to ship from `crates/tome` unchanged.
- [x] **CORE-03**: All structured types crossing the Rust↔JS boundary (`StatusReport`, `SkillSummary`, `LockfileDiff`, `RemovePlan`, `Config`, `MachinePrefs`, etc.) generate matching TypeScript types via `specta` + `tauri-specta`, exposed to the front-end as a generated `bindings.ts`. No hand-rolled type definitions on the JS side.
- [x] **CORE-04**: Long-running operations (`sync`, git clone in `add`, backup snapshot/restore) emit progress events via Tauri's event system; the front-end subscribes and renders progress without blocking the IPC reply.
- [x] **CORE-05**: All Rust errors crossing into the front-end carry a stable `code` (enum) and `message` (`anyhow` chain) — the GUI can render targeted error UI ("permissions" / "not found" / "validation") without string-matching messages.

### Read-only views (VIEW)

Replacement for `tome status` + `tome list` + `tome browse`. First user-visible value; ships in the alpha cut.

- [ ] **VIEW-01**: Status dashboard window shows: resolved `tome_home`, library directory, configured directories (with role/type badges), skill count, last sync time, lockfile state, and machine pref summary — equivalent to `tome status --json` rendered as a UI.
- [ ] **VIEW-02**: Skill list view with virtualised rendering (handles ≥2000 skills at 60 fps), fuzzy search (matches `nucleo`-style ranking from the CLI), sort modes (name / source / recent), and group-by (none / source / role).
- [ ] **VIEW-03**: Skill detail pane shows frontmatter (parsed via existing `lint.rs` logic), source directory path, content hash, last sync timestamp, managed/local status, and disabled state — clicking actions (open source dir / copy path / disable on this machine) match the existing browse TUI.
- [ ] **VIEW-04**: Markdown preview pane renders SKILL.md body (post-frontmatter) with the same Markdown subset the existing `browse/markdown.rs` supports.
- [ ] **VIEW-05**: Health pane surfaces `tome doctor` findings (orphan dirs, broken symlinks, missing manifest entries, missing source paths) with one-click "fix" actions that call into the same repair handlers used by the CLI's interactive `tome doctor`.
- [ ] **VIEW-06**: All read-only views auto-refresh when the file watcher (CORE-04 event channel) detects manifest, lockfile, or library changes — the GUI cannot drift from on-disk state.

### Sync + triage (SYNC)

Replacement for `tome sync` and its interactive triage flow. Highest-UX-risk surface; warrants its own phase.

- [ ] **SYNC-01**: A "Sync" action runs the same pipeline as `tome sync` (discover → consolidate → distribute → cleanup → save) and renders progress per stage; the user sees what stage is running and which directory is currently being processed.
- [ ] **SYNC-02**: When the lockfile diff produces new/changed/removed skills, a triage panel lists them with diff metadata (source, hash, timestamp) and per-skill actions: keep (default), disable on this machine, or for git-sourced skills view-source. Bulk actions (disable all new from `<directory>`) supported.
- [ ] **SYNC-03**: Triage decisions are previewable: the user sees the resulting `machine.toml` diff before applying. No silent writes.
- [ ] **SYNC-04**: Sync runs are cancellable. The user can abort discovery or consolidation in progress; the cancel leaves the library in a consistent state (no half-written manifest, no partial lockfile).
- [ ] **SYNC-05**: Failed sync surfaces a per-stage failure summary (matching CLI's `⚠ K operations failed` semantics shipped in SAFE-01) with a retry action that resumes from the failed stage where possible.

### Configuration UI (CFG)

Replacement for `tome init` + `tome add` + hand-edited `tome.toml` / `machine.toml`.

- [ ] **CFG-01**: First-run experience launches when no `tome.toml` exists at the resolved `tome_home`. Wizard flow mirrors `tome init`: pick `tome_home`, auto-discovered directories presented with checkboxes, custom directories addable inline. Greenfield/brownfield/legacy logic from WUX-01..05 applies.
- [ ] **CFG-02**: Directory editor lets the user add, edit, remove, and reorder directories. Type/role combos validated live against `valid_roles()` (the same 12-combo matrix WHARD-06 covers). Path validation runs `Config::validate()` before save and surfaces overlap errors inline.
- [ ] **CFG-03**: `tome add <git url>` is replaced by an "Add git repository" form that captures URL, optional name, and ref pinning (branch/tag/rev — same `clap` choices). Clone progress streams via the SYNC-01 event channel.
- [ ] **CFG-04**: Machine preferences editor exposes per-machine `disabled` skills, `disabled_directories`, and per-directory `enabled`/`disabled` lists. Changes are previewed as a `machine.toml` diff before save.
- [ ] **CFG-05**: All config writes go through `Config::save_checked` (expand → validate → TOML round-trip → write) — the GUI cannot produce a config the CLI would reject.

### Mutating operations (OPS)

Replacement for `tome remove`, `tome reassign`, `tome fork`, `tome relocate`, `tome eject`.

- [ ] **OPS-01**: Removing a directory shows a `RemovePlan` preview (which symlinks, library dirs, manifest entries, lockfile entries are affected) and requires explicit confirmation before execute. Partial-failure aggregation (SAFE-01 semantics) renders failures in the result UI with retry-per-item.
- [ ] **OPS-02**: Reassigning or forking a skill shows the same plan/render/execute flow used by the CLI commands. The destination directory is picked from a dropdown of compatible directories.
- [ ] **OPS-03**: Relocating the library shows a preview (target path, count of symlinks to rewrite) and runs atomically; failure leaves the library at the original path (matches existing `relocate.rs` semantics).
- [ ] **OPS-04**: Ejecting (removing all symlinks from distribution directories) shows the count and a confirmation; the action is reversible by re-running sync.

### Backup UI (BAK)

Replacement for `tome backup init/snapshot/list/restore/diff`.

- [ ] **BAK-01**: Backup history view shows a table of git commits in the library: timestamp, message, commit SHA. Equivalent to `tome backup list` rendered as a list view.
- [ ] **BAK-02**: Snapshot action prompts for a message (default: timestamp) and runs `git add . && git commit`; success/failure surfaces as a toast.
- [ ] **BAK-03**: Diff view (against a selected commit, default `HEAD`) shows changed skills with per-file diff rendering — replaces `tome backup diff`.
- [ ] **BAK-04**: Restore flow shows a preview of what will change (skills affected, lockfile state) and requires explicit confirmation. After restore, sync is re-run automatically to reconcile distribution targets.

### Distribution (DIST)

Code signing, notarization, auto-update, and packaging required for a shippable v1.0.

- [ ] **DIST-01**: macOS bundle is signed with the existing Developer ID, notarized via `notarytool`, stapled, and packaged as a DMG. CI builds for both `aarch64-apple-darwin` and `x86_64-apple-darwin` (universal binary preferred if Tauri/Apple recommend it at the time).
- [ ] **DIST-02**: Auto-update is wired via `tauri-plugin-updater` against a signed update manifest hosted on GitHub Releases; updates are gated on user opt-in (no silent updates).
- [ ] **DIST-03**: GitHub Actions release workflow produces signed/notarized DMGs alongside the existing CLI cargo-dist artifacts. CLI release flow is not regressed.
- [ ] **DIST-04**: First-launch UX surfaces a "this is what tome does" overview, links to the docs, and offers to either run the CFG-01 wizard or import an existing `tome.toml`. Hardened runtime is enabled with the minimum entitlements required (no library validation disable).
- [ ] **DIST-05**: The desktop app embeds (or shells out to) the `tome` CLI binary so users can copy a "show in terminal" command from any view. The CLI continues to be installable independently via Homebrew / cargo-dist.

### Non-functional (NF)

Cross-cutting requirements that apply to multiple phases.

- [ ] **NF-01**: Skill list with 2000 skills renders search-as-you-type at 60 fps on M1 (8 GB) — chosen as the perf budget; verified via a synthetic-skills bench.
- [ ] **NF-02**: All views are keyboard-navigable; primary actions have keyboard shortcuts (matching macOS HIG conventions: ⌘N add, ⌘R sync, ⌘F search, etc.). VoiceOver labels on every interactive element.
- [ ] **NF-03**: Native macOS menu bar with File / Edit / View / Library / Help menus. App responds to system appearance changes (light/dark) — no in-app theme switcher initially.
- [ ] **NF-04**: All destructive operations surface their plan and require explicit confirmation (no "always confirm" toggle that bypasses this in v1.0). Undo via `tome backup restore` is documented in confirmations where relevant.
- [ ] **NF-05**: The desktop app and CLI share a single `tome.lock` and `.tome-manifest.json`. Concurrent CLI usage while the app is open does not corrupt either file (file watcher reloads on external change).

## v2 Requirements

Deferred to post-v1.0.

- **GUI-EDIT-01**: SKILL.md editor with markdown preview and frontmatter form. Save through the same `Config::save_checked`-style validation pipeline.
- **GUI-WATCH-01**: Real-time file watcher with auto-sync when source directories change.
- **GUI-CONFLICT-01**: Visual merge tool for `git pull` conflicts in git-sourced directories.
- **GUI-LINUX-01**: Linux build via `webkit2gtk` (Tauri's Linux backend). Out of v1.0 scope because the v0.8 Linux UAT items are still carry-over.
- **GUI-WIN-01**: Windows build. Blocked on whole-project Windows support (currently out of scope).
- **GUI-STATS-01**: Library analytics — most-used skills, by-tool distribution, last-touched timestamps in aggregate.

## Out of Scope

| Feature | Reason |
|---------|--------|
| Mac App Store distribution | App Sandbox would block tome's symlink-into-`~/.claude`/`~/.codex`/`~/.gemini` model. Direct distribution (Developer ID + notarization) is the only viable path in v1.0. |
| Windows / Linux GUI on day 1 | Project is currently Unix-only; Linux GUI lives behind the v0.8 Linux UAT carry-over. |
| Mobile / iPad app | Different distribution, different sandboxing, no symlinks. |
| Web / remote access | tome is a local-machine tool. No server component planned. |
| Skill content authoring (markdown editor) | Read-only viewing in v1.0 (VIEW-04); editing deferred to GUI-EDIT-01. |
| Replacing the CLI | The CLI ships unchanged. The GUI is additive — both share the same library. |
| Real-time auto-sync on file change | Manual sync only in v1.0 (deferred to GUI-WATCH-01). |
| Plug-in / extension system for the GUI | Not enough variation in user requirements to justify the architecture cost in v1.0. |

## Constraints

- **Single-user product**. No accounts, no sync, no telemetry.
- **macOS-first**. v1.0 ships macOS only; Linux deferred (v2). Windows out of scope.
- **No regression of the CLI**. The CLI ships from `crates/tome` unchanged. cargo-dist release flow continues.
- **Strict Tauri 2.x**. Pin Tauri major version; upgrade only at milestone boundaries.
- **No JS-side business logic**. The frontend renders Tauri-command results and dispatches commands. Validation, planning, and side effects all live in Rust.
- **Hardened runtime + notarization**. Required for direct-distribution UX; rules out features needing `disable-library-validation` or arbitrary code execution.

## Key Decisions

| ID | Decision | Rationale |
|----|----------|-----------|
| **D-GUI-01** | Tauri 2 over Electron + napi-rs | Existing Rust core; ~8 MB vs ~150 MB bundle; no N-API ABI layer; built-in code-signed auto-update; reuses Developer ID flow. |
| **D-GUI-02** | New `crates/tome-desktop` workspace member, not a feature flag in `crates/tome` | Isolates Tauri / webview deps from the CLI; CLI binary stays slim; CI can build either independently. |
| **D-GUI-03** | `specta` + `tauri-specta` for TS type generation | First-class Tauri 2 integration; generates `bindings.ts` at build time; eliminates JS-side type drift. |
| **D-GUI-04** | Frontend framework: **React** (chosen in Phase 25 spike, 25-06) | Built 3-way spike (React/Solid/Svelte) scored 1-5 across four criteria; React + Svelte tied 16, React wins the two compounding criteria (bindings.ts ergonomics + ecosystem fit for NF-01 virtualization / NF-02 a11y / NF-03 HIG). Irreversible from Phase 26. See `.planning/research/v1.0-frontend-framework-decision.md`. |
| **D-GUI-05** | Auto-update via `tauri-plugin-updater` + GitHub Releases manifest | Built into Tauri 2; signed updates; no third-party service dependency. |
| **D-GUI-06** | macOS only for v1.0; Linux behind v0.8 carry-over | Linux runtime UAT items still pending hardware; GUI adds the same surface. Defer to v2. |
| **D-GUI-07** | App and CLI share `tome.lock` + `.tome-manifest.json`; file watcher in app reloads on external change | Single source of truth; no GUI-private state files. |
| **D-GUI-08** | Tauri commands return structured types; CLI continues to format strings | Domain logic returns `StatusReport` etc.; CLI's `lib.rs::run` is decomposed into a presenter layer over the same domain calls. |
| **D-GUI-09** | Sequence v0.9 → v1.0 (default) but allow swap | v0.9 cross-machine portability is small; landing it first means GUI inherits stable `machine.toml` semantics. Swappable if priorities shift. |

## Non-Goals (clarifying)

- **Not** a wrapper around the CLI binary. The GUI talks to the Rust library directly via Tauri commands; the CLI is referenced only for "show in terminal" UX (DIST-05).
- **Not** a config editor that bypasses validation. CFG-05 routes all writes through `Config::save_checked`.
- **Not** a feature-parity guarantee for niche CLI behaviour. `tome completions <shell>` and `tome version` stay CLI-only — they have no GUI surface.

## Traceability

| Requirement | Phase | GitHub Issue | Status |
|-------------|-------|--------------|--------|
| CORE-01..05 | Phase 25 | TBD | Pending |
| VIEW-01..06 | Phase 26 | TBD | Pending |
| SYNC-01..05 | Phase 27 | TBD | Pending |
| CFG-01..05 | Phase 28 | TBD | Pending |
| OPS-01..04 | Phase 29 | TBD | Pending |
| BAK-01..04 | Phase 30 | TBD | Pending |
| DIST-01..05 | Phase 31 | TBD | Pending |
| NF-01..05 | Cross-phase | — | Pending |

**Coverage:**
- v1 requirements: 32 total
- Mapped to phases: 32 / 32 ✓
- Cross-phase non-functional: 5 / 5 (verified at phase boundaries + final pre-ship audit)

## Release cuts (suggested)

These are non-binding suggestions to give a sense of intermediate ship points. Final cut decisions happen at phase-completion checkpoints.

| Cut | Phases | Capability |
|-----|--------|------------|
| **v1.0-alpha** | 25 + 26 | Read-only desktop client. Status, list, browse-replacement, doctor. No mutating actions in the GUI; CLI still required for sync/edit. |
| **v1.0-beta** | + 27 + 28 | Sync + config UI. Daily-driver capable for the primary user. |
| **v1.0-rc** | + 29 + 30 | All mutating commands available in the GUI. CLI is functionally optional for steady-state use. |
| **v1.0** | + 31 | Signed, notarized, auto-updating DMG. Public-shippable. |

---

*Requirements defined: 2026-04-28 in response to user request to scope a Tauri 2 GUI milestone after exploring Rust→TS migration costs and Electron+napi-rs alternatives.*
*Ratified into active planning: 2026-05-23 via `/gsd-new-milestone` after v0.16 shipped. Phase numbering continued from v0.16 (last phase: 24) → v1.0 starts at Phase 25.*
