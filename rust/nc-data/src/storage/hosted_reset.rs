use rusqlite::{OptionalExtension, params};

use super::{CampaignStore, CampaignStoreError, HostedSeat};
use crate::{CoreGameData, QueuedPlayerMail, ReportBlockRow, merge_player_intel_from_runtime};

impl CampaignStore {
    pub fn reissue_hosted_seat_and_reset_runtime(
        &self,
        player_record_index_1_based: usize,
        invite_code: &str,
        baseline_game_data: &CoreGameData,
        now_unix_seconds: u64,
    ) -> Result<Option<HostedSeat>, CampaignStoreError> {
        let current_state = self.load_latest_runtime_state()?.ok_or_else(|| {
            CampaignStoreError::InvalidState("campaign store has no runtime snapshots".to_string())
        })?;
        if current_state.game_year != 3000 {
            return Err(CampaignStoreError::InvalidState(
                "nuke-seat is only allowed during year 3000 before the first maintenance turn"
                    .to_string(),
            ));
        }
        if baseline_game_data.conquest.game_year() != 3000 {
            return Err(CampaignStoreError::InvalidState(
                "nuke-seat baseline must be a year-3000 new game".to_string(),
            ));
        }
        if baseline_game_data.conquest.player_count()
            != current_state.game_data.conquest.player_count()
        {
            return Err(CampaignStoreError::InvalidState(format!(
                "nuke-seat baseline player count {} does not match campaign player count {}",
                baseline_game_data.conquest.player_count(),
                current_state.game_data.conquest.player_count(),
            )));
        }

        let player_count = current_state.game_data.conquest.player_count() as usize;
        if !(1..=player_count).contains(&player_record_index_1_based) {
            return Err(CampaignStoreError::InvalidState(format!(
                "player {} exceeds campaign player count {}",
                player_record_index_1_based, player_count
            )));
        }

        let updated_game_data = reset_runtime_player_slice(
            &current_state.game_data,
            baseline_game_data,
            player_record_index_1_based,
        )?;
        let queued_mail = reset_runtime_mail(
            &current_state.queued_mail,
            player_record_index_1_based as u8,
        );
        let report_block_rows = Vec::<ReportBlockRow>::new();
        let updated_game_data =
            sync_review_flags(updated_game_data, &queued_mail, &report_block_rows);
        let mut planet_intel_by_viewer = self.load_snapshot_planet_intel_by_viewer(
            current_state.snapshot_id,
            current_state.game_data.conquest.player_count(),
        )?;
        planet_intel_by_viewer[player_record_index_1_based - 1] = merge_player_intel_from_runtime(
            baseline_game_data,
            player_record_index_1_based as u8,
            current_state.game_year,
            None,
            None,
        );

        let mut conn = self.connection()?;
        let tx = conn.transaction()?;
        let latest_snapshot = tx
            .query_row(
                "SELECT id, game_year FROM snapshots ORDER BY game_year DESC LIMIT 1",
                [],
                |row| Ok((row.get::<_, i64>(0)?, row.get::<_, i64>(1)? as u16)),
            )
            .optional()?;
        let Some((latest_snapshot_id, latest_year)) = latest_snapshot else {
            return Err(CampaignStoreError::InvalidState(
                "campaign store has no runtime snapshots".to_string(),
            ));
        };
        if latest_snapshot_id != current_state.snapshot_id || latest_year != current_state.game_year
        {
            return Err(CampaignStoreError::InvalidState(
                "campaign runtime changed while preparing nuke-seat; retry the command".to_string(),
            ));
        }
        if super::settings::has_live_session_leases_tx(&tx, now_unix_seconds)? {
            return Err(CampaignStoreError::InvalidState(
                "nuke-seat is blocked while any live session lease exists".to_string(),
            ));
        }
        let Some(_) =
            super::hosted_seats::load_hosted_seat_by_player_tx(&tx, player_record_index_1_based)?
        else {
            tx.commit()?;
            return Ok(None);
        };

        tx.execute(
            "DELETE FROM player_client_preferences
             WHERE player_record_index = ?1",
            [player_record_index_1_based as i64],
        )?;
        tx.execute(
            "DELETE FROM hosted_publish_jobs
             WHERE player_record_index = ?1",
            [player_record_index_1_based as i64],
        )?;
        tx.execute(
            "UPDATE hosted_player_seats
             SET invite_code = ?2, claim_status = 'pending', player_npub = NULL
             WHERE player_record_index = ?1",
            params![
                player_record_index_1_based as i64,
                invite_code.trim().to_ascii_lowercase()
            ],
        )?;

        super::runtime::save_runtime_state_internal_tx(
            &tx,
            &updated_game_data,
            current_state.game_year,
            &current_state.planet_scorch_orders,
            &report_block_rows,
            &queued_mail,
            Some(&planet_intel_by_viewer),
            Some(current_state.campaign_seed),
            None,
            None,
            None,
        )?;
        let seat =
            super::hosted_seats::load_hosted_seat_by_player_tx(&tx, player_record_index_1_based)?;
        tx.commit()?;
        Ok(seat)
    }
}

