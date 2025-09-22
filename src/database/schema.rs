use rusqlite::{Connection, Result};

pub const SCHEMA_VERSION: u32 = 1;

pub fn create_schema(conn: &Connection) -> Result<()> {
    // Create schema version table
    conn.execute(
        "CREATE TABLE IF NOT EXISTS schema_versions (
            version INTEGER PRIMARY KEY,
            applied_at TEXT NOT NULL DEFAULT (datetime('now', 'utc'))
        )",
        [],
    )?;

    // Chat sessions table
    conn.execute(
        "CREATE TABLE IF NOT EXISTS chat_sessions (
            id TEXT PRIMARY KEY,
            provider TEXT NOT NULL,
            project_name TEXT,
            start_time TEXT NOT NULL,
            end_time TEXT,
            message_count INTEGER NOT NULL DEFAULT 0,
            token_count INTEGER,
            file_path TEXT NOT NULL,
            file_hash TEXT NOT NULL,
            created_at TEXT NOT NULL DEFAULT (datetime('now', 'utc')),
            updated_at TEXT NOT NULL DEFAULT (datetime('now', 'utc')),
            state TEXT NOT NULL DEFAULT 'Created',
            FOREIGN KEY (project_name) REFERENCES projects(name),
            UNIQUE(file_hash, file_path)
        )",
        [],
    )?;

    // Messages table
    conn.execute(
        "CREATE TABLE IF NOT EXISTS messages (
            id TEXT PRIMARY KEY,
            session_id TEXT NOT NULL,
            role TEXT NOT NULL CHECK (role IN ('User', 'Assistant', 'System')),
            content TEXT NOT NULL CHECK (length(content) > 0),
            timestamp TEXT NOT NULL,
            token_count INTEGER CHECK (token_count >= 0),
            tool_calls TEXT, -- JSON array
            metadata TEXT,   -- JSON object
            sequence_number INTEGER NOT NULL,
            FOREIGN KEY (session_id) REFERENCES chat_sessions(id) ON DELETE CASCADE,
            UNIQUE(session_id, sequence_number)
        )",
        [],
    )?;

    // Projects table
    conn.execute(
        "CREATE TABLE IF NOT EXISTS projects (
            id TEXT PRIMARY KEY,
            name TEXT NOT NULL UNIQUE,
            description TEXT,
            working_directory TEXT,
            created_at TEXT NOT NULL DEFAULT (datetime('now', 'utc')),
            updated_at TEXT NOT NULL DEFAULT (datetime('now', 'utc')),
            session_count INTEGER NOT NULL DEFAULT 0,
            total_tokens INTEGER NOT NULL DEFAULT 0
        )",
        [],
    )?;

    // Usage analysis table
    conn.execute(
        "CREATE TABLE IF NOT EXISTS usage_analyses (
            id TEXT PRIMARY KEY,
            analysis_type TEXT NOT NULL,
            time_period_start TEXT NOT NULL,
            time_period_end TEXT NOT NULL,
            provider_filter TEXT,
            project_filter TEXT,
            total_sessions INTEGER NOT NULL DEFAULT 0,
            total_messages INTEGER NOT NULL DEFAULT 0,
            total_tokens INTEGER NOT NULL DEFAULT 0,
            average_session_length REAL NOT NULL DEFAULT 0,
            most_active_day TEXT,
            purpose_categories TEXT, -- JSON object
            quality_scores TEXT,     -- JSON object
            recommendations TEXT,    -- JSON array
            generated_at TEXT NOT NULL DEFAULT (datetime('now', 'utc'))
        )",
        [],
    )?;

    // Provider configuration table
    conn.execute(
        "CREATE TABLE IF NOT EXISTS llm_providers (
            id TEXT PRIMARY KEY,
            name TEXT NOT NULL,
            file_patterns TEXT NOT NULL, -- JSON array
            default_locations TEXT NOT NULL, -- JSON object
            parser_type TEXT NOT NULL,
            supports_tokens BOOLEAN NOT NULL DEFAULT FALSE,
            supports_tools BOOLEAN NOT NULL DEFAULT FALSE,
            last_updated TEXT NOT NULL DEFAULT (datetime('now', 'utc'))
        )",
        [],
    )?;

    create_indexes(conn)?;
    create_triggers(conn)?;
    create_fts_table(conn)?;

    Ok(())
}

