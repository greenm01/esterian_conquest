use crossterm::event::{KeyCode, KeyEvent};
use ec_data::DiplomaticRelation;

use crate::app::Action;
use crate::screen::layout::{draw_command_prompt, draw_plain_prompt, draw_status_line, draw_title_bar, new_playfield};
use crate::screen::table::{TableColumn, write_table_window};
use crate::screen::{PlayfieldBuffer, ScreenFrame};
use crate::theme::classic;

pub struct EnemiesScreen;

const ENEMIES_COLUMNS: [TableColumn<'static>; 3] = [
    TableColumn::right("ID", 3),
    TableColumn::left("Empire", 26),
    TableColumn::left("Status", 8),
];

impl EnemiesScreen {
    pub fn new() -> Self {
        Self
    }

    pub fn render(
        &mut self,
        frame: &ScreenFrame<'_>,
        input: &str,
        status: Option<&str>,
        scroll_offset: usize,
    ) -> Result<PlayfieldBuffer, Box<dyn std::error::Error>> {
        let mut buffer = new_playfield();
        draw_title_bar(&mut buffer, 0, "ENEMIES, DECLARE OR LIST:");
        buffer.write_text(
            1,
            0,
            "Declare empires as enemies or restore them to neutral status.",
            classic::body_style(),
        );

        let viewer_empire = frame.player.record_index_1_based as u8;
        let mut others = frame
            .game_data
            .player
            .records
            .iter()
            .enumerate()
            .filter(|(idx, _)| *idx + 1 != frame.player.record_index_1_based)
            .map(|(idx, player)| {
                let empire_id = (idx + 1) as u8;
                let name = player.controlled_empire_name_summary();
                let fallback = player.legacy_status_name_summary();
                let display = if !name.is_empty() { name } else { fallback };
                let relation = frame
                    .game_data
                    .stored_diplomatic_relation(viewer_empire, empire_id)
                    .unwrap_or(DiplomaticRelation::Neutral);
                (empire_id, display, relation)
            })
            .collect::<Vec<_>>();
        others.sort_by_key(|(empire_id, _, _)| *empire_id);

        let rows = others
            .into_iter()
            .map(|(empire_id, name, relation)| {
                vec![
                    empire_id.to_string(),
                    name,
                    match relation {
                        DiplomaticRelation::Enemy => "ENEMY".to_string(),
                        DiplomaticRelation::Neutral => "NEUTRAL".to_string(),
                    },
                ]
            })
            .collect::<Vec<_>>();

        write_table_window(
            &mut buffer,
            3,
            &ENEMIES_COLUMNS,
            &rows,
            scroll_offset,
            11,
            classic::status_value_style(),
            classic::status_value_style(),
        );

        let prompt = format!("Enter empire number to toggle: {input}");
        let cursor_col = draw_plain_prompt(&mut buffer, 17, &prompt);
        if let Some(status) = status {
            draw_status_line(&mut buffer, 18, "", status);
        }
        draw_command_prompt(&mut buffer, 19, "GENERAL COMMAND", "ARROWS J K Q");
        buffer.set_cursor(cursor_col as u16, 17);
        Ok(buffer)
    }

    pub fn handle_key(&self, key: KeyEvent) -> Action {
        match key.code {
            KeyCode::Up | KeyCode::Char('k') | KeyCode::Char('K') => Action::ScrollEnemies(-1),
            KeyCode::Down | KeyCode::Char('j') | KeyCode::Char('J') => Action::ScrollEnemies(1),
            KeyCode::PageUp => Action::ScrollEnemies(-8),
            KeyCode::PageDown => Action::ScrollEnemies(8),
            KeyCode::Char('q') | KeyCode::Char('Q') | KeyCode::Esc => Action::OpenGeneralMenu,
            KeyCode::Enter => Action::SubmitEnemiesInput,
            KeyCode::Backspace => Action::BackspaceEnemiesInput,
            KeyCode::Char(ch) if ch.is_ascii_digit() => Action::AppendEnemiesChar(ch),
            _ => Action::Noop,
        }
    }
}
