use anyhow::Result;
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState, Wrap},
    Frame,
};
use std::sync::Arc;

use crate::database::DatabaseManager;
use crate::models::{ChatSession, Message, MessageRole};
use crate::services::{QueryService, SessionDetailRequest};

pub struct SessionDetailWidget {
    session: Option<ChatSession>,
    messages: Vec<Message>,
    scroll_state: ScrollbarState,
    current_scroll: usize,
    query_service: QueryService,
    session_id: Option<String>,
    loading: bool,
    message_wrap: bool,
}

impl SessionDetailWidget {
    pub fn new(db_manager: Arc<DatabaseManager>) -> Self {
        Self {
            session: None,
            messages: Vec::new(),
            scroll_state: ScrollbarState::default(),
            current_scroll: 0,
            query_service: QueryService::with_database(db_manager),
            session_id: None,
            loading: false,
            message_wrap: true,
        }
    }

    pub async fn set_session_id(&mut self, session_id: Option<String>) -> Result<()> {
        self.session_id = session_id;
        if self.session_id.is_some() {
            self.refresh().await?;
        }
        Ok(())
    }

    pub async fn refresh(&mut self) -> Result<()> {
        if let Some(session_id) = &self.session_id {
            self.loading = true;

            let request = SessionDetailRequest {
                session_id: session_id.clone(),
                include_content: Some(true),
                message_limit: None,
                message_offset: None,
            };

            match self.query_service.get_session_detail(request).await {
                Ok(response) => {
                    self.session = Some(response.session);
                    self.messages = response.messages;
                    self.current_scroll = 0;
                    self.update_scroll_state();
                }
                Err(e) => {
                    eprintln!("Failed to load session details: {e}");
                }
            }

            self.loading = false;
        }
        Ok(())
    }

    pub async fn handle_key(&mut self, key: KeyEvent) -> Result<()> {
        match key.code {
            KeyCode::Up => {
                self.scroll_up();
            }
            KeyCode::Down => {
                self.scroll_down();
            }
            KeyCode::PageUp => {
                self.scroll_page_up();
            }
            KeyCode::PageDown => {
                self.scroll_page_down();
            }
            KeyCode::Home => {
                self.scroll_to_top();
            }
            KeyCode::End => {
                self.scroll_to_bottom();
            }
            KeyCode::Char('w') => {
                self.toggle_wrap();
            }
            KeyCode::Char('r') => {
                self.refresh().await?;
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
                Constraint::Min(0),    // Messages
            ])
            .split(area);

        // Render session header
        self.render_session_header(f, chunks[0]);

        // Render messages
        self.render_messages(f, chunks[1]);
    }

