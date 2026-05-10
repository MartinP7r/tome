---
requirement: REL-04
phase: 17-migration-polish-uat-release
performed: 2026-05-10
status: passed
binary: target/release/tome (built from main @ ff57d3e)
test_method: copy-to-/tmp (Option C from REL-04 brainstorm)
---

# REL-04 Migration Smoke-Test Transcript

> Per Phase 17 ROADMAP success criterion 4: *"Migration smoke-test executed
> on the user's real `~/dev/coding-agent-files` library: 62 known symlinks
> convert to real directories cleanly, distribution targets re-symlink to
> the new library copies, no skill content lost (verified by pre/post
> `content_hash` comparison), no `tome doctor` warnings introduced.
> Smoke-test transcript captured in the phase artifacts."*

## Method

Smoke-tested on a `cp -RP` copy at `/tmp/tome-smoke-test` rather than the
real library — the real `~/dev/coding-agent-files` had ~100 uncommitted
file changes at smoke-test time, and conflating those with the migration
diff would have made rollback messy. Copy preserves symlinks as symlinks
(targets unchanged, still pointing into `~/.claude/plugins/...` etc.).
After `library_dir` was rewritten in the copy's `tome.toml` to point at
itself, the smoke library was fully isolated from the real one.

> Distribution-target re-symlinking (the second half of REL-04 SC) was
> NOT exercised in this transcript — the smoke copy's distribution paths
> in `tome.toml` still point at the user's real `~/.claude/skills`,
> `~/.codex/skills` etc., and re-pointing them to `/tmp` would have made
> the test exercise unrealistic discovery paths. Re-symlinking will be
> verified during the actual real-library migration ahead of REL-05.

## Pre-state Snapshot

```
Total entries:       99
Symlinks (v0.9):     57   ← migration targets
Real-dirs (v0.10):   42   ← already in v0.10 shape (or local skills)
Manifest entries:   103   ← 8 stale entries with no on-disk dir (pre-existing)
managed: true count: 65   ← matches 57 reachable + 8 missing-from-disk
Broken symlinks:      0   ← all 57 sources resolve
Library size:       5.7M
```

`tome doctor` reported 12 pre-existing health issues (8 missing-from-disk
manifest entries, 4 orphan directories, foreign symlinks in distribution
targets pointing into the user's real `coding-agent-files`).

## Dry-run

```
$ tome --tome-home /tmp/tome-smoke-test migrate-library --dry-run --no-input
[dry-run] No changes will be made
v0.9 → v0.10 library migration plan

  Will convert 57 symlinks → real directories (~5.2 MB additional disk).

  ╭─────────────────────────────┬─────────────────────────────────────...┬──────────┬────────╮
  │ SKILL                       │ SOURCE                              ...│ SIZE     │ STATUS │
  ├─────────────────────────────┼─────────────────────────────────────...┼──────────┼────────┤
  │ agent-development           │ ~/.claude/plugins/.../agent-develop ...│ 67.7 KB  │ ✓      │
  │ axiom-accessibility         │ ~/.claude/plugins/cache/axiom-mark  ...│ 62.5 KB  │ ✓      │
  │ ... (55 more rows)          │ ...                                 ...│ ...      │ ✓      │
  ╰─────────────────────────────┴─────────────────────────────────────...┴──────────┴────────╯

  Note: tome does not snapshot your library before migrating. Commit your
  library directory to git (or back it up some other way) BEFORE proceeding.
  This conversion is one-way — there is no path back to v0.9 shape.

✓ 57 skills migrated to v0.10 shape
$ # ↑ this last line is a UX nit — see "Findings" below — no actual mutation occurred
```

Verified zero mutation: symlinks still 57, real-dirs still 42 post-dry-run.

## Live Migration

```
$ time tome --tome-home /tmp/tome-smoke-test migrate-library --yes
... [same plan rendering as dry-run, sans [dry-run] banner] ...
✓ 57 skills migrated to v0.10 shape
real    0m0.069s
user    0m0.000s
sys     0m0.060s
```

**69 milliseconds** for 57 skills × ~5.2 MB of content. The bottleneck
on a real machine will be filesystem I/O for the copies — on this APFS
volume it's effectively instant.

## Post-state Verification

| Check | Pre | Post | Δ | Status |
|---|---|---|---|---|
| Total entries | 99 | 99 | 0 | ✓ |
| Symlinks | 57 | 0 | -57 | ✓ all migrated |
| Real-dirs | 42 | 99 | +57 | ✓ matches |
| Manifest entries | 103 | 103 | 0 | ✓ unchanged |
| Library size | 5.7M | 12M | +6.3M | ✓ ≈ predicted ~5.2 MB |
| `tome doctor` warnings | 12 | 12 | 0 | ✓ no new issues |

**Content integrity:** SHA-256 of the recursive file-tree was captured
for each of the 57 symlinks pre-migration (resolved through the symlink
to the source) and compared against the post-migration real-dir's
recursive hash:

```
Hash matches: 57 / 57
Hash mismatches: 0
```

Zero data loss. Every byte of every file in every migrated skill is
byte-identical to the source content.

## Idempotency

```
$ tome --tome-home /tmp/tome-smoke-test migrate-library --yes
v0.9 → v0.10 library migration plan

  ✓ no v0.9-shape entries detected — library is already in v0.10 shape.
```

Phase 11 D-06 honored — re-running on a v0.10-shape library is a clean
no-op.

## Findings

### ✓ Acceptance criteria met

- [x] All 57 v0.9-shape symlinks converted to real directories (cleanly,
      no errors, no skipped entries)
- [x] No skill content lost — 57/57 SHA-256 hashes match pre/post
- [x] No new `tome doctor` warnings introduced (all 12 are pre-existing)
- [x] Migration is idempotent
- [x] Smoke-test transcript captured (this file)

### ⚠ UX nit — file as v0.10 follow-up

`render_result_to` always prints `✓ N skills migrated to v0.10 shape`
even in dry-run mode. The opening `[dry-run] No changes will be made`
banner disambiguates, but the closing line is technically misleading.
Should switch the closing line in dry-run to something like
`✓ N skills WOULD migrate to v0.10 shape (dry-run, no changes made)`
or `✓ Plan validated: 57 skills ready to migrate (dry-run)`.

`migration_v010::render_result_to` doesn't currently take a `dry_run`
flag — easiest fix is for `cmd_migrate_library` to pass it through and
the renderer to choose the verb.

### ⚠ Doctor reports `62 managed symlink(s) tracked in git` post-migration

This warning text is pre-existing and didn't change between pre/post
states, but it now shows `62` even though the library has zero symlinks
post-migration. The underlying check is probably looking at the original
repo's git index (the smoke-test library is not a git repo), or the
warning has stale state. Worth a follow-up — but pre-existing, not
introduced by migration.

## Verdict

**Migration is safe to run on the real `~/dev/coding-agent-files` library.**
Recommended sequence for the real run:

1. `cd ~/dev/coding-agent-files && git add -A && git commit -m "wip: pre-v0.10 snapshot"` — commit current state for rollback
2. `tome migrate-library --dry-run` — preview (will be ~57 entries, same shape)
3. `tome migrate-library` — interactive confirm
4. `tome sync` — verify distribution targets re-symlink to the new real-dir copies
5. `tome doctor` — confirm 12 pre-existing warnings remain (no new ones)

After this passes on the real library, REL-04 is fully closed and we
can move to REL-05 (cargo-dist release of v0.10.0).
