use anyhow::Result;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Scrollbar, ScrollbarOrientation, Wrap},
    Frame,
};
use std::sync::Arc;

use crate::database::{DatabaseManager, RetrospectionRepository};
use crate::models::MessageRole;
use crate::services::{QueryService, SessionDetailRequest};

use super::state::SessionDetailState;
use super::tool_display::{ToolDisplayConfig, ToolDisplayFormatter};
use super::utils::text::{truncate_text, wrap_text};

pub struct SessionDetailWidget {
    pub state: SessionDetailState,
    query_service: QueryService,
    retrospection_repo: RetrospectionRepository,
    tool_formatter: ToolDisplayFormatter,
}

impl SessionDetailWidget {
    pub fn new(db_manager: Arc<DatabaseManager>) -> Self {
        Self {
            state: SessionDetailState::new(),
            query_service: QueryService::with_database(db_manager.clone()),
            retrospection_repo: RetrospectionRepository::new(db_manager),
            tool_formatter: ToolDisplayFormatter::new(),
        }
    }

    pub async fn set_session_id(&mut self, session_id: Option<String>) -> Result<()> {
        self.state.set_session_id(session_id.clone());
        if session_id.is_some() {
            self.refresh().await?;
        }
        Ok(())
    }

    pub async fn refresh(&mut self) -> Result<()> {
        if let Some(session_id) = &self.state.session_id.clone() {
            self.state.loading = true;

            let request = SessionDetailRequest {
                session_id: session_id.clone(),
                include_content: Some(true),
                message_limit: None,
                message_offset: None,
            };

            match self.query_service.get_session_detail(request).await {
                Ok(response) => {
                    self.state
                        .update_session(response.session, response.messages);
                    self.update_scroll_state();

                    // Load retrospection results for this session
                    self.load_retrospections().await;
                }
                Err(e) => {
                    tracing::error!(error = %e, "Failed to load session details");
                }
            }

            self.state.loading = false;
        }
        Ok(())
    }

    pub async fn handle_key(&mut self, key: KeyEvent) -> Result<()> {
        match key.code {
            KeyCode::Up => {
                self.state.scroll_up();
                self.update_scroll_state();
            }
            KeyCode::Down => {
                let max_scroll = self.get_max_scroll();
                self.state.scroll_down(max_scroll);
                self.update_scroll_state();
            }
            KeyCode::PageUp => {
                let page_size = 10;
                self.state.scroll_page_up(page_size);
                self.update_scroll_state();
            }
            KeyCode::PageDown => {
                let page_size = 10;
                let max_scroll = self.get_max_scroll();
                self.state.scroll_page_down(page_size, max_scroll);
                self.update_scroll_state();
            }
            KeyCode::Home => {
                self.state.scroll_to_top();
                self.update_scroll_state();
            }
            KeyCode::End => {
                let max_scroll = self.get_max_scroll();
                self.state.scroll_to_bottom(max_scroll);
                self.update_scroll_state();
            }
            KeyCode::Char('w') => {
                self.state.toggle_wrap();
            }
            KeyCode::Char('r') => {
                if key.modifiers.contains(KeyModifiers::CONTROL) {
                    // Ctrl+R: Toggle retrospection view
                    self.state.toggle_retrospection();
                }
                // Removed manual refresh - auto-refresh is now handled by the app
            }
            KeyCode::Char('t') => {
                // T: Toggle retrospection view
                self.state.toggle_retrospection();
            }
            KeyCode::Char('d') => {
                // D: Toggle tool details (expand/collapse)
                self.state.toggle_tool_details();
            }
            KeyCode::Char('a') => {
                // A: Start retrospection analysis for current session
                // This would require returning a signal to the app
                // For now, just show a message that this functionality is available via CLI
                // TODO: Implement analysis start from session detail
            }
            _ => {}
        }
        Ok(())
    }

