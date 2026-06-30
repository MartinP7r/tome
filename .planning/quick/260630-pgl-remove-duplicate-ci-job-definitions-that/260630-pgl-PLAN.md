---
quick_id: 260630-pgl
status: complete
---

# Remove duplicate CI job definitions that invalidate ci.yml

## Task 1: Restore a valid CI workflow

- Remove the stale duplicate `bindings`, `desktop-build`, and `a11y` job definitions from `.github/workflows/ci.yml`.
- Retain the current action versions (`actions/checkout@v7`, `actions/setup-node@v6`).
- Verify the workflow with `actionlint` and confirm every job identifier is unique.

## Task 2: Verify and publish

- Record the fix in GSD quick-task state.
- Commit and push the correction, then confirm the remote CI run starts with jobs.
