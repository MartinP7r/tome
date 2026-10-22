# Upgrade from 0.16.4 to 0.17.0 (CLI)

Tome 0.17.0 changes the configuration layout and the target-deployment model.
There is no general automatic migration. Back up your Tome home and local
settings before editing anything:

```bash
cp -a ~/.tome ~/.tome.backup-before-0.17
if [ -d ~/.config/tome ]; then
  cp -a ~/.config/tome ~/.config/tome.backup-before-0.17
fi
```

If you rely on 0.16.4's symlink-based distribution being refreshed or removed by
`sync`/`eject`, stay on 0.16.4 for now. Version 0.17.0 creates independent
copies only at absent target paths; the copy operation never overwrites an
occupied path. Existing copy deployments are not migrated, refreshed, or
removed. Separate cleanup can remove a legacy Tome-owned symlink only when
persisted provenance proves ownership; foreign or ambiguous links remain.
`tome eject` removes supported legacy symlinks, not copy deployments. This
lifecycle is intentionally conservative and is not a drop-in replacement for
the prior symlink behavior.

## Move configuration into layers

The old flat `tome.toml` mixed shared Git sources and machine-specific paths.
Version 0.17.0 separates them:

- `~/.tome/tome.toml` — shared repository policy and Git sources.
- `~/.tome/machines/<profile>.toml` — machine-wide local directories and
  destination routes.
- The nearest project `.tome.toml` — optional project-only destinations/routes.
- `~/.config/tome/settings.toml` — the selected profile and local consent.

Profile names are user-defined, not built-in roles. The profile files are shared
configuration; each machine's local `settings.toml` selects which one it uses.
Tome does not infer profile selection from the hostname. Reuse an appropriate
existing profile or create a distinct one when machine paths/routes differ.

Create a profile, then edit its TOML file:

```bash
tome profile create personal
tome profile select personal
```

Move each ordinary local `[directories.<name>]` entry from the old shared
`~/.tome/tome.toml` into `~/.tome/machines/personal.toml`. Move its current
machine-specific path from `[directory_overrides.<name>]` into that profile's
`path` field. Do not leave ordinary local paths in the shared repository policy.

Keep Git sources in shared `tome.toml`. For each Git directory, replace the old
remote-valued `path` with the explicit `url` field; keep its `branch`, `tag`,
`rev`, and `subdir` values as applicable:

```toml
[directories.team-skills]
url = "https://github.com/myorg/team-skills.git"
type = "git"
branch = "main"
subdir = "skills"
```

The legacy `path` spelling for Git sources is still parsed for compatibility,
but new or edited configuration should use `url`. Local filesystem paths
continue to use `path` in the selected profile.

Repository-policy fields such as `library_dir`, `exclude`, `backup`, and
`source_pins` remain in `tome.toml`. Review every old setting rather than
copying old machine-only fields into the new shared file.

## Convert machine preferences

The old `machine.toml` is no longer read. There is no `--machine` option; the
new `--settings` option selects the local settings file and is not a replacement
for the old machine configuration.

- Put local sources and destinations in the selected profile.
- Convert each old `directory_overrides` entry by using its resolved path in the
  profile.
- Convert per-destination enabled lists to shared skill tags plus a
  `[routes.<destination>] tags = [...]` selector. Tags use OR matching.
- Convert per-destination disabled lists to
  `[routes.<destination>] exclude = [...]`.
- A former global disabled list has no exact one-field equivalent. Use pool
  exclusions only if the skill should be excluded from shared discovery and the
  library; otherwise classify skills with tags and define routes deliberately.
- A former disabled-directory setting has no disabled flag in profiles. Omit
  that directory from the active profile or maintain a separate profile.
- Move the old plugin-install consent to local `managed_plugin_install` in
  `settings.toml` (`always`, `ask`, or `never`). Keep `git_sync` and
  `backup_runtime` local as well.

For example, local settings can look like:

```toml
profile = "personal"
git_sync = "ask"
managed_plugin_install = "ask"
backup_runtime = "ask"
```

After reviewing the paths, tags, and routes, verify the selected profile and
preview sync before applying it:

```bash
tome profile list
tome status
tome sync --dry-run --no-input --no-install
```

Inspect the preview. Only then run `tome sync` if the proposed changes are
intended. Commit shared Tome files with your normal signed-commit workflow;
do not commit local `~/.config/tome/settings.toml` to the shared repository.

## Command changes

- `tome add <git-url>` registers a shared discovery source. Git sources no
  longer accept `--to` or `--role`; classify skills with tags and routes.
- `tome add <local-path> --role <managed|source|synced|target>` registers a
  machine-local directory in the selected profile.
- Use `tome --version`, not the removed `tome version` command.
- Use `tome pool exclude` / `tome pool restore`, not the removed
  `tome remove pool` command.
- `tome migrate-library` is no longer shipped. If your library still needs the
  legacy pre-0.10 migration, run `tome migrate-library` on 0.16.4 before
  upgrading.

See [configuration](configuration.md), [cross-machine sync](cross-machine-sync.md),
and [commands](commands.md) for the current CLI reference.
