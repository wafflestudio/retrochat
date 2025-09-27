# Contract Tests

Contract tests validate the **API interfaces and behavior contracts** between different components of the retrochat application. These tests ensure that each service layer maintains its expected interface and behavior regardless of implementation details.

## Test-Driven Development (TDD) Approach

These tests follow the **Red-Green-Refactor** cycle:
1. **Red**: Write failing tests that define expected behavior
2. **Green**: Implement minimal code to make tests pass
3. **Refactor**: Improve code while keeping tests passing

## Automated Testing

```bash
# Run all contract tests
cargo test

# Run specific contract test
cargo test --test test_import_scan

# Run with verbose output
cargo test --test test_import_scan --verbose
```

## Contract Test Specifications

| Test File | Purpose | TDD Phase | Test Focus | Coverage Area | Dependencies | Expected Behavior | Automation |
|-----------|---------|-----------|------------|---------------|--------------|-------------------|------------|
| `test_analytics_export.rs` | Validates analytics export functionality | Red-Green | API Contract | Export service interface | DatabaseManager, AnalyticsService | CSV/JSON export returns structured data | ✅ `cargo test` |
| `test_cli_retrospect_cancel.rs` | Tests retrospection cancellation commands | Red-Green | CLI Interface | Command argument parsing | CLI handlers | Cancel command accepts request IDs | ✅ `cargo test` |
| `test_cli_retrospect_execute.rs` | Tests retrospection execution commands | Red-Green | CLI Interface | Command execution flow | Google AI API key | Execute command triggers analysis | ✅ `cargo test` |
| `test_cli_retrospect_show.rs` | Tests retrospection result display | Red-Green | CLI Interface | Output formatting | Database, CLI handlers | Show command displays results | ✅ `cargo test` |
| `test_cli_retrospect_status.rs` | Tests retrospection status monitoring | Red-Green | CLI Interface | Status reporting | Database, CLI handlers | Status command reports progress | ✅ `cargo test` |
| `test_google_ai_api.rs` | Validates Google AI integration contract | Red-Green | External API | API request/response structure | Google AI client | API calls follow expected format | ✅ `cargo test` |
| `test_import_batch.rs` | Tests batch import functionality | Red-Green | Service Contract | Batch processing interface | TempDir, ImportService | Batch import processes multiple files | ✅ `cargo test` |
| `test_import_file.rs` | Tests single file import | Red-Green | Service Contract | File processing interface | ImportService, file parsers | Single file import extracts sessions | ✅ `cargo test` |
| `test_import_scan.rs` | Tests directory scanning | Red-Green | Service Contract | Directory traversal interface | TempDir, ImportService | Scan returns file discovery results | ✅ `cargo test` |
| `test_search.rs` | Tests message search functionality | Red-Green | Service Contract | Search query interface | Database, QueryService | Search returns relevant messages | ✅ `cargo test` |
| `test_session_detail.rs` | Tests session detail retrieval | Red-Green | Service Contract | Session data interface | Database, QueryService | Session detail returns complete data | ✅ `cargo test` |
| `test_sessions_query.rs` | Tests session listing and filtering | Red-Green | Service Contract | Query interface | Database, QueryService | Session queries support filters | ✅ `cargo test` |

## Contract Test Categories

### 🔌 **API Interface Tests**
- **Purpose**: Validate service layer APIs maintain expected signatures
- **Files**: `test_analytics_export.rs`, `test_import_*.rs`, `test_search.rs`
- **Focus**: Input validation, return types, error handling

### 🖥️ **CLI Interface Tests**
- **Purpose**: Ensure command-line interface contracts
- **Files**: `test_cli_retrospect_*.rs`
- **Focus**: Argument parsing, command execution, output formatting

### 🌐 **External API Tests**
- **Purpose**: Validate third-party service integration
- **Files**: `test_google_ai_api.rs`
- **Focus**: Request structure, response parsing, error handling

### 📊 **Data Contract Tests**
- **Purpose**: Ensure data layer interface consistency
- **Files**: `test_session*.rs`, `test_search.rs`
- **Focus**: Database queries, data transformation, filtering

## TDD Best Practices in Contract Tests

### ✅ **What Contract Tests Should Do:**
- Define clear input/output expectations
- Test error conditions and edge cases
- Validate interface compatibility
- Ensure consistent behavior across implementations
- Test with realistic data structures

### ❌ **What Contract Tests Should NOT Do:**
- Test internal implementation details
- Depend on external services being available
- Test user interface interactions
- Focus on performance optimization
- Test integration between multiple services

## Running Contract Tests in Development

```bash
# TDD Workflow - Run after each code change
cargo check && cargo test --test test_import_scan && cargo clippy

# Test specific functionality area
cargo test --test test_cli_retrospect_execute
cargo test --test test_google_ai_api

# Test all retrospection contracts
cargo test test_cli_retrospect

# Test all import contracts
cargo test test_import
```

## CI/CD Integration

These tests run automatically in GitHub Actions on:
- Every push to `main` or `develop` branches
- All pull requests
- Includes linting (`cargo clippy`) and formatting (`cargo fmt --check`)

Contract tests serve as **living documentation** of the API interfaces and ensure backward compatibility as the codebase evolves.