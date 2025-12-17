use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use super::event::{AppEvent, NavigationDirection, TabDirection, UserAction};
use crate::app::AppMode;

/// Handles conversion of low-level events to high-level user actions
pub struct EventHandler;

impl EventHandler {
    /// Create a new event handler
    pub fn new() -> Self {
        Self
    }

    /// Convert an AppEvent to UserActions based on current app mode
    ///
    /// Returns a vector of actions since one event might trigger multiple actions
    pub fn handle_event(
        &self,
        event: &AppEvent,
        mode: &AppMode,
        show_help: bool,
        has_error_dialog: bool,
    ) -> Vec<UserAction> {
        match event {
            AppEvent::Input(key) => self.handle_key_event(*key, mode, show_help, has_error_dialog),
            AppEvent::Tick => vec![],
            AppEvent::Resize(_, _) => vec![UserAction::RefreshCurrentView],
        }
    }

    fn handle_key_event(
        &self,
        key: KeyEvent,
        mode: &AppMode,
        show_help: bool,
        has_error_dialog: bool,
    ) -> Vec<UserAction> {
        // Error dialog consumes all input except dismissal
        if has_error_dialog {
            return vec![UserAction::DismissDialog];
        }

        // Global key bindings
        match (key.modifiers, key.code) {
            // Quit
            (KeyModifiers::CONTROL, KeyCode::Char('c')) => return vec![UserAction::Quit],
            (KeyModifiers::NONE, KeyCode::Char('q')) if !show_help => {
                return vec![UserAction::Quit]
            }

            // Help
            (KeyModifiers::NONE, KeyCode::Char('?')) | (KeyModifiers::NONE, KeyCode::F(1)) => {
                return vec![UserAction::ToggleHelp]
            }

            // Escape
            (KeyModifiers::NONE, KeyCode::Esc) => {
                if show_help {
                    return vec![UserAction::ToggleHelp];
                } else if mode == &AppMode::SessionDetail {
                    return vec![UserAction::NavigateBack];
                }
            }

            // Tab navigation
            (KeyModifiers::NONE, KeyCode::Tab) => {
                return vec![UserAction::SwitchTab(TabDirection::Next)]
            }
            (KeyModifiers::SHIFT, KeyCode::BackTab) => {
                return vec![UserAction::SwitchTab(TabDirection::Previous)]
            }

            _ => {}
        }

        // Help screen consumes all other input
        if show_help {
            return vec![];
        }

        // Mode-specific key bindings
        match mode {
            AppMode::SessionList => self.handle_session_list_keys(key),
            AppMode::SessionDetail => self.handle_session_detail_keys(key),
            AppMode::Help => vec![],
        }
    }

    fn handle_session_list_keys(&self, key: KeyEvent) -> Vec<UserAction> {
        match key.code {
            KeyCode::Up => vec![UserAction::SessionListNavigate(NavigationDirection::Up)],
            KeyCode::Down => vec![UserAction::SessionListNavigate(NavigationDirection::Down)],
            KeyCode::PageUp => vec![UserAction::SessionListPageUp],
            KeyCode::PageDown => vec![UserAction::SessionListPageDown],
            KeyCode::Home => vec![UserAction::SessionListHome],
            KeyCode::End => vec![UserAction::SessionListEnd],
            KeyCode::Char('s') => vec![UserAction::SessionListCycleSortBy],
            KeyCode::Char('o') => vec![UserAction::SessionListToggleSortOrder],
            // Note: Enter and 'a' need session context, so they're handled in the app
            // via the session_list widget's handle_key method
            _ => vec![],
        }
    }

    fn handle_session_detail_keys(&self, _key: KeyEvent) -> Vec<UserAction> {
        // All SessionDetail keys are now handled by the widget's handle_key method
        // This allows for proper analytics-aware scrolling
        vec![]
    }
}

impl Default for EventHandler {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_quit_actions() {
        let handler = EventHandler::new();

        // Ctrl+C should always quit
        let event = AppEvent::Input(KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL));
        let actions = handler.handle_event(&event, &AppMode::SessionList, false, false);
        assert_eq!(actions, vec![UserAction::Quit]);

        // 'q' should quit when not in help
        let event = AppEvent::Input(KeyEvent::new(KeyCode::Char('q'), KeyModifiers::NONE));
        let actions = handler.handle_event(&event, &AppMode::SessionList, false, false);
        assert_eq!(actions, vec![UserAction::Quit]);

        // 'q' should not quit in help mode
        let actions = handler.handle_event(&event, &AppMode::SessionList, true, false);
        assert_eq!(actions, vec![]);
    }

    #[test]
    fn test_help_toggle() {
        let handler = EventHandler::new();

        let event = AppEvent::Input(KeyEvent::new(KeyCode::Char('?'), KeyModifiers::NONE));
        let actions = handler.handle_event(&event, &AppMode::SessionList, false, false);
        assert_eq!(actions, vec![UserAction::ToggleHelp]);
    }

    #[test]
    fn test_error_dialog_consumes_input() {
        let handler = EventHandler::new();

        let event = AppEvent::Input(KeyEvent::new(KeyCode::Char('x'), KeyModifiers::NONE));
        let actions = handler.handle_event(&event, &AppMode::SessionList, false, true);
        assert_eq!(actions, vec![UserAction::DismissDialog]);
    }

    #[test]
    fn test_session_list_navigation() {
        let handler = EventHandler::new();

        let event = AppEvent::Input(KeyEvent::new(KeyCode::Up, KeyModifiers::NONE));
        let actions = handler.handle_event(&event, &AppMode::SessionList, false, false);
        assert_eq!(
            actions,
            vec![UserAction::SessionListNavigate(NavigationDirection::Up)]
        );

        let event = AppEvent::Input(KeyEvent::new(KeyCode::Down, KeyModifiers::NONE));
        let actions = handler.handle_event(&event, &AppMode::SessionList, false, false);
        assert_eq!(
            actions,
            vec![UserAction::SessionListNavigate(NavigationDirection::Down)]
        );
    }

    #[test]
    fn test_tab_navigation() {
        let handler = EventHandler::new();

        let event = AppEvent::Input(KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE));
        let actions = handler.handle_event(&event, &AppMode::SessionList, false, false);
        assert_eq!(actions, vec![UserAction::SwitchTab(TabDirection::Next)]);
    }
}
