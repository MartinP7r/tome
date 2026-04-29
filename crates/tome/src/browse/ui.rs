use std::collections::HashSet;

use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Margin, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{
    Block, Borders, Cell, Clear, List, ListItem, ListState, Paragraph, Row, Scrollbar,
    ScrollbarOrientation, ScrollbarState, Table, Wrap,
};

use super::app::{App, Mode, SortMode};
use super::theme::Theme;

pub fn render(frame: &mut Frame, app: &mut App) {
    // Clone theme out to avoid borrow conflict with &mut app
    let theme = app.theme.clone();

    match app.mode {
        Mode::Detail => render_detail(frame, app, &theme),
        _ => render_normal(frame, app, &theme),
    }

    // Help overlay renders on top of any mode
    if app.mode == Mode::Help {
        render_help_overlay(frame, frame.area(), &theme);
    }
}

fn render_normal(frame: &mut Frame, app: &mut App, theme: &Theme) {
    let area = frame.area();

    let chunks = Layout::vertical([
        Constraint::Length(1), // header
        Constraint::Length(1), // separator
        Constraint::Min(1),    // body split
        Constraint::Length(1), // status bar
    ])
    .split(area);

    let body_chunks = Layout::horizontal([Constraint::Percentage(45), Constraint::Percentage(55)])
        .split(chunks[2]);

    // Update visible_height so App can compute scroll distances
    app.visible_height = body_chunks[0].height as usize;

    // -- Header --
    let header = Row::new(vec![
        Cell::from("SKILL").style(Style::default().add_modifier(Modifier::BOLD)),
        Cell::from("SOURCE").style(Style::default().add_modifier(Modifier::BOLD)),
        Cell::from("PATH").style(Style::default().add_modifier(Modifier::BOLD)),
    ])
    .style(Style::default().fg(theme.accent));

    let header_table = Table::new(std::iter::empty::<Row>(), widths())
        .header(header)
        .block(Block::default());
    frame.render_widget(header_table, chunks[0]);

    // -- Separator line --
    let separator = Paragraph::new(Line::from("\u{2500}".repeat(area.width as usize)))
        .style(Style::default().fg(theme.selected_bg));
    frame.render_widget(separator, chunks[1]);

    // -- Left body: skills table --
    let show_groups =
        app.group_by_source && app.sort_mode == SortMode::Source && app.search_input.is_empty();

    let visible_rows = build_visible_rows(app, show_groups, theme);

    let body_table = Table::new(visible_rows, widths()).block(Block::default());
    frame.render_widget(body_table, body_chunks[0]);

    // -- Scrollbar (only when items exceed viewport) --
    let total_items = app.filtered_indices.len();
    if total_items > app.visible_height {
        let mut scrollbar_state = ScrollbarState::new(total_items).position(app.scroll_offset);
        let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
            .begin_symbol(None)
            .end_symbol(None);
        frame.render_stateful_widget(
            scrollbar,
            body_chunks[0].inner(Margin {
                vertical: 0,
                horizontal: 0,
            }),
            &mut scrollbar_state,
        );
    }

    // -- Right body: selected skill preview --
    let preview_lines = super::markdown::render_markdown(&app.preview_content, theme);
    let preview = Paragraph::new(preview_lines)
        .block(
            Block::default()
                .title(app.preview_title.as_str())
                .borders(Borders::LEFT),
        )
        .wrap(Wrap { trim: false });
    frame.render_widget(preview, body_chunks[1]);

    // -- Status bar --
    render_status_bar(frame, app, area.width, chunks[3], theme);
}

/// Build the visible table rows, optionally inserting group headers when
/// `show_groups` is true. Group headers are non-selectable visual separators.
fn build_visible_rows<'a>(app: &'a App, show_groups: bool, theme: &Theme) -> Vec<Row<'a>> {
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
                let header_text = format!(
                    "\u{2500}\u{2500} {} ({}) \u{2500}\u{2500}",
                    current_source, group_count
                );
                rows.push(
                    Row::new(vec![
                        Cell::from(header_text),
                        Cell::from(""),
                        Cell::from(""),
                    ])
                    .style(theme.group_header()),
                );
                prev_source = Some(current_source);
            }
        }

        let style = if abs_idx == app.selected {
            Style::default()
                .bg(theme.selected_bg)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default()
        };

        // Build skill name cell with fuzzy match highlighting
        let name_indices = app.match_indices.get(row_idx).cloned().unwrap_or_default();
        let name_cell = if name_indices.is_empty() {
            Cell::from(row.name.as_str())
        } else {
            Cell::from(highlight_name(&row.name, &name_indices, theme))
        };

        rows.push(
            Row::new(vec![
                name_cell,
                Cell::from(row.source.as_str()),
                Cell::from(row.path.as_str()),
            ])
            .style(style),
        );
    }

    rows
}

