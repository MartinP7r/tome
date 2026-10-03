# MCO-163 preview-only curation run

Date: 2026-10-03

## Scope and safety boundary

- Preview-only run for MCO-163.
- No `tome paperclip-agents apply` invocation.
- No `skills:sync` invocation.
- No canonical-library mutation.
- No model calls.
- Paperclip API use was limited to read-only `GET` requests for the company skill catalog and company skill details.

## Repository baseline inspected

- Repaired PR branch base: `origin/main` at `392883ad43e8729550e4324368bb08981943b696`.
- Restacked branch: `paperclip/MCO-163-restack`, later pushed to `paperclip/MCO-163-preview-curation`.
- Relevant replayed commits only: the MCO-163 preview evidence commit and the read-only Paperclip skill attachment boundary commit.
- The dirty `/Users/martin/dev/tome` checkout, including MCO-165 files and untracked `ui/`, was not reset, cleaned, or overwritten.

## Canonical skill inventory

Observed live Tome library:

| Skill | Location | Digest evidence | Decision |
|---|---|---:|---|
| `demo-skill` | `~/.tome/skills/demo-skill` | `903d4a7d7948d35159f315d091af0b97c89110f5713b2125ff98d57180e3f1bb` | Omitted. Frontmatter says `E2E fixture`, so assigning it to a Paperclip agent would be speculative. |

Observed repo-owned official Tome skill:

| Skill | Location | Digest evidence | Paperclip company skill | Decision |
|---|---|---:|---|---|
| `using-tome` | `skills/using-tome` | `a1d9e649724f95e39e71884ea5ebd5a28ffc09bfe97a99bf57b794e4aac127f5` | `local/d64eaed4a6/using-tome` | Included in `tome-maintainer` because it is the official setup/sync/recovery skill offered by `tome init`. |

Observed existing Paperclip desired skill to preserve:

| Skill | Paperclip company skill | Live attachment evidence | Compatible constraints | Decision |
|---|---|---|---|---|
| `rtk-command-output` | `local/fbfe9eb2f4/rtk-command-output` | Current desired skill for `FoundingEngineer`; company catalog `attachedAgentCount = 7`. | `paperclip-agent`, `codex` | Included in `tome-maintainer` to preserve Martin's explicit decision. The live Paperclip detail exposes a stable id/key/slug/name and a Paperclip Codex-run description, but `source` and `provenance` fields are null, so the catalog does not claim a repo-owned source digest. |

Paperclip company skill catalog read returned 30 skills. Sanitized live detail evidence is stored in `curation/paperclip-agents/live-skill-evidence.snapshot.json`.
Live details used for the two proposed skills:

| Skill | Company skill id | Key | Attached agents |
|---|---|---|---:|
| `rtk-command-output` | `b01320ae-a9d0-4b4f-bc2a-11369709c574` | `local/fbfe9eb2f4/rtk-command-output` | 7 |
| `using-tome` | `cfa45993-3618-49df-8d5d-4cc0cc28f7c4` | `local/d64eaed4a6/using-tome` | 0 |

## Proposed catalog and assignment

- Catalog: `curation/paperclip-agents/catalog.toml`
- Assignment registry: `curation/paperclip-agents/agents.toml`
- Catalog SHA-256: `c2b752f975e6d49c9edd71108a819f3283bd1db52cf2bce956184a2fb38506b9`
- Assignment SHA-256: `81af53d2b2a45888ae9041ac027df7eaef6835578d29ad5d9ed6ef36675ce907`
- Catalog revision: `catalog-2026-10-03-mco-163-preview`
- Command-selected constraint: `paperclip-agent`
- Assignment-local constraint for `FoundingEngineer`: `codex`
- Proposed set: `tome-maintainer`
- Assigned agent: `FoundingEngineer` (`ecadbdc0-fcf5-4d18-b62f-111999f269c0`)

## Fleet impact

Raw CLI output is stored in `curation/paperclip-agents/preview-output.txt`.

