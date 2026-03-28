# Vercel Skills Comparison

Research into [vercel-labs/skills](https://github.com/vercel-labs/skills) (`npx skills`) — the closest comparable project to tome. Both manage AI coding skills across multiple tools. This doc catalogs features and tooling patterns tome is missing to inform roadmap decisions.

*Last updated: March 2026*

---

## 1. Overview

|                  | **tome**                                           | **Vercel Skills**                                  |
| ---------------- | -------------------------------------------------- | -------------------------------------------------- |
| **Language**     | Rust (edition 2024)                                | TypeScript (Node.js 18+)                           |
| **Install**      | `cargo install tome` / Homebrew                    | `npx skills` (zero-install)                        |
| **Version**      | v0.3.1                                             | v1.4.5                                             |
| **Architecture** | Library-first: discover → consolidate → distribute | Installer-first: fetch → install (symlink/copy)    |
| **Scope**        | Multi-machine library manager with lockfile sync   | Single-machine skill installer with remote sources |

**Core philosophical difference:** Tome treats the library as the source of truth — skills are consolidated into a local library, then distributed to targets. Vercel Skills is an installer — it fetches from remote sources and symlinks directly into agent directories. There's no intermediate "library" abstraction.

---

## 2. Feature Comparison

| Feature                       | tome             | Vercel Skills           | Notes                                                                               |
| ----------------------------- | ---------------- | ----------------------- | ----------------------------------------------------------------------------------- |
| **Local directory sources**   | ✅                | ✅                       | Both scan local paths for `SKILL.md` dirs                                           |
| **Claude plugin sources**     | ✅                | ✅                       | Tome reads `installed_plugins.json`; Vercel reads `.claude-plugin/marketplace.json` |
| **GitHub remote sources**     | 🔜 v0.6           | ✅                       | `skills add owner/repo`, shorthand syntax, branch specs                             |
| **GitLab remote sources**     | 🔜 v0.6           | ✅                       | Full URL support                                                                    |
| **Well-known HTTP providers** | ❌                | ✅                       | RFC 8615 `/.well-known/skills/index.json` endpoints                                 |
| **npm/node_modules sync**     | ❌                | ✅ (experimental)        | Crawls node_modules for skills                                                      |
| **Symlink distribution**      | ✅                | ✅                       | Both use symlinks as primary distribution method                                    |
| **MCP distribution**          | ❌ (removed)      | ❌                       | Was removed — all tools now scan SKILL.md dirs natively                              |
| **Copy fallback**             | ❌                | ✅                       | Vercel falls back to copy when symlinks fail                                        |
| **Lockfile**                  | ✅ `tome.lock`    | ✅ `.skill-lock.json` v3 | Both track content hashes and provenance                                            |
| **Per-machine preferences**   | ✅ `machine.toml` | ❌                       | Tome can disable skills per machine                                                 |
| **Multi-machine sync**        | ✅ `tome sync`    | ❌                       | Lockfile diffing with interactive triage                                            |
| **Library consolidation**     | ✅                | ❌                       | Tome's two-tier model; Vercel installs directly                                     |
| **Interactive browse**        | ✅ `tome browse`  | ❌                       | TUI with fuzzy search (ratatui + nucleo)                                            |
| **Skill scaffolding**         | ❌                | ✅ `skills init`         | Generates SKILL.md template                                                         |
| **Public search/registry**    | ❌                | ✅ `skills find`         | API-backed search at skills.sh with install counts                                  |
| **Remote update checking**    | ❌                | ✅ `skills check`        | Compares GitHub tree SHAs for available updates                                     |
| **Agent auto-detection**      | 🔜 (wizard only)  | ✅                       | Async detection of 50+ installed agents                                             |
| **Format transforms**         | 🔜 v0.4           | ❌                       | Planned: SKILL.md ↔ .mdc ↔ .instructions.md                                         |
| **Frontmatter validation**    | 🔜 v0.4           | Partial                 | Vercel parses name/description/metadata.internal                                    |
| **Doctor/diagnostics**        | ✅ `tome doctor`  | ❌                       | Orphan detection, manifest repair, symlink health                                   |
| **MCP server**                | ❌ (removed)      | ❌                       | Was removed — no known consumers                                                    |
| **Dry-run mode**              | ✅                | ❌                       | Preview changes without filesystem writes                                           |
| **Git commit integration**    | ✅                | ❌                       | Auto-offers commit after sync when library is a git repo                            |
| **Telemetry**                 | ❌                | ✅                       | Anonymous usage tracking (disabled in CI)                                           |
| **Known agent targets**       | 7                | 50+                     | Significant coverage gap                                                            |

---

## 3. Notable Features Tome Lacks

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

**Tome status:** Planned for v0.6 (Git Sources). Vercel's UX — especially the shorthand syntax and subpath targeting — is worth studying when designing `tome add`.

### 3.2 Skill Scaffolding (`skills init`)

```
npx skills init my-skill
```

Generates a SKILL.md template with frontmatter boilerplate. Low complexity, high convenience for skill authors.

**Tome status:** Not on roadmap. Would be a simple addition — `tome new <name>` that creates `<name>/SKILL.md` with a frontmatter template. Consider adding as a quick win.

### 3.3 Public Search & Registry

`skills find [query]` provides:
- Interactive terminal UI with keyboard navigation
- API-backed search at `https://skills.sh/` (top 10 results, sorted by install count)
- Debounced queries with formatted output

The registry at skills.sh acts as a public directory of community skills. This creates a discovery loop: authors publish, users search, install counts drive ranking.

**Tome status:** Not on roadmap. A public registry is a significant undertaking. However, integrating with skills.sh as a read-only source could be a lighter-weight option — tome could query the same API without building its own registry.

### 3.4 Remote Update Checking

`skills check` POSTs to a backend API with current lockfile state, compares GitHub tree SHAs to detect available updates. `skills update` then fetches and replaces.

**Tome status:** `tome sync` exists but only diffs the local lockfile against the current discovery state. It doesn't check remote sources for newer versions. Once git sources land (v0.6), remote update checking should follow naturally.

### 3.5 Well-Known Providers

Vercel supports RFC 8615 `/.well-known/skills/index.json` endpoints — any HTTP server can advertise available skills by hosting a JSON manifest at a well-known URL. This enables decentralized skill distribution without a central registry.

**Tome status:** Not on roadmap. Novel approach worth considering for the connector architecture. Could be a lightweight alternative to a full registry.

### 3.6 Agent Target Coverage (50+)

Vercel supports 50+ agents. Their `agents.ts` defines per-agent configuration including:
- Project and global skill paths
- Whether the agent shares the universal `.agents/skills/` directory
- Installation detection method

**Agents in Vercel not in tome's KnownTarget list:**

| Agent            | Skills Path                 | Notes             |
| ---------------- | --------------------------- | ----------------- |
| Cline            | `.cline/skills/`            | VS Code extension |
| Warp             | `.warp/skills/`             | Terminal-native   |
| OpenCode         | `.agents/skills/`           | Universal path    |
| CodeBuddy        | `.codebuddy/skills/`        |                   |
| Goose            | `.goose/skills/`            |                   |
| Amp              | `.amp/skills/`              |                   |
| Aider            | `.aider/skills/`            |                   |
| Kilo Code        | `.kilo-code/skills/`        |                   |
| RooCode          | `.roo-code/skills/`         |                   |
| Zed              | `.zed/skills/`              |                   |
| Trae             | `.trae/skills/`             |                   |
| Melty            | `.melty/skills/`            |                   |
| otto-eng         | `.otto/skills/`             |                   |
| Pear             | `.pear/skills/`             |                   |
| Sourcegraph Cody | `.sourcegraph-cody/skills/` |                   |
| Void             | `.void/skills/`             |                   |
| Junie            | `.junie/skills/`            |                   |
| Augment          | `.augment/skills/`          |                   |
| Aide             | `.aide/skills/`             |                   |
| Blackbox AI      | `.blackbox-ai/skills/`      |                   |
| Qodo             | `.qodo/skills/`             |                   |
| Tabnine          | `.tabnine/skills/`          |                   |
| GitHub Spark     | `.spark/skills/`            |                   |

Many share the universal `.agents/skills/` path. Tome's data-driven target config already supports arbitrary agents, but expanding `KnownTarget` auto-discovery would improve the wizard experience.

**Notable exception — OpenClaw:** Unlike most tools that have a single skills path, OpenClaw has a two-level structure: a shared `.openclaw/skills/` directory across all agents *plus* per-agent `skills/` directories under each agent's workspace. This may require a multi-path target model or an OpenClaw-specific connector extension.

**Design consideration — per-target skill selection:** Vercel's `--agent` flag filters which agents receive a skill at install time, but the assignment is **not persisted** — their lockfile has no per-skill agent tracking. `lastSelectedAgents` is just a UX hint for the next prompt. Changing which agents have a skill requires reinstalling. This is a significant limitation.

Tome can do better by managing assignments entirely in `machine.toml` (no skill frontmatter changes needed). Proposed resolution model with layered precedence:

```toml
# machine.toml

# Global: applies to all targets unless overridden (existing behavior)
[disabled]
skills = ["noisy-skill"]

# Per-target: disable additional skills for this target
[targets.codex]
disabled = ["claude-only-skill"]

# Per-target allowlist: ONLY these skills go to this target
[targets.openclaw-agent-x]
enabled = ["specialized-skill"]
```

**Resolution order:**
1. Skill is **enabled by default** for all targets
2. Global `disabled` removes it everywhere (existing `machine.toml` behavior)
3. Per-target `disabled` removes it from specific targets only
4. Per-target `enabled` (if present) acts as an allowlist — only listed skills reach that target

This keeps the common case simple (everything goes everywhere) while supporting opt-out at two granularity levels. The `enabled` allowlist is only needed for niche cases like OpenClaw's per-agent workspaces. All managed in tome settings — no skill frontmatter modifications required.

**Tome status:** Partially addressed in #248 (audit known targets against platform docs). The data-driven config means users can add any target manually, but wizard auto-discovery only covers 7 agents.

### 3.7 npm/node_modules Sync

`skills experimental_sync` scans `node_modules/` for packages containing skills. This supports distributing skills as npm packages — a novel distribution channel.

**Tome status:** Not on roadmap. Low priority given the Rust ecosystem focus, but the concept of "skills as packages" in language-specific package managers is worth noting.

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

This decouples source resolution from installation logic. When tome implements git sources, a similar parser would be valuable.

### Lockfile Versioning

Vercel's lockfile has a `version` field (currently v3). When an old-format lockfile is detected, it's wiped entirely — users must reinstall. This aggressive migration strategy avoids complex upgrade code at the cost of user inconvenience.

Tome's `tome.lock` doesn't yet have a version migration strategy. Worth adding a version field early to avoid future pain.

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
| **Multi-machine**      | Lockfile + per-machine prefs                     | Single-machine only                        |
| **Offline support**    | Full (library is local)                          | Partial (needs network for remote sources) |
| **Update model**       | Diff-based triage (`tome sync`)                | Replace-based (`skills update`)            |
| **Cleanup**            | Automated stale removal with interactive confirm | Manual `skills remove`                     |
| **Diagnostics**        | `tome doctor` with repair                        | None                                       |

**Key takeaway:** Tome's library abstraction adds complexity but enables features Vercel can't easily replicate (multi-machine sync, lockfile diffing, automated cleanup, diagnostics). Vercel's installer model is simpler but single-machine.

---

## 6. Recommendations

Prioritized by effort-to-value ratio, mapped to existing roadmap items where applicable.

### Quick Wins (small effort, immediate value)

1. **Expand KnownTarget list** — Add 15–20 more agents from Vercel's list to wizard auto-discovery. Data-only change in `wizard.rs`. *(Extends #248)*

2. **`tome new <name>` scaffolding** — Generate a `<name>/SKILL.md` template with standard frontmatter. Simple new command. *(New issue)*

3. **Lockfile version field** — Add `"version": 1` to `tome.lock` now, before we need migration logic. *(New issue)*

### Medium-Term (aligns with existing roadmap)

4. **Per-target skill selection** — Extend `machine.toml` with per-target `disabled`/`enabled` lists. Layered resolution: global disabled → per-target disabled → per-target enabled allowlist. Enables OpenClaw per-agent workspaces and general skill-to-agent affinity. Vercel's `--agent` flag is install-time-only with no persistence — tome can do better. *(#253)*

5. **Source parser for git remotes** — Study Vercel's shorthand syntax (`owner/repo`, `@skill-name`, `/tree/branch`) when designing `tome add`. *(Informs v0.6: Git Sources, #58)*

6. **Remote update checking** — Extend `tome sync` to check remote sources, not just local lockfile diffs. *(After v0.6)*

7. **Agent auto-detection** — Upgrade wizard to detect installed agents dynamically rather than just checking path existence. *(Enhancement to wizard)*

### Future Consideration (worth watching)

8. **Well-known providers** — RFC 8615 skill endpoints could complement git sources as a lightweight discovery mechanism. Novel and decentralized.

9. **skills.sh integration** — Read-only integration with Vercel's public registry as a discovery source. Avoids building our own registry while providing discoverability.

10. **Copy fallback** — Vercel supports copy when symlinks fail. Tome is Unix-only and symlink-only. Worth considering if Windows support ever becomes a goal.
