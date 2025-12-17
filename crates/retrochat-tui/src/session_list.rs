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

use retrochat_core::database::DatabaseManager;
use retrochat_core::models::OperationStatus;
use retrochat_core::services::{QueryService, SessionSummary, SessionsQueryRequest};

use super::{
    state::{SessionListState, SortOrder},
    utils::text::{get_spinner_char, truncate_text},
};

// Re-export types for backward compatibility
pub use super::state::{SortBy, SortOrder as SessionListSortOrder};

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
            filters: None,
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
                // Start analytics for selected session
                if let Some(session) = self.state.selected_session() {
                    // Return a special signal that we want to start analysis
                    // This will be handled by the main app
                    return Ok(Some(format!("ANALYZE:{}", session.session_id)));
                }
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
                "Sessions: {} | Page: {}/{} | Sort: {} {}",
                self.state.total_count,
                self.state.page,
                total_pages.max(1),
                self.state.sort_by.as_str(),
                if matches!(self.state.sort_order, SortOrder::Ascending) {
                    "↑"
                } else {
                    "↓"
                }
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

        // Use different colors based on analytics status
        let project_style = if session.has_analytics {
            Style::default()
                .fg(Color::LightGreen)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::Yellow)
        };

        let preview_style = if session.has_analytics {
            Style::default().fg(Color::LightBlue)
        } else {
            Style::default().fg(Color::Gray)
        };

        // Apply truncate_text with ellipsis to all columns
        let project_text = session.project.as_deref().unwrap_or("No Project");
        let provider_text = Self::truncate_and_pad(&session.provider, 11);
        let project_text = Self::truncate_and_pad(project_text, 20);
        let start_time_text = Self::truncate_and_pad(&session.start_time, 16);
        let msg_count_text = format!("{:4} msgs", session.message_count);
        let preview_text = Self::truncate_and_pad(&session.first_message_preview, 40);

        // Add analytics status indicator
        let analytics_indicator = match &session.analytics_status {
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
                format!("{spinner_char} "),
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
                if session.has_analytics {
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
            analytics_indicator,
            Span::styled(provider_text, provider_style.add_modifier(Modifier::BOLD)),
            Span::raw(" │ "),
            Span::styled(project_text, project_style),
            Span::raw(" │ "),
            Span::styled(start_time_text, Style::default().fg(Color::Cyan)),
            Span::raw(" │ "),
            Span::styled(msg_count_text, Style::default().fg(Color::Magenta)),
            Span::raw(" │ "),
            Span::styled(preview_text, preview_style),
        ])
    }

    /// Truncates text with ellipsis and pads to fixed width
    fn truncate_and_pad(text: &str, width: usize) -> String {
        let truncated = truncate_text(text, width);
        format!("{truncated:width$}")
    }
}
