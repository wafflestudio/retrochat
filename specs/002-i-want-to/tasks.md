# Tasks: LLM-Powered Chat Session Retrospection

**Input**: Design documents from `/specs/002-i-want-to/`
**Prerequisites**: plan.md (required), research.md, data-model.md, contracts/

## Execution Flow (main)
```
1. Load plan.md from feature directory
   → If not found: ERROR "No implementation plan found"
   → Extract: tech stack, libraries, structure
2. Load optional design documents:
   → data-model.md: Extract entities → model tasks
   → contracts/: Each file → contract test task
   → research.md: Extract decisions → setup tasks
3. Generate tasks by category:
   → Setup: project init, dependencies, linting
   → Tests: contract tests, integration tests
   → Core: models, services, CLI commands
   → Integration: DB, middleware, logging
   → Polish: unit tests, performance, docs
4. Apply task rules:
   → Different files = mark [P] for parallel
   → Same file = sequential (no [P])
   → Tests before implementation (TDD)
5. Number tasks sequentially (T001, T002...)
6. Generate dependency graph
7. Create parallel execution examples
8. Validate task completeness:
   → All contracts have tests?
   → All entities have models?
   → All endpoints implemented?
9. Return: SUCCESS (tasks ready for execution)
```

## Format: `[ID] [P?] Description`
- **[P]**: Can run in parallel (different files, no dependencies)
- Include exact file paths in descriptions

## Path Conventions
- **Single project**: `src/`, `tests/` at repository root
- Paths assume single Rust project structure from existing codebase

## Phase 3.1: Setup
- [x] T001 Add new dependencies to Cargo.toml: reqwest, toml, directories, regex
- [x] T002 Create database migration for retrospection schema in src/database/migrations.rs
- [x] T003 [P] Configure environment variable validation for GEMINI_API_KEY

## Phase 3.2: Tests First (TDD) ⚠️ MUST COMPLETE BEFORE 3.3
**CRITICAL: These tests MUST be written and MUST FAIL before ANY implementation**
- [x] T004 [P] Contract test Gemini API generateContent in tests/contract/test_gemini_api.rs
- [x] T005 [P] Contract test retrospection service analyze in tests/contract/test_retrospection_service.rs
- [x] T006 [P] Integration test basic session analysis in tests/integration/test_session_analysis.rs
- [x] T007 [P] Integration test custom prompt analysis in tests/integration/test_custom_prompt.rs
- [x] T008 [P] Integration test TUI retrospection interface in tests/integration/test_tui_retrospection.rs
- [x] T009 [P] Integration test prompt template management in tests/integration/test_prompt_templates.rs

## Phase 3.3: Core Models (ONLY after tests are failing)
- [x] T010 [P] RetrospectionAnalysis model in src/models/retrospection_analysis.rs
- [x] T011 [P] PromptTemplate model in src/models/prompt_template.rs
- [x] T012 [P] AnalysisRequest model in src/models/analysis_request.rs
- [x] T013 [P] AnalysisMetadata model in src/models/analysis_metadata.rs
- [x] T014 [P] AnalysisStatus and RequestStatus enums in src/models/mod.rs

## Phase 3.4: Database Layer
- [ ] T015 [P] RetrospectionAnalysisRepository in src/database/retrospection_repo.rs
- [ ] T016 [P] PromptTemplateRepository in src/database/prompt_template_repo.rs
- [ ] T017 [P] AnalysisRequestRepository in src/database/analysis_request_repo.rs
- [ ] T018 Update database schema.rs with new table definitions
- [ ] T019 Run database migrations and seed default prompt templates

## Phase 3.5: Core Services
- [ ] T020 GeminiClient HTTP service in src/services/gemini_client.rs
- [ ] T021 PromptService for template management in src/services/prompt_service.rs
- [ ] T022 RetrospectionService orchestration in src/services/retrospection_service.rs
- [ ] T023 Integrate RetrospectionService with existing AnalyticsService in src/services/analytics_service.rs

## Phase 3.6: CLI Commands
- [ ] T024 [P] CLI analyze retrospect command in src/cli/analytics.rs
- [ ] T025 [P] CLI analyze show command in src/cli/analytics.rs
- [ ] T026 [P] CLI prompts list command in src/cli/prompts.rs
- [ ] T027 [P] CLI prompts create command in src/cli/prompts.rs
- [ ] T028 [P] CLI prompts edit command in src/cli/prompts.rs
- [ ] T029 [P] CLI prompts delete command in src/cli/prompts.rs
- [ ] T030 Update main CLI parser in src/cli/mod.rs to include new commands

