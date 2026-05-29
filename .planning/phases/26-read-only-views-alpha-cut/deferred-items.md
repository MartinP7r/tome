# Deferred items — phase 26-read-only-views-alpha-cut

## 2026-05-29 — plan 26-08 fmt drift in unrelated files

Cargo fmt surfaced pre-existing formatting drift in `crates/tome/src/doctor.rs` and `crates/tome/src/skill.rs` (unrelated to plan 26-08's surface). Per executor scope-boundary rules these are NOT auto-fixed here — log only.

- `crates/tome/src/doctor.rs:1486` — `FindingId::LibraryStaleManifest { skill: name.clone() }` should split across multiple lines.
- `crates/tome/src/doctor.rs:1529` — `FindingId::LibraryBrokenSymlink { path: path.clone() }` should collapse onto one line.
- `crates/tome/src/doctor.rs:1569` — `FindingId::UnparsableFrontmatter { skill: name.clone() }` should split across multiple lines.
- `crates/tome/src/doctor.rs:3690` — assertion line should collapse onto one line.
- `crates/tome/src/skill.rs:183` — `Ok(json) =>` arm should collapse onto one line.

CI's `cargo fmt --all -- --check` is currently passing on `main`, so this drift was introduced by an unrelated landing between the last fmt sweep and now. Filing as a follow-up cleanup rather than touching the files in this plan.
- `crates/tome-desktop/src/commands.rs:13-15` — `use tome::SkillName;` should sort before `use tome::TomePaths;` alphabetically.
