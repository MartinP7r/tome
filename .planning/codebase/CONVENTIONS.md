# Coding Conventions

**Analysis Date:** 2026-04-30 (v0.9.0)

## Naming Patterns

**Files:**
- Lowercase snake_case for all module files: `discover.rs`, `library.rs`, `cleanup.rs`
- Tests co-located in same file using `#[cfg(test)] mod tests { }` blocks
- Integration tests in separate `tests/cli.rs` directory

**Functions:**
- Lowercase snake_case: `hash_directory()`, `resolve_machine_path()`, `expand_tilde()`
- Descriptive action verbs: `discover_`, `consolidate_`, `distribute_`, `cleanup_`
- Helper functions marked with `pub(crate)` for internal use

**Variables:**
- Lowercase snake_case: `tmp_dir`, `source_path`, `skill_name`
- Single-letter loop variables acceptable in short contexts: `for (k, v) in...`
- Collection variables use plural forms: `skills`, `directories`, `failures`

**Types:**
- PascalCase for struct/enum names: `SkillName`, `DirectoryName`, `DiscoveredSkill`, `SkillOrigin`, `SyncReport`
- Newtype wrappers use transparent repr: `pub struct SkillName(String);`
- Enums descriptive and specific: `DirectoryType::ClaudePlugins`, `DirectoryRole::Synced`, `SkillOrigin::Managed { provenance }`

## Code Style

**Formatting:**
- `cargo fmt` (rustfmt default settings)
- No explicit `.rustfmt.toml` — uses Rust edition 2024 defaults
- Max line length: implicit, around 100-120 characters

**Linting:**
- `cargo clippy --all-targets -- -D warnings` enforced in CI
- Clippy warnings treated as build failures (`-D warnings`)
- Use `#[allow(dead_code)]` or `#[allow(unused)]` with justification when necessary (e.g., builder pattern with optional methods)

## Import Organization

**Order:**
1. Standard library (`use std::...`)
2. External crates (`use anyhow::...`, `use serde::...`)
3. Internal modules (`use crate::...`)
4. Conditional/test imports (`#[cfg(test)] use ...`)

**Path Aliases:**
- No module path aliases used
- Full qualified paths preferred for clarity: `crate::validation::validate_identifier()`

**Example from `crates/tome/src/config.rs`:**
```rust
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use crate::discover::SkillName;
```

## Error Handling

**Patterns:**
- `anyhow::Result<T>` used throughout for application-level error handling
- `anyhow::Context` trait for adding context: `.context("description of what failed")?` or `.with_context(|| format!(...))?`
- `anyhow::ensure!()` macro for validation: `ensure!(condition, "error message")`
- `anyhow::bail!()` for error returns: `bail!("descriptive error")`
- `Option::is_some_and()` for conditional checks: `p.parent().is_some_and(|d| d.exists())`

**Example from `crates/tome/src/config.rs`:**
```rust
pub fn load(path: &Path) -> Result<Self> {
    if path.exists() {
        let content = std::fs::read_to_string(path)
            .with_context(|| format!("failed to read {}", path.display()))?;
        let mut config: Config = toml::from_str(&content)
            .with_context(|| format!("failed to parse {}", path.display()))?;
        config.expand_tildes()?;
        Ok(config)
    } else {
        let mut config = Self::default();
        config.expand_tildes()?;
        Ok(config)
    }
}
```

**Validation Errors:**
- Centralized in `crate::validation` module
- `validate_identifier()` function rejects: empty names, `.` and `..`, whitespace-only, path separators
- Newtype types enforce validation at construction time

## Logging

**Framework:** `eprintln!` for errors, `println!` for normal output, no structured logging library

**Patterns:**
- User-facing errors: `eprintln!("error: {e:#}");` with debug formatting for context
- Progress/feedback: spinners via `indicatif::ProgressBar`
- Status messages: colored text via `console::style()`
- Verbose output: conditioned on `--verbose` flag

**Example from `crates/tome/src/lib.rs`:**
```rust
fn spinner(msg: &str) -> ProgressBar {
    let sp = ProgressBar::new_spinner();
    sp.set_style(
        ProgressStyle::default_spinner()
            .template("{spinner:.cyan} {msg}")
            .expect("valid template"),
    );
    sp.set_message(msg.to_string());
    sp.enable_steady_tick(std::time::Duration::from_millis(80));
    sp
}
```

