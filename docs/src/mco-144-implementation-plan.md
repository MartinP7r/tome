# MCO-144 implementation plan: portable skill pool and curated copy deployments

> **Status:** Proposed implementation plan derived from the [skill-organization architecture](skill-organization-architecture.md). Paperclip is the authoritative execution record: [MCO-144](https://mmini.zuul-bee.ts.net:8443/MCO/issues/MCO-144).
>
> **Product focus:** organize and curate AI-agent skills across machines and projects. Validation and evaluation are supporting evidence. Desktop/Tauri work remains paused unless Martin explicitly reprioritizes it.

## 1. Objective and success criteria

Tome must manage skills through a portable, independently usable deployment chain:

```text
sources and validated Git caches
  → copied canonical skill pool
  → copied machine and project targets
```

A successful implementation has these properties:

1. Canonical library entries are real, owned directories, never symlinks into an upstream package cache or source checkout.
2. Every Tome-managed target is a real copied directory. It keeps working if Tome is uninstalled, moved, misconfigured, or never run again.
3. A target copy is derived state, never canonical input. Editing it cannot silently alter or become pool content.
4. Tome never overwrites, removes, or adopts a foreign or drifted target directory without an explicit, previewed decision.
5. The CLI/TUI can explain a skill's provenance, canonical hash, curation state, selected routes, target health, and drift.
6. Existing portable `tome.toml` plus local `machine.toml` remain the base configuration boundary. Do not introduce a second named-profile framework unless a demonstrated multi-machine need exceeds the current model.
7. A project `.tome.toml`, when introduced, can add project-local destinations but cannot change global sources, pool policy, canonical curation, or the active machine configuration.

## 2. Scope boundaries

### In scope

- Copy-only canonical library and target materialization.
- Deployment ownership, hashes, drift detection, preview, refresh and safe removal.
- Migration of existing Tome-created target symlinks.
- Target status and doctor diagnostics.
- Existing machine-local configuration clarification and constrained project routes.
- Curation records, deterministic intake, provenance, lifecycle, overlap/gap views, customization and fork lineage.
- Structural validation as curation evidence.
- Terminal/TUI-first inspection and actions.

### Explicitly deferred

- Desktop/Tauri implementation or desktop-only state.
- Automatic model calls or a CI quality gate for skill evaluation.
- Semantic/LLM overlap decisions that automatically route, delete, accept, customize or fork skills.
- Marketplace/ecosystem expansion beyond reliable generic discovery and provenance.
- Automatic adoption of user/project edits in a target as canonical changes.

## 3. Preconditions and sequencing

### 3.1 Reliability lane first

Before changing distribution semantics, complete the source-reliability work:

- Merge and release the MCO-52 cached Git-source discovery/security work after normal review.
- Complete the linked Git source end-to-end and clone/update coverage.
- Keep all read-only commands network-free: an unavailable Git cache must produce actionable `tome sync` guidance, not clone or fetch.
- Preserve the Git cache security predicate: only a direct, non-symlink cache root with a direct, non-symlink `.git` directory and bounded Git validation is trusted.

### 3.2 Compatibility inventory

Before the copy-deployment migration:

1. Inventory every current target directory and classify each entry as Tome-managed symlink, broken Tome symlink, foreign symlink, foreign real directory, missing, or already copied.
2. Record current manifest/lockfile semantics, ownership assumptions, cleanup behavior, doctor behavior and CLI/TUI output snapshots.
3. Identify public configuration and JSON/status compatibility requirements.
4. Define migration recovery behavior before writing destructive code.

No migration should be inferred from a path name alone. A path must be proven to be a current Tome-managed symlink before Tome offers to replace it.

## 4. Target deployment contract

### 4.1 Materialization model

Replace target-side symlink creation with a copy deployment engine.

For a selected `(skill, target)` route:

1. Validate the canonical source directory and calculate its canonical content hash.
2. Inspect the destination with symlink-aware metadata.
3. Compare it with the recorded deployment state.
4. Render a plan: create, refresh, report drift, migrate, skip, remove, or require explicit repair/force.
5. Make an approved change through a staging sibling directory and atomic rename where the filesystem allows it.
6. Persist deployment state only after the copy succeeds.

A destination is never replaced merely because it has the expected directory name.

### 4.2 Deployment record

Introduce a versioned, atomic deployment-state file outside the target directories. The exact file name and serialization format should be decided during implementation, but every record must contain:

- schema version;
- stable canonical skill ID/name;
- canonical content hash at last materialization;
- target identifier and absolute target path;
- materialization mode, initially only `copy`;
- last successful materialization timestamp;
- optional observed target hash and last-observed timestamp;
- ownership/migration provenance sufficient to distinguish a Tome-managed copy from foreign content;
- source/canonical reference useful for diagnostics, but not a live target dependency.

Do not put a required mutable metadata marker inside an agent tool's skill directory. Target directories must remain native, self-contained skill directories. If a small marker is later considered, it must be optional and never the sole ownership proof.

### 4.3 State machine

Status/doctor should classify every candidate route as one of:

- `healthy`: recorded copy exists and matches its recorded canonical hash;
- `canonical-updated`: target matches its record but the canonical pool has newer content;
- `drifted`: target exists but differs from its last recorded canonical hash;
- `missing`: recorded target copy no longer exists;
- `legacy-symlink`: current Tome-managed symlink eligible for explicit migration;
- `foreign`: unrecorded or ownership-mismatched file, directory, or symlink;
- `unavailable`: configured target path/tool is unavailable on this machine;
- `disabled-locally`: disabled by existing `machine.toml` settings;
- `blocked-by-constraint`: route conflicts with declared target capability/constraint;
- `stale-record`: deployment record exists but no longer corresponds to a valid route or canonical skill.

`drifted`, `foreign`, and `legacy-symlink` are never silently refreshed or pruned.

### 4.4 Safe operations

- **Create:** only if the destination is absent or an explicit create plan is approved.
- **Refresh:** replace only a healthy Tome-managed copy after preview; never refresh a drifted target without explicit conflict resolution.
- **Migrate:** transform only a proven Tome-managed symlink into a copied deployment after preview. Preserve a foreign/broken symlink and explain why it was not changed.
- **Remove:** remove only a record-matching healthy Tome-managed copy after preview. A drifted copy becomes a report/repair decision, not an automatic cleanup.
- **Repair:** require an explicit `--force`/interactive confirmation mode with a clear description of affected paths and lost target-local edits.
- **Failure recovery:** retain the previous target until the replacement is fully staged. Remove staging artifacts on normal failure and make leftovers detectable by doctor.

## 5. Configuration boundary

### 5.1 Preserve existing layers

Build on the current configuration boundary rather than duplicating it:

- Shared, portable `tome.toml`: pool source policy, source exclusions, targets and shared routing policy.
- Shared library/lockfile/manifest: canonical content, reproducibility and provenance.
- Local `~/.config/tome/machine.toml`: disabled skills/directories, target filters, path overrides and machine-local consent.

The existing `machine.toml` is useful per-machine configuration. It is not a committed named-profile system, and named profiles are not required for the first delivery.

### 5.2 Project routes

Add a project configuration format only when copy deployment supports project targets. A nearest `.tome.toml` may add project-local routes/destinations under that checkout.

Validation rules:

- project config may not add or alter global sources;
- project config may not select or rewrite machine-local settings;
- project config may not mutate canonical pool content, curation, exclusions or provenance;
- project destinations inherit the copy-only materialization contract;
- all project operations must resolve paths safely under the intended project boundary.

### 5.3 Future named profiles

Revisit committed named profiles only when concrete machines need portable, selectable target topologies that cannot be expressed safely by shared `tome.toml` and existing local overrides. If added later, profile selection must be explicit—not inferred from hostnames—and profiles must not redefine canonical source policy.

## 6. Curation catalog and intake

### 6.1 Curation record

Add a versioned, committed curation record for each canonical skill. It must include:

- stable skill ID/name and canonical hash;
- source provenance, upstream URL/identity and source pin where applicable;
- domains and capabilities, modeled many-to-many;
- supported/incompatible targets and declared constraints;
- lifecycle: `candidate`, `accepted`, `routed`, `excluded`, `customized`, `forked`, or `deprecated`;
- relationships: `complements`, `overlaps`, `supersedes`, `derived_from`, and `conflicts_with`;
- licensing and attribution notes when applicable;
- evidence references: lint/validation results, review notes, tests and optional evaluation suite/results.

The catalog must not treat free-form tags as a sufficient substitute for provenance, lifecycle, relationships and decision rationale.

### 6.2 Deterministic intake workflow

1. Discover/import a candidate and store its observed provenance plus immutable hash.
2. Validate package layout, frontmatter, identifiers, paths, declared requirements and target compatibility.
3. Compare it against the catalog using deterministic signals: same upstream, domains/capabilities, target compatibility, frontmatter/text similarity and shared evidence.
4. Require an explicit outcome: accept, keep library-only, route, customize, fork, exclude, deprecate or investigate.
5. Deploy only after the curation state and a valid route permit it.
6. Reassess whenever canonical content or source provenance changes.

Same-name/different-content intake must stop before changing the pool. Equal content from several sources should merge provenance rather than hide the additional source.

### 6.3 Customization and forks

- Use a **customization** when a small reviewable delta can remain linked to a stable upstream base.
- Use a **fork** when behavior or ownership intentionally diverges, the delta cannot be safely maintained, or upstream is no longer an appropriate update channel.
- Both retain `derived_from` and base-hash provenance.
- Neither modifies an upstream snapshot in place.
- Target edits do not become either a customization or fork automatically.

## 7. Validation and evaluation

### 7.1 Deterministic validation

Run validation at intake, before materialization and in suitable CI. Cover:

- SKILL.md structure and frontmatter/schema;
- identifiers and path safety;
- content hashing;
- provenance consistency;
- target capability and route constraints;
- deployment-state schema and ownership invariants.

### 7.2 Evaluation as evidence

The provider-neutral MCO-143 evaluation model remains a separate later capability. It must:

- use hash-addressed, disposable wrappers/workspaces;
- never mutate the canonical library to evaluate a skill;
- keep traces, generated results, credentials and provider/model cost data machine/CI-local;
- preserve `not_comparable` outcomes;
- never automatically route, accept, remove or fork a skill.

## 8. Terminal/TUI experience

Prioritize terminal and ratatui surfaces. The TUI must render the same core state as the CLI; it must not create its own deployment or curation state.

Required views/actions:

- canonical pool inventory with lifecycle, provenance, domains and constraints;
- selected local configuration and active project routes;
- per-target deployment mode, canonical version, health and drift;
- dry-run/preview for create, refresh, migrate, remove and repair;
- clear recovery guidance for missing Git caches, unavailable targets and target drift;
- deterministic overlap candidates and gap reports;
- visible customization/fork lineage.

## 9. Delivery phases

### Phase A — source reliability and baseline inventory

**Outcome:** reliable, safe source discovery and an evidence-backed migration baseline.

- Land MCO-52 and linked Git tests.
- Add/verify source type validation and cache safety invariants.
- Inventory current target artifact states and freeze representative fixtures.
- Document compatibility and migration cases.

**Acceptance:** read-only discovery remains network-free; invalid/redirected Git caches are not trusted; fixtures cover real, broken and foreign target artifacts.

### Phase B — copy materialization and deployment state

**Outcome:** target symlinks are replaced by safe copied deployments for a bounded set of existing tool targets.

- Implement deployment record schema and atomic persistence.
- Implement symlink-aware inspection, staging copy, atomic replacement and rollback/recovery behavior.
- Add dry-run plan rendering.
- Add explicit migration for verified Tome-managed target symlinks.
- Preserve foreign and drifted targets.

**Acceptance:** deleting/moving the canonical library after a successful deployment does not break a target; target edits are detected as drift; unowned files are untouched; interrupted refresh does not leave a partial active target.

### Phase C — status, doctor, cleanup and explicit refresh

**Outcome:** users can understand and safely reconcile target state.

- Implement the deployment state machine in status and doctor.
- Replace symlink-oriented cleanup/eject semantics with ownership-aware copy semantics.
- Add previewed refresh, remove and repair flows.
- Add migration and recovery documentation.

**Acceptance:** every status category has focused tests and clear CLI output; cleanup never removes drifted/foreign target content automatically.

### Phase D — project routes and configuration validation

**Outcome:** safe project-local copied targets without allowing projects to rewrite the global pool.

- Define the minimal project `.tome.toml` schema.
- Implement nearest-project discovery and boundary-safe path resolution.
- Validate prohibited configuration fields and route conflicts.
- Add project-specific deployment records and tests around Git checkout deletion/reset scenarios.

**Acceptance:** a project can receive an independently usable target copy; a project config cannot add sources or change canonical/ machine-local policy.

### Phase E — curation records and intake

**Outcome:** a growing pool becomes explainable and reviewable.

- Define curation record schema and migration/validation rules.
- Add catalog read/write operations and deterministic views.
- Implement candidate lifecycle and explicit triage outcomes.
- Add provenance merge and same-name/different-content conflict behavior.

**Acceptance:** every canonical skill can state what it is, where it came from, how it is classified, why it is routed or excluded, and what content hash that decision applies to.

### Phase F — overlap/gap, customization and fork workflows

**Outcome:** Tome supports deliberate collection maintenance instead of accumulating folders.

- Implement deterministic overlap and gap reports.
- Add traceable customization and fork records.
- Add upstream-delta reporting where source information permits.

**Acceptance:** suggestions remain advisory; no automated deletion/routing/forking occurs; all lineage is inspectable.

### Phase G — evaluation evidence

**Outcome:** evaluation can inform curation without becoming deployment authority.

- Implement MCO-143 as a separate provider-neutral core.
- Attach optional evaluation evidence references to curation records.
- Keep runners/adapters, model calls and UI expansion separate follow-up work.

## 10. Test strategy

Use unit, integration and end-to-end filesystem tests. Required scenarios include:

- canonical copy and target copy content equality;
- target independence after canonical path deletion/relocation;
- copy update with atomic replacement and simulated failure recovery;
- healthy target, canonical-updated, drifted, missing, foreign and stale-record states;
- symlink migration: valid Tome-managed, broken Tome-managed, foreign and redirecting cases;
- safe cleanup/removal with and without target drift;
- project route boundary and configuration restrictions;
- same-name/different-content intake conflict and equal-content provenance merge;
- deterministic curation catalog serialization/validation;
- CLI status/doctor output and TUI state mapping;
- no network clone/fetch in list/browse/status;
- Git cache root / `.git` file / `.git` symlink rejection regression tests.

Run formatting, clippy, focused tests, full `cargo test -p tome`, doc build where available, and `git diff --check` for every implementation PR. Add platform coverage for macOS and Linux where filesystem behavior can differ.

## 11. Risks and mitigations

| Risk | Mitigation |
| --- | --- |
| Copy deployment can overwrite user/project work. | External deployment records, hash-based drift detection, preview-first mutation, explicit force/repair only. |
| Migration mistakes classify foreign links as Tome-owned. | Require strong ownership proof and preserve uncertain paths. |
| Staging/rename differs across filesystems. | Stage beside destination where possible; detect cross-device cases and fail safely rather than partially replacing. |
| Curation metadata becomes a second hidden source of truth. | Bind every record to canonical hash; require explicit reassessment when content changes. |
| Configuration grows into competing policy layers. | Keep current shared/local split; restrict project config; defer named profiles until evidence requires them. |
| Evaluation becomes an automatic quality or security claim. | Keep evaluation optional, isolated, diagnostic and non-authoritative. |
| Desktop work pulls effort away from dependable core behavior. | Treat CLI/TUI core as the only active product surface; desktop remains paused. |

## 12. Paperclip work breakdown

Paperclip remains authoritative. Convert these phases into individually testable issues rather than one long-lived implementation ticket:

1. Close the MCO-52 reliability lane and linked source tests.
2. Create copy-deployment engine and deployment-state issue.
3. Create target migration/doctor/cleanup issue.
4. Create safe project-route issue.
5. Create curation record and intake issue.
6. Create overlap/gap and customization/fork issue.
7. Keep MCO-143 as the later evaluation-evidence issue.

Each issue must state its acceptance tests, migration/safety boundary, affected paths, and whether it changes a user-visible configuration contract. Desktop/Tauri work stays paused and must not be pulled into these issues.
