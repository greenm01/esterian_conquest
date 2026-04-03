use rusqlite::{OptionalExtension, params};

use super::{CampaignStore, CampaignStoreError};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HostedSeatStatus {
    Pending,
    Claimed,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HostedSeat {
    pub player_record_index_1_based: usize,
    pub invite_code: String,
    pub status: HostedSeatStatus,
    pub player_npub: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct HostedSeatClaimResult {
    pub seat: HostedSeat,
    pub newly_claimed: bool,
}

#[derive(Debug)]
pub enum ClaimHostedSeatError {
    InvalidCode,
    CodeClaimed,
    IdentityAlreadyClaimedDifferentSeat { player_record_index_1_based: usize },
    Store(CampaignStoreError),
}

impl HostedSeatStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Claimed => "claimed",
        }
    }

    fn parse(value: &str) -> Option<Self> {
        match value {
            "pending" => Some(Self::Pending),
            "claimed" => Some(Self::Claimed),
            _ => None,
        }
    }
}

impl std::fmt::Display for ClaimHostedSeatError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidCode => write!(f, "invite code not found"),
            Self::CodeClaimed => write!(f, "invite code already claimed"),
            Self::IdentityAlreadyClaimedDifferentSeat {
                player_record_index_1_based,
            } => write!(
                f,
                "identity already claimed hosted seat {player_record_index_1_based} in this game"
            ),
            Self::Store(source) => write!(f, "{source}"),
        }
    }
}

impl std::error::Error for ClaimHostedSeatError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Store(source) => Some(source),
            _ => None,
        }
    }
}

impl From<CampaignStoreError> for ClaimHostedSeatError {
    fn from(value: CampaignStoreError) -> Self {
        Self::Store(value)
    }
}

impl From<rusqlite::Error> for ClaimHostedSeatError {
    fn from(value: rusqlite::Error) -> Self {
        Self::Store(CampaignStoreError::Sql(value))
    }
}

impl CampaignStore {
    pub fn hosted_seats(&self) -> Result<Vec<HostedSeat>, CampaignStoreError> {
        let conn = self.connection()?;
        load_hosted_seats_conn(&conn)
    }

    pub fn has_hosted_seats(&self) -> Result<bool, CampaignStoreError> {
        let conn = self.connection()?;
        let exists = conn
            .query_row("SELECT 1 FROM hosted_player_seats LIMIT 1", [], |row| {
                row.get::<_, i64>(0)
            })
            .optional()?;
        Ok(exists.is_some())
    }

    pub fn initialize_hosted_seats_if_empty(
        &self,
        seats: &[HostedSeat],
    ) -> Result<bool, CampaignStoreError> {
        validate_hosted_seats(seats)?;
        let mut conn = self.connection()?;
        let tx = conn.transaction()?;
        let count: i64 = tx.query_row("SELECT COUNT(*) FROM hosted_player_seats", [], |row| {
            row.get(0)
        })?;
        if count > 0 {
            tx.commit()?;
            return Ok(false);
        }
        insert_hosted_seats_tx(&tx, seats)?;
        tx.commit()?;
        Ok(true)
    }

    pub fn replace_hosted_seats(&self, seats: &[HostedSeat]) -> Result<(), CampaignStoreError> {
        validate_hosted_seats(seats)?;
        let mut conn = self.connection()?;
        let tx = conn.transaction()?;
        tx.execute("DELETE FROM hosted_player_seats", [])?;
        insert_hosted_seats_tx(&tx, seats)?;
        tx.commit()?;
        Ok(())
    }

    pub fn claim_hosted_seat(
        &self,
        invite_code: &str,
        player_npub: &str,
    ) -> Result<HostedSeat, ClaimHostedSeatError> {
        let mut conn = self.connection()?;
        let tx = conn.transaction()?;
        let normalized = invite_code.trim().to_ascii_lowercase();
        let Some(mut seat) = load_hosted_seat_by_code_tx(&tx, &normalized)? else {
            return Err(ClaimHostedSeatError::InvalidCode);
        };
        match seat.status {
            HostedSeatStatus::Claimed => {
                if seat.player_npub.as_deref() == Some(player_npub) {
                    tx.commit()?;
                    return Ok(seat);
                }
                return Err(ClaimHostedSeatError::CodeClaimed);
            }
            HostedSeatStatus::Pending => {}
        }
        if let Some(existing_seat) =
            find_claimed_seat_for_npub_tx(&tx, player_npub, Some(seat.player_record_index_1_based))?
        {
            return Err(ClaimHostedSeatError::IdentityAlreadyClaimedDifferentSeat {
                player_record_index_1_based: existing_seat.player_record_index_1_based,
            });
        }
        tx.execute(
            "UPDATE hosted_player_seats
             SET claim_status = 'claimed', player_npub = ?2
             WHERE player_record_index = ?1",
            params![seat.player_record_index_1_based as i64, player_npub],
        )?;
        seat.status = HostedSeatStatus::Claimed;
        seat.player_npub = Some(player_npub.to_string());
        tx.commit()?;
        Ok(seat)
    }

