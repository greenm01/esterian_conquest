use std::collections::BTreeMap;

use nc_data::{CampaignStore, PlanetIntelSnapshot, ReportBlockRow};

use super::state::DashApp;

impl DashApp {
    pub(crate) fn append_report_block(&mut self, text: impl Into<String>) {
        let next_index = self
            .report_block_rows
            .iter()
            .map(|row| row.block_index)
            .max()
            .map(|idx| idx + 1)
            .unwrap_or(0);
        self.report_block_rows.push(ReportBlockRow {
            viewer_empire_id: self.player_record_index_1_based as u8,
            block_index: next_index,
            decoded_text: text.into(),
            raw_bytes: None,
            recipient_deleted: false,
        });
    }

    pub(crate) fn save_and_refresh_runtime(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let Some(store) = self.campaign_store.clone() else {
            return Err("dashboard campaign store unavailable".into());
        };

        let planet_intel_by_viewer =
            planet_intel_by_viewer(&store, self.game_data.conquest.player_count())?;
        store.save_runtime_state_structured_with_intel_and_activity(
            &self.game_data,
            &self.planet_scorch_orders,
            &self.report_block_rows,
            &self.queued_mail,
            &planet_intel_by_viewer,
            &self.player_activity_states,
        )?;

        let runtime_state = store.load_latest_runtime_state()?.ok_or(
            "campaign store has no snapshots; initialize the campaign with nc-sysop first",
        )?;
        self.game_data = runtime_state.game_data;
        self.report_block_rows = runtime_state.report_block_rows;
        self.queued_mail = runtime_state.queued_mail;
        self.planet_scorch_orders = runtime_state.planet_scorch_orders;
        self.player_activity_states =
            store.latest_player_activity_states(self.game_data.conquest.player_count())?;
        self.owned_planet_years =
            store.latest_owned_planet_years_for_empire(self.player_record_index_1_based as u8)?;
        self.planet_intel_snapshots =
            store.latest_planet_intel_for_viewer(self.player_record_index_1_based as u8)?;
        self.player_war_stats = store
            .latest_player_war_stats(self.game_data.conquest.player_count())?
            .get(self.player_record_index_1_based.saturating_sub(1))
            .copied()
            .unwrap_or_else(|| {
                nc_data::PlayerWarStatsState::for_player(self.player_record_index_1_based)
            });
        Ok(())
    }
}

fn planet_intel_by_viewer(
    store: &CampaignStore,
    player_count: u8,
) -> Result<Vec<BTreeMap<usize, PlanetIntelSnapshot>>, Box<dyn std::error::Error>> {
    (1..=player_count)
        .map(|viewer_empire_id| {
            store
                .latest_planet_intel_for_viewer(viewer_empire_id)
                .map(|snapshots| {
                    snapshots
                        .into_iter()
                        .map(|snapshot| (snapshot.planet_record_index_1_based, snapshot))
                        .collect::<BTreeMap<_, _>>()
                })
                .map_err(|err| -> Box<dyn std::error::Error> { Box::new(err) })
        })
        .collect()
}
