//! Validation logic for [`Config`] — role/type combos and library/distribution overlap.
//!
//! Hosts:
//! - `Config::validate` — the public entry point called by `Config::load` and `save_checked`.
//! - `path_contains` — lexical-only path-prefix helper (no canonicalisation).
//!
//! Cases A/B/C overlap detection (Phase 4 WHARD-01) is the bulk of `validate()`:
//! library_dir vs distribution-dir equality (A), library inside dist (B),
//! dist inside library (C).

use anyhow::Result;
use std::path::Path;

use super::types::{Config, DirectoryRole, DirectoryType};
use crate::paths::expand_tilde;

impl Config {
    /// Validate config for common misconfigurations.
    ///
    /// Checks:
    /// - library_dir is not a file
    /// - Role/type combos are valid (Managed only for ClaudePlugins, Target not for Git)
    /// - Git fields (branch/tag/rev) only on Git type directories
    pub fn validate(&self) -> Result<()> {
        // library_dir exists but is a file, not a directory
        if self.library_dir.exists() && !self.library_dir.is_dir() {
            anyhow::bail!(
                "library_dir exists but is not a directory: {}",
                self.library_dir.display()
            );
        }

        for (name, dir) in &self.directories {
            let role = dir.role();

            // Managed role only valid with ClaudePlugins type
            if role == DirectoryRole::Managed && dir.directory_type != DirectoryType::ClaudePlugins
            {
                anyhow::bail!(
                    "directory '{name}': role/type conflict\n\
                     Conflict: role is {} but type is '{}'\n\
                     Why: the Managed role means skills are owned by a package manager; only the claude-plugins type is known to behave this way, so any other type with Managed would be sync'd incorrectly.\n\
                     hint: either change type to 'claude-plugins', or change role to {} or {}.",
                    DirectoryRole::Managed.description(),
                    dir.directory_type,
                    DirectoryRole::Synced.description(),
                    DirectoryRole::Source.description(),
                );
            }

            // Target role invalid with Git type
            if role == DirectoryRole::Target && dir.directory_type == DirectoryType::Git {
                anyhow::bail!(
                    "directory '{name}': role/type conflict\n\
                     Conflict: role is {} but type is 'git'\n\
                     Why: the Target role means skills are distributed into this directory, but git-type directories are remote clones that tome must not write skills into — pushing symlinks into a git clone would clash with the working tree.\n\
                     hint: change role to {} (git repos are read-only skill sources).",
                    DirectoryRole::Target.description(),
                    DirectoryRole::Source.description(),
                );
            }

            // Catch-all: enforce the DirectoryType::valid_roles() contract.
            // The specific Managed-only and Target-not-on-Git rejections above
            // produce tailored hints for their common cases; this fallback
            // covers the remaining valid_roles()-violating combos so the
            // validator never disagrees with the wizard's role-picker filter.
            //
            // Examples this catches (not already caught above):
            //   - ClaudePlugins + Synced / Source / Target
            //   - Git + Synced
            let valid = dir.directory_type.valid_roles();
            if !valid.contains(&role) {
                let valid_descriptions: Vec<&'static str> =
                    valid.iter().map(|r| r.description()).collect();
                anyhow::bail!(
                    "directory '{name}': role/type conflict\n\
                     Conflict: role is {} but type is '{}' accepts only: {}\n\
                     Why: each directory type has a fixed set of roles it supports; other combinations would be sync'd incorrectly.\n\
                     hint: change role to one of: {}.",
                    role.description(),
                    dir.directory_type,
                    valid_descriptions.join(", "),
                    valid_descriptions.join(" or "),
                );
            }

            // Git ref only valid with Git type. Mutual exclusion of
            // branch/tag/rev is enforced at deserialize time by
            // `TryFrom<DirectoryConfigRaw> for DirectoryConfig` (closes
            // #490), so we only need to check the type here.
            if dir.git_ref.is_some() && dir.directory_type != DirectoryType::Git {
                anyhow::bail!(
                    "directory '{name}': git ref pin on non-git directory\n\
                     Conflict: branch/tag/rev is set but type is '{}'\n\
                     Why: branch, tag, and rev pin a remote git clone to a specific commit; they have no meaning for a local directory or a claude-plugins cache.\n\
                     hint: either change type to 'git', or remove the branch/tag/rev fields from this directory.",
                    dir.directory_type,
                );
            }

            // subdir only valid with Git type
            if dir.subdir.is_some() && dir.directory_type != DirectoryType::Git {
                anyhow::bail!(
                    "directory '{name}': subdir on non-git directory\n\
                     Conflict: subdir is set but type is '{}'\n\
                     Why: subdir scopes skill discovery to a sub-path within a remote git clone; for a plain directory you can just point 'path' at the sub-path directly.\n\
                     hint: either change type to 'git', or remove 'subdir' and adjust 'path' to point where skills actually live.",
                    dir.directory_type,
                );
            }
        }

        // --- Path overlap between library_dir and distribution directories ---
        // Lexical only: tilde-expand both sides, normalize trailing '/', compare
        // without hitting the filesystem. Scope is library_dir vs each
        // distribution (Synced or Target) directory — Source dirs are read-only
        // and never written to, so they cannot self-loop at sync time.
        let lib = expand_tilde(&self.library_dir)?;
        for (name, dir) in self.distribution_dirs() {
            let dist = expand_tilde(&dir.path)?;
            let role_desc = dir.role().description();

            // Case A: exact equality (also tolerates a trailing '/' on either side)
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

            // Case B: library_dir is inside the distribution directory — the
            // "library lives inside a synced tree" circular-symlink case.
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

        Ok(())
    }
}

/// Check whether `ancestor` is a path-prefix of `descendant` (or equal),
/// with trailing-separator normalization so that `/foo/bar` does NOT contain
/// `/foo/barbaz`.
///
/// Lexical only — no canonicalization. Both inputs must already be
/// tilde-expanded by the caller.
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

#[cfg(test)]
mod tests {
    use super::super::types::{
        Config, DirectoryConfig, DirectoryName, DirectoryRole, DirectoryType, GitRef,
    };
    use std::collections::BTreeMap;
    use std::path::PathBuf;

