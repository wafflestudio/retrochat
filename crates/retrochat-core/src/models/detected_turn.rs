use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

/// Detected turn with computed metrics (no LLM required).
/// A turn starts with a User message and includes all following messages until the next User message.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DetectedTurn {
    pub id: Uuid,
    pub session_id: Uuid,
    pub turn_number: i32, // 0-indexed within session

    // Boundaries (references to messages table)
    pub start_sequence: i32,
    pub end_sequence: i32,
    pub user_message_id: Option<Uuid>, // FK to first user message (nullable for turn 0)

    // Message metrics (computed from messages table)
    pub message_count: i32,
    pub user_message_count: i32,
    pub assistant_message_count: i32,
    pub system_message_count: i32,

    // By message type
    pub simple_message_count: i32,
    pub tool_request_count: i32,
    pub tool_result_count: i32,
    pub thinking_count: i32,
    pub slash_command_count: i32,

    // Token metrics (aggregated from messages.token_count)
    pub total_token_count: Option<i32>,
    pub user_token_count: Option<i32>,
    pub assistant_token_count: Option<i32>,

    // Tool operation metrics
    pub tool_call_count: i32,
    pub tool_success_count: i32,
    pub tool_error_count: i32,

    // Tool breakdown by name (stored as JSON)
    #[serde(default)]
    pub tool_usage: HashMap<String, i32>,

    // File metrics
    #[serde(default)]
    pub files_read: Vec<String>,
    #[serde(default)]
    pub files_written: Vec<String>,
    #[serde(default)]
    pub files_modified: Vec<String>,
    pub unique_files_touched: i32,

    // Line change metrics
    pub total_lines_added: i32,
    pub total_lines_removed: i32,
    pub total_lines_changed: i32,

    // Bash metrics
    pub bash_command_count: i32,
    pub bash_success_count: i32,
    pub bash_error_count: i32,
    #[serde(default)]
    pub commands_executed: Vec<String>,

    // Content preview
    pub user_message_preview: Option<String>,
    pub assistant_message_preview: Option<String>,

    // Timestamps
    pub started_at: DateTime<Utc>,
    pub ended_at: DateTime<Utc>,
    pub duration_seconds: Option<i64>,

    pub created_at: DateTime<Utc>,
}

impl DetectedTurn {
    pub fn new(session_id: Uuid, turn_number: i32, started_at: DateTime<Utc>) -> Self {
        Self {
            id: Uuid::new_v4(),
            session_id,
            turn_number,
            start_sequence: 0,
            end_sequence: 0,
            user_message_id: None,
            message_count: 0,
            user_message_count: 0,
            assistant_message_count: 0,
            system_message_count: 0,
            simple_message_count: 0,
            tool_request_count: 0,
            tool_result_count: 0,
            thinking_count: 0,
            slash_command_count: 0,
            total_token_count: None,
            user_token_count: None,
            assistant_token_count: None,
            tool_call_count: 0,
            tool_success_count: 0,
            tool_error_count: 0,
            tool_usage: HashMap::new(),
            files_read: Vec::new(),
            files_written: Vec::new(),
            files_modified: Vec::new(),
            unique_files_touched: 0,
            total_lines_added: 0,
            total_lines_removed: 0,
            total_lines_changed: 0,
            bash_command_count: 0,
            bash_success_count: 0,
            bash_error_count: 0,
            commands_executed: Vec::new(),
            user_message_preview: None,
            assistant_message_preview: None,
            started_at,
            ended_at: started_at,
            duration_seconds: None,
            created_at: Utc::now(),
        }
    }

    /// Builder method: set boundaries
    pub fn with_boundaries(mut self, start_sequence: i32, end_sequence: i32) -> Self {
        self.start_sequence = start_sequence;
        self.end_sequence = end_sequence;
        self
    }

    /// Builder method: set user message id
    pub fn with_user_message_id(mut self, user_message_id: Uuid) -> Self {
        self.user_message_id = Some(user_message_id);
        self
    }

