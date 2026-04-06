use rusqlite::{Connection, params};

use super::{CampaignStore, CampaignStoreError};
use crate::{EmpireUnitSummary, default_player_war_stats_states};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct PlayerWarStatsState {
    pub player_record_index_1_based: usize,
    pub colonies_established: u32,
    pub worlds_taken: u32,
    pub worlds_lost: u32,
    pub bombardments_launched: u32,
    pub bombardments_suffered: u32,
    pub invade_attempts: u32,
    pub invade_successes: u32,
    pub blitz_attempts: u32,
    pub blitz_successes: u32,
    pub attacks_repelled: u32,
    pub units_lost: EmpireUnitSummary,
    pub enemy_units_destroyed: EmpireUnitSummary,
}

impl PlayerWarStatsState {
    pub fn for_player(player_record_index_1_based: usize) -> Self {
        Self {
            player_record_index_1_based,
            ..Self::default()
        }
    }

    pub fn total_units_lost(self) -> u32 {
        total_unit_count(self.units_lost)
    }

    pub fn total_enemy_units_destroyed(self) -> u32 {
        total_unit_count(self.enemy_units_destroyed)
    }

    pub fn invade_failures(self) -> u32 {
        self.invade_attempts.saturating_sub(self.invade_successes)
    }

    pub fn blitz_failures(self) -> u32 {
        self.blitz_attempts.saturating_sub(self.blitz_successes)
    }
}

pub(crate) fn total_unit_count(summary: EmpireUnitSummary) -> u32 {
    summary.destroyers
        + summary.cruisers
        + summary.battleships
        + summary.scouts
        + summary.transports
        + summary.etacs
        + summary.starbases
        + summary.armies
        + summary.ground_batteries
}

impl CampaignStore {
    pub fn latest_player_war_stats(
        &self,
        player_count: u8,
    ) -> Result<Vec<PlayerWarStatsState>, CampaignStoreError> {
        let mut conn = self.connection()?;
        let Some((snapshot_id, _)) = super::metadata::latest_snapshot_id_and_year(&mut conn)?
        else {
            return Ok(default_player_war_stats_states(player_count));
        };
        load_player_war_stats_rows(&conn, snapshot_id, player_count)
    }
}

pub(super) fn write_player_war_stats_rows(
    tx: &rusqlite::Transaction<'_>,
    snapshot_id: i64,
    player_count: u8,
    previous_snapshot_id: Option<i64>,
    override_states: Option<&[PlayerWarStatsState]>,
) -> Result<(), CampaignStoreError> {
    let states = if let Some(states) = override_states {
        normalize_player_war_stats(states, player_count)
    } else if let Some(previous_snapshot_id) = previous_snapshot_id {
        load_player_war_stats_rows(tx, previous_snapshot_id, player_count)?
    } else {
        default_player_war_stats_states(player_count)
    };

    let mut stmt = tx.prepare(
        "INSERT INTO player_war_stats(
             snapshot_id,
             player_record_index,
             colonies_established,
             worlds_taken,
             worlds_lost,
             bombardments_launched,
             bombardments_suffered,
             invade_attempts,
             invade_successes,
             blitz_attempts,
             blitz_successes,
             attacks_repelled,
             units_lost_destroyers,
             units_lost_cruisers,
             units_lost_battleships,
             units_lost_scouts,
             units_lost_transports,
             units_lost_etacs,
             units_lost_starbases,
             units_lost_armies,
             units_lost_ground_batteries,
             enemy_destroyed_destroyers,
             enemy_destroyed_cruisers,
             enemy_destroyed_battleships,
             enemy_destroyed_scouts,
             enemy_destroyed_transports,
             enemy_destroyed_etacs,
             enemy_destroyed_starbases,
             enemy_destroyed_armies,
             enemy_destroyed_ground_batteries
         ) VALUES (
             ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18,
             ?19, ?20, ?21, ?22, ?23, ?24, ?25, ?26, ?27, ?28, ?29, ?30
         )",
    )?;

    for state in states {
        stmt.execute(params![
            snapshot_id,
            state.player_record_index_1_based as i64,
            i64::from(state.colonies_established),
            i64::from(state.worlds_taken),
            i64::from(state.worlds_lost),
            i64::from(state.bombardments_launched),
            i64::from(state.bombardments_suffered),
            i64::from(state.invade_attempts),
            i64::from(state.invade_successes),
            i64::from(state.blitz_attempts),
            i64::from(state.blitz_successes),
            i64::from(state.attacks_repelled),
            i64::from(state.units_lost.destroyers),
            i64::from(state.units_lost.cruisers),
            i64::from(state.units_lost.battleships),
            i64::from(state.units_lost.scouts),
            i64::from(state.units_lost.transports),
            i64::from(state.units_lost.etacs),
            i64::from(state.units_lost.starbases),
            i64::from(state.units_lost.armies),
            i64::from(state.units_lost.ground_batteries),
            i64::from(state.enemy_units_destroyed.destroyers),
            i64::from(state.enemy_units_destroyed.cruisers),
            i64::from(state.enemy_units_destroyed.battleships),
            i64::from(state.enemy_units_destroyed.scouts),
            i64::from(state.enemy_units_destroyed.transports),
            i64::from(state.enemy_units_destroyed.etacs),
            i64::from(state.enemy_units_destroyed.starbases),
            i64::from(state.enemy_units_destroyed.armies),
            i64::from(state.enemy_units_destroyed.ground_batteries),
        ])?;
    }

    Ok(())
}

