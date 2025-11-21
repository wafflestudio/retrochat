use super::models::{
    QualitativeCategoryList, QualitativeInput, QualitativeItem, QualitativeOutput,
    QuantitativeInput, QuantitativeOutput, Rubric, RubricEvaluationSummary, RubricList,
    RubricScore,
};
use crate::models::message::MessageType;
use crate::models::{Message, MessageRole};
use crate::services::google_ai::GoogleAiClient;
use anyhow::Result;
use regex::Regex;

// =============================================================================
// AI Analysis Functions
// =============================================================================

pub async fn generate_quantitative_analysis_ai(
    quantitative_input: &QuantitativeInput,
    ai_client: &GoogleAiClient,
) -> Result<QuantitativeOutput> {
    let prompt = build_quantitative_analysis_prompt(quantitative_input);

    let analysis_request = crate::services::google_ai::models::AnalysisRequest {
        prompt,
        max_tokens: Some(2048),
        temperature: Some(0.5),
    };

    match ai_client.analytics(analysis_request).await {
        Ok(response) => parse_quantitative_response(&response.text),
        Err(e) => {
            tracing::warn!("AI analysis failed, falling back to rule-based: {}", e);
            generate_quantitative_analysis_fallback(quantitative_input)
        }
    }
}

pub async fn generate_qualitative_analysis_ai(
    qualitative_input: &QualitativeInput,
    ai_client: &GoogleAiClient,
) -> Result<QualitativeOutput> {
    let prompt = build_qualitative_analysis_prompt(qualitative_input);

    let analysis_request = crate::services::google_ai::models::AnalysisRequest {
        prompt,
        max_tokens: Some(3072),
        temperature: Some(0.7),
    };

    match ai_client.analytics(analysis_request).await {
        Ok(response) => parse_qualitative_response(&response.text),
        Err(e) => {
            tracing::warn!("AI analysis failed, falling back to rule-based: {}", e);
            generate_qualitative_analysis_fallback(qualitative_input)
        }
    }
}

// =============================================================================
// Prompt Building Functions
// =============================================================================

fn build_quantitative_analysis_prompt(input: &QuantitativeInput) -> String {
    format!(
        r#"Analytics the following development session metrics and provide quantitative scores.

## Session Metrics

### File Changes:
- Files Modified: {}
- Files Read: {}
- Lines Added: {}
- Lines Removed: {}
- Net Code Growth: {}
- Refactoring Operations: {}
- Bulk Edit Operations: {}

### Time Metrics:
- Session Duration: {:.1} minutes
- Average Session Length: {:.1} minutes
- Peak Hours: {:?}

### Token Metrics:
- Total Tokens: {}
- Input Tokens: {}
- Output Tokens: {}
- Token Efficiency: {:.2}
- Tokens per Hour: {:.1}

### Tool Usage:
- Total Operations: {}
- Successful: {}
- Failed: {}
- Success Rate: {:.1}%

## Task

Provide scores (0-100) for the following dimensions based on the metrics above:

1. **Overall Score**: General assessment of the development session
2. **Code Quality Score**: Based on refactoring ratio, code organization, and edit patterns
3. **Productivity Score**: Based on lines per hour, files modified, and time efficiency
4. **Efficiency Score**: Based on token usage, tool success rate, and resource utilization
5. **Collaboration Score**: Based on AI interaction patterns and query quality
6. **Learning Score**: Based on exploration vs modification ratio and problem-solving approach

Return ONLY a valid JSON object with this exact structure:
{{
  "overall_score": 0.0,
  "code_quality_score": 0.0,
  "productivity_score": 0.0,
  "efficiency_score": 0.0,
  "collaboration_score": 0.0,
  "learning_score": 0.0
}}

Important: Return ONLY the JSON object, no additional text or explanation."#,
        input.file_changes.total_files_modified,
        input.file_changes.total_files_read,
        input.file_changes.lines_added,
        input.file_changes.lines_removed,
        input.file_changes.net_code_growth,
        input.file_changes.refactoring_operations,
        input.file_changes.bulk_edit_operations,
        input.time_metrics.total_session_time_minutes,
        input.time_metrics.average_session_length_minutes,
        input.time_metrics.peak_hours,
        input.token_metrics.total_tokens_used,
        input.token_metrics.input_tokens,
        input.token_metrics.output_tokens,
        input.token_metrics.token_efficiency,
        input.token_metrics.tokens_per_hour,
        input.tool_usage.total_operations,
        input.tool_usage.successful_operations,
        input.tool_usage.failed_operations,
        if input.tool_usage.total_operations > 0 {
            (input.tool_usage.successful_operations as f64
                / input.tool_usage.total_operations as f64)
                * 100.0
        } else {
            0.0
        }
    )
}

