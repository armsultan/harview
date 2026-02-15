use base64::prelude::*;
use crossterm::{
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{prelude::*, widgets::*};
use std::io::Write;
use std::process::Command;
use tempfile::NamedTempFile;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActiveFocus {
    Table,
    Preview,
}

#[derive(Debug)]
pub struct App {
    pub running: bool,
    index: usize,
    pub har: Har,
    //pub preview_widget_state: PreviewWidetState,
    pub tabbar_state: TabBarState,
    pub scroll: u16,
    pub should_redraw: bool,
    pub window_size: Rect,
    pub active_focus: ActiveFocus,
}

impl App {
    pub fn init(har: Har) -> Self {
        Self {
            running: true,
            index: 0,
            tabbar_state: TabBarState::Headers,
            har,
            scroll: 0,
            should_redraw: false,
            window_size: Rect::default(),
            active_focus: ActiveFocus::Table,
        }
    }

    pub fn tick(&self) {}

    pub fn get_index(&self) -> usize {
        self.index
    }

    pub fn max_index(&self) -> usize {
        self.har.log.entries.len()
    }

    pub fn update_index(&mut self, delta: i32) {
        let max = self.max_index();
        let added = self.index as i32 + delta;
        self.index = if added < 0 {
            0
        } else if added >= max as i32 {
            max - 1
        } else {
            added as usize
        };
        self.scroll = 0;
    }
    pub fn update_index_first(&mut self) {
        self.index = 0;
        self.scroll = 0;
    }

    pub fn update_index_last(&mut self) {
        self.index = self.har.log.entries.len() - 1;
        self.scroll = 0;
    }

    pub fn on_up(&mut self) {
        if self.scroll > 0 {
            self.scroll -= 1;
        }
    }

    pub fn on_down(&mut self) {
        self.scroll += 1;
    }

    pub fn on_page_up(&mut self) {
        if self.scroll > 10 {
            self.scroll -= 10;
        } else {
            self.scroll = 0;
        }
    }

    pub fn on_page_down(&mut self) {
        self.scroll += 10;
    }

    pub fn next_tab(&mut self) {
        let current_index = self.tabbar_state.to_index();
        let next_index = (current_index + 1) % 4;
        self.tabbar_state = TabBarState::from_index(next_index).unwrap();
        self.scroll = 0;
    }

    pub fn prev_tab(&mut self) {
        let current_index = self.tabbar_state.to_index();
        let prev_index = if current_index == 0 {
            3
        } else {
            current_index - 1
        };
        self.tabbar_state = TabBarState::from_index(prev_index).unwrap();
        self.scroll = 0;
    }

    //pub fn set_preview_widget_state(&mut self, state: &PreviewWidetState) {
    //    self.preview_widget_state = state.clone();
    //}
    pub fn set_tabbar_state(&mut self, state: &TabBarState) {
        self.tabbar_state = state.clone();
        self.scroll = 0;
    }

    pub fn open_in_fx(&mut self) {
        let body = match self.tabbar_state {
            TabBarState::Request => self.har.to_request_body(self.get_index()),
            TabBarState::Response => self.har.to_response_body(self.get_index()),
            _ => None,
        };

        if let Some(content) = body {
            // Suspend TUI
            // Suspend TUI
            disable_raw_mode().unwrap();
            execute!(std::io::stderr(), LeaveAlternateScreen).unwrap();

            // Write content to temp file
            let mut temp_file = NamedTempFile::new().expect("Failed to create temp file");
            write!(temp_file, "{}", content).expect("Failed to write to temp file");
            temp_file.flush().expect("Failed to flush temp file");
            let temp_path = temp_file.path().to_owned();

            println!("Opening fx with {}...", temp_path.display());

            // Run fx with file path
            let mut child = Command::new("fx")
                .arg(temp_path)
                .spawn()
                .expect("Failed to start fx");

            let _ = child.wait();

            // Restore TUI
            // Restore TUI
            execute!(std::io::stderr(), EnterAlternateScreen).unwrap();
            enable_raw_mode().unwrap();
            self.should_redraw = true;
        }
    }

    pub fn quit(&mut self) {
        self.running = false;
    }
}

type Har = crate::Har;

impl Har {
    pub fn to_table_items(&self) -> Vec<TableItem> {
        self.log
            .entries
            .iter()
            .map(|entry| {
                let result = url::Url::parse(entry.request.url.as_str());

                TableItem {
                    status: entry.response.status as u16,
                    method: entry.request.method.clone(),
                    domain: {
                        match result {
                            Ok(ref url) => url.domain().unwrap_or("").to_string(),
                            Err(_) => "".to_string(),
                        }
                    },
                    file_name: {
                        match result {
                            Ok(ref url) => url.path().to_string(),
                            Err(_) => "".to_string(),
                        }
                    },
                    mime_type: entry
                        .response
                        .content
                        .mime_type
                        .clone()
                        .unwrap_or("".to_string()),
                    size: entry.response.content.size,
                    timestamp: entry.started_date_time.clone(),
                }
            })
            .collect()
    }

    pub fn to_header_info(&self, index: usize) -> Option<HeaderInfo> {
        if let Some(entry) = self.log.entries.get(index) {
            return Some(HeaderInfo {
                status: entry.response.status,
                method: entry.request.method.clone(),
                http_version: entry.request.http_version.clone(),
                url: entry.request.url.clone(),
                referrer_policy: entry
                    .request
                    .headers
                    .iter()
                    .filter(|header| header.name.eq_ignore_ascii_case("Referrer-Policy"))
                    .map(|header| header.value.clone())
                    .next(),
                query_params: entry
                    .request
                    .query_string
                    .iter()
                    .map(|query| (query.name.clone(), query.value.clone()))
                    .collect(),
                req_headers: entry
                    .request
                    .headers
                    .iter()
                    .map(|header| (header.name.clone(), header.value.clone()))
                    .collect(),
                resp_headers: entry
                    .response
                    .headers
                    .iter()
                    .map(|header| (header.name.clone(), header.value.clone()))
                    .collect(),
            });
        }

        None
    }

    pub fn to_cookie_info(har: &Har, index: usize) -> Option<CookieInfo> {
        if let Some(entry) = har.log.entries.get(index) {
            return Some(CookieInfo {
                req_cookies: entry
                    .request
                    .cookies
                    .iter()
                    .map(|cookie| (cookie.name.clone(), cookie.value.clone()))
                    .collect(),
                resp_cookies: entry
                    .response
                    .cookies
                    .iter()
                    .map(|cookie| (cookie.name.clone(), cookie.value.clone()))
                    .collect(),
            });
        }

        None
    }

    pub fn to_request_body(&self, index: usize) -> Option<String> {
        if let Some(entry) = self.log.entries.get(index) {
            if let Some(post_data) = &entry.request.post_data {
                return Some(post_data.text.clone());
            }
        }
        None
    }

    pub fn to_response_body(&self, index: usize) -> Option<String> {
        if let Some(entry) = self.log.entries.get(index) {
            if let Some(text) = &entry.response.content.text {
                if let Some(encoding) = &entry.response.content.encoding {
                    if encoding.eq_ignore_ascii_case("base64") {
                        match BASE64_STANDARD.decode(text) {
                            Ok(decoded) => {
                                return Some(String::from_utf8_lossy(&decoded).to_string())
                            }
                            Err(_) => return Some(format!("Error decoding base64: {}", text)),
                        }
                    }
                }
                return Some(text.clone());
            }
        }
        None
    }
}
#[derive(Debug, Clone)]
pub enum TabBarState {
    Headers,
    Cookies,
    Request,
    Response,
}

impl TabBarState {
    pub fn from_index(index: usize) -> Option<Self> {
        match index {
            0 => Some(Self::Headers),
            1 => Some(Self::Cookies),
            2 => Some(Self::Request),
            3 => Some(Self::Response),
            _ => None,
        }
    }

    pub fn to_index(&self) -> usize {
        match self {
            Self::Headers => 0,
            Self::Cookies => 1,
            Self::Request => 2,
            Self::Response => 3,
        }
    }
}

impl std::fmt::Display for TabBarState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            Self::Headers => " [1] Headers ",
            Self::Cookies => " [2] Cookies ",
            Self::Request => " [3] Request ",
            Self::Response => " [4] Response ",
        };
        write!(f, "{}", s)
    }
}

