use rusqlite::{OptionalExtension, params};

use super::{CampaignStore, CampaignStoreError};
use crate::{GameConfig, SeatReservation};

pub const DEFAULT_CAMPAIGN_THEME_KEY: &str = "tokyo_night";
pub const DEFAULT_MAINTENANCE_INTERVAL_MINUTES: u32 = 24 * 60;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CampaignSettings {
    pub slug: String,
    pub game_name: String,
    pub default_theme_key: String,
    pub snoop_enabled: bool,
    pub session_max_idle_minutes: u8,
    pub session_minimum_time_minutes: u8,
    pub session_local_timeout: bool,
    pub session_remote_timeout: bool,
    pub inactivity_purge_after_turns: u8,
    pub inactivity_autopilot_after_turns: u8,
    pub maintenance_enabled: bool,
    pub maintenance_interval_minutes: u32,
    pub maintenance_next_due_unix_seconds: Option<u64>,
    pub reservations: Vec<SeatReservation>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SessionLeaseState {
    PendingSsh,
    Active,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionLease {
    pub session_token: String,
    pub player_record_index_1_based: usize,
    pub player_npub: String,
    pub state: SessionLeaseState,
    pub started_at_unix_seconds: u64,
    pub last_heartbeat_at_unix_seconds: u64,
    pub expires_at_unix_seconds: u64,
}

#[derive(Debug)]
pub enum SessionLeaseError {
    SeatBusy { player_record_index_1_based: usize },
    InvalidToken,
    Store(CampaignStoreError),
}

impl CampaignSettings {
    pub fn new(slug: impl Into<String>, game_name: impl Into<String>) -> Self {
        Self {
            slug: slug.into(),
            game_name: game_name.into(),
            default_theme_key: DEFAULT_CAMPAIGN_THEME_KEY.to_string(),
            snoop_enabled: true,
            session_max_idle_minutes: 10,
            session_minimum_time_minutes: 0,
            session_local_timeout: false,
            session_remote_timeout: true,
            inactivity_purge_after_turns: 0,
            inactivity_autopilot_after_turns: 0,
            maintenance_enabled: false,
            maintenance_interval_minutes: DEFAULT_MAINTENANCE_INTERVAL_MINUTES,
            maintenance_next_due_unix_seconds: None,
            reservations: Vec::new(),
        }
    }

    pub fn from_legacy_game_config(
        slug: impl Into<String>,
        config: &GameConfig,
        maintenance_next_due_unix_seconds: Option<u64>,
    ) -> Self {
        Self {
            slug: slug.into(),
            game_name: config.game_name.clone(),
            default_theme_key: config
                .theme
                .as_ref()
                .and_then(|path| path.file_stem())
                .and_then(|stem| stem.to_str())
                .map(normalize_theme_key)
                .unwrap_or_else(|| DEFAULT_CAMPAIGN_THEME_KEY.to_string()),
            snoop_enabled: config.snoop,
            session_max_idle_minutes: config.session.max_idle_minutes,
            session_minimum_time_minutes: config.session.minimum_time_minutes,
            session_local_timeout: config.session.local_timeout,
            session_remote_timeout: config.session.remote_timeout,
            inactivity_purge_after_turns: config.inactivity.purge_after_turns,
            inactivity_autopilot_after_turns: config.inactivity.autopilot_after_turns,
            maintenance_enabled: false,
            maintenance_interval_minutes: DEFAULT_MAINTENANCE_INTERVAL_MINUTES,
            maintenance_next_due_unix_seconds,
            reservations: config.reservations.clone(),
        }
    }

    pub fn validate(self) -> Result<Self, CampaignStoreError> {
        let slug = self.slug.trim();
        if slug.is_empty() {
            return Err(CampaignStoreError::InvalidState(
                "campaign slug must not be blank".to_string(),
            ));
        }
        if self.game_name.trim().is_empty() {
            return Err(CampaignStoreError::InvalidState(
                "campaign game_name must not be blank".to_string(),
            ));
        }
        if self.default_theme_key.trim().is_empty() {
            return Err(CampaignStoreError::InvalidState(
                "campaign default_theme_key must not be blank".to_string(),
            ));
        }
        if self.session_max_idle_minutes > 120 {
            return Err(CampaignStoreError::InvalidState(format!(
                "session_max_idle_minutes must be <= 120, got {}",
                self.session_max_idle_minutes
            )));
        }
        if self.session_minimum_time_minutes > 120 {
            return Err(CampaignStoreError::InvalidState(format!(
                "session_minimum_time_minutes must be <= 120, got {}",
                self.session_minimum_time_minutes
            )));
        }
        if self.inactivity_purge_after_turns > 100 {
            return Err(CampaignStoreError::InvalidState(format!(
                "inactivity_purge_after_turns must be <= 100, got {}",
                self.inactivity_purge_after_turns
            )));
        }
        if self.inactivity_autopilot_after_turns > 100 {
            return Err(CampaignStoreError::InvalidState(format!(
                "inactivity_autopilot_after_turns must be <= 100, got {}",
                self.inactivity_autopilot_after_turns
            )));
        }
        if self.maintenance_interval_minutes == 0 {
            return Err(CampaignStoreError::InvalidState(
                "maintenance_interval_minutes must be >= 1".to_string(),
            ));
        }
        validate_reservations(&self.reservations)?;
        Ok(self)
    }

    pub fn reservation_for_alias(&self, alias: &str) -> Option<&SeatReservation> {
        let alias = alias.trim();
        self.reservations
            .iter()
            .find(|reservation| reservation.alias.eq_ignore_ascii_case(alias))
    }

    pub fn reservation_for_player(
        &self,
        player_record_index_1_based: usize,
    ) -> Option<&SeatReservation> {
        self.reservations.iter().find(|reservation| {
            reservation.player_record_index_1_based == player_record_index_1_based
        })
    }

    pub fn validate_reservations_for_player_count(
        &self,
        player_count: usize,
    ) -> Result<(), CampaignStoreError> {
        for reservation in &self.reservations {
            if reservation.player_record_index_1_based > player_count {
                return Err(CampaignStoreError::InvalidState(format!(
                    "reservation player {} exceeds player count {}",
                    reservation.player_record_index_1_based, player_count
                )));
            }
        }
        Ok(())
    }

    pub fn maintenance_due_at(&self, now_unix_seconds: u64) -> bool {
        self.maintenance_enabled
            && self
                .maintenance_next_due_unix_seconds
                .map(|due| due <= now_unix_seconds)
                .unwrap_or(false)
    }
}

impl SessionLeaseState {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::PendingSsh => "pending_ssh",
            Self::Active => "active",
        }
    }

    fn parse(value: &str) -> Option<Self> {
        match value {
            "pending_ssh" => Some(Self::PendingSsh),
            "active" => Some(Self::Active),
            _ => None,
        }
    }
}

