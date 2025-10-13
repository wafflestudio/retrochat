use ratatui::style::{Color, Modifier, Style};

/// Returns a style for a given provider name
///
/// # Arguments
/// * `provider` - The provider name (e.g., "claude-code", "gemini", "cursor", "chatgpt")
///
/// # Returns
/// A `Style` with appropriate color for the provider
pub fn provider_style(provider: &str) -> Style {
    match provider {
        "claude-code" => Style::default().fg(Color::Blue),
        "gemini" => Style::default().fg(Color::Green),
        "cursor" => Style::default().fg(Color::Magenta),
        "chatgpt" => Style::default().fg(Color::Cyan),
        _ => Style::default().fg(Color::White),
    }
}

/// Returns a style for a given message role
///
/// # Arguments
/// * `role` - The message role ("user", "assistant", "system")
///
/// # Returns
/// A `Style` with appropriate color and modifier for the role
pub fn role_style(role: &str) -> Style {
    match role {
        "user" => Style::default()
            .fg(Color::Green)
            .add_modifier(Modifier::BOLD),
        "assistant" => Style::default()
            .fg(Color::Blue)
            .add_modifier(Modifier::BOLD),
        "system" => Style::default()
            .fg(Color::Yellow)
            .add_modifier(Modifier::BOLD),
        _ => Style::default().fg(Color::White),
    }
}

/// Common style constants
pub mod colors {
    use ratatui::style::Color;

    pub const ERROR: Color = Color::Red;
    pub const SUCCESS: Color = Color::LightGreen;
    pub const WARNING: Color = Color::Yellow;
    pub const INFO: Color = Color::Cyan;
    pub const DIMMED: Color = Color::Gray;
    pub const HIGHLIGHT: Color = Color::Yellow;
    pub const BACKGROUND: Color = Color::DarkGray;
}

/// Preset styles for common UI elements
pub mod presets {
    use ratatui::style::{Color, Modifier, Style};

    pub fn error() -> Style {
        Style::default().fg(Color::Red).add_modifier(Modifier::BOLD)
    }

    pub fn success() -> Style {
        Style::default()
            .fg(Color::LightGreen)
            .add_modifier(Modifier::BOLD)
    }

    pub fn warning() -> Style {
        Style::default()
            .fg(Color::Yellow)
            .add_modifier(Modifier::BOLD)
    }

    pub fn info() -> Style {
        Style::default()
            .fg(Color::Cyan)
            .add_modifier(Modifier::BOLD)
    }

    pub fn dimmed() -> Style {
        Style::default().fg(Color::Gray).add_modifier(Modifier::DIM)
    }

    pub fn highlight() -> Style {
        Style::default()
            .fg(Color::Yellow)
            .add_modifier(Modifier::BOLD)
    }

    pub fn title() -> Style {
        Style::default()
            .fg(Color::Cyan)
            .add_modifier(Modifier::BOLD)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_provider_styles() {
        assert_eq!(provider_style("claude-code").fg, Some(Color::Blue));
        assert_eq!(provider_style("gemini").fg, Some(Color::Green));
        assert_eq!(provider_style("cursor").fg, Some(Color::Magenta));
        assert_eq!(provider_style("chatgpt").fg, Some(Color::Cyan));
        assert_eq!(provider_style("unknown").fg, Some(Color::White));
    }

    #[test]
    fn test_role_styles() {
        assert_eq!(role_style("user").fg, Some(Color::Green));
        assert_eq!(role_style("assistant").fg, Some(Color::Blue));
        assert_eq!(role_style("system").fg, Some(Color::Yellow));
    }
}