## Comments

**When to Comment:**
- Above functions with `///` doc comments explaining purpose, parameters, examples
- Module-level `//!` doc comments in each module file
- Inline comments for non-obvious logic or workarounds
- Avoid redundant comments that simply restate code

**JSDoc/RustDoc:**
- Comprehensive doc comments on all public types and functions
- Doc comments include `# Examples` sections for complex functionality
- Code examples in doc comments are formatted as executable code

**Example from `crates/tome/src/discover.rs`:**
```rust
/// Create a new skill name from any string-like value.
///
/// Rejects empty names and names containing path separators (`/` or `\`).
///
/// # Examples
///
/// ```text
/// let name = SkillName::new("my-skill").unwrap();
/// assert_eq!(name.as_str(), "my-skill");
///
/// // Empty names and path separators are rejected
/// assert!(SkillName::new("").is_err());
/// assert!(SkillName::new("foo/bar").is_err());
/// ```
pub fn new(name: impl Into<String>) -> Result<Self> {
    let name = name.into();
    crate::validation::validate_identifier(&name, "skill name")?;
    Ok(Self(name))
}
```

## Function Design

**Size:** Generally 20-50 lines; complex operations broken into smaller helpers

**Parameters:**
- Accept references or owned types depending on lifetime needs: `&Path` vs `PathBuf`
- Generic constraints used where appropriate: `impl Into<String>`
- Builder patterns for complex initialization

**Return Values:**
- `anyhow::Result<T>` for fallible operations
- `Option<T>` for optional values (not defaults)
- Struct types with public fields (e.g., `SyncReport`, `DiscoveredSkill`)

**Example from `crates/tome/src/manifest.rs`:**
```rust
pub fn insert(&mut self, name: SkillName, entry: SkillEntry) {
    self.skills.insert(name, entry);
}

pub fn remove(&mut self, name: &str) {
    self.skills.remove(name);
}

pub fn keys(&self) -> impl Iterator<Item = &SkillName> {
    self.skills.keys()
}
```

## Module Design

**Exports:**
- `pub` for public API items
- `pub(crate)` for internal-only helpers (not exported from crate root)
- `pub(crate)` on internal struct fields that should not be directly accessed
- Minimal public surface area

**Barrel Files:**
- No barrel re-exports (no `pub use`)
- Crate root (`lib.rs`) explicitly lists all modules and re-exports key types

**Example from `crates/tome/src/lib.rs`:**
```rust
pub(crate) mod backup;
pub(crate) mod browse;
pub(crate) mod cleanup;
pub mod cli;
pub mod config;
// ... public API items:
pub use paths::TomePaths;
pub struct SyncReport { ... }
```

## Type Safety

**Newtype Pattern:**
- Used for domain types to prevent mixing (e.g., `SkillName`, `DirectoryName`, `ContentHash`)
- Provides validation at construction time
- Implements `AsRef<str>`, `Display`, `Borrow<str>` for ergonomics
- Custom `Deserialize` impl validates on deserialization

**Example from `crates/tome/src/discover.rs`:**
```rust
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, serde::Serialize)]
#[serde(transparent)]
pub struct SkillName(String);

impl SkillName {
    pub fn new(name: impl Into<String>) -> Result<Self> {
        let name = name.into();
        crate::validation::validate_identifier(&name, "skill name")?;
        Ok(Self(name))
    }

    pub fn as_str(&self) -> &str { &self.0 }
}

impl AsRef<str> for SkillName { ... }
impl Borrow<str> for SkillName { ... }
impl<'de> serde::Deserialize<'de> for SkillName { ... }
```

## Trait Implementations

**Standard Traits:**
- `Debug` always derived
- `Clone` derived unless expensive (rare)
- `Default` implemented for configuration structs
- `Display` implemented for user-facing types
- `AsRef<T>`, `Borrow<T>`, `TryFrom<T>` for ergonomics

**Serde:**
- `Serialize`, `Deserialize` derived for data-holding structs
- `#[serde(transparent)]` for newtype wrappers
- `#[serde(default)]` for optional fields
- Custom deserialize impls validate during parsing

---

*Convention analysis: 2026-04-05*