fn build_qualitative_analysis_prompt(input: &QualitativeInput) -> String {
    let file_list = input
        .file_contexts
        .iter()
        .map(|f| {
            format!(
                "  - {} ({}): {}",
                f.file_path, f.file_type, f.modification_type
            )
        })
        .collect::<Vec<_>>()
        .join("\n");

    let tech_stack = input.project_context.technology_stack.join(", ");

    // Load categories from JSON to generate the schema dynamically
    let categories = QualitativeCategoryList::default_categories();

    // Build category descriptions and schema
    let category_list = categories
        .categories
        .iter()
        .enumerate()
        .map(|(i, cat)| {
            format!(
                "{}. **{}**: {} (1-3 items)",
                i + 1,
                cat.name,
                cat.description
            )
        })
        .collect::<Vec<_>>()
        .join("\n");

    // Build JSON schema from categories
    let schema = build_json_schema_from_categories(&categories);

    format!(
        r#"Analyze the following development session and provide qualitative insights.

## Session Context

### Files Worked On:
{}

### Chat Context:
- Conversation Flow: {}
- Problem-Solving Patterns: {:?}
- AI Interaction Quality: {:.1}/1.0
- Key Topics: {:?}

### Project Context:
- Project Type: {}
- Technology Stack: {}
- Project Complexity: {:.1}/1.0
- Development Stage: {}

## Task

Provide a comprehensive qualitative analysis with the following categories:

{}

Return ONLY a valid JSON object with this structure:
{}

Important: Return ONLY the JSON object, no additional text or explanation."#,
        file_list,
        input.chat_context.conversation_flow,
        input.chat_context.problem_solving_patterns,
        input.chat_context.ai_interaction_quality,
        input.chat_context.key_topics,
        input.project_context.project_type,
        tech_stack,
        input.project_context.project_complexity,
        input.project_context.development_stage,
        category_list,
        schema
    )
}

/// Build a JSON schema from the qualitative categories
fn build_json_schema_from_categories(categories: &QualitativeCategoryList) -> String {
    let mut schema = String::from("{\n  \"items\": [\n");

    for (i, cat) in categories.categories.iter().enumerate() {
        if i > 0 {
            schema.push_str(",\n");
        }

        // Build metadata fields
        let metadata_fields: Vec<String> = cat
            .metadata_schema
            .iter()
            .map(|field| {
                let value_example = match field.value_type.as_str() {
                    "number" => "0.0".to_string(),
                    "array" => "[\"string\"]".to_string(),
                    _ => "\"string\"".to_string(),
                };
                format!("        \"{}\": {}", field.key, value_example)
            })
            .collect();

        schema.push_str(&format!(
            r#"    {{
      "category_id": "{}",
      "title": "string",
      "description": "string",
      "metadata": {{
{}
      }}
    }}"#,
            cat.id,
            metadata_fields.join(",\n")
        ));
    }

    schema.push_str("\n  ]\n}");
    schema
}

// =============================================================================
// Response Parsing Functions
// =============================================================================

fn parse_quantitative_response(response_text: &str) -> Result<QuantitativeOutput> {
    // Try to extract JSON from the response
    let json_text = extract_json_from_text(response_text);

    match serde_json::from_str::<QuantitativeOutput>(&json_text) {
        Ok(output) => Ok(output),
        Err(e) => {
            tracing::warn!(
                "Failed to parse AI response as JSON: {}. Response: {}",
                e,
                response_text
            );
            // If JSON parsing fails, try to extract numbers from the text
            parse_quantitative_from_natural_text(response_text)
        }
    }
}

