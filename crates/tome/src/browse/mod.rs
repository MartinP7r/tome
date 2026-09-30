// `app`, `theme`, and `ui` are widened to `pub` under the `test-support`
// feature so the integration snapshot tests in `tests/browse_snapshots/`
// (HARD-12) can construct `App` fixtures, pick a `Theme`, and call
// `ui::render` against a `ratatui::backend::TestBackend`. Production
// builds keep the old `pub(crate)` visibility byte-for-byte.
#[cfg(any(test, feature = "test-support"))]
pub mod app;
#[cfg(not(any(test, feature = "test-support")))]
pub(crate) mod app;

pub(crate) mod fuzzy;
pub(crate) mod markdown;

#[cfg(any(test, feature = "test-support"))]
pub mod theme;
#[cfg(not(any(test, feature = "test-support")))]
pub(crate) mod theme;

#[cfg(any(test, feature = "test-support"))]
pub mod ui;
#[cfg(not(any(test, feature = "test-support")))]
pub(crate) mod ui;

use std::path::Path;
use std::time::Duration;

use anyhow::{Context, Result};
use crossterm::event::{self, Event};

use crate::discover::DiscoveredSkill;
use crate::machine::MachinePrefs;
use app::{App, SkillRow};

/// Project the canonical library into the rows consumed by the browser.
///
/// Browse is a view of skills that have already been consolidated, not a
/// source discovery command. The manifest supplies ownership, sync time, and
/// managed/provenance metadata while the library remains the filesystem
/// authority for which skills are present.
pub(crate) fn load_library_skills(
    paths: &crate::paths::TomePaths,
) -> Result<(Vec<DiscoveredSkill>, crate::manifest::Manifest)> {
    let manifest = crate::manifest::load(paths.config_dir())?;
    let lockfile = crate::lockfile::load(paths.config_dir())?;
    let mut skills = Vec::new();

    let mut paths_in_library = if paths.library_dir().is_dir() {
        std::fs::read_dir(paths.library_dir())
            .with_context(|| {
                format!(
                    "failed to read canonical library {}",
                    paths.library_dir().display()
                )
            })?
            .filter_map(|entry| entry.ok().map(|entry| entry.path()))
            .filter(|path| is_skill_directory(path))
            .collect::<Vec<_>>()
    } else {
        Vec::new()
    };
    paths_in_library.sort();

    for path in paths_in_library {
        let Some(name) = path
            .file_name()
            .and_then(|name| name.to_str())
            .and_then(|name| crate::discover::SkillName::new(name).ok())
        else {
            continue;
        };
        let entry = manifest.get(name.as_str());
        let source_name = entry
            .and_then(|entry| entry.source_name().or_else(|| entry.previous_source()))
            .cloned()
            .unwrap_or_else(|| {
                crate::config::DirectoryName::new("library")
                    .expect("the static canonical-library source name is valid")
            });
        let provenance = lockfile
            .as_ref()
            .and_then(|lockfile| lockfile.skills.get(&name))
            .and_then(|entry| {
                entry
                    .registry_id
                    .as_ref()
                    .map(|registry_id| crate::discover::SkillProvenance {
                        registry_id: registry_id.clone(),
                        version: entry.version.clone(),
                        git_commit_sha: entry.git_commit_sha.clone(),
                    })
            });
        let origin = if entry.is_some_and(|entry| entry.managed) {
            crate::discover::SkillOrigin::Managed { provenance }
        } else {
            crate::discover::SkillOrigin::Local
        };

        skills.push(DiscoveredSkill {
            name,
            path,
            source_name,
            origin,
            frontmatter: None,
            synced_at: entry.map(|entry| entry.synced_at.clone()),
        });
    }

    Ok((skills, manifest))
}

fn is_skill_directory(path: &Path) -> bool {
    path.is_dir() && path.join("SKILL.md").is_file()
}

/// Launch the interactive skill browser.
///
/// Detail-menu exclusion toggles are unavailable until they can select the
/// destination required by `tome route exclude`.
pub fn browse(
    skills: Vec<DiscoveredSkill>,
    manifest: &crate::manifest::Manifest,
    machine_prefs: MachinePrefs,
) -> Result<()> {
    let rows: Vec<SkillRow> = skills
        .into_iter()
        .map(|s| {
            let skill_name = s.name.to_string();
            let synced_at = manifest
                .get(&skill_name)
                .map(|e| e.synced_at.clone())
                .unwrap_or_default();
            let managed = s.origin.is_managed();
            SkillRow {
                name: skill_name,
                source: s.source_name.as_str().to_string(),
                path: s.path.display().to_string(),
                managed,
                synced_at,
                source_directory: Some(s.source_name),
            }
        })
        .collect();

    let mut app = App::new(rows).with_machine_prefs(machine_prefs);
    let mut terminal = ratatui::init();

    let result = run_loop(&mut terminal, &mut app);

    ratatui::restore();
    result
}

