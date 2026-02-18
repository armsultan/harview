use crate::app::{ActiveFocus, App, CookieInfo, HeaderInfo, TabBarState, TableItem};
use ratatui::{prelude::*, widgets::*};

pub fn render(app: &mut App, frame: &mut Frame) {
    let main_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Fill(1), Constraint::Fill(1)])
        .split(frame.size());

    render_table(app, main_layout[0], frame.buffer_mut());
    render_preview(app, main_layout[1], frame.buffer_mut());
}

pub fn render_table(app: &mut App, area: Rect, buf: &mut Buffer) {
    let (table_area, search_area) = if app.search_mode || app.search_active {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Fill(1), Constraint::Length(1)])
            .split(area);
        (chunks[0], Some(chunks[1]))
    } else {
        (area, None)
    };

    let table = EntriesTable::init(app);
    let mut state = TableState::default();
    table.render(table_area, buf, &mut state);

    if let Some(sb_area) = search_area {
        render_search_bar(app, sb_area, buf);
    }
}

pub fn render_preview(app: &mut App, area: Rect, buf: &mut Buffer) {
    let preview = PreviewWidget::init(app);
    preview.render(area, buf);
}

fn render_search_bar(app: &App, area: Rect, buf: &mut Buffer) {
    let scope_label = format!("[{}]", app.search_scope.display_name());
    let match_count = app.display_entry_indices.len();
    let total_count = app.table_items.len();

    let right_text = if app.search_error {
        "Invalid regex".to_string()
    } else if app.search_active || app.search_mode {
        format!("{}/{}", match_count, total_count)
    } else {
        String::new()
    };

    let right_width = right_text.len() as u16 + 1;
    let cursor = if app.search_mode { "▏" } else { "" };
    let left_width = area.width.saturating_sub(right_width);

    let right_style = if app.search_error {
        Style::default().fg(Color::LightRed)
    } else if match_count == 0 && (app.search_active || (!app.search_query.is_empty() && app.search_mode)) {
        Style::default().fg(Color::LightRed)
    } else {
        Style::default().fg(Color::LightGreen)
    };

    let query_style = if app.search_mode {
        Style::default().fg(Color::White)
    } else {
        Style::default().fg(Color::DarkGray)
    };

    let query_display: String = app
        .search_query
        .chars()
        .take(left_width.saturating_sub(scope_label.len() as u16 + 4) as usize)
        .collect();

    let line = Line::from(vec![
        Span::styled("/ ", Style::default().fg(Color::Yellow)),
        Span::styled(scope_label, Style::default().fg(Color::Yellow)),
        Span::raw(" "),
        Span::styled(format!("{}{}", query_display, cursor), query_style),
    ]);

    let left_area = Rect { x: area.x, y: area.y, width: left_width, height: 1 };
    Widget::render(Paragraph::new(line), left_area, buf);

    if !right_text.is_empty() {
        let right_area = Rect {
            x: area.x + left_width,
            y: area.y,
            width: right_width,
            height: 1,
        };
        Widget::render(
            Paragraph::new(Span::styled(right_text, right_style)).alignment(Alignment::Right),
            right_area,
            buf,
        );
    }
}

// ── Highlight helper ─────────────────────────────────────────────────────────

/// Rebuild a line with regex match positions highlighted (yellow bg, black fg).
/// Always returns `Line<'static>` so it's safe to compose with any lifetime.
fn highlight_line_matches(line: Line<'_>, re: &regex::Regex) -> Line<'static> {
    let base_style = line.style;
    let hl = Style::default().bg(Color::Yellow).fg(Color::Black).bold();
    let mut new_spans: Vec<Span<'static>> = Vec::new();

    for span in line.spans {
        let style = span.style;
        let text = span.content.as_ref().to_string();
        let mut last = 0;

        for m in re.find_iter(&text) {
            if m.start() > last {
                new_spans.push(Span::styled(text[last..m.start()].to_string(), style));
            }
            new_spans.push(Span::styled(
                text[m.start()..m.end()].to_string(),
                style.patch(hl),
            ));
            last = m.end();
        }

        // Remaining text after last match (or full text if no matches).
        if last < text.len() {
            new_spans.push(Span::styled(text[last..].to_string(), style));
        }
    }

    Line::from(new_spans).style(base_style)
}

fn apply_highlights<'a>(
    lines: impl Iterator<Item = Line<'a>>,
    re_opt: Option<&regex::Regex>,
) -> Vec<Line<'static>> {
    match re_opt {
        Some(re) => lines.map(|l| highlight_line_matches(l, re)).collect(),
        None => lines.map(line_to_static).collect(),
    }
}

