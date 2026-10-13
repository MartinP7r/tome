# Development Workflow

`tome` uses a lightweight, Paperclip-led workflow:

- **Paperclip** is the authoritative work queue, roadmap, priority, and execution-state system.
- **GitHub Issues** are linked external history and repository-facing discussion; they do not automatically set current priority.
- **Repository planning documents** are optional, versioned design/implementation evidence for substantial changes.
- **Git commits and pull requests** are the implementation and review evidence.

This workflow exists for traceability, not ceremony. Use the smallest amount of written design that makes a significant change understandable and safe.

## When written planning is warranted

Create or update a repository planning document when work involves:

- a new feature or substantial refactor;
- an architecture or configuration-model change;
- a migration, security boundary, or destructive-state transition;
- an implementation sequence whose acceptance criteria need review before coding.

Small fixes—typos, isolated bugs, and mechanical cleanups—can proceed directly from a Paperclip issue to code, tests, and pull request.

## Default flow

1. Create or update the relevant **Paperclip issue**. It records the objective, priority, dependencies, current status, and the decision to start work.
2. Inspect existing repository documentation, prior PRs, tests, and linked GitHub history.
3. For significant work, add a concise, versioned design or implementation plan in the repository. Link it from the Paperclip issue and PR.
4. Implement in an issue-specific branch/worktree with focused tests and small commits.
5. Open a pull request, run the applicable quality gates, and record the PR, verification, and remaining follow-ups on the Paperclip issue.
6. Update Paperclip status only after the stated acceptance criteria have evidence.

## Roles and boundaries

### Paperclip

Paperclip answers: **what is currently authorized and prioritized, who owns it, what blocks it, and what remains?**

Use it for:

- goals, project priorities, tasks, dependencies, and execution state;
- durable status updates and completion evidence;
- links to GitHub issues, pull requests, design documents, and verification output.

### GitHub Issues

GitHub Issues answer: **what is the repository-visible historical or external context?**

Use them as linked evidence when useful, but do not let an open GitHub issue begin work or override a Paperclip decision by itself.

### Repository documents

Repository documents answer: **why is this design safe and how should it be implemented?**

Use normal Markdown under the relevant documentation or planning location. Keep a document close to the code only while it remains useful to maintainers; avoid creating a second task queue or duplicating Paperclip state.

### Git and pull requests

Git and PRs answer: **what actually changed and what verification/review evidence exists?**

## Traceability convention

For a meaningful change, include the relevant Paperclip issue identifier and link in the PR description or commit body. Add GitHub references and repository planning-document paths only when they genuinely help future readers.

Example:

```text
Paperclip: MCO-52
https://mmini.zuul-bee.ts.net:8443/MCO/issues/MCO-52
```

For a PR that implements a written design, include a compact traceability section:

```text
## Traceability

- Paperclip: MCO-52
- Design: docs/src/example-design.md
- Verification: cargo test -p tome
```

## Practical rule of thumb

- **Paperclip** = source of truth for planning and execution state
- **GitHub** = linked repository history and external discussion
- **Repository docs** = durable design/implementation context when warranted
- **Git / PR** = shipped evidence

Do not create a parallel backlog, checklist, or status system in the repository. Keep task status in Paperclip.

## CLI Stable Releases

A merged version-bump PR is not a release. Keep this process CLI-only unless
Martin explicitly reprioritizes the desktop crate:

1. Check the latest published GitHub release/tag, Paperclip blockers, breaking
   changes, and upgrade notes. Under 0.x semver, use a minor bump for breaking
   CLI changes.
2. Update the workspace version, `Cargo.lock`, and `CHANGELOG.md`; include an
   upgrade guide for config or deployment changes.
3. Run `cargo dist plan --tag vX.Y.Z`. It must list only the `tome` CLI package
   and expected CLI artifacts; `tome-desktop` remains `dist = false`. Set
   `precise-builds = true`: cargo-dist otherwise uses Cargo's workspace-wide
   build, which can compile every workspace crate (including non-distributed
   `tome-desktop`). This is build-graph coupling from the shared Cargo workspace,
   not a runtime dependency; precise builds compile only the package needed for
   each release binary. Leave Desktop source and release status unchanged.
4. Keep `publish-jobs = []`. The stock cargo-dist Homebrew job commits to the
   tap without a signature. Keep `installers = ["homebrew"]` so the formula is
   included as a release asset, then update the tap in a separate locally signed
   commit using the Bitwarden SSH agent.
5. Review the release PR and require its CLI checks before merge. Create and
   push an annotated signed `vX.Y.Z` tag only after the merged commit is read
   back from `main`; this triggers cargo-dist's GitHub release workflow.
6. Read back the completed workflow, exact release tag, and assets. Download the
   generated formula, verify its version/checksums against the release assets,
   run the formula checks, and publish it with a signed tap commit. Read that
   commit back and verify its signature before closing the Paperclip task.

Do not use `make release` for a stable cut: it auto-merges and tags without the
guarded review/read-back sequence. Do not re-enable the built-in Homebrew
publisher until it has a verified signing path.