    pub fn render(&mut self, f: &mut Frame, area: Rect) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(4), // Session header
                Constraint::Min(0),    // Main content
            ])
            .split(area);

        // Render session header
        self.render_session_header(f, chunks[0]);

        // Render main content area
        if self.state.show_retrospection && !self.state.retrospections.is_empty() {
            // Split horizontally for messages and retrospection
            let main_chunks = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([
                    Constraint::Percentage(60), // Messages
                    Constraint::Percentage(40), // Retrospection
                ])
                .split(chunks[1]);

            self.render_messages(f, main_chunks[0]);
            self.render_retrospections(f, main_chunks[1]);
        } else {
            // Full width for messages
            self.render_messages(f, chunks[1]);
        }
    }

    fn render_session_header(&self, f: &mut Frame, area: Rect) {
        let content = if self.state.loading {
            "Loading session details...".to_string()
        } else if let Some(session) = &self.state.session {
            let project_str = session.project_name.as_deref().unwrap_or("No Project");
            let _duration = if let Some(_end_time) = &session.end_time {
                // Calculate duration between start and end time
                "Duration calculated"
            } else {
                "Ongoing"
            };

            let retrospection_info = if self.state.retrospections.is_empty() {
                "No retrospections".to_string()
            } else {
                format!("{} retrospections", self.state.retrospections.len())
            };

            format!(
                "Provider: {} | Project: {} | Messages: {} | Tokens: {} | Started: {} | Status: {} | {} | Keys: 'w'=wrap, 't'=retro, 'd'=tool-details",
                session.provider,
                project_str,
                session.message_count,
                session.token_count.unwrap_or(0),
                &session.start_time.format("%Y-%m-%d %H:%M").to_string(),
                session.state,
                retrospection_info
            )
        } else {
            "No session selected".to_string()
        };

        let header = Paragraph::new(content)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("Session Details"),
            )
            .style(Style::default().fg(Color::Cyan))
            .wrap(Wrap { trim: true });

        f.render_widget(header, area);
    }

    fn render_messages(&mut self, f: &mut Frame, area: Rect) {
        if self.state.messages.is_empty() {
            let empty_msg = if self.state.loading {
                "Loading messages..."
            } else {
                "No messages in this session"
            };

            let paragraph = Paragraph::new(empty_msg)
                .block(Block::default().borders(Borders::ALL))
                .style(Style::default().fg(Color::Gray));

            f.render_widget(paragraph, area);
            return;
        }

        // Calculate visible messages based on scroll
        let available_height = area.height.saturating_sub(2) as usize; // Account for borders
        let message_lines = self.calculate_message_lines(area.width.saturating_sub(4) as usize);

        let visible_lines: Vec<Line> = message_lines
            .into_iter()
            .skip(self.state.current_scroll)
            .take(available_height)
            .collect();

        let messages_block = Paragraph::new(visible_lines)
            .block(Block::default().borders(Borders::ALL).title("Messages"))
            .wrap(if self.state.message_wrap {
                Wrap { trim: true }
            } else {
                Wrap { trim: false }
            })
            .scroll((0, 0));

        f.render_widget(messages_block, area);

        // Render scrollbar
        if self.get_total_lines() > available_height {
            let scrollbar_area = Rect {
                x: area.x + area.width - 1,
                y: area.y + 1,
                width: 1,
                height: area.height - 2,
            };

            let scrollbar = Scrollbar::default()
                .orientation(ScrollbarOrientation::VerticalRight)
                .begin_symbol(Some("↑"))
                .end_symbol(Some("↓"));

            f.render_stateful_widget(scrollbar, scrollbar_area, &mut self.state.scroll_state);
        }
    }

    fn calculate_message_lines(&self, width: usize) -> Vec<Line<'_>> {
        let mut lines = Vec::new();

        for (i, message) in self.state.messages.iter().enumerate() {
            // Add separator between messages (except for first)
            if i > 0 {
                lines.push(Line::from(vec![Span::styled(
                    "─".repeat(width.min(80)),
                    Style::default().fg(Color::DarkGray),
                )]));
            }

            // Message header
            let role_style = match message.role {
                MessageRole::User => Style::default()
                    .fg(Color::Green)
                    .add_modifier(Modifier::BOLD),
                MessageRole::Assistant => Style::default()
                    .fg(Color::Blue)
                    .add_modifier(Modifier::BOLD),
                MessageRole::System => Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            };

            let timestamp = &message.timestamp.format("%H:%M:%S").to_string(); // Just show time
            let header = format!("[{}] {:?}", timestamp, message.role);

            lines.push(Line::from(vec![
                Span::styled(header, role_style),
                Span::raw(format!(" ({})", message.sequence_number)),
            ]));

            // Message content - wrap if needed
            let content_lines = if self.state.message_wrap {
                wrap_text(&message.content, width.saturating_sub(2))
            } else {
                vec![message.content.clone()]
            };

            for content_line in content_lines {
                lines.push(Line::from(vec![Span::styled(
                    format!("  {content_line}"),
                    Style::default().fg(Color::White),
                )]));
            }

            // Show tool uses inline (new unified format)
            if let Some(tool_uses) = &message.tool_uses {
                if !tool_uses.is_empty() {
                    let tool_results = message.tool_results.as_deref().unwrap_or(&[]);

                    // Create tool display config
                    let tool_config = ToolDisplayConfig {
                        width: width.saturating_sub(4),
                        show_details: self.state.show_tool_details,
                        max_output_lines: 10,
                    };

                    // Format and add tool display lines
                    let tool_lines =
                        self.tool_formatter
                            .format_tools(tool_uses, tool_results, &tool_config);

                    // Indent tool lines
                    for tool_line in tool_lines {
                        let indented_spans: Vec<Span> = std::iter::once(Span::raw("  "))
                            .chain(tool_line.spans.into_iter())
                            .collect();
                        lines.push(Line::from(indented_spans));
                    }
                }
            }
            // Show old tool calls format for backwards compatibility (if no tool_uses)
            else if let Some(tool_calls) = &message.tool_calls {
                if !tool_calls.is_empty() {
                    lines.push(Line::from(vec![Span::styled(
                        "  [Tool Calls - Legacy Format]",
                        Style::default()
                            .fg(Color::Magenta)
                            .add_modifier(Modifier::ITALIC),
                    )]));

                    // Parse and display tool calls (simplified)
                    let tools_preview =
                        truncate_text(&format!("{tool_calls:?}"), width.saturating_sub(4));
                    for tool_line in wrap_text(&tools_preview, width.saturating_sub(4)) {
                        lines.push(Line::from(vec![Span::styled(
                            format!("    {tool_line}"),
                            Style::default().fg(Color::DarkGray),
                        )]));
                    }
                }
            }

            // Token count if available
            if let Some(tokens) = message.token_count {
                lines.push(Line::from(vec![Span::styled(
                    format!("  [Tokens: {tokens}]"),
                    Style::default().fg(Color::Cyan).add_modifier(Modifier::DIM),
                )]));
            }

            // Add empty line for readability
            lines.push(Line::from(""));
        }

        lines
    }

    fn get_total_lines(&self) -> usize {
        self.calculate_message_lines(80).len() // Use standard width for calculation
    }

    pub fn get_max_scroll(&self) -> usize {
        let total_lines = self.get_total_lines();
        let visible_lines = 20; // Approximate visible lines
        total_lines.saturating_sub(visible_lines)
    }

    fn update_scroll_state(&mut self) {
        let total_lines = self.get_total_lines();
        self.state.update_scroll_state(total_lines);
    }

    async fn load_retrospections(&mut self) {
        if let Some(session_id) = &self.state.session_id.clone() {
            match self.retrospection_repo.get_by_session_id(session_id).await {
                Ok(retrospections) => {
                    self.state.update_retrospections(retrospections);
                }
                Err(e) => {
                    tracing::error!(error = %e, "Failed to load retrospections");
                    self.state.update_retrospections(Vec::new());
                }
            }
        }
    }

    fn render_retrospections(&mut self, f: &mut Frame, area: Rect) {
        if self.state.retrospections.is_empty() {
            let empty_msg = Paragraph::new("No retrospection analysis available\n\nUse 'retrochat retrospect execute' to analyze this session")
                .block(Block::default().borders(Borders::ALL).title("Retrospection"))
                .style(Style::default().fg(Color::Gray))
                .wrap(Wrap { trim: true });

            f.render_widget(empty_msg, area);
            return;
        }

        // For now, show the most recent retrospection
        // TODO: Allow user to navigate between multiple retrospections
        if let Some(retrospection) = self.state.retrospections.first() {
            let lines = vec![
                Line::from(vec![Span::styled(
                    "Retrospection Analysis",
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD),
                )]),
                Line::from(""),
                Line::from(vec![Span::styled(
                    "Insights:",
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD),
                )]),
                Line::from(""),
            ];

            // Create content from retrospection fields
            let mut content_lines = Vec::new();

            // Add insights
            content_lines.extend(retrospection.insights.lines().map(Line::from));
            content_lines.push(Line::from(""));

            // Add reflection section
            content_lines.push(Line::from(vec![Span::styled(
                "Reflection:",
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            )]));
            content_lines.push(Line::from(""));
            content_lines.extend(retrospection.reflection.lines().map(Line::from));
            content_lines.push(Line::from(""));

            // Add recommendations section
            content_lines.push(Line::from(vec![Span::styled(
                "Recommendations:",
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            )]));
            content_lines.push(Line::from(""));
            content_lines.extend(retrospection.recommendations.lines().map(Line::from));

            let all_lines = [lines, content_lines].concat();

            // Handle scrolling for retrospection panel
            let available_height = area.height.saturating_sub(2) as usize;
            let visible_lines: Vec<Line> = all_lines
                .into_iter()
                .skip(self.state.retrospection_scroll)
                .take(available_height)
                .collect();

            let retrospection_block = Paragraph::new(visible_lines)
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .title("Retrospection Analysis"),
                )
                .wrap(Wrap { trim: true })
                .scroll((0, 0));

            f.render_widget(retrospection_block, area);

            // Add metadata at bottom if space allows
            if area.height > 10 {
                let metadata_area = Rect {
                    x: area.x,
                    y: area.y + area.height - 3,
                    width: area.width,
                    height: 2,
                };

                let metadata_text = if let Some(tokens) = retrospection.token_usage {
                    format!(
                        "Tokens: {} | Created: {}",
                        tokens,
                        retrospection.created_at.format("%Y-%m-%d %H:%M")
                    )
                } else {
                    format!(
                        "Created: {}",
                        retrospection.created_at.format("%Y-%m-%d %H:%M")
                    )
                };

                let metadata = Paragraph::new(metadata_text)
                    .style(Style::default().fg(Color::DarkGray))
                    .wrap(Wrap { trim: true });

                f.render_widget(metadata, metadata_area);
            }
        }
    }
}
