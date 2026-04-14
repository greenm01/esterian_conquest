use ratatui::buffer::Buffer;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::Style;
use ratatui::widgets::{Clear, Paragraph, Widget, Wrap};

use crate::lobby::state::{LobbyApp, LobbyState};
use crate::overlays::frame::RelativePopupOrigin;
use crate::theme;

use super::chrome::{chrome_block, popup_block, status_style, toast_text_style, with_panel_bg};
use super::layout::{help_popup_size, popup_rect, scroll_offset};

const FOOTER_HEIGHT: u16 = 5;
const SETTINGS_ROWS: [&str; 7] = [
    "Handle",
    "Idle Lock",
    "Mouse Follow",
    "Grid Dots",
    "Theme",
    "Save",
    "Cancel",
];

pub(super) fn render_settings_popup(buffer: &mut Buffer, app: &LobbyApp, parent: Rect) {
    let popup = popup_rect(parent, (60, 17), app.popup_position);
    let styles = theme::tui_theme();
    let block = popup_block(" LOBBY SETTINGS ", styles.border);
    let inner = block.inner(popup);
    Clear.render(popup, buffer);
    block.render(popup, buffer);

    for (idx, label) in SETTINGS_ROWS.iter().enumerate() {
        let row = inner.y + idx as u16;
        if row >= inner.bottom() {
            break;
        }
        let value = match *label {
            "Handle" => self_or_unset(app.state.player_handle.as_deref()),
            "Idle Lock" => {
                super::super::storage::settings::lock_timeout_label(
                    app.state.settings_draft.lock_timeout_minutes,
                )
            }
            "Mouse Follow" => on_off(app.state.settings_draft.follow_mouse_on_map).to_string(),
            "Grid Dots" => on_off(app.state.settings_draft.dense_empty_sector_dots).to_string(),
            "Theme" => theme::display_name_for_key(&app.state.settings_draft.theme_key),
            _ => String::new(),
        };
        let prefix = if app.state.settings_selected == idx { ">" } else { " " };
        let line = if value.is_empty() {
            format!("{prefix} {label}")
        } else {
            format!("{prefix} {label:<12} : {value}")
        };
        let style = if app.state.settings_selected == idx {
            styles.selected
        } else if idx >= 5 {
            with_panel_bg(styles.accent)
        } else {
            with_panel_bg(styles.value)
        };
        buffer.set_stringn(inner.x, row, line, inner.width as usize, style);
    }

    let info_row = inner.y + SETTINGS_ROWS.len() as u16 + 1;
    if info_row < inner.bottom() {
        buffer.set_stringn(
            inner.x,
            info_row,
            "Theme selection previews immediately and applies to the hosted dashboard too.",
            inner.width as usize,
            with_panel_bg(styles.dim),
        );
    }
    if let Some(status) = app.state.status_message.as_deref() {
        let row = info_row.saturating_add(2);
        if row < inner.bottom() {
            buffer.set_stringn(
                inner.x,
                row,
                status,
                inner.width as usize,
                with_panel_bg(toast_text_style(app.state.status_tone)),
            );
        }
    }
}

pub(super) fn render_theme_picker_popup(buffer: &mut Buffer, app: &LobbyApp, parent: Rect) {
    let popup = popup_rect(parent, (82, 20), app.popup_position);
    let styles = theme::tui_theme();
    let block = popup_block(" THEME PICKER ", styles.border);
    let inner = block.inner(popup);
    Clear.render(popup, buffer);
    block.render(popup, buffer);
    if inner.width == 0 || inner.height == 0 {
        return;
    }

    let [list_area, preview_area] =
        Layout::horizontal([Constraint::Percentage(52), Constraint::Percentage(48)])
            .spacing(1)
            .areas(inner);

    let list_block = super::chrome::panel_block(" Themes ", false);
    let preview_block = super::chrome::panel_block(" Preview ", false);
    let list_inner = list_block.inner(list_area);
    let preview_inner = preview_block.inner(preview_area);
    list_block.render(list_area, buffer);
    preview_block.render(preview_area, buffer);

    let themes = app.state.available_themes();
    let visible_rows = list_inner.height as usize;
    let scroll = scroll_offset(themes.len(), visible_rows, app.state.theme_selected);
    for (offset, entry) in themes.iter().skip(scroll).take(visible_rows).enumerate() {
        let row = list_inner.y + offset as u16;
        let absolute_index = scroll + offset;
        let prefix = if absolute_index == app.state.theme_selected {
            ">"
        } else {
            " "
        };
        let line = format!("{prefix} {}", entry.display_name);
        let style = if absolute_index == app.state.theme_selected {
            styles.selected
        } else if entry.key == app.state.settings_draft.theme_key {
            with_panel_bg(styles.accent)
        } else {
            with_panel_bg(styles.value)
        };
        buffer.set_stringn(list_inner.x, row, line, list_inner.width as usize, style);
    }

    let preview_lines = [
        format!(
            "Current : {}",
            theme::display_name_for_key(&app.state.settings_draft.theme_key)
        ),
        format!("Key     : {}", app.state.settings_draft.theme_key),
        String::new(),
        "Accent preview".to_string(),
        "Status label / value".to_string(),
        "Selected row preview".to_string(),
    ];
    for (idx, line) in preview_lines.iter().enumerate() {
        let row = preview_inner.y + idx as u16;
        if row >= preview_inner.bottom() {
            break;
        }
        let style = match idx {
            0 => with_panel_bg(styles.label),
            1 => with_panel_bg(styles.dim),
            3 => with_panel_bg(styles.accent),
            4 => with_panel_bg(styles.value),
            5 => styles.selected,
            _ => with_panel_bg(styles.value),
        };
        buffer.set_stringn(preview_inner.x, row, line, preview_inner.width as usize, style);
    }
}

