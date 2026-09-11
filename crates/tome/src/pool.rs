//! Shared-pool candidate reconciliation and durable removal state.
//!
//! This module deliberately plans reconciliation before callers mutate the
//! library, manifest, catalog, exclusions, or distribution targets.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};

use crate::config::{DirectoryName, DirectoryType};
use crate::discover::{DiscoveredSkill, SkillName};
use crate::lockfile::{LockEntry, Observation};
use crate::validation::ContentHash;

/// A discovered source proposal, including its stable identity and content.
#[derive(Debug, Clone)]
pub(crate) struct Candidate {
    pub skill: DiscoveredSkill,
    pub identity: String,
    pub locator: String,
    pub hash: ContentHash,
}

/// A deterministic, read-only conflict report.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Conflict {
    pub skill: SkillName,
    pub candidates: Vec<ConflictCandidate>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ConflictCandidate {
    pub identity: String,
    pub hash: ContentHash,
    pub location: PathBuf,
}

/// Produce candidates without applying discovery's legacy first-wins policy.
pub(crate) fn collect(
    skills: Vec<DiscoveredSkill>,
    directory_types: &BTreeMap<DirectoryName, DirectoryType>,
    profile: &str,
) -> Result<Vec<Candidate>> {
    let mut candidates = Vec::with_capacity(skills.len());
    for skill in skills {
        let kind = directory_types
            .get(&skill.source_name)
            .cloned()
            .unwrap_or(DirectoryType::Directory);
        let locator = skill.path.display().to_string();
        let identity = stable_identity(&skill, kind, profile);
        let hash = crate::manifest::hash_directory(&skill.path).with_context(|| {
            format!(
                "failed to hash candidate '{}' at {}",
                skill.name,
                skill.path.display()
            )
        })?;
        candidates.push(Candidate {
            skill,
            identity,
            locator,
            hash,
        });
    }
    candidates.sort_by(|a, b| {
        a.skill
            .name
            .cmp(&b.skill.name)
            .then_with(|| a.identity.cmp(&b.identity))
            .then_with(|| a.hash.cmp(&b.hash))
            .then_with(|| a.locator.cmp(&b.locator))
    });
    Ok(candidates)
}

fn stable_identity(skill: &DiscoveredSkill, kind: DirectoryType, profile: &str) -> String {
    if let Some(provenance) = skill.origin.provenance()
        && !provenance.registry_id.is_empty()
    {
        return format!("registry:{}", provenance.registry_id);
    }
    if kind == DirectoryType::Git {
        return format!("git:{}", normalize_git_url(&skill.path.to_string_lossy()));
    }
    format!("local:{profile}/{}", skill.source_name)
}

fn normalize_git_url(url: &str) -> String {
    url.trim()
        .trim_end_matches('/')
        .trim_end_matches(".git")
        .to_ascii_lowercase()
}

