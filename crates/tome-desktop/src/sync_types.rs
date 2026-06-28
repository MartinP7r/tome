//! IPC types for the SYNC-02 lockfile-diff triage panel (Phase 27 plan 27-02).
//!
//! Projects [`tome::update::UpdateDiff`] across the Tauri IPC boundary as
//! [`LockfileDiff`] — three pre-sorted vectors of [`TriageEntry`]s (added,
//! changed, removed). Each [`TriageEntry`] carries enough metadata for the
//! React triage panel to render per-skill diff details without round-tripping
//! to the Rust side again (CONTEXT §"No JS-side business logic" — the React
//! side renders a structured payload; it does not compute the diff itself).
//!
//! The projection is **read-only**: it never writes to disk, never mutates
//! manifest / lockfile / machine.toml state. Triage decisions live in React
//! state until the user clicks `[Apply N decisions]`, which fires the
//! SYNC-03 preview-then-confirm flow (plan 27-03).

use tome::SkillName;
use tome::SkillOrigin;
use tome::SkillProvenance;
use tome::config::DirectoryName;
use tome::manifest::Manifest;
use tome::update::{SkillChange, UpdateDiff};

/// A single skill's per-row payload in the triage panel.
///
/// One entry per row in the GUI. Shape covers all three change kinds:
///
/// - **Added** (`change_kind: "added"`): `content_hash_new = Some`,
///   `content_hash_old = None`, `synced_at = None` (the manifest does not
///   yet have an entry for this skill).
/// - **Changed** (`change_kind: "changed"`): both hashes populated; `synced_at`
///   from the manifest entry for the current (old-state) skill.
/// - **Removed** (`change_kind: "removed"`): `content_hash_old = Some`,
///   `content_hash_new = None`. `previous_source` carries the owning
///   directory at the moment the skill was last seen.
///
/// All fields are serializable via `specta::Type` so the React side gets a
/// fully-typed payload; no string parsing on the JS side.
#[derive(Debug, Clone, serde::Serialize, specta::Type)]
pub struct TriageEntry {
    /// Skill name (matches the manifest key + the lockfile key).
    pub name: SkillName,
    /// Which change kind this row belongs to. The Rust side already sorts
    /// entries into the three Vecs of [`LockfileDiff`], but echoing the kind
    /// on every entry keeps the boundary self-describing for diagnostics and
    /// for the React-side aria-label templates (UI-SPEC §VoiceOver labels).
    pub change_kind: TriageEntryChangeKind,
    /// The directory currently owning this skill (Added → new owner;
    /// Changed → the new owner; Removed → the owner at the moment of removal
    /// per the old lockfile entry). `None` for the Unowned state (an Added
    /// skill never appears Unowned, but a Removed-while-Unowned skill can).
    pub source_name: Option<DirectoryName>,
    /// For Removed entries that transitioned through Unowned: the last
    /// directory that owned the skill (D-C1 breadcrumb).
    pub previous_source: Option<DirectoryName>,
    /// Origin classification (managed vs. local). For Added/Changed: derived
    /// from the new entry's `registry_id` (managed) vs. absence (local). For
    /// Removed: derived from the old entry's `registry_id`. The
    /// `provenance` payload (when `kind = managed`) carries `version` and
    /// `git_commit_sha` from the lockfile — the React-side `TriageDetail`
    /// reads `git_commit_sha` to decide whether to show the "View source"
    /// radio (D-14: git-sourced only).
    pub origin: SkillOrigin,
    /// Old SHA-256 hex (present for Changed + Removed; absent for Added).
    pub content_hash_old: Option<String>,
    /// New SHA-256 hex (present for Added + Changed; absent for Removed).
    pub content_hash_new: Option<String>,
    /// Registry identifier (e.g. "axiom@npm"). Mirrors the lockfile's
    /// `registry_id` field; `None` for local skills. For Changed entries
    /// where the registry identifier itself changed (rare, but possible
    /// across a marketplace rename), the value is the **new** registry id;
    /// the React-side disclosure shows the old via the diff metadata.
    pub registry_id: Option<String>,
    /// Old version string (for Changed + Removed).
    pub version_old: Option<String>,
    /// New version string (for Added + Changed).
    pub version_new: Option<String>,
    /// Old git commit SHA (for Changed + Removed).
    pub git_commit_sha_old: Option<String>,
    /// New git commit SHA (for Added + Changed).
    pub git_commit_sha_new: Option<String>,
    /// ISO-8601 timestamp from the manifest's `SkillEntry::synced_at` for
    /// this skill at the moment the diff was computed. `None` for an Added
    /// skill (no manifest entry yet) or for a skill the manifest stamped
    /// with no value (legacy).
    pub synced_at: Option<String>,
}

