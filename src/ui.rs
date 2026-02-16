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
    let table = EntriesTable::init(app);
    let mut state = TableState::default();
    table.render(area, buf, &mut state);
}

pub fn render_preview(app: &mut App, area: Rect, buf: &mut Buffer) {
    let preview = PreviewWidget::init(app);
    preview.render(area, buf);
}

#[derive(Debug)]
pub struct EntriesTable<'a> {
    table_items: &'a [TableItem],
    active_focus: ActiveFocus,
    table_offset: usize,
    selected_index: usize,
}

impl<'a> EntriesTable<'a> {
    pub fn init(app: &'a App) -> Self {
        Self {
            table_items: &app.table_items,
            active_focus: app.active_focus,
            table_offset: app.table_offset,
            selected_index: app.get_index(),
        }
    }
}

impl<'a> StatefulWidget for EntriesTable<'a> {
    type State = TableState;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        // Calculate visible items
        // Table block has borders (2) + header (1) = 3 lines overhead
        let list_height = (area.height as usize).saturating_sub(3);
        
        let start_index = self.table_offset;
        let end_index = (start_index + list_height).min(self.table_items.len());
        
        let visible_items = if start_index < self.table_items.len() {
            &self.table_items[start_index..end_index]
        } else {
            &[]
        };

        let headers = Row::new(vec![
            Cell::from("Status"),
            Cell::from("Method"),
            Cell::from("Domain"),
            Cell::from("FileName"),
            Cell::from("ContentType"),
            Cell::from("     Size  "),
            Cell::from("Timestamp"),
        ])
        .style(Style::default().bold().underlined());

        let widths = [
            Constraint::Length(6),
            Constraint::Length(7),
            Constraint::Fill(3),
            Constraint::Fill(1),
            Constraint::Length(20),
            Constraint::Length(10),
            Constraint::Length(14),
        ];

        let rows: Vec<Row> = visible_items
            .iter()
            .map(|item| item.to_table_row())
            .collect();
        
        // Adjust selection to be relative to the sliced view
        if self.selected_index >= start_index && self.selected_index < end_index {
            state.select(Some(self.selected_index - start_index));
        } else {
            state.select(None); 
        }
        *state.offset_mut() = 0; // We are handling offset manually by slicing

        let table = Table::new(rows, &widths)
            .header(headers)
            .highlight_style(Style::default().reversed())
            .block(
                Block::default()
                    .padding(Padding::horizontal(1))
                    .borders(Borders::ALL)
                    .border_style(if self.active_focus == ActiveFocus::Table {
                        Style::default().fg(Color::Green)
                    } else {
                        Style::default().fg(Color::DarkGray)
                    }),
            );

        StatefulWidget::render(table, area, buf, state);
    }
}

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

        // Split tab bar row: main tabs left, help tab right
        let tab_row = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Fill(1), Constraint::Length(12)])
            .split(layout[0]);

        // Main tabs (1-4) — only highlight if current tab is one of these
        let main_tabs = Tabs::new(vec![
            " [1] Headers ",
            " [2] Cookies ",
            " [3] Request ",
            " [4] Response ",
        ])
        .select(if self.tabbar_state == TabBarState::Help {
            usize::MAX // nothing selected
        } else {
            self.tabbar_state.to_index()
        })
        .padding(" ", " ");

        Widget::render(main_tabs, tab_row[0], buf);

        // Help tab — right-aligned, highlighted when active
        let help_style = if self.tabbar_state == TabBarState::Help {
            Style::default().reversed()
        } else {
            Style::default()
        };
        let help_label = Paragraph::new(Span::styled(" [?] Help ", help_style))
            .alignment(Alignment::Right);
        Widget::render(help_label, tab_row[1], buf);

        match self.tabbar_state {
            TabBarState::Headers => {
                let header_preview = HeaderPreview::init(self.app);
                header_preview.render(layout[1], buf);
            }
            TabBarState::Cookies => {
                let cookie_preview = CookiePreview::init(self.app);
                cookie_preview.render(layout[1], buf);
            }
            TabBarState::Request => {
                let request_preview = RequestPreview::init(self.app);
                request_preview.render(layout[1], buf);
            }
            TabBarState::Response => {
                let response_preview = ResponsePreview::init(self.app);
                response_preview.render(layout[1], buf);
            }
            TabBarState::Help => {
                let help_preview = HelpPreview::init(self.app);
                help_preview.render(layout[1], buf);
            }
        }
    }
}

#[derive(Debug)]
struct HeaderPreview {
    header_info: Option<HeaderInfo>,
    scroll: u16,
    active_focus: ActiveFocus,
}

impl HeaderPreview {
    pub fn init(app: &App) -> Self {
        Self {
            header_info: app.to_header_info(app.get_index()),
            scroll: app.scroll,
            active_focus: app.active_focus,
        }
    }
}