fn parse_qualitative_response(response_text: &str) -> Result<QualitativeOutput> {
    // Try to extract JSON from the response
    let json_text = extract_json_from_text(response_text);

    // Try to parse the new structure first
    match serde_json::from_str::<QualitativeOutput>(&json_text) {
        Ok(output) => {
            // Rubric fields will be populated separately by rubric evaluation
            Ok(output)
        }
        Err(e) => {
            tracing::warn!(
                "Failed to parse AI response as JSON: {}. Response: {}",
                e,
                response_text
            );
            // If JSON parsing fails, return with error item
            let mut output = QualitativeOutput::empty();
            output.add_item(
                QualitativeItem::new(
                    "insight",
                    "Analysis Error",
                    &format!("Failed to parse AI response: {e}"),
                )
                .with_string("category", "System")
                .with_number("confidence", 0.0),
            );
            Ok(output)
        }
    }
}

fn extract_json_from_text(text: &str) -> String {
    // Try to find JSON block between ```json and ``` (or ````json and ````)
    // Handle both ```json and ````json cases
    let json_marker = if text.contains("````json") {
        "````json"
    } else if text.contains("```json") {
        "```json"
    } else {
        ""
    };

    if !json_marker.is_empty() {
        if let Some(start) = text.find(json_marker) {
            let content_start = start + json_marker.len();
            let remaining = &text[content_start..];

            // Find the closing backticks
            let closing_marker = if json_marker.starts_with("````") {
                "````"
            } else {
                "```"
            };
            if let Some(end) = remaining.find(closing_marker) {
                return remaining[..end].trim().to_string();
            }
        }
    }

    // Try to find standalone JSON object
    if let Some(start) = text.find('{') {
        if let Some(end) = text.rfind('}') {
            return text[start..=end].trim().to_string();
        }
    }

    // If no JSON markers found, return as-is
    text.trim().to_string()
}

fn parse_quantitative_from_natural_text(text: &str) -> Result<QuantitativeOutput> {
    // Simple heuristic parsing as fallback
    // Extract numbers that appear after keywords
    let overall_score = extract_score_after_keyword(text, "overall");
    let code_quality_score = extract_score_after_keyword(text, "code quality");
    let productivity_score = extract_score_after_keyword(text, "productivity");
    let efficiency_score = extract_score_after_keyword(text, "efficiency");
    let collaboration_score = extract_score_after_keyword(text, "collaboration");
    let learning_score = extract_score_after_keyword(text, "learning");

    Ok(QuantitativeOutput {
        overall_score,
        code_quality_score,
        productivity_score,
        efficiency_score,
        collaboration_score,
        learning_score,
    })
}

fn extract_score_after_keyword(text: &str, keyword: &str) -> f64 {
    let text_lower = text.to_lowercase();
    let keyword_lower = keyword.to_lowercase();

    if let Some(pos) = text_lower.find(&keyword_lower) {
        let after_keyword = &text[pos..];
        // Look for a number in the next 50 characters
        let search_range = &after_keyword[..after_keyword.len().min(50)];

        // Find first number that could be a score (0-100)
        for word in search_range.split_whitespace() {
            if let Ok(num) = word
                .trim_matches(|c: char| !c.is_numeric() && c != '.')
                .parse::<f64>()
            {
                if (0.0..=100.0).contains(&num) {
                    return num;
                }
            }
        }
    }

    // Default score if not found
    50.0
}

// =============================================================================
// Fallback Analysis Functions (Rule-based)
// =============================================================================

pub fn generate_quantitative_analysis_fallback(
    quantitative_input: &QuantitativeInput,
) -> Result<QuantitativeOutput> {
    let overall_score = calculate_overall_score(quantitative_input);
    let code_quality_score = calculate_code_quality_score(quantitative_input);
    let productivity_score = calculate_productivity_score(quantitative_input);
    let efficiency_score = calculate_efficiency_score(quantitative_input);
    let collaboration_score = calculate_collaboration_score(quantitative_input);
    let learning_score = calculate_learning_score(quantitative_input);

    Ok(QuantitativeOutput {
        overall_score,
        code_quality_score,
        productivity_score,
        efficiency_score,
        collaboration_score,
        learning_score,
    })
}

