use anyhow::Result;
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph},
    Frame,
};
use std::sync::Arc;

use crate::database::DatabaseManager;
use crate::models::OperationStatus;
use crate::services::{
    DateRange, QueryService, SessionFilters, SessionSummary, SessionsQueryRequest,
};

use super::utils::text::{get_spinner_char, truncate_text};

#[derive(Debug, Clone)]
pub enum SortBy {
    StartTime,
    MessageCount,
    Provider,
    Project,
}

#[derive(Debug, Clone)]
pub enum SortOrder {
    Ascending,
    Descending,
}

#[derive(Debug, Clone, Default)]
pub struct FilterOptions {
    pub provider: Option<String>,
    pub project: Option<String>,
    pub date_range: Option<DateRange>,
    pub min_messages: Option<i32>,
}

pub struct SessionListWidget {
    sessions: Vec<SessionSummary>,
    list_state: ListState,
    query_service: QueryService,
    sort_by: SortBy,
    sort_order: SortOrder,
    filters: FilterOptions,
    page: i32,
    page_size: i32,
    total_count: i32,
    loading: bool,
}

impl SessionListWidget {
    pub fn new(db_manager: Arc<DatabaseManager>) -> Self {
        let mut list_state = ListState::default();
        list_state.select(Some(0));

        Self {
            sessions: Vec::new(),
            list_state,
            query_service: QueryService::with_database(db_manager),
            sort_by: SortBy::StartTime,
            sort_order: SortOrder::Descending,
            filters: FilterOptions::default(),
            page: 1,
            page_size: 50,
            total_count: 0,
            loading: false,
        }
    }

