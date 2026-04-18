# Feature Landscape

**Domain:** Interactive CLI setup wizard for config tool (tome init)
**Researched:** 2026-04-16
**Confidence:** HIGH (code review + domain patterns)

## Current State Assessment

The wizard rewrite is **substantially complete**. All five Active items from PROJECT.md have been implemented in `wizard.rs`:

| Feature | Status | Evidence |
|---------|--------|----------|
| Merged KNOWN_DIRECTORIES registry | Done | `wizard.rs:41` -- 11-entry `KNOWN_DIRECTORIES` const |
| Auto-discovery with role auto-assignment | Done | `find_known_directories_in()` + `default_role` per entry |
| Summary table (name, path, type, role) | Done | `show_directory_summary()` with formatted columns |
| Custom directory addition with role selection | Done | Loop at line 227 with type + role pickers |
| Remove find_source_target_overlaps() dead code | Done | grep confirms zero references in codebase |

The remaining v0.7 work is polish, testing, and minor UX refinements -- not a ground-up rewrite.

## Table Stakes

Features users expect from an interactive CLI wizard. Missing = setup feels broken or confusing.

| Feature | Why Expected | Complexity | Status | Notes |
|---------|--------------|------------|--------|-------|
| Auto-discover existing directories | Users should not type paths the tool can find | Low | Done | `find_known_directories_in()` scans 11 known paths |
| Pre-select all discovered dirs | Convention from ESLint, npm init -- discovered = wanted | Low | Done | `defaults(&vec![true; found.len()])` |
| Role explanation before prompts | Users need to understand Managed/Synced/Source/Target before choosing | Low | Done | Printed at wizard start with descriptions |
| Summary table before write | Review-before-commit is standard wizard UX (ESLint, Cargo) | Low | Done | `show_directory_summary()` |
| Save confirmation prompt | Never write config without explicit consent | Low | Done | `Confirm::new().with_prompt("Save configuration?")` |
| Dry-run mode | Let users preview without side effects | Low | Done | `--dry-run` prints generated TOML without saving |
| Custom directory addition | Not all dirs are in the known registry | Low | Done | Loop with name/path/type/role prompts |
| Exclusion picker | Fine-grained control over what gets synced | Med | Done | `configure_exclusions()` with MultiSelect |
| Library location picker | Users may want non-default library paths | Low | Done | `configure_library()` with Select + Input fallback |
| Role editing after summary | Fix mistakes without restarting the wizard | Low | Done | "Would you like to edit any directory's role?" loop |

## Differentiators

Features that elevate the wizard from functional to polished. Not expected, but valued.

| Feature | Value Proposition | Complexity | Status | Notes |
|---------|-------------------|------------|--------|-------|
| Valid-role filtering by type | Prevents invalid combos (e.g. Git+Target, Directory+Managed) | Low | Done | `directory_type.valid_roles()` gates role picker |
| ClaudePlugins locked to Managed | Prevents user error on immutable source | Low | Done | Filtered out of editable list |
| Tilde-collapsed paths in display | Readable paths matching config file format | Low | Done | Uses `collapse_home_path()` |
| Terminal-height-aware lists | Prevents scroll overflow on small terminals | Low | Done | `max_rows` from `Term::stderr().size()` |
| Git backup init offer | Encourages backup practice post-setup | Low | Done | Offered after config save |
| Summary table via `tabled` crate | Proper column alignment with borders, visual consistency with `tome list`/`tome status` | Low | Not done | Current: manual `format!` padding. `tabled` already in deps |
| Existing config detection | Warn if config exists, offer merge/overwrite/abort | Med | Not done | Currently overwrites silently on confirm |
| Path existence warning | Warn when custom path does not exist to catch typos | Low | Not done | `Input` accepts any string without validation |
| Role recommendation hints | Show "Recommended: Synced" next to options based on type default | Low | Not done | `default_role()` exists but is not surfaced in picker UI |
| Post-init next-steps message | Print "Run `tome sync` to populate" after save -- standard pattern (ESLint, npm, cargo) | Low | Not done | Silent after save currently |
| Skill count per directory in summary | Show how many skills were discovered per dir before confirming | Low | Not done | Discovery already runs before exclusions step |

## Anti-Features

Features to explicitly NOT build. Each would add complexity without proportional value.

