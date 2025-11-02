use super::models::{
    GoodPattern, ImprovementArea, Insight, LearningObservation, QualitativeInput,
    QualitativeOutput, QuantitativeInput, QuantitativeOutput, Recommendation,
};
use crate::services::google_ai::GoogleAiClient;
use anyhow::Result;

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

    format!(
        r#"Analytics the following development session and provide qualitative insights.

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

Provide a comprehensive qualitative analysis with:

1. **Insights**: Key observations about the development patterns (2-4 insights)
2. **Good Patterns**: Positive habits and practices observed (1-3 patterns)
3. **Improvement Areas**: Areas that could be enhanced (1-3 areas)
4. **Recommendations**: Actionable suggestions for improvement (2-3 recommendations)
5. **Learning Observations**: Growth and learning indicators (1-2 observations)

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
        file_list,
        input.chat_context.conversation_flow,
        input.chat_context.problem_solving_patterns,
        input.chat_context.ai_interaction_quality,
        input.chat_context.key_topics,
        input.project_context.project_type,
        tech_stack,
        input.project_context.project_complexity,
        input.project_context.development_stage
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
        Ok(output) => Ok(output),
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
    let insights = generate_insights(qualitative_input);
    let good_patterns = generate_good_patterns(qualitative_input);
    let improvement_areas = generate_improvement_areas(qualitative_input);
    let recommendations = generate_recommendations(qualitative_input);
    let learning_observations = generate_learning_observations(qualitative_input);

    Ok(QualitativeOutput {
        insights,
        good_patterns,
        improvement_areas,
        recommendations,
        learning_observations,
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
// Qualitative Analysis Functions
// =============================================================================

fn generate_insights(input: &QualitativeInput) -> Vec<Insight> {
    let mut insights = Vec::new();

    // File modification insights
    if input.file_contexts.len() > 5 {
        insights.push(Insight {
            title: "High File Activity".to_string(),
            description:
                "You worked on many files during this session, showing good project organization."
                    .to_string(),
            category: "Productivity".to_string(),
            confidence: 0.8,
        });
    }

    // Code quality insights
    if input.project_context.project_complexity > 0.7 {
        insights.push(Insight {
            title: "Complex Project Work".to_string(),
            description:
                "You're working on a complex project, which shows advanced development skills."
                    .to_string(),
            category: "Technical".to_string(),
            confidence: 0.9,
        });
    }

    // Learning insights
    if !input.project_context.technology_stack.is_empty() {
        insights.push(Insight {
            title: "Technology Exploration".to_string(),
            description: format!(
                "You're working with: {}",
                input.project_context.technology_stack.join(", ")
            ),
            category: "Learning".to_string(),
            confidence: 0.7,
        });
    }

    insights
}

fn generate_good_patterns(input: &QualitativeInput) -> Vec<GoodPattern> {
    let mut patterns = Vec::new();

    // File organization pattern
    if input.file_contexts.len() > 3 {
        patterns.push(GoodPattern {
            pattern_name: "Modular Development".to_string(),
            description:
                "You're working across multiple files, showing good modular development practices."
                    .to_string(),
            frequency: input.file_contexts.len() as u64,
            impact: "High - improves code maintainability".to_string(),
        });
    }

    // Technology usage pattern
    if input.project_context.technology_stack.len() > 1 {
        patterns.push(GoodPattern {
            pattern_name: "Technology Integration".to_string(),
            description: "You're using multiple technologies together effectively.".to_string(),
            frequency: 1,
            impact: "Medium - shows technical versatility".to_string(),
        });
    }

    patterns
}

fn generate_improvement_areas(input: &QualitativeInput) -> Vec<ImprovementArea> {
    let mut areas = Vec::new();

    // Code organization improvement
    if input.file_contexts.len() < 2 {
        areas.push(ImprovementArea {
            area_name: "File Organization".to_string(),
            current_state: "Working on single file".to_string(),
            suggested_improvement:
                "Consider breaking code into multiple files for better organization".to_string(),
            expected_impact: "Improved maintainability and readability".to_string(),
            priority: "Medium".to_string(),
        });
    }

    // Technology stack improvement
    if input.project_context.technology_stack.is_empty() {
        areas.push(ImprovementArea {
            area_name: "Technology Documentation".to_string(),
            current_state: "No clear technology stack identified".to_string(),
            suggested_improvement: "Document the technologies you're using for better context"
                .to_string(),
            expected_impact: "Better project understanding and maintenance".to_string(),
            priority: "Low".to_string(),
        });
    }

    areas
}

fn generate_recommendations(input: &QualitativeInput) -> Vec<Recommendation> {
    let mut recommendations = Vec::new();

    // General recommendations based on project complexity
    if input.project_context.project_complexity > 0.8 {
        recommendations.push(Recommendation {
            title: "Consider Code Documentation".to_string(),
            description: "For complex projects, consider adding more documentation to help with future maintenance.".to_string(),
            impact_score: 0.8,
            implementation_difficulty: "Medium".to_string(),
        });
    }

    // Learning recommendations
    if input.project_context.technology_stack.len() > 2 {
        recommendations.push(Recommendation {
            title: "Deep Dive into Technologies".to_string(),
            description: "You're using multiple technologies. Consider focusing on mastering one or two core technologies.".to_string(),
            impact_score: 0.7,
            implementation_difficulty: "Low".to_string(),
        });
    }

    recommendations
}

fn generate_learning_observations(input: &QualitativeInput) -> Vec<LearningObservation> {
    let mut observations = Vec::new();

    // Technology learning observation
    if !input.project_context.technology_stack.is_empty() {
        observations.push(LearningObservation {
            observation: "Working with multiple technologies".to_string(),
            skill_area: "Technology Integration".to_string(),
            progress_indicator: "Active exploration".to_string(),
            next_steps: vec!["Consider creating a technology reference guide".to_string()],
        });
    }

    // Code organization learning observation
    if input.file_contexts.len() > 3 {
        observations.push(LearningObservation {
            observation: "Good file organization practices".to_string(),
            skill_area: "Code Architecture".to_string(),
            progress_indicator: "Consistent application".to_string(),
            next_steps: vec!["Continue modular development approach".to_string()],
        });
    }

    observations
}
