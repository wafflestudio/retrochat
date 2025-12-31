use std::collections::HashMap;
use uuid::Uuid;

use crate::models::{DetectedTurn, Message, MessageRole, MessageType, ToolOperation};

/// Turn boundary detection service.
///
/// A turn starts with a User message (SimpleMessage or SlashCommand) and includes
/// all following messages until the next User message.
///
/// Rules:
/// - User + SimpleMessage -> starts new turn
/// - User + SlashCommand -> starts new turn
/// - Assistant + * -> part of current turn
/// - System + * -> part of current turn
/// - If session starts with non-User message -> turn_number = 0 (system-initiated)
pub struct TurnDetector;

/// Builder for constructing a DetectedTurn during detection
struct TurnBuilder {
    session_id: Uuid,
    turn_number: i32,
    messages: Vec<Message>,
    user_message_id: Option<Uuid>,
}

impl TurnBuilder {
    fn new_user_turn(session_id: Uuid, turn_number: i32, first_message: &Message) -> Self {
        Self {
            session_id,
            turn_number,
            messages: vec![first_message.clone()],
            user_message_id: Some(first_message.id),
        }
    }

    fn new_system_turn(session_id: Uuid) -> Self {
        Self {
            session_id,
            turn_number: 0,
            messages: Vec::new(),
            user_message_id: None,
        }
    }

    fn add_message(&mut self, message: &Message) {
        self.messages.push(message.clone());
    }

