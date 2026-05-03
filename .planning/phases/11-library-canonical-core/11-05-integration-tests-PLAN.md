---
phase: 11-library-canonical-core
plan: 05
type: execute
wave: 4
depends_on:
  - 11-01
  - 11-02
  - 11-03
  - 11-04
files_modified:
  - crates/tome/tests/cli.rs
autonomous: true
requirements:
  - LIB-01
  - LIB-04
  - LIB-05
must_haves:
  truths:
    - "An end-to-end CLI test exercises the full v0.9 → v0.10 conversion: synthetic v0.9 library fixture with mix of managed symlinks + local real-dirs + broken symlink + user-created symlink → `tome migrate-library` → assertion that managed→real-dir, local untouched, broken preserved + warned, user-created symlink untouched."
    - "An end-to-end CLI test exercises `tome sync` refuse-with-hint on a v0.9-shape library (D-02): the command exits non-zero AND stderr contains both 'v0.9 shape' and 'tome migrate-library'."
    - "An end-to-end CLI test exercises source removal preservation (LIB-04 / D-09 Case 1): set up tome.toml with directory D containing skill X → `tome sync` → remove D from tome.toml → `tome sync` again → assert library still contains X with same `content_hash` AND manifest entry has `source_name == None`."
    - "An end-to-end CLI test exercises `tome migrate-library --dry-run`: command exits 0 (or non-zero only when broken-symlinks present), no filesystem mutation, plan rendered to stdout."
    - "An end-to-end CLI test exercises post-migration idempotent `tome sync`: after `tome migrate-library` succeeds, running `tome sync` does NOT refuse anymore — sync succeeds (consolidate sees real dirs, treats them as no-op or hash-matched updates)."
  artifacts:
    - path: "crates/tome/tests/cli.rs"
      provides: "Integration tests for v0.10 library-canonical core"
      contains: "tome migrate-library"
  key_links:
    - from: "tests/cli.rs::v0_10_migration tests"
      to: "tome binary via assert_cmd"
      via: "Command::cargo_bin(\"tome\")"
      pattern: "Command::cargo_bin"
---

<objective>
Add end-to-end integration tests for Phase 11 deliverables. These tests exercise the
binary surface (via `assert_cmd`) and lock in the user-visible behavior of:

- `tome migrate-library` (LIB-05) — happy path, broken-symlink case, user-created
  symlink preservation, dry-run.
- `tome sync` v0.9-shape refuse-with-hint (D-02 / LIB-05 supporting check).
- Source-removal → Unowned preservation (LIB-04 / D-09 Case 1).
- Post-migration idempotent sync.

Wave 4 — depends on all of Plans 11-01..11-04. These are the tests that prove the
phase's success criteria 1, 2, 4 from ROADMAP.md Phase 11.

Output: New tests appended to `crates/tome/tests/cli.rs` (or a new section if
preferred; HARD-13 will eventually split this file in Phase 15, but for Phase 11 we
follow the existing single-file convention).
</objective>

<execution_context>
@$HOME/.claude/get-shit-done/workflows/execute-plan.md
@$HOME/.claude/get-shit-done/templates/summary.md
</execution_context>

<context>
@.planning/STATE.md
@.planning/ROADMAP.md
@.planning/REQUIREMENTS.md
@.planning/phases/11-library-canonical-core/11-CONTEXT.md
@CLAUDE.md
@crates/tome/tests/cli.rs
@crates/tome/src/cli.rs
@crates/tome/src/migration_v010.rs

<interfaces>
<!-- Existing test conventions from `crates/tome/tests/cli.rs`:
     - Use `assert_cmd::Command::cargo_bin("tome")` to invoke the binary.
     - Use `assert_fs::TempDir` or `tempfile::TempDir` for isolation.
     - Tests do not auto-cleanup (TempDir handles drop).
     - Set `--config <path>` and `--tome-home <path>` to point at the temp tree.
     - For machine.toml, use `--machine <path>` to isolate.
     - For non-interactive tests, pass `--no-input`. -->

<!-- The synthetic v0.9 library fixture should mimic the user's real layout per
     CONTEXT.md <specifics>:
     - 2-3 managed symlinks pointing to a fake "plugin cache" directory
     - 1-2 local real-dir entries (already v0.10-shape)
     - 1 broken symlink (target deleted) — exercises D-04
     - 1 user-created symlink NOT in manifest — exercises D-03 conservatism -->
