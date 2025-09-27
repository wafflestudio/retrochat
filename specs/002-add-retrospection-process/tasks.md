# Tasks: Add Retrospection Process for Chat Sessions

**Input**: Design documents from `/specs/002-add-retrospection-process/`
**Prerequisites**: plan.md (required), research.md, data-model.md, contracts/

## Execution Flow (main)
```
1. Load plan.md from feature directory
   ✓ Tech stack: Rust 1.75+, Ratatui, SQLx, Serde, Clap, Tokio, reqwest
   ✓ Structure: Single project (TUI application with CLI interface)
2. Load optional design documents:
   ✓ data-model.md: RetrospectRequest, Retrospection entities
   ✓ contracts/: CLI, TUI, Google AI API interfaces
   ✓ research.md: reqwest+Tokio, channels, exponential backoff
3. Generate tasks by category:
   ✓ Setup: dependencies, database migrations
   ✓ Tests: contract tests, integration tests
   ✓ Core: models, services, Google AI client
   ✓ Integration: CLI commands, TUI widgets
   ✓ Polish: error handling, documentation
4. Apply task rules:
   ✓ Different files = mark [P] for parallel
   ✓ Same file = sequential (no [P])
   ✓ Tests before implementation (TDD)
5. Number tasks sequentially (T001, T002...)
6. Generate dependency graph
7. Create parallel execution examples
8. Validate task completeness
9. Return: SUCCESS (tasks ready for execution)
```

## Format: `[ID] [P?] Description`
- **[P]**: Can run in parallel (different files, no dependencies)
- Include exact file paths in descriptions

## Path Conventions
Single project structure (from plan.md):
- **Source**: `src/models/`, `src/services/`, `src/cli/`, `src/tui/`
- **Tests**: `tests/unit/`, `tests/integration/`, `tests/contract/`
- **Migrations**: Database migrations in existing SQLx structure

## Phase 3.1: Setup
- [x] T001 Add retrospection dependencies to Cargo.toml (reqwest, backoff, tokio-util, uuid)
- [x] T002 [P] Create database migration for retrospect_requests table in migrations/
- [x] T003 [P] Create database migration for retrospections table in migrations/
- [x] T004 [P] Configure clippy and formatting rules for retrospection module

## Phase 3.2: Tests First (TDD) ⚠️ MUST COMPLETE BEFORE 3.3
**CRITICAL: These tests MUST be written and MUST FAIL before ANY implementation**

### Contract Tests
- [x] T005 [P] Contract test Google AI API request/response in tests/contract/test_google_ai_api.rs
- [x] T006 [P] Contract test CLI retrospect execute command in tests/contract/test_cli_retrospect_execute.rs
- [x] T007 [P] Contract test CLI retrospect show command in tests/contract/test_cli_retrospect_show.rs
- [x] T008 [P] Contract test CLI retrospect status command in tests/contract/test_cli_retrospect_status.rs
- [x] T009 [P] Contract test CLI retrospect cancel command in tests/contract/test_cli_retrospect_cancel.rs

### Integration Tests
- [x] T010 [P] Integration test single session analysis workflow in tests/integration/test_single_session_analysis.rs
- [x] T012 [P] Integration test background operation management in tests/integration/test_background_operations.rs
- [x] T013 [P] Integration test error handling and recovery in tests/integration/test_error_handling.rs
- [x] T014 [P] Integration test TUI retrospection widgets in tests/integration/test_tui_retrospection.rs

## Phase 3.3: Core Implementation (ONLY after tests are failing)

### Data Models
- [x] T015 [P] RetrospectRequest model in src/models/retrospect_request.rs
- [x] T016 [P] Retrospection model in src/models/retrospection.rs
- [x] T017 [P] AnalysisType enum in src/models/analysis_type.rs (integrated in retrospect_request.rs)
- [x] T018 [P] OperationStatus enum in src/models/operation_status.rs (integrated in retrospect_request.rs)

### Google AI Integration
- [x] T019 [P] Google AI client in src/services/google_ai/client.rs
- [x] T020 [P] Request/response models in src/services/google_ai/models.rs
- [x] T021 [P] Error types and handling in src/services/google_ai/errors.rs
- [x] T022 [P] Retry logic with exponential backoff in src/services/google_ai/retry.rs

