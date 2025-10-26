use anyhow::Result;
use crossterm::event::{self, Event, KeyEvent};
use ratatui::{
    backend::Backend,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Tabs},
    Frame, Terminal,
};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::task;
use tokio::time::timeout;

use crate::database::DatabaseManager;
use crate::env::apis as env_vars;
use crate::services::google_ai::{GoogleAiClient, GoogleAiConfig};
use crate::services::{AnalyticsService, QueryService, RetrospectionService};

use super::{
    analytics::AnalyticsWidget,
    components::dialog::{Dialog, DialogType},
    events::{AppEvent, EventHandler, UserAction},
    session_detail::SessionDetailWidget,
    session_list::SessionListWidget,
};

#[derive(Debug, Clone, PartialEq)]
pub enum AppMode {
    SessionList,
    SessionDetail,
    Analytics,
    Help,
}

#[derive(Debug, Clone)]
pub struct AppState {
    pub mode: AppMode,
    pub selected_session_id: Option<String>,
    pub should_quit: bool,
    pub show_help: bool,
    pub last_updated: Instant,
    pub retrospection_active: bool,
    pub active_analysis_requests: Vec<String>, // Track active request IDs
    pub error_dialog: Option<String>,          // Error message to display in dialog
    pub processing_status: Option<String>,     // Status message for background processing
}

impl AppState {
    pub fn new() -> Self {
        Self {
            mode: AppMode::SessionList,
            selected_session_id: None,
            should_quit: false,
            show_help: false,
            last_updated: Instant::now(),
            retrospection_active: false,
            active_analysis_requests: Vec::new(),
            error_dialog: None,
            processing_status: None,
        }
    }

    pub fn quit(&mut self) {
        self.should_quit = true;
    }

    pub fn toggle_help(&mut self) {
        self.show_help = !self.show_help;
    }

    pub fn set_mode(&mut self, mode: AppMode) {
        self.mode = mode;
        self.last_updated = Instant::now();
    }

    pub fn select_session(&mut self, session_id: String) {
        self.selected_session_id = Some(session_id);
        self.set_mode(AppMode::SessionDetail);
    }

    pub fn back_to_list(&mut self) {
        self.selected_session_id = None;
        self.set_mode(AppMode::SessionList);
    }

    pub fn start_retrospection(&mut self, request_id: String) {
        self.retrospection_active = true;
        self.active_analysis_requests.push(request_id);
    }

    pub fn complete_retrospection(&mut self, request_id: &str) {
        self.active_analysis_requests.retain(|id| id != request_id);
        if self.active_analysis_requests.is_empty() {
            self.retrospection_active = false;
        }
    }

    pub fn cancel_all_retrospections(&mut self) {
        self.active_analysis_requests.clear();
        self.retrospection_active = false;
    }

    pub fn show_error(&mut self, message: String) {
        self.error_dialog = Some(message);
    }

    pub fn dismiss_error(&mut self) {
        self.error_dialog = None;
    }

    pub fn set_processing_status(&mut self, status: String) {
        self.processing_status = Some(status);
    }

    pub fn clear_processing_status(&mut self) {
        self.processing_status = None;
    }

    pub fn update_processing_status(&mut self) {
        if self.active_analysis_requests.is_empty() {
            self.processing_status = None;
        } else {
            let count = self.active_analysis_requests.len();
            self.processing_status = Some(format!(
                "{} request{} processing...",
                count,
                if count == 1 { "" } else { "s" }
            ));
        }
    }
}

impl Default for AppState {
    fn default() -> Self {
        Self::new()
    }
}

pub struct App {
    pub state: AppState,
    pub session_list: SessionListWidget,
    pub session_detail: SessionDetailWidget,
    pub analytics: AnalyticsWidget,
    pub query_service: QueryService,
    pub analytics_service: AnalyticsService,
    pub retrospection_service: Option<Arc<RetrospectionService>>,
    pub event_handler: EventHandler,
}

impl App {
    pub fn new(db_manager: std::sync::Arc<DatabaseManager>) -> Result<Self> {
        let query_service = QueryService::with_database(db_manager.clone());
        let analytics_service = AnalyticsService::new((*db_manager).clone());

        // Try to create retrospection service if Google AI API key is available
        let retrospection_service = if std::env::var(env_vars::GOOGLE_AI_API_KEY).is_ok() {
            let config = GoogleAiConfig::default();
            match GoogleAiClient::new(config) {
                Ok(client) => Some(Arc::new(RetrospectionService::new(
                    db_manager.clone(),
                    client,
                ))),
                Err(_) => None,
            }
        } else {
            None
        };

        Ok(Self {
            state: AppState::new(),
            session_list: SessionListWidget::new(db_manager.clone()),
            session_detail: SessionDetailWidget::new(db_manager.clone()),
            analytics: AnalyticsWidget::new(db_manager.clone()),
            query_service,
            analytics_service,
            retrospection_service,
            event_handler: EventHandler::new(),
        })
    }

