use super::metrics::{
    calculate_file_change_metrics, calculate_time_consumption_metrics,
    calculate_token_consumption_metrics, calculate_tool_usage_metrics,
};
use super::models::{
    ChatContext, FileContext, ProjectContext, QualitativeInput, QuantitativeInput,
};
use crate::models::{
    tool_operation::FileMetadata, ChatSession, Message, MessageRole, ToolOperation,
};
use anyhow::Result;

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

pub async fn collect_qualitative_data(
    tool_operations: &[ToolOperation],
    messages: &[Message],
    session: &ChatSession,
) -> Result<QualitativeInput> {
    let file_contexts = extract_file_contexts(tool_operations);
    let chat_context = extract_chat_context(messages);
    let project_context = extract_project_context(session, tool_operations);

    Ok(QualitativeInput {
        file_contexts,
        chat_context,
        project_context,
    })
}

// =============================================================================
// File Context Extraction
// =============================================================================

fn extract_file_contexts(tool_operations: &[ToolOperation]) -> Vec<FileContext> {
    let mut file_contexts = Vec::new();
    let mut processed_files = std::collections::HashSet::new();

    for op in tool_operations {
        if let Some(metadata) = &op.file_metadata {
            let file_path = &metadata.file_path;
            if !processed_files.contains(file_path) {
                processed_files.insert(file_path.clone());

                let file_type = determine_file_type(file_path);
                let modification_type = determine_modification_type(&op.tool_name);
                let content_snippet = extract_content_snippet_from_metadata(metadata);
                let complexity_indicators = analyze_complexity_indicators_from_metadata(metadata);

                file_contexts.push(FileContext {
                    file_path: file_path.clone(),
                    file_type,
                    modification_type,
                    content_snippet,
                    complexity_indicators,
                });
            }
        }
    }

    file_contexts
}

fn determine_file_type(file_path: &str) -> String {
    if let Some(extension) = std::path::Path::new(file_path).extension() {
        match extension.to_str().unwrap_or("") {
            "rs" => "Rust".to_string(),
            "js" | "ts" => "JavaScript/TypeScript".to_string(),
            "py" => "Python".to_string(),
            "java" => "Java".to_string(),
            "go" => "Go".to_string(),
            "cpp" | "cc" | "cxx" => "C++".to_string(),
            "c" => "C".to_string(),
            "html" => "HTML".to_string(),
            "css" => "CSS".to_string(),
            "json" => "JSON".to_string(),
            "yaml" | "yml" => "YAML".to_string(),
            "md" => "Markdown".to_string(),
            "sql" => "SQL".to_string(),
            "sh" | "bash" => "Shell".to_string(),
            _ => "Unknown".to_string(),
        }
    } else {
        "Unknown".to_string()
    }
}

fn determine_modification_type(tool_name: &str) -> String {
    match tool_name {
        "search_replace" => "Text Replacement".to_string(),
        "MultiEdit" => "Bulk Edit".to_string(),
        "write" => "File Creation".to_string(),
        "read_file" => "File Reading".to_string(),
        "grep" => "Text Search".to_string(),
        "codebase_search" => "Code Search".to_string(),
        _ => "Other".to_string(),
    }
}

fn extract_content_snippet_from_metadata(_metadata: &FileMetadata) -> String {
    // TODO: Extract actual content snippet from file metadata
    "Content snippet not available".to_string()
}

fn analyze_complexity_indicators_from_metadata(metadata: &FileMetadata) -> Vec<String> {
    let mut indicators = Vec::new();

    if let Some(lines_added) = metadata.lines_added {
        if lines_added > 100 {
            indicators.push("Large addition".to_string());
        }
    }

    if let Some(is_refactoring) = metadata.is_refactoring {
        if is_refactoring {
            indicators.push("Refactoring operation".to_string());
        }
    }

    indicators
}

// =============================================================================
// Chat Context Extraction
// =============================================================================

