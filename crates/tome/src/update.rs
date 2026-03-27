//! Lockfile diffing and interactive triage for `tome update`.
//!
//! Compares old and new lockfiles to surface skill changes (added, changed, removed),
//! then optionally lets the user disable unwanted skills via an interactive prompt.

use std::collections::BTreeMap;
use std::io::IsTerminal;

use anyhow::Result;

use crate::discover::SkillName;
use crate::lockfile::{LockEntry, Lockfile};
use crate::machine::MachinePrefs;

/// A single skill change between two lockfile versions.
#[derive(Debug)]
#[allow(dead_code)]
pub enum SkillChange {
    Added(LockEntry),
    Changed { old: LockEntry, new: LockEntry },
    Removed(LockEntry),
}

/// The diff between two lockfile versions.
#[derive(Debug)]
pub struct UpdateDiff {
    pub changes: BTreeMap<SkillName, SkillChange>,
}

impl UpdateDiff {
    pub fn is_empty(&self) -> bool {
        self.changes.is_empty()
    }
}

/// Compare two lockfiles and produce a diff.
pub fn diff(old: &Lockfile, new: &Lockfile) -> UpdateDiff {
    let mut changes = BTreeMap::new();

    // Added or changed
    for (name, new_entry) in &new.skills {
        match old.skills.get(name) {
            None => {
                changes.insert(name.clone(), SkillChange::Added(new_entry.clone()));
            }
            Some(old_entry) if old_entry.content_hash != new_entry.content_hash => {
                changes.insert(
                    name.clone(),
                    SkillChange::Changed {
                        old: old_entry.clone(),
                        new: new_entry.clone(),
                    },
                );
            }
            _ => {} // unchanged
        }
    }

    // Removed
    for (name, old_entry) in &old.skills {
        if !new.skills.contains_key(name) {
            changes.insert(name.clone(), SkillChange::Removed(old_entry.clone()));
        }
    }

    UpdateDiff { changes }
}

