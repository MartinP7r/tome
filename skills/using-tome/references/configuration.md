# Tome Configuration

## Tome 0.17.0 Configuration Layers

Tome 0.17.0 separates shared repository policy, named machine profiles,
project-local destinations, and settings local to each machine:

| Layer | Default path | Contents |
|---|---|---|
| Repository policy | `~/.tome/tome.toml` | Library path, shared Git sources, shared exclusions, backup policy, source pins |
| Machine profile | `~/.tome/machines/<profile>.toml` | Local sources, destinations, and destination routes for that profile |
| Project | Nearest ancestor `.tome.toml` | Additive project-only destinations and routes |
| Local settings | `~/.config/tome/settings.toml` | Active profile and local runtime consent |

Profile files are shared definitions, normally version-controlled with the
Tome repository. Each machine chooses its active profile locally in
`settings.toml`. Profile names are user-defined; Tome does not infer one from
the hostname. Use different profiles when local paths or routes differ.

Released CLI commands do not read or write the legacy `machine.toml`, and there
is no global `--machine` option. Use `--config`, `--settings`, or `--tome-home`
when the defaults do not fit.

## Manual Upgrade From 0.16.4

There is no general automatic migration for the old flat configuration. Back
up the Tome home and local settings before editing. Move shared Git sources and
repository-policy fields into `tome.toml`; move ordinary local directories,
resolved path overrides, and destinations/routes into the selected profile;
put the selected profile and local consent in `settings.toml`.

The old `machine.toml` preferences require deliberate conversion:

- Per-destination enabled lists become skill tags plus `[routes.<destination>].tags`.
- Per-destination disabled lists become `[routes.<destination>].exclude`.
- A global disabled list has no exact one-field equivalent; do not guess.
- Disabled directories have no disabled flag in profiles; omit them from the
  active profile or use a separate profile only after confirming the intent.
- Move local consent to `settings.toml`, using `managed_plugin_install`,
  `git_sync`, and `backup_runtime` as applicable.

For complete migration guidance, consult `docs/src/upgrade-v0.17.md` in the Tome
repository. `tome migrate-library` is not a configuration migrator and is not
shipped in 0.17.0; its legacy pre-0.10 library conversion had to be run on
0.16.4 before upgrading.

After converting and reviewing the files, verify with `tome profile list`,
`tome status`, and `tome sync --dry-run --no-input --no-install`. Review the
preview before any real sync.

## Directory Roles

- `managed`: discover from a read-only upstream owned by a package manager; do
  not distribute back into it.
- `source`: discover from a local or Git directory without distributing into it.
- `target`: distribute to the directory without discovering skills there.
- `synced`: both discover from and distribute into the directory; use only when
  write-back is intended.

Check `tome add --help` before version-sensitive commands and specify roles
explicitly when write-back is not intended.

## Shared Git Sources

Git discovery sources belong in shared `tome.toml` and use `url`, not a
remote-valued `path`:

```toml
[directories.team-skills]
url = "https://github.com/myorg/team-skills.git"
type = "git"
branch = "main"
subdir = "skills"
```

Keep `branch`, `tag`, `rev`, and `subdir` as applicable. Local filesystem
sources continue to use `path` in the selected profile.

## Tags And Routes

Tags are stored with skills in `.tome-manifest.json`. A destination route uses
OR matching for `tags`; `exclude` overrides a match:

```toml
[routes.codex]
tags = ["portable", "coding"]
exclude = ["claude-only-skill"]
```

Project `.tome.toml` files may add project-only destinations and routes; they
cannot replace shared policy or select the machine profile.

## Consent And Safe Changes

`~/.config/tome/settings.toml` keeps local runtime choices such as
`managed_plugin_install`, `git_sync`, and `backup_runtime`. Use
`tome sync --no-install` for a one-run managed-plugin-install override.

Before changing an existing setup, inspect the installed CLI's help and back up
the relevant config. If `tome status` reports legacy flat configuration, follow
the manual upgrade guide rather than trying `tome init` or an unsupported
migration command. Validate the new files and review a dry-run before applying
sync changes.
