use chrono::Timelike;
use std::collections::HashMap;

use super::models::{
    FileChangeMetrics, TimeConsumptionMetrics, TokenConsumptionMetrics, ToolUsageMetrics,
};
use crate::models::{ChatSession, Message, MessageRole, ToolOperation};

// =============================================================================
// File Change Metrics Calculation
// =============================================================================

pub fn calculate_file_change_metrics(tool_operations: &[ToolOperation]) -> FileChangeMetrics {
    let mut total_files_modified = 0u64;
    let mut total_files_read = 0u64;
    let mut lines_added = 0u64;
    let mut lines_removed = 0u64;

    for op in tool_operations {
        if let Some(metadata) = &op.file_metadata {
            if let Some(lines_added_val) = metadata.lines_added {
                lines_added += lines_added_val as u64;
            }
            if let Some(lines_removed_val) = metadata.lines_removed {
                lines_removed += lines_removed_val as u64;
            }
        }

        // Count file operations based on tool type
        match op.tool_name.as_str() {
            "search_replace" | "MultiEdit" | "write" => {
                total_files_modified += 1;
            }
            "read_file" | "grep" | "codebase_search" => {
                total_files_read += 1;
            }
            _ => {}
        }
    }

    let net_code_growth = lines_added as i64 - lines_removed as i64;

    FileChangeMetrics {
        total_files_modified,
        total_files_read,
        lines_added,
        lines_removed,
        net_code_growth,
    }
}

// =============================================================================
// Time Consumption Metrics Calculation
// =============================================================================

pub fn calculate_time_consumption_metrics(
    session: &ChatSession,
    messages: &[Message],
) -> TimeConsumptionMetrics {
    let session_duration =
        if let (Some(end_time), Some(start_time)) = (session.end_time, Some(session.start_time)) {
            end_time.signed_duration_since(start_time).num_minutes() as f64
        } else {
            0.0
        };

    let mut peak_hours = Vec::new();
    let mut hour_counts: HashMap<u32, u32> = HashMap::new();

    for message in messages {
        let hour = message.timestamp.hour();
        *hour_counts.entry(hour).or_insert(0) += 1;
    }

    // Find peak hours (hours with most activity)
    if let Some(max_count) = hour_counts.values().max() {
        for (hour, count) in &hour_counts {
            if count == max_count {
                peak_hours.push(*hour);
            }
        }
    }

    TimeConsumptionMetrics {
        total_session_time_minutes: session_duration,
        peak_hours,
    }
}

// =============================================================================
// Token Consumption Metrics Calculation
// =============================================================================

pub fn calculate_token_consumption_metrics(messages: &[Message]) -> TokenConsumptionMetrics {
    let mut total_tokens_used = 0u64;
    let mut input_tokens = 0u64;
    let mut output_tokens = 0u64;

    for message in messages {
        if let Some(tokens) = message.token_count {
            total_tokens_used += tokens as u64;

            match message.role {
                MessageRole::User => input_tokens += tokens as u64,
                MessageRole::Assistant => output_tokens += tokens as u64,
                MessageRole::System => input_tokens += tokens as u64,
            }
        }
    }

    let token_efficiency = if total_tokens_used > 0 {
        output_tokens as f64 / total_tokens_used as f64
    } else {
        0.0
    };

    TokenConsumptionMetrics {
        total_tokens_used,
        input_tokens,
        output_tokens,
        token_efficiency,
    }
}

// =============================================================================
// Tool Usage Metrics Calculation
// =============================================================================

pub fn calculate_tool_usage_metrics(tool_operations: &[ToolOperation]) -> ToolUsageMetrics {
    let mut tool_distribution: HashMap<String, u64> = HashMap::new();
    let mut successful_operations = 0u64;
    let mut failed_operations = 0u64;

    for op in tool_operations {
        *tool_distribution.entry(op.tool_name.clone()).or_insert(0) += 1;

        if op.success.unwrap_or(false) {
            successful_operations += 1;
        } else {
            failed_operations += 1;
        }
    }

    let total_operations = tool_operations.len() as u64;
    let average_execution_time_ms = 0.0; // TODO: Calculate from execution times

    ToolUsageMetrics {
        total_operations,
        successful_operations,
        failed_operations,
        tool_distribution,
        average_execution_time_ms,
    }
}
