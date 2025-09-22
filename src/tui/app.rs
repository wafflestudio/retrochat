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
use std::time::{Duration, Instant};
use tokio::time::timeout;

use crate::database::DatabaseManager;
use crate::services::{AnalyticsService, QueryService};

use super::{
    analytics::AnalyticsWidget, session_detail::SessionDetailWidget,
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
}

impl AppState {
    pub fn new() -> Self {
        Self {
            mode: AppMode::SessionList,
            selected_session_id: None,
            should_quit: false,
            show_help: false,
            last_updated: Instant::now(),
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
}

impl App {
    pub fn new(db_manager: std::sync::Arc<DatabaseManager>) -> Result<Self> {
        let query_service = QueryService::with_database(db_manager.clone());
        let analytics_service = AnalyticsService::new((*db_manager).clone());

        Ok(Self {
            state: AppState::new(),
            session_list: SessionListWidget::new(db_manager.clone()),
            session_detail: SessionDetailWidget::new(db_manager.clone()),
            analytics: AnalyticsWidget::new(db_manager),
            query_service,
            analytics_service,
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

            // Auto-refresh data periodically
            if self.state.last_updated.elapsed() > Duration::from_secs(30) {
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

        // Tab navigation
        match key.code {
            KeyCode::Tab => {
                self.next_tab();
                return Ok(true);
            }
            KeyCode::BackTab => {
                self.previous_tab();
                return Ok(true);
            }
            _ => {}
        }

        // Mode-specific key bindings
        match self.state.mode {
            AppMode::SessionList => {
                if let Some(selected_id) = self.session_list.handle_key(key).await? {
                    self.state.select_session(selected_id);
                    self.session_detail
                        .set_session_id(self.state.selected_session_id.clone())
                        .await?;
                }
            }
            AppMode::SessionDetail => {
                self.session_detail.handle_key(key).await?;
            }
            AppMode::Analytics => {
                self.analytics.handle_key(key).await?;
            }
            AppMode::Help => {
                // Help is handled above
            }
        }

        Ok(true)
    }

    fn next_tab(&mut self) {
        self.state.mode = match self.state.mode {
            AppMode::SessionList => AppMode::Analytics,
            AppMode::Analytics => AppMode::SessionList,
            AppMode::SessionDetail => AppMode::SessionList,
            AppMode::Help => AppMode::SessionList,
        };
    }

    fn previous_tab(&mut self) {
        self.state.mode = match self.state.mode {
            AppMode::SessionList => AppMode::Analytics,
            AppMode::Analytics => AppMode::SessionList,
            AppMode::SessionDetail => AppMode::SessionList,
            AppMode::Help => AppMode::SessionList,
        };
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
                "↑/↓: Navigate | Enter: View Session | Tab: Switch Views | ?: Help | q: Quit"
            }
            AppMode::SessionDetail => {
                "↑/↓: Scroll | Esc: Back | Tab: Switch Views | ?: Help | q: Quit"
            }
            AppMode::Analytics => "↑/↓: Navigate | Tab: Switch Views | ?: Help | q: Quit",
            AppMode::Help => "Any key: Close Help",
        };

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
            Line::from("  r              - Refresh session list"),
            Line::from(""),
            Line::from("Session Detail:"),
            Line::from("  ↑/↓            - Scroll messages"),
            Line::from("  Page Up/Down   - Fast scroll"),
            Line::from("  Home/End       - Jump to start/end"),
            Line::from(""),
            Line::from("Analytics:"),
            Line::from("  ↑/↓            - Navigate insights"),
            Line::from("  r              - Refresh analytics"),
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
