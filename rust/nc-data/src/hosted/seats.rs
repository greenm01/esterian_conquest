use rusqlite::{params, Connection, Result as SqliteResult};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Seat {
    pub id: i64,
    pub game_id: String,
    pub seat_number: u32,
    pub invite_code: String,
    pub invite_code_hash: String,
    pub player_pubkey: Option<String>,
    pub status: SeatStatus,
    pub claimed_at: Option<i64>,
    pub created_at: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SeatStatus {
    Pending,
    Claimed,
}

impl SeatStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            SeatStatus::Pending => "pending",
            SeatStatus::Claimed => "claimed",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "pending" => Some(SeatStatus::Pending),
            "claimed" => Some(SeatStatus::Claimed),
            _ => None,
        }
    }
}

pub fn list_seats(conn: &Connection, game_id: &str) -> SqliteResult<Vec<Seat>> {
    let mut stmt = conn.prepare(
        "SELECT id, game_id, seat_number, invite_code, invite_code_hash, player_pubkey,
                status, claimed_at, created_at
         FROM seats WHERE game_id = ?1 ORDER BY seat_number",
    )?;

    let seats = stmt.query_map(params![game_id], |row| {
        Ok(Seat {
            id: row.get(0)?,
            game_id: row.get(1)?,
            seat_number: row.get(2)?,
            invite_code: row.get(3)?,
            invite_code_hash: row.get(4)?,
            player_pubkey: row.get(5)?,
            status: SeatStatus::from_str(&row.get::<_, String>(6)?).unwrap_or(SeatStatus::Pending),
            claimed_at: row.get(7)?,
            created_at: row.get(8)?,
        })
    })?;

    seats.collect()
}

pub fn get_seat_by_number(
    conn: &Connection,
    game_id: &str,
    seat_number: u32,
) -> SqliteResult<Option<Seat>> {
    let mut stmt = conn.prepare(
        "SELECT id, game_id, seat_number, invite_code, invite_code_hash, player_pubkey,
                status, claimed_at, created_at
         FROM seats WHERE game_id = ?1 AND seat_number = ?2",
    )?;

    let mut rows = stmt.query(params![game_id, seat_number])?;
    if let Some(row) = rows.next()? {
        Ok(Some(Seat {
            id: row.get(0)?,
            game_id: row.get(1)?,
            seat_number: row.get(2)?,
            invite_code: row.get(3)?,
            invite_code_hash: row.get(4)?,
            player_pubkey: row.get(5)?,
            status: SeatStatus::from_str(&row.get::<_, String>(6)?).unwrap_or(SeatStatus::Pending),
            claimed_at: row.get(7)?,
            created_at: row.get(8)?,
        }))
    } else {
        Ok(None)
    }
}

pub fn get_seat_by_pubkey(
    conn: &Connection,
    game_id: &str,
    player_pubkey: &str,
) -> SqliteResult<Option<Seat>> {
    let mut stmt = conn.prepare(
        "SELECT id, game_id, seat_number, invite_code, invite_code_hash, player_pubkey,
                status, claimed_at, created_at
         FROM seats WHERE game_id = ?1 AND player_pubkey = ?2",
    )?;

    let mut rows = stmt.query(params![game_id, player_pubkey])?;
    if let Some(row) = rows.next()? {
        Ok(Some(Seat {
            id: row.get(0)?,
            game_id: row.get(1)?,
            seat_number: row.get(2)?,
            invite_code: row.get(3)?,
            invite_code_hash: row.get(4)?,
            player_pubkey: row.get(5)?,
            status: SeatStatus::from_str(&row.get::<_, String>(6)?).unwrap_or(SeatStatus::Pending),
            claimed_at: row.get(7)?,
            created_at: row.get(8)?,
        }))
    } else {
        Ok(None)
    }
}

pub fn claim_seat(
    conn: &Connection,
    game_id: &str,
    seat_number: u32,
    player_pubkey: &str,
) -> SqliteResult<()> {
    let now = chrono::Utc::now().timestamp();
    conn.execute(
        "UPDATE seats SET player_pubkey = ?1, status = 'claimed', claimed_at = ?2
         WHERE game_id = ?3 AND seat_number = ?4 AND status = 'pending'",
        params![player_pubkey, now, game_id, seat_number],
    )?;
    Ok(())
}

pub fn reset_seat(conn: &Connection, game_id: &str, seat_number: u32) -> SqliteResult<()> {
    conn.execute(
        "UPDATE seats SET player_pubkey = NULL, status = 'pending', claimed_at = NULL
         WHERE game_id = ?1 AND seat_number = ?2",
        params![game_id, seat_number],
    )?;
    Ok(())
}

pub fn reissue_seat(
    conn: &Connection,
    game_id: &str,
    seat_number: u32,
    new_invite_code: &str,
) -> SqliteResult<()> {
    let new_hash = blake3::hash(new_invite_code.as_bytes())
        .to_hex()
        .to_string();
    let now = chrono::Utc::now().timestamp();
    conn.execute(
        "UPDATE seats SET invite_code = ?1, invite_code_hash = ?2, player_pubkey = NULL,
         status = 'pending', claimed_at = NULL, created_at = ?3
         WHERE game_id = ?4 AND seat_number = ?5",
        params![new_invite_code, new_hash, now, game_id, seat_number],
    )?;
    Ok(())
}

pub fn open_seat(
    conn: &Connection,
    game_id: &str,
    seat_number: u32,
    invite_code: &str,
) -> SqliteResult<()> {
    let hash = blake3::hash(invite_code.as_bytes()).to_hex().to_string();
    let now = chrono::Utc::now().timestamp();
    conn.execute(
        "INSERT OR REPLACE INTO seats (game_id, seat_number, invite_code, invite_code_hash,
         player_pubkey, status, claimed_at, created_at)
         VALUES (?1, ?2, ?3, ?4, NULL, 'pending', NULL, ?5)",
        params![game_id, seat_number, invite_code, hash, now],
    )?;
    Ok(())
}

pub fn close_seat(conn: &Connection, game_id: &str, seat_number: u32) -> SqliteResult<()> {
    conn.execute(
        "DELETE FROM seats WHERE game_id = ?1 AND seat_number = ?2",
        params![game_id, seat_number],
    )?;
    Ok(())
}

pub fn create_seats(conn: &Connection, game_id: &str, player_count: u32) -> SqliteResult<()> {
    let now = chrono::Utc::now().timestamp();
    for i in 1..=player_count {
        let invite_code = uuid::Uuid::new_v4().to_string();
        let hash = blake3::hash(invite_code.as_bytes()).to_hex().to_string();
        conn.execute(
            "INSERT INTO seats (game_id, seat_number, invite_code, invite_code_hash,
             player_pubkey, status, claimed_at, created_at)
             VALUES (?1, ?2, ?3, ?4, NULL, 'pending', NULL, ?5)",
            params![game_id, i, invite_code, hash, now],
        )?;
    }
    Ok(())
}