    pub async fn refresh(&mut self) -> Result<()> {
        self.loading = true;

        let request = SessionsQueryRequest {
            page: Some(self.page),
            page_size: Some(self.page_size),
            sort_by: Some(self.sort_by_string()),
            sort_order: Some(self.sort_order_string()),
            filters: Some(SessionFilters {
                provider: self.filters.provider.clone(),
                project: self.filters.project.clone(),
                date_range: self.filters.date_range.clone(),
                min_messages: self.filters.min_messages,
                max_messages: None,
            }),
        };

        match self.query_service.query_sessions(request).await {
            Ok(response) => {
                self.sessions = response.sessions;
                self.total_count = response.total_count;

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
            Err(e) => {
                tracing::error!(error = %e, "Failed to load sessions");
            }
        }

        self.loading = false;
        Ok(())
    }

    pub async fn handle_key(&mut self, key: KeyEvent) -> Result<Option<String>> {
        match key.code {
            KeyCode::Up => {
                self.previous_session();
            }
            KeyCode::Down => {
                self.next_session();
            }
            KeyCode::Enter => {
                if let Some(selected) = self.list_state.selected() {
                    if let Some(session) = self.sessions.get(selected) {
                        return Ok(Some(session.session_id.clone()));
                    }
                }
            }
            KeyCode::PageUp => {
                self.previous_page().await?;
            }
            KeyCode::PageDown => {
                self.next_page().await?;
            }
            KeyCode::Home => {
                self.first_session();
            }
            KeyCode::End => {
                self.last_session();
            }
            KeyCode::Char('s') => {
                self.cycle_sort_by().await?;
            }
            KeyCode::Char('o') => {
                self.toggle_sort_order().await?;
            }
            KeyCode::Char('a') => {
                // Start retrospection analysis for selected session
                if let Some(selected) = self.list_state.selected() {
                    if let Some(session) = self.sessions.get(selected) {
                        // Return a special signal that we want to start analysis
                        // This will be handled by the main app
                        return Ok(Some(format!("ANALYZE:{}", session.session_id)));
                    }
                }
            }
            KeyCode::Char('f') => {
                // TODO: Implement filter dialog
            }
            _ => {}
        }
        Ok(None)
    }

    pub fn render(&mut self, f: &mut Frame, area: Rect) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3), // Header info
                Constraint::Min(0),    // Session list
            ])
            .split(area);

        // Render header with stats and controls
        self.render_header(f, chunks[0]);

        // Render session list
        self.render_session_list(f, chunks[1]);
    }

    fn render_header(&self, f: &mut Frame, area: Rect) {
        let total_pages = (self.total_count + self.page_size - 1) / self.page_size;

        let header_text = if self.loading {
            "Loading sessions...".to_string()
        } else {
            format!(
                "Sessions: {} | Page: {}/{} | Sort: {} {} | Press 's' to change sort, 'o' to toggle order, 'f' for filters",
                self.total_count,
                self.page,
                total_pages.max(1),
                self.sort_by_string(),
                if matches!(self.sort_order, SortOrder::Ascending) { "↑" } else { "↓" }
            )
        };

        let header = Paragraph::new(header_text)
            .block(Block::default().borders(Borders::ALL).title("Session List"))
            .style(Style::default().fg(Color::Cyan));

        f.render_widget(header, area);
    }

    fn render_session_list(&mut self, f: &mut Frame, area: Rect) {
        if self.sessions.is_empty() {
            let empty_msg = if self.loading {
                "Loading sessions..."
            } else {
                "No sessions found. Import some chat history files first."
            };

            let paragraph = Paragraph::new(empty_msg)
                .block(Block::default().borders(Borders::ALL))
                .style(Style::default().fg(Color::Gray));

            f.render_widget(paragraph, area);
            return;
        }

        let spinner_char = get_spinner_char();
        let items: Vec<ListItem> = self
            .sessions
            .iter()
            .enumerate()
            .map(|(i, session)| {
                let line = Self::format_session_line_with_spinner(session, i, spinner_char);
                ListItem::new(line)
            })
            .collect();

        let list = List::new(items)
            .block(Block::default().borders(Borders::ALL))
            .highlight_style(
                Style::default()
                    .bg(Color::DarkGray)
                    .add_modifier(Modifier::BOLD),
            )
            .highlight_symbol("▶ ");

        f.render_stateful_widget(list, area, &mut self.list_state);
    }

    fn format_session_line_with_spinner(
        session: &SessionSummary,
        _index: usize,
        spinner_char: char,
    ) -> Line<'_> {
        let provider_style = match session.provider.as_str() {
            "claude-code" => Style::default().fg(Color::Blue),
            "gemini" => Style::default().fg(Color::Green),
            "cursor" => Style::default().fg(Color::Magenta),
            "chatgpt" => Style::default().fg(Color::Cyan),
            _ => Style::default().fg(Color::White),
        };

        // Use different colors based on retrospection status
        let project_style = if session.has_retrospection {
            Style::default()
                .fg(Color::LightGreen)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::Yellow)
        };

        let preview_style = if session.has_retrospection {
            Style::default().fg(Color::LightBlue)
        } else {
            Style::default().fg(Color::Gray)
        };

        let project_text = session.project.as_deref().unwrap_or("No Project");
        let start_time = if session.start_time.chars().count() >= 16 {
            let truncated: String = session.start_time.chars().take(16).collect();
            truncated
        } else {
            session.start_time.clone()
        };

        // Add retrospection status indicator
        let retrospection_indicator = match &session.retrospection_status {
            Some(OperationStatus::Completed) => Span::styled(
                "✓ ",
                Style::default()
                    .fg(Color::LightGreen)
                    .add_modifier(Modifier::BOLD),
            ),
            Some(OperationStatus::Running) => Span::styled(
                format!("{spinner_char} "),
                Style::default()
                    .fg(Color::LightBlue)
                    .add_modifier(Modifier::BOLD),
            ),
            Some(OperationStatus::Pending) => Span::styled(
                "⋯ ",
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            ),
            Some(OperationStatus::Failed) => Span::styled(
                "✗ ",
                Style::default()
                    .fg(Color::LightRed)
                    .add_modifier(Modifier::BOLD),
            ),
            Some(OperationStatus::Cancelled) => Span::styled(
                "⊘ ",
                Style::default()
                    .fg(Color::Gray)
                    .add_modifier(Modifier::BOLD),
            ),
            None => {
                if session.has_retrospection {
                    Span::styled(
                        "✓ ",
                        Style::default()
                            .fg(Color::LightGreen)
                            .add_modifier(Modifier::BOLD),
                    )
                } else {
                    Span::raw("  ")
                }
            }
        };

        Line::from(vec![
            retrospection_indicator,
            Span::styled(
                format!("{:11}", session.provider),
                provider_style.add_modifier(Modifier::BOLD),
            ),
            Span::raw(" │ "),
            Span::styled(format!("{project_text:20}"), project_style),
            Span::raw(" │ "),
            Span::styled(format!("{start_time:16}"), Style::default().fg(Color::Cyan)),
            Span::raw(" │ "),
            Span::styled(
                format!("{:4} msgs", session.message_count),
                Style::default().fg(Color::Magenta),
            ),
            Span::raw(" │ "),
            Span::styled(
                truncate_text(&session.first_message_preview, 40),
                preview_style,
            ),
        ])
    }

    fn next_session(&mut self) {
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

    fn previous_session(&mut self) {
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

    fn first_session(&mut self) {
        if !self.sessions.is_empty() {
            self.list_state.select(Some(0));
        }
    }

    fn last_session(&mut self) {
        if !self.sessions.is_empty() {
            self.list_state.select(Some(self.sessions.len() - 1));
        }
    }

    async fn next_page(&mut self) -> Result<()> {
        let total_pages = (self.total_count + self.page_size - 1) / self.page_size;
        if self.page < total_pages {
            self.page += 1;
            self.refresh().await?;
            self.list_state.select(Some(0));
        }
        Ok(())
    }

    async fn previous_page(&mut self) -> Result<()> {
        if self.page > 1 {
            self.page -= 1;
            self.refresh().await?;
            self.list_state.select(Some(0));
        }
        Ok(())
    }

    async fn cycle_sort_by(&mut self) -> Result<()> {
        self.sort_by = match self.sort_by {
            SortBy::StartTime => SortBy::MessageCount,
            SortBy::MessageCount => SortBy::Provider,
            SortBy::Provider => SortBy::Project,
            SortBy::Project => SortBy::StartTime,
        };
        self.page = 1; // Reset to first page
        self.refresh().await?;
        Ok(())
    }

    async fn toggle_sort_order(&mut self) -> Result<()> {
        self.sort_order = match self.sort_order {
            SortOrder::Ascending => SortOrder::Descending,
            SortOrder::Descending => SortOrder::Ascending,
        };
        self.page = 1; // Reset to first page
        self.refresh().await?;
        Ok(())
    }

    fn sort_by_string(&self) -> String {
        match self.sort_by {
            SortBy::StartTime => "start_time".to_string(),
            SortBy::MessageCount => "message_count".to_string(),
            SortBy::Provider => "provider".to_string(),
            SortBy::Project => "project".to_string(),
        }
    }

    fn sort_order_string(&self) -> String {
        match self.sort_order {
            SortOrder::Ascending => "asc".to_string(),
            SortOrder::Descending => "desc".to_string(),
        }
    }
}
