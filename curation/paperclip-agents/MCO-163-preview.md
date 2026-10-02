# MCO-163 preview-only curation run

Date: 2026-10-02

## Scope and safety boundary

- Preview-only run for MCO-163.
- No `tome paperclip-agents apply` invocation.
- No `skills:sync` invocation.
- No canonical-library mutation.
- No model calls.
- Paperclip API use was limited to read-only `GET` requests for agent roster, company skill catalog, OpenAPI schema, and attempted agent skill state reads.

## Repository baseline inspected

- Current checkout at curation start: `8e6ec220280444de9ac772eb4df7e869cb95d7f9` (`paperclip/MCO-158-agent-skill-materialization`).
- `origin/main` inspected: `392883ad43e8729550e4324368bb08981943b696` (`feat: add Paperclip agent skill materialization (#618)`).
- The inherited branch and `origin/main` diverged from merge base `8500a725caed06930c842f51d2ea052b6d59e37a`; this curation was isolated onto `paperclip/MCO-163-preview-curation`.

## Canonical skill inventory

Observed live Tome library:

| Skill | Location | Digest evidence | Decision |
|---|---|---:|---|
| `demo-skill` | `~/.tome/skills/demo-skill` | `903d4a7d7948d35159f315d091af0b97c89110f5713b2125ff98d57180e3f1bb` | Omitted. Frontmatter says `E2E fixture`, so assigning it to a Paperclip agent would be speculative. |

Observed repo-owned official Tome skill:

| Skill | Location | Digest evidence | Decision |
|---|---|---:|---|
| `using-tome` | `skills/using-tome` | `a1d9e649724f95e39e71884ea5ebd5a28ffc09bfe97a99bf57b794e4aac127f5` | Included in `tome-maintainer` because it is the official setup/sync/recovery skill offered by `tome init`. |

Paperclip company skill catalog read returned 29 skills. `using-tome` was not present in that read-only catalog, so eventual apply approval should first confirm/import the company skill identity that `company_skill = "using-tome"` is meant to address.

## Proposed catalog and assignment

- Catalog: `curation/paperclip-agents/catalog.toml`
- Assignment registry: `curation/paperclip-agents/agents.toml`
- Catalog SHA-256: `1d245482f66e28dcceb3380161b91dc906282d232514e5985abccb71eb6214df`
- Assignment SHA-256: `81af53d2b2a45888ae9041ac027df7eaef6835578d29ad5d9ed6ef36675ce907`
- Proposed set: `tome-maintainer`
- Assigned agent: `FoundingEngineer` (`ecadbdc0-fcf5-4d18-b62f-111999f269c0`)
- Proposed skill addition: `using-tome`

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

## Paperclip read boundary

The roster endpoint was readable:

- `GET /api/companies/{companyId}/agents` -> HTTP 200, 9 agents.

The command's real read-back endpoint was not usable with this run token:

- `GET /api/agents/{id}/skills` -> HTTP 403 for every agent checked.
- Error text: `Missing permission: agents:suggest-changes.`

Because current desired/runtime skills were unavailable, `preview-current-state.snapshot.json` is a documented fallback snapshot for the assigned agent only, with empty desired skills and unavailable runtime skills. This makes the preview impact deterministic but not an assertion that the live agent currently has no desired skills.

## Preview artifact

Preview output is stored in `curation/paperclip-agents/preview-output.txt`.

Confirmation token from the recorded isolated fallback preview artifact:

- `apply-v2-d8e72af27dc8e0a26de40395569ffea7`

The preview impact is deterministic for this catalog, assignment registry, constraints, and fallback state snapshot. The confirmation token itself is minted per preview run, stored in the active Tome config home, and expires after 15 minutes. Because this proof used an isolated scratch `TOME_HOME` to avoid mutating real `~/.tome`, any approved future `apply` should first rerun preview in its intended persistent `TOME_HOME` and use that freshly issued matching token.

Expected fleet impact from the fallback snapshot:

- `FoundingEngineer`: add `using-tome`; remove none; keep none.
- All omitted agents: unchanged by this registry because they are not assigned.

## Non-mutation verification

Commands intentionally not run:

- `tome paperclip-agents apply`
- `skills:sync`
- `tome sync`

Preview was run with an isolated scratch `TOME_HOME`, so the confirmation token store was written only under the run scratch directory, not the user's real Tome config or canonical library.

The real API attempt is captured in `curation/paperclip-agents/preview-real-api-attempt.txt`; it failed before token issuance and left no scratch confirmation files.
