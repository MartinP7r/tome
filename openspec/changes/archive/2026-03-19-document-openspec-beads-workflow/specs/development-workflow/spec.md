## ADDED Requirements

### Requirement: Significant development changes are documented with a workflow artifact
The repository SHALL define a documented workflow for significant development changes that require planning before implementation.

#### Scenario: Feature work needs planning guidance
- **WHEN** a contributor prepares a new feature, significant refactor, architecture-impacting change, or process change affecting future development
- **THEN** the repository provides guidance describing how to plan and track that work

### Requirement: The workflow distinguishes planning from execution tracking
The repository SHALL explain the distinct roles of OpenSpec and Beads so contributors know which system to update.

#### Scenario: Contributor needs to know where to record work
- **WHEN** a contributor is deciding how to track a significant change
- **THEN** the documentation explains that OpenSpec is used for requirements, design, and task checklists
- **AND** the documentation explains that Beads is used for execution state, dependencies, and task completion tracking

### Requirement: Coding agents are instructed to follow the repository workflow
The repository SHALL provide agent-facing instructions for using the documented workflow during significant changes.

#### Scenario: Agent starts a substantial change
- **WHEN** a coding agent begins a substantial change in the repository
- **THEN** the repository instructions describe when to create or update an OpenSpec change
- **AND** the repository instructions describe how to use Beads for task execution tracking

### Requirement: Small fixes can bypass the heavier workflow
The repository SHALL preserve a lightweight path for minor changes that do not justify full planning overhead.

#### Scenario: Contributor fixes a trivial issue
- **WHEN** a contributor makes a typo fix, tiny bug fix, or narrowly scoped non-architectural change
- **THEN** the documentation states that the full OpenSpec + Beads workflow is optional
