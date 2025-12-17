//! Error handling for MCP server
//!
//! Provides conversion utilities from retrochat errors to MCP protocol errors.

use rmcp::ErrorData as McpError;

/// Convert an anyhow error to an MCP internal error
pub fn to_mcp_error(err: anyhow::Error) -> McpError {
    McpError::internal_error(err.to_string(), None)
}

/// Create an MCP invalid params error
pub fn validation_error(msg: &str) -> McpError {
    McpError::invalid_params(msg.to_string(), None)
}

/// Create an MCP not found error
pub fn not_found_error(msg: &str) -> McpError {
    McpError::internal_error(format!("Not found: {}", msg), None)
}

#[cfg(test)]
mod tests {
    use super::*;
    use rmcp::model::ErrorCode;

    #[test]
    fn test_anyhow_error_to_mcp_error() {
        let err = anyhow::anyhow!("Database connection failed");
        let mcp_err = to_mcp_error(err);

        assert_eq!(mcp_err.code, ErrorCode::INTERNAL_ERROR);
        assert!(mcp_err.message.contains("Database connection failed"));
    }

    #[test]
    fn test_validation_error() {
        let err = validation_error("Invalid UUID format");

        assert_eq!(err.code, ErrorCode::INVALID_PARAMS);
        assert_eq!(err.message, "Invalid UUID format");
    }

    #[test]
    fn test_not_found_error() {
        let err = not_found_error("Session with ID abc123");

        assert_eq!(err.code, ErrorCode::INTERNAL_ERROR);
        assert!(err.message.contains("Not found"));
        assert!(err.message.contains("Session with ID abc123"));
    }

    #[test]
    fn test_nested_error_conversion() {
        // Test with a nested error chain
        let inner_err = std::io::Error::new(std::io::ErrorKind::NotFound, "file not found");
        let outer_err = anyhow::anyhow!(inner_err).context("Failed to read configuration");
        let mcp_err = to_mcp_error(outer_err);

        assert_eq!(mcp_err.code, ErrorCode::INTERNAL_ERROR);
        assert!(mcp_err.message.contains("Failed to read configuration"));
    }

    #[test]
    fn test_error_message_formatting() {
        let err = validation_error("Expected format: YYYY-MM-DD, got: invalid-date");

        assert!(err.message.contains("YYYY-MM-DD"));
        assert!(err.message.contains("invalid-date"));
    }
}
