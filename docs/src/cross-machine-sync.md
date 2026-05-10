# Cross-machine sync

tome's library is designed to be a portable, version-controlled artifact —
you commit `~/.tome/` to your dotfiles and clone it onto every machine you
work on. This page walks the workflow end-to-end: setting up the
source-of-truth machine, bootstrapping a fresh machine, and what happens
when things drift.

> **Why this matters.** Pre-v0.10, tome's library was a thin layer of
> symlinks pointing into machine-specific marketplace caches; cloning the
> library onto a fresh machine was meaningless because the symlink targets
> didn't exist. v0.10 makes the library a real-directory copy of every
> skill, with `tome.lock` recording exactly what versions are installed.
> Now the library is portable. See
> [Architecture — Library-canonical model](architecture.md#library-canonical-model)
> for the underlying mechanic.

## Walkthrough — Machine A (source of truth)

Machine A is the machine you use to curate your skill library. You run
`tome init` here, install plugins, edit local skills, and commit the
result.

```bash
# 1. Set up tome on Machine A.
tome init

# 2. Curate your library — install Claude plugins, add git directories,
#    enable/disable skills via `tome browse`.
claude plugin install foo@bar
tome add https://github.com/my-org/my-skills.git
tome sync

# 3. Commit ~/.tome/ to your dotfiles (or whatever portable storage you
#    use). The library content is real-directory copies; the manifest
#    and tome.lock pin exactly what's installed.
cd ~/.tome
git init
git add .
git commit -m "Initial tome library"
git remote add origin git@github.com:you/your-dotfiles.git
git push -u origin main
```

What's in `~/.tome/` after this:

- `tome.toml` — your portable directory configuration (sources,
  distribution targets, exclude lists)
- `library/` — real-directory copies of every skill (managed and local)
- `.tome-manifest.json` — content hashes and provenance for every library
  entry
- `tome.lock` — Cargo.lock-shaped: pinned versions and content hashes for
  managed skills

Machine-specific preferences (`~/.config/tome/machine.toml`) are
deliberately NOT in `~/.tome/` — they're per-machine by design. Things
like `disabled` skills, `auto_install_plugins` consent, and
`[directory_overrides.<name>]` belong there. See
[Configuration — `machine.toml` Machine-Local Preferences](configuration.md#machinetoml--machine-local-preferences)
for the full schema.

## Walkthrough — Machine B (fresh machine)

Machine B is a fresh machine that's never seen your library. You install
tome and Claude Code, clone your dotfiles, and run `tome sync`.

```bash
# 1. Install tome (Homebrew or cargo).
brew install martinp7r/tap/tome
# or: cargo install tome

# 2. Install Claude Code (required if your library has Claude plugins).
#    See https://claude.com/product/claude-code for instructions.

# 3. Clone your dotfiles into ~/.tome (or wherever the portable artifact lives).
git clone git@github.com:you/your-dotfiles.git ~/.tome

# 4. First sync. tome reads tome.lock and reconciles what's actually
#    installed against what should be installed.
tome sync
```

On the first sync, tome detects drift (your machine has zero plugins
installed; the lockfile expects N) and prompts:

```
Tome detected N missing or out-of-date managed plugins. Install/update them now?
> Yes (always — install on every sync)
  Yes (ask me again next time)
  No (never ask again on this machine)
```

Your choice persists in `machine.toml::auto_install_plugins` as one of
`Always | Ask | Never`. Pick `Always` on a personal machine
(auto-install on every sync); pick `Never` on a locked-down workstation
where you want to inspect drift before applying; pick `Ask` to skip this
run only and be prompted again next time.

Once you confirm, tome shells out to
`claude plugin install <plugin>@<marketplace>` for every missing plugin,
re-discovers the skill content, verifies the resulting `content_hash`
matches `tome.lock`, and then distributes the skills to your configured
target directories (`~/.claude/skills`, `~/.codex/skills`, etc.). You're
set up.

## Reference — `tome.lock` semantics

`tome.lock` is the cross-machine state contract. It's the equivalent of
`Cargo.lock` for your skill library: a snapshot of every installed
version plus content hash that tome can use to bring a fresh machine into
the same state.

Each managed-skill entry records:

- `name` — the skill name
- `version` — the actual installed version (display-only; see drift basis
  below)
- `content_hash` — SHA-256 of the skill directory contents
- `source_name` — the directory in `tome.toml` that owns the skill
  (`Option<DirectoryName>` — `None` for Unowned skills)
- `previous_source` — the previous owner if the skill has been
  re-anchored (closes the Phase 13 fork-in-place gap)
- `registry_id`, `git_commit_sha` — provenance metadata when applicable

**Drift basis: `content_hash`, NOT version.** When `tome sync` reconciles
the lockfile against actually-installed plugins, drift is computed from
`content_hash(library/<skill>) != lockfile.content_hash`. The version
string is display-only in the diff output (e.g.
`plugin X: 5.0.5 → 5.0.7`). Because Claude CLI doesn't accept
`--version` on `claude plugin install`, true version pinning is upstream
future work — see
[Architecture — Lockfile-authoritative reconciliation](architecture.md#lockfile-authoritative-reconciliation).

## Reference — `auto_install_plugins` consent

`~/.config/tome/machine.toml`:

```toml
auto_install_plugins = "always"  # auto-install on every sync (CI / personal)
auto_install_plugins = "never"   # warn-only; never modify; require manual install
auto_install_plugins = "ask"     # re-prompt on every sync that detects drift
```

The unset case (field absent / `None`) is treated as a first-time
prompt — tome asks once per `tome sync` invocation that detects drift
and persists your choice. Pick `always` once and tome remembers; pick
`never` and you'll see drift warnings on every sync but tome won't touch
installed plugins.

`--no-install` is a global flag that overrides the persisted choice for
the current invocation. Use this when you want to inspect drift on a
machine where `auto_install_plugins = "always"` is set:

```bash
tome sync --no-install
```

Mirrors Cargo's `--frozen` / `--locked` semantics — temporary, doesn't
change the persisted setting.

## Reference — `directory_overrides` for path remapping

Different machines have different home layouts. macOS keeps Claude
plugins at `~/Library/Application Support/Claude/plugins/cache`; Linux
keeps them at `~/.claude/plugins/cache`. Your portable `tome.toml` can
only express one path; `machine.toml::[directory_overrides.<name>]`
provides the machine-specific remap (PORT-01..05).

`~/.config/tome/machine.toml` on Linux:

```toml
[directory_overrides.claude-plugins]
path = "~/.claude/plugins/cache"
```

`~/.config/tome/machine.toml` on macOS would override the same name to a
different path.

Override application happens at config load (after tilde expansion,
before validation), so all downstream code sees the canonical
post-override path. Unknown override directory names emit a
typo-target stderr warning. Override-induced validation failures are
wrapped with a distinct error attributing them to `machine.toml` rather
than `tome.toml`.

`tome status` and `tome doctor` annotate any directory whose path was
rewritten by an override with `(override)` so you can tell at a glance
which paths come from the portable config and which come from the
machine-local layer.

See [Configuration](configuration.md) for the full schema.

## Reference — what happens when `claude` is missing on Machine B

The `ClaudeMarketplaceAdapter` shells out to the `claude` binary. If
`claude` isn't on `PATH` and your `tome.toml` has any
`[directories.<name>]` with `type = "claude-plugins"`, `tome sync`
produces a clear actionable error naming the binary:

```
error: claude CLI not found on PATH — install Claude Code, or remove
[directories.<name>] entries with type = "claude-plugins" from tome.toml
```

Install Claude Code from <https://claude.com/product/claude-code> and
re-run `tome sync`. If your library only has git-source skills and no
Claude plugins, missing `claude` won't break sync — you'd only see this
error if the lockfile contains entries the Claude adapter would need to
install. A library that's purely git-sourced and local-source is fully
portable to a machine without Claude installed.

Vanished plugins (the marketplace removed a plugin you had installed)
surface as a stderr warning
(`plugin X vanished from marketplace Y; using preserved library copy`)
and `tome sync` continues. Distribution still happens from the preserved
library copy.

Adapter `install`/`update` failures aggregate into a
`⚠ N install operations failed` summary; library distribution still
completes for skills whose adapter calls succeeded; sync exits non-zero
on partial install failure.

## Reference — migrating a v0.9 library on Machine B

If your dotfiles repo predates v0.10, the library was stored as
machine-specific symlinks. v0.10's first `tome sync` against such a
library refuses with a Conflict / Why / Suggestion error pointing at the
migration command:

```bash
tome migrate-library --dry-run    # preview the conversion
tome migrate-library              # run it (confirmation prompt; default no)
```

The dry-run shows a summary table — count of symlinks to convert,
approximate additional disk usage, per-skill SKILL / SOURCE / SIZE /
STATUS columns — and the live run prompts via `dialoguer::Confirm`
defaulting to no. Pressing anything other than `y` aborts cleanly with
no filesystem mutation. See the
[v0.10 release notes](https://github.com/MartinP7r/tome/blob/main/CHANGELOG.md)
for the full migration walkthrough.

For CI or non-interactive automation, use `--yes` (mirrors
`tome remove skill --yes`):

```bash
tome migrate-library --yes
```

Under `--no-input` without `--yes`, migration bails with a
Conflict / Why / Suggestion error — destructive operations require
explicit consent in non-interactive mode.

Broken managed symlinks (target gone) are SKIPPED and preserved in
place so you can recover manually. Idempotent on re-run; subsequent
syncs proceed normally.

The conversion is one-way — there is no `--undo-migrate`. Commit your
library directory to git (or back it up some other way) BEFORE running.
