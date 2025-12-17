# retrochat Development Guidelines

## Active Technologies

### Core Application (CLI/TUI)
- Rust 1.75+ with Ratatui, SQLite, Serde, Clap, Tokio (001-i-want-to)
- Rust 1.75+ (from existing project) + Ratatui (TUI), SQLite/SQLx (storage), Serde (serialization), Clap (CLI), Tokio (async), reqwest (HTTP client for Google AI) (002-add-retrospection-process)
- SQLite with SQLx migration from rusqlite (existing) (002-add-retrospection-process)

### Tauri Desktop Application
- Tauri 2.0 (Rust backend)
- React 19 with TypeScript
- Vite for build tooling
- shadcn/ui component library
- Tailwind CSS for styling
- Biome for code formatting and linting
- pnpm for package management

## Project Structure

The project uses a Cargo workspace with 5 separate packages:

```
crates/
├── retrochat-core/       # Core library
│   ├── src/              # Database, models, services, parsers, tools, utils
│   ├── migrations/       # SQL migrations
│   ├── .sqlx/            # SQLx metadata
│   └── tests/            # Unit and integration tests
│
├── retrochat-tui/        # Terminal UI library
│   └── src/              # TUI components, events, state
│
├── retrochat-cli/        # CLI binary (default, includes TUI)
│   ├── src/
│   │   ├── main.rs       # Entry point (launches TUI or CLI)
│   │   └── commands/     # CLI command handlers
│   └── tests/contract/   # CLI contract tests
│
├── retrochat-gui/        # Tauri desktop application
│   ├── src/              # Tauri Rust backend
│   ├── icons/            # App icons
│   └── tauri.conf.json   # Tauri configuration
│
└── retrochat-mcp/        # MCP server
    ├── src/
    │   ├── main.rs       # MCP server entry point
    │   ├── server.rs     # Server handler implementation
    │   └── tools/        # MCP tool implementations
    └── tests/            # Unit and integration tests

ui-react/                 # React frontend for Tauri desktop app
├── src/                  # React components and application code
├── package.json          # Frontend dependencies
├── biome.json            # Biome configuration
└── vite.config.js        # Vite build configuration
```

## Commands

### Development & Testing

#### Cargo Aliases (defined in .cargo/config.toml)
```bash
# Test aliases
cargo t              # Run test suite (test --workspace --verbose)
cargo tc             # Test core package (test -p retrochat-core --verbose)
cargo tcli           # Test CLI package (test -p retrochat-cli --verbose)

# Build aliases
cargo c              # Cargo check (check --workspace --verbose)
cargo b              # Cargo build (build --workspace)
cargo br             # Cargo build --release (build --release --workspace)

# Code quality
cargo fmt-check      # Check formatting (fmt --all -- --check)
cargo fmt-fix        # Apply formatting (fmt --all)
cargo clippy-strict  # Run clippy with -D warnings (clippy --workspace -- -D warnings)

# Package-specific run commands
cargo cli            # Run CLI (run -p retrochat-cli)
cargo tui            # Launch TUI (run -p retrochat-cli, same as cli)
cargo gui            # Run GUI (run -p retrochat-gui)
cargo mcp            # Run MCP server (run -p retrochat-mcp)
```

#### Shell Scripts (in ./scripts)
```bash
./scripts/clean-db.sh    # Remove retrochat database files
./scripts/doctor.sh      # Check system dependencies (rustc, cargo, python)
./scripts/e2e.sh         # Run end-to-end tests (generate + import examples)
```

