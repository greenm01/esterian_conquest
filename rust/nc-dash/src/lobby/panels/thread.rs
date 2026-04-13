use nc_ui::modal::Rect;

use crate::lobby::state::{LobbyFocus, LobbyState};
use crate::lobby::threads;
use crate::lobby::{draw_panel_frame, write_panel_rows};

pub fn render(
    buffer: &mut nc_ui::PlayfieldBuffer,
    rect: Rect,
    state: &LobbyState,
    focus: LobbyFocus,
) {
    draw_panel_frame(buffer, rect, "THREADS", focus == LobbyFocus::Thread);
    let rows = threads::direct_thread_render_lines(state, 72)
        .into_iter()
        .map(|line| {
            let mut row = String::new();
            if let Some(timestamp) = line.timestamp {
                row.push_str(&timestamp);
            } else if line.indent > 0 {
                row.push_str(&" ".repeat(line.indent));
            }
            if let Some(nick) = line.nick {
                row.push_str(&nick);
            }
            row.push_str(&line.body);
            row
        })
        .collect::<Vec<_>>();
    write_panel_rows(
        buffer,
        rect,
        &rows,
        None,
    );
}
