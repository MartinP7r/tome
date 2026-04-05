# Technology Stack

**Analysis Date:** 2026-04-05

## Languages

**Primary:**
- Rust 1.85.0+ (Edition 2024) - CLI binary (`crates/tome`) with library re-exports

## Runtime

**Environment:**
- Standalone binary (no runtime required beyond OS)
- Targets: `aarch64-apple-darwin`, `x86_64-apple-darwin`, `x86_64-unknown-linux-gnu`

**Package Manager:**
- Cargo (Rust 1.85.0+)
- Lockfile: `Cargo.lock` (present)

## Frameworks

**Core:**
- `clap` 4 - CLI argument parsing with derive macros
- `clap_complete` 4 - Shell completion generation

**Interactive UI:**
- `ratatui` 0.30 - Terminal UI framework (TUI) for `tome browse` command
- `crossterm` 0.29 - Terminal event handling and cursor control
- `nucleo-matcher` 0.3 - Fuzzy matching for interactive search in browse view

**Data & Configuration:**
- `serde` 1 with derive - Serialization/deserialization framework
- `toml` 1 - TOML configuration parsing (`~/.tome/tome.toml`)
- `serde_json` 1 - JSON for manifest files (`.tome-manifest.json`, lockfiles)
- `serde_yaml` 0.9 - YAML frontmatter parsing from SKILL.md files

**File & Directory Handling:**
- `walkdir` 2 - Recursive directory traversal
- `dirs` 6 - Platform-aware home directory detection
- `tempfile` 3 (dev) - Temporary file creation for tests

**Terminal & Display:**
- `dialoguer` 0.12 - Interactive prompts (MultiSelect, Input, Confirm, Select) in wizard
- `indicatif` 0.18 - Progress bars and spinners
- `console` 0.16 - Terminal colors and formatting
- `tabled` 0.20 - ASCII table output for `tome list` and `tome status`

**Cryptography & Hashing:**
- `sha2` 0.11 - SHA-256 hashing for content integrity (skill directory hashes)

**Error Handling:**
- `anyhow` 1 - Error handling and context propagation

## Testing

**Test Framework:**
- `assert_cmd` 2 - CLI binary assertion testing
- `assert_fs` 1 - Filesystem assertion helpers (TempDir)
- `insta` 1 with json+filters features - Snapshot testing with path redaction
- `predicates` 3 - Assertion predicates for test conditions

**Test Organization:**
- Unit tests: co-located in modules via `#[cfg(test)] mod tests`
- Integration tests: `crates/tome/tests/cli.rs` exercises binary via `assert_cmd`
- Snapshot tests: stored in `crates/tome/tests/snapshots/`

## Key Dependencies

**Critical:**
- `serde` + `toml` - Config loading/saving; schema validation via deserialization
- `walkdir` - Skill discovery from configured sources
- `sha2` - Content hashing for idempotent sync (detects unchanged skills)
- `clap` - CLI parsing and help text generation

**Infrastructure:**
- `dialoguer` - Interactive setup (wizard) via `tome init`
- `ratatui`/`crossterm` - Terminal UI for `tome browse` command
- `indicatif` - Progress feedback during long operations

## Build System

**Build Configuration:**
- Workspace manifest: `Cargo.toml` (root, defines all dependencies)
- Crate manifest: `crates/tome/Cargo.toml` (binary-specific)
- Profile configuration in root `Cargo.toml`:
  - **Release:** LTO enabled, binary stripped, single codegen unit, panic abort
  - **Dist:** Thin LTO variant for distribution builds

**Distribution:**
- `cargo-dist` 0.30.3 - Artifact building and release automation
- Targets: Homebrew (primary), GitHub Releases (hosting)
- CI: GitHub Actions (ubuntu-latest, macos-latest)

## Configuration

**Application:**
- Primary config: `~/.tome/tome.toml` (TOML format)
- Per-machine prefs: `~/.config/tome/machine.toml` (disabled skills/targets)
- Library manifest: `~/.tome/.tome-manifest.json` (provenance + hashes)
- Lockfile: `~/.tome/tome.lock` (reproducibility snapshot)

**Build Time:**
- Rust formatting: `cargo fmt` (no separate prettier/rustfmt.toml)
- Linting: `cargo clippy --all-targets -- -D warnings`
- Dependency auditing: `cargo deny` (policy in `deny.toml`)
- Typo checking: `typos` CLI
- Unused dependency detection: `cargo machete`

## Platform Requirements

**Development:**
- Rust 1.85.0+ (via `dtolnay/rust-toolchain@stable` in CI)
- macOS (tested) or Linux (tested) — Unix-only (`std::os::unix::fs::symlink`)
- Cargo and workspace resolver v3

**Production:**
- macOS 10.15+ (aarch64-apple-darwin, x86_64-apple-darwin)
- Linux x86_64 (GNU libc, x86_64-unknown-linux-gnu)
- No external services or network requirements

## Dependency Audit Policy

**License whitelist:** MIT, Apache-2.0, BSD-2/3-Clause, ISC, MPL-2.0, Zlib, Unicode, Unlicense, BSL-1.0, CC0-1.0 (defined in `deny.toml`)

**Vulnerability scanning:** GitHub Actions via `cargo-deny` (no known exceptions)

**Version constraints:**
- Multiple versions of the same crate trigger warnings (highlight all)
- Unknown registries and git sources trigger warnings

---

*Stack analysis: 2026-04-05*