impl Widget for HeaderPreview {
    fn render(self, area: Rect, buf: &mut Buffer)
    where
        Self: Sized,
    {
        if let Some(header_info) = self.header_info {
            let mut lines = vec![
                // General Info
                Line::from(vec![Span::styled(
                    "General",
                    Style::default().bold().underlined(),
                )]),
                Line::from(vec![
                    Span::raw("Request URL: "),
                    Span::styled(
                        header_info.url.to_string(),
                        Style::default().fg(Color::Cyan),
                    ),
                ]),
                Line::from(vec![
                    Span::raw("Request Method: "),
                    Span::styled(
                        header_info.method.clone(),
                        Style::default().fg(Color::Yellow),
                    ),
                ]),
                Line::from(vec![
                    Span::raw("Status Code: "),
                    Span::styled(
                        header_info.status.to_string(),
                        Style::default().fg(Color::Green),
                    ),
                ]),
                Line::from(""),
            ];

            // Request Headers
            lines.push(Line::from(vec![Span::styled(
                "Request Headers",
                Style::default().bold().underlined(),
            )]));
            for (name, value) in header_info.req_headers {
                lines.push(Line::from(vec![
                    Span::styled(format!("{}: ", name), Style::default().fg(Color::Blue)),
                    Span::raw(value),
                ]));
            }
            lines.push(Line::from(""));

            // Response Headers
            lines.push(Line::from(vec![Span::styled(
                "Response Headers",
                Style::default().bold().underlined(),
            )]));
            for (name, value) in header_info.resp_headers {
                lines.push(Line::from(vec![
                    Span::styled(format!("{}: ", name), Style::default().fg(Color::Blue)),
                    Span::raw(value),
                ]));
            }

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

pub struct CookiePreview {
    cookie_info: Option<CookieInfo>,
    scroll: u16,
    active_focus: ActiveFocus,
}

impl CookiePreview {
    pub fn init(app: &App) -> Self {
        Self {
            cookie_info: app.to_cookie_info(app.get_index()),
            scroll: app.scroll,
            active_focus: app.active_focus,
        }
    }
}

impl Widget for CookiePreview {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if let Some(cookie_info) = self.cookie_info {
            let mut lines = vec![];

            // Request Cookies
            lines.push(Line::from(vec![Span::styled(
                "Request Cookies",
                Style::default().bold().underlined(),
            )]));
            if cookie_info.req_cookies.is_empty() {
                lines.push(Line::from("No request cookies"));
            }
            for (name, value) in cookie_info.req_cookies {
                lines.push(Line::from(vec![
                    Span::styled(format!("{}: ", name), Style::default().fg(Color::Blue)),
                    Span::raw(value),
                ]));
            }
            lines.push(Line::from(""));

            // Response Cookies
            lines.push(Line::from(vec![Span::styled(
                "Response Cookies",
                Style::default().bold().underlined(),
            )]));
            if cookie_info.resp_cookies.is_empty() {
                lines.push(Line::from("No response cookies"));
            }
            for (name, value) in cookie_info.resp_cookies {
                lines.push(Line::from(vec![
                    Span::styled(format!("{}: ", name), Style::default().fg(Color::Blue)),
                    Span::raw(value),
                ]));
            }

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
        // View Slicing Optimization:
        // Instead of cloning the entire text, we only clone the lines that are currently visible.
        let text = if let Some(cached) = &self.app.cached_preview_text {
            let start = self.scroll as usize;
            let height = area.height as usize;
            if start >= cached.lines.len() {
                Text::default()
            } else {
                let lines: Vec<Line> = cached.lines
                    .iter()
                    .skip(start)
                    .take(height)
                    .map(|line| truncate_line(line, 2000))
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
            .scroll((0, 0)); // We handled scrolling manually

        if self.app.enable_syntax_highlighting {
            paragraph = paragraph.wrap(Wrap { trim: false });
        }
        
        Widget::render(paragraph, area, buf);
    }

}

pub struct ResponsePreview<'a> {
    app: &'a App,
    scroll: u16,
    active_focus: ActiveFocus,
    was_base64_decoded: bool,
}

impl<'a> ResponsePreview<'a> {
    pub fn init(app: &'a App) -> Self {
        let was_base64_decoded = app.har.log.entries
            .get(app.get_index())
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
        // View Slicing Optimization:
        let text = if let Some(cached) = &self.app.cached_preview_text {
            let start = self.scroll as usize;
            let height = area.height as usize;
            if start >= cached.lines.len() {
                Text::default()
            } else {
                let lines: Vec<Line> = cached.lines
                    .iter()
                    .skip(start)
                    .take(height)
                    .map(|line| truncate_line(line, 2000))
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
            .scroll((0, 0)); // We handled scrolling manually

        if self.app.enable_syntax_highlighting {
            paragraph = paragraph.wrap(Wrap { trim: false });
        }
        Widget::render(paragraph, area, buf);
    }
}

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
            Line::from(Span::styled("External Viewers (Request/Response tabs)", bold_underline)),
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

// Helper to truncate lines for performance
fn truncate_line<'a>(line: &'a Line<'a>, max_width: usize) -> Line<'a> {
    let mut current_width = 0;
    let mut new_spans = Vec::new();
    
    for span in &line.spans {
        let content = span.content.as_ref();
        let remaining = max_width.saturating_sub(current_width);
        
        if remaining == 0 {
            break;
        }

        if content.len() <= remaining {
            new_spans.push(span.clone());
            current_width += content.len();
        } else {
            let truncated = &content[..remaining];
            let mut new_span = span.clone();
            new_span.content = std::borrow::Cow::Owned(truncated.to_string());
            new_spans.push(new_span);
            current_width += remaining;
            break;
        }
    }
    
    Line::from(new_spans).style(line.style)
}


// syntax_highlight removed, moved to app.rs
