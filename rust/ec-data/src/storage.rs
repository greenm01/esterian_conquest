use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use rusqlite::{params, Connection, OptionalExtension};

use crate::{
    decode_report_block_rows, extract_player_intel_from_compat_database, generate_campaign_seed,
    merge_player_intel_from_runtime, rebuild_results_bytes, BaseDat, CoreGameData, DatabaseDat,
    FleetDat, IpbmDat, PlanetDat, PlayerDat, QueuedPlayerMail, ReportBlockRow, SetupDat,
    BASE_RECORD_SIZE, DATABASE_RECORD_SIZE, FLEET_RECORD_SIZE, IPBM_RECORD_SIZE,
    PLANET_RECORD_SIZE, PLAYER_RECORD_SIZE,
};

pub const DEFAULT_CAMPAIGN_DB_NAME: &str = "ecgame.db";
const PLAYER_RECORD_FIELDS_TABLE: &str = "player_record_fields";
const PLANET_RECORD_FIELDS_TABLE: &str = "planet_record_fields";
const FLEET_RECORD_FIELDS_TABLE: &str = "fleet_record_fields";
const BASE_RECORD_FIELDS_TABLE: &str = "base_record_fields";
const IPBM_RECORD_FIELDS_TABLE: &str = "ipbm_record_fields";
const SETUP_RECORD_FIELDS_TABLE: &str = "setup_record_fields";
const CONQUEST_RECORD_FIELDS_TABLE: &str = "conquest_record_fields";
const COMPAT_DATABASE_RECORD_FIELDS_TABLE: &str = "compat_database_record_fields";

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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlanetIntelSnapshot {
    pub planet_record_index_1_based: usize,
    pub intel_tier: IntelTier,
    pub last_intel_year: Option<u16>,
    pub known_name: Option<String>,
    pub known_owner_empire_id: Option<u8>,
    pub known_potential_production: Option<u16>,
    pub known_armies: Option<u8>,
    pub known_ground_batteries: Option<u8>,
    pub known_current_production: Option<u8>,
    pub known_stored_points: Option<u16>,
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

    fn from_str(value: &str) -> Self {
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

    pub fn import_directory_snapshot(&self, dir: &Path) -> Result<i64, CampaignStoreError> {
        self.import_directory_snapshot_with_seed(dir, None)
    }

    pub fn import_directory_snapshot_with_seed(
        &self,
        dir: &Path,
        campaign_seed: Option<u64>,
    ) -> Result<i64, CampaignStoreError> {
        let game_data = CoreGameData::load(dir)?;
        let database = load_database_snapshot_or_default(dir, &game_data)?;
        let results_bytes = read_optional_path(dir.join("RESULTS.DAT"))?;
        let queued_mail = load_mail_queue_file(dir)?;
        let report_block_rows = decode_report_block_rows(&results_bytes);
        let planet_intel_by_viewer = extract_player_intel_from_compat_database(
            &game_data,
            &database,
            game_data.conquest.game_year(),
        );
        self.save_runtime_state_internal(
            &game_data,
            &report_block_rows,
            &queued_mail,
            Some(&planet_intel_by_viewer),
            Some(&database),
            campaign_seed,
        )
    }

    pub fn export_latest_snapshot_to_dir(&self, dir: &Path) -> Result<u16, CampaignStoreError> {
        let mut conn = self.connection()?;
        let Some((snapshot_id, year)) = latest_snapshot_id_and_year(&mut conn)? else {
            return Ok(0);
        };
        self.export_snapshot_to_dir(snapshot_id, dir)?;
        Ok(year)
    }

    pub fn export_snapshot_to_dir(
        &self,
        snapshot_id: i64,
        dir: &Path,
    ) -> Result<(), CampaignStoreError> {
        fs::create_dir_all(dir).map_err(|source| CampaignStoreError::Io {
            path: dir.to_path_buf(),
            source,
        })?;
        let mut conn = self.connection()?;
        let game_data = load_snapshot_game_data(&mut conn, snapshot_id)?;
        game_data.save(dir)?;
        if let Some(database) = load_compat_database_rows(&mut conn, snapshot_id)? {
            write_path(dir.join("DATABASE.DAT"), &database.to_bytes())?;
        }
        // RESULTS.DAT: rebuild from report_blocks
        let report_rows = load_all_report_block_rows(&mut conn, snapshot_id)?;
        let active: Vec<_> = report_rows
            .iter()
            .filter(|r| !r.recipient_deleted)
            .cloned()
            .collect();
        let results_bytes = rebuild_results_bytes(&active).unwrap_or_default();
        write_path(dir.join("RESULTS.DAT"), &results_bytes)?;
        // MESSAGES.DAT: empty (messages live in queued_mail / runtime state)
        write_path(dir.join("MESSAGES.DAT"), &[])?;
        Ok(())
    }

    pub fn latest_planet_intel_for_viewer(
        &self,
        viewer_empire_id: u8,
    ) -> Result<Vec<PlanetIntelSnapshot>, CampaignStoreError> {
        let mut conn = self.connection()?;
        let Some((snapshot_id, _)) = latest_snapshot_id_and_year(&mut conn)? else {
            return Ok(Vec::new());
        };
        let mut stmt = conn.prepare(
            "SELECT planet_record_index, intel_tier, last_intel_year,
                    known_name, known_owner_empire_id, known_potential_production,
                    known_armies, known_ground_batteries,
                    known_current_production, known_stored_points
             FROM planet_intel
             WHERE snapshot_id = ?1 AND viewer_empire_id = ?2
             ORDER BY planet_record_index",
        )?;
        let rows = stmt
            .query_map(params![snapshot_id, i64::from(viewer_empire_id)], |row| {
                Ok(PlanetIntelSnapshot {
                    planet_record_index_1_based: row.get::<_, i64>(0)? as usize,
                    intel_tier: IntelTier::from_str(&row.get::<_, String>(1)?),
                    last_intel_year: row.get::<_, Option<i64>>(2)?.map(|value| value as u16),
                    known_name: row.get(3)?,
                    known_owner_empire_id: row.get::<_, Option<i64>>(4)?.map(|value| value as u8),
                    known_potential_production: row
                        .get::<_, Option<i64>>(5)?
                        .map(|value| value as u16),
                    known_armies: row.get::<_, Option<i64>>(6)?.map(|value| value as u8),
                    known_ground_batteries: row.get::<_, Option<i64>>(7)?.map(|value| value as u8),
                    known_current_production: row
                        .get::<_, Option<i64>>(8)?
                        .map(|value| value as u8),
                    known_stored_points: row.get::<_, Option<i64>>(9)?.map(|value| value as u16),
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(rows)
    }

    pub fn has_snapshots(&self) -> Result<bool, CampaignStoreError> {
        let mut conn = self.connection()?;
        Ok(latest_snapshot_id_and_year(&mut conn)?.is_some())
    }

    pub fn load_latest_runtime_state(
        &self,
    ) -> Result<Option<CampaignRuntimeState>, CampaignStoreError> {
        let mut conn = self.connection()?;
        let Some((snapshot_id, game_year)) = latest_snapshot_id_and_year(&mut conn)? else {
            return Ok(None);
        };
        let stored_campaign_seed = load_campaign_seed(&mut conn)?;
        let campaign_seed = stored_campaign_seed.unwrap_or_else(generate_campaign_seed);
        if stored_campaign_seed.is_none() {
            persist_campaign_seed(&mut conn, campaign_seed)?;
        }
        let game_data = load_snapshot_game_data(&mut conn, snapshot_id)?;
        let report_block_rows = load_report_block_rows(&mut conn, snapshot_id)?;
        let queued_mail = load_queued_mail_rows(&mut conn, snapshot_id)?;
        Ok(Some(CampaignRuntimeState {
            snapshot_id,
            game_year,
            campaign_seed,
            game_data,
            report_block_rows,
            queued_mail,
        }))
    }

    pub fn load_latest_compat_database(&self) -> Result<Option<DatabaseDat>, CampaignStoreError> {
        let mut conn = self.connection()?;
        let Some((snapshot_id, _)) = latest_snapshot_id_and_year(&mut conn)? else {
            return Ok(None);
        };
        load_compat_database_rows(&mut conn, snapshot_id)
    }

    pub fn save_runtime_state(
        &self,
        game_data: &CoreGameData,
        database: &DatabaseDat,
        results_bytes: &[u8],
        _messages_bytes: &[u8],
        queued_mail: &[QueuedPlayerMail],
    ) -> Result<i64, CampaignStoreError> {
        let report_block_rows = decode_report_block_rows(results_bytes);
        let planet_intel_by_viewer = extract_player_intel_from_compat_database(
            game_data,
            database,
            game_data.conquest.game_year(),
        );
        self.save_runtime_state_internal(
            game_data,
            &report_block_rows,
            queued_mail,
            Some(&planet_intel_by_viewer),
            Some(database),
            None,
        )
    }

    pub fn save_runtime_state_structured(
        &self,
        game_data: &CoreGameData,
        report_block_rows: &[ReportBlockRow],
        queued_mail: &[QueuedPlayerMail],
    ) -> Result<i64, CampaignStoreError> {
        self.save_runtime_state_internal(
            game_data,
            report_block_rows,
            queued_mail,
            None,
            None,
            None,
        )
    }

    pub fn save_runtime_state_structured_with_intel(
        &self,
        game_data: &CoreGameData,
        report_block_rows: &[ReportBlockRow],
        queued_mail: &[QueuedPlayerMail],
        planet_intel_by_viewer: &[BTreeMap<usize, PlanetIntelSnapshot>],
    ) -> Result<i64, CampaignStoreError> {
        self.save_runtime_state_internal(
            game_data,
            report_block_rows,
            queued_mail,
            Some(planet_intel_by_viewer),
            None,
            None,
        )
    }

    pub fn save_runtime_state_structured_with_intel_and_compat(
        &self,
        game_data: &CoreGameData,
        report_block_rows: &[ReportBlockRow],
        queued_mail: &[QueuedPlayerMail],
        planet_intel_by_viewer: &[BTreeMap<usize, PlanetIntelSnapshot>],
        database: &DatabaseDat,
    ) -> Result<i64, CampaignStoreError> {
        self.save_runtime_state_internal(
            game_data,
            report_block_rows,
            queued_mail,
            Some(planet_intel_by_viewer),
            Some(database),
            None,
        )
    }

    pub fn save_runtime_state_structured_with_compat(
        &self,
        game_data: &CoreGameData,
        report_block_rows: &[ReportBlockRow],
        queued_mail: &[QueuedPlayerMail],
        database: &DatabaseDat,
    ) -> Result<i64, CampaignStoreError> {
        self.save_runtime_state_internal(
            game_data,
            report_block_rows,
            queued_mail,
            None,
            Some(database),
            None,
        )
    }

    fn save_runtime_state_internal(
        &self,
        game_data: &CoreGameData,
        report_block_rows: &[ReportBlockRow],
        queued_mail: &[QueuedPlayerMail],
        planet_intel_by_viewer_override: Option<&[BTreeMap<usize, PlanetIntelSnapshot>]>,
        compat_database_override: Option<&DatabaseDat>,
        campaign_seed_override: Option<u64>,
    ) -> Result<i64, CampaignStoreError> {
        let year = game_data.conquest.game_year();
        let mut conn = self.connection()?;
        let tx = conn.transaction()?;
        let campaign_seed = load_campaign_seed_tx(&tx)?
            .or(campaign_seed_override)
            .unwrap_or_else(generate_campaign_seed);
        persist_campaign_seed_tx(&tx, campaign_seed)?;
        let previous_snapshot_id = tx
            .query_row(
                "SELECT id FROM snapshots ORDER BY game_year DESC LIMIT 1",
                [],
                |row| row.get::<_, i64>(0),
            )
            .optional()?;
        let previous_intel = if let Some(previous_snapshot_id) = previous_snapshot_id {
            load_intel_rows(&tx, previous_snapshot_id)?
        } else {
            BTreeMap::new()
        };
        let previous_compat_database = if compat_database_override.is_none() {
            previous_snapshot_id
                .map(|snapshot_id| load_compat_database_rows_tx(&tx, snapshot_id))
                .transpose()?
                .flatten()
        } else {
            None
        };
        tx.execute(
            "DELETE FROM snapshots WHERE game_year = ?1",
            params![i64::from(year)],
        )?;
        tx.execute(
            "INSERT INTO snapshots(game_year) VALUES (?1)",
            params![i64::from(year)],
        )?;
        let snapshot_id = tx.last_insert_rowid();
        write_typed_record_rows(
            &tx,
            PLAYER_RECORD_FIELDS_TABLE,
            snapshot_id,
            &game_data.player.to_bytes(),
            PLAYER_RECORD_SIZE,
        )?;
        write_typed_record_rows(
            &tx,
            PLANET_RECORD_FIELDS_TABLE,
            snapshot_id,
            &game_data.planets.to_bytes(),
            PLANET_RECORD_SIZE,
        )?;
        write_typed_record_rows(
            &tx,
            FLEET_RECORD_FIELDS_TABLE,
            snapshot_id,
            &game_data.fleets.to_bytes(),
            FLEET_RECORD_SIZE,
        )?;
        write_typed_record_rows(
            &tx,
            BASE_RECORD_FIELDS_TABLE,
            snapshot_id,
            &game_data.bases.to_bytes(),
            BASE_RECORD_SIZE,
        )?;
        write_typed_record_rows(
            &tx,
            IPBM_RECORD_FIELDS_TABLE,
            snapshot_id,
            &game_data.ipbm.to_bytes(),
            IPBM_RECORD_SIZE,
        )?;
        write_typed_record_rows(
            &tx,
            SETUP_RECORD_FIELDS_TABLE,
            snapshot_id,
            &game_data.setup.to_bytes(),
            crate::SETUP_DAT_SIZE,
        )?;
        write_typed_record_rows(
            &tx,
            CONQUEST_RECORD_FIELDS_TABLE,
            snapshot_id,
            &game_data.conquest.to_bytes(),
            crate::CONQUEST_DAT_SIZE,
        )?;
        if let Some(database) = compat_database_override.or(previous_compat_database.as_ref()) {
            write_typed_record_rows(
                &tx,
                COMPAT_DATABASE_RECORD_FIELDS_TABLE,
                snapshot_id,
                &database.to_bytes(),
                DATABASE_RECORD_SIZE,
            )?;
        }
        write_report_block_rows(&tx, snapshot_id, report_block_rows)?;
        write_queued_mail_rows(&tx, snapshot_id, queued_mail)?;

        write_planet_intel_rows(
            &tx,
            snapshot_id,
            game_data,
            year,
            planet_intel_by_viewer_override,
            &previous_intel,
        )?;
        tx.commit()?;
        Ok(snapshot_id)
    }

    fn initialize(&self) -> Result<(), CampaignStoreError> {
        let mut conn = self.connection()?;
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
                 last_intel_year INTEGER,
                 known_name TEXT,
                 known_owner_empire_id INTEGER,
                 known_potential_production INTEGER,
                 known_armies INTEGER,
                 known_ground_batteries INTEGER,
                 known_current_production INTEGER,
                 known_stored_points INTEGER,
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
        create_typed_record_table(&conn, PLAYER_RECORD_FIELDS_TABLE)?;
        create_typed_record_table(&conn, PLANET_RECORD_FIELDS_TABLE)?;
        create_typed_record_table(&conn, FLEET_RECORD_FIELDS_TABLE)?;
        create_typed_record_table(&conn, BASE_RECORD_FIELDS_TABLE)?;
        create_typed_record_table(&conn, IPBM_RECORD_FIELDS_TABLE)?;
        create_typed_record_table(&conn, SETUP_RECORD_FIELDS_TABLE)?;
        create_typed_record_table(&conn, CONQUEST_RECORD_FIELDS_TABLE)?;
        create_typed_record_table(&conn, COMPAT_DATABASE_RECORD_FIELDS_TABLE)?;
        migrate_legacy_report_blocks_table(&mut conn)?;
        ensure_queued_mail_recipient_deleted_column(&conn)?;
        ensure_planet_intel_production_columns(&conn)?;
        migrate_legacy_blob_record_tables(&mut conn)?;
        migrate_legacy_compat_files(&mut conn)?;
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

fn migrate_legacy_blob_record_tables(conn: &mut Connection) -> Result<(), CampaignStoreError> {
    migrate_legacy_record_table(
        conn,
        "player_records",
        PLAYER_RECORD_FIELDS_TABLE,
        PLAYER_RECORD_SIZE,
        true,
        "player_records",
    )?;
    migrate_legacy_record_table(
        conn,
        "planet_records",
        PLANET_RECORD_FIELDS_TABLE,
        PLANET_RECORD_SIZE,
        true,
        "planet_records",
    )?;
    migrate_legacy_record_table(
        conn,
        "fleet_records",
        FLEET_RECORD_FIELDS_TABLE,
        FLEET_RECORD_SIZE,
        true,
        "fleet_records",
    )?;
    migrate_legacy_record_table(
        conn,
        "base_records",
        BASE_RECORD_FIELDS_TABLE,
        BASE_RECORD_SIZE,
        true,
        "base_records",
    )?;
    migrate_legacy_record_table(
        conn,
        "ipbm_records",
        IPBM_RECORD_FIELDS_TABLE,
        IPBM_RECORD_SIZE,
        true,
        "ipbm_records",
    )?;
    migrate_legacy_record_table(
        conn,
        "setup_records",
        SETUP_RECORD_FIELDS_TABLE,
        crate::SETUP_DAT_SIZE,
        false,
        "SETUP.DAT",
    )?;
    migrate_legacy_record_table(
        conn,
        "conquest_records",
        CONQUEST_RECORD_FIELDS_TABLE,
        crate::CONQUEST_DAT_SIZE,
        false,
        "CONQUEST.DAT",
    )?;
    Ok(())
}

fn migrate_legacy_record_table(
    conn: &mut Connection,
    old_table: &str,
    new_table: &str,
    record_size: usize,
    has_record_index: bool,
    file_type: &'static str,
) -> Result<(), CampaignStoreError> {
    if !table_exists(conn, old_table)? {
        return Ok(());
    }

    let new_count: i64 =
        conn.query_row(&format!("SELECT COUNT(*) FROM {new_table}"), [], |row| {
            row.get(0)
        })?;
    if new_count == 0 {
        let tx = conn.transaction()?;
        let insert_sql = format!(
            "INSERT INTO {new_table}(snapshot_id, record_index, byte_offset, byte_value)
             VALUES (?1, ?2, ?3, ?4)"
        );
        let select_sql = if has_record_index {
            format!("SELECT snapshot_id, record_index, raw FROM {old_table} ORDER BY snapshot_id, record_index")
        } else {
            format!("SELECT snapshot_id, raw FROM {old_table} ORDER BY snapshot_id")
        };
        {
            let mut select = tx.prepare(&select_sql)?;
            let mut insert = tx.prepare(&insert_sql)?;
            if has_record_index {
                let rows = select.query_map([], |row| {
                    Ok((
                        row.get::<_, i64>(0)?,
                        row.get::<_, i64>(1)?,
                        row.get::<_, Vec<u8>>(2)?,
                    ))
                })?;
                for row in rows {
                    let (snapshot_id, record_index, raw) = row?;
                    if raw.len() != record_size {
                        return Err(CampaignStoreError::Parse(
                            crate::ParseError::WrongRecordMultiple {
                                file_type,
                                record_size,
                                actual: raw.len(),
                            },
                        ));
                    }
                    for (byte_offset, byte_value) in raw.iter().copied().enumerate() {
                        insert.execute(params![
                            snapshot_id,
                            record_index,
                            byte_offset as i64,
                            i64::from(byte_value),
                        ])?;
                    }
                }
            } else {
                let rows = select.query_map([], |row| {
                    Ok((row.get::<_, i64>(0)?, row.get::<_, Vec<u8>>(1)?))
                })?;
                for row in rows {
                    let (snapshot_id, raw) = row?;
                    if raw.len() != record_size {
                        return Err(CampaignStoreError::Parse(
                            crate::ParseError::WrongRecordMultiple {
                                file_type,
                                record_size,
                                actual: raw.len(),
                            },
                        ));
                    }
                    for (byte_offset, byte_value) in raw.iter().copied().enumerate() {
                        insert.execute(params![
                            snapshot_id,
                            1_i64,
                            byte_offset as i64,
                            i64::from(byte_value),
                        ])?;
                    }
                }
            }
        }
        tx.commit()?;
    }

    conn.execute_batch(&format!("DROP TABLE IF EXISTS {old_table};"))?;
    Ok(())
}

fn table_exists(conn: &Connection, table: &str) -> Result<bool, CampaignStoreError> {
    conn.query_row(
        "SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = ?1",
        params![table],
        |_| Ok(()),
    )
    .optional()
    .map(|row| row.is_some())
    .map_err(CampaignStoreError::Sql)
}

fn table_columns(conn: &Connection, table: &str) -> Result<Vec<String>, CampaignStoreError> {
    let mut stmt = conn.prepare(&format!("PRAGMA table_info({table})"))?;
    stmt.query_map([], |row| row.get::<_, String>(1))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(CampaignStoreError::Sql)
}

fn migrate_legacy_report_blocks_table(conn: &mut Connection) -> Result<(), CampaignStoreError> {
    if !table_exists(conn, "report_blocks")? {
        return Ok(());
    }
    let columns = table_columns(conn, "report_blocks")?;
    if !columns.iter().any(|column| column == "raw_bytes") {
        return Ok(());
    }
    conn.execute_batch(
        "BEGIN;
         CREATE TABLE report_blocks_v2 (
             snapshot_id INTEGER NOT NULL REFERENCES snapshots(id) ON DELETE CASCADE,
             block_index INTEGER NOT NULL,
             decoded_text TEXT NOT NULL,
             raw_hex TEXT,
             recipient_deleted INTEGER NOT NULL DEFAULT 0,
             PRIMARY KEY(snapshot_id, block_index)
         );
         INSERT INTO report_blocks_v2(snapshot_id, block_index, decoded_text, raw_hex, recipient_deleted)
         SELECT
             snapshot_id,
             block_index,
             decoded_text,
             CASE WHEN raw_bytes IS NULL THEN NULL ELSE hex(raw_bytes) END,
             COALESCE(recipient_deleted, 0)
         FROM report_blocks;
         DROP TABLE report_blocks;
         ALTER TABLE report_blocks_v2 RENAME TO report_blocks;
         COMMIT;",
    )?;
    Ok(())
}

fn migrate_legacy_compat_files(conn: &mut Connection) -> Result<(), CampaignStoreError> {
    if !table_exists(conn, "compat_files")? {
        return Ok(());
    }

    let tx = conn.transaction()?;
    {
        let mut stmt = tx.prepare(
            "SELECT snapshot_id, name, bytes
             FROM compat_files
             ORDER BY snapshot_id, name",
        )?;
        let rows = stmt.query_map([], |row| {
            Ok((
                row.get::<_, i64>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, Vec<u8>>(2)?,
            ))
        })?;
        for row in rows {
            let (snapshot_id, name, bytes) = row?;
            match name.as_str() {
                "DATABASE.DAT" => {
                    let existing_count: i64 = tx.query_row(
                        &format!(
                            "SELECT COUNT(*) FROM {COMPAT_DATABASE_RECORD_FIELDS_TABLE}
                             WHERE snapshot_id = ?1"
                        ),
                        params![snapshot_id],
                        |row| row.get(0),
                    )?;
                    if existing_count == 0 {
                        write_typed_record_rows(
                            &tx,
                            COMPAT_DATABASE_RECORD_FIELDS_TABLE,
                            snapshot_id,
                            &bytes,
                            DATABASE_RECORD_SIZE,
                        )?;
                    }
                }
                "RESULTS.DAT" => {
                    let existing_count: i64 = tx.query_row(
                        "SELECT COUNT(*) FROM report_blocks WHERE snapshot_id = ?1",
                        params![snapshot_id],
                        |row| row.get(0),
                    )?;
                    if existing_count == 0 {
                        let rows = decode_report_block_rows(&bytes);
                        write_report_block_rows(&tx, snapshot_id, &rows)?;
                    }
                }
                _ => {}
            }
        }
    }
    tx.execute_batch("DROP TABLE compat_files;")?;
    tx.commit()?;
    Ok(())
}

fn latest_snapshot_id_and_year(
    conn: &mut Connection,
) -> Result<Option<(i64, u16)>, CampaignStoreError> {
    conn.query_row(
        "SELECT id, game_year FROM snapshots ORDER BY game_year DESC LIMIT 1",
        [],
        |row| Ok((row.get::<_, i64>(0)?, row.get::<_, i64>(1)? as u16)),
    )
    .optional()
    .map_err(CampaignStoreError::Sql)
}

fn load_campaign_seed(conn: &mut Connection) -> Result<Option<u64>, CampaignStoreError> {
    conn.query_row(
        "SELECT int_value FROM campaign_metadata WHERE key = 'campaign_seed'",
        [],
        |row| row.get::<_, i64>(0),
    )
    .optional()
    .map(|value| value.map(|seed| seed as u64))
    .map_err(CampaignStoreError::Sql)
}

fn load_campaign_seed_tx(
    tx: &rusqlite::Transaction<'_>,
) -> Result<Option<u64>, CampaignStoreError> {
    tx.query_row(
        "SELECT int_value FROM campaign_metadata WHERE key = 'campaign_seed'",
        [],
        |row| row.get::<_, i64>(0),
    )
    .optional()
    .map(|value| value.map(|seed| seed as u64))
    .map_err(CampaignStoreError::Sql)
}

fn persist_campaign_seed(conn: &mut Connection, seed: u64) -> Result<(), CampaignStoreError> {
    conn.execute(
        "INSERT INTO campaign_metadata(key, int_value)
         VALUES ('campaign_seed', ?1)
         ON CONFLICT(key) DO UPDATE SET int_value = excluded.int_value",
        params![seed as i64],
    )?;
    Ok(())
}

fn persist_campaign_seed_tx(
    tx: &rusqlite::Transaction<'_>,
    seed: u64,
) -> Result<(), CampaignStoreError> {
    tx.execute(
        "INSERT INTO campaign_metadata(key, int_value)
         VALUES ('campaign_seed', ?1)
         ON CONFLICT(key) DO UPDATE SET int_value = excluded.int_value",
        params![seed as i64],
    )?;
    Ok(())
}

fn write_typed_record_rows(
    tx: &rusqlite::Transaction<'_>,
    table: &str,
    snapshot_id: i64,
    bytes: &[u8],
    record_size: usize,
) -> Result<(), CampaignStoreError> {
    let sql = format!(
        "INSERT INTO {table}(snapshot_id, record_index, byte_offset, byte_value)
         VALUES (?1, ?2, ?3, ?4)"
    );
    let mut stmt = tx.prepare(&sql)?;
    for (idx, chunk) in bytes.chunks_exact(record_size).enumerate() {
        let record_index = (idx + 1) as i64;
        for (byte_offset, byte_value) in chunk.iter().copied().enumerate() {
            stmt.execute(params![
                snapshot_id,
                record_index,
                byte_offset as i64,
                i64::from(byte_value),
            ])?;
        }
    }
    Ok(())
}

fn read_typed_record_rows(
    conn: &mut Connection,
    table: &str,
    snapshot_id: i64,
    expected_size: usize,
) -> Result<Vec<u8>, CampaignStoreError> {
    let sql = format!(
        "SELECT record_index, byte_offset, byte_value
         FROM {table}
         WHERE snapshot_id = ?1
         ORDER BY record_index, byte_offset"
    );
    let mut stmt = conn.prepare(&sql)?;
    let rows = stmt.query_map(params![snapshot_id], |row| {
        Ok((
            row.get::<_, i64>(0)?,
            row.get::<_, i64>(1)?,
            row.get::<_, i64>(2)?,
        ))
    })?;
    let mut bytes = Vec::new();
    let mut current_record_index: Option<i64> = None;
    let mut expected_offset = 0usize;
    for row in rows {
        let (record_index, byte_offset, byte_value) = row?;
        let byte_offset = byte_offset as usize;
        if current_record_index != Some(record_index) {
            if current_record_index.is_some() && expected_offset != expected_size {
                return Err(CampaignStoreError::Parse(
                    crate::ParseError::WrongRecordMultiple {
                        file_type: "sqlite-record",
                        record_size: expected_size,
                        actual: expected_offset,
                    },
                ));
            }
            current_record_index = Some(record_index);
            expected_offset = 0;
        }
        if byte_offset != expected_offset {
            return Err(CampaignStoreError::Parse(
                crate::ParseError::WrongRecordMultiple {
                    file_type: "sqlite-record",
                    record_size: expected_size,
                    actual: byte_offset,
                },
            ));
        }
        bytes.push(byte_value as u8);
        expected_offset += 1;
    }
    if current_record_index.is_some() && expected_offset != expected_size {
        return Err(CampaignStoreError::Parse(
            crate::ParseError::WrongRecordMultiple {
                file_type: "sqlite-record",
                record_size: expected_size,
                actual: expected_offset,
            },
        ));
    }
    Ok(bytes)
}

fn load_snapshot_game_data(
    conn: &mut Connection,
    snapshot_id: i64,
) -> Result<CoreGameData, CampaignStoreError> {
    Ok(CoreGameData {
        player: PlayerDat::parse(&read_typed_record_rows(
            conn,
            PLAYER_RECORD_FIELDS_TABLE,
            snapshot_id,
            PLAYER_RECORD_SIZE,
        )?)?,
        planets: PlanetDat::parse(&read_typed_record_rows(
            conn,
            PLANET_RECORD_FIELDS_TABLE,
            snapshot_id,
            PLANET_RECORD_SIZE,
        )?)?,
        fleets: FleetDat::parse(&read_typed_record_rows(
            conn,
            FLEET_RECORD_FIELDS_TABLE,
            snapshot_id,
            FLEET_RECORD_SIZE,
        )?)?,
        bases: BaseDat::parse(&read_typed_record_rows(
            conn,
            BASE_RECORD_FIELDS_TABLE,
            snapshot_id,
            BASE_RECORD_SIZE,
        )?)?,
        ipbm: IpbmDat::parse(&read_typed_record_rows(
            conn,
            IPBM_RECORD_FIELDS_TABLE,
            snapshot_id,
            IPBM_RECORD_SIZE,
        )?)?,
        setup: SetupDat::parse(&read_typed_record_rows(
            conn,
            SETUP_RECORD_FIELDS_TABLE,
            snapshot_id,
            crate::SETUP_DAT_SIZE,
        )?)?,
        conquest: crate::ConquestDat::parse(&read_typed_record_rows(
            conn,
            CONQUEST_RECORD_FIELDS_TABLE,
            snapshot_id,
            crate::CONQUEST_DAT_SIZE,
        )?)?,
    })
}

fn load_compat_database_rows(
    conn: &mut Connection,
    snapshot_id: i64,
) -> Result<Option<DatabaseDat>, CampaignStoreError> {
    let row_count: i64 = conn.query_row(
        &format!(
            "SELECT COUNT(*) FROM {COMPAT_DATABASE_RECORD_FIELDS_TABLE}
             WHERE snapshot_id = ?1"
        ),
        params![snapshot_id],
        |row| row.get(0),
    )?;
    if row_count == 0 {
        return Ok(None);
    }
    let bytes = read_typed_record_rows(
        conn,
        COMPAT_DATABASE_RECORD_FIELDS_TABLE,
        snapshot_id,
        DATABASE_RECORD_SIZE,
    )?;
    DatabaseDat::parse(&bytes).map(Some).map_err(Into::into)
}

fn load_compat_database_rows_tx(
    tx: &rusqlite::Transaction<'_>,
    snapshot_id: i64,
) -> Result<Option<DatabaseDat>, CampaignStoreError> {
    let row_count: i64 = tx.query_row(
        &format!(
            "SELECT COUNT(*) FROM {COMPAT_DATABASE_RECORD_FIELDS_TABLE}
             WHERE snapshot_id = ?1"
        ),
        params![snapshot_id],
        |row| row.get(0),
    )?;
    if row_count == 0 {
        return Ok(None);
    }
    let mut stmt = tx.prepare(&format!(
        "SELECT record_index, byte_offset, byte_value
         FROM {COMPAT_DATABASE_RECORD_FIELDS_TABLE}
         WHERE snapshot_id = ?1
         ORDER BY record_index, byte_offset"
    ))?;
    let rows = stmt.query_map(params![snapshot_id], |row| {
        Ok((
            row.get::<_, i64>(0)?,
            row.get::<_, i64>(1)?,
            row.get::<_, i64>(2)?,
        ))
    })?;
    let mut bytes = Vec::new();
    let mut current_record_index: Option<i64> = None;
    let mut expected_offset = 0usize;
    for row in rows {
        let (record_index, byte_offset, byte_value) = row?;
        let byte_offset = byte_offset as usize;
        if current_record_index != Some(record_index) {
            if current_record_index.is_some() && expected_offset != DATABASE_RECORD_SIZE {
                return Err(CampaignStoreError::Parse(
                    crate::ParseError::WrongRecordMultiple {
                        file_type: "DATABASE.DAT",
                        record_size: DATABASE_RECORD_SIZE,
                        actual: expected_offset,
                    },
                ));
            }
            current_record_index = Some(record_index);
            expected_offset = 0;
        }
        if byte_offset != expected_offset {
            return Err(CampaignStoreError::Parse(
                crate::ParseError::WrongRecordMultiple {
                    file_type: "DATABASE.DAT",
                    record_size: DATABASE_RECORD_SIZE,
                    actual: byte_offset,
                },
            ));
        }
        bytes.push(byte_value as u8);
        expected_offset += 1;
    }
    if current_record_index.is_some() && expected_offset != DATABASE_RECORD_SIZE {
        return Err(CampaignStoreError::Parse(
            crate::ParseError::WrongRecordMultiple {
                file_type: "DATABASE.DAT",
                record_size: DATABASE_RECORD_SIZE,
                actual: expected_offset,
            },
        ));
    }
    DatabaseDat::parse(&bytes).map(Some).map_err(Into::into)
}

fn load_intel_rows(
    tx: &rusqlite::Transaction<'_>,
    snapshot_id: i64,
) -> Result<BTreeMap<(u8, usize), PlanetIntelSnapshot>, CampaignStoreError> {
    let mut stmt = tx.prepare(
        "SELECT viewer_empire_id, planet_record_index, intel_tier, last_intel_year,
                known_name, known_owner_empire_id, known_potential_production,
                known_armies, known_ground_batteries,
                known_current_production, known_stored_points
         FROM planet_intel
         WHERE snapshot_id = ?1",
    )?;
    let rows = stmt.query_map(params![snapshot_id], |row| {
        Ok((
            (row.get::<_, i64>(0)? as u8, row.get::<_, i64>(1)? as usize),
            PlanetIntelSnapshot {
                planet_record_index_1_based: row.get::<_, i64>(1)? as usize,
                intel_tier: IntelTier::from_str(&row.get::<_, String>(2)?),
                last_intel_year: row.get::<_, Option<i64>>(3)?.map(|value| value as u16),
                known_name: row.get(4)?,
                known_owner_empire_id: row.get::<_, Option<i64>>(5)?.map(|value| value as u8),
                known_potential_production: row.get::<_, Option<i64>>(6)?.map(|value| value as u16),
                known_armies: row.get::<_, Option<i64>>(7)?.map(|value| value as u8),
                known_ground_batteries: row.get::<_, Option<i64>>(8)?.map(|value| value as u8),
                known_current_production: row.get::<_, Option<i64>>(9)?.map(|value| value as u8),
                known_stored_points: row.get::<_, Option<i64>>(10)?.map(|value| value as u16),
            },
        ))
    })?;
    Ok(rows.collect::<Result<BTreeMap<_, _>, _>>()?)
}

fn write_planet_intel_rows(
    tx: &rusqlite::Transaction<'_>,
    snapshot_id: i64,
    game_data: &CoreGameData,
    year: u16,
    current_rows_override: Option<&[BTreeMap<usize, PlanetIntelSnapshot>]>,
    previous: &BTreeMap<(u8, usize), PlanetIntelSnapshot>,
) -> Result<(), CampaignStoreError> {
    let player_count = game_data.conquest.player_count();
    let mut stmt = tx.prepare(
        "INSERT INTO planet_intel(
             snapshot_id, viewer_empire_id, planet_record_index, intel_tier, last_intel_year,
             known_name, known_owner_empire_id, known_potential_production,
             known_armies, known_ground_batteries,
             known_current_production, known_stored_points
         ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
    )?;
    for viewer_empire_id in 1..=player_count {
        let previous_rows = previous
            .iter()
            .filter_map(|((empire_id, planet_record_index), snapshot)| {
                (*empire_id == viewer_empire_id).then_some((*planet_record_index, snapshot.clone()))
            })
            .collect::<BTreeMap<_, _>>();
        let merged_rows = current_rows_override
            .and_then(|rows| rows.get(viewer_empire_id.saturating_sub(1) as usize))
            .cloned()
            .unwrap_or_else(|| {
                merge_player_intel_from_runtime(
                    game_data,
                    viewer_empire_id,
                    year,
                    Some(&previous_rows),
                    None,
                )
            });
        for (planet_record_index_1_based, snapshot) in merged_rows {
            stmt.execute(params![
                snapshot_id,
                i64::from(viewer_empire_id),
                planet_record_index_1_based as i64,
                snapshot.intel_tier.as_str(),
                snapshot.last_intel_year.map(i64::from),
                snapshot.known_name,
                snapshot.known_owner_empire_id.map(i64::from),
                snapshot.known_potential_production.map(i64::from),
                snapshot.known_armies.map(i64::from),
                snapshot.known_ground_batteries.map(i64::from),
                snapshot.known_current_production.map(i64::from),
                snapshot.known_stored_points.map(i64::from),
            ])?;
        }
    }
    Ok(())
}

fn read_optional_path(path: PathBuf) -> Result<Vec<u8>, CampaignStoreError> {
    match fs::read(&path) {
        Ok(bytes) => Ok(bytes),
        Err(source) if source.kind() == std::io::ErrorKind::NotFound => Ok(Vec::new()),
        Err(source) => Err(CampaignStoreError::Io { path, source }),
    }
}

fn write_path(path: PathBuf, bytes: &[u8]) -> Result<(), CampaignStoreError> {
    fs::write(&path, bytes).map_err(|source| CampaignStoreError::Io { path, source })
}

fn encode_hex(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789ABCDEF";
    let mut out = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        out.push(HEX[(byte >> 4) as usize] as char);
        out.push(HEX[(byte & 0x0F) as usize] as char);
    }
    out
}

fn decode_hex(text: &str) -> Result<Vec<u8>, String> {
    if text.len() % 2 != 0 {
        return Err("hex text length must be even".to_string());
    }
    let mut out = Vec::with_capacity(text.len() / 2);
    let bytes = text.as_bytes();
    for idx in (0..bytes.len()).step_by(2) {
        let hi = decode_hex_nibble(bytes[idx])
            .ok_or_else(|| format!("invalid hex character at offset {idx}"))?;
        let lo = decode_hex_nibble(bytes[idx + 1])
            .ok_or_else(|| format!("invalid hex character at offset {}", idx + 1))?;
        out.push((hi << 4) | lo);
    }
    Ok(out)
}

fn decode_hex_nibble(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        b'A'..=b'F' => Some(byte - b'A' + 10),
        _ => None,
    }
}

fn load_mail_queue_file(dir: &Path) -> Result<Vec<QueuedPlayerMail>, CampaignStoreError> {
    let path = crate::player_mail::queue_path(dir);
    if !path.exists() {
        return Ok(Vec::new());
    }
    crate::load_mail_queue(dir).map_err(|err| CampaignStoreError::Io {
        path,
        source: std::io::Error::other(err.to_string()),
    })
}

fn load_database_snapshot_or_default(
    dir: &Path,
    game_data: &CoreGameData,
) -> Result<DatabaseDat, CampaignStoreError> {
    let path = dir.join("DATABASE.DAT");
    match fs::read(&path) {
        Ok(bytes) => Ok(DatabaseDat::parse(&bytes)?),
        Err(source) if source.kind() == std::io::ErrorKind::NotFound => {
            let planet_names = game_data
                .planets
                .records
                .iter()
                .map(|planet| {
                    let name = planet.planet_name();
                    if name.eq_ignore_ascii_case("unowned")
                        || name.eq_ignore_ascii_case("not named yet")
                    {
                        "UNKNOWN".to_string()
                    } else {
                        name
                    }
                })
                .collect::<Vec<_>>();
            Ok(DatabaseDat::generate_from_planets_and_year(
                &planet_names,
                game_data.conquest.game_year(),
                game_data.conquest.player_count() as usize,
                None,
            ))
        }
        Err(source) => Err(CampaignStoreError::Io { path, source }),
    }
}

fn write_queued_mail_rows(
    tx: &rusqlite::Transaction<'_>,
    snapshot_id: i64,
    queued_mail: &[QueuedPlayerMail],
) -> Result<(), CampaignStoreError> {
    let mut stmt = tx.prepare(
        "INSERT INTO queued_mail(
             snapshot_id, queue_index, sender_empire_id, recipient_empire_id, year, subject, body,
             recipient_deleted
         ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
    )?;
    for (idx, mail) in queued_mail.iter().enumerate() {
        stmt.execute(params![
            snapshot_id,
            idx as i64,
            i64::from(mail.sender_empire_id),
            i64::from(mail.recipient_empire_id),
            i64::from(mail.year),
            &mail.subject,
            &mail.body,
            i64::from(u8::from(mail.recipient_deleted)),
        ])?;
    }
    Ok(())
}

fn load_queued_mail_rows(
    conn: &mut Connection,
    snapshot_id: i64,
) -> Result<Vec<QueuedPlayerMail>, CampaignStoreError> {
    let mut stmt = conn.prepare(
        "SELECT sender_empire_id, recipient_empire_id, year, subject, body, recipient_deleted
         FROM queued_mail
         WHERE snapshot_id = ?1
         ORDER BY queue_index",
    )?;
    let rows = stmt.query_map(params![snapshot_id], |row| {
        Ok(QueuedPlayerMail {
            sender_empire_id: row.get::<_, i64>(0)? as u8,
            recipient_empire_id: row.get::<_, i64>(1)? as u8,
            year: row.get::<_, i64>(2)? as u16,
            subject: row.get(3)?,
            body: row.get(4)?,
            recipient_deleted: row.get::<_, i64>(5)? != 0,
        })
    })?;
    Ok(rows.collect::<Result<Vec<_>, _>>()?)
}

fn ensure_queued_mail_recipient_deleted_column(
    conn: &Connection,
) -> Result<(), CampaignStoreError> {
    let mut stmt = conn.prepare("PRAGMA table_info(queued_mail)")?;
    let has_column = stmt
        .query_map([], |row| row.get::<_, String>(1))?
        .collect::<Result<Vec<_>, _>>()?
        .into_iter()
        .any(|name| name == "recipient_deleted");
    if !has_column {
        conn.execute(
            "ALTER TABLE queued_mail
             ADD COLUMN recipient_deleted INTEGER NOT NULL DEFAULT 0",
            [],
        )?;
    }
    Ok(())
}

fn ensure_planet_intel_production_columns(conn: &Connection) -> Result<(), CampaignStoreError> {
    let mut stmt = conn.prepare("PRAGMA table_info(planet_intel)")?;
    let columns = stmt
        .query_map([], |row| row.get::<_, String>(1))?
        .collect::<Result<Vec<_>, _>>()?;
    if !columns
        .iter()
        .any(|name| name == "known_current_production")
    {
        conn.execute(
            "ALTER TABLE planet_intel ADD COLUMN known_current_production INTEGER",
            [],
        )?;
    }
    if !columns.iter().any(|name| name == "known_stored_points") {
        conn.execute(
            "ALTER TABLE planet_intel ADD COLUMN known_stored_points INTEGER",
            [],
        )?;
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// report_blocks storage
// ---------------------------------------------------------------------------

fn write_report_block_rows(
    tx: &rusqlite::Transaction<'_>,
    snapshot_id: i64,
    rows: &[ReportBlockRow],
) -> Result<(), CampaignStoreError> {
    let mut stmt = tx.prepare(
        "INSERT INTO report_blocks(snapshot_id, block_index, decoded_text, raw_hex, recipient_deleted)
         VALUES (?1, ?2, ?3, ?4, ?5)",
    )?;
    for row in rows {
        stmt.execute(params![
            snapshot_id,
            row.block_index as i64,
            &row.decoded_text,
            row.raw_bytes.as_ref().map(|bytes| encode_hex(bytes)),
            i64::from(u8::from(row.recipient_deleted)),
        ])?;
    }
    Ok(())
}

/// Load active (non-deleted) report block rows for a snapshot.
fn load_report_block_rows(
    conn: &mut Connection,
    snapshot_id: i64,
) -> Result<Vec<ReportBlockRow>, CampaignStoreError> {
    load_report_block_rows_filtered(conn, snapshot_id, false)
}

/// Load all report block rows for a snapshot, including soft-deleted ones.
fn load_all_report_block_rows(
    conn: &mut Connection,
    snapshot_id: i64,
) -> Result<Vec<ReportBlockRow>, CampaignStoreError> {
    load_report_block_rows_filtered(conn, snapshot_id, true)
}

fn load_report_block_rows_filtered(
    conn: &mut Connection,
    snapshot_id: i64,
    include_deleted: bool,
) -> Result<Vec<ReportBlockRow>, CampaignStoreError> {
    let sql = if include_deleted {
        "SELECT block_index, decoded_text, raw_hex, recipient_deleted
         FROM report_blocks WHERE snapshot_id = ?1
         ORDER BY block_index"
    } else {
        "SELECT block_index, decoded_text, raw_hex, recipient_deleted
         FROM report_blocks WHERE snapshot_id = ?1 AND recipient_deleted = 0
         ORDER BY block_index"
    };
    let mut stmt = conn.prepare(sql)?;
    let rows = stmt.query_map(params![snapshot_id], |row| {
        Ok(ReportBlockRow {
            block_index: row.get::<_, i64>(0)? as usize,
            decoded_text: row.get(1)?,
            raw_bytes: row
                .get::<_, Option<String>>(2)?
                .map(|hex| decode_hex(&hex))
                .transpose()
                .map_err(|err| {
                    rusqlite::Error::FromSqlConversionFailure(
                        2,
                        rusqlite::types::Type::Text,
                        Box::new(std::io::Error::new(std::io::ErrorKind::InvalidData, err)),
                    )
                })?,
            recipient_deleted: row.get::<_, i64>(3)? != 0,
        })
    })?;
    Ok(rows.collect::<Result<Vec<_>, _>>()?)
}

// ---------------------------------------------------------------------------
// Public report-block mutation API (for ec-client soft-delete)
// ---------------------------------------------------------------------------

impl CampaignStore {
    /// Soft-delete a single report block by index.
    pub fn mark_report_block_deleted(
        &self,
        snapshot_id: i64,
        block_index: usize,
    ) -> Result<(), CampaignStoreError> {
        let conn = self.connection()?;
        conn.execute(
            "UPDATE report_blocks SET recipient_deleted = 1
             WHERE snapshot_id = ?1 AND block_index = ?2",
            params![snapshot_id, block_index as i64],
        )?;
        Ok(())
    }

    /// Soft-delete all report blocks for a snapshot.
    pub fn mark_all_report_blocks_deleted(
        &self,
        snapshot_id: i64,
    ) -> Result<(), CampaignStoreError> {
        let conn = self.connection()?;
        conn.execute(
            "UPDATE report_blocks SET recipient_deleted = 1 WHERE snapshot_id = ?1",
            params![snapshot_id],
        )?;
        Ok(())
    }

    /// Check whether any active (non-deleted) report blocks exist.
    pub fn has_active_report_blocks(&self, snapshot_id: i64) -> Result<bool, CampaignStoreError> {
        let conn = self.connection()?;
        let exists: bool = conn.query_row(
            "SELECT EXISTS(
                 SELECT 1 FROM report_blocks
                 WHERE snapshot_id = ?1 AND recipient_deleted = 0
             )",
            params![snapshot_id],
            |row| row.get(0),
        )?;
        Ok(exists)
    }

    /// Load the active (non-deleted) report block rows for the latest snapshot.
    pub fn load_latest_report_block_rows(&self) -> Result<Vec<ReportBlockRow>, CampaignStoreError> {
        let mut conn = self.connection()?;
        let Some((snapshot_id, _)) = latest_snapshot_id_and_year(&mut conn)? else {
            return Ok(Vec::new());
        };
        load_report_block_rows(&mut conn, snapshot_id)
    }
}
