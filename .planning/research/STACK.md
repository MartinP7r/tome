# Technology Stack: Wizard Rewrite

**Project:** tome v0.7 -- Wizard Rewrite
**Researched:** 2026-04-16

## Recommendation: No New Dependencies

The wizard rewrite requires **zero new crate additions**. Every capability needed is already in the dependency tree. The work is about using existing dependencies better and cleaning up the wizard code.

## Current Stack (Already Validated, All at Latest Versions)

| Technology | Version | Purpose | Status |
|------------|---------|---------|--------|
| `dialoguer` | 0.12.0 (latest) | Interactive prompts (MultiSelect, Select, Input, Confirm) | Already used in wizard |
| `tabled` | 0.20.0 (latest) | ASCII table rendering | Used in `status.rs` and `lib.rs` but NOT in wizard |
| `console` | 0.16.3 (latest) | Terminal colors, styling, terminal size detection | Already used in wizard |
| `dirs` | 6.x | Home directory detection for auto-discovery | Already used |

## Changes Within Existing Dependencies

### 1. Use `tabled` for Summary Table (HIGH priority)

**Current state:** `show_directory_summary()` in `wizard.rs` uses manual `println!` with fixed-width format strings (`{:<20}`, `{:<35}`, etc.). This breaks when directory names or paths exceed the column width, producing misaligned output.

**Recommended:** Replace with `tabled::Table` using the same pattern already established in `status.rs`:

```rust
use tabled::settings::{Modify, Style, object::Rows};

let rows: Vec<[String; 4]> = directories
    .iter()
    .map(|(name, cfg)| {
        [
            name.to_string(),
            cfg.path.display().to_string(),
            cfg.directory_type.to_string(),
            cfg.role().description().to_string(),
        ]
    })
    .collect();

let table = tabled::Table::from_iter(
    std::iter::once(["Name".into(), "Path".into(), "Type".into(), "Role".into()])
        .chain(rows),
)
.with(Style::rounded())
.with(Modify::new(Rows::first()).with(
    tabled::settings::Format::content(|s| s.to_uppercase()),
));
println!("{table}");
```

**Why:** Consistent with existing table rendering in the codebase. Handles variable-width content automatically. Zero additional compile cost since `tabled` 0.20 is already linked. The `Style::rounded()` matches the visual language used elsewhere in tome's CLI output.

**Confidence: HIGH** -- Verified this pattern works by examining existing usage in `status.rs` lines 185-191 and `lib.rs` lines 1128-1134.

### 2. `dialoguer` Feature Flags -- No Changes Needed

**Evaluated and rejected:**

| Feature | Purpose | Verdict |
|---------|---------|---------|
| `fuzzy-select` | Enables `FuzzySelect` prompt with inline filtering | **Reject** -- single-select only. The exclusion picker (Step 3) needs multi-select. Current `MultiSelect` with `max_length` is correct. |
| `history` | Input prompt history tracking | **Reject** -- wizard is run once; history has no value. |
| `completion` | Tab-completion for input prompts | **Reject** -- path input (custom directory) could benefit, but adds complexity for a rarely-used feature. Shell tab-completion already works for path entry. |
| `editor` | Launch external editor | **Reject** -- no text editing in the wizard flow. |

**Confidence: HIGH** -- Reviewed all dialoguer 0.12.0 features against wizard requirements.

### 3. `dialoguer` Theme Customization -- Keep Default

**Current state:** Uses default theme (no explicit `ColorfulTheme`).

**Recommendation:** Keep the default theme. The wizard already uses `console::style()` for headers, dividers, and status messages. Adding a custom dialoguer theme would create visual inconsistency. The default dialoguer theme pairs naturally with `console` styling.

**Confidence: HIGH** -- This is a stylistic decision, not a technical one.

## What NOT to Add

| Library | Why Not |
|---------|---------|
| `inquire` | Alternative to dialoguer. Would add a second prompt library for identical functionality. dialoguer 0.12 covers all wizard needs. |
| `cliclack` | Beautiful prompts but opinionated visual style that clashes with the existing console-based design language. |
| `comfy-table` | Alternative to tabled. The project already uses tabled 0.20; switching would be churn for zero benefit. |
| `term-table` | Unmaintained alternative to tabled. |
| `skim` / `fzf` crates | Overkill for the exclusion picker. MultiSelect with max_length is sufficient for the expected number of skills (10-50). |
| `termimad` | Markdown terminal rendering. Not needed for wizard; browse TUI already handles rich display with ratatui. |

## Auto-Discovery -- No Stack Changes Needed

The auto-discovery pattern in `find_known_directories_in()` is well-implemented:

- Uses `std::fs::metadata()` (not `path.is_dir()`) to surface permission errors as warnings
- Iterates a static `KNOWN_DIRECTORIES` registry with compiled-in defaults
- Returns found directories paired with their registry metadata (type, role, display name)

Any expansion is data-only (adding entries to `KNOWN_DIRECTORIES`). No library changes required.

## Integration Points (All Stable)

| Module | Interface Used | Change Needed |
|--------|---------------|---------------|
| `config.rs` | `Config`, `DirectoryConfig`, `DirectoryName`, `DirectoryRole`, `DirectoryType` | None -- unified model in place since v0.6 |
| `discover.rs` | `discover_all()`, `SkillName`, `DiscoveredSkill` | None -- used for exclusion picker |
| `paths.rs` | `collapse_home_path()` | None |
| `backup.rs` | `init()` | None |
| `tabled` (existing dep) | `Table::from_iter()`, `Style::rounded()` | Import into wizard.rs (currently unused there) |

## Installation

```bash
# No changes to Cargo.toml needed.
# All dependencies are already declared at their latest versions.
```

## Summary

The wizard rewrite is a **code-level refactor**, not a dependency change. The only action item is importing `tabled` into `wizard.rs` for the summary table -- a crate that's already compiled and used elsewhere in the binary. Everything else stays as-is.

## Sources

- [dialoguer 0.12.0 docs](https://docs.rs/dialoguer/0.12.0/dialoguer/) -- feature flags and API reference
- [tabled 0.20.0 docs](https://docs.rs/tabled/0.20.0/tabled/) -- table rendering API
- [Comparison of Rust CLI Prompts](https://fadeevab.com/comparison-of-rust-cli-prompts/) -- evaluated alternatives (cliclack, inquire, promptly)
- Existing codebase: `status.rs:185-191` and `lib.rs:1128-1134` for established tabled usage patterns
- `cargo search dialoguer` / `cargo search tabled` -- verified both at latest versions (2026-04-16)

---

*Stack analysis: 2026-04-16*
