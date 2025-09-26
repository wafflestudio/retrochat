# retrochat Development Guidelines

Auto-generated from all feature plans. Last updated: 2025-09-21

## Active Technologies
- Rust 1.75+ with Ratatui, SQLite, Serde, Clap, Tokio (001-i-want-to)
- Rust 1.75+ (edition 2021) + Tokio (async), Reqwest (HTTP client for Gemini API), Serde (JSON), SQLite (Rusqlite), Ratatui (TUI), Clap (CLI) (002-i-want-to)
- SQLite database with existing schema (chat_sessions, messages tables) (002-i-want-to)

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
# Build and test commands
cargo check && cargo test && cargo clippy
cargo run -- tui                    # Launch TUI interface
cargo run -- import scan            # Scan for chat files
cargo run -- analyze insights       # Generate usage insights

## Code Style
Rust: Follow standard rustfmt conventions, use constitutional TDD approach

## Recent Changes
- 002-i-want-to: Added Rust 1.75+ (edition 2021) + Tokio (async), Reqwest (HTTP client for Gemini API), Serde (JSON), SQLite (Rusqlite), Ratatui (TUI), Clap (CLI)
- 001-i-want-to: Added Rust TUI app for LLM chat history analysis with SQLite persistence

<!-- MANUAL ADDITIONS START -->
<!-- MANUAL ADDITIONS END -->
