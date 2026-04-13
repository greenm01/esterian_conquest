use rusqlite::{Connection, Result as SqliteResult, params};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InviteRequest {
    pub id: String,
    pub game_id: String,
    pub player_pubkey: String,
    pub message: String,
    pub status: InviteRequestStatus,
    pub created_at: i64,
    pub processed_at: Option<i64>,
    pub decision_message: Option<String>,
    pub issued_invite_code: Option<String>,
    pub decision_published_at: Option<i64>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InviteRequestStatus {
    Pending,
    Approved,
    Rejected,
}

impl InviteRequestStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            InviteRequestStatus::Pending => "pending",
            InviteRequestStatus::Approved => "approved",
            InviteRequestStatus::Rejected => "rejected",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "pending" => Some(InviteRequestStatus::Pending),
            "approved" => Some(InviteRequestStatus::Approved),
            "rejected" => Some(InviteRequestStatus::Rejected),
            _ => None,
        }
    }
}

pub fn create_request(
    conn: &Connection,
    id: &str,
    game_id: &str,
    player_pubkey: &str,
    message: &str,
) -> SqliteResult<()> {
    let now = chrono::Utc::now().timestamp();
    conn.execute(
        "INSERT INTO invite_requests (id, game_id, player_pubkey, message, status, created_at)
         VALUES (?1, ?2, ?3, ?4, 'pending', ?5)",
        params![id, game_id, player_pubkey, message, now],
    )?;
    Ok(())
}

pub fn list_requests(conn: &Connection, game_id: &str) -> SqliteResult<Vec<InviteRequest>> {
    let mut stmt = conn.prepare(
        "SELECT id, game_id, player_pubkey, message, status, created_at, processed_at,
                decision_message, issued_invite_code, decision_published_at
         FROM invite_requests WHERE game_id = ?1 ORDER BY created_at DESC",
    )?;

    let requests = stmt.query_map(params![game_id], |row| {
        Ok(InviteRequest {
            id: row.get(0)?,
            game_id: row.get(1)?,
            player_pubkey: row.get(2)?,
            message: row.get(3)?,
            status: InviteRequestStatus::from_str(&row.get::<_, String>(4)?)
                .unwrap_or(InviteRequestStatus::Pending),
            created_at: row.get(5)?,
            processed_at: row.get(6)?,
            decision_message: row.get(7)?,
            issued_invite_code: row.get(8)?,
            decision_published_at: row.get(9)?,
        })
    })?;

    requests.collect()
}

pub fn get_request(conn: &Connection, id: &str) -> SqliteResult<Option<InviteRequest>> {
    let mut stmt = conn.prepare(
        "SELECT id, game_id, player_pubkey, message, status, created_at, processed_at,
                decision_message, issued_invite_code, decision_published_at
         FROM invite_requests WHERE id = ?1",
    )?;

    let mut rows = stmt.query(params![id])?;
    if let Some(row) = rows.next()? {
        Ok(Some(InviteRequest {
            id: row.get(0)?,
            game_id: row.get(1)?,
            player_pubkey: row.get(2)?,
            message: row.get(3)?,
            status: InviteRequestStatus::from_str(&row.get::<_, String>(4)?)
                .unwrap_or(InviteRequestStatus::Pending),
            created_at: row.get(5)?,
            processed_at: row.get(6)?,
            decision_message: row.get(7)?,
            issued_invite_code: row.get(8)?,
            decision_published_at: row.get(9)?,
        }))
    } else {
        Ok(None)
    }
}

pub fn approve_request(
    conn: &Connection,
    id: &str,
    decision_message: &str,
    invite_code: &str,
) -> SqliteResult<()> {
    let now = chrono::Utc::now().timestamp();
    conn.execute(
        "UPDATE invite_requests SET status = 'approved', processed_at = ?1,
         decision_message = ?2, issued_invite_code = ?3 WHERE id = ?4",
        params![now, decision_message, invite_code, id],
    )?;
    Ok(())
}

