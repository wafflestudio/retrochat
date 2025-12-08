# RetroChat

<p align="center">
  <strong>LLM Agent Chat History Retrospect Application</strong><br>
  A powerful desktop application for analyzing and exploring your LLM conversation history from multiple providers.
</p>

> **⚠️ Project Status Warning**
>
> This project is currently under active development and **not yet complete**. It may contain bugs, incomplete features, and breaking changes. **Not recommended for production use** at this time. Use at your own risk.

## Features

- **Desktop GUI Application**: Modern desktop interface built with Tauri 2.0, React 19, and shadcn/ui
- **Multi-Provider Support**: Import chat histories from Claude Code, Gemini, and other LLM providers
- **Advanced Analytics & Visualizations**: Interactive charts and histograms showing session activity, message timelines, and usage patterns
- **AI-Powered Insights**: Generate comprehensive session analysis using Google AI
- **Real-time Sync**: Watch mode for automatic import of new chat files
- **Session Management**: Browse, search, filter, and analyze individual chat sessions
- **Multiple Export Formats**: Export data to JSON, JSONL, or text formats
- **SQLite Database**: Persistent storage for all your chat data
- **Command Line Interface**: Full-featured CLI for automation and advanced users

> **Note**: The Terminal User Interface (TUI) is being phased out in favor of the desktop GUI application.

## Usage

RetroChat provides both a desktop GUI application and a command-line interface for different use cases.

**Note**: On first run, RetroChat automatically initializes the database and runs a setup wizard.

### Desktop GUI Application

Launch the desktop application for the full visual experience:

```bash
cd src-tauri
cargo tauri dev  # Development mode
# or use the built application from your applications folder
```

The GUI provides:
- **Session Browser**: Browse and filter all imported chat sessions
- **Session Details**: View full conversation threads with syntax highlighting
- **Analytics Dashboard**: Interactive charts showing:
  - Session activity histograms
  - Message timeline visualizations
  - Provider usage statistics
  - Session duration and message count metrics
- **Provider Management**: Configure and import from multiple LLM providers
- **Search & Filter**: Advanced search with multiple criteria
- **Dark/Light Theme**: System-aware theme with manual toggle

### Command Line Interface

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
retrochat analysis run [SESSION_ID]

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

## Desktop GUI Features

The desktop application provides a rich visual interface built with modern web technologies:

### Session Management
- **Session Browser**: Grid or list view of all imported sessions
- **Advanced Filtering**: Filter by provider, project, date range, and custom criteria
- **Quick Search**: Instant search across session titles and content
- **Session Details**: Full conversation view with message threading
- **Syntax Highlighting**: Code blocks and technical content rendered beautifully

### Analytics Dashboard
- **Session Activity Histogram**: Visualize chat activity over time
- **Message Timeline**: Track message patterns and conversation flow
- **Provider Statistics**: Compare usage across different LLM providers
- **Usage Metrics**: Session counts, message volumes, and duration analysis
- **Interactive Charts**: Built with Recharts and Plotly.js for rich data visualization

### User Experience
- **Dark/Light Theme**: System-aware theme with manual toggle
- **Responsive Layout**: Adaptive UI that works at any window size
- **Keyboard Shortcuts**: Navigate efficiently with hotkeys
- **Real-time Updates**: Live sync status and progress indicators
- **Modern UI**: Built with shadcn/ui components and Tailwind CSS

### Provider Management
- **Multi-Provider Support**: Configure multiple LLM providers simultaneously
- **Custom Directories**: Set custom import paths for each provider
- **Preset Import**: Import provider presets for quick setup
- **Watch Mode**: Auto-import new conversations as they're created

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

### Desktop GUI Development

#### Frontend (React)

```bash
cd ui-react

# Install dependencies
pnpm install

# Start development server (UI only)
pnpm dev

# Code quality
pnpm biome:format      # Format code with Biome
pnpm biome:lint        # Lint code
pnpm biome:lint:fix    # Fix linting issues
pnpm biome:ci          # Run CI checks (format + lint)

# Build
pnpm build             # Build production bundle
```

