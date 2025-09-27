# Google AI API Contract: Gemini Integration

## API Endpoint Configuration

### Base Configuration
- **Base URL**: `https://generativelanguage.googleapis.com/v1beta`
- **Model**: `gemini-2.5-flash-lite` (optimized for low latency and cost efficiency)
- **Alternative**: `gemini-1.5-pro` (for complex analysis if needed)
- **Authentication**: API Key via `x-goog-api-key` header
- **Content-Type**: `application/json`

### Rate Limits
- **Free Tier**: 15 requests/minute, 1M tokens/minute
- **Paid Tier**: 300 requests/minute, 4M tokens/minute
- **Implementation**: Use token bucket algorithm with configurable limits

## Request/Response Contracts

### Generate Content Request
**Endpoint**: `POST /models/{model}:generateContent`

**Request Schema**:
```json
{
  "contents": [
    {
      "role": "user",
      "parts": [
        {
          "text": "Analysis prompt + formatted chat session data"
        }
      ]
    }
  ],
  "generationConfig": {
    "temperature": 0.7,
    "maxOutputTokens": 2048,
    "topP": 0.8,
    "topK": 40
  },
  "safetySettings": [
    {
      "category": "HARM_CATEGORY_DANGEROUS_CONTENT",
      "threshold": "BLOCK_MEDIUM_AND_ABOVE"
    }
  ]
}
```

**Response Schema**:
```json
{
  "candidates": [
    {
      "content": {
        "parts": [
          {
            "text": "Analysis response text"
          }
        ],
        "role": "model"
      },
      "finishReason": "STOP",
      "safetyRatings": [
        {
          "category": "HARM_CATEGORY_DANGEROUS_CONTENT",
          "probability": "NEGLIGIBLE"
        }
      ]
    }
  ],
  "usageMetadata": {
    "promptTokenCount": 1250,
    "candidatesTokenCount": 847,
    "totalTokenCount": 2097
  }
}
```

## Analysis Prompt Templates

### User Interaction Analysis
```text
Analyze this chat session between a user and an AI coding assistant. Focus on the user's communication patterns, question quality, and interaction effectiveness.

Evaluate the following aspects:
1. Communication Clarity: How clearly does the user express their needs and problems?
2. Question Quality: Are questions specific, well-structured, and provide sufficient context?
3. Follow-up Effectiveness: How well does the user iterate and build on AI responses?
4. Task Breakdown: Does the user effectively break down complex problems?
5. Collaboration Style: How effectively does the user collaborate with the AI?

Provide:
- Overall assessment (1-10 scale for each aspect)
- Specific examples of strengths and areas for improvement
- Actionable recommendations for better AI collaboration

Chat Session:
{chat_data}
```

### Collaboration Insights Analysis
```text
Analyze this coding session to identify collaboration patterns between the user and AI assistant.

Focus on:
1. Problem-solving approach and methodology
2. Use of AI capabilities and limitations awareness
3. Iteration and refinement patterns
4. Technical communication effectiveness
5. Learning and adaptation throughout the session

Provide insights on:
- What collaboration patterns work well
- Areas where collaboration could be improved
- Specific examples of effective/ineffective interactions
- Recommendations for optimizing AI-assisted coding workflows

Chat Session:
{chat_data}
```

### Question Quality Analysis
```text
Evaluate the quality and effectiveness of user questions in this coding session.

Analyze:
1. Question specificity and clarity
2. Context provision and background information
3. Technical accuracy and appropriate terminology
4. Follow-up question effectiveness
5. Progressive questioning strategy

For each question category, provide:
- Quality rating (1-10)
- Best examples from the session
- Areas for improvement
- Recommendations for better question formulation

Chat Session:
{chat_data}
```

### Task Breakdown Analysis
```text
Analyze how effectively the user breaks down and approaches complex coding tasks in this session.

Examine:
1. Problem decomposition strategy
2. Sequential approach and logical flow
3. Dependency identification and management
4. Scope management and focus
5. Iterative refinement approach

Provide:
- Assessment of task breakdown effectiveness
- Examples of good/poor decomposition
- Patterns in problem-solving approach
- Suggestions for improved task management

Chat Session:
{chat_data}
```

### Follow-up Patterns Analysis
```text
Analyze the user's follow-up patterns and iteration strategies in this coding session.

Focus on:
1. Response to AI suggestions and feedback
2. Clarification-seeking behavior
3. Building on previous responses
4. Error correction and debugging approach
5. Learning progression throughout the session

Evaluate:
- Follow-up timing and relevance
- Quality of iterative improvements
- Adaptation based on AI feedback
- Overall learning and progression patterns

Chat Session:
{chat_data}
```

## Error Handling Contracts

### Error Response Schema
```json
{
  "error": {
    "code": 400,
    "message": "Invalid request",
    "status": "INVALID_ARGUMENT",
    "details": [
      {
        "@type": "type.googleapis.com/google.rpc.BadRequest",
        "fieldViolations": [
          {
            "field": "contents",
            "description": "contents is required"
          }
        ]
      }
    ]
  }
}
```

