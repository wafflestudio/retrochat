# retrochat Development Guidelines

Auto-generated from all feature plans. Last updated: 2025-09-21

## Active Technologies
- Rust 1.75+ with Ratatui, SQLite, Serde, Clap, Tokio (001-i-want-to)
- Rust 1.75+ (from existing project) + Ratatui (TUI), SQLite/SQLx (storage), Serde (serialization), Clap (CLI), Tokio (async), reqwest (HTTP client for Google AI) (002-add-retrospection-process)
- SQLite with SQLx migration from rusqlite (existing) (002-add-retrospection-process)

## Project Structure
```
src/
├── models/       # Data structures and database models
├── services/     # Business logic and file processing
├── cli/          # Command-line interface
├── tui/          # Terminal user interface
└── lib/          # Shared utilities

tests/
├── contract/     # API contract tests
├── integration/  # Integration tests
└── unit/         # Unit tests
```

## Commands

### Development & Testing (using Makefile)
```bash
make help          # Show all available targets
make test          # Run test suite (like CI)
make clippy        # Run clippy with -D warnings (like CI)
make fmt           # Check formatting with rustfmt --check
make fmt-fix       # Apply formatting changes
make fix           # Apply rustfmt and clippy fixes (requires nightly for clippy --fix)
make check         # Cargo check
make build         # Cargo build
make build-release # Cargo build --release
make ci            # Run fmt, clippy, then tests (full CI validation)
```

### Direct Cargo Commands
```bash
# Build and test commands
cargo check && cargo test && cargo clippy
cargo run -- tui                                      # Launch TUI interface

# Import commands
cargo run -- import claude cursor                     # Import from provider directories
cargo run -- import gemini codex                      # Import from other providers
cargo run -- import --path /path/to/files             # Import from specific path
cargo run -- import --path /path/to/file.jsonl        # Import a single file

# Analytics commands
cargo run -- analyze insights                         # Generate usage insights
cargo run -- analyze export json                      # Export to JSON
cargo run -- analyze export csv                       # Export to CSV

# Retrospection commands (requires GOOGLE_AI_API_KEY env var)
cargo run -- retrospect execute [SESSION_ID] --analysis-type [TYPE]  # Analyze sessions
cargo run -- retrospect show [SESSION_ID] --format [text|json|markdown]  # View results
cargo run -- retrospect status [--all|--history]      # Check analysis status
cargo run -- retrospect cancel [REQUEST_ID] [--all]   # Cancel operations
```

## Code Style
Rust: Follow standard rustfmt conventions, use constitutional TDD approach

## Recent Changes
- 002-add-retrospection-process: COMPLETED - Added retrospection analysis with Google AI integration, CLI interface (execute/show/status/cancel), simplified approach without complex background operations
- 001-i-want-to: Added Rust TUI app for LLM chat history analysis with SQLite persistence

<!-- MANUAL ADDITIONS START -->

## Development Rules

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

<!-- MANUAL ADDITIONS END -->
