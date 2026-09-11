//! Optional project-local destination configuration.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

use crate::config::{DirectoryConfig, DirectoryName, DirectoryRole};
use crate::routing::{RouteMutation, RoutingPolicy};

/// Project-local destinations and their routing rules.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ProjectConfig {
    #[serde(default)]
    pub(crate) directories: BTreeMap<DirectoryName, DirectoryConfig>,
    #[serde(default)]
    pub(crate) routes: RoutingPolicy,
}

/// Finds and loads the nearest `.tome.toml` from `cwd` upward.
pub(crate) fn find_project_config(cwd: &Path) -> Result<Option<(PathBuf, ProjectConfig)>> {
    for directory in cwd.ancestors() {
        let path = directory.join(".tome.toml");
        if path.is_file() {
            let text = std::fs::read_to_string(&path)
                .with_context(|| format!("failed to read {}", path.display()))?;
            let config = toml::from_str(&text)
                .with_context(|| format!("failed to parse {}", path.display()))?;
            return Ok(Some((path, config)));
        }
    }
    Ok(None)
}

/// Validates and atomically saves project configuration.
fn save_project_checked(config: &ProjectConfig, path: &Path) -> Result<()> {
    validate(config)?;
    let text = toml::to_string_pretty(config).context("failed to serialize project config")?;
    let _: ProjectConfig =
        toml::from_str(&text).context("round-trip: generated project config did not reparse")?;
    let tmp = path.with_extension("toml.tmp");
    std::fs::write(&tmp, text).with_context(|| format!("failed to write {}", tmp.display()))?;
    if let Err(error) = std::fs::rename(&tmp, path) {
        let _ = std::fs::remove_file(&tmp);
        return Err(error).with_context(|| format!("failed to rename {}", path.display()));
    }
    Ok(())
}

/// Apply a routing mutation when the nearest project configuration owns the destination.
pub(crate) fn mutate_project_route(
    cwd: &Path,
    destination: DirectoryName,
    mutation: RouteMutation,
) -> Result<bool> {
    let Some((path, mut config)) = find_project_config(cwd)? else {
        return Ok(false);
    };
    let Some(directory) = config.directories.get(&destination) else {
        return Ok(false);
    };
    anyhow::ensure!(
        directory.role() == DirectoryRole::Target,
        "route destination '{destination}' must be a target directory"
    );
    anyhow::ensure!(
        config.routes.apply(destination.clone(), mutation),
        "route selector is not configured for destination '{destination}'"
    );
    save_project_checked(&config, &path)?;
    Ok(true)
}

/// Validate whether the nearest project configuration owns a route destination.
pub(crate) fn validates_project_route_mutation(
    cwd: &Path,
    destination: &DirectoryName,
    mutation: RouteMutation,
) -> Result<bool> {
    let Some((_, mut config)) = find_project_config(cwd)? else {
        return Ok(false);
    };
    let Some(directory) = config.directories.get(destination) else {
        return Ok(false);
    };
    anyhow::ensure!(
        directory.role() == DirectoryRole::Target,
        "route destination '{destination}' must be a target directory"
    );
    anyhow::ensure!(
        config.routes.apply(destination.clone(), mutation),
        "route selector is not configured for destination '{destination}'"
    );
    validate(&config)?;
    Ok(true)
}

pub(crate) fn validate(config: &ProjectConfig) -> Result<()> {
    for (name, directory) in &config.directories {
        anyhow::ensure!(
            directory.role() == DirectoryRole::Target,
            "project directory '{name}' must use role = \"target\""
        );
    }
    for destination in config.routes.routes.keys() {
        anyhow::ensure!(
            config.directories.contains_key(destination),
            "project route destination '{destination}' is not a project directory"
        );
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use crate::config::DirectoryName;
    use crate::manifest::SkillTag;
    use crate::routing::RouteMutation;

    use super::{find_project_config, validates_project_route_mutation};

    #[test]
    fn finds_the_nearest_project_config() {
        let tmp = tempfile::TempDir::new().unwrap();
        let nested = tmp.path().join("a/b");
        std::fs::create_dir_all(&nested).unwrap();
        std::fs::write(tmp.path().join(".tome.toml"), "").unwrap();

        assert!(find_project_config(&nested).unwrap().is_some());
    }

    #[test]
    fn dry_run_route_validation_rejects_preexisting_dangling_route() {
        let tmp = tempfile::TempDir::new().unwrap();
        std::fs::write(
            tmp.path().join(".tome.toml"),
            format!(
                "[directories.target]\npath = \"{}\"\ntype = \"directory\"\nrole = \"target\"\n\n[routes.dangling]\ntags = [\"reference\"]\n",
                tmp.path().join("target").display()
            ),
        )
        .unwrap();

        let error = validates_project_route_mutation(
            tmp.path(),
            &DirectoryName::new("target").unwrap(),
            RouteMutation::AddTag(SkillTag::new("reference").unwrap()),
        )
        .unwrap_err();

        assert!(error.to_string().contains("dangling"));
    }
}
