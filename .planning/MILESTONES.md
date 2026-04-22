# Milestones

## v0.7 Wizard Hardening (Shipped: 2026-04-22)

**Phases completed:** 3 phases, 9 plans, 8 tasks

**Key accomplishments:**

- All four Config::validate() bail! bodies rewritten to the D-10 Conflict+Why+Suggestion template with DirectoryRole::description() used for every role mention
- Config::validate() now rejects every path relation where library_dir overlaps a distribution directory — equality, nesting either direction — using lexical, tilde-aware, trailing-separator-normalized comparison
- Config::save_checked enforces expand → validate → TOML round-trip → write; wizard save + dry-run now share the same pipeline so invalid configs never reach disk
- Two `assert_cmd` integration tests drive `tome init --dry-run --no-input` end-to-end against empty and seeded TempDir HOMEs, proving the generated Config validates and round-trips through TOML byte-equal.
- In-scope correctness fix to `Config::validate()`.
- Migrated `wizard::show_directory_summary` from manual `println!` column formatting to `tabled::Table` with `Style::rounded()` borders, `PriorityMax::right()` truncation, and an 80-column non-TTY fallback.
- Closed the v0.7 doc half of WHARD-08: PROJECT.md now explicitly marks WIZ-01–05 as shipped-in-v0.6 and hardened-in-v0.7 (Phases 4+5), stale "Known Gaps (deferred from v0.6)" subsection removed, footer dated 2026-04-21, CHANGELOG cites WHARD-07 + WHARD-08 under [Unreleased].

---

## v0.6 Unified Directory Model (Shipped: 2026-04-16)

**Phases completed:** 3 phases, 11 plans, 19 tasks

**Key accomplishments:**

- Unified directory type system (DirectoryName/Type/Role/Config) replacing Source/TargetName/TargetConfig with deny_unknown_fields, migration hint, validation, and convenience iterators
- Four pipeline modules (discover, distribute) rewritten for unified directory model with manifest-based circular prevention replacing shares_tool_root()
- Unified directory terminology in manifest, lockfile, machine prefs, status, and doctor -- disabled_directories replaces disabled_targets, DirectoryStatus replaces SourceStatus/TargetStatus
- Self-contained git.rs module with clone/update/SHA-reading plus subdir config field and repos_dir path method
- RED:
- Git directory clone/update wired as pre-discovery sync step with per-directory skill filtering in distribution
- `tome remove` command with full source cleanup: symlinks, library dirs, manifest entries, config save, and lockfile regeneration
- Three new CLI commands (add, reassign, fork) for git repo registration and skill provenance management
- Terminal-adaptive theming, fuzzy match highlighting, scrollbar, markdown preview rendering, and help overlay for the browse TUI

---
