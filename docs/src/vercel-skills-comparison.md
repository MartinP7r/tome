# Vercel Skills Comparison

Research into [vercel-labs/skills](https://github.com/vercel-labs/skills) (`npx skills`) — a close comparison for cross-agent skill management. This document compares current capabilities and architecture trade-offs; it is not a list of Tome roadmap commitments.

*Last updated: October 24, 2026*

> **Scope note:** Tome's current status and path presets were checked on this date. Vercel details not cited below remain comparison snapshots and were not all revalidated for this update.

---

## 1. Overview

|                  | **tome**                                           | **Vercel Skills**                                  |
| ---------------- | -------------------------------------------------- | -------------------------------------------------- |
| **Language**     | Rust (edition 2024)                                | TypeScript (Node.js 18+)                           |
| **Install**      | `cargo install tome` / Homebrew                    | `npx skills` (zero-install)                        |
| **Version**      | v0.17.0                                            | Not rechecked for this update                      |
| **Architecture** | Library-first: discover → consolidate → distribute | Installer-first: fetch → install (symlink/copy)    |
| **Scope**        | Multi-machine library manager with lockfile sync   | Single-machine skill installer with remote sources |

**Core philosophical difference:** Tome treats the library as the source of truth — skills are consolidated into a local library, then distributed to targets. Vercel Skills is an installer — it fetches from remote sources and symlinks directly into agent directories. There's no intermediate "library" abstraction.

---

## 2. Feature Comparison

| Feature                       | tome             | Vercel Skills           | Notes                                                                               |
| ----------------------------- | ---------------- | ----------------------- | ----------------------------------------------------------------------------------- |
| **Local directory sources**   | ✅                | ✅                       | Both scan local paths for `SKILL.md` dirs                                           |
| **Claude plugin sources**     | ✅                | ✅                       | Tome reads `installed_plugins.json`; Vercel reads `.claude-plugin/marketplace.json` |
| **GitHub remote sources**     | ✅                | ✅                       | Tome stores Git sources once in shared repository policy                            |
| **GitLab remote sources**     | ✅                | ✅                       | Full URL support                                                                    |
| **Well-known HTTP providers** | ❌                | ✅                       | RFC 8615 `/.well-known/skills/index.json` endpoints                                 |
| **npm/node_modules sync**     | ❌                | ✅ (experimental)        | Crawls node_modules for skills                                                      |
| **Directory-copy deployments** | ✅                | ✅ (option)              | Tome deploys copies; Vercel offers symlink or direct-copy modes.[14]                |
| **MCP distribution**          | ❌                | ❌                       | Tome has no MCP-based skill distribution.                                           |
| **Lockfile**                  | ✅ `tome.lock`    | ✅ `.skill-lock.json` v3 | Both track content hashes and provenance                                            |
| **Persistent destination routing** | ✅ profile/project routes | ❌              | Tome OR-matches shared skill tags and supports explicit destination exclusions      |
| **Multi-machine sync**        | ✅ `tome sync`    | ❌                       | Lockfile diffing with interactive triage                                            |
| **Library consolidation**     | ✅                | ❌                       | Tome's two-tier model; Vercel installs directly                                     |
| **Interactive browse**        | ✅ `tome browse`  | ❌                       | TUI with fuzzy search (ratatui + nucleo)                                            |
| **Skill scaffolding**         | ❌                | ✅ `skills init`         | Generates SKILL.md template                                                         |
| **Public search/registry**    | ❌                | ✅ `skills find`         | API-backed search at skills.sh with install counts                                  |
| **Remote update checking**    | ✅ `tome sync`    | ✅ `skills check`        | Tome synchronizes repository-owned Git sources under local consent                  |
| **Installed-agent detection** | ❌ (known paths only) | ✅                    | Vercel detects installed agents; Tome's wizard discovers registered paths          |
| **Format transforms**         | ❌                | ❌                       | Tome does not currently convert SKILL.md to `.mdc` or `.instructions.md`            |
| **Frontmatter validation**    | ✅ `tome lint`    | Partial                 | Tome enforces required fields and Agent Skills base constraints.[1][2]             |
| **Doctor/diagnostics**        | ✅ `tome doctor`  | ❌                       | Orphan detection, manifest repair, symlink health                                   |
| **MCP server**                | ❌                | —                        | Tome does not implement an MCP server or MCP command.                              |
| **Dry-run mode**              | ✅                | ❌                       | Preview changes without filesystem writes                                           |
| **Git commit integration**    | ✅                | ❌                       | Auto-offers commit after sync when library is a git repo                            |
| **Telemetry**                 | ❌                | ✅                       | Anonymous usage tracking (disabled in CI)                                           |
| **Home-relative wizard presets** | 15             | Large catalog            | Tome's exact preset list is documented in the [Tool Landscape](tool-landscape.md#home-relative-wizard-directory-presets); project paths are separate. |

---

## 3. Capability Details and Gaps

### 3.1 Remote Git Sources

Vercel's source parser accepts multiple formats:

```
skills add owner/repo                    # GitHub shorthand
skills add owner/repo@skill-name         # specific skill from repo
skills add owner/repo/tree/main/skills/  # subpath targeting
skills add https://gitlab.com/org/repo   # GitLab
skills add git@github.com:owner/repo.git # SSH
skills add ./local-path                  # local directory
```

Branch/tag targeting via `/tree/<ref>` syntax. Subpath extraction lets users install a single skill from a multi-skill repo.

**Tome status:** Implemented. `tome add` accepts GitHub slugs, full URLs,
`/tree/<ref>/<subdir>` shortcuts, explicit ref pins, and subdirectories. Git
sources are shared repository policy and do not accept a destination `--to`.

### 3.2 Skill Scaffolding (`skills init`)

```
npx skills init my-skill
```

Generates a SKILL.md template with frontmatter boilerplate. Low complexity, high convenience for skill authors.

**Tome status:** Not implemented. A `tome new <name>` command could create `<name>/SKILL.md` with a frontmatter template if skill-authoring support is prioritized.

### 3.3 Public Search & Registry

`skills find [query]` provides:
- Interactive terminal UI with keyboard navigation
- API-backed search at `https://skills.sh/` (top 10 results, sorted by install count)
- Debounced queries with formatted output

The registry at skills.sh acts as a public directory of community skills. This creates a discovery loop: authors publish, users search, install counts drive ranking.

**Tome status:** Not implemented. A public registry is a significant undertaking. Integrating with skills.sh as a read-only source could be a lighter-weight option if public discovery is prioritized.

### 3.4 Remote Update Checking

`skills check` POSTs to a backend API with current lockfile state, compares GitHub tree SHAs to detect available updates. `skills update` then fetches and replaces.

**Tome status:** `tome sync` can synchronize repository-owned Git sources and
reconcile managed state. Local `git_sync = "always" | "ask" | "never"`
controls shared-repository synchronization.

### 3.5 Well-Known Providers

Vercel supports RFC 8615 `/.well-known/skills/index.json` endpoints — any HTTP server can advertise available skills by hosting a JSON manifest at a well-known URL. This enables decentralized skill distribution without a central registry.

**Tome status:** Not implemented. This could be a lightweight discovery option if Tome adds support for HTTP-hosted skill catalogs.

### 3.6 Agent Target Coverage

Tome's wizard currently defines 15 home-relative directory presets. See the [current Tome wizard preset list](tool-landscape.md#home-relative-wizard-directory-presets); these are known directories for discovery, not a claim that every agent integration is complete. The linked preset list cites the vendor documentation behind each path.

Keep home-relative/global paths distinct from project-local directories. For example, `.github/skills/` is project-local and must not be described as a global target.[5][7] Project-local paths are configured separately and may vary by project. The preset list links to the vendor documentation used to verify each global path.

The agent-path list formerly compared here was a historical snapshot, not a verified current catalog, so this page no longer presents it as current Tome support.


**Notable exception — OpenClaw:** Unlike most tools that have a single skills path, OpenClaw has a two-level structure: a shared `.openclaw/skills/` directory across all agents *plus* per-agent `skills/` directories under each agent's workspace. This may require a multi-path target model or an OpenClaw-specific connector extension.

**Persistent destination selection:** Vercel's `--agent` flag filters which
agents receive a skill at install time, but the assignment is not persisted in
its lockfile. Tome persists classification and routing separately: shared tags
live in `.tome-manifest.json`, while the profile or project that owns a
destination stores its route.

```toml
# machines/work.toml or a project .tome.toml
[routes.codex]
tags = ["portable", "coding"]
exclude = ["claude-only-skill"]
```

Any selected tag can match; an explicit exclusion wins. Newly imported skills
are untagged and remain library-only for routed destinations. This keeps source
provenance independent from destination choice and avoids changing skill
frontmatter.

### 3.7 npm/node_modules Sync

`skills experimental_sync` scans `node_modules/` for packages containing skills. This supports distributing skills as npm packages — a novel distribution channel.

**Tome status:** Not implemented. Skills distributed through package managers remain a possible integration, not a current capability.

### 3.8 Plugin Manifest Compatibility

Vercel reads `.claude-plugin/marketplace.json` and `.claude-plugin/plugin.json` to discover skills bundled with Claude plugins. This enables compatibility with the Claude plugin marketplace ecosystem.

**Tome status:** Tome reads `installed_plugins.json` from the Claude plugin cache directory (a different integration point). The `.claude-plugin/` manifest format is not currently parsed. Both approaches achieve plugin-sourced skill discovery, but through different mechanisms.

---

## 4. Tooling & DX Patterns

### Source Parser

Vercel's `source-parser.ts` normalizes diverse input formats into a unified `ParsedSource` type:

```typescript
type ParsedSource = {
  owner: string;
  repo: string;
  provider: 'github' | 'gitlab' | 'local';
  ref?: string;           // branch/tag
  subpath?: string;       // path within repo
  skillName?: string;     // specific skill
}
```

This decouples source resolution from installation logic. Tome already supports GitHub/GitLab sources, refs, and subdirectories; Vercel's normalized `ParsedSource` model is a useful comparison for future parser maintenance.

### Lockfile Versioning

Vercel's lockfile has a `version` field (currently v3). When an old-format lockfile is detected, it's wiped entirely — users must reinstall. This aggressive migration strategy avoids complex upgrade code at the cost of user inconvenience.

Tome's `tome.lock` already includes schema `version: 1`, but the loader does not yet dispatch explicit migrations or reject unsupported versions with a dedicated compatibility error. Future schema changes should add that handling before the version is incremented.

### Agent Auto-Detection

Vercel detects installed agents asynchronously by checking for agent-specific markers (config directories, binaries). This enables smart defaults during installation — only install to agents the user actually has.

Tome's wizard does basic path existence checks for known source/target locations, but doesn't detect agents as a first-class concept. The wizard could benefit from a richer detection step.

### Security: Path Sanitization

Vercel's `sanitizeName()` prevents directory traversal via skill names, and `isSubpathSafe()` rejects `..` segments. Tome's `SkillName` type rejects path separators (`/`, `\`) at parse time, achieving the same goal through the type system. Tome's approach is arguably stronger — invalid names can't even be constructed.

---

## 5. Architectural Differences

| Aspect                 | tome                                             | Vercel Skills                              |
| ---------------------- | ------------------------------------------------ | ------------------------------------------ |
| **Data flow**          | Sources → Library → Targets                      | Remote → Agent directories                 |
| **Canonical location** | Library dir (`~/.tome/skills/`)                  | Agent skills dirs (`.agents/skills/`)      |
| **Multi-machine**      | Shared policy + profiles + manifest + lockfile   | Single-machine only                        |
| **Offline support**    | Full (library is local)                          | Partial (needs network for remote sources) |
| **Update model**       | Diff-based triage (`tome sync`)                | Replace-based (`skills update`)            |
| **Cleanup**            | Automated stale removal with interactive confirm | Manual `skills remove`                     |
| **Diagnostics**        | `tome doctor` with repair                        | None                                       |

**Key takeaway:** Tome's library abstraction adds complexity but enables features Vercel can't easily replicate (multi-machine sync, lockfile diffing, automated cleanup, diagnostics). Vercel's installer model is simpler but single-machine.

---

## 6. Recommendations

Prioritized by effort-to-value ratio, mapped to existing roadmap items where applicable.

### Quick Wins (small effort, immediate value)

1. **Maintain verified wizard presets** — Add home-relative paths only when current vendor documentation confirms them; keep project-local paths out of global discovery. The registry is data-driven in `wizard.rs`.

2. **`tome new <name>` scaffolding** — Generate a `<name>/SKILL.md` template with standard frontmatter. Simple new command. *(New issue)*

3. **Lockfile migration handling** — Add an explicit compatibility path for unsupported `tome.lock` schema versions before changing the current `version: 1` format.

### Medium-Term (aligns with existing roadmap)

4. **Route-management ergonomics** — Build on the implemented profile/project
   tag routes with better inspection and bulk operations. Vercel's `--agent`
   flag remains install-time-only, while Tome persists routes.

5. **Source parser extensions** — Tome already supports GitHub shorthand and
   `/tree/<ref>/<subdir>`; evaluate single-skill selectors and other provider
   conventions only if users need them.

6. **Status-only update checks** — `tome sync` already synchronizes configured Git sources; consider a separate non-mutating update check only if users need discovery without syncing.

7. **Agent auto-detection** — Upgrade wizard to detect installed agents dynamically rather than just checking path existence. *(Enhancement to wizard)*

### Future Consideration (worth watching)

8. **Well-known providers** — RFC 8615 skill endpoints could complement git sources as a lightweight discovery mechanism. Novel and decentralized.

9. **skills.sh integration** — Read-only integration with Vercel's public registry as a discovery source. Avoids building our own registry while providing discoverability.

## Sources

[1] https://agentskills.io/specification — Agent Skills specification
[2] https://github.com/agentskills/agentskills/blob/main/skills-ref/src/skills_ref/validator.py — Agent Skills reference validator
[5] https://code.visualstudio.com/docs/agent-customization/agent-skills — VS Code Agent Skills docs
[7] https://learn.microsoft.com/en-us/visualstudio/ide/copilot-agent-skills?view=visualstudio — Microsoft Copilot Agent Skills locations
[14] https://github.com/vercel-labs/skills/blob/main/docs/commands.md — Vercel Skills CLI commands
