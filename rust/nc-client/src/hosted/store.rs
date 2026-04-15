use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use rusqlite::{Connection, OptionalExtension, params};

use crate::keychain::crypto::{decrypt_blob, encrypt_blob};
use crate::paths::data_root;
use nc_data::TurnSubmission;
use nc_nostr::state_sync::GameState;

const INIT_SQL: &str = r#"
CREATE TABLE IF NOT EXISTS hosted_player_snapshots (
    game_id TEXT NOT NULL,
    player_pubkey TEXT NOT NULL,
    seat INTEGER NOT NULL,
    turn INTEGER NOT NULL,
    state_hash TEXT NOT NULL,
    payload BLOB NOT NULL,
    updated_at INTEGER NOT NULL,
    PRIMARY KEY (game_id, player_pubkey)
);

CREATE TABLE IF NOT EXISTS hosted_order_drafts (
    game_id TEXT NOT NULL,
    player_pubkey TEXT NOT NULL,
    turn INTEGER NOT NULL,
    base_hash TEXT NOT NULL,
    status TEXT NOT NULL,
    submit_id TEXT,
    payload BLOB NOT NULL,
    updated_at INTEGER NOT NULL,
    PRIMARY KEY (game_id, player_pubkey)
);
"#;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HostedDraftStatus {
    Local,
    SubmittedPending,
    Conflict,
}

impl HostedDraftStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Local => "local",
            Self::SubmittedPending => "submitted_pending",
            Self::Conflict => "conflict",
        }
    }

    fn parse(value: &str) -> Result<Self, String> {
        match value {
            "local" => Ok(Self::Local),
            "submitted_pending" => Ok(Self::SubmittedPending),
            "conflict" => Ok(Self::Conflict),
            other => Err(format!("unknown hosted draft status: {other}")),
        }
    }
}

#[derive(Debug, Clone)]
pub struct CachedHostedSnapshot {
    pub game_id: String,
    pub player_pubkey: String,
    pub seat: u32,
    pub turn: u32,
    pub state_hash: String,
    pub snapshot: GameState,
}

#[derive(Debug, Clone)]
pub struct CachedHostedDraft {
    pub game_id: String,
    pub player_pubkey: String,
    pub turn: u32,
    pub base_hash: String,
    pub status: HostedDraftStatus,
    pub submit_id: Option<String>,
    pub draft: TurnSubmission,
}

pub struct HostedStateStore {
    conn: Connection,
}

impl HostedStateStore {
    pub fn open_default() -> Result<Self, Box<dyn std::error::Error>> {
        Self::open(&hosted_state_db_path())
    }

