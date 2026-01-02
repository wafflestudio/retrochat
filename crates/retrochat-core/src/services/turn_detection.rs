use anyhow::{Context, Result as AnyhowResult};

use uuid::Uuid;

use crate::database::{DatabaseManager, MessageRepository};
use crate::models::message::MessageType;
use crate::models::{DetectedTurn, Message, MessageRole};

/// Service for detecting turn boundaries within chat sessions
///
/// A turn is defined as starting with a User message and includes all following messages
/// until the next User message.
pub struct TurnDetector {
    message_repo: MessageRepository,
}

impl TurnDetector {
    pub fn new(db: &DatabaseManager) -> Self {
        Self {
            message_repo: MessageRepository::new(db),
        }
    }

    /// Detect all turns in a session
    ///
    /// Turn boundary rules:
    /// - New turn starts with User message (SimpleMessage or SlashCommand)
    /// - Merge consecutive User messages into single turn
    /// - Handle edge case: session starting with Assistant (turn_number = 0)
    pub async fn detect_turns(&self, session_id: &Uuid) -> AnyhowResult<Vec<DetectedTurn>> {
        let messages = self
            .message_repo
            .get_by_session(session_id)
            .await
            .context("Failed to fetch messages for session")?;

        if messages.is_empty() {
            return Ok(Vec::new());
        }

        Ok(Self::detect_turns_from_messages(&messages))
    }

    /// Pure function to detect turns from a list of messages
    /// This is separated for easier testing
    fn detect_turns_from_messages(messages: &[Message]) -> Vec<DetectedTurn> {
        if messages.is_empty() {
            return Vec::new();
        }

        let mut turns = Vec::new();
        let mut current_turn_start: Option<usize> = None;
        let mut turn_number = 0;

        // Check if session starts with a non-User message (edge case)
        if !Self::is_turn_boundary(&messages[0]) {
            // Create turn 0 for system-initiated conversation
            current_turn_start = Some(0);
        }

        for (i, message) in messages.iter().enumerate() {
            if Self::is_turn_boundary(message) {
                // Close previous turn if exists
                if let Some(start_idx) = current_turn_start {
                    // Don't create a turn if we're at the same position
                    // (this handles consecutive User messages)
                    if start_idx < i {
                        let turn = Self::create_turn(
                            turn_number,
                            &messages[start_idx..i],
                            messages[start_idx].sequence_number as i32,
                            messages[i - 1].sequence_number as i32,
                        );
                        turns.push(turn);
                        turn_number += 1;
                    }
                }
                current_turn_start = Some(i);
            }
        }

        // Close the last turn
        if let Some(start_idx) = current_turn_start {
            let turn = Self::create_turn(
                turn_number,
                &messages[start_idx..],
                messages[start_idx].sequence_number as i32,
                messages.last().unwrap().sequence_number as i32,
            );
            turns.push(turn);
        }

        turns
    }

    /// Check if a message starts a new turn
    fn is_turn_boundary(message: &Message) -> bool {
        matches!(message.role, MessageRole::User)
            && matches!(
                message.message_type,
                MessageType::SimpleMessage | MessageType::SlashCommand
            )
    }

    /// Create a DetectedTurn from a slice of messages
    fn create_turn(
        turn_number: i32,
        messages: &[Message],
        start_sequence: i32,
        end_sequence: i32,
    ) -> DetectedTurn {
        let started_at = messages.first().map(|m| m.timestamp).unwrap_or_default();
        let ended_at = messages.last().map(|m| m.timestamp).unwrap_or(started_at);

        DetectedTurn::new(
            turn_number,
            start_sequence,
            end_sequence,
            started_at,
            ended_at,
        )
    }
}

/// Metrics computed on-demand for a turn
#[derive(Debug, Clone, Default)]
pub struct TurnMetrics {
    pub message_count: i32,
    pub user_message_count: i32,
    pub assistant_message_count: i32,
    pub system_message_count: i32,
    pub tool_request_count: i32,
    pub tool_result_count: i32,
    pub thinking_count: i32,
    pub total_tokens: i64,
}

