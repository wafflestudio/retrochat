use anyhow::Result;
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Gauge, List, ListItem, ListState, Paragraph, Wrap},
    Frame,
};
use tokio::sync::mpsc;

use crate::models::{RetrospectRequest, RetrospectionAnalysisType, OperationStatus};

#[derive(Debug, Clone)]
pub struct RetrospectionProgress {
    pub request_id: String,
    pub session_id: String,
    pub analysis_type: RetrospectionAnalysisType,
    pub status: OperationStatus,
    pub progress_percent: u16,
    pub message: String,
    pub error: Option<String>,
}

pub struct RetrospectionWidget {
    active_requests: Vec<RetrospectionProgress>,
    completed_requests: Vec<RetrospectRequest>,
    selected_index: usize,
    list_state: ListState,
    show_details: bool,
    selected_session_id: Option<String>,

    // Progress tracking
    progress_receiver: Option<mpsc::UnboundedReceiver<RetrospectionProgress>>,
    progress_sender: Option<mpsc::UnboundedSender<RetrospectionProgress>>,
}

impl RetrospectionWidget {
    pub fn new() -> Self {
        let (progress_sender, progress_receiver) = mpsc::unbounded_channel();

        Self {
            active_requests: Vec::new(),
            completed_requests: Vec::new(),
            selected_index: 0,
            list_state: ListState::default(),
            show_details: false,
            selected_session_id: None,
            progress_receiver: Some(progress_receiver),
            progress_sender: Some(progress_sender),
        }
    }

    pub async fn refresh(&mut self) -> Result<()> {
        // Check for progress updates
        let mut new_progress = Vec::new();
        if let Some(receiver) = &mut self.progress_receiver {
            while let Ok(progress) = receiver.try_recv() {
                new_progress.push(progress);
            }
        }

        // Update progress outside of the borrow
        for progress in new_progress {
            self.update_progress(progress);
        }

        // Load active and completed requests from database
        // This would need to be implemented in the repository
        // For now, we'll maintain the in-memory state

        Ok(())
    }

    pub async fn handle_key(&mut self, key: KeyEvent) -> Result<()> {
        match key.code {
            KeyCode::Up => {
                self.move_selection_up();
            }
            KeyCode::Down => {
                self.move_selection_down();
            }
            KeyCode::Enter => {
                self.toggle_details();
            }
            KeyCode::Char('r') => {
                self.refresh().await?;
            }
            KeyCode::Char('c') => {
                self.cancel_selected_request().await?;
            }
            KeyCode::Char('d') => {
                self.show_details = !self.show_details;
            }
            _ => {}
        }
        Ok(())
    }

