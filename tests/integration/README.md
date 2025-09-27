# Integration Tests

Integration tests validate **complete workflows and multi-component interactions** in the retrochat application. These tests ensure that different services work together correctly to deliver end-to-end functionality.

## Test-Driven Development (TDD) Approach

These tests follow the **Red-Green-Refactor** cycle with emphasis on:
1. **Red**: Write failing tests that define complete user workflows
2. **Green**: Implement integration code to make workflows pass
3. **Refactor**: Optimize multi-component interactions while preserving behavior

## Automated Testing

```bash
# Run all integration tests
cargo test --test test_first_time_setup

# Run all integration tests (alternative)
cargo test test_first_time_setup test_export_reporting test_error_handling

# Run with verbose output for debugging
cargo test --test test_single_session_analysis --verbose
```

## Integration Test Specifications

| Test File | Purpose | TDD Phase | Test Focus | Coverage Area | Dependencies | Expected Behavior | Automation |
|-----------|---------|-----------|------------|---------------|--------------|-------------------|------------|
| `test_error_handling.rs` | Tests error recovery across components | Red-Green | Error Workflows | Cross-service error propagation | Google AI, Database, Services | Graceful degradation on failures | ✅ `cargo test` |
| `test_export_reporting.rs` | Tests complete export workflow | Red-Green-Refactor | End-to-End Export | Analytics → Export → File output | Database, AnalyticsService, TempDir | Complete export pipeline works | ✅ `cargo test` |
| `test_first_time_setup.rs` | Tests complete first-run workflow | Red-Green-Refactor | User Onboarding | Scan → Import → Query workflow | Database, ImportService, QueryService | New user can import and browse | ✅ `cargo test` |
| `test_first_time_setup_simple.rs` | Simplified first-run test | Red-Green | Basic Setup | Core setup workflow | Minimal dependencies | Basic functionality works | ✅ `cargo test` |
| `test_session_detail.rs` | Tests session detail workflow | Red-Green | Detail View | Session → Messages → Display | Database, QueryService | Session details load completely | ✅ `cargo test` |
| `test_session_detail_simple.rs` | Simplified session detail test | Red-Green | Basic Detail View | Core detail functionality | Minimal dependencies | Basic session info loads | ✅ `cargo test` |
| `test_single_session_analysis.rs` | Tests retrospection analysis workflow | Red-Green | AI Analysis | Session → Google AI → Storage | Google AI, RetrospectionService | Analysis request completes end-to-end | ✅ `cargo test` |
| `test_tui_retrospection.rs` | Tests TUI retrospection integration | Red-Green | UI Integration | TUI → Services → Display | TUI components, Services | TUI retrospection interface works | ⚠️ Commented out |

## Integration Test Categories

### 🚀 **User Workflow Tests**
- **Purpose**: Validate complete user journeys from start to finish
- **Files**: `test_first_time_setup*.rs`, `test_session_detail*.rs`
- **Focus**: Multi-step user interactions, realistic data flows

### 🔄 **Service Integration Tests**
- **Purpose**: Test interactions between different service layers
- **Files**: `test_single_session_analysis.rs`, `test_export_reporting.rs`
- **Focus**: Service-to-service communication, data transformation pipelines

### ⚠️ **Error Handling Tests**
- **Purpose**: Validate system behavior under failure conditions
- **Files**: `test_error_handling.rs`
- **Focus**: Cross-component error propagation, graceful degradation

### 🖥️ **UI Integration Tests**
- **Purpose**: Test user interface with real backend services
- **Files**: `test_tui_retrospection.rs` (currently disabled)
- **Focus**: UI-service integration, user interaction flows

## TDD Integration Testing Strategy

### ✅ **What Integration Tests Should Do:**
- Test realistic user scenarios end-to-end
- Validate data flows between multiple components
- Test error handling across service boundaries
- Ensure configuration changes work in practice
- Test performance with realistic data volumes

### ❌ **What Integration Tests Should NOT Do:**
- Test individual component logic (use contract tests)
- Mock external dependencies unnecessarily
- Test user interface details (use unit tests)
- Focus on implementation-specific optimizations
- Test every possible error combination

## Test Complexity and Maintenance

### 🟢 **Simple Integration Tests**
- **Files**: `*_simple.rs` variants
- **Purpose**: Fast, focused integration validation
- **Maintenance**: Low - minimal dependencies

### 🟡 **Full Integration Tests**
- **Files**: Main test files
- **Purpose**: Complete workflow validation
- **Maintenance**: Medium - realistic but manageable complexity

### 🔴 **Complex Integration Tests**
- **Files**: Multi-component tests like TUI integration
- **Purpose**: Full system validation
- **Maintenance**: High - may be temporarily disabled during development

## Running Integration Tests in Development

```bash
# TDD Workflow - Run key integration tests after major changes
cargo check && cargo test --test test_first_time_setup && cargo clippy

# Test specific integration areas
cargo test --test test_export_reporting        # Analytics integration
cargo test --test test_single_session_analysis  # AI integration
cargo test --test test_error_handling          # Error scenarios

# Test all integration workflows
cargo test test_first_time_setup test_export test_error test_session

# Debug integration test with logs
RUST_LOG=debug cargo test --test test_single_session_analysis
```

## Test Data Management

Integration tests use:
- **In-memory databases** for speed and isolation
- **Temporary directories** for file operations
- **Mock/test data** that represents realistic scenarios
- **Cleanup mechanisms** to prevent test pollution

## CI/CD Integration

These tests run automatically in GitHub Actions:
- **On push/PR**: All integration tests execute
- **Timeout protection**: Tests have reasonable time limits
- **Isolation**: Each test runs in clean environment
- **Reporting**: Detailed logs help debug failures

## Known Test Status

| Status | Tests | Reason |
|--------|-------|--------|
| ✅ **Active** | Most integration tests | Core functionality implemented |
| ⚠️ **Disabled** | `test_tui_retrospection.rs` | TUI components under development |
| 🔄 **In Progress** | AI-related tests | Dependent on Google AI API availability |

Integration tests serve as **executable documentation** of the system's complete workflows and ensure that the application delivers value to end users.