# CLI v1.0 Configuration Design

## Purpose

Ship the full shared-library configuration model in the CLI before resuming the
existing desktop milestone. The desktop phases remain defined in the roadmap,
but Desktop code, bindings, builds, and tests are outside this release scope.
Compatibility exports used only by the paused Desktop may remain until that
milestone resumes.

## Configuration Layers

Tome resolves four layers for each command:

1. Repository policy in the shared Tome repository owns the canonical library,
   shared Git source definitions, shared exclusions, and conflict pins.
2. Named profiles in `machines/<profile>.toml` own machine-wide discovery and
   distribution directories, plus their routing rules.
3. Project configuration in `<project-root>/.tome.toml` adds project-only
   distribution destinations and their routing rules.
4. Local settings in `~/.config/tome/settings.toml` select the active profile
   and hold local runtime consent, such as Git synchronization policy.

Repository policy, the active profile, and local settings are always loaded.
Tome searches upward from the command's working directory for `.tome.toml`.
When found, that project configuration is additive: it may add destinations
and routing rules, but it cannot change repository policy, select a profile,
or alter profile destinations.

The resolved layers are merged into the existing effective configuration so the
current discovery, consolidation, distribution, status, and validation flows
can retain their core interfaces.

## Sources, Tags, and Routing

Git source definitions belong to repository policy. A Git URL, source name,
and optional ref pin are therefore defined once and can be fetched by every
profile and machine.

Ordinary directory sources remain profile-specific because their paths and
tool roles are machine-dependent.

Library membership and distribution are separate. Tome owns a shared tag set
for each individual library skill. Sources supply skills and provenance; they
do not determine distribution. A destination selects one or more tags and
receives a skill when its tag set intersects the destination's selected tags.
An untagged skill remains in the library without being linked into any
destination. A per-destination per-skill exclusion is the only routing
exception.

Profile and project configurations own destination tag selection because they
own their destinations. Repository policy does not automatically route source
contents to every destination. New skills from an upstream source arrive
untagged, so upstream changes cannot silently expand a destination.

## CLI Behavior

`tome add <git-url>` creates a repository-owned Git source. It has no routing
prompt or `--to` flag. After sync imports skills, explicit tag commands assign
shared library tags to individual skills.

Tag commands add, remove, and list skill tags. Route commands select tags for
a destination or exclude one tagged skill from that destination.

`tome sync` resolves all applicable layers, discovers shared Git sources and
profile directory sources, reconciles the library, then distributes only to
destinations whose tag rules select the relevant skills. Git sync continues to
obey local `ask`, `always`, and `never` policy.

### v1.0 Command Surface Cleanup

The v1.0 CLI removes obsolete compatibility surface rather than carrying it
alongside the layered model:

- Remove global `--machine`; released commands never read or write
  `machine.toml`.
- Remove `tome migrate-library`; v0.9-to-v0.10 storage migration is not part of
  the v1.0 product.
- Remove `tome version`; standard `tome --version` remains.
- Remove `tome remove pool`; shared exclusion remains available through
  `tome pool exclude`, with `tome pool restore` as its inverse.
- Legacy TUI and paused Desktop disable actions do not guess a destination or
  persist a global preference. Destination exclusions are managed explicitly
  through `tome route exclude`.

This release does not rename or split otherwise viable commands. A later CLI
UX pass may revisit `tome add`, `tome init`, `tome config`, `tome remove dir`,
`tome profile select`, `tome fork`, `synced`, and the nested route syntax.

## Migration

No migration command or legacy configuration reader ships. At most three
current machines need conversion, so the team will perform and verify a
one-off migration outside the product before release.

That operational conversion creates repository policy and profiles, selects a
profile in local settings on each machine, verifies `tome status` and a dry-run
sync, then removes old files. If existing Git entries disagree on name or ref
pin for the same URL, the conversion stops for an explicit human decision.

## Validation and Error Handling

All persisted TOML writes remain checked and atomic. Tome validates source
identity, destination identity, tag identifiers, destination tag rules, and
per-skill exclusions before writing. Routes cannot reference unknown
destinations.

If Tome discovers a project `.tome.toml` that is invalid, it reports the error
instead of silently falling back to profile-only configuration. Errors identify
the affected layer and file.

## Verification

- A Git source is defined once and usable from multiple profiles.
- Profile destinations receive skills matching any selected tag.
- Project configuration applies only from its project tree and adds, rather
  than replaces, profile destinations.
- Untagged library skills are distributed nowhere.
- One per-skill exclusion overrides a matching tag route.
- Invalid project configuration fails without fallback or partial writes.
- Existing sync and status integration behavior remains covered.
- Top-level help exposes no `--machine`, `migrate-library`, `version`, or
  `remove pool` compatibility surface.
- `tome --version`, `tome pool exclude`, and `tome pool restore` remain.
- No released command reads or writes `machine.toml`.
- `make ci` passes.
