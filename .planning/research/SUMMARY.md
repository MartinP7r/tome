# Project Research Summary

**Project:** tome v0.6 — Unified Directory Model
**Domain:** Rust CLI config refactor + git source integration
**Researched:** 2026-04-10
**Confidence:** HIGH

## Executive Summary

tome v0.6 is a config model refactor that replaces the artificial source/target split with a unified `[directories.*]` concept, plus adds git-based skill sources. Research across all four dimensions is unanimous: **zero new crate dependencies are needed**. Git operations use the system `git` CLI (already required for backup). Config schema changes use standard serde patterns. URL hashing reuses the existing `sha2` crate.

The primary risk is **silent config parse success** — serde's `#[serde(default)]` means old `tome.toml` files parse into an empty directories map, cascading into cleanup deleting the entire library. Two specific guards prevent this: `deny_unknown_fields` on the Config struct and an empty-directories guard in the cleanup stage.

The architecture change is surgical: the sync pipeline stages (discover → consolidate → distribute → cleanup) remain intact. What changes is how they receive inputs — role-based iterator methods on Config replace direct access to `config.sources` and `config.targets`.

## Key Findings

### Recommended Stack

No new dependencies. All v0.6 requirements are covered by existing crates + stdlib.

- **`std::process::Command` (git CLI)**: Clone/pull/checkout for git sources. git2 adds a C dependency; gix lacks pull support. tome already requires git for backup.
- **`sha2` (existing dep)**: URL-to-cache-path hashing following Cargo's `<name>-<short-hash>` pattern.
- **`serde` + `toml` (existing deps)**: New `DirectoryConfig` struct with role/type enums. Hard break means no migration library needed.

### Expected Features

**Table stakes (must have):**
- Unified directory config — every mature tool uses single declarations per dependency
- Git source clone/pull — Cargo, Go, SPM, Terraform all support git-based deps
- Git ref pinning (branch/tag/SHA) — universal pattern, record resolved SHA in lockfile
- `tome remove` CLI — counterpart to `tome init` for config management

**Differentiators:**
- Bidirectional `synced` role (discover + distribute) — genuinely novel; no comparable tool models this
- Per-target skill selection — extends machine.toml with per-directory disabled/enabled lists

**Anti-features for v0.6:**
- Format transforms / connector trait — separate concern, defers complexity
- Registry/marketplace hosting — ecosystem already has Skills.sh
- Template engine for config — chezmoi-style templates add complexity without proportional value

### Architecture Patterns

The unified directory model maps cleanly to the existing pipeline:

| Role | Discovery | Distribution | Consolidation |
|------|-----------|-------------|---------------|
| Managed | ✓ | ✗ | Symlink (pkg manager owns) |
| Synced | ✓ | ✓ | Copy (library owns) |
| Source | ✓ | ✗ | Copy (library owns) |
| Target | ✗ | ✓ | N/A |

Git resolution fits as a **pre-discovery step** that resolves git URLs to local cache paths. The `ResolvedDirectories` wrapper carries effective paths while keeping config immutable.

The `shares_tool_root()` path heuristic for circular symlink prevention is replaced by a clean **role-based check**: managed skills are never distributed back to directories that share their tool root.

### Critical Pitfalls

1. **Silent config parse → library deletion** — `#[serde(default)]` on `directories` field means old config parses as empty. Guard: `deny_unknown_fields` + empty-directories cleanup guard.
2. **`git pull` on shallow clones fails on force-push** — Use `git fetch --depth 1 origin <ref> && git reset --hard FETCH_HEAD` instead.
3. **GIT_DIR environment leakage** — tome runs inside a git repo (backup). All git-source Commands must clear `GIT_DIR`, `GIT_WORK_TREE`, `GIT_INDEX_FILE`.
4. **BTreeMap ordering surprises** — Alphabetical priority is deterministic but not intuitive. Emit conflict warnings at sync time.
5. **Test coverage regression** — All integration tests reference old config format. Catalog test scenarios before rewriting.

## Roadmap Implications

**Suggested phasing (coarse, 3-5 phases):**

1. **Foundation** (highest risk): Unified directory config + wizard rewrite + pipeline adaptation + state schema migration. One atomic PR. Must include `deny_unknown_fields` and empty-config guard.
2. **Git Sources** (additive): New `git.rs` module, clone to `~/.tome/repos/<hash>/`, fetch+reset pattern, lockfile SHA. Separate PR to isolate network complexity.
3. **Selection & Management** (quality-of-life): Per-target skill selection, `tome remove`, standalone GitHub import, source reassignment, browse polish. Largely independent items.

Phases 1 is v0.6.0 (breaking config change). Phases 2-3 are v0.6.x point releases.

## Research Gaps

- Per-target selection machine.toml schema needs detailed design in Phase 3
- Standalone import URL parsing strategy (GitHub API vs raw URLs) unresolved
- Shallow clone + `--single-branch` interaction needs testing in CI

---
*Synthesized: 2026-04-10 from STACK.md, FEATURES.md, ARCHITECTURE.md, PITFALLS.md*