pub fn generate_qualitative_analysis_fallback(
    qualitative_input: &QualitativeInput,
) -> Result<QualitativeOutput> {
    let mut items = Vec::new();

    // Generate items for each category
    items.extend(generate_insight_items(qualitative_input));
    items.extend(generate_good_pattern_items(qualitative_input));
    items.extend(generate_improvement_items(qualitative_input));
    items.extend(generate_recommendation_items(qualitative_input));
    items.extend(generate_learning_items(qualitative_input));

    let (rubric_scores, rubric_summary) = generate_rubric_evaluation_fallback();

    Ok(QualitativeOutput {
        items,
        rubric_scores,
        rubric_summary: Some(rubric_summary),
    })
}

// =============================================================================
// Scoring Functions
// =============================================================================

fn calculate_overall_score(input: &QuantitativeInput) -> f64 {
    let code_quality = calculate_code_quality_score(input);
    let productivity = calculate_productivity_score(input);
    let efficiency = calculate_efficiency_score(input);

    (code_quality + productivity + efficiency) / 3.0
}

fn calculate_code_quality_score(input: &QuantitativeInput) -> f64 {
    let refactoring_ratio = if input.file_changes.total_files_modified > 0 {
        input.file_changes.refactoring_operations as f64
            / input.file_changes.total_files_modified as f64
    } else {
        0.0
    };

    let net_growth_positive = if input.file_changes.net_code_growth > 0 {
        1.0
    } else {
        0.0
    };

    // Score based on refactoring ratio and positive code growth
    (refactoring_ratio * 50.0 + net_growth_positive * 30.0 + 20.0).min(100.0)
}

fn calculate_productivity_score(input: &QuantitativeInput) -> f64 {
    let lines_per_hour = if input.time_metrics.total_session_time_minutes > 0.0 {
        input.file_changes.lines_added as f64
            / (input.time_metrics.total_session_time_minutes / 60.0)
    } else {
        0.0
    };

    let files_per_hour = if input.time_metrics.total_session_time_minutes > 0.0 {
        input.file_changes.total_files_modified as f64
            / (input.time_metrics.total_session_time_minutes / 60.0)
    } else {
        0.0
    };

    // Score based on lines and files per hour
    ((lines_per_hour * 0.5 + files_per_hour * 0.5) * 2.0).min(100.0)
}

fn calculate_efficiency_score(input: &QuantitativeInput) -> f64 {
    let token_efficiency = input.token_metrics.token_efficiency;
    let tool_success_rate = if input.tool_usage.total_operations > 0 {
        input.tool_usage.successful_operations as f64 / input.tool_usage.total_operations as f64
    } else {
        0.0
    };

    // Score based on token efficiency and tool success rate
    (token_efficiency * 50.0 + tool_success_rate * 50.0).min(100.0)
}

fn calculate_collaboration_score(input: &QuantitativeInput) -> f64 {
    let message_ratio = if input.token_metrics.input_tokens > 0 {
        input.token_metrics.output_tokens as f64 / input.token_metrics.input_tokens as f64
    } else {
        0.0
    };

    // Score based on balanced conversation ratio
    (message_ratio * 30.0 + 70.0).min(100.0)
}

fn calculate_learning_score(input: &QuantitativeInput) -> f64 {
    let exploration_ratio = if input.file_changes.total_files_modified > 0 {
        input.file_changes.total_files_read as f64 / input.file_changes.total_files_modified as f64
    } else {
        0.0
    };

    // Score based on file exploration vs modification
    (exploration_ratio * 40.0 + 60.0).min(100.0)
}

// =============================================================================
// Qualitative Analysis Functions (Generic Item Generation)
// =============================================================================

fn generate_insight_items(input: &QualitativeInput) -> Vec<QualitativeItem> {
    let mut items = Vec::new();

    // File modification insights
    if input.file_contexts.len() > 5 {
        items.push(
            QualitativeItem::new(
                "insight",
                "High File Activity",
                "You worked on many files during this session, showing good project organization.",
            )
            .with_string("category", "Productivity")
            .with_number("confidence", 0.8),
        );
    }

    // Code quality insights
    if input.project_context.project_complexity > 0.7 {
        items.push(
            QualitativeItem::new(
                "insight",
                "Complex Project Work",
                "You're working on a complex project, which shows advanced development skills.",
            )
            .with_string("category", "Technical")
            .with_number("confidence", 0.9),
        );
    }

    // Learning insights
    if !input.project_context.technology_stack.is_empty() {
        items.push(
            QualitativeItem::new(
                "insight",
                "Technology Exploration",
                &format!(
                    "You're working with: {}",
                    input.project_context.technology_stack.join(", ")
                ),
            )
            .with_string("category", "Learning")
            .with_number("confidence", 0.7),
        );
    }

    items
}

