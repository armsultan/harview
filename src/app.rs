use base64::prelude::*;
use chrono;
use crossterm::{
    event::{DisableMouseCapture, EnableMouseCapture},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{prelude::*, widgets::*};
use std::io::Write;
use std::process::Command;
use syntect::{
    easy::HighlightLines,
    highlighting::{Style as SyntectStyle, ThemeSet},
    parsing::SyntaxSet,
    util::LinesWithEndings,
};
use tempfile::{Builder, NamedTempFile};

use crate::har::Har;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PendingAction {
    OpenInBat,
    OpenInFx,
    OpenInEditor,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActiveFocus {
    Table,
    Preview,
}

#[derive(Debug, Clone, PartialEq, Eq, Copy)]
pub enum TabBarState {
    Headers,
    Cookies,
    Request,
    Response,
    Help,
}

impl std::fmt::Display for TabBarState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            Self::Headers => " [1] Headers ",
            Self::Cookies => " [2] Cookies ",
            Self::Request => " [3] Request ",
            Self::Response => " [4] Response ",
            Self::Help => " [?] Help ",
        };
        write!(f, "{}", s)
    }
}

impl TabBarState {
    pub fn next(&self) -> Self {
        match self {
            Self::Headers => Self::Cookies,
            Self::Cookies => Self::Request,
            Self::Request => Self::Response,
            Self::Response => Self::Help,
            Self::Help => Self::Headers,
        }
    }

    pub fn prev(&self) -> Self {
        match self {
            Self::Headers => Self::Help,
            Self::Cookies => Self::Headers,
            Self::Request => Self::Cookies,
            Self::Response => Self::Request,
            Self::Help => Self::Response,
        }
    }

    pub fn to_index(&self) -> usize {
        match self {
            Self::Headers => 0,
            Self::Cookies => 1,
            Self::Request => 2,
            Self::Response => 3,
            Self::Help => 4,
        }
    }
}

#[derive(Debug)]
pub struct App {
    pub running: bool,
    index: usize,
    pub har: Har,
    pub tabbar_state: TabBarState,
    pub scroll: u16,
    pub should_redraw: bool,
    pub window_size: Rect,
    pub active_focus: ActiveFocus,
    // Caching for performance
    pub cached_preview_text: Option<Text<'static>>,
    pub cached_key: Option<(usize, TabBarState)>,
    // Caching table items
    pub table_items: Vec<TableItem>,
    pub enable_syntax_highlighting: bool,
    // Add manual offset for table
    pub table_offset: usize,
    // Deferred external program action (handled by main loop which owns the event handler)
    pub pending_action: Option<PendingAction>,
}

impl App {
    pub fn init(har: Har) -> Self {
        let mut app = Self {
            running: true,
            index: 0,
            tabbar_state: TabBarState::Headers,
            har,
            scroll: 0,
            should_redraw: false,
            window_size: Rect::default(),
            active_focus: ActiveFocus::Table,
            cached_preview_text: None,
            cached_key: None,
            table_items: Vec::new(),
            enable_syntax_highlighting: false,
            table_offset: 0,
            pending_action: None,
        };
        app.table_items = app.generate_table_items();
        app
    }

    pub fn tick(&self) {}

    pub fn get_index(&self) -> usize {
        self.index
    }

    pub fn max_index(&self) -> usize {
        self.har.log.entries.len()
    }

    pub fn get_table_height(&self) -> usize {
        let area_height = self.window_size.height / 2;
        if area_height > 3 {
            (area_height - 3) as usize
        } else {
            1
        }
    }

    fn ensure_visible(&mut self) {
        let table_height = self.get_table_height();
        if self.index < self.table_offset {
            self.table_offset = self.index;
        } else if self.index >= self.table_offset + table_height {
            self.table_offset = self.index - table_height + 1;
        }
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
        self.cached_preview_text = None; // Invalidate cache on index change
        self.ensure_visible();
    }

    pub fn update_index_absolute(&mut self, index: usize) {
        if index < self.max_index() {
            self.index = index;
            self.scroll = 0;
            self.cached_preview_text = None; // Invalidate cache on index change
            self.ensure_visible();
        }
    }

    pub fn update_index_first(&mut self) {
        self.index = 0;
        self.scroll = 0;
        self.cached_preview_text = None; // Invalidate cache
        self.ensure_visible();
    }

    pub fn update_index_last(&mut self) {
        self.index = self.har.log.entries.len() - 1;
        self.scroll = 0;
        self.cached_preview_text = None; // Invalidate cache
        self.ensure_visible();
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
        self.tabbar_state = self.tabbar_state.next();
        self.scroll = 0;
        self.cached_preview_text = None; // Invalidate cache on tab change
    }