/// Discriminator carried on each `TriageEntry`. Stable string union on the
/// TS side so React's pattern-match stays exhaustive.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, specta::Type)]
#[serde(rename_all = "lowercase")]
pub enum TriageEntryChangeKind {
    Added,
    Changed,
    Removed,
}

/// Three pre-sorted buckets, alphabetical by skill name within each.
///
/// The React side renders three vertical sections (NEW / CHANGED / REMOVED —
/// UI-SPEC §TriagePanel) by mapping each Vec; no re-sorting needed on the
/// JS side. The buckets are independent — a skill never appears in more than
/// one Vec.
#[derive(Debug, Clone, serde::Serialize, specta::Type)]
pub struct LockfileDiff {
    pub added: Vec<TriageEntry>,
    pub changed: Vec<TriageEntry>,
    pub removed: Vec<TriageEntry>,
}

impl LockfileDiff {
    /// True iff every bucket is empty. The React `useLockfileDiff` hook uses
    /// this to decide whether to mount the `TriagePanel` at all (UI-SPEC
    /// §"In-progress state" — panel hidden until non-empty).
    pub fn is_empty(&self) -> bool {
        self.added.is_empty() && self.changed.is_empty() && self.removed.is_empty()
    }
}

/// Pure projection from a domain [`UpdateDiff`] over [`Lockfile`] and
/// [`Manifest`] inputs into the boundary [`LockfileDiff`] payload.
///
/// Read-only: takes references, computes a new value, never mutates the
/// inputs. Extracted as a pub fn (rather than inlined into the Tauri
/// command) so it is directly unit-testable without an `AppHandle` (mirrors
/// the [`crate::sink::event_to_sync_progress`] pattern from 27-01a — pure
/// conversion, no I/O).
///
/// The `BTreeMap` iteration in [`UpdateDiff::changes`] yields entries in
/// alphabetical order by skill name, so each bucket's Vec inherits that
/// order without an explicit sort.
pub fn lockfile_diff_projection(diff: &UpdateDiff, manifest: &Manifest) -> LockfileDiff {
    let mut added = Vec::new();
    let mut changed = Vec::new();
    let mut removed = Vec::new();

    for (name, change) in &diff.changes {
        match change {
            SkillChange::Added(new) => {
                added.push(TriageEntry {
                    name: name.clone(),
                    change_kind: TriageEntryChangeKind::Added,
                    source_name: new.source_name.clone(),
                    previous_source: new.previous_source.clone(),
                    origin: classify_origin(
                        new.registry_id.as_deref(),
                        new.version.clone(),
                        new.git_commit_sha.clone(),
                    ),
                    content_hash_old: None,
                    content_hash_new: Some(new.content_hash.as_str().to_string()),
                    registry_id: new.registry_id.clone(),
                    version_old: None,
                    version_new: new.version.clone(),
                    git_commit_sha_old: None,
                    git_commit_sha_new: new.git_commit_sha.clone(),
                    // Added → no manifest entry yet.
                    synced_at: None,
                });
            }
            SkillChange::Changed { old, new } => {
                changed.push(TriageEntry {
                    name: name.clone(),
                    change_kind: TriageEntryChangeKind::Changed,
                    source_name: new.source_name.clone(),
                    previous_source: new.previous_source.clone(),
                    origin: classify_origin(
                        new.registry_id.as_deref(),
                        new.version.clone(),
                        new.git_commit_sha.clone(),
                    ),
                    content_hash_old: Some(old.content_hash.as_str().to_string()),
                    content_hash_new: Some(new.content_hash.as_str().to_string()),
                    registry_id: new.registry_id.clone(),
                    version_old: old.version.clone(),
                    version_new: new.version.clone(),
                    git_commit_sha_old: old.git_commit_sha.clone(),
                    git_commit_sha_new: new.git_commit_sha.clone(),
                    synced_at: manifest.get(name.as_str()).map(|e| e.synced_at.clone()),
                });
            }
            SkillChange::Removed(old) => {
                removed.push(TriageEntry {
                    name: name.clone(),
                    change_kind: TriageEntryChangeKind::Removed,
                    source_name: old.source_name.clone(),
                    previous_source: old.previous_source.clone(),
                    origin: classify_origin(
                        old.registry_id.as_deref(),
                        old.version.clone(),
                        old.git_commit_sha.clone(),
                    ),
                    content_hash_old: Some(old.content_hash.as_str().to_string()),
                    content_hash_new: None,
                    registry_id: old.registry_id.clone(),
                    version_old: old.version.clone(),
                    version_new: None,
                    git_commit_sha_old: old.git_commit_sha.clone(),
                    git_commit_sha_new: None,
                    synced_at: manifest.get(name.as_str()).map(|e| e.synced_at.clone()),
                });
            }
        }
    }

    LockfileDiff {
        added,
        changed,
        removed,
    }
}