    pub fn render(&mut self, f: &mut Frame, area: Rect) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3), // Header
                Constraint::Min(0),    // Main content
                Constraint::Length(3), // Footer with controls
            ])
            .split(area);

        self.render_header(f, chunks[0]);

        if self.show_details && self.selected_index < self.active_requests.len() {
            self.render_request_details(f, chunks[1]);
        } else {
            self.render_request_list(f, chunks[1]);
        }

        self.render_footer(f, chunks[2]);
    }

    pub fn start_analysis(&mut self, session_id: String, analysis_type: RetrospectionAnalysisType) {
        let request_id = uuid::Uuid::new_v4().to_string();
        let progress = RetrospectionProgress {
            request_id: request_id.clone(),
            session_id: session_id.clone(),
            analysis_type,
            status: OperationStatus::Pending,
            progress_percent: 0,
            message: "Starting analysis...".to_string(),
            error: None,
        };

        self.active_requests.push(progress);
        self.selected_session_id = Some(session_id);
    }

    pub fn get_progress_sender(&self) -> Option<mpsc::UnboundedSender<RetrospectionProgress>> {
        self.progress_sender.clone()
    }

    fn update_progress(&mut self, progress: RetrospectionProgress) {
        // Update existing request or add new one
        if let Some(existing) = self.active_requests.iter_mut()
            .find(|r| r.request_id == progress.request_id) {
            *existing = progress;
        } else {
            self.active_requests.push(progress);
        }

        // Move completed requests to completed list
        self.active_requests.retain(|req| {
            if matches!(req.status, OperationStatus::Completed | OperationStatus::Failed | OperationStatus::Cancelled) {
                // Would normally move to completed_requests, but for now just remove
                false
            } else {
                true
            }
        });
    }

    fn render_header(&self, f: &mut Frame, area: Rect) {
        let active_count = self.active_requests.len();
        let completed_count = self.completed_requests.len();

        let header_text = format!(
            "Retrospection Status - Active: {} | Completed: {} | Press 'd' for details",
            active_count, completed_count
        );

        let header = Paragraph::new(header_text)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("Retrospection Manager"),
            )
            .style(Style::default().fg(Color::Cyan));

        f.render_widget(header, area);
    }

    fn render_request_list(&mut self, f: &mut Frame, area: Rect) {
        if self.active_requests.is_empty() {
            let empty_msg = Paragraph::new("No active retrospection requests\n\nPress 'r' to start analysis from session detail")
                .block(Block::default().borders(Borders::ALL))
                .style(Style::default().fg(Color::Gray))
                .wrap(Wrap { trim: true });

            f.render_widget(empty_msg, area);
            return;
        }

        let items: Vec<ListItem> = self.active_requests
            .iter()
            .map(|request| {
                let status_color = match request.status {
                    OperationStatus::Pending => Color::Yellow,
                    OperationStatus::Running => Color::Blue,
                    OperationStatus::Completed => Color::Green,
                    OperationStatus::Failed => Color::Red,
                    OperationStatus::Cancelled => Color::Gray,
                };

                let progress_bar = if request.progress_percent > 0 {
                    format!(" [{}%]", request.progress_percent)
                } else {
                    String::new()
                };

                let content = format!(
                    "{} | {} | {}{}",
                    request.session_id.chars().take(12).collect::<String>(),
                    request.analysis_type,
                    request.status,
                    progress_bar
                );

                ListItem::new(Line::from(vec![
                    Span::styled(content, Style::default().fg(status_color)),
                ]))
            })
            .collect();

        let list = List::new(items)
            .block(Block::default().borders(Borders::ALL).title("Active Requests"))
            .highlight_style(
                Style::default()
                    .bg(Color::DarkGray)
                    .add_modifier(Modifier::BOLD),
            );

        // Update list state
        if self.selected_index >= self.active_requests.len() && !self.active_requests.is_empty() {
            self.selected_index = self.active_requests.len() - 1;
        }
        self.list_state.select(Some(self.selected_index));

        f.render_stateful_widget(list, area, &mut self.list_state);
    }

    fn render_request_details(&self, f: &mut Frame, area: Rect) {
        if let Some(request) = self.active_requests.get(self.selected_index) {
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(8), // Request info
                    Constraint::Length(3), // Progress bar
                    Constraint::Min(0),    // Status/error details
                ])
                .split(area);

            // Request information
            let info_lines = vec![
                Line::from(vec![Span::raw(format!("Request ID: {}", request.request_id))]),
                Line::from(vec![Span::raw(format!("Session ID: {}", request.session_id))]),
                Line::from(vec![Span::raw(format!("Analysis Type: {}", request.analysis_type))]),
                Line::from(vec![Span::raw(format!("Status: {}", request.status))]),
                Line::from(vec![Span::raw(format!("Progress: {}%", request.progress_percent))]),
                Line::from(vec![Span::raw(format!("Message: {}", request.message))]),
            ];

            let info_block = Paragraph::new(info_lines)
                .block(Block::default().borders(Borders::ALL).title("Request Details"))
                .style(Style::default().fg(Color::White));

            f.render_widget(info_block, chunks[0]);

            // Progress bar
            let progress = Gauge::default()
                .block(Block::default().borders(Borders::ALL).title("Progress"))
                .gauge_style(Style::default().fg(Color::Blue))
                .percent(request.progress_percent)
                .label(format!("{}%", request.progress_percent));

            f.render_widget(progress, chunks[1]);

            // Error details if any
            if let Some(error) = &request.error {
                let error_block = Paragraph::new(error.as_str())
                    .block(Block::default().borders(Borders::ALL).title("Error Details"))
                    .style(Style::default().fg(Color::Red))
                    .wrap(Wrap { trim: true });

                f.render_widget(error_block, chunks[2]);
            }
        }
    }

    fn render_footer(&self, f: &mut Frame, area: Rect) {
        let controls = "↑/↓: Navigate | Enter/d: Toggle Details | r: Refresh | c: Cancel Selected | Esc: Back";

        let footer = Paragraph::new(controls)
            .block(Block::default().borders(Borders::ALL))
            .style(Style::default().fg(Color::Gray));

        f.render_widget(footer, area);
    }

    fn move_selection_up(&mut self) {
        if self.selected_index > 0 {
            self.selected_index -= 1;
        }
    }

    fn move_selection_down(&mut self) {
        if self.selected_index < self.active_requests.len().saturating_sub(1) {
            self.selected_index += 1;
        }
    }

    fn toggle_details(&mut self) {
        self.show_details = !self.show_details;
    }

    async fn cancel_selected_request(&mut self) -> Result<()> {
        if let Some(request) = self.active_requests.get_mut(self.selected_index) {
            if matches!(request.status, OperationStatus::Pending | OperationStatus::Running) {
                request.status = OperationStatus::Cancelled;
                request.message = "Cancelled by user".to_string();

                // TODO: Send cancellation signal to actual background task
            }
        }
        Ok(())
    }
}