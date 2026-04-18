# Domain Pitfalls

**Domain:** Wizard rewrite — unified directory model (tome v0.7)  
**Researched:** 2026-04-16  
**Confidence:** HIGH (code-informed, single-crate analysis)

## Critical Pitfalls

Mistakes that cause rewrites or major issues.

### Pitfall 1: Wizard bypasses config validation — invalid type/role combos save successfully

**What goes wrong:** The wizard builds `DirectoryConfig` structs in memory and passes them directly to `Config` without round-tripping through TOML deserialization. The `Deserialize` impl in `config.rs` (lines 344-370) validates type/role combinations (e.g., rejects `Git + Target`, rejects `Directory + Managed`), but the wizard constructs structs directly and never hits those checks. If the role-editing loop or custom directory flow produces an invalid combination, it saves without error.

**Why it happens:** Two validation paths exist: the serde `Deserialize` impl (used when loading from disk) and the wizard's in-memory construction. The wizard relies on `valid_roles()` to filter the role picker, but that filter can be bypassed during refactoring.

**Consequences:** Config saves successfully but `tome sync` fails on next run with a confusing validation error. User blames sync, not init. Worse, if `deny_unknown_fields` is not set, the invalid config might partially work and produce subtle bugs.

**Prevention:**
- Call `Config::validate()` (or round-trip through `toml::from_str(toml::to_string_pretty(&config)?)`) on the assembled config before saving.
- Unit test: construct every `(DirectoryType, DirectoryRole)` combination via the wizard's struct-building path and assert invalid ones are caught before save.

**Detection:** Integration test: wizard output round-trips through TOML serialization/deserialization without error.

**Phase:** Must be in the wizard rewrite PR. Add the validation call before `config.save()`.

### Pitfall 2: Synced role creates circular symlinks when library_dir overlaps a synced directory

**What goes wrong:** A `Synced` directory is both a discovery source and a distribution target. If the library directory lives inside (or is a parent of) a synced directory, the distribute step creates symlinks pointing back into the library, causing infinite loops or duplicate skill entries on every sync.

**Why it happens:** The old model had separate source and target lists, making this architecturally impossible. With `Synced`, the same path participates in both pipelines. The wizard doesn't validate that `library_dir` doesn't overlap with any distribution directory.

**Consequences:** `tome sync` loops or duplicates every skill. Manifest hashes change on every run (non-idempotent sync). Potentially fills disk with symlink chains.

**Prevention:**
- At the wizard summary step (before save), validate that `library_dir` does not overlap with any directory where `is_distribution() == true`.
- Port the existing manifest-based circular symlink detection to also cover the wizard's config assembly.

**Detection:** Test: synced directory containing library_dir should produce a wizard error, not a saved config. `tome doctor` should also flag this.

**Phase:** Wizard rewrite PR. Add overlap check before the summary table.

### Pitfall 3: BTreeMap ordering silently changes duplicate resolution vs. wizard display order

**What goes wrong:** When two directories contain a skill with the same name, `discover_all` resolves by alphabetical BTreeMap key order (first key wins). The wizard presents directories in `KNOWN_DIRECTORIES` array order (roughly by popularity). Users see "claude-skills" listed first in the wizard prompt but "amp" wins in the BTreeMap because `"a" < "c"`.

**Why it happens:** Display order (KNOWN_DIRECTORIES array position) and resolution order (BTreeMap key sort) are different.

**Consequences:** User selects directories expecting one priority order but gets another. Skills from unexpected sources appear in the library. Hard to debug because the wizard shows one order and sync follows another.

**Prevention:**
- Sort the summary table alphabetically (matching BTreeMap iteration order) so users see actual resolution order.
- Add a note: "In case of duplicate skill names, the first directory alphabetically wins."
- Run `discover_all` before the summary step (already done for exclusions) and show warnings about duplicate skill names across selected directories.

**Detection:** Test: two directories with overlapping skill names, verify the wizard warns about the conflict.