    fn render_session_header(&self, f: &mut Frame, area: Rect) {
        let content = if self.loading {
            "Loading session details...".to_string()
        } else if let Some(session) = &self.session {
            let project_str = session.project_name.as_deref().unwrap_or("No Project");
            let _duration = if let Some(_end_time) = &session.end_time {
                // Calculate duration between start and end time
                "Duration calculated"
            } else {
                "Ongoing"
            };

            format!(
                "Provider: {} | Project: {} | Messages: {} | Tokens: {} | Started: {} | Status: {} | Press 'w' to toggle word wrap",
                session.provider,
                project_str,
                session.message_count,
                session.token_count.unwrap_or(0),
                &session.start_time.format("%Y-%m-%d %H:%M").to_string(),
                session.state
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
        if self.messages.is_empty() {
            let empty_msg = if self.loading {
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
            .skip(self.current_scroll)
            .take(available_height)
            .collect();

        let messages_block = Paragraph::new(visible_lines)
            .block(Block::default().borders(Borders::ALL).title("Messages"))
            .wrap(if self.message_wrap {
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

            f.render_stateful_widget(scrollbar, scrollbar_area, &mut self.scroll_state);
        }
    }

    fn calculate_message_lines(&self, width: usize) -> Vec<Line<'_>> {
        let mut lines = Vec::new();

        for (i, message) in self.messages.iter().enumerate() {
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
            let content_lines = if self.message_wrap {
                self.wrap_text(&message.content, width.saturating_sub(2))
            } else {
                vec![message.content.clone()]
            };

            for content_line in content_lines {
                lines.push(Line::from(vec![Span::styled(
                    format!("  {content_line}"),
                    Style::default().fg(Color::White),
                )]));
            }

            // Show tool calls if any
            if let Some(tool_calls) = &message.tool_calls {
                if !tool_calls.is_empty() {
                    lines.push(Line::from(vec![Span::styled(
                        "  [Tool Calls]",
                        Style::default()
                            .fg(Color::Magenta)
                            .add_modifier(Modifier::ITALIC),
                    )]));

                    // Parse and display tool calls (simplified)
                    let tools_preview =
                        self.truncate_text(&format!("{tool_calls:?}"), width.saturating_sub(4));
                    for tool_line in self.wrap_text(&tools_preview, width.saturating_sub(4)) {
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

    fn wrap_text(&self, text: &str, width: usize) -> Vec<String> {
        if width == 0 {
            return vec![text.to_string()];
        }

        let mut lines = Vec::new();
        let mut current_line = String::new();

        for word in text.split_whitespace() {
            if current_line.len() + word.len() < width {
                if !current_line.is_empty() {
                    current_line.push(' ');
                }
                current_line.push_str(word);
            } else {
                if !current_line.is_empty() {
                    lines.push(current_line);
                    current_line = String::new();
                }
                if word.len() <= width {
                    current_line = word.to_string();
                } else {
                    // Handle very long words by breaking them
                    let mut remaining = word;
                    while remaining.len() > width {
                        if width > 0 {
                            lines.push(remaining[..width].to_string());
                            remaining = &remaining[width..];
                        } else {
                            break;
                        }
                    }
                    if !remaining.is_empty() {
                        current_line = remaining.to_string();
                    }
                }
            }
        }

        if !current_line.is_empty() {
            lines.push(current_line);
        }

        if lines.is_empty() {
            lines.push(String::new());
        }

        lines
    }

    fn truncate_text(&self, text: &str, max_len: usize) -> String {
        if text.len() <= max_len {
            text.to_string()
        } else {
            format!("{}...", &text[0..max_len.saturating_sub(3)])
        }
    }

    fn scroll_up(&mut self) {
        if self.current_scroll > 0 {
            self.current_scroll -= 1;
            self.update_scroll_state();
        }
    }

    fn scroll_down(&mut self) {
        let max_scroll = self.get_max_scroll();
        if self.current_scroll < max_scroll {
            self.current_scroll += 1;
            self.update_scroll_state();
        }
    }

    fn scroll_page_up(&mut self) {
        let page_size = 10; // Lines per page
        self.current_scroll = self.current_scroll.saturating_sub(page_size);
        self.update_scroll_state();
    }

    fn scroll_page_down(&mut self) {
        let page_size = 10; // Lines per page
        let max_scroll = self.get_max_scroll();
        self.current_scroll = (self.current_scroll + page_size).min(max_scroll);
        self.update_scroll_state();
    }

    fn scroll_to_top(&mut self) {
        self.current_scroll = 0;
        self.update_scroll_state();
    }

    fn scroll_to_bottom(&mut self) {
        self.current_scroll = self.get_max_scroll();
        self.update_scroll_state();
    }

    fn toggle_wrap(&mut self) {
        self.message_wrap = !self.message_wrap;
    }

    fn get_total_lines(&self) -> usize {
        self.calculate_message_lines(80).len() // Use standard width for calculation
    }

    fn get_max_scroll(&self) -> usize {
        let total_lines = self.get_total_lines();
        let visible_lines = 20; // Approximate visible lines
        total_lines.saturating_sub(visible_lines)
    }

    fn update_scroll_state(&mut self) {
        let total_lines = self.get_total_lines();
        self.scroll_state = self.scroll_state.content_length(total_lines);
        self.scroll_state = self.scroll_state.position(self.current_scroll);
    }
}
