use nc_ui::modal::Rect;

use crate::lobby::{draw_panel_frame, focus_selected, write_panel_rows};
use crate::lobby::state::{LobbyFocus, LobbyState};
use crate::lobby::threads;

pub fn render(
    buffer: &mut nc_ui::PlayfieldBuffer,
    rect: Rect,
    state: &LobbyState,
    focus: LobbyFocus,
) {
    draw_panel_frame(buffer, rect, "NOTICES", focus == LobbyFocus::Notices);
    let rows = threads::notice_rows(state);
    write_panel_rows(
        buffer,
        rect,
        &rows,
        focus_selected(focus, LobbyFocus::Notices, state.notices_selected),
    );
}
