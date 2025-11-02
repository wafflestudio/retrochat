# retrochat Development Guidelines

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

### Development & Testing

#### Cargo Aliases (defined in .cargo/config.toml)
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
cargo tui            # Launch TUI interface (run -- tui)
cargo watch          # Watch all providers with verbose output (run -- watch all --verbose)
cargo init           # Initialize retrochat (run -- init)
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
cargo check && cargo test && cargo clippy
cargo run -- tui                                      # Launch TUI interface

# Import commands
cargo run -- import claude gemini                     # Import from provider directories
cargo run -- import codex                             # Import from other providers
cargo run -- import --path /path/to/files             # Import from specific path
cargo run -- import --path /path/to/file.jsonl        # Import a single file

# Watch commands
cargo run -- watch all --verbose                      # Watch all providers with detailed output
cargo run -- watch claude gemini --verbose            # Watch specific providers
cargo run -- watch --path /path/to/files --verbose    # Watch specific path

# Analytics commands
cargo run -- analyze insights                         # Generate usage insights
cargo run -- analyze export json                      # Export to JSON
cargo run -- analyze export csv                       # Export to CSV

# Analytics commands (requires GOOGLE_AI_API_KEY env var)
cargo run -- analytics execute [SESSION_ID] [--all] [--custom-prompt PROMPT]  # Analyze sessions
cargo run -- analytics show [SESSION_ID] [--all] [--format text|json|markdown]  # View results
cargo run -- analytics status [--all|--history|--watch]      # Check analysis status
cargo run -- analytics cancel [REQUEST_ID] [--all]   # Cancel operations
```

## Code Style
Rust: Follow standard rustfmt conventions, use constitutional TDD approach

## Development Rules

### Code Formatting (CRITICAL)
- **ALWAYS run `cargo fmt` before committing**: This is mandatory and must never be forgotten
- **Run `cargo fmt --check` during development**: Verify formatting before pushing
- **Format first, then commit**: Make it a habit - fmt → test → commit → push

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

<!-- MANUAL ADDITIONS END -->
