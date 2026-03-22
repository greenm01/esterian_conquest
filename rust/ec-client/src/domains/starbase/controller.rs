use crate::app::helpers::{center_scroll_to_cursor, sync_scroll_to_cursor};
use crate::app::state::App;
use crate::domains::starbase::StarbaseAction;
use crate::screen::{CommandMenu, ScreenId, StarbaseRow};

impl App {
    pub fn show_starbase_expert_mode_notice(&mut self) {
        self.show_command_menu_notice(
            CommandMenu::Starbase,
            "Expert mode not implemented yet. Plan for Helix style commands.",
        );
    }

    pub fn show_starbase_move_notice(&mut self) {
        self.show_command_menu_notice(
            CommandMenu::Starbase,
            "Starbase hauling is not implemented yet.",
        );
    }

    pub fn open_starbase_menu(&mut self) {
        self.clear_command_menu_notice();
        self.current_screen = ScreenId::StarbaseMenu;
    }

    pub fn open_starbase_help(&mut self) {
        self.clear_command_menu_notice();
        self.current_screen = ScreenId::StarbaseHelp;
    }

    pub fn open_starbase_list(&mut self) {
        let total = self.starbase_rows().len();
        if total == 0 {
            self.show_command_menu_notice(CommandMenu::Starbase, "You have no active starbases.");
            return;
        }
        self.clear_command_menu_notice();
        self.starbase.cursor = self.starbase.cursor.min(total - 1);
        center_scroll_to_cursor(
            &mut self.starbase.scroll_offset,
            self.starbase.cursor,
            crate::screen::STARBASE_VISIBLE_ROWS,
            total,
        );
        self.current_screen = ScreenId::StarbaseList;
    }

    pub fn open_starbase_review_select(&mut self) {
        let total = self.starbase_rows().len();
        if total == 0 {
            self.show_command_menu_notice(CommandMenu::Starbase, "You have no active starbases.");
            return;
        }
        self.clear_command_menu_notice();
        self.starbase.cursor = self.starbase.cursor.min(total - 1);
        self.starbase.review_input.clear();
        self.starbase.review_status = None;
        center_scroll_to_cursor(
            &mut self.starbase.scroll_offset,
            self.starbase.cursor,
            crate::screen::STARBASE_VISIBLE_ROWS,
            total,
        );
        self.current_screen = ScreenId::StarbaseReviewSelect;
    }

    pub fn move_starbase_select(&mut self, delta: i8) {
        if !matches!(
            self.current_screen,
            ScreenId::StarbaseList | ScreenId::StarbaseReviewSelect
        ) {
            return;
        }
        self.starbase
            .move_select(delta, &self.game_data, self.player.record_index_1_based);
    }

    pub fn append_starbase_char(&mut self, ch: char) {
        if self.current_screen != ScreenId::StarbaseReviewSelect || !ch.is_ascii_digit() {
            return;
        }
        if self.starbase.review_input.len() >= 3 {
            return;
        }
        self.starbase
            .append_char(ch, &self.game_data, self.player.record_index_1_based);
    }

    pub fn backspace_starbase_input(&mut self) {
        if self.current_screen != ScreenId::StarbaseReviewSelect {
            return;
        }
        self.starbase
            .backspace_input(&self.game_data, self.player.record_index_1_based);
    }

    pub fn submit_starbase_review_select(&mut self) {
        if self.current_screen != ScreenId::StarbaseReviewSelect {
            return;
        }
        let rows = self.starbase_rows();
        let Some(_) = rows.get(self.starbase.cursor) else {
            self.current_screen = ScreenId::StarbaseMenu;
            return;
        };
        if !self.starbase.review_input.trim().is_empty() {
            let target_base_id = match self.starbase.review_input.trim().parse::<u8>() {
                Ok(value) => value,
                Err(_) => {
                    self.starbase.review_status =
                        Some("Enter a starbase number from the table.".to_string());
                    return;
                }
            };
            let Some(index) = rows.iter().position(|row| row.base_id == target_base_id) else {
                self.starbase.review_status =
                    Some(format!("Starbase #{target_base_id} is not in your list."));
                return;
            };
            self.starbase.cursor = index;
            sync_scroll_to_cursor(
                &mut self.starbase.scroll_offset,
                self.starbase.cursor,
                crate::screen::STARBASE_VISIBLE_ROWS,
            );
        }
        self.starbase.review_input.clear();
        self.starbase.review_status = None;
        self.open_starbase_review();
    }

    pub(crate) fn starbase_rows(&self) -> Vec<StarbaseRow> {
        self.starbase
            .starbase_rows(&self.game_data, self.player.record_index_1_based)
    }

    pub(crate) fn handle_starbase_review_select_key(
        &self,
        key: crossterm::event::KeyEvent,
    ) -> crate::app::Action {
        use crossterm::event::KeyCode;

        match key.code {
            KeyCode::Up | KeyCode::Char('k') | KeyCode::Char('K') => {
                crate::app::Action::Starbase(StarbaseAction::MoveSelect(-1))
            }
            KeyCode::Down | KeyCode::Char('j') | KeyCode::Char('J') => {
                crate::app::Action::Starbase(StarbaseAction::MoveSelect(1))
            }
            KeyCode::PageUp => crate::app::Action::Starbase(StarbaseAction::MoveSelect(-8)),
            KeyCode::PageDown => crate::app::Action::Starbase(StarbaseAction::MoveSelect(8)),
            KeyCode::Enter => crate::app::Action::Starbase(StarbaseAction::SubmitReviewSelect),
            KeyCode::Backspace => crate::app::Action::Starbase(StarbaseAction::BackspaceInput),
            KeyCode::Char(ch) if ch.is_ascii_digit() => {
                crate::app::Action::Starbase(StarbaseAction::AppendChar(ch))
            }
            KeyCode::Char('q') | KeyCode::Char('Q') | KeyCode::Esc => {
                crate::app::Action::Starbase(StarbaseAction::OpenMenu)
            }
            _ => crate::app::Action::Noop,
        }
    }

    pub fn open_starbase_review(&mut self) {
        let rows = self.starbase_rows();
        if rows.is_empty() {
            self.show_command_menu_notice(CommandMenu::Starbase, "You have no active starbases.");
            return;
        }
        let Some(_) = rows.get(self.starbase.cursor) else {
            self.current_screen = ScreenId::StarbaseMenu;
            return;
        };
        self.starbase.review_index = self.starbase.cursor;
        self.current_screen = ScreenId::StarbaseReview;
    }
}
