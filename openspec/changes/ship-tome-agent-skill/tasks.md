## 1. Tested Skill Package

- [x] 1.1 Run and record baseline agent scenarios for role selection, nested repository installation, and safe troubleshooting
- [x] 1.2 Create `skills/using-tome/SKILL.md` with trigger metadata, operating contract, role decisions, command map, and progressive-disclosure links
- [x] 1.3 Create focused configuration, operations, and troubleshooting references
- [x] 1.4 Validate the skill with Tome lint and rerun the baseline scenarios as passing behavior tests

## 2. Local Path Addition

- [x] 2.1 Add failing classification and CLI tests for explicit local paths, managed role, default name and role, portable save, and Git-only flag rejection
- [x] 2.2 Implement deterministic local-vs-Git source classification without filesystem-existence inference
- [x] 2.3 Construct and checked-save directory-type entries while preserving existing Git behavior
- [x] 2.4 Run focused add tests, format, and Clippy checks

## 3. Claude Distribution

- [x] 3.1 Add plugin metadata for `tome` version `1.0.0`
- [x] 3.2 Add marketplace metadata exposing the repository-root plugin as `tome@tome`
- [x] 3.3 Validate JSON, Claude schemas, and isolated local marketplace installation

## 4. Canonical Desktop Data Folder

- [x] 4.1 Add failing Rust coverage for canonical `tome_home` independent from `library_dir`
- [x] 4.2 Add `tome_home` to `StatusReport` and regenerate TypeScript bindings
- [x] 4.3 Replace the desktop heuristic with `TOME DATA FOLDER`, separate library display, and explanatory copy
- [x] 4.4 Add React rendering coverage and run focused Rust/UI checks

## 5. Interactive Init Recommendation

- [x] 5.1 Add tests for canonical custom data-folder threading and existing `.tome/tome.toml` detection
- [x] 5.2 Move Step 0 selection before machine-state detection and update user-facing data-folder copy
- [x] 5.3 Add failing unit tests for official source construction, insertion, equivalent-source detection, and name-collision preservation
- [x] 5.4 Add CLI regression coverage proving `--no-input` omits the recommendation
- [x] 5.5 Implement pure official-source helpers and the default-yes interactive prompt with clone disclosure
- [x] 5.6 Run focused wizard and init tests plus Rust format and Clippy checks

## 6. Documentation And Closure

- [ ] 6.1 Document local-path add behavior plus cross-tool and Claude plugin installation in README and command reference
- [ ] 6.2 Restore the Unreleased changelog section and current comparison links
- [ ] 6.3 Refine the existing auto-subdirectory todo with the official repository regression fixture without implementing the feature
- [ ] 6.4 Complete the fulfilled agent-skills planning todo through `gsd-sdk`
- [ ] 6.5 Run skill, plugin, focused Rust/UI, and full `make ci` verification
