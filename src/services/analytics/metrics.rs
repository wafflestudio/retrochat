use chrono::Timelike;
use std::collections::HashMap;

use super::models::{
    FileChangeMetrics, ProcessedCodeMetrics, ProcessedTokenMetrics, SessionMetrics,
    TimeConsumptionMetrics, TimeEfficiencyMetrics, TokenConsumptionMetrics, ToolUsageMetrics,
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
    let mut refactoring_operations = 0u64;
    let mut bulk_edit_operations = 0u64;

    for op in tool_operations {
        if let Some(metadata) = &op.file_metadata {
            if let Some(lines_added_val) = metadata.lines_added {
                lines_added += lines_added_val as u64;
            }
            if let Some(lines_removed_val) = metadata.lines_removed {
                lines_removed += lines_removed_val as u64;
            }
            if let Some(is_refactoring) = metadata.is_refactoring {
                if is_refactoring {
                    refactoring_operations += 1;
                }
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

        // Count bulk operations
        if op.tool_name == "MultiEdit" {
            bulk_edit_operations += 1;
        }
    }

    let net_code_growth = lines_added as i64 - lines_removed as i64;

    FileChangeMetrics {
        total_files_modified,
        total_files_read,
        lines_added,
        lines_removed,
        net_code_growth,
        refactoring_operations,
        bulk_edit_operations,
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
        average_session_length_minutes: session_duration,
        peak_hours,
        break_duration_minutes: 0.0, // TODO: Calculate based on message gaps
        context_switching_time_minutes: 0.0, // TODO: Calculate based on tool switching
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

    let tokens_per_hour = if total_tokens_used > 0 {
        total_tokens_used as f64 / 1.0 // TODO: Calculate based on actual session duration
    } else {
        0.0
    };

    TokenConsumptionMetrics {
        total_tokens_used,
        input_tokens,
        output_tokens,
        token_efficiency,
        tokens_per_hour,
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

// =============================================================================
// Processed Metrics Calculation
// =============================================================================

pub fn calculate_processed_token_metrics(
    total_tokens: u64,
    session_duration_hours: f64,
    input_tokens: u64,
    output_tokens: u64,
) -> ProcessedTokenMetrics {
    let tokens_per_hour = if session_duration_hours > 0.0 {
        total_tokens as f64 / session_duration_hours
    } else {
        0.0
    };

    let input_output_ratio = if output_tokens > 0 {
        input_tokens as f64 / output_tokens as f64
    } else {
        0.0
    };

    let token_efficiency_score = if total_tokens > 0 {
        output_tokens as f64 / total_tokens as f64
    } else {
        0.0
    };

    // Rough cost estimate (assuming $0.002 per 1K tokens)
    let cost_estimate = total_tokens as f64 * 0.002 / 1000.0;

    ProcessedTokenMetrics {
        total_tokens,
        tokens_per_hour,
        input_output_ratio,
        token_efficiency_score,
        cost_estimate,
    }
}

pub fn calculate_processed_code_metrics(
    net_lines_changed: i64,
    files_modified: u64,
    session_duration_hours: f64,
    refactoring_operations: u64,
    total_operations: u64,
) -> ProcessedCodeMetrics {
    let files_per_session = files_modified as f64;
    let lines_per_hour = if session_duration_hours > 0.0 {
        net_lines_changed.abs() as f64 / session_duration_hours
    } else {
        0.0
    };

    let refactoring_ratio = if total_operations > 0 {
        refactoring_operations as f64 / total_operations as f64
    } else {
        0.0
    };

    let code_velocity = if session_duration_hours > 0.0 {
        net_lines_changed as f64 / session_duration_hours
    } else {
        0.0
    };

    ProcessedCodeMetrics {
        net_lines_changed,
        files_per_session,
        lines_per_hour,
        refactoring_ratio,
        code_velocity,
    }
}

pub fn calculate_time_efficiency_metrics(
    session_duration_hours: f64,
    productive_work_hours: f64,
    context_switches: u64,
) -> TimeEfficiencyMetrics {
    let productivity_score = if session_duration_hours > 0.0 {
        productive_work_hours / session_duration_hours
    } else {
        0.0
    };

    let context_switching_cost = if context_switches > 0 {
        (context_switches as f64 * 0.1).min(0.5) // Max 50% cost
    } else {
        0.0
    };

    let deep_work_ratio = if session_duration_hours > 0.0 {
        (productive_work_hours - context_switching_cost) / session_duration_hours
    } else {
        0.0
    };

    let time_utilization = productivity_score * (1.0 - context_switching_cost);

    TimeEfficiencyMetrics {
        productivity_score,
        context_switching_cost,
        deep_work_ratio,
        time_utilization,
    }
}

pub fn calculate_session_metrics(
    total_sessions: u64,
    average_duration_minutes: f64,
) -> SessionMetrics {
    // Simple consistency score based on session count
    let session_consistency_score = if total_sessions > 0 {
        (total_sessions as f64 / 10.0).min(1.0) // Normalize to 0-1
    } else {
        0.0
    };

    SessionMetrics {
        total_sessions,
        average_session_duration_minutes: average_duration_minutes,
        session_consistency_score,
    }
}
