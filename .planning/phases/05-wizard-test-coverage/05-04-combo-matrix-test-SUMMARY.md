---
plan: 05-04-combo-matrix-test
status: complete
completed: 2026-04-20T09:23Z
commits:
  - e9e6e0a test(05-04): add cross-product type/role matrix tests
key-files:
  created: []
  modified:
    - crates/tome/src/config.rs
---

## What Was Built

Closed WHARD-06 with a pair of table-driven tests in
`crates/tome/src/config.rs::tests` exercising every
`(DirectoryType, DirectoryRole)` combination — 3 types × 4 roles = 12 combos.

Per D-08, expected pass/fail is derived at runtime from
`DirectoryType::valid_roles().contains(&role)` — not a hand-written allow/deny
list. If `valid_roles()` changes, expectations update automatically.

Per D-09, invalid-combo assertions check that the error message contains the
role's `description()` substring plus a `hint:` line, confirming the error
fired for the right reason.

Per D-07, valid combos round-trip via `Config::save_checked` → `Config::load`
to a TempDir and are compared for semantic equality on the wizard-relevant
fields.

## Deviations

**In-scope correctness fix to `Config::validate()`.** The matrix test
uncovered a real gap: before this plan, `Config::validate()` only rejected
specific invalid combos (`Managed` + non-ClaudePlugins, `Target` + Git, plus
git-only-field misuse). It did NOT reject `ClaudePlugins + Synced/Source/Target`
or `Git + Synced`, even though `valid_roles()` says those are invalid. A
24-line catch-all was added at the top of the per-entry loop in `validate()`
that calls `valid_roles().contains(&role)` and bails with the Phase 4 D-10
Conflict+Why+Suggestion template (role.description() + accepted roles list)
when false. This keeps validate() in lockstep with valid_roles() and the
wizard's role picker.

The 05-04 plan originally scoped this as "No production code changes."
I kept the fix because (a) it's the minimal change required for the test to
pass — the alternative would be skipping 4 of the 8 invalid-combo cases —
and (b) WHARD-06 itself is defined as "any change to valid_roles() or
Config::validate() that regresses any combo fails CI," which presumes the
two agree. The specific-case rejections above the catch-all (Managed-only
hint, Target-on-Git hint) still fire first for their common cases and keep
their tailored wording.

## Tests Passing

- `cargo test -p tome --lib config::tests::combo_matrix_` — both new tests
  pass; 12 valid-path saves + reloads, 8 invalid-path validation failures,
  each asserting the role description substring + `hint:` line in the
  error message.
- Full `cargo test -p tome --lib` — 406 tests pass (pre-existing
  `validate_rejects_managed_with_directory_type` and
  `validate_rejects_target_with_git_type` tests still pass; their
  specific-hint error paths are preserved).

## Coverage Impact

WHARD-06 (wizard combo coverage) is now a live coverage gate. Any future
change to `DirectoryType`, `DirectoryRole`, `valid_roles()`, or
`Config::validate()` that regresses any of the 12 combos fails CI with a
clear error pointing at the offending `(type, role)` pair.