    pub fn claim_hosted_seat_for_player(
        &self,
        player_record_index_1_based: usize,
        player_npub: &str,
    ) -> Result<Option<HostedSeat>, CampaignStoreError> {
        let mut conn = self.connection()?;
        let tx = conn.transaction()?;
        let seat = claim_hosted_seat_for_player_tx(&tx, player_record_index_1_based, player_npub)?
            .map(|result| result.seat);
        tx.commit()?;
        Ok(seat)
    }

    pub fn reissue_hosted_seat(
        &self,
        player_record_index_1_based: usize,
        invite_code: &str,
    ) -> Result<Option<HostedSeat>, CampaignStoreError> {
        let mut conn = self.connection()?;
        let tx = conn.transaction()?;
        let updated = tx.execute(
            "UPDATE hosted_player_seats
             SET invite_code = ?2, claim_status = 'pending', player_npub = NULL
             WHERE player_record_index = ?1",
            params![
                player_record_index_1_based as i64,
                invite_code.trim().to_ascii_lowercase()
            ],
        )?;
        if updated == 0 {
            tx.commit()?;
            return Ok(None);
        }
        let seat = load_hosted_seat_by_player_tx(&tx, player_record_index_1_based)?;
        tx.commit()?;
        Ok(seat)
    }
}

fn validate_hosted_seats(seats: &[HostedSeat]) -> Result<(), CampaignStoreError> {
    let mut seen_players = std::collections::BTreeSet::new();
    let mut seen_codes = std::collections::BTreeSet::new();
    for seat in seats {
        if seat.player_record_index_1_based == 0 {
            return Err(CampaignStoreError::InvalidState(
                "hosted player seat index must be >= 1".to_string(),
            ));
        }
        if !seen_players.insert(seat.player_record_index_1_based) {
            return Err(CampaignStoreError::InvalidState(
                "duplicate hosted player seat index".to_string(),
            ));
        }
        let invite_code = seat.invite_code.trim().to_ascii_lowercase();
        if invite_code.is_empty() {
            return Err(CampaignStoreError::InvalidState(
                "hosted invite code must not be blank".to_string(),
            ));
        }
        if !seen_codes.insert(invite_code) {
            return Err(CampaignStoreError::InvalidState(
                "duplicate hosted invite code".to_string(),
            ));
        }
        match seat.status {
            HostedSeatStatus::Pending if seat.player_npub.is_some() => {
                return Err(CampaignStoreError::InvalidState(
                    "pending hosted seat must not have player_npub".to_string(),
                ));
            }
            HostedSeatStatus::Claimed
                if seat.player_npub.as_deref().unwrap_or("").trim().is_empty() =>
            {
                return Err(CampaignStoreError::InvalidState(
                    "claimed hosted seat must have player_npub".to_string(),
                ));
            }
            _ => {}
        }
    }
    Ok(())
}

fn insert_hosted_seats_tx(
    tx: &rusqlite::Transaction<'_>,
    seats: &[HostedSeat],
) -> Result<(), CampaignStoreError> {
    for seat in seats {
        tx.execute(
            "INSERT INTO hosted_player_seats (
                 player_record_index,
                 invite_code,
                 claim_status,
                 player_npub
             ) VALUES (?1, ?2, ?3, ?4)",
            params![
                seat.player_record_index_1_based as i64,
                seat.invite_code.trim().to_ascii_lowercase(),
                seat.status.as_str(),
                seat.player_npub.as_deref(),
            ],
        )?;
    }
    Ok(())
}

fn load_hosted_seats_conn(
    conn: &rusqlite::Connection,
) -> Result<Vec<HostedSeat>, CampaignStoreError> {
    let mut stmt = conn.prepare(
        "SELECT player_record_index, invite_code, claim_status, player_npub
         FROM hosted_player_seats
         ORDER BY player_record_index ASC",
    )?;
    let rows = stmt.query_map([], |row| {
        let status_raw: String = row.get(2)?;
        let status = HostedSeatStatus::parse(&status_raw).ok_or_else(|| {
            rusqlite::Error::FromSqlConversionFailure(
                2,
                rusqlite::types::Type::Text,
                Box::new(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    format!("unknown hosted seat status: {status_raw}"),
                )),
            )
        })?;
        Ok(HostedSeat {
            player_record_index_1_based: row.get::<_, i64>(0)? as usize,
            invite_code: row.get(1)?,
            status,
            player_npub: row.get(3)?,
        })
    })?;
    Ok(rows.collect::<Result<Vec<_>, _>>()?)
}