### Error Code Mapping
- **400 INVALID_ARGUMENT**: Malformed request or invalid parameters
- **401 UNAUTHENTICATED**: Missing or invalid API key
- **403 PERMISSION_DENIED**: API access denied or quota exceeded
- **429 RESOURCE_EXHAUSTED**: Rate limit exceeded
- **500 INTERNAL**: Google AI service error
- **503 UNAVAILABLE**: Service temporarily unavailable

### Retry Strategy
```rust
pub enum RetryableError {
    RateLimit,      // 429 - retry with exponential backoff
    ServerError,    // 500, 503 - retry with shorter backoff
    NetworkError,   // Connection issues - retry immediately
}

pub enum PermanentError {
    Authentication, // 401 - requires user intervention
    PermissionDenied, // 403 - requires account/billing check
    InvalidRequest, // 400 - requires request modification
    ContentBlocked, // Safety filter triggered
}
```

## Data Formatting Specifications

### Chat Session Formatting
```text
Session: {session_id}
Date: {session_date}
Duration: {session_duration}
Message Count: {message_count}

--- Chat History ---
[{timestamp}] User: {user_message}
[{timestamp}] Assistant: {assistant_response}
[{timestamp}] User: {user_follow_up}
...

--- Session Metadata ---
Tools Used: {tools_list}
Code Languages: {programming_languages}
Primary Topics: {topic_categories}
```

### Token Estimation
- **Average tokens per message**: ~100-200 tokens
- **Analysis prompt**: ~200-300 tokens
- **Session metadata**: ~50-100 tokens
- **Safety margin**: 20% additional for encoding variations

**Estimation Formula**:
```
total_tokens = prompt_tokens + (message_count * avg_message_tokens) + metadata_tokens + safety_margin
```

## Configuration and Optimization

### Generation Config Optimization
```json
{
  "temperature": 0.7,    // Balanced creativity/consistency
  "maxOutputTokens": 2048, // Sufficient for detailed analysis
  "topP": 0.8,          // Focus on high-probability tokens
  "topK": 40,           // Reasonable diversity
  "candidateCount": 1,   // Single response for efficiency
  "stopSequences": ["--- END ANALYSIS ---"] // Optional termination
}
```

### Safety Settings
```json
{
  "safetySettings": [
    {
      "category": "HARM_CATEGORY_HARASSMENT",
      "threshold": "BLOCK_MEDIUM_AND_ABOVE"
    },
    {
      "category": "HARM_CATEGORY_HATE_SPEECH",
      "threshold": "BLOCK_MEDIUM_AND_ABOVE"
    },
    {
      "category": "HARM_CATEGORY_SEXUALLY_EXPLICIT",
      "threshold": "BLOCK_MEDIUM_AND_ABOVE"
    },
    {
      "category": "HARM_CATEGORY_DANGEROUS_CONTENT",
      "threshold": "BLOCK_MEDIUM_AND_ABOVE"
    }
  ]
}
```

## Cost Optimization Strategies

### Token Management
- **Input token optimization**: Remove redundant metadata, compress timestamps
- **Output token limits**: Set appropriate maxOutputTokens based on analysis type
- **Batch processing**: Combine multiple short sessions for efficiency
- **Model selection**: Use gemini-2.5-flash-lite for routine analysis, pro for complex sessions

### Request Optimization
- **Session filtering**: Skip very short or incomplete sessions
- **Content preprocessing**: Remove non-essential formatting
- **Progressive analysis**: Start with summary, detailed analysis on demand
- **Caching**: Store analysis results to avoid duplicate processing

### Monitoring and Alerting
- **Token usage tracking**: Monitor daily/monthly consumption
- **Cost thresholds**: Alert when approaching budget limits
- **Error rate monitoring**: Track API failure rates
- **Performance metrics**: Response time and quality tracking

## Implementation Examples

### Request Builder
```rust
impl ChatAnalysisRequest {
    pub fn to_gemini_request(&self) -> GenerateContentRequest {
        let prompt = self.build_analysis_prompt();
        let formatted_chat = self.format_chat_session();

        GenerateContentRequest {
            contents: vec![Content {
                parts: vec![Part::Text {
                    text: format!("{}\n\n{}", prompt, formatted_chat)
                }],
                role: Some("user".to_string()),
            }],
            generation_config: Some(GenerationConfig {
                temperature: Some(0.7),
                max_output_tokens: Some(2048),
                top_p: Some(0.8),
                top_k: Some(40),
            }),
            safety_settings: Some(default_safety_settings()),
        }
    }
}
```

### Response Processing
```rust
impl GeminiResponse {
    pub fn extract_analysis(&self) -> Result<AnalysisResult> {
        let candidate = self.candidates
            .first()
            .ok_or_else(|| anyhow!("No candidates in response"))?;

        let text = candidate.content.parts
            .first()
            .and_then(|part| match part {
                Part::Text { text } => Some(text.clone()),
            })
            .ok_or_else(|| anyhow!("No text content in response"))?;

        let token_usage = self.usage_metadata
            .as_ref()
            .and_then(|meta| meta.total_token_count);

        Ok(AnalysisResult {
            response_text: text,
            token_usage,
            finish_reason: candidate.finish_reason.clone(),
            safety_ratings: candidate.safety_ratings.clone(),
        })
    }
}
```