### Direct Cargo Commands
```bash
# Build and test commands
cargo check --workspace && cargo test --workspace && cargo clippy --workspace
cargo run -p retrochat-cli                            # Launch TUI interface (default)

# Sync commands
cargo run -p retrochat-cli -- sync claude gemini      # Import from provider directories
cargo run -p retrochat-cli -- sync all                # Import from all providers
cargo run -p retrochat-cli -- sync --path /path/to/files  # Import from specific path
cargo run -p retrochat-cli -- sync --path /path/to/file.jsonl  # Import a single file
cargo run -p retrochat-cli -- sync claude -w --verbose     # Watch mode with detailed output
cargo run -p retrochat-cli -- sync all --watch --verbose   # Watch all providers

# Query commands
cargo run -p retrochat-cli -- list                    # List all sessions
cargo run -p retrochat-cli -- list --provider claude  # List sessions by provider
cargo run -p retrochat-cli -- show SESSION_ID         # Show session details
cargo run -p retrochat-cli -- search "query"          # Search messages

# Analysis commands (requires GOOGLE_AI_API_KEY env var)
cargo run -p retrochat-cli -- analysis run [SESSION_ID] [--all]  # Analyze sessions
cargo run -p retrochat-cli -- analysis show [SESSION_ID] [--all]  # View results
cargo run -p retrochat-cli -- analysis status [--all|--history|--watch]  # Check status
cargo run -p retrochat-cli -- analysis cancel [REQUEST_ID] [--all]  # Cancel operations

# Export commands
cargo run -p retrochat-cli -- export --format json    # Export to JSON
cargo run -- export --format jsonl                    # Export to JSONL
```

### Tauri Desktop Application

The project includes a Tauri desktop application with a React frontend.

#### Frontend Development (ui-react/)
```bash
cd ui-react

# Development
pnpm install           # Install dependencies
pnpm dev              # Start Vite dev server (for UI development only)

# Code quality
pnpm biome:check      # Check linting and formatting
pnpm biome:format     # Format code with Biome
pnpm biome:lint       # Lint code
pnpm biome:lint:fix   # Fix linting issues
pnpm biome:ci         # Run CI checks (format + lint)

# Build
pnpm build            # Build production bundle
```

#### Tauri Application (src-tauri/)

**Prerequisites**: Install Tauri CLI before development
```bash
cargo install tauri-cli
```

```bash
cd src-tauri

# Development
cargo tauri dev       # Run Tauri app in development mode (hot reload)

# Build
cargo tauri build     # Build production Tauri application

# Icon generation
cargo tauri icon path/to/icon.png  # Generate app icons from source image

# Testing
cargo test            # Run Tauri backend tests
cargo clippy          # Run clippy on Tauri code
```

#### Working with Both Modules

The Tauri application consists of two main parts:
1. **Backend (src-tauri/)**: Rust code that provides system integration and exposes commands to the frontend
2. **Frontend (ui-react/)**: React app that provides the UI, built with Vite + shadcn/ui + Tailwind CSS

Development workflow:
- The frontend (`ui-react/`) uses Biome for code formatting and linting (not Prettier/ESLint)
- The backend (`src-tauri/`) follows the same Rust conventions as the main CLI/TUI app
- Use `cargo tauri dev` to run both frontend and backend together with hot reload
- The Tauri backend can import and use the main `retrochat` library (from `src/`)

## Code Style
Rust: Follow standard rustfmt conventions, use constitutional TDD approach

## Development Rules

### Code Formatting (CRITICAL)
- **ALWAYS run `cargo fmt` before committing**: This is mandatory and must never be forgotten
- **Run `cargo fmt --check` during development**: Verify formatting before pushing
- **Format first, then commit**: Make it a habit - fmt → test → commit → push

### Code Quality Checks (CRITICAL)
- **Clippy with strict warnings**: All code must pass `cargo clippy -- -D warnings`
- **Zero warnings policy**: Clippy warnings are treated as errors with the `-D warnings` flag
- **CI enforcement**: The clippy check runs in CI with `-D warnings` to enforce code quality
- **Fix before committing**: Address all clippy suggestions before pushing code

### Test-Driven Development (TDD)
- **Sequential TDD**: Write one test at a time, then implement the corresponding functionality
- **No bulk testing**: Do not write all tests upfront - follow the red-green-refactor cycle strictly
- **One test, one implementation**: Each test should drive exactly one piece of implementation

### Architecture & Dependency Rules
- **Layer Dependencies**: Maintain strict dependency hierarchy: `Repo <- Service <- TUI/CLI`
- **No Direct Repo Access**: TUI and CLI modules must never directly access Repo layer
- **Service Layer**: All business logic must go through the Service layer

### Output & UI Rules
- **No stdout in Core Modules**: Repo, Service, and TUI modules must not use stdout directly
- **TUI Protection**: Avoid stdout usage to prevent TUI interface from breaking
- **Output Isolation**: Keep output handling separate from core business logic

