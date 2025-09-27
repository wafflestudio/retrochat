
# Implementation Plan: Add Retrospection Process for Chat Sessions

**Branch**: `002-add-retrospection-process` | **Date**: 2025-09-26 | **Spec**: [spec.md](./spec.md)
**Input**: Feature specification from `/specs/002-add-retrospection-process/spec.md`

## Execution Flow (/plan command scope)
```
1. Load feature spec from Input path
   → If not found: ERROR "No feature spec at {path}"
2. Fill Technical Context (scan for NEEDS CLARIFICATION)
   → Detect Project Type from context (web=frontend+backend, mobile=app+api)
   → Set Structure Decision based on project type
3. Fill the Constitution Check section based on the content of the constitution document.
4. Evaluate Constitution Check section below
   → If violations exist: Document in Complexity Tracking
   → If no justification possible: ERROR "Simplify approach first"
   → Update Progress Tracking: Initial Constitution Check
5. Execute Phase 0 → research.md
   → If NEEDS CLARIFICATION remain: ERROR "Resolve unknowns"
6. Execute Phase 1 → contracts, data-model.md, quickstart.md, agent-specific template file (e.g., `CLAUDE.md` for Claude Code, `.github/copilot-instructions.md` for GitHub Copilot, `GEMINI.md` for Gemini CLI, `QWEN.md` for Qwen Code or `AGENTS.md` for opencode).
7. Re-evaluate Constitution Check section
   → If new violations: Refactor design, return to Phase 1
   → Update Progress Tracking: Post-Design Constitution Check
8. Plan Phase 2 → Describe task generation approach (DO NOT create tasks.md)
9. STOP - Ready for /tasks command
```

**IMPORTANT**: The /plan command STOPS at step 7. Phases 2-4 are executed by other commands:
- Phase 2: /tasks command creates tasks.md
- Phase 3-4: Implementation execution (manual or via tools)

## Summary
Implement retrospection analysis system that allows users to analyze their coding sessions with AI agents using Google AI. Users can trigger analysis via CLI (`retrospect execute`) or TUI, store results persistently, and view insights through CLI (`retrospect show`) or TUI interfaces including session detail side panels. The system analyzes user interaction patterns to identify communication effectiveness, collaboration strengths/weaknesses, and areas for improvement in AI-assisted coding workflows.

## Technical Context
**Language/Version**: Rust 1.75+ (from existing project)
**Primary Dependencies**: Ratatui (TUI), SQLite/SQLx (storage), Serde (serialization), Clap (CLI), Tokio (async), reqwest (HTTP client for Google AI)
**Storage**: SQLite with SQLx migration from rusqlite (existing)
**Testing**: cargo test (Rust standard)
**Target Platform**: Cross-platform CLI/TUI application
**Project Type**: single (TUI application with CLI interface)
**Performance Goals**: Handle analysis of large chat sessions efficiently, responsive UI during long-running operations
**Constraints**: LLM requests can take time or fail - requires status management for retrospection requests
**Scale/Scope**: Support analysis of potentially thousands of chat sessions with robust error handling and progress tracking

## Constitution Check
*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

**I. Data Processing First**: ✅ PASS
- Feature reads chat session data without modification
- Google AI integration processes data externally but results stored locally
- Fault-tolerant design required for handling API failures

**II. Test-Driven Development**: ✅ PASS
- Unit tests required for retrospection logic
- Integration tests for Google AI service integration
- Build/test commands mandatory after implementation

**III. Analysis Quality and Accuracy**: ✅ PASS
- LLM analysis results stored as-is with metadata
- Error handling for failed/partial analysis
- Status tracking for long-running operations

**IV. Privacy and Security**: ⚠ ATTENTION REQUIRED
- Google AI service integration sends data externally
- Need user consent for external processing
- Clear disclosure of data sharing with Google AI

**V. Build Validation**: ✅ PASS
- Standard Rust toolchain validation
- Performance considerations for large datasets

## Project Structure

### Documentation (this feature)
```
specs/[###-feature]/
├── plan.md              # This file (/plan command output)
├── research.md          # Phase 0 output (/plan command)
├── data-model.md        # Phase 1 output (/plan command)
├── quickstart.md        # Phase 1 output (/plan command)
├── contracts/           # Phase 1 output (/plan command)
└── tasks.md             # Phase 2 output (/tasks command - NOT created by /plan)
```

### Source Code (repository root)
```
# Option 1: Single project (DEFAULT)
src/
├── models/
├── services/
├── cli/
└── lib/

tests/
├── contract/
├── integration/
└── unit/

# Option 2: Web application (when "frontend" + "backend" detected)
backend/
├── src/
│   ├── models/
│   ├── services/
│   └── api/
└── tests/

frontend/
├── src/
│   ├── components/
│   ├── pages/
│   └── services/
└── tests/

# Option 3: Mobile + API (when "iOS/Android" detected)
api/
└── [same as backend above]

ios/ or android/
└── [platform-specific structure]
```

**Structure Decision**: Option 1 (Single project) - TUI application with CLI interface fits existing Rust project structure

