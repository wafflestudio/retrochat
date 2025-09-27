use anyhow::Result;
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph, Wrap},
    Frame,
};
use tokio::sync::mpsc;
use std::sync::Arc;

use crate::models::{RetrospectRequest, Retrospection, RetrospectionAnalysisType, OperationStatus};
use crate::services::retrospection_service::RetrospectionService;

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
    all_requests: Vec<RetrospectRequest>,
    selected_index: usize,
    show_details: bool,
    selected_session_id: Option<String>,
    selected_retrospection: Option<Retrospection>,

    // Service dependency
    retrospection_service: Arc<RetrospectionService>,

    // Progress tracking
    progress_receiver: Option<mpsc::UnboundedReceiver<RetrospectionProgress>>,
    progress_sender: Option<mpsc::UnboundedSender<RetrospectionProgress>>,
}

impl RetrospectionWidget {
    pub fn new(retrospection_service: Arc<RetrospectionService>) -> Self {
        let (progress_sender, progress_receiver) = mpsc::unbounded_channel();

        Self {
            all_requests: Vec::new(),
            selected_index: 0,
            show_details: false,
            selected_session_id: None,
            selected_retrospection: None,
            retrospection_service,
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

        // Load all requests from database
        self.all_requests = self.retrospection_service.list_analyses(None, Some(100)).await
            .map_err(|e| anyhow::anyhow!("Failed to load requests: {}", e))?;

        // Sort requests: active first (pending, running), then completed/failed/cancelled
        self.all_requests.sort_by(|a, b| {
            use OperationStatus::*;
            let a_priority = match a.status {
                Pending => 0,
                Running => 1,
                Failed => 2,
                Cancelled => 3,
                Completed => 4,
            };
            let b_priority = match b.status {
                Pending => 0,
                Running => 1,
                Failed => 2,
                Cancelled => 3,
                Completed => 4,
            };
            a_priority.cmp(&b_priority)
        });

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
                self.toggle_details().await?;
            }
            KeyCode::Char('c') => {
                self.cancel_selected_request().await?;
            }
            KeyCode::Char('d') => {
                self.show_details = !self.show_details;
            }
            KeyCode::Char('v') => {
                self.view_retrospection_result().await?;
            }
            KeyCode::Char('r') => {
                self.rerun_selected_request().await?;
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

        if self.show_details {
            if let Some(retrospection) = &self.selected_retrospection {
                self.render_retrospection_details(f, chunks[1], retrospection);
            } else {
                self.render_request_details(f, chunks[1]);
            }
        } else {
            self.render_request_list(f, chunks[1]);
        }

        self.render_footer(f, chunks[2]);
    }

    pub fn start_analysis(&mut self, session_id: String, _analysis_type: RetrospectionAnalysisType) {
        // This method is kept for backward compatibility
        // Analysis requests are now created through the RetrospectionService
        self.selected_session_id = Some(session_id);
    }

    pub fn get_progress_sender(&self) -> Option<mpsc::UnboundedSender<RetrospectionProgress>> {
        self.progress_sender.clone()
    }

    fn update_progress(&mut self, _progress: RetrospectionProgress) {
        // Progress updates are now handled through database refresh
        // This method is kept for backward compatibility with progress_receiver
    }

    fn render_header(&self, f: &mut Frame, area: Rect) {
        let active_count = self.all_requests.iter()
            .filter(|r| matches!(r.status, OperationStatus::Pending | OperationStatus::Running))
            .count();
        let completed_count = self.all_requests.len() - active_count;

        let header_text = format!(
            "Retrospection Status - Active: {} | Completed: {} | Total: {} | Press 'd' for details",
            active_count, completed_count, self.all_requests.len()
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
        if self.all_requests.is_empty() {
            let empty_msg = Paragraph::new("No retrospection requests\n\n(Auto-refreshes every 2 seconds)")
                .block(Block::default().borders(Borders::ALL).title("Retrospection Requests"))
                .style(Style::default().fg(Color::Gray))
                .wrap(Wrap { trim: true });

            f.render_widget(empty_msg, area);
            return;
        }

        let items: Vec<ListItem> = self.all_requests
            .iter()
            .map(|request| {
                let status_color = match request.status {
                    OperationStatus::Pending => Color::Yellow,
                    OperationStatus::Running => Color::Blue,
                    OperationStatus::Completed => Color::Green,
                    OperationStatus::Failed => Color::Red,
                    OperationStatus::Cancelled => Color::Gray,
                };

                let content = format!(
                    "{} | {} | {}",
                    request.session_id.chars().take(8).collect::<String>(),
                    request.analysis_type,
                    request.status
                );

                ListItem::new(Line::from(vec![
                    Span::styled(content, Style::default().fg(status_color)),
                ]))
            })
            .collect();

        let list = List::new(items)
            .block(Block::default().borders(Borders::ALL).title("Retrospection Requests"))
            .highlight_style(
                Style::default()
                    .bg(Color::DarkGray)
                    .add_modifier(Modifier::BOLD),
            );

        let mut list_state = ListState::default();
        if !self.all_requests.is_empty() && self.selected_index < self.all_requests.len() {
            list_state.select(Some(self.selected_index));
        }
        f.render_stateful_widget(list, area, &mut list_state);
    }


    fn render_request_details(&self, f: &mut Frame, area: Rect) {
        if let Some(request) = self.all_requests.get(self.selected_index) {
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
                Line::from(vec![Span::raw(format!("Request ID: {}", request.id))]),
                Line::from(vec![Span::raw(format!("Session ID: {}", request.session_id))]),
                Line::from(vec![Span::raw(format!("Analysis Type: {}", request.analysis_type))]),
                Line::from(vec![Span::raw(format!("Status: {}", request.status))]),
                Line::from(vec![Span::raw(format!("Started: {}", request.started_at.format("%Y-%m-%d %H:%M:%S")))]),
                Line::from(vec![Span::raw(format!("Completed: {}",
                    request.completed_at.map(|dt| dt.format("%Y-%m-%d %H:%M:%S").to_string())
                        .unwrap_or_else(|| "N/A".to_string())))]),
            ];

            let info_block = Paragraph::new(info_lines)
                .block(Block::default().borders(Borders::ALL).title("Request Details"))
                .style(Style::default().fg(Color::White));

            f.render_widget(info_block, chunks[0]);

            // Status information
            let status_info = match request.status {
                OperationStatus::Completed => "Request completed successfully. Press 'v' to view results.",
                OperationStatus::Failed => "Request failed. Check error message below.",
                OperationStatus::Cancelled => "Request was cancelled.",
                OperationStatus::Pending => "Request is pending execution.",
                OperationStatus::Running => "Request is currently running.",
            };

            let status_block = Paragraph::new(status_info)
                .block(Block::default().borders(Borders::ALL).title("Status"))
                .style(Style::default().fg(Color::White))
                .wrap(Wrap { trim: true });

            f.render_widget(status_block, chunks[1]);

            // Error details if any
            if let Some(error_message) = &request.error_message {
                let error_block = Paragraph::new(error_message.as_str())
                    .block(Block::default().borders(Borders::ALL).title("Error Details"))
                    .style(Style::default().fg(Color::Red))
                    .wrap(Wrap { trim: true });

                f.render_widget(error_block, chunks[2]);
            }
        }
    }

    fn render_retrospection_details(&self, f: &mut Frame, area: Rect, retrospection: &Retrospection) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(5),   // Header info
                Constraint::Percentage(33), // Insights
                Constraint::Percentage(33), // Reflection
                Constraint::Percentage(34), // Recommendations
            ])
            .split(area);

        // Header
        let header = Paragraph::new(format!(
            "Retrospection ID: {}\nCreated: {}",
            retrospection.id,
            retrospection.created_at.format("%Y-%m-%d %H:%M:%S")
        ))
        .block(Block::default().borders(Borders::ALL).title("Retrospection Details"))
        .style(Style::default().fg(Color::Cyan));

        f.render_widget(header, chunks[0]);

        // Insights
        let insights = Paragraph::new(retrospection.insights.as_str())
            .block(Block::default().borders(Borders::ALL).title("Insights"))
            .style(Style::default().fg(Color::Green))
            .wrap(Wrap { trim: true });

        f.render_widget(insights, chunks[1]);

        // Reflection
        let reflection = Paragraph::new(retrospection.reflection.as_str())
            .block(Block::default().borders(Borders::ALL).title("Reflection"))
            .style(Style::default().fg(Color::Yellow))
            .wrap(Wrap { trim: true });

        f.render_widget(reflection, chunks[2]);

        // Recommendations
        let recommendations = Paragraph::new(retrospection.recommendations.as_str())
            .block(Block::default().borders(Borders::ALL).title("Recommendations"))
            .style(Style::default().fg(Color::Blue))
            .wrap(Wrap { trim: true });

        f.render_widget(recommendations, chunks[3]);
    }

    fn render_footer(&self, f: &mut Frame, area: Rect) {
        let controls = "↑/↓: Navigate | Enter/d: Toggle Details | v: View Results | c: Cancel | r: Rerun | Esc: Back | Auto-refreshes every 2s";

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
        if self.selected_index < self.all_requests.len().saturating_sub(1) {
            self.selected_index += 1;
        }
    }

    async fn toggle_details(&mut self) -> Result<()> {
        self.show_details = !self.show_details;

        // Clear retrospection when toggling off details
        if !self.show_details {
            self.selected_retrospection = None;
        }

        Ok(())
    }

    async fn view_retrospection_result(&mut self) -> Result<()> {
        if let Some(request) = self.all_requests.get(self.selected_index) {
            if matches!(request.status, OperationStatus::Completed) {
                // Load the retrospection result
                match self.retrospection_service.get_analysis_result(request.id.clone()).await
                    .map_err(|e| anyhow::anyhow!("Failed to get analysis result: {}", e))? {
                    Some(retrospection) => {
                        self.selected_retrospection = Some(retrospection);
                        self.show_details = true;
                    }
                    None => {
                        // No retrospection result found
                    }
                }
            }
        }

        Ok(())
    }

    async fn cancel_selected_request(&mut self) -> Result<()> {
        if let Some(request) = self.all_requests.get(self.selected_index) {
            if matches!(request.status, OperationStatus::Pending | OperationStatus::Running) {
                // Use the service to cancel the request
                if let Err(e) = self.retrospection_service.cancel_analysis(request.id.clone()).await {
                    // Handle error if needed
                    eprintln!("Failed to cancel request: {}", e);
                } else {
                    // Refresh to update the UI
                    self.refresh().await?;
                }
            }
        }
        Ok(())
    }

    async fn rerun_selected_request(&mut self) -> Result<()> {
        if let Some(request) = self.all_requests.get(self.selected_index) {
            if matches!(request.status, OperationStatus::Failed | OperationStatus::Cancelled | OperationStatus::Pending) {
                // Execute the existing request (this will restart it)
                match self.retrospection_service.execute_analysis(request.id.clone()).await {
                    Ok(_) => {
                        // Refresh to update the UI
                        self.refresh().await?;
                    }
                    Err(e) => {
                        eprintln!("Failed to rerun analysis: {}", e);
                    }
                }
            }
        }
        Ok(())
    }
}