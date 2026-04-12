use rusqlite::{params, Connection, Result as SqliteResult};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ThreadMessage {
    pub id: String,
    pub game_id: String,
    pub player_pubkey: String,
    pub sender_role: String,
    pub sender_pubkey: String,
    pub sender_handle: Option<String>,
    pub body: String,
    pub created_at: i64,
}

pub fn store_message(
    conn: &Connection,
    id: &str,
    game_id: &str,
    player_pubkey: &str,
    sender_role: &str,
    sender_pubkey: &str,
    sender_handle: Option<&str>,
    body: &str,
) -> SqliteResult<()> {
    let now = chrono::Utc::now().timestamp();
    conn.execute(
        "INSERT OR REPLACE INTO thread_messages
         (id, game_id, player_pubkey, sender_role, sender_pubkey, sender_handle, body, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
        params![
            id,
            game_id,
            player_pubkey,
            sender_role,
            sender_pubkey,
            sender_handle,
            body,
            now
        ],
    )?;
    Ok(())
}

pub fn list_messages(
    conn: &Connection,
    game_id: &str,
    player_pubkey: &str,
) -> SqliteResult<Vec<ThreadMessage>> {
    let mut stmt = conn.prepare(
        "SELECT id, game_id, player_pubkey, sender_role, sender_pubkey, sender_handle, body, created_at
         FROM thread_messages
         WHERE game_id = ?1 AND player_pubkey = ?2
         ORDER BY created_at ASC, id ASC",
    )?;

    let rows = stmt.query_map(params![game_id, player_pubkey], |row| {
        Ok(ThreadMessage {
            id: row.get(0)?,
            game_id: row.get(1)?,
            player_pubkey: row.get(2)?,
            sender_role: row.get(3)?,
            sender_pubkey: row.get(4)?,
            sender_handle: row.get(5)?,
            body: row.get(6)?,
            created_at: row.get(7)?,
        })
    })?;

    rows.collect()
}

pub fn list_thread_players(conn: &Connection, game_id: &str) -> SqliteResult<Vec<String>> {
    let mut stmt = conn.prepare(
        "SELECT DISTINCT player_pubkey
         FROM thread_messages
         WHERE game_id = ?1
         ORDER BY player_pubkey ASC",
    )?;
    let rows = stmt.query_map(params![game_id], |row| row.get::<_, String>(0))?;
    rows.collect()
}
