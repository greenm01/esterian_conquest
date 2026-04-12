use rusqlite::{params, Connection, Result as SqliteResult};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OutboxItem {
    pub id: String,
    pub game_id: String,
    pub kind: u32,
    pub pubkey: String,
    pub content: String,
    pub tags: String,
    pub status: OutboxStatus,
    pub created_at: i64,
    pub published_at: Option<i64>,
    pub relay_url: Option<String>,
    pub error_message: Option<String>,
    pub retry_count: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OutboxStatus {
    Pending,
    Published,
    Failed,
}

impl OutboxStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            OutboxStatus::Pending => "pending",
            OutboxStatus::Published => "published",
            OutboxStatus::Failed => "failed",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "pending" => Some(OutboxStatus::Pending),
            "published" => Some(OutboxStatus::Published),
            "failed" => Some(OutboxStatus::Failed),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutboxEvent {
    pub kind: u32,
    pub content: String,
    pub tags: Vec<Vec<String>>,
}

pub fn enqueue(
    conn: &Connection,
    id: &str,
    game_id: &str,
    kind: u32,
    pubkey: &str,
    content: &str,
    tags: &str,
) -> SqliteResult<()> {
    let now = chrono::Utc::now().timestamp();
    conn.execute(
        "INSERT INTO outbox (id, game_id, kind, pubkey, content, tags, status, created_at, retry_count)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, 'pending', ?7, 0)",
        params![id, game_id, kind, pubkey, content, tags, now],
    )?;
    Ok(())
}

pub fn get_pending(conn: &Connection, game_id: &str, limit: u32) -> SqliteResult<Vec<OutboxItem>> {
    let mut stmt = conn.prepare(
        "SELECT id, game_id, kind, pubkey, content, tags, status, created_at,
                published_at, relay_url, error_message, retry_count
         FROM outbox WHERE game_id = ?1 AND status = 'pending' AND retry_count < 5
         ORDER BY created_at LIMIT ?2",
    )?;

    let items = stmt.query_map(params![game_id, limit], |row| {
        Ok(OutboxItem {
            id: row.get(0)?,
            game_id: row.get(1)?,
            kind: row.get(2)?,
            pubkey: row.get(3)?,
            content: row.get(4)?,
            tags: row.get(5)?,
            status: OutboxStatus::from_str(&row.get::<_, String>(6)?)
                .unwrap_or(OutboxStatus::Pending),
            created_at: row.get(7)?,
            published_at: row.get(8)?,
            relay_url: row.get(9)?,
            error_message: row.get(10)?,
            retry_count: row.get(11)?,
        })
    })?;

    items.collect()
}

pub fn mark_published(conn: &Connection, id: &str, relay_url: &str) -> SqliteResult<()> {
    let now = chrono::Utc::now().timestamp();
    conn.execute(
        "UPDATE outbox SET status = 'published', published_at = ?1, relay_url = ?2 WHERE id = ?3",
        params![now, relay_url, id],
    )?;
    Ok(())
}

pub fn mark_failed(conn: &Connection, id: &str, error_message: &str) -> SqliteResult<()> {
    conn.execute(
        "UPDATE outbox
         SET status = CASE WHEN retry_count + 1 >= 5 THEN 'failed' ELSE 'pending' END,
             error_message = ?1,
             retry_count = retry_count + 1
         WHERE id = ?2",
        params![error_message, id],
    )?;
    Ok(())
}

pub fn increment_retry(conn: &Connection, id: &str) -> SqliteResult<()> {
    conn.execute(
        "UPDATE outbox SET retry_count = retry_count + 1, status = 'pending' WHERE id = ?1",
        params![id],
    )?;
    Ok(())
}

pub fn delete_published_older_than(
    conn: &Connection,
    game_id: &str,
    before_timestamp: i64,
) -> SqliteResult<u64> {
    let deleted = conn.execute(
        "DELETE FROM outbox WHERE game_id = ?1 AND status = 'published' AND published_at < ?2",
        params![game_id, before_timestamp],
    )?;
    Ok(deleted as u64)
}

pub fn count_by_status(conn: &Connection, game_id: &str, status: OutboxStatus) -> SqliteResult<u32> {
    let mut stmt = conn.prepare(
        "SELECT COUNT(*) FROM outbox WHERE game_id = ?1 AND status = ?2",
    )?;
    stmt.query_row(params![game_id, status.as_str()], |row| row.get(0))
}
