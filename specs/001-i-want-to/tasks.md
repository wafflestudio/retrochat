# Tasks: LLM Agent Chat History Retrospect Application

**Input**: Design documents from `/Users/sanggggg/Project/retrochat/specs/001-i-want-to/`
**Prerequisites**: plan.md (✓), research.md (✓), data-model.md (✓), contracts/ (✓), quickstart.md (✓)

## Execution Flow (main)
```
1. Load plan.md from feature directory
   → Tech stack: Rust 1.75+, Ratatui, SQLite, Serde, Clap, Tokio
   → Structure: Single desktop application with src/ and tests/
2. Load design documents:
   → data-model.md: 5 entities (ChatSession, Message, Project, UsageAnalysis, LlmProvider)
   → contracts/: 3 API files (import_api.json, query_api.json, analytics_api.json)
   → quickstart.md: User journey scenarios for validation
3. Generate tasks by category:
   → Setup: Cargo project, dependencies, structure
   → Tests: Contract tests (3), integration tests (4), unit tests
   → Core: Models (5), parsers (2), database layer, CLI, TUI
   → Integration: SQLite migrations, async processing, export system
   → Polish: Performance optimization, documentation
4. Apply task rules:
   → Different modules = mark [P] for parallel
   → Same file = sequential (no [P])
   → Tests before implementation (TDD)
5. Number tasks sequentially (T001-T038)
6. All contracts have tests ✓, all entities have models ✓, TDD workflow ✓
```

## Format: `[ID] [P?] Description`
- **[P]**: Can run in parallel (different files, no dependencies)
- Include exact file paths in descriptions

## Path Conventions
- **Single project**: `src/`, `tests/` at repository root
- Rust module structure: `src/models/`, `src/services/`, `src/cli/`, `src/tui/`

## Phase 3.1: Project Setup
- [x] T001 Create Rust project structure with Cargo.toml and dependencies
- [x] T002 Initialize Git repository with .gitignore for Rust projects
- [x] T003 [P] Configure Rust linting tools (clippy, rustfmt) and CI workflow

## Phase 3.2: Tests First (TDD) ⚠️ MUST COMPLETE BEFORE 3.3
**CRITICAL: These tests MUST be written and MUST FAIL before ANY implementation**

### Contract Tests (Based on contracts/ APIs)
- [ ] T004 [P] Contract test import scan endpoint in tests/contract/test_import_scan.rs
- [ ] T005 [P] Contract test import file endpoint in tests/contract/test_import_file.rs
- [ ] T006 [P] Contract test import batch endpoint in tests/contract/test_import_batch.rs
- [ ] T007 [P] Contract test sessions query endpoint in tests/contract/test_sessions_query.rs
- [ ] T008 [P] Contract test session detail endpoint in tests/contract/test_session_detail.rs
- [ ] T009 [P] Contract test search endpoint in tests/contract/test_search.rs
- [ ] T010 [P] Contract test analytics usage endpoint in tests/contract/test_analytics_usage.rs
- [ ] T011 [P] Contract test analytics insights endpoint in tests/contract/test_analytics_insights.rs
- [ ] T012 [P] Contract test analytics export endpoint in tests/contract/test_analytics_export.rs

### Integration Tests (Based on quickstart scenarios)
- [ ] T013 [P] Integration test first-time user setup in tests/integration/test_first_time_setup.rs
- [ ] T014 [P] Integration test daily usage analysis in tests/integration/test_daily_analysis.rs
- [ ] T015 [P] Integration test session deep dive in tests/integration/test_session_detail.rs
- [ ] T016 [P] Integration test export and reporting in tests/integration/test_export_reporting.rs

## Phase 3.3: Core Implementation (ONLY after tests are failing)

### Database Schema and Models (Based on data-model.md entities)
- [x] T017 [P] ChatSession model struct with Serde derives in src/models/chat_session.rs
- [x] T018 [P] Message model struct with Serde derives in src/models/message.rs
- [x] T019 [P] Project model struct with Serde derives in src/models/project.rs
- [x] T020 [P] UsageAnalysis model struct with Serde derives in src/models/usage_analysis.rs
- [x] T021 [P] LlmProvider model struct with Serde derives in src/models/llm_provider.rs
- [x] T022 SQLite database schema creation in src/database/schema.rs
- [x] T023 Database migration system in src/database/migrations.rs

### File Parsing Services
- [x] T024 [P] Claude Code JSONL parser with streaming support in src/parsers/claude_code.rs
- [x] T025 [P] Gemini JSON parser with memory mapping in src/parsers/gemini.rs
- [x] T026 Parser registry and file detection logic in src/parsers/mod.rs

### Database Layer
- [x] T027 Database connection manager with rusqlite in src/database/connection.rs
- [x] T028 ChatSession repository with CRUD operations in src/database/chat_session_repo.rs
- [x] T029 Message repository with full-text search in src/database/message_repo.rs
- [x] T030 Analytics queries and aggregation functions in src/database/analytics_repo.rs

### CLI Interface (Based on quickstart commands)
- [x] T031 Main CLI structure with Clap derives in src/cli/mod.rs
- [x] T032 Import subcommands (scan, file, batch) in src/cli/import.rs
- [x] T033 TUI launcher and database initialization in src/cli/tui.rs
- [x] T034 Analytics and export subcommands in src/cli/analytics.rs

