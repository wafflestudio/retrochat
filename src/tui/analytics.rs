use anyhow::Result;
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    symbols,
    text::{Line, Span},
    widgets::{
        Axis, Block, Borders, Chart, Dataset, GraphType, List, ListItem, ListState, Paragraph,
    },
    Frame,
};
use std::sync::Arc;

use crate::database::DatabaseManager;
use crate::services::AnalyticsService;

#[derive(Debug, Clone)]
pub enum AnalyticsView {
    Overview,
    UsageStats,
    Insights,
    Trends,
}

// Local structs for TUI display
#[derive(Debug, Clone)]
struct TuiDailyUsage {
    #[allow(dead_code)]
    date: String,
    #[allow(dead_code)]
    sessions: i32,
    messages: i32,
    #[allow(dead_code)]
    tokens: i32,
}

#[derive(Debug, Clone)]
struct TuiProviderUsage {
    provider: String,
    sessions: i32,
    messages: i32,
    #[allow(dead_code)]
    tokens: i32,
    percentage: f64,
}

#[derive(Debug, Clone)]
struct TuiProjectUsage {
    project: String,
    sessions: i32,
    messages: i32,
    #[allow(dead_code)]
    tokens: i32,
    percentage: f64,
}

#[derive(Debug, Clone)]
struct TuiUsageData {
    total_sessions: i32,
    total_messages: i32,
    total_tokens: i32,
    average_session_length: f64,
    daily_breakdown: Vec<TuiDailyUsage>,
    provider_breakdown: Vec<TuiProviderUsage>,
    project_breakdown: Vec<TuiProjectUsage>,
}

#[derive(Debug, Clone)]
struct TuiInsight {
    title: String,
    description: String,
    confidence_score: f64,
    #[allow(dead_code)]
    insight_type: String,
}

#[derive(Debug, Clone)]
struct TuiTrend {
    metric: String,
    direction: String,
    change_percentage: f64,
    period: String,
    significance: String,
}

#[derive(Debug, Clone)]
struct TuiRecommendation {
    title: String,
    description: String,
    priority: String,
    #[allow(dead_code)]
    category: String,
    #[allow(dead_code)]
    actionable_steps: String,
}

#[derive(Debug, Clone)]
struct TuiInsightsData {
    insights: Vec<TuiInsight>,
    trends: Vec<TuiTrend>,
    recommendations: Vec<TuiRecommendation>,
    #[allow(dead_code)]
    analysis_timestamp: String,
}

pub struct AnalyticsWidget {
    current_view: AnalyticsView,
    usage_data: Option<TuiUsageData>,
    insights_data: Option<TuiInsightsData>,
    analytics_service: AnalyticsService,
    list_state: ListState,
    loading: bool,
    last_refresh: std::time::Instant,
}

impl AnalyticsWidget {
    pub fn new(db_manager: Arc<DatabaseManager>) -> Self {
        let mut list_state = ListState::default();
        list_state.select(Some(0));

        Self {
            current_view: AnalyticsView::Overview,
            usage_data: None,
            insights_data: None,
            analytics_service: AnalyticsService::new((*db_manager).clone()),
            list_state,
            loading: false,
            last_refresh: std::time::Instant::now(),
        }
    }