pub fn create_indexes(conn: &Connection) -> Result<()> {
    // Session queries
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_sessions_provider ON chat_sessions(provider)",
        [],
    )?;
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_sessions_project ON chat_sessions(project_name)",
        [],
    )?;
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_sessions_start_time ON chat_sessions(start_time)",
        [],
    )?;
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_sessions_file_hash ON chat_sessions(file_hash)",
        [],
    )?;
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_sessions_state ON chat_sessions(state)",
        [],
    )?;

    // Message queries
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_messages_session ON messages(session_id)",
        [],
    )?;
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_messages_timestamp ON messages(timestamp)",
        [],
    )?;
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_messages_role ON messages(role)",
        [],
    )?;
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_messages_sequence ON messages(session_id, sequence_number)",
        [],
    )?;

    // Analysis queries
    conn.execute("CREATE INDEX IF NOT EXISTS idx_analysis_type_period ON usage_analyses(analysis_type, time_period_start, time_period_end)", [])?;
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_analysis_provider ON usage_analyses(provider_filter)",
        [],
    )?;
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_analysis_project ON usage_analyses(project_filter)",
        [],
    )?;

    // Project queries
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_projects_name ON projects(name)",
        [],
    )?;

    Ok(())
}

pub fn create_triggers(conn: &Connection) -> Result<()> {
    // Update session message count when messages added/removed
    conn.execute(
        "CREATE TRIGGER IF NOT EXISTS update_session_message_count_insert
            AFTER INSERT ON messages
        BEGIN
            UPDATE chat_sessions
            SET message_count = (
                SELECT COUNT(*) FROM messages WHERE session_id = NEW.session_id
            ),
            updated_at = datetime('now', 'utc')
            WHERE id = NEW.session_id;
        END",
        [],
    )?;

    conn.execute(
        "CREATE TRIGGER IF NOT EXISTS update_session_message_count_delete
            AFTER DELETE ON messages
        BEGIN
            UPDATE chat_sessions
            SET message_count = (
                SELECT COUNT(*) FROM messages WHERE session_id = OLD.session_id
            ),
            updated_at = datetime('now', 'utc')
            WHERE id = OLD.session_id;
        END",
        [],
    )?;

    // Update project aggregates when sessions change
    conn.execute(
        "CREATE TRIGGER IF NOT EXISTS update_project_aggregates_insert
            AFTER INSERT ON chat_sessions
        BEGIN
            UPDATE projects
            SET session_count = session_count + 1,
                total_tokens = total_tokens + COALESCE(NEW.token_count, 0),
                updated_at = datetime('now', 'utc')
            WHERE name = NEW.project_name;
        END",
        [],
    )?;

    conn.execute(
        "CREATE TRIGGER IF NOT EXISTS update_project_aggregates_delete
            AFTER DELETE ON chat_sessions
        BEGIN
            UPDATE projects
            SET session_count = session_count - 1,
                total_tokens = total_tokens - COALESCE(OLD.token_count, 0),
                updated_at = datetime('now', 'utc')
            WHERE name = OLD.project_name;
        END",
        [],
    )?;

    conn.execute(
        "CREATE TRIGGER IF NOT EXISTS update_project_aggregates_update
            AFTER UPDATE ON chat_sessions
        BEGIN
            -- Update old project if project changed
            UPDATE projects
            SET session_count = session_count - 1,
                total_tokens = total_tokens - COALESCE(OLD.token_count, 0),
                updated_at = datetime('now', 'utc')
            WHERE name = OLD.project_name AND OLD.project_name != NEW.project_name;

            -- Update new project if project changed
            UPDATE projects
            SET session_count = session_count + 1,
                total_tokens = total_tokens + COALESCE(NEW.token_count, 0),
                updated_at = datetime('now', 'utc')
            WHERE name = NEW.project_name AND OLD.project_name != NEW.project_name;

            -- Update token count if same project but tokens changed
            UPDATE projects
            SET total_tokens = total_tokens - COALESCE(OLD.token_count, 0) + COALESCE(NEW.token_count, 0),
                updated_at = datetime('now', 'utc')
            WHERE name = NEW.project_name AND OLD.project_name = NEW.project_name;
        END",
        [],
    )?;

    // Update session updated_at on message changes
    conn.execute(
        "CREATE TRIGGER IF NOT EXISTS update_session_timestamp
            AFTER UPDATE ON messages
        BEGIN
            UPDATE chat_sessions
            SET updated_at = CURRENT_TIMESTAMP
            WHERE id = NEW.session_id;
        END",
        [],
    )?;

    Ok(())
}

