# Feature Landscape

**Domain:** AI agent skill/config distribution tool (CLI)
**Researched:** 2026-04-10

## Table Stakes

Features users expect. Missing = product feels incomplete.

| Feature | Why Expected | Complexity | Notes |
|---------|--------------|------------|-------|
| Unified directory config | Every mature tool (Cargo workspaces, Terraform modules, Nix) uses a single config block per directory with role annotations rather than separate source/target lists. The current source/target split creates the exact overlap problem `find_source_target_overlaps()` exists to detect. | High | Core of v0.6. Replaces `[[sources]]` + `[targets.*]` with `[directories.*]`. Cargo's `[workspace.members]` and Terraform's `module` blocks both use a single declaration per external dependency with role implied by usage. |
| Git source: clone and pull | Cargo (`git = "url"`), Go (`require github.com/...`), SPM (`.package(url:)`), Terraform (`source = "git::https://..."`) all support git-based dependencies as a first-class primitive. The agent skills ecosystem has 351k+ skills as of March 2026, many in GitHub repos. Without git sources, users must manually clone and point a Directory source at the clone. | High | Clone to `~/.tome/repos/<hash>/`, pull on sync. Cargo and SPM both store resolved commits in lockfiles (Cargo.lock, Package.resolved). tome should record resolved commit SHA in `tome.lock`. |
| Git ref pinning (branch/tag/SHA) | Every git dependency system supports at least branch, tag, and commit SHA. Cargo: `branch`, `tag`, `rev` (mutually exclusive). SPM: `.branch()`, `.revision()`, `.exact()`. Terraform: `?ref=v1.0.0`. Go: `@commit-hash` pseudo-versions. | Medium | Implement as `ref` field in directory config. One of: branch name, tag, or full SHA. Default: repo default branch. Record resolved SHA in lockfile regardless of ref type. |
| `tome remove` CLI | Any tool that has `add` or `init` for config entries needs a `remove` counterpart. `npm uninstall`, `cargo remove`, `brew uninstall`. Without it, users edit TOML by hand -- error-prone and hostile to the wizard-based UX. | Low | Remove directory entry from config, clean up library/symlinks. Interactive confirmation. |
| Per-target skill selection | chezmoi uses per-machine templating (`{{ if eq .chezmoi.hostname "work" }}`). Nix home-manager uses `lib.mkIf` guards and per-host imports. tome already has global disable via `machine.toml` -- but "disable skill X only for target Y" is the natural next step. Every user with 2+ tools wants different skill subsets per tool. | Medium | Extend `machine.toml` with `[targets.<name>]` sections containing `disabled` sets. Distribution checks both global and per-target disables. |
| Standalone skill import from URL | `npx skills add <package>` is the ecosystem standard. Users expect to grab a skill from a GitHub URL without configuring a full git source directory. Analogous to `go get` for a single package or `cargo add` for a single dependency. | Medium | Download SKILL.md (+ siblings) from GitHub URL, place in library. Could use GitHub API or raw URL fetch. Record provenance in manifest. |
| Lockfile records git commit SHA | Cargo.lock records exact git commit hashes. SPM's Package.resolved does the same. Go's go.sum records hashes. Reproducibility requires knowing exactly what was synced. | Low | Already have lockfile infrastructure. Add `git_commit_sha` field (already exists in lockfile schema for managed plugins -- extend to git sources). |

## Differentiators

Features that set tome apart from `npx skills` and manual symlink management.

| Feature | Value Proposition | Complexity | Notes |
|---------|-------------------|------------|-------|
| Bidirectional directory roles | No comparable tool treats a single directory as both source and target. Stow is source-only (stow dir -> target). chezmoi is source-only (source state -> home). tome's unified model where `~/.claude/skills` can be both "read skills from here" AND "distribute skills to here" is unique. The `role` field (managed/synced/source/target) on each directory entry is a genuinely novel config model. | High | This IS the v0.6 headline. Roles: `managed` (package manager owns files, symlink from library), `synced` (bidirectional -- library is canonical but skills can originate here), `source` (read-only input), `target` (receive-only output). |
| Source reassignment | No package manager lets you change where a dependency comes from without removing and re-adding. `cargo` requires editing Cargo.toml manually. tome can offer `tome reassign <skill> --from local --to git` to migrate a skill's provenance without losing the skill content. | Medium | Updates manifest provenance metadata. May need to convert between copy (local) and symlink (managed/git) strategies. |
| Automatic role inference in wizard | `npx skills` has no concept of tool relationships. tome's wizard can auto-detect known directories (Claude plugins, Codex skills, Cursor rules) and assign sensible default roles. Cargo workspaces require manual `members` listing. tome can be smarter. | Medium | Merge `KNOWN_SOURCES` and `KNOWN_TARGETS` into `KNOWN_DIRECTORIES` with default roles. Wizard shows summary: "Detected ~/.claude/skills as synced directory (source + target)". |
| Cross-tool skill distribution with single library | `npx skills` installs per-tool. chezmoi distributes dotfiles but has no concept of "the same config going to multiple tools". tome's library-as-hub model where one canonical copy fans out to 16+ AI tools is its core differentiator vs. every other tool in the space. | Already built | v0.2 delivered this. v0.6 just unifies the config model around it. |
| Private git repo support | Terraform supports SSH keys per workspace. Cargo uses SSH agent or credential helpers. Go modules support `GOPRIVATE`. For enterprise users with internal skill repos, SSH-based git clone is essential. | Low | Use system git (inherits SSH agent, credential helpers). No custom auth needed -- just `git clone <url>` with whatever credentials the user's git is configured with. |
| Shallow clone for large repos | Go modules and Cargo both fetch minimal data. For skill repos with thousands of commits, `git clone --depth 1` is the sensible default. | Low | Default to `--depth 1` for initial clone. Full clone available via config flag if needed. `git pull` with `--depth 1` for updates. |

