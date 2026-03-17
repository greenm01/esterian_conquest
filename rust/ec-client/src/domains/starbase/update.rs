use crate::app::state::App;
use crate::domains::starbase::StarbaseAction;
use crate::screen::{ScreenId, CommandMenu, STARBASE_VISIBLE_ROWS};

pub fn update(app: &mut App, action: StarbaseAction) {
    match action {
        StarbaseAction::OpenMenu => open_menu(app),
        StarbaseAction::OpenHelp => open_help(app),
        StarbaseAction::OpenList => open_list(app),
        StarbaseAction::OpenReviewSelect => open_review_select(app),
        StarbaseAction::OpenReview => open_review(app),
        StarbaseAction::ShowExpertModeNotice => show_expert_mode_notice(app),
        StarbaseAction::ShowMoveNotice => show_move_notice(app),
        StarbaseAction::MoveSelect(delta) => {
            let player_idx = app.player.record_index_1_based;
            app.starbase.move_select(delta, &app.game_data, player_idx);
        }
        StarbaseAction::AppendChar(ch) => {
            if app.current_screen != ScreenId::StarbaseReviewSelect {
                return;
            }
            let player_idx = app.player.record_index_1_based;
            app.starbase.append_char(ch, &app.game_data, player_idx);
        }
        StarbaseAction::BackspaceInput => {
            if app.current_screen != ScreenId::StarbaseReviewSelect {
                return;
            }
            let player_idx = app.player.record_index_1_based;
            app.starbase.backspace_input(&app.game_data, player_idx);
        }
        StarbaseAction::SubmitReviewSelect => submit_review_select(app),
    }
}

fn open_menu(app: &mut App) {
    app.clear_command_menu_notice();
    app.current_screen = ScreenId::StarbaseMenu;
}

fn open_help(app: &mut App) {
    app.clear_command_menu_notice();
    app.current_screen = ScreenId::StarbaseHelp;
}

fn open_list(app: &mut App) {
    let player_idx = app.player.record_index_1_based;
    let total = app.starbase.starbase_rows(&app.game_data, player_idx).len();
    if total == 0 {
        app.show_command_menu_notice(CommandMenu::Starbase, "You have no active starbases.");
        return;
    }
    app.clear_command_menu_notice();
    app.starbase.cursor = app.starbase.cursor.min(total - 1);
    center_scroll_to_cursor(
        &mut app.starbase.scroll_offset,
        app.starbase.cursor,
        STARBASE_VISIBLE_ROWS,
        total,
    );
    app.current_screen = ScreenId::StarbaseList;
}

fn open_review_select(app: &mut App) {
    let player_idx = app.player.record_index_1_based;
    let total = app.starbase.starbase_rows(&app.game_data, player_idx).len();
    if total == 0 {
        app.show_command_menu_notice(CommandMenu::Starbase, "You have no active starbases.");
        return;
    }
    app.clear_command_menu_notice();
    app.starbase.cursor = app.starbase.cursor.min(total - 1);
    app.starbase.review_input.clear();
    app.starbase.review_status = None;
    center_scroll_to_cursor(
        &mut app.starbase.scroll_offset,
        app.starbase.cursor,
        STARBASE_VISIBLE_ROWS,
        total,
    );
    app.current_screen = ScreenId::StarbaseReviewSelect;
}

fn open_review(app: &mut App) {
    let player_idx = app.player.record_index_1_based;
    let rows = app.starbase.starbase_rows(&app.game_data, player_idx);
    if rows.is_empty() {
        app.show_command_menu_notice(CommandMenu::Starbase, "You have no active starbases.");
        return;
    }
    let Some(_) = rows.get(app.starbase.cursor) else {
        app.current_screen = ScreenId::StarbaseMenu;
        return;
    };
    app.starbase.review_index = app.starbase.cursor;
    app.current_screen = ScreenId::StarbaseReview;
}

fn show_expert_mode_notice(app: &mut App) {
    app.show_command_menu_notice(
        CommandMenu::Starbase,
        "Expert mode not implemented yet. Plan for Helix style commands.",
    );
}

fn show_move_notice(app: &mut App) {
    app.show_command_menu_notice(
        CommandMenu::Starbase,
        "Starbase hauling is not implemented yet.",
    );
}

fn submit_review_select(app: &mut App) {
    if app.current_screen != ScreenId::StarbaseReviewSelect {
        return;
    }
    let player_idx = app.player.record_index_1_based;
    let rows = app.starbase.starbase_rows(&app.game_data, player_idx);
    let Some(_) = rows.get(app.starbase.cursor) else {
        app.current_screen = ScreenId::StarbaseMenu;
        return;
    };
    if !app.starbase.review_input.trim().is_empty() {
        let target_base_id = match app.starbase.review_input.trim().parse::<u8>() {
            Ok(value) => value,
            Err(_) => {
                app.starbase.review_status =
                    Some("Enter a starbase number from the table.".to_string());
                return;
            }
        };
        let Some(index) = rows.iter().position(|row| row.base_id == target_base_id) else {
            app.starbase.review_status =
                Some(format!("Starbase #{target_base_id} is not in your list."));
            return;
        };
        app.starbase.cursor = index;
        let total = rows.len();
        app.starbase.sync_scroll(total);
    }
    app.starbase.review_input.clear();
    app.starbase.review_status = None;
    open_review(app);
}

fn center_scroll_to_cursor(scroll_offset: &mut usize, cursor: usize, visible: usize, total: usize) {
    if total <= visible {
        *scroll_offset = 0;
        return;
    }
    let half = visible / 2;
    let max_offset = total - visible;
    *scroll_offset = cursor.saturating_sub(half).min(max_offset);
}