fn generate_good_pattern_items(input: &QualitativeInput) -> Vec<QualitativeItem> {
    let mut items = Vec::new();

    // File organization pattern
    if input.file_contexts.len() > 3 {
        items.push(
            QualitativeItem::new(
                "good_pattern",
                "Modular Development",
                "You're working across multiple files, showing good modular development practices.",
            )
            .with_number("frequency", input.file_contexts.len() as f64)
            .with_string("impact", "High - improves code maintainability"),
        );
    }

    // Technology usage pattern
    if input.project_context.technology_stack.len() > 1 {
        items.push(
            QualitativeItem::new(
                "good_pattern",
                "Technology Integration",
                "You're using multiple technologies together effectively.",
            )
            .with_number("frequency", 1.0)
            .with_string("impact", "Medium - shows technical versatility"),
        );
    }

    items
}

fn generate_improvement_items(input: &QualitativeInput) -> Vec<QualitativeItem> {
    let mut items = Vec::new();

    // Code organization improvement
    if input.file_contexts.len() < 2 {
        items.push(
            QualitativeItem::new("improvement", "File Organization", "Working on single file")
                .with_string("current_state", "Working on single file")
                .with_string(
                    "suggestion",
                    "Consider breaking code into multiple files for better organization",
                )
                .with_string("priority", "Medium"),
        );
    }

    // Technology stack improvement
    if input.project_context.technology_stack.is_empty() {
        items.push(
            QualitativeItem::new(
                "improvement",
                "Technology Documentation",
                "No clear technology stack identified",
            )
            .with_string("current_state", "No clear technology stack identified")
            .with_string(
                "suggestion",
                "Document the technologies you're using for better context",
            )
            .with_string("priority", "Low"),
        );
    }

    items
}

fn generate_recommendation_items(input: &QualitativeInput) -> Vec<QualitativeItem> {
    let mut items = Vec::new();

    // General recommendations based on project complexity
    if input.project_context.project_complexity > 0.8 {
        items.push(
            QualitativeItem::new(
                "recommendation",
                "Consider Code Documentation",
                "For complex projects, consider adding more documentation to help with future maintenance.",
            )
            .with_number("impact_score", 0.8)
            .with_string("difficulty", "Medium"),
        );
    }

    // Learning recommendations
    if input.project_context.technology_stack.len() > 2 {
        items.push(
            QualitativeItem::new(
                "recommendation",
                "Deep Dive into Technologies",
                "You're using multiple technologies. Consider focusing on mastering one or two core technologies.",
            )
            .with_number("impact_score", 0.7)
            .with_string("difficulty", "Low"),
        );
    }

    items
}

fn generate_learning_items(input: &QualitativeInput) -> Vec<QualitativeItem> {
    let mut items = Vec::new();

    // Technology learning observation
    if !input.project_context.technology_stack.is_empty() {
        items.push(
            QualitativeItem::new(
                "learning",
                "Working with multiple technologies",
                "Active exploration of technology stack",
            )
            .with_string("skill_area", "Technology Integration")
            .with_string("progress", "Active exploration")
            .with_array(
                "next_steps",
                vec!["Consider creating a technology reference guide".to_string()],
            ),
        );
    }

    // Code organization learning observation
    if input.file_contexts.len() > 3 {
        items.push(
            QualitativeItem::new(
                "learning",
                "Good file organization practices",
                "Consistent application of modular development",
            )
            .with_string("skill_area", "Code Architecture")
            .with_string("progress", "Consistent application")
            .with_array(
                "next_steps",
                vec!["Continue modular development approach".to_string()],
            ),
        );
    }

    items
}

// =============================================================================
// Rubric-Based Evaluation (LLM-as-a-judge)
// =============================================================================

