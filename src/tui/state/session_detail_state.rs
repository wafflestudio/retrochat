use ratatui::widgets::ScrollbarState;

use crate::models::{ChatSession, Message};
use crate::services::SessionAnalytics;

/// State for the session detail view
#[derive(Debug)]
pub struct SessionDetailState {
    /// The session being displayed
    pub session: Option<ChatSession>,
    /// Messages in this session
    pub messages: Vec<Message>,
    /// Currently selected session ID
    pub session_id: Option<String>,
    /// Analytics data for this session
    pub analytics: Option<SessionAnalytics>,
    /// Scrollbar state for messages
    pub scroll_state: ScrollbarState,
    /// Current scroll position (line number) for messages
    pub current_scroll: usize,
    /// Scrollbar state for analytics
    pub analytics_scroll_state: ScrollbarState,
    /// Current scroll position for analytics
    pub analytics_scroll: usize,
    /// Loading indicator
    pub loading: bool,
    /// Whether to show detailed tool output (expanded view)
    pub show_tool_details: bool,
    /// Whether to show analytics panel
    pub show_analytics: bool,
    /// Last known viewport height for messages (used for scroll calculations)
    pub viewport_height: usize,
    /// Last known viewport height for analytics (used for scroll calculations)
    pub analytics_viewport_height: usize,
}

impl SessionDetailState {
    /// Create a new session detail state with default values
    pub fn new() -> Self {
        Self {
            session: None,
            messages: Vec::new(),
            session_id: None,
            analytics: None,
            scroll_state: ScrollbarState::default(),
            current_scroll: 0,
            analytics_scroll_state: ScrollbarState::default(),
            analytics_scroll: 0,
            loading: false,
            show_tool_details: false,
            show_analytics: false,
            viewport_height: 20,           // Default fallback
            analytics_viewport_height: 20, // Default fallback
        }
    }

    /// Set the session ID and clear current data
    pub fn set_session_id(&mut self, session_id: Option<String>) {
        self.session_id = session_id;
        if self.session_id.is_some() {
            // Clear old data when switching sessions
            self.session = None;
            self.messages.clear();
            self.current_scroll = 0;
        }
    }

    /// Update the session data from query result
    pub fn update_session(&mut self, session: ChatSession, messages: Vec<Message>) {
        // Only reset scroll if we're switching to a different session
        let is_same_session = self
            .session
            .as_ref()
            .map(|s| s.id == session.id)
            .unwrap_or(false);

        self.session = Some(session);
        self.messages = messages;

        // Only reset scroll position when switching to a different session
        if !is_same_session {
            self.current_scroll = 0;
        }
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

    /// Toggle tool details visibility
    pub fn toggle_tool_details(&mut self) {
        self.show_tool_details = !self.show_tool_details;
    }

    /// Toggle analytics panel visibility
    pub fn toggle_analytics(&mut self) {
        self.show_analytics = !self.show_analytics;
    }

    /// Update analytics data
    pub fn update_analytics(&mut self, analytics: Option<SessionAnalytics>) {
        self.analytics = analytics;
    }

    /// Update the scrollbar state
    pub fn update_scroll_state(&mut self, total_lines: usize) {
        self.scroll_state = self.scroll_state.content_length(total_lines);
        self.scroll_state = self
            .scroll_state
            .viewport_content_length(self.viewport_height);
        self.scroll_state = self.scroll_state.position(self.current_scroll);
    }

    /// Analytics scroll methods
    pub fn analytics_scroll_up(&mut self) {
        if self.analytics_scroll > 0 {
            self.analytics_scroll -= 1;
        }
    }

    pub fn analytics_scroll_down(&mut self, max_scroll: usize) {
        if self.analytics_scroll < max_scroll {
            self.analytics_scroll += 1;
        }
    }

    pub fn analytics_scroll_page_up(&mut self, page_size: usize) {
        self.analytics_scroll = self.analytics_scroll.saturating_sub(page_size);
    }

    pub fn analytics_scroll_page_down(&mut self, page_size: usize, max_scroll: usize) {
        self.analytics_scroll = (self.analytics_scroll + page_size).min(max_scroll);
    }

    pub fn analytics_scroll_to_top(&mut self) {
        self.analytics_scroll = 0;
    }

    pub fn analytics_scroll_to_bottom(&mut self, max_scroll: usize) {
        self.analytics_scroll = max_scroll;
    }

    /// Update analytics scrollbar state
    pub fn update_analytics_scroll_state(&mut self, total_lines: usize) {
        self.analytics_scroll_state = self.analytics_scroll_state.content_length(total_lines);
        self.analytics_scroll_state = self
            .analytics_scroll_state
            .viewport_content_length(self.analytics_viewport_height);
        self.analytics_scroll_state = self.analytics_scroll_state.position(self.analytics_scroll);
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
        assert_eq!(state.current_scroll, 0);
        assert!(!state.show_tool_details);
        assert!(!state.loading);
    }

    #[test]
    fn test_set_session_id_clears_data() {
        let mut state = SessionDetailState::new();
        state.current_scroll = 10;

        state.set_session_id(Some("new_session".to_string()));

        assert_eq!(state.session_id, Some("new_session".to_string()));
        assert_eq!(state.current_scroll, 0);
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
    fn test_update_session_preserves_scroll_for_same_session() {
        use crate::models::{Provider, SessionState as ModelSessionState};
        use chrono::Utc;

        let mut state = SessionDetailState::new();

        // Create first session and set scroll position
        let session1 = ChatSession {
            id: uuid::Uuid::new_v4(),
            provider: Provider::ClaudeCode,
            project_name: None,
            start_time: Utc::now(),
            end_time: None,
            message_count: 5,
            token_count: Some(100),
            file_path: "/test/path.jsonl".to_string(),
            file_hash: "test_hash".to_string(),
            state: ModelSessionState::Imported,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        state.update_session(session1.clone(), vec![]);
        state.current_scroll = 42;

        // Update with same session - scroll should be preserved
        state.update_session(session1, vec![]);
        assert_eq!(
            state.current_scroll, 42,
            "Scroll position should be preserved for same session"
        );
    }

    #[test]
    fn test_update_session_resets_scroll_for_different_session() {
        use crate::models::{Provider, SessionState as ModelSessionState};
        use chrono::Utc;

        let mut state = SessionDetailState::new();

        // Create first session and set scroll position
        let session1 = ChatSession {
            id: uuid::Uuid::new_v4(),
            provider: Provider::ClaudeCode,
            project_name: None,
            start_time: Utc::now(),
            end_time: None,
            message_count: 5,
            token_count: Some(100),
            file_path: "/test/path1.jsonl".to_string(),
            file_hash: "test_hash_1".to_string(),
            state: ModelSessionState::Imported,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        state.update_session(session1, vec![]);
        state.current_scroll = 42;

        // Update with different session - scroll should be reset
        let session2 = ChatSession {
            id: uuid::Uuid::new_v4(),
            provider: Provider::ClaudeCode,
            project_name: None,
            start_time: Utc::now(),
            end_time: None,
            message_count: 3,
            token_count: Some(50),
            file_path: "/test/path2.jsonl".to_string(),
            file_hash: "test_hash_2".to_string(),
            state: ModelSessionState::Imported,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        state.update_session(session2, vec![]);
        assert_eq!(
            state.current_scroll, 0,
            "Scroll position should be reset for different session"
        );
    }
}