    pub fn prev_tab(&mut self) {
        self.tabbar_state = self.tabbar_state.prev();
        self.scroll = 0;
        self.cached_preview_text = None; // Invalidate cache on tab change
    }

    pub fn quit(&mut self) {
        self.running = false;
    }

    pub fn open_in_fx(&mut self) -> anyhow::Result<()> {
        let entry = &self.har.log.entries[self.index];
        let (body, is_json) = match self.tabbar_state {
            TabBarState::Request => {
                let text = entry.request.post_data.as_ref().map(|p| p.text.clone()).unwrap_or_default();
                let mime = entry.request.post_data.as_ref().map(|p| p.mime_type.clone()).unwrap_or_default();
                (text, mime.contains("json"))
            }
            TabBarState::Response => {
                 let text = self.to_response_body(self.index).unwrap_or_else(|| "No response body".to_string());
                 let mime = entry.response.content.mime_type.clone().unwrap_or_default();
                 (text, mime.contains("json"))
            }
            _ => return Ok(()),
        };

        let is_json_heuristic = !is_json && !body.trim_start().starts_with('{') && !body.trim_start().starts_with('[');
        if is_json_heuristic {
            return Ok(());
        }

        let mut temp_file = NamedTempFile::new()?;
        write!(temp_file, "{}", body)?;
        temp_file.flush()?; 

        // Temporarily suspend TUI
        execute!(std::io::stderr(), LeaveAlternateScreen, DisableMouseCapture)?;
        disable_raw_mode()?;

        let status = Command::new("fx")
            .arg(temp_file.path())
            .status();

        // Resume TUI
        enable_raw_mode()?;
        execute!(std::io::stderr(), EnterAlternateScreen, EnableMouseCapture)?;

        // Force a full redraw
        self.should_redraw = true;

        if let Err(e) = status {
             eprintln!("Failed to open fx: {}", e);
        }

        Ok(())
    }

    pub fn open_in_bat(&mut self) -> anyhow::Result<()> {
        let entry = &self.har.log.entries[self.index];
        let (body, mime) = match self.tabbar_state {
            TabBarState::Request => {
                let text = entry.request.post_data.as_ref().map(|p| p.text.clone()).unwrap_or_default();
                let mime = entry.request.post_data.as_ref().map(|p| p.mime_type.clone()).unwrap_or_default();
                (text, mime)
            }
            TabBarState::Response => {
                 let text = self.to_response_body(self.index).unwrap_or_else(|| "No response body".to_string());
                 let mime = entry.response.content.mime_type.clone().unwrap_or_default();
                 (text, mime)
            }
            _ => return Ok(()),
        };

        let extension = if mime.contains("json") {
            "json"
        } else if mime.contains("html") {
            "html"
        } else if mime.contains("javascript") || mime.contains("js") {
            "js"
        } else if mime.contains("css") {
            "css"
        } else if mime.contains("xml") {
            "xml"
        } else {
            "txt"
        };

        let mut temp_file = Builder::new()
            .suffix(&format!(".{}", extension))
            .tempfile()?;
            
        write!(temp_file, "{}", body)?;
        temp_file.flush()?;

        // Temporarily suspend TUI
        execute!(std::io::stderr(), LeaveAlternateScreen, DisableMouseCapture)?;
        disable_raw_mode()?;

        let status = Command::new("bat")
            .arg(temp_file.path())
            .status();

        // Resume TUI
        enable_raw_mode()?;
        execute!(std::io::stderr(), EnterAlternateScreen, EnableMouseCapture)?;

        // Force a full redraw
        self.should_redraw = true;

        if let Err(e) = status {
             eprintln!("Failed to open bat: {}", e);
        }

        Ok(())
    }

