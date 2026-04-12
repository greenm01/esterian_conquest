use rusqlite::{params, Connection, Result as SqliteResult};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GameSettings {
    pub maintenance_enabled: bool,
    pub maintenance_interval_minutes: u32,
    pub maintenance_next_due_unix_seconds: Option<i64>,
    pub lobby_visibility: LobbyVisibility,
    pub recruiting: RecruitingMode,
    pub host_alias: Option<String>,
    pub summary: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LobbyVisibility {
    Public,
    Private,
}

impl LobbyVisibility {
    pub fn as_str(&self) -> &'static str {
        match self {
            LobbyVisibility::Public => "public",
            LobbyVisibility::Private => "private",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "public" => Some(LobbyVisibility::Public),
            "private" => Some(LobbyVisibility::Private),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RecruitingMode {
    None,
    NewPlayers,
    ReplacementPlayers,
}

impl RecruitingMode {
    pub fn as_str(&self) -> &'static str {
        match self {
            RecruitingMode::None => "none",
            RecruitingMode::NewPlayers => "new_players",
            RecruitingMode::ReplacementPlayers => "replacement_players",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "none" => Some(RecruitingMode::None),
            "new_players" => Some(RecruitingMode::NewPlayers),
            "replacement_players" => Some(RecruitingMode::ReplacementPlayers),
            _ => None,
        }
    }
}

pub fn get_settings(conn: &Connection, game_id: &str) -> SqliteResult<GameSettings> {
    let mut stmt = conn.prepare(
        "SELECT maintenance_enabled, maintenance_interval_minutes, maintenance_next_due_unix_seconds,
                lobby_visibility, recruiting, host_alias, summary
         FROM game_metadata WHERE id = ?1"
    )?;

    stmt.query_row(params![game_id], |row| {
        Ok(GameSettings {
            maintenance_enabled: row.get::<_, i32>(0)? != 0,
            maintenance_interval_minutes: row.get(1)?,
            maintenance_next_due_unix_seconds: row.get(2)?,
            lobby_visibility: LobbyVisibility::from_str(&row.get::<_, String>(3)?)
                .unwrap_or(LobbyVisibility::Private),
            recruiting: RecruitingMode::from_str(&row.get::<_, String>(4)?)
                .unwrap_or(RecruitingMode::None),
            host_alias: row.get(5)?,
            summary: row.get(6)?,
        })
    })
}

pub fn update_settings(
    conn: &Connection,
    game_id: &str,
    settings: &GameSettings,
) -> SqliteResult<()> {
    conn.execute(
        "UPDATE game_metadata SET
            maintenance_enabled = ?1,
            maintenance_interval_minutes = ?2,
            maintenance_next_due_unix_seconds = ?3,
            lobby_visibility = ?4,
            recruiting = ?5,
            host_alias = ?6,
            summary = ?7,
            updated_at = ?8
         WHERE id = ?9",
        params![
            settings.maintenance_enabled as i32,
            settings.maintenance_interval_minutes,
            settings.maintenance_next_due_unix_seconds,
            settings.lobby_visibility.as_str(),
            settings.recruiting.as_str(),
            settings.host_alias,
            settings.summary,
            chrono::Utc::now().timestamp(),
            game_id,
        ],
    )?;
    Ok(())
}
