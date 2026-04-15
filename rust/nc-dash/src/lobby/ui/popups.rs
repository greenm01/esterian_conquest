use ratatui::buffer::Buffer;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::Style;
use ratatui::widgets::{Clear, Widget};

use crate::lobby::state::{LobbyApp, LobbyRoute, LobbyState, LobbyTab};
use crate::modal::{
    WrappedHelpLines, WrappedTextLines, format_help_rows, max_content_width,
    measure_modal_text_lines, wrap_formatted_help_lines,
};
use crate::overlays::frame::RelativePopupOrigin;
use crate::theme;

use super::chrome::{chrome_block, popup_block, status_style, toast_text_style, with_panel_bg};
use super::layout::{popup_rect, scroll_offset};

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

type HelpRow = (&'static str, &'static str);
const THEME_PICKER_PANEL_GAP: u16 = 1;
const THEME_PICKER_PANEL_CHROME: u16 = 2;
const THEME_PICKER_POPUP_CHROME: u16 = 2;

const MY_GAMES_HELP_ROWS: &[HelpRow] = &[
    ("Tab", "cycle dashboard tabs"),
    ("J / K", "move within MY GAMES"),
    ("Enter", "open the selected joined game"),
    ("Alt-L", "lock nc-dash"),
    ("S", "open lobby settings, including handle and idle lock"),
    ("R", "refresh the hosted lobby"),
    ("? / Esc", "close this help popup"),
    ("Q / Esc", "quit nc-dash from the lobby"),
];

const OPEN_GAMES_HELP_ROWS: &[HelpRow] = &[
    ("Tab", "cycle dashboard tabs"),
    ("J / K", "move within OPEN GAMES"),
    ("Enter", "request to join the selected game"),
    ("J", "compose a join request"),
    ("Alt-L", "lock nc-dash"),
    ("S", "open lobby settings, including handle and idle lock"),
    ("R", "refresh the hosted lobby"),
    ("? / Esc", "close this help popup"),
    ("Q / Esc", "quit nc-dash from the lobby"),
];

const COMMS_HELP_ROWS: &[HelpRow] = &[
    ("Tab", "cycle dashboard tabs"),
    ("Left/Right", "cycle Chat / New / Threads"),
    ("J / K", "move within the focused COMMS pane"),
    ("Enter", "send chat or open the selected unread/thread row"),
    ("Alt-A", "open the address book"),
    ("Delete", "hide the selected direct contact"),
    ("Alt-L", "lock nc-dash"),
    ("? / Esc", "close this help popup"),
    ("Q / Esc", "quit nc-dash from the lobby"),
];

const ADDRESS_BOOK_HELP_ROWS: &[HelpRow] = &[
    ("J / K", "move within the address book"),
    ("Enter", "open the selected direct contact"),
    ("A", "add a contact by npub or NIP-05"),
    ("B", "block the selected direct contact"),
    ("Delete", "hide the selected direct contact"),
    ("? / Esc", "close this help popup"),
    ("Esc", "close the address book"),
];