/// Convert a line with any lifetime to `Line<'static>` by making all span content owned.
fn line_to_static(line: Line<'_>) -> Line<'static> {
    let style = line.style;
    let spans: Vec<Span<'static>> = line
        .spans
        .into_iter()
        .map(|s| Span::styled(s.content.into_owned(), s.style))
        .collect();
    Line::from(spans).style(style)
}

// ── EntriesTable ─────────────────────────────────────────────────────────────

#[derive(Debug)]
pub struct EntriesTable<'a> {
    display_items: Vec<&'a TableItem>,
    active_focus: ActiveFocus,
    table_offset: usize,
    selected_index: usize,
    search_active: bool,
    match_count: usize,
    total_count: usize,
    search_regex: Option<regex::Regex>,
}

impl<'a> EntriesTable<'a> {
    pub fn init(app: &'a App) -> Self {
        let display_items = app
            .display_entry_indices
            .iter()
            .map(|&i| &app.table_items[i])
            .collect();

        Self {
            display_items,
            active_focus: app.active_focus,
            table_offset: app.table_offset,
            selected_index: app.get_index(),
            search_active: app.search_active,
            match_count: app.display_entry_indices.len(),
            total_count: app.table_items.len(),
            search_regex: app.search_regex.clone(),
        }
    }
}

impl<'a> StatefulWidget for EntriesTable<'a> {
    type State = TableState;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        let list_height = (area.height as usize).saturating_sub(3);
        let start_index = self.table_offset;
        let end_index = (start_index + list_height).min(self.display_items.len());

        let visible_items: &[&TableItem] = if start_index < self.display_items.len() {
            &self.display_items[start_index..end_index]
        } else {
            &[]
        };

        let headers = Row::new(vec![
            Cell::from("Status"),
            Cell::from("Method"),
            Cell::from("URL"),
            Cell::from("ContentType"),
            Cell::from("     Size  "),
            Cell::from("Timestamp"),
        ])
        .style(Style::default().bold().underlined());

        let widths = [
            Constraint::Length(6),
            Constraint::Length(7),
            Constraint::Fill(1),
            Constraint::Length(20),
            Constraint::Length(10),
            Constraint::Length(14),
        ];

        let re_opt = self.search_regex.as_ref();
        let rows: Vec<Row> = visible_items.iter().map(|item| make_row(item, re_opt)).collect();

        if self.selected_index >= start_index && self.selected_index < end_index {
            state.select(Some(self.selected_index - start_index));
        } else {
            state.select(None);
        }
        *state.offset_mut() = 0;

        let title = if self.search_active {
            format!(" {}/{} matches ", self.match_count, self.total_count)
        } else {
            String::new()
        };

        let table = Table::new(rows, &widths)
            .header(headers)
            .highlight_style(Style::default().reversed())
            .block(
                Block::default()
                    .padding(Padding::horizontal(1))
                    .borders(Borders::ALL)
                    .title(title)
                    .title_style(Style::default().fg(Color::LightGreen))
                    .border_style(if self.active_focus == ActiveFocus::Table {
                        Style::default().fg(Color::Green)
                    } else {
                        Style::default().fg(Color::DarkGray)
                    }),
            );

        StatefulWidget::render(table, area, buf, state);
    }
}

// ── PreviewWidget ─────────────────────────────────────────────────────────────

#[derive(Debug)]
pub struct PreviewWidget<'a> {
    tabbar_state: TabBarState,
    app: &'a App,
}

impl<'a> PreviewWidget<'a> {
    pub fn init(app: &'a App) -> Self {
        Self {
            tabbar_state: app.tabbar_state.clone(),
            app,
        }
    }
}

