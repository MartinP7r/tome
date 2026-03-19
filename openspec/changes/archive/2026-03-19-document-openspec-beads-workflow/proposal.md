## Why

`tome` now has OpenSpec and Beads installed, but the repository does not yet explain how they should be used together in day-to-day development. Without a documented workflow, the setup is just tooling residue: future work will ignore it, contributors will guess, and the repo will drift back to ad-hoc planning.

This change dogfoods the stack on a small, low-risk task. It documents a sane workflow for planning work in OpenSpec and tracking execution in Beads so the team can decide whether the setup is genuinely useful before using it on larger features.

## What Changes

- Add a development workflow document describing when to use OpenSpec, when to use Beads, and how they fit together.
- Update `CLAUDE.md` so coding agents in this repo follow the documented workflow.
- Add a lightweight README link so humans can find the workflow doc without spelunking.
- Define a minimal, repeatable process for converting an OpenSpec change into Beads execution tasks.

## Capabilities

### New Capabilities
- `development-workflow`: Documented workflow for planning changes in OpenSpec and tracking implementation in Beads.

### Modified Capabilities
- None.

## Impact

- Adds repository documentation under `docs/src/`
- Updates `CLAUDE.md`
- Updates `README.md`
- Establishes a repeatable planning/execution workflow for future development
