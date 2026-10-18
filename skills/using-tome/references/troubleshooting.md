# Tome Troubleshooting

## Diagnosis Order

1. Record `tome --version`, the active binary path, and the exact error.
2. Inspect the effective config path (`tome config --path`), selected profile
   (`tome profile list`), and the relevant repository, profile, project, and
   local-settings files. Redact credentials from any report.
3. If `tome status` reports legacy flat configuration after upgrading to 0.17.0,
   stop before running `sync` or `init`. Back up the old files and follow
   `docs/src/upgrade-v0.17.md`; conversion is manual.
4. Once `tome status` succeeds, reproduce sync problems with
   `tome sync --dry-run --no-input --no-install --verbose` and inspect the plan.
5. Apply only the narrow, understood repair. Run a real sync only after the
   preview is understood and the user has authorized the resulting changes.

Do not begin by editing `.tome-manifest.json`, `tome.lock`, or target contents.

## Legacy Flat Configuration After Upgrade

Tome 0.17.0 uses shared `tome.toml`, a selected `machines/<profile>.toml`, an
optional project `.tome.toml`, and local `settings.toml`. There is no general
automatic flat-config migration. `tome migrate-library` is not a config
migration command and is not available in 0.17.0; it was a separate pre-0.10
library conversion that had to be run on 0.16.4 before upgrading.

Back up first. Keep shared Git sources and repository policy in `tome.toml`;
move local directories, resolved machine-specific paths, and routes to the
selected profile; keep profile selection and local consent in `settings.toml`.
For ambiguous old global disabled lists or disabled directories, do not guess a
replacement. See the full upgrade guide before changing files.

After conversion, verify with `tome profile list` and `tome status`. Then inspect
`tome sync --dry-run --no-input --no-install`; do not apply it automatically.

## Zero Skills From A Git Repository

Check whether skills are nested under `skills/`. Confirm supported syntax with
`tome add --help`, then register the repository with an explicit subdirectory
and source role:

```bash
tome add MartinP7r/tome --subdir skills --role source --dry-run
```

Inspect `tome status` and a sync dry-run before applying a real sync. Do not
assume a failed or empty preview is safe to apply.

## Package-Manager Write-Back Pollution

Stop before cleanup. Change the package-manager directory to role `managed` in
the selected profile so Tome does not distribute back into it. Verify with
`tome status` and a dry-run. Do not use cleanup or a real sync until the exact
effects are understood.

## Broken Legacy Links Or Foreign Target Entries

Version 0.17.0 creates independent copies only at absent target paths; it does
not overwrite occupied paths. Run `tome doctor --dry-run` and inspect the
classification before requesting or applying a supported repair. Foreign or
ambiguous entries require a deliberate ownership decision.

## Unowned Skills

Preserve the library copy. Use `tome reassign <skill> --to <directory> --dry-run`
to preview re-anchoring it to a configured owner. Apply only after confirming
the preview. Delete only after confirming Unowned status and reviewing
`tome remove skill <skill> --dry-run`.

## Failed Managed-Plugin Reconciliation

Inspect status, lockfile-reported drift, the local `managed_plugin_install`
setting, and whether the upstream package-manager CLI is available. Use
`tome sync --no-install --dry-run --verbose` to inspect without installing. Do
not rewrite lockfile entries to hide drift; apply a real sync only after
reviewing its effects.

## Damaged State

Inspect `tome backup list` and `tome backup diff` before considering restore.
Preserve a copy of the current state before restoring a backup. Direct edits to
generated manifest or lockfile data are a last resort after normal status,
doctor, config, sync-preview, and restore paths have failed.