fn extract_chat_context(messages: &[Message]) -> ChatContext {
    let conversation_flow = analyze_conversation_flow(messages);
    let problem_solving_patterns = identify_problem_solving_patterns(messages);
    let ai_interaction_quality = calculate_ai_interaction_quality(messages);
    let key_topics = extract_key_topics(messages);

    ChatContext {
        conversation_flow,
        problem_solving_patterns,
        ai_interaction_quality,
        key_topics,
    }
}

fn analyze_conversation_flow(messages: &[Message]) -> String {
    let user_messages = messages
        .iter()
        .filter(|m| matches!(m.role, MessageRole::User))
        .count();
    let assistant_messages = messages
        .iter()
        .filter(|m| matches!(m.role, MessageRole::Assistant))
        .count();

    if user_messages > assistant_messages * 2 {
        "User-driven conversation with many questions".to_string()
    } else if assistant_messages > user_messages * 2 {
        "AI-driven conversation with detailed responses".to_string()
    } else {
        "Balanced conversation between user and AI".to_string()
    }
}

fn identify_problem_solving_patterns(messages: &[Message]) -> Vec<String> {
    let mut patterns = Vec::new();

    // Look for common problem-solving patterns in message content
    for message in messages {
        let content = message.content.to_lowercase();

        if content.contains("error") || content.contains("bug") || content.contains("issue") {
            patterns.push("Error debugging".to_string());
        }
        if content.contains("implement") || content.contains("create") || content.contains("build")
        {
            patterns.push("Feature implementation".to_string());
        }
        if content.contains("refactor")
            || content.contains("optimize")
            || content.contains("improve")
        {
            patterns.push("Code improvement".to_string());
        }
        if content.contains("test") || content.contains("debug") {
            patterns.push("Testing and debugging".to_string());
        }
    }

    // Remove duplicates
    patterns.sort();
    patterns.dedup();
    patterns
}

fn calculate_ai_interaction_quality(messages: &[Message]) -> f64 {
    let total_messages = messages.len() as f64;
    if total_messages == 0.0 {
        return 0.0;
    }

    let assistant_messages = messages
        .iter()
        .filter(|m| matches!(m.role, MessageRole::Assistant))
        .count() as f64;

    let user_messages = messages
        .iter()
        .filter(|m| matches!(m.role, MessageRole::User))
        .count() as f64;

    // Quality based on balanced interaction and message length
    let balance_score = if user_messages > 0.0 {
        (assistant_messages / user_messages).min(2.0) / 2.0
    } else {
        0.0
    };

    let avg_message_length =
        messages.iter().map(|m| m.content.len() as f64).sum::<f64>() / total_messages;

    let length_score = (avg_message_length / 100.0).min(1.0);

    (balance_score + length_score) / 2.0
}

fn extract_key_topics(messages: &[Message]) -> Vec<String> {
    let mut topics = Vec::new();

    // Simple keyword extraction
    let common_keywords = vec![
        "function",
        "class",
        "method",
        "variable",
        "import",
        "export",
        "database",
        "api",
        "server",
        "client",
        "frontend",
        "backend",
        "test",
        "debug",
        "error",
        "exception",
        "validation",
        "authentication",
    ];

    for message in messages {
        let content = message.content.to_lowercase();
        for keyword in &common_keywords {
            if content.contains(keyword) && !topics.contains(&keyword.to_string()) {
                topics.push(keyword.to_string());
            }
        }
    }

    topics
}

// =============================================================================
// Project Context Extraction
// =============================================================================

fn extract_project_context(
    _session: &ChatSession,
    tool_operations: &[ToolOperation],
) -> ProjectContext {
    let project_type = infer_project_type(tool_operations);
    let technology_stack = extract_technology_stack(tool_operations);
    let project_complexity = calculate_project_complexity(tool_operations);
    let development_stage = infer_development_stage(tool_operations);

    ProjectContext {
        project_type,
        technology_stack,
        project_complexity,
        development_stage,
    }
}