    fn build(self, tool_ops: &[ToolOperation]) -> DetectedTurn {
        if self.messages.is_empty() {
            return DetectedTurn::new(self.session_id, self.turn_number, chrono::Utc::now());
        }

        let start_sequence = self
            .messages
            .first()
            .map(|m| m.sequence_number as i32)
            .unwrap_or(0);
        let end_sequence = self
            .messages
            .last()
            .map(|m| m.sequence_number as i32)
            .unwrap_or(0);
        let started_at = self
            .messages
            .first()
            .map(|m| m.timestamp)
            .unwrap_or_else(chrono::Utc::now);
        let ended_at = self
            .messages
            .last()
            .map(|m| m.timestamp)
            .unwrap_or(started_at);

        // Count messages by role
        let user_count = self
            .messages
            .iter()
            .filter(|m| m.role == MessageRole::User)
            .count() as i32;
        let assistant_count = self
            .messages
            .iter()
            .filter(|m| m.role == MessageRole::Assistant)
            .count() as i32;
        let system_count = self
            .messages
            .iter()
            .filter(|m| m.role == MessageRole::System)
            .count() as i32;

        // Count messages by type
        let simple_count = self
            .messages
            .iter()
            .filter(|m| m.message_type == MessageType::SimpleMessage)
            .count() as i32;
        let tool_request_count = self
            .messages
            .iter()
            .filter(|m| m.message_type == MessageType::ToolRequest)
            .count() as i32;
        let tool_result_count = self
            .messages
            .iter()
            .filter(|m| m.message_type == MessageType::ToolResult)
            .count() as i32;
        let thinking_count = self
            .messages
            .iter()
            .filter(|m| m.message_type == MessageType::Thinking)
            .count() as i32;
        let slash_command_count = self
            .messages
            .iter()
            .filter(|m| m.message_type == MessageType::SlashCommand)
            .count() as i32;

        // Token metrics
        let total_tokens: i32 = self
            .messages
            .iter()
            .filter_map(|m| m.token_count)
            .map(|t| t as i32)
            .sum();
        let user_tokens: i32 = self
            .messages
            .iter()
            .filter(|m| m.role == MessageRole::User)
            .filter_map(|m| m.token_count)
            .map(|t| t as i32)
            .sum();
        let assistant_tokens: i32 = self
            .messages
            .iter()
            .filter(|m| m.role == MessageRole::Assistant)
            .filter_map(|m| m.token_count)
            .map(|t| t as i32)
            .sum();

        // Get tool operation IDs from messages in this turn
        let turn_tool_op_ids: std::collections::HashSet<Uuid> = self
            .messages
            .iter()
            .filter_map(|m| m.tool_operation_id)
            .collect();

        // Filter tool operations to those in this turn
        let turn_tool_ops: Vec<&ToolOperation> = tool_ops
            .iter()
            .filter(|op| turn_tool_op_ids.contains(&op.id))
            .collect();

        // Tool metrics
        let tool_call_count = turn_tool_ops.len() as i32;
        let tool_success_count = turn_tool_ops
            .iter()
            .filter(|op| op.success == Some(true))
            .count() as i32;
        let tool_error_count = turn_tool_ops
            .iter()
            .filter(|op| op.success == Some(false))
            .count() as i32;

        // Tool usage breakdown
        let mut tool_usage: HashMap<String, i32> = HashMap::new();
        for op in &turn_tool_ops {
            *tool_usage.entry(op.tool_name.clone()).or_insert(0) += 1;
        }

        // File metrics
        let mut files_read: Vec<String> = Vec::new();
        let mut files_written: Vec<String> = Vec::new();
        let mut files_modified: Vec<String> = Vec::new();
        let mut total_lines_added = 0;
        let mut total_lines_removed = 0;

        for op in &turn_tool_ops {
            if let Some(ref file_meta) = op.file_metadata {
                match op.tool_name.as_str() {
                    "Read" => {
                        if !files_read.contains(&file_meta.file_path) {
                            files_read.push(file_meta.file_path.clone());
                        }
                    }
                    "Write" => {
                        if !files_written.contains(&file_meta.file_path) {
                            files_written.push(file_meta.file_path.clone());
                        }
                        total_lines_added += file_meta.lines_added.unwrap_or(0);
                    }
                    "Edit" => {
                        if !files_modified.contains(&file_meta.file_path) {
                            files_modified.push(file_meta.file_path.clone());
                        }
                        total_lines_added += file_meta.lines_added.unwrap_or(0);
                        total_lines_removed += file_meta.lines_removed.unwrap_or(0);
                    }
                    _ => {}
                }
            }
        }

        // Bash metrics
        let bash_ops: Vec<&&ToolOperation> = turn_tool_ops
            .iter()
            .filter(|op| op.tool_name == "Bash" && op.bash_metadata.is_some())
            .collect();

        let bash_command_count = bash_ops.len() as i32;
        let bash_success_count = bash_ops
            .iter()
            .filter(|op| {
                op.bash_metadata
                    .as_ref()
                    .is_some_and(|b| b.exit_code == Some(0))
            })
            .count() as i32;
        let bash_error_count = bash_ops
            .iter()
            .filter(|op| {
                op.bash_metadata
                    .as_ref()
                    .is_some_and(|b| b.exit_code.is_some_and(|c| c != 0))
            })
            .count() as i32;

        let commands_executed: Vec<String> = bash_ops
            .iter()
            .filter_map(|op| op.bash_metadata.as_ref())
            .map(|b| b.command.clone())
            .collect();

        // Content previews
        let user_preview = self
            .messages
            .iter()
            .find(|m| m.role == MessageRole::User && m.message_type == MessageType::SimpleMessage)
            .map(|m| truncate_content(&m.content, 500));

        let assistant_preview = self
            .messages
            .iter()
            .rev()
            .find(|m| {
                m.role == MessageRole::Assistant && m.message_type == MessageType::SimpleMessage
            })
            .map(|m| truncate_content(&m.content, 500));

        let mut turn = DetectedTurn::new(self.session_id, self.turn_number, started_at);

        turn = turn
            .with_boundaries(start_sequence, end_sequence)
            .with_ended_at(ended_at)
            .with_message_counts(
                self.messages.len() as i32,
                user_count,
                assistant_count,
                system_count,
            )
            .with_type_counts(
                simple_count,
                tool_request_count,
                tool_result_count,
                thinking_count,
                slash_command_count,
            )
            .with_token_metrics(
                if total_tokens > 0 {
                    Some(total_tokens)
                } else {
                    None
                },
                if user_tokens > 0 {
                    Some(user_tokens)
                } else {
                    None
                },
                if assistant_tokens > 0 {
                    Some(assistant_tokens)
                } else {
                    None
                },
            )
            .with_tool_metrics(tool_call_count, tool_success_count, tool_error_count)
            .with_tool_usage(tool_usage)
            .with_file_lists(files_read, files_written, files_modified)
            .with_line_metrics(total_lines_added, total_lines_removed)
            .with_bash_metrics(
                bash_command_count,
                bash_success_count,
                bash_error_count,
                commands_executed,
            )
            .with_previews(user_preview, assistant_preview);

        if let Some(user_msg_id) = self.user_message_id {
            turn = turn.with_user_message_id(user_msg_id);
        }

        turn
    }
}

/// Helper function to truncate content to max_len characters
fn truncate_content(content: &str, max_len: usize) -> String {
    if content.len() <= max_len {
        return content.to_string();
    }

    // Find a valid char boundary before max_len
    let mut end_idx = max_len.min(content.len());
    while end_idx > 0 && !content.is_char_boundary(end_idx) {
        end_idx -= 1;
    }

    if end_idx == 0 {
        return String::new();
    }

    format!("{}...", &content[..end_idx])
}