pub(super) fn render_compose_invite_popup(buffer: &mut Buffer, app: &LobbyApp, parent: Rect) {
    render_popup_lines(
        buffer,
        popup_rect(parent, (64, 11), app.popup_position),
        " REQUEST TO JOIN ",
        &[
            format!(
                "Game    : {}",
                app.state
                    .selected_open_game()
                    .map(|row| row.game.as_str())
                    .unwrap_or("<none>")
            ),
            format!("Message : {}", app.state.compose_message_input),
            "Enter sends a 30513 join request.".to_string(),
        ],
        theme::tui_theme().value,
    );
}

pub(super) fn render_edit_handle_popup(buffer: &mut Buffer, app: &LobbyApp, parent: Rect) {
    render_popup_lines(
        buffer,
        popup_rect(parent, (58, 11), app.popup_position),
        " EDIT HANDLE ",
        &[
            format!(
                "Current handle: {}",
                app.state.player_handle.as_deref().unwrap_or("<unset>")
            ),
            format!("New handle   : {}", app.state.edit_handle_input),
            "Enter saves the local keychain handle.".to_string(),
        ],
        theme::tui_theme().value,
    );
}

pub(super) fn render_contact_picker_popup(buffer: &mut Buffer, app: &LobbyApp, parent: Rect) {
    let popup = popup_rect(parent, (64, 16), app.popup_position);
    let styles = theme::tui_theme();
    let block = popup_block(" ADDRESS BOOK ", styles.border);
    let inner = block.inner(popup);
    Clear.render(popup, buffer);
    block.render(popup, buffer);
    if inner.width == 0 || inner.height == 0 {
        return;
    }

    let contacts = app.state.selectable_direct_contacts();
    if contacts.is_empty() {
        buffer.set_stringn(
            inner.x,
            inner.y,
            "<no contacts>",
            inner.width as usize,
            with_panel_bg(styles.dim),
        );
    } else {
        let selected = app
            .state
            .contact_picker_selected
            .min(contacts.len().saturating_sub(1));
        let visible = inner.height.saturating_sub(2) as usize;
        let scroll = scroll_offset(contacts.len(), visible.max(1), selected);
        for (offset, (_, contact)) in contacts.iter().skip(scroll).take(visible).enumerate() {
            let row = inner.y + offset as u16;
            let absolute = scroll + offset;
            let style = if absolute == selected {
                styles.selected
            } else {
                with_panel_bg(styles.value)
            };
            let marker = if contact.blocked {
                " !"
            } else if contact.hidden {
                " h"
            } else {
                ""
            };
            let line = format!(
                "{}{}{}",
                super::chrome::truncate_title(
                    &contact.label,
                    inner.width.saturating_sub(marker.len() as u16) as usize,
                ),
                if contact.nip05.is_some() { " @" } else { "" },
                marker
            );
            buffer.set_stringn(inner.x, row, &line, inner.width as usize, style);
        }
    }

    let footer_row = inner.bottom().saturating_sub(1);
    buffer.set_stringn(
        inner.x,
        footer_row,
        "Enter select  A add  B block  Delete hide  Esc close",
        inner.width as usize,
        with_panel_bg(styles.dim),
    );
}

pub(super) fn render_add_contact_popup(buffer: &mut Buffer, app: &LobbyApp, parent: Rect) {
    render_popup_lines(
        buffer,
        popup_rect(parent, (60, 10), app.popup_position),
        " ADD CONTACT ",
        &[
            "Enter a valid npub or NIP-05.".to_string(),
            app.state.add_contact_input.clone(),
        ],
        theme::tui_theme().value,
    );
}

