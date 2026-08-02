# Tome Agent Skill Design

**Date:** 2026-08-01
**Status:** Approved for implementation planning
**Source:** `.planning/todos/pending/2026-07-15-define-agent-skills-for-tome.md`

## Problem

Tome documents its commands and configuration for humans, but it does not ship
an agent skill that teaches coding agents how to operate an installed Tome
library safely. Agents must rediscover directory-role semantics, safe sync and
repair order, and nested skill-repository installation from general project
documentation. This increases the risk of selecting a write-back role for a
read-only source, mutating generated state directly, or reaching for a
destructive recovery step before using Tome's diagnostics.

The repository also lacks Claude plugin and marketplace metadata, so its own
agent skill cannot be installed through Claude's normal plugin workflow.

## Scope

Ship one cohesive, user-facing `using-tome` skill. Cover setup, configuration,
daily operations, diagnosis, and recovery for users operating Tome. Keep
contributor development, GSD, release, and repository-maintenance workflows in
the existing repository instructions.

Add an interactive `tome init` recommendation that can register the official
skill as a Git source. Package the same skill as a Claude plugin and marketplace
entry so users can choose either installation route.

Make the already-documented local-path form of `tome add` real so agents can
add package-manager-owned directories with an explicit safe role instead of
being taught a command the CLI rejects.

## Skill Package

Store the skill in the repository's standard root-level skill directory:

```text
skills/
└── using-tome/
    ├── SKILL.md
    └── references/
        ├── configuration.md
        ├── operations.md
        └── troubleshooting.md
```

`SKILL.md` contains trigger metadata, core safety rules, a concise decision
flow, and a command map. Detailed guidance uses progressive disclosure:

- `configuration.md` explains directory types and roles, Git sources,
  machine preferences, path overrides, and safe defaults.
- `operations.md` covers initialization, dry-run-first sync, inspection,
  adding and removing directories, reassigning and forking skills, backup,
  relocation, and eject.
- `troubleshooting.md` defines diagnosis order, verbose logging, doctor usage,
  failure recovery, and generated files that should not be edited as an
  initial repair technique.

Do not add scripts or assets. Exact flags remain authoritative in the installed
binary; instruct agents to consult `tome <command> --help` before constructing
version-sensitive invocations.

## Skill Behavior

Trigger the skill for requests such as:

- "Set up Tome."
- "Add this skill repository to Tome."
- "Sync my skills across Claude Code and Codex."
- "Why is Tome not finding this skill?"
- "Repair my Tome configuration."

Guide agents to:

1. Inspect current state before changing it with `tome status`, `tome config`,
   or `tome doctor` as appropriate.
2. Use `--dry-run` before mutating operations where the command supports it.
3. Distinguish `managed`, `source`, `target`, and `synced`, explicitly warning
   that a plain directory defaults to `synced` and can receive write-back
   symlinks.
4. Prefer `managed` for package-manager-owned directories and `source` for
   generic read-only discovery directories.
5. Use Tome commands rather than manually editing the manifest or lockfile as
   the first repair response.
6. Preserve user data and use backup or plan/preview flows before destructive
   operations.

## Local Path Support In `tome add`

Classify explicitly path-shaped inputs as local directories before Git URL
parsing: absolute paths; `~` and `~/...`; and `.`, `..`, `./...`, or `../...`.
Continue treating URLs, SCP-style SSH inputs, bare `owner/repo` slugs, GitHub
`/tree/<ref>/<subdir>` forms, and other legacy inputs as Git sources. Do not
classify by filesystem existence because that would make `owner/repo` behavior
depend on the current working directory.

For a local path, construct `DirectoryType::Directory`, allow every role from
`DirectoryType::Directory::valid_roles()`, and default to `synced` when no role
is supplied. Derive the default name from the final path component and preserve
`--name` overrides. Reject Git-only `--branch`, `--tag`, `--rev`, and `--subdir`
flags with actionable errors. Save through `Config::save_checked` so validation,
atomic write, round-trip checks, and portable `~/...` serialization all apply.
Keep existing Git add parsing, role validation, ref/subdirectory precedence,
and success output unchanged.

## Desktop Data Folder Label

Extend `StatusReport` with the canonical `TomePaths::tome_home()` value rather
than deriving it from `library_dir`. In the desktop status view, label this row
`TOME DATA FOLDER` and describe it as the portable Tome root, explicitly noting
that machine settings live under `~/.config/tome`. Keep `LIBRARY` as a separate
row sourced from `library_dir`.

