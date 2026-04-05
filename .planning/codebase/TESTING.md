# Testing Patterns

**Analysis Date:** 2026-04-05

## Test Framework

**Runner:**
- Rust built-in test framework (`cargo test`)
- No external test framework (cargo's native support used directly)
- Config: No explicit test config file

**Assertion Library:**
- Built-in Rust assertions: `assert_eq!()`, `assert!()`, `assert_ne!()`
- `predicates` crate for CLI output assertions (matching patterns in stdout/stderr)
- `insta` crate for snapshot testing

**Run Commands:**
```bash
cargo test                      # Run all tests (unit + integration)
cargo test -p tome             # Run tests for 'tome' crate only
cargo test -p tome --test cli  # Integration tests only
cargo test test_name           # Run specific test by function name
cargo test module::tests       # Module-scoped tests
make test                       # make target wrapping cargo test
```

## Test File Organization

**Location:**
- Co-located with implementation: `#[cfg(test)] mod tests { }` blocks in same file
- Integration tests in `crates/tome/tests/cli.rs` (separate directory)

**Naming:**
- Unit test modules: `#[cfg(test)] mod tests`
- Test functions: `#[test] fn test_describes_what_is_tested()`
- Convention: test names are descriptive, present-tense verbs: `test_hash_directory_deterministic()`, `test_expand_tilde_expands_home()`

**Structure:**
```
crates/tome/
├── src/
│   ├── config.rs
│   │   └── #[cfg(test)] mod tests { ... }  # 50 unit tests
│   ├── discover.rs
│   │   └── #[cfg(test)] mod tests { ... }
│   └── ...
└── tests/
    └── cli.rs                              # ~30 integration tests
```

## Test Structure

**Suite Organization:**

Unit tests are organized by functionality within modules. Each module with tests follows this pattern:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_happy_path() {
        // Setup
        let tmp = TempDir::new().unwrap();
        
        // Action
        let result = function_under_test(&tmp.path());
        
        // Assertion
        assert!(result.is_ok());
    }

    #[test]
    fn test_error_case() {
        assert!(failing_function().is_err());
    }
}
```

**Patterns:**

1. **Setup/Action/Assert Pattern:**
   - Clear separation of test phases
   - No complex setup methods — minimal fixture boilerplate

2. **Teardown Pattern:**
   - Automatic via `tempfile::TempDir` drop
   - No manual cleanup needed

3. **Assertion Pattern:**
   - Built-in `assert_eq!()` and `assert!()` for simple cases
   - `predicates` for pattern matching on command output
   - `insta::assert_snapshot!()` for output snapshots

**Example from `crates/tome/src/config.rs`:**
```rust
#[test]
fn expand_tilde_expands_home() {
    let result = expand_tilde(Path::new("~/foo/bar")).unwrap();
    assert!(result.is_absolute());
    assert!(result.ends_with("foo/bar"));
}

#[test]
fn config_roundtrip_toml() {
    let config = Config {
        library_dir: PathBuf::from("/tmp/skills"),
        exclude: [SkillName::new("test-skill").unwrap()].into(),
        sources: vec![Source {
            name: "test".into(),
            path: PathBuf::from("/tmp/source"),
            source_type: SourceType::Directory,
        }],
        targets: BTreeMap::new(),
        ..Default::default()
    };
    let toml_str = toml::to_string_pretty(&config).unwrap();
    let parsed: Config = toml::from_str(&toml_str).unwrap();
    assert_eq!(parsed.library_dir, config.library_dir);
}
```

## Mocking

**Framework:** No dedicated mocking library (no mockall, no mock trait objects)

**Patterns:**
- Filesystem mocking via `tempfile::TempDir` for isolated test directories
- Direct invocation of functions under test
- No dependency injection or trait-based mocking

**Fixtures:**
- Test helpers use builder patterns (e.g., `TestEnvBuilder`) for complex setup
- Temporary directories created fresh per test

**What to Mock:**
- Filesystem operations (via `tempfile::TempDir`)
- Command invocation (via `assert_cmd` wrapper)
- Config files (written to temp dirs)

**What NOT to Mock:**
- Core business logic (test the actual code)
- Serde/JSON parsing (test real serialization)
- Validation logic (test real validators)

**Example from `crates/tome/tests/cli.rs`:**
```rust
struct TestEnv {
    tmp: TempDir,
    config_path: PathBuf,
    machine_path: Option<PathBuf>,
    library_dir: PathBuf,
    source_dirs: Vec<(String, PathBuf)>,
    target_dirs: Vec<(String, PathBuf)>,
}

