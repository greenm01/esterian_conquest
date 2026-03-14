use ec_data::CoreGameData;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlayerContext {
    pub record_index_1_based: usize,
    pub empire_name: String,
    pub handle: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MainMenuSummary {
    pub game_year: u16,
    pub player_count: usize,
    pub owned_planets: usize,
    pub owned_fleets: usize,
    pub pending_messages: bool,
    pub pending_results: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GeneralMenuSummary {
    pub pending_messages: bool,
    pub pending_results: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReviewSummary {
    pub reviewable_messages: bool,
    pub reviewable_results: bool,
}

impl PlayerContext {
    pub fn from_game_data(
        game_data: &CoreGameData,
        record_index_1_based: usize,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let record = game_data
            .player
            .records
            .get(record_index_1_based - 1)
            .ok_or_else(|| format!("PLAYER.DAT missing record {record_index_1_based}"))?;
        let empire_name = record.controlled_empire_name_summary();
        let handle = record.assigned_player_handle_summary();
        Ok(Self {
            record_index_1_based,
            empire_name,
            handle,
        })
    }
}

impl MainMenuSummary {
    pub fn from_game_data(
        game_data: &CoreGameData,
        player_record_index_1_based: usize,
        pending_results: bool,
    ) -> Self {
        let owned_planets = game_data
            .planets
            .records
            .iter()
            .filter(|planet| planet.owner_empire_slot_raw() as usize == player_record_index_1_based)
            .count();
        let owned_fleets = game_data
            .fleets
            .records
            .iter()
            .filter(|fleet| fleet.owner_empire_raw() as usize == player_record_index_1_based)
            .count();

        let player_record = game_data
            .player
            .records
            .get(player_record_index_1_based - 1);
        let pending_messages = player_record
            .map(|record| record.raw[0x30] != 0 || record.raw[0x34] != 0)
            .unwrap_or(false);

        Self {
            game_year: game_data.conquest.game_year(),
            player_count: game_data.player.records.len(),
            owned_planets,
            owned_fleets,
            pending_messages,
            pending_results,
        }
    }
}

impl GeneralMenuSummary {
    pub fn from_main_menu(summary: &MainMenuSummary) -> Self {
        Self {
            pending_messages: summary.pending_messages,
            pending_results: summary.pending_results,
        }
    }
}

impl ReviewSummary {
    pub fn from_main_menu(summary: &MainMenuSummary) -> Self {
        Self {
            reviewable_messages: summary.pending_messages,
            reviewable_results: summary.pending_results,
        }
    }
}