/// Format messages into a chat session representation for rubric evaluation
pub fn format_messages_for_prompt(messages: &[Message]) -> String {
    let mut formatted = String::new();
    let mut turn_number = 0;

    // Group messages into user-assistant turns
    let mut i = 0;
    while i < messages.len() {
        let message = &messages[i];

        // Skip thinking and system messages for evaluation
        if message.is_thinking() || message.is_system_message() {
            i += 1;
            continue;
        }

        // Start a new turn when we see a user message
        if message.is_user_message() {
            turn_number += 1;
            formatted.push_str(&format!("\n--- Turn {} ---\n", turn_number));
        }

        // Format the message
        let role_str = match message.role {
            MessageRole::User => "[User]",
            MessageRole::Assistant => "[Assistant]",
            MessageRole::System => "[System]",
        };

        formatted.push_str(&format!("{}\n", role_str));

        // Add content (truncate if too long)
        let content = if message.content.len() > 2000 {
            format!("{}... (truncated)", &message.content[..2000])
        } else {
            message.content.clone()
        };

        // For tool operations, add a summary
        match message.message_type {
            MessageType::ToolRequest => {
                formatted.push_str(&format!("[Tool Request]\n{}\n", content));
            }
            MessageType::ToolResult => {
                // Truncate tool results more aggressively
                let result_content = if content.len() > 500 {
                    format!("{}... (truncated)", &content[..500])
                } else {
                    content
                };
                formatted.push_str(&format!("[Tool Result]\n{}\n", result_content));
            }
            _ => {
                formatted.push_str(&format!("{}\n", content));
            }
        }

        formatted.push('\n');
        i += 1;
    }

    formatted
}

/// Build a judge prompt for a specific rubric
fn build_rubric_judge_prompt(rubric: &Rubric, formatted_session: &str) -> String {
    format!(
        r#"You are an expert evaluator assessing how effectively a user interacts with an AI coding assistant.

## Evaluation Rubric

{rubric_content}

## Scoring Scale

1 - Poor: User demonstrates significant deficiencies in this area
2 - Below Average: User shows some attempt but missing important elements
3 - Average: User demonstrates adequate behavior with room for improvement
4 - Good: User demonstrates strong skills in this area
5 - Excellent: User demonstrates exceptional mastery in this area

## Chat Session to Evaluate

{session}

## Instructions

1. Read the entire chat session carefully
2. Focus ONLY on the USER's behavior and communication, not the AI's responses
3. Find specific evidence from the session that relates to this rubric
4. Assign a score from 1-5 based strictly on the scoring criteria
5. Provide 2-3 sentences of reasoning with specific evidence from the session

## Required Output Format

Respond EXACTLY in this format:
SCORE: [1-5]
REASONING: [Your 2-3 sentence explanation with specific evidence]

Example:
SCORE: 4
REASONING: The user provided clear requirements by specifying the exact functionality needed and mentioning edge cases. They could improve by providing more context about the existing codebase structure."#,
        rubric_content = rubric.format_for_prompt(),
        session = formatted_session
    )
}

/// Parse the LLM response to extract score and reasoning
fn parse_rubric_score_response(response: &str) -> (Option<f64>, String) {
    // Extract score using regex
    let score_re = Regex::new(r"SCORE:\s*(\d+(?:\.\d+)?)").unwrap();
    let score = score_re.captures(response).and_then(|caps| {
        caps.get(1)
            .and_then(|m| m.as_str().parse::<f64>().ok())
            .map(|s| s.clamp(1.0, 5.0))
    });

    // Extract reasoning using regex
    let reasoning_re = Regex::new(r"(?i)REASONING:\s*(.+?)(?:\n\n|\z)").unwrap();
    let reasoning = reasoning_re
        .captures(response)
        .and_then(|caps| caps.get(1).map(|m| m.as_str().trim().to_string()))
        .unwrap_or_default();

    (score, reasoning)
}

