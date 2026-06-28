//! `tome list` domain computation — discover every skill and return it as a
//! structured [`ListReport`].
//!
//! This is the CORE-01 / D-GUI-08 extraction for the `list` command: the
//! presentation (text table / JSON) stays inline in `lib.rs::cmd_list`, while
//! the *computation* (discover + sort + collect warnings) moves here behind a
//! `pub fn` returning a structured type. The GUI (`tome-desktop`, later phases)
//! calls [`collect`] directly and renders the [`ListReport`] without going
//! through any CLI formatting — mirroring the `status::gather` / `status::show`
//! split that is the CORE-01 template.

use std::collections::BTreeMap;

use anyhow::Result;

use crate::config::Config;
use crate::discover::{self, DiscoveredSkill};

/// The structured result of `tome list`: every discovered skill (sorted by
/// name) plus any non-fatal discovery warnings.
///
/// Field shapes are deliberately the same `DiscoveredSkill` / `String` types
/// the rest of the crate already uses — no list-specific wrapper types — so the
/// GUI consumes the same vocabulary the CLI does (the library-canonical types
/// are the contract, STATE.md).
#[derive(serde::Serialize)]
#[cfg_attr(feature = "bindings", derive(specta::Type))]
pub struct ListReport {
    /// Discovered skills, sorted alphabetically by skill name.
    pub skills: Vec<DiscoveredSkill>,
    /// Non-fatal warnings emitted during discovery (naming-convention hints,
    /// deduplication notices). The CLI prints these to stderr unless `--quiet`;
    /// the GUI can surface them in a diagnostics view.
    pub warnings: Vec<String>,
}

/// Discover all skills for `tome list` and return them as a structured
/// [`ListReport`].
///
/// Discovery uses an empty `resolved_paths` map (git directories are listed at
/// their config URL, matching the previous inline `list()` behavior — listing
/// does not clone). Skills are sorted by name so both the CLI table and the GUI
/// list get a stable order.
pub fn collect(config: &Config) -> Result<ListReport> {
    let mut warnings = Vec::new();
    let mut skills = discover::discover_all(config, &BTreeMap::new(), &mut warnings)?;
    skills.sort_by(|a, b| a.name.as_str().cmp(b.name.as_str()));
    Ok(ListReport { skills, warnings })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{DirectoryConfig, DirectoryName, DirectoryRole, DirectoryType};
    use crate::discover::{DiscoveredSkill, SkillName, SkillOrigin};
    use std::path::PathBuf;
    use tempfile::TempDir;

    /// Build a Config with a single Source directory pointing at `path`.
    fn config_with_source(path: PathBuf) -> Config {
        let mut directories = BTreeMap::new();
        directories.insert(
            DirectoryName::new("test").unwrap(),
            DirectoryConfig {
                path,
                directory_type: DirectoryType::Directory,
                role: Some(DirectoryRole::Source),
                git_ref: None,
                subdir: None,
                override_applied: false,
            },
        );
        Config {
            directories,
            ..Config::default()
        }
    }

    fn create_skill(dir: &std::path::Path, name: &str) {
        let skill_dir = dir.join(name);
        std::fs::create_dir_all(&skill_dir).unwrap();
        std::fs::write(
            skill_dir.join("SKILL.md"),
            format!("---\nname: {name}\n---\n# {name}"),
        )
        .unwrap();
    }

    /// D-16: a discover-only run (no manifest join) returns skills with
    /// `synced_at: None`. Pins that `collect()` does NOT spontaneously
    /// stamp a value — `list` is read-only and never writes the manifest.
    #[test]
    fn collect_leaves_synced_at_none_for_unstamped_skills() {
        let tmp = TempDir::new().unwrap();
        create_skill(tmp.path(), "alpha");
        create_skill(tmp.path(), "beta");

        let config = config_with_source(tmp.path().to_path_buf());
        let report = collect(&config).unwrap();

        assert_eq!(report.skills.len(), 2);
        for skill in &report.skills {
            assert!(
                skill.synced_at.is_none(),
                "collect() must not populate synced_at — that's sync()'s job; \
                 saw {:?} for {}",
                skill.synced_at,
                skill.name,
            );
        }
    }

    /// D-16: ListReport's serde round-trip surfaces the `synced_at` field
    /// at the JSON boundary. Pin both the populated and missing cases so
    /// a future refactor that drops the `#[serde(default)]` or the field
    /// itself fails loudly here.
    ///
    /// Constructs a `DiscoveredSkill` by hand (skipping discovery) so the
    /// populated case doesn't require a fixture manifest — the manifest
    /// join semantic is owned by `lib.rs::sync` and exercised end-to-end
    /// by the sync integration tests (and the discover-side invariant by
    /// `discover_all_leaves_synced_at_none`).
    #[test]
    fn list_report_serializes_synced_at_in_json() {
        let stamped = DiscoveredSkill {
            name: SkillName::new("stamped").unwrap(),
            path: PathBuf::from("/tmp/stamped"),
            source_name: DirectoryName::new("test").unwrap(),
            origin: SkillOrigin::Local,
            frontmatter: None,
            synced_at: Some("2026-06-05T10:00:00Z".to_string()),
        };
        let unstamped = DiscoveredSkill {
            name: SkillName::new("unstamped").unwrap(),
            path: PathBuf::from("/tmp/unstamped"),
            source_name: DirectoryName::new("test").unwrap(),
            origin: SkillOrigin::Local,
            frontmatter: None,
            synced_at: None,
        };
        let report = ListReport {
            skills: vec![stamped, unstamped],
            warnings: vec![],
        };

        let json = serde_json::to_string(&report).unwrap();
        assert!(
            json.contains("\"synced_at\":\"2026-06-05T10:00:00Z\""),
            "populated synced_at must serialize as a string literal; got: {json}",
        );
        assert!(
            json.contains("\"synced_at\":null"),
            "None synced_at must serialize as JSON null; got: {json}",
        );
    }
}
