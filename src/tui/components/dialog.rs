use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::Line,
    widgets::{Block, Borders, Clear, Paragraph, Wrap},
    Frame,
};

use crate::tui::utils::layout::centered_rect;

/// Dialog type determines the styling and behavior
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DialogType {
    Error,
    Info,
    Help,
    Warning,
}

impl DialogType {
    /// Get the border style for this dialog type
    fn border_style(&self) -> Style {
        match self {
            DialogType::Error => Style::default().fg(Color::Red),
            DialogType::Info => Style::default().fg(Color::Cyan),
            DialogType::Help => Style::default().fg(Color::Cyan),
            DialogType::Warning => Style::default().fg(Color::Yellow),
        }
    }

    /// Get the default title for this dialog type
    fn default_title(&self) -> &str {
        match self {
            DialogType::Error => "Error",
            DialogType::Info => "Info",
            DialogType::Help => "Help",
            DialogType::Warning => "Warning",
        }
    }

    /// Get the footer text for this dialog type
    fn footer_text(&self) -> &str {
        match self {
            DialogType::Error => "Press any key to continue",
            DialogType::Info => "Press any key to continue",
            DialogType::Help => "Press any key to close this help screen",
            DialogType::Warning => "Press any key to continue",
        }
    }
}

/// A reusable dialog component for displaying messages
pub struct Dialog<'a> {
    dialog_type: DialogType,
    title: Option<&'a str>,
    content: Vec<Line<'a>>,
    width_percent: u16,
    height_percent: u16,
    wrap: bool,
    show_footer: bool,
}

impl<'a> Dialog<'a> {
    /// Create a new dialog with the specified type and content
    pub fn new(dialog_type: DialogType, content: Vec<Line<'a>>) -> Self {
        Self {
            dialog_type,
            title: None,
            content,
            width_percent: 60,
            height_percent: 40,
            wrap: true,
            show_footer: true,
        }
    }

    /// Set a custom title (overrides the default)
    pub fn title(mut self, title: &'a str) -> Self {
        self.title = Some(title);
        self
    }

    /// Set the dialog size as a percentage of the screen
    pub fn size(mut self, width_percent: u16, height_percent: u16) -> Self {
        self.width_percent = width_percent;
        self.height_percent = height_percent;
        self
    }

    /// Enable or disable text wrapping
    pub fn wrap(mut self, wrap: bool) -> Self {
        self.wrap = wrap;
        self
    }

    /// Show or hide the footer text
    pub fn show_footer(mut self, show: bool) -> Self {
        self.show_footer = show;
        self
    }

    /// Render the dialog to the frame
    pub fn render(&self, f: &mut Frame, area: Rect) {
        let popup_area = centered_rect(self.width_percent, self.height_percent, area);

        // Build the complete content with footer if needed
        let mut lines = self.content.clone();

        if self.show_footer {
            use ratatui::text::Span;

            lines.push(Line::from(""));
            lines.push(Line::from(vec![Span::styled(
                self.dialog_type.footer_text(),
                Style::default().fg(Color::Gray),
            )]));
        }

        let paragraph = Paragraph::new(lines)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(self.title.unwrap_or(self.dialog_type.default_title()))
                    .style(self.dialog_type.border_style()),
            )
            .style(Style::default().fg(Color::White));

        let paragraph = if self.wrap {
            paragraph.wrap(Wrap { trim: true })
        } else {
            paragraph
        };

        // Clear the area and render the dialog
        f.render_widget(Clear, popup_area);
        f.render_widget(paragraph, popup_area);
    }
}

/// Helper function to create an error dialog with a simple message
pub fn error_dialog(message: &str) -> Dialog<'_> {
    use ratatui::text::Span;

    let content = vec![
        Line::from(vec![Span::styled(
            "Error",
            Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
        )]),
        Line::from(""),
        Line::from(message),
    ];

    Dialog::new(DialogType::Error, content).size(60, 40)
}

/// Helper function to create an info dialog with a simple message
pub fn info_dialog(message: &str) -> Dialog<'_> {
    use ratatui::text::Span;

    let content = vec![
        Line::from(vec![Span::styled(
            "Information",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )]),
        Line::from(""),
        Line::from(message),
    ];

    Dialog::new(DialogType::Info, content).size(60, 40)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dialog_type_styles() {
        assert_eq!(DialogType::Error.border_style().fg, Some(Color::Red));
        assert_eq!(DialogType::Info.border_style().fg, Some(Color::Cyan));
        assert_eq!(DialogType::Help.border_style().fg, Some(Color::Cyan));
        assert_eq!(DialogType::Warning.border_style().fg, Some(Color::Yellow));
    }

    #[test]
    fn test_dialog_titles() {
        assert_eq!(DialogType::Error.default_title(), "Error");
        assert_eq!(DialogType::Info.default_title(), "Info");
        assert_eq!(DialogType::Help.default_title(), "Help");
        assert_eq!(DialogType::Warning.default_title(), "Warning");
    }

    #[test]
    fn test_dialog_builder() {
        let content = vec![Line::from("Test content")];
        let dialog = Dialog::new(DialogType::Info, content)
            .title("Custom Title")
            .size(80, 60)
            .wrap(false)
            .show_footer(false);

        assert_eq!(dialog.title, Some("Custom Title"));
        assert_eq!(dialog.width_percent, 80);
        assert_eq!(dialog.height_percent, 60);
        assert!(!dialog.wrap);
        assert!(!dialog.show_footer);
    }
}
