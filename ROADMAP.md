# Roadmap

| Version    | Theme                  | Key Features                                                            | Status |
| ---------- | ---------------------- | ----------------------------------------------------------------------- | ------ |
| **v0.1.x** | Polish & UX            | Wizard improvements, progress spinners, table output, GitHub Pages docs | ✓ |
| **v0.2**   | Scoped SOT             | Library copies skills (not symlinks), git-friendly library dir          | ✓ |
| **v0.2.1** | Output Layer           | Data struct extraction, warning collection, `--json` for list           | ✓ |
| **v0.3**   | Connector Architecture | `BTreeMap` targets, `KnownTarget` registry, npm skill source research  | ✓ |
| **v0.3.x** | Portable Library (MVP) | Per-machine preferences, `tome update`, lockfile                        | ✓ |
| **v0.4.1** | Browse                 | `tome browse` (ratatui+nucleo): fuzzy search, preview, sort, actions   | ✓ |
| **v0.4.2** | Skill Validation       | `tome lint`, frontmatter parsing, cross-tool compatibility checks       |        |
| **v0.5**   | Managed Sources        | Claude marketplace auto-install, git-backed backup                      |        |
| **v0.6**   | Git Sources            | Remote skill repos, branch/tag/SHA pinning, private repo support        |        |
| **v0.7**   | Skill Composition      | Wolpertinger: merge/synthesize skills from multiple sources via LLM     |        |

---

## v0.1.x — Polish & UX

