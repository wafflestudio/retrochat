use anyhow::Result;
use chrono::Utc;
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph, Wrap},
    Frame,
};
use std::collections::HashMap;
use uuid::Uuid;

use crate::database::connection::DatabaseManager;
use crate::models::retrospection_analysis::{AnalysisStatus, RetrospectionAnalysis};
use crate::services::{PromptService, RetrospectionService};

#[derive(Debug, Clone, PartialEq)]
pub enum RetrospectionMode {
    AnalysisList,
    AnalysisDetail,
    TemplateList,
    TemplateDetail,
    NewAnalysis,
}

#[derive(Debug, Clone)]
pub struct RetrospectionWidget {
    pub mode: RetrospectionMode,
    pub analyses: Vec<RetrospectionAnalysis>,
    pub selected_analysis: Option<RetrospectionAnalysis>,
    pub analysis_list_state: ListState,
    pub analysis_enabled: bool,
    pub template_list_state: ListState,
    pub selected_template: Option<String>,
    pub session_id: Option<Uuid>,
    pub loading: bool,
    pub error_message: Option<String>,
    pub scroll_offset: usize,
    pub last_updated: std::time::Instant,
}

impl Default for RetrospectionWidget {
    fn default() -> Self {
        Self::new()
    }
}

impl RetrospectionWidget {
    pub fn new() -> Self {
        Self {
            mode: RetrospectionMode::AnalysisList,
            analyses: Vec::new(),
            selected_analysis: None,
            analysis_list_state: ListState::default(),
            analysis_enabled: true,
            template_list_state: ListState::default(),
            selected_template: None,
            session_id: None,
            loading: false,
            error_message: None,
            scroll_offset: 0,
            last_updated: std::time::Instant::now(),
        }
    }

    pub fn with_session(session_id: Uuid) -> Self {
        let mut widget = Self::new();
        widget.session_id = Some(session_id);
        widget
    }

    pub async fn refresh_analyses(&mut self, db_manager: &DatabaseManager) -> Result<()> {
        self.loading = true;
        self.error_message = None;

        let retrospection_service = RetrospectionService::new(db_manager.clone())?;

        match if let Some(session_id) = self.session_id {
            // Load analyses for specific session
            retrospection_service
                .get_analyses_for_session(session_id)
                .await
        } else {
            // Load recent analyses
            retrospection_service.get_recent_analyses(50).await
        } {
            Ok(analyses) => {
                self.analyses = analyses;
                self.loading = false;
                self.last_updated = std::time::Instant::now();

                // Reset selection if current selection is invalid
                if let Some(selected_index) = self.analysis_list_state.selected() {
                    if selected_index >= self.analyses.len() {
                        self.analysis_list_state.select(None);
                    }
                }
            }
            Err(e) => {
                self.error_message = Some(format!("Failed to load analyses: {e}"));
                self.loading = false;
            }
        }

        Ok(())
    }

    pub async fn refresh_templates(&mut self, _db_manager: &DatabaseManager) -> Result<()> {
        self.loading = true;
        self.error_message = None;

        let _prompt_service = PromptService::new();

        self.analysis_enabled = true;
        self.loading = false;
        self.last_updated = std::time::Instant::now();

        Ok(())
    }

    pub async fn trigger_analysis(
        &mut self,
        db_manager: &DatabaseManager,
        session_id: Uuid,
        template_id: Option<String>,
    ) -> Result<()> {
        self.loading = true;
        self.error_message = None;

        let retrospection_service = RetrospectionService::new(db_manager.clone())?;

        // Get session content
        let chat_content = self.get_session_content(db_manager, session_id).await?;

        // Use default template if not specified
        let template_id = template_id.unwrap_or_else(|| "session_summary".to_string());

        // Create analysis request
        let mut variables = HashMap::new();
        variables.insert("chat_content".to_string(), chat_content);

        let request = crate::models::analysis_request::AnalysisRequest {
            id: Uuid::new_v4(),
            session_id,
            prompt_template_id: template_id,
            template_variables: variables,
            status: crate::models::analysis_request::RequestStatus::Queued,
            error_message: None,
            created_at: Utc::now(),
            started_at: None,
            completed_at: None,
        };

        match retrospection_service
            .process_analysis_request(request)
            .await
        {
            Ok(_) => {
                self.loading = false;
                self.refresh_analyses(db_manager).await?;
            }
            Err(e) => {
                self.error_message = Some(format!("Analysis failed: {e}"));
                self.loading = false;
            }
        }

        Ok(())
    }