## Phase 0: Outline & Research
1. **Extract unknowns from Technical Context** above:
   - For each NEEDS CLARIFICATION → research task
   - For each dependency → best practices task
   - For each integration → patterns task

2. **Generate and dispatch research agents**:
   ```
   For each unknown in Technical Context:
     Task: "Research {unknown} for {feature context}"
   For each technology choice:
     Task: "Find best practices for {tech} in {domain}"
   ```

3. **Consolidate findings** in `research.md` using format:
   - Decision: [what was chosen]
   - Rationale: [why chosen]
   - Alternatives considered: [what else evaluated]

**Output**: research.md with all NEEDS CLARIFICATION resolved

## Phase 1: Design & Contracts
*Prerequisites: research.md complete*

1. **Extract entities from feature spec** → `data-model.md`:
   - Entity name, fields, relationships
   - Validation rules from requirements
   - State transitions if applicable

2. **Generate API contracts** from functional requirements:
   - For each user action → endpoint
   - Use standard REST/GraphQL patterns
   - Output OpenAPI/GraphQL schema to `/contracts/`

3. **Generate contract tests** from contracts:
   - One test file per endpoint
   - Assert request/response schemas
   - Tests must fail (no implementation yet)

4. **Extract test scenarios** from user stories:
   - Each story → integration test scenario
   - Quickstart test = story validation steps

5. **Update agent file incrementally** (O(1) operation):
   - Run `.specify/scripts/bash/update-agent-context.sh claude`
     **IMPORTANT**: Execute it exactly as specified above. Do not add or remove any arguments.
   - If exists: Add only NEW tech from current plan
   - Preserve manual additions between markers
   - Update recent changes (keep last 3)
   - Keep under 150 lines for token efficiency
   - Output to repository root

**Output**: data-model.md, /contracts/*, failing tests, quickstart.md, agent-specific file

## Phase 2: Task Planning Approach
*This section describes what the /tasks command will do - DO NOT execute during /plan*

**Task Generation Strategy**:
- Load `.specify/templates/tasks-template.md` as base
- Generate tasks from Phase 1 design docs (data model, contracts, quickstart)
- Database migration tasks for retrospection tables [P]
- Google AI client implementation with error handling [P]
- Background operation manager with progress tracking [P]
- CLI command implementation (execute, show, status, cancel) [P]
- TUI integration with existing widgets and panels
- Status management and persistence layer
- Integration tests for complete workflows
- Contract tests for API interfaces

**Ordering Strategy**:
- TDD order: Tests before implementation
- Foundation first: Database migrations → Models → Services → CLI → TUI
- Independent components marked [P] for parallel execution
- Integration tasks after all components are complete
- User acceptance testing based on quickstart scenarios

**Key Task Categories**:
1. **Database Layer** (3-4 tasks): Migrations, repositories, schema validation
2. **Google AI Integration** (4-5 tasks): Client, error handling, retry logic, request/response models
3. **Background Operations** (4-5 tasks): Task manager, progress tracking, cancellation, persistence
4. **CLI Interface** (6-7 tasks): Commands, argument parsing, output formatting, error handling
5. **TUI Integration** (5-6 tasks): Progress widgets, status panels, keyboard shortcuts, session detail updates
6. **Testing** (4-5 tasks): Unit tests, integration tests, contract tests, user acceptance tests
7. **Documentation** (2-3 tasks): API docs, user guides, troubleshooting

**Estimated Output**: 28-35 numbered, ordered tasks in tasks.md

**Priority Focus**:
- Core functionality first (Google AI client, background operations)
- User-facing interfaces second (CLI commands, TUI integration)
- Advanced features last (optimization, extended error handling)
- Constitutional compliance: TDD approach with comprehensive testing
- Privacy considerations: User consent and data handling validation

**IMPORTANT**: This phase is executed by the /tasks command, NOT by /plan

## Phase 3+: Future Implementation
*These phases are beyond the scope of the /plan command*

**Phase 3**: Task execution (/tasks command creates tasks.md)  
**Phase 4**: Implementation (execute tasks.md following constitutional principles)  
**Phase 5**: Validation (run tests, execute quickstart.md, performance validation)

## Complexity Tracking
*Fill ONLY if Constitution Check has violations that must be justified*

| Violation | Why Needed | Simpler Alternative Rejected Because |
|-----------|------------|-------------------------------------|
| [e.g., 4th project] | [current need] | [why 3 projects insufficient] |
| [e.g., Repository pattern] | [specific problem] | [why direct DB access insufficient] |


## Progress Tracking
*This checklist is updated during execution flow*

**Phase Status**:
- [x] Phase 0: Research complete (/plan command)
- [x] Phase 1: Design complete (/plan command)
- [x] Phase 2: Task planning complete (/plan command - describe approach only)
- [ ] Phase 3: Tasks generated (/tasks command)
- [ ] Phase 4: Implementation complete
- [ ] Phase 5: Validation passed

**Gate Status**:
- [x] Initial Constitution Check: PASS
- [x] Post-Design Constitution Check: PASS
- [x] All NEEDS CLARIFICATION resolved
- [ ] Complexity deviations documented

---
*Based on Constitution v2.1.1 - See `/memory/constitution.md`*