pub fn create_fts_table(conn: &Connection) -> Result<()> {
    // Full-text search on message content
    conn.execute(
        "CREATE VIRTUAL TABLE IF NOT EXISTS messages_fts USING fts5(
            content,
            session_id UNINDEXED,
            message_id UNINDEXED
        )",
        [],
    )?;

    // Trigger to populate FTS table
    conn.execute(
        "CREATE TRIGGER IF NOT EXISTS messages_fts_insert
            AFTER INSERT ON messages
        BEGIN
            INSERT INTO messages_fts(content, session_id, message_id)
            VALUES (NEW.content, NEW.session_id, NEW.id);
        END",
        [],
    )?;

    conn.execute(
        "CREATE TRIGGER IF NOT EXISTS messages_fts_delete
            AFTER DELETE ON messages
        BEGIN
            DELETE FROM messages_fts WHERE message_id = OLD.id;
        END",
        [],
    )?;

    conn.execute(
        "CREATE TRIGGER IF NOT EXISTS messages_fts_update
            AFTER UPDATE ON messages
        BEGIN
            DELETE FROM messages_fts WHERE message_id = OLD.id;
            INSERT INTO messages_fts(content, session_id, message_id)
            VALUES (NEW.content, NEW.session_id, NEW.id);
        END",
        [],
    )?;

    Ok(())
}

pub fn drop_schema(conn: &Connection) -> Result<()> {
    // Drop tables in reverse dependency order
    conn.execute("DROP TABLE IF EXISTS messages_fts", [])?;
    conn.execute("DROP TABLE IF EXISTS usage_analyses", [])?;
    conn.execute("DROP TABLE IF EXISTS messages", [])?;
    conn.execute("DROP TABLE IF EXISTS chat_sessions", [])?;
    conn.execute("DROP TABLE IF EXISTS projects", [])?;
    conn.execute("DROP TABLE IF EXISTS llm_providers", [])?;
    conn.execute("DROP TABLE IF EXISTS schema_versions", [])?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::Connection;

    #[test]
    fn test_create_schema() {
        let conn = Connection::open_in_memory().unwrap();
        create_schema(&conn).unwrap();

        // Verify tables exist
        let mut stmt = conn
            .prepare("SELECT name FROM sqlite_master WHERE type='table'")
            .unwrap();
        let table_names: Vec<String> = stmt
            .query_map([], |row| row.get::<_, String>(0))
            .unwrap()
            .collect::<Result<Vec<_>>>()
            .unwrap();

        assert!(table_names.contains(&"chat_sessions".to_string()));
        assert!(table_names.contains(&"messages".to_string()));
        assert!(table_names.contains(&"projects".to_string()));
        assert!(table_names.contains(&"usage_analyses".to_string()));
        assert!(table_names.contains(&"llm_providers".to_string()));
        assert!(table_names.contains(&"messages_fts".to_string()));
    }

    #[test]
    fn test_schema_constraints() {
        let conn = Connection::open_in_memory().unwrap();
        create_schema(&conn).unwrap();

        // Test message role constraint
        let result = conn.execute(
            "INSERT INTO messages (id, session_id, role, content, timestamp, sequence_number)
             VALUES ('test', 'session1', 'InvalidRole', 'content', '2023-01-01T00:00:00Z', 1)",
            [],
        );
        assert!(result.is_err());

        // Test empty content constraint
        let result = conn.execute(
            "INSERT INTO messages (id, session_id, role, content, timestamp, sequence_number)
             VALUES ('test', 'session1', 'User', '', '2023-01-01T00:00:00Z', 1)",
            [],
        );
        assert!(result.is_err());
    }

    #[test]
    fn test_drop_schema() {
        let conn = Connection::open_in_memory().unwrap();
        create_schema(&conn).unwrap();
        drop_schema(&conn).unwrap();

        // Verify tables are dropped
        let mut stmt = conn
            .prepare("SELECT name FROM sqlite_master WHERE type='table'")
            .unwrap();
        let table_names: Vec<String> = stmt
            .query_map([], |row| row.get::<_, String>(0))
            .unwrap()
            .collect::<Result<Vec<_>>>()
            .unwrap();

        // Only sqlite internal tables should remain
        assert!(!table_names.contains(&"chat_sessions".to_string()));
        assert!(!table_names.contains(&"messages".to_string()));
    }
}