impl TestEnvBuilder {
    fn new() -> Self {
        Self {
            sources: Vec::new(),
            targets: Vec::new(),
            skills: Vec::new(),
            // ... more fields
        }
    }

    fn source(mut self, name: &str, source_type: &str) -> Self {
        self.sources.push((name.to_string(), source_type.to_string()));
        self
    }

    fn skill(mut self, name: &str, source: &str) -> Self {
        self.skills.push((name.to_string(), source.to_string(), None));
        self
    }

    fn build(self) -> TestEnv {
        let tmp = TempDir::new().unwrap();
        let library_dir = tmp.path().join("library");
        std::fs::create_dir_all(&library_dir).unwrap();
        // ... complex setup
        TestEnv { tmp, config_path, library_dir, ... }
    }
}
```

## Fixtures and Factories

**Test Data:**
- No external fixture files (JSON, YAML)
- Inline test data construction via builder patterns
- Temporary directories as the single fixture resource

**Factories:**
- `TestEnvBuilder` for integration test setup (in `crates/tome/tests/cli.rs`)
- Helper functions like `create_skill()`, `write_config()` for repetitive setup
- Builder methods chain for readable test composition

**Location:**
- Integration test helpers at top of `tests/cli.rs`
- Unit test helpers defined within `#[cfg(test)] mod tests { }`

**Example from `crates/tome/tests/cli.rs`:**
```rust
fn write_config(dir: &std::path::Path, sources_toml: &str) -> std::path::PathBuf {
    let config_path = dir.join("config.toml");
    let library_dir = dir.join("library");
    std::fs::create_dir_all(&library_dir).unwrap();
    std::fs::write(
        &config_path,
        format!(
            "library_dir = \"{}\"\n{}",
            library_dir.display(),
            sources_toml
        ),
    )
    .unwrap();
    config_path
}

fn create_skill(dir: &std::path::Path, name: &str) {
    let skill_dir = dir.join(name);
    std::fs::create_dir_all(&skill_dir).unwrap();
    std::fs::write(
        skill_dir.join("SKILL.md"),
        format!("---\nname: {name}\n---\n# {name}\nA test skill."),
    )
    .unwrap();
}

let env = TestEnvBuilder::new()
    .source("local", "directory")
    .target("test-target")
    .skill("my-skill", "local")
    .build();
```

## Coverage

**Requirements:** No explicit coverage target enforced in CI

**View Coverage:**
```bash
# Tarpaulin (requires installation: cargo install cargo-tarpaulin)
cargo tarpaulin --verbose

# Generate HTML report
cargo tarpaulin --out Html
```

**Current Practice:**
- Focused coverage on core modules: `config.rs`, `manifest.rs`, `discover.rs`, `distribute.rs`
- Most modules have unit tests co-located with implementation
- Integration tests exercise full CLI workflows
- No coverage percentage requirement, pragmatic approach

## Test Types

**Unit Tests:**
- Scope: Single function or small module
- Approach: Direct invocation, inline setup
- Location: `#[cfg(test)] mod tests { }` in each module
- Examples: validation functions, path expansion, hash generation, config parsing

**Example from `crates/tome/src/manifest.rs`:**
```rust
#[test]
fn hash_directory_deterministic() {
    let tmp = TempDir::new().unwrap();
    std::fs::write(tmp.path().join("a.txt"), "hello").unwrap();
    std::fs::write(tmp.path().join("b.txt"), "world").unwrap();

    let h1 = hash_directory(tmp.path()).unwrap();
    let h2 = hash_directory(tmp.path()).unwrap();
    assert_eq!(h1, h2);
}

#[test]
fn manifest_roundtrip() {
    let tmp = TempDir::new().unwrap();

    let mut manifest = Manifest::default();
    let hash = test_hash("my-skill");
    manifest.insert(
        crate::discover::SkillName::new("my-skill").unwrap(),
        SkillEntry {
            source_path: PathBuf::from("/tmp/source/my-skill"),
            source_name: "test".to_string(),
            content_hash: hash.clone(),
            synced_at: "2024-01-01T00:00:00Z".to_string(),
            managed: false,
        },
    );

    save(&manifest, tmp.path()).unwrap();
    let loaded = load(tmp.path()).unwrap();
    assert_eq!(loaded.len(), 1);
}
```