/// Score a session against a single rubric
pub async fn score_rubric(
    rubric: &Rubric,
    formatted_session: &str,
    ai_client: &GoogleAiClient,
) -> Result<RubricScore> {
    let prompt = build_rubric_judge_prompt(rubric, formatted_session);

    let analysis_request = crate::services::google_ai::models::AnalysisRequest {
        prompt: prompt.clone(),
        max_tokens: Some(512),
        temperature: Some(0.3), // Lower temperature for more consistent scoring
    };

    let (score, reasoning) = match ai_client.analytics(analysis_request).await {
        Ok(response) => {
            let (parsed_score, parsed_reasoning) = parse_rubric_score_response(&response.text);

            // If parsing failed, retry with explicit format instruction
            if parsed_score.is_none() {
                tracing::warn!(
                    "Failed to parse rubric score for {}, retrying...",
                    rubric.id
                );
                let retry_prompt = format!(
                    "{}\n\nIMPORTANT: Please respond EXACTLY in this format:\nSCORE: [1-5]\nREASONING: [your explanation]",
                    prompt
                );

                let retry_request = crate::services::google_ai::models::AnalysisRequest {
                    prompt: retry_prompt,
                    max_tokens: Some(512),
                    temperature: Some(0.3),
                };

                match ai_client.analytics(retry_request).await {
                    Ok(retry_response) => parse_rubric_score_response(&retry_response.text),
                    Err(_) => (None, String::new()),
                }
            } else {
                (parsed_score, parsed_reasoning)
            }
        }
        Err(e) => {
            tracing::warn!("Rubric scoring failed for {}: {}", rubric.id, e);
            (None, format!("Scoring failed: {}", e))
        }
    };

    // Default to middle score if parsing failed
    let final_score = score.unwrap_or_else(|| {
        tracing::warn!(
            "Could not parse score for rubric {}, defaulting to 3.0",
            rubric.id
        );
        3.0
    });

    let final_reasoning = if reasoning.is_empty() {
        "Unable to parse LLM response. Default score assigned.".to_string()
    } else {
        reasoning
    };

    Ok(RubricScore {
        rubric_id: rubric.id.clone(),
        rubric_name: rubric.name.clone(),
        score: final_score,
        max_score: 5.0,
        reasoning: final_reasoning,
    })
}

/// Score a session against all rubrics
pub async fn score_all_rubrics(
    messages: &[Message],
    ai_client: &GoogleAiClient,
    rubrics: Option<&RubricList>,
) -> Result<(Vec<RubricScore>, RubricEvaluationSummary)> {
    // Use provided rubrics or load defaults
    let rubric_list = match rubrics {
        Some(r) => r.clone(),
        None => RubricList::default_rubrics(),
    };

    // Format messages once for all rubrics
    let formatted_session = format_messages_for_prompt(messages);

    // Score against each rubric
    let mut scores = Vec::new();
    for rubric in &rubric_list.rubrics {
        match score_rubric(rubric, &formatted_session, ai_client).await {
            Ok(score) => scores.push(score),
            Err(e) => {
                tracing::error!("Failed to score rubric {}: {}", rubric.id, e);
                // Add a default score on error
                scores.push(RubricScore {
                    rubric_id: rubric.id.clone(),
                    rubric_name: rubric.name.clone(),
                    score: 3.0,
                    max_score: 5.0,
                    reasoning: format!("Scoring error: {}", e),
                });
            }
        }
    }

    // Calculate summary
    let total_score: f64 = scores.iter().map(|s| s.score).sum();
    let max_score: f64 = scores.iter().map(|s| s.max_score).sum();
    let percentage = if max_score > 0.0 {
        (total_score / max_score) * 100.0
    } else {
        0.0
    };

    let summary = RubricEvaluationSummary {
        total_score,
        max_score,
        percentage,
        rubrics_evaluated: scores.len(),
        rubrics_version: rubric_list.version.clone(),
    };

    Ok((scores, summary))
}

/// Generate rubric evaluation with fallback (no AI)
pub fn generate_rubric_evaluation_fallback() -> (Vec<RubricScore>, RubricEvaluationSummary) {
    let rubric_list = RubricList::default_rubrics();

    let scores: Vec<RubricScore> = rubric_list
        .rubrics
        .iter()
        .map(|rubric| RubricScore {
            rubric_id: rubric.id.clone(),
            rubric_name: rubric.name.clone(),
            score: 3.0, // Default middle score
            max_score: 5.0,
            reasoning: "Fallback evaluation - AI analysis unavailable".to_string(),
        })
        .collect();

    let total_score: f64 = scores.iter().map(|s| s.score).sum();
    let max_score: f64 = scores.iter().map(|s| s.max_score).sum();

    let summary = RubricEvaluationSummary {
        total_score,
        max_score,
        percentage: if max_score > 0.0 {
            (total_score / max_score) * 100.0
        } else {
            0.0
        },
        rubrics_evaluated: scores.len(),
        rubrics_version: rubric_list.version,
    };

    (scores, summary)
}
