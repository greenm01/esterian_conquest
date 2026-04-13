use rusqlite::{Connection, Result as SqliteResult, params};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlayerMessage {
    pub id: String,
    pub game_id: String,
    pub sender_pubkey: String,
    pub sender_empire_id: u8,
    pub sender_empire_name: String,
    pub recipient_pubkey: String,
    pub recipient_empire_id: u8,
    pub recipient_empire_name: String,
    pub body: String,
    pub created_at: i64,
}

pub fn store_message(
    conn: &Connection,
    id: &str,
    game_id: &str,
    sender_pubkey: &str,
    sender_empire_id: u8,
    sender_empire_name: &str,
    recipient_pubkey: &str,
    recipient_empire_id: u8,
    recipient_empire_name: &str,
    body: &str,
) -> SqliteResult<()> {
    let now = chrono::Utc::now().timestamp();
    conn.execute(
        "INSERT OR REPLACE INTO player_messages
         (id, game_id, sender_pubkey, sender_empire_id, sender_empire_name,
          recipient_pubkey, recipient_empire_id, recipient_empire_name, body, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
        params![
            id,
            game_id,
            sender_pubkey,
            i64::from(sender_empire_id),
            sender_empire_name,
            recipient_pubkey,
            i64::from(recipient_empire_id),
            recipient_empire_name,
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
) -> SqliteResult<Vec<PlayerMessage>> {
    let mut stmt = conn.prepare(
        "SELECT id, game_id, sender_pubkey, sender_empire_id, sender_empire_name,
                recipient_pubkey, recipient_empire_id, recipient_empire_name, body, created_at
         FROM player_messages
         WHERE game_id = ?1 AND (sender_pubkey = ?2 OR recipient_pubkey = ?2)
         ORDER BY created_at ASC, id ASC",
    )?;

    let rows = stmt.query_map(params![game_id, player_pubkey], |row| {
        Ok(PlayerMessage {
            id: row.get(0)?,
            game_id: row.get(1)?,
            sender_pubkey: row.get(2)?,
            sender_empire_id: row.get::<_, u8>(3)?,
            sender_empire_name: row.get(4)?,
            recipient_pubkey: row.get(5)?,
            recipient_empire_id: row.get::<_, u8>(6)?,
            recipient_empire_name: row.get(7)?,
            body: row.get(8)?,
            created_at: row.get(9)?,
        })
    })?;

    rows.collect()
}
