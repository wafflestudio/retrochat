# RetroChat

<p align="center">
  <strong>LLM Agent Chat History Retrospect Application</strong><br>
  A powerful tool for analyzing and exploring your LLM conversation history from multiple providers.
</p>

> **⚠️ Project Status Warning**
>
> This project is currently under active development and **not yet complete**. It may contain bugs, incomplete features, and breaking changes. **Not recommended for production use** at this time. Use at your own risk.

## Features

- **Multi-Provider Support**: Import chat histories from Claude Code, Gemini, and other LLM providers
- **Terminal User Interface (TUI)**: Interactive terminal-based interface for browsing sessions
- **Advanced Analytics**: Generate detailed usage insights and statistics
- **Multiple Export Formats**: Export data to JSON, CSV, or text formats
- **Session Management**: Browse, search, and analyze individual chat sessions
- **SQLite Database**: Persistent storage for all your chat data

## Installation

### Quick Install (npm)

The fastest way to install RetroChat:

```bash
npm install -g @sanggggg/retrochat
```

After installation, you can run:
```bash
retrochat --help
```

**Note**: The npm package includes pre-built binaries. Make sure you have Node.js 16 or later installed.

### Prerequisites

- For npm installation: Node.js 16 or later
- For building from source: Rust 1.75 or later
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

**Note**: On first run, RetroChat automatically initializes the database and runs a setup wizard.

### Terminal User Interface (TUI)

Launch the interactive terminal interface (default mode):

```bash
retrochat
```

This opens an interactive interface where you can:
- Browse all imported chat sessions
- View detailed session information
- Navigate through messages
- View analytics and insights

### Sync Commands

RetroChat provides a unified sync command for importing and watching chat history files.

#### Import Mode (Default)

Import chat history from providers or specific paths:

```bash
# Import from provider default directories
retrochat sync claude gemini

# Import from all providers
retrochat sync all

# Import from a specific path
retrochat sync --path ~/.claude/projects

# Import a single file
retrochat sync --path /path/to/chat/file.jsonl

# Import with overwrite flag
retrochat sync claude --overwrite
```

#### Watch Mode

Watch for file changes and auto-import in real-time:

```bash
# Watch all providers with verbose output
retrochat sync all -w --verbose

# Watch specific providers
retrochat sync claude gemini --watch --verbose

# Watch a specific path
retrochat sync --path /path/to/chat/directory -w --verbose
```

The watch mode monitors file changes and displays:
- File system events (create, modify, delete)
- Provider detection for each file
- Detailed diffs for JSON/JSONL files (with --verbose)
- Parsed session information (with --verbose)

#### Environment Configuration

Configure default directories for each provider (optional):

```bash
# Claude Code directories (default: ~/.claude/projects)
export RETROCHAT_CLAUDE_DIRS="~/.claude/projects:/another/path"

# Gemini directories (default: ~/.gemini/tmp)
export RETROCHAT_GEMINI_DIRS="/path/to/gemini/chats"

# Codex directories (no default, must be configured)
export RETROCHAT_CODEX_DIRS="/path/to/codex/chats"
```

**Note**: Use colon (`:`) to separate multiple directories, e.g., `"/path1:/path2"`

### Query Commands

Search and browse your chat history:

```bash
# List all sessions
retrochat list

# List sessions with filters
retrochat list --provider claude --project myproject

# Show session details
retrochat show SESSION_ID

# Search messages
retrochat search "search query"

# Search with time range
retrochat search "query" --since "7 days ago" --until now
```

### Analysis Commands

#### AI-Powered Session Analysis

Analyze chat sessions using Google AI to generate comprehensive insights:

```bash
# Run analysis for a specific session
retrochat analysis run [SESSION_ID]

# Run analysis for all sessions
retrochat analysis run --all

# Run with custom prompt
retrochat analysis run SESSION_ID --custom-prompt "Analyze coding patterns"

# View analysis results
retrochat analysis show [SESSION_ID]

# View all analysis results
retrochat analysis show --all

# Check analysis status
retrochat analysis status

# Check analysis history
retrochat analysis status --history

# Cancel an analysis request
retrochat analysis cancel [REQUEST_ID]

# Cancel all active requests
retrochat analysis cancel --all
```