### Database Migration Rules
- **SQLx Prepare**: After creating database migrations, use `sqlx prepare` to update .sqlx files appropriately
- **Migration Updates**: Ensure .sqlx files are kept in sync with schema changes

### Environment Variable Management
- **Centralized Constants**: All environment variable names are defined in `src/env.rs`
- **Organized by Category**: Environment variables are grouped into modules (logging, providers, apis, system, analytics)
- **Adding New Variables**: When adding or modifying environment variables, always:
  1. Add the constant to the appropriate module in `src/env.rs`
  2. Use the constant throughout the codebase instead of hardcoded strings
  3. Document the purpose and expected values in the constant's comment
- **Example Usage**:
  ```rust
  use crate::env::providers as env_vars;
  let dirs = std::env::var(env_vars::CLAUDE_DIRS)?;
  ```

### Tauri Frontend Development Rules (ui-react/)

#### Code Formatting & Linting (CRITICAL)
- **ALWAYS use Biome**: The frontend uses Biome, not Prettier or ESLint
- **Run before committing**: Always run `pnpm biome:format` and `pnpm biome:lint:fix` before committing
- **CI enforcement**: CI runs `pnpm biome:ci` to enforce code quality
- **Zero warnings policy**: All Biome warnings must be addressed before pushing

#### Frontend Architecture
- **Component Structure**: Use functional components with hooks
- **shadcn/ui Components**: Prefer using shadcn/ui components from `src/components/ui/`
- **Styling**: Use Tailwind CSS utility classes, follow the project's design system
- **Type Safety**: Always use TypeScript types, avoid `any` types

#### Tauri Integration
- **Backend Communication**: Use Tauri commands to communicate with the Rust backend
- **Shared Types**: Keep TypeScript types in sync with Rust types when communicating between frontend and backend
- **Error Handling**: Handle Tauri command errors gracefully in the UI

### MCP Server (retrochat-mcp/)

The project includes a Model Context Protocol (MCP) server that exposes RetroChat's query and analytics capabilities to AI assistants like Claude, Cursor, and others.

#### What is the MCP Server?
The MCP server provides a read-only interface for AI assistants to:
- Query and filter chat sessions
- Search messages across all sessions
- Retrieve detailed session information including messages
- Access analytics data for sessions

#### Running the MCP Server
```bash
# Using cargo alias
cargo mcp

# Or directly
cargo run -p retrochat-mcp

# With logging (logs go to stderr, won't interfere with stdio transport)
RUST_LOG=debug cargo mcp
```

#### Available Tools
The server exposes 4 MCP tools:

1. **list_sessions**: Query and filter chat sessions
   - Supports filtering by provider, project, date range, message count
   - Pagination support
   - Sortable by various fields

2. **get_session_detail**: Get full session details including all messages
   - Requires session UUID
   - Returns complete message history

3. **search_messages**: Full-text search across all messages
   - Supports filtering by providers, projects, date range
   - Pagination support
   - Returns message snippets with context

4. **get_session_analytics**: Get analytics for a specific session
   - Requires session UUID
   - Returns completed analytics or pending status

#### AI Assistant Configuration

**For Claude Desktop** (`~/Library/Application Support/Claude/claude_desktop_config.json`):
```json
{
  "mcpServers": {
    "retrochat": {
      "command": "/absolute/path/to/retrochat-mcp",
      "args": [],
      "env": {
        "RUST_LOG": "info"
      }
    }
  }
}
```

**For Cursor** (`.cursor/mcp.json` in project):
```json
{
  "mcpServers": {
    "retrochat": {
      "command": "cargo",
      "args": ["run", "-p", "retrochat-mcp"],
      "cwd": "/absolute/path/to/retrochat"
    }
  }
}
```

#### Development Notes
- The MCP server is read-only (no write operations)
- Uses stdio transport for communication
- Logs to stderr to avoid interfering with MCP protocol
- Shares the same database as CLI/TUI (uses default database path)
- All responses are pretty-printed JSON for easy AI consumption

#### Testing
```bash
# Run MCP server tests
cargo tmcp

# Or full test command
cargo test -p retrochat-mcp --verbose
```

<!-- MANUAL ADDITIONS END -->