pub fn approve_request_for_seat(
    conn: &Connection,
    request_id: &str,
    game_id: &str,
    seat_number: u32,
    invite_token: &str,
    issued_invite: &str,
    decision_message: &str,
) -> SqliteResult<()> {
    conn.execute_batch("BEGIN IMMEDIATE")?;

    let seat_exists: bool = conn.query_row(
        "SELECT EXISTS(SELECT 1 FROM seats WHERE game_id = ?1 AND seat_number = ?2)",
        params![game_id, seat_number],
        |row| row.get(0),
    )?;

    let result = if seat_exists {
        super::seats::reissue_seat(conn, game_id, seat_number, invite_token)
    } else {
        super::seats::open_seat(conn, game_id, seat_number, invite_token)
    }
    .and_then(|_| approve_request(conn, request_id, decision_message, issued_invite))
    .and_then(|_| super::settings::mark_catalog_dirty(conn, game_id));

    match result {
        Ok(()) => {
            conn.execute_batch("COMMIT")?;
            Ok(())
        }
        Err(err) => {
            let _ = conn.execute_batch("ROLLBACK");
            Err(err)
        }
    }
}

pub fn reject_request(conn: &Connection, id: &str, decision_message: &str) -> SqliteResult<()> {
    let now = chrono::Utc::now().timestamp();
    conn.execute(
        "UPDATE invite_requests SET status = 'rejected', processed_at = ?1,
         decision_message = ?2 WHERE id = ?3",
        params![now, decision_message, id],
    )?;
    Ok(())
}

pub fn get_pending_request_count(
    conn: &Connection,
    game_id: &str,
    player_pubkey: &str,
) -> SqliteResult<u32> {
    let mut stmt = conn.prepare(
        "SELECT COUNT(*) FROM invite_requests
         WHERE game_id = ?1 AND player_pubkey = ?2 AND status = 'pending'",
    )?;
    stmt.query_row(params![game_id, player_pubkey], |row| row.get(0))
}

pub fn count_pending_requests(conn: &Connection, game_id: &str) -> SqliteResult<u32> {
    let mut stmt = conn.prepare(
        "SELECT COUNT(*) FROM invite_requests
         WHERE game_id = ?1 AND status = 'pending'",
    )?;
    stmt.query_row(params![game_id], |row| row.get(0))
}

pub fn count_unpublished_decisions(conn: &Connection, game_id: &str) -> SqliteResult<u32> {
    let mut stmt = conn.prepare(
        "SELECT COUNT(*) FROM invite_requests
         WHERE game_id = ?1
           AND status IN ('approved', 'rejected')
           AND decision_published_at IS NULL",
    )?;
    stmt.query_row(params![game_id], |row| row.get(0))
}

pub fn list_pending_decisions(
    conn: &Connection,
    game_id: &str,
) -> SqliteResult<Vec<InviteRequest>> {
    let mut stmt = conn.prepare(
        "SELECT id, game_id, player_pubkey, message, status, created_at, processed_at,
                decision_message, issued_invite_code, decision_published_at
         FROM invite_requests
         WHERE game_id = ?1 AND status IN ('approved', 'rejected') AND decision_published_at IS NULL
         ORDER BY processed_at ASC",
    )?;

    let requests = stmt.query_map(params![game_id], |row| {
        Ok(InviteRequest {
            id: row.get(0)?,
            game_id: row.get(1)?,
            player_pubkey: row.get(2)?,
            message: row.get(3)?,
            status: InviteRequestStatus::from_str(&row.get::<_, String>(4)?)
                .unwrap_or(InviteRequestStatus::Pending),
            created_at: row.get(5)?,
            processed_at: row.get(6)?,
            decision_message: row.get(7)?,
            issued_invite_code: row.get(8)?,
            decision_published_at: row.get(9)?,
        })
    })?;

    requests.collect()
}

pub fn mark_decision_published(conn: &Connection, id: &str) -> SqliteResult<()> {
    let now = chrono::Utc::now().timestamp();
    conn.execute(
        "UPDATE invite_requests SET decision_published_at = ?1 WHERE id = ?2",
        params![now, id],
    )?;
    Ok(())
}
