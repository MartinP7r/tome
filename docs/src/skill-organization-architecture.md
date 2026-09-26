# Skill-organization architecture

> **Status:** Recommended direction from [MCO-144](https://mmini.zuul-bee.ts.net:8443/MCO/issues/MCO-144). This document is a decision and execution guide, not a claim that every proposed capability is already shipped.

## Goal

Tome should organize Martin's AI-agent skills across machines and projects without losing provenance, silently changing canonical content, or installing a skill where it does not belong.

The system has four separate questions:

1. **Pool:** Which skills exist in the canonical collection, and where did they come from?
2. **Curation:** What domain(s), capabilities, constraints, overlaps, gaps, and local customizations apply to each skill?
3. **Deployment:** Which machine and project destinations should receive a curated skill?
4. **Evidence:** Is a skill structurally valid and does it demonstrably improve a task?

Keeping these questions separate is the architectural guardrail. Evaluation is important evidence for curation, but it is not the product's primary purpose.

## Current state and evidence

Tome already implements most of the right storage boundary:

- `library.rs` stores both managed and local skills as real directory copies. The manifest's `managed` field indicates the update channel; it is not a choice to leave canonical content in a package-manager cache.
- `distribute.rs` creates target-side symlinks to the real library entries and protects foreign links from accidental replacement.
- `profiles.rs` and the cross-machine configuration document define shared pool policy, committed named machine profiles, and local runtime settings.
- `machine.rs` preserves a compatibility model for local disable lists, directory filters, path overrides, and install consent.
- The current cross-machine flow treats shared tags as routing inputs, keeps newly discovered skills library-only until classified, and permits project `.tome.toml` files to add destinations without changing shared sources or selecting a profile.

The remaining work is to make those boundaries consistently visible and useful to a person curating a growing skill collection.

## Decision: copy the canonical pool and every target

**Adopt copy materialization as Tome's default and supported deployment model.**

The canonical pool is the place where Tome owns and curates skills; targets are self-contained deployments. A target must continue working if Tome is uninstalled, unavailable, misconfigured, moved, or simply not run again.

| Layer | Storage rule | Why |
|---|---|---|
| Source/package-manager cache | Read-only input | A cache or installed plugin can change or disappear independently of Tome. |
| Canonical pool (`skills/`) | Real directory copies | Content can be reviewed, hashed, committed, backed up, migrated, and used on another machine. |
| Tool target directory | Real directory copy from the canonical pool | The tool remains self-contained and usable if the pool or Tome executable disappears. |
| Project target directory | Real directory copy from the canonical pool | Git resets, project tooling, deletion of the checkout, or removal of Tome cannot dereference, break, or mutate the canonical pool. |

Do **not** reintroduce source/cache symlinks inside the canonical pool. Do not use target symlinks as normal materialization either. Symlinks economize on local copies but couple every target to Tome's continued path correctness and turn ordinary target-file writes into a possible canonical-pool mutation.

A target copy is a derived deployment, not a second source of truth. Its deployment record must identify the canonical hash, target path, materialization mode `copy`, and last successful sync. A later `tome sync` can report drift and offer an explicit, previewed refresh; it must never silently treat a changed target copy as canonical content.

## State model

### 1. Shared pool: committed and reviewable

The shared repository owns:

- canonical skill directories;
- pool-wide source policy, exclusions, conflict resolutions, and source pins;
- immutable content hashes and complete provenance observations;
- shared curation metadata: domains, capabilities, tool constraints, maturity, licensing/attribution notes, and customization/fork relationships;
- named machine profiles and project-safe route policy; and
- the lockfile/catalog needed to reproduce or reconcile source state.

A same-name/different-content discovery must stop before it changes the pool. Equal content from multiple sources merges provenance rather than selecting an invisible winner.

### 2. Machine profile: committed, named topology

A committed profile declares a machine role and its known target topology: paths, target capabilities, preferred materialization mode, route predicates, and explicit exclusions. Profiles are selected by name, never guessed from hostname.

A profile answers *where this machine can receive skills*. It must not redefine canonical content or silently add a competing source policy.

### 3. Machine-local settings: private runtime choice

Local settings select one committed profile and hold facts/consents that should not be committed: Git synchronization policy, native-plugin installation permission, backup behavior, credentials, and temporary local overrides.

The status surface should distinguish at least: configured-but-unavailable destination, installed/healthy materialization, drifted/missing materialization, disabled-by-profile, disabled-locally, and blocked-by-capability.

### 4. Project layer: additive destinations only

A nearest project `.tome.toml` may add routes and destinations under that project. It may not add global sources, replace pool policy, mutate canonical curation, or select a machine profile. This prevents a repository checkout from unexpectedly changing the shared collection.

## Curation model

Curation is a deterministic, reviewable layer between intake and deployment.

### Skill record

Each canonical skill needs a curation record with at least:

- stable skill ID/name and canonical content hash;
- source provenance and upstream relationship;
- domains and capabilities (many-to-many rather than one category);
- supported targets/constraints and incompatible targets;
- lifecycle state: `candidate`, `accepted`, `routed`, `excluded`, `customized`, `forked`, or `deprecated`;
- relationship links: complements, overlaps, supersedes, derives-from, and conflicts-with; and
- explicit evidence references: lint result, review notes, test/evaluation suite/result, and attribution/license notes where applicable.

Tags alone are useful routing primitives, but they are insufficient as the entire curation model: a tag cannot explain overlap, a fork's upstream, or why a candidate is excluded.

### Intake workflow

1. **Discover/import** a candidate and record its provenance and immutable hash.
2. **Validate structurally**: package layout, frontmatter, path safety, declared requirements, and target compatibility.
3. **Classify** by domain/capability and compare it against the existing catalog.
4. **Triage** it explicitly as accept, accept-but-library-only, route, customize, fork, reject/exclude, or investigate further.
5. **Deploy** only after a profile/project route makes the decision explicit.
6. **Reassess** on content/hash or source-provenance changes; do not silently inherit old acceptance for changed content.

Overlap analysis should first be explainable and deterministic: shared domain/capability labels, common target compatibility, same upstream, high textual/frontmatter similarity, and shared evaluation suites. A semantic similarity service can be added later as advisory evidence, never as an automatic deletion or routing decision.

### Customization and forking

Use customization when a small, maintained overlay can be traced to a stable upstream base and the delta is reviewable. Use a fork when the behavior or ownership purposefully diverges, when the overlay cannot be applied safely, or when the source is no longer suitable as an update channel. Both must retain `derived_from` provenance and a base hash; neither should overwrite an upstream snapshot in place.

## Validation and evaluation

Validation and evaluation belong in the curation layer.

- **Validation** is deterministic and cheap: structural parsing, frontmatter/schema checks, identifier/path safety, capability declarations, provenance consistency, and route/materialization constraints. It runs at intake, before deployment, and in CI where appropriate.
- **Evaluation** measures whether an accepted artifact improves a task. It must be isolated from the canonical library, use hash-addressed artifacts, preserve terminal and `not_comparable` states, and compare matched treatment/control runs only when score-bearing evidence is comparable.

The provider-neutral core in [MCO-143](https://mmini.zuul-bee.ts.net:8443/MCO/issues/MCO-143) remains valuable, but it is a **next** capability—not the current lead lane. It should remain a library-domain module with authored portable suites and fixtures in the shared repository, while generated result documents, traces, temporary workspaces, credentials, and provider/model cost data remain machine/CI-local.

Evaluation evidence may inform a curation decision. It must not automatically route a skill, remove an overlapping skill, or declare a fork safe.

## Phased roadmap

### Now: reliability and accurate inventory

1. Fix Git source discovery and prove it with end-to-end coverage.
2. Fix stale target links during role transitions and retain foreign-link protection.
3. Make the source/configuration type boundary explicit enough that URLs are not treated as filesystem paths.
4. Publish an accurate status/doctor inventory for pool, selected profile, project routes, materialization mode, and divergence.

### Next: curation and controlled deployment

1. Add the curation-record contract and deterministic catalog views: domains, capabilities, lifecycle, provenance, overlap candidates, and gap reports.
2. Add explicit target deployment records for copy materialization, including canonical hash, target path, last successful sync, and previewed drift refresh.
3. Add project route inspection and selection on top of the existing profile boundary.
4. Implement the provider-neutral evaluation core as curation evidence, followed by a separate runner-adapter spike.

### Later: advisory intelligence and ecosystem expansion

1. Semantic overlap/gap suggestions with human approval.
2. Customization-overlay and fork workflows with upstream-delta reporting.
3. New ecosystems and marketplaces only after generic source discovery, provenance, and materialization are reliable.
4. Desktop/Tauri work only after an explicit reprioritization; it must render the same pool/profile/curation model rather than create GUI-only state.

## Safety constraints

- No source disappearance may delete canonical pool content without an explicit pool removal decision.
- No same-name/different-content candidate may overwrite content or provenance silently.
- Never delete or replace a foreign target link/file without an explicit force/repair decision.
- Every deployed target copy must be identifiable and repairable from its canonical hash, but a changed target copy is never canonical input.
- Project configuration cannot mutate the shared pool.
- Provider credentials, raw evaluation traces, and machine-specific consent do not enter shared Git state.
- A suggestion engine may prioritize review but cannot enact acceptance, routing, customization, forking, or deletion on its own.

## Paperclip triage

Paperclip is the authoritative work queue. The immediate lane is source/distribution correctness:

1. MCO-52 — Git source discovery bug.
2. Imported GitHub #422 and #436 — end-to-end Git source and clone/update coverage.
3. Imported GitHub #548 — remove orphaned target links on role transitions.
4. Imported GitHub #424 — separate URL source identity from filesystem path after behavioral repairs are protected.

Keep MCO-143 / GitHub #604 as the next curation-evidence task. Keep desktop/Tauri issue MCO-141 / GitHub #581 paused. Re-triage older aggregate review issues into individually reproducible work before scheduling them.
