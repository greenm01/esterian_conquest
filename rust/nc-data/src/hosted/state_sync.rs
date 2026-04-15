use nc_nostr::state_sync::GameState;
use rusqlite::{Connection, OptionalExtension, params};

const SNAPSHOT_RETENTION_LIMIT: i64 = 10;

pub fn save_state_snapshot(
    conn: &Connection,
    game_id: &str,
    seat_number: u32,
    state: &GameState,
) -> rusqlite::Result<()> {
    conn.execute(
        "INSERT INTO hosted_state_snapshots
         (game_id, seat_number, state_hash, turn, year, payload_json, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, unixepoch())
         ON CONFLICT(game_id, seat_number, state_hash) DO UPDATE SET
             turn = excluded.turn,
             year = excluded.year,
             payload_json = excluded.payload_json,
             created_at = unixepoch()",
        params![
            game_id,
            seat_number,
            state.state_hash,
            state.turn,
            state.year,
            serde_json::to_string(state).map_err(json_err)?,
        ],
    )?;
    prune_old_state_snapshots(conn, game_id, seat_number)?;
    Ok(())
}

pub fn get_latest_state_snapshot(
    conn: &Connection,
    game_id: &str,
    seat_number: u32,
) -> rusqlite::Result<Option<GameState>> {
    load_snapshot_row(
        conn,
        "SELECT payload_json
         FROM hosted_state_snapshots
         WHERE game_id = ?1 AND seat_number = ?2
         ORDER BY created_at DESC
         LIMIT 1",
        params![game_id, seat_number],
    )
}

pub fn get_state_snapshot_by_hash(
    conn: &Connection,
    game_id: &str,
    seat_number: u32,
    state_hash: &str,
) -> rusqlite::Result<Option<GameState>> {
    load_snapshot_row(
        conn,
        "SELECT payload_json
         FROM hosted_state_snapshots
         WHERE game_id = ?1 AND seat_number = ?2 AND state_hash = ?3",
        params![game_id, seat_number, state_hash],
    )
}

fn load_snapshot_row<P: rusqlite::Params>(
    conn: &Connection,
    sql: &str,
    params: P,
) -> rusqlite::Result<Option<GameState>> {
    let payload = conn
        .query_row(sql, params, |row| row.get::<_, String>(0))
        .optional()?;
    payload
        .map(|json| serde_json::from_str(&json).map_err(json_err))
        .transpose()
}

fn prune_old_state_snapshots(
    conn: &Connection,
    game_id: &str,
    seat_number: u32,
) -> rusqlite::Result<()> {
    conn.execute(
        "DELETE FROM hosted_state_snapshots
         WHERE game_id = ?1
           AND seat_number = ?2
           AND rowid NOT IN (
               SELECT rowid
               FROM hosted_state_snapshots
               WHERE game_id = ?1 AND seat_number = ?2
               ORDER BY created_at DESC
               LIMIT ?3
           )",
        params![game_id, seat_number, SNAPSHOT_RETENTION_LIMIT],
    )?;
    Ok(())
}

fn json_err(err: serde_json::Error) -> rusqlite::Error {
    rusqlite::Error::ToSqlConversionFailure(Box::new(err))
}
