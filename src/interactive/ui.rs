use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState, Wrap},
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
        AppMode::Indexing | AppMode::Normal => render_results_area(f, chunks[2], app),
    }

    render_footer(f, chunks[3], app);
}

fn render_header(f: &mut Frame, area: Rect, app: &InteractiveApp) {
    let palette = &app.theme().palette;
    let input_focused = matches!(app.focus_state(), FocusState::Input);

    // Build title line with status on the right
    let title_left = if input_focused {
        " Search [TYPING - Press Tab/Enter to navigate] "
    } else {
        " Search [Press Tab to focus, / to type] "
    };

    // Build status indicator for title right
    let status_indicator = match app.index_status() {
        IndexStatusState::Ready { file_count, .. } => {
            format!("✓ {} files ", file_count)
        }
        IndexStatusState::Missing => {
            "⚠ No index ".to_string()
        }
        IndexStatusState::Stale { files_changed } => {
            format!("⚠ {} changed ", files_changed)
        }
        IndexStatusState::Indexing { current, total, .. } => {
            format!("⏳ {}/{} ", current, total)
        }
    };

    // Calculate spacing to push status to the right
    let available_width = area.width.saturating_sub(2) as usize; // Subtract borders
    let title_len = title_left.chars().count();
    let status_len = status_indicator.chars().count();
    let spaces_needed = available_width.saturating_sub(title_len + status_len);
    let spacing = " ".repeat(spaces_needed);

    // Build complete title line
    let mut title_spans = vec![
        Span::raw(title_left),
        Span::raw(spacing),
        Span::styled(
            status_indicator,
            Style::default().fg(palette.muted)
        ),
    ];

    let input_block = Block::default()
        .borders(Borders::ALL)
        .title_top(Line::from(title_spans))
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

    // Show animated indexing modal (takes priority over search)
    if app.indexing() {
        // Create a centered modal for the indexing animation
        let modal_width = 50.min(area.width.saturating_sub(4));
        let modal_height = 9;
        let modal_x = (area.width.saturating_sub(modal_width)) / 2;
        let modal_y = (area.height.saturating_sub(modal_height)) / 2;
        let modal_area = Rect::new(
            area.x + modal_x,
            area.y + modal_y,
            modal_width,
            modal_height,
        );

        // Render background (dimmed results area)
        let background = Block::default()
            .borders(Borders::ALL)
            .title(" Results ")
            .border_style(Style::default().fg(palette.muted));
        f.render_widget(background, area);

        // Animate the spinner character based on frame count
        let spinner_frames = ['◐', '◓', '◑', '◒'];
        let frame_idx = (app.effects().frame() / 3) as usize % spinner_frames.len();
        let spinner = spinner_frames[frame_idx];

        // Create animated loading text with multiple lines
        let loading_lines = vec![
            Line::from(vec![
                Span::raw("  "),
                Span::styled(
                    spinner.to_string(),
                    Style::default()
                        .fg(Color::Rgb(255, 150, 0))
                        .add_modifier(Modifier::BOLD)
                ),
                Span::raw("  "),
            ]),
            Line::from(""),
            Line::from(vec![
                Span::styled(
                    "Building index",
                    Style::default()
                        .fg(palette.accent)
                        .add_modifier(Modifier::BOLD),
                ),
            ]),
            Line::from(""),
            Line::from(vec![
                Span::styled("━━━━━━━━━━━━━━━━", Style::default().fg(palette.warning)),
            ]),
            Line::from(""),
            Line::from(vec![
                Span::styled(
                    "This may take a moment...",
                    Style::default().fg(palette.muted),
                ),
            ]),
        ];

        // Render modal with animated border
        let modal = Paragraph::new(loading_lines)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(
                        Style::default()
                            .fg(Color::Rgb(255, 150, 0))
                            .add_modifier(Modifier::BOLD),
                    )
                    .title(Line::from(vec![
                        Span::raw(" "),
                        Span::styled("📦", Style::default().fg(Color::Rgb(255, 200, 0))),
                        Span::raw(" Indexing "),
                        Span::styled("📦", Style::default().fg(Color::Rgb(255, 200, 0))),
                        Span::raw(" "),
                    ]))
                    .style(Style::default().bg(Color::Rgb(25, 20, 30))),
            )
            .alignment(Alignment::Center);

        f.render_widget(modal, modal_area);
        return;
    }

    // Show animated loading modal if query is executing
    if app.searching() {
        // Create a centered modal for the loading animation
        let modal_width = 50.min(area.width.saturating_sub(4));
        let modal_height = 9;
        let modal_x = (area.width.saturating_sub(modal_width)) / 2;
        let modal_y = (area.height.saturating_sub(modal_height)) / 2;
        let modal_area = Rect::new(
            area.x + modal_x,
            area.y + modal_y,
            modal_width,
            modal_height,
        );

        // Render background (dimmed results area)
        let background = Block::default()
            .borders(Borders::ALL)
            .title(" Results ")
            .border_style(Style::default().fg(palette.muted));
        f.render_widget(background, area);

        // Animate the spinner character based on frame count
        let spinner_frames = ['◐', '◓', '◑', '◒'];
        let frame_idx = (app.effects().frame() / 3) as usize % spinner_frames.len();
        let spinner = spinner_frames[frame_idx];

        // Create animated loading text with multiple lines
        let loading_lines = vec![
            Line::from(vec![
                Span::raw("  "),
                Span::styled(
                    spinner.to_string(),
                    Style::default()
                        .fg(Color::Rgb(0, 200, 255))
                        .add_modifier(Modifier::BOLD)
                ),
                Span::raw("  "),
            ]),
            Line::from(""),
            Line::from(vec![
                Span::styled(
                    "Searching codebase",
                    Style::default()
                        .fg(palette.accent)
                        .add_modifier(Modifier::BOLD),
                ),
            ]),
            Line::from(""),
            Line::from(vec![
                Span::styled("━━━━━━━━━━━━━━━━", Style::default().fg(palette.info)),
            ]),
            Line::from(""),
            Line::from(vec![
                Span::styled(
                    "Hang tight...",
                    Style::default().fg(palette.muted),
                ),
            ]),
        ];

        // Render modal with animated border
        let modal = Paragraph::new(loading_lines)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(
                        Style::default()
                            .fg(Color::Rgb(0, 200, 255))
                            .add_modifier(Modifier::BOLD),
                    )
                    .title(Line::from(vec![
                        Span::raw(" "),
                        Span::styled("⚡", Style::default().fg(Color::Rgb(255, 200, 0))),
                        Span::raw(" Loading "),
                        Span::styled("⚡", Style::default().fg(Color::Rgb(255, 200, 0))),
                        Span::raw(" "),
                    ]))
                    .style(Style::default().bg(Color::Rgb(20, 20, 30))),
            )
            .alignment(Alignment::Center);

        f.render_widget(modal, modal_area);
        return;
    }

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

    // Render scrollbar if there are more results than visible
    let visible_height = area.height.saturating_sub(2) as usize;
    if result_count > visible_height {
        let mut scrollbar_state = ScrollbarState::new(result_count)
            .position(results.selected_index());

        let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
            .begin_symbol(Some("▲"))
            .end_symbol(Some("▼"))
            .track_symbol(Some("│"))
            .thumb_symbol("█")
            .style(Style::default().fg(palette.accent));

        f.render_stateful_widget(
            scrollbar,
            area.inner(ratatui::layout::Margin { horizontal: 0, vertical: 1 }),
            &mut scrollbar_state,
        );
    }
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
        AppMode::Indexing | AppMode::Normal => {
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
