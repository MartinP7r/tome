use ratatui::Frame;
use ratatui::layout::{Constraint, Layout};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Cell, Paragraph, Row, Table};

use super::app::{App, Mode};

pub fn render(frame: &mut Frame, app: &mut App) {
    let area = frame.area();

    let chunks = Layout::vertical([
        Constraint::Length(1), // header
        Constraint::Length(1), // separator
        Constraint::Min(1),    // table body
        Constraint::Length(1), // status bar
    ])
    .split(area);

    // Update visible_height so App can compute scroll distances
    app.visible_height = chunks[2].height as usize;

    // -- Header --
    let header = Row::new(vec![
        Cell::from("SKILL").style(Style::default().add_modifier(Modifier::BOLD)),
        Cell::from("SOURCE").style(Style::default().add_modifier(Modifier::BOLD)),
        Cell::from("PATH").style(Style::default().add_modifier(Modifier::BOLD)),
    ])
    .style(Style::default().fg(Color::Cyan));

    let header_table = Table::new(std::iter::empty::<Row>(), widths())
        .header(header)
        .block(Block::default());
    frame.render_widget(header_table, chunks[0]);

    // -- Separator line --
    let separator = Paragraph::new(Line::from("─".repeat(area.width as usize)))
        .style(Style::default().fg(Color::DarkGray));
    frame.render_widget(separator, chunks[1]);

    // -- Body --
    let visible_rows: Vec<Row> = app
        .filtered_indices
        .iter()
        .skip(app.scroll_offset)
        .take(app.visible_height)
        .enumerate()
        .map(|(vis_idx, &row_idx)| {
            let row = &app.rows[row_idx];
            let abs_idx = app.scroll_offset + vis_idx;
            let style = if abs_idx == app.selected {
                Style::default()
                    .bg(Color::DarkGray)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };
            Row::new(vec![
                Cell::from(row.name.as_str()),
                Cell::from(row.source.as_str()),
                Cell::from(row.path.as_str()),
            ])
            .style(style)
        })
        .collect();

    let body_table = Table::new(visible_rows, widths()).block(Block::default());
    frame.render_widget(body_table, chunks[2]);

    // -- Status bar --
    let filtered = app.filtered_indices.len();
    let total = app.rows.len();

    let count_text = if filtered == total {
        format!("{total} skills")
    } else {
        format!("{filtered}/{total} skills")
    };

    let mode_text = match app.mode {
        Mode::Normal => String::new(),
        Mode::Search => format!("/{}", app.search_input),
    };

    let hints = " │ j/k ↕  / search  q quit";

    let status = Line::from(vec![
        Span::styled(
            format!(" {count_text} "),
            Style::default()
                .fg(Color::Black)
                .bg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            format!(" {mode_text}"),
            Style::default().fg(Color::Yellow).bg(Color::DarkGray),
        ),
        Span::styled(
            hints.to_string(),
            Style::default().fg(Color::Gray).bg(Color::DarkGray),
        ),
        // Fill the rest
        Span::styled(
            " ".repeat(area.width as usize),
            Style::default().bg(Color::DarkGray),
        ),
    ]);

    frame.render_widget(Paragraph::new(status), chunks[3]);
}

fn widths() -> [Constraint; 3] {
    [
        Constraint::Percentage(25),
        Constraint::Percentage(20),
        Constraint::Fill(1),
    ]
}