    // --- Config validation tests ---

    #[test]
    fn validate_rejects_managed_with_directory_type() {
        let config = Config {
            directories: BTreeMap::from([(
                DirectoryName::new("bad").unwrap(),
                DirectoryConfig {
                    path: PathBuf::from("/tmp"),
                    directory_type: DirectoryType::Directory,
                    role: Some(DirectoryRole::Managed),
                    git_ref: None,
                    subdir: None,
                    override_applied: false,
                },
            )]),
            ..Default::default()
        };
        let err = config.validate().unwrap_err();
        let msg = err.to_string();
        assert!(
            msg.contains("Managed (read-only, owned by package manager)"),
            "missing role description: {msg}"
        );
        assert!(msg.contains("directory"), "missing type name: {msg}");
        assert!(msg.contains("hint:"), "missing hint line: {msg}");
    }

    #[test]
    fn validate_rejects_target_with_git_type() {
        let config = Config {
            directories: BTreeMap::from([(
                DirectoryName::new("bad").unwrap(),
                DirectoryConfig {
                    path: PathBuf::from("/tmp"),
                    directory_type: DirectoryType::Git,
                    role: Some(DirectoryRole::Target),
                    git_ref: None,
                    subdir: None,
                    override_applied: false,
                },
            )]),
            ..Default::default()
        };
        let err = config.validate().unwrap_err();
        let msg = err.to_string();
        assert!(
            msg.contains("Target (skills distributed here, not discovered here)"),
            "missing role description: {msg}"
        );
        assert!(msg.contains("git"), "missing type name: {msg}");
        assert!(msg.contains("hint:"), "missing hint line: {msg}");
    }

    #[test]
    fn validate_rejects_git_fields_with_non_git_type() {
        let config = Config {
            directories: BTreeMap::from([(
                DirectoryName::new("bad").unwrap(),
                DirectoryConfig {
                    path: PathBuf::from("/tmp"),
                    directory_type: DirectoryType::Directory,
                    role: None,
                    git_ref: Some(GitRef::Branch("main".to_string())),
                    subdir: None,
                    override_applied: false,
                },
            )]),
            ..Default::default()
        };
        let err = config.validate().unwrap_err();
        let msg = err.to_string();
        assert!(
            msg.contains("branch") || msg.contains("tag") || msg.contains("rev"),
            "missing git-field mention: {msg}"
        );
        assert!(msg.contains("git"), "missing type name: {msg}");
        assert!(msg.contains("hint:"), "missing hint line: {msg}");
    }

