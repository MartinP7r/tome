# development-workflow Specification

## Purpose
Document the layered workflow tome uses for substantial changes so contributors and coding agents know when to use GitHub Issues, OpenSpec, and GSD — and how those layers connect to commits and PRs. The full workflow is opt-in for non-trivial work; small fixes follow a lightweight issue → code → PR path.

## Requirements
### Requirement: Significant development changes are documented with a workflow artifact
The repository SHALL define a documented workflow for significant development changes that require planning before implementation.

#### Scenario: Feature work needs planning guidance
- **WHEN** a contributor prepares a new feature, significant refactor, architecture-impacting change, or process change affecting future development
- **THEN** the repository provides guidance describing how to plan and track that work

### Requirement: The workflow distinguishes planning from execution tracking
The repository SHALL explain the distinct roles of OpenSpec and GSD so contributors know which system to update.

#### Scenario: Contributor needs to know where to record work
- **WHEN** a contributor is deciding how to track a significant change
- **THEN** the documentation explains that OpenSpec is used for requirements, design, and task checklists
- **AND** the documentation explains that GSD (`.planning/` artifacts and `/gsd:*` commands) is used for phase/plan execution state and verification tracking

### Requirement: Coding agents are instructed to follow the repository workflow
The repository SHALL provide agent-facing instructions for using the documented workflow during significant changes.

#### Scenario: Agent starts a substantial change
- **WHEN** a coding agent begins a substantial change in the repository
- **THEN** the repository instructions describe when to create or update an OpenSpec change
- **AND** the repository instructions describe how to use GSD phases and plans for execution tracking (`/gsd:plan-phase`, `/gsd:execute-phase`, `/gsd:verify-work`)

### Requirement: Small fixes can bypass the heavier workflow
The repository SHALL preserve a lightweight path for minor changes that do not justify full planning overhead.

#### Scenario: Contributor fixes a trivial issue
- **WHEN** a contributor makes a typo fix, tiny bug fix, or narrowly scoped non-architectural change
- **THEN** the documentation states that the full OpenSpec + GSD workflow is optional
- **AND** the documentation mentions `/gsd:quick` or `/gsd:fast` as a lightweight structured path when some tracking is still desirable
