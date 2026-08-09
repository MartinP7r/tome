# Tome Operations

## Initial Setup

1. Run `tome init`.
2. Inspect the result with `tome status`.
3. Check health with `tome doctor`.

## Routine Sync

1. Capture current state with `tome status`.
2. Preview with `tome sync --dry-run`.
3. Apply with `tome sync`.
4. Verify with `tome status`.

`tome sync` is the normal reconciliation operation: it reconciles managed state, discovers skills, consolidates them into the library, distributes them to targets, cleans up stale state, and saves generated state. Do not treat direct target symlink edits as a normal operation.

## Add A Git Source

1. Confirm flags with `tome add --help`.
2. Preview `tome add <repo> [--subdir <path>] --role source --dry-run`.
3. Repeat the command without `--dry-run`.
4. Run `tome sync --dry-run`, then `tome sync`.
5. Verify with `tome status`.

## Remove A Directory

1. Inspect its name and ownership with `tome status` and `tome list`.
2. Preview `tome remove dir <name> --dry-run`.
3. Repeat without `--dry-run`.
4. Verify that preserved skills became Unowned as expected.

## Delete An Unowned Skill

1. Verify the skill is Unowned with `tome list` or `tome status`.
2. Preview `tome remove skill <name> --dry-run`.
3. Repeat without `--dry-run` only after confirming deletion is intended.

## Reassign Or Fork

1. Inspect current ownership and the destination.
2. Run `tome reassign --help` or `tome fork --help`.
3. Preview the chosen command with `--dry-run`.
4. Apply it without `--dry-run`.
5. Run `tome sync` and verify state.

## Backup Or Restore

1. Run `tome backup --help` and the selected subcommand's help.
2. Initialize backup tracking if needed with `tome backup init`.
3. Run `tome backup snapshot -m "before <operation>"` before risky work.
4. Inspect `tome backup list` or `tome backup diff` before choosing a restore reference.

## Relocate Or Eject

1. Capture `tome status` and create a backup snapshot.
2. Run `tome relocate --help` or `tome eject --help`.
3. Preview the command with `--dry-run`.
4. Apply it without `--dry-run`.
5. Verify with `tome status` and `tome doctor`.

Global `--dry-run` is currently available broadly, but do not assume support across Tome versions. Confirm it in the selected command's help before presenting a preview command.
