use std::collections::BTreeMap;
use std::collections::BTreeSet;
use std::time::{SystemTime, UNIX_EPOCH};

use rusqlite::{OptionalExtension, params};

use super::{
    CampaignRuntimeState, CampaignStore, CampaignStoreError, PlanetIntelSnapshot,
    PlayerActivityState, PlayerWarStatsState,
};
use crate::{
    CoreGameData, QueuedPlayerMail, ReportBlockRow, derive_campaign_seed_from_runtime,
    generate_campaign_seed,
};

impl CampaignStore {
    pub fn load_latest_runtime_game_data(&self) -> Result<CoreGameData, CampaignStoreError> {
        let Some(state) = self.load_latest_runtime_state()? else {
            return Err(CampaignStoreError::InvalidState(
                "campaign store has no runtime snapshots".to_string(),
            ));
        };
        Ok(state.game_data)
    }

    pub fn load_snapshot_game_data(
        &self,
        snapshot_id: i64,
    ) -> Result<CoreGameData, CampaignStoreError> {
        let mut conn = self.connection()?;
        super::snapshot_core::load_snapshot_game_data(&mut conn, snapshot_id)
    }

    pub fn load_latest_runtime_state(
        &self,
    ) -> Result<Option<CampaignRuntimeState>, CampaignStoreError> {
        let mut conn = self.connection()?;
        let Some((snapshot_id, game_year)) =
            super::metadata::latest_snapshot_id_and_year(&mut conn)?
        else {
            return Ok(None);
        };
        let game_data = super::snapshot_core::load_snapshot_game_data(&mut conn, snapshot_id)?;
        let planet_scorch_orders =
            super::planet_scorch_orders::load_planet_scorch_orders(&mut conn, snapshot_id)?;
        let report_block_rows =
            super::report_blocks::load_report_block_rows(&mut conn, snapshot_id)?;
        let queued_mail = super::mail::load_queued_mail_rows(&mut conn, snapshot_id)?;
        let stored_campaign_seed = super::metadata::load_campaign_seed(&mut conn)?;
        let campaign_seed = stored_campaign_seed.unwrap_or_else(|| {
            derive_campaign_seed_from_runtime(&game_data, &report_block_rows, &queued_mail)
        });
        if stored_campaign_seed.is_none() {
            super::metadata::persist_campaign_seed(&mut conn, campaign_seed)?;
        }
        Ok(Some(CampaignRuntimeState {
            snapshot_id,
            game_year,
            campaign_seed,
            game_data,
            planet_scorch_orders,
            report_block_rows,
            queued_mail,
        }))
    }

    pub fn save_runtime_state_structured(
        &self,
        game_data: &CoreGameData,
        planet_scorch_orders: &BTreeSet<usize>,
        report_block_rows: &[ReportBlockRow],
        queued_mail: &[QueuedPlayerMail],
    ) -> Result<i64, CampaignStoreError> {
        self.save_runtime_state_internal(
            game_data,
            planet_scorch_orders,
            report_block_rows,
            queued_mail,
            None,
            None,
            None,
            None,
            None,
        )
    }

    pub fn save_runtime_state_structured_and_claim_hosted_seat(
        &self,
        game_data: &CoreGameData,
        planet_scorch_orders: &BTreeSet<usize>,
        report_block_rows: &[ReportBlockRow],
        queued_mail: &[QueuedPlayerMail],
        player_record_index_1_based: usize,
        player_npub: &str,
    ) -> Result<i64, CampaignStoreError> {
        self.save_runtime_state_internal(
            game_data,
            planet_scorch_orders,
            report_block_rows,
            queued_mail,
            None,
            None,
            None,
            None,
            Some((player_record_index_1_based, player_npub)),
        )
    }

