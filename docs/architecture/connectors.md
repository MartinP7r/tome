# Connector Architecture (ADR Draft)

Status: Proposed  
Related: #93, #38, #176

## Context

Tome is being positioned as:

1. **Local-first** — users customize skills as they go.
2. **Authoring-first** — customization is expected, not exceptional.
3. **Format-transformation strong** — interoperability across ecosystems is a core differentiator.

This architecture note captures decisions discussed in planning threads and issue comments.

## Decision

Use **Connector** as the umbrella abstraction, with explicit directional semantics:

- `source`: where a skill definition comes from
- `target`: where/how a skill is installed/activated

The lockfile should remain **declarative**. Imperative install behavior should be derived from connector capabilities.

## Key Design Requirements

### 1) Path discovery is first-class

Connectors must provide robust path discovery (not ad-hoc path checks).

This is especially critical for OpenClaw, where locations may vary by:

- install root
- package manager layout (npm vs pnpm)
- user configuration (additional skill folders)
- overlap with other connectors

Required behavior:

1. discover valid candidate paths
2. deduplicate overlaps
3. honor user overrides
4. fail/warn clearly on missing/unreadable paths

### 2) Lockfile implications (#38)

- Use TOML (`tome.lock`)
- Keep defaults simple and human-editable
- Represent external dependencies declaratively so sync can install missing deps via connector logic

### 3) Collision handling (simplicity-first)

Do not force complex canonical IDs by default.

On sync:

- detect duplicate skill names across sources
- prompt interactive resolution (keep / rename / diff)
- if renamed, preserve `original_name` for traceability
- default toward builtin/official source

### 4) Validation workflow linkage (#176)

Add validator/evaluator workflow for conformance to open skill standards.

Transformation pipeline should support **post-transform validation** to catch incompatibilities early.

## Why this direction

- Keeps everyday setup simple
- Supports power-user customization and migration workflows
- Avoids hardcoding source-specific install commands into lock metadata
- Scales to multiple ecosystems without overfitting the default UX

## Consequences

### Positive

- Clear extension point for new sources/targets
- Better diagnostics and reproducibility
- Strong foundation for format transformation

### Tradeoffs

- Connector implementations need more upfront rigor (discovery + diagnostics)
- Interactive collision handling needs UX care in non-interactive/CI modes

## Follow-ups

- #93: finalize connector trait/capabilities and discovery contracts
- #38: implement TOML lockfile with collision strategy integration
- #176: define validator interfaces and strict/pedantic modes
