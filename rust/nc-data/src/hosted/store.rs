use rusqlite::{Connection, Result as SqliteResult};
use std::path::Path;

use super::schema::INIT_SQL;

pub struct HostedStore {
    conn: Connection,
}

impl HostedStore {
    pub fn open(path: &Path) -> SqliteResult<Self> {
        let conn = Connection::open(path)?;
        conn.execute_batch(INIT_SQL)?;
        Ok(HostedStore { conn })
    }

    pub fn create(path: &Path) -> SqliteResult<Self> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| {
                rusqlite::Error::InvalidPath(std::path::PathBuf::from(format!(
                    "failed to create parent directory: {}",
                    e
                )))
            })?;
        }
        let conn = Connection::open(path)?;
        conn.execute_batch(INIT_SQL)?;
        Ok(HostedStore { conn })
    }

    pub fn connection(&self) -> &Connection {
        &self.conn
    }
}