impl std::fmt::Display for SessionLeaseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::SeatBusy {
                player_record_index_1_based,
            } => write!(
                f,
                "seat {player_record_index_1_based} already has an active session"
            ),
            Self::InvalidToken => write!(f, "session token is invalid or expired"),
            Self::Store(source) => write!(f, "{source}"),
        }
    }
}

impl std::error::Error for SessionLeaseError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Store(source) => Some(source),
            _ => None,
        }
    }
}

impl From<CampaignStoreError> for SessionLeaseError {
    fn from(value: CampaignStoreError) -> Self {
        Self::Store(value)
    }
}

impl From<rusqlite::Error> for SessionLeaseError {
    fn from(value: rusqlite::Error) -> Self {
        Self::Store(CampaignStoreError::Sql(value))
    }
}

impl CampaignStore {
    pub fn campaign_settings(&self) -> Result<Option<CampaignSettings>, CampaignStoreError> {
        let conn = self.connection()?;
        let Some(mut settings) = conn
            .query_row(
                "SELECT slug,
                        game_name,
                        default_theme_key,
                        snoop_enabled,
                        session_max_idle_minutes,
                        session_minimum_time_minutes,
                        session_local_timeout,
                        session_remote_timeout,
                        inactivity_purge_after_turns,
                        inactivity_autopilot_after_turns,
                        maintenance_enabled,
                        maintenance_interval_minutes,
                        maintenance_next_due_unix_seconds
                 FROM campaign_settings
                 WHERE singleton = 1",
                [],
                |row| {
                    Ok(CampaignSettings {
                        slug: row.get(0)?,
                        game_name: row.get(1)?,
                        default_theme_key: row.get(2)?,
                        snoop_enabled: row.get::<_, i64>(3)? != 0,
                        session_max_idle_minutes: row.get::<_, i64>(4)? as u8,
                        session_minimum_time_minutes: row.get::<_, i64>(5)? as u8,
                        session_local_timeout: row.get::<_, i64>(6)? != 0,
                        session_remote_timeout: row.get::<_, i64>(7)? != 0,
                        inactivity_purge_after_turns: row.get::<_, i64>(8)? as u8,
                        inactivity_autopilot_after_turns: row.get::<_, i64>(9)? as u8,
                        maintenance_enabled: row.get::<_, i64>(10)? != 0,
                        maintenance_interval_minutes: row.get::<_, i64>(11)? as u32,
                        maintenance_next_due_unix_seconds: row
                            .get::<_, Option<i64>>(12)?
                            .map(|value| value as u64),
                        reservations: Vec::new(),
                    })
                },
            )
            .optional()?
        else {
            return Ok(None);
        };
        settings.reservations = load_reservations_conn(&conn)?;
        Ok(Some(settings.validate()?))
    }