**Phase:** Wizard rewrite PR. Add conflict detection to the summary step.

## Moderate Pitfalls

### Pitfall 4: Empty directory selection produces a useless but valid config

**What goes wrong:** `MultiSelect` with all defaults `true` — if the user deselects everything and hits Enter, the result is an empty `Vec`. The wizard proceeds to save a config with zero directories. `tome sync` does nothing.

**Why it happens:** dialoguer's `MultiSelect` treats empty selection as valid. The wizard doesn't guard against it.

**Consequences:** User runs `tome sync`, nothing happens, no error. Has to re-run `tome init`.

**Prevention:** After MultiSelect for directories, check `selections.is_empty()`. If so, warn and re-prompt, or proceed only if user confirms "No directories selected. Continue anyway?"

**Detection:** Unit test: assert wizard rejects or warns on zero-directory configs.

**Phase:** Wizard rewrite PR.

### Pitfall 5: Custom directory path overlaps with existing entries

**What goes wrong:** The "Add a custom directory" flow accepts any path, with no check for: path already registered under a different name, path is a parent/child of an existing entry, resolved path matches another entry's resolved path.

**Why it happens:** Auto-discovered directories have pre-validated unique paths (from KNOWN_DIRECTORIES). Custom directories skip that.

**Consequences:**
- Two entries pointing to the same resolved path with different names — undefined sync behavior.
- Overlapping subtrees — one directory is a parent of another, causing duplicate skill discovery.
- Worse: a custom entry that overlaps with `library_dir` creates the circular symlink problem (Pitfall 2).

**Prevention:**
- Block if resolved path is already registered under another name.
- Warn if path is a parent/child of an existing directory.
- Validate against `library_dir` for overlap.

**Detection:** Add `validate_no_path_overlaps()` helper. Test with overlapping paths.

**Phase:** Wizard rewrite PR. Add validation before inserting custom directories.

### Pitfall 6: Test coverage gap — interactive flows are untestable with dialoguer

**What goes wrong:** The wizard's `run()` function directly calls `dialoguer::MultiSelect::interact()`, `Confirm::interact()`, etc. These require a real TTY. The current test suite only covers `find_known_directories_in()` and registry invariants — zero coverage on wizard flow logic (role assignment, config assembly, validation).

**Why it happens:** dialoguer has no built-in mock/fake. The wizard function is monolithic — all logic and I/O in one function.

**Consequences:** Regressions in wizard logic are only caught by manual testing. Refactors are risky. The rewrite itself is the highest-risk change with the lowest test coverage.

**Prevention:** Structure the wizard as separable layers:
1. **Data gathering** (dialoguer — untestable, keep thin)
2. **Config assembly** (pure function: selected directories + library path + exclusions -> Config — fully testable)
3. **Validation** (pure function: Config -> Result<(), Vec<Warning>> — fully testable)
4. **Presentation** (summary table formatting — testable with snapshot tests)

Test layers 2, 3, 4 exhaustively. Layer 1 stays as thin as possible.

**Detection:** Track which wizard code paths have test coverage. After rewrite, every logic branch in layers 2-4 should have a test.

**Phase:** Wizard rewrite PR. Extract testable core before adding new features.

### Pitfall 7: DirectoryName collision after case normalization

**What goes wrong:** `DirectoryName::new()` validates basic rules (no empty, no path separators) but doesn't normalize case. Custom names like "Claude-Skills" and "claude-skills" are different BTreeMap keys but look identical to humans. One silently shadows the other.

**Why it happens:** `DirectoryName` wraps a `String` with case-sensitive comparison. KNOWN_DIRECTORIES uses lowercase-kebab, but custom names have no such enforcement.

**Consequences:** Config has two near-duplicate entries. Confusing behavior, hard to debug.

**Prevention:**
- In the custom directory flow, normalize to lowercase-kebab before insertion.
- Check for case-insensitive duplicates against existing entries and warn/block.
- The `DirectoryName` type has `check_convention()` — call it in the wizard and show the warning.

