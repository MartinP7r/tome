# Development Workflow

`tome` uses a lightweight layered workflow for substantial changes:

- **GitHub Issues** track product intent, roadmap placement, and user-visible scope.
- **OpenSpec** tracks the change proposal, requirements, design notes, and implementation checklist.
- **GSD** (`.planning/` + `/gsd:*` commands) tracks phase and plan execution state — what's been researched, discussed, planned, executed, and verified.
- **Git commits / PRs** are the implementation evidence.

This is meant to improve traceability, not create process theater. If the workflow becomes bureaucratic sludge, scale it back.

## When to Use This Workflow

Use the full OpenSpec + GSD flow for:

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

For small fixes, issue → code → PR is fine. `/gsd:quick` or `/gsd:fast` can be used for small-but-structured work without the full planning overhead.

## Role of Each Layer

### GitHub Issues

Use GitHub Issues for:

- deciding what to work on
- roadmap / milestone planning
- discussion with humans
- repository-visible backlog management

GitHub Issues answer: **why does this work exist at all?**

### OpenSpec

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

#### Core OpenSpec flow

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

### GSD

Use GSD for:

- turning a milestone into phases, and phases into executable plans
- tracking which plans are ready, in progress, and done
- recording verification outcomes tied to implementation
- advancing STATE.md across phase transitions

GSD answers: **what should be worked on next, who's working on it, and what already landed?**

Selected artifacts under `.planning/` (list is non-exhaustive — GSD adds files as new workflows are used):

```text
.planning/
├── PROJECT.md                          # core value, constraints, decisions, requirements
├── ROADMAP.md                          # milestones, phases, status
├── REQUIREMENTS.md                     # requirement IDs and traceability
├── STATE.md                            # current focus, current phase/plan
└── phases/<NN>-<name>/
    ├── <NN>-CONTEXT.md                 # context gathered before planning
    ├── <NN>-RESEARCH.md                # technical approach research (when created)
    ├── <NN>-DISCUSSION-LOG.md          # /gsd:discuss-phase transcript (when created)
    ├── <NN>-UI-SPEC.md                 # UI/UX design contract for frontend phases (when created)
    ├── <NN>-<MM>-<slug>-PLAN.md        # executable plan per wave/task
    ├── <NN>-<MM>-<slug>-SUMMARY.md     # created when each plan completes
    └── <NN>-VERIFICATION.md            # created by the verifier when the phase completes
```

Minimal command flow used in `tome`:

```bash
# see current state and next actions
/gsd:progress

# gather phase context, then plan, then execute
/gsd:discuss-phase <N>
/gsd:plan-phase <N>
/gsd:execute-phase <N>

# verify manually when needed
/gsd:verify-work <N>

# capture follow-up ideas without leaving flow
/gsd:add-backlog "<idea>"
/gsd:note "<short note>"
```

When creating a GSD phase that implements an OpenSpec change, reference the OpenSpec change id in the phase CONTEXT.md and in commit/PR footers.

## Default Flow for Significant Changes

1. Start from a **GitHub issue** or clear idea.
2. Create an **OpenSpec change** for the substantial work.
3. Write or refine:
   - `proposal.md`
   - `design.md`
   - `tasks.md`
   - any relevant spec delta files
4. Bring the work into **GSD** by adding a phase to `.planning/ROADMAP.md` (or creating a new milestone with `/gsd:new-milestone`).
5. Use `/gsd:plan-phase` and `/gsd:execute-phase` to drive implementation. Each plan's `SUMMARY.md` becomes the per-plan closure note; the phase's `VERIFICATION.md` is the phase-level sign-off.
6. Land code in normal git commits / PRs.
7. Archive the OpenSpec change when the work is complete.

## Traceability Convention

For meaningful changes, link the layers explicitly. Don't invent a new footer shape per PR — use one of the two forms used in recent merged PRs:

**Small / incremental PRs** — a one-liner in the commit body is enough:

```text
Refs #123
OpenSpec: <change-id>
```

**Phase-closing PRs** — add a `## Traceability` section in the PR body:

```text
## Traceability

- Requirements: WHARD-04, WHARD-05, WHARD-06
- Phase artifacts: .planning/phases/05-wizard-test-coverage/
- OpenSpec: <change-id>   (if an OpenSpec change exists)
```

This gives a practical audit trail across backlog, planning, execution, and code history.

## Practical Rule of Thumb

- **GitHub Issue** = backlog / business reason
- **OpenSpec** = requirements + design + checklist
- **GSD** = execution state (phases, plans, verification)
- **git / PR** = shipped evidence

Don't stack ceremony for its own sake. Use the minimum structure needed to stop future-you from asking, "What the hell were we doing here?"
