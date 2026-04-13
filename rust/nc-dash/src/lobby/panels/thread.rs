use nc_ui::modal::Rect;

use crate::lobby::state::{LobbyFocus, LobbyState};
use crate::lobby::threads;
use crate::lobby::{draw_panel_frame, focus_selected, write_panel_rows};

pub fn render(
    buffer: &mut nc_ui::PlayfieldBuffer,
    rect: Rect,
    state: &LobbyState,
    focus: LobbyFocus,
) {
    draw_panel_frame(buffer, rect, "THREAD", focus == LobbyFocus::Thread);
    let rows = threads::thread_rows(state);
    write_panel_rows(
        buffer,
        rect,
        &rows,
        focus_selected(focus, LobbyFocus::Thread, state.thread_selected),
    );
}
