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