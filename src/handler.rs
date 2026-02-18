use crate::app;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers, MouseEvent, MouseEventKind};

#[derive(Debug)]
pub enum Command {
    Quit,
    TableFocusDelta(i32),
    TableFocusTop,
    TableFocusBottom,
    SetTabBarState(app::TabBarState),
    ScrollUp,
    ScrollDown,
    PageUp,
    PageDown,
    OpenInFx,
    OpenInBat,
    OpenInEditor,
    TabNext,
    TabPrev,
    ToggleSyntaxHighlighting,
    SetTableIndex(usize),
    // Search
    EnterSearchMode,
    SearchChar(char),
    SearchBackspace,
    SearchConfirm,
    SearchCancel,
    SearchCycleScope,
    ClearSearch,
}

impl Command {
    pub fn exec(&self, app: &mut app::App) {
        match self {
            Self::Quit => app.quit(),
            Self::TableFocusTop => app.update_index_first(),
            Self::TableFocusBottom => app.update_index_last(),
            Self::TableFocusDelta(count) => app.update_index(*count),
            Self::SetTabBarState(state) => app.set_tabbar_state(*state),
            Self::ScrollUp => app.on_up(),
            Self::ScrollDown => app.on_down(),
            Self::PageUp => app.on_page_up(),
            Self::PageDown => app.on_page_down(),
            Self::OpenInFx => {
                app.pending_action = Some(app::PendingAction::OpenInFx);
            }
            Self::OpenInBat => {
                app.pending_action = Some(app::PendingAction::OpenInBat);
            }
            Self::OpenInEditor => {
                app.pending_action = Some(app::PendingAction::OpenInEditor);
            }
            Self::TabNext => app.next_tab(),
            Self::TabPrev => app.prev_tab(),
            Self::ToggleSyntaxHighlighting => app.toggle_syntax_highlighting(),
            Self::SetTableIndex(index) => app.update_index_absolute(*index),
            Self::EnterSearchMode => app.enter_search_mode(),
            Self::SearchChar(c) => app.push_search_char(*c),
            Self::SearchBackspace => app.pop_search_char(),
            Self::SearchConfirm => app.confirm_search(),
            Self::SearchCancel => app.cancel_search(),
            Self::SearchCycleScope => app.cycle_search_scope(),
            Self::ClearSearch => app.clear_search(),
        }
    }
}

pub fn handle_key_events(key_event: KeyEvent, app: &app::App) -> Option<Command> {
    // In search mode, most keys are captured for the query input.
    if app.search_mode {
        return handle_search_key(key_event);
    }

    // Normal mode
    match key_event.code {
        KeyCode::Char('q') => Some(Command::Quit),
        KeyCode::Char('c') | KeyCode::Char('C') => {
            if key_event.modifiers == KeyModifiers::CONTROL {
                Some(Command::Quit)
            } else {
                None
            }
        }
        KeyCode::Char('/') => Some(Command::EnterSearchMode),
        KeyCode::Esc => {
            if app.search_active {
                Some(Command::ClearSearch)
            } else {
                None
            }
        }
        KeyCode::Char('j') => Some(Command::TableFocusDelta(1)),
        KeyCode::Char('k') => Some(Command::TableFocusDelta(-1)),
        KeyCode::Up if key_event.modifiers.contains(KeyModifiers::SHIFT) => Some(Command::ScrollUp),
        KeyCode::Down if key_event.modifiers.contains(KeyModifiers::SHIFT) => {
            Some(Command::ScrollDown)
        }
        KeyCode::Char('J') => Some(Command::OpenInFx),
        KeyCode::Char('b') => Some(Command::OpenInBat),
        KeyCode::Char('o') => Some(Command::OpenInEditor),
        KeyCode::Down => Some(Command::TableFocusDelta(1)),
        KeyCode::Up => Some(Command::TableFocusDelta(-1)),
        KeyCode::Char('d') => Some(Command::TableFocusDelta(3)),
        KeyCode::Char('u') => Some(Command::TableFocusDelta(-3)),
        KeyCode::Char('g') => Some(Command::TableFocusTop),
        KeyCode::Char('G') => Some(Command::TableFocusBottom),
        KeyCode::Char('1') => Some(Command::SetTabBarState(app::TabBarState::Headers)),
        KeyCode::Char('2') => Some(Command::SetTabBarState(app::TabBarState::Cookies)),
        KeyCode::Char('3') => Some(Command::SetTabBarState(app::TabBarState::Request)),
        KeyCode::Char('4') => Some(Command::SetTabBarState(app::TabBarState::Response)),
        KeyCode::Right => Some(Command::TabNext),
        KeyCode::Left => Some(Command::TabPrev),
        KeyCode::PageUp => Some(Command::PageUp),
        KeyCode::PageDown => Some(Command::PageDown),
        KeyCode::Char('h') => Some(Command::ToggleSyntaxHighlighting),
        KeyCode::Char('?') => Some(Command::SetTabBarState(app::TabBarState::Help)),
        _ => None,
    }
}