    pub fn load_campaign_settings(&self) -> Result<CampaignSettings, CampaignStoreError> {
        self.campaign_settings()?.ok_or_else(|| {
            CampaignStoreError::InvalidState(
                "campaign settings are missing from ecgame.db".to_string(),
            )
        })
    }

    pub fn save_campaign_settings(
        &self,
        settings: &CampaignSettings,
    ) -> Result<(), CampaignStoreError> {
        let settings = settings.clone().validate()?;
        let mut conn = self.connection()?;
        let tx = conn.transaction()?;
        tx.execute(
            "INSERT INTO campaign_settings (
                 singleton,
                 slug,
                 game_name,
                 default_theme_key,
                 snoop_enabled,
                 session_max_idle_minutes,
                 session_minimum_time_minutes,
                 session_local_timeout,
                 session_remote_timeout,
                 inactivity_purge_after_turns,
                 inactivity_autopilot_after_turns,
                 maintenance_enabled,
                 maintenance_interval_minutes,
                 maintenance_next_due_unix_seconds
             ) VALUES (1, ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)
             ON CONFLICT(singleton) DO UPDATE SET
                 slug = excluded.slug,
                 game_name = excluded.game_name,
                 default_theme_key = excluded.default_theme_key,
                 snoop_enabled = excluded.snoop_enabled,
                 session_max_idle_minutes = excluded.session_max_idle_minutes,
                 session_minimum_time_minutes = excluded.session_minimum_time_minutes,
                 session_local_timeout = excluded.session_local_timeout,
                 session_remote_timeout = excluded.session_remote_timeout,
                 inactivity_purge_after_turns = excluded.inactivity_purge_after_turns,
                 inactivity_autopilot_after_turns = excluded.inactivity_autopilot_after_turns,
                 maintenance_enabled = excluded.maintenance_enabled,
                 maintenance_interval_minutes = excluded.maintenance_interval_minutes,
                 maintenance_next_due_unix_seconds = excluded.maintenance_next_due_unix_seconds",
            params![
                settings.slug,
                settings.game_name,
                settings.default_theme_key,
                i64::from(settings.snoop_enabled),
                i64::from(settings.session_max_idle_minutes),
                i64::from(settings.session_minimum_time_minutes),
                i64::from(settings.session_local_timeout),
                i64::from(settings.session_remote_timeout),
                i64::from(settings.inactivity_purge_after_turns),
                i64::from(settings.inactivity_autopilot_after_turns),
                i64::from(settings.maintenance_enabled),
                i64::from(settings.maintenance_interval_minutes),
                settings
                    .maintenance_next_due_unix_seconds
                    .map(|value| value as i64),
            ],
        )?;
        tx.execute("DELETE FROM seat_reservations", [])?;
        for reservation in &settings.reservations {
            tx.execute(
                "INSERT INTO seat_reservations(player_record_index, alias)
                 VALUES (?1, ?2)",
                params![
                    reservation.player_record_index_1_based as i64,
                    reservation.alias.trim()
                ],
            )?;
        }
        tx.commit()?;
        self.apply_runtime_policy_settings(&settings)?;
        Ok(())
    }

    pub fn create_pending_session_lease(
        &self,
        session_token: &str,
        player_record_index_1_based: usize,
        player_npub: &str,
        now_unix_seconds: u64,
        ttl_seconds: u64,
    ) -> Result<SessionLease, SessionLeaseError> {
        let mut conn = self.connection()?;
        let tx = conn.transaction()?;
        prune_expired_session_leases_tx(&tx, now_unix_seconds)?;
        let existing_player = tx
            .query_row(
                "SELECT player_record_index
                 FROM active_sessions
                 WHERE player_record_index = ?1
                 LIMIT 1",
                [player_record_index_1_based as i64],
                |row| row.get::<_, i64>(0),
            )
            .optional()?;
        if existing_player.is_some() {
            return Err(SessionLeaseError::SeatBusy {
                player_record_index_1_based,
            });
        }
        let expires_at_unix_seconds = now_unix_seconds.saturating_add(ttl_seconds.max(1));
        tx.execute(
            "INSERT INTO active_sessions (
                 session_token,
                 player_record_index,
                 player_npub,
                 state,
                 started_at,
                 last_heartbeat_at,
                 expires_at
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                session_token,
                player_record_index_1_based as i64,
                player_npub,
                SessionLeaseState::PendingSsh.as_str(),
                now_unix_seconds as i64,
                now_unix_seconds as i64,
                expires_at_unix_seconds as i64,
            ],
        )?;
        let lease =
            load_session_lease_tx(&tx, session_token)?.ok_or(SessionLeaseError::InvalidToken)?;
        tx.commit()?;
        Ok(lease)
    }

    pub fn activate_session_lease(
        &self,
        session_token: &str,
        now_unix_seconds: u64,
        ttl_seconds: u64,
    ) -> Result<SessionLease, SessionLeaseError> {
        self.touch_session_lease(
            session_token,
            SessionLeaseState::Active,
            now_unix_seconds,
            ttl_seconds,
        )
    }

    pub fn heartbeat_session_lease(
        &self,
        session_token: &str,
        now_unix_seconds: u64,
        ttl_seconds: u64,
    ) -> Result<SessionLease, SessionLeaseError> {
        let existing = self.load_session_lease(session_token, now_unix_seconds)?;
        let next_state = existing.state;
        self.touch_session_lease(session_token, next_state, now_unix_seconds, ttl_seconds)
    }

    pub fn load_session_lease(
        &self,
        session_token: &str,
        now_unix_seconds: u64,
    ) -> Result<SessionLease, SessionLeaseError> {
        let mut conn = self.connection()?;
        let tx = conn.transaction()?;
        prune_expired_session_leases_tx(&tx, now_unix_seconds)?;
        let lease =
            load_session_lease_tx(&tx, session_token)?.ok_or(SessionLeaseError::InvalidToken)?;
        tx.commit()?;
        Ok(lease)
    }

    pub fn release_session_lease(&self, session_token: &str) -> Result<(), CampaignStoreError> {
        let conn = self.connection()?;
        conn.execute(
            "DELETE FROM active_sessions WHERE session_token = ?1",
            [session_token],
        )?;
        Ok(())
    }

    pub fn has_live_session_leases(
        &self,
        now_unix_seconds: u64,
    ) -> Result<bool, CampaignStoreError> {
        let mut conn = self.connection()?;
        let tx = conn.transaction()?;
        prune_expired_session_leases_tx(&tx, now_unix_seconds)?;
        let exists = tx
            .query_row("SELECT 1 FROM active_sessions LIMIT 1", [], |row| {
                row.get::<_, i64>(0)
            })
            .optional()?;
        tx.commit()?;
        Ok(exists.is_some())
    }

    pub fn live_session_for_npub(
        &self,
        player_npub: &str,
        now_unix_seconds: u64,
    ) -> Result<Option<SessionLease>, CampaignStoreError> {
        let mut conn = self.connection()?;
        let tx = conn.transaction()?;
        prune_expired_session_leases_tx(&tx, now_unix_seconds)?;
        let lease = tx
            .query_row(
                "SELECT session_token,
                        player_record_index,
                        player_npub,
                        state,
                        started_at,
                        last_heartbeat_at,
                        expires_at
                 FROM active_sessions
                 WHERE player_npub = ?1
                 LIMIT 1",
                [player_npub],
                |row| {
                    let state_raw: String = row.get(3)?;
                    let state = SessionLeaseState::parse(&state_raw).ok_or_else(|| {
                        rusqlite::Error::FromSqlConversionFailure(
                            3,
                            rusqlite::types::Type::Text,
                            Box::new(CampaignStoreError::InvalidState(format!(
                                "unknown session lease state: {state_raw}",
                            ))),
                        )
                    })?;
                    Ok(SessionLease {
                        session_token: row.get(0)?,
                        player_record_index_1_based: row.get::<_, i64>(1)? as usize,
                        player_npub: row.get(2)?,
                        state,
                        started_at_unix_seconds: row.get::<_, i64>(4)? as u64,
                        last_heartbeat_at_unix_seconds: row.get::<_, i64>(5)? as u64,
                        expires_at_unix_seconds: row.get::<_, i64>(6)? as u64,
                    })
                },
            )
            .optional()?;
        tx.commit()?;
        Ok(lease)
    }

    fn touch_session_lease(
        &self,
        session_token: &str,
        state: SessionLeaseState,
        now_unix_seconds: u64,
        ttl_seconds: u64,
    ) -> Result<SessionLease, SessionLeaseError> {
        let mut conn = self.connection()?;
        let tx = conn.transaction()?;
        prune_expired_session_leases_tx(&tx, now_unix_seconds)?;
        let updated = tx.execute(
            "UPDATE active_sessions
             SET state = ?2,
                 last_heartbeat_at = ?3,
                 expires_at = ?4
             WHERE session_token = ?1",
            params![
                session_token,
                state.as_str(),
                now_unix_seconds as i64,
                now_unix_seconds.saturating_add(ttl_seconds.max(1)) as i64,
            ],
        )?;
        if updated == 0 {
            return Err(SessionLeaseError::InvalidToken);
        }
        let lease =
            load_session_lease_tx(&tx, session_token)?.ok_or(SessionLeaseError::InvalidToken)?;
        tx.commit()?;
        Ok(lease)
    }

    fn apply_runtime_policy_settings(
        &self,
        settings: &CampaignSettings,
    ) -> Result<(), CampaignStoreError> {
        let Some(runtime_state) = self.load_latest_runtime_state()? else {
            return Ok(());
        };
        let mut game_data = runtime_state.game_data;
        let setup = &mut game_data.setup;
        let mut changed = false;

        if setup.snoop_enabled() != settings.snoop_enabled {
            setup.set_snoop_enabled(settings.snoop_enabled);
            changed = true;
        }
        if setup.max_time_between_keys_minutes_raw() != settings.session_max_idle_minutes {
            setup.set_max_time_between_keys_minutes_raw(settings.session_max_idle_minutes);
            changed = true;
        }
        if setup.minimum_time_granted_minutes_raw() != settings.session_minimum_time_minutes {
            setup.set_minimum_time_granted_minutes_raw(settings.session_minimum_time_minutes);
            changed = true;
        }
        if setup.local_timeout_enabled() != settings.session_local_timeout {
            setup.set_local_timeout_enabled(settings.session_local_timeout);
            changed = true;
        }
        if setup.remote_timeout_enabled() != settings.session_remote_timeout {
            setup.set_remote_timeout_enabled(settings.session_remote_timeout);
            changed = true;
        }
        if setup.purge_after_turns_raw() != settings.inactivity_purge_after_turns {
            setup.set_purge_after_turns_raw(settings.inactivity_purge_after_turns);
            changed = true;
        }
        if setup.autopilot_inactive_turns_raw() != settings.inactivity_autopilot_after_turns {
            setup.set_autopilot_inactive_turns_raw(settings.inactivity_autopilot_after_turns);
            changed = true;
        }
        if changed {
            self.save_runtime_state_structured(
                &game_data,
                &runtime_state.planet_scorch_orders,
                &runtime_state.report_block_rows,
                &runtime_state.queued_mail,
            )?;
        }
        Ok(())
    }
}

