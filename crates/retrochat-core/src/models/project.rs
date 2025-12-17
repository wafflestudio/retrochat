use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Project {
    pub id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub working_directory: Option<PathBuf>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub session_count: u32,
    pub total_tokens: u64,
}

impl Project {
    pub fn new(name: String) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            name,
            description: None,
            working_directory: None,
            created_at: now,
            updated_at: now,
            session_count: 0,
            total_tokens: 0,
        }
    }

    pub fn with_description(mut self, description: String) -> Self {
        self.description = Some(description);
        self
    }

    pub fn with_working_directory(mut self, working_directory: PathBuf) -> Self {
        self.working_directory = Some(working_directory);
        self
    }

    pub fn add_session(&mut self, token_count: Option<u32>) {
        self.session_count += 1;
        if let Some(tokens) = token_count {
            self.total_tokens += tokens as u64;
        }
        self.updated_at = Utc::now();
    }

    pub fn remove_session(&mut self, token_count: Option<u32>) {
        if self.session_count > 0 {
            self.session_count -= 1;
        }
        if let Some(tokens) = token_count {
            self.total_tokens = self.total_tokens.saturating_sub(tokens as u64);
        }
        self.updated_at = Utc::now();
    }

    pub fn update_token_count(&mut self, old_count: Option<u32>, new_count: Option<u32>) {
        if let Some(old_tokens) = old_count {
            self.total_tokens = self.total_tokens.saturating_sub(old_tokens as u64);
        }
        if let Some(new_tokens) = new_count {
            self.total_tokens += new_tokens as u64;
        }
        self.updated_at = Utc::now();
    }

    pub fn average_tokens_per_session(&self) -> f64 {
        if self.session_count == 0 {
            0.0
        } else {
            self.total_tokens as f64 / self.session_count as f64
        }
    }

    pub fn is_valid(&self) -> bool {
        !self.name.trim().is_empty()
    }

    pub fn has_working_directory(&self) -> bool {
        self.working_directory.is_some()
    }

    pub fn working_directory_exists(&self) -> bool {
        self.working_directory
            .as_ref()
            .is_some_and(|path| path.exists())
    }

    pub fn display_name(&self) -> &str {
        &self.name
    }

    pub fn summary(&self) -> String {
        format!(
            "{} ({} sessions, {} total tokens)",
            self.name, self.session_count, self.total_tokens
        )
    }
}

impl PartialEq for Project {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl Eq for Project {}

impl PartialOrd for Project {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Project {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.name.cmp(&other.name)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_project() {
        let name = "Test Project".to_string();
        let project = Project::new(name.clone());

        assert_eq!(project.name, name);
        assert!(project.description.is_none());
        assert!(project.working_directory.is_none());
        assert_eq!(project.session_count, 0);
        assert_eq!(project.total_tokens, 0);
        assert!(project.is_valid());
    }

    #[test]
    fn test_project_with_empty_name_is_invalid() {
        let project = Project::new("".to_string());
        assert!(!project.is_valid());

        let project = Project::new("   ".to_string());
        assert!(!project.is_valid());
    }

    #[test]
    fn test_add_session() {
        let mut project = Project::new("Test".to_string());

        project.add_session(Some(100));
        assert_eq!(project.session_count, 1);
        assert_eq!(project.total_tokens, 100);

        project.add_session(Some(200));
        assert_eq!(project.session_count, 2);
        assert_eq!(project.total_tokens, 300);

        project.add_session(None);
        assert_eq!(project.session_count, 3);
        assert_eq!(project.total_tokens, 300);
    }

    #[test]
    fn test_remove_session() {
        let mut project = Project::new("Test".to_string());
        project.add_session(Some(300));
        project.add_session(Some(200));

        project.remove_session(Some(100));
        assert_eq!(project.session_count, 1);
        assert_eq!(project.total_tokens, 400);

        project.remove_session(None);
        assert_eq!(project.session_count, 0);
        assert_eq!(project.total_tokens, 400);

        // Should not go below 0
        project.remove_session(Some(500));
        assert_eq!(project.session_count, 0);
        assert_eq!(project.total_tokens, 0);
    }

    #[test]
    fn test_average_tokens_per_session() {
        let mut project = Project::new("Test".to_string());
        assert_eq!(project.average_tokens_per_session(), 0.0);

        project.add_session(Some(100));
        project.add_session(Some(200));
        assert_eq!(project.average_tokens_per_session(), 150.0);
    }

    #[test]
    fn test_update_token_count() {
        let mut project = Project::new("Test".to_string());
        project.add_session(Some(100));

        project.update_token_count(Some(100), Some(200));
        assert_eq!(project.total_tokens, 200);

        project.update_token_count(Some(200), None);
        assert_eq!(project.total_tokens, 0);
    }

    #[test]
    fn test_with_description() {
        let project =
            Project::new("Test".to_string()).with_description("A test project".to_string());

        assert_eq!(project.description.unwrap(), "A test project");
    }

    #[test]
    fn test_with_working_directory() {
        let path = PathBuf::from("/tmp/test");
        let project = Project::new("Test".to_string()).with_working_directory(path.clone());

        assert_eq!(project.working_directory.as_ref().unwrap(), &path);
        assert!(project.has_working_directory());
    }

    #[test]
    fn test_project_ordering() {
        let project1 = Project::new("Alpha".to_string());
        let project2 = Project::new("Beta".to_string());

        assert!(project1 < project2);
    }

    #[test]
    fn test_summary() {
        let mut project = Project::new("Test Project".to_string());
        project.add_session(Some(100));
        project.add_session(Some(200));

        let summary = project.summary();
        assert_eq!(summary, "Test Project (2 sessions, 300 total tokens)");
    }
}