    pub fn save_runtime_state_structured_with_intel(
        &self,
        game_data: &CoreGameData,
        planet_scorch_orders: &BTreeSet<usize>,
        report_block_rows: &[ReportBlockRow],
        queued_mail: &[QueuedPlayerMail],
        planet_intel_by_viewer: &[BTreeMap<usize, PlanetIntelSnapshot>],
    ) -> Result<i64, CampaignStoreError> {
        self.save_runtime_state_structured_with_intel_and_seed(
            game_data,
            planet_scorch_orders,
            report_block_rows,
            queued_mail,
            planet_intel_by_viewer,
            None,
        )
    }

    pub fn save_runtime_state_structured_with_intel_and_seed(
        &self,
        game_data: &CoreGameData,
        planet_scorch_orders: &BTreeSet<usize>,
        report_block_rows: &[ReportBlockRow],
        queued_mail: &[QueuedPlayerMail],
        planet_intel_by_viewer: &[BTreeMap<usize, PlanetIntelSnapshot>],
        campaign_seed: Option<u64>,
    ) -> Result<i64, CampaignStoreError> {
        self.save_runtime_state_internal(
            game_data,
            planet_scorch_orders,
            report_block_rows,
            queued_mail,
            Some(planet_intel_by_viewer),
            campaign_seed,
            None,
            None,
            None,
        )
    }

    pub fn save_runtime_state_structured_with_intel_and_activity(
        &self,
        game_data: &CoreGameData,
        planet_scorch_orders: &BTreeSet<usize>,
        report_block_rows: &[ReportBlockRow],
        queued_mail: &[QueuedPlayerMail],
        planet_intel_by_viewer: &[BTreeMap<usize, PlanetIntelSnapshot>],
        player_activity_states: &[PlayerActivityState],
    ) -> Result<i64, CampaignStoreError> {
        self.save_runtime_state_internal(
            game_data,
            planet_scorch_orders,
            report_block_rows,
            queued_mail,
            Some(planet_intel_by_viewer),
            None,
            Some(player_activity_states),
            None,
            None,
        )
    }

    pub fn save_runtime_state_structured_with_intel_activity_and_war_stats(
        &self,
        game_data: &CoreGameData,
        planet_scorch_orders: &BTreeSet<usize>,
        report_block_rows: &[ReportBlockRow],
        queued_mail: &[QueuedPlayerMail],
        planet_intel_by_viewer: &[BTreeMap<usize, PlanetIntelSnapshot>],
        player_activity_states: &[PlayerActivityState],
        player_war_stats: &[PlayerWarStatsState],
    ) -> Result<i64, CampaignStoreError> {
        self.save_runtime_state_internal(
            game_data,
            planet_scorch_orders,
            report_block_rows,
            queued_mail,
            Some(planet_intel_by_viewer),
            None,
            Some(player_activity_states),
            Some(player_war_stats),
            None,
        )
    }

    pub fn save_runtime_state_structured_with_intel_activity_and_claim_hosted_seat(
        &self,
        game_data: &CoreGameData,
        planet_scorch_orders: &BTreeSet<usize>,
        report_block_rows: &[ReportBlockRow],
        queued_mail: &[QueuedPlayerMail],
        planet_intel_by_viewer: &[BTreeMap<usize, PlanetIntelSnapshot>],
        player_activity_states: &[PlayerActivityState],
        player_record_index_1_based: usize,
        player_npub: &str,
    ) -> Result<i64, CampaignStoreError> {
        self.save_runtime_state_internal(
            game_data,
            planet_scorch_orders,
            report_block_rows,
            queued_mail,
            Some(planet_intel_by_viewer),
            None,
            Some(player_activity_states),
            None,
            Some((player_record_index_1_based, player_npub)),
        )
    }

    fn save_runtime_state_internal(
        &self,
        game_data: &CoreGameData,
        planet_scorch_orders: &BTreeSet<usize>,
        report_block_rows: &[ReportBlockRow],
        queued_mail: &[QueuedPlayerMail],
        planet_intel_by_viewer_override: Option<&[BTreeMap<usize, PlanetIntelSnapshot>]>,
        campaign_seed_override: Option<u64>,
        player_activity_override: Option<&[PlayerActivityState]>,
        player_war_stats_override: Option<&[PlayerWarStatsState]>,
        hosted_claim: Option<(usize, &str)>,
    ) -> Result<i64, CampaignStoreError> {
        let year = game_data.conquest.game_year();
        let mut conn = self.connection()?;
        let tx = conn.transaction()?;
        let snapshot_id = save_runtime_state_internal_tx(
            &tx,
            game_data,
            year,
            planet_scorch_orders,
            report_block_rows,
            queued_mail,
            planet_intel_by_viewer_override,
            campaign_seed_override,
            player_activity_override,
            player_war_stats_override,
            hosted_claim,
        )?;
        tx.commit()?;
        Ok(snapshot_id)
    }
}

