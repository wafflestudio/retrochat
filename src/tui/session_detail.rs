use anyhow::Result;
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Scrollbar, ScrollbarOrientation, Wrap},
    Frame,
};
use std::sync::Arc;

use crate::database::DatabaseManager;
use crate::models::{Message, MessageRole};
use crate::services::{MessageGroup, QueryService, SessionDetailRequest};

use super::state::SessionDetailState;
use super::tool_display::{ToolDisplayConfig, ToolDisplayFormatter};
use super::utils::text::wrap_text;

pub struct SessionDetailWidget {
    pub state: SessionDetailState,
    query_service: QueryService,
    tool_formatter: ToolDisplayFormatter,
}

impl SessionDetailWidget {
    pub fn new(db_manager: Arc<DatabaseManager>) -> Self {
        Self {
            state: SessionDetailState::new(),
            query_service: QueryService::with_database(db_manager.clone()),
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
                }
                Err(e) => {
                    tracing::error!(error = %e, "Failed to load session details");
                }
            }

            // Load analytics data for this session
            match self.query_service.get_session_analytics(session_id).await {
                Ok(analytics) => {
                    self.state.update_analytics(analytics);
                }
                Err(e) => {
                    tracing::debug!(error = %e, "No analytics available for session");
                    self.state.update_analytics(None);
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
            KeyCode::Char('d') => {
                // D: Toggle tool details (expand/collapse)
                self.state.toggle_tool_details();
            }
            KeyCode::Char('a') => {
                // A: Toggle analytics panel
                self.state.toggle_analytics();
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

        // Render content based on toggle state
        if self.state.show_analytics && self.state.analytics.is_some() {
            // Show only analytics when toggled
            self.render_analytics(f, chunks[1]);
        } else {
            // Show messages by default
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

            // Check if analytics is available
            let analytics_str = if self.state.analytics.is_some() {
                " | Analytics: Available"
            } else {
                ""
            };

            format!(
                "Provider: {} | Project: {} | Messages: {} | Tokens: {} | Started: {} | Status: {}{} | Keys: 'd'=tool-details 'a'=analytics",
                session.provider,
                project_str,
                session.message_count,
                session.token_count.unwrap_or(0),
                &session.start_time.format("%Y-%m-%d %H:%M").to_string(),
                session.state,
                analytics_str,
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
            .wrap(Wrap { trim: true })
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

        // Pair tool_use and tool_result messages
        let message_groups = MessageGroup::pair_tool_messages(self.state.messages.clone());

        for (group_idx, group) in message_groups.iter().enumerate() {
            // Add separator between groups (except for first)
            if group_idx > 0 {
                lines.push(Line::from(vec![Span::styled(
                    "─".repeat(width.min(80)),
                    Style::default().fg(Color::DarkGray),
                )]));
            }

            match group {
                MessageGroup::Single(message) => {
                    self.render_message_block(message, width, &mut lines);
                }
                MessageGroup::ToolPair {
                    tool_use_message,
                    tool_result_message,
                } => {
                    self.render_tool_pair_block(
                        tool_use_message,
                        tool_result_message,
                        width,
                        &mut lines,
                    );
                }
            }
        }

        lines
    }

    /// Renders a single message block
    fn render_message_block(&self, message: &Message, width: usize, lines: &mut Vec<Line<'_>>) {
        // Check if this is a thinking message
        let is_thinking = message.is_thinking();

        // Message header
        let role_style = if is_thinking {
            // Thinking messages have distinct styling
            Style::default()
                .fg(Color::Magenta)
                .add_modifier(Modifier::BOLD | Modifier::ITALIC)
        } else {
            match message.role {
                MessageRole::User => Style::default()
                    .fg(Color::Green)
                    .add_modifier(Modifier::BOLD),
                MessageRole::Assistant => Style::default()
                    .fg(Color::Blue)
                    .add_modifier(Modifier::BOLD),
                MessageRole::System => Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            }
        };

        let timestamp = &message.timestamp.format("%H:%M:%S").to_string(); // Just show time
        let header = if is_thinking {
            format!("[{timestamp}] Thinking")
        } else {
            format!("[{timestamp}] {:?}", message.role)
        };

        lines.push(Line::from(vec![
            Span::styled(header, role_style),
            Span::raw(format!(" ({})", message.sequence_number)),
        ]));

        // Message content - wrap text and preserve newlines
        let content_lines = wrap_text(&message.content, width.saturating_sub(2));

        // Use different styling for thinking content
        let content_style = if is_thinking {
            Style::default()
                .fg(Color::Rgb(180, 140, 200)) // Light purple for thinking
                .add_modifier(Modifier::ITALIC)
        } else {
            Style::default().fg(Color::White)
        };

        for content_line in content_lines {
            lines.push(Line::from(vec![Span::styled(
                format!("  {content_line}"),
                content_style,
            )]));
        }

        // Show tool operation indicator if message has tool operations
        if message.has_tool_operation() {
            let indicator_text = match message.message_type {
                crate::models::message::MessageType::ToolRequest => "  [Tool Request]",
                crate::models::message::MessageType::ToolResult => "  [Tool Result]",
                _ => "  [Tool Operation]",
            };

            lines.push(Line::from(vec![Span::styled(
                indicator_text,
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::ITALIC),
            )]));

            // TODO: Load and display full tool operation details from database
            // This would require fetching ToolOperation via tool_operation_id
            if self.state.show_tool_details {
                lines.push(Line::from(vec![Span::styled(
                    "    (Tool details available - future enhancement)",
                    Style::default()
                        .fg(Color::DarkGray)
                        .add_modifier(Modifier::DIM),
                )]));
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

    /// Renders a tool use/result pair as a unified block
    fn render_tool_pair_block(
        &self,
        tool_use_msg: &Message,
        tool_result_msg: &Message,
        width: usize,
        lines: &mut Vec<Line<'_>>,
    ) {
        // Render the tool use message header
        let role_style = Style::default()
            .fg(Color::Blue)
            .add_modifier(Modifier::BOLD);

        let timestamp = &tool_use_msg.timestamp.format("%H:%M:%S").to_string();
        let header = format!("[{}] {:?} → Tool Execution", timestamp, tool_use_msg.role);

        lines.push(Line::from(vec![
            Span::styled(header, role_style),
            Span::raw(format!(
                " ({}, {})",
                tool_use_msg.sequence_number, tool_result_msg.sequence_number
            )),
        ]));

        // Show tool use message content - wrap text and preserve newlines
        let content_lines = wrap_text(&tool_use_msg.content, width.saturating_sub(2));

        for content_line in content_lines {
            lines.push(Line::from(vec![Span::styled(
                format!("  {content_line}"),
                Style::default().fg(Color::White),
            )]));
        }

        // Render tool uses with their results
        if let (Some(tool_uses), Some(tool_results)) =
            (&tool_use_msg.tool_uses, &tool_result_msg.tool_results)
        {
            let tool_config = ToolDisplayConfig {
                width: width.saturating_sub(4),
                show_details: self.state.show_tool_details,
                max_output_lines: 10,
            };

            // Format and add tool display lines
            let tool_lines =
                self.tool_formatter
                    .format_tools(tool_uses, tool_results, &tool_config);

            // Add visual indicator for paired tool execution
            lines.push(Line::from(vec![Span::styled(
                "  ├─ Tool Execution & Result",
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::ITALIC),
            )]));

            // Indent tool lines
            for tool_line in tool_lines {
                let indented_spans: Vec<Span> = std::iter::once(Span::raw("  │  "))
                    .chain(tool_line.spans.into_iter())
                    .collect();
                lines.push(Line::from(indented_spans));
            }
        }

        // Token counts from both messages
        if let (Some(use_tokens), Some(result_tokens)) =
            (tool_use_msg.token_count, tool_result_msg.token_count)
        {
            let total = use_tokens + result_tokens;
            lines.push(Line::from(vec![Span::styled(
                format!("  [Tokens: {total} (use: {use_tokens}, result: {result_tokens})]"),
                Style::default().fg(Color::Cyan).add_modifier(Modifier::DIM),
            )]));
        } else if let Some(tokens) = tool_use_msg.token_count.or(tool_result_msg.token_count) {
            lines.push(Line::from(vec![Span::styled(
                format!("  [Tokens: {tokens}]"),
                Style::default().fg(Color::Cyan).add_modifier(Modifier::DIM),
            )]));
        }

        // Add empty line for readability
        lines.push(Line::from(""));
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

    fn render_analytics(&self, f: &mut Frame, area: Rect) {
        let analytics_data = match &self.state.analytics {
            Some(session_analytics) => session_analytics,
            None => {
                let paragraph = Paragraph::new("No analytics available")
                    .block(Block::default().borders(Borders::ALL).title("Analytics"))
                    .style(Style::default().fg(Color::Gray));
                f.render_widget(paragraph, area);
                return;
            }
        };

        let mut lines = Vec::new();

        // Show status of latest request
        if let Some(request) = &analytics_data.latest_request {
            lines.push(Line::from(vec![
                Span::styled("Status: ", Style::default().add_modifier(Modifier::BOLD)),
                Span::styled(
                    format!("{:?}", request.status),
                    Style::default().fg(match request.status {
                        crate::models::OperationStatus::Completed => Color::Green,
                        crate::models::OperationStatus::Running => Color::Yellow,
                        crate::models::OperationStatus::Pending => Color::Blue,
                        crate::models::OperationStatus::Failed => Color::Red,
                        crate::models::OperationStatus::Cancelled => Color::Gray,
                    }),
                ),
            ]));
            lines.push(Line::from(""));
        }

        // Show active request if any
        if let Some(active) = &analytics_data.active_request {
            lines.push(Line::from(vec![Span::styled(
                format!("⏳ Analysis in progress: {:?}", active.status),
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::ITALIC),
            )]));
            lines.push(Line::from(""));
        }

        // Show analytics results if available
        if let Some(analytics) = &analytics_data.latest_analytics {
            // Scores section
            lines.push(Line::from(vec![Span::styled(
                "📊 Scores",
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD | Modifier::UNDERLINED),
            )]));
            lines.push(Line::from(""));

            let scores = &analytics.scores;
            lines.push(Line::from(vec![
                Span::raw("  Overall:        "),
                Span::styled(
                    format!("{:.1}/10", scores.overall),
                    Style::default()
                        .fg(Self::score_color(scores.overall))
                        .add_modifier(Modifier::BOLD),
                ),
            ]));
            lines.push(Line::from(vec![
                Span::raw("  Code Quality:   "),
                Span::styled(
                    format!("{:.1}/10", scores.code_quality),
                    Style::default().fg(Self::score_color(scores.code_quality)),
                ),
            ]));
            lines.push(Line::from(vec![
                Span::raw("  Productivity:   "),
                Span::styled(
                    format!("{:.1}/10", scores.productivity),
                    Style::default().fg(Self::score_color(scores.productivity)),
                ),
            ]));
            lines.push(Line::from(vec![
                Span::raw("  Efficiency:     "),
                Span::styled(
                    format!("{:.1}/10", scores.efficiency),
                    Style::default().fg(Self::score_color(scores.efficiency)),
                ),
            ]));
            lines.push(Line::from(vec![
                Span::raw("  Collaboration:  "),
                Span::styled(
                    format!("{:.1}/10", scores.collaboration),
                    Style::default().fg(Self::score_color(scores.collaboration)),
                ),
            ]));
            lines.push(Line::from(vec![
                Span::raw("  Learning:       "),
                Span::styled(
                    format!("{:.1}/10", scores.learning),
                    Style::default().fg(Self::score_color(scores.learning)),
                ),
            ]));
            lines.push(Line::from(""));

            // Metrics section
            lines.push(Line::from(vec![Span::styled(
                "📈 Metrics",
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD | Modifier::UNDERLINED),
            )]));
            lines.push(Line::from(""));

            let metrics = &analytics.metrics;
            lines.push(Line::from(format!(
                "  Files Modified:     {}",
                metrics.total_files_modified
            )));
            lines.push(Line::from(format!(
                "  Files Read:         {}",
                metrics.total_files_read
            )));
            lines.push(Line::from(format!(
                "  Lines Added:        {}",
                metrics.lines_added
            )));
            lines.push(Line::from(format!(
                "  Lines Removed:      {}",
                metrics.lines_removed
            )));
            lines.push(Line::from(format!(
                "  Tokens Used:        {}",
                metrics.total_tokens_used
            )));
            lines.push(Line::from(format!(
                "  Duration:           {:.1} min",
                metrics.session_duration_minutes
            )));
            lines.push(Line::from(""));

            // Key insights from qualitative output
            if !analytics.qualitative_output.insights.is_empty() {
                lines.push(Line::from(vec![Span::styled(
                    "💡 Key Insights",
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD | Modifier::UNDERLINED),
                )]));
                lines.push(Line::from(""));

                // Show first few insights
                for (idx, insight) in analytics
                    .qualitative_output
                    .insights
                    .iter()
                    .take(3)
                    .enumerate()
                {
                    lines.push(Line::from(vec![Span::styled(
                        format!("  {}. {}", idx + 1, insight.title),
                        Style::default()
                            .fg(Color::Yellow)
                            .add_modifier(Modifier::BOLD),
                    )]));

                    // Wrap the insight description
                    let wrapped_desc =
                        wrap_text(&insight.description, area.width.saturating_sub(6) as usize);
                    for line in wrapped_desc {
                        lines.push(Line::from(format!("     {}", line)));
                    }
                    lines.push(Line::from(""));
                }
            }

            // Show model used
            if let Some(model) = &analytics.model_used {
                lines.push(Line::from(vec![
                    Span::styled("Model: ", Style::default().fg(Color::DarkGray)),
                    Span::styled(model, Style::default().fg(Color::Gray)),
                ]));
            }
        } else if analytics_data.active_request.is_none() {
            lines.push(Line::from(vec![Span::styled(
                "No completed analysis available",
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::ITALIC),
            )]));
        }

        let paragraph = Paragraph::new(lines)
            .block(Block::default().borders(Borders::ALL).title("Analytics"))
            .wrap(Wrap { trim: true })
            .scroll((0, 0));

        f.render_widget(paragraph, area);
    }

    fn score_color(score: f64) -> Color {
        if score >= 8.0 {
            Color::Green
        } else if score >= 6.0 {
            Color::Yellow
        } else if score >= 4.0 {
            Color::Rgb(255, 165, 0) // Orange
        } else {
            Color::Red
        }
    }
}
