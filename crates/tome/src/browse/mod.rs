pub(crate) mod app;
pub(crate) mod fuzzy;
pub(crate) mod markdown;
pub(crate) mod theme;
pub(crate) mod ui;

use std::time::Duration;

use anyhow::Result;
use crossterm::event::{self, Event};

use crate::discover::DiscoveredSkill;
use app::{App, SkillRow};

/// Launch the interactive skill browser.
pub fn browse(skills: Vec<DiscoveredSkill>, manifest: &crate::manifest::Manifest) -> Result<()> {
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
                source: s.source_name,
                path: s.path.display().to_string(),
                managed,
                synced_at,
            }
        })
        .collect();

    let mut app = App::new(rows);
    let mut terminal = ratatui::init();

    let result = run_loop(&mut terminal, &mut app);

    ratatui::restore();
    result
}

fn run_loop(terminal: &mut ratatui::DefaultTerminal, app: &mut App) -> Result<()> {
    loop {
        terminal.draw(|frame| ui::render(frame, app))?;

        if event::poll(Duration::from_millis(100))?
            && let Event::Key(key) = event::read()?
        {
            app.handle_key(key);
        }

        if app.should_quit {
            break;
        }
    }
    Ok(())
}