    pub async fn refresh(&mut self) -> Result<()> {
        self.loading = true;

        // Load usage insights using the new API
        match self.analytics_service.generate_usage_insights().await {
            Ok(insights) => {
                // Convert to the expected format for the TUI
                self.usage_data = Some(TuiUsageData {
                    total_sessions: insights.total_sessions as i32,
                    total_messages: insights.total_messages as i32,
                    total_tokens: insights.total_tokens as i32,
                    average_session_length: if insights.total_sessions > 0 {
                        insights.total_messages as f64 / insights.total_sessions as f64
                    } else {
                        0.0
                    },
                    daily_breakdown: insights
                        .daily_activity
                        .into_iter()
                        .map(|da| TuiDailyUsage {
                            date: da.date,
                            sessions: da.sessions as i32,
                            messages: da.messages as i32,
                            tokens: da.tokens as i32,
                        })
                        .collect(),
                    provider_breakdown: insights
                        .provider_breakdown
                        .into_iter()
                        .map(|(provider, stats)| TuiProviderUsage {
                            provider,
                            sessions: stats.sessions as i32,
                            messages: stats.messages as i32,
                            tokens: stats.tokens as i32,
                            percentage: stats.percentage_of_total,
                        })
                        .collect(),
                    project_breakdown: insights
                        .top_projects
                        .into_iter()
                        .map(|project| TuiProjectUsage {
                            project: project.project_name,
                            sessions: project.sessions as i32,
                            messages: project.messages as i32,
                            tokens: project.tokens as i32,
                            percentage: 0.0, // Would need to calculate this
                        })
                        .collect(),
                });
            }
            Err(e) => {
                tracing::error!(error = %e, "Failed to load usage analytics");
            }
        }

        // Set sample insights data for TUI display
        self.insights_data = Some(TuiInsightsData {
            insights: vec![
                TuiInsight {
                    title: "Peak Usage Hours".to_string(),
                    description: "Most activity occurs between 2-4 PM".to_string(),
                    confidence_score: 0.85,
                    insight_type: "usage_patterns".to_string(),
                },
                TuiInsight {
                    title: "Average Session Duration".to_string(),
                    description: "Sessions typically last 15-20 minutes".to_string(),
                    confidence_score: 0.9,
                    insight_type: "productivity".to_string(),
                },
            ],
            trends: vec![TuiTrend {
                metric: "daily_sessions".to_string(),
                direction: "increasing".to_string(),
                change_percentage: 15.0,
                period: "last_7_days".to_string(),
                significance: "moderate".to_string(),
            }],
            recommendations: vec![TuiRecommendation {
                title: "Optimize Peak Hours".to_string(),
                description: "Consider scheduling important tasks during peak usage hours"
                    .to_string(),
                priority: "medium".to_string(),
                category: "productivity".to_string(),
                actionable_steps:
                    "Schedule important tasks between 2-4 PM when activity is highest".to_string(),
            }],
            analysis_timestamp: "2024-01-01T12:00:00Z".to_string(),
        });

        self.loading = false;
        self.last_refresh = std::time::Instant::now();
        Ok(())
    }

    pub async fn handle_key(&mut self, key: KeyEvent) -> Result<()> {
        match key.code {
            KeyCode::Up => {
                self.previous_item();
            }
            KeyCode::Down => {
                self.next_item();
            }
            KeyCode::Left => {
                self.previous_view();
            }
            KeyCode::Right => {
                self.next_view();
            }
            KeyCode::Char('1') => {
                self.current_view = AnalyticsView::Overview;
                self.list_state.select(Some(0));
            }
            KeyCode::Char('2') => {
                self.current_view = AnalyticsView::UsageStats;
                self.list_state.select(Some(0));
            }
            KeyCode::Char('3') => {
                self.current_view = AnalyticsView::Insights;
                self.list_state.select(Some(0));
            }
            KeyCode::Char('4') => {
                self.current_view = AnalyticsView::Trends;
                self.list_state.select(Some(0));
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
                Constraint::Length(3), // Header
                Constraint::Min(0),    // Content
            ])
            .split(area);

        // Render header
        self.render_header(f, chunks[0]);

        // Render content based on current view
        match self.current_view {
            AnalyticsView::Overview => self.render_overview(f, chunks[1]),
            AnalyticsView::UsageStats => self.render_usage_stats(f, chunks[1]),
            AnalyticsView::Insights => self.render_insights(f, chunks[1]),
            AnalyticsView::Trends => self.render_trends(f, chunks[1]),
        }
    }

    fn render_header(&self, f: &mut Frame, area: Rect) {
        let view_name = match self.current_view {
            AnalyticsView::Overview => "Overview",
            AnalyticsView::UsageStats => "Usage Statistics",
            AnalyticsView::Insights => "Insights",
            AnalyticsView::Trends => "Trends",
        };

        let header_text = if self.loading {
            format!("Analytics - {view_name} (Loading...)")
        } else {
            format!(
                "Analytics - {} | Last updated: {} | Use 1-4 to switch views, ← → to navigate, 'r' to refresh",
                view_name,
                self.format_elapsed_time()
            )
        };

        let header = Paragraph::new(header_text)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("Analytics Dashboard"),
            )
            .style(Style::default().fg(Color::Cyan));

