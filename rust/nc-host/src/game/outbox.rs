use rusqlite::Connection;

use crate::support::ids::new_outbox_id;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OutboxJob {
    pub game_id: String,
    pub kind: String,
}

pub fn enqueue_public_event(
    conn: &Connection,
    game_id: &str,
    kind: u32,
    content: &str,
    tags: Vec<Vec<String>>,
) -> Result<String, Box<dyn std::error::Error>> {
    let id = new_outbox_id("outbox", game_id);
    let tags = serde_json::to_string(&tags)?;
    nc_data::hosted::enqueue(conn, &id, game_id, kind, "", content, &tags)?;
    Ok(id)
}

pub fn enqueue_encrypted_event(
    conn: &Connection,
    game_id: &str,
    recipient_pubkey: &str,
    kind: u32,
    content: &str,
    tags: Vec<Vec<String>>,
) -> Result<String, Box<dyn std::error::Error>> {
    let id = new_outbox_id("outbox", game_id);
    let tags = serde_json::to_string(&tags)?;
    nc_data::hosted::enqueue(conn, &id, game_id, kind, recipient_pubkey, content, &tags)?;
    Ok(id)
}
