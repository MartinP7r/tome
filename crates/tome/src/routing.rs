//! Tag-based distribution routing.

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

use crate::config::DirectoryName;
use crate::discover::SkillName;
use crate::manifest::SkillTag;

/// The tag selector and per-skill exclusions for one destination.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Route {
    #[serde(default)]
    tags: BTreeSet<SkillTag>,
    #[serde(default)]
    exclude: BTreeSet<SkillName>,
}

/// Routes that limit which tagged skills are linked to each destination.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(transparent)]
pub struct RoutingPolicy {
    pub routes: BTreeMap<DirectoryName, Route>,
}

/// A single route mutation requested by the CLI.
#[derive(Debug, Clone)]
pub(crate) enum RouteMutation {
    AddTag(SkillTag),
    RemoveTag(SkillTag),
    AddExclude(SkillName),
    RemoveExclude(SkillName),
}

impl RoutingPolicy {
    /// Applies a route mutation, returning whether it changed the policy.
    pub(crate) fn apply(&mut self, destination: DirectoryName, mutation: RouteMutation) -> bool {
        match mutation {
            RouteMutation::AddTag(tag) => {
                self.routes.entry(destination).or_default().tags.insert(tag)
            }
            RouteMutation::RemoveTag(tag) => self
                .routes
                .get_mut(&destination)
                .is_some_and(|route| route.tags.remove(&tag)),
            RouteMutation::AddExclude(skill) => self
                .routes
                .entry(destination)
                .or_default()
                .exclude
                .insert(skill),
            RouteMutation::RemoveExclude(skill) => self
                .routes
                .get_mut(&destination)
                .is_some_and(|route| route.exclude.remove(&skill)),
        }
    }

    /// Returns whether a skill may be distributed to a destination.
    ///
    /// Destinations without a route retain the existing unrestricted behavior.
    pub fn allows(
        &self,
        destination: &DirectoryName,
        skill: &SkillName,
        tags: &BTreeSet<SkillTag>,
    ) -> bool {
        let Some(route) = self.routes.get(destination) else {
            return true;
        };

        !route.exclude.contains(skill) && !tags.is_disjoint(&route.tags)
    }
}

#[cfg(test)]
mod tests {
    use std::collections::{BTreeMap, BTreeSet};

    use crate::config::DirectoryName;
    use crate::discover::SkillName;
    use crate::manifest::SkillTag;

    use super::{Route, RoutingPolicy};

    fn route(tags: &[&str], exclude: &[&str]) -> Route {
        Route {
            tags: tags
                .iter()
                .map(|tag| SkillTag::new(*tag).unwrap())
                .collect(),
            exclude: exclude
                .iter()
                .map(|skill| SkillName::new(*skill).unwrap())
                .collect(),
        }
    }

    fn policy(route: Route) -> RoutingPolicy {
        RoutingPolicy {
            routes: BTreeMap::from([(DirectoryName::new("codex").unwrap(), route)]),
        }
    }

    #[test]
    fn allows_a_skill_with_any_selected_tag() {
        let policy = policy(route(&["reference", "rust"], &[]));
        let skill = SkillName::new("docs").unwrap();
        let tags = BTreeSet::from([SkillTag::new("rust").unwrap()]);

        assert!(policy.allows(&DirectoryName::new("codex").unwrap(), &skill, &tags));
    }

    #[test]
    fn rejects_an_excluded_skill_even_when_its_tag_matches() {
        let policy = policy(route(&["reference"], &["docs"]));
        let skill = SkillName::new("docs").unwrap();
        let tags = BTreeSet::from([SkillTag::new("reference").unwrap()]);

        assert!(!policy.allows(&DirectoryName::new("codex").unwrap(), &skill, &tags));
    }

    #[test]
    fn rejects_an_untagged_skill() {
        let policy = policy(route(&["reference"], &[]));
        let skill = SkillName::new("docs").unwrap();

        assert!(!policy.allows(
            &DirectoryName::new("codex").unwrap(),
            &skill,
            &BTreeSet::new(),
        ));
    }

    #[test]
    fn invalid_tags_are_rejected() {
        assert!(SkillTag::new("invalid/tag").is_err());
    }
}
