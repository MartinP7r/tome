---
name: using-tome
description: Use when setting up Tome, adding skill repositories or local skill directories, syncing skills across AI tools, changing Tome directory roles or machine preferences, diagnosing missing skills or failed syncs, or recovering a Tome-managed skill library.
---

# Using Tome

## Operating Contract

- Inspect state with `tome status`, `tome config`, or `tome doctor` before attempting a repair.
- Run `tome <command> --help` before version-sensitive invocations; trust the installed CLI over remembered syntax.
- Use `--dry-run` before supported mutating commands. Confirm support in that command's help.
- Do not hand-edit `.tome-manifest.json` or `tome.lock` as a first response.
- Preserve library content. Create a backup snapshot before destructive recovery.

## Choose The Workflow

- Read [configuration](references/configuration.md) for setup, directory roles, Git sources, or machine preferences.
- Read [operations](references/operations.md) for ordinary adds, syncs, removals, backups, and library moves.
- Read [troubleshooting](references/troubleshooting.md) for missing skills, unexpected state, failed syncs, or recovery.

## Safe Starting Sequence

Start by selecting the checks relevant to the request:

```bash
tome status
tome config
tome doctor
tome <command> --help
```

Do not run all four mechanically for every request. Use status for topology and health, config for effective settings, doctor for damage, and help before constructing a version-sensitive or mutating command.

## Role Decision

- Use `managed` for read-only directories owned by a package manager. Tome discovers their skills but does not write back.
- Use `source` for generic read-only discovery directories.
- Use `target` for distribution only; Tome does not discover skills there.
- Use `synced` only when both discovery and distribution write-back are intended.

Never rely on the local `type = "directory"` default when write-back is unwanted: it defaults to `synced`. For example, add a package-manager directory with `tome add ~/.pfw/skills --role managed`.

## Command Map

- `tome init`: run interactive initial setup.
- `tome add`: register a local or Git skill directory.
- `tome sync`: reconcile managed state, discover, consolidate, distribute, clean up, and save.
- `tome status`, `tome list`, `tome browse`: inspect topology, skills, and library contents.
- `tome doctor`: diagnose health issues and perform supported repairs.
- `tome remove`, `tome reassign`, `tome fork`: remove directories or Unowned skills, change ownership, or make an editable local copy.
- `tome backup`: initialize, snapshot, inspect, or restore Git-backed library backups.
- `tome relocate`: move the library and update distributions safely.
- `tome eject`: remove Tome-managed target symlinks; a later sync recreates them.
- `tome lint`: validate `SKILL.md` frontmatter and structure.

## Resources

- [Configuration and roles](references/configuration.md)
- [Safe operations](references/operations.md)
- [Diagnosis and recovery](references/troubleshooting.md)
