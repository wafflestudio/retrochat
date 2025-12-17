use ratatui::{
    style::{Color, Modifier, Style},
    text::{Line, Span},
};
use retrochat_core::models::message::{ToolResult, ToolUse};
use retrochat_core::tools::parsers::{
    bash::BashData, edit::EditData, read::ReadData, write::WriteData, ToolData,
};
use retrochat_core::tools::ToolParsingService;

/// Tool display configuration
pub struct ToolDisplayConfig {
    /// Maximum width available for rendering
    pub width: usize,
    /// Whether to show detailed output (can be toggled)
    pub show_details: bool,
    /// Maximum lines to show for tool output before truncation
    pub max_output_lines: usize,
}

impl Default for ToolDisplayConfig {
    fn default() -> Self {
        Self {
            width: 80,
            show_details: false,
            max_output_lines: 10,
        }
    }
}

/// Format tool uses and results for display in TUI
pub struct ToolDisplayFormatter {
    parsing_service: ToolParsingService,
}

impl ToolDisplayFormatter {
    pub fn new() -> Self {
        Self {
            parsing_service: ToolParsingService::new(),
        }
    }

    /// Format all tools from a message into display lines
    pub fn format_tools(
        &self,
        tool_uses: &[ToolUse],
        tool_results: &[ToolResult],
        config: &ToolDisplayConfig,
    ) -> Vec<Line<'static>> {
        let mut lines = Vec::new();

        for tool_use in tool_uses {
            // Find matching result
            let result = tool_results.iter().find(|r| r.tool_use_id == tool_use.id);

            // Parse the tool
            let tool_lines = match self.parsing_service.parse_tool(tool_use) {
                Ok(parsed_tool) => match parsed_tool.data {
                    ToolData::Bash(data) => {
                        self.format_bash_tool(&data, result, config, &tool_use.name)
                    }
                    ToolData::Read(data) => {
                        self.format_read_tool(&data, result, config, &tool_use.name)
                    }
                    ToolData::Write(data) => {
                        self.format_write_tool(&data, result, config, &tool_use.name)
                    }
                    ToolData::Edit(data) => {
                        self.format_edit_tool(&data, result, config, &tool_use.name)
                    }
                    ToolData::Unknown => self.format_unknown_tool(tool_use, result, config),
                },
                Err(_) => self.format_unknown_tool(tool_use, result, config),
            };

            lines.extend(tool_lines);
            lines.push(Line::from("")); // Empty line after each tool
        }