**Integration Tests:**
- Scope: Full CLI commands and workflows
- Approach: Spawn `tome` binary via `assert_cmd::Command`, verify outputs
- Location: `crates/tome/tests/cli.rs`
- Examples: `sync` command with sources/targets, `list` output, `doctor` repairs

**Example from `crates/tome/tests/cli.rs`:**
```rust
#[test]
fn sync_copies_skills_to_library() {
    let tmp = TempDir::new().unwrap();
    let skills_dir = tmp.path().join("skills");
    create_skill(&skills_dir, "alpha");
    create_skill(&skills_dir, "beta");

    let config = write_config(
        tmp.path(),
        &format!(
            "[[sources]]\nname = \"test\"\npath = \"{}\"\ntype = \"directory\"\n",
            skills_dir.display()
        ),
    );

    let output = tome()
        .args(["--config", config.to_str().unwrap(), "sync"])
        .env("NO_COLOR", "1")
        .output()
        .unwrap();
    assert!(output.status.success());

    let library = tmp.path().join("library");
    assert!(library.join("alpha").is_dir());
    assert!(library.join("alpha/SKILL.md").is_file());
}
```

**E2E Tests:**
- Not used separately — integration tests function as E2E
- Full end-to-end workflows tested via CLI invocation

## Snapshot Testing

**Framework:** `insta` crate with `json` and `filters` features

**Usage Pattern:**
1. Redact temporary paths with regex filters before comparing
2. CLI output redacted to focus on content, not temp dirs
3. Snapshots stored in `tests/cli/snapshots/`

**Example from `crates/tome/tests/cli.rs`:**
```rust
fn snapshot_settings(tmp: &TempDir) -> Settings {
    let mut settings = Settings::clone_current();
    let tmp_str = tmp.path().display().to_string();
    // Escape regex metacharacters in the tmpdir path
    let escaped = tmp_str
        .chars()
        .flat_map(|c| {
            if r"\.+*?()|[]{}^$-".contains(c) {
                vec!['\\', c]
            } else {
                vec![c]
            }
        })
        .collect::<String>();
    settings.add_filter(&escaped, "[TMPDIR]");
    settings.add_filter(r" +\n", "\n");
    settings.set_snapshot_path("snapshots");
    settings
}

#[test]
fn list_shows_discovered_skills() {
    let tmp = TempDir::new().unwrap();
    // ... setup ...
    let output = tome()
        .args(["--config", config.to_str().unwrap(), "list"])
        .env("NO_COLOR", "1")
        .output()
        .unwrap();
    
    let stdout = String::from_utf8_lossy(&output.stdout);
    let settings = snapshot_settings(&tmp);
    settings.bind(|| {
        insta::assert_snapshot!("list_table_two_skills", stdout);
    });
}
```

## Common Patterns

**Async Testing:**
- Not used — codebase is synchronous
- No `tokio::test` or async/await in tests

**Error Testing:**
```rust
#[test]
fn validate_passes_for_valid_config() {
    let config = Config { /* ... */ };
    assert!(config.validate().is_ok());
}

#[test]
fn expand_tilde_with_home_unavailable() {
    // Expects validation error when home dir cannot be determined
    assert!(expand_tilde(Path::new("~")).is_err());
}

#[test]
fn config_load_fails_on_malformed_toml() {
    let dir = tempfile::TempDir::new().unwrap();
    let path = dir.path().join("config.toml");
    std::fs::write(&path, "this is [[[not valid toml").unwrap();
    assert!(Config::load(&path).is_err());
}
```

**Result Testing:**
```rust
#[test]
fn load_missing_manifest_returns_empty() {
    let tmp = TempDir::new().unwrap();
    let manifest = load(tmp.path()).unwrap();
    assert!(manifest.is_empty());
}

#[test]
fn load_corrupt_json_returns_error() {
    let tmp = TempDir::new().unwrap();
    std::fs::write(tmp.path().join(".tome-manifest.json"), "not valid json{{{").unwrap();
    assert!(load(tmp.path()).is_err());
}
```

## CI Testing

**CI Runner:** GitHub Actions (both `ubuntu-latest` and `macos-latest`)

**Test Pipeline:**
1. `cargo fmt --check` — format check
2. `cargo clippy --all-targets -- -D warnings` — linting
3. `cargo test` — unit + integration tests
4. `cargo build --release` — release build

**Run via Make:**
```bash
make ci                 # Runs: fmt-check, lint, test (matches GitHub Actions)
make test               # cargo test
make lint               # cargo clippy
make fmt-check          # cargo fmt --check
```

---

*Testing analysis: 2026-04-05*
