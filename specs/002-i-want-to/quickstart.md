# Quickstart: LLM-Powered Chat Session Retrospection

## Prerequisites

### Environment Setup
1. **Google AI API Key**: Export your Gemini API key as environment variable
   ```bash
   export GEMINI_API_KEY="your-api-key-here"
   ```

2. **Existing Chat Data**: Ensure you have imported chat sessions using existing import commands
   ```bash
   # Import chat files (if not already done)
   retrochat import scan
   retrochat import file path/to/chat/file.json
   ```

3. **Build and Test**: Verify the application builds and tests pass
   ```bash
   cargo check && cargo test && cargo clippy
   ```

## Quick Start Scenarios

### Scenario 1: Basic Session Analysis

**Goal**: Analyze a single chat session for key insights

**Steps**:
1. **List available sessions**:
   ```bash
   retrochat analyze sessions
   ```

2. **Trigger retrospection analysis**:
   ```bash
   retrochat analyze retrospect --session <session-id>
   ```

3. **View analysis results**:
   ```bash
   retrochat analyze show --session <session-id>
   ```

**Expected Outcome**:
- Analysis request queued and processed
- LLM-generated insights stored locally
- Results displayed in readable format with metadata

### Scenario 2: Custom Prompt Analysis

**Goal**: Use custom analysis prompt for specific insights

**Steps**:
1. **List available prompt templates**:
   ```bash
   retrochat prompts list
   ```

2. **Create custom prompt template**:
   ```bash
   retrochat prompts create "project_analysis" \
     --description "Analyze chat for project management insights" \
     --template "Focus on: project milestones, blockers, and team communication patterns. Content: {chat_content}"
   ```

3. **Analyze with custom prompt**:
   ```bash
   retrochat analyze retrospect --session <session-id> --template "project_analysis"
   ```

4. **View custom analysis**:
   ```bash
   retrochat analyze show --session <session-id> --analysis <analysis-id>
   ```

**Expected Outcome**:
- Custom prompt template created and saved
- Analysis uses custom prompt for targeted insights
- Results reflect custom analysis focus

### Scenario 3: TUI Interface Usage

**Goal**: Browse and manage retrospection analyses interactively

**Steps**:
1. **Launch TUI interface**:
   ```bash
   retrochat tui
   ```

2. **Navigate to Retrospection section**:
   - Use arrow keys to navigate menu
   - Select "Retrospection Analyses"

3. **View analysis results**:
   - Browse list of completed analyses
   - Select specific analysis to view full content
   - View metadata (tokens used, cost, execution time)

4. **Trigger new analysis**:
   - Select "New Analysis" option
   - Choose session from list
   - Select prompt template
   - Confirm analysis request

**Expected Outcome**:
- Interactive interface for retrospection management
- Easy browsing of analysis history
- Seamless analysis triggering from TUI

### Scenario 4: Batch Analysis Workflow

**Goal**: Analyze multiple recent sessions for pattern recognition

**Steps**:
1. **List recent sessions**:
   ```bash
   retrochat analyze sessions --limit 10 --sort-by date
   ```

2. **Queue multiple analyses**:
   ```bash
   # Script to analyze last 5 sessions
   for session in $(retrochat analyze sessions --limit 5 --format json | jq -r '.[].id'); do
     retrochat analyze retrospect --session $session --template "pattern_analysis"
   done
   ```

3. **Monitor analysis progress**:
   ```bash
   retrochat analyze status
   ```

4. **View batch results**:
   ```bash
   retrochat analyze list --template "pattern_analysis" --limit 5
   ```

**Expected Outcome**:
- Multiple analyses queued and processed
- Progress tracking for batch operations
- Consolidated view of pattern analysis results

## Validation Tests

### Test 1: API Integration Test
```bash
# Verify Gemini API connectivity
retrochat analyze test-api

# Expected: Successful API connection with test prompt
# Should return: "API connection successful, model: gemini-2.5-flash-lite"
```

### Test 2: Prompt Template Validation
```bash
# Test template creation and validation
retrochat prompts create "test_template" \
  --template "Test prompt with {variable1} and {variable2}" \
  --variables "variable1:required,variable2:optional:default_value"

# Expected: Template created with proper variable validation
# Should return: "Template 'test_template' created successfully"
```

### Test 3: Analysis Storage Test
```bash
# Trigger analysis and verify storage
session_id=$(retrochat analyze sessions --limit 1 --format json | jq -r '.[0].id')
analysis_id=$(retrochat analyze retrospect --session $session_id --format json | jq -r '.requestId')

# Wait for completion (or use status polling)
sleep 30

# Verify storage
retrochat analyze show --analysis $analysis_id

# Expected: Analysis stored with complete metadata
# Should display: analysis content, token usage, execution time, cost
```

### Test 4: Error Handling Test
```bash
# Test invalid API key handling
GEMINI_API_KEY="invalid-key" retrochat analyze retrospect --session <session-id>

# Expected: Clear error message about authentication
# Should return: "Error: Invalid API key. Please check GEMINI_API_KEY environment variable"

# Test invalid session ID
retrochat analyze retrospect --session "invalid-uuid"

# Expected: Clear error about session not found
# Should return: "Error: Session 'invalid-uuid' not found"
```

## Performance Benchmarks

### Token Usage Estimation
- **Small session** (< 1000 messages): ~2000 tokens → $0.002
- **Medium session** (1000-5000 messages): ~8000 tokens → $0.008
- **Large session** (> 5000 messages): May require chunking

### Response Time Expectations
- **API latency**: 2-10 seconds depending on content size
- **Storage time**: < 100ms for database operations
- **Queue processing**: Near real-time for single requests

### Rate Limiting
- **Free tier**: 5 requests per minute (12-second delays)
- **Paid tier**: 1000 requests per minute
- **Automatic retry**: Exponential backoff for rate limit errors

## Troubleshooting

### Common Issues

1. **"API key not found"**
   ```bash
   # Check environment variable
   echo $GEMINI_API_KEY

   # Set if missing
   export GEMINI_API_KEY="your-key"
   ```

2. **"Session not found"**
   ```bash
   # List available sessions
   retrochat analyze sessions

   # Import more data if needed
   retrochat import scan
   ```

3. **"Template validation failed"**
   ```bash
   # Check template syntax
   retrochat prompts validate --template "template_id"

   # View template details
   retrochat prompts show "template_id"
   ```

4. **"Analysis stuck in processing"**
   ```bash
   # Check analysis status
   retrochat analyze status

   # Retry failed analyses
   retrochat analyze retry --analysis <analysis-id>
   ```

### Debug Mode
```bash
# Enable verbose logging
RUST_LOG=debug retrochat analyze retrospect --session <session-id>

# View recent logs
retrochat logs --tail 50
```

## Configuration

### Default Settings
```toml
# ~/.config/retrochat/config.toml
[retrospection]
default_template = "session_summary"
auto_retry_failed = true
max_retry_attempts = 3
rate_limit_delay_ms = 12000

[retrospection.cost_controls]
max_tokens_per_request = 100000
warn_cost_threshold = 0.10
max_cost_per_day = 1.00
```

### Template Management
```bash
# Export templates for backup
retrochat prompts export --all --file templates_backup.json

# Import templates from file
retrochat prompts import --file templates_backup.json

# Set active template
retrochat prompts set-active "session_summary"
```

This quickstart guide provides a comprehensive introduction to the retrospection feature, covering basic usage, advanced scenarios, testing, and troubleshooting. Users can follow these scenarios to verify the feature works as expected and understand its capabilities.