    pub fn open_in_editor(&mut self) -> anyhow::Result<()> {
        let entry = &self.har.log.entries[self.index];
        let (body, mime) = match self.tabbar_state {
            TabBarState::Request => {
                let text = entry.request.post_data.as_ref().map(|p| p.text.clone()).unwrap_or_default();
                let mime = entry.request.post_data.as_ref().map(|p| p.mime_type.clone()).unwrap_or_default();
                (text, mime)
            }
            TabBarState::Response => {
                 let text = self.to_response_body(self.index).unwrap_or_else(|| "No response body".to_string());
                 let mime = entry.response.content.mime_type.clone().unwrap_or_default();
                 (text, mime)
            }
            _ => return Ok(()),
        };

        let extension = if mime.contains("json") {
            "json"
        } else if mime.contains("html") {
            "html"
        } else if mime.contains("javascript") || mime.contains("js") {
            "js"
        } else if mime.contains("css") {
            "css"
        } else if mime.contains("xml") {
            "xml"
        } else {
            "txt"
        };

        let mut temp_file = Builder::new()
            .suffix(&format!(".{}", extension))
            .tempfile()?;
            
        write!(temp_file, "{}", body)?;
        temp_file.flush()?;

        let editor = std::env::var("EDITOR").unwrap_or_else(|_| "vi".to_string());

        // Temporarily suspend TUI
        execute!(std::io::stderr(), LeaveAlternateScreen, DisableMouseCapture)?;
        disable_raw_mode()?;

        let status = Command::new(editor)
            .arg(temp_file.path())
            .status();

        // Resume TUI
        enable_raw_mode()?;
        execute!(std::io::stderr(), EnterAlternateScreen, EnableMouseCapture)?;

        // Force a full redraw
        self.should_redraw = true;

        if let Err(e) = status {
             eprintln!("Failed to open editor: {}", e);
        }

        Ok(())
    }


    pub fn get_preview_text(&mut self) -> &Text<'static> {
        // If cache is valid for current index and tab, return it (but we can't return reference to self field if we just mutated it)
        // Rust borrow checker issue: can't return ref to field while borrowing self mutably.
        // We need to check validity first, update if needed, then return ref.
        
        let key = (self.index, self.tabbar_state);
        if self.cached_key == Some(key) && self.cached_preview_text.is_some() {
             return self.cached_preview_text.as_ref().unwrap();
        }

        // Cache miss or invalid, regenerate
        let text_content: String;
        let mime_type: String;

        match self.tabbar_state {
            TabBarState::Request => {
                 let entry = &self.har.log.entries[self.index];
                 text_content = entry.request.post_data.as_ref().map(|p| p.text.clone()).unwrap_or_else(|| "No request body".to_string());
                 mime_type = entry.request.post_data.as_ref().map(|p| p.mime_type.clone()).unwrap_or_default();
            },
            TabBarState::Response => {
                 text_content = self.to_response_body(self.index).unwrap_or_else(|| "No response body".to_string());
                 let entry = &self.har.log.entries[self.index];
                 mime_type = entry.response.content.mime_type.clone().unwrap_or_default();
            },
            _ => {
                // For Headers/Cookies, we don't use this text cache (they differ in structure), 
                // effectively this function shouldn't be called or returns empty.
                // But for safety, return empty.
                text_content = String::new();
                mime_type = String::new();
            }
        }

        if self.enable_syntax_highlighting {
            let highlighted = syntax_highlight(&text_content, &mime_type);
            self.cached_preview_text = Some(highlighted);
        } else {
             self.cached_preview_text = Some(Text::from(text_content));
        }
        self.cached_key = Some(key);

        self.cached_preview_text.as_ref().unwrap()
    }
    
    pub fn toggle_syntax_highlighting(&mut self) {
        self.enable_syntax_highlighting = !self.enable_syntax_highlighting;
        self.cached_preview_text = None; // Invalidate cache
    }

    pub fn set_tabbar_state(&mut self, state: TabBarState) {
        self.tabbar_state = state;
        self.scroll = 0;
        self.cached_preview_text = None;
    }

    pub fn generate_table_items(&self) -> Vec<TableItem> {
        self.har.log
            .entries
            .iter()
            .map(|entry| {
                let url = entry.request.url.as_str().to_string();
                let mime_type = entry.response.content.mime_type.clone().unwrap_or_default();
                let status = entry.response.status as u16;

                let size = if let Some(s) = entry.response.content.size {
                    if s < 0 {
                         "0 B".to_string()
                    } else {
                        byte_unit::Byte::from_u64(s as u64)
                            .get_appropriate_unit(byte_unit::UnitType::Decimal)
                            .to_string()
                    }
                } else {
                     "0 B".to_string()
                };
                
                let timestamp = chrono::DateTime::parse_from_rfc3339(&entry.started_date_time)
                     .map(|dt| dt.format("%H:%M:%S%.3f").to_string())
                     .unwrap_or_else(|_| "".to_string());

                TableItem {
                    status,
                    method: entry.request.method.clone(),
                    url,
                    mime_type,
                    total_size: size,
                    timestamp,
                }
            })
            .collect()
    }

    pub fn to_header_info(&self, index: usize) -> Option<HeaderInfo> {
        let entry = self.har.log.entries.get(index)?;

        let req_headers = entry
            .request
            .headers
            .iter()
            .map(|h| (h.name.clone(), h.value.clone()))
            .collect();
        let resp_headers = entry
            .response
            .headers
            .iter()
            .map(|h| (h.name.clone(), h.value.clone()))
            .collect();

        Some(HeaderInfo {
            url: entry.request.url.to_string(),
            method: entry.request.method.clone(),
            status: entry.response.status,
            req_headers,
            resp_headers,
        })
    }

    pub fn to_cookie_info(&self, index: usize) -> Option<CookieInfo> {
        let entry = self.har.log.entries.get(index)?;

        let req_cookies = entry
            .request
            .cookies
            .iter()
            .map(|c| (c.name.clone(), c.value.clone()))
            .collect();
        let resp_cookies = entry
            .response
            .cookies
            .iter()
            .map(|c| (c.name.clone(), c.value.clone()))
            .collect();

        Some(CookieInfo {
            req_cookies,
            resp_cookies,
        })
    }
    
    pub fn to_request_body(&self, index: usize) -> Option<String> {
        let entry = self.har.log.entries.get(index)?;
        entry.request.post_data.as_ref().map(|p| p.text.clone())
    }

    pub fn to_response_body(&self, index: usize) -> Option<String> {
        let entry = self.har.log.entries.get(index)?;
        let content = &entry.response.content;

        if let Some(text) = &content.text {
            // Handle base64 encoding if necessary
            if content.encoding.as_deref() == Some("base64") {
                 use base64::prelude::*;
                 match BASE64_STANDARD.decode(text) {
                     Ok(decoded) => Some(String::from_utf8_lossy(&decoded).to_string()),
                     Err(_) => Some(text.clone()),
                 }
            } else {
                 Some(text.clone())
            }
        } else {
            None
        }
    }
}

