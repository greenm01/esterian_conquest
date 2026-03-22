use std::collections::BTreeMap;

use super::state::App;

impl App {
    pub(crate) fn save_game_data(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let new_snapshot_id = self.planet.campaign_store.save_runtime_state_structured(
            &self.game_data,
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

    pub(crate) fn has_active_report_blocks(&self) -> bool {
        self.report_block_rows.iter().any(|r| !r.recipient_deleted)
    }
}