pub(super) fn save_runtime_state_internal_tx(
    tx: &rusqlite::Transaction<'_>,
    game_data: &CoreGameData,
    year: u16,
    planet_scorch_orders: &BTreeSet<usize>,
    report_block_rows: &[ReportBlockRow],
    queued_mail: &[QueuedPlayerMail],
    planet_intel_by_viewer_override: Option<&[BTreeMap<usize, PlanetIntelSnapshot>]>,
    campaign_seed_override: Option<u64>,
    player_activity_override: Option<&[PlayerActivityState]>,
    player_war_stats_override: Option<&[PlayerWarStatsState]>,
    hosted_claim: Option<(usize, &str)>,
) -> Result<i64, CampaignStoreError> {
    let campaign_seed = super::metadata::load_campaign_seed_tx(&tx)?
        .or(campaign_seed_override)
        .unwrap_or_else(generate_campaign_seed);
    super::metadata::persist_campaign_seed_tx(&tx, campaign_seed)?;
    let previous_snapshot_id = tx
        .query_row(
            "SELECT id FROM snapshots ORDER BY game_year DESC LIMIT 1",
            [],
            |row| row.get::<_, i64>(0),
        )
        .optional()?;
    let previous_intel = if let Some(previous_snapshot_id) = previous_snapshot_id {
        super::intel::load_intel_rows(&tx, previous_snapshot_id)?
    } else {
        BTreeMap::new()
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
    super::snapshot_core::write_snapshot_core_rows(&tx, snapshot_id, game_data)?;
    super::planet_scorch_orders::write_planet_scorch_orders(
        &tx,
        snapshot_id,
        planet_scorch_orders,
    )?;
    super::report_blocks::write_report_block_rows(&tx, snapshot_id, report_block_rows)?;
    super::mail::write_queued_mail_rows(&tx, snapshot_id, queued_mail)?;
    super::intel::write_planet_intel_rows(
        &tx,
        snapshot_id,
        game_data,
        year,
        planet_intel_by_viewer_override,
        &previous_intel,
    )?;
    super::planet_owned_since::write_owned_planet_year_rows(
        &tx,
        snapshot_id,
        game_data,
        year,
        previous_snapshot_id,
    )?;
    super::player_activity::write_player_activity_rows(
        &tx,
        snapshot_id,
        game_data,
        previous_snapshot_id,
        player_activity_override,
    )?;
    super::player_war_stats::write_player_war_stats_rows(
        &tx,
        snapshot_id,
        game_data.conquest.player_count(),
        previous_snapshot_id,
        player_war_stats_override,
    )?;
    if let Some((player_record_index_1_based, player_npub)) = hosted_claim {
        let claim_result = super::hosted_seats::claim_hosted_seat_for_player_tx(
            &tx,
            player_record_index_1_based,
            player_npub,
        )?
        .ok_or_else(|| {
            CampaignStoreError::InvalidState(format!(
                "hosted seat {} is missing",
                player_record_index_1_based
            ))
        })?;
        if claim_result.newly_claimed {
            let created_at_unix_seconds = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map(|duration| duration.as_secs())
                .unwrap_or(0);
            super::hosted_publish_jobs::enqueue_hosted_publish_job_tx(
                &tx,
                super::hosted_publish_jobs::HostedPublishJobKind::MapPackOnFirstClaim,
                player_record_index_1_based,
                player_npub,
                created_at_unix_seconds,
            )?;
        }
    }
    Ok(snapshot_id)
}