impl<'a> Widget for PreviewWidget<'a> {
    fn render(self, area: Rect, buf: &mut Buffer)
    where
        Self: Sized,
    {
        let layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(1), Constraint::Fill(1)])
            .split(area);

        let tab_row = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Fill(1), Constraint::Length(12)])
            .split(layout[0]);

        let main_tabs = Tabs::new(vec![
            " [1] Headers ",
            " [2] Cookies ",
            " [3] Request ",
            " [4] Response ",
        ])
        .select(if self.tabbar_state == TabBarState::Help {
            usize::MAX
        } else {
            self.tabbar_state.to_index()
        })
        .padding(" ", " ");

        Widget::render(main_tabs, tab_row[0], buf);

        let help_style = if self.tabbar_state == TabBarState::Help {
            Style::default().reversed()
        } else {
            Style::default()
        };
        let help_label = Paragraph::new(Span::styled(" [?] Help ", help_style))
            .alignment(Alignment::Right);
        Widget::render(help_label, tab_row[1], buf);

        match self.tabbar_state {
            TabBarState::Headers => HeaderPreview::init(self.app).render(layout[1], buf),
            TabBarState::Cookies => CookiePreview::init(self.app).render(layout[1], buf),
            TabBarState::Request => RequestPreview::init(self.app).render(layout[1], buf),
            TabBarState::Response => ResponsePreview::init(self.app).render(layout[1], buf),
            TabBarState::Help => HelpPreview::init(self.app).render(layout[1], buf),
        }
    }
}

// ── HeaderPreview ─────────────────────────────────────────────────────────────

struct HeaderPreview {
    header_info: Option<HeaderInfo>,
    scroll: u16,
    active_focus: ActiveFocus,
    search_regex: Option<regex::Regex>,
}

impl HeaderPreview {
    pub fn init(app: &App) -> Self {
        Self {
            header_info: app.to_header_info(app.get_entry_index()),
            scroll: app.scroll,
            active_focus: app.active_focus,
            search_regex: app.search_regex.clone(),
        }
    }
}

impl Widget for HeaderPreview {
    fn render(self, area: Rect, buf: &mut Buffer)
    where
        Self: Sized,
    {
        if let Some(header_info) = self.header_info {
            let raw_lines: Vec<Line<'static>> = {
                let mut v: Vec<Line<'static>> = vec![
                    Line::from(vec![Span::styled(
                        "General",
                        Style::default().bold().underlined(),
                    )]),
                    Line::from(vec![
                        Span::raw("Request URL: "),
                        Span::styled(header_info.url.clone(), Style::default().fg(Color::Cyan)),
                    ]),
                    Line::from(vec![
                        Span::raw("Request Method: "),
                        Span::styled(header_info.method.clone(), Style::default().fg(Color::Yellow)),
                    ]),
                    Line::from(vec![
                        Span::raw("Status Code: "),
                        Span::styled(
                            header_info.status.to_string(),
                            Style::default().fg(Color::Green),
                        ),
                    ]),
                    Line::raw(""),
                    Line::from(vec![Span::styled(
                        "Request Headers",
                        Style::default().bold().underlined(),
                    )]),
                ];
                for (name, value) in &header_info.req_headers {
                    v.push(Line::from(vec![
                        Span::styled(format!("{}: ", name), Style::default().fg(Color::Blue)),
                        Span::raw(value.clone()),
                    ]));
                }
                v.push(Line::raw(""));
                v.push(Line::from(vec![Span::styled(
                    "Response Headers",
                    Style::default().bold().underlined(),
                )]));
                for (name, value) in &header_info.resp_headers {
                    v.push(Line::from(vec![
                        Span::styled(format!("{}: ", name), Style::default().fg(Color::Blue)),
                        Span::raw(value.clone()),
                    ]));
                }
                v
            };

            let lines = apply_highlights(raw_lines.into_iter(), self.search_regex.as_ref());

            let paragraph = Paragraph::new(lines)
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .title("Headers")
                        .border_style(if self.active_focus == ActiveFocus::Preview {
                            Style::default().fg(Color::Green)
                        } else {
                            Style::default().fg(Color::DarkGray)
                        }),
                )
                .wrap(Wrap { trim: false })
                .scroll((self.scroll, 0));

            Widget::render(paragraph, area, buf);
        }
    }
}

// ── CookiePreview ─────────────────────────────────────────────────────────────

pub struct CookiePreview {
    cookie_info: Option<CookieInfo>,
    scroll: u16,
    active_focus: ActiveFocus,
    search_regex: Option<regex::Regex>,
}