    /// Builder method: set ended_at and calculate duration
    pub fn with_ended_at(mut self, ended_at: DateTime<Utc>) -> Self {
        self.ended_at = ended_at;
        self.duration_seconds = Some((ended_at - self.started_at).num_seconds());
        self
    }

    /// Builder method: set message counts
    pub fn with_message_counts(
        mut self,
        message_count: i32,
        user_count: i32,
        assistant_count: i32,
        system_count: i32,
    ) -> Self {
        self.message_count = message_count;
        self.user_message_count = user_count;
        self.assistant_message_count = assistant_count;
        self.system_message_count = system_count;
        self
    }

    /// Builder method: set message type counts
    pub fn with_type_counts(
        mut self,
        simple: i32,
        tool_request: i32,
        tool_result: i32,
        thinking: i32,
        slash_command: i32,
    ) -> Self {
        self.simple_message_count = simple;
        self.tool_request_count = tool_request;
        self.tool_result_count = tool_result;
        self.thinking_count = thinking;
        self.slash_command_count = slash_command;
        self
    }

    /// Builder method: set token metrics
    pub fn with_token_metrics(
        mut self,
        total: Option<i32>,
        user: Option<i32>,
        assistant: Option<i32>,
    ) -> Self {
        self.total_token_count = total;
        self.user_token_count = user;
        self.assistant_token_count = assistant;
        self
    }

    /// Builder method: set tool call metrics
    pub fn with_tool_metrics(
        mut self,
        call_count: i32,
        success_count: i32,
        error_count: i32,
    ) -> Self {
        self.tool_call_count = call_count;
        self.tool_success_count = success_count;
        self.tool_error_count = error_count;
        self
    }

    /// Builder method: set tool usage breakdown
    pub fn with_tool_usage(mut self, tool_usage: HashMap<String, i32>) -> Self {
        self.tool_usage = tool_usage;
        self
    }

    /// Builder method: set file lists
    pub fn with_file_lists(
        mut self,
        files_read: Vec<String>,
        files_written: Vec<String>,
        files_modified: Vec<String>,
    ) -> Self {
        // Store files first
        self.files_read = files_read;
        self.files_written = files_written;
        self.files_modified = files_modified;

        // Calculate unique files touched from stored values
        let mut all_files: std::collections::HashSet<&str> = std::collections::HashSet::new();
        for f in &self.files_read {
            all_files.insert(f);
        }
        for f in &self.files_written {
            all_files.insert(f);
        }
        for f in &self.files_modified {
            all_files.insert(f);
        }
        self.unique_files_touched = all_files.len() as i32;

        self
    }

    /// Builder method: set line change metrics
    pub fn with_line_metrics(mut self, added: i32, removed: i32) -> Self {
        self.total_lines_added = added;
        self.total_lines_removed = removed;
        self.total_lines_changed = added + removed;
        self
    }

    /// Builder method: set bash metrics
    pub fn with_bash_metrics(
        mut self,
        command_count: i32,
        success_count: i32,
        error_count: i32,
        commands: Vec<String>,
    ) -> Self {
        self.bash_command_count = command_count;
        self.bash_success_count = success_count;
        self.bash_error_count = error_count;
        self.commands_executed = commands;
        self
    }

    /// Builder method: set content previews
    pub fn with_previews(
        mut self,
        user_preview: Option<String>,
        assistant_preview: Option<String>,
    ) -> Self {
        self.user_message_preview = user_preview;
        self.assistant_message_preview = assistant_preview;
        self
    }

    /// Check if this is a system-initiated turn (turn 0 with no user message)
    pub fn is_system_initiated(&self) -> bool {
        self.turn_number == 0 && self.user_message_id.is_none()
    }

    /// Check if this turn has any tool operations
    pub fn has_tool_operations(&self) -> bool {
        self.tool_call_count > 0
    }