fn run_loop(terminal: &mut ratatui::DefaultTerminal, app: &mut App) -> Result<()> {
    loop {
        let area = terminal.draw(|frame| ui::render(frame, app))?.area;
        // ui::render takes `&App` (so it can be invoked from the
        // POLISH-01 redraw closure inside handle_key, which only has
        // a shared borrow on App). The viewport-cache mutation that
        // used to live in render_normal is hoisted here so scroll
        // distances stay correct on the next handle_key tick.
        app.visible_height = ui::body_height_for_area(area);

        if event::poll(Duration::from_millis(100))?
            && let Event::Key(key) = event::read()?
        {
            // POLISH-01: redraw closure threaded into handle_key so the
            // ViewSource arm can surface a `Pending("Opening: ...")` message
            // BEFORE `.status()` blocks. The closure receives `&App` (the
            // current state from inside `handle_key`) and re-renders via
            // the captured `terminal`. Draw errors are dropped — a draw
            // failure must not abort the open action; the top-of-loop
            // `terminal.draw(...)` will recover on the next tick.
            let mut redraw = |a: &App| {
                let _ = terminal.draw(|frame| ui::render(frame, a));
            };
            app.handle_key(key, &mut redraw);
        }

        if app.should_quit {
            break;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::DirectoryName;
    use crate::manifest::{Manifest, SkillEntry};
    use crate::paths::TomePaths;
    use crate::validation::test_hash;
    use tempfile::TempDir;

    fn create_skill(library_dir: &Path, name: &str) -> std::path::PathBuf {
        let skill_dir = library_dir.join(name);
        std::fs::create_dir_all(&skill_dir).unwrap();
        std::fs::write(
            skill_dir.join("SKILL.md"),
            format!("---\nname: {name}\n---\n# Demo skill"),
        )
        .unwrap();
        skill_dir
    }

    #[test]
    fn library_skills_are_loaded_without_configured_sources() {
        let tmp = TempDir::new().unwrap();
        let library_dir = tmp.path().join("skills");
        let skill_dir = create_skill(&library_dir, "demo-skill");
        let paths = TomePaths::new(tmp.path().to_path_buf(), library_dir.clone()).unwrap();

        let mut manifest = Manifest::default();
        let mut entry = SkillEntry::new(
            skill_dir.clone(),
            DirectoryName::new("original-source").unwrap(),
            test_hash("demo-skill"),
            true,
        );
        entry.synced_at = "2026-09-29T00:00:00Z".to_string();
        manifest.insert(
            crate::discover::SkillName::new("demo-skill").unwrap(),
            entry,
        );
        crate::manifest::save(&manifest, paths.config_dir()).unwrap();

        let (skills, loaded_manifest) = load_library_skills(&paths).unwrap();

        assert_eq!(skills.len(), 1);
        assert_eq!(skills[0].name.as_str(), "demo-skill");
        assert_eq!(skills[0].path, skill_dir);
        assert_eq!(skills[0].source_name.as_str(), "original-source");
        assert!(skills[0].origin.is_managed());
        assert_eq!(skills[0].synced_at.as_deref(), Some("2026-09-29T00:00:00Z"));
        assert!(loaded_manifest.contains_key("demo-skill"));
    }

    #[test]
    fn library_skills_are_loaded_without_a_manifest_entry() {
        let tmp = TempDir::new().unwrap();
        let library_dir = tmp.path().join("skills");
        let skill_dir = create_skill(&library_dir, "untracked-skill");
        let paths = TomePaths::new(tmp.path().to_path_buf(), library_dir).unwrap();

        let (skills, manifest) = load_library_skills(&paths).unwrap();

        assert_eq!(skills.len(), 1);
        assert_eq!(skills[0].name.as_str(), "untracked-skill");
        assert_eq!(skills[0].path, skill_dir);
        assert_eq!(skills[0].source_name.as_str(), "library");
        assert!(!skills[0].origin.is_managed());
        assert_eq!(skills[0].synced_at, None);
        assert!(!manifest.contains_key("untracked-skill"));
    }
}