impl CookiePreview {
    pub fn init(app: &App) -> Self {
        Self {
            cookie_info: app.to_cookie_info(app.get_entry_index()),
            scroll: app.scroll,
            active_focus: app.active_focus,
            search_regex: app.search_regex.clone(),
        }
    }
}

impl Widget for CookiePreview {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if let Some(cookie_info) = self.cookie_info {
            let raw_lines: Vec<Line<'static>> = {
                let mut v: Vec<Line<'static>> = vec![Line::from(vec![Span::styled(
                    "Request Cookies",
                    Style::default().bold().underlined(),
                )])];
                if cookie_info.req_cookies.is_empty() {
                    v.push(Line::raw("No request cookies"));
                }
                for (name, value) in &cookie_info.req_cookies {
                    v.push(Line::from(vec![
                        Span::styled(format!("{}: ", name), Style::default().fg(Color::Blue)),
                        Span::raw(value.clone()),
                    ]));
                }
                v.push(Line::raw(""));
                v.push(Line::from(vec![Span::styled(
                    "Response Cookies",
                    Style::default().bold().underlined(),
                )]));
                if cookie_info.resp_cookies.is_empty() {
                    v.push(Line::raw("No response cookies"));
                }
                for (name, value) in &cookie_info.resp_cookies {
                    v.push(Line::from(vec![
                        Span::styled(format!("{}: ", name), Style::default().fg(Color::Blue)),
                        Span::raw(value.clone()),
                    ]));
                }
                v
            };

            let lines = apply_highlights(raw_lines.into_iter(), self.search_regex.as_ref());

            let paragraph = Paragraph::new(lines)
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .title("Cookies")
                        .border_style(if self.active_focus == ActiveFocus::Preview {
                            Style::default().fg(Color::Green)
                        } else {
                            Style::default().fg(Color::DarkGray)
                        }),
                )
                .wrap(Wrap { trim: false })
                .scroll((self.scroll, 0));

            Widget::render(paragraph, area, buf);
        }
    }
}

// ── RequestPreview ────────────────────────────────────────────────────────────

pub struct RequestPreview<'a> {
    app: &'a App,
    scroll: u16,
    active_focus: ActiveFocus,
}

impl<'a> RequestPreview<'a> {
    pub fn init(app: &'a App) -> Self {
        Self {
            app,
            scroll: app.scroll,
            active_focus: app.active_focus,
        }
    }
}

impl<'a> Widget for RequestPreview<'a> {
    fn render(self, area: Rect, buf: &mut Buffer)
    where
        Self: Sized,
    {
        let text = if let Some(cached) = &self.app.cached_preview_text {
            let start = self.scroll as usize;
            let height = area.height as usize;
            if start >= cached.lines.len() {
                Text::default()
            } else {
                let re_opt = self.app.search_regex.as_ref();
                let lines: Vec<Line<'static>> = cached
                    .lines
                    .iter()
                    .skip(start)
                    .take(height)
                    .map(|line| truncate_line(line, 2000))
                    .map(|line| match re_opt {
                        Some(re) => highlight_line_matches(line, re),
                        None => line,
                    })
                    .collect();
                Text::from(lines)
            }
        } else {
            Text::raw("Loading or No Body...")
        };

        let mut paragraph = Paragraph::new(text)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("Request Body")
                    .border_style(if self.active_focus == ActiveFocus::Preview {
                        Style::default().fg(Color::Green)
                    } else {
                        Style::default().fg(Color::DarkGray)
                    }),
            )
            .scroll((0, 0));

        if self.app.enable_syntax_highlighting {
            paragraph = paragraph.wrap(Wrap { trim: false });
        }

        Widget::render(paragraph, area, buf);
    }
}

// ── ResponsePreview ───────────────────────────────────────────────────────────

pub struct ResponsePreview<'a> {
    app: &'a App,
    scroll: u16,
    active_focus: ActiveFocus,
    was_base64_decoded: bool,
}

impl<'a> ResponsePreview<'a> {
    pub fn init(app: &'a App) -> Self {
        let was_base64_decoded = app
            .har
            .log
            .entries
            .get(app.get_entry_index())
            .and_then(|e| e.response.content.encoding.as_deref())
            .is_some_and(|enc| enc == "base64");

        Self {
            app,
            scroll: app.scroll,
            active_focus: app.active_focus,
            was_base64_decoded,
        }
    }
}

