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

- [ ] 3.1 Add plugin metadata for `tome` version `1.0.0`
- [ ] 3.2 Add marketplace metadata exposing the repository-root plugin as `tome@tome`
- [ ] 3.3 Validate JSON, Claude schemas, and isolated local marketplace installation

## 4. Interactive Init Recommendation

- [ ] 4.1 Add failing unit tests for official source construction, insertion, equivalent-source detection, and name-collision preservation
- [ ] 4.2 Add CLI regression coverage proving `--no-input` omits the recommendation
- [ ] 4.3 Implement pure official-source helpers and the default-yes interactive prompt with clone disclosure
- [ ] 4.4 Run focused wizard and init tests plus Rust format and Clippy checks

## 5. Documentation And Closure

- [ ] 5.1 Document local-path add behavior plus cross-tool and Claude plugin installation in README and command reference
- [ ] 5.2 Restore the Unreleased changelog section and current comparison links
- [ ] 5.3 Refine the existing auto-subdirectory todo with the official repository regression fixture without implementing the feature
- [ ] 5.4 Complete the fulfilled agent-skills planning todo through `gsd-sdk`
- [ ] 5.5 Run skill, plugin, focused Rust, and full `make ci` verification
