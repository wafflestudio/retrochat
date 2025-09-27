# CLI Interface Contract: Retrospection Commands

## Command Structure

### retrospect execute
Initiates retrospection analysis for chat sessions.

**Syntax**:
```bash
retrochat retrospect execute [OPTIONS] [SESSION_IDS...]
```

**Options**:
- `--all`: Analyze all imported chat sessions
- `--type <TYPE>`: Analysis type (user-interaction, collaboration, question-quality, task-breakdown, follow-up, custom)
- `--prompt <TEXT>`: Custom analysis prompt (required with --type custom)
- `--background`: Run analysis in background and return immediately
- `--timeout <SECONDS>`: Timeout for analysis operation (default: 300)

**Arguments**:
- `SESSION_IDS`: Space-separated list of specific session IDs to analyze

**Examples**:
```bash
# Analyze specific sessions
retrochat retrospect execute session-123 session-456

# Analyze all sessions with user interaction analysis
retrochat retrospect execute --all --type user-interaction

# Custom analysis with specific prompt
retrochat retrospect execute --type custom --prompt "Analyze coding patterns" session-123

# Background analysis
retrochat retrospect execute --background --all
```

**Exit Codes**:
- 0: Success
- 1: Invalid arguments or missing session IDs
- 2: Analysis failed or timeout
- 3: Google AI API error
- 4: Database error

**Output Format**:
```
Starting retrospection analysis...
Session 1/3: session-123 [████████████████████░] 95%
Session 2/3: session-456 [██████████████░░░░░░] 70%
Session 3/3: session-789 [░░░░░░░░░░░░░░░░░░░░] 0%

Analysis completed successfully.
- Total sessions analyzed: 3
- Analysis time: 2m 45s
- Total tokens used: 12,847
```

### retrospect show
Displays stored retrospection results.

**Syntax**:
```bash
retrochat retrospect show [OPTIONS] [SESSION_IDS...]
```

**Options**:
- `--all`: Show results for all analyzed sessions
- `--type <TYPE>`: Filter by analysis type
- `--since <DATE>`: Show results since date (YYYY-MM-DD format)
- `--format <FORMAT>`: Output format (text, json, markdown)
- `--limit <N>`: Limit number of results (default: 10)
- `--sort <FIELD>`: Sort by field (date, session, tokens)

**Arguments**:
- `SESSION_IDS`: Specific session IDs to show results for

**Examples**:
```bash
# Show results for specific session
retrochat retrospect show session-123

# Show all recent results
retrochat retrospect show --all --since 2025-09-20

# Export results as JSON
retrochat retrospect show --format json session-123 > analysis.json

# Show collaboration insights only
retrochat retrospect show --type collaboration --limit 5
```

**Output Format (text)**:
```
=== Retrospection Results ===

Session: session-123 (analyzed 2025-09-26 10:30:00 UTC)
Type: User Interaction Analysis
Tokens: 2,847 | Processing Time: 45s

--- Analysis ---
The user demonstrates strong communication patterns with clear, specific
questions. They effectively break down complex problems into manageable
chunks and provide sufficient context for the AI assistant...

--- Key Insights ---
• Clear communication style with specific requirements
• Effective use of examples and context
• Good follow-up questions for clarification
• Could improve initial problem statements

--- Collaboration Score ---
Communication Clarity: 8/10
Question Quality: 9/10
Follow-up Effectiveness: 7/10
Overall Collaboration: 8/10

=====================================
```

### retrospect status
Shows status of ongoing or recent retrospection operations.

**Syntax**:
```bash
retrochat retrospect status [OPTIONS]
```

**Options**:
- `--active`: Show only currently running operations
- `--history`: Show completed operations history
- `--watch`: Continuously monitor active operations
- `--json`: Output in JSON format

**Examples**:
```bash
# Show current status
retrochat retrospect status

# Monitor active operations
retrochat retrospect status --watch

# Show operation history
retrochat retrospect status --history --limit 10
```

**Output Format**:
```
=== Retrospection Status ===

Active Operations:
  retro-456 | User Interaction | Running    | [████████░░] 80% | 2/3 sessions
  retro-789 | Collaboration    | Pending    | [░░░░░░░░░░] 0%  | 0/5 sessions

Recent Operations:
  retro-123 | Task Breakdown   | Completed  | 2025-09-26 09:45 | 1 session
  retro-321 | Question Quality | Failed     | 2025-09-26 09:30 | API timeout
  retro-654 | Custom Analysis  | Cancelled  | 2025-09-26 09:15 | User cancelled

Total: 2 active, 3 recent
```

### retrospect cancel
Cancels running retrospection operations.

**Syntax**:
```bash
retrochat retrospect cancel [OPERATION_IDS...]
```

**Options**:
- `--all`: Cancel all active operations
- `--force`: Force immediate cancellation without graceful shutdown

**Arguments**:
- `OPERATION_IDS`: Specific operation IDs to cancel

**Examples**:
```bash
# Cancel specific operation
retrochat retrospect cancel retro-456

# Cancel all operations
retrochat retrospect cancel --all

# Force cancel operation
retrochat retrospect cancel --force retro-789
```

## Error Handling

### Common Error Scenarios

**Invalid Session ID**:
```
Error: Session 'invalid-session' not found
Available sessions: session-123, session-456, session-789
Use 'retrochat import scan' to discover sessions
```

**Google AI API Errors**:
```
Error: Google AI API request failed
Reason: Rate limit exceeded (quota: 60 requests/minute)
Retry after: 45 seconds
Use --timeout option to wait automatically
```

**Network Connectivity**:
```
Error: Unable to connect to Google AI service
Check your internet connection and API key configuration
Set GOOGLE_AI_API_KEY environment variable
```

**Analysis Timeout**:
```
Error: Analysis operation timed out after 300 seconds
Consider using --background option for large datasets
Increase timeout with --timeout option
```

## Configuration

### Environment Variables
- `GOOGLE_AI_API_KEY`: Required API key for Google AI service
- `RETROCHAT_TIMEOUT`: Default timeout for operations (seconds)
- `RETROCHAT_CONCURRENT`: Maximum concurrent analysis operations

### Configuration File
```toml
[retrospection]
default_timeout = 300
max_concurrent = 3
auto_cleanup_days = 30
api_rate_limit = 60

[retrospection.analysis_types]
user_interaction = "Analyze user communication patterns and interaction effectiveness"
collaboration = "Evaluate collaboration strengths and areas for improvement"
question_quality = "Assess clarity and effectiveness of user questions"
task_breakdown = "Examine task decomposition and problem-solving approach"
follow_up = "Analyze follow-up patterns and iteration strategies"
```

## Integration Points

### Existing CLI Commands
- Integrates with `retrochat import` for session discovery
- Uses existing database connection and configuration
- Follows existing error handling and logging patterns
- Respects existing CLI styling and output formatting

### TUI Integration
- CLI commands available as TUI shortcuts
- Status updates propagated to TUI progress indicators
- Results displayable in TUI session detail panels
- Background operations managed through TUI interface