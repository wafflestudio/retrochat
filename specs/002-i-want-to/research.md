# Research: LLM-Powered Chat Session Retrospection

## Gemini API Integration

### Decision: Direct reqwest + tokio + serde integration
**Rationale**: The `google-generative-ai-rs` crate is archived and unmaintained. Direct HTTP client implementation using the mature Rust ecosystem provides better control, maintenance, and feature access.

**Implementation Approach**:
- Use `reqwest` 0.12 with JSON features for HTTP operations
- Implement custom rate limiting with `tokio::sync::Semaphore`
- Add exponential backoff for 429 error handling
- Structure requests using Gemini's REST API format

**Key Technical Details**:
- Endpoint: `https://generativelanguage.googleapis.com/v1beta/models/gemini-2.5-flash-lite:generateContent`
- Authentication: `x-goog-api-key` header with GEMINI_API_KEY environment variable
- Rate limits: 5 RPM for free tier, requiring 12-second delays between requests
- Context window: 1M tokens (free) / 2M tokens (paid)
- Cost: $0.10/1M input tokens, $0.40/1M output tokens

**Alternatives Considered**:
- google-generative-ai-rs crate: Rejected due to archived/unmaintained status
- OpenAI compatibility endpoint: Rejected for limited feature access
- Vertex AI: Rejected for authentication complexity vs. simple API key

## Configurable Prompts System

### Decision: TOML-based configuration with XDG compliance
**Rationale**: TOML is the Rust ecosystem standard, human-readable, supports comments, and avoids YAML's whitespace issues. XDG compliance provides predictable cross-platform configuration locations.

**Implementation Approach**:
- Store templates in `~/.config/retrochat/prompts/` using XDG directories
- Separate default templates (built-in) from user templates
- Use `serde` + `toml` for serialization with validation layer
- Implement template variables with required/optional parameters

**Directory Structure**:
```
~/.config/retrochat/
├── config.toml                 # Main configuration
├── prompts/
│   ├── defaults/              # Built-in templates (read-only)
│   │   ├── session_summary.toml
│   │   ├── pattern_analysis.toml
│   │   └── retrospective.toml
│   └── user/                  # User-defined templates
└── cache/                     # Analysis results cache
```

**Default Prompt Templates**:
1. **Session Summary**: Analyzes single sessions for topics, intent, and outcomes
2. **Pattern Analysis**: Identifies recurring patterns across multiple sessions
3. **Retrospective Analysis**: Provides timeline and improvement recommendations

**Alternatives Considered**:
- JSON configuration: Rejected due to lack of comment support
- YAML configuration: Rejected due to whitespace sensitivity and security concerns
- Database storage: Rejected for unnecessary complexity for this use case

## Token Management and Large Data Handling

### Decision: Smart chunking with context preservation
**Rationale**: Chat sessions may exceed Gemini's token limits. Intelligent chunking maintains conversation context while staying within API constraints.

**Implementation Strategies**:
- Implement sliding window chunking for large sessions
- Preserve conversation boundaries (don't split mid-exchange)
- Add context bridging between chunks
- Track token usage via API response metadata
- Implement cost estimation and user warnings

**Chunking Algorithm**:
1. Estimate tokens using character count approximation (4 chars ≈ 1 token)
2. Split at natural conversation boundaries
3. Add overlap context between chunks for continuity
4. Combine chunk analyses into cohesive final report

## Data Storage and Retrieval

### Decision: Extend existing SQLite schema
**Rationale**: Leverage existing database infrastructure with new tables for retrospection data. Maintains consistency with current data model.

**New Database Entities**:
- `retrospection_analyses`: Stores analysis results with metadata
- `prompt_templates`: Stores user-defined prompt configurations
- `analysis_requests`: Tracks analysis jobs and status

**Storage Considerations**:
- Store full LLM responses for offline viewing
- Include metadata: timestamp, model used, prompt template, token usage
- Index by session_id for quick retrieval
- Implement soft delete for analysis history

## Error Handling and Resilience

### Decision: Comprehensive error types with graceful degradation
**Rationale**: Network operations and external API calls require robust error handling with clear user feedback and recovery options.

**Error Categories**:
- **API Errors**: Rate limits, authentication, service unavailable
- **Data Errors**: Token limits exceeded, malformed responses
- **Configuration Errors**: Invalid templates, missing API keys
- **Storage Errors**: Database write failures, disk space issues

**Recovery Strategies**:
- Retry with exponential backoff for transient failures
- Queue analysis requests for offline processing
- Cache successful responses to avoid re-analysis
- Provide clear error messages with suggested actions

## Dependencies Required

**New Dependencies**:
```toml
reqwest = { version = "0.12", features = ["json"] }
toml = "0.8"
directories = "5.0"
regex = "1.10"
```

**Existing Dependencies to Leverage**:
- `tokio`: Async runtime for HTTP operations
- `serde`: JSON serialization for API communication
- `rusqlite`: Database storage for analysis results
- `uuid`: Generate unique IDs for analysis records
- `chrono`: Timestamp management
- `thiserror`: Error type definitions