        f.render_widget(header, area);
    }

    fn render_overview(&mut self, f: &mut Frame, area: Rect) {
        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
            .split(area);

        // Left side - Key metrics
        self.render_key_metrics(f, chunks[0]);

        // Right side - Quick insights
        self.render_quick_insights(f, chunks[1]);
    }

    fn render_key_metrics(&self, f: &mut Frame, area: Rect) {
        let metrics = if let Some(usage_data) = &self.usage_data {
            vec![
                format!("Total Sessions: {}", usage_data.total_sessions),
                format!("Total Messages: {}", usage_data.total_messages),
                format!("Total Tokens: {}", usage_data.total_tokens),
                format!(
                    "Avg Session Length: {:.1} messages",
                    usage_data.average_session_length
                ),
                String::new(),
                "Provider Breakdown:".to_string(),
            ]
            .into_iter()
            .chain(usage_data.provider_breakdown.iter().map(|p| {
                format!(
                    "  {}: {} sessions ({:.1}%)",
                    p.provider, p.sessions, p.percentage
                )
            }))
            .collect()
        } else {
            vec!["No data available".to_string()]
        };

        let items: Vec<ListItem> = metrics
            .into_iter()
            .map(|metric| ListItem::new(Line::from(metric)))
            .collect();

        let list = List::new(items)
            .block(Block::default().borders(Borders::ALL).title("Key Metrics"))
            .style(Style::default().fg(Color::White));

        f.render_widget(list, area);
    }

    fn render_quick_insights(&mut self, f: &mut Frame, area: Rect) {
        let insights = if let Some(insights_data) = &self.insights_data {
            insights_data
                .insights
                .iter()
                .take(5) // Show top 5 insights
                .map(|insight| format!("• {}: {}", insight.title, insight.description))
                .collect()
        } else {
            vec!["Loading insights...".to_string()]
        };

        let items: Vec<ListItem> = insights
            .into_iter()
            .enumerate()
            .map(|(i, insight)| {
                let style = if Some(i) == self.list_state.selected() {
                    Style::default()
                        .bg(Color::DarkGray)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default()
                };
                ListItem::new(Line::from(insight)).style(style)
            })
            .collect();

        let list = List::new(items)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("Quick Insights"),
            )
            .highlight_style(
                Style::default()
                    .bg(Color::DarkGray)
                    .add_modifier(Modifier::BOLD),
            )
            .highlight_symbol("▶ ");

        f.render_stateful_widget(list, area, &mut self.list_state);
    }

    fn render_usage_stats(&mut self, f: &mut Frame, area: Rect) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Percentage(60), Constraint::Percentage(40)])
            .split(area);

        // Top - Usage chart
        self.render_usage_chart(f, chunks[0]);

        // Bottom - Provider/Project breakdown
        self.render_breakdown_tables(f, chunks[1]);
    }

    fn render_usage_chart(&self, f: &mut Frame, area: Rect) {
        if let Some(usage_data) = &self.usage_data {
            let data: Vec<(f64, f64)> = usage_data
                .daily_breakdown
                .iter()
                .enumerate()
                .map(|(i, day)| (i as f64, day.messages as f64))
                .collect();

            let dataset = Dataset::default()
                .name("Daily Messages")
                .marker(symbols::Marker::Braille)
                .graph_type(GraphType::Line)
                .style(Style::default().fg(Color::Cyan))
                .data(&data);

            let x_bounds = [0.0, data.len().max(1) as f64 - 1.0];
            let y_max = data.iter().map(|(_, y)| *y).fold(0.0, f64::max).max(1.0);

            let chart = Chart::new(vec![dataset])
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .title("Daily Message Usage"),
                )
                .x_axis(
                    Axis::default()
                        .title("Days")
                        .style(Style::default().fg(Color::Gray))
                        .bounds(x_bounds)
                        .labels(vec!["Start".into(), "End".into()]),
                )
                .y_axis(
                    Axis::default()
                        .title("Messages")
                        .style(Style::default().fg(Color::Gray))
                        .bounds([0.0, y_max])
                        .labels(vec!["0".into(), format!("{y_max:.0}").into()]),
                );

            f.render_widget(chart, area);
        } else {
            let placeholder = Paragraph::new("Loading usage chart...")
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .title("Daily Message Usage"),
                )
                .style(Style::default().fg(Color::Gray));

            f.render_widget(placeholder, area);
        }
    }

    fn render_breakdown_tables(&mut self, f: &mut Frame, area: Rect) {
        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
            .split(area);

        // Provider breakdown
        if let Some(usage_data) = &self.usage_data {
            let provider_items: Vec<ListItem> = usage_data
                .provider_breakdown
                .iter()
                .map(|p| {
                    ListItem::new(Line::from(format!(
                        "{}: {} sessions, {} messages ({:.1}%)",
                        p.provider, p.sessions, p.messages, p.percentage
                    )))
                })
                .collect();

            let provider_list = List::new(provider_items)
                .block(Block::default().borders(Borders::ALL).title("By Provider"))
                .style(Style::default().fg(Color::White));

            f.render_widget(provider_list, chunks[0]);

            // Project breakdown
            let project_items: Vec<ListItem> = usage_data
                .project_breakdown
                .iter()
                .map(|p| {
                    ListItem::new(Line::from(format!(
                        "{}: {} sessions, {} messages ({:.1}%)",
                        p.project, p.sessions, p.messages, p.percentage
                    )))
                })
                .collect();

            let project_list = List::new(project_items)
                .block(Block::default().borders(Borders::ALL).title("By Project"))
                .style(Style::default().fg(Color::White));

            f.render_widget(project_list, chunks[1]);
        }
    }

    fn render_insights(&mut self, f: &mut Frame, area: Rect) {
        if let Some(insights_data) = &self.insights_data {
            let items: Vec<ListItem> = insights_data
                .insights
                .iter()
                .map(|insight| {
                    ListItem::new(vec![
                        Line::from(vec![
                            Span::styled(
                                &insight.title,
                                Style::default()
                                    .fg(Color::Yellow)
                                    .add_modifier(Modifier::BOLD),
                            ),
                            Span::styled(
                                format!(" (confidence: {:.1}%)", insight.confidence_score * 100.0),
                                Style::default().fg(Color::Gray),
                            ),
                        ]),
                        Line::from(format!("  {}", insight.description)),
                        Line::from(""),
                    ])
                })
                .collect();

            let list = List::new(items)
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .title("Detailed Insights"),
                )
                .highlight_style(
                    Style::default()
                        .bg(Color::DarkGray)
                        .add_modifier(Modifier::BOLD),
                )
                .highlight_symbol("▶ ");

            f.render_stateful_widget(list, area, &mut self.list_state);
        } else {
            let placeholder = Paragraph::new("Loading insights...")
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .title("Detailed Insights"),
                )
                .style(Style::default().fg(Color::Gray));

            f.render_widget(placeholder, area);
        }
    }

    fn render_trends(&mut self, f: &mut Frame, area: Rect) {
        if let Some(insights_data) = &self.insights_data {
            let trends_and_recs: Vec<ListItem> = insights_data
                .trends
                .iter()
                .map(|trend| {
                    let direction_color = match trend.direction.as_str() {
                        "increasing" => Color::Green,
                        "decreasing" => Color::Red,
                        _ => Color::Yellow,
                    };

                    ListItem::new(vec![
                        Line::from(vec![
                            Span::styled(
                                &trend.metric,
                                Style::default()
                                    .fg(Color::Cyan)
                                    .add_modifier(Modifier::BOLD),
                            ),
                            Span::styled(
                                format!(" {}", trend.direction),
                                Style::default().fg(direction_color),
                            ),
                            Span::styled(
                                format!(" ({:+.1}%)", trend.change_percentage),
                                Style::default().fg(direction_color),
                            ),
                        ]),
                        Line::from(format!(
                            "  Period: {} | Significance: {}",
                            trend.period, trend.significance
                        )),
                        Line::from(""),
                    ])
                })
                .chain(insights_data.recommendations.iter().map(|rec| {
                    let priority_color = match rec.priority.as_str() {
                        "high" => Color::Red,
                        "medium" => Color::Yellow,
                        "low" => Color::Green,
                        _ => Color::White,
                    };

                    ListItem::new(vec![
                        Line::from(vec![
                            Span::styled("💡 ", Style::default().fg(Color::Yellow)),
                            Span::styled(
                                &rec.title,
                                Style::default()
                                    .fg(Color::White)
                                    .add_modifier(Modifier::BOLD),
                            ),
                            Span::styled(
                                format!(" [{}]", rec.priority),
                                Style::default().fg(priority_color),
                            ),
                        ]),
                        Line::from(format!("  {}", rec.description)),
                        Line::from(""),
                    ])
                }))
                .collect();

            let list = List::new(trends_and_recs)
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .title("Trends & Recommendations"),
                )
                .highlight_style(
                    Style::default()
                        .bg(Color::DarkGray)
                        .add_modifier(Modifier::BOLD),
                )
                .highlight_symbol("▶ ");

            f.render_stateful_widget(list, area, &mut self.list_state);
        } else {
            let placeholder = Paragraph::new("Loading trends and recommendations...")
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .title("Trends & Recommendations"),
                )
                .style(Style::default().fg(Color::Gray));

            f.render_widget(placeholder, area);
        }
    }

    fn next_item(&mut self) {
        let item_count = match self.current_view {
            AnalyticsView::Overview => {
                if let Some(insights_data) = &self.insights_data {
                    insights_data.insights.len().min(5)
                } else {
                    1
                }
            }
            AnalyticsView::Insights => {
                if let Some(insights_data) = &self.insights_data {
                    insights_data.insights.len()
                } else {
                    1
                }
            }
            AnalyticsView::Trends => {
                if let Some(insights_data) = &self.insights_data {
                    insights_data.trends.len() + insights_data.recommendations.len()
                } else {
                    1
                }
            }
            _ => return,
        };

        if item_count == 0 {
            return;
        }

        let selected = self.list_state.selected().unwrap_or(0);
        let next = if selected >= item_count - 1 {
            0
        } else {
            selected + 1
        };
        self.list_state.select(Some(next));
    }

    fn previous_item(&mut self) {
        let item_count = match self.current_view {
            AnalyticsView::Overview => {
                if let Some(insights_data) = &self.insights_data {
                    insights_data.insights.len().min(5)
                } else {
                    1
                }
            }
            AnalyticsView::Insights => {
                if let Some(insights_data) = &self.insights_data {
                    insights_data.insights.len()
                } else {
                    1
                }
            }
            AnalyticsView::Trends => {
                if let Some(insights_data) = &self.insights_data {
                    insights_data.trends.len() + insights_data.recommendations.len()
                } else {
                    1
                }
            }
            _ => return,
        };

        if item_count == 0 {
            return;
        }

        let selected = self.list_state.selected().unwrap_or(0);
        let previous = if selected == 0 {
            item_count - 1
        } else {
            selected - 1
        };
        self.list_state.select(Some(previous));
    }

    fn next_view(&mut self) {
        self.current_view = match self.current_view {
            AnalyticsView::Overview => AnalyticsView::UsageStats,
            AnalyticsView::UsageStats => AnalyticsView::Insights,
            AnalyticsView::Insights => AnalyticsView::Trends,
            AnalyticsView::Trends => AnalyticsView::Overview,
        };
        self.list_state.select(Some(0));
    }

    fn previous_view(&mut self) {
        self.current_view = match self.current_view {
            AnalyticsView::Overview => AnalyticsView::Trends,
            AnalyticsView::UsageStats => AnalyticsView::Overview,
            AnalyticsView::Insights => AnalyticsView::UsageStats,
            AnalyticsView::Trends => AnalyticsView::Insights,
        };
        self.list_state.select(Some(0));
    }

    fn format_elapsed_time(&self) -> String {
        let elapsed = self.last_refresh.elapsed();
        if elapsed.as_secs() < 60 {
            format!("{}s ago", elapsed.as_secs())
        } else {
            format!("{}m ago", elapsed.as_secs() / 60)
        }
    }
}
