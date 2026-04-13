use nc_ui::modal::Rect;

use crate::lobby::state::{LobbyFocus, LobbyState};
use crate::lobby::{draw_panel_frame, focus_selected, write_panel_rows};

pub fn render(
    buffer: &mut nc_ui::PlayfieldBuffer,
    rect: Rect,
    state: &LobbyState,
    focus: LobbyFocus,
) {
    draw_panel_frame(
        buffer,
        rect,
        "JOINED GAMES",
        focus == LobbyFocus::JoinedGames,
    );
    let rows = state
        .joined_games
        .iter()
        .map(|row| {
            let seat = row
                .seat
                .map(|seat| format!("seat {seat}"))
                .unwrap_or_else(|| "no seat".to_string());
            format!(
                "{} | {} | {} | {}",
                row.status, row.game, seat, row.turn_summary
            )
        })
        .collect::<Vec<_>>();
    write_panel_rows(
        buffer,
        rect,
        &rows,
        focus_selected(focus, LobbyFocus::JoinedGames, state.joined_selected),
    );
}
