use nc_ui::modal::Rect;

use crate::lobby::{draw_panel_frame, focus_selected, write_panel_rows};
use crate::lobby::state::{LobbyFocus, LobbyState};

pub fn render(
    buffer: &mut nc_ui::PlayfieldBuffer,
    rect: Rect,
    state: &LobbyState,
    focus: LobbyFocus,
) {
    draw_panel_frame(buffer, rect, "OPEN GAMES", focus == LobbyFocus::OpenGames);
    let rows = state
        .open_games
        .iter()
        .map(|row| {
            format!(
                "{} | {} | {} | {} seats | {}",
                row.game, row.host, row.recruiting, row.open_seats, row.turn_summary
            )
        })
        .collect::<Vec<_>>();
    write_panel_rows(
        buffer,
        rect,
        &rows,
        focus_selected(focus, LobbyFocus::OpenGames, state.open_selected),
    );
}
