use super::models::{
    AIQualitativeOutput, QualitativeEntryList, QualitativeInput, QuantitativeInput,
    QuantitativeOutput, Rubric, RubricEvaluationSummary, RubricList, RubricScore,
};
use crate::models::message::MessageType;
use crate::models::{Message, MessageRole};
use crate::services::google_ai::GoogleAiClient;
use anyhow::Result;
use regex::Regex;
use std::collections::HashMap;

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

    let response = ai_client.analytics(analysis_request).await?;
    parse_quantitative_response(&response.text)
}

pub async fn generate_qualitative_analysis_ai(
    qualitative_input: &QualitativeInput,
    ai_client: &GoogleAiClient,
    entries: Option<&QualitativeEntryList>,
) -> Result<AIQualitativeOutput> {
    // Use provided entries or load defaults
    let entry_list = match entries {
        Some(e) => e.clone(),
        None => QualitativeEntryList::default_entries(),
    };

    let prompt = build_qualitative_analysis_prompt(qualitative_input, &entry_list);

    let analysis_request = crate::services::google_ai::models::AnalysisRequest {
        prompt,
        max_tokens: Some(3072),
        temperature: Some(0.7),
    };

    let response = ai_client.analytics(analysis_request).await?;
    parse_qualitative_response(&response.text, &entry_list)
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

fn build_qualitative_analysis_prompt(
    input: &QualitativeInput,
    entry_list: &QualitativeEntryList,
) -> String {
    let entries_description = entry_list.format_for_prompt();
    let json_schema = entry_list.format_json_schema();

    format!(
        r#"Analyze the following development session and provide qualitative insights.

## Full Session Transcript (JSON)

The following is a complete transcript of the user's conversation with an AI coding assistant.
Each turn includes the message content and any tool uses (file reads, writes, edits, bash commands, etc.).

```json
{session}
```

## Task

Based on the complete session transcript above, provide a comprehensive qualitative analysis with the following categories:

{entries_description}

Return ONLY a valid JSON object with this exact structure:
{json_schema}

Important: Return ONLY the JSON object, no additional text or explanation."#,
        session = input.raw_session,
        entries_description = entries_description,
        json_schema = json_schema
    )
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

fn parse_qualitative_response(
    response_text: &str,
    entry_list: &QualitativeEntryList,
) -> Result<AIQualitativeOutput> {
    // Try to extract JSON from the response
    let json_text = extract_json_from_text(response_text);

    // Parse the response as a dynamic JSON object
    let parsed: serde_json::Value = serde_json::from_str(&json_text).map_err(|e| {
        tracing::warn!(
            "Failed to parse AI response as JSON: {}. Response: {}",
            e,
            response_text
        );
        anyhow::anyhow!("Failed to parse AI qualitative response: {}", e)
    })?;

    // Extract entries based on the entry list configuration
    let mut entries: HashMap<String, Vec<serde_json::Value>> = HashMap::new();

    if let Some(obj) = parsed.as_object() {
        for entry_def in &entry_list.entries {
            if let Some(value) = obj.get(&entry_def.key) {
                if let Some(arr) = value.as_array() {
                    entries.insert(entry_def.key.clone(), arr.clone());
                }
            }
        }
    }

    Ok(AIQualitativeOutput::new(
        entries,
        entry_list.version.clone(),
    ))
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