/// Build a `Line` with matched characters highlighted using the theme.
fn highlight_name<'a>(name: &'a str, indices: &[u32], theme: &Theme) -> Line<'a> {
    let index_set: HashSet<u32> = indices.iter().copied().collect();
    let spans: Vec<Span> = name
        .chars()
        .enumerate()
        .map(|(i, ch)| {
            if index_set.contains(&(i as u32)) {
                Span::styled(ch.to_string(), theme.match_highlight())
            } else {
                Span::raw(ch.to_string())
            }
        })
        .collect();
    Line::from(spans)
}

fn render_detail(frame: &mut Frame, app: &mut App, theme: &Theme) {
    let area = frame.area();

    let chunks = Layout::vertical([
        Constraint::Min(1),    // body
        Constraint::Length(1), // status bar
    ])
    .split(area);

    let body_chunks = Layout::horizontal([Constraint::Percentage(40), Constraint::Percentage(60)])
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
        Span::styled("< ", Style::default().fg(theme.accent)),
        Span::styled(
            name,
            Style::default()
                .fg(theme.accent)
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
    let label_style = Style::default().fg(theme.muted);
    let metadata = Paragraph::new(vec![
        Line::from(
            "\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}",
        ),
        Line::from(vec![
            Span::styled("Source:   ", label_style),
            Span::raw(source),
        ]),
        Line::from(vec![
            Span::styled("Type:     ", label_style),
            Span::raw(type_label),
        ]),
        Line::from(vec![
            Span::styled("Path:     ", label_style),
            Span::raw(crate::paths::collapse_home(std::path::Path::new(path))),
        ]),
        Line::from(vec![
            Span::styled("Synced:   ", label_style),
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
                    .fg(theme.accent)
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
        .border_style(Style::default().fg(theme.muted));
    let actions_list = List::new(items).block(actions_block);
    let mut list_state = ListState::default().with_selected(Some(app.detail_selected));
    frame.render_stateful_widget(actions_list, left_chunks[2], &mut list_state);

    // -- Right side: preview with markdown rendering --
    let preview_lines = super::markdown::render_markdown(&app.preview_content, theme);
    let preview = Paragraph::new(preview_lines)
        .block(
            Block::default()
                .title(app.preview_title.as_str())
                .borders(Borders::LEFT),
        )
        .wrap(Wrap { trim: false });
    frame.render_widget(preview, body_chunks[1]);

    // -- Status bar for Detail mode --
    // When execute_action has set a status message, render it in place of the
    // usual Detail keybind line. Switch on StatusSeverity for color dispatch —
    // matches the same semantic as render_status_bar (Normal mode) so the two
    // call sites feel identical to the user across modes. Cleared by
    // handle_key on next keypress.
    let status = if let Some(msg) = &app.status_message {
        let msg_style = match msg.severity() {
            super::app::StatusSeverity::Warning => {
                Style::default().fg(theme.alert).bg(theme.status_bar_bg)
            }
            super::app::StatusSeverity::Success => {
                Style::default().fg(theme.accent).bg(theme.status_bar_bg)
            }
            super::app::StatusSeverity::Pending => {
                Style::default().fg(theme.muted).bg(theme.status_bar_bg)
            }
        };
        Line::from(vec![
            Span::styled(format!(" {} {} ", msg.glyph(), msg.body()), msg_style),
            Span::styled(
                " ".repeat(area.width as usize),
                Style::default().bg(theme.status_bar_bg),
            ),
        ])
    } else {
        Line::from(vec![
            Span::styled(
                " Detail ",
                Style::default()
                    .fg(theme.status_bar_fg)
                    .bg(theme.accent)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                " j/k select  \u{23ce} run action  esc back",
                Style::default()
                    .fg(theme.status_bar_fg)
                    .bg(theme.status_bar_bg),
            ),
            Span::styled(
                " ".repeat(area.width as usize),
                Style::default().bg(theme.status_bar_bg),
            ),
        ])
    };
    frame.render_widget(Paragraph::new(status), chunks[1]);
}

fn render_status_bar(
    frame: &mut Frame,
    app: &App,
    width: u16,
    area: ratatui::layout::Rect,
    theme: &Theme,
) {
    // NOTE: as of phase 8, `status_message` is only set from DetailAction
    // handlers, which leave the app in Mode::Detail — so this Normal-mode
    // block is currently latent and exercises only when Normal-mode status
    // sources are added (e.g., future bulk actions like "copied N paths").
    // The block is kept so the invariant (any-key-dismisses in any mode)
    // is preserved at the call site and the switch-on-severity logic lives
    // in one place rather than being duplicated if/when Normal-mode sources
    // appear.
    if let Some(msg) = &app.status_message {
        let style = match msg.severity() {
            super::app::StatusSeverity::Warning => {
                Style::default().fg(theme.alert).bg(theme.status_bar_bg)
            }
            super::app::StatusSeverity::Success => {
                Style::default().fg(theme.accent).bg(theme.status_bar_bg)
            }
            super::app::StatusSeverity::Pending => {
                Style::default().fg(theme.muted).bg(theme.status_bar_bg)
            }
        };
        let bg_style = Style::default().bg(theme.status_bar_bg);
        let spans = vec![
            Span::styled(format!(" {} {} ", msg.glyph(), msg.body()), style),
            Span::styled(" ".repeat(width as usize), bg_style),
        ];
        frame.render_widget(Paragraph::new(Line::from(spans)), area);
        return;
    }

    let filtered = app.filtered_indices.len();
    let total = app.rows.len();

    let count_text = if filtered == total {
        format!("{total} skills")
    } else {
        format!("{filtered}/{total} skills")
    };

    let mode_text = match app.mode {
        Mode::Normal | Mode::Detail | Mode::Help => String::new(),
        Mode::Search => format!("/{}", app.search_input),
    };

    let sort_label = app.sort_mode.label();
    let sort_hint = format!("sort:{}", sort_label);

    // Key/label pairs rendered with distinct styles for scanability.
    // Keys get accent+bold, labels get muted. Inspired by zellij's status bar.
    let hint_pairs: &[(&str, &str)] = &[
        ("j/k", "\u{2195}"),
        ("/", "search"),
        ("s", sort_hint.as_str()),
        ("tab", "group"),
        ("?", "help"),
        ("\u{23ce}", "detail"),
        ("q", "quit"),
    ];

    let key_style = Style::default()
        .fg(theme.badge_bg)
        .bg(theme.status_bar_bg)
        .add_modifier(Modifier::BOLD);
    let label_style = Style::default().fg(theme.muted).bg(theme.status_bar_bg);
    let bg_style = Style::default().bg(theme.status_bar_bg);

    let mut spans = vec![
        // Count badge: high-contrast foreground on badge_bg
        Span::styled(
            format!(" {count_text} "),
            Style::default()
                .fg(theme.badge_fg)
                .bg(theme.badge_bg)
                .add_modifier(Modifier::BOLD),
        ),
        // Separator + optional search input
        Span::styled(
            format!(" {} ", mode_text),
            Style::default().fg(theme.alert).bg(theme.status_bar_bg),
        ),
    ];

    for (i, (key, label)) in hint_pairs.iter().enumerate() {
        if i > 0 {
            spans.push(Span::styled("  ", bg_style));
        }
        spans.push(Span::styled((*key).to_string(), key_style));
        spans.push(Span::styled(" ", bg_style));
        spans.push(Span::styled((*label).to_string(), label_style));
    }

    // Fill the rest of the line with bg
    spans.push(Span::styled(" ".repeat(width as usize), bg_style));

    frame.render_widget(Paragraph::new(Line::from(spans)), area);
}

/// Render a centered help overlay showing all keyboard shortcuts.
fn render_help_overlay(frame: &mut Frame, area: Rect, theme: &Theme) {
    let popup_width: u16 = 40;
    let popup_height: u16 = 18;

    // Ensure terminal is large enough
    if area.width < popup_width || area.height < popup_height {
        return;
    }

    let x = (area.width - popup_width) / 2;
    let y = (area.height - popup_height) / 2;
    let popup_area = Rect::new(x, y, popup_width, popup_height);

    // Clear the area behind the popup
    frame.render_widget(Clear, popup_area);

    let block = Block::default()
        .title(Span::styled(
            "Keyboard Shortcuts",
            Style::default().add_modifier(Modifier::BOLD),
        ))
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme.accent));

    let inner = block.inner(popup_area);
    frame.render_widget(block, popup_area);

    let help_lines = vec![
        Line::from("  j / Down      Move down"),
        Line::from("  k / Up        Move up"),
        Line::from("  g             Jump to top"),
        Line::from("  G             Jump to bottom"),
        Line::from("  Ctrl+d        Half page down"),
        Line::from("  Ctrl+u        Half page up"),
        Line::from("  /             Search"),
        Line::from("  Esc           Clear search / back"),
        Line::from("  s             Cycle sort mode"),
        Line::from("  Tab           Toggle source grouping"),
        Line::from("  Enter         Skill detail"),
        Line::from("  q             Quit"),
        Line::from("  ?             This help"),
        Line::from(""),
        Line::from(Span::styled(
            "      Press any key to close",
            Style::default().fg(theme.muted),
        )),
    ];

    let help_text = Paragraph::new(help_lines);
    frame.render_widget(help_text, inner);
}

fn widths() -> [Constraint; 3] {
    [
        Constraint::Percentage(25),
        Constraint::Percentage(20),
        Constraint::Fill(1),
    ]
}