    async fn get_session_content(
        &self,
        db_manager: &DatabaseManager,
        session_id: Uuid,
    ) -> Result<String> {
        db_manager.with_connection(|conn| {
            let mut stmt = conn.prepare(
                "SELECT role, content FROM messages WHERE session_id = ?1 ORDER BY created_at ASC",
            )?;

            let message_rows = stmt.query_map([session_id.to_string()], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
            })?;

            let mut content = String::new();
            for message_result in message_rows {
                let (role, message_content) = message_result?;
                content.push_str(&format!("{role}: {message_content}\n\n"));
            }

            if content.is_empty() {
                return Err(rusqlite::Error::InvalidColumnType(
                    0,
                    format!("No messages found for session {session_id}"),
                    rusqlite::types::Type::Text,
                ));
            }

            Ok(content)
        })
    }

    pub fn handle_key_event(&mut self, key: KeyEvent) -> bool {
        match self.mode {
            RetrospectionMode::AnalysisList => self.handle_analysis_list_key(key),
            RetrospectionMode::AnalysisDetail => self.handle_analysis_detail_key(key),
            RetrospectionMode::TemplateList => self.handle_template_list_key(key),
            RetrospectionMode::TemplateDetail => self.handle_template_detail_key(key),
            RetrospectionMode::NewAnalysis => self.handle_new_analysis_key(key),
        }
    }

    fn handle_analysis_list_key(&mut self, key: KeyEvent) -> bool {
        match key.code {
            KeyCode::Up | KeyCode::Char('k') => {
                if let Some(selected) = self.analysis_list_state.selected() {
                    if selected > 0 {
                        self.analysis_list_state.select(Some(selected - 1));
                    }
                } else if !self.analyses.is_empty() {
                    self.analysis_list_state.select(Some(0));
                }
                false
            }
            KeyCode::Down | KeyCode::Char('j') => {
                if let Some(selected) = self.analysis_list_state.selected() {
                    if selected < self.analyses.len().saturating_sub(1) {
                        self.analysis_list_state.select(Some(selected + 1));
                    }
                } else if !self.analyses.is_empty() {
                    self.analysis_list_state.select(Some(0));
                }
                false
            }
            KeyCode::Enter => {
                if let Some(selected) = self.analysis_list_state.selected() {
                    if let Some(analysis) = self.analyses.get(selected) {
                        self.selected_analysis = Some(analysis.clone());
                        self.mode = RetrospectionMode::AnalysisDetail;
                        self.scroll_offset = 0;
                    }
                }
                false
            }
            KeyCode::Char('t') => {
                self.mode = RetrospectionMode::TemplateList;
                self.template_list_state.select(Some(0));
                false
            }
            KeyCode::Char('n') => {
                self.mode = RetrospectionMode::NewAnalysis;
                false
            }
            KeyCode::Char('r') => {
                // Trigger refresh - this will be handled by the parent
                true
            }
            _ => false,
        }
    }

    fn handle_analysis_detail_key(&mut self, key: KeyEvent) -> bool {
        match key.code {
            KeyCode::Esc | KeyCode::Char('q') => {
                self.mode = RetrospectionMode::AnalysisList;
                self.selected_analysis = None;
                self.scroll_offset = 0;
                false
            }
            KeyCode::Up | KeyCode::Char('k') => {
                if self.scroll_offset > 0 {
                    self.scroll_offset -= 1;
                }
                false
            }
            KeyCode::Down | KeyCode::Char('j') => {
                self.scroll_offset += 1;
                false
            }
            KeyCode::PageUp => {
                self.scroll_offset = self.scroll_offset.saturating_sub(10);
                false
            }
            KeyCode::PageDown => {
                self.scroll_offset += 10;
                false
            }
            _ => false,
        }
    }

    fn handle_template_list_key(&mut self, key: KeyEvent) -> bool {
        match key.code {
            KeyCode::Esc | KeyCode::Char('q') => {
                self.mode = RetrospectionMode::AnalysisList;
                false
            }
            KeyCode::Up | KeyCode::Char('k') => {
                if let Some(selected) = self.template_list_state.selected() {
                    if selected > 0 {
                        self.template_list_state.select(Some(selected - 1));
                    }
                } else if self.analysis_enabled {
                    self.template_list_state.select(Some(0));
                }
                false
            }
            KeyCode::Down | KeyCode::Char('j') => {
                if let Some(selected) = self.template_list_state.selected() {
                    if selected < 1 {
                        // Simplified for single template
                        self.template_list_state.select(Some(selected + 1));
                    }
                } else if self.analysis_enabled {
                    self.template_list_state.select(Some(0));
                }
                false
            }
            KeyCode::Enter => {
                if self.template_list_state.selected().is_some() {
                    self.selected_template = Some("default".to_string());
                    self.mode = RetrospectionMode::TemplateDetail;
                }
                false
            }
            _ => false,
        }
    }

    fn handle_template_detail_key(&mut self, key: KeyEvent) -> bool {
        match key.code {
            KeyCode::Esc | KeyCode::Char('q') => {
                self.mode = RetrospectionMode::TemplateList;
                self.selected_template = None;
                false
            }
            _ => false,
        }
    }

    fn handle_new_analysis_key(&mut self, key: KeyEvent) -> bool {
        match key.code {
            KeyCode::Esc | KeyCode::Char('q') => {
                self.mode = RetrospectionMode::AnalysisList;
                false
            }
            _ => false,
        }
    }

    pub fn render(&mut self, f: &mut Frame, area: Rect) {
        match self.mode {
            RetrospectionMode::AnalysisList => self.render_analysis_list(f, area),
            RetrospectionMode::AnalysisDetail => self.render_analysis_detail(f, area),
            RetrospectionMode::TemplateList => self.render_template_list(f, area),
            RetrospectionMode::TemplateDetail => self.render_template_detail(f, area),
            RetrospectionMode::NewAnalysis => self.render_new_analysis(f, area),
        }
    }

    fn render_analysis_list(&mut self, f: &mut Frame, area: Rect) {
        let title = if let Some(session_id) = self.session_id {
            format!("Retrospection Analyses - Session {session_id}")
        } else {
            "Recent Retrospection Analyses".to_string()
        };

        let block = Block::default()
            .title(title)
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Blue));

        if self.loading {
            let loading_text = Paragraph::new("Loading analyses...")
                .block(block)
                .style(Style::default().fg(Color::Yellow));
            f.render_widget(loading_text, area);
            return;
        }

        if let Some(error) = &self.error_message {
            let error_text = Paragraph::new(error.as_str())
                .block(block)
                .style(Style::default().fg(Color::Red))
                .wrap(Wrap { trim: true });
            f.render_widget(error_text, area);
            return;
        }

        if self.analyses.is_empty() {
            let empty_text =
                Paragraph::new("No analyses found. Press 'n' to create a new analysis.")
                    .block(block)
                    .style(Style::default().fg(Color::Gray));
            f.render_widget(empty_text, area);
            return;
        }

        let items: Vec<ListItem> = self
            .analyses
            .iter()
            .map(|analysis| {
                let status_style = match analysis.status {
                    AnalysisStatus::Complete => Style::default().fg(Color::Green),
                    AnalysisStatus::InProgress => Style::default().fg(Color::Yellow),
                    AnalysisStatus::Failed => Style::default().fg(Color::Red),
                    _ => Style::default().fg(Color::Gray),
                };

                let line = Line::from(vec![
                    Span::styled(
                        format!("{:?}", analysis.status),
                        status_style.add_modifier(Modifier::BOLD),
                    ),
                    Span::raw(" | "),
                    Span::styled(
                        analysis.prompt_template_id.clone(),
                        Style::default().fg(Color::Cyan),
                    ),
                    Span::raw(" | "),
                    Span::raw(analysis.created_at.format("%Y-%m-%d %H:%M").to_string()),
                ]);

                ListItem::new(line)
            })
            .collect();

        let list = List::new(items)
            .block(block)
            .highlight_style(Style::default().bg(Color::Blue).fg(Color::White))
            .highlight_symbol("→ ");

        f.render_stateful_widget(list, area, &mut self.analysis_list_state);

        // Render help at the bottom
        let help_area = Rect {
            x: area.x,
            y: area.y + area.height.saturating_sub(3),
            width: area.width,
            height: 3,
        };

        let help_text = Paragraph::new("↑/↓: Navigate | Enter: View Detail | t: Templates | n: New Analysis | r: Refresh | q: Back")
            .block(Block::default().borders(Borders::TOP))
            .style(Style::default().fg(Color::Gray));

        f.render_widget(Clear, help_area);
        f.render_widget(help_text, help_area);
    }

    fn render_analysis_detail(&mut self, f: &mut Frame, area: Rect) {
        if let Some(analysis) = &self.selected_analysis {
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Length(8), Constraint::Min(0)])
                .split(area);

            // Header with metadata
            let header_block = Block::default()
                .title("Analysis Details")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Blue));

            let metadata_text = vec![
                Line::from(vec![
                    Span::styled("ID: ", Style::default().add_modifier(Modifier::BOLD)),
                    Span::raw(analysis.id.to_string()),
                ]),
                Line::from(vec![
                    Span::styled("Session: ", Style::default().add_modifier(Modifier::BOLD)),
                    Span::raw(analysis.session_id.to_string()),
                ]),
                Line::from(vec![
                    Span::styled("Template: ", Style::default().add_modifier(Modifier::BOLD)),
                    Span::styled(
                        analysis.prompt_template_id.clone(),
                        Style::default().fg(Color::Cyan),
                    ),
                ]),
                Line::from(vec![
                    Span::styled("Status: ", Style::default().add_modifier(Modifier::BOLD)),
                    Span::styled(
                        format!("{:?}", analysis.status),
                        match analysis.status {
                            AnalysisStatus::Complete => Style::default().fg(Color::Green),
                            AnalysisStatus::InProgress => Style::default().fg(Color::Yellow),
                            AnalysisStatus::Failed => Style::default().fg(Color::Red),
                            _ => Style::default().fg(Color::Gray),
                        },
                    ),
                ]),
                Line::from(vec![
                    Span::styled("Created: ", Style::default().add_modifier(Modifier::BOLD)),
                    Span::raw(analysis.created_at.format("%Y-%m-%d %H:%M:%S").to_string()),
                ]),
                Line::from(vec![
                    Span::styled("Tokens: ", Style::default().add_modifier(Modifier::BOLD)),
                    Span::raw(format!("{} total", analysis.metadata.total_tokens)),
                ]),
            ];

            let header_paragraph = Paragraph::new(metadata_text)
                .block(header_block)
                .wrap(Wrap { trim: true });

            f.render_widget(header_paragraph, chunks[0]);

            // Content area
            let content_block = Block::default()
                .title("Analysis Content")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Green));

            let content_lines: Vec<Line> = analysis
                .analysis_content
                .lines()
                .skip(self.scroll_offset)
                .take((chunks[1].height.saturating_sub(2)) as usize)
                .map(|line| Line::from(line.to_string()))
                .collect();

            let content_paragraph = Paragraph::new(content_lines)
                .block(content_block)
                .wrap(Wrap { trim: true });

            f.render_widget(content_paragraph, chunks[1]);

            // Help text
            let help_area = Rect {
                x: area.x,
                y: area.y + area.height.saturating_sub(1),
                width: area.width,
                height: 1,
            };

            let help_text =
                Paragraph::new("↑/↓: Scroll | PageUp/PageDown: Fast Scroll | q/Esc: Back")
                    .style(Style::default().fg(Color::Gray));

            f.render_widget(Clear, help_area);
            f.render_widget(help_text, help_area);
        }
    }

    fn render_template_list(&mut self, f: &mut Frame, area: Rect) {
        let block = Block::default()
            .title("Prompt Templates")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Magenta));

        if self.loading {
            let loading_text = Paragraph::new("Loading templates...")
                .block(block)
                .style(Style::default().fg(Color::Yellow));
            f.render_widget(loading_text, area);
            return;
        }

        if let Some(error) = &self.error_message {
            let error_text = Paragraph::new(error.as_str())
                .block(block)
                .style(Style::default().fg(Color::Red))
                .wrap(Wrap { trim: true });
            f.render_widget(error_text, area);
            return;
        }

        if !self.analysis_enabled {
            let empty_text = Paragraph::new("No templates found.")
                .block(block)
                .style(Style::default().fg(Color::Gray));
            f.render_widget(empty_text, area);
            return;
        }

        let items: Vec<ListItem> = vec![ListItem::new(Line::from(vec![
            Span::styled(
                "default".to_string(),
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(" | "),
            Span::raw("Default Analysis Template".to_string()),
            Span::raw(" | "),
            Span::styled("analysis".to_string(), Style::default().fg(Color::Yellow)),
        ]))];

        let list = List::new(items)
            .block(block)
            .highlight_style(Style::default().bg(Color::Magenta).fg(Color::White))
            .highlight_symbol("→ ");

        f.render_stateful_widget(list, area, &mut self.template_list_state);

        // Help text
        let help_area = Rect {
            x: area.x,
            y: area.y + area.height.saturating_sub(2),
            width: area.width,
            height: 2,
        };

        let help_text = Paragraph::new("↑/↓: Navigate | Enter: View Template | q/Esc: Back")
            .block(Block::default().borders(Borders::TOP))
            .style(Style::default().fg(Color::Gray));

        f.render_widget(Clear, help_area);
        f.render_widget(help_text, help_area);
    }

    fn render_template_detail(&mut self, f: &mut Frame, area: Rect) {
        if let Some(_template_id) = &self.selected_template {
            // Static template information since we only have one hardcoded template
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Length(6), Constraint::Min(0)])
                .split(area);

            // Header
            let header_block = Block::default()
                .title("Template Details")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Magenta));

            let header_text = vec![
                Line::from(vec![
                    Span::styled("ID: ", Style::default().add_modifier(Modifier::BOLD)),
                    Span::raw("default".to_string()),
                ]),
                Line::from(vec![
                    Span::styled("Name: ", Style::default().add_modifier(Modifier::BOLD)),
                    Span::raw("Default Analysis Template".to_string()),
                ]),
                Line::from(vec![
                    Span::styled("Category: ", Style::default().add_modifier(Modifier::BOLD)),
                    Span::styled("analysis".to_string(), Style::default().fg(Color::Yellow)),
                ]),
                Line::from(vec![
                    Span::styled(
                        "Description: ",
                        Style::default().add_modifier(Modifier::BOLD),
                    ),
                    Span::raw("Hardcoded prompt template for chat analysis".to_string()),
                ]),
            ];

            let header_paragraph = Paragraph::new(header_text)
                .block(header_block)
                .wrap(Wrap { trim: true });

            f.render_widget(header_paragraph, chunks[0]);

            // Content
            let content_block = Block::default()
                .title("Template Content")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Green));

            let content_paragraph = Paragraph::new("Hardcoded prompt for chat analysis")
                .block(content_block)
                .wrap(Wrap { trim: true });

            f.render_widget(content_paragraph, chunks[1]);
        }

        // Help text
        let help_area = Rect {
            x: area.x,
            y: area.y + area.height.saturating_sub(1),
            width: area.width,
            height: 1,
        };

        let help_text = Paragraph::new("q/Esc: Back").style(Style::default().fg(Color::Gray));

        f.render_widget(Clear, help_area);
        f.render_widget(help_text, help_area);
    }

    fn render_new_analysis(&mut self, f: &mut Frame, area: Rect) {
        let block = Block::default()
            .title("New Analysis (Coming Soon)")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Yellow));

        let content = Paragraph::new(
            "New analysis creation interface will be implemented in future versions.",
        )
        .block(block)
        .style(Style::default().fg(Color::Gray))
        .wrap(Wrap { trim: true });

        f.render_widget(content, area);

        // Help text
        let help_area = Rect {
            x: area.x,
            y: area.y + area.height.saturating_sub(1),
            width: area.width,
            height: 1,
        };

        let help_text = Paragraph::new("q/Esc: Back").style(Style::default().fg(Color::Gray));

        f.render_widget(Clear, help_area);
        f.render_widget(help_text, help_area);
    }
}