/// Present changes to the user and allow disabling new skills.
///
/// Returns the list of skill names that were newly disabled during this triage.
/// In non-TTY or quiet mode, just prints notifications as warnings (no interactive prompt).
pub fn present_changes(
    diff: &UpdateDiff,
    machine_prefs: &mut MachinePrefs,
    quiet: bool,
) -> Result<Vec<SkillName>> {
    let interactive = std::io::stdin().is_terminal() && !quiet;

    let mut added_names: Vec<SkillName> = Vec::new();

    for (name, change) in &diff.changes {
        match change {
            SkillChange::Added(entry) => {
                let msg = if entry.registry_id.is_some() {
                    format!(
                        "New managed skill '{}' from source '{}' is now available.",
                        name, entry.source_name
                    )
                } else {
                    format!(
                        "New skill '{}' from source '{}' is now available.",
                        name, entry.source_name
                    )
                };
                if interactive {
                    println!("  {}", msg);
                } else if !quiet {
                    eprintln!("info: {}", msg);
                }
                added_names.push(name.clone());
            }
            SkillChange::Changed { .. } => {
                let msg = format!("Skill '{}' was updated (hash changed).", name);
                if interactive {
                    println!("  {}", msg);
                } else if !quiet {
                    eprintln!("info: {}", msg);
                }
            }
            SkillChange::Removed(_) => {
                let msg = format!("Skill '{}' was removed from the library.", name);
                if interactive {
                    println!("  {}", msg);
                } else if !quiet {
                    eprintln!("info: {}", msg);
                }
            }
        }
    }

    let mut newly_disabled = Vec::new();

    // Only offer to disable added skills interactively
    if interactive && !added_names.is_empty() {
        println!();
        let display_names: Vec<&str> = added_names.iter().map(|n| n.as_str()).collect();
        let selections = dialoguer::MultiSelect::new()
            .with_prompt("Disable any of these new skills on this machine?")
            .items(&display_names)
            .interact_opt()?;

        if let Some(indices) = selections {
            for idx in indices {
                let name = added_names[idx].clone();
                machine_prefs.disable(name.clone());
                newly_disabled.push(name);
            }
        }
    }

    Ok(newly_disabled)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lockfile::LockEntry;
    use crate::validation::test_hash;

    fn entry(source: &str, hash_seed: &str) -> LockEntry {
        LockEntry {
            source_name: source.to_string(),
            content_hash: test_hash(hash_seed),
            registry_id: None,
            version: None,
            git_commit_sha: None,
        }
    }

    fn managed_entry(source: &str, hash_seed: &str, registry_id: &str) -> LockEntry {
        LockEntry {
            source_name: source.to_string(),
            content_hash: test_hash(hash_seed),
            registry_id: Some(registry_id.to_string()),
            version: Some("1.0.0".to_string()),
            git_commit_sha: None,
        }
    }

    fn lockfile(entries: Vec<(&str, LockEntry)>) -> Lockfile {
        Lockfile {
            version: 1,
            skills: entries
                .into_iter()
                .map(|(k, v)| (SkillName::new(k).unwrap(), v))
                .collect(),
        }
    }

    #[test]
    fn diff_empty_lockfiles() {
        let old = lockfile(vec![]);
        let new = lockfile(vec![]);
        let d = diff(&old, &new);
        assert!(d.is_empty());
    }

    #[test]
    fn diff_identical_lockfiles() {
        let old = lockfile(vec![("skill-a", entry("src", "abc"))]);
        let new = lockfile(vec![("skill-a", entry("src", "abc"))]);
        let d = diff(&old, &new);
        assert!(d.is_empty());
    }

    #[test]
    fn diff_added_skill() {
        let old = lockfile(vec![]);
        let new = lockfile(vec![("new-skill", entry("src", "abc"))]);
        let d = diff(&old, &new);
        assert_eq!(d.changes.len(), 1);
        assert!(matches!(d.changes["new-skill"], SkillChange::Added(_)));
    }

    #[test]
    fn diff_removed_skill() {
        let old = lockfile(vec![("gone-skill", entry("src", "abc"))]);
        let new = lockfile(vec![]);
        let d = diff(&old, &new);
        assert_eq!(d.changes.len(), 1);
        assert!(matches!(d.changes["gone-skill"], SkillChange::Removed(_)));
    }

    #[test]
    fn diff_changed_skill() {
        let old = lockfile(vec![("skill-a", entry("src", "old-hash"))]);
        let new = lockfile(vec![("skill-a", entry("src", "new-hash"))]);
        let d = diff(&old, &new);
        assert_eq!(d.changes.len(), 1);
        assert!(matches!(d.changes["skill-a"], SkillChange::Changed { .. }));
    }

    #[test]
    fn diff_mixed_changes() {
        let old = lockfile(vec![
            ("unchanged", entry("src", "aaa")),
            ("changed", entry("src", "bbb")),
            ("removed", entry("src", "ccc")),
        ]);
        let new = lockfile(vec![
            ("unchanged", entry("src", "aaa")),
            ("changed", entry("src", "ddd")),
            ("added", entry("src", "eee")),
        ]);
        let d = diff(&old, &new);
        assert_eq!(d.changes.len(), 3);
        assert!(matches!(d.changes["added"], SkillChange::Added(_)));
        assert!(matches!(d.changes["changed"], SkillChange::Changed { .. }));
        assert!(matches!(d.changes["removed"], SkillChange::Removed(_)));
    }

    #[test]
    fn diff_detects_managed_skill() {
        let old = lockfile(vec![]);
        let new = lockfile(vec![(
            "managed",
            managed_entry("plugins", "abc", "pkg@npm"),
        )]);
        let d = diff(&old, &new);
        assert_eq!(d.changes.len(), 1);
        if let SkillChange::Added(entry) = &d.changes["managed"] {
            assert_eq!(entry.registry_id.as_deref(), Some("pkg@npm"));
        } else {
            panic!("expected Added variant");
        }
    }

    #[test]
    fn diff_returns_structured_changes() {
        // Verify the structure of UpdateDiff returned by diff():
        // each change type carries the expected LockEntry data.
        //
        // Note: present_changes() in quiet/non-TTY mode is exercised by
        // integration tests (crates/tome/tests/cli.rs) because it relies on
        // dialoguer which requires a real TTY for interactive prompts.
        let old = lockfile(vec![
            ("kept", entry("src", "aaa")),
            ("updated", entry("src", "old-hash")),
            ("deleted", managed_entry("plugins", "ccc", "pkg@npm")),
        ]);
        let new = lockfile(vec![
            ("kept", entry("src", "aaa")),
            ("updated", entry("src", "new-hash")),
            ("fresh", managed_entry("plugins", "ddd", "new-pkg@npm")),
        ]);

        let d = diff(&old, &new);
        assert_eq!(d.changes.len(), 3, "should have 3 changes (no unchanged)");
        assert!(!d.is_empty());

        // Verify Added carries the new entry
        if let SkillChange::Added(ref e) = d.changes["fresh"] {
            assert_eq!(e.source_name, "plugins");
            assert_eq!(e.content_hash, test_hash("ddd"));
            assert_eq!(e.registry_id.as_deref(), Some("new-pkg@npm"));
        } else {
            panic!("expected Added for 'fresh'");
        }

        // Verify Changed carries both old and new entries
        if let SkillChange::Changed { ref old, ref new } = d.changes["updated"] {
            assert_eq!(old.content_hash, test_hash("old-hash"));
            assert_eq!(new.content_hash, test_hash("new-hash"));
        } else {
            panic!("expected Changed for 'updated'");
        }

        // Verify Removed carries the old entry
        if let SkillChange::Removed(ref e) = d.changes["deleted"] {
            assert_eq!(e.content_hash, test_hash("ccc"));
            assert_eq!(e.registry_id.as_deref(), Some("pkg@npm"));
        } else {
            panic!("expected Removed for 'deleted'");
        }
    }

    #[test]
    fn diff_same_hash_different_source_is_unchanged() {
        // Source name change alone doesn't trigger a diff (hash is what matters)
        let old = lockfile(vec![("skill-a", entry("old-source", "same-hash"))]);
        let new = lockfile(vec![("skill-a", entry("new-source", "same-hash"))]);
        let d = diff(&old, &new);
        assert!(d.is_empty());
    }
}
