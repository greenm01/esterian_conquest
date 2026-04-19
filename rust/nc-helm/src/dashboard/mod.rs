pub mod app;
pub mod buffer;
pub mod client_settings;
pub mod coords;
pub mod diplomacy_view;
pub mod geometry;
pub mod inbox;
pub mod input;
pub mod launch;
pub mod layout;
pub mod modal;
pub mod native;
pub mod overlays;
pub mod panels;
pub mod planet_view;
pub mod popups;
pub mod prompt;
pub mod table;
pub mod table_filter;
pub mod table_layout;
pub mod table_selection;
pub mod theme;
pub mod ui;

pub use app::state::DashApp;
pub use geometry::ScreenGeometry;
pub use launch::DashLaunchState;

use nc_data::TurnSubmission;
use nc_nostr::state_sync::GameState;
use std::io;

use self::native::NativeApp;

pub fn build_hosted_dash_app(
    snapshot: &GameState,
    geometry: ScreenGeometry,
) -> Result<DashApp, Box<dyn std::error::Error>> {
    DashLaunchState::from_hosted_snapshot(snapshot)?.into_app(geometry)
}

pub fn replay_hosted_draft(
    dashboard: &mut DashApp,
    draft: &TurnSubmission,
) -> Result<(), nc_data::TurnSubmissionError> {
    draft.apply_to(&mut dashboard.game_data, &mut dashboard.queued_mail)?;
    dashboard.hosted_turn_draft = Some(draft.clone());
    Ok(())
}

pub fn resize_hosted_app(dashboard: &mut DashApp, cols: usize, rows: usize) {
    NativeApp::resize_canvas(dashboard, cols as u16, rows as u16);
}

pub fn dispatch_hosted_key(dashboard: &mut DashApp, key: input::KeyEvent) {
    NativeApp::dispatch_key_event(dashboard, key);
}

pub fn dispatch_hosted_mouse(dashboard: &mut DashApp, mouse: input::MouseEvent) -> bool {
    NativeApp::dispatch_mouse_event(dashboard, mouse)
}

pub fn hosted_should_quit(dashboard: &DashApp) -> bool {
    NativeApp::should_quit(dashboard)
}

pub fn hosted_wants_text_input(dashboard: &DashApp) -> bool {
    NativeApp::wants_text_input(dashboard)
}

pub fn hosted_wants_window_focus(dashboard: &DashApp) -> bool {
    NativeApp::wants_window_focus(dashboard)
}

pub fn render_hosted_buffer(
    dashboard: &DashApp,
) -> Result<crate::PlayfieldBuffer, Box<dyn std::error::Error>> {
    let Some(playfield) = NativeApp::render_scene(dashboard)?.into_playfield() else {
        return Err(Box::new(io::Error::other(
            "hosted dashboard returned a non-playfield scene",
        )));
    };

    let base_style = if playfield.width() > 0 && playfield.height() > 0 {
        convert_style(playfield.row(0)[0].style)
    } else {
        crate::CellStyle::new(crate::GameColor::Black, crate::GameColor::Black, false)
    };
    let mut buffer = crate::PlayfieldBuffer::new(playfield.width(), playfield.height(), base_style);
    for row in 0..playfield.height() {
        for (col, cell) in playfield.row(row).iter().copied().enumerate() {
            buffer.set_cell(row, col, cell.ch, convert_style(cell.style));
        }
    }
    if let Some((column, row)) = playfield.cursor() {
        buffer.set_cursor(crate::Point::from_usize(column as usize, row as usize));
    }
    Ok(buffer)
}

fn convert_style(style: buffer::CellStyle) -> crate::CellStyle {
    crate::CellStyle::new(convert_color(style.fg), convert_color(style.bg), style.bold)
}

fn convert_color(color: buffer::GameColor) -> crate::GameColor {
    match color {
        buffer::GameColor::Black => crate::GameColor::Black,
        buffer::GameColor::Red => crate::GameColor::Red,
        buffer::GameColor::Green => crate::GameColor::Green,
        buffer::GameColor::Yellow => crate::GameColor::Yellow,
        buffer::GameColor::Blue => crate::GameColor::Blue,
        buffer::GameColor::Magenta => crate::GameColor::Magenta,
        buffer::GameColor::Cyan => crate::GameColor::Cyan,
        buffer::GameColor::White => crate::GameColor::White,
        buffer::GameColor::BrightBlack => crate::GameColor::BrightBlack,
        buffer::GameColor::BrightRed => crate::GameColor::BrightRed,
        buffer::GameColor::BrightGreen => crate::GameColor::BrightGreen,
        buffer::GameColor::BrightYellow => crate::GameColor::BrightYellow,
        buffer::GameColor::BrightBlue => crate::GameColor::BrightBlue,
        buffer::GameColor::BrightMagenta => crate::GameColor::BrightMagenta,
        buffer::GameColor::BrightCyan => crate::GameColor::BrightCyan,
        buffer::GameColor::BrightWhite => crate::GameColor::BrightWhite,
        buffer::GameColor::Indexed(value) => crate::GameColor::Indexed(value),
        buffer::GameColor::Rgb(red, green, blue) => crate::GameColor::Rgb(red, green, blue),
    }
}
