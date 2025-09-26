use anyhow::Result;
use rusqlite::Connection;

pub fn seed_database_defaults(_conn: &Connection) -> Result<()> {
    tracing::info!("No database seeding required for simplified prompt system");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::schema::create_schema;
    use rusqlite::Connection;

    #[test]
    fn test_seed_database_defaults() {
        let conn = Connection::open_in_memory().unwrap();
        create_schema(&conn).unwrap();

        assert!(seed_database_defaults(&conn).is_ok());
    }
}
