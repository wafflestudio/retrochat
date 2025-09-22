use std::path::Path;

/// Utility for inferring project names from Claude Code's encoded directory patterns
pub struct ProjectInference {
    file_path: String,
}

impl ProjectInference {
    /// Create a new ProjectInference instance for the given file path
    pub fn new(file_path: impl AsRef<Path>) -> Self {
        Self {
            file_path: file_path.as_ref().to_string_lossy().to_string(),
        }
    }

    /// Infer project name from file path by checking Claude Code project directory patterns
    pub fn infer_project_name(&self) -> Option<String> {
        let path = Path::new(&self.file_path);

        // Check if this is a Claude Code project directory pattern
        if let Some(parent_dir) = path.parent() {
            let parent_name = parent_dir.file_name()?.to_str()?;

            // Pattern: -Users-sanggggg-Project-retrochat
            if parent_name.starts_with('-') && parent_name.contains('-') {
                if let Some(original_path) = self.resolve_original_path(parent_name) {
                    // Extract project name (last component of validated path)
                    if let Some(project_name) = Path::new(&original_path).file_name() {
                        return project_name.to_str().map(|s| s.to_string());
                    }

                    // Fallback: use the entire validated path as project name
                    return Some(original_path);
                } else {
                    // If path resolution fails, try to extract project name from encoded name
                    return self.extract_project_name_from_encoded(parent_name);
                }
            }

            // Fallback: use parent directory name as project
            return Some(parent_name.to_string());
        }

        None
    }

    /// Extract project name from encoded directory name when filesystem validation fails
    fn extract_project_name_from_encoded(&self, encoded_name: &str) -> Option<String> {
        let without_prefix = encoded_name.trim_start_matches('-');
        let parts: Vec<&str> = without_prefix.split('-').collect();

        if parts.len() < 3 {
            // Not enough parts to be a valid Claude pattern
            return parts.last().map(|s| s.to_string());
        }

        // Heuristic: assume the structure is Users-username-[path-segments]-project
        // Try to identify common path patterns and extract the project name intelligently

        // Look for common base patterns like "Users", "home", etc.
        let start_idx = if parts.first() == Some(&"Users") || parts.first() == Some(&"home") {
            // Skip Users/username or home/username
            2
        } else {
            1
        };

        // Look for common intermediate patterns like "Project", "workspace", "code", etc.
        let mut found_pattern = false;
        let mut project_start_idx = start_idx;

        for (i, part) in parts.iter().enumerate().skip(start_idx) {
            if matches!(
                part.to_lowercase().as_str(),
                "project"
                    | "projects"
                    | "workspace"
                    | "workspaces"
                    | "code"
                    | "development"
                    | "dev"
            ) {
                project_start_idx = i + 1;
                found_pattern = true;
                break;
            }
        }

        // If we found a pattern and there are parts after it, take everything after it as the project name
        if found_pattern && project_start_idx < parts.len() {
            let project_parts = &parts[project_start_idx..];
            return Some(project_parts.join("-"));
        }

        // If no pattern found, try intelligent fallbacks
        if !found_pattern {
            // For patterns like "Users-username-projectname" (without intermediate dirs)
            if parts.len() == 3 && start_idx == 2 {
                return parts.last().map(|s| s.to_string());
            }

            // For longer paths, take everything after the username as potential project path
            if start_idx < parts.len() {
                let remaining_parts = &parts[start_idx..];

                // Strategy: For paths without clear markers, assume the last 1-2 parts form the project name
                if remaining_parts.len() >= 2 {
                    // Check if the last two parts look like a project name (e.g., "sub-folder", "test-project")
                    let last_two = &remaining_parts[remaining_parts.len() - 2..];

                    // If the second-to-last part looks like it could be part of a project name
                    // (not a system directory like "src", "lib", "bin", etc.)
                    let second_to_last = last_two[0];
                    if !matches!(
                        second_to_last.to_lowercase().as_str(),
                        "src"
                            | "lib"
                            | "bin"
                            | "target"
                            | "node_modules"
                            | "dist"
                            | "build"
                            | "out"
                            | "tmp"
                            | "temp"
                    ) {
                        return Some(last_two.join("-"));
                    }
                }

                // Fallback: return just the last part
                return remaining_parts.last().map(|s| s.to_string());
            }
        }

        // Final fallback: just the last part
        parts.last().map(|s| s.to_string())
    }

    /// Resolve the original filesystem path from Claude's encoded directory name
    /// by trying different hyphen/slash combinations and validating against filesystem
    fn resolve_original_path(&self, encoded_name: &str) -> Option<String> {
        let without_prefix = encoded_name.trim_start_matches('-');
        let parts: Vec<&str> = without_prefix.split('-').collect();

        // Get the current file's parent directory to use as a base for resolution
        let current_file_path = Path::new(&self.file_path);
        let base_dir = current_file_path.parent()?.parent()?; // Go up to find the base directory

        let mut valid_paths = Vec::new();

        // Generate ALL possible combinations and test them
        // For parts like ["Users", "testuser", "my", "project", "sub", "folder"]
        // We need to try different ways to split into path segments vs hyphenated names

        // Try all possible ways to split the parts into path components
        Self::generate_path_combinations(&parts, 0, Vec::new(), base_dir, &mut valid_paths);

        // Return the longest valid path
        valid_paths.into_iter().max_by_key(|path| path.len())
    }

