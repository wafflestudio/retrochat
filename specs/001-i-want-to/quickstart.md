# Quickstart Guide: LLM Chat History Analysis

## Overview
This guide walks through the complete user journey from installation to generating insights about your LLM usage patterns.

## Prerequisites
- Rust 1.75+ installed
- SQLite 3.x available
- Chat history files from Claude Code or Gemini
- Terminal with 80x24 minimum size

## Installation

### From Source
```bash
git clone https://github.com/user/retrochat.git
cd retrochat
cargo build --release
```

### First Run Setup
```bash
# Initialize application database
./target/release/retrochat init

# Verify installation
./target/release/retrochat --version
```

## Quick Start Workflow

### 1. Import Chat History Files

#### Auto-Discovery
```bash
# Scan for chat files in current directory
retrochat import scan

# Scan common chat directories
retrochat import scan ~/.claude/projects
retrochat import scan ~/.gemini/tmp

# Scan specific directory
retrochat import scan ~/.claude
```

#### Manual Import
```bash
# Import single file
retrochat import file ~/.claude/projects/project-123/session-456.jsonl

# Batch import from common directories
retrochat import batch ~/.claude/projects
retrochat import batch ~/.gemini/tmp

# Batch import from any directory
retrochat import batch ~/Downloads
```

### 2. Launch TUI Interface
```bash
# Start the interactive TUI
retrochat tui
```

### 3. Basic Navigation (TUI Mode)

#### Session List View (Default)
- **↑/↓**: Navigate session list
- **Enter**: View session details
- **f**: Open filter dialog
- **s**: Open search dialog
- **a**: Switch to analytics view
- **q**: Quit application

#### Filter Options
- **Date Range**: Last week, month, quarter, custom
- **Provider**: Claude Code, Gemini, All
- **Project**: Select from discovered projects
- **Search**: Full-text search in message content

#### Session Detail View
- **↑/↓**: Scroll through messages
- **PageUp/PageDown**: Fast scroll
- **Home/End**: Jump to start/end
- **Esc**: Return to session list
- **e**: Export session to file

### 4. Analytics and Insights

#### Usage Statistics
- Navigate to Analytics view (press 'a' from main view)
- Select time period (default: last 30 days)
- View provider comparison, temporal trends, project breakdown

#### Generate Insights Report
```bash
# Command-line insights generation
retrochat analyze insights --period last_month --format markdown

# Export comprehensive report
retrochat export --format html --output ~/llm-usage-report.html --include-charts
```

## User Journey Validation

### Scenario 1: First-Time User Setup
1. **Given**: User has Claude Code and Gemini chat histories
2. **When**: User runs `retrochat import scan --provider all`
3. **Then**: System discovers and lists all available chat files
4. **When**: User runs `retrochat import batch --confirm`
5. **Then**: All files are imported with progress feedback
6. **When**: User runs `retrochat tui`
7. **Then**: Dashboard shows imported sessions with usage overview

### Scenario 2: Daily Usage Analysis
1. **Given**: User has imported 3 months of chat history
2. **When**: User opens TUI and navigates to Analytics
3. **Then**: System displays usage trends and provider breakdown
4. **When**: User selects "Last Week" filter
5. **Then**: Charts update to show weekly activity patterns
6. **When**: User requests insights report
7. **Then**: System generates actionable recommendations

### Scenario 3: Session Deep Dive
1. **Given**: User wants to review coding assistance quality
2. **When**: User filters by project "Web Development"
3. **Then**: System shows only relevant sessions
4. **When**: User enters session detail view
5. **Then**: Complete conversation history is displayed with metadata
6. **When**: User searches for "debugging" across all sessions
7. **Then**: System highlights relevant conversations

### Scenario 4: Export and Reporting
1. **Given**: User wants to share usage analysis with team
2. **When**: User runs export command with HTML format
3. **Then**: System generates comprehensive report with charts
4. **When**: User opens generated HTML file
5. **Then**: Report displays usage patterns, insights, and recommendations

## Integration Testing Scenarios

### Import Validation
```bash
# Test Claude Code JSONL parsing
retrochat test import --sample-file tests/fixtures/claude_sample.jsonl

# Test Gemini JSON parsing
retrochat test import --sample-file tests/fixtures/gemini_sample.json

# Test malformed file handling
retrochat test import --sample-file tests/fixtures/corrupted.jsonl
```

### Performance Validation
```bash
# Test large file processing
retrochat test performance --file-size 100MB --provider ClaudeCode

# Test UI responsiveness
retrochat test ui --session-count 10000
```

### Data Integrity Validation
```bash
# Verify import accuracy
retrochat validate --compare-original

# Check analysis consistency
retrochat validate --recompute-analytics
```

## Expected Outcomes

### After Import
- All chat sessions stored in local SQLite database
- Message content preserved with original timestamps
- Projects automatically categorized by file paths
- Duplicate files detected and skipped

### After Analysis
- Usage patterns identified by provider and time period
- Purpose categorization of conversations
- Quality assessment for each LLM provider
- Efficiency metrics and recommendations generated

### Performance Targets
- Import: <1 second per MB of chat data
- UI Response: <100ms for navigation actions
- Search: <500ms for full-text queries across 100k messages
- Analytics: <2 seconds for monthly analysis generation

## Troubleshooting

### Common Issues

#### Import Failures
```bash
# Check file permissions
ls -la ~/.claude/projects/

# Verify file format
retrochat validate file --input problematic_file.jsonl
```

#### Performance Issues
```bash
# Check database size and indexes
retrochat info database

# Optimize database
retrochat maintenance optimize
```

#### TUI Display Issues
```bash
# Verify terminal size
tput cols && tput lines

# Test with different terminal
TERM=xterm-256color retrochat tui
```

## Advanced Usage

### Custom Analysis
```bash
# Generate insights for specific provider
retrochat analyze --provider ClaudeCode --focus efficiency

# Compare provider effectiveness
retrochat compare --providers ClaudeCode,Gemini --metric quality
```

### Data Management
```bash
# Backup analysis database
retrochat backup --output ~/retrochat-backup-$(date +%Y%m%d).db

# Clean old analysis cache
retrochat maintenance clean --older-than 30days
```

### Automation
```bash
# Schedule daily import check
retrochat import scan --auto-import --schedule daily

# Generate weekly reports
retrochat export --schedule weekly --format pdf --email team@company.com
```

This quickstart guide ensures users can successfully import, analyze, and gain insights from their LLM chat histories while validating all functional requirements from the specification.