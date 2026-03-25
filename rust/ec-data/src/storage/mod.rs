use std::path::{Path, PathBuf};

use rusqlite::Connection;

use crate::{CoreGameData, QueuedPlayerMail, ReportBlockRow};

mod intel;
mod mail;
mod metadata;
mod records;
mod report_blocks;
mod runtime;

pub const DEFAULT_CAMPAIGN_DB_NAME: &str = "ecgame.db";
const PLAYER_RECORD_FIELDS_TABLE: &str = "player_record_fields";
const PLANET_RECORD_FIELDS_TABLE: &str = "planet_record_fields";
const FLEET_RECORD_FIELDS_TABLE: &str = "fleet_record_fields";
const BASE_RECORD_FIELDS_TABLE: &str = "base_record_fields";
const IPBM_RECORD_FIELDS_TABLE: &str = "ipbm_record_fields";
const SETUP_RECORD_FIELDS_TABLE: &str = "setup_record_fields";
const CONQUEST_RECORD_FIELDS_TABLE: &str = "conquest_record_fields";

#[derive(Debug)]
pub enum CampaignStoreError {
    Io {
        path: PathBuf,
        source: std::io::Error,
    },
    Sql(rusqlite::Error),
    Parse(crate::ParseError),
    Directory(crate::GameDirectoryError),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum IntelTier {
    Owned,
    Full,
    Partial,
    Unknown,
}

impl IntelTier {
    /// Infer intel tier from a `PlayerStarmapWorld` as seen by `viewer_empire_id`.
    ///
    /// Owned  — the viewer owns this world.
    /// Full   — armies or ground batteries are known (Scout System level).
    /// Partial — name, owner, or potential production is known (View/ETAC level).
    /// Unknown — no intel at all.
    pub fn infer_from_world(viewer_empire_id: u8, world: &crate::PlayerStarmapWorld) -> IntelTier {
        Self::infer_from_fields(
            viewer_empire_id,
            world.known_owner_empire_id,
            world.known_name.as_deref(),
            world.known_potential_production,
            world.known_armies,
            world.known_ground_batteries,
        )
    }

    /// Infer intel tier from raw constituent fields.
    ///
    /// Use this when constructing a `PlayerStarmapWorld` before the struct is
    /// complete. Use `infer_from_world` when you already have the struct.
    pub fn infer_from_fields(
        viewer_empire_id: u8,
        known_owner_empire_id: Option<u8>,
        known_name: Option<&str>,
        known_potential_production: Option<u16>,
        known_armies: Option<u8>,
        known_ground_batteries: Option<u8>,
    ) -> IntelTier {
        if known_owner_empire_id == Some(viewer_empire_id) {
            IntelTier::Owned
        } else if known_armies.is_some() || known_ground_batteries.is_some() {
            IntelTier::Full
        } else if known_name.is_some()
            || known_owner_empire_id.is_some()
            || known_potential_production.is_some()
        {
            IntelTier::Partial
        } else {
            IntelTier::Unknown
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlanetIntelSnapshot {
    pub planet_record_index_1_based: usize,
    pub intel_tier: IntelTier,
    pub compat_is_orbit_seed: bool,
    pub last_intel_year: Option<u16>,
    pub seen_year: Option<u16>,
    pub scout_year: Option<u16>,
    pub known_name: Option<String>,
    pub known_owner_empire_id: Option<u8>,
    pub known_potential_production: Option<u16>,
    pub known_armies: Option<u8>,
    pub known_ground_batteries: Option<u8>,
    pub known_current_production: Option<u8>,
    pub known_stored_points: Option<u16>,
    pub known_docked_summary: Option<String>,
    pub known_orbit_summary: Option<String>,
    pub compat_word_1e: Option<u16>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CampaignStore {
    path: PathBuf,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CampaignRuntimeState {
    pub snapshot_id: i64,
    pub game_year: u16,
    pub campaign_seed: u64,
    pub game_data: CoreGameData,
    /// Structured report blocks. This is the authoritative runtime review
    /// state; callers should not rely on classic RESULTS.DAT byte payloads.
    pub report_block_rows: Vec<ReportBlockRow>,
    pub queued_mail: Vec<QueuedPlayerMail>,
}

impl std::fmt::Display for CampaignStoreError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io { path, source } => write!(f, "{}: {}", path.display(), source),
            Self::Sql(source) => write!(f, "{source}"),
            Self::Parse(source) => write!(f, "{source}"),
            Self::Directory(source) => write!(f, "{source}"),
        }
    }
}

impl std::error::Error for CampaignStoreError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io { source, .. } => Some(source),
            Self::Sql(source) => Some(source),
            Self::Parse(source) => Some(source),
            Self::Directory(source) => Some(source),
        }
    }
}

impl From<rusqlite::Error> for CampaignStoreError {
    fn from(value: rusqlite::Error) -> Self {
        Self::Sql(value)
    }
}

impl From<crate::ParseError> for CampaignStoreError {
    fn from(value: crate::ParseError) -> Self {
        Self::Parse(value)
    }
}

impl From<crate::GameDirectoryError> for CampaignStoreError {
    fn from(value: crate::GameDirectoryError) -> Self {
        Self::Directory(value)
    }
}

impl IntelTier {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Owned => "owned",
            Self::Full => "full",
            Self::Partial => "partial",
            Self::Unknown => "unknown",
        }
    }

    pub(super) fn from_str(value: &str) -> Self {
        match value {
            "owned" => Self::Owned,
            "full" => Self::Full,
            "partial" => Self::Partial,
            _ => Self::Unknown,
        }
    }
}