pub(super) fn render_settings_popup(buffer: &mut Buffer, app: &LobbyApp, parent: Rect) {
    let popup = popup_rect(parent, settings_popup_size(app, parent), app.popup_position);
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
            "Idle Lock" => super::super::storage::settings::lock_timeout_label(
                app.state.settings_draft.lock_timeout_minutes,
            ),
            "Mouse Follow" => on_off(app.state.settings_draft.follow_mouse_on_map).to_string(),
            "Grid Dots" => on_off(app.state.settings_draft.dense_empty_sector_dots).to_string(),
            "Theme" => theme::display_name_for_key(&app.state.settings_draft.theme_key),
            _ => String::new(),
        };
        let prefix = if app.state.settings_selected == idx {
            ">"
        } else {
            " "
        };
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

    let info_lines = measure_modal_text_lines(
        &[String::from(
            "Theme selection previews immediately and applies to the hosted dashboard too.",
        )],
        inner.width as usize,
    );
    let mut row = inner.y + SETTINGS_ROWS.len() as u16 + 1;
    for line in info_lines.lines.iter() {
        if row >= inner.bottom() {
            break;
        }
        buffer.set_stringn(
            inner.x,
            row,
            line,
            inner.width as usize,
            with_panel_bg(styles.dim),
        );
        row += 1;
    }
    if let Some(status) = app.state.status_message.as_deref() {
        row = row.saturating_add(1);
        for line in measure_popup_text(parent, &[status.to_string()])
            .lines
            .iter()
        {
            if row >= inner.bottom() {
                break;
            }
            buffer.set_stringn(
                inner.x,
                row,
                line,
                inner.width as usize,
                with_panel_bg(toast_text_style(app.state.status_tone)),
            );
            row += 1;
        }
    }
}

pub(super) fn render_theme_picker_popup(buffer: &mut Buffer, app: &LobbyApp, parent: Rect) {
    let popup = popup_rect(
        parent,
        theme_picker_popup_size(app, parent),
        app.popup_position,
    );
    let styles = theme::tui_theme();
    let block = popup_block(" THEME PICKER ", styles.border);
    let inner = block.inner(popup);
    Clear.render(popup, buffer);
    block.render(popup, buffer);
    if inner.width == 0 || inner.height == 0 {
        return;
    }

    let (list_inner_width, preview_inner_width, _, _) = theme_picker_dimensions(app);
    let (list_panel_width, preview_panel_width) = theme_picker_panel_widths(
        inner.width,
        list_inner_width as u16 + THEME_PICKER_PANEL_CHROME,
        preview_inner_width as u16 + THEME_PICKER_PANEL_CHROME,
    );
    let [list_area, preview_area] = Layout::horizontal([
        Constraint::Length(list_panel_width),
        Constraint::Length(preview_panel_width),
    ])
    .spacing(THEME_PICKER_PANEL_GAP)
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
        buffer.set_stringn(
            preview_inner.x,
            row,
            line,
            preview_inner.width as usize,
            style,
        );
    }
}

pub(super) fn render_compose_invite_popup(buffer: &mut Buffer, app: &LobbyApp, parent: Rect) {
    let lines = compose_invite_popup_lines(app);
    render_popup_lines(
        buffer,
        popup_rect(parent, popup_text_size(parent, &lines), app.popup_position),
        " REQUEST TO JOIN ",
        &measure_popup_text(parent, &lines).lines,
        theme::tui_theme().value,
    );
}

pub(super) fn render_edit_handle_popup(buffer: &mut Buffer, app: &LobbyApp, parent: Rect) {
    let lines = edit_handle_popup_lines(app);
    render_popup_lines(
        buffer,
        popup_rect(parent, popup_text_size(parent, &lines), app.popup_position),
        " EDIT HANDLE ",
        &measure_popup_text(parent, &lines).lines,
        theme::tui_theme().value,
    );
}

pub(super) fn render_contact_picker_popup(buffer: &mut Buffer, app: &LobbyApp, parent: Rect) {
    let popup = popup_rect(
        parent,
        contact_picker_popup_size(app, parent),
        app.popup_position,
    );
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
            with_panel_bg(styles.error),
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
    let lines = add_contact_popup_lines(app);
    render_popup_lines(
        buffer,
        popup_rect(parent, popup_text_size(parent, &lines), app.popup_position),
        " ADD CONTACT ",
        &measure_popup_text(parent, &lines).lines,
        theme::tui_theme().value,
    );
}

fn help_popup_rows(app: &LobbyApp) -> &'static [HelpRow] {
    match app.state.route {
        LobbyRoute::ContactPicker | LobbyRoute::AddContact => ADDRESS_BOOK_HELP_ROWS,
        _ => match app.state.active_tab {
            LobbyTab::MyGames => MY_GAMES_HELP_ROWS,
            LobbyTab::OpenGames => OPEN_GAMES_HELP_ROWS,
            LobbyTab::Comms => COMMS_HELP_ROWS,
        },
    }
}