## Phase 3.4: TUI Implementation
- [ ] T035 TUI application state and event handling in src/tui/app.rs
- [ ] T036 Session list view with filtering and sorting in src/tui/session_list.rs
- [ ] T037 Session detail view with message display in src/tui/session_detail.rs
- [ ] T038 Analytics dashboard with charts and insights in src/tui/analytics.rs

## Phase 3.5: Integration & Async Processing
- [ ] T039 Async file import pipeline with Tokio in src/services/import_service.rs
- [ ] T040 Background analytics computation in src/services/analytics_service.rs
- [ ] T041 Export service for multiple formats in src/services/export_service.rs
- [ ] T042 Error handling and logging configuration in src/lib.rs

## Phase 3.6: Polish & Optimization
- [ ] T043 [P] Unit tests for parser modules in tests/unit/test_parsers.rs
- [ ] T044 [P] Unit tests for database repositories in tests/unit/test_repositories.rs
- [ ] T045 [P] Unit tests for analytics algorithms in tests/unit/test_analytics.rs
- [ ] T046 Performance optimization for large file processing in src/services/performance.rs
- [ ] T047 [P] Documentation generation with cargo doc in docs/
- [ ] T048 End-to-end validation using quickstart scenarios in tests/e2e/

## Dependencies
```
Setup (T001-T003) → Tests (T004-T016) → Models (T017-T021) → Database (T022-T030) →
CLI (T031-T034) → TUI (T035-T038) → Services (T039-T042) → Polish (T043-T048)
```

### Critical Path
- T001 (project setup) blocks everything
- T004-T016 (all tests) must complete before any implementation
- T017-T021 (models) block T022-T030 (database layer)
- T022-T030 (database) blocks T031-T042 (application layer)
- T031-T034 (CLI) and T035-T038 (TUI) can run in parallel
- T039-T042 (services) require both CLI and database completion

## Parallel Execution Examples

### Phase 3.2: Contract Tests (Run all in parallel)
```bash
# Launch T004-T012 together (different contract files):
Task: "Contract test import scan endpoint in tests/contract/test_import_scan.rs"
Task: "Contract test import file endpoint in tests/contract/test_import_file.rs"
Task: "Contract test import batch endpoint in tests/contract/test_import_batch.rs"
Task: "Contract test sessions query endpoint in tests/contract/test_sessions_query.rs"
Task: "Contract test session detail endpoint in tests/contract/test_session_detail.rs"
Task: "Contract test search endpoint in tests/contract/test_search.rs"
Task: "Contract test analytics usage endpoint in tests/contract/test_analytics_usage.rs"
Task: "Contract test analytics insights endpoint in tests/contract/test_analytics_insights.rs"
Task: "Contract test analytics export endpoint in tests/contract/test_analytics_export.rs"
```

### Phase 3.2: Integration Tests (Run all in parallel)
```bash
# Launch T013-T016 together (different test scenarios):
Task: "Integration test first-time user setup in tests/integration/test_first_time_setup.rs"
Task: "Integration test daily usage analysis in tests/integration/test_daily_analysis.rs"
Task: "Integration test session deep dive in tests/integration/test_session_detail.rs"
Task: "Integration test export and reporting in tests/integration/test_export_reporting.rs"
```

### Phase 3.3: Model Creation (Run all in parallel)
```bash
# Launch T017-T021 together (different model files):
Task: "ChatSession model struct with Serde derives in src/models/chat_session.rs"
Task: "Message model struct with Serde derives in src/models/message.rs"
Task: "Project model struct with Serde derives in src/models/project.rs"
Task: "UsageAnalysis model struct with Serde derives in src/models/usage_analysis.rs"
Task: "LlmProvider model struct with Serde derives in src/models/llm_provider.rs"
```

### Phase 3.3: File Parsers (Run in parallel)
```bash
# Launch T024-T025 together (different parser files):
Task: "Claude Code JSONL parser with streaming support in src/parsers/claude_code.rs"
Task: "Gemini JSON parser with memory mapping in src/parsers/gemini.rs"
```

## Constitutional Compliance Notes
- **Build Validation**: Run `cargo check && cargo test && cargo clippy` after EVERY task completion
- **Test-First**: All contract and integration tests (T004-T016) MUST fail before implementation
- **Local Processing**: All parsers and services maintain read-only access to source files
- **Performance**: Tasks T046+ focus on meeting <1s import and <100ms UI response targets
- **Privacy**: No external API calls, all processing happens locally

## Task Validation Checklist
- [x] All 3 contract files have corresponding tests (T004-T012)
- [x] All 5 entities have model creation tasks (T017-T021)
- [x] All quickstart scenarios have integration tests (T013-T016)
- [x] TDD workflow enforced (tests before implementation)
- [x] Parallel tasks target different files with no dependencies
- [x] Each task specifies exact file path for implementation
- [x] Dependencies clearly mapped and enforced
- [x] Constitutional build validation integrated throughout

## Notes
- Total of 48 tasks covering complete Rust TUI application
- Estimated 2-3 weeks for full implementation following TDD
- Each task is atomic and executable by LLM agents
- Parallel execution can reduce timeline by 60% in test phases
- Build validation ensures constitutional compliance at every step