use rusqlite::{Connection, Result as SqliteResult};
use std::path::Path;

use super::schema::INIT_SQL;

const MIGRATIONS: &[&str] = &[
    "ALTER TABLE game_metadata ADD COLUMN game_tier TEXT NOT NULL DEFAULT 'league'",
    "ALTER TABLE game_metadata ADD COLUMN catalog_state TEXT NOT NULL DEFAULT 'listed'",
    "ALTER TABLE seats ADD COLUMN claimed_year INTEGER",
];

pub struct HostedStore {
    conn: Connection,
}

impl HostedStore {
    pub fn open(path: &Path) -> SqliteResult<Self> {
        let conn = Connection::open(path)?;
        conn.execute_batch(INIT_SQL)?;
        run_migrations(&conn);
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
        run_migrations(&conn);
        Ok(HostedStore { conn })
    }

    pub fn connection(&self) -> &Connection {
        &self.conn
    }
}

fn run_migrations(conn: &Connection) {
    for sql in MIGRATIONS {
        // Ignore errors — duplicate column means the migration already ran.
        let _ = conn.execute_batch(sql);
    }
}
