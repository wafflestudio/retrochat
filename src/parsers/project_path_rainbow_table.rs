use sha2::{Digest, Sha256};
use std::collections::HashMap;

/// Cross-Provider Rainbow Table for mapping projectHash to project_path
///
/// Gemini CLI stores only SHA256 hashes of project paths, making reverse lookup impossible.
/// This Rainbow Table collects project_path values from other providers (Claude, Cursor, Codex)
/// and creates a hash-to-path mapping, allowing us to recover the original path for Gemini sessions.
///
/// Expected Coverage: 60-95% depending on provider combination
/// Accuracy: 100% (SHA256 collision-free in practice)
#[derive(Debug, Clone)]
pub struct ProjectPathRainbowTable {
    hash_to_path: HashMap<String, String>,
}

impl ProjectPathRainbowTable {
    /// Create a new empty Rainbow Table
    pub fn new() -> Self {
        Self {
            hash_to_path: HashMap::new(),
        }
    }

    /// Add a project path to the Rainbow Table
    ///
    /// Computes SHA256 hash and stores the mapping
    pub fn add_path(&mut self, path: &str) {
        if path.is_empty() {
            return;
        }

        let hash = Self::compute_hash(path);
        self.hash_to_path.insert(hash, path.to_string());
    }

    /// Compute SHA256 hash of a project path (matching Gemini CLI's implementation)
    ///
    /// Gemini CLI uses: `crypto.createHash('sha256').update(projectRoot).digest('hex')`
    pub fn compute_hash(path: &str) -> String {
        let mut hasher = Sha256::new();
        hasher.update(path.as_bytes());
        format!("{:x}", hasher.finalize())
    }

    /// Lookup project path by hash
    ///
    /// Returns the original project path if found in the Rainbow Table
    pub fn lookup(&self, project_hash: &str) -> Option<&String> {
        self.hash_to_path.get(project_hash)
    }

    /// Get the number of entries in the Rainbow Table
    pub fn len(&self) -> usize {
        self.hash_to_path.len()
    }

    /// Check if the Rainbow Table is empty
    pub fn is_empty(&self) -> bool {
        self.hash_to_path.is_empty()
    }

    /// Verify if a project path matches the given hash
    pub fn verify(path: &str, expected_hash: &str) -> bool {
        let computed_hash = Self::compute_hash(path);
        computed_hash == expected_hash
    }
}

impl Default for ProjectPathRainbowTable {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compute_hash() {
        // Test vectors from the research document
        let path1 = "/Users/lullu/study/retrochat";
        let expected1 = "658435c1dc7019e23d4d83de3afb2fdde4012c7a6122680b5be7c8698ce0e516";
        assert_eq!(ProjectPathRainbowTable::compute_hash(path1), expected1);

        let path2 = "/Users/lullu/projects/x37";
        let expected2 = "b7c62724c472dd2ca9fdc8305211b643b0d7f3a29825d8c80891e84bd5647f48";
        assert_eq!(ProjectPathRainbowTable::compute_hash(path2), expected2);

        let path3 = "/Users/lullu/turing/gpai/gpai-monorepo-3";
        let expected3 = "f80b91b389f45890e9ed84c0e946f4514a27619060d440aa28896b40da3265fe";
        assert_eq!(ProjectPathRainbowTable::compute_hash(path3), expected3);
    }

    #[test]
    fn test_add_and_lookup() {
        let mut table = ProjectPathRainbowTable::new();

        table.add_path("/Users/lullu/study/retrochat");
        table.add_path("/Users/lullu/projects/x37");

        let hash1 = "658435c1dc7019e23d4d83de3afb2fdde4012c7a6122680b5be7c8698ce0e516";
        assert_eq!(
            table.lookup(hash1),
            Some(&"/Users/lullu/study/retrochat".to_string())
        );

        let hash2 = "b7c62724c472dd2ca9fdc8305211b643b0d7f3a29825d8c80891e84bd5647f48";
        assert_eq!(
            table.lookup(hash2),
            Some(&"/Users/lullu/projects/x37".to_string())
        );

        let nonexistent = "0000000000000000000000000000000000000000000000000000000000000000";
        assert_eq!(table.lookup(nonexistent), None);
    }

    #[test]
    fn test_verify() {
        let path = "/Users/lullu/study/retrochat";
        let correct_hash = "658435c1dc7019e23d4d83de3afb2fdde4012c7a6122680b5be7c8698ce0e516";
        let wrong_hash = "0000000000000000000000000000000000000000000000000000000000000000";

        assert!(ProjectPathRainbowTable::verify(path, correct_hash));
        assert!(!ProjectPathRainbowTable::verify(path, wrong_hash));
    }

    #[test]
    fn test_empty_path() {
        let mut table = ProjectPathRainbowTable::new();
        table.add_path("");

        assert_eq!(table.len(), 0); // Empty paths should not be added
    }
}