## Anti-Features

Features to explicitly NOT build.

| Anti-Feature | Why Avoid | What to Do Instead |
|--------------|-----------|-------------------|
| Bidirectional file sync (two-way merge) | Dropbox-style conflict resolution is a bottomless pit. Stow deliberately avoids it. chezmoi uses source-state-wins semantics. Git itself is the only good merge tool. | tome's model is clear: library is canonical. Sources flow IN, targets flow OUT. "Synced" directories are just directories that are both source and target, not two-way-merged. |
| Template rendering for per-machine config | chezmoi's Go template system is powerful but adds enormous complexity (template syntax, variable resolution, debugging). Nix's module system is even more complex. | Use tome's existing `machine.toml` for per-machine differences. It handles the 90% case (enable/disable skills per machine/target) without template complexity. |
| Dependency resolution between skills | npm, Cargo, and Go all have dependency solvers. Skills are independent documents, not code with import graphs. Adding dependency resolution would be massive overengineering. | Skills are flat. If skill A needs skill B, document it in the SKILL.md description. The user manages the relationship. |
| Auto-update on schedule / watch mode | Homebrew's auto-update is widely hated. Watch mode (#59) adds daemon complexity for marginal benefit. `tome sync` is fast and explicit. | Keep sync explicit. Users run `tome sync` when they want updates. CI can run it on schedule if desired. |
| Registry / marketplace hosting | `npx skills` already has Skills.sh. Crates.io exists for Cargo. Building a competing registry is a different product entirely. | Support consuming from existing registries (GitHub, npm/npx skills). Don't host. |
| Config migration command | Single user (Martin). Manual migration with documented steps is sufficient. Building `tome migrate` for a one-time use by one person is negative ROI. | Document the config format change in CHANGELOG. Provide a before/after example. |
| Nested git repos in library | Git submodules are universally despised. Placing `.git` directories inside the library (which may itself be a git repo) causes confusion and tooling conflicts. | Clone git sources to `~/.tome/repos/<hash>/`. Consolidate into library via copy or symlink. Keep library clean. |
| Format transforms in v0.6 | Rules syncing (#193), instruction file syncing (#194), and format conversion (#57) are a separate concern. Mixing them with the directory model refactor bloats scope and delays shipping. | Defer to post-v0.6. The unified directory model is the foundation; format transforms build on top of it. |

## Feature Dependencies

```
Unified directory config (#396) --> Git sources (#58)
  (git sources need the new DirectoryConfig with type=git and ref fields)

Unified directory config (#396) --> Per-target skill selection (#253)
  (per-target selection references directory names from the new config)

Unified directory config (#396) --> tome remove (#392)
  (remove operates on the new directory entries)

Unified directory config (#396) --> Source reassignment (#395)
  (reassignment changes directory config entries)

Unified directory config (#396) --> Wizard rewrite (#362)
  (wizard must produce the new config format)

Git sources (#58) --> Standalone SKILL.md import (#92)
  (import is a simplified form of git source -- fetch from URL, place in library)

Git sources (#58) --> Lockfile git SHA recording
  (lockfile must record resolved commit for reproducibility)
```

## MVP Recommendation

The dependency graph makes the ordering clear:

**Phase 1 -- Foundation (must ship together):**
1. Unified directory config (#396) -- everything depends on this
2. Wizard rewrite (#362) -- config is useless without a way to create it

**Phase 2 -- Git ecosystem:**
3. Git sources with ref pinning (#58) -- highest-value new capability
4. Lockfile git SHA recording -- comes naturally with git sources

**Phase 3 -- Selection and management:**
5. Per-target skill selection (#253) -- unblocked by unified config
6. `tome remove` (#392) -- low complexity, high quality-of-life
7. Standalone SKILL.md import (#92) -- simplified git fetch

**Phase 4 -- Polish:**
8. Source reassignment (#395) -- nice-to-have, not blocking
9. Browse TUI polish (#365) -- visual only, no functional dependency

**Defer:** Format transforms, watch mode, registry hosting, template rendering.

## Sources

- [Cargo workspaces](https://doc.rust-lang.org/cargo/reference/workspaces.html) -- unified member/dependency config
- [Cargo git dependencies](https://doc.rust-lang.org/cargo/reference/specifying-dependencies.html) -- branch/tag/rev specification
- [Go modules reference](https://go.dev/ref/mod) -- replace directive, version pinning
- [Swift Package Manager](https://docs.swift.org/package-manager/PackageDescription/PackageDescription.html) -- .branch/.revision/.exact
- [Terraform module sources](https://developer.hashicorp.com/terraform/language/modules/sources) -- git::url?ref= pattern
- [chezmoi machine differences](https://www.chezmoi.io/user-guide/manage-machine-to-machine-differences/) -- per-machine templating
- [GNU Stow manual](https://www.gnu.org/software/stow/manual/stow.html) -- symlink farm management
- [Vercel skills CLI](https://github.com/vercel-labs/skills) -- npx skills add/find/update
- [Agent Skills ecosystem 2026](https://vercel.com/docs/agent-resources/skills) -- 16+ tools, 351k+ skills
- [Nix home-manager per-host config](https://github.com/rycee/home-manager/issues/8) -- conditional module selection
- [Cargo git rev locking](https://github.com/rust-lang/cargo/issues/7497) -- commit hash in Cargo.lock
