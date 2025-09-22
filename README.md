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

#### Scan for Chat Files

Scan a directory to find supported chat files:

```bash
# Scan current directory
retrochat import scan

# Scan specific directory
retrochat import scan /path/to/chat/files

# Scan common chat directories
retrochat import scan ~/.claude/projects
retrochat import scan ~/.gemini/tmp
```

#### Import Single File

Import a specific chat file:

```bash
retrochat import file /path/to/chat/file.jsonl
```

#### Batch Import

Import all supported files from a directory:

```bash
retrochat import batch /path/to/chat/directory

# Import from common chat directories
retrochat import batch ~/.claude/projects
retrochat import batch ~/.gemini/tmp
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
- **Default Locations**:
  - macOS: `~/Library/Application Support/Claude Code/`
  - Linux: `~/.config/claude-code/`
  - Windows: `%APPDATA%/Claude Code/`
- **File Pattern**: `*claude-code*.json*`

### Gemini
- **File Format**: JSON export files
- **File Pattern**: `*gemini*.json`

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

1. **Scan for existing chat files:**
   ```bash
   # Scan common chat directories
   retrochat import scan ~/.claude/projects
   retrochat import scan ~/.gemini/tmp
   
   # Or scan any directory
   retrochat import scan ~/Downloads
   ```

2. **Import your chat history:**
   ```bash
   # Import from common chat directories
   retrochat import batch ~/.claude/projects
   retrochat import batch ~/.gemini/tmp
   
   # Or import from any directory
   retrochat import batch ~/Downloads
   ```

3. **Launch the TUI to explore:**
   ```bash
   retrochat tui
   ```

4. **Generate analytics:**
   ```bash
   retrochat analyze insights
   ```

5. **Export detailed report:**
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