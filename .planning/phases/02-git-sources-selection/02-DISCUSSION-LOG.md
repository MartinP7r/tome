# Phase 2: Git Sources & Selection - Discussion Log

> **Audit trail only.** Do not use as input to planning, research, or execution agents.
> Decisions are captured in CONTEXT.md — this log preserves the alternatives considered.

**Date:** 2026-04-15
**Phase:** 02-git-sources-selection
**Areas discussed:** Git clone/update strategy, Per-directory skill selection, Failure & offline behavior, tome remove UX

---

## Git Clone/Update Strategy

| Option | Description | Selected |
|--------|-------------|----------|
| SHA-256 of URL | ~/.tome/repos/a1b2c3.../ — deterministic, path-safe | :heavy_check_mark: |
| Slugified name from config | ~/.tome/repos/my-skills/ — human-readable | |
| URL-derived slug | ~/.tome/repos/github-com-user-repo/ — readable + deterministic | |

**User's choice:** SHA-256 of URL
**Notes:** Matches GIT-02 spec. Opaque when browsing manually but avoids path-safety issues.

---

| Option | Description | Selected |
|--------|-------------|----------|
| No subdirectory support | Skills must be at repo root. Simpler. | |
| Optional subdir field | Add `subdir` to DirectoryConfig for monorepos/nested layouts | :heavy_check_mark: |

**User's choice:** Optional subdir field
**Notes:** User asked for thorough explanation of the problem space. After seeing both repo structures (skills at root vs nested in subfolder), chose subdir support to handle monorepos and dotfiles repos.

---

| Option | Description | Selected |
|--------|-------------|----------|
| Remote HEAD | Whatever the repo's default branch is | :heavy_check_mark: |
| Always main | Hardcode main as default | |
| Require explicit ref | Make branch/tag/rev mandatory | |

**User's choice:** Remote HEAD
**Notes:** Most intuitive — follows the repo author's intent.

---

## Per-Directory Skill Selection

| Option | Description | Selected |
|--------|-------------|----------|
| Nested tables | `[directory.<name>]` sections in machine.toml | :heavy_check_mark: |
| Flat dotted keys | `disabled.claude = [...]` at top level | |

**User's choice:** Nested tables
**Notes:** Clean separation per directory.

---

| Option | Description | Selected |
|--------|-------------|----------|
| Global disabled always wins | Global disabled = hard block everywhere | |
| Local wins | Per-directory enabled overrides global disabled | :heavy_check_mark: |

**User's choice:** Local wins (per-directory enabled overrides global disabled)
**Notes:** User's intuition was that more local/specific settings should win, citing Claude Code's settings precedent where project overrides global. After discussing the security argument for global-wins, agreed it doesn't apply here (single user, skills are markdown not code). This overrides MACH-05 as originally written in REQUIREMENTS.md.

---

## Failure & Offline Behavior

| Option | Description | Selected |
|--------|-------------|----------|
| Warn and use cached | Continue sync with last successful clone state | :heavy_check_mark: |
| Warn and skip entirely | Skip failed directory completely | |
| Fail the whole sync | Abort all sync on any git failure | |

**User's choice:** Warn and use cached
**Notes:** Graceful degradation. Local directories always sync; git failures use cached state.

---

| Option | Description | Selected |
|--------|-------------|----------|
| Yes, different messages | Distinct messages for never-cloned vs update-failed | :heavy_check_mark: |
| Same warning either way | Generic "git operation failed" message | |

**User's choice:** Different messages
**Notes:** Helps user understand whether they're getting stale data or no data.

---

## tome remove UX

| Option | Description | Selected |
|--------|-------------|----------|
| Full cleanup | Config + library + symlinks + cache + manifest/lockfile | :heavy_check_mark: |
| Config only | Just remove config entry, sync cleans rest | |
| Config + library, leave cache | Keep git clone for potential re-add | |

**User's choice:** Full cleanup
**Notes:** One command, clean slate.

---

| Option | Description | Selected |
|--------|-------------|----------|
| Confirm in TTY, auto in pipe | Interactive confirmation with non-TTY auto-remove | :heavy_check_mark: |
| Always confirm | Even in pipes | |
| Never confirm | Use --dry-run to preview | |

**User's choice:** Confirm in TTY, auto in pipe
**Notes:** Matches existing cleanup behavior.

---

| Option | Description | Selected |
|--------|-------------|----------|
| Both flags | --dry-run and --force | :heavy_check_mark: |
| Only --dry-run | No force skip | |
| Neither | Keep it simple | |

**User's choice:** Both flags
**Notes:** Consistent with tome sync flag patterns.

---

## Claude's Discretion

- Git error message exact wording
- Internal module organization for git operations
- Whether subdir field appears in wizard or is config-only
- Exact format of tome remove preview table

## Deferred Ideas

None — discussion stayed within phase scope.
