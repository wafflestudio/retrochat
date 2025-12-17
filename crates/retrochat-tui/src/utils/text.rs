use std::time::{SystemTime, UNIX_EPOCH};

/// Truncates text to a maximum length, adding ellipsis if needed
///
/// # Arguments
/// * `text` - The text to truncate
/// * `max_len` - Maximum length including the ellipsis
///
/// # Returns
/// The truncated text with "..." appended if it was truncated
///
/// # Example
/// ```
/// use retrochat_tui::utils::text::truncate_text;
///
/// let short = truncate_text("Hello, world!", 8);
/// assert_eq!(short, "Hello...");
/// ```
pub fn truncate_text(text: &str, max_len: usize) -> String {
    if text.chars().count() <= max_len {
        text.to_string()
    } else if max_len <= 3 {
        "...".to_string()
    } else {
        let truncate_len = max_len.saturating_sub(3);
        if truncate_len == 0 || text.is_empty() {
            "...".to_string()
        } else {
            // Use chars() to safely truncate at character boundaries
            let truncated: String = text.chars().take(truncate_len).collect();
            format!("{truncated}...")
        }
    }
}

/// Wraps text to fit within a specified width, preserving newlines
///
/// # Arguments
/// * `text` - The text to wrap
/// * `width` - The maximum width per line
///
/// # Returns
/// A vector of strings, each representing one line
///
/// # Example
/// ```
/// use retrochat_tui::utils::text::wrap_text;
///
/// let lines = wrap_text("This is a long line that needs wrapping", 10);
/// assert!(!lines.is_empty());
/// ```
pub fn wrap_text(text: &str, width: usize) -> Vec<String> {
    if width == 0 {
        return text.lines().map(|l| l.to_string()).collect();
    }

    let mut result_lines = Vec::new();

    // First split by newlines to preserve them
    for line in text.lines() {
        let mut wrapped = wrap_single_line(line, width);
        result_lines.append(&mut wrapped);
    }

    if result_lines.is_empty() {
        result_lines.push(String::new());
    }

    result_lines
}

/// Wraps a single line of text (no newlines) to fit within a specified width
fn wrap_single_line(text: &str, width: usize) -> Vec<String> {
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
                while remaining.chars().count() > width {
                    if width > 0 {
                        let mut chars = remaining.chars();
                        let chunk: String = chars.by_ref().take(width).collect();
                        lines.push(chunk);
                        remaining = chars.as_str();
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

/// Returns a spinner character that animates over time
///
/// This function returns different characters from a spinner animation
/// based on the current time. It's useful for showing loading states.
///
/// # Returns
/// A Unicode spinner character
///
/// # Example
/// ```
/// use retrochat_tui::utils::text::get_spinner_char;
///
/// let spinner = get_spinner_char();
/// assert!("⠋⠙⠹⠸⠼⠴⠦⠧".contains(spinner));
/// ```
pub fn get_spinner_char() -> char {
    const SPINNER_CHARS: [char; 8] = ['⠋', '⠙', '⠹', '⠸', '⠼', '⠴', '⠦', '⠧'];
    let now_millis = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis();
    let frame = (now_millis / 100) % 8; // Change frame every 100ms
    SPINNER_CHARS[frame as usize]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_truncate_text_short() {
        assert_eq!(truncate_text("Hello", 10), "Hello");
    }

    #[test]
    fn test_truncate_text_long() {
        assert_eq!(truncate_text("Hello, World!", 8), "Hello...");
    }

    #[test]
    fn test_truncate_text_exact() {
        assert_eq!(truncate_text("Hello", 5), "Hello");
    }

    #[test]
    fn test_truncate_text_very_short() {
        assert_eq!(truncate_text("Hello", 3), "...");
    }

    #[test]
    fn test_wrap_text_no_wrap() {
        let lines = wrap_text("Hello", 10);
        assert_eq!(lines, vec!["Hello"]);
    }

    #[test]
    fn test_wrap_text_simple_wrap() {
        let lines = wrap_text("Hello World", 8);
        assert_eq!(lines, vec!["Hello", "World"]);
    }

    #[test]
    fn test_wrap_text_multiple_lines() {
        let lines = wrap_text("Hello World This Is A Test", 10);
        // "Hello" -> line 1
        // "World This" -> line 2
        // "Is A Test" -> line 3
        assert_eq!(lines.len(), 3);
        assert_eq!(lines, vec!["Hello", "World This", "Is A Test"]);
    }

    #[test]
    fn test_get_spinner_char() {
        // Just test that it returns a valid character
        let spinner = get_spinner_char();
        assert!("⠋⠙⠹⠸⠼⠴⠦⠧".contains(spinner));
    }

    #[test]
    fn test_wrap_text_preserves_newlines() {
        let text = "Line 1\nLine 2\nLine 3";
        let lines = wrap_text(text, 100);
        assert_eq!(lines, vec!["Line 1", "Line 2", "Line 3"]);
    }

    #[test]
    fn test_wrap_text_with_newlines_and_wrapping() {
        let text = "Short\nThis is a very long line that needs wrapping";
        let lines = wrap_text(text, 15);
        assert_eq!(lines[0], "Short");
        assert!(lines.len() > 2); // Should wrap the long line into multiple lines
    }

    #[test]
    fn test_wrap_text_empty_lines() {
        let text = "Line 1\n\nLine 3";
        let lines = wrap_text(text, 100);
        assert_eq!(lines, vec!["Line 1", "", "Line 3"]);
    }
}
