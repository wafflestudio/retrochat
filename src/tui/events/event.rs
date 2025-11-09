use crossterm::event::KeyEvent;

/// Low-level events from the terminal
#[derive(Debug, Clone)]
pub enum AppEvent {
    /// Keyboard input
    Input(KeyEvent),
    /// Periodic tick for updates
    Tick,
    /// Terminal resize
    Resize(u16, u16),
}

/// High-level user actions derived from events
#[derive(Debug, Clone, PartialEq)]
pub enum UserAction {
    // Application-level actions
    Quit,
    ToggleHelp,
    DismissDialog,

    // Navigation actions
    NavigateBack,
    SwitchTab(TabDirection),

    // Session list actions
    SelectSession(String),
    StartAnalysis(String),
    SessionListNavigate(NavigationDirection),
    SessionListPageUp,
    SessionListPageDown,
    SessionListHome,
    SessionListEnd,
    SessionListCycleSortBy,
    SessionListToggleSortOrder,

    // Session detail actions
    SessionDetailScrollUp,
    SessionDetailScrollDown,
    SessionDetailPageUp,
    SessionDetailPageDown,
    SessionDetailHome,
    SessionDetailEnd,
    SessionDetailToggleToolDetails,
    SessionDetailToggleAnalytics,

    // Analytics actions
    AnalyticsNavigate(NavigationDirection),
    AnalyticsRefresh,

    // Data refresh actions
    RefreshCurrentView,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TabDirection {
    Next,
    Previous,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NavigationDirection {
    Up,
    Down,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_user_action_equality() {
        assert_eq!(UserAction::Quit, UserAction::Quit);
        assert_eq!(
            UserAction::SwitchTab(TabDirection::Next),
            UserAction::SwitchTab(TabDirection::Next)
        );
        assert_ne!(
            UserAction::SwitchTab(TabDirection::Next),
            UserAction::SwitchTab(TabDirection::Previous)
        );
    }

    #[test]
    fn test_navigation_direction() {
        let up = NavigationDirection::Up;
        let down = NavigationDirection::Down;
        assert_ne!(up, down);
    }
}
