# Git Source OS Cache Design

**Date:** 2026-07-03
**Backlog:** Phase 999.6
**Status:** Approved for implementation planning

## Problem

Tome currently stores Git-source checkouts at
`<tome_home>/repos/<sha256(url)>`. A custom `tome_home` may itself be a
version-controlled dotfiles repository. In that layout, `git add --all` sees
Tome's checkout as an embedded repository and stages a gitlink that cannot be
reconstructed because the outer repository has no `.gitmodules` entry.

The checkout is operational cache data. It is reproducible from `tome.toml`
and `tome.lock` and must not live in the portable Tome home.

## Decision

Store persistent Git-source checkouts under the operating system's cache root:

- macOS: `~/Library/Caches/tome/repos/<sha256(url)>`
- Linux: `${XDG_CACHE_HOME:-~/.cache}/tome/repos/<sha256(url)>`

Use `dirs::cache_dir()` to select the platform path. Keep the existing URL hash,
shallow-clone, fetch/reset, cached-state fallback, ref pinning, and removal
semantics unchanged.

This is a location change, not a switch to ephemeral clones. Persistent cache
state avoids unnecessary network work and preserves the current offline
fallback when an update fails.

## Library and Distribution Behavior

The canonical, usable skill remains a real directory in
`<library_dir>/<skill-name>`. For the current custom layout that is
`coding-agent-files/skills/<skill-name>`.

The source checkout's repository metadata remains in the OS cache. Tome copies
the discovered skill directory into the library; it does not distribute the
checkout itself. Tool-specific skill directories continue to symlink to the
plain library copy.

Managed library copies remain reproducible and continue to be listed in the
library's generated `.gitignore`. This change does not alter that policy.

## Path Model

Extend `TomePaths` with an explicit cache root so path selection remains
centralized and tests do not write into the developer's real OS cache.

- `TomePaths::new(...)` resolves the production cache root with
  `dirs::cache_dir()?.join("tome")`.
- A crate-visible constructor accepts an explicit cache root for isolated
  tests.
- `repos_dir()` returns `<cache_root>/repos`.

Failure to determine the platform cache directory is reported as a path
configuration error. Tome must not silently fall back to `tome_home`, because
that would recreate the original embedded-repository failure.

## Existing Installations

No product migration is required. Tome has no external users relying on the old
location. On the next sync, each configured Git source is cloned into the new
cache path.

For the current development machine only, remove the accidentally staged
gitlink and delete the obsolete `coding-agent-files/repos` checkout after the
new behavior is verified. This local cleanup is not shipped as migration code.

## Removal Behavior

`tome remove dir <name>` continues to delete the deterministic cached checkout,
now resolved under the OS cache root. Library ownership and Unowned transitions
remain unchanged.

## Documentation

Update command, configuration, architecture, and cross-machine documentation
that currently describes `~/.tome/repos`. Document Git checkouts as persistent,
reconstructible OS cache data and distinguish them from canonical library
copies.

## Testing

Implementation follows red-green-refactor:

1. Add a failing `TomePaths` test proving `repos_dir()` uses an injected cache
   root rather than `tome_home`.
2. Add or update Git-source lifecycle tests proving clone/update and removal use
   the new cache path.
3. Preserve tests proving deterministic URL hashing and cached-state fallback.
4. Run the focused tests, then `make ci`.

## Non-goals

- No automatic `.gitignore` edits in a containing repository.
- No migration or copying of old `<tome_home>/repos` checkouts.
- No change to library ownership, managed-skill ignore policy, lockfile format,
  Git ref semantics, or distribution symlinks.
- No cache-pruning command or time-based eviction policy.
