use ratatui::widgets::ScrollbarState;

use crate::models::{ChatSession, Message, Retrospection};

/// State for the session detail view
#[derive(Debug)]
pub struct SessionDetailState {
    /// The session being displayed
    pub session: Option<ChatSession>,
    /// Messages in this session
    pub messages: Vec<Message>,
    /// Retrospection analyses for this session
    pub retrospections: Vec<Retrospection>,
    /// Currently selected session ID
    pub session_id: Option<String>,
    /// Scrollbar state for messages
    pub scroll_state: ScrollbarState,
    /// Current scroll position (line number)
    pub current_scroll: usize,
    /// Scroll position for retrospection panel
    pub retrospection_scroll: usize,
    /// Loading indicator
    pub loading: bool,
    /// Whether to wrap message text
    pub message_wrap: bool,
    /// Whether to show the retrospection panel
    pub show_retrospection: bool,
}

impl SessionDetailState {
    /// Create a new session detail state with default values
    pub fn new() -> Self {
        Self {
            session: None,
            messages: Vec::new(),
            retrospections: Vec::new(),
            session_id: None,
            scroll_state: ScrollbarState::default(),
            current_scroll: 0,
            retrospection_scroll: 0,
            loading: false,
            message_wrap: true,
            show_retrospection: false,
        }
    }

    /// Set the session ID and clear current data
    pub fn set_session_id(&mut self, session_id: Option<String>) {
        self.session_id = session_id;
        if self.session_id.is_some() {
            // Clear old data when switching sessions
            self.session = None;
            self.messages.clear();
            self.retrospections.clear();
            self.current_scroll = 0;
            self.retrospection_scroll = 0;
        }
    }

    /// Update the session data from query result
    pub fn update_session(&mut self, session: ChatSession, messages: Vec<Message>) {
        self.session = Some(session);
        self.messages = messages;
        self.current_scroll = 0;
    }

    /// Update retrospections from query result
    pub fn update_retrospections(&mut self, retrospections: Vec<Retrospection>) {
        self.retrospections = retrospections;
    }

    /// Scroll up one line
    pub fn scroll_up(&mut self) {
        if self.current_scroll > 0 {
            self.current_scroll -= 1;
        }
    }

    /// Scroll down one line
    pub fn scroll_down(&mut self, max_scroll: usize) {
        if self.current_scroll < max_scroll {
            self.current_scroll += 1;
        }
    }

    /// Scroll up by a page
    pub fn scroll_page_up(&mut self, page_size: usize) {
        self.current_scroll = self.current_scroll.saturating_sub(page_size);
    }

    /// Scroll down by a page
    pub fn scroll_page_down(&mut self, page_size: usize, max_scroll: usize) {
        self.current_scroll = (self.current_scroll + page_size).min(max_scroll);
    }

    /// Scroll to the top
    pub fn scroll_to_top(&mut self) {
        self.current_scroll = 0;
    }

    /// Scroll to the bottom
    pub fn scroll_to_bottom(&mut self, max_scroll: usize) {
        self.current_scroll = max_scroll;
    }

    /// Toggle word wrap
    pub fn toggle_wrap(&mut self) {
        self.message_wrap = !self.message_wrap;
    }

    /// Toggle retrospection panel visibility
    pub fn toggle_retrospection(&mut self) {
        self.show_retrospection = !self.show_retrospection;
    }

    /// Update the scrollbar state
    pub fn update_scroll_state(&mut self, total_lines: usize) {
        self.scroll_state = self.scroll_state.content_length(total_lines);
        self.scroll_state = self.scroll_state.position(self.current_scroll);
    }
}

impl Default for SessionDetailState {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_session_detail_state_default() {
        let state = SessionDetailState::new();
        assert!(state.session.is_none());
        assert!(state.messages.is_empty());
        assert!(state.retrospections.is_empty());
        assert_eq!(state.current_scroll, 0);
        assert!(state.message_wrap);
        assert!(!state.show_retrospection);
        assert!(!state.loading);
    }

    #[test]
    fn test_set_session_id_clears_data() {
        let mut state = SessionDetailState::new();
        state.current_scroll = 10;
        state.retrospection_scroll = 5;

        state.set_session_id(Some("new_session".to_string()));

        assert_eq!(state.session_id, Some("new_session".to_string()));
        assert_eq!(state.current_scroll, 0);
        assert_eq!(state.retrospection_scroll, 0);
    }

    #[test]
    fn test_scrolling() {
        let mut state = SessionDetailState::new();
        let max_scroll = 100;

        // Scroll down
        state.scroll_down(max_scroll);
        assert_eq!(state.current_scroll, 1);

        // Scroll up
        state.scroll_up();
        assert_eq!(state.current_scroll, 0);

        // Can't scroll up past 0
        state.scroll_up();
        assert_eq!(state.current_scroll, 0);

        // Scroll to bottom
        state.scroll_to_bottom(max_scroll);
        assert_eq!(state.current_scroll, 100);

        // Can't scroll down past max
        state.scroll_down(max_scroll);
        assert_eq!(state.current_scroll, 100);

        // Scroll to top
        state.scroll_to_top();
        assert_eq!(state.current_scroll, 0);
    }

    #[test]
    fn test_page_scrolling() {
        let mut state = SessionDetailState::new();
        let max_scroll = 100;
        let page_size = 10;

        state.scroll_page_down(page_size, max_scroll);
        assert_eq!(state.current_scroll, 10);

        state.scroll_page_down(page_size, max_scroll);
        assert_eq!(state.current_scroll, 20);

        state.scroll_page_up(page_size);
        assert_eq!(state.current_scroll, 10);
    }

    #[test]
    fn test_toggle_wrap() {
        let mut state = SessionDetailState::new();
        assert!(state.message_wrap);

        state.toggle_wrap();
        assert!(!state.message_wrap);

        state.toggle_wrap();
        assert!(state.message_wrap);
    }

    #[test]
    fn test_toggle_retrospection() {
        let mut state = SessionDetailState::new();
        assert!(!state.show_retrospection);

        state.toggle_retrospection();
        assert!(state.show_retrospection);

        state.toggle_retrospection();
        assert!(!state.show_retrospection);
    }
}