impl TurnDetector {
    /// Detect turn boundaries from a list of messages.
    ///
    /// Messages should be sorted by sequence_number.
    /// Tool operations are used to extract metrics for each turn.
    pub fn detect_turns(
        session_id: Uuid,
        messages: &[Message],
        tool_ops: &[ToolOperation],
    ) -> Vec<DetectedTurn> {
        if messages.is_empty() {
            return Vec::new();
        }

        let mut turns = Vec::new();
        let mut current_builder: Option<TurnBuilder> = None;
        let mut next_turn_number = 0;

        for msg in messages {
            let is_turn_start = Self::is_turn_start(msg);

            if is_turn_start {
                // Finalize previous turn
                if let Some(builder) = current_builder.take() {
                    turns.push(builder.build(tool_ops));
                }

                // Start new turn
                current_builder = Some(TurnBuilder::new_user_turn(
                    session_id,
                    next_turn_number,
                    msg,
                ));
                next_turn_number += 1;
            } else {
                // Add to current turn or create turn 0
                if current_builder.is_none() {
                    // Session starts with non-user message -> system-initiated turn 0
                    current_builder = Some(TurnBuilder::new_system_turn(session_id));
                    next_turn_number = 1; // Next user turn will be turn 1
                }
                current_builder.as_mut().unwrap().add_message(msg);
            }
        }

        // Finalize last turn
        if let Some(builder) = current_builder {
            turns.push(builder.build(tool_ops));
        }

        turns
    }

