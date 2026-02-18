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

use crate::har::{self, Har};

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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SearchScope {
    All,
    Url,
    Host,
    QueryString,
    RequestHeaders,
    ResponseHeaders,
    RequestBody,
    ResponseBody,
    Method,
    StatusCode,
    RequestBodySize,
    ResponseBodySize,
    Duration,
}

impl SearchScope {
    pub fn display_name(&self) -> &'static str {
        match self {
            Self::All => "ALL",
            Self::Url => "URL",
            Self::Host => "Host",
            Self::QueryString => "QueryStr",
            Self::RequestHeaders => "ReqHdrs",
            Self::ResponseHeaders => "RespHdrs",
            Self::RequestBody => "ReqBody",
            Self::ResponseBody => "RespBody",
            Self::Method => "Method",
            Self::StatusCode => "Status",
            Self::RequestBodySize => "ReqSize",
            Self::ResponseBodySize => "RespSize",
            Self::Duration => "Duration",
        }
    }

    pub fn next(&self) -> Self {
        match self {
            Self::All => Self::Url,
            Self::Url => Self::Host,
            Self::Host => Self::QueryString,
            Self::QueryString => Self::RequestHeaders,
            Self::RequestHeaders => Self::ResponseHeaders,
            Self::ResponseHeaders => Self::RequestBody,
            Self::RequestBody => Self::ResponseBody,
            Self::ResponseBody => Self::Method,
            Self::Method => Self::StatusCode,
            Self::StatusCode => Self::RequestBodySize,
            Self::RequestBodySize => Self::ResponseBodySize,
            Self::ResponseBodySize => Self::Duration,
            Self::Duration => Self::All,
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
    // All table items (never filtered)
    pub table_items: Vec<TableItem>,
    pub enable_syntax_highlighting: bool,
    // Manual offset for table
    pub table_offset: usize,
    // Deferred external program action
    pub pending_action: Option<PendingAction>,
    // Search/filter state
    pub search_mode: bool,
    pub search_query: String,
    pub search_scope: SearchScope,
    pub search_active: bool,
    pub search_error: bool,
    /// Compiled regex kept in sync with search_query for use by the renderer.
    pub search_regex: Option<regex::Regex>,
    /// Indices into har.log.entries that are currently displayed (filtered subset or all).
    pub display_entry_indices: Vec<usize>,
    // Saved state so Esc can restore pre-search position
    search_saved_query: String,
    search_saved_active: bool,
    search_saved_indices: Vec<usize>,
    search_saved_index: usize,
    search_saved_offset: usize,
}

impl App {
    pub fn init(har: Har) -> Self {
        let n = har.log.entries.len();
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
            search_mode: false,
            search_query: String::new(),
            search_scope: SearchScope::All,
            search_active: false,
            search_error: false,
            search_regex: None,
            display_entry_indices: (0..n).collect(),
            search_saved_query: String::new(),
            search_saved_active: false,
            search_saved_indices: (0..n).collect(),
            search_saved_index: 0,
            search_saved_offset: 0,
        };
        app.table_items = app.generate_table_items();
        app
    }

    pub fn tick(&self) {}

    pub fn get_index(&self) -> usize {
        self.index
    }

    /// Returns the actual index into har.log.entries for the current display selection.
    pub fn get_entry_index(&self) -> usize {
        self.display_entry_indices
            .get(self.index)
            .copied()
            .unwrap_or(0)
    }

    pub fn max_index(&self) -> usize {
        self.display_entry_indices.len()
    }

    pub fn get_table_height(&self) -> usize {
        let search_bar = if self.search_mode || self.search_active { 1u16 } else { 0u16 };
        let area_height = self.window_size.height / 2;
        if area_height > 3 + search_bar {
            (area_height - 3 - search_bar) as usize
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
        if max == 0 {
            return;
        }
        let added = self.index as i32 + delta;
        self.index = if added < 0 {
            0
        } else if added >= max as i32 {
            max - 1
        } else {
            added as usize
        };
        self.scroll = 0;
        self.cached_preview_text = None;
        self.ensure_visible();
    }

    pub fn update_index_absolute(&mut self, index: usize) {
        if index < self.max_index() {
            self.index = index;
            self.scroll = 0;
            self.cached_preview_text = None;
            self.ensure_visible();
        }
    }

    pub fn update_index_first(&mut self) {
        self.index = 0;
        self.scroll = 0;
        self.cached_preview_text = None;
        self.ensure_visible();
    }

    pub fn update_index_last(&mut self) {
        let n = self.display_entry_indices.len();
        if n > 0 {
            self.index = n - 1;
        }
        self.scroll = 0;
        self.cached_preview_text = None;
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
        self.cached_preview_text = None;
    }

    pub fn prev_tab(&mut self) {
        self.tabbar_state = self.tabbar_state.prev();
        self.scroll = 0;
        self.cached_preview_text = None;
    }

    pub fn quit(&mut self) {
        self.running = false;
    }

    // ── Search ──────────────────────────────────────────────────────────────

    /// Enter search mode, saving current display state for Esc cancellation.
    pub fn enter_search_mode(&mut self) {
        self.search_saved_query = self.search_query.clone();
        self.search_saved_active = self.search_active;
        self.search_saved_indices = self.display_entry_indices.clone();
        self.search_saved_index = self.index;
        self.search_saved_offset = self.table_offset;
        self.search_mode = true;
    }

    /// Append a character to the search query and re-filter live.
    pub fn push_search_char(&mut self, c: char) {
        self.search_query.push(c);
        self.apply_filter();
    }

    /// Remove last character from query and re-filter.
    pub fn pop_search_char(&mut self) {
        self.search_query.pop();
        self.apply_filter();
    }

    /// Confirm search: exit search mode keeping the current filter.
    pub fn confirm_search(&mut self) {
        self.search_mode = false;
    }

    /// Cancel search: exit search mode and restore pre-search state.
    pub fn cancel_search(&mut self) {
        self.search_mode = false;
        self.search_query = self.search_saved_query.clone();
        self.search_active = self.search_saved_active;
        self.display_entry_indices = self.search_saved_indices.clone();
        self.index = self.search_saved_index;
        self.table_offset = self.search_saved_offset;
        self.search_error = false;
        self.search_regex = if self.search_active && !self.search_query.is_empty() {
            regex::Regex::new(&self.search_query).ok()
        } else {
            None
        };
        self.cached_preview_text = None;
    }

    /// Clear any active filter (called by Esc in normal mode).
    pub fn clear_search(&mut self) {
        self.search_active = false;
        self.search_error = false;
        self.search_query.clear();
        self.search_regex = None;
        self.display_entry_indices = (0..self.har.log.entries.len()).collect();
        self.index = 0;
        self.table_offset = 0;
        self.cached_preview_text = None;
    }

    /// Cycle to the next search scope and re-filter.
    pub fn cycle_search_scope(&mut self) {
        self.search_scope = self.search_scope.next();
        self.apply_filter();
    }

    /// Recompute display_entry_indices from the current query and scope.
    fn apply_filter(&mut self) {
        // Remember which original entry we were on so we can try to keep it selected.
        let current_entry_idx = self.display_entry_indices.get(self.index).copied();

        if self.search_query.is_empty() {
            self.search_active = false;
            self.search_error = false;
            self.search_regex = None;
            self.display_entry_indices = (0..self.har.log.entries.len()).collect();
        } else {
            match regex::Regex::new(&self.search_query) {
                Err(_) => {
                    self.search_error = true;
                    // Leave display_entry_indices and search_regex unchanged on invalid regex.
                    return;
                }
                Ok(re) => {
                    self.search_error = false;
                    self.search_active = true;
                    let scope = self.search_scope;
                    let entries = &self.har.log.entries;
                    self.display_entry_indices = (0..entries.len())
                        .filter(|&i| entry_matches(&entries[i], scope, &re))
                        .collect();
                    self.search_regex = Some(re);
                }
            }
        }

        // Try to keep the same original entry selected; fall back to first.
        self.index = current_entry_idx
            .and_then(|ei| self.display_entry_indices.iter().position(|&i| i == ei))
            .unwrap_or(0);
        self.table_offset = 0;
        self.ensure_visible();
        self.cached_preview_text = None;
    }

    // ── External viewers ────────────────────────────────────────────────────

    pub fn open_in_fx(&mut self) -> anyhow::Result<()> {
        let entry = &self.har.log.entries[self.get_entry_index()];
        let (body, is_json) = match self.tabbar_state {
            TabBarState::Request => {
                let text = entry.request.post_data.as_ref().map(|p| p.text.clone()).unwrap_or_default();
                let mime = entry.request.post_data.as_ref().map(|p| p.mime_type.clone()).unwrap_or_default();
                (text, mime.contains("json"))
            }
            TabBarState::Response => {
                let text = self.to_response_body(self.get_entry_index()).unwrap_or_else(|| "No response body".to_string());
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

        execute!(std::io::stderr(), LeaveAlternateScreen, DisableMouseCapture)?;
        disable_raw_mode()?;

        let status = Command::new("fx").arg(temp_file.path()).status();

        enable_raw_mode()?;
        execute!(std::io::stderr(), EnterAlternateScreen, EnableMouseCapture)?;

        self.should_redraw = true;

        if let Err(e) = status {
            eprintln!("Failed to open fx: {}", e);
        }

        Ok(())
    }

    pub fn open_in_bat(&mut self) -> anyhow::Result<()> {
        let entry = &self.har.log.entries[self.get_entry_index()];
        let (body, mime) = match self.tabbar_state {
            TabBarState::Request => {
                let text = entry.request.post_data.as_ref().map(|p| p.text.clone()).unwrap_or_default();
                let mime = entry.request.post_data.as_ref().map(|p| p.mime_type.clone()).unwrap_or_default();
                (text, mime)
            }
            TabBarState::Response => {
                let text = self.to_response_body(self.get_entry_index()).unwrap_or_else(|| "No response body".to_string());
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

        execute!(std::io::stderr(), LeaveAlternateScreen, DisableMouseCapture)?;
        disable_raw_mode()?;

        let status = Command::new("bat").arg(temp_file.path()).status();

        enable_raw_mode()?;
        execute!(std::io::stderr(), EnterAlternateScreen, EnableMouseCapture)?;

        self.should_redraw = true;

        if let Err(e) = status {
            eprintln!("Failed to open bat: {}", e);
        }

        Ok(())
    }

    pub fn open_in_editor(&mut self) -> anyhow::Result<()> {
        let entry = &self.har.log.entries[self.get_entry_index()];
        let (body, mime) = match self.tabbar_state {
            TabBarState::Request => {
                let text = entry.request.post_data.as_ref().map(|p| p.text.clone()).unwrap_or_default();
                let mime = entry.request.post_data.as_ref().map(|p| p.mime_type.clone()).unwrap_or_default();
                (text, mime)
            }
            TabBarState::Response => {
                let text = self.to_response_body(self.get_entry_index()).unwrap_or_else(|| "No response body".to_string());
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

        execute!(std::io::stderr(), LeaveAlternateScreen, DisableMouseCapture)?;
        disable_raw_mode()?;

        let status = Command::new(editor).arg(temp_file.path()).status();

        enable_raw_mode()?;
        execute!(std::io::stderr(), EnterAlternateScreen, EnableMouseCapture)?;

        self.should_redraw = true;

        if let Err(e) = status {
            eprintln!("Failed to open editor: {}", e);
        }

        Ok(())
    }

    // ── Preview text ─────────────────────────────────────────────────────────

    pub fn get_preview_text(&mut self) -> &Text<'static> {
        if self.display_entry_indices.is_empty() {
            self.cached_preview_text = Some(Text::raw("No matching entries."));
            self.cached_key = None;
            return self.cached_preview_text.as_ref().unwrap();
        }

        let key = (self.get_entry_index(), self.tabbar_state);
        if self.cached_key == Some(key) && self.cached_preview_text.is_some() {
            return self.cached_preview_text.as_ref().unwrap();
        }

        let text_content: String;
        let mime_type: String;

        let entry_idx = self.get_entry_index();

        match self.tabbar_state {
            TabBarState::Request => {
                let entry = &self.har.log.entries[entry_idx];
                text_content = entry.request.post_data.as_ref().map(|p| p.text.clone()).unwrap_or_else(|| "No request body".to_string());
                mime_type = entry.request.post_data.as_ref().map(|p| p.mime_type.clone()).unwrap_or_default();
            }
            TabBarState::Response => {
                text_content = self.to_response_body(entry_idx).unwrap_or_else(|| "No response body".to_string());
                let entry = &self.har.log.entries[entry_idx];
                mime_type = entry.response.content.mime_type.clone().unwrap_or_default();
            }
            _ => {
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
        self.cached_preview_text = None;
    }

    pub fn set_tabbar_state(&mut self, state: TabBarState) {
        self.tabbar_state = state;
        self.scroll = 0;
        self.cached_preview_text = None;
    }

    // ── Data helpers ─────────────────────────────────────────────────────────

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

// ── Free function: per-entry match ────────────────────────────────────────────

pub fn entry_matches(entry: &har::Entry, scope: SearchScope, re: &regex::Regex) -> bool {
    match scope {
        SearchScope::All => {
            if re.is_match(entry.request.url.as_str()) { return true; }
            if entry.request.url.host_str().map_or(false, |h| re.is_match(h)) { return true; }
            let qs: String = entry.request.query_string.iter()
                .map(|q| format!("{}={}", q.name, q.value))
                .collect::<Vec<_>>()
                .join("&");
            if re.is_match(&qs) { return true; }
            if entry.request.headers.iter().any(|h| re.is_match(&format!("{}: {}", h.name, h.value))) { return true; }
            if entry.response.headers.iter().any(|h| re.is_match(&format!("{}: {}", h.name, h.value))) { return true; }
            if let Some(pd) = &entry.request.post_data {
                if re.is_match(&pd.text) { return true; }
            }
            if let Some(text) = &entry.response.content.text {
                let body = decode_body(text, entry.response.content.encoding.as_deref());
                if re.is_match(&body) { return true; }
            }
            if re.is_match(&entry.request.method) { return true; }
            if re.is_match(&entry.response.status.to_string()) { return true; }
            if let Some(sz) = entry.request.body_size {
                if re.is_match(&sz.to_string()) { return true; }
            }
            if let Some(sz) = entry.response.content.size {
                if re.is_match(&sz.to_string()) { return true; }
            }
            if re.is_match(&format!("{:.0}", entry.time)) { return true; }
            false
        }
        SearchScope::Url => re.is_match(entry.request.url.as_str()),
        SearchScope::Host => entry.request.url.host_str().map_or(false, |h| re.is_match(h)),
        SearchScope::QueryString => {
            let qs: String = entry.request.query_string.iter()
                .map(|q| format!("{}={}", q.name, q.value))
                .collect::<Vec<_>>()
                .join("&");
            re.is_match(&qs)
        }
        SearchScope::RequestHeaders => entry.request.headers.iter()
            .any(|h| re.is_match(&format!("{}: {}", h.name, h.value))),
        SearchScope::ResponseHeaders => entry.response.headers.iter()
            .any(|h| re.is_match(&format!("{}: {}", h.name, h.value))),
        SearchScope::RequestBody => entry.request.post_data.as_ref()
            .map_or(false, |pd| re.is_match(&pd.text)),
        SearchScope::ResponseBody => {
            entry.response.content.text.as_ref().map_or(false, |text| {
                let body = decode_body(text, entry.response.content.encoding.as_deref());
                re.is_match(&body)
            })
        }
        SearchScope::Method => re.is_match(&entry.request.method),
        SearchScope::StatusCode => re.is_match(&entry.response.status.to_string()),
        SearchScope::RequestBodySize => entry.request.body_size
            .map_or(false, |sz| re.is_match(&sz.to_string())),
        SearchScope::ResponseBodySize => entry.response.content.size
            .map_or(false, |sz| re.is_match(&sz.to_string())),
        SearchScope::Duration => re.is_match(&format!("{:.0}", entry.time)),
    }
}

fn decode_body(text: &str, encoding: Option<&str>) -> String {
    if encoding == Some("base64") {
        use base64::prelude::*;
        BASE64_STANDARD.decode(text)
            .ok()
            .and_then(|b| String::from_utf8(b).ok())
            .unwrap_or_else(|| text.to_string())
    } else {
        text.to_string()
    }
}

// ── TableItem ─────────────────────────────────────────────────────────────────

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

    const MAX_HIGHLIGHT_BYTES: usize = 200_000;

    let ps = &*SYNTAX_SET;
    let ts = &*THEME_SET;
    let mime_type = mime_type.to_lowercase();

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
            highlighter.highlight_line(line, ps).unwrap_or_default();
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

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::har;
    use ratatui::prelude::Rect;

    // ── Helpers ───────────────────────────────────────────────────────────────

    fn re(pattern: &str) -> regex::Regex {
        regex::Regex::new(pattern).unwrap()
    }

    /// Build a fully-populated Entry for testing search scopes.
    fn make_entry() -> har::Entry {
        har::Entry {
            started_date_time: "2024-06-01T12:00:00.000Z".to_string(),
            time: 75.0,
            request: har::Request {
                method: "POST".to_string(),
                url: url::Url::parse(
                    "https://api.example.com/v1/users?page=2&limit=50",
                )
                .unwrap(),
                http_version: "HTTP/1.1".to_string(),
                headers: vec![
                    har::Header {
                        name: "Authorization".to_string(),
                        value: "Bearer secret-token".to_string(),
                    },
                    har::Header {
                        name: "Content-Type".to_string(),
                        value: "application/json".to_string(),
                    },
                ],
                cookies: vec![],
                query_string: vec![
                    har::QueryString {
                        name: "page".to_string(),
                        value: "2".to_string(),
                    },
                    har::QueryString {
                        name: "limit".to_string(),
                        value: "50".to_string(),
                    },
                ],
                headers_size: None,
                body_size: Some(42),
                post_data: Some(har::PostData {
                    mime_type: "application/json".to_string(),
                    params: None,
                    text: r#"{"username":"alice","role":"admin"}"#.to_string(),
                }),
            },
            response: har::Response {
                status: 201,
                status_text: "Created".to_string(),
                http_version: "HTTP/1.1".to_string(),
                headers: vec![
                    har::Header {
                        name: "Content-Type".to_string(),
                        value: "application/json".to_string(),
                    },
                    har::Header {
                        name: "X-Request-Id".to_string(),
                        value: "req-abc-123".to_string(),
                    },
                ],
                cookies: vec![],
                content: har::Content {
                    mime_type: Some("application/json".to_string()),
                    size: Some(512),
                    text: Some(r#"{"id":99,"name":"Alice","active":true}"#.to_string()),
                    encoding: None,
                },
                redirect_url: String::new(),
                headers_size: None,
                body_size: Some(512),
            },
            cache: har::Cache {},
            timings: har::Timings {
                blocked: None,
                dns: Some(2.0),
                ssl: Some(8.0),
                connect: Some(12.0),
                send: Some(1.0),
                wait: Some(50.0),
                receive: Some(2.0),
            },
            security_state: None,
            pageref: None,
            server_ipaddress: None,
            connection: None,
        }
    }

    /// Build an App from a list of entries with a sensible window size set.
    fn make_app(entries: Vec<har::Entry>) -> App {
        let har = Har {
            log: har::Log {
                version: Some("1.2".to_string()),
                creator: None,
                browser: None,
                pages: None,
                entries,
            },
        };
        let mut app = App::init(har);
        app.window_size = Rect::new(0, 0, 220, 50);
        app
    }

    fn push_str(app: &mut App, s: &str) {
        app.enter_search_mode();
        for c in s.chars() {
            app.push_search_char(c);
        }
    }

    // ── decode_body ───────────────────────────────────────────────────────────

    #[test]
    fn decode_body_plain_passthrough() {
        assert_eq!(decode_body("hello world", None), "hello world");
        assert_eq!(decode_body("hello world", Some("utf-8")), "hello world");
    }

    #[test]
    fn decode_body_base64_roundtrip() {
        use base64::prelude::*;
        let original = "Hello, base64!";
        let encoded = BASE64_STANDARD.encode(original);
        assert_eq!(decode_body(&encoded, Some("base64")), original);
    }

    #[test]
    fn decode_body_invalid_base64_returns_original() {
        let garbage = "not!!valid??base64@@";
        assert_eq!(decode_body(garbage, Some("base64")), garbage);
    }

    // ── SearchScope::next() cycles ────────────────────────────────────────────

    #[test]
    fn search_scope_cycles_all_variants() {
        let start = SearchScope::All;
        let mut scope = start;
        let expected = [
            SearchScope::Url,
            SearchScope::Host,
            SearchScope::QueryString,
            SearchScope::RequestHeaders,
            SearchScope::ResponseHeaders,
            SearchScope::RequestBody,
            SearchScope::ResponseBody,
            SearchScope::Method,
            SearchScope::StatusCode,
            SearchScope::RequestBodySize,
            SearchScope::ResponseBodySize,
            SearchScope::Duration,
            SearchScope::All, // wraps back
        ];
        for expected_next in expected {
            scope = scope.next();
            assert_eq!(scope, expected_next);
        }
    }

    // ── entry_matches – individual scopes ────────────────────────────────────

    #[test]
    fn match_scope_url_hit_and_miss() {
        let e = make_entry();
        assert!(entry_matches(&e, SearchScope::Url, &re("api\\.example\\.com")));
        assert!(entry_matches(&e, SearchScope::Url, &re("v1/users")));
        assert!(!entry_matches(&e, SearchScope::Url, &re("other\\.com")));
    }

    #[test]
    fn match_scope_host_only() {
        let e = make_entry();
        // Should match on the host part
        assert!(entry_matches(&e, SearchScope::Host, &re("example\\.com")));
        // Should NOT match a path segment when searching by host
        assert!(!entry_matches(&e, SearchScope::Host, &re("v1/users")));
    }

    #[test]
    fn match_scope_query_string() {
        let e = make_entry();
        assert!(entry_matches(&e, SearchScope::QueryString, &re("page=2")));
        assert!(entry_matches(&e, SearchScope::QueryString, &re("limit")));
        assert!(!entry_matches(&e, SearchScope::QueryString, &re("offset")));
    }

    #[test]
    fn match_scope_request_headers_by_name() {
        let e = make_entry();
        assert!(entry_matches(&e, SearchScope::RequestHeaders, &re("Authorization")));
        assert!(!entry_matches(&e, SearchScope::RequestHeaders, &re("X-Request-Id")));
    }

    #[test]
    fn match_scope_request_headers_by_value() {
        let e = make_entry();
        assert!(entry_matches(&e, SearchScope::RequestHeaders, &re("secret-token")));
        assert!(entry_matches(&e, SearchScope::RequestHeaders, &re("application/json")));
    }

    #[test]
    fn match_scope_request_headers_name_and_value_combined() {
        let e = make_entry();
        // The format is "Name: Value", so this should match
        assert!(entry_matches(
            &e,
            SearchScope::RequestHeaders,
            &re("Content-Type: application/json")
        ));
    }

    #[test]
    fn match_scope_response_headers() {
        let e = make_entry();
        assert!(entry_matches(&e, SearchScope::ResponseHeaders, &re("X-Request-Id")));
        assert!(entry_matches(&e, SearchScope::ResponseHeaders, &re("req-abc-123")));
        assert!(!entry_matches(&e, SearchScope::ResponseHeaders, &re("Authorization")));
    }

    #[test]
    fn match_scope_request_body() {
        let e = make_entry();
        assert!(entry_matches(&e, SearchScope::RequestBody, &re("alice")));
        assert!(entry_matches(&e, SearchScope::RequestBody, &re("admin")));
        assert!(!entry_matches(&e, SearchScope::RequestBody, &re("bob")));
    }

    #[test]
    fn match_scope_response_body() {
        let e = make_entry();
        assert!(entry_matches(&e, SearchScope::ResponseBody, &re("Alice")));
        assert!(entry_matches(&e, SearchScope::ResponseBody, &re("\"active\":true")));
        assert!(!entry_matches(&e, SearchScope::ResponseBody, &re("inactive")));
    }

    #[test]
    fn match_scope_method() {
        let e = make_entry();
        assert!(entry_matches(&e, SearchScope::Method, &re("POST")));
        assert!(entry_matches(&e, SearchScope::Method, &re("^POST$")));
        assert!(!entry_matches(&e, SearchScope::Method, &re("^GET$")));
    }

    #[test]
    fn match_scope_status_code() {
        let e = make_entry();
        assert!(entry_matches(&e, SearchScope::StatusCode, &re("201")));
        assert!(entry_matches(&e, SearchScope::StatusCode, &re("^2")));   // 2xx
        assert!(!entry_matches(&e, SearchScope::StatusCode, &re("^4")));  // not 4xx
    }

    #[test]
    fn match_scope_request_body_size() {
        let e = make_entry(); // body_size = Some(42)
        assert!(entry_matches(&e, SearchScope::RequestBodySize, &re("42")));
        assert!(!entry_matches(&e, SearchScope::RequestBodySize, &re("999")));
    }

    #[test]
    fn match_scope_response_body_size() {
        let e = make_entry(); // content.size = Some(512)
        assert!(entry_matches(&e, SearchScope::ResponseBodySize, &re("512")));
        assert!(!entry_matches(&e, SearchScope::ResponseBodySize, &re("^42$")));
    }

    #[test]
    fn match_scope_duration() {
        let e = make_entry(); // time = 75.0 → "75"
        assert!(entry_matches(&e, SearchScope::Duration, &re("75")));
        assert!(entry_matches(&e, SearchScope::Duration, &re("^75$")));
        assert!(!entry_matches(&e, SearchScope::Duration, &re("^100$")));
    }

    #[test]
    fn match_scope_all_catches_each_field() {
        let e = make_entry();
        // URL
        assert!(entry_matches(&e, SearchScope::All, &re("v1/users")));
        // Host
        assert!(entry_matches(&e, SearchScope::All, &re("example\\.com")));
        // Query string
        assert!(entry_matches(&e, SearchScope::All, &re("page=2")));
        // Request header name
        assert!(entry_matches(&e, SearchScope::All, &re("Authorization")));
        // Response header value
        assert!(entry_matches(&e, SearchScope::All, &re("req-abc-123")));
        // Request body
        assert!(entry_matches(&e, SearchScope::All, &re("admin")));
        // Response body
        assert!(entry_matches(&e, SearchScope::All, &re("\"active\":true")));
        // Method
        assert!(entry_matches(&e, SearchScope::All, &re("^POST$")));
        // Status
        assert!(entry_matches(&e, SearchScope::All, &re("^201$")));
        // Duration
        assert!(entry_matches(&e, SearchScope::All, &re("^75$")));
        // Nothing should match this
        assert!(!entry_matches(&e, SearchScope::All, &re("XYZZY_NOMATCH_8675309")));
    }

    #[test]
    fn match_regex_patterns() {
        let e = make_entry();
        // Case-sensitive by default
        assert!(entry_matches(&e, SearchScope::Url, &re("example")));
        assert!(!entry_matches(&e, SearchScope::Url, &re("EXAMPLE")));
        // Case-insensitive via inline flag
        assert!(entry_matches(&e, SearchScope::Url, &re("(?i)EXAMPLE")));
        // Anchored patterns
        assert!(entry_matches(&e, SearchScope::Method, &re("^POST$")));
        assert!(!entry_matches(&e, SearchScope::Method, &re("^post$")));
        // Alternation
        assert!(entry_matches(&e, SearchScope::StatusCode, &re("200|201")));
        // Digit class
        assert!(entry_matches(&e, SearchScope::StatusCode, &re(r"\d{3}")));
    }

    // ── App filter state ─────────────────────────────────────────────────────

    #[test]
    fn filter_reduces_display_count() {
        let mut app = make_app(vec![
            make_entry(), // POST /v1/users  201
            {
                let mut e = make_entry();
                e.request.method = "GET".to_string();
                e.response.status = 404;
                e
            },
        ]);
        assert_eq!(app.max_index(), 2);

        push_str(&mut app, "^GET$");

        assert_eq!(app.display_entry_indices.len(), 1);
        assert_eq!(app.max_index(), 1);
    }

    #[test]
    fn filter_shows_all_when_query_cleared() {
        let mut app = make_app(vec![make_entry(), make_entry()]);
        push_str(&mut app, "NOMATCH_EVER");
        assert_eq!(app.max_index(), 0);

        // Backspace until empty
        for _ in 0.."NOMATCH_EVER".len() {
            app.pop_search_char();
        }
        assert_eq!(app.max_index(), 2);
        assert!(!app.search_active);
        assert!(app.search_regex.is_none());
    }

    #[test]
    fn invalid_regex_does_not_crash_or_change_display() {
        let mut app = make_app(vec![make_entry()]);
        // Apply a valid filter first
        push_str(&mut app, "POST");
        assert_eq!(app.max_index(), 1);
        assert!(!app.search_error);

        // Now type an invalid regex character — add an unclosed '('
        app.push_search_char('(');
        assert!(app.search_error);
        // Display count must be unchanged (still showing previous valid result)
        assert_eq!(app.max_index(), 1);
    }

    #[test]
    fn clear_search_restores_all_entries() {
        let mut app = make_app(vec![make_entry(), make_entry(), make_entry()]);
        push_str(&mut app, "NOMATCH");
        assert_eq!(app.max_index(), 0);

        app.confirm_search(); // exit search mode
        app.clear_search();

        assert_eq!(app.max_index(), 3);
        assert!(!app.search_active);
        assert!(app.search_query.is_empty());
        assert!(app.search_regex.is_none());
    }

    #[test]
    fn cancel_search_restores_pre_search_state() {
        let mut e1 = make_entry(); // POST 201
        let mut e2 = make_entry();
        e2.request.method = "DELETE".to_string(); // unique method not in any other field
        let mut app = make_app(vec![e1, e2]);
        assert_eq!(app.max_index(), 2);

        // Filter to DELETE only — narrows to 1
        push_str(&mut app, "^DELETE$");
        assert_eq!(app.max_index(), 1);

        // Cancel should restore all 2 entries
        app.cancel_search();
        assert_eq!(app.max_index(), 2);
        assert!(!app.search_active);
    }

    #[test]
    fn get_entry_index_maps_display_to_original() {
        let entries = vec![
            {
                let mut e = make_entry();
                e.request.method = "GET".to_string();
                e
            },
            make_entry(), // POST — original index 1
            {
                let mut e = make_entry();
                e.request.method = "DELETE".to_string();
                e
            },
        ];
        let mut app = make_app(entries);

        // Filter to POST only → display index 0 should map to original index 1
        push_str(&mut app, "^POST$");
        assert_eq!(app.max_index(), 1);
        assert_eq!(app.get_entry_index(), 1); // original HAR index of the POST entry
    }

    #[test]
    fn navigation_stays_within_filtered_results() {
        let mut app = make_app(vec![
            make_entry(),
            make_entry(),
            make_entry(),
        ]);
        push_str(&mut app, "POST");
        app.confirm_search();

        // All 3 match POST; navigate past the end
        app.update_index(10);
        assert_eq!(app.get_index(), 2); // clamped at last filtered entry

        app.update_index(-10);
        assert_eq!(app.get_index(), 0); // clamped at first
    }

    // ── to_response_body base64 ───────────────────────────────────────────────

    #[test]
    fn to_response_body_plain_text() {
        let app = make_app(vec![make_entry()]);
        let body = app.to_response_body(0).unwrap();
        assert_eq!(body, r#"{"id":99,"name":"Alice","active":true}"#);
    }

    #[test]
    fn to_response_body_decodes_base64() {
        use base64::prelude::*;
        let original = r#"{"secret":"value"}"#;
        let mut e = make_entry();
        e.response.content.text = Some(BASE64_STANDARD.encode(original));
        e.response.content.encoding = Some("base64".to_string());
        let app = make_app(vec![e]);
        assert_eq!(app.to_response_body(0).unwrap(), original);
    }

    #[test]
    fn to_response_body_none_when_no_text() {
        let mut e = make_entry();
        e.response.content.text = None;
        let app = make_app(vec![e]);
        assert!(app.to_response_body(0).is_none());
    }

    // ── generate_table_items ─────────────────────────────────────────────────

    #[test]
    fn generate_table_items_count_matches_entries() {
        let app = make_app(vec![make_entry(), make_entry(), make_entry()]);
        assert_eq!(app.table_items.len(), 3);
    }

    #[test]
    fn generate_table_items_captures_method_and_status() {
        let app = make_app(vec![make_entry()]);
        assert_eq!(app.table_items[0].method, "POST");
        assert_eq!(app.table_items[0].status, 201);
    }

    #[test]
    fn generate_table_items_url_is_full_url() {
        let app = make_app(vec![make_entry()]);
        assert!(app.table_items[0].url.contains("api.example.com"));
        assert!(app.table_items[0].url.contains("v1/users"));
    }
}