fn load_hosted_seat_by_code_tx(
    tx: &rusqlite::Transaction<'_>,
    invite_code: &str,
) -> Result<Option<HostedSeat>, CampaignStoreError> {
    tx.query_row(
        "SELECT player_record_index, invite_code, claim_status, player_npub
         FROM hosted_player_seats
         WHERE invite_code = ?1
         LIMIT 1",
        [invite_code],
        |row| {
            let status_raw: String = row.get(2)?;
            let status = HostedSeatStatus::parse(&status_raw).ok_or_else(|| {
                rusqlite::Error::FromSqlConversionFailure(
                    2,
                    rusqlite::types::Type::Text,
                    Box::new(std::io::Error::new(
                        std::io::ErrorKind::InvalidData,
                        format!("unknown hosted seat status: {status_raw}"),
                    )),
                )
            })?;
            Ok(HostedSeat {
                player_record_index_1_based: row.get::<_, i64>(0)? as usize,
                invite_code: row.get(1)?,
                status,
                player_npub: row.get(3)?,
            })
        },
    )
    .optional()
    .map_err(CampaignStoreError::Sql)
}

pub(super) fn load_hosted_seat_by_player_tx(
    tx: &rusqlite::Transaction<'_>,
    player_record_index_1_based: usize,
) -> Result<Option<HostedSeat>, CampaignStoreError> {
    tx.query_row(
        "SELECT player_record_index, invite_code, claim_status, player_npub
         FROM hosted_player_seats
         WHERE player_record_index = ?1
         LIMIT 1",
        [player_record_index_1_based as i64],
        |row| {
            let status_raw: String = row.get(2)?;
            let status = HostedSeatStatus::parse(&status_raw).ok_or_else(|| {
                rusqlite::Error::FromSqlConversionFailure(
                    2,
                    rusqlite::types::Type::Text,
                    Box::new(std::io::Error::new(
                        std::io::ErrorKind::InvalidData,
                        format!("unknown hosted seat status: {status_raw}"),
                    )),
                )
            })?;
            Ok(HostedSeat {
                player_record_index_1_based: row.get::<_, i64>(0)? as usize,
                invite_code: row.get(1)?,
                status,
                player_npub: row.get(3)?,
            })
        },
    )
    .optional()
    .map_err(CampaignStoreError::Sql)
}

pub(super) fn claim_hosted_seat_for_player_tx(
    tx: &rusqlite::Transaction<'_>,
    player_record_index_1_based: usize,
    player_npub: &str,
) -> Result<Option<HostedSeatClaimResult>, CampaignStoreError> {
    let Some(mut seat) = load_hosted_seat_by_player_tx(tx, player_record_index_1_based)? else {
        return Ok(None);
    };
    match seat.status {
        HostedSeatStatus::Claimed => {
            if seat.player_npub.as_deref() == Some(player_npub) {
                return Ok(Some(HostedSeatClaimResult {
                    seat,
                    newly_claimed: false,
                }));
            }
            return Err(CampaignStoreError::InvalidState(format!(
                "hosted seat {} is already claimed by another player identity",
                player_record_index_1_based
            )));
        }
        HostedSeatStatus::Pending => {}
    }
    if let Some(existing_seat) =
        find_claimed_seat_for_npub_tx(tx, player_npub, Some(player_record_index_1_based))?
    {
        return Err(CampaignStoreError::InvalidState(format!(
            "player identity already claimed hosted seat {} in this game",
            existing_seat.player_record_index_1_based
        )));
    }
    tx.execute(
        "UPDATE hosted_player_seats
         SET claim_status = 'claimed', player_npub = ?2
         WHERE player_record_index = ?1",
        params![player_record_index_1_based as i64, player_npub],
    )?;
    seat.status = HostedSeatStatus::Claimed;
    seat.player_npub = Some(player_npub.to_string());
    Ok(Some(HostedSeatClaimResult {
        seat,
        newly_claimed: true,
    }))
}

fn find_claimed_seat_for_npub_tx(
    tx: &rusqlite::Transaction<'_>,
    player_npub: &str,
    exclude_player_record_index_1_based: Option<usize>,
) -> Result<Option<HostedSeat>, CampaignStoreError> {
    let mut stmt = tx.prepare(
        "SELECT player_record_index, invite_code, claim_status, player_npub
         FROM hosted_player_seats
         WHERE claim_status = 'claimed'
           AND player_npub = ?1
         ORDER BY player_record_index ASC",
    )?;
    let rows = stmt.query_map([player_npub], |row| {
        let status_raw: String = row.get(2)?;
        let status = HostedSeatStatus::parse(&status_raw).ok_or_else(|| {
            rusqlite::Error::FromSqlConversionFailure(
                2,
                rusqlite::types::Type::Text,
                Box::new(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    format!("unknown hosted seat status: {status_raw}"),
                )),
            )
        })?;
        Ok(HostedSeat {
            player_record_index_1_based: row.get::<_, i64>(0)? as usize,
            invite_code: row.get(1)?,
            status,
            player_npub: row.get(3)?,
        })
    })?;
    for row in rows {
        let seat = row?;
        if exclude_player_record_index_1_based != Some(seat.player_record_index_1_based) {
            return Ok(Some(seat));
        }
    }
    Ok(None)
}