**Note**: Use Biome for code formatting and linting, not Prettier or ESLint.

#### Tauri Application

```bash
cd src-tauri

# Development (runs both frontend and backend with hot reload)
cargo tauri dev

# Build production application
cargo tauri build

# Icon generation
cargo tauri icon path/to/icon.png

# Testing
cargo test
cargo clippy
```

### CLI/Core Development

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

#### Direct Cargo Commands

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

### Technology Stack

#### Desktop Application
- **Backend**: Tauri 2.0 (Rust)
- **Frontend**: React 19 with TypeScript
- **UI Components**: shadcn/ui (Radix UI primitives)
- **Styling**: Tailwind CSS v4
- **Charts**: Recharts, Plotly.js
- **Build Tool**: Vite
- **Code Quality**: Biome (formatting & linting)
- **Package Manager**: pnpm

#### CLI/Core
- **Language**: Rust 1.75+
- **TUI Framework**: Ratatui (being phased out)
- **Database**: SQLite with SQLx
- **Async Runtime**: Tokio
- **CLI Framework**: Clap
- **Serialization**: Serde

### Project Structure

```
src/                    # Rust core library
├── models/             # Data structures and database models
├── services/           # Business logic and file processing
├── cli/                # Command-line interface
├── tui/                # Terminal user interface (being phased out)
├── parsers/            # Chat file format parsers
├── database/           # Database repositories and schema
└── lib/                # Shared utilities

src-tauri/              # Tauri desktop application backend
├── src/                # Tauri Rust application code
├── Cargo.toml          # Tauri dependencies
└── tauri.conf.json     # Tauri configuration

ui-react/               # React frontend for desktop app
├── src/
│   ├── components/     # React components (session manager, analytics, charts)
│   ├── hooks/          # Custom React hooks
│   ├── lib/            # Utilities and helpers
│   └── types/          # TypeScript type definitions
├── package.json        # Frontend dependencies
├── biome.json          # Biome configuration
└── vite.config.ts      # Vite build configuration

tests/
├── contract/           # API contract tests
├── integration/        # Integration tests
└── unit/               # Unit tests
```

## Examples

### Quick Start Workflow

#### Using the Desktop GUI (Recommended)

1. **Build and launch the application:**
   ```bash
   cd src-tauri
   cargo tauri dev
   ```

2. **First-time setup:**
   - The GUI will automatically initialize the database on first run
   - Configure provider directories in the settings panel
   - Import your first chat history using the sync feature

3. **Explore your chat history:**
   - Browse sessions in the session manager
   - View detailed conversations with syntax highlighting
   - Analyze usage patterns in the analytics dashboard
   - Use interactive charts to understand your LLM usage

4. **Real-time sync:**
   - Enable watch mode to automatically import new conversations
   - Monitor file changes in real-time

#### Using the CLI

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

   # Watch mode for real-time sync
   retrochat sync all --watch --verbose
   ```

3. **Search and query:**
   ```bash
   # List all sessions
   retrochat list

   # Search for specific content
   retrochat search "debugging issue"

   # Show session details
   retrochat show SESSION_ID
   ```

4. **Run AI analysis (requires GOOGLE_AI_API_KEY):**
   ```bash
   # Analyze specific session
   retrochat analysis run SESSION_ID

   # Analyze all sessions
   retrochat analysis run --all

   # View results
   retrochat analysis show --all
   ```

5. **Export data:**
   ```bash
   retrochat export --format json --provider claude
   ```

### Typical Use Cases

- **Personal Usage Tracking**: Monitor your LLM usage patterns across different providers with interactive charts
- **Project Analysis**: Understand which projects generate the most AI conversations using visual analytics
- **Historical Research**: Search through past conversations for specific topics or solutions
- **Data Migration**: Consolidate chat histories from multiple LLM tools into one database
- **Session Insights**: Generate AI-powered analysis and summaries of your conversations
- **Real-time Monitoring**: Watch and auto-import new conversations as they happen

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