| Anti-Feature | Why Avoid | What to Do Instead |
|--------------|-----------|-------------------|
| TUI-based wizard (ratatui) | Over-engineered for one-time setup; dialoguer is the correct abstraction for linear prompts | Keep dialoguer prompts |
| Git URL input in wizard | Git sources need branch/tag/rev/subdir -- too many fields for wizard flow | Use `tome add <url>` post-init |
| Undo/back navigation | dialoguer does not support back-navigation; linear wizards work fine for 4-5 steps | Let users re-run `tome init` or edit config manually |
| Config migration from old format | Single user, hard break documented in CHANGELOG | Already handled |
| Full home directory scan | Scanning entire $HOME is slow, produces false positives | Keep curated KNOWN_DIRECTORIES registry |
| Template-based config generation | Over-engineering; TOML serialization from Config struct is sufficient | Keep `config.save()` |
| Remote skill repo browsing | Network dependency in init is fragile | Suggest `tome add` in post-init message |
| Interactive config editor (re-open wizard for existing config) | Scope creep; `tome init` is for first setup, manual TOML editing works for changes | Document TOML format in docs |

## Feature Dependencies

```
Auto-discovery --> Summary table (need dirs to display)
Summary table --> Role editing loop (edit what you see)
Role editing loop --> Custom directory addition (add after reviewing)
Custom directory addition --> Summary table refresh (show updated state)
All above --> Save confirmation --> Config write
Config write --> Git backup init offer (only if config saved)
Exclusion picker depends on: directory selection (discovers skills from selected dirs)

Polish features (no hard dependencies on each other):
  tabled summary -- standalone
  existing config detection -- standalone (check before step 1)
  path existence warning -- standalone (in custom dir addition)
  post-init message -- standalone (after save)
  role recommendation hints -- standalone (in role picker)
```

## Polish Candidates (v0.7 scope)

Since the core rewrite is done, v0.7 should focus on polish items:

### P1: Summary table with `tabled` (Low complexity)
Replace manual `format!`-based table with the `tabled` crate already in dependencies. Gives proper column alignment, borders, and visual consistency with `tome list` and `tome status`.

**Dependency:** None -- `tabled` already a dependency.

### P2: Post-init next-steps message (Low complexity)
After saving config, print actionable guidance: "Run `tome sync` to populate your library" and "Run `tome add <url>` to add git skill repos." Every major CLI tool does this (ESLint prints next steps, `cargo init` suggests `cargo run`, `npm init` says "run `npm install`").

**Dependency:** None.

### P3: Custom path existence warning (Low complexity)
When user types a custom directory path, check if it exists. If not, show: "Path does not exist. It will be created during sync. Continue?" Catches typos before they become bad config entries.

**Dependency:** None.

### P4: Existing config detection (Low-Med complexity)
When `tome.toml` already exists, warn and offer: overwrite, abort, or continue (merge discovered dirs). Prevents accidental config loss on re-init. `Config::load_or_default()` already provides the loading infrastructure.

**Dependency:** Config loading infrastructure (exists).

### P5: Role recommendation hints (Low complexity)
In role picker for custom directories, highlight the default: "Synced (recommended for directory type)" vs just "Synced". The `default_role()` method exists but is not surfaced in the picker UI.

**Dependency:** None.

### P6: Skill count in summary table (Low complexity)
Show discovered skill count per directory in the summary table. Discovery already runs between step 1 and step 3 -- the data is available. Helps users judge whether directories are correctly configured before saving.

**Dependency:** Discovery results (already computed at line 148).

### P7: Integration tests for non-interactive parts (Med complexity)
Test discovery, summary rendering, and config generation in isolation. dialoguer requires a TTY so full wizard flow cannot be integration-tested, but the helper functions (`find_known_directories_in`, `show_directory_summary`, config struct assembly) are testable.

**Dependency:** Test infrastructure.

## MVP Recommendation

**Ship (low effort, high impact):**
1. P1: `tabled` summary table -- visual consistency with rest of CLI
2. P2: Post-init next-steps message -- standard UX pattern
3. P3: Custom path existence warning -- prevents typos

**Consider (medium effort, medium impact):**
4. P4: Existing config detection -- prevents data loss
5. P6: Skill count in summary -- better informed decisions
6. P5: Role recommendation hints -- reduces confusion

**Defer:**
7. P7: Integration tests -- valuable but not blocking; dialoguer testability is a known limitation

## Sources

- [ESLint CLI init wizard](https://eslint.org/docs/latest/use/command-line-interface) -- `--init` pattern: prompts, auto-detect, config generation, next-steps message
- [chezmoi setup](https://www.chezmoi.io/user-guide/setup/) -- dotfile manager init flow
- [dialoguer MultiSelect](https://docs.rs/dialoguer/latest/dialoguer/struct.MultiSelect.html) -- prompt library API
- [tabled crate](https://docs.rs/crate/tabled/latest) -- table formatting, already in tome deps
- [Progressive Disclosure (NN/G)](https://www.nngroup.com/articles/progressive-disclosure/) -- wizard UX: staged reveal of complexity
- Code review: `crates/tome/src/wizard.rs` -- 602 lines, fully implemented unified model
- Code review: `crates/tome/src/config.rs` -- DirectoryRole, DirectoryType, valid_roles(), default_role()