**Detection:** Test: inserting "My-Dir" when "my-dir" exists should warn or normalize.

**Phase:** Wizard rewrite PR.

## Minor Pitfalls

### Pitfall 8: Summary table truncates long paths

**What goes wrong:** `show_directory_summary()` uses fixed-width formatting (`{:<35}` for paths). Git source paths (`~/.tome/repos/<sha256>/subdir`) routinely exceed 35 characters and misalign the table.

**Prevention:** Use `tabled` crate (already a dependency) for the summary table instead of manual `format!` strings.

**Phase:** Wizard rewrite PR. Low effort, high polish.

### Pitfall 9: Git directories not addable in wizard

**What goes wrong:** The wizard's custom directory flow only offers "directory" and "claude-plugins" types. Git directories require `tome add <url>` post-wizard. No mention of this in the wizard, so users expecting to add git repos during init are stuck.

**Prevention:** Add a note after the custom directory loop: "For git-based skill repos, use `tome add <url>` after setup."

**Phase:** Wizard rewrite PR. One-line println.

### Pitfall 10: Dry-run path uses different serialization than save path

**What goes wrong:** Dry-run calls `toml::to_string_pretty(&config)` directly. The actual save path goes through `config.save()` which may format differently (e.g., custom serialization, comments). The dry-run output doesn't match what would actually be saved.

**Prevention:** Extract `Config::to_toml_string()` and use it in both dry-run display and `config.save()`.

**Phase:** Wizard rewrite PR. Minor refactor.

### Pitfall 11: Role editing for ClaudePlugins shows "No editable directories" when it's the only entry

**What goes wrong:** The role-editing loop filters out ClaudePlugins directories (they're always Managed). If the user only has `claude-plugins` configured, they see "No editable directories" and can't understand why.

**Prevention:** Show a more helpful message: "ClaudePlugins directories are always Managed. Other directory types support role changes."

**Phase:** Wizard rewrite PR.

## Phase-Specific Warnings

| Phase Topic | Likely Pitfall | Mitigation |
|-------------|---------------|------------|
| KNOWN_DIRECTORIES registry | Already merged; verify no stale KNOWN_SOURCES/KNOWN_TARGETS references in tests/docs | `rg` search for old names before closing |
| Role auto-assignment | Synced role on directories overlapping library_dir (Pitfall 2) | Validate library_dir against all distribution directories |
| Summary table | Fixed-width formatting breaks on long paths (Pitfall 8) | Use `tabled` or dynamic column widths |
| Custom directory addition | No duplicate/overlap detection (Pitfall 5) | Add validation helper before insertion |
| Interactive testing | Zero test coverage on wizard flow logic (Pitfall 6) | Extract testable core; test config assembly as pure function |
| Config assembly | Bypasses deserialization validation (Pitfall 1) | Validate or round-trip assembled config before save |
| BTreeMap ordering | Display order != resolution order (Pitfall 3) | Sort summary alphabetically; warn on skill name conflicts |
| Name validation | Case-insensitive collisions (Pitfall 7) | Normalize custom names to lowercase-kebab |

## Sources

- `/Users/martin/dev/opensource/tome/crates/tome/src/wizard.rs` — current wizard implementation (full analysis)
- `/Users/martin/dev/opensource/tome/crates/tome/src/config.rs` — DirectoryType, DirectoryRole, valid_roles(), deserialization validation
- `/Users/martin/dev/opensource/tome/crates/tome/src/discover.rs` — discover_all(), BTreeMap dedup behavior
- `/Users/martin/dev/opensource/tome/.planning/PROJECT.md` — v0.7 milestone goals, active requirements
- [dialoguer crate docs](https://docs.rs/dialoguer/latest/dialoguer/) — MultiSelect behavior, no mock support

---

*Pitfalls research: 2026-04-16*
