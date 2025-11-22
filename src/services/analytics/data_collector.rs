use super::metrics::{
    calculate_file_change_metrics, calculate_time_consumption_metrics,
    calculate_token_consumption_metrics, calculate_tool_usage_metrics,
};
use super::models::{QualitativeInput, QuantitativeInput, SessionTranscript, SessionTurn};
use crate::models::message::MessageType;
use crate::models::{ChatSession, Message, MessageRole, ToolOperation};
use anyhow::Result;
use std::collections::HashMap;
use uuid::Uuid;

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

        // Determine content based on message type
        let content = match message.message_type {
            MessageType::ToolRequest => {
                // For tool request messages, use raw_input from tool operation
                get_tool_request_content(message, &tool_ops_map)
            }
            MessageType::ToolResult => {
                // For tool result messages, use raw_result from tool operation
                get_tool_result_content(message, &tool_ops_map)
            }
            _ => {
                // For other messages, use the message content
                truncate_content(&message.content, TOOL_CONTENT_MAX_LENGTH * 2)
            }
        };

        turns.push(SessionTurn {
            turn_number,
            role,
            content,
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

/// Builds a map from ToolOperation.id (Uuid) to ToolOperation for quick lookup.
fn build_tool_operations_map(tool_operations: &[ToolOperation]) -> HashMap<Uuid, &ToolOperation> {
    let mut map = HashMap::new();

    for op in tool_operations {
        map.insert(op.id, op);
    }

    map
}

/// Gets content for a ToolRequest message by extracting raw_input from the tool operation.
fn get_tool_request_content(
    message: &Message,
    tool_ops_map: &HashMap<Uuid, &ToolOperation>,
) -> String {
    // Look up tool operation using message.tool_operation_id
    if let Some(tool_op_id) = message.tool_operation_id {
        if let Some(tool_op) = tool_ops_map.get(&tool_op_id) {
            if let Some(raw_input) = &tool_op.raw_input {
                let input_str = format_tool_input(raw_input);
                return truncate_content(&input_str, TOOL_CONTENT_MAX_LENGTH * 2);
            }
        }
    }
    // Fallback to message content if no raw_input found
    truncate_content(&message.content, TOOL_CONTENT_MAX_LENGTH * 2)
}

/// Gets content for a ToolResult message by extracting raw_result from the tool operation.
fn get_tool_result_content(
    message: &Message,
    tool_ops_map: &HashMap<Uuid, &ToolOperation>,
) -> String {
    // Look up tool operation using message.tool_operation_id
    if let Some(tool_op_id) = message.tool_operation_id {
        if let Some(tool_op) = tool_ops_map.get(&tool_op_id) {
            if let Some(raw_result) = &tool_op.raw_result {
                let result_str = format_tool_input(raw_result);
                return truncate_content(&result_str, TOOL_CONTENT_MAX_LENGTH * 2);
            }
            // Fall back to result_summary if raw_result is not available
            if let Some(summary) = &tool_op.result_summary {
                return truncate_content(summary, TOOL_CONTENT_MAX_LENGTH * 2);
            }
        }
    }
    // Fallback to message content if no raw_result found
    truncate_content(&message.content, TOOL_CONTENT_MAX_LENGTH * 2)
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