impl CampaignStore {
    pub fn open(path: impl AsRef<Path>) -> Result<Self, CampaignStoreError> {
        let store = Self {
            path: path.as_ref().to_path_buf(),
        };
        store.initialize()?;
        Ok(store)
    }

    pub fn open_default_in_dir(dir: &Path) -> Result<Self, CampaignStoreError> {
        Self::open(dir.join(DEFAULT_CAMPAIGN_DB_NAME))
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn has_snapshots(&self) -> Result<bool, CampaignStoreError> {
        let mut conn = self.connection()?;
        Ok(metadata::latest_snapshot_id_and_year(&mut conn)?.is_some())
    }

    pub fn latest_snapshot_metadata(&self) -> Result<Option<(i64, u16)>, CampaignStoreError> {
        let mut conn = self.connection()?;
        metadata::latest_snapshot_id_and_year(&mut conn)
    }

    fn initialize(&self) -> Result<(), CampaignStoreError> {
        let conn = self.connection()?;
        conn.execute_batch(
            "PRAGMA foreign_keys = ON;
             CREATE TABLE IF NOT EXISTS snapshots (
                 id INTEGER PRIMARY KEY,
                 game_year INTEGER NOT NULL UNIQUE
             );
             CREATE TABLE IF NOT EXISTS campaign_metadata (
                 key TEXT PRIMARY KEY,
                 int_value INTEGER NOT NULL
             );
             CREATE TABLE IF NOT EXISTS planet_intel (
                 snapshot_id INTEGER NOT NULL REFERENCES snapshots(id) ON DELETE CASCADE,
                 viewer_empire_id INTEGER NOT NULL,
                 planet_record_index INTEGER NOT NULL,
                 intel_tier TEXT NOT NULL,
                 compat_is_orbit_seed INTEGER NOT NULL DEFAULT 0,
                 last_intel_year INTEGER,
                 seen_year INTEGER,
                 scout_year INTEGER,
                 known_name TEXT,
                 known_owner_empire_id INTEGER,
                 known_potential_production INTEGER,
                 known_armies INTEGER,
                 known_ground_batteries INTEGER,
                 known_current_production INTEGER,
                 known_stored_points INTEGER,
                 known_docked_summary TEXT,
                 known_orbit_summary TEXT,
                 compat_word_1e INTEGER,
                 PRIMARY KEY(snapshot_id, viewer_empire_id, planet_record_index)
             );
             CREATE TABLE IF NOT EXISTS queued_mail (
                 snapshot_id INTEGER NOT NULL REFERENCES snapshots(id) ON DELETE CASCADE,
                 queue_index INTEGER NOT NULL,
                 sender_empire_id INTEGER NOT NULL,
                 recipient_empire_id INTEGER NOT NULL,
                 year INTEGER NOT NULL,
                 subject TEXT NOT NULL,
                 body TEXT NOT NULL,
                 recipient_deleted INTEGER NOT NULL DEFAULT 0,
                 PRIMARY KEY(snapshot_id, queue_index)
             );
             CREATE TABLE IF NOT EXISTS report_blocks (
                 snapshot_id INTEGER NOT NULL REFERENCES snapshots(id) ON DELETE CASCADE,
                 block_index INTEGER NOT NULL,
                 decoded_text TEXT NOT NULL,
                 raw_hex TEXT,
                 recipient_deleted INTEGER NOT NULL DEFAULT 0,
                 PRIMARY KEY(snapshot_id, block_index)
             );",
        )?;
        ensure_column(&conn, "planet_intel", "known_docked_summary", "TEXT")?;
        ensure_column(&conn, "planet_intel", "known_orbit_summary", "TEXT")?;
        create_typed_record_table(&conn, PLAYER_RECORD_FIELDS_TABLE)?;
        create_typed_record_table(&conn, PLANET_RECORD_FIELDS_TABLE)?;
        create_typed_record_table(&conn, FLEET_RECORD_FIELDS_TABLE)?;
        create_typed_record_table(&conn, BASE_RECORD_FIELDS_TABLE)?;
        create_typed_record_table(&conn, IPBM_RECORD_FIELDS_TABLE)?;
        create_typed_record_table(&conn, SETUP_RECORD_FIELDS_TABLE)?;
        create_typed_record_table(&conn, CONQUEST_RECORD_FIELDS_TABLE)?;
        Ok(())
    }

    fn connection(&self) -> Result<Connection, CampaignStoreError> {
        Connection::open(&self.path).map_err(CampaignStoreError::Sql)
    }
}

fn create_typed_record_table(conn: &Connection, table: &str) -> Result<(), CampaignStoreError> {
    let sql = format!(
        "CREATE TABLE IF NOT EXISTS {table} (
             snapshot_id INTEGER NOT NULL REFERENCES snapshots(id) ON DELETE CASCADE,
             record_index INTEGER NOT NULL,
             byte_offset INTEGER NOT NULL,
             byte_value INTEGER NOT NULL,
             PRIMARY KEY(snapshot_id, record_index, byte_offset)
         )"
    );
    conn.execute_batch(&sql)?;
    Ok(())
}

fn ensure_column(
    conn: &Connection,
    table: &str,
    column: &str,
    sql_type: &str,
) -> Result<(), CampaignStoreError> {
    let pragma = format!("PRAGMA table_info({table})");
    let mut stmt = conn.prepare(&pragma)?;
    let columns = stmt.query_map([], |row| row.get::<_, String>(1))?;
    let has_column = columns
        .collect::<Result<Vec<_>, _>>()?
        .into_iter()
        .any(|name| name == column);
    if !has_column {
        let sql = format!("ALTER TABLE {table} ADD COLUMN {column} {sql_type}");
        conn.execute(&sql, [])?;
    }
    Ok(())
}