fn validate_reservations(reservations: &[SeatReservation]) -> Result<(), CampaignStoreError> {
    let mut seen_players = std::collections::BTreeSet::new();
    let mut seen_aliases = std::collections::BTreeSet::new();
    for reservation in reservations {
        if reservation.player_record_index_1_based == 0 {
            return Err(CampaignStoreError::InvalidState(
                "reservation player must be >= 1".to_string(),
            ));
        }
        if !seen_players.insert(reservation.player_record_index_1_based) {
            return Err(CampaignStoreError::InvalidState(format!(
                "duplicate reservation for player {}",
                reservation.player_record_index_1_based
            )));
        }
        let alias = reservation.alias.trim();
        if alias.is_empty() {
            return Err(CampaignStoreError::InvalidState(
                "reservation alias must contain at least one visible character".to_string(),
            ));
        }
        if !seen_aliases.insert(alias.to_ascii_lowercase()) {
            return Err(CampaignStoreError::InvalidState(format!(
                "duplicate reservation alias '{}'",
                reservation.alias
            )));
        }
    }
    Ok(())
}

fn load_reservations_conn(
    conn: &rusqlite::Connection,
) -> Result<Vec<SeatReservation>, CampaignStoreError> {
    let mut stmt = conn.prepare(
        "SELECT player_record_index, alias
         FROM seat_reservations
         ORDER BY player_record_index ASC",
    )?;
    let rows = stmt.query_map([], |row| {
        Ok(SeatReservation {
            player_record_index_1_based: row.get::<_, i64>(0)? as usize,
            alias: row.get(1)?,
        })
    })?;
    Ok(rows.collect::<Result<Vec<_>, _>>()?)
}

