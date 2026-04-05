use std::collections::BTreeMap;

use nc_data::ReportBlockRow;

use super::state::App;
use crate::reports::has_visible_runtime_reports;

impl App {
    pub(crate) fn bind_player_record_index_1_based(
        &mut self,
        player_record_index_1_based: usize,
    ) -> Result<(), Box<dyn std::error::Error>> {
        self.startup_state.reserved_seat_alias = self
            .game_config
            .reservation_for_player(player_record_index_1_based)
            .map(|reservation| reservation.alias.clone());
        self.player = crate::model::PlayerContext::from_game_data(
            &self.game_data,
            player_record_index_1_based,
        )?;
        self.planet_intel_snapshots = self
            .planet
            .campaign_store
            .latest_planet_intel_for_viewer(player_record_index_1_based as u8)?
            .into_iter()
            .map(|snapshot| (snapshot.planet_record_index_1_based, snapshot))
            .collect::<BTreeMap<_, _>>();
        self.planet.intel_snapshots = self.planet_intel_snapshots.clone();
        self.refresh_review_context()?;
        Ok(())
    }

    pub(crate) fn reload_runtime_state(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let runtime_state = self
            .planet
            .campaign_store
            .load_latest_runtime_state()?
            .ok_or(
                "campaign store has no snapshots; initialize the campaign with nc-sysop first",
            )?;
        self.snapshot_id = runtime_state.snapshot_id;
        self.campaign_seed = runtime_state.campaign_seed;
        self.game_data = runtime_state.game_data;
        self.report_block_rows = runtime_state.report_block_rows;
        self.queued_mail = runtime_state.queued_mail;
        self.planet_scorch_orders = runtime_state.planet_scorch_orders;
        Ok(())
    }

    pub(crate) fn reload_runtime_state_and_bind_player_record_index_1_based(
        &mut self,
        player_record_index_1_based: usize,
    ) -> Result<(), Box<dyn std::error::Error>> {
        self.reload_runtime_state()?;
        self.bind_player_record_index_1_based(player_record_index_1_based)
    }

    pub(crate) fn append_report_block(&mut self, text: impl Into<String>) {
        let next_index = self
            .report_block_rows
            .iter()
            .map(|row| row.block_index)
            .max()
            .map(|idx| idx + 1)
            .unwrap_or(0);
        self.report_block_rows.push(ReportBlockRow {
            viewer_empire_id: self.player.record_index_1_based as u8,
            block_index: next_index,
            decoded_text: text.into(),
            raw_bytes: None,
            recipient_deleted: false,
        });
    }

    pub(crate) fn save_game_data(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let new_snapshot_id = self.planet.campaign_store.save_runtime_state_structured(
            &self.game_data,
            &self.planet_scorch_orders,
            &self.report_block_rows,
            &self.queued_mail,
        )?;
        self.snapshot_id = new_snapshot_id;
        self.planet_intel_snapshots = self
            .planet
            .campaign_store
            .latest_planet_intel_for_viewer(self.player.record_index_1_based as u8)?
            .into_iter()
            .map(|snapshot| (snapshot.planet_record_index_1_based, snapshot))
            .collect::<BTreeMap<_, _>>();
        self.planet.intel_snapshots = self.planet_intel_snapshots.clone();
        Ok(())
    }

    pub(crate) fn save_game_data_and_claim_hosted_seat(
        &mut self,
        player_npub: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let new_snapshot_id = self
            .planet
            .campaign_store
            .save_runtime_state_structured_and_claim_hosted_seat(
                &self.game_data,
                &self.planet_scorch_orders,
                &self.report_block_rows,
                &self.queued_mail,
                self.player.record_index_1_based,
                player_npub,
            )?;
        self.snapshot_id = new_snapshot_id;
        self.planet_intel_snapshots = self
            .planet
            .campaign_store
            .latest_planet_intel_for_viewer(self.player.record_index_1_based as u8)?
            .into_iter()
            .map(|snapshot| (snapshot.planet_record_index_1_based, snapshot))
            .collect::<BTreeMap<_, _>>();
        self.planet.intel_snapshots = self.planet_intel_snapshots.clone();
        Ok(())
    }

    pub(crate) fn has_active_report_blocks(&self) -> bool {
        has_visible_runtime_reports(
            self.player.record_index_1_based as u8,
            &self.report_block_rows,
        )
    }
}