//#[derive(Debug, Clone)]
//pub enum PreviewWidetState {
//    Hidden,
//    Bottom,
//    Right,
//    Full,
//}

#[derive(Debug, Clone)]
pub struct TableItem {
    pub status: u16,
    pub method: String,
    pub domain: String,
    pub file_name: String,
    pub mime_type: String,
    pub size: Option<i64>,
    pub timestamp: String,
}

impl TableItem {
    pub fn to_table_row(&self) -> ratatui::widgets::Row<'static> {
        let status_span = match self.status {
            100..=199 => Span::styled(
                self.status.to_string(),
                Style::default().fg(Color::LightBlue).bold(),
            ),
            200..=299 => Span::styled(
                self.status.to_string(),
                Style::default().fg(Color::LightGreen).bold(),
            ),
            300..=399 => Span::styled(
                self.status.to_string(),
                Style::default().fg(Color::LightCyan).bold(),
            ),
            400..=499 => Span::styled(
                self.status.to_string(),
                Style::default().fg(Color::LightYellow).bold(),
            ),
            500..=599 => Span::styled(
                self.status.to_string(),
                Style::default().fg(Color::LightMagenta).bold(),
            ),
            0 => Span::styled("---", Style::default().fg(Color::DarkGray).bold()),
            _ => Span::styled(
                self.status.to_string(),
                Style::default().bg(Color::DarkGray),
            ),
        };

        let mime_type = self.mime_type.clone();
        let shorten_mime = match mime_type.as_str().parse::<mime::Mime>() {
            Ok(m) => m.subtype().to_string(),
            Err(_) => mime_type,
        };

        let size_span = match self.size {
            Some(s) => {
                let b = byte_unit::Byte::from_u64(s as u64);
                Span::styled(
                    format!(
                        "{:>8.2} {:<2}",
                        b.get_appropriate_unit(byte_unit::UnitType::Decimal)
                            .get_value(),
                        b.get_appropriate_unit(byte_unit::UnitType::Decimal)
                            .get_unit()
                    ),
                    Style::default(),
                )
            }
            None => Span::styled("     --- B", Style::default().fg(Color::DarkGray)),
        };

        Row::new([
            Cell::new(status_span),
            Cell::new(Span::styled(
                self.method.clone(),
                Style::default().fg(Color::White).bold(),
            )),
            Cell::new(Span::styled(
                self.domain.clone(),
                Style::default().fg(Color::White),
            )),
            Cell::new(self.file_name.clone()),
            Cell::new(shorten_mime),
            Cell::new(size_span),
            Cell::new(self.timestamp.clone()),
        ])
    }
}

#[derive(Debug)]
pub struct HeaderInfo {
    pub status: i64,
    pub method: String,
    pub http_version: String,
    pub url: url::Url,
    pub query_params: Vec<(String, String)>,
    pub referrer_policy: Option<String>,
    pub req_headers: Vec<(String, String)>,
    pub resp_headers: Vec<(String, String)>,
}

#[derive(Debug)]
pub struct CookieInfo {
    pub req_cookies: Vec<(String, String)>,
    pub resp_cookies: Vec<(String, String)>,
}