impl TurnMetrics {
    /// Compute metrics for a turn from its messages
    pub fn from_messages(messages: &[Message]) -> Self {
        let mut metrics = Self::default();

        for message in messages {
            metrics.message_count += 1;

            match message.role {
                MessageRole::User => metrics.user_message_count += 1,
                MessageRole::Assistant => metrics.assistant_message_count += 1,
                MessageRole::System => metrics.system_message_count += 1,
            }

            match message.message_type {
                MessageType::ToolRequest => metrics.tool_request_count += 1,
                MessageType::ToolResult => metrics.tool_result_count += 1,
                MessageType::Thinking => metrics.thinking_count += 1,
                _ => {}
            }

            if let Some(tokens) = message.token_count {
                metrics.total_tokens += tokens as i64;
            }
        }

        metrics
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use uuid::Uuid;

    fn create_message(
        session_id: Uuid,
        sequence: u32,
        role: MessageRole,
        message_type: MessageType,
    ) -> Message {
        Message::new(
            session_id,
            role,
            format!("Message {sequence}"),
            Utc::now(),
            sequence,
        )
        .with_message_type(message_type)
    }

    #[test]
    fn test_empty_messages() {
        let turns = TurnDetector::detect_turns_from_messages(&[]);
        assert!(turns.is_empty());
    }

    #[test]
    fn test_single_user_turn() {
        let session_id = Uuid::new_v4();
        let messages = vec![
            create_message(session_id, 1, MessageRole::User, MessageType::SimpleMessage),
            create_message(
                session_id,
                2,
                MessageRole::Assistant,
                MessageType::SimpleMessage,
            ),
            create_message(
                session_id,
                3,
                MessageRole::Assistant,
                MessageType::ToolRequest,
            ),
            create_message(session_id, 4, MessageRole::System, MessageType::ToolResult),
        ];

        let turns = TurnDetector::detect_turns_from_messages(&messages);
        assert_eq!(turns.len(), 1);
        assert_eq!(turns[0].turn_number, 0);
        assert_eq!(turns[0].start_sequence, 1);
        assert_eq!(turns[0].end_sequence, 4);
        assert_eq!(turns[0].message_count(), 4);
    }

    #[test]
    fn test_multiple_turns() {
        let session_id = Uuid::new_v4();
        let messages = vec![
            // Turn 0
            create_message(session_id, 1, MessageRole::User, MessageType::SimpleMessage),
            create_message(
                session_id,
                2,
                MessageRole::Assistant,
                MessageType::SimpleMessage,
            ),
            // Turn 1
            create_message(session_id, 3, MessageRole::User, MessageType::SimpleMessage),
            create_message(
                session_id,
                4,
                MessageRole::Assistant,
                MessageType::SimpleMessage,
            ),
            create_message(
                session_id,
                5,
                MessageRole::Assistant,
                MessageType::ToolRequest,
            ),
            // Turn 2
            create_message(session_id, 6, MessageRole::User, MessageType::SlashCommand),
            create_message(
                session_id,
                7,
                MessageRole::Assistant,
                MessageType::SimpleMessage,
            ),
        ];

        let turns = TurnDetector::detect_turns_from_messages(&messages);
        assert_eq!(turns.len(), 3);

        assert_eq!(turns[0].turn_number, 0);
        assert_eq!(turns[0].start_sequence, 1);
        assert_eq!(turns[0].end_sequence, 2);

        assert_eq!(turns[1].turn_number, 1);
        assert_eq!(turns[1].start_sequence, 3);
        assert_eq!(turns[1].end_sequence, 5);

        assert_eq!(turns[2].turn_number, 2);
        assert_eq!(turns[2].start_sequence, 6);
        assert_eq!(turns[2].end_sequence, 7);
    }

    #[test]
    fn test_session_starts_with_assistant() {
        let session_id = Uuid::new_v4();
        let messages = vec![
            // Turn 0 (system-initiated)
            create_message(
                session_id,
                1,
                MessageRole::Assistant,
                MessageType::SimpleMessage,
            ),
            create_message(
                session_id,
                2,
                MessageRole::Assistant,
                MessageType::ToolRequest,
            ),
            // Turn 1
            create_message(session_id, 3, MessageRole::User, MessageType::SimpleMessage),
            create_message(
                session_id,
                4,
                MessageRole::Assistant,
                MessageType::SimpleMessage,
            ),
        ];

        let turns = TurnDetector::detect_turns_from_messages(&messages);
        assert_eq!(turns.len(), 2);

        assert_eq!(turns[0].turn_number, 0);
        assert_eq!(turns[0].start_sequence, 1);
        assert_eq!(turns[0].end_sequence, 2);

        assert_eq!(turns[1].turn_number, 1);
        assert_eq!(turns[1].start_sequence, 3);
        assert_eq!(turns[1].end_sequence, 4);
    }

    #[test]
    fn test_slash_command_starts_turn() {
        let session_id = Uuid::new_v4();
        let messages = vec![
            create_message(session_id, 1, MessageRole::User, MessageType::SlashCommand),
            create_message(
                session_id,
                2,
                MessageRole::Assistant,
                MessageType::SimpleMessage,
            ),
        ];

        let turns = TurnDetector::detect_turns_from_messages(&messages);
        assert_eq!(turns.len(), 1);
        assert_eq!(turns[0].start_sequence, 1);
    }

    #[test]
    fn test_tool_result_from_user_does_not_start_turn() {
        let session_id = Uuid::new_v4();
        let messages = vec![
            create_message(session_id, 1, MessageRole::User, MessageType::SimpleMessage),
            create_message(
                session_id,
                2,
                MessageRole::Assistant,
                MessageType::ToolRequest,
            ),
            // User providing tool result should not start new turn
            create_message(session_id, 3, MessageRole::User, MessageType::ToolResult),
            create_message(
                session_id,
                4,
                MessageRole::Assistant,
                MessageType::SimpleMessage,
            ),
        ];

        let turns = TurnDetector::detect_turns_from_messages(&messages);
        assert_eq!(turns.len(), 1);
        assert_eq!(turns[0].message_count(), 4);
    }

    #[test]
    fn test_turn_metrics() {
        let session_id = Uuid::new_v4();
        let messages = vec![
            create_message(session_id, 1, MessageRole::User, MessageType::SimpleMessage),
            create_message(session_id, 2, MessageRole::Assistant, MessageType::Thinking),
            create_message(
                session_id,
                3,
                MessageRole::Assistant,
                MessageType::ToolRequest,
            ),
            create_message(session_id, 4, MessageRole::System, MessageType::ToolResult),
            create_message(
                session_id,
                5,
                MessageRole::Assistant,
                MessageType::SimpleMessage,
            ),
        ];

        let metrics = TurnMetrics::from_messages(&messages);

        assert_eq!(metrics.message_count, 5);
        assert_eq!(metrics.user_message_count, 1);
        assert_eq!(metrics.assistant_message_count, 3);
        assert_eq!(metrics.system_message_count, 1);
        assert_eq!(metrics.tool_request_count, 1);
        assert_eq!(metrics.tool_result_count, 1);
        assert_eq!(metrics.thinking_count, 1);
    }
}