impl<'a> Widget for ResponsePreview<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let text = if let Some(cached) = &self.app.cached_preview_text {
            let start = self.scroll as usize;
            let height = area.height as usize;
            if start >= cached.lines.len() {
                Text::default()
            } else {
                let re_opt = self.app.search_regex.as_ref();
                let lines: Vec<Line<'static>> = cached
                    .lines
                    .iter()
                    .skip(start)
                    .take(height)
                    .map(|line| truncate_line(line, 2000))
                    .map(|line| match re_opt {
                        Some(re) => highlight_line_matches(line, re),
                        None => line,
                    })
                    .collect();
                Text::from(lines)
            }
        } else {
            Text::raw("Loading or No Response Body...")
        };

        let title = if self.was_base64_decoded {
            "Response Body (base64 decoded)"
        } else {
            "Response Body"
        };

        let mut paragraph = Paragraph::new(text)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(title)
                    .border_style(if self.active_focus == ActiveFocus::Preview {
                        Style::default().fg(Color::Green)
                    } else {
                        Style::default().fg(Color::DarkGray)
                    }),
            )
            .scroll((0, 0));

        if self.app.enable_syntax_highlighting {
            paragraph = paragraph.wrap(Wrap { trim: false });
        }
        Widget::render(paragraph, area, buf);
    }
}

// ── HelpPreview ───────────────────────────────────────────────────────────────

pub struct HelpPreview {
    scroll: u16,
    active_focus: ActiveFocus,
}

impl HelpPreview {
    pub fn init(app: &App) -> Self {
        Self {
            scroll: app.scroll,
            active_focus: app.active_focus,
        }
    }
}

impl Widget for HelpPreview {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let bold_underline = Style::default().bold().underlined();
        let key_style = Style::default().fg(Color::Yellow);
        let dim = Style::default().fg(Color::DarkGray);

        let lines = vec![
            Line::from(Span::styled("Navigation", bold_underline)),
            Line::from(vec![
                Span::styled("  j / Down      ", key_style),
                Span::raw("Move selection down"),
            ]),
            Line::from(vec![
                Span::styled("  k / Up        ", key_style),
                Span::raw("Move selection up"),
            ]),
            Line::from(vec![
                Span::styled("  d             ", key_style),
                Span::raw("Move down by 3"),
            ]),
            Line::from(vec![
                Span::styled("  u             ", key_style),
                Span::raw("Move up by 3"),
            ]),
            Line::from(vec![
                Span::styled("  g             ", key_style),
                Span::raw("Jump to first entry"),
            ]),
            Line::from(vec![
                Span::styled("  G             ", key_style),
                Span::raw("Jump to last entry"),
            ]),
            Line::from(""),
            Line::from(Span::styled("Search / Filter", bold_underline)),
            Line::from(vec![
                Span::styled("  /             ", key_style),
                Span::raw("Enter search mode (supports regex)"),
            ]),
            Line::from(vec![
                Span::styled("  Tab           ", key_style),
                Span::raw("Cycle search scope (ALL/URL/Host/QueryStr/…)"),
            ]),
            Line::from(vec![
                Span::styled("  Enter         ", key_style),
                Span::raw("Confirm filter"),
            ]),
            Line::from(vec![
                Span::styled("  Esc           ", key_style),
                Span::raw("Cancel search (restore) / clear active filter"),
            ]),
            Line::from(""),
            Line::from(Span::styled("Details Pane Scrolling", bold_underline)),
            Line::from(vec![
                Span::styled("  Shift+Up      ", key_style),
                Span::raw("Scroll up by 1 line"),
            ]),
            Line::from(vec![
                Span::styled("  Shift+Down    ", key_style),
                Span::raw("Scroll down by 1 line"),
            ]),
            Line::from(vec![
                Span::styled("  PageUp        ", key_style),
                Span::raw("Scroll up by 10 lines"),
            ]),
            Line::from(vec![
                Span::styled("  PageDown      ", key_style),
                Span::raw("Scroll down by 10 lines"),
            ]),
            Line::from(""),
            Line::from(Span::styled("Tabs", bold_underline)),
            Line::from(vec![
                Span::styled("  1-4           ", key_style),
                Span::raw("Switch to tab (Headers, Cookies, Request, Response)"),
            ]),
            Line::from(vec![
                Span::styled("  Left / Right  ", key_style),
                Span::raw("Cycle through tabs"),
            ]),
            Line::from(vec![
                Span::styled("  ?             ", key_style),
                Span::raw("Show this help"),
            ]),
            Line::from(""),
            Line::from(Span::styled("Display", bold_underline)),
            Line::from(vec![
                Span::styled("  h             ", key_style),
                Span::raw("Toggle syntax highlighting"),
            ]),
            Line::from(""),
            Line::from(Span::styled(
                "External Viewers (Request/Response tabs)",
                bold_underline,
            )),
            Line::from(vec![
                Span::styled("  b             ", key_style),
                Span::raw("Open body in bat"),
            ]),
            Line::from(vec![
                Span::styled("  J             ", key_style),
                Span::raw("Open JSON in fx"),
            ]),
            Line::from(vec![
                Span::styled("  o             ", key_style),
                Span::raw("Open body in $EDITOR"),
            ]),
            Line::from(""),
            Line::from(Span::styled("Mouse", bold_underline)),
            Line::from(vec![
                Span::styled("  Scroll        ", key_style),
                Span::raw("Navigate entries or scroll details"),
            ]),
            Line::from(vec![
                Span::styled("  Click row     ", key_style),
                Span::raw("Select entry"),
            ]),
            Line::from(vec![
                Span::styled("  Click tab     ", key_style),
                Span::raw("Switch tab"),
            ]),
            Line::from(""),
            Line::from(Span::styled("General", bold_underline)),
            Line::from(vec![
                Span::styled("  q / Ctrl+C    ", key_style),
                Span::raw("Quit"),
            ]),
            Line::from(""),
            Line::from(Span::styled(
                "  Base64-encoded responses are automatically decoded.",
                dim,
            )),
        ];

