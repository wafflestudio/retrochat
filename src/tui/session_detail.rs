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

use super::state::session_detail_state::AnalyticsPanelFocus;
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
        // Check if we should scroll analytics instead of messages
        let scroll_analytics = self.state.show_analytics && self.state.analytics.is_some();

        match key.code {
            KeyCode::Up => {
                if scroll_analytics {
                    self.state.focused_panel_scroll_up();
                    self.update_dual_panel_scroll_state();
                } else {
                    self.state.scroll_up();
                    self.update_scroll_state();
                }
            }
            KeyCode::Down => {
                if scroll_analytics {
                    let (quant_max, qual_max) = self.get_dual_panel_max_scroll();
                    self.state.focused_panel_scroll_down(quant_max, qual_max);
                    self.update_dual_panel_scroll_state();
                } else {
                    let max_scroll = self.get_max_scroll();
                    self.state.scroll_down(max_scroll);
                    self.update_scroll_state();
                }
            }
            KeyCode::PageUp => {
                let page_size = 10;
                if scroll_analytics {
                    self.state.focused_panel_page_up(page_size);
                    self.update_dual_panel_scroll_state();
                } else {
                    self.state.scroll_page_up(page_size);
                    self.update_scroll_state();
                }
            }
            KeyCode::PageDown => {
                let page_size = 10;
                if scroll_analytics {
                    let (quant_max, qual_max) = self.get_dual_panel_max_scroll();
                    self.state
                        .focused_panel_page_down(page_size, quant_max, qual_max);
                    self.update_dual_panel_scroll_state();
                } else {
                    let max_scroll = self.get_max_scroll();
                    self.state.scroll_page_down(page_size, max_scroll);
                    self.update_scroll_state();
                }
            }
            KeyCode::Home => {
                if scroll_analytics {
                    self.state.focused_panel_scroll_to_top();
                    self.update_dual_panel_scroll_state();
                } else {
                    self.state.scroll_to_top();
                    self.update_scroll_state();
                }
            }
            KeyCode::End => {
                if scroll_analytics {
                    let (quant_max, qual_max) = self.get_dual_panel_max_scroll();
                    self.state
                        .focused_panel_scroll_to_bottom(quant_max, qual_max);
                    self.update_dual_panel_scroll_state();
                } else {
                    let max_scroll = self.get_max_scroll();
                    self.state.scroll_to_bottom(max_scroll);
                    self.update_scroll_state();
                }
            }
            KeyCode::Left | KeyCode::Right => {
                // Left/Right: Switch focus between quantitative and qualitative panels
                if scroll_analytics {
                    self.state.toggle_analytics_panel_focus();
                }
            }
            KeyCode::Char('d') => {
                // D: Toggle tool details (expand/collapse)
                self.state.toggle_tool_details();
            }
            KeyCode::Char('a') => {
                // A: Toggle analytics panel
                self.state.toggle_analytics();
            }
            KeyCode::Char('t') => {
                // T: Toggle thinking messages visibility
                self.state.toggle_thinking();
                // Clamp scroll position if it's now out of bounds
                let max_scroll = self.get_max_scroll();
                if self.state.current_scroll > max_scroll {
                    self.state.current_scroll = max_scroll;
                }
                self.update_scroll_state();
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
                "Provider: {} | Project: {} | Messages: {} | Tokens: {} | Started: {} | Status: {}{}",
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

        // Store viewport height for scroll calculations
        self.state.viewport_height = available_height;

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

        // Filter out thinking messages if hidden
        let messages = if self.state.show_thinking {
            self.state.messages.clone()
        } else {
            self.state
                .messages
                .iter()
                .filter(|msg| !msg.is_thinking())
                .cloned()
                .collect()
        };

        // Pair tool_use and tool_result messages
        let message_groups = MessageGroup::pair_tool_messages(messages);

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
        // Use actual viewport height from last render
        total_lines.saturating_sub(self.state.viewport_height)
    }

    fn update_scroll_state(&mut self) {
        let total_lines = self.get_total_lines();
        self.state.update_scroll_state(total_lines);
    }

    fn render_analytics(&mut self, f: &mut Frame, area: Rect) {
        if self.state.analytics.is_none() {
            let paragraph = Paragraph::new("No analytics available")
                .block(Block::default().borders(Borders::ALL).title("Analytics"))
                .style(Style::default().fg(Color::Gray));
            f.render_widget(paragraph, area);
            return;
        }

        // Split area into two horizontal panels
        let panels = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
            .split(area);

        // Render quantitative panel (left)
        self.render_quantitative_panel(f, panels[0]);

        // Render qualitative panel (right)
        self.render_qualitative_panel(f, panels[1]);
    }

    fn render_quantitative_panel(&mut self, f: &mut Frame, area: Rect) {
        let analytics_data = self.state.analytics.as_ref().unwrap();
        let is_focused = self.state.analytics_panel_focus == AnalyticsPanelFocus::Quantitative;
        let border_style = if is_focused {
            Style::default().fg(Color::Cyan)
        } else {
            Style::default().fg(Color::DarkGray)
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
                format!("Analysis in progress: {:?}", active.status),
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::ITALIC),
            )]));
            lines.push(Line::from(""));
        }

        if let Some(analytics) = &analytics_data.latest_analytics {
            let bar_width = area.width.saturating_sub(20) as usize;

            // AI Quantitative Output - Rubric Scores
            if !analytics.ai_quantitative_output.rubric_scores.is_empty() {
                lines.push(Line::from(vec![Span::styled(
                    "Rubric Scores",
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD),
                )]));
                lines.push(Line::from(""));

                for score in &analytics.ai_quantitative_output.rubric_scores {
                    // Rubric name
                    lines.push(Line::from(vec![Span::styled(
                        format!("  {}", score.rubric_name),
                        Style::default().fg(Color::White),
                    )]));

                    // Bar visualization
                    let percentage = score.percentage();
                    let filled = (percentage / 100.0 * bar_width as f64) as usize;
                    let empty = bar_width.saturating_sub(filled);

                    let bar_color = if percentage >= 80.0 {
                        Color::Green
                    } else if percentage >= 60.0 {
                        Color::Yellow
                    } else {
                        Color::Red
                    };

                    lines.push(Line::from(vec![
                        Span::raw("  "),
                        Span::styled("█".repeat(filled), Style::default().fg(bar_color)),
                        Span::styled("░".repeat(empty), Style::default().fg(Color::DarkGray)),
                        Span::styled(
                            format!(" {:.0}/{:.0}", score.score, score.max_score),
                            Style::default().fg(Color::White),
                        ),
                    ]));
                    lines.push(Line::from(""));
                }

                // Show summary if available
                if let Some(summary) = &analytics.ai_quantitative_output.rubric_summary {
                    lines.push(Line::from(vec![Span::styled(
                        format!(
                            "Total: {:.1}/{:.1} ({:.0}%)",
                            summary.total_score, summary.max_score, summary.percentage
                        ),
                        Style::default()
                            .fg(Color::Cyan)
                            .add_modifier(Modifier::BOLD),
                    )]));
                    lines.push(Line::from(""));
                }
            }

            // Metric Quantitative Output
            lines.push(Line::from(vec![Span::styled(
                "Session Metrics",
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            )]));
            lines.push(Line::from(""));

            let metrics = &analytics.metric_quantitative_output;

            // Token metrics
            lines.push(Line::from(vec![Span::styled(
                "  Tokens",
                Style::default().fg(Color::White),
            )]));
            let token_total = metrics.token_metrics.total_tokens_used;
            let input_ratio = if token_total > 0 {
                metrics.token_metrics.input_tokens as f64 / token_total as f64
            } else {
                0.0
            };
            let input_filled = (input_ratio * bar_width as f64) as usize;
            let output_filled = bar_width.saturating_sub(input_filled);
            lines.push(Line::from(vec![
                Span::raw("  "),
                Span::styled("█".repeat(input_filled), Style::default().fg(Color::Blue)),
                Span::styled("█".repeat(output_filled), Style::default().fg(Color::Green)),
                Span::styled(
                    format!(" {} total", token_total),
                    Style::default().fg(Color::White),
                ),
            ]));
            lines.push(Line::from(vec![
                Span::raw("  "),
                Span::styled("■", Style::default().fg(Color::Blue)),
                Span::styled(
                    format!(" In: {} ", metrics.token_metrics.input_tokens),
                    Style::default().fg(Color::DarkGray),
                ),
                Span::styled("■", Style::default().fg(Color::Green)),
                Span::styled(
                    format!(" Out: {}", metrics.token_metrics.output_tokens),
                    Style::default().fg(Color::DarkGray),
                ),
            ]));
            lines.push(Line::from(""));

            // File changes
            lines.push(Line::from(vec![Span::styled(
                "  File Changes",
                Style::default().fg(Color::White),
            )]));
            let file_changes = &metrics.file_changes;
            let total_lines_changed =
                (file_changes.lines_added + file_changes.lines_removed) as usize;
            let add_ratio = if total_lines_changed > 0 {
                file_changes.lines_added as f64 / total_lines_changed as f64
            } else {
                0.5
            };
            let add_filled = (add_ratio * bar_width as f64) as usize;
            let remove_filled = bar_width.saturating_sub(add_filled);
            lines.push(Line::from(vec![
                Span::raw("  "),
                Span::styled("█".repeat(add_filled), Style::default().fg(Color::Green)),
                Span::styled("█".repeat(remove_filled), Style::default().fg(Color::Red)),
            ]));
            lines.push(Line::from(vec![
                Span::raw("  "),
                Span::styled("+", Style::default().fg(Color::Green)),
                Span::styled(
                    format!("{} ", file_changes.lines_added),
                    Style::default().fg(Color::DarkGray),
                ),
                Span::styled("-", Style::default().fg(Color::Red)),
                Span::styled(
                    format!("{} ", file_changes.lines_removed),
                    Style::default().fg(Color::DarkGray),
                ),
                Span::styled(
                    format!(
                        "(net: {}{})",
                        if file_changes.net_code_growth >= 0 {
                            "+"
                        } else {
                            ""
                        },
                        file_changes.net_code_growth
                    ),
                    Style::default().fg(Color::White),
                ),
            ]));
            lines.push(Line::from(""));

            // Tool usage
            lines.push(Line::from(vec![Span::styled(
                "  Tool Usage",
                Style::default().fg(Color::White),
            )]));
            let tool_usage = &metrics.tool_usage;
            let total_ops = tool_usage.total_operations as usize;
            let success_ratio = if total_ops > 0 {
                tool_usage.successful_operations as f64 / total_ops as f64
            } else {
                1.0
            };
            let success_filled = (success_ratio * bar_width as f64) as usize;
            let failed_filled = bar_width.saturating_sub(success_filled);
            lines.push(Line::from(vec![
                Span::raw("  "),
                Span::styled(
                    "█".repeat(success_filled),
                    Style::default().fg(Color::Green),
                ),
                Span::styled("█".repeat(failed_filled), Style::default().fg(Color::Red)),
                Span::styled(
                    format!(" {} ops", total_ops),
                    Style::default().fg(Color::White),
                ),
            ]));
            lines.push(Line::from(vec![
                Span::raw("  "),
                Span::styled(
                    format!("OK: {} ", tool_usage.successful_operations),
                    Style::default().fg(Color::Green),
                ),
                Span::styled(
                    format!("Fail: {}", tool_usage.failed_operations),
                    Style::default().fg(Color::Red),
                ),
            ]));
            lines.push(Line::from(""));

            // Time metrics
            lines.push(Line::from(vec![Span::styled(
                format!(
                    "  Duration: {:.1} min",
                    metrics.time_metrics.total_session_time_minutes
                ),
                Style::default().fg(Color::White),
            )]));

            // Model used
            if let Some(model) = &analytics.model_used {
                lines.push(Line::from(""));
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

        let available_height = area.height.saturating_sub(2) as usize;
        self.state.quantitative_viewport_height = available_height;

        let total_lines = lines.len();
        let visible_lines: Vec<Line> = lines
            .into_iter()
            .skip(self.state.quantitative_scroll)
            .take(available_height)
            .collect();

        let title = if is_focused {
            "Quantitative [*]"
        } else {
            "Quantitative"
        };
        let paragraph = Paragraph::new(visible_lines)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(title)
                    .border_style(border_style),
            )
            .wrap(Wrap { trim: true });

        f.render_widget(paragraph, area);

        // Render scrollbar if needed
        if total_lines > available_height {
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

            f.render_stateful_widget(
                scrollbar,
                scrollbar_area,
                &mut self.state.quantitative_scroll_state,
            );
        }
    }

    fn render_qualitative_panel(&mut self, f: &mut Frame, area: Rect) {
        let analytics_data = self.state.analytics.as_ref().unwrap();
        let is_focused = self.state.analytics_panel_focus == AnalyticsPanelFocus::Qualitative;
        let border_style = if is_focused {
            Style::default().fg(Color::Cyan)
        } else {
            Style::default().fg(Color::DarkGray)
        };

        let mut lines = Vec::new();
        let content_width = area.width.saturating_sub(6) as usize;

        if let Some(analytics) = &analytics_data.latest_analytics {
            // Render all qualitative entries
            for entry in &analytics.ai_qualitative_output.entries {
                // Entry title
                lines.push(Line::from(vec![Span::styled(
                    &entry.title,
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD),
                )]));

                // Summary
                if !entry.summary.is_empty() {
                    let wrapped_summary = wrap_text(&entry.summary, content_width);
                    for line in wrapped_summary {
                        lines.push(Line::from(vec![Span::styled(
                            format!("  {line}"),
                            Style::default()
                                .fg(Color::Yellow)
                                .add_modifier(Modifier::ITALIC),
                        )]));
                    }
                }
                lines.push(Line::from(""));

                // Items
                for (idx, item) in entry.items.iter().enumerate() {
                    let wrapped = wrap_text(item, content_width.saturating_sub(4));
                    for (i, line) in wrapped.iter().enumerate() {
                        if i == 0 {
                            lines.push(Line::from(vec![Span::styled(
                                format!("  {}. {line}", idx + 1),
                                Style::default().fg(Color::White),
                            )]));
                        } else {
                            lines.push(Line::from(vec![Span::styled(
                                format!("     {line}"),
                                Style::default().fg(Color::White),
                            )]));
                        }
                    }
                }
                lines.push(Line::from(""));
            }

            // If no entries, show summary info
            if analytics.ai_qualitative_output.entries.is_empty() {
                if let Some(summary) = &analytics.ai_qualitative_output.summary {
                    lines.push(Line::from(vec![Span::styled(
                        format!(
                            "Categories: {}, Entries: {}",
                            summary.categories_evaluated, summary.total_entries
                        ),
                        Style::default().fg(Color::DarkGray),
                    )]));
                }
            }
        } else {
            lines.push(Line::from(vec![Span::styled(
                "No qualitative analysis available",
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::ITALIC),
            )]));
        }

        let available_height = area.height.saturating_sub(2) as usize;
        self.state.qualitative_viewport_height = available_height;

        let total_lines = lines.len();
        let visible_lines: Vec<Line> = lines
            .into_iter()
            .skip(self.state.qualitative_scroll)
            .take(available_height)
            .collect();

        let title = if is_focused {
            "Qualitative [*]"
        } else {
            "Qualitative"
        };
        let paragraph = Paragraph::new(visible_lines)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(title)
                    .border_style(border_style),
            )
            .wrap(Wrap { trim: true });

        f.render_widget(paragraph, area);

        // Render scrollbar if needed
        if total_lines > available_height {
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

            f.render_stateful_widget(
                scrollbar,
                scrollbar_area,
                &mut self.state.qualitative_scroll_state,
            );
        }
    }

    fn get_quantitative_total_lines(&self) -> usize {
        if let Some(analytics_data) = &self.state.analytics {
            let mut count = 0;

            // Status and active request
            if analytics_data.latest_request.is_some() {
                count += 2;
            }
            if analytics_data.active_request.is_some() {
                count += 2;
            }

            if let Some(analytics) = &analytics_data.latest_analytics {
                // Rubric scores
                if !analytics.ai_quantitative_output.rubric_scores.is_empty() {
                    count += 2; // header
                    count += analytics.ai_quantitative_output.rubric_scores.len() * 3;
                    if analytics.ai_quantitative_output.rubric_summary.is_some() {
                        count += 2;
                    }
                }
                // Session metrics: header + tokens(4) + files(4) + tools(4) + time(1) + model(2)
                count += 2 + 4 + 4 + 4 + 1 + 2;
            } else {
                count += 1;
            }
            count
        } else {
            1
        }
    }

    fn get_qualitative_total_lines(&self) -> usize {
        if let Some(analytics_data) = &self.state.analytics {
            if let Some(analytics) = &analytics_data.latest_analytics {
                let mut count = 0;
                for entry in &analytics.ai_qualitative_output.entries {
                    count += 1; // title
                    count += 2; // summary + blank
                    count += entry.items.len() * 2; // items with spacing
                    count += 1; // blank line between entries
                }
                if count == 0 {
                    count = 1;
                }
                count
            } else {
                1
            }
        } else {
            1
        }
    }

    fn get_dual_panel_max_scroll(&self) -> (usize, usize) {
        let quant_total = self.get_quantitative_total_lines();
        let qual_total = self.get_qualitative_total_lines();
        let quant_max = quant_total.saturating_sub(self.state.quantitative_viewport_height);
        let qual_max = qual_total.saturating_sub(self.state.qualitative_viewport_height);
        (quant_max, qual_max)
    }

    fn update_dual_panel_scroll_state(&mut self) {
        let quant_total = self.get_quantitative_total_lines();
        let qual_total = self.get_qualitative_total_lines();
        self.state.update_quantitative_scroll_state(quant_total);
        self.state.update_qualitative_scroll_state(qual_total);
    }
}
