use crate::models::message::{ToolResult, ToolUse};
use crate::tools::parsers::{
    bash::BashData, edit::EditData, read::ReadData, write::WriteData, ToolData,
};
use crate::tools::ToolParsingService;
use ratatui::{
    style::{Color, Modifier, Style},
    text::{Line, Span},
};

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
        let bottom_line =
            self.create_tool_border(&format!(" {} ", tool_label), border_width, false);
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
        lines.push(self.create_tool_border(&format!(" {} ", tool_label), border_width, false));

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

        // File info line
        let size_info = if let Some(size) = data.content_size {
            if size > 1024 {
                format!("{:.1} KB", size as f64 / 1024.0)
            } else {
                format!("{} bytes", size)
            }
        } else {
            "unknown size".to_string()
        };

        lines.push(Line::from(vec![
            Span::raw("┃ "),
            Span::styled("📝 ", Style::default().fg(Color::Yellow)),
            Span::styled(
                format!("Write file ({})", size_info),
                Style::default().fg(Color::Yellow),
            ),
        ]));

        // Bottom border
        lines.push(self.create_tool_border(&format!(" {} ", tool_label), border_width, false));

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

        // Edit type line
        let edit_type = if data.is_refactoring() {
            "✏️  Refactoring (bulk replacement)"
        } else {
            "✏️  Editing file"
        };

        lines.push(Line::from(vec![
            Span::raw("┃ "),
            Span::styled(edit_type.to_string(), Style::default().fg(Color::Magenta)),
        ]));

        // Show diff if details enabled
        if config.show_details {
            lines.push(Line::from("┃"));

            if let Some(old) = &data.old_string {
                let old_lines: Vec<&str> = old.lines().take(3).collect();
                for line in old_lines {
                    lines.push(Line::from(vec![
                        Span::raw("┃ "),
                        Span::styled("- ", Style::default().fg(Color::Red)),
                        Span::styled(line.to_string(), Style::default().fg(Color::Red)),
                    ]));
                }
            }

            if let Some(new) = &data.new_string {
                let new_lines: Vec<&str> = new.lines().take(3).collect();
                for line in new_lines {
                    lines.push(Line::from(vec![
                        Span::raw("┃ "),
                        Span::styled("+ ", Style::default().fg(Color::Green)),
                        Span::styled(line.to_string(), Style::default().fg(Color::Green)),
                    ]));
                }
            }
        }

        // Bottom border
        lines.push(self.create_tool_border(&format!(" {} ", tool_label), border_width, false));

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
        lines.push(self.create_tool_border(
            &format!(" {} ", tool_use.name),
            border_width,
            false,
        ));

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
        let stdout = result
            .details
            .as_ref()
            .or(Some(&result.raw))
            .and_then(|details| {
                // Check if details is an array (Claude format)
                if let Some(array) = details.as_array() {
                    // Look for toolUseResult in the array
                    for item in array {
                        if let Some(obj) = item.as_object() {
                            if obj.get("type").and_then(|t| t.as_str()) == Some("toolUseResult") {
                                if let Some(metadata) = obj.get("toolUseResult") {
                                    return metadata
                                        .get("stdout")
                                        .and_then(|s| s.as_str())
                                        .map(String::from);
                                }
                            }
                        }
                    }
                }
                // Fallback to direct stdout field
                details
                    .get("stdout")
                    .and_then(|s| s.as_str())
                    .map(String::from)
            });

        let stderr = result
            .details
            .as_ref()
            .or(Some(&result.raw))
            .and_then(|details| {
                // Check if details is an array (Claude format)
                if let Some(array) = details.as_array() {
                    for item in array {
                        if let Some(obj) = item.as_object() {
                            if obj.get("type").and_then(|t| t.as_str()) == Some("toolUseResult") {
                                if let Some(metadata) = obj.get("toolUseResult") {
                                    return metadata
                                        .get("stderr")
                                        .and_then(|s| s.as_str())
                                        .map(String::from);
                                }
                            }
                        }
                    }
                }
                details
                    .get("stderr")
                    .and_then(|s| s.as_str())
                    .map(String::from)
            });

        (stdout, stderr)
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
