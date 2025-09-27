use anyhow::Result;
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyModifiers};
use ratatui::{
    backend::Backend,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph, Tabs},
    Frame, Terminal,
};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::time::timeout;
use tokio::task;

use crate::database::DatabaseManager;
use crate::services::{AnalyticsService, QueryService, RetrospectionService};
use crate::services::google_ai::{GoogleAiClient, GoogleAiConfig};

use super::{
    analytics::AnalyticsWidget, retrospection::RetrospectionWidget, session_detail::SessionDetailWidget,
    session_list::SessionListWidget,
};

#[derive(Debug, Clone, PartialEq)]
pub enum AppMode {
    SessionList,
    SessionDetail,
    Analytics,
    Retrospection,
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
    pub error_dialog: Option<String>, // Error message to display in dialog
    pub processing_status: Option<String>, // Status message for background processing
    pub spinner_frame: usize, // Current frame of the spinner animation
    pub last_spinner_update: Instant, // Last time the spinner was updated
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
            spinner_frame: 0,
            last_spinner_update: Instant::now(),
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
            let spinner = self.get_spinner_char();
            self.processing_status = Some(format!("{} {} request{} processing...",
                spinner,
                count,
                if count == 1 { "" } else { "s" }
            ));
        }
    }

    pub fn advance_spinner(&mut self) {
        self.spinner_frame = (self.spinner_frame + 1) % 8;
        self.last_spinner_update = Instant::now();
    }

    pub fn should_update_spinner(&self) -> bool {
        self.last_spinner_update.elapsed() >= Duration::from_millis(150)
    }

    fn get_spinner_char(&self) -> char {
        const SPINNER_CHARS: [char; 8] = ['⠋', '⠙', '⠹', '⠸', '⠼', '⠴', '⠦', '⠧'];
        SPINNER_CHARS[self.spinner_frame]
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
    pub retrospection: RetrospectionWidget,
    pub query_service: QueryService,
    pub analytics_service: AnalyticsService,
    pub retrospection_service: Option<Arc<RetrospectionService>>,
}

