# Research Findings: Retrospection Feature Implementation

## Google AI (Gemini) API Integration

### Decision: HTTP Client with reqwest + Tokio
**Rationale**: reqwest is the most mature HTTP client for Rust with excellent async support and built-in JSON handling. Tokio provides robust async runtime already used in the project.

**Key Implementation Points**:
- Use reqwest::Client with connection pooling and timeout configuration
- Configure 60-second timeout for LLM processing requests
- Implement custom header injection for Google AI API key authentication
- Use structured request/response types with serde for JSON handling

**Alternatives Considered**:
- hyper (too low-level for this use case)
- ureq (synchronous, doesn't fit async architecture)

### Decision: Exponential Backoff for Rate Limiting
**Rationale**: Google AI API has rate limits and can experience temporary failures. Exponential backoff prevents overwhelming the service while providing resilient error recovery.

**Key Implementation Points**:
- Use `backoff` crate for retry logic with configurable max attempts
- Implement rate limiting with tokio::sync::Semaphore
- Distinguish between transient (retryable) and permanent errors
- Maximum 5-minute total retry window to avoid infinite waits

**Alternatives Considered**:
- Fixed interval retry (less efficient, could hit rate limits)
- No retry logic (poor user experience for transient failures)

## Background Operation Status Management

### Decision: Channel-Based Task Management with Persistent Storage
**Rationale**: Combines real-time progress updates via channels with persistent storage for operation history and recovery after application restart.

**Key Implementation Points**:
- Use mpsc channels for task requests and watch channels for status updates
- Background operation manager with concurrent task spawning
- SQLite storage for operation status persistence using existing SQLx infrastructure
- Cancellation support via tokio_util::CancellationToken

**Alternatives Considered**:
- Memory-only status (lost on restart)
- Database-only updates (too slow for real-time progress)
- Shared state with Arc<Mutex> (channel approach is more idiomatic)

### Decision: Progress Reporting through Callback Interface
**Rationale**: Allows flexible progress reporting without tight coupling between business logic and UI updates.

**Key Implementation Points**:
- Progress reporter with async update methods
- Operation progress includes current/total steps, status, messages, and error details
- UI widgets can subscribe to progress updates via channels
- Graceful degradation when progress reporting fails

**Alternatives Considered**:
- Polling-based status checking (less responsive)
- Direct UI updates from business logic (violates separation of concerns)

## Error Handling and Recovery

### Decision: Comprehensive Error Types with Context
**Rationale**: Different error types require different handling strategies. Rich error information helps users understand and potentially resolve issues.

**Key Implementation Points**:
- Custom error types for Google AI API failures (authentication, rate limiting, content blocking)
- Distinguish between recoverable and permanent failures
- Error context preservation for debugging
- User-friendly error messages for common failure scenarios

**Alternatives Considered**:
- Generic error handling (less actionable error information)
- Silent error recovery (poor user experience)

### Decision: Graceful Degradation for Network Issues
**Rationale**: Network issues are common and should not break the application. Users should be able to continue using other features while background operations handle connectivity problems.

**Key Implementation Points**:
- Timeout handling with configurable duration
- Offline operation detection and queuing
- Clear status indication when operations are blocked by network issues
- Option to retry failed operations manually

## Data Models and Storage

### Decision: Simple LLM Response Storage
**Rationale**: Store the raw LLM response text and metadata without complex parsing. This preserves full context and avoids data loss from parsing errors.

**Key Implementation Points**:
- Store response text as-is in database TEXT field
- Separate metadata table for token usage, timestamps, API parameters
- Link retrospection results to source chat sessions via foreign keys
- Optional response caching to avoid duplicate analysis

**Alternatives Considered**:
- Structured analysis storage (complex and lossy)
- File-based storage (harder to query and manage)

### Decision: Async Analysis Pipeline
**Rationale**: Chat session analysis can take significant time and should not block the UI. Pipeline approach allows for batch processing and progress tracking.

**Key Implementation Points**:
- Background task queue for analysis requests
- Batch processing capability for multiple sessions
- Analysis status tracking (pending, running, completed, failed)
- Cancellation support for long-running analysis

**Alternatives Considered**:
- Synchronous analysis (poor user experience)
- Real-time streaming analysis (more complex, limited benefit)

## UI Integration Patterns

### Decision: Non-Blocking Progress Display
**Rationale**: Users should be able to continue using the application while retrospection analysis runs in the background.

**Key Implementation Points**:
- Progress indicators in TUI using ratatui Gauge widgets
- Background operation status panel (toggleable)
- Keyboard shortcuts for canceling operations
- Status persistence across application restarts

**Alternatives Considered**:
- Modal progress dialogs (blocks user interaction)
- Status bar only (insufficient detail for long operations)

### Decision: Contextual Result Display
**Rationale**: Retrospection results are most valuable when viewed in context of the specific chat session being analyzed.

**Key Implementation Points**:
- Integration with existing session detail side panel
- Quick access from session list via keyboard shortcut
- Separate retrospection management interface for bulk operations
- Historical analysis results browsing

## Privacy and Security Considerations

### Decision: User Consent for External Processing
**Rationale**: Constitution requires privacy protection. Users must explicitly consent to sending chat data to Google AI.

**Key Implementation Points**:
- Explicit consent dialog before first retrospection
- Clear indication when data is being sent externally
- Option to disable external processing entirely
- Data retention policy for analysis results

**Alternatives Considered**:
- Implicit consent (violates privacy principles)
- Local-only analysis (limited analytical capability)

### Decision: Selective Data Transmission
**Rationale**: Minimize privacy exposure by sending only necessary chat content for analysis.

**Key Implementation Points**:
- Option to exclude sensitive messages from analysis
- Automatic detection and filtering of potential credentials/keys
- User review of data before transmission
- Clear audit trail of what data was sent when

## Technical Integration Points

### Decision: Extend Existing CLI/TUI Architecture
**Rationale**: Leverage existing patterns for consistency and reduced development effort.

**Key Implementation Points**:
- Extend existing Clap CLI interface with retrospection subcommands
- Integrate with existing TUI state management and widget system
- Use existing SQLx connection pool and migration system
- Follow existing error handling and logging patterns

**Key Dependencies**:
- reqwest 0.12 for HTTP client
- tokio-util for cancellation tokens
- backoff crate for retry logic
- uuid for operation IDs

## Implementation Priority

1. **Phase 1**: Core Google AI client and basic analysis
2. **Phase 2**: Background operation management and status tracking
3. **Phase 3**: TUI integration and progress display
4. **Phase 4**: CLI commands and batch processing
5. **Phase 5**: Privacy controls and data management