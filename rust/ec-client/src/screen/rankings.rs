use crossterm::event::{KeyCode, KeyEvent};

use crate::app::Action;
use crate::screen::layout::{draw_command_prompt, draw_title_bar, new_playfield};
use crate::screen::table::{
    format_empire_id, table_divider, write_table_header, write_table_row, TableColumn,
};
use crate::screen::{PlayfieldBuffer, ScreenFrame};
use crate::theme::classic;
use ec_data::{DiplomaticRelation, EmpireProductionRankingSort};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RankingsView {
    Prompt,
    Table(EmpireProductionRankingSort),
}

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

    pub fn render_prompt(
        &mut self,
        _frame: &ScreenFrame<'_>,
    ) -> Result<PlayfieldBuffer, Box<dyn std::error::Error>> {
        let mut buffer = new_playfield();
        draw_title_bar(&mut buffer, 0, "OTHER EMPIRES (RANKINGS):");
        buffer.write_text(
            2,
            0,
            "Rank empires by <I>D, <P>roduction, <N>umber of planets or <A>bort? [I] ->",
            classic::prompt_style(),
        );
        buffer.set_cursor(76, 2);
        Ok(buffer)
    }

    pub fn render_table(
        &mut self,
        frame: &ScreenFrame<'_>,
        sort: EmpireProductionRankingSort,
    ) -> Result<PlayfieldBuffer, Box<dyn std::error::Error>> {
        let viewer_idx = frame.player.record_index_1_based;
        let viewer = &frame.game_data.player.records[viewer_idx - 1];
        let rows = frame.game_data.empire_production_ranking_rows(sort);

        let mut buffer = new_playfield();
        draw_title_bar(&mut buffer, 0, "OTHER EMPIRES (RANKINGS):");
        write_table_header(
            &mut buffer,
            2,
            &RANKINGS_COLUMNS,
            classic::status_value_style(),
        );
        buffer.write_text(
            3,
            0,
            &table_divider(&RANKINGS_COLUMNS),
            classic::menu_style(),
        );

        for (row_idx, row) in rows.into_iter().take(12).enumerate() {
            let status = if row.empire_id as usize == viewer_idx {
                "*".to_string()
            } else {
                diplomacy_status(viewer.diplomatic_relation_toward(row.empire_id)).to_string()
            };
            let empire_id = format_empire_id(row.empire_id);
            let planets_owned = row.planets_owned.to_string();
            let current_production = row.current_production.to_string();
            write_table_row(
                &mut buffer,
                row_idx + 4,
                &RANKINGS_COLUMNS,
                &[
                    &row.empire_name,
                    &empire_id,
                    &planets_owned,
                    &current_production,
                    &status,
                ],
                classic::status_value_style(),
            );
        }

        draw_command_prompt(&mut buffer, 19, "GENERAL COMMAND", "SLAP A KEY");
        Ok(buffer)
    }

    pub fn handle_prompt_key(&self, key: KeyEvent) -> Action {
        match key.code {
            KeyCode::Char('i') | KeyCode::Char('I') | KeyCode::Enter => {
                Action::OpenRankingsTable(EmpireProductionRankingSort::Id)
            }
            KeyCode::Char('p') | KeyCode::Char('P') => {
                Action::OpenRankingsTable(EmpireProductionRankingSort::Production)
            }
            KeyCode::Char('n') | KeyCode::Char('N') => {
                Action::OpenRankingsTable(EmpireProductionRankingSort::NumberOfPlanets)
            }
            KeyCode::Char('a') | KeyCode::Char('A') | KeyCode::Esc => Action::OpenGeneralMenu,
            _ => Action::Noop,
        }
    }

    pub fn handle_table_key(&self, _key: KeyEvent) -> Action {
        Action::OpenGeneralMenu
    }
}

fn diplomacy_status(relation: Option<DiplomaticRelation>) -> &'static str {
    match relation {
        Some(DiplomaticRelation::Enemy) => "ENEMY",
        Some(DiplomaticRelation::Neutral) => "Neutral",
        None => "Neutral",
    }
}