    pub fn open(path: &Path) -> Result<Self, Box<dyn std::error::Error>> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let conn = Connection::open(path)?;
        conn.execute_batch(INIT_SQL)?;
        Ok(Self { conn })
    }

    pub fn save_snapshot(
        &self,
        password: &str,
        player_pubkey: &str,
        snapshot: &GameState,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let payload = encrypt_json(snapshot, password)?;
        self.conn.execute(
            "INSERT INTO hosted_player_snapshots
             (game_id, player_pubkey, seat, turn, state_hash, payload, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
             ON CONFLICT(game_id, player_pubkey) DO UPDATE SET
                 seat = excluded.seat,
                 turn = excluded.turn,
                 state_hash = excluded.state_hash,
                 payload = excluded.payload,
                 updated_at = excluded.updated_at",
            params![
                snapshot.game_id,
                player_pubkey,
                snapshot.player_seat,
                snapshot.turn,
                snapshot.state_hash,
                payload,
                now_unix_seconds(),
            ],
        )?;
        Ok(())
    }

    pub fn load_snapshot(
        &self,
        password: &str,
        player_pubkey: &str,
        game_id: &str,
    ) -> Result<Option<CachedHostedSnapshot>, Box<dyn std::error::Error>> {
        let row = self
            .conn
            .query_row(
                "SELECT seat, turn, state_hash, payload
                 FROM hosted_player_snapshots
                 WHERE game_id = ?1 AND player_pubkey = ?2",
                params![game_id, player_pubkey],
                |row| {
                    Ok((
                        row.get::<_, u32>(0)?,
                        row.get::<_, u32>(1)?,
                        row.get::<_, String>(2)?,
                        row.get::<_, Vec<u8>>(3)?,
                    ))
                },
            )
            .optional()?;

        let Some((seat, turn, state_hash, payload)) = row else {
            return Ok(None);
        };
        let snapshot: GameState = decrypt_json(&payload, password)?;
        Ok(Some(CachedHostedSnapshot {
            game_id: game_id.to_string(),
            player_pubkey: player_pubkey.to_string(),
            seat,
            turn,
            state_hash,
            snapshot,
        }))
    }

    pub fn save_draft(
        &self,
        password: &str,
        player_pubkey: &str,
        game_id: &str,
        base_hash: &str,
        draft: &TurnSubmission,
        status: HostedDraftStatus,
        submit_id: Option<&str>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let payload = encrypt_string(&draft.to_kdl_string(), password)?;
        self.conn.execute(
            "INSERT INTO hosted_order_drafts
             (game_id, player_pubkey, turn, base_hash, status, submit_id, payload, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
             ON CONFLICT(game_id, player_pubkey) DO UPDATE SET
                 turn = excluded.turn,
                 base_hash = excluded.base_hash,
                 status = excluded.status,
                 submit_id = excluded.submit_id,
                 payload = excluded.payload,
                 updated_at = excluded.updated_at",
            params![
                game_id,
                player_pubkey,
                draft.year.saturating_sub(3000) as u32,
                base_hash,
                status.as_str(),
                submit_id,
                payload,
                now_unix_seconds(),
            ],
        )?;
        Ok(())
    }

    pub fn load_draft(
        &self,
        password: &str,
        player_pubkey: &str,
        game_id: &str,
    ) -> Result<Option<CachedHostedDraft>, Box<dyn std::error::Error>> {
        let row = self
            .conn
            .query_row(
                "SELECT turn, base_hash, status, submit_id, payload
                 FROM hosted_order_drafts
                 WHERE game_id = ?1 AND player_pubkey = ?2",
                params![game_id, player_pubkey],
                |row| {
                    Ok((
                        row.get::<_, u32>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, String>(2)?,
                        row.get::<_, Option<String>>(3)?,
                        row.get::<_, Vec<u8>>(4)?,
                    ))
                },
            )
            .optional()?;

        let Some((turn, base_hash, status, submit_id, payload)) = row else {
            return Ok(None);
        };
        let draft = TurnSubmission::parse_kdl_str(&decrypt_string(&payload, password)?)?;
        Ok(Some(CachedHostedDraft {
            game_id: game_id.to_string(),
            player_pubkey: player_pubkey.to_string(),
            turn,
            base_hash,
            status: HostedDraftStatus::parse(&status)?,
            submit_id,
            draft,
        }))
    }

    pub fn clear_draft(
        &self,
        player_pubkey: &str,
        game_id: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        self.conn.execute(
            "DELETE FROM hosted_order_drafts WHERE game_id = ?1 AND player_pubkey = ?2",
            params![game_id, player_pubkey],
        )?;
        Ok(())
    }
}

pub fn hosted_state_db_path() -> PathBuf {
    data_root().join("hosted-state.db")
}

fn now_unix_seconds() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|value| value.as_secs() as i64)
        .unwrap_or(0)
}

fn encrypt_json<T: serde::Serialize>(
    value: &T,
    password: &str,
) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    encrypt_string(&serde_json::to_string(value)?, password)
}

fn decrypt_json<T: serde::de::DeserializeOwned>(
    payload: &[u8],
    password: &str,
) -> Result<T, Box<dyn std::error::Error>> {
    Ok(serde_json::from_str(&decrypt_string(payload, password)?)?)
}

fn encrypt_string(
    value: &str,
    password: &str,
) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    Ok(encrypt_blob(value, password)?)
}

fn decrypt_string(
    payload: &[u8],
    password: &str,
) -> Result<String, Box<dyn std::error::Error>> {
    Ok(decrypt_blob(payload, password)?)
}
