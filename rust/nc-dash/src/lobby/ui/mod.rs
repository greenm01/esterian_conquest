mod chrome;
mod home;
mod layout;
mod popups;
mod tables;

pub use self::chrome::{panel_block, truncate_title, with_panel_bg, write_text};
pub use self::layout::{
    active_popup_rect, contains, hit_test_home, home_layout, home_tab_content_area, padded_inner,
    popup_title_bar_contains, scroll_offset,
};
pub(crate) use self::home::hit_test_tabs;

use nc_ui::PlayfieldBuffer;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::Modifier;

use crate::lobby::state::{LobbyApp, LobbyRoute};
use crate::lobby::threads;
use crate::theme;

pub fn render_scene(playfield: &mut PlayfieldBuffer, app: &LobbyApp) {
    let width = playfield.width() as u16;
    let height = playfield.height() as u16;
    let area = Rect::new(0, 0, width, height);
    let mut buffer = Buffer::empty(area);

    let Some(layout) = home_layout(area) else {
        popups::render_too_small(&mut buffer, area);
        paint_buffer(playfield, &buffer);
        return;
    };

    match app.state.route {
        LobbyRoute::Home => {
            home::render_home_base(&mut buffer, app, layout);
        }
        LobbyRoute::ContactPicker | LobbyRoute::AddContact => {
            threads::render_comms_scene(&mut buffer, layout.body, app)
        }
        _ => home::render_home_base(&mut buffer, app, layout),
    }

    if app.state.route == LobbyRoute::Home
        && app.state.status_message.is_some()
        && !app.state.show_help
    {
        popups::render_toast(
            &mut buffer,
            layout.body,
            app.state_ref(),
        );
    }

    if app.state.show_help {
        popups::render_help_popup(&mut buffer, layout.body, app.popup_position);
        paint_buffer(playfield, &buffer);
        return;
    }

    match app.state.route {
        LobbyRoute::Home => {}
        LobbyRoute::Settings => popups::render_settings_popup(&mut buffer, app, layout.body),
        LobbyRoute::ThemePicker => popups::render_theme_picker_popup(&mut buffer, app, layout.body),
        LobbyRoute::ComposeInvite => {
            popups::render_compose_invite_popup(&mut buffer, app, layout.body)
        }
        LobbyRoute::EditHandle => popups::render_edit_handle_popup(&mut buffer, app, layout.body),
        LobbyRoute::ContactPicker => {
            popups::render_contact_picker_popup(&mut buffer, app, layout.body)
        }
        LobbyRoute::AddContact => popups::render_add_contact_popup(&mut buffer, app, layout.body),
        _ => {}
    }

    paint_buffer(playfield, &buffer);
}

fn paint_buffer(playfield: &mut PlayfieldBuffer, buffer: &Buffer) {
    for y in 0..buffer.area.height {
        for x in 0..buffer.area.width {
            let Some(cell) = buffer.cell((buffer.area.x + x, buffer.area.y + y)) else {
                continue;
            };
            let symbol = cell.symbol().chars().next().unwrap_or(' ');
            let fg = theme::from_tui_color(cell.fg, theme::body_style().fg);
            let bg = theme::from_tui_color(cell.bg, theme::body_style().bg);
            let bold = cell.modifier.contains(Modifier::BOLD);
            playfield.set_cell(
                (buffer.area.y + y) as usize,
                (buffer.area.x + x) as usize,
                symbol,
                nc_ui::CellStyle::new(fg, bg, bold),
            );
        }
    }
}

trait LobbyAppExt {
    fn state_ref(&self) -> &crate::lobby::state::LobbyState;
}

impl LobbyAppExt for LobbyApp {
    fn state_ref(&self) -> &crate::lobby::state::LobbyState {
        &self.state
    }
}
