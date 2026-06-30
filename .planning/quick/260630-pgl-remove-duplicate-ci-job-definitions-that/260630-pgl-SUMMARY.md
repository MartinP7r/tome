---
status: complete
quick_id: 260630-pgl
commit: f80e47d
---

# CI workflow duplicate-job fix

Removed stale duplicate definitions of the `bindings`, `desktop-build`, and
`a11y` jobs from `.github/workflows/ci.yml`. The retained definitions use the
current action versions from the dependency updates.

## Verification

- `actionlint .github/workflows/ci.yml` passed.
- `git diff --check` passed.
- GitHub's preceding failed runs had zero jobs, consistent with workflow YAML
  rejection before scheduling.

Implementation commit: `f80e47d`.