pub(super) fn render_help_popup(
    buffer: &mut Buffer,
    parent: Rect,
    origin: Option<RelativePopupOrigin>,
) {
    let area = popup_rect(parent, help_popup_size(parent), origin);
    let styles = theme::tui_theme();
    let block = popup_block(" HELP ", styles.accent);
    let inner = block.inner(area);
    Clear.render(area, buffer);
    block.render(area, buffer);
    Paragraph::new(
        [
            "Tab        : cycle focus across lobby panels",
            "J / K      : move within the focused panel",
            "Enter      : open selected game or open the selected COMMS thread",
            "J          : compose a join request",
            "T          : open full-screen COMMS",
            "COMMS Tab  : cycle Chat / New / Threads",
            "COMMS Enter: send chat or open selected unread/thread row",
            "Alt-A      : open the address book from COMMS",
            "B / Delete : block or hide the active direct contact",
            "Alt-L      : lock nc-dash",
            "S          : open lobby settings, including handle and idle lock",
            "R          : refresh the hosted lobby",
            "? / Esc    : close this help popup",
            "Q          : quit nc-dash from the lobby",
        ]
        .join("\n"),
    )
    .style(with_panel_bg(styles.value))
    .wrap(Wrap { trim: false })
    .render(inner, buffer);
}

pub(super) fn render_too_small(buffer: &mut Buffer, area: Rect) {
    let popup = popup_rect(area, (42, 9), None);
    render_popup_lines(
        buffer,
        popup,
        " WINDOW TOO SMALL ",
        &[
            "nc-lobby needs a larger window.".to_string(),
            "Resize and reopen the lobby.".to_string(),
        ],
        theme::tui_theme().value,
    );
}

pub(super) fn render_toast(buffer: &mut Buffer, area: Rect, state: &LobbyState) {
    let Some(message) = state.status_message.as_deref() else {
        return;
    };
    let max_width = area.width.saturating_sub(8) as usize;
    let lines = wrap_lines(message, max_width);
    let content_width = lines
        .iter()
        .map(|line| line.chars().count() as u16)
        .max()
        .unwrap_or(0)
        .clamp(1, max_width as u16);
    let popup = Rect::new(
        area.x + area.width.saturating_sub(content_width.saturating_add(4)) / 2,
        area.bottom()
            .saturating_sub(lines.len() as u16 + FOOTER_HEIGHT + 4),
        content_width.saturating_add(4).max(18),
        (lines.len() as u16).saturating_add(4).clamp(5, 8),
    );
    let block = chrome_block(status_style(state.status_tone));
    let inner = block.inner(popup);
    Clear.render(popup, buffer);
    block.render(popup, buffer);
    for (idx, line) in lines.iter().take(inner.height as usize).enumerate() {
        buffer.set_stringn(
            inner.x,
            inner.y + idx as u16,
            line,
            inner.width as usize,
            with_panel_bg(toast_text_style(state.status_tone)),
        );
    }
}

fn render_popup_lines(
    buffer: &mut Buffer,
    area: Rect,
    title: &'static str,
    lines: &[String],
    style: Style,
) {
    let styles = theme::tui_theme();
    let block = popup_block(title, styles.border);
    let inner = block.inner(area);
    Clear.render(area, buffer);
    block.render(area, buffer);
    for (idx, line) in lines.iter().enumerate() {
        let row = inner.y + idx as u16;
        if row >= inner.bottom() {
            break;
        }
        buffer.set_stringn(inner.x, row, line, inner.width as usize, with_panel_bg(style));
    }
}

fn wrap_lines(text: &str, max_width: usize) -> Vec<String> {
    let width = max_width.max(8);
    let mut out = Vec::new();
    for raw in text.lines() {
        let mut current = String::new();
        for word in raw.split_whitespace() {
            let extra = if current.is_empty() { 0 } else { 1 };
            if current.chars().count() + extra + word.chars().count() > width && !current.is_empty()
            {
                out.push(current);
                current = word.to_string();
            } else {
                if !current.is_empty() {
                    current.push(' ');
                }
                current.push_str(word);
            }
        }
        if current.is_empty() {
            out.push(String::new());
        } else {
            out.push(current);
        }
    }
    if out.is_empty() {
        out.push(String::new());
    }
    out
}

fn self_or_unset(value: Option<&str>) -> String {
    value.unwrap_or("<unset>").to_string()
}

fn on_off(value: bool) -> &'static str {
    if value { "ON" } else { "OFF" }
}
