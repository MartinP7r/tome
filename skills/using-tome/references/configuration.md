# Tome Configuration

## Configuration Files

- Portable config: `~/.tome/tome.toml`
- Per-machine preferences: `~/.config/tome/machine.toml`

Keep shared topology in the portable config. Keep machine-specific filtering, consent, and path differences in the per-machine file.

## Directory Roles

- `managed`: discover from a read-only upstream owned by a package manager; mark its skills as managed and never distribute back into it.
- `source`: discover from a generic local or Git directory without distributing into it.
- `target`: distribute library symlinks into the directory without discovering from it.
- `synced`: both discover from and distribute into the directory. Use only when write-back is intentional.

`type = "directory"` defaults to `synced`. Always specify `--role managed` or `--role source` for a read-only local directory so Tome does not pollute it with distribution symlinks.

## Add Directories

Confirm the installed syntax with `tome add --help`, preview each supported mutation, then apply it.

```bash
tome add ~/.pfw/skills --role managed
tome add ~/work/team-skills --role source
tome add MartinP7r/tome --subdir skills --role source
tome add owner/repo/tree/main/skills --role source
```

Use `--subdir skills` when `SKILL.md` directories live below a repository's `skills/` directory. A GitHub `/tree/<ref>/<subdir>` slug can encode the branch and subdirectory instead. `--branch`, `--tag`, and `--rev` are mutually exclusive; confirm their exact flags with `tome add --help`.

## Machine Preferences

Use `disabled` to prevent named skills from being distributed on one machine. Use `disabled_directories` to skip complete configured directories on that machine.

Use one per-directory filter under `[directory.<name>]`: `disabled` is a blocklist and `enabled` is an allowlist. Do not set both for one directory.

Use `[directory_overrides.<name>].path` to replace a portable directory path on a machine with a different filesystem layout. `tome status` and `tome doctor` mark active overrides.

Use `auto_install_plugins = "always"`, `"ask"`, or `"never"` to control machine-local consent for managed-plugin reconciliation. Use `tome sync --no-install` for a one-run override that does not change the persisted preference.

Do not reproduce or guess the complete TOML schema. Inside the Tome repository, consult `docs/src/configuration.md`. Elsewhere, use `tome config` for effective directory topology and resolved paths, inspect `machine.toml` for filters and consent, locate the installed documentation, and use the installed CLI's help before changing configuration.
