---
phase: 4
plan: 2
type: execute
wave: 2
depends_on:
  - "04-01"
files_modified:
  - crates/tome/src/config.rs
requirements:
  - WHARD-02
  - WHARD-03
autonomous: true
must_haves:
  truths:
    - "A config where library_dir equals a distribution directory path fails Config::validate() with a clear overlap error"
    - "A config where library_dir is inside a synced (distribution) directory fails Config::validate() with a circular-symlink error"
    - "A config where a distribution directory is inside library_dir fails Config::validate() with an overlap error"
    - "Tilde-prefixed paths and paths with trailing separators are compared correctly after normalization"
    - "Sibling paths (e.g. /a/foo vs /a/foobar) do NOT falsely match as overlapping"
  artifacts:
    - path: "crates/tome/src/config.rs"
      provides: "New overlap-detection block inside Config::validate()"
      contains: "Config::validate"
    - path: "crates/tome/src/config.rs"
      provides: "Seven new #[cfg(test)] unit tests covering Cases A/B/C and negative cases"
      contains: "validate_rejects_library_equals_distribution"
  key_links:
    - from: "crates/tome/src/config.rs::Config::validate()"
      to: "crates/tome/src/config.rs::Config::distribution_dirs()"
      via: "iterator call from overlap block"
      pattern: "self\\.distribution_dirs\\(\\)"
    - from: "crates/tome/src/config.rs::Config::validate()"
      to: "crates/tome/src/config.rs::expand_tilde()"
      via: "both sides of overlap comparison are tilde-expanded first (D-07)"
      pattern: "expand_tilde"
---

