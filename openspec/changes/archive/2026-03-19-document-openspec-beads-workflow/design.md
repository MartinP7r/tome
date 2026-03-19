## Context

The repository now includes two complementary workflow tools:

- **OpenSpec** for change proposals, requirements, design notes, and task checklists
- **Beads** for durable execution tracking, dependency management, and session-to-session task state

The missing piece is operational guidance. `tome` already has strong project documentation (`README.md`, `ROADMAP.md`, `CLAUDE.md`, architecture docs), so the cleanest next step is to add one focused workflow document and link to it from the places developers and agents already read.

## Goals / Non-Goals

**Goals:**
- Explain the role of OpenSpec in this repository
- Explain the role of Beads in this repository
- Define a small default workflow from change proposal → execution → completion
- Keep the workflow lightweight enough for a solo-maintainer Rust CLI project
- Make agent expectations explicit in `CLAUDE.md`

**Non-Goals:**
- Mandate OpenSpec for every typo fix or tiny refactor
- Replace GitHub Issues, milestones, or roadmap planning
- Introduce automation that syncs OpenSpec tasks into Beads automatically
- Reorganize existing project documentation beyond necessary links and instructions

## Decisions

### 1. Add a dedicated development workflow doc
Create `docs/src/development-workflow.md` as the canonical explanation of how OpenSpec and Beads are used in `tome`.

Why:
- avoids bloating `README.md`
- keeps durable project process docs near other mdBook source material
- gives agents and humans one canonical reference

### 2. Keep README guidance short
Add only a short pointer in `README.md` so the top-level repo page stays focused on product value and user-facing usage.

### 3. Update CLAUDE.md with explicit workflow rules
`CLAUDE.md` should tell coding agents when to:
- create or update an OpenSpec change
- create Beads tasks from the change's `tasks.md`
- use Beads for claim/ready/close flow during implementation

This is the highest-leverage place to reinforce behavior.

### 4. Treat OpenSpec as planning and Beads as execution state
The repo should not blur the two systems:
- OpenSpec owns **what/why/design/checklist**
- Beads owns **ready work, dependencies, claims, and closure notes**

### 5. Start lightweight
The default workflow should only require OpenSpec for:
- new features
- significant refactors
- architecture-impacting changes
- documentation/process changes that affect future development behavior

Small fixes can still go straight to issue → code → PR.

## Risks / Trade-offs

- **Risk: workflow theater.** If the process is too heavy, it will be ignored.
  - Mitigation: keep the documented default lightweight and explicitly exempt small fixes.

- **Risk: duplicate planning state across GitHub Issues, OpenSpec, and Beads.**
  - Mitigation: define clear responsibilities for each system.

- **Risk: Beads adoption stays shallow.**
  - Mitigation: document a minimal command set (`bd ready`, `bd create`, `bd update --claim`, `bd close`) rather than the whole tool.
