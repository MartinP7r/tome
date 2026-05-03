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
                source: s.source_name.as_str().to_string(),
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