fn prune_expired_session_leases_tx(
    tx: &rusqlite::Transaction<'_>,
    now_unix_seconds: u64,
) -> Result<(), CampaignStoreError> {
    tx.execute(
        "DELETE FROM active_sessions WHERE expires_at <= ?1",
        [now_unix_seconds as i64],
    )?;
    Ok(())
}

fn load_session_lease_tx(
    tx: &rusqlite::Transaction<'_>,
    session_token: &str,
) -> Result<Option<SessionLease>, CampaignStoreError> {
    tx.query_row(
        "SELECT session_token,
                player_record_index,
                player_npub,
                state,
                started_at,
                last_heartbeat_at,
                expires_at
         FROM active_sessions
         WHERE session_token = ?1
         LIMIT 1",
        [session_token],
        |row| {
            let state_raw: String = row.get(3)?;
            let state = SessionLeaseState::parse(&state_raw).ok_or_else(|| {
                rusqlite::Error::FromSqlConversionFailure(
                    3,
                    rusqlite::types::Type::Text,
                    Box::new(std::io::Error::new(
                        std::io::ErrorKind::InvalidData,
                        format!("unknown session lease state: {state_raw}"),
                    )),
                )
            })?;
            Ok(SessionLease {
                session_token: row.get(0)?,
                player_record_index_1_based: row.get::<_, i64>(1)? as usize,
                player_npub: row.get(2)?,
                state,
                started_at_unix_seconds: row.get::<_, i64>(4)? as u64,
                last_heartbeat_at_unix_seconds: row.get::<_, i64>(5)? as u64,
                expires_at_unix_seconds: row.get::<_, i64>(6)? as u64,
            })
        },
    )
    .optional()
    .map_err(CampaignStoreError::Sql)
}

fn normalize_theme_key(raw: &str) -> String {
    raw.trim().replace('-', "_").to_ascii_lowercase()
}
