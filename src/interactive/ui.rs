use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph, Wrap},
    Frame,
};

use super::app::{AppMode, FocusState, IndexStatusState, InteractiveApp};

/// Main render function
pub fn render(f: &mut Frame, app: &InteractiveApp) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Header (input)
            Constraint::Length(3), // Filters
            Constraint::Min(1),    // Content area
            Constraint::Length(1), // Footer/help bar
        ])
        .split(f.area());

    render_header(f, chunks[0], app);
    render_filters(f, chunks[1], app);

    match app.mode() {
        AppMode::Help => render_help_screen(f, chunks[2], app),
        AppMode::FilePreview => render_file_preview(f, chunks[2], app),
        _ => render_results_area(f, chunks[2], app),
    }

    render_footer(f, chunks[3], app);
}

fn render_header(f: &mut Frame, area: Rect, app: &InteractiveApp) {
    let palette = &app.theme().palette;
    let input_focused = matches!(app.focus_state(), FocusState::Input);

    // Render query input with prominent focus indicator
    let title = if input_focused {
        " Search [TYPING - Press Tab/Enter to navigate] "
    } else {
        " Search [Press Tab to focus, / to type] "
    };

    let input_block = Block::default()
        .borders(Borders::ALL)
        .title(title)
        .border_style(if input_focused {
            Style::default()
                .fg(palette.accent)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(palette.muted)
        });

    let input_text = app.input().value();
    let input_style = if input_focused {
        Style::default()
            .fg(palette.foreground)
            .bg(Color::Rgb(40, 40, 40)) // Subtle background highlight when focused
    } else {
        Style::default().fg(palette.foreground)
    };

    let input_paragraph = Paragraph::new(input_text)
        .block(input_block)
        .style(input_style);

    f.render_widget(input_paragraph, area);

    // Set cursor position if input is focused
    if input_focused {
        let cursor_x = area.x + 1 + app.input().visual_cursor() as u16;
        let cursor_y = area.y + 1;
        f.set_cursor_position((cursor_x.min(area.right() - 2), cursor_y));
    }
}

