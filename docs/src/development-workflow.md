# Development Workflow

`tome` uses a lightweight layered workflow for substantial changes:

- **GitHub Issues** track product intent, roadmap placement, and user-visible scope.
- **OpenSpec** tracks the change proposal, requirements, design notes, and implementation checklist.
- **Beads** tracks live execution state: what is ready, what is claimed, what is blocked, and what is done.
- **Git commits / PRs** are the implementation evidence.

This is meant to improve traceability, not create process theater. If the workflow becomes bureaucratic sludge, scale it back.

## When to Use This Workflow

Use the full OpenSpec + Beads flow for:

- new features
- significant refactors
- architecture-impacting changes
- process or documentation changes that affect future development behavior
- any change where requirements/design should be reviewed before implementation

You do **not** need the full workflow for:

- typo fixes
- tiny bug fixes with obvious scope
- narrowly scoped internal cleanups
- mechanical edits with no design impact

For small fixes, issue → code → PR is fine.

## Role of Each Layer

## GitHub Issues

Use GitHub Issues for:

- deciding what to work on
- roadmap / milestone planning
- discussion with humans
- repository-visible backlog management

GitHub Issues answer: **why does this work exist at all?**

## OpenSpec

Use OpenSpec for:

- change proposals
- requirement deltas
- design decisions
- implementation checklists for substantial changes

OpenSpec answers: **what are we changing, and why does the shape of the change make sense?**

Typical artifact layout:

```text
openspec/changes/<change-id>/
├── proposal.md
├── design.md
├── tasks.md
└── specs/<capability>/spec.md
```

### Core OpenSpec flow

```bash
# create a new change scaffold
openspec new change <change-id>

# inspect what exists
openspec list
openspec show <change-id>
openspec status --change <change-id>

# validate before implementation / archival
openspec validate <change-id>

# after implementation is complete
openspec archive <change-id>
```

## Beads

Use Beads for:

- turning an OpenSpec task checklist into live executable tasks
- claiming work
- tracking dependencies / blocking relationships
- recording closure notes tied to implementation

Beads answers: **what should be worked on next, who owns it, and what already landed?**

Minimal command flow used in `tome`:

```bash
# see unblocked work
bd ready

# inspect a task
bd show <task-id>

# claim work
bd update <task-id> --claim

# close work with an implementation note
bd close <task-id> "Done in commit <sha>"
```

When creating Beads tasks from an OpenSpec change, set `spec_id` to the OpenSpec change id.

## Default Flow for Significant Changes

1. Start from a **GitHub issue** or clear idea.
2. Create an **OpenSpec change** for the substantial work.
3. Write or refine:
   - `proposal.md`
   - `design.md`
   - `tasks.md`
   - any relevant spec delta files
4. Create **Beads** tasks for the executable work items.
5. Use `bd ready` / `bd update --claim` / `bd close` during implementation.
6. Land code in normal git commits / PRs.
7. Archive the OpenSpec change when the work is complete.

## Traceability Convention

For meaningful changes, link the layers explicitly.

### In Beads

- set `spec_id` to the OpenSpec change id
- use task descriptions that reference the actual repo artifact being changed
- close tasks with a note that includes the commit hash when possible

### In commits or PR descriptions

Include the IDs when they exist:

```text
Refs #123
OpenSpec: document-openspec-beads-workflow
Beads: tome-8vs.1
```

Recommended PR footer shape:

```text
Closes #123
OpenSpec: <change-id>
Beads: <task-id>[, <task-id>...]
```

This gives a practical audit trail across backlog, planning, execution, and code history.

## Practical Rule of Thumb

- **GitHub Issue** = backlog / business reason
- **OpenSpec** = requirements + design + checklist
- **Beads** = execution state
- **git / PR** = shipped evidence

Don’t stack ceremony for its own sake. Use the minimum structure needed to stop future-you from asking, “What the hell were we doing here?”