## Phase 3.7: TUI Interface
- [ ] T031 [P] RetrospectionView widget in src/tui/retrospection.rs
- [ ] T032 [P] PromptTemplateView widget in src/tui/prompt_templates.rs
- [ ] T033 [P] AnalysisDetailView widget in src/tui/analysis_detail.rs
- [ ] T034 Integrate retrospection views with main TUI app in src/tui/app.rs

## Phase 3.8: Configuration System
- [ ] T035 [P] TOML configuration parser in src/config/prompt_config.rs
- [ ] T036 [P] XDG directory management in src/config/directories.rs
- [ ] T037 [P] Default prompt template loader in src/config/defaults.rs
- [ ] T038 Configuration validation and error handling in src/config/mod.rs

## Phase 3.9: Error Handling & Integration
- [ ] T039 Custom error types for retrospection in src/error.rs
- [ ] T040 Rate limiting and retry logic for Gemini API
- [ ] T041 Token usage tracking and cost estimation
- [ ] T042 Background job processing for analysis requests
- [ ] T043 Progress reporting and status updates

## Phase 3.10: Polish & Validation
- [ ] T044 [P] Unit tests for GeminiClient in tests/unit/test_gemini_client.rs
- [ ] T045 [P] Unit tests for PromptService in tests/unit/test_prompt_service.rs
- [ ] T046 [P] Unit tests for RetrospectionService in tests/unit/test_retrospection_service.rs
- [ ] T047 [P] Unit tests for prompt template validation in tests/unit/test_template_validation.rs
- [ ] T048 Performance test for large session analysis (>10k messages)
- [ ] T049 [P] Update README.md with retrospection feature documentation
- [ ] T050 [P] Update CLAUDE.md with implementation details
- [ ] T051 Execute quickstart.md validation scenarios
- [ ] T052 Code cleanup and remove debug statements

## Dependencies
- Setup (T001-T003) before everything
- Tests (T004-T009) before implementation (T010+)
- Models (T010-T014) before repositories (T015-T017)
- Repositories (T015-T017) before services (T020-T023)
- Services (T020-T023) before CLI/TUI (T024-T034)
- Configuration (T035-T038) parallel with CLI/TUI
- Integration (T039-T043) after core services
- Polish (T044-T052) after all implementation

## Parallel Example
```bash
# Launch contract tests together (T004-T009):
Task: "Contract test Gemini API generateContent in tests/contract/test_gemini_api.rs"
Task: "Contract test retrospection service analyze in tests/contract/test_retrospection_service.rs"
Task: "Integration test basic session analysis in tests/integration/test_session_analysis.rs"
Task: "Integration test custom prompt analysis in tests/integration/test_custom_prompt.rs"

# Launch model creation together (T010-T014):
Task: "RetrospectionAnalysis model in src/models/retrospection_analysis.rs"
Task: "PromptTemplate model in src/models/prompt_template.rs"
Task: "AnalysisRequest model in src/models/analysis_request.rs"
Task: "AnalysisMetadata model in src/models/analysis_metadata.rs"

# Launch CLI commands together (T024-T029):
Task: "CLI analyze retrospect command in src/cli/analytics.rs"
Task: "CLI prompts list command in src/cli/prompts.rs"
Task: "CLI prompts create command in src/cli/prompts.rs"
```

## Notes
- [P] tasks = different files, no dependencies
- Verify tests fail before implementing
- Run `cargo check && cargo test && cargo clippy` after each task
- Test with `GEMINI_API_KEY` environment variable set
- Avoid modifying existing functionality - only extend

## Task Generation Rules
*Applied during main() execution*

1. **From Contracts**:
   - gemini_api.json → contract test task T004 [P]
   - retrospection_service.json → contract test task T005 [P]

2. **From Data Model**:
   - RetrospectionAnalysis → model creation task T010 [P]
   - PromptTemplate → model creation task T011 [P]
   - AnalysisRequest → model creation task T012 [P]
   - AnalysisMetadata → model creation task T013 [P]

3. **From User Stories (quickstart.md)**:
   - Basic session analysis → integration test T006 [P]
   - Custom prompt analysis → integration test T007 [P]
   - TUI interface usage → integration test T008 [P]
   - Prompt template management → integration test T009 [P]

4. **Ordering**:
   - Setup → Tests → Models → Repositories → Services → CLI/TUI → Polish
   - Database migrations before repository creation
   - Services before CLI/TUI integration

## Validation Checklist
*GATE: Checked by main() before returning*

- [x] All contracts have corresponding tests (T004-T005)
- [x] All entities have model tasks (T010-T013)
- [x] All tests come before implementation (T004-T009 before T010+)
- [x] Parallel tasks truly independent (different files)
- [x] Each task specifies exact file path
- [x] No task modifies same file as another [P] task
- [x] Constitutional TDD requirements satisfied
- [x] Integration with existing analytics service planned
- [x] Environment variable requirements documented