fn render_filters(f: &mut Frame, area: Rect, app: &InteractiveApp) {
    let palette = &app.theme().palette;
    let filters_focused = matches!(app.focus_state(), FocusState::Filters);

    let border_style = if filters_focused {
        Style::default()
            .fg(palette.accent)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(palette.muted)
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .title(" Filters [s: symbols, r: regex] ")
        .border_style(border_style);

    let filters = app.filters();

    // Create clickable filter buttons
    let mut filter_spans = vec![];

    // Symbols button
    let symbols_style = if filters.symbols_mode {
        Style::default()
            .fg(Color::Black)
            .bg(palette.badge_active)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default()
            .fg(palette.muted)
            .bg(Color::Rgb(30, 30, 30))
    };
    filter_spans.push(Span::styled(" [s] Symbols ", symbols_style));
    filter_spans.push(Span::raw("  "));

    // Regex button
    let regex_style = if filters.regex_mode {
        Style::default()
            .fg(Color::Black)
            .bg(palette.warning)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default()
            .fg(palette.muted)
            .bg(Color::Rgb(30, 30, 30))
    };
    filter_spans.push(Span::styled(" [r] Regex ", regex_style));

    // Language filter (if set)
    if let Some(ref lang) = filters.language {
        filter_spans.push(Span::raw("  "));
        filter_spans.push(Span::styled(
            format!(" Lang: {} ", lang),
            Style::default()
                .fg(Color::Black)
                .bg(palette.info)
                .add_modifier(Modifier::BOLD),
        ));
    }

    // Kind filter (if set)
    if let Some(ref kind) = filters.kind {
        filter_spans.push(Span::raw("  "));
        filter_spans.push(Span::styled(
            format!(" Kind: {} ", kind),
            Style::default()
                .fg(Color::Black)
                .bg(palette.info)
                .add_modifier(Modifier::BOLD),
        ));
    }

    if filter_spans.is_empty() {
        filter_spans.push(Span::styled(
            " No filters active ",
            Style::default().fg(palette.muted),
        ));
    }

    let paragraph = Paragraph::new(Line::from(filter_spans))
        .block(block)
        .style(Style::default().fg(palette.foreground));

    f.render_widget(paragraph, area);
}

fn render_results_area(f: &mut Frame, area: Rect, app: &InteractiveApp) {
    let palette = &app.theme().palette;

    // Show error message if present
    if let Some(error) = app.error_message() {
        let error_text = Paragraph::new(error)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(" Error ")
                    .border_style(Style::default().fg(palette.error)),
            )
            .style(Style::default().fg(palette.error))
            .wrap(Wrap { trim: true });
        f.render_widget(error_text, area);
        return;
    }

    // Show info message if present
    if let Some(info) = app.info_message() {
        let info_text = Paragraph::new(info)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(" Info ")
                    .border_style(Style::default().fg(palette.info)),
            )
            .style(Style::default().fg(palette.info))
            .wrap(Wrap { trim: true });
        f.render_widget(info_text, area);
        return;
    }

    let results = app.results();

    if results.is_empty() {
        // Show empty state with helpful instructions
        let empty_message = if app.input().value().trim().is_empty() {
            if matches!(app.focus_state(), FocusState::Input) {
                "Start typing to search...\n\nKeyboard shortcuts:\n  j/k or ↓/↑ - Navigate results\n  / - Focus search\n  Esc/Enter - Unfocus search\n  ? - Show help\n  q - Quit"
            } else {
                "Press / to start typing a search query\nPress ? for full help"
            }
        } else {
            "No results found. Try a different query.\n\nTip: Press / to edit your search"
        };

        let empty_text = Paragraph::new(empty_message)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(" Results ")
                    .border_style(Style::default().fg(palette.muted)),
            )
            .style(Style::default().fg(palette.muted))
            .alignment(Alignment::Center)
            .wrap(Wrap { trim: true });

        f.render_widget(empty_text, area);
        return;
    }

    // Render result list
    let items: Vec<ListItem> = results
        .visible_results((area.height.saturating_sub(2)) as usize)
        .iter()
        .enumerate()
        .map(|(idx, result)| {
            let global_idx = idx + results.scroll_offset();
            let is_selected = global_idx == results.selected_index();

            let file_display = format!("{}:{}", result.path, result.span.start_line);
            let match_display = result.preview.trim();

            let style = if is_selected {
                Style::default()
                    .fg(Color::Black)
                    .bg(palette.highlight)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(palette.foreground)
            };

            let content = format!("{:<60} {}", file_display, match_display);
            ListItem::new(content).style(style)
        })
        .collect();

    let result_count = results.len();
    let title = format!(" Results ({}) ", result_count);

    let list = List::new(items).block(
        Block::default()
            .borders(Borders::ALL)
            .title(title)
            .border_style(Style::default().fg(palette.accent)),
    );

    f.render_widget(list, area);
}

fn render_help_screen(f: &mut Frame, area: Rect, app: &InteractiveApp) {
    let palette = &app.theme().palette;

    let help_text = vec![
        "",
        "  Reflex Interactive Mode - Keyboard Shortcuts",
        "  ═══════════════════════════════════════════════",
        "",
        "  Navigation:",
        "    j / ↓         Move to next result",
        "    k / ↑         Move to previous result",
        "    PageDown      Jump 10 results down",
        "    PageUp        Jump 10 results up",
        "    Home / g      Go to first result",
        "    End / G       Go to last result",
        "",
        "  Search:",
        "    /             Focus search input",
        "    Esc           Unfocus input / close help",
        "    Ctrl+P        Previous query from history",
        "    Ctrl+N        Next query from history",
        "",
        "  Filters:",
        "    s             Toggle symbols-only mode",
        "    r             Toggle regex mode",
        "    l             Filter by language (not yet implemented)",
        "    k             Filter by kind (not yet implemented)",
        "",
        "  Actions:",
        "    o / Enter     Open file in $EDITOR",
        "    i             Trigger reindex",
        "    ?             Toggle this help screen",
        "    q / Ctrl+C    Quit",
        "",
        "  Mouse:",
        "    Click         Select result",
        "    Scroll        Navigate results",
        "",
        "  Press '?' to close this help screen",
        "",
    ];

    let help_paragraph = Paragraph::new(help_text.join("\n"))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(" Help ")
                .border_style(Style::default().fg(palette.accent)),
        )
        .style(Style::default().fg(palette.foreground))
        .alignment(Alignment::Left)
        .wrap(Wrap { trim: false });

    f.render_widget(help_paragraph, area);
}

