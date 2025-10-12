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
- `import --path <path>` - Import from specific file or directory
- `import --claude` - Import from Claude Code default directories
- `import --cursor-agent` - Import from Cursor default directories
- `import --gemini` - Import from Gemini default directories
- `import --codex` - Import from Codex default directories
- `import --claude --cursor-agent` - Import from multiple providers
- `import --path <path> --overwrite` - Import with overwrite option

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

# Import from specific path
/retrochat import --path /path/to/directory

# Import from provider directories
/retrochat import --claude --cursor-agent

# Import with overwrite
/retrochat import --path ~/.claude/projects --overwrite

# Launch TUI interface
/retrochat tui

# Generate insights
/retrochat analyze insights

# Search for specific content
/retrochat query search "claude"
```

## Supported File Formats
- Claude Code (.jsonl files)
- Cursor (store.db SQLite database)
- Gemini/Bard (.json files)
- Codex (various formats, experimental)

## Instructions for Claude Code
When executing retrochat commands, STRICTLY display ONLY the terminal output. Do not abbreviate, summarize, or add commentary. Show the complete raw output exactly as it appears.

$ARGUMENTS will be passed as command line arguments to `cargo run -- $ARGUMENTS`