Remove the client-side parent-directory heuristic. A custom library may be
outside Tome home, and the default library ends in `/skills`, so derivation is
both conceptually ambiguous and technically incorrect.

## Claude Plugin And Marketplace

Add the standard Claude plugin metadata at the repository root:

```text
.claude-plugin/
├── plugin.json
└── marketplace.json
```

The root repository is the plugin source and its `skills/` directory is
auto-discovered by Claude Code. Name both the plugin and marketplace `tome`.
Use a plugin version independent of the Tome binary, starting at `1.0.0`, and
bump it only when plugin content changes.

Document this installation route:

```bash
claude plugin marketplace add MartinP7r/tome
claude plugin install tome@tome
```

Also document the cross-tool Tome route:

```bash
tome add MartinP7r/tome --subdir skills --role source
tome sync
```

## Init Recommendation

During interactive `tome init`, offer:

```text
Add Tome's official agent skills? [Y/n]
```

The prompt is optional and defaults to yes. Explain before confirmation that
accepting registers a Git source and that the post-init sync clones the
repository immediately. Show the equivalent standalone command:

```bash
tome add MartinP7r/tome --subdir skills --role source
```

On acceptance, add a validated `tome-skills` directory entry equivalent to:

```toml
[directories.tome-skills]
path = "https://github.com/MartinP7r/tome"
type = "git"
role = "source"
subdir = "skills"
```

Construct the entry inside the wizard's config assembly path. Do not spawn the
current executable recursively. Route the completed configuration through the
existing validation and checked-save pipeline.

Skip the recommendation under `tome init --no-input` so automation retains its
current network behavior. When editing an existing configuration, suppress the
recommendation if an equivalent Tome repository source is already configured;
never add a duplicate entry or overwrite a differently configured
`tome-skills` entry.

Move the Step 0 custom data-folder selection before machine-state detection.
Label it `Tome data folder` and explain that it is the portable root, while
`~/.config/tome` contains machine-local settings and an optional pointer. If the
selected folder contains `.tome/tome.toml` (or root `tome.toml`), run the normal
brownfield Use existing / Edit / Reinitialize / Cancel flow against that config.
Thread the selected folder unchanged through wizard save and post-init sync.
Do not retain the initially resolved default after the user selects a custom
folder.

## Documentation

Add a concise "Agent skill" installation section to the README and relevant
user documentation. Present the Tome Git-source route first because it makes
the skill available to every configured target tool. Present the Claude plugin
route as an alternative for Claude Code users.

## Verification

Develop the skill with baseline and post-skill agent scenarios:

1. Configure a package-manager-owned source without selecting a write-back
   role.
2. Install a repository whose skills live under `skills/`.
3. Diagnose an incomplete sync without destructive manual state edits.

Record the baseline behavior before adding `SKILL.md`, then rerun the same
scenarios with the skill available and verify the expected decisions.

Run these repository checks:

- `tome lint skills/using-tome`
- JSON parsing and Claude plugin/marketplace validation for both manifests
- Focused Rust tests for recommendation insertion and duplicate suppression
- CLI regression coverage proving `tome init --no-input` does not add the
  recommended Git source
- Unit and CLI coverage for local-path classification, managed local entries,
  default roles and names, Git-only flag rejection, portable save, and
  unchanged Git URL/slug behavior
- Rust and React coverage proving the desktop displays canonical Tome data and
  library paths independently with explanatory copy
- Init coverage proving an existing custom data folder is detected before
  configuration and the selected path reaches post-init path construction
- `make ci`

## Deferred Work

Do not implement automatic Git skill-subdirectory selection in this change.
The existing task
`.planning/todos/pending/2026-06-26-tome-add-auto-detect-subdir.md` already
captures auto-detecting and adopting `skills/` after clone. Refine that task if
implementation reveals new acceptance criteria; do not create a duplicate.

## Non-goals

- No contributor, GSD, release, or repository-development skill.
- No multiple narrowly scoped Tome skills in the initial release.
- No scripts bundled with the skill.
- No automatic recommendation under `--no-input`.
- No recursive `tome add` subprocess from `tome init`.
- No automatic `skills/` subdirectory adoption in `tome add`.
- No existence-based inference for ambiguous relative local paths; require an
  explicit `./` or `../` prefix.
- No rename of the internal `tome_home` API or environment variable; this is a
  user-facing copy and canonical-data fix.
