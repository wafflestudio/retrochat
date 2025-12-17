use chrono::Utc;
use retrochat_core::models::{Message, MessageRole, ToolResult, ToolUse};
use retrochat_core::services::MessageGroup;
use serde_json::json;
use uuid::Uuid;

/// Helper function to create a test message
fn create_message(session_id: Uuid, role: MessageRole, content: &str, sequence: u32) -> Message {
    Message::new(session_id, role, content.to_string(), Utc::now(), sequence)
}

/// Helper function to create a test tool use
fn create_tool_use(id: &str, name: &str) -> ToolUse {
    ToolUse {
        id: id.to_string(),
        name: name.to_string(),
        input: json!({}),
        raw: json!({}),
    }
}

/// Helper function to create a test tool result
fn create_tool_result(tool_use_id: &str, content: &str, is_error: bool) -> ToolResult {
    ToolResult {
        tool_use_id: tool_use_id.to_string(),
        content: content.to_string(),
        is_error,
        details: None,
        raw: json!({}),
    }
}

#[test]
fn test_pair_tool_messages_with_separate_messages() {
    let session_id = Uuid::new_v4();

    // Message 1: User message
    let msg1 = create_message(session_id, MessageRole::User, "Run ls command", 1);

    // Message 2: Assistant with tool use
    let mut msg2 = create_message(session_id, MessageRole::Assistant, "Running command", 2);
    let tool_use = create_tool_use("tool_1", "Bash");
    msg2 = msg2.with_tool_uses(vec![tool_use]);

    // Message 3: Tool result (separate message)
    let mut msg3 = create_message(session_id, MessageRole::Assistant, "[Tool Result]", 3);
    let tool_result = create_tool_result("tool_1", "file1.txt\nfile2.txt", false);
    msg3 = msg3.with_tool_results(vec![tool_result]);

    // Message 4: Assistant response
    let msg4 = create_message(session_id, MessageRole::Assistant, "Here are the files", 4);

    let messages = vec![msg1.clone(), msg2.clone(), msg3.clone(), msg4.clone()];
    let groups = MessageGroup::pair_tool_messages(messages);

    // Expected: [Single(msg1), ToolPair(msg2, msg3), Single(msg4)]
    assert_eq!(groups.len(), 3);

    match &groups[0] {
        MessageGroup::Single(m) => assert_eq!(m.sequence_number, 1),
        _ => panic!("Expected Single for first message"),
    }

    match &groups[1] {
        MessageGroup::ToolPair {
            tool_use_message,
            tool_result_message,
        } => {
            assert_eq!(tool_use_message.sequence_number, 2);
            assert_eq!(tool_result_message.sequence_number, 3);
        }
        _ => panic!("Expected ToolPair for messages 2 and 3"),
    }

    match &groups[2] {
        MessageGroup::Single(m) => assert_eq!(m.sequence_number, 4),
        _ => panic!("Expected Single for last message"),
    }
}

#[test]
fn test_pair_tool_messages_with_same_message() {
    let session_id = Uuid::new_v4();

    // Message with both tool use and tool result (already paired in the message)
    let mut msg = create_message(session_id, MessageRole::Assistant, "Running command", 1);
    let tool_use = create_tool_use("tool_1", "Bash");
    let tool_result = create_tool_result("tool_1", "output", false);
    msg = msg.with_tool_uses(vec![tool_use]);
    msg = msg.with_tool_results(vec![tool_result]);

    let messages = vec![msg.clone()];
    let groups = MessageGroup::pair_tool_messages(messages);

    // Expected: [Single(msg)] - already paired within the message
    assert_eq!(groups.len(), 1);
    match &groups[0] {
        MessageGroup::Single(_) => {} // Expected
        _ => panic!("Expected Single for message with both tool_use and tool_result"),
    }
}

#[test]
fn test_pair_tool_messages_no_matching_tool_result() {
    let session_id = Uuid::new_v4();

    // Message with tool use but no matching tool result
    let mut msg1 = create_message(session_id, MessageRole::Assistant, "Running command", 1);
    let tool_use = create_tool_use("tool_1", "Bash");
    msg1 = msg1.with_tool_uses(vec![tool_use]);

    // Next message has tool result with different ID
    let mut msg2 = create_message(session_id, MessageRole::Assistant, "[Tool Result]", 2);
    let tool_result = create_tool_result("tool_2", "output", false);
    msg2 = msg2.with_tool_results(vec![tool_result]);

    let messages = vec![msg1.clone(), msg2.clone()];
    let groups = MessageGroup::pair_tool_messages(messages);

    // Expected: [Single(msg1), Single(msg2)] - IDs don't match
    assert_eq!(groups.len(), 2);
    match &groups[0] {
        MessageGroup::Single(m) => assert_eq!(m.sequence_number, 1),
        _ => panic!("Expected Single for first message"),
    }
    match &groups[1] {
        MessageGroup::Single(m) => assert_eq!(m.sequence_number, 2),
        _ => panic!("Expected Single for second message"),
    }
}

#[test]
fn test_pair_tool_messages_multiple_pairs() {
    let session_id = Uuid::new_v4();

    // First pair
    let mut msg1 = create_message(session_id, MessageRole::Assistant, "Running ls", 1);
    msg1 = msg1.with_tool_uses(vec![create_tool_use("tool_1", "Bash")]);

    let mut msg2 = create_message(session_id, MessageRole::Assistant, "[Tool Result]", 2);
    msg2 = msg2.with_tool_results(vec![create_tool_result("tool_1", "file1.txt", false)]);

    // Second pair
    let mut msg3 = create_message(session_id, MessageRole::Assistant, "Running pwd", 3);
    msg3 = msg3.with_tool_uses(vec![create_tool_use("tool_2", "Bash")]);

    let mut msg4 = create_message(session_id, MessageRole::Assistant, "[Tool Result]", 4);
    msg4 = msg4.with_tool_results(vec![create_tool_result("tool_2", "/home/user", false)]);

    let messages = vec![msg1, msg2, msg3, msg4];
    let groups = MessageGroup::pair_tool_messages(messages);

    // Expected: [ToolPair(1,2), ToolPair(3,4)]
    assert_eq!(groups.len(), 2);

    for (i, group) in groups.iter().enumerate() {
        match group {
            MessageGroup::ToolPair { .. } => {} // Expected
            _ => panic!("Expected ToolPair for pair {}", i + 1),
        }
    }
}

#[test]
fn test_pair_tool_messages_empty_list() {
    let messages: Vec<Message> = vec![];
    let groups = MessageGroup::pair_tool_messages(messages);
    assert_eq!(groups.len(), 0);
}

#[test]
fn test_pair_tool_messages_no_tools() {
    let session_id = Uuid::new_v4();

    let msg1 = create_message(session_id, MessageRole::User, "Hello", 1);
    let msg2 = create_message(session_id, MessageRole::Assistant, "Hi there", 2);

    let messages = vec![msg1, msg2];
    let groups = MessageGroup::pair_tool_messages(messages);

    // Expected: [Single(msg1), Single(msg2)]
    assert_eq!(groups.len(), 2);
    for group in &groups {
        match group {
            MessageGroup::Single(_) => {} // Expected
            _ => panic!("Expected all Single for messages without tools"),
        }
    }
}
