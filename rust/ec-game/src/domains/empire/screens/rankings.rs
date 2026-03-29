use crossterm::event::KeyEvent;

use crate::app::Action;
use crate::screen::layout::new_playfield;
use crate::screen::table::{
    HorizontalAlign, LayoutRect, TableColumn, TableFooter, TableWidthMode, VerticalAlign,
    draw_table_footer, draw_table_title, format_empire_id, layout_standard_table_block,
    resolve_table_columns, write_table_window_with_states_at,
};
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
        let columns = resolve_table_columns(
            &RANKINGS_COLUMNS,
            &table_rows,
            buffer.width(),
            false,
            TableWidthMode::Compact,
        );
        let layout = layout_standard_table_block(
            LayoutRect::new(0, 0, buffer.width(), buffer.height()),
            &columns,
            table_rows.len(),
            Some("OTHER EMPIRES (RANKINGS):"),
            Some(TableFooter::Dismiss),
            false,
            HorizontalAlign::Center,
            VerticalAlign::Center,
        );
        let _ = layout.title_row;
        draw_table_title(
            &mut buffer,
            layout.table_row,
            layout.table_col,
            "OTHER EMPIRES (RANKINGS):",
        );
        let metrics = write_table_window_with_states_at(
            &mut buffer,
            layout.table_row,
            layout.table_col,
            &columns,
            &table_rows,
            0,
            table_rows.len(),
            classic::status_value_style(),
            classic::status_value_style(),
            None,
            0,
            None,
        );

        let _ = menu;
        draw_table_footer(
            &mut buffer,
            frame.geometry,
            layout.command_col,
            metrics.bottom_row,
            TableFooter::Dismiss,
        );
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
