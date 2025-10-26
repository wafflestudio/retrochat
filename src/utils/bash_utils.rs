use crate::models::message::ToolResult;

/// Extract stdout and stderr from bash tool result
/// This function handles different formats from various AI providers
pub fn extract_bash_output(result: &ToolResult) -> (Option<String>, Option<String>) {
    let stdout = result
        .details
        .as_ref()
        .and_then(|details| {
            // Check if details is an array (Claude format)
            if let Some(array) = details.as_array() {
                // Look for toolUseResult in the array
                for item in array {
                    if let Some(obj) = item.as_object() {
                        if obj.get("type").and_then(|t| t.as_str()) == Some("toolUseResult") {
                            if let Some(metadata) = obj.get("toolUseResult") {
                                return metadata
                                    .get("stdout")
                                    .and_then(|s| s.as_str())
                                    .map(String::from);
                            }
                        }
                    }
                }
            }
            // Fallback to direct stdout field
            details
                .get("stdout")
                .and_then(|s| s.as_str())
                .map(String::from)
        })
        .or_else(|| {
            // If details doesn't have stdout, try raw
            result
                .raw
                .get("stdout")
                .and_then(|s| s.as_str())
                .map(String::from)
        });

    let stderr = result
        .details
        .as_ref()
        .and_then(|details| {
            // Check if details is an array (Claude format)
            if let Some(array) = details.as_array() {
                for item in array {
                    if let Some(obj) = item.as_object() {
                        if obj.get("type").and_then(|t| t.as_str()) == Some("toolUseResult") {
                            if let Some(metadata) = obj.get("toolUseResult") {
                                return metadata
                                    .get("stderr")
                                    .and_then(|s| s.as_str())
                                    .map(String::from);
                            }
                        }
                    }
                }
            }
            // Fallback to direct stderr field
            details
                .get("stderr")
                .and_then(|s| s.as_str())
                .map(String::from)
        })
        .or_else(|| {
            // If details doesn't have stderr, try raw
            result
                .raw
                .get("stderr")
                .and_then(|s| s.as_str())
                .map(String::from)
        });

    (stdout, stderr)
}

/// Extract exit code from bash tool result
pub fn extract_bash_exit_code(result: &ToolResult) -> Option<i32> {
    result
        .details
        .as_ref()
        .and_then(|details| {
            // Check if details is an array (Claude format)
            if let Some(array) = details.as_array() {
                for item in array {
                    if let Some(obj) = item.as_object() {
                        if obj.get("type").and_then(|t| t.as_str()) == Some("toolUseResult") {
                            if let Some(metadata) = obj.get("toolUseResult") {
                                return metadata
                                    .get("exit_code")
                                    .and_then(|c| c.as_i64())
                                    .map(|c| c as i32);
                            }
                        }
                    }
                }
            }
            // Fallback to direct exit_code field
            details
                .get("exit_code")
                .and_then(|c| c.as_i64())
                .map(|c| c as i32)
        })
        .or_else(|| {
            // If details doesn't have exit_code, try raw
            result
                .raw
                .get("exit_code")
                .and_then(|c| c.as_i64())
                .map(|c| c as i32)
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::message::ToolResult;
    use serde_json::json;

    fn create_tool_result_with_details(details: serde_json::Value) -> ToolResult {
        ToolResult {
            tool_use_id: "test-id".to_string(),
            content: "test content".to_string(),
            is_error: false,
            details: Some(details),
            raw: json!({}),
        }
    }

    #[test]
    fn test_extract_bash_output_direct_fields() {
        let details = json!({
            "stdout": "Hello World\n",
            "stderr": "Warning: deprecated\n",
            "exit_code": 0
        });
        let result = create_tool_result_with_details(details);

        let (stdout, stderr) = extract_bash_output(&result);
        assert_eq!(stdout, Some("Hello World\n".to_string()));
        assert_eq!(stderr, Some("Warning: deprecated\n".to_string()));
    }

    #[test]
    fn test_extract_bash_output_claude_format() {
        let details = json!([
            {
                "type": "toolUseResult",
                "toolUseResult": {
                    "stdout": "Command executed successfully\n",
                    "stderr": "",
                    "exit_code": 0
                }
            }
        ]);
        let result = create_tool_result_with_details(details);

        let (stdout, stderr) = extract_bash_output(&result);
        assert_eq!(stdout, Some("Command executed successfully\n".to_string()));
        assert_eq!(stderr, Some("".to_string()));
    }

    #[test]
    fn test_extract_bash_output_no_output() {
        let details = json!({});
        let result = create_tool_result_with_details(details);

        let (stdout, stderr) = extract_bash_output(&result);
        assert_eq!(stdout, None);
        assert_eq!(stderr, None);
    }

    #[test]
    fn test_extract_bash_exit_code_direct_field() {
        let details = json!({
            "exit_code": 1
        });
        let result = create_tool_result_with_details(details);

        let exit_code = extract_bash_exit_code(&result);
        assert_eq!(exit_code, Some(1));
    }

    #[test]
    fn test_extract_bash_exit_code_claude_format() {
        let details = json!([
            {
                "type": "toolUseResult",
                "toolUseResult": {
                    "exit_code": 2
                }
            }
        ]);
        let result = create_tool_result_with_details(details);

        let exit_code = extract_bash_exit_code(&result);
        assert_eq!(exit_code, Some(2));
    }

    #[test]
    fn test_extract_bash_exit_code_no_exit_code() {
        let details = json!({});
        let result = create_tool_result_with_details(details);

        let exit_code = extract_bash_exit_code(&result);
        assert_eq!(exit_code, None);
    }

    #[test]
    fn test_extract_bash_output_fallback_to_raw() {
        let details = json!({});
        let raw = json!({
            "stdout": "Fallback output\n",
            "stderr": "Fallback error\n"
        });
        let result = ToolResult {
            tool_use_id: "test-id".to_string(),
            content: "test content".to_string(),
            is_error: false,
            details: Some(details),
            raw,
        };

        let (stdout, stderr) = extract_bash_output(&result);
        assert_eq!(stdout, Some("Fallback output\n".to_string()));
        assert_eq!(stderr, Some("Fallback error\n".to_string()));
    }
}
