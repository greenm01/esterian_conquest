use std::collections::BTreeMap;

use ec_data::ReportBlockRow;

use super::state::App;

impl App {
    pub(crate) fn append_report_block(&mut self, text: impl Into<String>) {
        let next_index = self
            .report_block_rows
            .iter()
            .map(|row| row.block_index)
            .max()
            .map(|idx| idx + 1)
            .unwrap_or(0);
        self.report_block_rows.push(ReportBlockRow {
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
        self.report_block_rows.iter().any(|r| !r.recipient_deleted)
    }
}
