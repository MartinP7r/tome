# MCO-165 read boundary decision

Date: 2026-10-02

## Decision

Use Paperclip's company skill attachment reads as the least-privilege live
before-state route for `tome paperclip-agents preview`.

The preview boundary now derives each agent's desired skills from:

1. `GET /api/agents/{agentId}` -> `companyId`
2. `GET /api/companies/{companyId}/skills` -> company skill ids and attachment counts
3. `GET /api/companies/{companyId}/skills/{skillId}` -> `key` plus `usedByAgents[]`

For every `usedByAgents` item with `desired: true`, Tome adds that company skill
`key` to the agent's live desired-skill set. Runtime adapter state is currently
unavailable through this read-only route, so the preview records runtime as
unknown/optional rather than fabricating it.

The old direct read remains a compatibility fallback:

- `GET /api/agents/{agentId}/skills`

In this run it still returned HTTP 403 with `Missing permission:
agents:suggest-changes`, so it is not the least-privilege route for preview.

## Safety boundary

No mutation route was called:

- no `POST /api/agents/{agentId}/skills/sync`
- no `tome paperclip-agents apply`

The live preview verification used an isolated scratch `TOME_HOME`, so the only
write was a disposable local confirmation-token store under the run scratch
directory.

## Live verification

Command:

```bash
TOME_HOME="$PAPERCLIP_RUN_SCRATCH_DIR/mco-165-live-preview/tome-home" \
  cargo run -p tome -- paperclip-agents preview \
  --catalog curation/paperclip-agents/catalog.toml \
  --assignments curation/paperclip-agents/agents.toml \
  --constraint paperclip-agent \
  --paperclip-api-url "$PAPERCLIP_API_URL"
```

Result: exit 0. The preview established a live before state for
`FoundingEngineer` (`ecadbdc0-fcf5-4d18-b62f-111999f269c0`):

```text
before desired: local/fbfe9eb2f4/rtk-command-output
intended desired: using-tome
add: using-tome
remove: local/fbfe9eb2f4/rtk-command-output
keep: (none)
```

This means the current MCO-163 assignment is now trustworthy as a live preview,
but it also shows a real apply risk: because apply uses replace semantics, the
current assignment would remove `local/fbfe9eb2f4/rtk-command-output` from
FoundingEngineer unless the parent plan intentionally preserves it.

## Verification

- `cargo test -p tome paperclip_agents` -> 22 passed.
- `cargo clippy -p tome --all-targets -- -D warnings` -> passed.
- `cargo test -p tome` -> passed.
- `make ci` reached the final `typos` step after Rust gates, then failed because
  the local `typos` binary is not installed.
- Broader workspace clippy (`cargo clippy --all-targets -- -D warnings`) failed
  in `tome-desktop` on existing/concurrent `SyncOptions` initializer errors
  missing `selected_profile`; the touched `tome` crate clippy passed.
