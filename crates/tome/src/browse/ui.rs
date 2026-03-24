use ratatui::Frame;
use ratatui::layout::{Constraint, Layout};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{
    Block, Borders, Cell, List, ListItem, ListState, Paragraph, Row, Table, Wrap,
};

use super::app::{App, Mode, SortMode};

pub fn render(frame: &mut Frame, app: &mut App) {
    match app.mode {
        Mode::Detail => render_detail(frame, app),
        _ => render_normal(frame, app),
    }
}

fn render_normal(frame: &mut Frame, app: &mut App) {
    let area = frame.area();

    let chunks = Layout::vertical([
        Constraint::Length(1), // header
        Constraint::Length(1), // separator
        Constraint::Min(1),    // body split
        Constraint::Length(1), // status bar
    ])
    .split(area);

    let body_chunks =
        Layout::horizontal([Constraint::Percentage(45), Constraint::Percentage(55)])
            .split(chunks[2]);

    // Update visible_height so App can compute scroll distances
    app.visible_height = body_chunks[0].height as usize;

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

    // -- Left body: skills table --
    let show_groups =
        app.group_by_source && app.sort_mode == SortMode::Source && app.search_input.is_empty();

    let visible_rows = build_visible_rows(app, show_groups);

    let body_table = Table::new(visible_rows, widths()).block(Block::default());
    frame.render_widget(body_table, body_chunks[0]);

    // -- Right body: selected skill preview --
    let preview = Paragraph::new(app.preview_content.as_str())
        .block(
            Block::default()
                .title(app.preview_title.as_str())
                .borders(Borders::LEFT),
        )
        .wrap(Wrap { trim: false });
    frame.render_widget(preview, body_chunks[1]);

    // -- Status bar --
    render_status_bar(frame, app, area.width, chunks[3]);
}

/// Build the visible table rows, optionally inserting group headers when
/// `show_groups` is true. Group headers are non-selectable visual separators.
fn build_visible_rows<'a>(app: &'a App, show_groups: bool) -> Vec<Row<'a>> {
    let mut rows: Vec<Row<'a>> = Vec::new();
    let mut prev_source: Option<&str> = None;

    for (vis_idx, &row_idx) in app
        .filtered_indices
        .iter()
        .skip(app.scroll_offset)
        .take(app.visible_height)
        .enumerate()
    {
        let row = &app.rows[row_idx];
        let abs_idx = app.scroll_offset + vis_idx;

        // Insert group header when source changes
        if show_groups {
            let current_source = row.source.as_str();
            if prev_source != Some(current_source) {
                // Count skills in this source group
                let group_count = app
                    .filtered_indices
                    .iter()
                    .filter(|&&idx| app.rows[idx].source == current_source)
                    .count();
                let header_text = format!("── {} ({}) ──", current_source, group_count);
                rows.push(
                    Row::new(vec![
                        Cell::from(header_text),
                        Cell::from(""),
                        Cell::from(""),
                    ])
                    .style(
                        Style::default()
                            .fg(Color::Yellow)
                            .add_modifier(Modifier::BOLD),
                    ),
                );
                prev_source = Some(current_source);
            }
        }

        let style = if abs_idx == app.selected {
            Style::default()
                .bg(Color::DarkGray)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default()
        };
        rows.push(
            Row::new(vec![
                Cell::from(row.name.as_str()),
                Cell::from(row.source.as_str()),
                Cell::from(row.path.as_str()),
            ])
            .style(style),
        );
    }

    rows
}