    #[test]
    fn validate_rejects_subdir_with_non_git_type() {
        let config = Config {
            directories: BTreeMap::from([(
                DirectoryName::new("bad").unwrap(),
                DirectoryConfig {
                    path: PathBuf::from("/tmp"),
                    directory_type: DirectoryType::Directory,
                    role: None,
                    git_ref: None,
                    subdir: Some("nested".to_string()),
                    override_applied: false,
                },
            )]),
            ..Default::default()
        };
        let err = config.validate().unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("subdir"), "missing 'subdir': {msg}");
        assert!(msg.contains("git"), "missing type name: {msg}");
        assert!(msg.contains("hint:"), "missing hint line: {msg}");
    }

    #[test]
    fn validate_passes_for_valid_config() {
        let config = Config {
            library_dir: PathBuf::from("/tmp/nonexistent-lib"),
            directories: BTreeMap::from([
                (
                    DirectoryName::new("claude-plugins").unwrap(),
                    DirectoryConfig {
                        path: PathBuf::from("/tmp/plugins"),
                        directory_type: DirectoryType::ClaudePlugins,
                        role: Some(DirectoryRole::Managed),
                        git_ref: None,

                        subdir: None,
                        override_applied: false,
                    },
                ),
                (
                    DirectoryName::new("my-skills").unwrap(),
                    DirectoryConfig {
                        path: PathBuf::from("/tmp/skills"),
                        directory_type: DirectoryType::Directory,
                        role: None, // defaults to Synced
                        git_ref: None,

                        subdir: None,
                        override_applied: false,
                    },
                ),
            ]),
            ..Default::default()
        };
        config.validate().unwrap();
    }

    #[test]
    fn validate_rejects_library_dir_that_is_a_file() {
        let dir = tempfile::TempDir::new().unwrap();
        let file_path = dir.path().join("not-a-dir");
        std::fs::write(&file_path, "I'm a file").unwrap();

        let config = Config {
            library_dir: file_path,
            ..Default::default()
        };
        let err = config.validate().unwrap_err();
        assert!(
            err.to_string().contains("not a directory"),
            "unexpected error: {err}"
        );
    }

    // --- Overlap tests (library_dir vs distribution directories) ---

    fn dir_cfg(path: &str, dt: DirectoryType, role: Option<DirectoryRole>) -> DirectoryConfig {
        DirectoryConfig {
            path: PathBuf::from(path),
            directory_type: dt,
            role,
            git_ref: None,
            subdir: None,
            override_applied: false,
        }
    }

    #[test]
    fn validate_rejects_library_equals_distribution() {
        let config = Config {
            library_dir: PathBuf::from("/tmp/shared"),
            directories: BTreeMap::from([(
                DirectoryName::new("shared").unwrap(),
                dir_cfg(
                    "/tmp/shared",
                    DirectoryType::Directory,
                    Some(DirectoryRole::Synced),
                ),
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
                dir_cfg(
                    "/tmp/outer",
                    DirectoryType::Directory,
                    Some(DirectoryRole::Synced),
                ),
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
                dir_cfg(
                    "/tmp/outer/inner",
                    DirectoryType::Directory,
                    Some(DirectoryRole::Target),
                ),
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
                dir_cfg(
                    "/tmp/foobar",
                    DirectoryType::Directory,
                    Some(DirectoryRole::Synced),
                ),
            )]),
            ..Default::default()
        };
        config
            .validate()
            .expect("sibling paths must not trigger overlap");
    }

    #[test]
    fn validate_rejects_equality_despite_trailing_separator() {
        let config = Config {
            library_dir: PathBuf::from("/tmp/lib/"),
            directories: BTreeMap::from([(
                DirectoryName::new("lib").unwrap(),
                dir_cfg(
                    "/tmp/lib",
                    DirectoryType::Directory,
                    Some(DirectoryRole::Synced),
                ),
            )]),
            ..Default::default()
        };
        let msg = config.validate().unwrap_err().to_string();
        assert!(msg.contains("Conflict:"), "missing Conflict line: {msg}");
    }

