## Why

Tome does not ship agent-facing guidance for operating a skill library safely, so agents must rediscover role semantics, nested repository setup, and recovery order from human documentation. Packaging an official skill also provides a useful first skill during interactive setup and demonstrates Tome's own cross-tool distribution model.

## What Changes

- Add one `using-tome` skill covering user setup, configuration, routine operations, diagnosis, and recovery.
- Implement the documented local-path form of `tome add`, including managed local directories and rejection of Git-only flags.
- Package the skill through a Claude plugin and marketplace while keeping the same root `skills/` package installable through Tome.
- Offer the official skill as a default-selected Git source during interactive `tome init`, with immediate-clone disclosure and no change to `--no-input` behavior.
- Document both installation routes and retain automatic Git subdirectory adoption as a separate deferred task.

## Capabilities

### New Capabilities

- `agent-skill-distribution`: Defines the official Tome operations skill, Claude plugin packaging, cross-tool Git installation, and interactive init recommendation.

### Modified Capabilities

None.

## Impact

- New files under `skills/using-tome/` and `.claude-plugin/`.
- Interactive wizard changes in `crates/tome/src/wizard.rs` with unit and CLI regression coverage.
- Add-command changes in `crates/tome/src/add.rs`, `cli.rs`, and `lib.rs` with unit and CLI regression coverage.
- User documentation and changelog updates.
- No new Rust or frontend dependencies, config schema changes, or noninteractive network behavior.