impl App {
    pub fn new(db_manager: std::sync::Arc<DatabaseManager>) -> Result<Self> {
        let query_service = QueryService::with_database(db_manager.clone());
        let analytics_service = AnalyticsService::new((*db_manager).clone());

        // Try to create retrospection service if Google AI API key is available
        let retrospection_service = if let Ok(_) = std::env::var("GOOGLE_AI_API_KEY") {
            let config = GoogleAiConfig::default();
            match GoogleAiClient::new(config) {
                Ok(client) => Some(Arc::new(RetrospectionService::new(db_manager.clone(), client))),
                Err(_) => None,
            }
        } else {
            None
        };

        // Create the retrospection widget with service
        let service_for_widget = if let Some(service) = &retrospection_service {
            service.clone()
        } else {
            // Create a fallback service with default config
            let config = GoogleAiConfig::default();
            let client = GoogleAiClient::new(config).expect("Failed to create Google AI client");
            Arc::new(RetrospectionService::new(db_manager.clone(), client))
        };
        let retrospection_widget = RetrospectionWidget::new(service_for_widget);

        Ok(Self {
            state: AppState::new(),
            session_list: SessionListWidget::new(db_manager.clone()),
            session_detail: SessionDetailWidget::new(db_manager.clone()),
            analytics: AnalyticsWidget::new(db_manager.clone()),
            retrospection: retrospection_widget,
            query_service,
            analytics_service,
            retrospection_service,
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

            // Advance spinner animation for processing status (throttled to ~6.7 FPS)
            if self.state.processing_status.is_some() && self.state.should_update_spinner() {
                self.state.advance_spinner();
                self.state.update_processing_status();
            }

            // Auto-refresh data periodically - frequent refresh for active views
            let refresh_interval = match self.state.mode {
                AppMode::Retrospection => Duration::from_secs(2), // Refresh every 2 seconds in retrospection view
                AppMode::SessionList | AppMode::SessionDetail => Duration::from_secs(5), // Refresh every 5 seconds for session views
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
        match event {
            Event::Key(key_event) => self.handle_key_event(key_event).await,
            Event::Resize(_, _) => {
                // Force refresh on resize
                self.refresh_current_view().await?;
                Ok(true)
            }
            _ => Ok(true),
        }
    }

    async fn handle_key_event(&mut self, key: KeyEvent) -> Result<bool> {
        // Global key bindings
        match (key.modifiers, key.code) {
            (KeyModifiers::CONTROL, KeyCode::Char('c')) => {
                self.state.quit();
                return Ok(false);
            }
            (KeyModifiers::NONE, KeyCode::Char('q')) => {
                if !self.state.show_help {
                    self.state.quit();
                    return Ok(false);
                }
            }
            (KeyModifiers::NONE, KeyCode::Char('?')) | (KeyModifiers::NONE, KeyCode::F(1)) => {
                self.state.toggle_help();
                return Ok(true);
            }
            (KeyModifiers::NONE, KeyCode::Esc) => {
                if self.state.show_help {
                    self.state.toggle_help();
                } else {
                    match self.state.mode {
                        AppMode::SessionDetail => self.state.back_to_list(),
                        AppMode::Analytics => self.state.set_mode(AppMode::SessionList),
                        AppMode::Retrospection => self.state.set_mode(AppMode::SessionList),
                        _ => {}
                    }
                }
                return Ok(true);
            }
            _ => {}
        }

        // Help screen - consume all other inputs
        if self.state.show_help {
            return Ok(true);
        }

        // Error dialog - dismiss on any key
        if self.state.error_dialog.is_some() {
            self.state.dismiss_error();
            return Ok(true);
        }

        // Tab navigation
        match key.code {
            KeyCode::Tab => {
                self.next_tab().await?;
                return Ok(true);
            }
            KeyCode::BackTab => {
                self.previous_tab().await?;
                return Ok(true);
            }
            _ => {}
        }

        // Mode-specific key bindings
        match self.state.mode {
            AppMode::SessionList => {
                if let Some(action) = self.session_list.handle_key(key).await? {
                    if action.starts_with("ANALYZE:") {
                        // Extract session ID and start analysis
                        let session_id = action.strip_prefix("ANALYZE:").unwrap().to_string();
                        // TODO: Show analysis type selection dialog
                        // For now, use default analysis type
                        use crate::models::RetrospectionAnalysisType;

                        if let Some(ref service) = self.retrospection_service {
                            // Start actual analysis
                            match service.create_analysis_request(
                                session_id.clone(),
                                RetrospectionAnalysisType::UserInteractionAnalysis,
                                None,
                                None,
                            ).await {
                                Ok(request) => {
                                    self.state.start_retrospection(request.id.clone());
                                    self.retrospection.start_analysis(session_id.clone(), RetrospectionAnalysisType::UserInteractionAnalysis);

                                    // Update processing status with count (stay on current tab)
                                    self.state.update_processing_status();

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
                                    self.state.show_error(format!("Failed to start analysis: {e}"));
                                }
                            }
                        } else {
                            // Show message that Google AI API key is required
                            self.state.show_error("Google AI API key not configured. Set GOOGLE_AI_API_KEY environment variable.".to_string());
                        }
                    } else {
                        // Normal session selection
                        self.state.select_session(action);
                        self.session_detail
                            .set_session_id(self.state.selected_session_id.clone())
                            .await?;
                    }
                }
            }
            AppMode::SessionDetail => {
                self.session_detail.handle_key(key).await?;
            }
            AppMode::Analytics => {
                self.analytics.handle_key(key).await?;
            }
            AppMode::Retrospection => {
                self.retrospection.handle_key(key).await?;
            }
            AppMode::Help => {
                // Help is handled above
            }
        }

        Ok(true)
    }

    async fn next_tab(&mut self) -> Result<()> {
        let old_mode = self.state.mode.clone();
        self.state.mode = match self.state.mode {
            AppMode::SessionList => AppMode::Analytics,
            AppMode::Analytics => AppMode::Retrospection,
            AppMode::Retrospection => AppMode::SessionList,
            AppMode::SessionDetail => AppMode::SessionList,
            AppMode::Help => AppMode::SessionList,
        };

        // Trigger refresh when entering retrospection tab
        if self.state.mode == AppMode::Retrospection && old_mode != AppMode::Retrospection {
            self.retrospection.refresh().await?;
        }

        Ok(())
    }

    async fn previous_tab(&mut self) -> Result<()> {
        let old_mode = self.state.mode.clone();
        self.state.mode = match self.state.mode {
            AppMode::SessionList => AppMode::Retrospection,
            AppMode::Analytics => AppMode::SessionList,
            AppMode::Retrospection => AppMode::Analytics,
            AppMode::SessionDetail => AppMode::SessionList,
            AppMode::Help => AppMode::SessionList,
        };

        // Trigger refresh when entering retrospection tab
        if self.state.mode == AppMode::Retrospection && old_mode != AppMode::Retrospection {
            self.retrospection.refresh().await?;
        }

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
            AppMode::Retrospection => {
                self.retrospection.refresh().await?;
                // Check if any active analysis requests have completed
                self.check_analysis_completion().await?;
            }
            AppMode::Help => {}
        }
        self.state.last_updated = Instant::now();
        Ok(())
    }

    async fn check_analysis_completion(&mut self) -> Result<()> {
        if let Some(ref service) = self.retrospection_service {
            let mut completed_requests = Vec::new();

            for request_id in &self.state.active_analysis_requests {
                if let Ok(request) = service.get_analysis_status(request_id.clone()).await {
                    match request.status {
                        crate::models::OperationStatus::Completed |
                        crate::models::OperationStatus::Failed |
                        crate::models::OperationStatus::Cancelled => {
                            completed_requests.push(request_id.clone());
                        }
                        _ => {}
                    }
                }
            }

            // Remove completed requests and update processing status
            for request_id in completed_requests {
                self.state.complete_retrospection(&request_id);
            }

            // Update processing status based on current active count
            self.state.update_processing_status();
        }
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
                AppMode::Retrospection => {
                    self.retrospection.render(f, main_layout[1]);
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
        let tab_titles = vec!["Sessions", "Analytics", "Retrospection"];
        let selected_tab = match self.state.mode {
            AppMode::SessionList | AppMode::SessionDetail => 0,
            AppMode::Analytics => 1,
            AppMode::Retrospection => 2,
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
        let mut key_hints = match self.state.mode {
            AppMode::SessionList => {
                "↑/↓: Navigate | Enter: View | a: Analyze | Tab: Switch | ?: Help | q: Quit | Auto-refreshes every 5s"
            }
            AppMode::SessionDetail => {
                "↑/↓: Scroll | t: Toggle Retrospection | w: Wrap | Esc: Back | ?: Help | q: Quit | Auto-refreshes every 5s"
            }
            AppMode::Analytics => "↑/↓: Navigate | Tab: Switch Views | ?: Help | q: Quit",
            AppMode::Retrospection => "↑/↓: Navigate | Enter/d: Details | c: Cancel | ?: Help | q: Quit",
            AppMode::Help => "Any key: Close Help",
        }.to_string();

        // Add processing status if present
        if let Some(ref status) = self.state.processing_status {
            key_hints = format!("{} | {}", key_hints, status);
        }

        let footer = Paragraph::new(key_hints)
            .block(Block::default().borders(Borders::ALL))
            .style(Style::default().fg(Color::Gray));

        f.render_widget(footer, area);
    }

    fn render_help(&self, f: &mut Frame, area: Rect) {
        let help_text = vec![
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
            Line::from(""),
            Line::from("Press any key to close this help screen"),
        ];

        let help_paragraph = Paragraph::new(help_text)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("Help")
                    .style(Style::default().fg(Color::Cyan)),
            )
            .style(Style::default().fg(Color::White));

        // Center the help dialog
        let popup_area = self.centered_rect(80, 70, area);
        f.render_widget(Clear, popup_area);
        f.render_widget(help_paragraph, popup_area);
    }

    fn render_error_dialog(&self, f: &mut Frame, area: Rect, error_message: &str) {
        let error_text = vec![
            Line::from(vec![Span::styled(
                "Error",
                Style::default()
                    .fg(Color::Red)
                    .add_modifier(Modifier::BOLD),
            )]),
            Line::from(""),
            Line::from(error_message),
            Line::from(""),
            Line::from(vec![Span::styled(
                "Press any key to continue",
                Style::default().fg(Color::Gray),
            )]),
        ];

        let error_paragraph = Paragraph::new(error_text)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("Error")
                    .style(Style::default().fg(Color::Red)),
            )
            .style(Style::default().fg(Color::White))
            .wrap(ratatui::widgets::Wrap { trim: true });

        // Center the error dialog
        let popup_area = self.centered_rect(60, 40, area);
        f.render_widget(Clear, popup_area);
        f.render_widget(error_paragraph, popup_area);
    }

    fn centered_rect(&self, percent_x: u16, percent_y: u16, r: Rect) -> Rect {
        let popup_layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Percentage((100 - percent_y) / 2),
                Constraint::Percentage(percent_y),
                Constraint::Percentage((100 - percent_y) / 2),
            ])
            .split(r);

        Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage((100 - percent_x) / 2),
                Constraint::Percentage(percent_x),
                Constraint::Percentage((100 - percent_x) / 2),
            ])
            .split(popup_layout[1])[1]
    }
}