    pub async fn run<B: Backend>(&mut self, terminal: &mut Terminal<B>) -> Result<()> {
        // Initial data load
        self.session_list.refresh().await?;

        loop {
            // Render UI
            terminal.draw(|f| self.render(f))?;

            // Handle events with timeout
            if let Ok(Ok(Some(event))) =
                timeout(Duration::from_millis(100), self.next_event()).await
            {
                if !self.handle_event(event).await? {
                    break;
                }
            }

            // Check if we should quit
            if self.state.should_quit {
                break;
            }

            // Processing status updates removed

            // Auto-refresh data periodically - frequent refresh for active views
            let refresh_interval = match self.state.mode {
                AppMode::SessionList => Duration::from_secs(3), // Refresh every 3 seconds for session list to catch status changes
                AppMode::SessionDetail => Duration::from_secs(5), // Refresh every 5 seconds for session detail
                _ => Duration::from_secs(30), // Normal 30 second refresh for other views
            };

            if self.state.last_updated.elapsed() > refresh_interval {
                self.refresh_current_view().await?;
            }
        }

        Ok(())
    }

    async fn next_event(&self) -> Result<Option<Event>> {
        if event::poll(Duration::from_millis(100))? {
            Ok(Some(event::read()?))
        } else {
            Ok(None)
        }
    }

    async fn handle_event(&mut self, event: Event) -> Result<bool> {
        // Convert crossterm event to AppEvent
        let app_event = match event {
            Event::Key(key) => AppEvent::Input(key),
            Event::Resize(w, h) => AppEvent::Resize(w, h),
            _ => return Ok(true),
        };

        // Get user actions from event handler
        let mut actions = self.event_handler.handle_event(
            &app_event,
            &self.state.mode,
            self.state.show_help,
            self.state.error_dialog.is_some(),
        );

        // If no actions were generated and it's a key event, check widget-specific handlers
        if actions.is_empty() {
            if let AppEvent::Input(key) = &app_event {
                actions = self.handle_widget_specific_keys(*key).await?;
            }
        }

        // Dispatch each action
        for action in actions {
            if !self.dispatch_action(action).await? {
                return Ok(false);
            }
        }

        Ok(true)
    }

    async fn handle_widget_specific_keys(&mut self, key: KeyEvent) -> Result<Vec<UserAction>> {
        // Handle widget-specific keys that require context (e.g., selected session)
        if self.state.mode == AppMode::SessionList {
            if let Some(action_str) = self.session_list.handle_key(key).await? {
                if action_str.starts_with("ANALYZE:") {
                    let session_id = action_str.strip_prefix("ANALYZE:").unwrap().to_string();
                    return Ok(vec![UserAction::StartAnalysis(session_id)]);
                } else {
                    // Normal session selection
                    return Ok(vec![UserAction::SelectSession(action_str)]);
                }
            }
        }

        Ok(vec![])
    }

