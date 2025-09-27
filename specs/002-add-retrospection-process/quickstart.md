# Quickstart Guide: Retrospection Feature

## Prerequisites

### Environment Setup
1. **Google AI API Key**: Obtain from [Google AI Studio](https://aistudio.google.com/)
2. **Environment Variable**: Set `GOOGLE_AI_API_KEY` in your environment
3. **Existing Data**: Have chat sessions imported via `retrochat import scan`

```bash
# Set API key
export GOOGLE_AI_API_KEY="your_api_key_here"

# Verify existing sessions
retrochat import scan
retrochat tui  # Should show imported sessions
```

### Verify Prerequisites
```bash
# Check if sessions exist
retrochat ls

# Should output:
# Found 15 chat sessions:
# - claude-coding-2025-09-26.json (45 messages)
# - chatgpt-debugging-2025-09-25.json (23 messages)
# ...
```

## Basic Usage (CLI)

### 1. Quick Analysis of Single Session
```bash
# Find session ID
retrochat ls | head -1
# Output: claude-coding-2025-09-26.json (ID: session-abc123)

# Analyze with default settings (user interaction analysis)
retrochat retrospect execute session-abc123

# Expected output:
# Starting retrospection analysis...
# Session 1/1: session-abc123 [████████████████████████] 100%
# Analysis completed successfully.
# - Total sessions analyzed: 1
# - Analysis time: 45s
# - Total tokens used: 2,847
```

### 2. View Analysis Results
```bash
# Show results for the analyzed session
retrochat retrospect show session-abc123

# Expected output:
# === Retrospection Results ===
# Session: session-abc123 (analyzed 2025-09-26 10:30:00 UTC)
# Type: User Interaction Analysis
# ...
```

### 3. Analyze Multiple Sessions
```bash
# Analyze all sessions with collaboration focus
retrochat retrospect execute --all --type collaboration --background

# Check status
retrochat retrospect status

# Expected output:
# === Retrospection Status ===
# Active Operations:
#   retro-456 | Collaboration | Running | [██████░░░░] 60% | 3/5 sessions
```

## Basic Usage (TUI)

### 1. Launch TUI and Navigate
```bash
retrochat tui
```

**Navigation Steps**:
1. Use arrow keys to highlight a session in the main list
2. Press `r` to start retrospection analysis for that session
3. Select analysis type from the dialog
4. Watch progress indicator
5. View results in session detail panel

### 2. Batch Analysis via TUI
1. Press `R` to analyze all sessions
2. Confirm analysis scope in dialog
3. Select analysis type
4. Press `Ctrl+r` to monitor progress
5. Navigate to individual sessions to view results

## Common Workflows

### Workflow 1: Daily Coding Review
**Goal**: Analyze yesterday's coding sessions for improvement insights

```bash
# 1. Find recent sessions
retrochat ls --since yesterday

# 2. Analyze with user interaction focus
retrochat retrospect execute --since yesterday --type user-interaction

# 3. Review results
retrochat retrospect show --since yesterday --format markdown > daily_review.md

# 4. Open in preferred editor
code daily_review.md
```

### Workflow 2: Project Retrospective
**Goal**: Comprehensive analysis of all sessions for a specific project

```bash
# 1. Analyze all sessions with multiple analysis types
retrochat retrospect execute --all --type collaboration --background
retrochat retrospect execute --all --type task-breakdown --background

# 2. Monitor progress
retrochat retrospect status --watch

# 3. Export comprehensive report
retrochat retrospect show --all --format json > project_retrospective.json
```

### Workflow 3: Learning and Improvement
**Goal**: Identify patterns and areas for improvement

```bash
# 1. Question quality analysis
retrochat retrospect execute --all --type question-quality

# 2. Follow-up pattern analysis
retrochat retrospect execute --all --type follow-up

# 3. Compare results and identify trends
retrochat retrospect show --type question-quality --sort date
retrochat retrospect show --type follow-up --sort date
```

## Testing Your Setup

### Test 1: Basic API Connectivity
```bash
# Simple test with single session
retrochat retrospect execute $(retrochat ls --limit 1 --id-only)

# Success indicators:
# - Progress bar appears and advances
# - Completes without API errors
# - Results are generated and stored
```

### Test 2: Error Handling
```bash
# Test with invalid API key
GOOGLE_AI_API_KEY="invalid" retrochat retrospect execute session-test

# Expected error:
# Error: Google AI API request failed
# Reason: Authentication failed (401 Unauthorized)
# Check your GOOGLE_AI_API_KEY environment variable
```

### Test 3: Background Operations
```bash
# Start background analysis
retrochat retrospect execute --all --background

# Verify background processing
retrochat retrospect status

# Should show active operations running
```

## Configuration Examples

### Basic Configuration File
Create `~/.config/retrochat/config.toml`:

```toml
[retrospection]
default_timeout = 300
max_concurrent = 2
auto_cleanup_days = 30

[retrospection.analysis_types]
default = "user-interaction"
available = [
    "user-interaction",
    "collaboration",
    "question-quality",
    "task-breakdown",
    "follow-up"
]

[retrospection.api]
model = "gemini-2.5-flash-lite"
max_tokens = 2048
temperature = 0.7
```

### Advanced Configuration
```toml
[retrospection.optimization]
batch_size = 5
retry_attempts = 3
rate_limit_rpm = 60
token_budget_daily = 100000

[retrospection.filters]
min_messages = 5
max_messages = 1000
exclude_patterns = ["test-session-*", "debug-*"]
```

## Troubleshooting

### Common Issues

#### API Key Problems
```bash
# Verify API key is set
echo $GOOGLE_AI_API_KEY

# Test API key validity
curl -H "x-goog-api-key: $GOOGLE_AI_API_KEY" \
     https://generativelanguage.googleapis.com/v1beta/models

# Should return model list, not 401 error
```

#### Rate Limiting
```
Error: Google AI API request failed
Reason: Rate limit exceeded (quota: 60 requests/minute)
```

**Solutions**:
- Wait for rate limit reset
- Reduce concurrent operations: `--max-concurrent 1`
- Use background processing: `--background`

#### Large Session Timeouts
```
Error: Analysis operation timed out after 300 seconds
```

**Solutions**:
- Increase timeout: `--timeout 600`
- Use background processing: `--background`
- Split large sessions into smaller chunks

#### Memory Issues
```
Error: Failed to process session - out of memory
```

**Solutions**:
- Reduce batch size in configuration
- Process sessions individually instead of batch
- Upgrade system memory or use streaming processing

### Diagnostic Commands

```bash
# Check system status
retrochat retrospect status --history

# Verify session data quality
retrochat ls --validate

# Test API connectivity
retrochat retrospect execute --dry-run session-test

# Check configuration
retrochat config show retrospection
```

## Next Steps

### Advanced Features
1. **Custom Analysis Prompts**: Create specialized analysis types
2. **Batch Processing**: Optimize for large datasets
3. **Export Integration**: Connect with external analysis tools
4. **Automated Scheduling**: Set up periodic analysis runs

### Integration Options
1. **CI/CD Pipeline**: Integrate retrospection into development workflow
2. **Team Dashboards**: Share insights across development teams
3. **Progress Tracking**: Monitor improvement over time
4. **Learning Systems**: Build personalized coaching recommendations

### Performance Optimization
1. **Caching Strategies**: Avoid duplicate analysis
2. **Incremental Analysis**: Process only new sessions
3. **Cost Management**: Optimize token usage and API calls
4. **Resource Scaling**: Handle large-scale retrospection efficiently

## Getting Help

### Documentation
- API Reference: `retrochat retrospect --help`
- Configuration Guide: `retrochat config --help`
- TUI Help: Press `F1` while in TUI mode

### Support Channels
- GitHub Issues: Report bugs and feature requests
- Discussion Forum: Ask questions and share tips
- Documentation: Comprehensive guides and examples

### Debugging
```bash
# Enable verbose logging
RUST_LOG=debug retrochat retrospect execute session-test

# Check logs
tail -f ~/.local/share/retrochat/logs/retrochat.log

# Export debug information
retrochat debug export > debug_info.json
```