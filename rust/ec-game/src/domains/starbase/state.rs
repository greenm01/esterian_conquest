use crate::domains::starbase::screens::starbase::{STARBASE_VISIBLE_ROWS, StarbaseRow};
use ec_data::CoreGameData;

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct StarbaseState {
    pub scroll_offset: usize,
    pub cursor: usize,
    pub review_index: usize,
    pub review_input: String,
    pub review_status: Option<String>,
}

impl StarbaseState {
    pub fn starbase_rows(
        &self,
        game_data: &CoreGameData,
        player_record_index_1_based: usize,
    ) -> Vec<StarbaseRow> {
        let mut rows = game_data
            .bases
            .records
            .iter()
            .enumerate()
            .filter(|(_, base)| base.owner_empire_raw() as usize == player_record_index_1_based)
            .map(|(idx, base)| {
                let escort_label = game_data
                    .fleets
                    .records
                    .iter()
                    .find(|fleet| fleet.fleet_id_word_raw() == base.chain_word_raw())
                    .map(|fleet| {
                        format!(
                            "The {} Fleet",
                            ordinal_number(fleet.local_slot_word_raw() as usize)
                        )
                    })
                    .unwrap_or_else(|| "Unknown escort".to_string());
                let destination_coords = base.trailing_coords_raw();
                let operation_label = if destination_coords == base.coords_raw() {
                    "Protection & Enhancement".to_string()
                } else {
                    "Starbase in transit".to_string()
                };
                let eta_label = if destination_coords == base.coords_raw() {
                    "0".to_string()
                } else {
                    "?".to_string()
                };
                StarbaseRow {
                    base_record_index_1_based: idx + 1,
                    base_id: base.base_id_raw(),
                    escort_label,
                    coords: base.coords_raw(),
                    destination_coords,
                    eta_label,
                    operation_label,
                }
            })
            .collect::<Vec<_>>();
        rows.sort_by_key(|row| row.base_id);
        rows
    }

    pub fn move_select(&mut self, delta: i8, game_data: &CoreGameData, player_idx: usize) {
        let total = self.starbase_rows(game_data, player_idx).len();
        if total == 0 {
            return;
        }
        let max_idx = total - 1;
        self.cursor = self
            .cursor
            .saturating_add_signed(delta as isize)
            .min(max_idx);
        self.sync_scroll(total);
    }

    pub fn append_char(&mut self, ch: char, game_data: &CoreGameData, player_idx: usize) {
        self.review_input.push(ch);
        self.sync_cursor_to_input(game_data, player_idx);
    }

    pub fn backspace_input(&mut self, game_data: &CoreGameData, player_idx: usize) {
        self.review_input.pop();
        self.sync_cursor_to_input(game_data, player_idx);
    }

    fn sync_cursor_to_input(&mut self, game_data: &CoreGameData, player_idx: usize) {
        if self.review_input.trim().is_empty() {
            return;
        }
        let Ok(target_base_id) = self.review_input.trim().parse::<u8>() else {
            return;
        };
        let rows = self.starbase_rows(game_data, player_idx);
        if let Some(index) = rows.iter().position(|row| row.base_id == target_base_id) {
            self.cursor = index;
            self.sync_scroll(rows.len());
        }
    }

    pub fn sync_scroll(&mut self, total: usize) {
        if total <= STARBASE_VISIBLE_ROWS {
            self.scroll_offset = 0;
            return;
        }
        let half = STARBASE_VISIBLE_ROWS / 2;
        let max_offset = total - STARBASE_VISIBLE_ROWS;
        self.scroll_offset = self.cursor.saturating_sub(half).min(max_offset);
    }
}

fn ordinal_number(value: usize) -> String {
    let suffix = match value % 100 {
        11..=13 => "th",
        _ => match value % 10 {
            1 => "st",
            2 => "nd",
            3 => "rd",
            _ => "th",
        },
    };
    format!("{value}{suffix}")
}
