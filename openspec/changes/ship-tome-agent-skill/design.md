## Context

Tome discovers skill directories containing `SKILL.md`, including Git repositories scoped by `subdir`. The setup wizard assembles a `BTreeMap<DirectoryName, DirectoryConfig>`, saves through `Config::save_checked`, and immediately runs sync after interactive initialization. Claude Code independently discovers root `skills/` directories in plugins and supports repository-hosted marketplace manifests.

The change must provide one canonical skill package for both ecosystems, preserve noninteractive initialization, and avoid teaching agents behavior that can drift from the installed CLI.

## Goals / Non-Goals

**Goals:**

- Ship one progressively disclosed skill for safe Tome user operations.
- Make the same files installable through Tome Git discovery and Claude's plugin marketplace.
- Add a default-selected, clearly disclosed recommendation to interactive init.
- Verify skill behavior, manifest validity, wizard insertion, duplicate suppression, and noninteractive preservation.

**Non-Goals:**

- Contributor, release, or GSD workflow guidance.
- Multiple narrowly scoped Tome skills.
- Automatic Git `skills/` subdirectory adoption.
- New config schema, Rust dependency, or noninteractive network behavior.

## Decisions

### Use one root `skills/using-tome` package

Root `skills/` is both a conventional skill repository layout and Claude's default plugin discovery path. A single skill with focused references avoids duplicated packages and lets Tome distribute it to every configured target. Project-local `.agents/skills` was rejected because it is repository configuration rather than a user-installable skill source.

### Keep exact command syntax at the CLI boundary

The skill defines decision order and safety invariants, then directs agents to `tome <command> --help` for exact flags. Copying the complete command reference into `SKILL.md` was rejected because it would bloat context and drift across Tome releases.

### Wrap the repository as a Claude plugin marketplace

Use `.claude-plugin/plugin.json` and `.claude-plugin/marketplace.json`, with plugin and marketplace name `tome`. Plugin version `1.0.0` evolves independently from the binary. The marketplace references the repository root so Claude discovers the same `skills/` package without a second copy.

### Insert the recommendation into wizard state

Accepting the prompt inserts a Git/source entry named `tome-skills`, URL `https://github.com/MartinP7r/tome`, and `subdir = "skills"`. The wizard does not recursively invoke `tome add`; existing validation, checked save, and post-init sync remain the only persistence and clone paths.

### Gate all recommendation behavior on interactive mode

The prompt defaults to yes but runs only when `no_input` is false. Existing equivalent sources and `tome-skills` name collisions suppress insertion without overwriting configuration. This keeps scripts and CI network-source-free while making the skill prominent for people using the wizard.

## Risks / Trade-offs

- [Plugin source is the repository root and includes unrelated project files] -> Keep one canonical package and validate the real installation; revisit a dedicated plugin subdirectory only if cache size becomes material.
- [Skill instructions drift from CLI behavior] -> Keep workflow guidance stable and require command-help inspection for exact syntax.
- [Default-yes prompt causes an unexpected clone] -> State the equivalent command and immediate post-init clone before confirmation; skip noninteractive mode.
- [Existing configuration is overwritten or duplicated] -> Use pure detection/insertion helpers and test equivalent-source and name-collision cases.
- [Agent guidance looks correct but does not change behavior] -> Run RED baseline scenarios before authoring and the same GREEN scenarios after authoring.

## Migration Plan

No user-data migration is required. Existing users can add the Git source or Claude marketplace manually. New interactive init sessions receive the recommendation; noninteractive sessions remain unchanged. Rollback removes the prompt and package metadata without affecting existing library copies, which remain normal Tome-managed skills.

## Open Questions

None.