fn render_file_preview(f: &mut Frame, area: Rect, app: &InteractiveApp) {
    let palette = &app.theme().palette;

    if let Some(preview) = app.preview_content() {
        let visible_height = area.height.saturating_sub(2) as usize;
        let start = preview.scroll_offset();
        let center = preview.center_line();

        // Get content lines in visible range
        let content_lines = preview.content();
        let end = (start + visible_height).min(content_lines.len());

        let items: Vec<ListItem> = content_lines[start..end]
            .iter()
            .enumerate()
            .map(|(idx, line)| {
                let line_number = start + idx + 1;
                let is_center = line_number == center;

                let style = if is_center {
                    Style::default()
                        .fg(Color::Black)
                        .bg(palette.highlight)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(palette.foreground)
                };

                let content = format!("{:4} │ {}", line_number, line);
                ListItem::new(content).style(style)
            })
            .collect();

        let title = format!(" {} (line {}) ", preview.path(), center);
        let list = List::new(items).block(
            Block::default()
                .borders(Borders::ALL)
                .title(title)
                .border_style(Style::default().fg(palette.accent)),
        );

        f.render_widget(list, area);
    }
}

fn render_footer(f: &mut Frame, area: Rect, app: &InteractiveApp) {
    let palette = &app.theme().palette;

    // Build footer content based on mode
    let footer_spans = match app.mode() {
        AppMode::Help => vec![
            Span::styled("Press ", Style::default().fg(palette.muted)),
            Span::styled("?", Style::default().fg(palette.accent).add_modifier(Modifier::BOLD)),
            Span::styled(" to close help", Style::default().fg(palette.muted)),
        ],
        AppMode::FilePreview => vec![
            Span::styled(
                "[PREVIEW MODE] ",
                Style::default()
                    .fg(palette.info)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled("j/k scroll  ", Style::default().fg(palette.muted)),
            Span::styled("Esc", Style::default().fg(palette.accent).add_modifier(Modifier::BOLD)),
            Span::styled(" close  ", Style::default().fg(palette.muted)),
            Span::styled("o", Style::default().fg(palette.accent).add_modifier(Modifier::BOLD)),
            Span::styled(" open in editor", Style::default().fg(palette.muted)),
        ],
        AppMode::Indexing => vec![
            Span::styled("⏳ Indexing... ", Style::default().fg(palette.info)),
        ],
        _ => {
            let mut spans = vec![];

            // Show mode indicator based on focus state
            match app.focus_state() {
                FocusState::Input => {
                    spans.push(Span::styled(
                        "[INPUT MODE] ",
                        Style::default()
                            .fg(palette.accent)
                            .add_modifier(Modifier::BOLD),
                    ));
                }
                FocusState::Filters => {
                    spans.push(Span::styled(
                        "[FILTERS MODE] ",
                        Style::default()
                            .fg(palette.info)
                            .add_modifier(Modifier::BOLD),
                    ));
                }
                FocusState::Results => {
                    spans.push(Span::styled(
                        "[NAVIGATE MODE] ",
                        Style::default()
                            .fg(palette.success)
                            .add_modifier(Modifier::BOLD),
                    ));
                }
            }

            // Show appropriate hint based on terminal capabilities
            let hint = app.capabilities().open_hint();
            spans.push(Span::styled(hint, Style::default().fg(palette.muted)));
            spans.push(Span::raw("  "));

            // Show index status
            match app.index_status() {
                IndexStatusState::Ready { file_count, .. } => {
                    spans.push(Span::styled(
                        format!("✓ {} files", file_count),
                        Style::default().fg(palette.success),
                    ));
                }
                IndexStatusState::Missing => {
                    spans.push(Span::styled(
                        "⚠ No index (press 'i')",
                        Style::default().fg(palette.warning),
                    ));
                }
                IndexStatusState::Stale { files_changed } => {
                    spans.push(Span::styled(
                        format!("⚠ Stale ({} changed)", files_changed),
                        Style::default().fg(palette.warning),
                    ));
                }
                IndexStatusState::Indexing { current, total, .. } => {
                    spans.push(Span::styled(
                        format!("⏳ {}/{}", current, total),
                        Style::default().fg(palette.info),
                    ));
                }
            }

            spans.push(Span::raw("  "));
            spans.push(Span::styled(
                "? help",
                Style::default().fg(palette.muted),
            ));

            spans
        }
    };

    let footer = Paragraph::new(Line::from(footer_spans))
        .style(Style::default().fg(palette.foreground));

    f.render_widget(footer, area);
}
