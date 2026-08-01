# Tome Troubleshooting

## Diagnosis Order

1. Capture `tome status` and `tome doctor` before changing anything.
2. Inspect effective directory topology and resolved path overrides with `tome config`; inspect `machine.toml` for disable filters and plugin-install consent.
3. Reproduce safely with `tome sync --dry-run --verbose`. If that is insufficient, rerun with `TOME_LOG=tome::sync=debug`.
4. Classify the issue as a missing source path, wrong directory role, missing Git `subdir`, machine disable/filter, Unowned ownership, foreign target entry, or failed managed-plugin reconciliation.
5. Apply the narrow Tome command, rerun `tome sync --dry-run` and `tome sync`, then verify with `tome status` and `tome doctor`.

Do not begin by editing `.tome-manifest.json`, `tome.lock`, or target symlinks.

## Zero Skills From A Git Repository

Check whether skills are nested under `skills/`. Confirm with `tome add --help`, then register the repository with an explicit subdirectory and source role:

```bash
tome add MartinP7r/tome --subdir skills --role source --dry-run
tome add MartinP7r/tome --subdir skills --role source
tome sync --dry-run
tome sync
tome status
```

If the directory is already registered incorrectly, inspect its configured name, preserve library content, and use the supported remove-and-add workflow rather than patching generated state.

## Package-Manager Write-Back Pollution

Stop before cleanup. Change the local package-manager directory from the default `synced` role to `managed` in the portable config so Tome cannot write back again. Confirm effective config, preview sync, and only then use `tome doctor` or sync's supported cleanup. Back up first if any real directory could be removed.

## Broken Symlinks Or Foreign Target Entries

Run `tome doctor --dry-run`, inspect its classification, then run `tome doctor` for supported repairs. Diverging real directories in targets require a deliberate content decision; do not overwrite them blindly.

## Unowned Skills

Preserve the library copy. Use `tome reassign <skill> --to <directory> --dry-run` to attach it to a configured owner, then apply and sync. Delete only after confirming Unowned status with `tome remove skill <skill> --dry-run` followed by the live command.

## Failed Managed-Plugin Reconciliation

Inspect status, lockfile-reported drift, `auto_install_plugins`, and whether the upstream package-manager CLI is available. Use `tome sync --no-install --dry-run --verbose` to inspect without installing. Correct machine consent or the upstream installation, then rerun sync; do not rewrite lockfile entries to hide drift.

## Damaged State

Inspect `tome backup list` and `tome backup diff`, then restore a known-good snapshot before attempting manual reconstruction. Direct edits to generated manifest or lockfile data require a separately justified, backed-up, last-resort procedure after normal status, doctor, config, sync, and restore paths have failed.