    #[test]
    fn validate_accepts_source_role_inside_library() {
        // Source dirs don't participate in distribution — no self-loop risk.
        let config = Config {
            library_dir: PathBuf::from("/tmp/outer"),
            directories: BTreeMap::from([(
                DirectoryName::new("inner-source").unwrap(),
                dir_cfg(
                    "/tmp/outer/inner",
                    DirectoryType::Directory,
                    Some(DirectoryRole::Source),
                ),
            )]),
            ..Default::default()
        };
        config
            .validate()
            .expect("Source-role nesting must not trigger overlap");
    }

    #[test]
    fn validate_rejects_tilde_equal_paths() {
        // Both library_dir and directory path use tilde; must expand before compare.
        let config = Config {
            library_dir: PathBuf::from("~/.tome/skills"),
            directories: BTreeMap::from([(
                DirectoryName::new("same").unwrap(),
                dir_cfg(
                    "~/.tome/skills",
                    DirectoryType::Directory,
                    Some(DirectoryRole::Synced),
                ),
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

    // --- Cross-product (DirectoryType, DirectoryRole) matrix ---
    //
    // Every combination is tested, no exclusions. Expected pass/fail is
    // derived from `DirectoryType::valid_roles().contains(&role)` at runtime,
    // NOT a hand-written table — this means drift between the wizard's role
    // picker and the validator is impossible: update valid_roles() and the
    // expectations update automatically.
    //
    // Invalid combos additionally assert the error message contains the role's
    // description() substring plus the literal "hint:" — the same
    // Conflict+Why+Suggestion shape produced by the other validator bails.

    const ALL_TYPES_FOR_MATRIX: [DirectoryType; 3] = [
        DirectoryType::ClaudePlugins,
        DirectoryType::Directory,
        DirectoryType::Git,
    ];
    const ALL_ROLES_FOR_MATRIX: [DirectoryRole; 4] = [
        DirectoryRole::Managed,
        DirectoryRole::Synced,
        DirectoryRole::Source,
        DirectoryRole::Target,
    ];

    /// Build a Config containing exactly one directory entry with the given
    /// (type, role) pair. library_dir and the entry path are placed under
    /// different subdirs of `tmp` to avoid triggering the library-overlap
    /// check in Config::validate — we want role/type failures to surface cleanly.
    ///
    /// The helper deliberately leaves branch/tag/rev/subdir as None for ALL
    /// types (including Git) because those fields have their own validation
    /// paths; this matrix isolates role/type conflicts only.
    fn build_single_entry_config(
        tmp: &std::path::Path,
        dir_type: DirectoryType,
        role: DirectoryRole,
    ) -> Config {
        let library_dir = tmp.join("lib");
        let entry_path = tmp.join("entry");
        let mut directories = BTreeMap::new();
        directories.insert(
            DirectoryName::new("combo").unwrap(),
            DirectoryConfig {
                path: entry_path,
                directory_type: dir_type,
                role: Some(role),
                git_ref: None,
                subdir: None,
                override_applied: false,
            },
        );
        Config {
            library_dir,
            directories,
            ..Default::default()
        }
    }

    #[test]
    fn combo_matrix_all_type_role_pairs() {
        // Iterate the full 3×4 cross-product. Track every combo we touch so
        // the final assertion proves exhaustiveness.
        let mut tested = Vec::new();

        for dir_type in &ALL_TYPES_FOR_MATRIX {
            for role in &ALL_ROLES_FOR_MATRIX {
                let combo = (dir_type.clone(), role.clone());
                tested.push(combo.clone());
                // Derive pass/fail from valid_roles() at runtime — no hand-written table.
                let should_pass = dir_type.valid_roles().contains(role);

                if should_pass {
                    // Valid combo: save_checked to a fresh TempDir, reload,
                    // and confirm the entry's type + role survived the round-trip.
                    let tmp = tempfile::TempDir::new().unwrap();
                    let path = tmp.path().join("tome.toml");
                    let config =
                        build_single_entry_config(tmp.path(), dir_type.clone(), role.clone());

                    config.save_checked(&path).unwrap_or_else(|e| {
                        panic!(
                            "expected VALID combo ({:?}, {:?}) to save, but got: {e:#}",
                            dir_type, role,
                        )
                    });
                    assert!(
                        path.exists(),
                        "save_checked reported success but file missing for combo ({:?}, {:?})",
                        dir_type,
                        role,
                    );

                    let reloaded = Config::load(&path).unwrap_or_else(|e| {
                        panic!(
                            "saved VALID combo ({:?}, {:?}) failed to reload: {e:#}",
                            dir_type, role,
                        )
                    });
                    let entry = reloaded
                        .directories
                        .get("combo")
                        .expect("reloaded Config missing 'combo' entry");
                    assert_eq!(
                        &entry.directory_type, dir_type,
                        "reloaded type drifted for combo ({:?}, {:?})",
                        dir_type, role,
                    );
                    assert_eq!(
                        entry.role(),
                        role.clone(),
                        "reloaded role drifted for combo ({:?}, {:?})",
                        dir_type,
                        role,
                    );
                } else {
                    // Invalid combo: validate() must return Err.
                    // We call validate() directly (no TempDir needed) because the
                    // library-overlap check is path-based and we want to isolate
                    // the role/type rejection.
                    //
                    // Idiomatic pattern matching the sibling test below:
                    // `.err().unwrap_or_else(|| panic!(...))` — no custom extension
                    // trait. The std idiom reads cleanly and matches existing style.
                    let tmp_unused =
                        std::path::PathBuf::from(format!("/tmp/combo-{:?}-{:?}", dir_type, role));
                    let config =
                        build_single_entry_config(&tmp_unused, dir_type.clone(), role.clone());
                    let _err = config.validate().err().unwrap_or_else(|| {
                        panic!(
                            "expected INVALID combo ({:?}, {:?}) to fail validate(), but it succeeded",
                            dir_type, role,
                        )
                    });
                    // The sibling test `combo_matrix_invalid_error_mentions_role_description`
                    // asserts the error's contents; here we only care that validate()
                    // produced Err for every invalid combo.
                }
            }
        }

        // Exhaustiveness guard: we touched every cell of the 3×4 grid.
        assert_eq!(
            tested.len(),
            ALL_TYPES_FOR_MATRIX.len() * ALL_ROLES_FOR_MATRIX.len(),
            "matrix should test exactly {} combos, got {}",
            ALL_TYPES_FOR_MATRIX.len() * ALL_ROLES_FOR_MATRIX.len(),
            tested.len(),
        );
    }

    #[test]
    fn combo_matrix_invalid_error_mentions_role_description() {
        // For every INVALID (type, role), Config::validate() must produce an error
        // message containing the role's description() substring AND the literal
        // "hint:" — the Conflict+Why+Suggestion shape used by the validator.
        // This is stable against wording tweaks that don't remove the role-description
        // parenthetical or the hint line.

        let tmp_unused = std::path::PathBuf::from("/tmp/does-not-need-to-exist");

        for dir_type in &ALL_TYPES_FOR_MATRIX {
            for role in &ALL_ROLES_FOR_MATRIX {
                // Derive the invalid set from valid_roles() at runtime.
                if dir_type.valid_roles().contains(role) {
                    continue;
                }

                let config = build_single_entry_config(&tmp_unused, dir_type.clone(), role.clone());
                let err = config.validate().err().unwrap_or_else(|| {
                    panic!(
                        "INVALID combo ({:?}, {:?}) passed validate() — validator bug",
                        dir_type, role,
                    )
                });
                let msg = err.to_string();

                assert!(
                    msg.contains(role.description()),
                    "error for combo ({:?}, {:?}) missing role description {:?}: {msg}",
                    dir_type,
                    role,
                    role.description(),
                );
                assert!(
                    msg.contains("hint:"),
                    "error for combo ({:?}, {:?}) missing 'hint:' line: {msg}",
                    dir_type,
                    role,
                );
            }
        }
    }
}
