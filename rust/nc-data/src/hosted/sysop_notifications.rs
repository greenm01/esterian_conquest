use rusqlite::{Connection, Result as SqliteResult, params};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SysopNotification {
    pub id: String,
    pub game_id: String,
    pub player_pubkey: String,
    pub category: String,
    pub summary: String,
    pub status: SysopNotificationStatus,
    pub created_at: i64,
    pub sent_at: Option<i64>,
    pub error_message: Option<String>,
    pub retry_count: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SysopNotificationStatus {
    Pending,
    Sent,
    Failed,
}

impl SysopNotificationStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Sent => "sent",
            Self::Failed => "failed",
        }
    }

    pub fn from_str(value: &str) -> Option<Self> {
        match value {
            "pending" => Some(Self::Pending),
            "sent" => Some(Self::Sent),
            "failed" => Some(Self::Failed),
            _ => None,
        }
    }
}

pub fn enqueue(
    conn: &Connection,
    id: &str,
    game_id: &str,
    player_pubkey: &str,
    category: &str,
    summary: &str,
) -> SqliteResult<()> {
    let now = chrono::Utc::now().timestamp();
    conn.execute(
        "INSERT OR IGNORE INTO sysop_notifications
         (id, game_id, player_pubkey, category, summary, status, created_at, retry_count)
         VALUES (?1, ?2, ?3, ?4, ?5, 'pending', ?6, 0)",
        params![id, game_id, player_pubkey, category, summary, now],
    )?;
    Ok(())
}

pub fn get_pending(
    conn: &Connection,
    game_id: &str,
    limit: u32,
) -> SqliteResult<Vec<SysopNotification>> {
    let mut stmt = conn.prepare(
        "SELECT id, game_id, player_pubkey, category, summary, status, created_at,
                sent_at, error_message, retry_count
         FROM sysop_notifications
         WHERE game_id = ?1 AND status = 'pending' AND retry_count < 5
         ORDER BY created_at ASC
         LIMIT ?2",
    )?;

    let rows = stmt.query_map(params![game_id, limit], |row| {
        Ok(SysopNotification {
            id: row.get(0)?,
            game_id: row.get(1)?,
            player_pubkey: row.get(2)?,
            category: row.get(3)?,
            summary: row.get(4)?,
            status: SysopNotificationStatus::from_str(&row.get::<_, String>(5)?)
                .unwrap_or(SysopNotificationStatus::Pending),
            created_at: row.get(6)?,
            sent_at: row.get(7)?,
            error_message: row.get(8)?,
            retry_count: row.get(9)?,
        })
    })?;

    rows.collect()
}

pub fn mark_sent(conn: &Connection, id: &str) -> SqliteResult<()> {
    let now = chrono::Utc::now().timestamp();
    conn.execute(
        "UPDATE sysop_notifications
         SET status = 'sent', sent_at = ?1
         WHERE id = ?2",
        params![now, id],
    )?;
    Ok(())
}

pub fn mark_failed(conn: &Connection, id: &str, error_message: &str) -> SqliteResult<()> {
    conn.execute(
        "UPDATE sysop_notifications
         SET status = CASE WHEN retry_count + 1 >= 5 THEN 'failed' ELSE 'pending' END,
             error_message = ?1,
             retry_count = retry_count + 1
         WHERE id = ?2",
        params![error_message, id],
    )?;
    Ok(())
}
