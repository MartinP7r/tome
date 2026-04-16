# Milestones

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