    /// Check if this turn has any file changes
    pub fn has_file_changes(&self) -> bool {
        self.total_lines_changed > 0
            || !self.files_written.is_empty()
            || !self.files_modified.is_empty()
    }

    /// Check if this turn has any errors
    pub fn has_errors(&self) -> bool {
        self.tool_error_count > 0 || self.bash_error_count > 0
    }

    /// Get total unique files touched
    pub fn unique_files(&self) -> std::collections::HashSet<&str> {
        let mut files = std::collections::HashSet::new();
        for f in &self.files_read {
            files.insert(f.as_str());
        }
        for f in &self.files_written {
            files.insert(f.as_str());
        }
        for f in &self.files_modified {
            files.insert(f.as_str());
        }
        files
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_detected_turn() {
        let session_id = Uuid::new_v4();
        let now = Utc::now();
        let turn = DetectedTurn::new(session_id, 1, now);

        assert_eq!(turn.session_id, session_id);
        assert_eq!(turn.turn_number, 1);
        assert_eq!(turn.message_count, 0);
        assert!(!turn.is_system_initiated());
    }

    #[test]
    fn test_system_initiated_turn() {
        let session_id = Uuid::new_v4();
        let now = Utc::now();
        let turn = DetectedTurn::new(session_id, 0, now);

        assert!(turn.is_system_initiated());
    }

    #[test]
    fn test_with_boundaries() {
        let session_id = Uuid::new_v4();
        let now = Utc::now();
        let turn = DetectedTurn::new(session_id, 1, now).with_boundaries(5, 10);

        assert_eq!(turn.start_sequence, 5);
        assert_eq!(turn.end_sequence, 10);
    }

    #[test]
    fn test_with_file_lists() {
        let session_id = Uuid::new_v4();
        let now = Utc::now();
        let turn = DetectedTurn::new(session_id, 1, now).with_file_lists(
            vec!["a.rs".to_string(), "b.rs".to_string()],
            vec!["c.rs".to_string()],
            vec!["a.rs".to_string()], // Duplicate
        );

        assert_eq!(turn.files_read.len(), 2);
        assert_eq!(turn.files_written.len(), 1);
        assert_eq!(turn.files_modified.len(), 1);
        assert_eq!(turn.unique_files_touched, 3); // a.rs, b.rs, c.rs
    }

    #[test]
    fn test_with_line_metrics() {
        let session_id = Uuid::new_v4();
        let now = Utc::now();
        let turn = DetectedTurn::new(session_id, 1, now).with_line_metrics(100, 50);

        assert_eq!(turn.total_lines_added, 100);
        assert_eq!(turn.total_lines_removed, 50);
        assert_eq!(turn.total_lines_changed, 150);
    }

    #[test]
    fn test_has_tool_operations() {
        let session_id = Uuid::new_v4();
        let now = Utc::now();
        let turn_without = DetectedTurn::new(session_id, 1, now);
        let turn_with = DetectedTurn::new(session_id, 1, now).with_tool_metrics(5, 4, 1);

        assert!(!turn_without.has_tool_operations());
        assert!(turn_with.has_tool_operations());
    }

    #[test]
    fn test_has_errors() {
        let session_id = Uuid::new_v4();
        let now = Utc::now();

        let turn_no_errors = DetectedTurn::new(session_id, 1, now);
        let turn_tool_errors = DetectedTurn::new(session_id, 1, now).with_tool_metrics(5, 4, 1);
        let turn_bash_errors =
            DetectedTurn::new(session_id, 1, now).with_bash_metrics(3, 2, 1, vec![]);

        assert!(!turn_no_errors.has_errors());
        assert!(turn_tool_errors.has_errors());
        assert!(turn_bash_errors.has_errors());
    }

    #[test]
    fn test_duration_calculation() {
        let session_id = Uuid::new_v4();
        let start = Utc::now();
        let end = start + chrono::Duration::seconds(120);
        let turn = DetectedTurn::new(session_id, 1, start).with_ended_at(end);

        assert_eq!(turn.duration_seconds, Some(120));
    }
}