<objective>
Extend `Config::validate()` with lexical path-overlap detection between `library_dir` and every distribution directory (Synced or Target role). Catches all three relations per D-04: Case A (equality), Case B (library inside distribution — WHARD-03's circular symlink case), Case C (distribution inside library). Covers WHARD-02 (general overlap) and WHARD-03 (library-inside-synced circular case). Lexical-only (no `canonicalize()`) per D-02. Tilde expansion runs before comparison per D-07. Trailing-separator normalization prevents sibling-path false positives per D-06.

Purpose: close the Phase 4 correctness gap where a hand-edited tome.toml — or a wizard (see Plan 04-03) — can put `library_dir` at a location that will self-loop at distribute time. Enforcement in `Config::validate()` (D-01) is load-symmetric: bad configs fail on `Config::load()` AND on wizard save.

Output: `config.rs` with (a) a new overlap-check block appended at the end of `Config::validate()` using the D-10 Conflict + Why + Suggestion template established in Plan 04-01, and (b) seven new unit tests covering Cases A/B/C, sibling-path negatives, trailing-separator normalization, and tilde-prefixed paths.
</objective>

<execution_context>
@$HOME/.claude/get-shit-done/workflows/execute-plan.md
@$HOME/.claude/get-shit-done/templates/summary.md
</execution_context>

<context>
@.planning/PROJECT.md
@.planning/ROADMAP.md
@.planning/STATE.md
@.planning/phases/04-wizard-correctness/04-CONTEXT.md
@.planning/phases/04-wizard-correctness/04-01-validate-error-template-PLAN.md

<interfaces>
<!-- Direct quotes from 04-CONTEXT.md — load-bearing decisions. Do not re-interpret. -->

D-01: New path-overlap / circularity checks go in Config::validate() — load-symmetric.
D-02: Path comparison is lexical only — after tilde expansion and PathBuf normalization.
      No Path::canonicalize(). validate() stays I/O-free for these new checks.
D-04: validate() rejects all three relations between library_dir and every distribution dir:
      - Case A: exact path equality
      - Case B: library_dir is a descendant of the distribution directory
      - Case C: the distribution directory is a descendant of library_dir
D-05: Distribution-to-distribution overlap is out of scope. Strictly library vs distribution.
D-06: Prefix-matching uses trailing-separator normalization — e.g., /foo/bar does NOT contain /foo/barbaz.
D-07: Tilde expansion runs before comparison. Wizard save order matches load order.
D-08: On validation failure at the wizard's save step: hard error + exit. No retry loop.
D-10: Error template = Conflict + Why + Suggestion. Established in Plan 04-01.
D-11: Role names in error messages MUST include the plain-english parenthetical.

Key existing helpers we reuse (DO NOT reinvent):
  - Config::distribution_dirs() at config.rs:389 — iterates (name, config) for Synced+Target dirs.
  - expand_tilde(path: &Path) -> Result<PathBuf> at config.rs:424 — already used by Config::load().
  - DirectoryRole::description() at config.rs:156 — authoritative plain-english role strings.

Existing validate() shape (insertion point = end of function, before Ok(())):
  fn validate(&self) -> Result<()> {
      // existing: library_dir-is-a-file check (lines 333-337)
      for (name, dir) in &self.directories {
          // existing: role/type checks (lines 340-375)
      }
      // INSERT NEW OVERLAP BLOCK HERE (post-Plan-04-01 line numbers will shift — search for "Ok(())" at end of validate())
      Ok(())
  }
</interfaces>
</context>

<tasks>

<task type="auto" tdd="true">
  <name>Task 1: Add path-overlap validation helper + Cases A/B/C check in Config::validate()</name>
  <files>crates/tome/src/config.rs</files>
  <read_first>
    - crates/tome/src/config.rs (full file; focus on `Config::validate()` body, `Config::distribution_dirs()` at line 389, `expand_tilde()` at line 424, and the test module starting at line 530)
    - .planning/phases/04-wizard-correctness/04-CONTEXT.md (D-02, D-04, D-06, D-07, D-10, D-11 — authoritative)
    - .planning/phases/04-wizard-correctness/04-01-validate-error-template-PLAN.md (D-10 template already established for existing errors; new overlap errors MUST match)
  </read_first>
  <behavior>
    After this task, `Config::validate()` rejects overlapping paths with descriptive errors:

    Test 1 — Case A (exact equality, post tilde-expansion):
      `library_dir = "/tmp/shared"`, a Synced directory at `"/tmp/shared"`.
      `validate()` returns Err with message containing all of:
        "Conflict:", "library_dir", directory name in quotes, path "/tmp/shared",
        "Synced (skills discovered here AND distributed here)",
        "hint:".

    Test 2 — Case B (library_dir inside a distribution directory — WHARD-03 circular case):
      `library_dir = "/tmp/outer/inner"`, a Synced directory at `"/tmp/outer"`.
      `validate()` returns Err with message mentioning "circular" AND "symlink" AND
      "Synced (skills discovered here AND distributed here)" AND a suggestion to
      pick a library_dir outside the distribution directory.

    Test 3 — Case C (distribution directory inside library_dir):
      `library_dir = "/tmp/outer"`, a Target directory at `"/tmp/outer/inner"`.
      `validate()` returns Err with message containing "Conflict:", "Target (skills distributed here, not discovered here)",
      and "hint:".

    Test 4 — No false positive on sibling paths (D-06):
      `library_dir = "/tmp/foo"`, a Synced directory at `"/tmp/foobar"`.
      `validate()` returns Ok.

    Test 5 — Trailing separator normalization (D-06):
      `library_dir = "/tmp/lib/"`, a Synced directory at `"/tmp/lib"` (no trailing slash).
      `validate()` returns Err (treats them as the same path) with Case A wording.

    Test 6 — Source-role directory does NOT overlap-trip (scope: distribution only, D-05):
      `library_dir = "/tmp/outer"`, a Source directory at `"/tmp/outer/inner"` (Source is discovery-only, not distribution).
      `validate()` returns Ok (Source dirs don't participate in distribution, so they cannot cause a distribute-time self-loop).

    Test 7 — Tilde paths are expanded before comparison (D-07):
      `library_dir = "~/.tome/skills"`, a Synced directory at `"~/.tome/skills"`.
      `validate()` returns Err with Case A wording. (Both sides expand to the same absolute path.)

    All messages follow Plan-04-01's D-10 template: Conflict + Why + hint:. Fail-fast: first overlap triggers error and returns (D-04 iteration order = BTreeMap iteration order, which is alphabetical by directory name — stable and deterministic).
  </behavior>
  <action>
Step 1 — Add a private module-level helper for trailing-separator-normalized comparison. Insert immediately AFTER the `expand_tilde()` function at config.rs:424, BEFORE `default_tome_home()` at line 440:

```rust
/// Check whether `ancestor` is a path-prefix of `descendant` (or equal),
/// with trailing-separator normalization so that `/foo/bar` does NOT contain
/// `/foo/barbaz`.
///
/// Lexical only — no canonicalization. Both inputs must already be
/// tilde-expanded by the caller (D-07).
fn path_contains(ancestor: &Path, descendant: &Path) -> bool {
    // Strip trailing separator so component-wise comparison is correct
    // even when the user writes "/foo/bar/" in config.
    let a: &Path = ancestor
        .to_str()
        .map(|s| Path::new(s.trim_end_matches('/')))
        .unwrap_or(ancestor);
    let d: &Path = descendant
        .to_str()
        .map(|s| Path::new(s.trim_end_matches('/')))
        .unwrap_or(descendant);
    d == a || d.starts_with(a)
}
```

Step 2 — Extend `Config::validate()` to run the overlap block. Insert the following block AFTER the existing `for (name, dir) in &self.directories { ... }` loop and BEFORE the final `Ok(())` (current position circa config.rs:377 post-Plan-04-01 edits; find it by searching for the `Ok(())` that closes `validate()`):

```rust
    // --- Path overlap between library_dir and distribution directories ---
    // D-01/D-02/D-04/D-06/D-07: lexical, tilde-aware, trailing-separator-normalized.
    // Scope (D-05): library_dir vs each distribution directory (Synced or Target).
    let lib = expand_tilde(&self.library_dir)?;
    for (name, dir) in self.distribution_dirs() {
        let dist = expand_tilde(&dir.path)?;
        let role_desc = dir.role().description();

        // Case A: exact equality
        if lib == dist
            || lib.to_string_lossy().trim_end_matches('/')
                == dist.to_string_lossy().trim_end_matches('/')
        {
            anyhow::bail!(
                "library_dir overlaps distribution directory '{name}'\n\
                 Conflict: library_dir ({}) is the same path as directory '{name}' ({})\n\
                 Why: this directory has role {role_desc}; tome would try to distribute the library into itself, creating a self-loop at sync time.\n\
                 hint: choose a library_dir outside any distribution directory, such as '~/.tome/skills'.",
                lib.display(),
                dist.display(),
            );
        }

        // Case B: library_dir is inside the distribution directory (WHARD-03 circular case)
        if path_contains(&dist, &lib) {
            anyhow::bail!(
                "library_dir is inside distribution directory '{name}' (circular symlink risk)\n\
                 Conflict: library_dir ({}) is a subdirectory of directory '{name}' ({})\n\
                 Why: directory '{name}' has role {role_desc}; tome would distribute the library back into a directory that contains it, producing circular symlinks at distribute time.\n\
                 hint: move library_dir outside '{}' — for example, '~/.tome/skills'.",
                lib.display(),
                dist.display(),
                dist.display(),
            );
        }

        // Case C: the distribution directory is inside library_dir
        if path_contains(&lib, &dist) {
            anyhow::bail!(
                "distribution directory '{name}' is inside library_dir\n\
                 Conflict: directory '{name}' ({}) is a subdirectory of library_dir ({})\n\
                 Why: directory '{name}' has role {role_desc}; tome would distribute library contents into a directory that already lives inside the library, producing a self-loop at sync time.\n\
                 hint: move library_dir to a location outside '{name}' — for example, '~/.tome/skills'.",
                dist.display(),
                lib.display(),
            );
        }
    }
```

Step 3 — Add seven new unit tests at the end of the existing `#[cfg(test)] mod tests` block in config.rs (append after the last existing test, before the closing `}`). Use the same struct-literal builder pattern as `validate_passes_for_valid_config` at line 874.

```rust
    // --- Overlap tests (WHARD-02 / WHARD-03) ---

    fn dir_cfg(path: &str, dt: DirectoryType, role: Option<DirectoryRole>) -> DirectoryConfig {
        DirectoryConfig {
            path: PathBuf::from(path),
            directory_type: dt,
            role,
            branch: None,
            tag: None,
            rev: None,
            subdir: None,
        }
    }

    #[test]
    fn validate_rejects_library_equals_distribution() {
        let config = Config {
            library_dir: PathBuf::from("/tmp/shared"),
            directories: BTreeMap::from([(
                DirectoryName::new("shared").unwrap(),
                dir_cfg("/tmp/shared", DirectoryType::Directory, Some(DirectoryRole::Synced)),
            )]),
            ..Default::default()
        };
        let msg = config.validate().unwrap_err().to_string();
        assert!(msg.contains("Conflict:"), "missing Conflict line: {msg}");
        assert!(msg.contains("shared"), "missing directory name: {msg}");
        assert!(
            msg.contains("Synced (skills discovered here AND distributed here)"),
            "missing role parenthetical: {msg}"
        );
        assert!(msg.contains("hint:"), "missing hint: {msg}");
    }

    #[test]
    fn validate_rejects_library_inside_synced_dir() {
        let config = Config {
            library_dir: PathBuf::from("/tmp/outer/inner"),
            directories: BTreeMap::from([(
                DirectoryName::new("outer").unwrap(),
                dir_cfg("/tmp/outer", DirectoryType::Directory, Some(DirectoryRole::Synced)),
            )]),
            ..Default::default()
        };
        let msg = config.validate().unwrap_err().to_string();
        assert!(msg.contains("circular"), "missing 'circular': {msg}");
        assert!(msg.contains("symlink"), "missing 'symlink': {msg}");
        assert!(
            msg.contains("Synced (skills discovered here AND distributed here)"),
            "missing role parenthetical: {msg}"
        );
        assert!(msg.contains("hint:"), "missing hint: {msg}");
    }

    #[test]
    fn validate_rejects_target_inside_library() {
        let config = Config {
            library_dir: PathBuf::from("/tmp/outer"),
            directories: BTreeMap::from([(
                DirectoryName::new("inner-target").unwrap(),
                dir_cfg("/tmp/outer/inner", DirectoryType::Directory, Some(DirectoryRole::Target)),
            )]),
            ..Default::default()
        };
        let msg = config.validate().unwrap_err().to_string();
        assert!(msg.contains("Conflict:"), "missing Conflict line: {msg}");
        assert!(
            msg.contains("Target (skills distributed here, not discovered here)"),
            "missing role parenthetical: {msg}"
        );
        assert!(msg.contains("hint:"), "missing hint: {msg}");
    }

    #[test]
    fn validate_accepts_sibling_paths_not_false_positive() {
        // /tmp/foo and /tmp/foobar are siblings, not nested.
        let config = Config {
            library_dir: PathBuf::from("/tmp/foo"),
            directories: BTreeMap::from([(
                DirectoryName::new("foobar").unwrap(),
                dir_cfg("/tmp/foobar", DirectoryType::Directory, Some(DirectoryRole::Synced)),
            )]),
            ..Default::default()
        };
        config.validate().expect("sibling paths must not trigger overlap");
    }

    #[test]
    fn validate_rejects_equality_despite_trailing_separator() {
        let config = Config {
            library_dir: PathBuf::from("/tmp/lib/"),
            directories: BTreeMap::from([(
                DirectoryName::new("lib").unwrap(),
                dir_cfg("/tmp/lib", DirectoryType::Directory, Some(DirectoryRole::Synced)),
            )]),
            ..Default::default()
        };
        let msg = config.validate().unwrap_err().to_string();
        assert!(msg.contains("Conflict:"), "missing Conflict line: {msg}");
    }

    #[test]
    fn validate_accepts_source_role_inside_library() {
        // Source dirs don't participate in distribution — no self-loop risk (D-05).
        let config = Config {
            library_dir: PathBuf::from("/tmp/outer"),
            directories: BTreeMap::from([(
                DirectoryName::new("inner-source").unwrap(),
                dir_cfg("/tmp/outer/inner", DirectoryType::Directory, Some(DirectoryRole::Source)),
            )]),
            ..Default::default()
        };
        config.validate().expect("Source-role nesting must not trigger overlap");
    }

    #[test]
    fn validate_rejects_tilde_equal_paths() {
        // Both library_dir and directory path use tilde; must expand before compare.
        let config = Config {
            library_dir: PathBuf::from("~/.tome/skills"),
            directories: BTreeMap::from([(
                DirectoryName::new("same").unwrap(),
                dir_cfg("~/.tome/skills", DirectoryType::Directory, Some(DirectoryRole::Synced)),
            )]),
            ..Default::default()
        };
        let msg = config.validate().unwrap_err().to_string();
        assert!(msg.contains("Conflict:"), "missing Conflict line: {msg}");
        assert!(
            msg.contains("Synced (skills discovered here AND distributed here)"),
            "missing role parenthetical: {msg}"
        );
    }
```

Step 4 — Run formatting, clippy, and tests:
```bash
cd /Users/martin/dev/opensource/tome
cargo fmt
cargo clippy --all-targets -- -D warnings
cargo test -p tome
```

Do NOT add a new public API. `path_contains` stays private (module-local). Do NOT introduce `Path::canonicalize` (D-02). Do NOT detect distro-to-distro overlaps (D-05).
  </action>
  <verify>
    <automated>cd /Users/martin/dev/opensource/tome && cargo test -p tome --lib config::tests::validate_rejects_library_equals_distribution config::tests::validate_rejects_library_inside_synced_dir config::tests::validate_rejects_target_inside_library config::tests::validate_accepts_sibling_paths_not_false_positive config::tests::validate_rejects_equality_despite_trailing_separator config::tests::validate_accepts_source_role_inside_library config::tests::validate_rejects_tilde_equal_paths</automated>
  </verify>
  <acceptance_criteria>
    - `rg "fn path_contains" crates/tome/src/config.rs` returns 1 hit
    - `rg "library_dir overlaps distribution directory" crates/tome/src/config.rs` returns 1 hit
    - `rg "library_dir is inside distribution directory" crates/tome/src/config.rs` returns 1 hit
    - `rg "distribution directory .* is inside library_dir" crates/tome/src/config.rs` returns 1 hit
    - `rg "circular symlink risk" crates/tome/src/config.rs` returns 1 hit
    - `rg "self\\.distribution_dirs\\(\\)" crates/tome/src/config.rs` returns ≥ 1 hit inside validate() (new overlap block uses the existing iterator)
    - `rg "canonicalize" crates/tome/src/config.rs` returns 0 hits (D-02: no canonicalization)
    - `rg "Synced \\(skills discovered here AND distributed here\\)" crates/tome/src/config.rs` returns ≥ 1 hit in the test module (D-11 parenthetical asserted)
    - `cargo test -p tome --lib config::tests::validate_rejects_library_equals_distribution` exits 0
    - `cargo test -p tome --lib config::tests::validate_rejects_library_inside_synced_dir` exits 0
    - `cargo test -p tome --lib config::tests::validate_rejects_target_inside_library` exits 0
    - `cargo test -p tome --lib config::tests::validate_accepts_sibling_paths_not_false_positive` exits 0
    - `cargo test -p tome --lib config::tests::validate_rejects_equality_despite_trailing_separator` exits 0
    - `cargo test -p tome --lib config::tests::validate_accepts_source_role_inside_library` exits 0
    - `cargo test -p tome --lib config::tests::validate_rejects_tilde_equal_paths` exits 0
    - `cargo clippy --all-targets -- -D warnings` exits 0
    - `cargo fmt -- --check` exits 0
  </acceptance_criteria>
  <done>
    `Config::validate()` rejects all three overlap relations (A/B/C) between `library_dir` and each distribution directory; sibling paths and Source-role nesting do not trigger false positives; tilde-prefixed paths and trailing-separator variants are handled. Seven new unit tests all pass. `make ci` clean.
  </done>
</task>

</tasks>

<verification>
Phase-exit checks for Plan 04-02:

1. `cd /Users/martin/dev/opensource/tome && cargo fmt -- --check` exits 0
2. `cd /Users/martin/dev/opensource/tome && cargo clippy --all-targets -- -D warnings` exits 0
3. `cd /Users/martin/dev/opensource/tome && cargo test -p tome` exits 0
4. `rg "distribution_dirs\(\)" crates/tome/src/config.rs` returns ≥ 1 hit inside validate() body
5. `rg "Ok\(\(\)\)" crates/tome/src/config.rs` — verify the overlap block is inserted BEFORE the terminal `Ok(())` of `validate()` (visual confirmation via `Read`)
</verification>

<success_criteria>
- Plan delivers Case A (equality), Case B (library inside distro), Case C (distro inside library) checks in `Config::validate()`.
- Path comparison is lexical (D-02), tilde-expanded (D-07), trailing-separator-normalized (D-06).
- Scope stays strictly `library_dir` vs distribution dirs (D-05 — no distro-to-distro check).
- Errors follow the D-10 Conflict + Why + Suggestion template and use `DirectoryRole::description()` (D-11).
- Seven unit tests cover the matrix of cases.
</success_criteria>

<output>
After completion, create `.planning/phases/04-wizard-correctness/04-02-SUMMARY.md`.
</output>