    /// Recursively generate all possible path combinations
    fn generate_path_combinations(
        remaining_parts: &[&str],
        current_index: usize,
        current_path_parts: Vec<String>,
        base_dir: &Path,
        valid_paths: &mut Vec<String>,
    ) {
        if current_index >= remaining_parts.len() {
            // We've processed all parts, test this path combination
            if !current_path_parts.is_empty() {
                let path_candidate = current_path_parts.join("/");

                // Try both absolute and relative paths
                let abs_path = format!("/{path_candidate}");
                if Path::new(&abs_path).exists() {
                    valid_paths.push(abs_path);
                }

                let rel_path = base_dir.join(&path_candidate);
                if rel_path.exists() {
                    if let Some(rel_path_str) = rel_path.to_str() {
                        valid_paths.push(rel_path_str.to_string());
                    }
                }
            }
            return;
        }

        // Option 1: Add current part as a separate path component
        let mut new_path_parts = current_path_parts.clone();
        new_path_parts.push(remaining_parts[current_index].to_string());
        Self::generate_path_combinations(
            remaining_parts,
            current_index + 1,
            new_path_parts,
            base_dir,
            valid_paths,
        );

        // Option 2: If we have a previous component, try joining with hyphen
        if !current_path_parts.is_empty() {
            let mut hyphen_path_parts = current_path_parts;
            let last_idx = hyphen_path_parts.len() - 1;
            hyphen_path_parts[last_idx] = format!(
                "{}-{}",
                hyphen_path_parts[last_idx], remaining_parts[current_index]
            );
            Self::generate_path_combinations(
                remaining_parts,
                current_index + 1,
                hyphen_path_parts,
                base_dir,
                valid_paths,
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_infer_project_name_from_claude_pattern() {
        let temp_dir = TempDir::new().unwrap();
        let base_path = temp_dir.path();

        // Create the actual project directory structure
        let project_path = base_path
            .join("Users")
            .join("testuser")
            .join("Project")
            .join("retrochat");
        fs::create_dir_all(&project_path).unwrap();

        // Create Claude's encoded directory
        let claude_dir = base_path.join("-Users-testuser-Project-retrochat");
        fs::create_dir_all(&claude_dir).unwrap();

        // Create a test file in the Claude directory
        let test_file = claude_dir.join("test.jsonl");
        fs::write(&test_file, "{}").unwrap();

        let inference = ProjectInference::new(&test_file);
        let project_name = inference.infer_project_name();

        assert_eq!(project_name, Some("retrochat".to_string()));
    }

    #[test]
    fn test_infer_project_name_with_hyphens_in_path() {
        let temp_dir = TempDir::new().unwrap();
        let base_path = temp_dir.path();

        // Create the actual project directory with hyphens
        let project_path = base_path
            .join("Users")
            .join("testuser")
            .join("my-project")
            .join("sub-folder");
        fs::create_dir_all(&project_path).unwrap();

        // Create Claude's encoded directory
        let claude_dir = base_path.join("-Users-testuser-my-project-sub-folder");
        fs::create_dir_all(&claude_dir).unwrap();

        // Create a test file in the Claude directory
        let test_file = claude_dir.join("test.jsonl");
        fs::write(&test_file, "{}").unwrap();

        let inference = ProjectInference::new(&test_file);
        let project_name = inference.infer_project_name();

        assert_eq!(project_name, Some("sub-folder".to_string()));
    }

    #[test]
    fn test_infer_project_name_complex_path() {
        let temp_dir = TempDir::new().unwrap();
        let base_path = temp_dir.path();

        // Create a complex path with multiple hyphens
        let project_path = base_path
            .join("Users")
            .join("testuser")
            .join("claude-squad")
            .join("worktrees")
            .join("test-project");
        fs::create_dir_all(&project_path).unwrap();

        // Create Claude's encoded directory
        let claude_dir = base_path.join("-Users-testuser-claude-squad-worktrees-test-project");
        fs::create_dir_all(&claude_dir).unwrap();

        let test_file = claude_dir.join("test.jsonl");
        fs::write(&test_file, "{}").unwrap();

        let inference = ProjectInference::new(&test_file);
        let project_name = inference.infer_project_name();

        assert_eq!(project_name, Some("test-project".to_string()));
    }

    #[test]
    fn test_infer_project_name_fallback_to_directory_name() {
        let temp_dir = TempDir::new().unwrap();
        let base_path = temp_dir.path();

        // Create a directory that doesn't follow Claude's pattern
        let regular_dir = base_path.join("regular-project-dir");
        fs::create_dir_all(&regular_dir).unwrap();

        let test_file = regular_dir.join("test.jsonl");
        fs::write(&test_file, "{}").unwrap();

        let inference = ProjectInference::new(&test_file);
        let project_name = inference.infer_project_name();

        assert_eq!(project_name, Some("regular-project-dir".to_string()));
    }

    #[test]
    fn test_extract_project_name_from_encoded() {
        let inference = ProjectInference::new("/dummy/path");

        // Test basic pattern with "Project" keyword
        let result =
            inference.extract_project_name_from_encoded("-Users-testuser-Project-myproject");
        assert_eq!(result, Some("myproject".to_string()));

        // Test complex pattern with multiple hyphens
        let result =
            inference.extract_project_name_from_encoded("-Users-testuser-my-project-sub-folder");
        assert_eq!(result, Some("sub-folder".to_string()));

        // Test pattern without clear keywords
        let result = inference.extract_project_name_from_encoded("-Users-testuser-projectname");
        assert_eq!(result, Some("projectname".to_string()));
    }
}
