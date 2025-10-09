# RetroChat

LLM Agent Chat History Retrospect Application - A powerful tool for analyzing and exploring your LLM conversation history from multiple providers.

## Features

- **Multi-Provider Support**: Import chat histories from Claude Code, Gemini, and other LLM providers
- **Terminal User Interface (TUI)**: Interactive terminal-based interface for browsing sessions
- **Advanced Analytics**: Generate detailed usage insights and statistics
- **Multiple Export Formats**: Export data to JSON, CSV, or text formats
- **Session Management**: Browse, search, and analyze individual chat sessions
- **SQLite Database**: Persistent storage for all your chat data

## Installation

### Prerequisites

- Rust 1.75 or later
- mise (for version management, optional but recommended)

### From Source

1. Clone the repository:
```bash
git clone <repository-url>
cd retrochat
```

2. Build the project:
```bash
cargo build --release
```

3. The binary will be available at `target/release/retrochat`

### Using mise (Recommended)

If you have mise installed, it will automatically use the correct Rust version specified in the project.

## Usage

RetroChat provides several command-line interfaces and a TUI for different use cases.

### Terminal User Interface (TUI)

Launch the interactive terminal interface:

```bash
retrochat tui
```

This opens an interactive interface where you can:
- Browse all imported chat sessions
- View detailed session information
- Navigate through messages
- View analytics and insights

### Import Commands

RetroChat provides a unified import command that can import from a specific path or from provider-specific default directories.

#### Import from Specific Path

Import a file or directory:

```bash
# Import a single file
retrochat import --path /path/to/chat/file.jsonl

# Import all files from a directory
retrochat import --path /path/to/chat/directory

# Import with overwrite flag
retrochat import --path ~/.claude/projects --overwrite
```

#### Import from Provider Directories

Import from configured default directories for each provider:

```bash
# Import from Claude Code default directories
retrochat import --claude

# Import from Cursor default directories
retrochat import --cursor

# Import from Gemini default directories
retrochat import --gemini

# Import from Codex default directories
retrochat import --codex

# Import from multiple providers at once
retrochat import --claude --cursor --overwrite
```

#### Environment Configuration

Configure default directories for each provider:

```bash
# Claude Code directories (default: ~/.claude/projects)
export RETROCHAT_CLAUDE_DIRS="~/.claude/projects:/another/path"

# Cursor directories (default: ~/.cursor/chats)
export RETROCHAT_CURSOR_DIRS="~/.cursor/chats"

# Gemini directories (no default)
export RETROCHAT_GEMINI_DIRS="/path/to/gemini/chats"

# Codex directories (no default)
export RETROCHAT_CODEX_DIRS="/path/to/codex/chats"

# Enable/disable specific providers
export RETROCHAT_ENABLE_CLAUDE=true
export RETROCHAT_ENABLE_CURSOR=true
export RETROCHAT_ENABLE_GEMINI=true
export RETROCHAT_ENABLE_CODEX=false
```

### Analytics Commands

#### Generate Usage Insights

Generate comprehensive usage statistics:

```bash
retrochat analyze insights
```

This provides:
- Total sessions, messages, and token counts
- Provider breakdown with percentages
- Date range analysis
- Session duration statistics
- Top projects by usage

#### Export Data

Export analytics data in various formats:

```bash
# Export to JSON (default format)
retrochat analyze export json

# Export to CSV
retrochat analyze export csv

# Export to text file
retrochat analyze export txt

# Export to specific file
retrochat analyze export json --output my_analysis.json
```

## Supported Chat Providers

RetroChat currently supports importing from:

### Claude Code
- **File Format**: JSONL files
- **Default Location**: `~/.claude/projects`
- **File Pattern**: `*.jsonl`
- **Environment Variable**: `RETROCHAT_CLAUDE_DIRS`

### Cursor
- **File Format**: SQLite database (store.db)
- **Default Location**: `~/.cursor/chats`
- **File Pattern**: `store.db`
- **Environment Variable**: `RETROCHAT_CURSOR_DIRS`

### Gemini
- **File Format**: JSON export files
- **File Pattern**: `*gemini*.json`
- **Environment Variable**: `RETROCHAT_GEMINI_DIRS`

### Codex (Experimental)
- **File Format**: Various formats
- **Environment Variable**: `RETROCHAT_CODEX_DIRS`

## Database

RetroChat uses SQLite for data persistence. The database file (`retrochat.db`) is created in the current working directory on first use.

### Data Structure

The application stores:
- **Chat Sessions**: Session metadata, provider info, timestamps
- **Messages**: Individual messages with role, content, and metadata
- **Analytics**: Computed insights and usage statistics

## Development

### Build and Test

```bash
# Check code quality
cargo check && cargo test && cargo clippy

# Run specific test suites
cargo test --test test_import_scan
cargo test --test test_analytics_usage
```

### Project Structure

```
src/
├── models/       # Data structures and database models
├── services/     # Business logic and file processing
├── cli/          # Command-line interface
├── tui/          # Terminal user interface
├── parsers/      # Chat file format parsers
├── database/     # Database repositories and schema
└── lib/          # Shared utilities

tests/
├── contract/     # API contract tests
├── integration/  # Integration tests
└── unit/         # Unit tests
```

## Examples

### Quick Start Workflow

1. **Import your chat history:**
   ```bash
   # Import from provider default directories
   retrochat import --claude --cursor

   # Or import from a specific path
   retrochat import --path ~/.claude/projects
   retrochat import --path ~/Downloads/my-chats
   ```

2. **Launch the TUI to explore:**
   ```bash
   retrochat tui
   ```

3. **Generate analytics:**
   ```bash
   retrochat analyze insights
   ```

4. **Export detailed report:**
   ```bash
   retrochat analyze export json --output my_chat_analysis.json
   ```

### Typical Use Cases

- **Personal Usage Tracking**: Monitor your LLM usage patterns across different providers
- **Project Analysis**: Understand which projects generate the most AI conversations
- **Historical Research**: Search through past conversations for specific topics or solutions
- **Data Migration**: Consolidate chat histories from multiple LLM tools into one database

## License

MIT

## Contributing

1. Fork the repository
2. Create a feature branch
3. Make your changes
4. Add tests for new functionality
5. Ensure all tests pass: `cargo test`
6. Run linting: `cargo clippy`
7. Submit a pull request

---

For more information about specific features or troubleshooting, please refer to the source code documentation or open an issue.