    /// Check if a message starts a new turn.
    ///
    /// A message starts a new turn if:
    /// - Role is User AND
    /// - Type is SimpleMessage OR SlashCommand
    fn is_turn_start(message: &Message) -> bool {
        message.role == MessageRole::User
            && matches!(
                message.message_type,
                MessageType::SimpleMessage | MessageType::SlashCommand
            )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    fn create_message(
        session_id: Uuid,
        role: MessageRole,
        msg_type: MessageType,
        sequence: u32,
    ) -> Message {
        Message::new(
            session_id,
            role,
            format!("Message {sequence}"),
            Utc::now(),
            sequence,
        )
        .with_message_type(msg_type)
    }

    #[test]
    fn test_simple_turn_detection() {
        let session_id = Uuid::new_v4();

        let messages = vec![
            create_message(session_id, MessageRole::User, MessageType::SimpleMessage, 1),
            create_message(
                session_id,
                MessageRole::Assistant,
                MessageType::SimpleMessage,
                2,
            ),
            create_message(session_id, MessageRole::User, MessageType::SimpleMessage, 3),
            create_message(
                session_id,
                MessageRole::Assistant,
                MessageType::SimpleMessage,
                4,
            ),
        ];

        let turns = TurnDetector::detect_turns(session_id, &messages, &[]);

        assert_eq!(turns.len(), 2);
        assert_eq!(turns[0].turn_number, 0);
        assert_eq!(turns[0].message_count, 2);
        assert_eq!(turns[1].turn_number, 1);
        assert_eq!(turns[1].message_count, 2);
    }

    #[test]
    fn test_system_initiated_turn() {
        let session_id = Uuid::new_v4();

        // Session starts with assistant message
        let messages = vec![
            create_message(
                session_id,
                MessageRole::Assistant,
                MessageType::SimpleMessage,
                1,
            ),
            create_message(session_id, MessageRole::User, MessageType::SimpleMessage, 2),
            create_message(
                session_id,
                MessageRole::Assistant,
                MessageType::SimpleMessage,
                3,
            ),
        ];

        let turns = TurnDetector::detect_turns(session_id, &messages, &[]);

        assert_eq!(turns.len(), 2);
        assert_eq!(turns[0].turn_number, 0);
        assert!(turns[0].is_system_initiated());
        assert_eq!(turns[0].message_count, 1);
        assert_eq!(turns[1].turn_number, 1);
        assert!(!turns[1].is_system_initiated());
    }

    #[test]
    fn test_slash_command_starts_turn() {
        let session_id = Uuid::new_v4();

        let messages = vec![
            create_message(session_id, MessageRole::User, MessageType::SlashCommand, 1),
            create_message(
                session_id,
                MessageRole::Assistant,
                MessageType::SimpleMessage,
                2,
            ),
        ];

        let turns = TurnDetector::detect_turns(session_id, &messages, &[]);

        assert_eq!(turns.len(), 1);
        assert_eq!(turns[0].slash_command_count, 1);
    }

    #[test]
    fn test_multiple_user_messages() {
        let session_id = Uuid::new_v4();

        // User sends multiple messages in a row
        let messages = vec![
            create_message(session_id, MessageRole::User, MessageType::SimpleMessage, 1),
            create_message(session_id, MessageRole::User, MessageType::SimpleMessage, 2),
            create_message(
                session_id,
                MessageRole::Assistant,
                MessageType::SimpleMessage,
                3,
            ),
        ];

        let turns = TurnDetector::detect_turns(session_id, &messages, &[]);

        // Each user message should start a new turn
        assert_eq!(turns.len(), 2);
        assert_eq!(turns[0].message_count, 1);
        assert_eq!(turns[1].message_count, 2);
    }

    #[test]
    fn test_tool_request_does_not_start_turn() {
        let session_id = Uuid::new_v4();

        let messages = vec![
            create_message(session_id, MessageRole::User, MessageType::SimpleMessage, 1),
            create_message(
                session_id,
                MessageRole::Assistant,
                MessageType::ToolRequest,
                2,
            ),
            create_message(session_id, MessageRole::System, MessageType::ToolResult, 3),
            create_message(
                session_id,
                MessageRole::Assistant,
                MessageType::SimpleMessage,
                4,
            ),
        ];

        let turns = TurnDetector::detect_turns(session_id, &messages, &[]);

        assert_eq!(turns.len(), 1);
        assert_eq!(turns[0].message_count, 4);
        assert_eq!(turns[0].tool_request_count, 1);
        assert_eq!(turns[0].tool_result_count, 1);
    }

    #[test]
    fn test_thinking_messages() {
        let session_id = Uuid::new_v4();

        let messages = vec![
            create_message(session_id, MessageRole::User, MessageType::SimpleMessage, 1),
            create_message(session_id, MessageRole::Assistant, MessageType::Thinking, 2),
            create_message(
                session_id,
                MessageRole::Assistant,
                MessageType::SimpleMessage,
                3,
            ),
        ];

        let turns = TurnDetector::detect_turns(session_id, &messages, &[]);

        assert_eq!(turns.len(), 1);
        assert_eq!(turns[0].thinking_count, 1);
    }

    #[test]
    fn test_empty_messages() {
        let session_id = Uuid::new_v4();
        let turns = TurnDetector::detect_turns(session_id, &[], &[]);
        assert!(turns.is_empty());
    }

    #[test]
    fn test_boundaries() {
        let session_id = Uuid::new_v4();

        let messages = vec![
            create_message(session_id, MessageRole::User, MessageType::SimpleMessage, 5),
            create_message(
                session_id,
                MessageRole::Assistant,
                MessageType::SimpleMessage,
                6,
            ),
            create_message(
                session_id,
                MessageRole::Assistant,
                MessageType::SimpleMessage,
                7,
            ),
        ];

        let turns = TurnDetector::detect_turns(session_id, &messages, &[]);

        assert_eq!(turns.len(), 1);
        assert_eq!(turns[0].start_sequence, 5);
        assert_eq!(turns[0].end_sequence, 7);
    }

    #[test]
    fn test_message_role_counts() {
        let session_id = Uuid::new_v4();

        let messages = vec![
            create_message(session_id, MessageRole::User, MessageType::SimpleMessage, 1),
            create_message(
                session_id,
                MessageRole::Assistant,
                MessageType::ToolRequest,
                2,
            ),
            create_message(session_id, MessageRole::System, MessageType::ToolResult, 3),
            create_message(
                session_id,
                MessageRole::Assistant,
                MessageType::SimpleMessage,
                4,
            ),
        ];

        let turns = TurnDetector::detect_turns(session_id, &messages, &[]);

        assert_eq!(turns[0].user_message_count, 1);
        assert_eq!(turns[0].assistant_message_count, 2);
        assert_eq!(turns[0].system_message_count, 1);
    }

    #[test]
    fn test_truncate_content() {
        assert_eq!(truncate_content("hello", 10), "hello");
        assert_eq!(truncate_content("hello world", 5), "hello...");
        assert_eq!(truncate_content("", 10), "");

        // Test with multi-byte characters
        // Korean characters are 3 bytes each. "안녕하세요" = 5 chars = 15 bytes
        // Truncating at 6 bytes will give us 2 Korean chars (6 bytes) + "..."
        let korean = "안녕하세요";
        let truncated = truncate_content(korean, 6);
        assert!(truncated.ends_with("..."));
        // The result should be: 2 Korean chars + "..." (3 dots)
        assert_eq!(truncated, "안녕...");
    }
}