/// Reconcile candidates with existing authoritative entries. The returned
/// selected skills are safe to consolidate only when `conflicts` is empty.
pub(crate) fn reconcile(
    existing: &BTreeMap<SkillName, LockEntry>,
    candidates: &[Candidate],
    pins: &BTreeMap<SkillName, String>,
    excluded: &BTreeSet<SkillName>,
    profile: &str,
) -> (
    Vec<DiscoveredSkill>,
    BTreeMap<SkillName, LockEntry>,
    Vec<Conflict>,
) {
    let mut entries = existing.clone();
    let mut selected = Vec::new();
    let mut conflicts = Vec::new();
    let mut by_name: BTreeMap<SkillName, Vec<&Candidate>> = BTreeMap::new();
    for candidate in candidates {
        if !excluded.contains(&candidate.skill.name) {
            by_name
                .entry(candidate.skill.name.clone())
                .or_default()
                .push(candidate);
        }
    }

    for (name, group) in by_name {
        let pin = pins.get(&name);
        let usable: Vec<&Candidate> = match pin {
            Some(identity) => group
                .into_iter()
                .filter(|c| &c.identity == identity)
                .collect(),
            None => group,
        };
        if usable.is_empty() {
            continue; // an unavailable pin preserves catalog and pool content
        }
        let hashes: BTreeSet<ContentHash> = usable.iter().map(|c| c.hash.clone()).collect();
        let existing_hash = entries.get(&name).map(|entry| entry.content_hash.clone());
        if hashes.len() > 1 {
            conflicts.push(Conflict {
                skill: name,
                candidates: usable
                    .iter()
                    .map(|c| ConflictCandidate {
                        identity: c.identity.clone(),
                        hash: c.hash.clone(),
                        location: c.skill.path.clone(),
                    })
                    .collect(),
            });
            continue;
        }
        let candidate = usable[0];
        if pin.is_none()
            && existing_hash.is_some_and(|hash| hash != candidate.hash)
            && !entries
                .get(&name)
                .is_some_and(|entry| entry.has_identity(&candidate.identity))
        {
            conflicts.push(Conflict {
                skill: name,
                candidates: usable
                    .iter()
                    .map(|c| ConflictCandidate {
                        identity: c.identity.clone(),
                        hash: c.hash.clone(),
                        location: c.skill.path.clone(),
                    })
                    .collect(),
            });
            continue;
        }
        let observation = Observation::from_candidate(candidate, profile);
        entries
            .entry(name.clone())
            .and_modify(|entry| entry.observe(observation.clone()))
            .or_insert_with(|| LockEntry::from_observation(observation));
        selected.push(candidate.skill.clone());
    }
    conflicts.sort_by(|a, b| a.skill.cmp(&b.skill));
    (selected, entries, conflicts)
}

/// Remove a skill from all derived artifacts after its exclusion has been
/// durably persisted. The marker makes interruptions observable and retryable.
pub(crate) fn removal_marker(config_dir: &Path, skill: &SkillName) -> PathBuf {
    config_dir.join(format!(".tome-pool-remove-{}.json", skill.as_str()))
}

/// Resume interrupted exclusion-first removals before discovering candidates.
/// Markers are cleared only after library, manifest, and catalog agree.
pub(crate) fn recover_pending_removals(paths: &crate::paths::TomePaths) -> Result<()> {
    let prefix = ".tome-pool-remove-";
    let entries = match std::fs::read_dir(paths.config_dir()) {
        Ok(entries) => entries,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(error) => {
            return Err(error)
                .with_context(|| format!("failed to read {}", paths.config_dir().display()));
        }
    };
    for entry in entries {
        let marker = entry?.path();
        let Some(file_name) = marker.file_name().and_then(|name| name.to_str()) else {
            continue;
        };
        let Some(name) = file_name
            .strip_prefix(prefix)
            .and_then(|name| name.strip_suffix(".json"))
        else {
            continue;
        };
        let skill = SkillName::new(name.to_owned())?;
        let library_path = paths.library_dir().join(skill.as_str());
        if library_path.is_dir() {
            std::fs::remove_dir_all(&library_path).with_context(|| {
                format!("failed to resume removal of {}", library_path.display())
            })?;
        } else if library_path.is_symlink() {
            std::fs::remove_file(&library_path).with_context(|| {
                format!("failed to resume removal of {}", library_path.display())
            })?;
        }
        let mut manifest = crate::manifest::load(paths.config_dir())?;
        manifest.remove(skill.as_str());
        crate::manifest::save(&manifest, paths.config_dir())?;
        if let Some(mut catalog) = crate::lockfile::load(paths.config_dir())? {
            catalog.skills.remove(&skill);
            crate::lockfile::save(&catalog, paths.config_dir())?;
        }
        std::fs::remove_file(&marker)
            .with_context(|| format!("failed to clear recovered marker {}", marker.display()))?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalized_git_urls_merge() {
        assert_eq!(
            normalize_git_url("HTTPS://EXAMPLE.COM/team/repo.git/"),
            "https://example.com/team/repo"
        );
    }
}
