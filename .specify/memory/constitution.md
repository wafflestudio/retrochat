<!--
Sync Impact Report:
- Version change: N/A → 1.0.0 (initial constitution)
- Added sections: All sections (initial creation)
- Templates requiring updates: ⚠ pending validation
- Follow-up TODOs: None
-->

# RetroChat Constitution

## Core Principles

### I. Data Processing First
Local chat history files are the primary data source; All file processing must be fault-tolerant and handle corrupted/incomplete data gracefully; Data extraction must preserve original timestamps and metadata; No data modification of source files - read-only access only.

**Rationale**: Chat history files may be large, varied in format, and contain incomplete sessions. Robust processing ensures reliable analysis regardless of data quality.

### II. Test-Driven Development (NON-NEGOTIABLE)
Every component must have unit tests before implementation; Integration tests required for file parsing and analysis logic; After completing ANY task, build and test commands MUST be run for sanity check; Red-Green-Refactor cycle strictly enforced.

**Rationale**: Chat analysis involves complex data transformations. TDD ensures accuracy and prevents regressions in analysis results.

### III. Analysis Quality and Accuracy
Analysis algorithms must be deterministic and reproducible; Statistical calculations require validation against known datasets; Aggregation logic must handle edge cases (empty chats, single messages, time gaps); Results must include confidence intervals or uncertainty indicators where applicable.

**Rationale**: Users rely on analysis for insights into their LLM usage patterns. Inaccurate analysis undermines the application's value.

### IV. Privacy and Security
All processing happens locally - no data transmitted to external services; Sensitive information detection and optional redaction in analysis outputs; User consent required before accessing any chat history files; Clear data retention and deletion policies.

**Rationale**: Chat histories may contain personal, professional, or sensitive information that must remain under user control.

### V. Build Validation
Every commit must pass linting, type checking, and all tests; Build artifacts must be validated before any release; Continuous integration gates prevent broken code from merging; Performance benchmarks for large chat file processing.

**Rationale**: Chat file processing can be computationally intensive. Ensuring build quality prevents performance regressions and runtime failures.

## Data Handling Constraints

All file format parsers must handle malformed JSON, CSV, or proprietary formats gracefully; Memory usage must be bounded for large chat history files (streaming/chunked processing); Export formats must be standard and interoperable (JSON, CSV, HTML reports); Timezone handling must be consistent across all timestamp processing.

## Development Workflow

Code reviews must verify privacy compliance and data handling safety; Performance testing required for file processing components exceeding 10MB test files; Documentation must include data format specifications and analysis methodology; Error handling must provide actionable feedback to users about file format issues.

## Governance

Constitution supersedes all other development practices; All code changes must demonstrate compliance with privacy and accuracy principles; Build and test validation is mandatory after every development task completion; Amendments require documentation of impact on data processing and analysis accuracy.

**Version**: 1.0.0 | **Ratified**: 2025-09-21 | **Last Amended**: 2025-09-21