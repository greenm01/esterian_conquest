use std::collections::BTreeMap;

use rusqlite::{params, OptionalExtension};

use super::{CampaignRuntimeState, CampaignStore, CampaignStoreError, PlanetIntelSnapshot};
use crate::{
    derive_campaign_seed_from_runtime, generate_campaign_seed, CoreGameData, QueuedPlayerMail,
    ReportBlockRow,
};

impl CampaignStore {
    pub fn load_snapshot_game_data(
        &self,
        snapshot_id: i64,
    ) -> Result<CoreGameData, CampaignStoreError> {
        let mut conn = self.connection()?;
        super::records::load_snapshot_game_data(&mut conn, snapshot_id)
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
        let game_data = super::records::load_snapshot_game_data(&mut conn, snapshot_id)?;
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
            report_block_rows,
            queued_mail,
        }))
    }

    pub fn save_runtime_state_structured(
        &self,
        game_data: &CoreGameData,
        report_block_rows: &[ReportBlockRow],
        queued_mail: &[QueuedPlayerMail],
    ) -> Result<i64, CampaignStoreError> {
        self.save_runtime_state_internal(game_data, report_block_rows, queued_mail, None, None)
    }

    pub fn save_runtime_state_structured_with_intel(
        &self,
        game_data: &CoreGameData,
        report_block_rows: &[ReportBlockRow],
        queued_mail: &[QueuedPlayerMail],
        planet_intel_by_viewer: &[BTreeMap<usize, PlanetIntelSnapshot>],
    ) -> Result<i64, CampaignStoreError> {
        self.save_runtime_state_structured_with_intel_and_seed(
            game_data,
            report_block_rows,
            queued_mail,
            planet_intel_by_viewer,
            None,
        )
    }

    pub fn save_runtime_state_structured_with_intel_and_seed(
        &self,
        game_data: &CoreGameData,
        report_block_rows: &[ReportBlockRow],
        queued_mail: &[QueuedPlayerMail],
        planet_intel_by_viewer: &[BTreeMap<usize, PlanetIntelSnapshot>],
        campaign_seed: Option<u64>,
    ) -> Result<i64, CampaignStoreError> {
        self.save_runtime_state_internal(
            game_data,
            report_block_rows,
            queued_mail,
            Some(planet_intel_by_viewer),
            campaign_seed,
        )
    }

    fn save_runtime_state_internal(
        &self,
        game_data: &CoreGameData,
        report_block_rows: &[ReportBlockRow],
        queued_mail: &[QueuedPlayerMail],
        planet_intel_by_viewer_override: Option<&[BTreeMap<usize, PlanetIntelSnapshot>]>,
        campaign_seed_override: Option<u64>,
    ) -> Result<i64, CampaignStoreError> {
        let year = game_data.conquest.game_year();
        let mut conn = self.connection()?;
        let tx = conn.transaction()?;
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
        super::records::write_typed_record_rows(
            &tx,
            super::PLAYER_RECORD_FIELDS_TABLE,
            snapshot_id,
            &game_data.player.to_bytes(),
            crate::PLAYER_RECORD_SIZE,
        )?;
        super::records::write_typed_record_rows(
            &tx,
            super::PLANET_RECORD_FIELDS_TABLE,
            snapshot_id,
            &game_data.planets.to_bytes(),
            crate::PLANET_RECORD_SIZE,
        )?;
        super::records::write_typed_record_rows(
            &tx,
            super::FLEET_RECORD_FIELDS_TABLE,
            snapshot_id,
            &game_data.fleets.to_bytes(),
            crate::FLEET_RECORD_SIZE,
        )?;
        super::records::write_typed_record_rows(
            &tx,
            super::BASE_RECORD_FIELDS_TABLE,
            snapshot_id,
            &game_data.bases.to_bytes(),
            crate::BASE_RECORD_SIZE,
        )?;
        super::records::write_typed_record_rows(
            &tx,
            super::IPBM_RECORD_FIELDS_TABLE,
            snapshot_id,
            &game_data.ipbm.to_bytes(),
            crate::IPBM_RECORD_SIZE,
        )?;
        super::records::write_typed_record_rows(
            &tx,
            super::SETUP_RECORD_FIELDS_TABLE,
            snapshot_id,
            &game_data.setup.to_bytes(),
            crate::SETUP_DAT_SIZE,
        )?;
        super::records::write_typed_record_rows(
            &tx,
            super::CONQUEST_RECORD_FIELDS_TABLE,
            snapshot_id,
            &game_data.conquest.to_bytes(),
            crate::CONQUEST_DAT_SIZE,
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
        tx.commit()?;
        Ok(snapshot_id)
    }
}
