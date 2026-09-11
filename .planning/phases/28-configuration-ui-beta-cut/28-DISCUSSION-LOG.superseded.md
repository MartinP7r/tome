# Phase 28: Configuration UI - Discussion Log

**Date:** 2026-08-09

## Decisions

- Stop desktop implementation after the Phase 28 beta cut and return to TUI/CLI work.
- Use a dedicated first-run setup screen, preserve CLI brownfield/legacy choices, and respect explicit Tome data-folder sources.
- Use a preselected discovery checklist and inline custom-directory addition.
- Use a list/detail directory editor with drag plus keyboard ordering, live validation, config-only removal, and one TOML-diff confirmation.
- Clone and validate Git sources before draft inclusion; prefill reviewed subdirectory detection; keep pinning advanced; retain failed form drafts.
- Use global plus per-directory machine preferences, searchable skill pickers, retained inactive rules for disabled directories, and one machine TOML diff confirmation.
- Put Configuration in the persistent fifth sidebar section.