        let paragraph = Paragraph::new(lines)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("Help")
                    .border_style(if self.active_focus == ActiveFocus::Preview {
                        Style::default().fg(Color::Green)
                    } else {
                        Style::default().fg(Color::DarkGray)
                    }),
            )
            .wrap(Wrap { trim: false })
            .scroll((self.scroll, 0));

        Widget::render(paragraph, area, buf);
    }
}

// ── Helpers ───────────────────────────────────────────────────────────────────

/// Build a table row for `item`, highlighting any regex matches in each cell.
fn make_row(item: &TableItem, re: Option<&regex::Regex>) -> Row<'static> {
    let status_style = match item.status {
        100..=199 => Style::default().fg(Color::LightBlue),
        200..=299 => Style::default().fg(Color::LightGreen),
        300..=399 => Style::default().fg(Color::LightCyan),
        400..=499 => Style::default().fg(Color::LightYellow),
        500..=599 => Style::default().fg(Color::LightMagenta),
        _ => Style::default().fg(Color::DarkGray),
    };
    Row::new(vec![
        hl_cell(&item.status.to_string(), status_style, re),
        hl_cell(&item.method, Style::default().fg(Color::Yellow), re),
        hl_cell(&item.url, Style::default().fg(Color::LightBlue), re),
        hl_cell(&item.mime_type, Style::default().fg(Color::Magenta), re),
        hl_cell(&item.total_size, Style::default().fg(Color::LightCyan), re),
        hl_cell(&item.timestamp, Style::default(), re),
    ])
}

/// Build a single `Cell` whose text has regex matches highlighted.
fn hl_cell(text: &str, base_style: Style, re: Option<&regex::Regex>) -> Cell<'static> {
    let line = Line::from(Span::styled(text.to_string(), base_style));
    let line = match re {
        Some(re) => highlight_line_matches(line, re),
        None => line_to_static(line),
    };
    Cell::from(Text::from(line))
}

/// Truncate a line to `max_width` characters, returning an owned `Line<'static>`.
fn truncate_line(line: &Line<'_>, max_width: usize) -> Line<'static> {
    let mut current_width = 0;
    let mut new_spans: Vec<Span<'static>> = Vec::new();

    for span in &line.spans {
        let content = span.content.as_ref();
        let remaining = max_width.saturating_sub(current_width);

        if remaining == 0 {
            break;
        }

        if content.len() <= remaining {
            new_spans.push(Span::styled(content.to_string(), span.style));
            current_width += content.len();
        } else {
            new_spans.push(Span::styled(content[..remaining].to_string(), span.style));
            break;
        }
    }

    Line::from(new_spans).style(line.style)
}