- [x] **Wizard interaction hints**: Show keybinding hints in MultiSelect prompts (space to toggle, enter to confirm) — embedded in prompt text to work around `dialoguer`'s limitation.
- [x] **Clarify plugin cache source**: Clarified in v0.4.1 (#312).
- [x] **Wizard visual polish**: Color, section dividers, and summary output via `console::style()` — implemented in `wizard.rs`.
- [x] **Modern TUI with welcome ASCII art**: Evaluate `ratatui` vs `console` + `indicatif` before committing to a framework. → Decision: ratatui + nucleo for interactive commands (`tome browse`), plain text for non-interactive commands. See v0.2.1 and v0.4.1.
- [x] **Progress spinners for sync** (`indicatif`): Spinners during discover → consolidate → distribute → cleanup steps, implemented in `lib.rs`.
- [x] **Table-formatted output** (`tabled`): `tabled::Table` used for `tome list` and `tome status` output.
- [x] **Explain symlink model in wizard**: Clarify that the library uses symlinks (originals are never moved or copied), so users understand there's no data loss risk.
- [x] **Optional git init for library**: Wizard asks whether to `git init` the library directory for change tracking — implemented in `wizard.rs`.
- [x] **Fix `installed_plugins.json` v2 parsing**: Current parser expects a flat JSON array (v1); v2 wraps plugins in `{ "version": 2, "plugins": { "name@registry": [...] } }` — discovery silently finds nothing. Support both formats going forward.
- [x] **Finalize tool name**: Decided on **tome** — *"Cook once, serve everywhere."*
- [x] **GitHub Pages deployment**: Add CI workflow to build and deploy mdBook + `cargo doc` to GitHub Pages.

## v0.2 — Scoped SOT

Make the library the source of truth for local skills. `tome sync` copies skill directories into the library instead of creating symlinks back to sources. Distribution to targets still uses symlinks (target → library).

- [x] **Library as canonical home** ([#37](https://github.com/MartinP7r/tome/issues/37)): Local skills live directly in the library (real directories, not symlinks). `tome sync` copies from sources into library, making the library the single source of truth.
- [x] **Git-friendly library directory** ([#42](https://github.com/MartinP7r/tome/issues/42)): Library directory works as a git repo — local skills tracked in git, distribution symlinks are separate.
- [x] **Two-tier symlink model**: Sources → (copy) → Library → (symlink) → Targets. Sources are read-only inputs; the library owns the canonical copies; targets get symlinks into the library.
- [x] **Idempotent copy semantics**: Only copy when source content has changed (compare timestamps or content hashes). Skip unchanged skills to keep syncs fast.

**Not in scope** (deferred to v0.5): lockfile, `tome update`, per-machine preferences, managed source support, git-backed backup.

## v0.2.1 — Output Layer ✓

Decouple output rendering from business logic. Prerequisite for `tome browse` (v0.4.1) and `--json` output (#167), ensuring new connectors in v0.3 get clean data separation from day one.

- [x] **Renderer trait** (`ui/mod.rs`): Abstract output interface for sync reporting, skill listing, status display, doctor diagnostics, warnings, and confirmations — Closed as superseded (#183). Data struct extraction was the real prerequisite; ratatui (v0.4.1) will consume data structs directly rather than going through a trait.
- [x] **Data struct extraction**: `status::gather() -> StatusReport`, `doctor::diagnose() -> DoctorReport`, sync pipeline returns `SyncReport` — pure computation separated from rendering
- [x] **Warning collection**: Replace scattered `eprintln!` in discover/library/distribute with `Vec<Warning>` returned alongside results
- [x] **TerminalRenderer**: Reimplements current output using `console`/`indicatif`/`tabled`/`dialoguer` — identical user-facing behavior, routed through the trait — Superseded along with Renderer trait.
- [x] **QuietRenderer**: Replaces `quiet: bool` parameter threading with a renderer that suppresses non-error output — Closed as superseded (#188). Not needed without the Renderer trait; `quiet` parameter threading is sufficient.
- [x] **`--json` for `tome list`** ([#167](https://github.com/MartinP7r/tome/issues/167)): Trivially enabled once data structs exist — serialize `Vec<SkillRow>` directly

## v0.3 — Connector Architecture ✓

Replaced the hardcoded `Targets` struct with a flexible, data-driven target configuration. Originally scoped as a full connector trait architecture, but the pragmatic first step — config flexibility — shipped as the milestone deliverable.

### Delivered

- [x] **Generic `[[targets]]` array**: Replaced the hardcoded `Targets` struct with `BTreeMap<String, TargetConfig>` ([#175](https://github.com/MartinP7r/tome/pull/175)). Each target has a `name`, `path`, `method` (symlink/mcp), and connector-specific options. Data-driven `KnownTarget` registry in the wizard enables custom target support without code changes.
- [x] **npm-based skill source research** ([#97](https://github.com/MartinP7r/tome/issues/97)): Investigated `npx skills` (Vercel Labs). Confirmed: canonical copies in `.agents/skills/<name>/`, lockfile at `.agents/.skill-lock.json` (v3) with content hashes and provenance. A `Directory` source pointed at `~/.agents/skills/` works for basic discovery; a dedicated source type would preserve provenance metadata from the lockfile.
- [x] **`.agents/skills/` as emerging universal path**: 9 agents converge on `.agents/skills/` as the project-scoped canonical skills directory. Documented in tool-landscape research.

### Moved forward

- **Connector trait** → [#192](https://github.com/MartinP7r/tome/issues/192). Unified source/target interface. The BTreeMap solved config flexibility; the trait solves architectural abstraction.
- **Built-in connectors** → Part of [#192](https://github.com/MartinP7r/tome/issues/192). Claude, Codex, Antigravity, Cursor, Windsurf, Amp, Goose, etc.
- **Format awareness per connector** → Captured in [#57](https://github.com/MartinP7r/tome/issues/57) (Format Transforms).
- **`.claude/rules/` syncing** → [#193](https://github.com/MartinP7r/tome/issues/193). Managed from `~/.tome/rules/`, distributed to each target's rules dir. See Tentative — Format Transforms.
- **Instruction file syncing** → [#194](https://github.com/MartinP7r/tome/issues/194). Managed from `~/.tome/instructions/`, mapped to tool-specific filenames. See Tentative — Format Transforms.

## v0.3.x — Portable Library (MVP) ✓

Complete the multi-machine skill management story. The lockfile (#38, shipped early) provides the diff mechanism; this milestone adds the interactive UX and per-machine control.

- [x] **Per-machine preferences** ([#39](https://github.com/MartinP7r/tome/issues/39)) (`~/.config/tome/machine.toml`): Per-machine opt-in/opt-out for skills — machine A uses skills 1,2,3 while machine B only wants 1 and 3. Disabled skills stay in the library but are skipped during distribution.
- [x] **`tome update` command** ([#40](https://github.com/MartinP7r/tome/issues/40)): Reads lockfile, diffs against local state, surfaces new/changed/removed skills interactively. Offers to disable unwanted new skills. Notification-only for managed plugins — auto-install deferred to v0.5.

## v0.4.1 — Browse

Interactive skill browser. Depends on v0.2.1 output layer for clean data access.

### `tome browse` — Interactive TUI ([#162](https://github.com/MartinP7r/tome/issues/162))

Full-screen interactive skill browser using **ratatui** for rendering and **nucleo** (Helix editor's fuzzy engine) for matching. skim was ruled out because it owns the terminal and can't be embedded in a ratatui layout.

- [x] **Basic list with fuzzy search** ([#164](https://github.com/MartinP7r/tome/issues/164)): fzf-style interactive filtering of library skills
- [x] **Preview panel** ([#165](https://github.com/MartinP7r/tome/issues/165)): Split-pane layout showing SKILL.md content alongside the list
- [x] **Sorting and grouping** ([#166](https://github.com/MartinP7r/tome/issues/166)): Sort by name/source/last synced, group by source
- [x] **Detail screen with actions** ([#169](https://github.com/MartinP7r/tome/issues/169)): Per-skill actions (view source, copy path, disable/enable)

### Other v0.4.1 Items

- [x] **Enhance `tome status` display** ([#168](https://github.com/MartinP7r/tome/issues/168)): Health indicators (✓/✗/⚠), tilde-collapsed paths
- [x] **Clarify plugin cache source wording** ([#312](https://github.com/MartinP7r/tome/issues/312)): Clarified as "active plugins installed from Claude Code marketplace"

## v0.4.2 — Skill Validation & Linting

YAML frontmatter parsing and a `tome lint` command that catches cross-tool compatibility issues. See [Frontmatter Compatibility](docs/src/frontmatter-compatibility.md) for the full spec comparison. Tracked in [#47](https://github.com/MartinP7r/tome/issues/47) and [#176](https://github.com/MartinP7r/tome/issues/176).

### Frontmatter Parsing

- [ ] Add `serde_yaml` dependency
- [ ] Create a `Skill` struct with typed fields for the base standard (name, description, license, compatibility, metadata, allowed-tools)
- [ ] Parse frontmatter during discovery (enrich `DiscoveredSkill`)
- [ ] Store parsed metadata for validation and status display

### `tome lint` Command

Validation checks ordered by severity:

**Errors** (skill will break on one or more targets):
- Missing required `name` field
- Missing required `description` field
- `name` doesn't match containing directory name
- `name` exceeds 64 chars or uses invalid characters (must be lowercase letters, numbers, hyphens)
- `description` exceeds 1024 chars

**Warnings** (cross-platform compatibility issues):
- Non-standard fields (`version`, `category`, `tags`, `last-updated`) — suggest moving to `metadata`
- Platform-specific fields used (`disable-model-invocation`, `excludeAgent`, etc.) — note which target they're for
- Multiline YAML description without block scalar indicator (`|`) — will break on Claude Code
- `description` exceeds 500 chars (Copilot limit)
- Body exceeds 6000 chars (Windsurf limit)
- **Hidden Unicode Tag codepoint scanning**: Detect U+E0001–U+E007F tag characters that can smuggle invisible instructions (security)

**Info** (best practices):
- `allowed-tools` used (experimental, may not be supported everywhere)
- Body exceeds ~5000 tokens (general recommendation)

### Enhance Existing Commands

- [ ] **`tome doctor`**: Add frontmatter health checks alongside existing symlink diagnostics — parse all library skills and report validation results
- [ ] **`tome status`**: Show parsed frontmatter summary per skill — name, description (truncated), field count, and any validation issues inline

### Target-Aware Warnings (Future)

Requires the v0.3 connector architecture. When distributing to specific targets, warn about:
- Fields unsupported by that target
- Description length exceeding target's limit
- Body syntax incompatible with target (e.g., XML tags, `!command`, `$ARGUMENTS`)

## v0.5 — Managed Sources

Auto-install managed plugins and backup the library. Builds on the portable library foundation from v0.3.x.

- [ ] **`tome update` auto-install**: Extend `tome update` to actively run `claude plugin install <name@registry>` for approved managed plugins, upgrading from notification-only to full reconciliation.
- [ ] **Claude marketplace first** ([#41](https://github.com/MartinP7r/tome/issues/41)): First managed source targeting the Claude plugin marketplace. Version pinning via version string or git commit SHA.
- [ ] **Git-backed backup & restore** ([#94](https://github.com/MartinP7r/tome/issues/94)): `tome backup` subcommand for snapshots, restore, and diff of library state. Optional automatic pre-sync snapshots (`auto_snapshot = true`). Includes `tome restore <git-url>` for bootstrapping a new machine from a git-backed library.
- [x] **Portable config paths**: Wizard writes `~/`-prefixed paths in `tome.toml` for portability across machines.
- [ ] **Git repo scope for library**: Design decision needed — should the git repo be scoped to just the library, or to a broader "tome home" that also tracks hooks, commands, agents, and other AI tool config? The broader scope is more useful for portable machine setup (backup everything, not just skills), but complicates `tome sync` git operations which currently assume library-only changes. Options: (a) git root = `~/.tome/` (includes skills + config), (b) git root = library only (skills), (c) let the user decide and scope git operations accordingly.
- [ ] **Remote library sync**: `tome pull` / `tome push` for syncing the git-backed library with a remote. Handle merge conflicts in skill files — detect conflicts after pull, offer `tome doctor` to verify integrity. `tome update` lockfile diffing should account for remote changes to the lockfile.
- [x] **Shell completions** ([#208](https://github.com/MartinP7r/tome/issues/208)): `tome completions <shell>` for bash, zsh, fish, PowerShell via `clap_complete`
- [x] **Demote lockfile write failure to warning** ([#224](https://github.com/MartinP7r/tome/issues/224)): Lockfile write failures demoted to warning
- [ ] **Skill lifecycle** ([#252](https://github.com/MartinP7r/tome/issues/252)): Forking, evaluation, and publishing workflow

## v0.6 — Git Sources

- [ ] **Git sources** ([#58](https://github.com/MartinP7r/tome/issues/58)): Add `type = "git"` source for remote skill repositories with clone/pull on sync, caching, branch/tag/SHA pinning, and private repo support via SSH keys or token auth. Store `source_url`, `git_ref`, `git_commit`, `skill_path_in_repo` in manifest/lockfile.
- [ ] **Standalone SKILL.md import** ([#92](https://github.com/MartinP7r/tome/issues/92)): Import standalone SKILL.md from arbitrary GitHub repos without requiring plugin.json
- [ ] **Update skill source after the fact**: Allow changing a skill's source (e.g. from local directory to git repo) without removing and re-adding. Use case: "I started with a local copy, now I want to track their git repo instead."

## v0.7 — Skill Composition ("Wolpertinger")

Highly experimental. Generate custom skills by combining or synthesizing content from multiple skill authors/sources.

- [ ] **Multi-source skill synthesis** ([#267](https://github.com/MartinP7r/tome/issues/267)): Select parts from multiple skills (GitHub repos, Claude marketplace, npx skills) and let an LLM create a merged "franken-skill"
- [ ] **ACP-based authentication**: LLM calls go through an Agent Communication Protocol (ACP) flow — authenticate via existing CLIs the user already has (codex-cli, claude-code, gemini CLI) rather than requiring a separate OAuth/API-key setup
- [ ] **Skill evaluation/creation skill** ([#268](https://github.com/MartinP7r/tome/issues/268)): A companion skill that agents can use to evaluate, validate, and author skills against the agent skills standard — dogfooding the format
- [ ] **`tome lint` standard validation** (extension): Extend `tome lint` (v0.4.1) to validate against the emerging agent skills standard, not just cross-tool frontmatter compat

Dependencies: v0.5 (managed sources for marketplace access), v0.6 (git sources for GitHub repos), v0.4.1 (lint infrastructure)

## Tentative — Per-Target Skill Management

Convenient UX for managing which skills are active per target, and whether per-target config should live centrally or locally. Builds on [#253](https://github.com/MartinP7r/tome/issues/253) (per-target skill selection in `machine.toml`).

- [ ] **Target skill management commands**: Convenient CLI for adding/removing active skills per target without editing TOML by hand. E.g. `tome target claude enable my-skill`, `tome target codex disable my-skill`, or interactive via `tome browse` actions.
- [ ] **Package-level toggling**: Enable/disable all skills from a package at once (e.g. `tome target codex disable --package axiom-ios-skills`). Requires the package/repo label from `SkillProvenance.registry_id`. Also support glob patterns (e.g. `asc-*`). In `machine.toml`, this could be `disabled_packages = [...]` alongside the existing `disabled` skill set.
- [ ] **Local per-target config**: Investigate whether per-target config should live *in* the target folder itself (e.g. `~/.claude/tome.toml`) instead of only centrally. Trade-offs:
  - Central (`~/.tome/tome.toml`): single source of truth, easy to version-control, but needs namespacing for per-target overrides
  - Local (e.g. `~/.claude/tome.toml`): self-contained per tool, discoverable where the tool lives, but scattered across filesystem
  - Hybrid: local overrides central if present — local file wins for that target's skill selection, central file is the default. Central config would need a `[targets.<name>.skills]` section or similar namespacing.
  - **Current leaning: local replaces central** for simplicity — if a local `tome.toml` exists in the target folder, it fully owns that target's skill selection. No merge semantics to reason about.
  - Remaining question: How does this interact with `machine.toml` per-machine preferences?

## Tentative — Format Transforms

Not yet scheduled. Needs more design work before committing to a milestone.

- **Rules syncing** ([#193](https://github.com/MartinP7r/tome/issues/193)): Manage tool-specific rule files from `~/.tome/rules/`, distributed via symlinks to each target's rules directory (`.claude/rules/`, `.cursor/rules/`, etc.)
- **Instruction file syncing** ([#194](https://github.com/MartinP7r/tome/issues/194)): Manage root-level instruction files (CLAUDE.md, AGENTS.md, GEMINI.md, .cursorrules) from `~/.tome/instructions/`. High complexity — each tool expects a different filename and format; needs a mapping layer and conflict handling.
- **Connector trait** ([#192](https://github.com/MartinP7r/tome/issues/192)): Unified source/target interface as an architectural abstraction over the existing `BTreeMap` config.
- **Pluggable transform pipeline**: Connectors declare input/output formats; the pipeline resolves the translation chain. Preserves original format — transforms are output-only.
- **Copilot `.instructions.md` format**: Copilot's `.instructions.md` as a transform target alongside Cursor `.mdc` and Windsurf rules.
- [x] **Deprecate `DistributionMethod::Mcp`**: Removed in [#262](https://github.com/MartinP7r/tome/issues/262). No known targets used MCP distribution — all major AI coding tools read SKILL.md files from disk via symlinks. The `tome-mcp` binary, `tome serve` command, and `TargetMethod::Mcp` distribution path were removed along with the `rmcp` and `tokio` dependencies. MCP support can be re-added if a concrete use case emerges.

## Tentative — Expand Wizard Auto-Discovery

Scope needs clarifying before committing. The question: which global home-dir skill paths exist for tools not yet covered by the wizard (e.g. `~/.cursor/skills/`, Windsurf's equivalent, etc.)? Per-project paths (`.github/skills/`, `.cursor/rules/`) are explicitly **out of scope** — only global home-dir paths qualify.

- Audit which global home-dir paths exist across all major tools
- Add any confirmed paths to `KNOWN_SOURCES` in `wizard.rs`

## Tentative — Watch Mode

Not yet scheduled. Low priority until core sync pipeline stabilizes.

- `tome watch` for auto-sync on filesystem changes ([#59](https://github.com/MartinP7r/tome/issues/59))
- Debounced fsnotify-based watcher
- Optional desktop notification on sync

## Future — Companion macOS App

Native macOS skill manager app (inspired by [CodexSkillManager](https://github.com/Dimillian/CodexSkillManager)):

- **Browse & manage library**: View all skills in the tome library with rendered Markdown previews using [swift-markdown-ui](https://github.com/gonzalezreal/swift-markdown-ui)
- **Visual skill editing**: Edit skill frontmatter and body with live preview
- **Sync trigger**: Run `tome sync` from the GUI with status feedback
- **Source & target management**: Configure sources and targets visually instead of editing `tome.toml`
- **Health dashboard**: Surface `tome doctor` and `tome status` diagnostics in a native UI
- **Import/export**: Import skills from folders or zip files; export skills for sharing
- **Tech stack**: SwiftUI (macOS 15+), swift-markdown-ui for rendering, invokes `tome` CLI under the hood

## Future Ideas

- **Plugin registry**: Browse and install community skill packs (precursor to v0.7 Wolpertinger)
- **Conflict resolution UI**: Interactive merge when skills collide
- ~~**Shell completions**~~: Shipped in v0.4.1 (#208)
- **Homebrew formula**: `brew install tome`
- **Backup snapshots**: Moved to v0.5 as git-backed backup (#94)
- **Token budget estimation**: Show estimated token cost per skill per target tool in `tome status` output
- **Security audit command**: `tome audit` to scan skills for prompt injection vectors, hidden unicode, and suspicious patterns
- **Portable memory extraction**: Suggest MEMORY.md entries that could be promoted to reusable skills (`tome suggest-skills`)
- **Plugin output generation**: Package the skill library as a distributable Claude plugin, Cursor plugin, etc.
- **Publish on crates.io**: Make `tome` installable via `cargo install tome` from the crates.io registry
- **Improve doc comments for `cargo doc`**: Module-level `//!` coverage is uneven across modules; no `# Examples` sections. Low priority polish.
- **Syntax highlighting in browse preview**: Render SKILL.md with markdown/YAML syntax highlighting in the `tome browse` detail panel (e.g. via `syntect` or `tree-sitter-highlight`). Low priority polish.
- **Package/repo label for skills**: Surface the plugin name (e.g. `martinp7r/axiom-ios-skills`) or git repo slug as a searchable `package` field in browse. Currently `SkillProvenance.registry_id` stores this for marketplace skills but it doesn't reach the browse UI or fuzzy search. Would also enable "group by package" in browse.
- ~~**`tome relocate`** ([#333](https://github.com/MartinP7r/tome/issues/333))~~: Shipped in v0.3.7
- ~~**`tome eject`** ([#334](https://github.com/MartinP7r/tome/issues/334))~~: Shipped in v0.3.7
- **Library inside a parent git repo**: Superseded by the "git repo scope" item in v0.5. Open design question: scope git to just skills, or broader `~/.tome/` home including hooks/commands/agents.
- **Plugin marketplace discovery** ([#309](https://github.com/MartinP7r/tome/issues/309)): Make tome skills discoverable in the Claude Code marketplace
- **Vercel skills.sh format compatibility** ([#304](https://github.com/MartinP7r/tome/issues/304)): Evaluate mapping tome lockfile to/from Vercel's `skills-lock.json` for cross-ecosystem compatibility
- **Central library architecture** ([#306](https://github.com/MartinP7r/tome/issues/306)): Source skills should not be used directly — always go through the library as single source of truth
- **Skill-scribe extraction** ([#307](https://github.com/MartinP7r/tome/issues/307)): Extract format conversion into a standalone `skill-scribe` package. See also format transform pipeline ([#57](https://github.com/MartinP7r/tome/issues/57))
