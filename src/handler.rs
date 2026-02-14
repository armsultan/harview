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
    TabNext,
    TabPrev,
}

impl Command {
    pub fn exec(&self, app: &mut app::App) {
        match self {
            Self::Quit => app.quit(),
            Self::TableFocusTop => app.update_index_first(),
            Self::TableFocusBottom => app.update_index_last(),
            Self::TableFocusDelta(count) => app.update_index(*count),
            Self::SetTabBarState(state) => app.set_tabbar_state(state),
            Self::ScrollUp => app.on_up(),
            Self::ScrollDown => app.on_down(),
            Self::PageUp => app.on_page_up(),
            Self::PageDown => app.on_page_down(),
            Self::OpenInFx => app.open_in_fx(),
            Self::TabNext => app.next_tab(),
            Self::TabPrev => app.prev_tab(),
        }
    }
}

pub fn handle_key_events(key_event: KeyEvent) -> Option<Command> {
    match key_event.code {
        KeyCode::Char('q') => Some(Command::Quit),
        KeyCode::Char('c') | KeyCode::Char('C') => {
            if key_event.modifiers == KeyModifiers::CONTROL {
                Some(Command::Quit)
            } else {
                None
            }
        }
        KeyCode::Char('j') => Some(Command::TableFocusDelta(1)),
        KeyCode::Char('k') => Some(Command::TableFocusDelta(-1)),
        KeyCode::Up if key_event.modifiers.contains(KeyModifiers::SHIFT) => Some(Command::ScrollUp),
        KeyCode::Down if key_event.modifiers.contains(KeyModifiers::SHIFT) => Some(Command::ScrollDown),
        KeyCode::Char('J') => Some(Command::OpenInFx),
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
            // Check for Tab Click
            // Tab bar is at the top of the second pane?
            // In ui.rs: layout[0] is table, layout[1] is preview. 
            // layout[1] is split into tabbar (1 line) and content.
            // So tab bar is at split_y.
            
            if mouse_event.row == split_y {
                // Approximate tab widths: " [1] Headers " is 13 chars.
                // Padding 1 space each side.
                // 0-14: Headers
                // 15-28: Cookies
                // 29-42: Request
                // 43-57: Response
                let x = mouse_event.column;
                if x < 15 {
                    Some(Command::SetTabBarState(app::TabBarState::Headers))
                } else if x < 29 {
                    Some(Command::SetTabBarState(app::TabBarState::Cookies))
                } else if x < 43 {
                    Some(Command::SetTabBarState(app::TabBarState::Request))
                } else if x < 58 {
                    Some(Command::SetTabBarState(app::TabBarState::Response))
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