| Agent | Before desired | Intended desired | Add | Remove | Keep |
|---|---|---|---|---|---|
| `FoundingEngineer` (`ecadbdc0-fcf5-4d18-b62f-111999f269c0`) | `local/fbfe9eb2f4/rtk-command-output` | `local/d64eaed4a6/using-tome`, `local/fbfe9eb2f4/rtk-command-output` | `local/d64eaed4a6/using-tome` | none | `local/fbfe9eb2f4/rtk-command-output` |

Reasons:

- Add `local/d64eaed4a6/using-tome`: selected by direct skill membership in use-case set `tome-maintainer`; satisfies the `paperclip-agent` constraint.
- Keep `local/fbfe9eb2f4/rtk-command-output`: selected by direct skill membership in use-case set `tome-maintainer`; satisfies the combined `paperclip-agent` and `codex` constraints; preserves Martin's 2026-10-03 decision.
- Remove none.

## Omitted agents and reasons

| Agent | ID | Reason |
|---|---|---|
| `CEO` | `630625ec-de6f-41d5-85ff-a0cbe8ad499c` | No explicit Tome-maintainer assignment approval in this issue. |
| `CTO` | `ea0a33e7-8f69-4f7b-aba5-131ad58ba9fa` | Likely reviewer/owner, but not an approved runtime recipient. |
| `ApplePlatformsEngineer` | `84516c44-3bb1-41d0-b902-8f48dcd2b7e4` | Could need Tome in future, but no current explicit Tome-maintainer assignment. |
| `FrontendEngineer` | `d2c35372-1945-49b0-8b2d-824c64ba4228` | Could need Tome in future, but no current explicit Tome-maintainer assignment. |
| `UXDesigner` | `b95d4232-0bd7-435f-817b-9a93b0f7f11b` | UX-facing review role does not imply skill-runtime assignment. |
| `CMO` | `c2a92d8f-3053-4a85-ae1f-d1dfe251f2d8` | No Tome-maintainer operating context. |
| `Summarizer` | `4979a78b-3c93-4ec6-b0a4-a8d3efc30b1a` | Built-in paused agent; no explicit curation decision. |
| `Reflection Coach` | `eaa22970-44f6-4de4-930c-3ec2abc4e003` | Built-in paused agent; no explicit curation decision. |

## Preview token and reproducibility

The final live API preview was run with an isolated scratch `TOME_HOME`:

```bash
TOME_HOME="$PAPERCLIP_RUN_SCRATCH_DIR/mco-163-preview-preserve/tome-home" \
  cargo run -p tome -- paperclip-agents preview \
  --catalog curation/paperclip-agents/catalog.toml \
  --assignments curation/paperclip-agents/agents.toml \
  --constraint paperclip-agent \
  --paperclip-api-url "$PAPERCLIP_API_URL"
```

Confirmation token from that live read-only preview:

- `apply-v2-85775d96c95c367387b98db6cea6a214`

Token metadata from the scratch confirmation store:

- Issued: 2026-10-03T12:38:25Z
- Expires: 2026-10-03T12:53:25Z
- Plan fingerprint: `b9bd18f08b306bceda817bec9c82f89560776782bda6f02fdd68cd2e17f713a6`
- Before-state fingerprint: `6602cf5a39b92ac95ccb59e764e021bb99db1a984ae9e49d49f78705b4f0bb8b`
- Bound before-state desired skills: `local/fbfe9eb2f4/rtk-command-output`

Because the preview used an isolated scratch `TOME_HOME`, the token above is
valid only from that same scratch home before its 2026-10-03T12:53:25Z expiry.
After heartbeat scratch cleanup, the token is historical evidence only. A later
approved apply must first rerun preview in the intended persistent `TOME_HOME`
and use that freshly issued matching token.

For reproducibility after the live state changes, the observed before-state snapshot is stored in `curation/paperclip-agents/preview-current-state.snapshot.json`.

## Non-mutation verification

Commands intentionally not run:

- `tome paperclip-agents apply`
- `POST /api/agents/{agentId}/skills/sync`
- `tome sync`

Preview was run with an isolated scratch `TOME_HOME`, so the confirmation token store was written only under the run scratch directory, not the user's real Tome config or canonical library.

The real API attempt is captured in `curation/paperclip-agents/preview-real-api-attempt.txt`; it exited 0 and used read-only Paperclip state before issuing the scratch-scoped token.