fn reset_runtime_player_slice(
    current_game_data: &CoreGameData,
    baseline_game_data: &CoreGameData,
    player_record_index_1_based: usize,
) -> Result<CoreGameData, CampaignStoreError> {
    let player_index = player_record_index_1_based - 1;
    let baseline_player = baseline_game_data
        .player
        .records
        .get(player_index)
        .ok_or_else(|| {
            CampaignStoreError::InvalidState(format!(
                "baseline is missing player {}",
                player_record_index_1_based
            ))
        })?;
    let mut updated = current_game_data.clone();
    updated.player.records[player_index] = baseline_player.clone();

    let homeworld_index = baseline_player.homeworld_planet_index_1_based_raw() as usize;
    if homeworld_index == 0 {
        return Err(CampaignStoreError::InvalidState(format!(
            "baseline player {} is missing a homeworld",
            player_record_index_1_based
        )));
    }
    let planet_index = homeworld_index - 1;
    let baseline_homeworld = baseline_game_data
        .planets
        .records
        .get(planet_index)
        .ok_or_else(|| {
            CampaignStoreError::InvalidState(format!(
                "baseline homeworld {} is missing for player {}",
                homeworld_index, player_record_index_1_based
            ))
        })?;
    updated.planets.records[planet_index] = baseline_homeworld.clone();

    let fleet_start = baseline_player.fleet_chain_head_raw() as usize;
    let fleet_end = baseline_player.fleet_chain_tail_raw() as usize;
    if fleet_start == 0 || fleet_end < fleet_start {
        return Err(CampaignStoreError::InvalidState(format!(
            "baseline player {} has invalid starter fleet range {}..={}",
            player_record_index_1_based, fleet_start, fleet_end
        )));
    }
    for fleet_record_index_1_based in fleet_start..=fleet_end {
        let fleet_index = fleet_record_index_1_based - 1;
        let baseline_fleet = baseline_game_data
            .fleets
            .records
            .get(fleet_index)
            .ok_or_else(|| {
                CampaignStoreError::InvalidState(format!(
                    "baseline fleet {} is missing for player {}",
                    fleet_record_index_1_based, player_record_index_1_based
                ))
            })?;
        updated.fleets.records[fleet_index] = baseline_fleet.clone();
    }

    Ok(updated)
}

fn reset_runtime_mail(
    queued_mail: &[QueuedPlayerMail],
    player_empire_id: u8,
) -> Vec<QueuedPlayerMail> {
    queued_mail
        .iter()
        .filter(|mail| {
            mail.sender_empire_id != player_empire_id
                && mail.recipient_empire_id != player_empire_id
        })
        .cloned()
        .collect()
}

fn sync_review_flags(
    mut game_data: CoreGameData,
    queued_mail: &[QueuedPlayerMail],
    report_block_rows: &[ReportBlockRow],
) -> CoreGameData {
    for (idx, player) in game_data.player.records.iter_mut().enumerate() {
        let player_empire_id = (idx + 1) as u8;
        let has_results = report_block_rows
            .iter()
            .any(|row| !row.recipient_deleted && row.is_visible_to_viewer(player_empire_id));
        let has_mail = queued_mail
            .iter()
            .any(|mail| mail.is_visible_to_recipient(player_empire_id));
        player.set_classic_login_reviewables_present(has_results || has_mail);
        player.set_classic_results_chain_state(has_results, if has_results { 1 } else { 0 });
    }
    game_data
}
