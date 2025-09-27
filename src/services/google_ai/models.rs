use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct GenerateContentRequest {
    pub contents: Vec<Content>,
    pub generation_config: Option<GenerationConfig>,
    pub safety_settings: Option<Vec<SafetySetting>>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Content {
    pub parts: Vec<Part>,
    pub role: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(untagged)]
pub enum Part {
    Text { text: String },
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct GenerationConfig {
    pub temperature: Option<f32>,
    #[serde(rename = "maxOutputTokens")]
    pub max_output_tokens: Option<u32>,
    #[serde(rename = "topP")]
    pub top_p: Option<f32>,
    #[serde(rename = "topK")]
    pub top_k: Option<u32>,
    #[serde(rename = "candidateCount")]
    pub candidate_count: Option<u32>,
    #[serde(rename = "stopSequences")]
    pub stop_sequences: Option<Vec<String>>,
}

impl Default for GenerationConfig {
    fn default() -> Self {
        Self {
            temperature: Some(0.7),
            max_output_tokens: Some(2048),
            top_p: Some(0.8),
            top_k: Some(40),
            candidate_count: Some(1),
            stop_sequences: None,
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SafetySetting {
    pub category: String,
    pub threshold: String,
}

impl Default for SafetySetting {
    fn default() -> Self {
        Self {
            category: "HARM_CATEGORY_DANGEROUS_CONTENT".to_string(),
            threshold: "BLOCK_MEDIUM_AND_ABOVE".to_string(),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct GenerateContentResponse {
    pub candidates: Vec<Candidate>,
    #[serde(rename = "usageMetadata")]
    pub usage_metadata: Option<UsageMetadata>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Candidate {
    pub content: Content,
    #[serde(rename = "finishReason")]
    pub finish_reason: Option<String>,
    #[serde(rename = "safetyRatings")]
    pub safety_ratings: Option<Vec<SafetyRating>>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SafetyRating {
    pub category: String,
    pub probability: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct UsageMetadata {
    #[serde(rename = "promptTokenCount")]
    pub prompt_token_count: Option<u32>,
    #[serde(rename = "candidatesTokenCount")]
    pub candidates_token_count: Option<u32>,
    #[serde(rename = "totalTokenCount")]
    pub total_token_count: Option<u32>,
}

impl GenerateContentRequest {
    pub fn new(text: String) -> Self {
        Self {
            contents: vec![Content {
                parts: vec![Part::Text { text }],
                role: Some("user".to_string()),
            }],
            generation_config: Some(GenerationConfig::default()),
            safety_settings: Some(vec![
                SafetySetting::default(),
                SafetySetting {
                    category: "HARM_CATEGORY_HARASSMENT".to_string(),
                    threshold: "BLOCK_MEDIUM_AND_ABOVE".to_string(),
                },
                SafetySetting {
                    category: "HARM_CATEGORY_HATE_SPEECH".to_string(),
                    threshold: "BLOCK_MEDIUM_AND_ABOVE".to_string(),
                },
                SafetySetting {
                    category: "HARM_CATEGORY_SEXUALLY_EXPLICIT".to_string(),
                    threshold: "BLOCK_MEDIUM_AND_ABOVE".to_string(),
                },
            ]),
        }
    }

    pub fn with_generation_config(mut self, config: GenerationConfig) -> Self {
        self.generation_config = Some(config);
        self
    }

    pub fn with_safety_settings(mut self, settings: Vec<SafetySetting>) -> Self {
        self.safety_settings = Some(settings);
        self
    }

    pub fn estimate_tokens(&self) -> u32 {
        // Simple token estimation - roughly 4 characters per token
        let total_chars: usize = self
            .contents
            .iter()
            .flat_map(|content| &content.parts)
            .map(|part| match part {
                Part::Text { text } => text.len(),
            })
            .sum();

        (total_chars / 4).max(1) as u32
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AnalysisRequest {
    pub prompt: String,
    pub max_tokens: Option<u32>,
    pub temperature: Option<f32>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AnalysisResponse {
    pub text: String,
    pub token_usage: Option<u32>,
    pub model_used: Option<String>,
    pub finish_reason: Option<String>,
}

impl GenerateContentResponse {
    pub fn extract_text(&self) -> Option<String> {
        self.candidates
            .first()?
            .content
            .parts
            .first()
            .map(|part| match part {
                Part::Text { text } => text.clone(),
            })
    }

    pub fn get_token_usage(&self) -> Option<u32> {
        self.usage_metadata
            .as_ref()
            .and_then(|meta| meta.total_token_count)
    }

    pub fn get_finish_reason(&self) -> Option<String> {
        self.candidates
            .first()
            .and_then(|candidate| candidate.finish_reason.clone())
    }

    pub fn is_blocked_by_safety(&self) -> bool {
        self.candidates
            .first()
            .and_then(|candidate| candidate.safety_ratings.as_ref())
            .map(|ratings| {
                ratings
                    .iter()
                    .any(|rating| rating.probability == "HIGH" || rating.probability == "MEDIUM")
            })
            .unwrap_or(false)
    }

    pub fn validate(&self) -> Result<(), String> {
        if self.candidates.is_empty() {
            return Err("No candidates in response".to_string());
        }

        let candidate = &self.candidates[0];
        if candidate.content.parts.is_empty() {
            return Err("No content parts in response".to_string());
        }

        if let Some(finish_reason) = &candidate.finish_reason {
            match finish_reason.as_str() {
                "STOP" => Ok(()),
                "MAX_TOKENS" => Ok(()), // Acceptable - just reached token limit
                "SAFETY" => Err("Response blocked by safety filters".to_string()),
                "RECITATION" => Err("Response blocked due to recitation".to_string()),
                "OTHER" => Err("Response generation stopped for unknown reason".to_string()),
                reason => Err(format!("Unexpected finish reason: {}", reason)),
            }
        } else {
            Ok(())
        }
    }
}
