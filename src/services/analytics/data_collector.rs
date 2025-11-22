use super::metrics::{
    calculate_file_change_metrics, calculate_time_consumption_metrics,
    calculate_token_consumption_metrics, calculate_tool_usage_metrics,
};
use super::models::{
    EmbeddedToolUse, QualitativeInput, QuantitativeInput, SessionTranscript, SessionTurn,
};
use crate::models::{ChatSession, Message, MessageRole, ToolOperation};
use anyhow::Result;
use std::collections::HashMap;

/// Maximum character length for a single tool input/result before truncation
const TOOL_CONTENT_MAX_LENGTH: usize = 2000;

/// Placeholder text to indicate truncated content
const TRUNCATION_PLACEHOLDER: &str = "\n... [content truncated] ...\n";

// =============================================================================
// Data Collection Functions
// =============================================================================

pub async fn collect_quantitative_data(
    session: &ChatSession,
    messages: &[Message],
    tool_operations: &[ToolOperation],
) -> Result<QuantitativeInput> {
    let file_changes = calculate_file_change_metrics(tool_operations);
    let time_metrics = calculate_time_consumption_metrics(session, messages);
    let token_metrics = calculate_token_consumption_metrics(messages);
    let tool_usage = calculate_tool_usage_metrics(tool_operations);

    Ok(QuantitativeInput {
        file_changes,
        time_metrics,
        token_metrics,
        tool_usage,
    })
}

/// Collects qualitative data by building a raw JSON string representation of the chat session.
/// The JSON includes multi-turn messages with all tool uses embedded in each corresponding message.
/// Long tool content is truncated by cutting the center portion to meet character thresholds.
pub async fn collect_qualitative_data(
    tool_operations: &[ToolOperation],
    messages: &[Message],
    session: &ChatSession,
) -> Result<QualitativeInput> {
    let raw_session = build_session_transcript(messages, tool_operations, session)?;
    Ok(QualitativeInput { raw_session })
}

// =============================================================================
// Session Transcript Building
// =============================================================================

/// Builds a JSON string representation of the session transcript with embedded tool uses.
fn build_session_transcript(
    messages: &[Message],
    tool_operations: &[ToolOperation],
    session: &ChatSession,
) -> Result<String> {
    let mut turns: Vec<SessionTurn> = Vec::new();
    let mut turn_number = 0u32;

    // Build a map of tool_use_id to ToolOperation for quick lookup
    let tool_ops_map = build_tool_operations_map(tool_operations);

    for message in messages {
        // Skip thinking messages as they don't represent actual conversation
        if message.is_thinking() {
            continue;
        }

        // Increment turn for user messages
        if message.is_user_message() {
            turn_number += 1;
        }

        let role = match message.role {
            MessageRole::User => "user".to_string(),
            MessageRole::Assistant => "assistant".to_string(),
            MessageRole::System => "system".to_string(),
        };

        // Truncate message content if too long
        let content = truncate_content(&message.content, TOOL_CONTENT_MAX_LENGTH * 2);

        // Build embedded tool uses for this message (only for messages with tool_uses)
        let tool_uses = build_embedded_tool_uses(message, &tool_ops_map);

        turns.push(SessionTurn {
            turn_number,
            role,
            content,
            tool_uses,
        });
    }

    let transcript = SessionTranscript {
        session_id: session.id.to_string(),
        total_turns: turn_number,
        turns,
    };

    // Serialize to JSON string
    let json = serde_json::to_string_pretty(&transcript)?;
    Ok(json)
}

/// Builds a map from tool_use_id to ToolOperation for quick lookup.
fn build_tool_operations_map(tool_operations: &[ToolOperation]) -> HashMap<String, &ToolOperation> {
    let mut map = HashMap::new();

    for op in tool_operations {
        map.insert(op.tool_use_id.clone(), op);
    }

    map
}