pub(super) fn render_help_popup(
    buffer: &mut Buffer,
    app: &LobbyApp,
    parent: Rect,
    origin: Option<RelativePopupOrigin>,
) {
    let area = popup_rect(parent, help_popup_size(app, parent), origin);
    let styles = theme::tui_theme();
    let block = popup_block(" HELP ", styles.accent);
    let inner = block.inner(area);
    Clear.render(area, buffer);
    block.render(area, buffer);
    let wrapped = wrapped_help_popup_lines(app, parent);
    for (idx, line) in wrapped.lines.iter().take(inner.height as usize).enumerate() {
        buffer.set_stringn(
            inner.x,
            inner.y + idx as u16,
            line,
            inner.width as usize,
            with_panel_bg(styles.value),
        );
    }
}

pub(super) fn help_popup_size(app: &LobbyApp, parent: Rect) -> (u16, u16) {
    let wrapped = wrapped_help_popup_lines(app, parent);
    popup_size(wrapped.content_width, wrapped.lines.len())
}

pub(super) fn render_too_small(buffer: &mut Buffer, area: Rect) {
    let lines = vec![
        "nc-lobby needs a larger window.".to_string(),
        "Resize and reopen the lobby.".to_string(),
    ];
    let popup = popup_rect(area, popup_text_size(area, &lines), None);
    render_popup_lines(
        buffer,
        popup,
        " WINDOW TOO SMALL ",
        &measure_popup_text(area, &lines).lines,
        theme::tui_theme().value,
    );
}

pub(super) fn render_toast(buffer: &mut Buffer, area: Rect, state: &LobbyState) {
    let Some(message) = state.status_message.as_deref() else {
        return;
    };
    let wrapped = measure_popup_text(area, &[message.to_string()]);
    let content_width = wrapped.content_width as u16;
    let popup = Rect::new(
        area.x + area.width.saturating_sub(content_width.saturating_add(4)) / 2,
        area.bottom()
            .saturating_sub(wrapped.lines.len() as u16 + FOOTER_HEIGHT + 4),
        content_width.saturating_add(4).max(18),
        (wrapped.lines.len() as u16).saturating_add(4).clamp(5, 8),
    );
    let block = chrome_block(status_style(state.status_tone));
    let inner = block.inner(popup);
    Clear.render(popup, buffer);
    block.render(popup, buffer);
    for (idx, line) in wrapped.lines.iter().take(inner.height as usize).enumerate() {
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
        buffer.set_stringn(
            inner.x,
            row,
            line,
            inner.width as usize,
            with_panel_bg(style),
        );
    }
}

pub(super) fn settings_popup_size(app: &LobbyApp, parent: Rect) -> (u16, u16) {
    let natural_row_width = settings_rows(app)
        .iter()
        .map(|line| line.chars().count())
        .max()
        .unwrap_or(1);
    let info_lines = measure_popup_text(
        parent,
        &[String::from(
            "Theme selection previews immediately and applies to the hosted dashboard too.",
        )],
    );
    let status_lines = app
        .state
        .status_message
        .as_deref()
        .map(|status| measure_popup_text(parent, &[status.to_string()]))
        .unwrap_or_else(empty_wrapped_text);
    let content_width = natural_row_width
        .max(info_lines.content_width)
        .max(status_lines.content_width);
    let content_height = SETTINGS_ROWS.len()
        + 1
        + info_lines.lines.len()
        + usize::from(!status_lines.lines.is_empty())
        + status_lines.lines.len();
    popup_size(content_width, content_height)
}