/// Classify the lockfile entry's origin for the boundary payload.
///
/// The lockfile's `registry_id`, `version`, and `git_commit_sha` mirror
/// [`tome::discover::SkillOrigin`]'s shape — managed skills carry a registry
/// identifier; local skills do not. Reconstructing the `SkillOrigin` enum at
/// the boundary lets the React side reuse the same discriminator the Skills
/// view already pattern-matches (DRY — `bindings.ts` exports one
/// `SkillOrigin` type used everywhere).
fn classify_origin(
    registry_id: Option<&str>,
    version: Option<String>,
    git_commit_sha: Option<String>,
) -> SkillOrigin {
    match registry_id {
        Some(rid) => SkillOrigin::Managed {
            provenance: Some(SkillProvenance {
                registry_id: rid.to_string(),
                version,
                git_commit_sha,
            }),
        },
        None => SkillOrigin::Local,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;
    use std::path::PathBuf;
    use tome::ContentHash;
    use tome::SkillName;
    use tome::config::DirectoryName;
    use tome::lockfile::{LockEntry, Lockfile};
    use tome::manifest::{Manifest, SkillEntry};
    use tome::update::diff;

    /// Construct a real (validated) ContentHash from a hex digest seed —
    /// 64 hex chars are required, so we repeat a 2-char value.
    fn hash_for(seed: &str) -> ContentHash {
        // SHA-256 hex digests are exactly 64 hex chars. Repeating the
        // 2-char seed 32 times yields a syntactically-valid digest.
        let s = seed.repeat(32);
        ContentHash::new(&s[..64]).expect("test hash must validate")
    }

    fn lock_entry_local(source: &str, hash_seed: &str) -> LockEntry {
        LockEntry {
            source_name: Some(DirectoryName::new(source).unwrap()),
            previous_source: None,
            content_hash: hash_for(hash_seed),
            registry_id: None,
            version: None,
            git_commit_sha: None,
        }
    }

    fn lock_entry_managed(
        source: &str,
        hash_seed: &str,
        registry: &str,
        version: &str,
        sha: &str,
    ) -> LockEntry {
        LockEntry {
            source_name: Some(DirectoryName::new(source).unwrap()),
            previous_source: None,
            content_hash: hash_for(hash_seed),
            registry_id: Some(registry.to_string()),
            version: Some(version.to_string()),
            git_commit_sha: Some(sha.to_string()),
        }
    }

    fn lockfile_with(entries: Vec<(&str, LockEntry)>) -> Lockfile {
        // Lockfile's fields are pub(crate), so we round-trip through JSON to
        // construct one for tests — Lockfile derives Deserialize publicly.
        let map: BTreeMap<String, LockEntry> = entries
            .into_iter()
            .map(|(k, v)| (k.to_string(), v))
            .collect();
        let json = serde_json::json!({
            "version": 1,
            "skills": map,
        });
        serde_json::from_value::<Lockfile>(json).expect("lockfile must deserialize")
    }

    fn manifest_with(entries: Vec<(&str, &str, &str)>) -> Manifest {
        let mut m = Manifest::default();
        for (name, source, hash_seed) in entries {
            m.insert(
                SkillName::new(name).unwrap(),
                SkillEntry::new(
                    PathBuf::from(format!("/tmp/{name}")),
                    DirectoryName::new(source).unwrap(),
                    hash_for(hash_seed),
                    false,
                ),
            );
        }
        m
    }

    /// Empty diff yields three empty buckets (and `is_empty()` is true).
    #[test]
    fn empty_diff_projects_to_empty_buckets() {
        let old = lockfile_with(vec![]);
        let new = lockfile_with(vec![]);
        let d = diff(&old, &new);
        let manifest = Manifest::default();
        let proj = lockfile_diff_projection(&d, &manifest);
        assert!(proj.added.is_empty());
        assert!(proj.changed.is_empty());
        assert!(proj.removed.is_empty());
        assert!(proj.is_empty());
    }

    /// Added skill: change_kind = added, content_hash_new is set,
    /// content_hash_old is None, synced_at is None (no manifest entry yet).
    #[test]
    fn added_skill_projects_with_no_old_hash_and_no_synced_at() {
        let old = lockfile_with(vec![]);
        let new = lockfile_with(vec![("new-skill", lock_entry_local("plugins", "ab"))]);
        let d = diff(&old, &new);
        let manifest = Manifest::default();
        let proj = lockfile_diff_projection(&d, &manifest);
        assert_eq!(proj.added.len(), 1);
        assert!(proj.changed.is_empty());
        assert!(proj.removed.is_empty());
        let entry = &proj.added[0];
        assert_eq!(entry.name.as_str(), "new-skill");
        assert_eq!(entry.change_kind, TriageEntryChangeKind::Added);
        assert_eq!(
            entry.source_name.as_ref().map(|d| d.as_str()),
            Some("plugins")
        );
        assert!(entry.content_hash_old.is_none());
        assert!(entry.content_hash_new.is_some());
        assert!(entry.synced_at.is_none());
        assert!(matches!(entry.origin, SkillOrigin::Local));
    }

    /// Changed skill: both hashes populated; synced_at flows in from the manifest.
    #[test]
    fn changed_skill_projects_with_both_hashes_and_manifest_synced_at() {
        let old = lockfile_with(vec![("updated", lock_entry_local("plugins", "aa"))]);
        let new = lockfile_with(vec![("updated", lock_entry_local("plugins", "bb"))]);
        let d = diff(&old, &new);
        // Manifest carries an existing entry for `updated`. The synced_at
        // value the projection surfaces is whatever the entry has at the
        // moment of the diff (the timestamp at consolidate time).
        let manifest = manifest_with(vec![("updated", "plugins", "aa")]);
        let proj = lockfile_diff_projection(&d, &manifest);
        assert_eq!(proj.changed.len(), 1);
        let entry = &proj.changed[0];
        assert_eq!(entry.change_kind, TriageEntryChangeKind::Changed);
        assert_eq!(entry.name.as_str(), "updated");
        assert!(entry.content_hash_old.is_some());
        assert!(entry.content_hash_new.is_some());
        assert_ne!(entry.content_hash_old, entry.content_hash_new);
        assert!(
            entry.synced_at.is_some(),
            "Changed entry must surface manifest synced_at when the entry exists",
        );
    }

    /// Removed skill: content_hash_old populated, content_hash_new None;
    /// previous_source forwarded from the old lockfile entry.
    #[test]
    fn removed_skill_projects_with_no_new_hash() {
        let removed_entry = LockEntry {
            source_name: None, // Unowned at removal time
            previous_source: Some(DirectoryName::new("plugins").unwrap()),
            content_hash: hash_for("aa"),
            registry_id: None,
            version: None,
            git_commit_sha: None,
        };
        let old = lockfile_with(vec![("gone", removed_entry)]);
        let new = lockfile_with(vec![]);
        let d = diff(&old, &new);
        let manifest = Manifest::default();
        let proj = lockfile_diff_projection(&d, &manifest);
        assert_eq!(proj.removed.len(), 1);
        let entry = &proj.removed[0];
        assert_eq!(entry.change_kind, TriageEntryChangeKind::Removed);
        assert!(entry.content_hash_old.is_some());
        assert!(entry.content_hash_new.is_none());
        assert_eq!(
            entry.previous_source.as_ref().map(|d| d.as_str()),
            Some("plugins"),
        );
        assert!(
            entry.source_name.is_none(),
            "Unowned removed entry has no current source"
        );
    }

    /// Managed skills surface the registry_id / version / git_commit_sha on
    /// the new (Added/Changed) or old (Removed) side. Pins that the React
    /// side can detect "git-sourced" (origin.git_commit_sha.is_some()) to
    /// decide whether to render the "View source" radio (D-14).
    #[test]
    fn managed_added_skill_carries_provenance() {
        let old = lockfile_with(vec![]);
        let new = lockfile_with(vec![(
            "managed-skill",
            lock_entry_managed("plugins", "ab", "axiom@npm", "1.2.3", "abc1234"),
        )]);
        let d = diff(&old, &new);
        let manifest = Manifest::default();
        let proj = lockfile_diff_projection(&d, &manifest);
        assert_eq!(proj.added.len(), 1);
        let entry = &proj.added[0];
        assert_eq!(entry.registry_id.as_deref(), Some("axiom@npm"));
        assert_eq!(entry.version_new.as_deref(), Some("1.2.3"));
        assert_eq!(entry.git_commit_sha_new.as_deref(), Some("abc1234"));
        // Origin carries the provenance — the React side reads it directly.
        match &entry.origin {
            SkillOrigin::Managed {
                provenance: Some(p),
            } => {
                assert_eq!(p.registry_id, "axiom@npm");
                assert_eq!(p.version.as_deref(), Some("1.2.3"));
                assert_eq!(p.git_commit_sha.as_deref(), Some("abc1234"));
            }
            SkillOrigin::Managed { provenance: None } => {
                panic!("expected provenance Some for entry with version/sha");
            }
            SkillOrigin::Local => panic!("expected Managed origin for entry with registry_id"),
        }
    }

    /// Mixed diff with Added + Changed + Removed in alphabetical order
    /// across all three buckets (BTreeMap iteration order is preserved).
    #[test]
    fn mixed_diff_buckets_remain_alphabetical() {
        let old = lockfile_with(vec![
            ("apple", lock_entry_local("plugins", "aa")),  // changed
            ("banana", lock_entry_local("plugins", "bb")), // removed
            ("cherry", lock_entry_local("plugins", "cc")), // unchanged
        ]);
        let new = lockfile_with(vec![
            ("apple", lock_entry_local("plugins", "ff")), // changed (different hash)
            ("cherry", lock_entry_local("plugins", "cc")), // unchanged (same hash)
            ("durian", lock_entry_local("plugins", "dd")), // added
            ("elderberry", lock_entry_local("plugins", "ee")), // added
        ]);
        let d = diff(&old, &new);
        let manifest = Manifest::default();
        let proj = lockfile_diff_projection(&d, &manifest);

        // Two added, one changed, one removed.
        assert_eq!(proj.added.len(), 2);
        assert_eq!(proj.changed.len(), 1);
        assert_eq!(proj.removed.len(), 1);

        // Alphabetical order within each bucket.
        assert_eq!(proj.added[0].name.as_str(), "durian");
        assert_eq!(proj.added[1].name.as_str(), "elderberry");
        assert_eq!(proj.changed[0].name.as_str(), "apple");
        assert_eq!(proj.removed[0].name.as_str(), "banana");
    }
}