fn load_player_war_stats_rows(
    conn: &Connection,
    snapshot_id: i64,
    player_count: u8,
) -> Result<Vec<PlayerWarStatsState>, CampaignStoreError> {
    let mut stmt = conn.prepare(
        "SELECT
             player_record_index,
             colonies_established,
             worlds_taken,
             worlds_lost,
             bombardments_launched,
             bombardments_suffered,
             invade_attempts,
             invade_successes,
             blitz_attempts,
             blitz_successes,
             attacks_repelled,
             units_lost_destroyers,
             units_lost_cruisers,
             units_lost_battleships,
             units_lost_scouts,
             units_lost_transports,
             units_lost_etacs,
             units_lost_starbases,
             units_lost_armies,
             units_lost_ground_batteries,
             enemy_destroyed_destroyers,
             enemy_destroyed_cruisers,
             enemy_destroyed_battleships,
             enemy_destroyed_scouts,
             enemy_destroyed_transports,
             enemy_destroyed_etacs,
             enemy_destroyed_starbases,
             enemy_destroyed_armies,
             enemy_destroyed_ground_batteries
         FROM player_war_stats
         WHERE snapshot_id = ?1
         ORDER BY player_record_index",
    )?;

    let mut states = stmt
        .query_map(params![snapshot_id], |row| {
            Ok(PlayerWarStatsState {
                player_record_index_1_based: row.get::<_, i64>(0)? as usize,
                colonies_established: row.get::<_, i64>(1)? as u32,
                worlds_taken: row.get::<_, i64>(2)? as u32,
                worlds_lost: row.get::<_, i64>(3)? as u32,
                bombardments_launched: row.get::<_, i64>(4)? as u32,
                bombardments_suffered: row.get::<_, i64>(5)? as u32,
                invade_attempts: row.get::<_, i64>(6)? as u32,
                invade_successes: row.get::<_, i64>(7)? as u32,
                blitz_attempts: row.get::<_, i64>(8)? as u32,
                blitz_successes: row.get::<_, i64>(9)? as u32,
                attacks_repelled: row.get::<_, i64>(10)? as u32,
                units_lost: EmpireUnitSummary {
                    destroyers: row.get::<_, i64>(11)? as u32,
                    cruisers: row.get::<_, i64>(12)? as u32,
                    battleships: row.get::<_, i64>(13)? as u32,
                    scouts: row.get::<_, i64>(14)? as u32,
                    transports: row.get::<_, i64>(15)? as u32,
                    etacs: row.get::<_, i64>(16)? as u32,
                    starbases: row.get::<_, i64>(17)? as u32,
                    armies: row.get::<_, i64>(18)? as u32,
                    ground_batteries: row.get::<_, i64>(19)? as u32,
                },
                enemy_units_destroyed: EmpireUnitSummary {
                    destroyers: row.get::<_, i64>(20)? as u32,
                    cruisers: row.get::<_, i64>(21)? as u32,
                    battleships: row.get::<_, i64>(22)? as u32,
                    scouts: row.get::<_, i64>(23)? as u32,
                    transports: row.get::<_, i64>(24)? as u32,
                    etacs: row.get::<_, i64>(25)? as u32,
                    starbases: row.get::<_, i64>(26)? as u32,
                    armies: row.get::<_, i64>(27)? as u32,
                    ground_batteries: row.get::<_, i64>(28)? as u32,
                },
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;

    if states.is_empty() {
        states = default_player_war_stats_states(player_count);
    } else {
        states = normalize_player_war_stats(&states, player_count);
    }

    Ok(states)
}

fn normalize_player_war_stats(
    states: &[PlayerWarStatsState],
    player_count: u8,
) -> Vec<PlayerWarStatsState> {
    let mut normalized = default_player_war_stats_states(player_count);
    for state in states {
        if let Some(slot) = normalized.get_mut(state.player_record_index_1_based.saturating_sub(1))
        {
            *slot = *state;
        }
    }
    normalized
}