### Background Operations
- [x] T023 [P] Background operation manager in src/services/background/operation_manager.rs
- [x] T024 [P] Progress tracking and reporting in src/services/background/progress.rs (integrated in operation_manager.rs)
- [x] T025 [P] Task cancellation support in src/services/background/cancellation.rs (integrated in operation_manager.rs)

### Repository Layer
- [x] T026 [P] RetrospectRequest repository in src/database/retrospect_request_repo.rs
- [ ] T027 [P] Retrospection repository in src/database/retrospection_repo.rs

### Core Service
- [ ] T028 Retrospection service orchestration in src/services/retrospection_service.rs
- [ ] T029 Analysis pipeline and workflow in src/services/analysis_pipeline.rs

## Phase 3.4: CLI Interface

### Command Implementation
- [ ] T030 [P] CLI retrospect execute subcommand in src/cli/commands/retrospect_execute.rs
- [ ] T031 [P] CLI retrospect show subcommand in src/cli/commands/retrospect_show.rs
- [ ] T032 [P] CLI retrospect status subcommand in src/cli/commands/retrospect_status.rs
- [ ] T033 [P] CLI retrospect cancel subcommand in src/cli/commands/retrospect_cancel.rs
- [ ] T034 CLI retrospect command group integration in src/cli/retrospect.rs
- [ ] T035 Main CLI integration for retrospect commands in src/cli/mod.rs

### Output Formatting
- [ ] T036 [P] CLI output formatting for analysis results in src/cli/formatters/retrospection_formatter.rs
- [ ] T037 [P] Progress display for CLI operations in src/cli/formatters/progress_formatter.rs

## Phase 3.5: TUI Integration

### Widgets
- [ ] T038 [P] Progress indicator widget in src/tui/widgets/progress_widget.rs
- [ ] T039 [P] Retrospection status panel in src/tui/widgets/retrospection_panel.rs
- [ ] T040 [P] Session detail retrospection section in src/tui/widgets/session_retrospection.rs

### State Management
- [ ] T041 TUI state integration for retrospection in src/tui/app/retrospection_state.rs
- [ ] T042 Event handling for retrospection actions in src/tui/app/retrospection_events.rs

### User Interface
- [ ] T043 Session list retrospection shortcuts in src/tui/screens/session_list.rs
- [ ] T044 Session detail panel updates in src/tui/screens/session_detail.rs
- [ ] T045 Retrospection management screen in src/tui/screens/retrospection_management.rs

## Phase 3.6: Integration & Polish

### System Integration
- [ ] T046 Database connection and migration integration in src/database/mod.rs
- [ ] T047 Configuration management for retrospection settings in src/config/retrospection.rs
- [ ] T048 Error handling and logging integration in src/lib.rs

### Privacy & Security
- [ ] T049 User consent dialog implementation in src/services/privacy/consent.rs
- [ ] T050 Data filtering and sanitization in src/services/privacy/data_filter.rs

### Performance & Reliability
- [ ] T051 [P] Unit tests for Google AI client in tests/unit/test_google_ai_client.rs
- [ ] T052 [P] Unit tests for background operations in tests/unit/test_background_operations.rs
- [ ] T053 [P] Unit tests for repositories in tests/unit/test_repositories.rs
- [ ] T054 [P] Unit tests for CLI commands in tests/unit/test_cli_commands.rs
- [ ] T055 Performance optimization and caching in src/services/performance/cache.rs

### Documentation & Validation
- [ ] T056 [P] API documentation updates in docs/api/retrospection.md
- [ ] T057 [P] User guide documentation in docs/user/retrospection_guide.md
- [ ] T058 Manual testing following quickstart scenarios
- [ ] T059 Build validation and clippy fixes
- [ ] T060 Final integration testing and error scenarios

## Dependencies

### Setup Dependencies
- T001 blocks T005-T060 (need dependencies)
- T002, T003 block T015-T017, T026-T027 (need database schema)

### Test Dependencies (TDD)
- T005-T014 MUST complete before T015-T060 (tests before implementation)

### Model Dependencies
- T015-T018 block T019-T029 (models before services)
- T019-T025 block T028-T029 (components before orchestration)

