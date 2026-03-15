use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use rusqlite::{Connection, OptionalExtension, params};

use crate::{
    BASE_RECORD_SIZE, BaseDat, CoreGameData, DatabaseDat, FLEET_RECORD_SIZE, FleetDat,
    IPBM_RECORD_SIZE, IpbmDat, PLANET_RECORD_SIZE, PLAYER_RECORD_SIZE, PlanetDat, PlayerDat,
    PlayerStarmapWorld, QueuedPlayerMail, SetupDat, build_player_starmap_projection,
};

pub const DEFAULT_CAMPAIGN_DB_NAME: &str = "ecgame.db";
const COMPAT_FILE_NAMES: &[&str] = &["DATABASE.DAT", "MESSAGES.DAT", "RESULTS.DAT"];

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
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CampaignStore {
    path: PathBuf,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CampaignRuntimeState {
    pub snapshot_id: i64,
    pub game_year: u16,
    pub game_data: CoreGameData,
    pub database: DatabaseDat,
    pub results_bytes: Vec<u8>,
    pub messages_bytes: Vec<u8>,
    pub queued_mail: Vec<QueuedPlayerMail>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct IntelSnapshotRow {
    intel_tier: IntelTier,
    last_intel_year: Option<u16>,
    known_name: Option<String>,
    known_owner_empire_id: Option<u8>,
    known_potential_production: Option<u16>,
    known_armies: Option<u8>,
    known_ground_batteries: Option<u8>,
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
        let game_data = CoreGameData::load(dir)?;
        let database = DatabaseDat::parse(&read_path(dir.join("DATABASE.DAT"))?)?;
        let results_bytes = read_path(dir.join("RESULTS.DAT"))?;
        let messages_bytes = read_path(dir.join("MESSAGES.DAT"))?;
        let queued_mail = load_mail_queue_file(dir)?;
        self.save_runtime_state(
            &game_data,
            &database,
            &results_bytes,
            &messages_bytes,
            &queued_mail,
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
        for name in COMPAT_FILE_NAMES {
            let bytes: Vec<u8> = conn.query_row(
                "SELECT bytes FROM compat_files WHERE snapshot_id = ?1 AND name = ?2",
                params![snapshot_id, *name],
                |row| row.get(0),
            )?;
            write_path(dir.join(name), &bytes)?;
        }
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
            "SELECT planet_record_index, intel_tier, last_intel_year
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
        let game_data = load_snapshot_game_data(&mut conn, snapshot_id)?;
        let database = DatabaseDat::parse(&compat_file_bytes(
            &mut conn,
            snapshot_id,
            "DATABASE.DAT",
        )?)?;
        let results_bytes = compat_file_bytes(&mut conn, snapshot_id, "RESULTS.DAT")?;
        let messages_bytes = compat_file_bytes(&mut conn, snapshot_id, "MESSAGES.DAT")?;
        let queued_mail = load_queued_mail_rows(&mut conn, snapshot_id)?;
        Ok(Some(CampaignRuntimeState {
            snapshot_id,
            game_year,
            game_data,
            database,
            results_bytes,
            messages_bytes,
            queued_mail,
        }))
    }

    pub fn save_runtime_state(
        &self,
        game_data: &CoreGameData,
        database: &DatabaseDat,
        results_bytes: &[u8],
        messages_bytes: &[u8],
        queued_mail: &[QueuedPlayerMail],
    ) -> Result<i64, CampaignStoreError> {
        let year = game_data.conquest.game_year();
        let mut conn = self.connection()?;
        let tx = conn.transaction()?;
        tx.execute("DELETE FROM snapshots WHERE game_year = ?1", params![i64::from(year)])?;
        tx.execute(
            "INSERT INTO snapshots(game_year) VALUES (?1)",
            params![i64::from(year)],
        )?;
        let snapshot_id = tx.last_insert_rowid();
        write_record_rows(
            &tx,
            "player_records",
            snapshot_id,
            &game_data.player.to_bytes(),
            PLAYER_RECORD_SIZE,
        )?;
        write_record_rows(
            &tx,
            "planet_records",
            snapshot_id,
            &game_data.planets.to_bytes(),
            PLANET_RECORD_SIZE,
        )?;
        write_record_rows(
            &tx,
            "fleet_records",
            snapshot_id,
            &game_data.fleets.to_bytes(),
            FLEET_RECORD_SIZE,
        )?;
        write_record_rows(
            &tx,
            "base_records",
            snapshot_id,
            &game_data.bases.to_bytes(),
            BASE_RECORD_SIZE,
        )?;
        write_record_rows(
            &tx,
            "ipbm_records",
            snapshot_id,
            &game_data.ipbm.to_bytes(),
            IPBM_RECORD_SIZE,
        )?;
        tx.execute(
            "INSERT INTO setup_records(snapshot_id, raw) VALUES (?1, ?2)",
            params![snapshot_id, game_data.setup.to_bytes()],
        )?;
        tx.execute(
            "INSERT INTO conquest_records(snapshot_id, raw) VALUES (?1, ?2)",
            params![snapshot_id, game_data.conquest.to_bytes()],
        )?;
        tx.execute(
            "INSERT INTO compat_files(snapshot_id, name, bytes) VALUES (?1, 'DATABASE.DAT', ?2)",
            params![snapshot_id, database.to_bytes()],
        )?;
        tx.execute(
            "INSERT INTO compat_files(snapshot_id, name, bytes) VALUES (?1, 'RESULTS.DAT', ?2)",
            params![snapshot_id, results_bytes],
        )?;
        tx.execute(
            "INSERT INTO compat_files(snapshot_id, name, bytes) VALUES (?1, 'MESSAGES.DAT', ?2)",
            params![snapshot_id, messages_bytes],
        )?;
        write_queued_mail_rows(&tx, snapshot_id, queued_mail)?;

        let previous_snapshot_id = tx
            .query_row(
                "SELECT id FROM snapshots WHERE game_year < ?1 ORDER BY game_year DESC LIMIT 1",
                params![i64::from(year)],
                |row| row.get::<_, i64>(0),
            )
            .optional()?;
        let previous = if let Some(previous_snapshot_id) = previous_snapshot_id {
            load_intel_rows(&tx, previous_snapshot_id)?
        } else {
            BTreeMap::new()
        };
        write_planet_intel_rows(&tx, snapshot_id, game_data, database, year, &previous)?;
        tx.commit()?;
        Ok(snapshot_id)
    }

    fn initialize(&self) -> Result<(), CampaignStoreError> {
        let conn = self.connection()?;
        conn.execute_batch(
            "PRAGMA foreign_keys = ON;
             CREATE TABLE IF NOT EXISTS snapshots (
                 id INTEGER PRIMARY KEY,
                 game_year INTEGER NOT NULL UNIQUE
             );
             CREATE TABLE IF NOT EXISTS player_records (
                 snapshot_id INTEGER NOT NULL REFERENCES snapshots(id) ON DELETE CASCADE,
                 record_index INTEGER NOT NULL,
                 raw BLOB NOT NULL,
                 PRIMARY KEY(snapshot_id, record_index)
             );
             CREATE TABLE IF NOT EXISTS planet_records (
                 snapshot_id INTEGER NOT NULL REFERENCES snapshots(id) ON DELETE CASCADE,
                 record_index INTEGER NOT NULL,
                 raw BLOB NOT NULL,
                 PRIMARY KEY(snapshot_id, record_index)
             );
             CREATE TABLE IF NOT EXISTS fleet_records (
                 snapshot_id INTEGER NOT NULL REFERENCES snapshots(id) ON DELETE CASCADE,
                 record_index INTEGER NOT NULL,
                 raw BLOB NOT NULL,
                 PRIMARY KEY(snapshot_id, record_index)
             );
             CREATE TABLE IF NOT EXISTS base_records (
                 snapshot_id INTEGER NOT NULL REFERENCES snapshots(id) ON DELETE CASCADE,
                 record_index INTEGER NOT NULL,
                 raw BLOB NOT NULL,
                 PRIMARY KEY(snapshot_id, record_index)
             );
             CREATE TABLE IF NOT EXISTS ipbm_records (
                 snapshot_id INTEGER NOT NULL REFERENCES snapshots(id) ON DELETE CASCADE,
                 record_index INTEGER NOT NULL,
                 raw BLOB NOT NULL,
                 PRIMARY KEY(snapshot_id, record_index)
             );
             CREATE TABLE IF NOT EXISTS setup_records (
                 snapshot_id INTEGER PRIMARY KEY REFERENCES snapshots(id) ON DELETE CASCADE,
                 raw BLOB NOT NULL
             );
             CREATE TABLE IF NOT EXISTS conquest_records (
                 snapshot_id INTEGER PRIMARY KEY REFERENCES snapshots(id) ON DELETE CASCADE,
                 raw BLOB NOT NULL
             );
             CREATE TABLE IF NOT EXISTS compat_files (
                 snapshot_id INTEGER NOT NULL REFERENCES snapshots(id) ON DELETE CASCADE,
                 name TEXT NOT NULL,
                 bytes BLOB NOT NULL,
                 PRIMARY KEY(snapshot_id, name)
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
                 PRIMARY KEY(snapshot_id, queue_index)
             );",
        )?;
        Ok(())
    }

    fn connection(&self) -> Result<Connection, CampaignStoreError> {
        Connection::open(&self.path).map_err(CampaignStoreError::Sql)
    }
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

fn write_record_rows(
    tx: &rusqlite::Transaction<'_>,
    table: &str,
    snapshot_id: i64,
    bytes: &[u8],
    record_size: usize,
) -> Result<(), CampaignStoreError> {
    let sql = format!("INSERT INTO {table}(snapshot_id, record_index, raw) VALUES (?1, ?2, ?3)");
    let mut stmt = tx.prepare(&sql)?;
    for (idx, chunk) in bytes.chunks_exact(record_size).enumerate() {
        stmt.execute(params![snapshot_id, (idx + 1) as i64, chunk])?;
    }
    Ok(())
}

fn read_record_rows(
    conn: &mut Connection,
    table: &str,
    snapshot_id: i64,
    expected_size: usize,
) -> Result<Vec<u8>, CampaignStoreError> {
    let sql = format!(
        "SELECT raw FROM {table} WHERE snapshot_id = ?1 ORDER BY record_index"
    );
    let mut stmt = conn.prepare(&sql)?;
    let rows = stmt.query_map(params![snapshot_id], |row| row.get::<_, Vec<u8>>(0))?;
    let mut bytes = Vec::new();
    for row in rows {
        let row = row?;
        if row.len() != expected_size {
            return Err(CampaignStoreError::Parse(crate::ParseError::WrongRecordMultiple {
                file_type: "sqlite-record",
                record_size: expected_size,
                actual: row.len(),
            }));
        }
        bytes.extend_from_slice(&row);
    }
    Ok(bytes)
}

fn load_snapshot_game_data(
    conn: &mut Connection,
    snapshot_id: i64,
) -> Result<CoreGameData, CampaignStoreError> {
    Ok(CoreGameData {
        player: PlayerDat::parse(&read_record_rows(conn, "player_records", snapshot_id, PLAYER_RECORD_SIZE)?)?,
        planets: PlanetDat::parse(&read_record_rows(conn, "planet_records", snapshot_id, PLANET_RECORD_SIZE)?)?,
        fleets: FleetDat::parse(&read_record_rows(conn, "fleet_records", snapshot_id, FLEET_RECORD_SIZE)?)?,
        bases: BaseDat::parse(&read_record_rows(conn, "base_records", snapshot_id, BASE_RECORD_SIZE)?)?,
        ipbm: IpbmDat::parse(&read_record_rows(conn, "ipbm_records", snapshot_id, IPBM_RECORD_SIZE)?)?,
        setup: SetupDat::parse(
            &conn.query_row(
                "SELECT raw FROM setup_records WHERE snapshot_id = ?1",
                params![snapshot_id],
                |row| row.get::<_, Vec<u8>>(0),
            )?,
        )?,
        conquest: crate::ConquestDat::parse(
            &conn.query_row(
                "SELECT raw FROM conquest_records WHERE snapshot_id = ?1",
                params![snapshot_id],
                |row| row.get::<_, Vec<u8>>(0),
            )?,
        )?,
    })
}

fn compat_file_bytes(
    conn: &mut Connection,
    snapshot_id: i64,
    name: &str,
) -> Result<Vec<u8>, CampaignStoreError> {
    conn.query_row(
        "SELECT bytes FROM compat_files WHERE snapshot_id = ?1 AND name = ?2",
        params![snapshot_id, name],
        |row| row.get::<_, Vec<u8>>(0),
    )
    .map_err(CampaignStoreError::Sql)
}

fn load_intel_rows(
    tx: &rusqlite::Transaction<'_>,
    snapshot_id: i64,
) -> Result<BTreeMap<(u8, usize), IntelSnapshotRow>, CampaignStoreError> {
    let mut stmt = tx.prepare(
        "SELECT viewer_empire_id, planet_record_index, intel_tier, last_intel_year,
                known_name, known_owner_empire_id, known_potential_production,
                known_armies, known_ground_batteries
         FROM planet_intel
         WHERE snapshot_id = ?1",
    )?;
    let rows = stmt.query_map(params![snapshot_id], |row| {
        Ok((
            (row.get::<_, i64>(0)? as u8, row.get::<_, i64>(1)? as usize),
            IntelSnapshotRow {
                intel_tier: IntelTier::from_str(&row.get::<_, String>(2)?),
                last_intel_year: row.get::<_, Option<i64>>(3)?.map(|value| value as u16),
                known_name: row.get(4)?,
                known_owner_empire_id: row.get::<_, Option<i64>>(5)?.map(|value| value as u8),
                known_potential_production: row.get::<_, Option<i64>>(6)?.map(|value| value as u16),
                known_armies: row.get::<_, Option<i64>>(7)?.map(|value| value as u8),
                known_ground_batteries: row.get::<_, Option<i64>>(8)?.map(|value| value as u8),
            },
        ))
    })?;
    Ok(rows.collect::<Result<BTreeMap<_, _>, _>>()?)
}

fn write_planet_intel_rows(
    tx: &rusqlite::Transaction<'_>,
    snapshot_id: i64,
    game_data: &CoreGameData,
    database: &DatabaseDat,
    year: u16,
    previous: &BTreeMap<(u8, usize), IntelSnapshotRow>,
) -> Result<(), CampaignStoreError> {
    let player_count = game_data.conquest.player_count();
    let mut stmt = tx.prepare(
        "INSERT INTO planet_intel(
             snapshot_id, viewer_empire_id, planet_record_index, intel_tier, last_intel_year,
             known_name, known_owner_empire_id, known_potential_production,
             known_armies, known_ground_batteries
         ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
    )?;
    for viewer_empire_id in 1..=player_count {
        let projection = build_player_starmap_projection(game_data, database, viewer_empire_id);
        for world in projection.worlds {
            let planet_record_index_1_based = world.planet_record_index_1_based;
            let intel_tier = infer_intel_tier(viewer_empire_id, &world);
            let previous_row = previous.get(&(viewer_empire_id, planet_record_index_1_based));
            let current_fingerprint = intel_fingerprint(intel_tier, &world);
            let last_intel_year = if intel_tier == IntelTier::Unknown {
                None
            } else if intel_tier == IntelTier::Owned {
                Some(year)
            } else if previous_row
                .map(|row| intel_snapshot_row_fingerprint(row) == current_fingerprint)
                .unwrap_or(false)
            {
                previous_row.and_then(|row| row.last_intel_year).or(Some(year))
            } else {
                Some(year)
            };
            stmt.execute(params![
                snapshot_id,
                i64::from(viewer_empire_id),
                planet_record_index_1_based as i64,
                intel_tier.as_str(),
                last_intel_year.map(i64::from),
                world.known_name,
                world.known_owner_empire_id.map(i64::from),
                world.known_potential_production.map(i64::from),
                world.known_armies.map(i64::from),
                world.known_ground_batteries.map(i64::from),
            ])?;
        }
    }
    Ok(())
}

fn infer_intel_tier(viewer_empire_id: u8, world: &PlayerStarmapWorld) -> IntelTier {
    if world.known_owner_empire_id == Some(viewer_empire_id) {
        IntelTier::Owned
    } else if world.known_armies.is_some() || world.known_ground_batteries.is_some() {
        IntelTier::Full
    } else if world.known_name.is_some()
        || world.known_owner_empire_id.is_some()
        || world.known_potential_production.is_some()
    {
        IntelTier::Partial
    } else {
        IntelTier::Unknown
    }
}

fn intel_fingerprint(
    intel_tier: IntelTier,
    world: &PlayerStarmapWorld,
) -> (
    IntelTier,
    Option<String>,
    Option<u8>,
    Option<u16>,
    Option<u8>,
    Option<u8>,
) {
    (
        intel_tier,
        world.known_name.clone(),
        world.known_owner_empire_id,
        world.known_potential_production,
        world.known_armies,
        world.known_ground_batteries,
    )
}

fn intel_snapshot_row_fingerprint(
    row: &IntelSnapshotRow,
) -> (
    IntelTier,
    Option<String>,
    Option<u8>,
    Option<u16>,
    Option<u8>,
    Option<u8>,
) {
    (
        row.intel_tier,
        row.known_name.clone(),
        row.known_owner_empire_id,
        row.known_potential_production,
        row.known_armies,
        row.known_ground_batteries,
    )
}

fn read_path(path: PathBuf) -> Result<Vec<u8>, CampaignStoreError> {
    fs::read(&path).map_err(|source| CampaignStoreError::Io { path, source })
}

fn write_path(path: PathBuf, bytes: &[u8]) -> Result<(), CampaignStoreError> {
    fs::write(&path, bytes).map_err(|source| CampaignStoreError::Io { path, source })
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

fn write_queued_mail_rows(
    tx: &rusqlite::Transaction<'_>,
    snapshot_id: i64,
    queued_mail: &[QueuedPlayerMail],
) -> Result<(), CampaignStoreError> {
    let mut stmt = tx.prepare(
        "INSERT INTO queued_mail(
             snapshot_id, queue_index, sender_empire_id, recipient_empire_id, year, subject, body
         ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
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
        ])?;
    }
    Ok(())
}

fn load_queued_mail_rows(
    conn: &mut Connection,
    snapshot_id: i64,
) -> Result<Vec<QueuedPlayerMail>, CampaignStoreError> {
    let mut stmt = conn.prepare(
        "SELECT sender_empire_id, recipient_empire_id, year, subject, body
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
        })
    })?;
    Ok(rows.collect::<Result<Vec<_>, _>>()?)
}