    async fn dispatch_action(&mut self, action: UserAction) -> Result<bool> {
        use super::events::UserAction::*;

        match action {
            // Application-level actions
            Quit => {
                self.state.quit();
                return Ok(false);
            }
            ToggleHelp => {
                self.state.toggle_help();
            }
            DismissDialog => {
                self.state.dismiss_error();
            }

            // Navigation actions
            NavigateBack => {
                self.state.back_to_list();
            }
            SwitchTab(direction) => {
                use super::events::TabDirection;
                match direction {
                    TabDirection::Next => self.next_tab().await?,
                    TabDirection::Previous => self.previous_tab().await?,
                }
            }

            // Session list actions
            SelectSession(session_id) => {
                self.state.select_session(session_id);
                self.session_detail
                    .set_session_id(self.state.selected_session_id.clone())
                    .await?;
            }
            StartAnalysis(session_id) => {
                self.handle_start_analysis(session_id).await?;
            }
            SessionListNavigate(direction) => {
                use super::events::NavigationDirection;
                match direction {
                    NavigationDirection::Up => self.session_list.state.previous_session(),
                    NavigationDirection::Down => self.session_list.state.next_session(),
                }
            }
            SessionListPageUp => {
                if self.session_list.state.previous_page() {
                    self.session_list.refresh().await?;
                }
            }
            SessionListPageDown => {
                if self.session_list.state.next_page() {
                    self.session_list.refresh().await?;
                }
            }
            SessionListHome => {
                self.session_list.state.first_session();
            }
            SessionListEnd => {
                self.session_list.state.last_session();
            }
            SessionListCycleSortBy => {
                self.session_list.state.cycle_sort_by();
                self.session_list.refresh().await?;
            }
            SessionListToggleSortOrder => {
                self.session_list.state.toggle_sort_order();
                self.session_list.refresh().await?;
            }

            // Session detail actions
            SessionDetailScrollUp => {
                self.session_detail.state.scroll_up();
            }
            SessionDetailScrollDown => {
                let max_scroll = self.session_detail.get_max_scroll();
                self.session_detail.state.scroll_down(max_scroll);
            }
            SessionDetailPageUp => {
                let page_size = 10;
                self.session_detail.state.scroll_page_up(page_size);
            }
            SessionDetailPageDown => {
                let page_size = 10;
                let max_scroll = self.session_detail.get_max_scroll();
                self.session_detail
                    .state
                    .scroll_page_down(page_size, max_scroll);
            }
            SessionDetailHome => {
                self.session_detail.state.scroll_to_top();
            }
            SessionDetailEnd => {
                let max_scroll = self.session_detail.get_max_scroll();
                self.session_detail.state.scroll_to_bottom(max_scroll);
            }
            SessionDetailToggleWrap => {
                self.session_detail.state.toggle_wrap();
            }
            SessionDetailToggleRetrospection => {
                self.session_detail.state.toggle_retrospection();
            }

            // Analytics actions
            AnalyticsNavigate(_direction) => {
                // TODO: Implement analytics navigation
            }
            AnalyticsRefresh => {
                self.analytics.refresh().await?;
            }

            // Data refresh actions
            RefreshCurrentView => {
                self.refresh_current_view().await?;
            }
        }

        Ok(true)
    }

    async fn handle_start_analysis(&mut self, session_id: String) -> Result<()> {
        if let Some(ref service) = self.retrospection_service {
            // Start actual analysis
            match service
                .create_analysis_request(session_id.clone(), None, None)
                .await
            {
                Ok(request) => {
                    self.state.start_retrospection(request.id.clone());

                    // Immediately refresh UI to show user acknowledgment
                    if let Err(e) = self.session_list.refresh().await {
                        tracing::error!(error = %e, "Failed to refresh session list after analysis start");
                    }

                    // Execute the analysis in background task
                    let service_clone = service.clone();
                    let request_id = request.id.clone();
                    task::spawn(async move {
                        if let Err(e) = service_clone.execute_analysis(request_id).await {
                            tracing::error!(error = %e, "Background analysis failed");
                        }
                    });
                }
                Err(e) => {
                    self.state
                        .show_error(format!("Failed to start analysis: {e}"));
                }
            }
        } else {
            // Show message that Google AI API key is required
            self.state.show_error(format!(
                "Google AI API key not configured. Set {} environment variable.",
                env_vars::GOOGLE_AI_API_KEY
            ));
        }

        Ok(())
    }

    async fn next_tab(&mut self) -> Result<()> {
        self.state.mode = match self.state.mode {
            AppMode::SessionList => AppMode::Analytics,
            AppMode::Analytics => AppMode::SessionList,
            AppMode::SessionDetail => AppMode::SessionList,
            AppMode::Help => AppMode::SessionList,
        };

        Ok(())
    }

    async fn previous_tab(&mut self) -> Result<()> {
        self.state.mode = match self.state.mode {
            AppMode::SessionList => AppMode::Analytics,
            AppMode::Analytics => AppMode::SessionList,
            AppMode::SessionDetail => AppMode::SessionList,
            AppMode::Help => AppMode::SessionList,
        };

        Ok(())
    }

    async fn refresh_current_view(&mut self) -> Result<()> {
        match self.state.mode {
            AppMode::SessionList => {
                self.session_list.refresh().await?;
            }
            AppMode::SessionDetail => {
                self.session_detail.refresh().await?;
            }
            AppMode::Analytics => {
                self.analytics.refresh().await?;
            }
            AppMode::Help => {}
        }
        self.state.last_updated = Instant::now();
        Ok(())
    }

