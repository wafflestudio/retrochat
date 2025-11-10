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
```
src/
├── models/       # Data structures and database models
├── services/     # Business logic and file processing
├── cli/          # Command-line interface
├── tui/          # Terminal user interface
└── lib/          # Shared utilities

src-tauri/        # Tauri desktop application backend
├── src/          # Tauri Rust application code
├── Cargo.toml    # Tauri dependencies
└── tauri.conf.json # Tauri configuration

ui-react/         # React frontend for Tauri desktop app
├── src/          # React components and application code
├── package.json  # Frontend dependencies
├── biome.json    # Biome configuration for linting/formatting
└── vite.config.js # Vite build configuration

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
cargo tui            # Launch TUI interface (run -- same as cargo run)
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
cargo run                                             # Launch TUI interface (default)

# Sync commands
cargo run -- sync claude gemini                       # Import from provider directories
cargo run -- sync all                                 # Import from all providers
cargo run -- sync --path /path/to/files               # Import from specific path
cargo run -- sync --path /path/to/file.jsonl          # Import a single file
cargo run -- sync claude -w --verbose                 # Watch mode with detailed output
cargo run -- sync all --watch --verbose               # Watch all providers

# Query commands
cargo run -- list                                     # List all sessions
cargo run -- list --provider claude                   # List sessions by provider
cargo run -- show SESSION_ID                          # Show session details
cargo run -- search "query"                           # Search messages

# Analysis commands (requires GOOGLE_AI_API_KEY env var)
cargo run -- analysis run [SESSION_ID] [--all] [--custom-prompt PROMPT]  # Analyze sessions
cargo run -- analysis show [SESSION_ID] [--all] [--format text|json|markdown]  # View results
cargo run -- analysis status [--all|--history|--watch]      # Check analysis status
cargo run -- analysis cancel [REQUEST_ID] [--all]    # Cancel operations

# Export commands
cargo run -- export --format json                     # Export to JSON
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
```bash
cd src-tauri

# Development
cargo tauri dev       # Run Tauri app in development mode (hot reload)

# Build
cargo tauri build     # Build production Tauri application

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

<!-- MANUAL ADDITIONS END -->
