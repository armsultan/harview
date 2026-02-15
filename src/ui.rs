use crate::app::{ActiveFocus, App, CookieInfo, HeaderInfo, TabBarState, TableItem};
use crate::har::Har;
use ratatui::{prelude::*, widgets::*};
use syntect::{
    easy::HighlightLines,
    highlighting::{Style as SyntectStyle, ThemeSet},
    parsing::SyntaxSet,
    util::LinesWithEndings,
};

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
    table.render(area, buf, &mut state);
}

pub fn render_preview(app: &mut App, area: Rect, buf: &mut Buffer) {
    let preview = PreviewWidget::init(app);
    preview.render(area, buf);
}

#[derive(Debug)]
pub struct EntriesTable {
    table_items: Vec<TableItem>,
    active_focus: ActiveFocus,
}

impl EntriesTable {
    pub fn init(app: &App) -> Self {
        let mut state = TableState::default();
        let index = app.get_index();
        state.select(Some(index));

        Self {
            table_items: app.har.to_table_items(),
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

impl StatefulWidget for EntriesTable {
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
            header_info: app.har.to_header_info(app.get_index()),
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
            let mut lines = vec![];

            // General Info
            lines.push(Line::from(vec![Span::styled(
                "General",
                Style::default().bold().underlined(),
            )]));
            lines.push(Line::from(vec![
                Span::raw("Request URL: "),
                Span::styled(
                    header_info.url.to_string(),
                    Style::default().fg(Color::Cyan),
                ),
            ]));
            lines.push(Line::from(vec![
                Span::raw("Request Method: "),
                Span::styled(
                    header_info.method.clone(),
                    Style::default().fg(Color::Yellow),
                ),
            ]));
            lines.push(Line::from(vec![
                Span::raw("Status Code: "),
                Span::styled(
                    header_info.status.to_string(),
                    Style::default().fg(Color::Green),
                ),
            ]));
            lines.push(Line::from(""));

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
            cookie_info: Har::to_cookie_info(&app.har, app.get_index()),
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

pub struct RequestPreview {
    body: Option<String>,
    mime_type: String,
    scroll: u16,
    active_focus: ActiveFocus,
}

impl RequestPreview {
    pub fn init(app: &App) -> Self {
        let table_item = app.har.to_table_items();
        let item = table_item
            .get(app.get_index())
            .expect("index out of bounds");
        Self {
            body: app.har.to_request_body(app.get_index()),
            mime_type: item.mime_type.clone(),
            scroll: app.scroll,
            active_focus: app.active_focus,
        }
    }
}

impl Widget for RequestPreview {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let text = self.body.unwrap_or_else(|| "No request body".to_string());

        let highlighted_text = syntax_highlight(&text, &self.mime_type);

        let paragraph = Paragraph::new(highlighted_text)
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
            .scroll((self.scroll, 0));
        Widget::render(paragraph, area, buf);
    }
}

pub struct ResponsePreview {
    body: Option<String>,
    mime_type: String,
    scroll: u16,
    active_focus: ActiveFocus,
}

impl ResponsePreview {
    pub fn init(app: &App) -> Self {
        let table_item = app.har.to_table_items();
        let item = table_item
            .get(app.get_index())
            .expect("index out of bounds");
        Self {
            body: app.har.to_response_body(app.get_index()),
            mime_type: item.mime_type.clone(),
            scroll: app.scroll,
            active_focus: app.active_focus,
        }
    }
}

impl Widget for ResponsePreview {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let text = self.body.unwrap_or_else(|| "No response body".to_string());

        let highlighted_text = syntax_highlight(&text, &self.mime_type);

        let paragraph = Paragraph::new(highlighted_text)
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
            .wrap(Wrap { trim: false })
            .scroll((self.scroll, 0));
        Widget::render(paragraph, area, buf);
    }
}

fn syntax_highlight(text: &str, mime_type: &str) -> Text<'static> {
    let ps = SyntaxSet::load_defaults_newlines();
    let ts = ThemeSet::load_defaults();
    let mime_type = mime_type.to_lowercase();

    // Try to format as JSON first if it looks like JSON or MIME matches
    let json_parsed = serde_json::from_str::<serde_json::Value>(text);
    let is_json = json_parsed.is_ok();

    let syntax = if mime_type.contains("json") || is_json {
        ps.find_syntax_by_extension("json").unwrap()
    } else if mime_type.contains("xml") {
        ps.find_syntax_by_extension("xml").unwrap()
    } else if mime_type.contains("html") {
        ps.find_syntax_by_extension("html").unwrap()
    } else if mime_type.contains("javascript") || mime_type.contains("js") {
        ps.find_syntax_by_extension("js").unwrap()
    } else if mime_type.contains("css") {
        ps.find_syntax_by_extension("css").unwrap()
    } else {
        ps.find_syntax_plain_text()
    };

    let formatted_text = if mime_type.contains("json") || is_json {
        json_parsed
            .and_then(|v| serde_json::to_string_pretty(&v))
            .unwrap_or_else(|_| text.to_string())
    } else if mime_type.contains("xml") {
        prettyish_html::prettify(text)
    } else {
        text.to_string()
    };

    let mut highlighter = HighlightLines::new(syntax, &ts.themes["base16-ocean.dark"]);
    let mut lines = Vec::new();

    for line in LinesWithEndings::from(&formatted_text) {
        let ranges: Vec<(SyntectStyle, &str)> =
            highlighter.highlight_line(line, &ps).unwrap_or_default();
        let spans: Vec<Span> = ranges
            .into_iter()
            .map(|(style, content)| {
                let fg = Color::Rgb(style.foreground.r, style.foreground.g, style.foreground.b);
                Span::styled(content.to_string(), Style::default().fg(fg))
            })
            .collect();
        lines.push(ratatui::text::Line::from(spans));
    }

    Text::from(lines)
}