        lines
    }

    /// Format a Bash tool execution
    fn format_bash_tool(
        &self,
        data: &BashData,
        result: Option<&ToolResult>,
        config: &ToolDisplayConfig,
        tool_label: &str,
    ) -> Vec<Line<'static>> {
        let mut lines = Vec::new();

        // Top border with title
        let title = format!(
            " [Bash] {} ",
            data.description.as_deref().unwrap_or("Command execution")
        );
        let border_width = config.width.saturating_sub(4);
        let title_line = self.create_tool_border(&title, border_width, true);
        lines.push(title_line);

        // Command line with icon
        let command_line = Line::from(vec![
            Span::raw("┃ "),
            Span::styled("💻 $ ", Style::default().fg(Color::Cyan)),
            Span::styled(
                data.command.clone(),
                Style::default()
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(" "),
        ]);
        lines.push(command_line);

        // Show result if available and details enabled
        if config.show_details {
            if let Some(result) = result {
                lines.push(Line::from("┃"));

                // Parse result details for stdout/stderr
                let (stdout, stderr) = self.extract_bash_output(result);

                if let Some(stdout_text) = stdout {
                    if !stdout_text.trim().is_empty() {
                        let output_lines: Vec<&str> = stdout_text.lines().collect();
                        let display_lines = if output_lines.len() > config.max_output_lines {
                            &output_lines[..config.max_output_lines]
                        } else {
                            &output_lines[..]
                        };

                        for line in display_lines {
                            lines.push(Line::from(vec![
                                Span::raw("┃ "),
                                Span::styled(
                                    if result.is_error { "✗" } else { "✓" },
                                    Style::default().fg(if result.is_error {
                                        Color::Red
                                    } else {
                                        Color::Green
                                    }),
                                ),
                                Span::raw(" "),
                                Span::styled(line.to_string(), Style::default().fg(Color::Gray)),
                            ]));
                        }

                        if output_lines.len() > config.max_output_lines {
                            lines.push(Line::from(vec![
                                Span::raw("┃ "),
                                Span::styled(
                                    format!(
                                        "... ({} more lines)",
                                        output_lines.len() - config.max_output_lines
                                    ),
                                    Style::default()
                                        .fg(Color::DarkGray)
                                        .add_modifier(Modifier::ITALIC),
                                ),
                            ]));
                        }
                    }
                }

                if let Some(stderr_text) = stderr {
                    if !stderr_text.trim().is_empty() {
                        for line in stderr_text.lines().take(config.max_output_lines) {
                            lines.push(Line::from(vec![
                                Span::raw("┃ "),
                                Span::styled("✗ ", Style::default().fg(Color::Red)),
                                Span::styled(line.to_string(), Style::default().fg(Color::Red)),
                            ]));
                        }
                    }
                }
            }
        }

        // Vendor badge on bottom border
        let bottom_line = self.create_tool_border(&format!(" {tool_label} "), border_width, false);
        lines.push(bottom_line);

        lines
    }

    /// Format a Read tool operation
    fn format_read_tool(
        &self,
        data: &ReadData,
        _result: Option<&ToolResult>,
        config: &ToolDisplayConfig,
        tool_label: &str,
    ) -> Vec<Line<'static>> {
        let mut lines = Vec::new();

        // Top border
        let title = format!(" [Read] {} ", data.file_path);
        let border_width = config.width.saturating_sub(4);
        lines.push(self.create_tool_border(&title, border_width, true));

        // File info line
        let file_info = if data.is_partial_read() {
            format!(
                "📄 Read partial (offset: {}, limit: {})",
                data.offset.unwrap_or(0),
                data.limit.unwrap_or(0)
            )
        } else {
            "📄 Read file".to_string()
        };

        lines.push(Line::from(vec![
            Span::raw("┃ "),
            Span::styled(file_info, Style::default().fg(Color::Cyan)),
        ]));

        // Bottom border
        lines.push(self.create_tool_border(&format!(" {tool_label} "), border_width, false));

        lines
    }

    /// Format a Write tool operation
    fn format_write_tool(
        &self,
        data: &WriteData,
        _result: Option<&ToolResult>,
        config: &ToolDisplayConfig,
        tool_label: &str,
    ) -> Vec<Line<'static>> {
        let mut lines = Vec::new();

        // Top border
        let title = format!(" [Write] {} ", data.file_path);
        let border_width = config.width.saturating_sub(4);
        lines.push(self.create_tool_border(&title, border_width, true));

        // File info line with size and line count
        let size_info = if let Some(size) = data.content_size {
            if size > 1024 {
                format!("{:.1} KB", size as f64 / 1024.0)
            } else {
                format!("{size} bytes")
            }
        } else {
            "unknown size".to_string()
        };

        let lines_count = data.lines_after().unwrap_or(0);
        let summary = if lines_count > 0 {
            format!(" ({lines_count} lines, {size_info})")
        } else {
            format!(" ({size_info})")
        };

        lines.push(Line::from(vec![
            Span::raw("┃ "),
            Span::styled("📝 ", Style::default().fg(Color::Yellow)),
            Span::styled(
                format!("Write file{summary}"),
                Style::default().fg(Color::Yellow),
            ),
        ]));

        // Show content preview if details enabled
        if config.show_details {
            if let Some(content) = &data.content {
                lines.push(Line::from("┃"));

                let content_lines: Vec<&str> = content.lines().collect();
                let max_lines = config.max_output_lines;
                let display_lines = if content_lines.len() > max_lines {
                    &content_lines[..max_lines]
                } else {
                    &content_lines[..]
                };

                // Show content with line numbers
                for (idx, line) in display_lines.iter().enumerate() {
                    lines.push(Line::from(vec![
                        Span::raw("┃ "),
                        Span::styled(
                            format!("{:>3} ", idx + 1),
                            Style::default().fg(Color::DarkGray),
                        ),
                        Span::styled("+ ", Style::default().fg(Color::Green)),
                        Span::styled(line.to_string(), Style::default().fg(Color::Green)),
                    ]));
                }

                // Show truncation indicator if needed
                if content_lines.len() > max_lines {
                    lines.push(Line::from(vec![
                        Span::raw("┃ "),
                        Span::styled(
                            format!("    ... ({} more lines)", content_lines.len() - max_lines),
                            Style::default()
                                .fg(Color::DarkGray)
                                .add_modifier(Modifier::ITALIC),
                        ),
                    ]));
                }
            }
        } else {
            // When details are not shown, still show a brief summary
            if lines_count > 0 {
                lines.push(Line::from(vec![
                    Span::raw("┃ "),
                    Span::styled(
                        format!("  Writing {lines_count} lines"),
                        Style::default()
                            .fg(Color::Gray)
                            .add_modifier(Modifier::ITALIC),
                    ),
                ]));
            }
        }

        // Bottom border
        lines.push(self.create_tool_border(&format!(" {tool_label} "), border_width, false));

        lines
    }

    /// Format an Edit tool operation
    fn format_edit_tool(
        &self,
        data: &EditData,
        _result: Option<&ToolResult>,
        config: &ToolDisplayConfig,
        tool_label: &str,
    ) -> Vec<Line<'static>> {
        let mut lines = Vec::new();

        // Top border
        let title = format!(" [Edit] {} ", data.file_path);
        let border_width = config.width.saturating_sub(4);
        lines.push(self.create_tool_border(&title, border_width, true));

        // Edit type and summary line
        let edit_type = if data.is_refactoring() {
            "✏️  Refactoring (bulk replacement)"
        } else {
            "✏️  Editing file"
        };

        let lines_before = data.lines_before().unwrap_or(0);
        let lines_after = data.lines_after().unwrap_or(0);
        let summary = if lines_before > 0 || lines_after > 0 {
            format!(" ({lines_before} → {lines_after} lines)")
        } else {
            String::new()
        };

        lines.push(Line::from(vec![
            Span::raw("┃ "),
            Span::styled(
                format!("{edit_type}{summary}"),
                Style::default().fg(Color::Magenta),
            ),
        ]));

        // Show diff if details enabled
        if config.show_details {
            lines.push(Line::from("┃"));

            // Create unified diff view
            let (old_str, new_str) = match (&data.old_string, &data.new_string) {
                (Some(o), Some(n)) => (o.as_str(), n.as_str()),
                (Some(o), None) => (o.as_str(), ""),
                (None, Some(n)) => ("", n.as_str()),
                (None, None) => ("", ""),
            };

            let old_lines_vec: Vec<&str> = old_str.lines().collect();
            let new_lines_vec: Vec<&str> = new_str.lines().collect();

            let max_lines = config.max_output_lines;
            let total_lines = old_lines_vec.len().max(new_lines_vec.len());

            // If content is small enough, show full diff
            if total_lines <= max_lines {
                // Show all old lines (removed)
                for (idx, line) in old_lines_vec.iter().enumerate() {
                    lines.push(Line::from(vec![
                        Span::raw("┃ "),
                        Span::styled(
                            format!("{:>3} ", idx + 1),
                            Style::default().fg(Color::DarkGray),
                        ),
                        Span::styled("- ", Style::default().fg(Color::Red)),
                        Span::styled(line.to_string(), Style::default().fg(Color::Red)),
                    ]));
                }

                // Separator if both old and new exist
                if !old_lines_vec.is_empty() && !new_lines_vec.is_empty() {
                    lines.push(Line::from(vec![
                        Span::raw("┃ "),
                        Span::styled("    ╌╌╌", Style::default().fg(Color::DarkGray)),
                    ]));
                }

                // Show all new lines (added)
                for (idx, line) in new_lines_vec.iter().enumerate() {
                    lines.push(Line::from(vec![
                        Span::raw("┃ "),
                        Span::styled(
                            format!("{:>3} ", idx + 1),
                            Style::default().fg(Color::DarkGray),
                        ),
                        Span::styled("+ ", Style::default().fg(Color::Green)),
                        Span::styled(line.to_string(), Style::default().fg(Color::Green)),
                    ]));
                }
            } else {
                // Content is large, show truncated view
                let show_lines = max_lines / 2;

                // Show first N old lines
                for (idx, line) in old_lines_vec.iter().take(show_lines).enumerate() {
                    lines.push(Line::from(vec![
                        Span::raw("┃ "),
                        Span::styled(
                            format!("{:>3} ", idx + 1),
                            Style::default().fg(Color::DarkGray),
                        ),
                        Span::styled("- ", Style::default().fg(Color::Red)),
                        Span::styled(line.to_string(), Style::default().fg(Color::Red)),
                    ]));
                }

                if old_lines_vec.len() > show_lines {
                    lines.push(Line::from(vec![
                        Span::raw("┃ "),
                        Span::styled(
                            format!("    ... ({} more lines)", old_lines_vec.len() - show_lines),
                            Style::default()
                                .fg(Color::DarkGray)
                                .add_modifier(Modifier::ITALIC),
                        ),
                    ]));
                }

                // Separator
                if !old_lines_vec.is_empty() && !new_lines_vec.is_empty() {
                    lines.push(Line::from(vec![
                        Span::raw("┃ "),
                        Span::styled("    ╌╌╌", Style::default().fg(Color::DarkGray)),
                    ]));
                }

                // Show first N new lines
                for (idx, line) in new_lines_vec.iter().take(show_lines).enumerate() {
                    lines.push(Line::from(vec![
                        Span::raw("┃ "),
                        Span::styled(
                            format!("{:>3} ", idx + 1),
                            Style::default().fg(Color::DarkGray),
                        ),
                        Span::styled("+ ", Style::default().fg(Color::Green)),
                        Span::styled(line.to_string(), Style::default().fg(Color::Green)),
                    ]));
                }

                if new_lines_vec.len() > show_lines {
                    lines.push(Line::from(vec![
                        Span::raw("┃ "),
                        Span::styled(
                            format!("    ... ({} more lines)", new_lines_vec.len() - show_lines),
                            Style::default()
                                .fg(Color::DarkGray)
                                .add_modifier(Modifier::ITALIC),
                        ),
                    ]));
                }
            }
        } else {
            // When details are not shown, still show a summary
            let lines_before = data.lines_before().unwrap_or(0);
            let lines_after = data.lines_after().unwrap_or(0);
            if lines_before > 0 || lines_after > 0 {
                let change_summary = if lines_before == lines_after {
                    format!("Modified {lines_after} lines")
                } else if lines_before < lines_after {
                    let added = lines_after - lines_before;
                    format!("Added {added} lines ({lines_before} → {lines_after})")
                } else {
                    let removed = lines_before - lines_after;
                    format!("Removed {removed} lines ({lines_before} → {lines_after})")
                };

                lines.push(Line::from(vec![
                    Span::raw("┃ "),
                    Span::styled(
                        format!("  {change_summary}"),
                        Style::default()
                            .fg(Color::Gray)
                            .add_modifier(Modifier::ITALIC),
                    ),
                ]));
            }
        }

        // Bottom border
        lines.push(self.create_tool_border(&format!(" {tool_label} "), border_width, false));

        lines
    }

    /// Format an unknown tool
    fn format_unknown_tool(
        &self,
        tool_use: &ToolUse,
        _result: Option<&ToolResult>,
        config: &ToolDisplayConfig,
    ) -> Vec<Line<'static>> {
        let mut lines = Vec::new();

        // Top border
        let title = format!(" [{}] Unknown Tool ", tool_use.name);
        let border_width = config.width.saturating_sub(4);
        lines.push(self.create_tool_border(&title, border_width, true));

        lines.push(Line::from(vec![
            Span::raw("┃ "),
            Span::styled(
                format!("❓ Tool type: {}", tool_use.name),
                Style::default().fg(Color::DarkGray),
            ),
        ]));

        // Bottom border
        lines.push(self.create_tool_border(&format!(" {} ", tool_use.name), border_width, false));

        lines
    }

    /// Create a tool box border line
    fn create_tool_border(&self, title: &str, width: usize, is_top: bool) -> Line<'static> {
        let (left, right, fill) = if is_top {
            ("┏", "┓", "━")
        } else {
            ("┗", "┛", "━")
        };

        let title_len = title.chars().count();
        let fill_len = width.saturating_sub(title_len);
        let left_fill = fill_len / 2;
        let right_fill = fill_len - left_fill;

        Line::from(vec![
            Span::styled(left.to_string(), Style::default().fg(Color::Cyan)),
            Span::styled(fill.repeat(left_fill), Style::default().fg(Color::Cyan)),
            Span::styled(
                title.to_string(),
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(fill.repeat(right_fill), Style::default().fg(Color::Cyan)),
            Span::styled(right.to_string(), Style::default().fg(Color::Cyan)),
        ])
    }

    /// Extract stdout and stderr from bash tool result
    fn extract_bash_output(&self, result: &ToolResult) -> (Option<String>, Option<String>) {
        retrochat_core::utils::bash_utils::extract_bash_output(result)
    }
}

impl Default for ToolDisplayFormatter {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn create_bash_tool_use() -> ToolUse {
        ToolUse {
            id: "test_id".to_string(),
            name: "Bash".to_string(),
            input: json!({
                "command": "cargo test",
                "description": "Run tests"
            }),
            raw: json!({}),
        }
    }

    #[test]
    fn test_format_bash_tool() {
        let formatter = ToolDisplayFormatter::new();
        let tool_use = create_bash_tool_use();
        let config = ToolDisplayConfig::default();

        let lines = formatter.format_tools(&[tool_use], &[], &config);

        assert!(!lines.is_empty());
        // Should have at least border, command, and bottom border
        assert!(lines.len() >= 3);
    }

    #[test]
    fn test_format_multiple_tools() {
        let formatter = ToolDisplayFormatter::new();
        let tool1 = create_bash_tool_use();
        let mut tool2 = create_bash_tool_use();
        tool2.id = "test_id_2".to_string();

        let config = ToolDisplayConfig::default();
        let lines = formatter.format_tools(&[tool1, tool2], &[], &config);

        // Should have lines for both tools plus empty line separator
        assert!(lines.len() >= 6);
    }
}
