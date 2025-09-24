---
description: Run retrochat CLI commands for importing and analyzing LLM chat history
---

# RetroChat Commands

Execute retrochat application commands for chat history analysis.

Usage: `/retrochat <subcommand> [args]`

## Available Commands

### Database Setup
- `init` - Initialize application database

### Import Commands
- `import scan` - Scan enabled AI service directories (respects .env config)
- `import scan <directory>` - Scan specific directory for chat files
- `import scan-claude` - Scan Claude Code directories
- `import scan-gemini` - Scan Gemini directories
- `import scan-codex` - Scan Codex directories
- `import file <path>` - Import specific chat file
- `import batch <directory>` - Import all files from directory

### Interface
- `tui` - Launch terminal user interface

### Analysis
- `analyze insights` - Generate usage insights and statistics
- `analyze export` - Export analytics data

### Query
- `query sessions` - List all chat sessions
- `query session <id>` - Show specific session details
- `query search <term>` - Search messages by content

## Examples

```bash
# Initialize database
/retrochat init

# Scan enabled AI services (respects .env config)
/retrochat import scan

# Scan specific directory
/retrochat import scan /path/to/directory

# Scan Claude Code directories only
/retrochat import scan-claude

# Launch TUI interface
/retrochat tui

# Generate insights
/retrochat analyze insights

# Search for specific content
/retrochat query search "claude"
```

## Supported File Formats
- Claude Code (.jsonl files)
- Gemini/Bard (.json files)

## Instructions for Claude Code
When executing retrochat commands, STRICTLY display ONLY the terminal output. Do not abbreviate, summarize, or add commentary. Show the complete raw output exactly as it appears.

$ARGUMENTS will be passed as command line arguments to `cargo run -- $ARGUMENTS`