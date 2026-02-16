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
    state.select(Some(app.get_index()));
    *state.offset_mut() = app.table_offset;
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
}

impl<'a> EntriesTable<'a> {
    pub fn init(app: &'a App) -> Self {
        let mut state = TableState::default();
        let index = app.get_index();
        state.select(Some(index));

        Self {
            table_items: &app.table_items,
            active_focus: app.active_focus,
        }
    }

    fn table(&self) -> Table<'_> {
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
            Constraint::Length(6),
            Constraint::Fill(1),
            Constraint::Fill(2),
            Constraint::Length(12),
            Constraint::Length(12),
            Constraint::Length(26),
        ];

        let rows: Vec<Row> = self
            .table_items
            .iter()
            .map(|item| item.to_table_row())
            .collect();

        Table::new(rows, &widths)
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
            )
    }
}

impl<'a> StatefulWidget for EntriesTable<'a> {
    type State = TableState;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        let table = self.table();

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

    fn tabbar(&self) -> Tabs<'_> {
        Tabs::new(vec![
            " [1] Headers ",
            " [2] Cookies ",
            " [3] Request ",
            " [4] Response ",
        ])
        .select(self.tabbar_state.to_index())
        .padding(" ", " ")
    }
}

impl<'a> Widget for PreviewWidget<'a> {
    fn render(self, area: Rect, buf: &mut Buffer)
    where
        Self: Sized,
    {
        let tabbar = self.tabbar();

        let layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(1), Constraint::Fill(1)])
            .split(area);

        Widget::render(tabbar, layout[0], buf);

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
                    .cloned()
                    .collect();
                Text::from(lines)
            }
        } else {
            Text::raw("Loading or No Body...")
        };

        let paragraph = Paragraph::new(text)
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
            .wrap(Wrap { trim: false })
            .scroll((0, 0)); // We handled scrolling manually
        Widget::render(paragraph, area, buf);
    }

}

pub struct ResponsePreview<'a> {
    app: &'a App,
    scroll: u16,
    active_focus: ActiveFocus,
}

impl<'a> ResponsePreview<'a> {
    pub fn init(app: &'a App) -> Self {
        Self {
            app,
            scroll: app.scroll,
            active_focus: app.active_focus,
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
                    .cloned()
                    .collect();
                Text::from(lines)
            }
        } else {
            Text::raw("Loading or No Response Body...")
        };

        let paragraph = Paragraph::new(text)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("Response Body")
                    .border_style(if self.active_focus == ActiveFocus::Preview {
                        Style::default().fg(Color::Green)
                    } else {
                        Style::default().fg(Color::DarkGray)
                    }),
            )
            .scroll((0, 0)) // We handled scrolling manually
            .wrap(Wrap { trim: false });
        Widget::render(paragraph, area, buf);
    }
}


// syntax_highlight removed, moved to app.rs