fn handle_search_key(key_event: KeyEvent) -> Option<Command> {
    match key_event.code {
        KeyCode::Enter => Some(Command::SearchConfirm),
        KeyCode::Esc => Some(Command::SearchCancel),
        KeyCode::Tab => Some(Command::SearchCycleScope),
        KeyCode::Backspace => Some(Command::SearchBackspace),
        KeyCode::Char(c) => {
            // Pass through Ctrl+C as quit even in search mode
            if key_event.modifiers == KeyModifiers::CONTROL && (c == 'c' || c == 'C') {
                Some(Command::Quit)
            } else if key_event.modifiers.is_empty() || key_event.modifiers == KeyModifiers::SHIFT {
                Some(Command::SearchChar(c))
            } else {
                None
            }
        }
        _ => None,
    }
}

pub fn handle_mouse_events(app: &mut app::App, mouse_event: MouseEvent) -> Option<Command> {
    let split_y = app.window_size.height / 2;

    // Update Focus
    if mouse_event.row < split_y {
        app.active_focus = app::ActiveFocus::Table;
    } else {
        app.active_focus = app::ActiveFocus::Preview;
    }

    match mouse_event.kind {
        MouseEventKind::ScrollDown => {
            if mouse_event.row < split_y {
                Some(Command::TableFocusDelta(1))
            } else {
                Some(Command::ScrollDown)
            }
        }
        MouseEventKind::ScrollUp => {
            if mouse_event.row < split_y {
                Some(Command::TableFocusDelta(-1))
            } else {
                Some(Command::ScrollUp)
            }
        }
        MouseEventKind::Down(crossterm::event::MouseButton::Left) => {
            if mouse_event.row >= split_y.saturating_sub(1) && mouse_event.row <= split_y + 1 {
                let x = mouse_event.column;
                if x < 15 {
                    Some(Command::SetTabBarState(app::TabBarState::Headers))
                } else if x < 30 {
                    Some(Command::SetTabBarState(app::TabBarState::Cookies))
                } else if x < 45 {
                    Some(Command::SetTabBarState(app::TabBarState::Request))
                } else if x < 61 {
                    Some(Command::SetTabBarState(app::TabBarState::Response))
                } else if x < 74 {
                    Some(Command::SetTabBarState(app::TabBarState::Help))
                } else {
                    None
                }
            } else if mouse_event.row < split_y {
                let header_height = 2; // Border + Header row
                if mouse_event.row >= header_height {
                    let clicked_row = (mouse_event.row - header_height) as usize;
                    let target_index = app.table_offset + clicked_row;
                    // Bounds check against filtered display list
                    if target_index < app.display_entry_indices.len() {
                        Some(Command::SetTableIndex(target_index))
                    } else {
                        None
                    }
                } else {
                    None
                }
            } else {
                None
            }
        }
        _ => None,
    }
}