### Service Dependencies
- T026-T029 block T030-T037 (services before CLI)
- T028-T029 block T038-T045 (services before TUI)

### Integration Dependencies
- T030-T037 block T046-T048 (CLI before system integration)
- T038-T045 block T046-T048 (TUI before system integration)
- T046-T050 block T051-T060 (integration before polish)

## Parallel Execution Examples

### Phase 3.1 Setup (All Parallel)
```bash
# All setup tasks can run in parallel
Task: "Add retrospection dependencies to Cargo.toml (reqwest, backoff, tokio-util, uuid)"
Task: "Create database migration for retrospect_requests table in migrations/"
Task: "Create database migration for retrospections table in migrations/"
Task: "Configure clippy and formatting rules for retrospection module"
```

### Phase 3.2 Contract Tests (All Parallel)
```bash
# All contract tests can run in parallel
Task: "Contract test Google AI API request/response in tests/contract/test_google_ai_api.rs"
Task: "Contract test CLI retrospect execute command in tests/contract/test_cli_retrospect_execute.rs"
Task: "Contract test CLI retrospect show command in tests/contract/test_cli_retrospect_show.rs"
Task: "Contract test CLI retrospect status command in tests/contract/test_cli_retrospect_status.rs"
Task: "Contract test CLI retrospect cancel command in tests/contract/test_cli_retrospect_cancel.rs"
```

### Phase 3.2 Integration Tests (All Parallel)
```bash
# All integration tests can run in parallel
Task: "Integration test single session analysis workflow in tests/integration/test_single_session_analysis.rs"
Task: "Integration test background operation management in tests/integration/test_background_operations.rs"
Task: "Integration test error handling and recovery in tests/integration/test_error_handling.rs"
Task: "Integration test TUI retrospection widgets in tests/integration/test_tui_retrospection.rs"
```

### Phase 3.3 Models (All Parallel)
```bash
# All model files can be created in parallel
Task: "RetrospectRequest model in src/models/retrospect_request.rs"
Task: "Retrospection model in src/models/retrospection.rs"
Task: "AnalysisType enum in src/models/analysis_type.rs"
Task: "OperationStatus enum in src/models/operation_status.rs"
```

### Phase 3.3 Google AI Components (All Parallel)
```bash
# All Google AI components can be developed in parallel
Task: "Google AI client in src/services/google_ai/client.rs"
Task: "Request/response models in src/services/google_ai/models.rs"
Task: "Error types and handling in src/services/google_ai/errors.rs"
Task: "Retry logic with exponential backoff in src/services/google_ai/retry.rs"
```

## Notes
- [P] tasks = different files, no dependencies
- Verify tests fail before implementing
- Commit after each task or logical group
- Focus on TDD: tests must be written first and must fail
- Follow existing project patterns for consistency
- Use existing SQLx migration system
- Integrate with existing CLI and TUI architecture

## Task Generation Rules Applied

1. **From Contracts**:
   ✓ CLI interface → 4 contract tests + 4 implementation tasks
   ✓ TUI interface → 1 integration test + 5 TUI tasks
   ✓ Google AI API → 1 contract test + 4 implementation tasks

2. **From Data Model**:
   ✓ RetrospectRequest entity → model + repository tasks
   ✓ Retrospection entity → model + repository tasks
   ✓ Enums → separate model tasks

3. **From User Stories (quickstart.md)**:
   ✓ Single session analysis → integration test
   ✓ Batch analysis → integration test
   ✓ Background operations → integration test
   ✓ Error handling → integration test

4. **Ordering**:
   ✓ Setup → Tests → Models → Services → CLI → TUI → Integration → Polish
   ✓ Dependencies properly mapped

## Validation Checklist

- [x] All contracts have corresponding tests (T005-T009, T014)
- [x] All entities have model tasks (T015-T018)
- [x] All tests come before implementation (T005-T014 before T015+)
- [x] Parallel tasks truly independent (different files)
- [x] Each task specifies exact file path
- [x] No task modifies same file as another [P] task
- [x] TDD approach enforced (tests must fail first)
- [x] Constitutional compliance (privacy, testing, build validation)
- [x] Integration with existing codebase patterns