fn render_detail(frame: &mut Frame, app: &mut App) {
    let area = frame.area();

    let chunks = Layout::vertical([
        Constraint::Min(1),    // body
        Constraint::Length(1), // status bar
    ])
    .split(area);

    let body_chunks = Layout::horizontal([
        Constraint::Percentage(45),
        Constraint::Length(1), // left padding
        Constraint::Length(1), // right padding
        Constraint::Percentage(55),
    ])
    .split(chunks[0]);

    // -- Left side: metadata + action list --
    let left_chunks = Layout::vertical([
        Constraint::Length(2), // title + separator
        Constraint::Length(6), // metadata
        Constraint::Min(1),    // actions
    ])
    .split(body_chunks[0]);

    // Get the selected row info
    let (name, source, path, managed, synced_at) =
        if let Some(&row_idx) = app.filtered_indices.get(app.selected) {
            let row = &app.rows[row_idx];
            (
                row.name.as_str(),
                row.source.as_str(),
                row.path.as_str(),
                row.managed,
                row.synced_at.as_str(),
            )
        } else {
            ("(none)", "", "", false, "")
        };

    // Title
    let title = Paragraph::new(Line::from(vec![
        Span::styled("< ", Style::default().fg(Color::Cyan)),
        Span::styled(
            name,
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ),
    ]));
    frame.render_widget(title, left_chunks[0]);

    // Metadata
    let type_label = if managed { "managed" } else { "local" };
    let synced_display = if synced_at.is_empty() {
        "(unknown)"
    } else {
        synced_at
    };
    let metadata = Paragraph::new(vec![
        Line::from("────────────────────"),
        Line::from(vec![
            Span::styled("Source:   ", Style::default().fg(Color::DarkGray)),
            Span::raw(source),
        ]),
        Line::from(vec![
            Span::styled("Type:     ", Style::default().fg(Color::DarkGray)),
            Span::raw(type_label),
        ]),
        Line::from(vec![
            Span::styled("Path:     ", Style::default().fg(Color::DarkGray)),
            Span::raw(path),
        ]),
        Line::from(vec![
            Span::styled("Synced:   ", Style::default().fg(Color::DarkGray)),
            Span::raw(synced_display),
        ]),
    ]);
    frame.render_widget(metadata, left_chunks[1]);

    // Actions list
    let items: Vec<ListItem> = app
        .detail_actions
        .iter()
        .enumerate()
        .map(|(i, action)| {
            let style = if i == app.detail_selected {
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };
            let prefix = if i == app.detail_selected { "> " } else { "  " };
            ListItem::new(format!("{}{}", prefix, action.label())).style(style)
        })
        .collect();

    let actions_block = Block::default()
        .title("Actions")
        .borders(Borders::TOP)
        .border_style(Style::default().fg(Color::DarkGray));
    let actions_list = List::new(items).block(actions_block);
    let mut list_state = ListState::default().with_selected(Some(app.detail_selected));
    frame.render_stateful_widget(actions_list, left_chunks[2], &mut list_state);

    // -- Right side: preview --
    let preview = Paragraph::new(app.preview_content.as_str())
        .block(
            Block::default()
                .title(app.preview_title.as_str())
                .borders(Borders::LEFT),
        )
        .wrap(Wrap { trim: false });
    frame.render_widget(preview, body_chunks[1]);

    // -- Status bar for Detail mode --
    let status = Line::from(vec![
        Span::styled(
            " Detail ",
            Style::default()
                .fg(Color::Black)
                .bg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            " j/k select  \u{23ce} run action  esc back",
            Style::default().fg(Color::Gray).bg(Color::DarkGray),
        ),
        Span::styled(
            " ".repeat(area.width as usize),
            Style::default().bg(Color::DarkGray),
        ),
    ]);
    frame.render_widget(Paragraph::new(status), chunks[1]);
}

fn render_status_bar(frame: &mut Frame, app: &App, width: u16, area: ratatui::layout::Rect) {
    let filtered = app.filtered_indices.len();
    let total = app.rows.len();

    let count_text = if filtered == total {
        format!("{total} skills")
    } else {
        format!("{filtered}/{total} skills")
    };

    let mode_text = match app.mode {
        Mode::Normal | Mode::Detail => String::new(),
        Mode::Search => format!("/{}", app.search_input),
    };

    let sort_label = app.sort_mode.label();
    let hints = format!(
        " | j/k \u{2195}  / search  s sort:{}  tab group  \u{23ce} detail  q quit",
        sort_label
    );

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
        Span::styled(hints, Style::default().fg(Color::Gray).bg(Color::DarkGray)),
        // Fill the rest
        Span::styled(
            " ".repeat(width as usize),
            Style::default().bg(Color::DarkGray),
        ),
    ]);

    frame.render_widget(Paragraph::new(status), area);
}

fn widths() -> [Constraint; 3] {
    [
        Constraint::Percentage(25),
        Constraint::Percentage(20),
        Constraint::Fill(1),
    ]
}