#[derive(Debug, Clone)]
pub struct TableItem {
    pub status: u16,
    pub method: String,
    pub url: String,

    pub mime_type: String,
    pub total_size: String,
    pub timestamp: String,
}

impl TableItem {
    pub fn to_table_row(&self) -> Row<'_> {
        Row::new(vec![
            Cell::from(self.status.to_string()).style(match self.status {
                100..=199 => Style::default().fg(Color::LightBlue),
                200..=299 => Style::default().fg(Color::LightGreen),
                300..=399 => Style::default().fg(Color::LightCyan),
                400..=499 => Style::default().fg(Color::LightYellow),
                500..=599 => Style::default().fg(Color::LightMagenta),
                _ => Style::default().fg(Color::DarkGray),
            }),
            Cell::from(self.method.as_str()).style(Style::default().fg(Color::Yellow)),
            Cell::from(self.url.as_str()).style(Style::default().fg(Color::LightBlue)),
            Cell::from(self.mime_type.as_str()).style(Style::default().fg(Color::Magenta)),
            Cell::from(self.total_size.as_str()).style(Style::default().fg(Color::LightCyan)),
            Cell::from(self.timestamp.as_str()),
        ])
    }
}

#[derive(Debug, Clone)]
pub struct HeaderInfo {
    pub url: String,
    pub method: String,
    pub status: i64,
    pub req_headers: Vec<(String, String)>,
    pub resp_headers: Vec<(String, String)>,
}

#[derive(Debug, Clone)]
pub struct CookieInfo {
    pub req_cookies: Vec<(String, String)>,
    pub resp_cookies: Vec<(String, String)>,
}

pub fn syntax_highlight(text: &str, mime_type: &str) -> Text<'static> {
    use std::sync::LazyLock;
    static SYNTAX_SET: LazyLock<SyntaxSet> = LazyLock::new(SyntaxSet::load_defaults_newlines);
    static THEME_SET: LazyLock<ThemeSet> = LazyLock::new(ThemeSet::load_defaults);

    // Skip highlighting for very large content to keep the UI responsive.
    // Use bat ('b') for detailed viewing of large payloads.
    const MAX_HIGHLIGHT_BYTES: usize = 200_000;

    let ps = &*SYNTAX_SET;
    let ts = &*THEME_SET;
    let mime_type = mime_type.to_lowercase();

    // Try to format as JSON first if it looks like JSON or MIME matches
    let json_parsed = serde_json::from_str::<serde_json::Value>(text);
    let is_json = json_parsed.is_ok();

    let formatted_text = if mime_type.contains("json") || is_json {
        json_parsed
            .and_then(|v| serde_json::to_string_pretty(&v))
            .unwrap_or_else(|_| text.to_string())
    } else if mime_type.contains("xml") {
        prettyish_html::prettify(text)
    } else {
        text.to_string()
    };

    if formatted_text.len() > MAX_HIGHLIGHT_BYTES {
        return Text::from(formatted_text);
    }

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
