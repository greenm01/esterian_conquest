use nc_ui::modal::Rect;

use crate::lobby::state::{LobbyFocus, LobbyState};
use crate::lobby::{draw_panel_frame, focus_selected, write_panel_rows};

pub fn render(
    buffer: &mut nc_ui::PlayfieldBuffer,
    rect: Rect,
    state: &LobbyState,
    focus: LobbyFocus,
) {
    draw_panel_frame(buffer, rect, "GAME INBOX", focus == LobbyFocus::Inbox);
    let rows = state
        .filtered_game_inbox()
        .iter()
        .map(|item| {
            format!("{} | {} | {}", item.game, item.other_empire_name, item.preview)
        })
        .collect::<Vec<_>>();
    write_panel_rows(
        buffer,
        rect,
        &rows,
        focus_selected(focus, LobbyFocus::Inbox, state.inbox_selected),
    );
}
