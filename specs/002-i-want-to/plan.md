
# Implementation Plan: LLM-Powered Chat Session Retrospection

**Branch**: `002-i-want-to` | **Date**: 2025-09-22 | **Spec**: [spec.md](./spec.md)
**Input**: Feature specification from `/specs/002-i-want-to/spec.md`

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
Implement LLM-powered retrospective analysis of chat sessions using Google AI Gemini 2.5 Flash Lite. Users can trigger analysis on specific sessions, configure custom prompts, and view results through CLI and TUI interfaces. Analysis results are stored locally with full metadata for future reference.

## Technical Context
**Language/Version**: Rust 1.75+ (edition 2021)
**Primary Dependencies**: Tokio (async), Reqwest (HTTP client for Gemini API), Serde (JSON), SQLite (Rusqlite), Ratatui (TUI), Clap (CLI)
**Storage**: SQLite database with existing schema (chat_sessions, messages tables)
**Testing**: cargo test (unit, integration, contract tests)
**Target Platform**: Cross-platform CLI/TUI application (Linux, macOS, Windows)
**Project Type**: single (existing Rust CLI/TUI application)
**Performance Goals**: Handle chat sessions up to 100k messages, API responses <10s
**Constraints**: GEMINI_API_KEY environment variable required, local storage only, offline viewing
**Scale/Scope**: Single-user application, support for multiple LLM provider histories, configurable prompts

## Constitution Check
*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

**Privacy and Security Review**:
- ✅ External LLM API calls disclosed (Gemini 2.5 Flash Lite)
- ⚠️ **VIOLATION**: Chat data transmitted to external service (Google AI)
- ✅ API key from environment variable (GEMINI_API_KEY)
- ✅ Analysis results stored locally
- ✅ User controls which sessions to analyze

**Data Processing Review**:
- ✅ Read-only access to existing chat sessions
- ✅ Fault-tolerant processing (existing parsers handle malformed data)
- ✅ Preserves original timestamps and metadata
- ✅ No modification of source files

**Test-Driven Development Review**:
- ✅ Unit tests required for all new components
- ✅ Integration tests for API integration and storage
- ✅ Contract tests for Gemini API interactions
- ✅ Build validation after implementation

**Analysis Quality Review**:
- ✅ Configurable prompts for reproducible analysis
- ✅ Metadata stored with analysis results (timestamp, model used, token usage, cost)
- ✅ Error handling for API failures with retry mechanisms
- ✅ Deterministic storage and retrieval with proper indexing
- ✅ Template validation ensures consistent prompt structure
- ✅ Token usage tracking for cost monitoring

**Post-Design Constitution Re-evaluation**:
- ✅ **Privacy Compliance**: Privacy violation documented and justified in Complexity Tracking
- ✅ **Data Processing**: New entities extend existing schema without modification
- ✅ **TDD Requirements**: Contract tests defined in /contracts/ directory
- ✅ **Analysis Quality**: Token tracking, cost estimation, and metadata preservation implemented
- ✅ **Build Validation**: All new dependencies compatible with existing build system

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

**Structure Decision**: Option 1 (Single project) - Extending existing Rust CLI/TUI application

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
- Generate tasks from Phase 1 design docs (contracts, data model, quickstart)
- Database tasks: Migration scripts for new tables (retrospection_analyses, prompt_templates, etc.)
- Model tasks: Rust structs for RetrospectionAnalysis, PromptTemplate, AnalysisRequest entities
- Service tasks: GeminiClient, RetrospectionService, PromptService implementations
- Repository tasks: Database access layers for new entities
- CLI tasks: New analyze retrospect commands and prompt management commands
- TUI tasks: Retrospection analysis browser and management interface
- Contract test tasks: API integration tests for Gemini service
- Integration test tasks: End-to-end scenarios from quickstart.md
- Configuration tasks: TOML-based prompt template system

**Ordering Strategy**:
- **Phase 1** (Models & Database): Migrations → Entity structs → Repository layers [P]
- **Phase 2** (Core Services): GeminiClient → PromptService → RetrospectionService
- **Phase 3** (API Integration): Contract tests → Service integration → Error handling
- **Phase 4** (CLI Interface): Command parsing → Business logic → Output formatting [P]
- **Phase 5** (TUI Interface): UI components → Navigation → Data display [P]
- **Phase 6** (Integration**: End-to-end tests → Quickstart validation → Performance testing
- Mark [P] for parallel execution within phases (independent components)

**Estimated Task Breakdown**:
- Database & Models: 8-10 tasks
- Core Services: 6-8 tasks
- API Integration: 4-6 tasks
- CLI Interface: 6-8 tasks
- TUI Interface: 4-6 tasks
- Testing & Validation: 6-8 tasks
- **Total**: 34-46 numbered, ordered tasks in tasks.md

**Dependencies Identified**:
- reqwest dependency for HTTP client
- toml dependency for configuration parsing
- directories dependency for XDG compliance
- New database schema extends existing migration system
- Existing CLI/TUI framework integration points

**Risk Mitigation**:
- Contract tests ensure API compatibility before implementation
- Database migrations are reversible
- Configuration system maintains backwards compatibility
- Existing functionality remains unmodified

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
| Chat data transmitted to external service (Google AI) | Core feature requirement for LLM-powered retrospection analysis | Local analysis insufficient - requires advanced language understanding capabilities only available from large language models |


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
- [x] Complexity deviations documented

---
*Based on Constitution v2.1.1 - See `/memory/constitution.md`*