</interfaces>
</context>

<tasks>

<task type="auto">
  <name>Task 1: Add fixture helper and core migrate-library happy-path test</name>
  <files>crates/tome/tests/cli.rs</files>
  <read_first>
    - crates/tome/tests/cli.rs (the WHOLE file — observe existing test conventions, fixture builders, common helpers)
    - crates/tome/src/migration_v010.rs (output banner format from Plan 11-04 — `⚠ N converted · K skipped · M failed`)
    - crates/tome/src/lib.rs (refuse-with-hint message text from Plan 11-04 Task 2)
    - .planning/phases/11-library-canonical-core/11-CONTEXT.md (`<specifics>` for synthetic library shape)
  </read_first>
  <action>
1. **Append a new test section** to `crates/tome/tests/cli.rs`. Place at the end of the file, before the final `}` of the test module (or after the last existing test if it's a flat file). Add a section delimiter comment to make this Phase 11's contribution visible:

   ```rust
   // ============================================================================
   // v0.10 Phase 11 — library-canonical-core integration tests
   // (LIB-01, LIB-04, LIB-05; CONTEXT.md D-01..D-06, D-09..D-14)
   // ============================================================================
   ```

2. **Add a fixture builder** at the top of the new section. Note the existing file already has `use` statements; check for collisions before re-importing. The fixture should:
   - Create a temp `tome_home` dir
   - Create a fake plugin cache dir (acts as the source for managed skills)
   - Create a fake local-source dir (acts as the source for local skills)
   - Pre-populate the manifest at `tome_home/.tome-manifest.json` with v0.9-shape entries
   - Create the v0.9-shape symlinks in `tome_home/skills/`
   - Optionally create broken symlinks and user-created symlinks per the test's needs
   - Create a minimal `tome.toml` referencing the source directories
   - Return a struct with the paths needed for assertions

   ```rust
   /// Synthetic v0.9 library fixture — exercises the migration boundary defenses.
   ///
   /// Layout produced (per CONTEXT.md <specifics>):
   ///   tome_home/
   ///     tome.toml                  ← references plugin_cache + local_source
   ///     .tome-manifest.json        ← entries for the managed skills + local skills
   ///     skills/
   ///       p1/                      ← v0.9-shape symlink → plugin_cache/p1
   ///       p2/                      ← v0.9-shape symlink → plugin_cache/p2
   ///       l1/                      ← v0.10-shape real dir copy of local_source/l1
   ///       broken/                  ← v0.9-shape symlink to /nonexistent (D-04)
   ///       user-symlink/            ← user-created symlink, NOT in manifest (D-03)
   ///   plugin_cache/
   ///     p1/SKILL.md
   ///     p2/SKILL.md
   ///   local_source/
   ///     l1/SKILL.md
   struct V09Fixture {
       _root: assert_fs::TempDir, // owns the temp dir; drop = cleanup
       tome_home: PathBuf,
       library_dir: PathBuf,
       plugin_cache: PathBuf,
       local_source: PathBuf,
       config_path: PathBuf,
       machine_path: PathBuf,
   }

   fn build_v09_fixture() -> V09Fixture {
       use std::os::unix::fs as unix_fs;
       let root = assert_fs::TempDir::new().unwrap();
       let tome_home = root.path().join("tome_home");
       let library_dir = tome_home.join("skills");
       let plugin_cache = root.path().join("plugin_cache");
       let local_source = root.path().join("local_source");
       std::fs::create_dir_all(&library_dir).unwrap();
       std::fs::create_dir_all(&plugin_cache).unwrap();
       std::fs::create_dir_all(&local_source).unwrap();

       // Plugin cache (acts as managed source — claude-plugins style).
       for n in &["p1", "p2"] {
           let d = plugin_cache.join(n);
           std::fs::create_dir_all(&d).unwrap();
           std::fs::write(d.join("SKILL.md"), format!("# {n}")).unwrap();
       }
       // Local source.
       let l1 = local_source.join("l1");
       std::fs::create_dir_all(&l1).unwrap();
       std::fs::write(l1.join("SKILL.md"), "# l1").unwrap();

       // v0.9-shape symlinks for managed skills.
       unix_fs::symlink(plugin_cache.join("p1"), library_dir.join("p1")).unwrap();
       unix_fs::symlink(plugin_cache.join("p2"), library_dir.join("p2")).unwrap();

       // v0.10-shape real-dir copy for local skill (already correct shape).
       let l1_lib = library_dir.join("l1");
       std::fs::create_dir_all(&l1_lib).unwrap();
       std::fs::write(l1_lib.join("SKILL.md"), "# l1").unwrap();

       // D-04: broken symlink (managed manifest entry, target gone).
       unix_fs::symlink("/nonexistent/target", library_dir.join("broken")).unwrap();

       // D-03 conservatism: user-created symlink NOT in manifest.
       let user_target = root.path().join("user_target");
       std::fs::create_dir_all(&user_target).unwrap();
       std::fs::write(user_target.join("SKILL.md"), "# user").unwrap();
       unix_fs::symlink(&user_target, library_dir.join("user-symlink")).unwrap();

       // Write the manifest with managed entries for p1, p2, broken; local entry for l1.
       // Use serde_json directly to bypass needing tome::manifest's pub API.
       let p1_hash = sha256_dir_simple(&plugin_cache.join("p1"));
       let p2_hash = sha256_dir_simple(&plugin_cache.join("p2"));
       let l1_hash = sha256_dir_simple(&l1_lib);
       let manifest_json = serde_json::json!({
           "skills": {
               "p1": {
                   "source_path": plugin_cache.join("p1").to_string_lossy(),
                   "source_name": "plugins",
                   "content_hash": p1_hash,
                   "synced_at": "2024-01-01T00:00:00Z",
                   "managed": true
               },
               "p2": {
                   "source_path": plugin_cache.join("p2").to_string_lossy(),
                   "source_name": "plugins",
                   "content_hash": p2_hash,
                   "synced_at": "2024-01-01T00:00:00Z",
                   "managed": true
               },
               "broken": {
                   "source_path": "/nonexistent/target",
                   "source_name": "plugins",
                   "content_hash": "0".repeat(64),
                   "synced_at": "2024-01-01T00:00:00Z",
                   "managed": true
               },
               "l1": {
                   "source_path": l1.to_string_lossy(),
                   "source_name": "local",
                   "content_hash": l1_hash,
                   "synced_at": "2024-01-01T00:00:00Z",
                   "managed": false
               }
           }
       });
       std::fs::write(
           tome_home.join(".tome-manifest.json"),
           serde_json::to_string_pretty(&manifest_json).unwrap(),
       )
       .unwrap();

       // Minimal tome.toml. Declare a plugins directory of type claude-plugins
       // pointing at plugin_cache, and a local source directory.
       // (For Phase 11 sync to find the managed skills it would need a real
       // installed_plugins.json — but our migration tests don't rely on
       // discover finding the managed skills; they operate on the manifest +
       // library directly. The tome.toml is mainly for sync's refuse-with-hint
       // test, which only needs valid syntax + a library_dir.)
       let config_path = tome_home.join("tome.toml");
       let toml = format!(
           r#"library_dir = "{}"

[directories.local]
path = "{}"
type = "directory"
role = "source"
"#,
           library_dir.display(),
           local_source.display(),
       );
       std::fs::write(&config_path, toml).unwrap();

       let machine_path = root.path().join("machine.toml");
       std::fs::write(&machine_path, "").unwrap();

       V09Fixture {
           _root: root,
           tome_home,
           library_dir,
           plugin_cache,
           local_source,
           config_path,
           machine_path,
       }
   }

   /// Compute a SHA-256 hex of a directory's contents matching the algorithm
   /// in `crates/tome/src/manifest.rs::hash_directory` (sorted relpath + content).
   fn sha256_dir_simple(dir: &Path) -> String {
       use sha2::{Digest, Sha256};
       let mut entries: Vec<(String, PathBuf)> = Vec::new();
       for e in walkdir::WalkDir::new(dir) {
           let e = e.unwrap();
           if e.file_type().is_file() {
               let rel = e.path().strip_prefix(dir).unwrap().to_string_lossy().into_owned();
               entries.push((rel, e.path().to_path_buf()));
           }
       }
       entries.sort_by(|a, b| a.0.cmp(&b.0));
       let mut hasher = Sha256::new();
       for (rel, abs) in &entries {
           hasher.update(rel.as_bytes());
           hasher.update(b"\0");
           hasher.update(std::fs::read(abs).unwrap());
       }
       hasher
           .finalize()
           .iter()
           .map(|b| format!("{:02x}", b))
           .collect()
       // NOTE: must match `manifest::hash_directory` byte-for-byte. If that
       // function changes its algorithm in a future phase, this helper must
       // be updated; the test will fail with a hash-mismatch assertion.
   }
   ```

   If `walkdir` and `sha2` are not already imported in tests/cli.rs, add `use walkdir;` is unnecessary (use full paths above). Confirm `sha2` is available in dev-deps; if not, add via Cargo.toml dev-dependencies (it's already a regular dep, so it's accessible from integration tests via `tome::` is NOT possible — instead, the simpler approach is to add `sha2 = "0.11"` under `[dev-dependencies]` if not already there. Run `rg "sha2" /Users/martin/dev/opensource/tome/crates/tome/Cargo.toml` to check; if absent, add to dev-dependencies).

2. **Add the migrate-library happy-path test:**
   ```rust
   #[test]
   fn migrate_library_converts_managed_symlinks_to_real_dirs() {
       let fix = build_v09_fixture();

       let output = assert_cmd::Command::cargo_bin("tome")
           .unwrap()
           .args([
               "migrate-library",
               "--config", fix.config_path.to_str().unwrap(),
               "--tome-home", fix.tome_home.to_str().unwrap(),
               "--machine", fix.machine_path.to_str().unwrap(),
           ])
           .output()
           .unwrap();
       let stderr = String::from_utf8_lossy(&output.stderr);
       let stdout = String::from_utf8_lossy(&output.stdout);

       // p1 and p2: managed symlinks should now be real directories with copied content.
       for n in &["p1", "p2"] {
           let dest = fix.library_dir.join(n);
           assert!(dest.is_dir(), "{n} must be a real directory after migration");
           assert!(!dest.is_symlink(), "{n} must NOT be a symlink after migration");
           assert!(dest.join("SKILL.md").is_file(), "{n}/SKILL.md must exist");
           let content = std::fs::read_to_string(dest.join("SKILL.md")).unwrap();
           assert_eq!(content, format!("# {n}"), "content for {n} must match source");
       }

       // l1: local skill, was already real-dir — UNCHANGED.
       let l1 = fix.library_dir.join("l1");
       assert!(l1.is_dir() && !l1.is_symlink());
       assert_eq!(std::fs::read_to_string(l1.join("SKILL.md")).unwrap(), "# l1");

       // broken: D-04 — symlink preserved, NOT deleted.
       let broken = fix.library_dir.join("broken");
       assert!(
           broken.is_symlink(),
           "broken symlink must be preserved per D-04, got: {stdout}\n{stderr}"
       );
       // D-04 stderr warning surfaced.
       assert!(
           stderr.contains("broken") && stderr.contains("unreachable"),
           "stderr must mention broken-source skip, got: {stderr}"
       );

       // user-symlink: D-03 conservatism — NOT in manifest, must be untouched.
       let user_sym = fix.library_dir.join("user-symlink");
       assert!(
           user_sym.is_symlink(),
           "user-created symlink (NOT in manifest) must be preserved per D-03"
       );

       // D-05: exit code non-zero because of the broken-symlink skip.
       assert!(!output.status.success(), "must exit non-zero on broken-symlink skip per D-05");

       // SAFE-01 banner format check.
       let combined = format!("{stdout}{stderr}");
       assert!(
           combined.contains("converted") && combined.contains("skipped"),
           "output must include SAFE-01 summary banner, got: {combined}"
       );
   }
   ```

3. **Add the dry-run test:**
   ```rust
   #[test]
   fn migrate_library_dry_run_makes_no_changes() {
       let fix = build_v09_fixture();

       // Snapshot library state pre-run.
       let p1_was_symlink = fix.library_dir.join("p1").is_symlink();
       let p2_was_symlink = fix.library_dir.join("p2").is_symlink();
       assert!(p1_was_symlink && p2_was_symlink, "fixture sanity");

       let output = assert_cmd::Command::cargo_bin("tome")
           .unwrap()
           .args([
               "migrate-library",
               "--dry-run",
               "--config", fix.config_path.to_str().unwrap(),
               "--tome-home", fix.tome_home.to_str().unwrap(),
               "--machine", fix.machine_path.to_str().unwrap(),
           ])
           .output()
           .unwrap();

       // Filesystem unchanged.
       assert!(fix.library_dir.join("p1").is_symlink(), "dry-run must not convert p1");
       assert!(fix.library_dir.join("p2").is_symlink(), "dry-run must not convert p2");
       assert!(fix.library_dir.join("broken").is_symlink(), "dry-run must not touch broken");

       // Output should mention dry-run.
       let combined = format!("{}{}",
           String::from_utf8_lossy(&output.stdout),
           String::from_utf8_lossy(&output.stderr));
       assert!(combined.contains("dry-run"), "output must mention dry-run, got: {combined}");
   }
   ```
  </action>
  <verify>
    <automated>cargo test --package tome --test cli migrate_library_converts_managed_symlinks_to_real_dirs migrate_library_dry_run_makes_no_changes</automated>
  </verify>
  <acceptance_criteria>
    - `rg -n "fn build_v09_fixture" crates/tome/tests/cli.rs` returns 1 match
    - `rg -n "fn migrate_library_converts_managed_symlinks_to_real_dirs|fn migrate_library_dry_run_makes_no_changes" crates/tome/tests/cli.rs` returns 2 matches
    - `cargo test --package tome --test cli migrate_library_converts_managed_symlinks_to_real_dirs` exits 0
    - `cargo test --package tome --test cli migrate_library_dry_run_makes_no_changes` exits 0
  </acceptance_criteria>
  <done>Fixture helper and the two core migrate-library tests are in place; happy-path conversion verified end-to-end; dry-run preserves filesystem state; broken-symlink and user-symlink boundary defenses are exercised.</done>
</task>

<task type="auto">
  <name>Task 2: Add tests for sync refuse-with-hint, source-removal preservation, post-migration idempotent sync</name>
  <files>crates/tome/tests/cli.rs</files>
  <read_first>
    - crates/tome/tests/cli.rs (after Task 1's additions)
    - crates/tome/src/lib.rs (refuse-with-hint message text from Plan 11-04 Task 2)
    - crates/tome/src/cleanup.rs (Case 1 transition behavior from Plan 11-03)
    - .planning/phases/11-library-canonical-core/11-CONTEXT.md (D-02, D-09, D-10)
  </read_first>
  <action>
1. **Add the sync refuse-with-hint test (D-02):**
   ```rust
   #[test]
   fn sync_refuses_on_v09_shape_library_with_hint() {
       let fix = build_v09_fixture();

       let output = assert_cmd::Command::cargo_bin("tome")
           .unwrap()
           .args([
               "sync",
               "--no-input",
               "--config", fix.config_path.to_str().unwrap(),
               "--tome-home", fix.tome_home.to_str().unwrap(),
               "--machine", fix.machine_path.to_str().unwrap(),
           ])
           .output()
           .unwrap();
       let stderr = String::from_utf8_lossy(&output.stderr);

       // D-02: sync must refuse with a Conflict/Why/Suggestion error.
       assert!(!output.status.success(), "sync must exit non-zero on v0.9-shape library");
       assert!(
           stderr.contains("v0.9 shape"),
           "stderr must mention 'v0.9 shape': {stderr}"
       );
       assert!(
           stderr.contains("tome migrate-library"),
           "stderr must point at `tome migrate-library`: {stderr}"
       );

       // Library must NOT have been modified by the refused sync.
       assert!(fix.library_dir.join("p1").is_symlink(), "refused sync must not modify library");
       assert!(fix.library_dir.join("p2").is_symlink());
   }
   ```

2. **Add the post-migration idempotent sync test:**
   ```rust
   #[test]
   fn sync_succeeds_after_migrate_library() {
       let fix = build_v09_fixture();

       // Remove the broken symlink first so migrate-library exits cleanly
       // (otherwise the broken-symlink D-04 path would block this test from
       // reaching the post-migration sync).
       std::fs::remove_file(fix.library_dir.join("broken")).unwrap();

       // Drop the broken manifest entry too — otherwise sync would still
       // detect a missing managed entry. Use a simple JSON edit since the
       // manifest is a plain serde-serialized BTreeMap.
       let manifest_path = fix.tome_home.join(".tome-manifest.json");
       let mut manifest: serde_json::Value =
           serde_json::from_str(&std::fs::read_to_string(&manifest_path).unwrap()).unwrap();
       manifest["skills"].as_object_mut().unwrap().remove("broken");
       std::fs::write(&manifest_path, serde_json::to_string_pretty(&manifest).unwrap()).unwrap();

       // Step 1: migrate-library.
       let migrate = assert_cmd::Command::cargo_bin("tome")
           .unwrap()
           .args([
               "migrate-library",
               "--config", fix.config_path.to_str().unwrap(),
               "--tome-home", fix.tome_home.to_str().unwrap(),
               "--machine", fix.machine_path.to_str().unwrap(),
           ])
           .output()
           .unwrap();
       assert!(
           migrate.status.success(),
           "migrate-library must succeed cleanly when no broken symlinks remain.\nstdout: {}\nstderr: {}",
           String::from_utf8_lossy(&migrate.stdout),
           String::from_utf8_lossy(&migrate.stderr),
       );

       // Step 2: sync. Should NOT refuse anymore (no v0.9-shape symlinks left).
       let sync = assert_cmd::Command::cargo_bin("tome")
           .unwrap()
           .args([
               "sync",
               "--no-input",
               "--config", fix.config_path.to_str().unwrap(),
               "--tome-home", fix.tome_home.to_str().unwrap(),
               "--machine", fix.machine_path.to_str().unwrap(),
           ])
           .output()
           .unwrap();

       let sync_stderr = String::from_utf8_lossy(&sync.stderr);
       assert!(
           !sync_stderr.contains("v0.9 shape"),
           "sync after migrate must NOT refuse with v0.9 hint, got: {sync_stderr}"
       );
       // We don't assert sync.status.success() because the synthetic fixture
       // doesn't have a real claude-plugins source so the managed entries
       // become orphans — but that's a Phase 13 concern. The KEY assertion
       // is that the v0.9 refuse-with-hint check no longer fires.
   }
   ```

3. **Add the source-removal → Unowned preservation test (LIB-04 / D-09 Case 1 / D-10 trigger 2):**
   ```rust
   #[test]
   fn sync_preserves_library_when_source_removed_from_config() {
       use std::os::unix::fs as unix_fs;
       let root = assert_fs::TempDir::new().unwrap();
       let tome_home = root.path().join("tome_home");
       let library_dir = tome_home.join("skills");
       let local_source = root.path().join("local_source");
       std::fs::create_dir_all(&library_dir).unwrap();
       std::fs::create_dir_all(&local_source).unwrap();

       // Create a local skill in source and pre-populate library + manifest.
       let src = local_source.join("alpha");
       std::fs::create_dir_all(&src).unwrap();
       std::fs::write(src.join("SKILL.md"), "# alpha").unwrap();

       let lib_alpha = library_dir.join("alpha");
       std::fs::create_dir_all(&lib_alpha).unwrap();
       std::fs::write(lib_alpha.join("SKILL.md"), "# alpha").unwrap();

       let alpha_hash = sha256_dir_simple(&lib_alpha);
       let manifest_json = serde_json::json!({
           "skills": {
               "alpha": {
                   "source_path": src.to_string_lossy(),
                   "source_name": "local",
                   "content_hash": alpha_hash,
                   "synced_at": "2024-01-01T00:00:00Z",
                   "managed": false
               }
           }
       });
       std::fs::write(
           tome_home.join(".tome-manifest.json"),
           serde_json::to_string_pretty(&manifest_json).unwrap(),
       )
       .unwrap();

       // Initial config with the source dir present.
       let config_path = tome_home.join("tome.toml");
       let machine_path = root.path().join("machine.toml");
       std::fs::write(&machine_path, "").unwrap();
       let config_with_source = format!(
           r#"library_dir = "{}"

[directories.local]
path = "{}"
type = "directory"
role = "source"
"#,
           library_dir.display(),
           local_source.display(),
       );
       std::fs::write(&config_path, &config_with_source).unwrap();

       // Step 1: edit tome.toml to remove the [directories.local] entry
       // (simulates D-10 trigger 2: user edits config outside `tome remove`).
       let config_without_source = format!(
           r#"library_dir = "{}"
"#,
           library_dir.display(),
       );
       std::fs::write(&config_path, &config_without_source).unwrap();

       // Step 2: run sync. Cleanup phase should detect the orphan (alpha's
       // source_name "local" is no longer in config.directories) and
       // transition it to Unowned — preserving the library content per LIB-04.
       let sync = assert_cmd::Command::cargo_bin("tome")
           .unwrap()
           .args([
               "sync",
               "--no-input",
               "--config", config_path.to_str().unwrap(),
               "--tome-home", tome_home.to_str().unwrap(),
               "--machine", machine_path.to_str().unwrap(),
           ])
           .output()
           .unwrap();
       let sync_stderr = String::from_utf8_lossy(&sync.stderr);
       let sync_stdout = String::from_utf8_lossy(&sync.stdout);

       // The library directory MUST still exist with the same content.
       assert!(
           library_dir.join("alpha").is_dir(),
           "LIB-04: library content must be preserved on source removal.\nstdout: {sync_stdout}\nstderr: {sync_stderr}"
       );
       let preserved = std::fs::read_to_string(library_dir.join("alpha/SKILL.md")).unwrap();
       assert_eq!(preserved, "# alpha", "library content must be unchanged");

       // The manifest entry must have transitioned to Unowned (source_name omitted/null).
       let manifest_after: serde_json::Value =
           serde_json::from_str(&std::fs::read_to_string(tome_home.join(".tome-manifest.json")).unwrap())
               .unwrap();
       let alpha_entry = &manifest_after["skills"]["alpha"];
       assert!(
           alpha_entry.get("source_name").map(|v| v.is_null()).unwrap_or(true),
           "manifest entry's source_name must be omitted or null after source removal: {alpha_entry}"
       );
       // content_hash unchanged.
       assert_eq!(
           alpha_entry["content_hash"].as_str().unwrap(),
           alpha_hash,
           "content_hash must remain unchanged across the Case 1 transition"
       );
   }
   ```
  </action>
  <verify>
    <automated>cargo test --package tome --test cli sync_refuses_on_v09_shape_library_with_hint sync_succeeds_after_migrate_library sync_preserves_library_when_source_removed_from_config</automated>
  </verify>
  <acceptance_criteria>
    - `rg -n "fn sync_refuses_on_v09_shape_library_with_hint|fn sync_succeeds_after_migrate_library|fn sync_preserves_library_when_source_removed_from_config" crates/tome/tests/cli.rs` returns 3 matches
    - `cargo test --package tome --test cli sync_refuses_on_v09_shape_library_with_hint` exits 0
    - `cargo test --package tome --test cli sync_succeeds_after_migrate_library` exits 0
    - `cargo test --package tome --test cli sync_preserves_library_when_source_removed_from_config` exits 0
    - `cargo test --package tome --test cli` (full integration suite) exits 0 — no regressions in existing tests
    - `make ci` exits 0
  </acceptance_criteria>
  <done>Three end-to-end CLI tests cover D-02 sync refuse-with-hint, D-09/D-10 trigger 2 source-removal preservation, and post-migration idempotent sync; the full integration suite still passes; the phase's success criteria 1, 2, 4 are now anchored by binary-level assertions.</done>
</task>

</tasks>

<verification>
- `cargo test --package tome --test cli` exits 0 (full integration suite passes)
- `cargo test --package tome` exits 0 (full unit + integration suite passes)
- `make ci` exits 0
- All five tests added in this plan pass:
  - `migrate_library_converts_managed_symlinks_to_real_dirs`
  - `migrate_library_dry_run_makes_no_changes`
  - `sync_refuses_on_v09_shape_library_with_hint`
  - `sync_succeeds_after_migrate_library`
  - `sync_preserves_library_when_source_removed_from_config`
</verification>

<success_criteria>
- ROADMAP.md Phase 11 success criterion 1 (zero symlinks after sync for managed) anchored by `migrate_library_converts_managed_symlinks_to_real_dirs`.
- ROADMAP.md Phase 11 success criterion 2 (source removal preserves library) anchored by `sync_preserves_library_when_source_removed_from_config`.
- ROADMAP.md Phase 11 success criterion 4 (migration idempotency / refuse-with-hint workflow) anchored by `sync_refuses_on_v09_shape_library_with_hint` + `sync_succeeds_after_migrate_library`.
- The user-created-symlink and broken-symlink boundary defenses (D-03, D-04) are tested at the binary level.
- The synthetic fixture mirrors Martin's real library shape per CONTEXT.md `<specifics>`, so Phase 17 / REL-04 (real-library smoke test) has high confidence the synthetic test transfers to production.
</success_criteria>

<output>
After completion, create `.planning/phases/11-library-canonical-core/11-05-SUMMARY.md`
documenting: the synthetic v0.9 fixture shape, all five integration tests added,
the success criteria they anchor, and any quirks (e.g. SHA-256 helper duplication
inside the test file vs. crate-internal `manifest::hash_directory` — note this for
HARD-13 future test-file split).
</output>
