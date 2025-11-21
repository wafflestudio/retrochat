use super::models::{
    GoodPattern, ImprovementArea, Insight, LearningObservation, QualitativeInput,
    QualitativeOutput, QuantitativeInput, QuantitativeOutput, Recommendation, Rubric,
    RubricEvaluationSummary, RubricList, RubricScore,
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
    format!(
        r#"Analyze the following development session and provide qualitative insights.

## Full Session Transcript (JSON)

The following is a complete transcript of the user's conversation with an AI coding assistant.
Each turn includes the message content and any tool uses (file reads, writes, edits, bash commands, etc.).

```json
{}
```

## Task

Based on the complete session transcript above, provide a comprehensive qualitative analysis with:

1. **Insights**: Key observations about the development patterns, communication style, and problem-solving approach (2-4 insights)
2. **Good Patterns**: Positive habits and practices observed in how the user interacts with the AI (1-3 patterns)
3. **Improvement Areas**: Areas where the user could enhance their workflow or communication (1-3 areas)
4. **Recommendations**: Actionable suggestions for improvement (2-3 recommendations)
5. **Learning Observations**: Growth and learning indicators based on what the user was working on (1-2 observations)

Return ONLY a valid JSON object with this exact structure:
{{
  "insights": [
    {{
      "title": "string",
      "description": "string",
      "category": "string (Productivity/Technical/Learning/Collaboration)",
      "confidence": 0.0
    }}
  ],
  "good_patterns": [
    {{
      "pattern_name": "string",
      "description": "string",
      "frequency": 1,
      "impact": "string (High/Medium/Low - description)"
    }}
  ],
  "improvement_areas": [
    {{
      "area_name": "string",
      "current_state": "string",
      "suggested_improvement": "string",
      "expected_impact": "string",
      "priority": "string (High/Medium/Low)"
    }}
  ],
  "recommendations": [
    {{
      "title": "string",
      "description": "string",
      "impact_score": 0.0,
      "implementation_difficulty": "string (Easy/Medium/Hard)"
    }}
  ],
  "learning_observations": [
    {{
      "observation": "string",
      "skill_area": "string",
      "progress_indicator": "string",
      "next_steps": ["string"]
    }}
  ]
}}

Important: Return ONLY the JSON object, no additional text or explanation."#,
        input.raw_session
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

fn parse_qualitative_response(response_text: &str) -> Result<QualitativeOutput> {
    // Try to extract JSON from the response
    let json_text = extract_json_from_text(response_text);

    match serde_json::from_str::<QualitativeOutput>(&json_text) {
        Ok(output) => {
            // Rubric fields will be populated separately by rubric evaluation
            // if they are empty/None after parsing
            Ok(output)
        }
        Err(e) => {
            tracing::warn!(
                "Failed to parse AI response as JSON: {}. Response: {}",
                e,
                response_text
            );
            // If JSON parsing fails, return empty structures with error message
            Ok(QualitativeOutput {
                insights: vec![Insight {
                    title: "Analysis Error".to_string(),
                    description: format!("Failed to parse AI response: {e}"),
                    category: "System".to_string(),
                    confidence: 0.0,
                }],
                good_patterns: vec![],
                improvement_areas: vec![],
                recommendations: vec![],
                learning_observations: vec![],
                rubric_scores: vec![],
                rubric_summary: None,
            })
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
    // Parse the raw session to extract basic stats for fallback analysis
    let session_stats = parse_session_stats(&qualitative_input.raw_session);

    let insights = generate_insights_from_stats(&session_stats);
    let good_patterns = generate_good_patterns_from_stats(&session_stats);
    let improvement_areas = generate_improvement_areas_from_stats(&session_stats);
    let recommendations = generate_recommendations_from_stats(&session_stats);
    let learning_observations = generate_learning_observations_from_stats(&session_stats);
    let (rubric_scores, rubric_summary) = generate_rubric_evaluation_fallback();

    Ok(QualitativeOutput {
        insights,
        good_patterns,
        improvement_areas,
        recommendations,
        learning_observations,
        rubric_scores,
        rubric_summary: Some(rubric_summary),
    })
}

/// Basic statistics extracted from a session for fallback analysis
struct SessionStats {
    total_turns: u32,
    user_turns: u32,
    assistant_turns: u32,
    tool_uses_count: u32,
    has_tool_uses: bool,
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
// Qualitative Analysis Functions (Fallback - based on session stats)
// =============================================================================

/// Parse basic statistics from the raw session JSON for fallback analysis
fn parse_session_stats(raw_session: &str) -> SessionStats {
    use super::models::SessionTranscript;

    // Try to parse the JSON, fall back to defaults if parsing fails
    match serde_json::from_str::<SessionTranscript>(raw_session) {
        Ok(transcript) => {
            let user_turns = transcript.turns.iter().filter(|t| t.role == "user").count() as u32;
            let assistant_turns = transcript
                .turns
                .iter()
                .filter(|t| t.role == "assistant")
                .count() as u32;
            let tool_uses_count: u32 = transcript
                .turns
                .iter()
                .map(|t| t.tool_uses.len() as u32)
                .sum();

            SessionStats {
                total_turns: transcript.total_turns,
                user_turns,
                assistant_turns,
                tool_uses_count,
                has_tool_uses: tool_uses_count > 0,
            }
        }
        Err(_) => SessionStats {
            total_turns: 0,
            user_turns: 0,
            assistant_turns: 0,
            tool_uses_count: 0,
            has_tool_uses: false,
        },
    }
}

fn generate_insights_from_stats(stats: &SessionStats) -> Vec<Insight> {
    let mut insights = Vec::new();

    // Conversation activity insight
    if stats.total_turns > 5 {
        insights.push(Insight {
            title: "Active Development Session".to_string(),
            description: format!(
                "This session had {} conversation turns, indicating an engaged development process.",
                stats.total_turns
            ),
            category: "Productivity".to_string(),
            confidence: 0.8,
        });
    }

    // Tool usage insight
    if stats.has_tool_uses {
        insights.push(Insight {
            title: "Active Tool Usage".to_string(),
            description: format!(
                "The session involved {} tool operations, showing hands-on development work.",
                stats.tool_uses_count
            ),
            category: "Technical".to_string(),
            confidence: 0.9,
        });
    }

    // Balanced conversation insight
    if stats.user_turns > 0 && stats.assistant_turns > 0 {
        let ratio = stats.assistant_turns as f64 / stats.user_turns as f64;
        if (0.5..=2.0).contains(&ratio) {
            insights.push(Insight {
                title: "Balanced Collaboration".to_string(),
                description:
                    "The conversation shows a good balance between user requests and AI responses."
                        .to_string(),
                category: "Collaboration".to_string(),
                confidence: 0.7,
            });
        }
    }

    insights
}

fn generate_good_patterns_from_stats(stats: &SessionStats) -> Vec<GoodPattern> {
    let mut patterns = Vec::new();

    // Consistent engagement pattern
    if stats.total_turns > 3 {
        patterns.push(GoodPattern {
            pattern_name: "Iterative Development".to_string(),
            description:
                "Multiple conversation turns indicate an iterative approach to problem-solving."
                    .to_string(),
            frequency: stats.total_turns as u64,
            impact: "High - enables progressive refinement".to_string(),
        });
    }

    // Tool utilization pattern
    if stats.tool_uses_count > 5 {
        patterns.push(GoodPattern {
            pattern_name: "Effective Tool Utilization".to_string(),
            description: "Good use of AI tools for file operations and code manipulation."
                .to_string(),
            frequency: stats.tool_uses_count as u64,
            impact: "Medium - increases development efficiency".to_string(),
        });
    }

    patterns
}

fn generate_improvement_areas_from_stats(stats: &SessionStats) -> Vec<ImprovementArea> {
    let mut areas = Vec::new();

    // Short session improvement
    if stats.total_turns < 3 {
        areas.push(ImprovementArea {
            area_name: "Session Engagement".to_string(),
            current_state: "Brief interaction session".to_string(),
            suggested_improvement:
                "Consider more iterative conversations to explore solutions thoroughly".to_string(),
            expected_impact: "Better problem understanding and solution quality".to_string(),
            priority: "Medium".to_string(),
        });
    }

    // No tool usage improvement
    if !stats.has_tool_uses {
        areas.push(ImprovementArea {
            area_name: "Tool Utilization".to_string(),
            current_state: "No tool operations detected".to_string(),
            suggested_improvement:
                "Leverage AI tool capabilities for file operations and code editing".to_string(),
            expected_impact: "Faster development workflow".to_string(),
            priority: "Low".to_string(),
        });
    }

    areas
}

fn generate_recommendations_from_stats(stats: &SessionStats) -> Vec<Recommendation> {
    let mut recommendations = Vec::new();

    // Based on session length
    if stats.total_turns > 10 {
        recommendations.push(Recommendation {
            title: "Session Organization".to_string(),
            description: "For longer sessions, consider breaking work into smaller, focused tasks for better tracking.".to_string(),
            impact_score: 0.7,
            implementation_difficulty: "Easy".to_string(),
        });
    }

    // Based on tool usage density
    if stats.has_tool_uses && stats.tool_uses_count > stats.total_turns {
        recommendations.push(Recommendation {
            title: "Batch Operations".to_string(),
            description: "Multiple tool operations per turn detected. Consider batching related operations for efficiency.".to_string(),
            impact_score: 0.6,
            implementation_difficulty: "Medium".to_string(),
        });
    }

    // Default recommendation
    recommendations.push(Recommendation {
        title: "Continuous Learning".to_string(),
        description:
            "Review session outcomes to identify patterns and improve future AI interactions."
                .to_string(),
        impact_score: 0.8,
        implementation_difficulty: "Low".to_string(),
    });

    recommendations
}

fn generate_learning_observations_from_stats(stats: &SessionStats) -> Vec<LearningObservation> {
    let mut observations = Vec::new();

    // General development observation
    if stats.total_turns > 0 {
        observations.push(LearningObservation {
            observation: "Active engagement with AI development assistant".to_string(),
            skill_area: "AI-Assisted Development".to_string(),
            progress_indicator: if stats.tool_uses_count > 5 {
                "Advanced usage"
            } else {
                "Developing proficiency"
            }
            .to_string(),
            next_steps: vec![
                "Explore more tool capabilities".to_string(),
                "Practice iterative problem-solving".to_string(),
            ],
        });
    }

    observations
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
