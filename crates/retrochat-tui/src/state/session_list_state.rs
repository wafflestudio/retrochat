use ratatui::widgets::ListState;

use retrochat_core::services::SessionSummary;

/// Sorting options for the session list
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SortBy {
    StartTime,
    MessageCount,
    Provider,
    Project,
}

impl SortBy {
    /// Convert to string for API requests
    pub fn as_str(&self) -> &str {
        match self {
            SortBy::StartTime => "start_time",
            SortBy::MessageCount => "message_count",
            SortBy::Provider => "provider",
            SortBy::Project => "project",
        }
    }

    /// Cycle to the next sort option
    pub fn next(&self) -> Self {
        match self {
            SortBy::StartTime => SortBy::MessageCount,
            SortBy::MessageCount => SortBy::Provider,
            SortBy::Provider => SortBy::Project,
            SortBy::Project => SortBy::StartTime,
        }
    }
}

/// Sort order
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SortOrder {
    Ascending,
    Descending,
}

impl SortOrder {
    /// Convert to string for API requests
    pub fn as_str(&self) -> &str {
        match self {
            SortOrder::Ascending => "asc",
            SortOrder::Descending => "desc",
        }
    }

    /// Toggle the sort order
    pub fn toggle(&self) -> Self {
        match self {
            SortOrder::Ascending => SortOrder::Descending,
            SortOrder::Descending => SortOrder::Ascending,
        }
    }
}

/// State for the session list view
#[derive(Debug)]
pub struct SessionListState {
    /// List of sessions
    pub sessions: Vec<SessionSummary>,
    /// UI state for the list widget
    pub list_state: ListState,
    /// Current sorting option
    pub sort_by: SortBy,
    /// Current sort order
    pub sort_order: SortOrder,
    /// Current page number (1-indexed)
    pub page: i32,
    /// Items per page
    pub page_size: i32,
    /// Total number of sessions
    pub total_count: i32,
    /// Loading indicator
    pub loading: bool,
}

impl SessionListState {
    /// Create a new session list state with default values
    pub fn new() -> Self {
        let mut list_state = ListState::default();
        list_state.select(Some(0));

        Self {
            sessions: Vec::new(),
            list_state,
            sort_by: SortBy::StartTime,
            sort_order: SortOrder::Descending,
            page: 1,
            page_size: 50,
            total_count: 0,
            loading: false,
        }
    }

    /// Get the total number of pages
    pub fn total_pages(&self) -> i32 {
        (self.total_count + self.page_size - 1) / self.page_size
    }

    /// Get the currently selected session
    pub fn selected_session(&self) -> Option<&SessionSummary> {
        self.list_state
            .selected()
            .and_then(|idx| self.sessions.get(idx))
    }

    /// Move selection to the next session
    pub fn next_session(&mut self) {
        if self.sessions.is_empty() {
            return;
        }

        let selected = self.list_state.selected().unwrap_or(0);
        let next = if selected >= self.sessions.len() - 1 {
            0
        } else {
            selected + 1
        };
        self.list_state.select(Some(next));
    }

    /// Move selection to the previous session
    pub fn previous_session(&mut self) {
        if self.sessions.is_empty() {
            return;
        }

        let selected = self.list_state.selected().unwrap_or(0);
        let previous = if selected == 0 {
            self.sessions.len() - 1
        } else {
            selected - 1
        };
        self.list_state.select(Some(previous));
    }

    /// Move selection to the first session
    pub fn first_session(&mut self) {
        if !self.sessions.is_empty() {
            self.list_state.select(Some(0));
        }
    }

    /// Move selection to the last session
    pub fn last_session(&mut self) {
        if !self.sessions.is_empty() {
            self.list_state.select(Some(self.sessions.len() - 1));
        }
    }

    /// Move to the next page if possible
    pub fn next_page(&mut self) -> bool {
        let total_pages = self.total_pages();
        if self.page < total_pages {
            self.page += 1;
            self.list_state.select(Some(0));
            true
        } else {
            false
        }
    }

    /// Move to the previous page if possible
    pub fn previous_page(&mut self) -> bool {
        if self.page > 1 {
            self.page -= 1;
            self.list_state.select(Some(0));
            true
        } else {
            false
        }
    }

    /// Cycle to the next sort option and reset to first page
    pub fn cycle_sort_by(&mut self) {
        self.sort_by = self.sort_by.next();
        self.page = 1;
    }

    /// Toggle sort order and reset to first page
    pub fn toggle_sort_order(&mut self) {
        self.sort_order = self.sort_order.toggle();
        self.page = 1;
    }

    /// Update sessions from query result
    pub fn update_sessions(&mut self, sessions: Vec<SessionSummary>, total_count: i32) {
        self.sessions = sessions;
        self.total_count = total_count;

        // Ensure selection is valid
        if !self.sessions.is_empty() {
            if let Some(selected) = self.list_state.selected() {
                if selected >= self.sessions.len() {
                    self.list_state.select(Some(0));
                }
            } else {
                self.list_state.select(Some(0));
            }
        }
    }
}

impl Default for SessionListState {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sort_by_cycle() {
        assert_eq!(SortBy::StartTime.next(), SortBy::MessageCount);
        assert_eq!(SortBy::MessageCount.next(), SortBy::Provider);
        assert_eq!(SortBy::Provider.next(), SortBy::Project);
        assert_eq!(SortBy::Project.next(), SortBy::StartTime);
    }

    #[test]
    fn test_sort_order_toggle() {
        assert_eq!(SortOrder::Ascending.toggle(), SortOrder::Descending);
        assert_eq!(SortOrder::Descending.toggle(), SortOrder::Ascending);
    }

    #[test]
    fn test_session_list_state_default() {
        let state = SessionListState::new();
        assert_eq!(state.page, 1);
        assert_eq!(state.page_size, 50);
        assert_eq!(state.sort_by, SortBy::StartTime);
        assert_eq!(state.sort_order, SortOrder::Descending);
        assert!(!state.loading);
    }

    #[test]
    fn test_total_pages_calculation() {
        let mut state = SessionListState::new();
        state.page_size = 10;

        state.total_count = 0;
        assert_eq!(state.total_pages(), 0);

        state.total_count = 5;
        assert_eq!(state.total_pages(), 1);

        state.total_count = 10;
        assert_eq!(state.total_pages(), 1);

        state.total_count = 11;
        assert_eq!(state.total_pages(), 2);

        state.total_count = 25;
        assert_eq!(state.total_pages(), 3);
    }

    #[test]
    fn test_pagination() {
        let mut state = SessionListState::new();
        state.total_count = 150; // 3 pages with page_size = 50

        assert_eq!(state.page, 1);
        assert!(state.next_page());
        assert_eq!(state.page, 2);
        assert!(state.next_page());
        assert_eq!(state.page, 3);
        assert!(!state.next_page()); // Can't go beyond last page
        assert_eq!(state.page, 3);

        assert!(state.previous_page());
        assert_eq!(state.page, 2);
        assert!(state.previous_page());
        assert_eq!(state.page, 1);
        assert!(!state.previous_page()); // Can't go below page 1
        assert_eq!(state.page, 1);
    }
}
