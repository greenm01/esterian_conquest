use crossterm::event::KeyEvent;

use crate::app::Action;
use crate::screen::layout::{draw_dismiss_prompt, draw_title_bar, new_playfield};
use crate::screen::table::{TableColumn, format_empire_id, write_table_window};
use crate::screen::{CommandMenu, PlayfieldBuffer, ScreenFrame};
use crate::theme::classic;
use ec_data::{DiplomaticRelation, EmpireProductionRankingSort};

pub struct RankingsScreen;

const RANKINGS_COLUMNS: [TableColumn<'static>; 5] = [
    TableColumn::left("Empire Name", 23),
    TableColumn::right("ID", 4),
    TableColumn::right("Planets", 11),
    TableColumn::right("Production", 16),
    TableColumn::right("Status", 12),
];

impl RankingsScreen {
    pub fn new() -> Self {
        Self
    }

    pub fn render_table(
        &mut self,
        frame: &ScreenFrame<'_>,
        sort: EmpireProductionRankingSort,
        menu: CommandMenu,
    ) -> Result<PlayfieldBuffer, Box<dyn std::error::Error>> {
        let viewer_idx = frame.player.record_index_1_based;
        let viewer = &frame.game_data.player.records[viewer_idx - 1];
        let rows = frame.game_data.empire_production_ranking_rows(sort);
        let table_rows = rows
            .into_iter()
            .take(12)
            .map(|row| {
                let status = if row.empire_id as usize == viewer_idx {
                    "*".to_string()
                } else {
                    diplomacy_status(viewer.diplomatic_relation_toward(row.empire_id)).to_string()
                };
                vec![
                    row.empire_name,
                    format_empire_id(row.empire_id),
                    row.planets_owned.to_string(),
                    row.current_production.to_string(),
                    status,
                ]
            })
            .collect::<Vec<_>>();

        let mut buffer = new_playfield();
        draw_title_bar(&mut buffer, 0, "OTHER EMPIRES (RANKINGS):");
        write_table_window(
            &mut buffer,
            2,
            &RANKINGS_COLUMNS,
            &table_rows,
            0,
            table_rows.len(),
            classic::status_value_style(),
            classic::status_value_style(),
        );

        let _ = menu;
        draw_dismiss_prompt(&mut buffer, 19);
        Ok(buffer)
    }
    pub fn handle_key(&self, _key: KeyEvent) -> Action {
        Action::ReturnToCommandMenu
    }
}

fn diplomacy_status(relation: Option<DiplomaticRelation>) -> &'static str {
    match relation {
        Some(DiplomaticRelation::Enemy) => "ENEMY",
        Some(DiplomaticRelation::Neutral) => "Neutral",
        None => "Neutral",
    }
}
