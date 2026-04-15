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
    pub assigned_seat: Option<u32>,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SandboxApprovalOutcome {
    Claimed { seat: u32 },
    Full,
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

pub fn delete_request(conn: &Connection, id: &str) -> SqliteResult<()> {
    conn.execute("DELETE FROM invite_requests WHERE id = ?1", params![id])?;
    Ok(())
}

pub fn list_requests(conn: &Connection, game_id: &str) -> SqliteResult<Vec<InviteRequest>> {
    let mut stmt = conn.prepare(
        "SELECT id, game_id, player_pubkey, message, status, created_at, processed_at,
                decision_message, assigned_seat, issued_invite_code, decision_published_at
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
            assigned_seat: row.get(8)?,
            issued_invite_code: row.get(9)?,
            decision_published_at: row.get(10)?,
        })
    })?;

    requests.collect()
}

pub fn get_request(conn: &Connection, id: &str) -> SqliteResult<Option<InviteRequest>> {
    let mut stmt = conn.prepare(
        "SELECT id, game_id, player_pubkey, message, status, created_at, processed_at,
                decision_message, assigned_seat, issued_invite_code, decision_published_at
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
            assigned_seat: row.get(8)?,
            issued_invite_code: row.get(9)?,
            decision_published_at: row.get(10)?,
        }))
    } else {
        Ok(None)
    }
}

pub fn approve_request(
    conn: &Connection,
    id: &str,
    decision_message: &str,
    assigned_seat: u32,
    invite_code: Option<&str>,
) -> SqliteResult<()> {
    let now = chrono::Utc::now().timestamp();
    conn.execute(
        "UPDATE invite_requests SET status = 'approved', processed_at = ?1,
         decision_message = ?2, assigned_seat = ?3, issued_invite_code = ?4 WHERE id = ?5",
        params![now, decision_message, assigned_seat, invite_code, id],
    )?;
    Ok(())
}

pub fn approve_request_for_seat(
    conn: &Connection,
    request_id: &str,
    game_id: &str,
    seat_number: u32,
    player_pubkey: &str,
    claimed_year: u16,
    reserve_invite_token: &str,
    decision_message: &str,
) -> SqliteResult<()> {
    conn.execute_batch("BEGIN IMMEDIATE")?;

    let result = match super::seats::get_seat_by_number(conn, game_id, seat_number)? {
        Some(seat) if seat.status == super::seats::SeatStatus::Pending => {
            super::seats::claim_seat(conn, game_id, seat_number, player_pubkey, claimed_year)
        }
        Some(seat) if seat.player_pubkey.as_deref() == Some(player_pubkey) => Ok(()),
        Some(_) => Err(rusqlite::Error::InvalidParameterName(format!(
            "seat {seat_number} is already claimed"
        ))),
        None => super::seats::open_seat(conn, game_id, seat_number, reserve_invite_token).and_then(
            |_| super::seats::claim_seat(conn, game_id, seat_number, player_pubkey, claimed_year),
        ),
    }
    .and_then(|_| approve_request(conn, request_id, decision_message, seat_number, None))
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

pub fn auto_approve_sandbox_request(
    conn: &Connection,
    request_id: &str,
    game_id: &str,
    player_pubkey: &str,
    claimed_year: u16,
    decision_message: &str,
) -> SqliteResult<SandboxApprovalOutcome> {
    conn.execute_batch("BEGIN IMMEDIATE")?;

    let result = (|| -> SqliteResult<SandboxApprovalOutcome> {
        if let Some(existing) = super::seats::get_seat_by_pubkey(conn, game_id, player_pubkey)? {
            if existing.status == super::seats::SeatStatus::Claimed {
                approve_request(
                    conn,
                    request_id,
                    decision_message,
                    existing.seat_number,
                    None,
                )?;
                return Ok(SandboxApprovalOutcome::Claimed {
                    seat: existing.seat_number,
                });
            }
        }

        let Some(open_seat) = super::seats::list_seats(conn, game_id)?
            .into_iter()
            .find(|seat| seat.status == super::seats::SeatStatus::Pending)
        else {
            return Ok(SandboxApprovalOutcome::Full);
        };

        super::seats::claim_seat(
            conn,
            game_id,
            open_seat.seat_number,
            player_pubkey,
            claimed_year,
        )?;
        approve_request(
            conn,
            request_id,
            decision_message,
            open_seat.seat_number,
            None,
        )?;
        super::settings::mark_catalog_dirty(conn, game_id)?;
        Ok(SandboxApprovalOutcome::Claimed {
            seat: open_seat.seat_number,
        })
    })();

    match result {
        Ok(outcome) => {
            conn.execute_batch("COMMIT")?;
            Ok(outcome)
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
                decision_message, assigned_seat, issued_invite_code, decision_published_at
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
            assigned_seat: row.get(8)?,
            issued_invite_code: row.get(9)?,
            decision_published_at: row.get(10)?,
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
