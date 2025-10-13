use anyhow::Result;
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph},
    Frame,
};
use std::sync::Arc;

use crate::database::DatabaseManager;
use crate::models::OperationStatus;
use crate::services::{QueryService, SessionFilters, SessionSummary, SessionsQueryRequest};

use super::{
    state::{SessionListState, SortOrder},
    utils::text::{get_spinner_char, truncate_text},
};

// Re-export types for backward compatibility
pub use super::state::{FilterOptions, SortBy, SortOrder as SessionListSortOrder};

pub struct SessionListWidget {
    pub state: SessionListState,
    query_service: QueryService,
}

impl SessionListWidget {
    pub fn new(db_manager: Arc<DatabaseManager>) -> Self {
        Self {
            state: SessionListState::new(),
            query_service: QueryService::with_database(db_manager),
        }
    }

    pub async fn refresh(&mut self) -> Result<()> {
        self.state.loading = true;

        let request = SessionsQueryRequest {
            page: Some(self.state.page),
            page_size: Some(self.state.page_size),
            sort_by: Some(self.state.sort_by.as_str().to_string()),
            sort_order: Some(self.state.sort_order.as_str().to_string()),
            filters: Some(SessionFilters {
                provider: self.state.filters.provider.clone(),
                project: self.state.filters.project.clone(),
                date_range: self.state.filters.date_range.clone(),
                min_messages: self.state.filters.min_messages,
                max_messages: None,
            }),
        };

        match self.query_service.query_sessions(request).await {
            Ok(response) => {
                self.state
                    .update_sessions(response.sessions, response.total_count);
            }
            Err(e) => {
                tracing::error!(error = %e, "Failed to load sessions");
            }
        }

        self.state.loading = false;
        Ok(())
    }

    pub async fn handle_key(&mut self, key: KeyEvent) -> Result<Option<String>> {
        match key.code {
            KeyCode::Up => {
                self.state.previous_session();
            }
            KeyCode::Down => {
                self.state.next_session();
            }
            KeyCode::Enter => {
                if let Some(session) = self.state.selected_session() {
                    return Ok(Some(session.session_id.clone()));
                }
            }
            KeyCode::PageUp => {
                if self.state.previous_page() {
                    self.refresh().await?;
                }
            }
            KeyCode::PageDown => {
                if self.state.next_page() {
                    self.refresh().await?;
                }
            }
            KeyCode::Home => {
                self.state.first_session();
            }
            KeyCode::End => {
                self.state.last_session();
            }
            KeyCode::Char('s') => {
                self.state.cycle_sort_by();
                self.refresh().await?;
            }
            KeyCode::Char('o') => {
                self.state.toggle_sort_order();
                self.refresh().await?;
            }
            KeyCode::Char('a') => {
                // Start retrospection analysis for selected session
                if let Some(session) = self.state.selected_session() {
                    // Return a special signal that we want to start analysis
                    // This will be handled by the main app
                    return Ok(Some(format!("ANALYZE:{}", session.session_id)));
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
        let total_pages = self.state.total_pages();

        let header_text = if self.state.loading {
            "Loading sessions...".to_string()
        } else {
            format!(
                "Sessions: {} | Page: {}/{} | Sort: {} {} | Press 's' to change sort, 'o' to toggle order, 'f' for filters",
                self.state.total_count,
                self.state.page,
                total_pages.max(1),
                self.state.sort_by.as_str(),
                if matches!(self.state.sort_order, SortOrder::Ascending) { "↑" } else { "↓" }
            )
        };

        let header = Paragraph::new(header_text)
            .block(Block::default().borders(Borders::ALL).title("Session List"))
            .style(Style::default().fg(Color::Cyan));

        f.render_widget(header, area);
    }

    fn render_session_list(&mut self, f: &mut Frame, area: Rect) {
        if self.state.sessions.is_empty() {
            let empty_msg = if self.state.loading {
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
            .state
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

        f.render_stateful_widget(list, area, &mut self.state.list_state);
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
}