    fn render(&mut self, f: &mut Frame) {
        let main_layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3), // Header/tabs
                Constraint::Min(0),    // Main content
                Constraint::Length(3), // Footer/status
            ])
            .split(f.size());

        // Render header with tabs
        self.render_header(f, main_layout[0]);

        // Render main content based on mode
        if self.state.show_help {
            self.render_help(f, main_layout[1]);
        } else {
            match self.state.mode {
                AppMode::SessionList => {
                    self.session_list.render(f, main_layout[1]);
                }
                AppMode::SessionDetail => {
                    self.session_detail.render(f, main_layout[1]);
                }
                AppMode::Analytics => {
                    self.analytics.render(f, main_layout[1]);
                }
                AppMode::Help => {
                    self.render_help(f, main_layout[1]);
                }
            }
        }

        // Render error dialog if present
        if let Some(ref error_message) = self.state.error_dialog {
            self.render_error_dialog(f, main_layout[1], error_message);
        }

        // Render footer
        self.render_footer(f, main_layout[2]);
    }

    fn render_header(&self, f: &mut Frame, area: Rect) {
        let tab_titles = vec!["Sessions", "Analytics"];
        let selected_tab = match self.state.mode {
            AppMode::SessionList | AppMode::SessionDetail => 0,
            AppMode::Analytics => 1,
            AppMode::Help => 0,
        };

        let tabs = Tabs::new(tab_titles)
            .block(Block::default().borders(Borders::ALL).title("RetroChat"))
            .style(Style::default().fg(Color::White))
            .highlight_style(
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            )
            .select(selected_tab);

        f.render_widget(tabs, area);
    }

    fn render_footer(&self, f: &mut Frame, area: Rect) {
        let key_hints = match self.state.mode {
            AppMode::SessionList => {
                "↑/↓: Navigate | Enter: View | a: Analyze | Tab: Switch | ?: Help | q: Quit | Auto-refreshes every 5s"
            }
            AppMode::SessionDetail => {
                "↑/↓: Scroll | t: Toggle Retrospection | w: Wrap | Esc: Back | ?: Help | q: Quit | Auto-refreshes every 5s"
            }
            AppMode::Analytics => "↑/↓: Navigate | Tab: Switch Views | ?: Help | q: Quit",
            AppMode::Help => "Any key: Close Help",
        }.to_string();

        // Processing status removed from bottom area

        let footer = Paragraph::new(key_hints)
            .block(Block::default().borders(Borders::ALL))
            .style(Style::default().fg(Color::Gray));

        f.render_widget(footer, area);
    }

    fn render_help(&self, f: &mut Frame, area: Rect) {
        let content = vec![
            Line::from(vec![Span::styled(
                "RetroChat - LLM Chat History Analysis",
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            )]),
            Line::from(""),
            Line::from("Global Controls:"),
            Line::from("  q / Ctrl+C     - Quit application"),
            Line::from("  ?              - Toggle this help"),
            Line::from("  Tab / Shift+Tab - Switch between views"),
            Line::from("  Esc            - Go back / Close help"),
            Line::from(""),
            Line::from("Session List:"),
            Line::from("  ↑/↓            - Navigate sessions"),
            Line::from("  Enter          - View session details"),
            Line::from("  a              - Start retrospection analysis"),
            Line::from("  (Auto-refreshes every 5 seconds)"),
            Line::from(""),
            Line::from("Session Detail:"),
            Line::from("  ↑/↓            - Scroll messages"),
            Line::from("  Page Up/Down   - Fast scroll"),
            Line::from("  Home/End       - Jump to start/end"),
            Line::from("  t              - Toggle retrospection panel"),
            Line::from("  w              - Toggle word wrap"),
            Line::from("  (Auto-refreshes every 5 seconds)"),
            Line::from(""),
            Line::from("Analytics:"),
            Line::from("  ↑/↓            - Navigate insights"),
            Line::from("  r              - Refresh analytics"),
            Line::from(""),
            Line::from("Retrospection:"),
            Line::from("  ↑/↓            - Navigate requests"),
            Line::from("  Enter/d        - Toggle details view"),
            Line::from("  c              - Cancel selected request"),
            Line::from("  (Auto-refreshes every 2 seconds)"),
        ];

        let dialog = Dialog::new(DialogType::Help, content).size(80, 70);
        dialog.render(f, area);
    }

    fn render_error_dialog(&self, f: &mut Frame, area: Rect, error_message: &str) {
        let content = vec![
            Line::from(vec![Span::styled(
                "Error",
                Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
            )]),
            Line::from(""),
            Line::from(error_message),
        ];

        let dialog = Dialog::new(DialogType::Error, content).size(60, 40);
        dialog.render(f, area);
    }
}
