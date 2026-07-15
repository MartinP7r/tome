---
created: 2026-07-15T01:10:16.896Z
title: Define agent skills for tome
area: planning
files:
  - AGENTS.md
  - .planning/PROJECT.md
  - .planning/ROADMAP.md
---

## Problem

The repository has strong AGENTS.md and GSD workflow guidance, but there is not yet a
captured work item for what repo-specific agent skills `tome` itself should expose or
standardize. That leaves future work under-specified when deciding whether the project
needs dedicated skills, clearer capture/debug/release flows, or repo-scoped guidance for
AI agents working in this codebase.

## Solution

Review the existing agent-facing surface area in this repo and decide what should become
explicit `tome` agent skills or supporting workflow docs. At minimum:

1. Inventory the current repo instructions, GSD commands, and repeated agent tasks.
2. Identify gaps where a dedicated skill would reduce ambiguity or duplicated guidance.
3. Define the minimal set of repo-specific skills, their responsibilities, and where they
   should live.
4. Capture any follow-up implementation or documentation work needed to make those skills
   usable in practice.