/// Builds embedded tool uses for a message by looking up tool operations.
fn build_embedded_tool_uses(
    message: &Message,
    tool_ops_map: &HashMap<String, &ToolOperation>,
) -> Vec<EmbeddedToolUse> {
    let mut embedded = Vec::new();

    // Get tool_use_ids from message's tool_uses field
    if let Some(tool_uses) = &message.tool_uses {
        for tool_use in tool_uses {
            // Look up the tool operation by tool_use_id
            if let Some(tool_op) = tool_ops_map.get(&tool_use.id) {
                // Extract input from raw_input in ToolOperation
                let input = if let Some(raw_input) = &tool_op.raw_input {
                    format_tool_input(raw_input)
                } else {
                    String::new()
                };
                let truncated_input = truncate_content(&input, TOOL_CONTENT_MAX_LENGTH);

                // Extract result from raw_result or result_summary in ToolOperation
                let result = tool_op
                    .raw_result
                    .as_ref()
                    .map(|raw_result| {
                        let result_str = format_tool_input(raw_result);
                        truncate_content(&result_str, TOOL_CONTENT_MAX_LENGTH)
                    })
                    .or_else(|| {
                        tool_op
                            .result_summary
                            .as_ref()
                            .map(|summary| truncate_content(summary, TOOL_CONTENT_MAX_LENGTH))
                    });

                embedded.push(EmbeddedToolUse {
                    tool_name: tool_op.tool_name.clone(),
                    input: truncated_input,
                    result,
                    success: tool_op.success,
                });
            }
        }
    }

    embedded
}

/// Formats tool input Value as a readable string.
fn format_tool_input(input: &serde_json::Value) -> String {
    match input {
        serde_json::Value::Object(obj) => {
            // For objects, format key-value pairs nicely
            obj.iter()
                .map(|(k, v)| {
                    let value_str = match v {
                        serde_json::Value::String(s) => s.clone(),
                        _ => v.to_string(),
                    };
                    format!("{}: {}", k, value_str)
                })
                .collect::<Vec<_>>()
                .join("\n")
        }
        serde_json::Value::String(s) => s.clone(),
        _ => input.to_string(),
    }
}

/// Truncates content that exceeds the maximum length by removing the center portion.
/// This preserves the beginning and end of the content for context.
fn truncate_content(content: &str, max_length: usize) -> String {
    if content.len() <= max_length {
        return content.to_string();
    }

    // Calculate how much to keep from beginning and end
    let placeholder_len = TRUNCATION_PLACEHOLDER.len();
    let available_length = max_length.saturating_sub(placeholder_len);
    let half_length = available_length / 2;

    // Find safe char boundaries for UTF-8
    let start_end = find_char_boundary(content, half_length);
    let end_start = find_char_boundary_from_end(content, half_length);

    if start_end >= end_start {
        // Content is too short to truncate meaningfully, just take the beginning
        let end = find_char_boundary(content, max_length.saturating_sub(3));
        return format!("{}...", &content[..end]);
    }

    format!(
        "{}{}{}",
        &content[..start_end],
        TRUNCATION_PLACEHOLDER,
        &content[end_start..]
    )
}

/// Finds the nearest char boundary at or before the given byte position.
fn find_char_boundary(s: &str, pos: usize) -> usize {
    if pos >= s.len() {
        return s.len();
    }

    let mut idx = pos;
    while idx > 0 && !s.is_char_boundary(idx) {
        idx -= 1;
    }
    idx
}

/// Finds the nearest char boundary from the end of the string.
fn find_char_boundary_from_end(s: &str, bytes_from_end: usize) -> usize {
    if bytes_from_end >= s.len() {
        return 0;
    }

    let pos = s.len() - bytes_from_end;
    let mut idx = pos;
    while idx < s.len() && !s.is_char_boundary(idx) {
        idx += 1;
    }
    idx
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_truncate_content_short() {
        let content = "Hello, world!";
        let result = truncate_content(content, 100);
        assert_eq!(result, content);
    }

    #[test]
    fn test_truncate_content_long() {
        let content = "a".repeat(5000);
        let result = truncate_content(&content, 2000);
        assert!(result.len() <= 2000);
        assert!(result.contains(TRUNCATION_PLACEHOLDER));
    }

    #[test]
    fn test_truncate_content_utf8() {
        // Test with Korean characters (3 bytes each)
        let content = "안녕하세요".repeat(500);
        let result = truncate_content(&content, 500);
        // Should not panic and should be valid UTF-8
        assert!(result.len() <= 500 + TRUNCATION_PLACEHOLDER.len());
        assert!(std::str::from_utf8(result.as_bytes()).is_ok());
    }

    #[test]
    fn test_find_char_boundary() {
        let s = "Hello";
        assert_eq!(find_char_boundary(s, 3), 3);
        assert_eq!(find_char_boundary(s, 100), 5);

        let korean = "안녕";
        // "안" is 3 bytes, "녕" is 3 bytes
        assert_eq!(find_char_boundary(korean, 2), 0); // Middle of first char
        assert_eq!(find_char_boundary(korean, 3), 3); // End of first char
    }
}