**Note**: Analysis commands require `GOOGLE_AI_API_KEY` environment variable to be set.

### Export Commands

Export chat history in various formats:

```bash
# Export to JSON
retrochat export --format json

# Export to JSONL
retrochat export --format jsonl

# Export with filters
retrochat export --format json --provider claude --since "30 days ago"
```

## Supported Chat Providers

RetroChat currently supports importing from:

### Claude Code
- **File Format**: JSONL files
- **Default Location**: `~/.claude/projects`
- **File Pattern**: `*.jsonl`
- **Environment Variable**: `RETROCHAT_CLAUDE_DIRS`

### Gemini
- **File Format**: JSON export files
- **File Pattern**: `session-*.json`
- **Environment Variable**: `RETROCHAT_GEMINI_DIRS`

### Codex (Experimental)
- **File Format**: Various formats
- **Environment Variable**: `RETROCHAT_CODEX_DIRS`

## Database

RetroChat uses SQLite for data persistence. The database file (`retrochat.db`) is created in `~/.retrochat/` directory on first use.

### Automatic Initialization

The database is automatically initialized on first run. The setup wizard will:
- Create the `~/.retrochat` configuration directory
- Initialize the SQLite database with the proper schema
- Run database migrations
- Guide you through importing your first chat history

### Data Structure

The application stores:
- **Chat Sessions**: Session metadata, provider info, timestamps
- **Messages**: Individual messages with role, content, and metadata
- **Analytics**: Computed insights and usage statistics

## Development

### Build and Test

The project uses cargo aliases and shell scripts for development workflows:

#### Cargo Aliases

```bash
# Short aliases for common commands
cargo t              # Run test suite (test --verbose)
cargo c              # Cargo check (check --verbose)
cargo b              # Cargo build
cargo br             # Cargo build --release

# Code quality
cargo fmt-check      # Check formatting (fmt --all -- --check)
cargo fmt-fix        # Apply formatting (fmt --all)
cargo clippy-strict  # Run clippy with -D warnings

# Application shortcuts
cargo tui            # Launch TUI interface (same as: cargo run)
```

#### Shell Scripts

```bash
# Full CI validation (format check, clippy, tests)
./scripts/ci.sh

# Auto-format + auto-fix clippy + verify
./scripts/fix.sh

# Apply clippy auto-fixes
./scripts/clippy-fix.sh

# Remove retrochat database files
./scripts/clean-db.sh

# Check system dependencies (rustc, cargo, python)
./scripts/doctor.sh

# Run end-to-end tests
./scripts/e2e.sh
```

You can also use cargo commands directly:

```bash
# Check code quality
cargo check && cargo test && cargo clippy

# Run specific test suites
cargo test --test test_import_file
cargo test --test test_import_batch

# Format code
cargo fmt

# Run clippy
cargo clippy
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

1. **First run (automatic setup):**
   ```bash
   retrochat
   ```
   This will automatically initialize the database and run the setup wizard.

2. **Import your chat history:**
   ```bash
   # Import from provider default directories
   retrochat sync claude gemini

   # Import from all providers
   retrochat sync all

   # Or import from a specific path
   retrochat sync --path ~/.claude/projects
   retrochat sync --path /path/to/chat/files
   ```

3. **Launch the TUI to explore:**
   ```bash
   retrochat
   ```

4. **Search and query:**
   ```bash
   # List all sessions
   retrochat list

   # Search for specific content
   retrochat search "debugging issue"

   # Show session details
   retrochat show SESSION_ID
   ```

5. **Run AI analysis (requires GOOGLE_AI_API_KEY):**
   ```bash
   # Analyze specific session
   retrochat analysis run SESSION_ID

   # Analyze all sessions
   retrochat analysis run --all

   # View results
   retrochat analysis show --all
   ```

6. **Export data:**
   ```bash
   retrochat export --format json --provider claude
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