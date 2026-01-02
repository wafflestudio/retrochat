use super::models::{
    AIQualitativeOutput, AIQuantitativeOutput, QualitativeEntry, QualitativeEntryList,
    QualitativeEntryOutput, QualitativeInput, Rubric, RubricEvaluationSummary, RubricList,
    RubricScore,
};
use crate::models::message::MessageType;
use crate::models::{Message, MessageRole};
use crate::services::llm::{GenerateRequest, LlmClient};
use anyhow::Result;
use regex::Regex;

// =============================================================================
// AI Analysis Functions
// =============================================================================

pub async fn generate_qualitative_analysis_ai(
    qualitative_input: &QualitativeInput,
    llm_client: &dyn LlmClient,
    entries: Option<&QualitativeEntryList>,
) -> Result<AIQualitativeOutput> {
    // Use provided entries or load defaults
    let entry_list = match entries {
        Some(e) => e.clone(),
        None => QualitativeEntryList::default_entries(),
    };

    // Process all entry types sequentially (can't easily pass trait object to spawned tasks)
    let mut results = Vec::new();
    for entry in &entry_list.entries {
        let result = generate_single_entry(qualitative_input, entry, llm_client).await;
        results.push((entry.clone(), result));
    }

    // Collect results into Vec<QualitativeEntryOutput>
    let mut all_entries: Vec<QualitativeEntryOutput> = Vec::new();
    for (entry, result) in results {
        match result {
            Ok(entry_output) => {
                all_entries.push(entry_output);
            }
            Err(e) => {
                tracing::warn!("Failed to generate entry {}: {}", entry.key, e);
                // Add empty entry on error
                all_entries.push(QualitativeEntryOutput {
                    key: entry.key.clone(),
                    title: entry.title.clone(),
                    description: entry.description.clone(),
                    summary: String::new(),
                    items: Vec::new(),
                });
            }
        }
    }

    Ok(AIQualitativeOutput::new(
        all_entries,
        entry_list.version.clone(),
    ))
}

/// Generate a single qualitative entry type with its own LLM request
async fn generate_single_entry(
    qualitative_input: &QualitativeInput,
    entry: &QualitativeEntry,
    llm_client: &dyn LlmClient,
) -> Result<QualitativeEntryOutput> {
    let prompt = build_single_entry_prompt(qualitative_input, entry);

    let request = GenerateRequest::new(prompt)
        .with_max_tokens(1024)
        .with_temperature(0.7);

    let response = llm_client
        .generate(request)
        .await
        .map_err(|e| anyhow::anyhow!("LLM generation failed: {e}"))?;
    parse_entry_response(&response.text, entry)
}

// =============================================================================
// Prompt Building Functions
// =============================================================================

/// Build a prompt for a single qualitative entry type
fn build_single_entry_prompt(input: &QualitativeInput, entry: &QualitativeEntry) -> String {
    format!(
        r#"Analyze the following development session and provide {title}.

## Full Session Transcript (JSON)

The following is a complete transcript of the user's conversation with an AI coding assistant.
Each turn includes the message content and any tool uses (file reads, writes, edits, bash commands, etc.).

```json
{session}
```

## Task

{entry_description}

Each item should be a single, concise markdown line that captures one specific observation.

## Required Output Format

Your response MUST follow this exact format:

SHORT_SUMMARY: [A single line (max 100 characters) summarizing the key finding for {title}]

ITEMS:
1. **Observation title**: Brief description of the observation with specific details.
2. **Another observation**: Another specific point with supporting evidence.

Example:
SHORT_SUMMARY: User demonstrated strong debugging skills but could improve test coverage practices.

ITEMS:
1. **Clear problem articulation**: User explained the bug clearly with specific error messages.
2. **Iterative approach**: User refined requirements based on initial results.

Important:
- SHORT_SUMMARY must be a single concise line (no more than 100 characters).
- Return numbered list items under ITEMS section.
- Each item must be a single line of markdown text.
- Focus on specific, actionable observations from the session."#,
        title = entry.title.to_lowercase(),
        session = input.raw_session,
        entry_description = entry.format_for_prompt(),
    )
}

// =============================================================================
// Response Parsing Functions
// =============================================================================