pub(super) fn theme_picker_popup_size(app: &LobbyApp, parent: Rect) -> (u16, u16) {
    let (list_width, preview_width, list_rows, preview_rows) = theme_picker_dimensions(app);
    let natural_width = list_width as u16
        + preview_width as u16
        + THEME_PICKER_PANEL_CHROME * 2
        + THEME_PICKER_PANEL_GAP
        + THEME_PICKER_POPUP_CHROME;
    let natural_height =
        list_rows.max(preview_rows) as u16 + THEME_PICKER_PANEL_CHROME + THEME_PICKER_POPUP_CHROME;
    (
        natural_width.min(parent.width.saturating_sub(2).max(10)),
        natural_height.min(parent.height.saturating_sub(2).max(6)),
    )
}

pub(super) fn compose_invite_popup_size(app: &LobbyApp, parent: Rect) -> (u16, u16) {
    popup_text_size(parent, &compose_invite_popup_lines(app))
}

pub(super) fn edit_handle_popup_size(app: &LobbyApp, parent: Rect) -> (u16, u16) {
    popup_text_size(parent, &edit_handle_popup_lines(app))
}

pub(super) fn contact_picker_popup_size(app: &LobbyApp, parent: Rect) -> (u16, u16) {
    let contacts = app.state.selectable_direct_contacts();
    let row_width = contacts
        .iter()
        .map(|(_, contact)| contact_picker_line(contact).chars().count())
        .max()
        .unwrap_or("<no contacts>".chars().count());
    let content_width = row_width.max(
        "Enter select  A add  B block  Delete hide  Esc close"
            .chars()
            .count(),
    );
    let natural_height = contacts.len().max(1) + 1;
    let max_height = parent.height.saturating_sub(4).max(5) as usize;
    popup_size(content_width, natural_height.min(max_height))
}

pub(super) fn add_contact_popup_size(app: &LobbyApp, parent: Rect) -> (u16, u16) {
    popup_text_size(parent, &add_contact_popup_lines(app))
}

fn settings_rows(app: &LobbyApp) -> Vec<String> {
    SETTINGS_ROWS
        .iter()
        .enumerate()
        .map(|(idx, label)| {
            let value = match *label {
                "Handle" => self_or_unset(app.state.player_handle.as_deref()),
                "Idle Lock" => super::super::storage::settings::lock_timeout_label(
                    app.state.settings_draft.lock_timeout_minutes,
                ),
                "Mouse Follow" => on_off(app.state.settings_draft.follow_mouse_on_map).to_string(),
                "Grid Dots" => on_off(app.state.settings_draft.dense_empty_sector_dots).to_string(),
                "Theme" => theme::display_name_for_key(&app.state.settings_draft.theme_key),
                _ => String::new(),
            };
            let prefix = if app.state.settings_selected == idx {
                ">"
            } else {
                " "
            };
            if value.is_empty() {
                format!("{prefix} {label}")
            } else {
                format!("{prefix} {label:<12} : {value}")
            }
        })
        .collect()
}

fn theme_preview_lines(app: &LobbyApp) -> Vec<String> {
    vec![
        format!(
            "Current : {}",
            theme::display_name_for_key(&app.state.settings_draft.theme_key)
        ),
        format!("Key     : {}", app.state.settings_draft.theme_key),
        String::new(),
        "Accent preview".to_string(),
        "Status label / value".to_string(),
        "Selected row preview".to_string(),
    ]
}

fn theme_picker_dimensions(app: &LobbyApp) -> (usize, usize, usize, usize) {
    let themes = app.state.available_themes();
    let list_width = themes
        .iter()
        .map(|entry| entry.display_name.chars().count() + 2)
        .max()
        .unwrap_or(8)
        .max(" Themes ".chars().count());
    let preview_lines = theme_preview_lines(app);
    let preview_width = preview_lines
        .iter()
        .map(|line| line.chars().count())
        .max()
        .unwrap_or(8)
        .saturating_add(2)
        .max(" Preview ".chars().count());
    (
        list_width,
        preview_width,
        themes.len().max(1),
        preview_lines.len().max(1),
    )
}

