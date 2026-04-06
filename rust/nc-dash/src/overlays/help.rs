//! ? overlay: keyboard reference, centered on screen.

use nc_ui::modal::format_help_rows;
use nc_ui::PlayfieldBuffer;

use crate::app::state::DashApp;
use crate::overlays::frame::{draw_overlay_frame, write_clipped};
use crate::theme;

pub fn draw(buf: &mut PlayfieldBuffer, _app: &DashApp) {
    let help_lines = [
        String::from("GLOBAL HOTKEYS"),
        String::new(),
        format_help_rows([
            ("P / F / I / R", "Planet, Fleet, Intel, Inbox overlays"),
            ("D / S / ?", "Diplomacy, Settings, Help"),
            ("Tab / Shift+Tab", "Cycle dashboard focus"),
            ("Esc / Q", "Close overlay or quit dashboard"),
        ])
        .join("\n"),
        String::new(),
        String::from("MAP AND LISTS"),
        String::new(),
        format_help_rows([
            ("J/K  Up/Down", "Move selection or crosshair"),
            ("^U /^D PgUp/PgDn", "Page through long lists"),
            ("Home / End", "Jump to start or end"),
            ("Tab", "Switch inbox list/preview focus"),
            ("M / R / A / Y", "Inbox filters and year scope"),
        ])
        .join("\n"),
        String::new(),
        String::from("Press ? or Esc to close."),
    ];

    let lines = help_lines
        .iter()
        .flat_map(|block| block.lines().map(str::to_string).collect::<Vec<_>>())
        .collect::<Vec<_>>();
    let frame = draw_overlay_frame(
        buf,
        "HELP",
        76,
        lines.len() + 5,
        "COMMAND <- ? / Esc to close ->",
    );

    for (idx, line) in lines.iter().enumerate().take(frame.body_height) {
        let style = if line.chars().all(|ch| !ch.is_lowercase()) && !line.is_empty() {
            theme::section_title_style()
        } else {
            theme::label_style()
        };
        write_clipped(buf, frame.body_row + idx, frame.body_col, frame.body_width, line, style);
    }
}