/// Parse the LLM response for a single entry type
/// Expects SHORT_SUMMARY and ITEMS sections
fn parse_entry_response(
    response_text: &str,
    entry: &QualitativeEntry,
) -> Result<QualitativeEntryOutput> {
    let mut items = Vec::new();
    let mut summary = String::new();

    // Parse SHORT_SUMMARY
    let summary_re = Regex::new(r"(?i)SHORT_SUMMARY:\s*(.+)").unwrap();
    if let Some(caps) = summary_re.captures(response_text) {
        if let Some(summary_match) = caps.get(1) {
            summary = summary_match.as_str().trim().to_string();
            // Truncate to 100 characters if needed
            if summary.len() > 100 {
                summary = format!("{}...", &summary[..97]);
            }
        }
    }

    // Parse numbered list items (e.g., "1. ...", "2. ...")
    let numbered_re = Regex::new(r"^\s*\d+\.\s*(.+)$").unwrap();

    // Find ITEMS section and parse from there
    let items_start = response_text.to_lowercase().find("items:").unwrap_or(0);
    let items_section = &response_text[items_start..];

    for line in items_section.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.to_lowercase().starts_with("items:") {
            continue;
        }

        // Check if it's a numbered list item
        if let Some(caps) = numbered_re.captures(trimmed) {
            if let Some(content) = caps.get(1) {
                let item = content.as_str().trim().to_string();
                if !item.is_empty() {
                    items.push(item);
                }
            }
        } else if trimmed.starts_with("- ") || trimmed.starts_with("* ") {
            // Also handle bullet points
            let item = trimmed[2..].trim().to_string();
            if !item.is_empty() {
                items.push(item);
            }
        } else if trimmed.starts_with("**") {
            // Handle lines that start with bold text (common in markdown)
            items.push(trimmed.to_string());
        }
    }

    if items.is_empty() {
        tracing::warn!(
            "No items parsed for entry {}, response: {}",
            entry.key,
            response_text
        );
    }

    if summary.is_empty() {
        tracing::warn!(
            "No summary parsed for entry {}, response: {}",
            entry.key,
            response_text
        );
    }

    Ok(QualitativeEntryOutput {
        key: entry.key.clone(),
        title: entry.title.clone(),
        description: entry.description.clone(),
        summary,
        items,
    })
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
async fn score_rubric(
    rubric: &Rubric,
    formatted_session: &str,
    llm_client: &dyn LlmClient,
) -> Result<RubricScore> {
    let prompt = build_rubric_judge_prompt(rubric, formatted_session);

    let request = GenerateRequest::new(prompt.clone())
        .with_max_tokens(512)
        .with_temperature(0.3); // Lower temperature for more consistent scoring

    let (score, reasoning) = match llm_client.generate(request).await {
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

                let retry_request = GenerateRequest::new(retry_prompt)
                    .with_max_tokens(512)
                    .with_temperature(0.3);

                match llm_client.generate(retry_request).await {
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

pub async fn generate_quantitative_analysis_ai(
    qualitative_input: &QualitativeInput,
    llm_client: &dyn LlmClient,
    rubrics: Option<&RubricList>,
) -> Result<AIQuantitativeOutput> {
    return match score_all_rubrics(qualitative_input, llm_client, rubrics).await {
        Ok((rubric_scores, rubric_summary)) => Ok(AIQuantitativeOutput {
            rubric_scores,
            rubric_summary: Some(rubric_summary),
        }),
        Err(e) => {
            tracing::warn!("Failed to generate rubric scores: {}", e);
            Err(anyhow::anyhow!("Failed to generate rubric scores: {}", e))
        }
    };
}

/// Score a session against all rubrics
async fn score_all_rubrics(
    qualitative_input: &QualitativeInput,
    llm_client: &dyn LlmClient,
    rubrics: Option<&RubricList>,
) -> Result<(Vec<RubricScore>, RubricEvaluationSummary)> {
    // Use provided rubrics or load defaults
    let rubric_list = match rubrics {
        Some(r) => r.clone(),
        None => RubricList::default_rubrics(),
    };

    // Format messages once for all rubrics
    let formatted_session = qualitative_input.raw_session.clone();

    // Score all rubrics sequentially (can't easily pass trait object to spawned tasks)
    let mut results = Vec::new();
    for rubric in &rubric_list.rubrics {
        let result = score_rubric(rubric, &formatted_session, llm_client).await;
        results.push((rubric.clone(), result));
    }

    // Collect results into Vec
    let mut scores = Vec::new();
    for (rubric, result) in results {
        match result {
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