fn theme_picker_panel_widths(
    available_width: u16,
    desired_list_width: u16,
    desired_preview_width: u16,
) -> (u16, u16) {
    let available_panels = available_width.saturating_sub(THEME_PICKER_PANEL_GAP);
    let min_panel_width = THEME_PICKER_PANEL_CHROME + 1;
    if desired_list_width + desired_preview_width <= available_panels {
        return (desired_list_width, desired_preview_width);
    }
    let preview_width = desired_preview_width.clamp(
        min_panel_width,
        available_panels.saturating_sub(min_panel_width),
    );
    let list_width = available_panels.saturating_sub(preview_width);
    if list_width >= min_panel_width {
        return (list_width, preview_width);
    }
    let list_width = min_panel_width.min(available_panels);
    let preview_width = available_panels.saturating_sub(list_width);
    (list_width, preview_width)
}

fn compose_invite_popup_lines(app: &LobbyApp) -> Vec<String> {
    vec![
        format!(
            "Game    : {}",
            app.state
                .selected_open_game()
                .map(|row| row.game.as_str())
                .unwrap_or("<none>")
        ),
        format!("Message : {}", app.state.compose_message_input),
        "Enter sends a 30513 join request.".to_string(),
    ]
}

fn edit_handle_popup_lines(app: &LobbyApp) -> Vec<String> {
    vec![
        format!(
            "Current handle: {}",
            app.state.player_handle.as_deref().unwrap_or("<unset>")
        ),
        format!("New handle   : {}", app.state.edit_handle_input),
        "Enter saves the local keychain handle.".to_string(),
    ]
}

fn add_contact_popup_lines(app: &LobbyApp) -> Vec<String> {
    vec![
        "Enter a valid npub or NIP-05.".to_string(),
        app.state.add_contact_input.clone(),
    ]
}

fn contact_picker_line(contact: &crate::lobby::models::DirectContactRow) -> String {
    let marker = if contact.blocked {
        " !"
    } else if contact.hidden {
        " h"
    } else {
        ""
    };
    format!(
        "{}{}{}",
        contact.label,
        if contact.nip05.is_some() { " @" } else { "" },
        marker
    )
}

fn wrapped_help_popup_lines(app: &LobbyApp, parent: Rect) -> WrappedHelpLines {
    let lines = format_help_rows(help_popup_rows(app).iter().copied());
    wrap_formatted_help_lines(&lines, max_popup_content_width(parent))
}

fn popup_text_size(parent: Rect, lines: &[String]) -> (u16, u16) {
    popup_size_from_wrapped(&measure_popup_text(parent, lines))
}

fn measure_popup_text(parent: Rect, lines: &[String]) -> WrappedTextLines {
    let parent = crate::modal::Rect::new(parent.x, parent.y, parent.width, parent.height);
    measure_modal_text_lines(lines, max_content_width(parent))
}

fn popup_size_from_wrapped(wrapped: &WrappedTextLines) -> (u16, u16) {
    popup_size(wrapped.content_width, wrapped.lines.len())
}

fn popup_size(content_width: usize, content_height: usize) -> (u16, u16) {
    (
        content_width.max(1) as u16 + 4,
        content_height.max(1) as u16 + 2,
    )
}

fn max_popup_content_width(parent: Rect) -> usize {
    parent.width.saturating_sub(6).max(1) as usize
}

fn empty_wrapped_text() -> WrappedTextLines {
    WrappedTextLines {
        lines: Vec::new(),
        content_width: 0,
    }
}

fn self_or_unset(value: Option<&str>) -> String {
    value.unwrap_or("<unset>").to_string()
}

fn on_off(value: bool) -> &'static str {
    if value { "ON" } else { "OFF" }
}