fn infer_project_type(tool_operations: &[ToolOperation]) -> String {
    let mut file_types = std::collections::HashSet::new();

    for op in tool_operations {
        if let Some(metadata) = &op.file_metadata {
            let file_path = &metadata.file_path;
            if let Some(extension) = std::path::Path::new(file_path).extension() {
                file_types.insert(extension.to_string_lossy().to_string());
            }
        }
    }

    if file_types.contains("rs") {
        "Rust Application".to_string()
    } else if file_types.contains("js") || file_types.contains("ts") {
        "Web Application".to_string()
    } else if file_types.contains("py") {
        "Python Application".to_string()
    } else if file_types.contains("java") {
        "Java Application".to_string()
    } else if file_types.contains("go") {
        "Go Application".to_string()
    } else {
        "Mixed Technology Project".to_string()
    }
}

fn extract_technology_stack(tool_operations: &[ToolOperation]) -> Vec<String> {
    let mut technologies = std::collections::HashSet::new();

    for op in tool_operations {
        if let Some(metadata) = &op.file_metadata {
            let file_path = &metadata.file_path;
            if let Some(extension) = std::path::Path::new(file_path).extension() {
                match extension.to_str().unwrap_or("") {
                    "rs" => {
                        technologies.insert("Rust".to_string());
                    }
                    "js" => {
                        technologies.insert("JavaScript".to_string());
                    }
                    "ts" => {
                        technologies.insert("TypeScript".to_string());
                    }
                    "py" => {
                        technologies.insert("Python".to_string());
                    }
                    "java" => {
                        technologies.insert("Java".to_string());
                    }
                    "go" => {
                        technologies.insert("Go".to_string());
                    }
                    "html" => {
                        technologies.insert("HTML".to_string());
                    }
                    "css" => {
                        technologies.insert("CSS".to_string());
                    }
                    "json" => {
                        technologies.insert("JSON".to_string());
                    }
                    "sql" => {
                        technologies.insert("SQL".to_string());
                    }
                    _ => {}
                }
            }
        }
    }

    technologies.into_iter().collect()
}

fn calculate_project_complexity(tool_operations: &[ToolOperation]) -> f64 {
    let file_count = tool_operations
        .iter()
        .filter_map(|op| op.file_metadata.as_ref())
        .map(|meta| &meta.file_path)
        .collect::<std::collections::HashSet<_>>()
        .len();

    let total_operations = tool_operations.len();
    let refactoring_ops = tool_operations
        .iter()
        .filter(|op| {
            op.file_metadata
                .as_ref()
                .and_then(|meta| meta.is_refactoring)
                .unwrap_or(false)
        })
        .count();

    // Complexity based on file count, operations, and refactoring
    let file_complexity = (file_count as f64 / 10.0).min(1.0);
    let operation_complexity = (total_operations as f64 / 50.0).min(1.0);
    let refactoring_complexity = (refactoring_ops as f64 / 10.0).min(1.0);

    (file_complexity + operation_complexity + refactoring_complexity) / 3.0
}

fn infer_development_stage(tool_operations: &[ToolOperation]) -> String {
    let refactoring_ops = tool_operations
        .iter()
        .filter(|op| {
            op.file_metadata
                .as_ref()
                .and_then(|meta| meta.is_refactoring)
                .unwrap_or(false)
        })
        .count();

    let total_ops = tool_operations.len();
    let refactoring_ratio = if total_ops > 0 {
        refactoring_ops as f64 / total_ops as f64
    } else {
        0.0
    };

    if refactoring_ratio > 0.3 {
        "Maintenance/Refactoring".to_string()
    } else if total_ops > 20 {
        "Active Development".to_string()
    } else if total_ops > 5 {
        "Initial Development".to_string()
    } else {
        "Planning/Setup".to_string()
    }
}
