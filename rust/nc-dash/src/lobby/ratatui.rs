use nc_ui::PlayfieldBuffer;
use ratatui::buffer::Buffer;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::widgets::{Block, Borders, Clear, Padding, Paragraph, Widget, Wrap};

use crate::lobby::state::{
    LobbyApp, LobbyFocus, LobbyNetworkStatus, LobbyRoute, LobbyState, LobbyStatusTone,
};
use crate::lobby::threads;
use crate::overlays::frame::RelativePopupOrigin;
use crate::theme;

const HOME_MIN_WIDTH: u16 = 72;
const HOME_MIN_HEIGHT: u16 = 26;
const HEADER_HEIGHT: u16 = 5;
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

#[derive(Debug, Clone, Copy)]
pub struct HomeLayout {
    pub header: Rect,
    pub joined: Rect,
    pub inbox: Rect,
    pub open: Rect,
    pub notices: Rect,
    pub thread: Rect,
    pub footer: Rect,
    pub body: Rect,
}

#[derive(Debug, Clone, Copy)]
pub struct PaneHit {
    pub focus: LobbyFocus,
    pub selected_row: Option<usize>,
}

pub fn render_scene(playfield: &mut PlayfieldBuffer, app: &LobbyApp) {
    let width = playfield.width() as u16;
    let height = playfield.height() as u16;
    let area = Rect::new(0, 0, width, height);
    let mut buffer = Buffer::empty(area);

    let Some(layout) = home_layout(area) else {
        render_too_small(&mut buffer, area);
        paint_buffer(playfield, &buffer);
        return;
    };

    render_home_base(&mut buffer, app.state_ref(), layout);

    if app.state.route == LobbyRoute::Home && app.state.status_message.is_some() && !app.state.show_help
    {
        ToastOverlayWidget {
            state: app.state_ref(),
        }
        .render(layout.body, &mut buffer);
    }

    if app.state.show_help {
        let popup = popup_rect(
            layout.body,
            help_popup_size(layout.body),
            app.popup_position,
        );
        render_help_popup(&mut buffer, popup);
        paint_buffer(playfield, &buffer);
        return;
    }

    match app.state.route {
        LobbyRoute::Home => {}
        LobbyRoute::Settings => render_settings_popup(&mut buffer, app, layout.body),
        LobbyRoute::ThemePicker => render_theme_picker_popup(&mut buffer, app, layout.body),
        LobbyRoute::ComposeInvite => render_compose_invite_popup(&mut buffer, app, layout.body),
        LobbyRoute::GameInboxThread => render_game_inbox_popup(&mut buffer, app, layout.body),
        LobbyRoute::ComposeThread => render_compose_thread_popup(&mut buffer, app, layout.body),
        LobbyRoute::ContactPicker => render_contact_picker_popup(&mut buffer, app, layout.body),
        LobbyRoute::AddContact => render_add_contact_popup(&mut buffer, app, layout.body),
        LobbyRoute::EditHandle => render_edit_handle_popup(&mut buffer, app, layout.body),
        _ => {}
    }

    paint_buffer(playfield, &buffer);
}

pub fn home_layout(area: Rect) -> Option<HomeLayout> {
    if area.width < HOME_MIN_WIDTH || area.height < HOME_MIN_HEIGHT {
        return None;
    }
    let [header, body, footer] = Layout::vertical([
        Constraint::Length(HEADER_HEIGHT),
        Constraint::Min(0),
        Constraint::Length(FOOTER_HEIGHT),
    ])
    .areas(area);
    let [left, open, right] = Layout::horizontal([
        Constraint::Fill(30),
        Constraint::Fill(34),
        Constraint::Fill(36),
    ])
    .spacing(1)
    .areas(body);
    let [joined, inbox] = Layout::vertical([Constraint::Fill(5), Constraint::Fill(3)])
        .spacing(1)
        .areas(left);
    let [notices, thread] = Layout::vertical([Constraint::Fill(2), Constraint::Fill(3)])
        .spacing(1)
        .areas(right);
    Some(HomeLayout {
        header,
        joined,
        inbox,
        open,
        notices,
        thread,
        footer,
        body,
    })
}

pub fn hit_test_home(
    state: &LobbyState,
    geometry: nc_ui::ScreenGeometry,
    column: u16,
    row: u16,
) -> Option<PaneHit> {
    let layout = home_layout(Rect::new(0, 0, geometry.width() as u16, geometry.height() as u16))?;
    pane_hit(state, layout.joined, LobbyFocus::JoinedGames, column, row, state.joined_selected)
        .or_else(|| pane_hit(state, layout.inbox, LobbyFocus::Inbox, column, row, state.inbox_selected))
        .or_else(|| pane_hit(state, layout.open, LobbyFocus::OpenGames, column, row, state.open_selected))
        .or_else(|| pane_hit(state, layout.notices, LobbyFocus::Notices, column, row, state.notices_selected))
        .or_else(|| pane_hit(state, layout.thread, LobbyFocus::Thread, column, row, 0))
}

pub fn popup_title_bar_contains(app: &LobbyApp, column: u16, row: u16) -> bool {
    active_popup_rect(app)
        .map(|popup| row == popup.y && column >= popup.x && column < popup.right())
        .unwrap_or(false)
}

pub fn active_popup_rect(app: &LobbyApp) -> Option<Rect> {
    let area = Rect::new(0, 0, app.geometry.width() as u16, app.geometry.height() as u16);
    let layout = home_layout(area)?;
    let size = match app.state.route {
        LobbyRoute::Settings => Some((60, 17)),
        LobbyRoute::ThemePicker => Some((82, 20)),
        LobbyRoute::ComposeInvite => Some((64, 11)),
        LobbyRoute::GameInboxThread => Some((88, 22)),
        LobbyRoute::ComposeThread => Some((88, 22)),
        LobbyRoute::ContactPicker => Some((56, 18)),
        LobbyRoute::AddContact => Some((62, 11)),
        LobbyRoute::EditHandle => Some((58, 11)),
        _ if app.state.show_help => Some(help_popup_size(layout.body)),
        _ => None,
    }?;
    Some(popup_rect(layout.body, size, app.popup_position))
}

fn render_home_base(buffer: &mut Buffer, state: &LobbyState, layout: HomeLayout) {
    HeaderHudWidget { state }.render(layout.header, buffer);
    JoinedGamesWidget { state }.render(layout.joined, buffer);
    InboxWidget { state }.render(layout.inbox, buffer);
    OpenGamesWidget { state }.render(layout.open, buffer);
    NoticesWidget { state }.render(layout.notices, buffer);
    ThreadWidget { state }.render(layout.thread, buffer);
    FooterMenuWidget.render(layout.footer, buffer);
}

fn render_settings_popup(buffer: &mut Buffer, app: &LobbyApp, parent: Rect) {
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
                super::storage::settings::lock_timeout_label(
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

fn render_theme_picker_popup(buffer: &mut Buffer, app: &LobbyApp, parent: Rect) {
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

    let list_block = panel_block(" Themes ", false);
    let preview_block = panel_block(" Preview ", false);
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

fn render_compose_invite_popup(buffer: &mut Buffer, app: &LobbyApp, parent: Rect) {
    render_popup_lines(
        buffer,
        popup_rect(parent, (64, 11), app.popup_position),
        " REQUEST INVITE ",
        &[
            format!(
                "Game    : {}",
                app.state
                    .selected_open_game()
                    .map(|row| row.game.as_str())
                    .unwrap_or("<none>")
            ),
            format!("Message : {}", app.state.compose_message_input),
            "Enter sends a 30513 invite request.".to_string(),
        ],
        theme::tui_theme().value,
    );
}

fn render_compose_thread_popup(buffer: &mut Buffer, app: &LobbyApp, parent: Rect) {
    let popup = popup_rect(parent, (88, 22), app.popup_position);
    let styles = theme::tui_theme();
    let block = popup_block(" DIRECT THREAD ", styles.border);
    let inner = block.inner(popup);
    Clear.render(popup, buffer);
    block.render(popup, buffer);
    render_thread_surface(buffer, inner, app.state_ref(), true);
}

fn render_game_inbox_popup(buffer: &mut Buffer, app: &LobbyApp, parent: Rect) {
    let popup = popup_rect(parent, (88, 22), app.popup_position);
    let styles = theme::tui_theme();
    let block = popup_block(" GAME INBOX ", styles.border);
    let inner = block.inner(popup);
    Clear.render(popup, buffer);
    block.render(popup, buffer);
    render_game_inbox_surface(buffer, inner, app.state_ref(), true);
}

fn render_contact_picker_popup(buffer: &mut Buffer, app: &LobbyApp, parent: Rect) {
    let popup = popup_rect(parent, (56, 18), app.popup_position);
    let styles = theme::tui_theme();
    let block = popup_block(" CONTACTS ", styles.border);
    let inner = block.inner(popup);
    Clear.render(popup, buffer);
    block.render(popup, buffer);
    if inner.width == 0 || inner.height == 0 {
        return;
    }
    let footer_row = inner.bottom().saturating_sub(1);
    let visible_rows = footer_row.saturating_sub(inner.y) as usize;
    for (offset, contact) in app
        .state
        .direct_contacts
        .iter()
        .take(visible_rows)
        .enumerate()
    {
        let row = inner.y + offset as u16;
        let prefix = if app.state.contact_picker_selected == offset {
            ">"
        } else {
            " "
        };
        let detail = contact
            .nip05
            .clone()
            .unwrap_or_else(|| contact.source.clone());
        let style = if app.state.contact_picker_selected == offset {
            styles.selected
        } else {
            with_panel_bg(styles.value)
        };
        buffer.set_stringn(
            inner.x,
            row,
            format!("{prefix} {:<16} {}", contact.label, detail),
            inner.width as usize,
            style,
        );
    }
    if footer_row >= inner.y {
        buffer.set_stringn(
            inner.x,
            footer_row,
            "Enter selects   A adds contact   Esc closes",
            inner.width as usize,
            with_panel_bg(styles.dim),
        );
    }
}

fn render_add_contact_popup(buffer: &mut Buffer, app: &LobbyApp, parent: Rect) {
    render_popup_lines(
        buffer,
        popup_rect(parent, (62, 11), app.popup_position),
        " ADD CONTACT ",
        &[
            "Enter an npub or NIP-05 address.".to_string(),
            format!("Contact : {}", app.state.add_contact_input),
            "Enter saves it to encrypted cache.kdl.".to_string(),
        ],
        theme::tui_theme().value,
    );
}

fn render_edit_handle_popup(buffer: &mut Buffer, app: &LobbyApp, parent: Rect) {
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

fn render_help_popup(buffer: &mut Buffer, area: Rect) {
    let styles = theme::tui_theme();
    let block = popup_block(" LOBBY HELP ", styles.accent);
    let inner = block.inner(area);
    Clear.render(area, buffer);
    block.render(area, buffer);
    Paragraph::new(
        [
            "Tab        : cycle focus across lobby panels",
            "J / K      : move within the focused panel",
            "Enter      : open selected game or pop out the focused thread chat",
            "L          : lock nc-dash",
            "N          : compose an invite request",
            "M          : start inline compose in THREADS",
            "C          : open direct contact picker",
            "F          : cycle GAME INBOX filter",
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

fn render_too_small(buffer: &mut Buffer, area: Rect) {
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

fn pane_hit(
    state: &LobbyState,
    area: Rect,
    focus: LobbyFocus,
    column: u16,
    row: u16,
    selected: usize,
) -> Option<PaneHit> {
    if !contains(area, column, row) {
        return None;
    }
    if focus == LobbyFocus::Thread {
        return Some(PaneHit {
            focus,
            selected_row: None,
        });
    }
    let content = padded_inner(area);
    let selected_row = if contains(content, column, row) {
        clicked_row(
            focus_rows(state, focus).len(),
            content,
            selected,
            row,
            table_header_rows(focus),
        )
    } else {
        None
    };
    Some(PaneHit { focus, selected_row })
}

fn focus_rows(state: &LobbyState, focus: LobbyFocus) -> Vec<String> {
    match focus {
        LobbyFocus::JoinedGames => state
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
            .collect(),
        LobbyFocus::Inbox => state
            .filtered_game_inbox()
            .iter()
            .map(|item| {
                format!("{} | {} | {}", item.game, item.other_empire_name, item.preview)
            })
            .collect(),
        LobbyFocus::OpenGames => state
            .open_games
            .iter()
            .map(|row| {
                format!(
                    "{} | {} | {} | {} | {} open / {} total | {}",
                    row.status,
                    row.game,
                    row.host,
                    row.recruiting,
                    row.open_seats,
                    row.total_seats,
                    row.turn_summary
                )
            })
            .collect(),
        LobbyFocus::Notices => threads::notice_rows(state),
        LobbyFocus::Thread => Vec::new(),
    }
}

fn clicked_row(
    total_rows: usize,
    content: Rect,
    selected: usize,
    row: u16,
    header_rows: usize,
) -> Option<usize> {
    if total_rows == 0 || !contains(content, content.x, row) {
        return None;
    }
    let relative_row = row.saturating_sub(content.y) as usize;
    if relative_row < header_rows {
        return None;
    }
    let visible_rows = (content.height as usize).saturating_sub(header_rows);
    if visible_rows == 0 {
        return None;
    }
    let scroll = scroll_offset(total_rows, visible_rows, selected);
    let absolute_row = scroll + relative_row.saturating_sub(header_rows);
    (absolute_row < total_rows).then_some(absolute_row)
}

fn table_header_rows(focus: LobbyFocus) -> usize {
    match focus {
        LobbyFocus::JoinedGames => 1,
        LobbyFocus::OpenGames => 2,
        LobbyFocus::Inbox | LobbyFocus::Notices | LobbyFocus::Thread => 0,
    }
}

fn popup_rect(parent: Rect, preferred: (u16, u16), origin: Option<RelativePopupOrigin>) -> Rect {
    let (preferred_width, preferred_height) = preferred;
    let width = preferred_width.min(parent.width.saturating_sub(2)).max(10);
    let height = preferred_height.min(parent.height.saturating_sub(2)).max(5);
    if let Some(origin) = origin {
        let max_x = parent.right().saturating_sub(width);
        let max_y = parent.bottom().saturating_sub(height);
        let x = (parent.x + origin.col_offset as u16).min(max_x);
        let y = (parent.y + origin.row_offset as u16).min(max_y);
        return Rect::new(x, y, width, height);
    }
    let x = parent.x + parent.width.saturating_sub(width) / 2;
    let y = parent.y + parent.height.saturating_sub(height) / 2;
    Rect::new(x, y, width, height)
}

fn help_popup_size(parent: Rect) -> (u16, u16) {
    (parent.width.saturating_sub(8).min(72), 17)
}

fn contains(area: Rect, column: u16, row: u16) -> bool {
    column >= area.x && column < area.right() && row >= area.y && row < area.bottom()
}

fn padded_inner(area: Rect) -> Rect {
    Rect::new(
        area.x.saturating_add(2),
        area.y.saturating_add(2),
        area.width.saturating_sub(4),
        area.height.saturating_sub(4),
    )
}

fn panel_block<'a>(title: &'a str, focused: bool) -> Block<'a> {
    let styles = theme::tui_theme();
    let border = if focused { styles.accent } else { styles.border };
    let title_style = if focused { styles.selected } else { styles.title };
    Block::default()
        .borders(Borders::ALL)
        .padding(Padding::uniform(1))
        .title(title)
        .style(styles.body)
        .border_style(with_panel_bg(border))
        .title_style(with_panel_bg(title_style))
}

fn chrome_block(border_style: Style) -> Block<'static> {
    let styles = theme::tui_theme();
    Block::default()
        .borders(Borders::ALL)
        .padding(Padding::uniform(1))
        .style(styles.body)
        .border_style(with_panel_bg(border_style))
}

fn popup_block<'a>(title: &'a str, border_style: Style) -> Block<'a> {
    let styles = theme::tui_theme();
    Block::default()
        .borders(Borders::ALL)
        .padding(Padding::uniform(1))
        .title(title)
        .style(styles.body)
        .border_style(with_panel_bg(border_style))
        .title_style(with_panel_bg(styles.title))
}

fn with_panel_bg(style: Style) -> Style {
    let panel = theme::tui_theme().body;
    let mut merged = Style::default();
    if let Some(fg) = style.fg.or(panel.fg) {
        merged = merged.fg(fg);
    }
    if let Some(bg) = panel.bg {
        merged = merged.bg(bg);
    }
    if !style.add_modifier.is_empty() {
        merged = merged.add_modifier(style.add_modifier);
    }
    if !style.sub_modifier.is_empty() {
        merged = merged.remove_modifier(style.sub_modifier);
    }
    merged
}

fn scroll_offset(total_rows: usize, visible_rows: usize, selected: usize) -> usize {
    if total_rows == 0 || visible_rows == 0 {
        return 0;
    }
    selected
        .saturating_sub(visible_rows.saturating_sub(1))
        .min(total_rows.saturating_sub(visible_rows))
}

struct HeaderHudWidget<'a> {
    state: &'a LobbyState,
}

impl Widget for HeaderHudWidget<'_> {
    fn render(self, area: Rect, buffer: &mut Buffer) {
        if area.width == 0 || area.height == 0 {
            return;
        }
        let styles = theme::tui_theme();
        let network_line = format!("NETWORK: {}", self.state.network_status.label());
        let block = chrome_block(styles.border);
        let inner = block.inner(area);
        block.render(area, buffer);
        if inner.width == 0 || inner.height == 0 {
            return;
        }
        buffer.set_stringn(
            inner.x,
            inner.y,
            "NOSTRIAN CONQUEST LOBBY",
            inner.width as usize,
            with_panel_bg(styles.title),
        );
        right_align(
            buffer,
            inner,
            inner.y,
            &network_line,
            with_panel_bg(network_style(self.state.network_status)),
        );
    }
}

struct JoinedGamesWidget<'a> {
    state: &'a LobbyState,
}

impl Widget for JoinedGamesWidget<'_> {
    fn render(self, area: Rect, buffer: &mut Buffer) {
        render_joined_games_panel(
            buffer,
            area,
            self.state.focus == LobbyFocus::JoinedGames,
            self.state,
        );
    }
}

struct InboxWidget<'a> {
    state: &'a LobbyState,
}

impl Widget for InboxWidget<'_> {
    fn render(self, area: Rect, buffer: &mut Buffer) {
        let rows = focus_rows(self.state, LobbyFocus::Inbox);
        render_rows_panel(
            buffer,
            area,
            " GAME INBOX ",
            &rows,
            focused_selection(self.state, LobbyFocus::Inbox, self.state.inbox_selected),
            self.state.focus == LobbyFocus::Inbox,
            "<no joined game message threads>",
        );
    }
}

struct OpenGamesWidget<'a> {
    state: &'a LobbyState,
}

impl Widget for OpenGamesWidget<'_> {
    fn render(self, area: Rect, buffer: &mut Buffer) {
        render_open_games_panel(
            buffer,
            area,
            self.state.focus == LobbyFocus::OpenGames,
            self.state,
        );
    }
}

struct NoticesWidget<'a> {
    state: &'a LobbyState,
}

impl Widget for NoticesWidget<'_> {
    fn render(self, area: Rect, buffer: &mut Buffer) {
        let rows = focus_rows(self.state, LobbyFocus::Notices);
        render_rows_panel(
            buffer,
            area,
            " NOTICES ",
            &rows,
            focused_selection(self.state, LobbyFocus::Notices, self.state.notices_selected),
            self.state.focus == LobbyFocus::Notices,
            "<no public notices>",
        );
    }
}

struct ThreadWidget<'a> {
    state: &'a LobbyState,
}

impl Widget for ThreadWidget<'_> {
    fn render(self, area: Rect, buffer: &mut Buffer) {
        let block = panel_block(" THREADS ", self.state.focus == LobbyFocus::Thread);
        let inner = block.inner(area);
        block.render(area, buffer);
        if inner.width == 0 || inner.height == 0 {
            return;
        }
        render_thread_surface(buffer, inner, self.state, false);
    }
}

struct FooterMenuWidget;

impl Widget for FooterMenuWidget {
    fn render(self, area: Rect, buffer: &mut Buffer) {
        let styles = theme::tui_theme();
        let block = chrome_block(styles.border);
        let inner = block.inner(area);
        block.render(area, buffer);
        if inner.width == 0 || inner.height == 0 {
            return;
        }
        render_footer_tokens(buffer, inner);
    }
}

struct ToastOverlayWidget<'a> {
    state: &'a LobbyState,
}

impl Widget for ToastOverlayWidget<'_> {
    fn render(self, area: Rect, buffer: &mut Buffer) {
        let Some(message) = self.state.status_message.as_deref() else {
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
        let block = chrome_block(status_style(self.state.status_tone));
        let inner = block.inner(popup);
        Clear.render(popup, buffer);
        block.render(popup, buffer);
        for (idx, line) in lines.iter().take(inner.height as usize).enumerate() {
            buffer.set_stringn(
                inner.x,
                inner.y + idx as u16,
                line,
                inner.width as usize,
                with_panel_bg(toast_text_style(self.state.status_tone)),
            );
        }
    }
}

fn render_rows_panel(
    buffer: &mut Buffer,
    area: Rect,
    title: &str,
    rows: &[String],
    selected: Option<usize>,
    focused: bool,
    empty: &str,
) {
    let styles = theme::tui_theme();
    let block = panel_block(title, focused);
    let inner = block.inner(area);
    block.render(area, buffer);
    if inner.width == 0 || inner.height == 0 {
        return;
    }

    if rows.is_empty() {
        buffer.set_stringn(
            inner.x,
            inner.y,
            empty,
            inner.width as usize,
            with_panel_bg(styles.dim),
        );
        return;
    }

    let visible_rows = inner.height as usize;
    let scroll = scroll_offset(rows.len(), visible_rows, selected.unwrap_or(0));
    for (offset, row) in rows.iter().skip(scroll).take(visible_rows).enumerate() {
        let absolute = scroll + offset;
        let style = if selected == Some(absolute) {
            styles.selected
        } else {
            with_panel_bg(styles.value)
        };
        buffer.set_stringn(inner.x, inner.y + offset as u16, row, inner.width as usize, style);
    }
}

fn render_thread_surface(
    buffer: &mut Buffer,
    area: Rect,
    state: &LobbyState,
    modal: bool,
) {
    if area.width == 0 || area.height == 0 {
        return;
    }
    let styles = theme::tui_theme();
    let [history_area, prompt_area] =
        Layout::vertical([Constraint::Min(0), Constraint::Length(1)]).areas(area);
    render_thread_history(buffer, history_area, state, modal);
    render_thread_prompt(buffer, prompt_area, state, modal, styles.dim);
}

fn render_thread_history(buffer: &mut Buffer, area: Rect, state: &LobbyState, modal: bool) {
    if area.width == 0 || area.height == 0 {
        return;
    }
    let styles = theme::tui_theme();
    let context_line = format!("*** direct: {}", state.direct_thread_context_display());
    buffer.set_stringn(
        area.x,
        area.y,
        &context_line,
        area.width as usize,
        with_panel_bg(styles.dim),
    );
    if area.height <= 1 {
        return;
    }
    let transcript_area = Rect::new(area.x, area.y + 1, area.width, area.height - 1);
    let lines = threads::direct_thread_render_lines(state, transcript_area.width as usize);
    if lines.is_empty() {
        buffer.set_stringn(
            transcript_area.x,
            transcript_area.y,
            "<no encrypted direct messages>",
            transcript_area.width as usize,
            with_panel_bg(styles.dim),
        );
        return;
    }
    let visible_rows = transcript_area.height as usize;
    let max_scroll = lines.len().saturating_sub(visible_rows);
    let scroll = state.thread_scroll.min(max_scroll);
    let end = lines.len().saturating_sub(scroll);
    let start = end.saturating_sub(visible_rows);
    let visible = &lines[start..end];
    let first_row = transcript_area
        .bottom()
        .saturating_sub(visible.len() as u16);
    for (idx, line) in visible.iter().enumerate() {
        let row = first_row + idx as u16;
        render_thread_line(
            buffer,
            row,
            transcript_area.x,
            transcript_area.width as usize,
            line,
        );
    }
    if modal && scroll > 0 && transcript_area.y < transcript_area.bottom() {
        let marker = format!("*** scrollback: {scroll}");
        buffer.set_stringn(
            transcript_area.x,
            transcript_area.y,
            &marker,
            transcript_area.width as usize,
            with_panel_bg(styles.dim),
        );
    }
}

fn render_thread_prompt(
    buffer: &mut Buffer,
    area: Rect,
    state: &LobbyState,
    modal: bool,
    inactive_style: Style,
) {
    if area.width == 0 || area.height == 0 {
        return;
    }
    let styles = theme::tui_theme();
    let nick = threads::thread_prompt_label(state);
    let prompt = format!("<{nick}>: ");
    let nick_style = self_nick_style();
    let prompt_text = if state.thread_composing {
        state.compose_message_input.as_str()
    } else if modal {
        ""
    } else {
        "press C to choose contact"
    };
    let prompt_style = if state.thread_composing || modal {
        with_panel_bg(styles.value)
    } else {
        with_panel_bg(inactive_style)
    };
    let mut col = area.x;
    col = write_text(buffer, area.y, col, area.right(), &prompt, nick_style);
    let remaining = area.right().saturating_sub(col) as usize;
    if remaining > 0 {
        buffer.set_stringn(col, area.y, prompt_text, remaining, prompt_style);
    }
}

fn render_game_inbox_surface(
    buffer: &mut Buffer,
    area: Rect,
    state: &LobbyState,
    modal: bool,
) {
    if area.width == 0 || area.height == 0 {
        return;
    }
    let styles = theme::tui_theme();
    let [history_area, prompt_area] =
        Layout::vertical([Constraint::Min(0), Constraint::Length(1)]).areas(area);
    let context_line = format!("*** game inbox: {}", state.game_inbox_context_display());
    buffer.set_stringn(
        history_area.x,
        history_area.y,
        &context_line,
        history_area.width as usize,
        with_panel_bg(styles.dim),
    );
    if history_area.height > 1 {
        let transcript_area = Rect::new(
            history_area.x,
            history_area.y + 1,
            history_area.width,
            history_area.height - 1,
        );
        let lines = threads::game_inbox_render_lines(state, transcript_area.width as usize);
        if lines.is_empty() {
            buffer.set_stringn(
                transcript_area.x,
                transcript_area.y,
                "<no encrypted game messages>",
                transcript_area.width as usize,
                with_panel_bg(styles.dim),
            );
        } else {
            let visible_rows = transcript_area.height as usize;
            let max_scroll = lines.len().saturating_sub(visible_rows);
            let scroll = state.game_inbox_scroll.min(max_scroll);
            let end = lines.len().saturating_sub(scroll);
            let start = end.saturating_sub(visible_rows);
            let visible = &lines[start..end];
            let first_row = transcript_area
                .bottom()
                .saturating_sub(visible.len() as u16);
            for (idx, line) in visible.iter().enumerate() {
                let row = first_row + idx as u16;
                render_thread_line(
                    buffer,
                    row,
                    transcript_area.x,
                    transcript_area.width as usize,
                    line,
                );
            }
        }
    }

    let nick = threads::thread_prompt_label(state);
    let prompt = format!("<{nick}>: ");
    let mut col = prompt_area.x;
    col = write_text(
        buffer,
        prompt_area.y,
        col,
        prompt_area.right(),
        &prompt,
        self_nick_style(),
    );
    let remaining = prompt_area.right().saturating_sub(col) as usize;
    if remaining > 0 {
        buffer.set_stringn(
            col,
            prompt_area.y,
            if state.game_inbox_composing || modal {
                state.game_inbox_message_input.as_str()
            } else {
                "Enter opens thread"
            },
            remaining,
            with_panel_bg(if state.game_inbox_composing || modal {
                styles.value
            } else {
                styles.dim
            }),
        );
    }
}

fn render_thread_line(
    buffer: &mut Buffer,
    row: u16,
    start_col: u16,
    width: usize,
    line: &threads::ThreadRenderLine,
) {
    let styles = theme::tui_theme();
    let mut col = start_col;
    let end = start_col.saturating_add(width as u16);
    if let Some(timestamp) = line.timestamp.as_deref() {
        col = write_text(buffer, row, col, end, timestamp, with_panel_bg(styles.dim));
    }
    if let Some(nick) = line.nick.as_deref() {
        col = write_text(buffer, row, col, end, nick, nick_style_for(line));
    } else {
        col = col.saturating_add(line.indent as u16);
    }
    if col < end {
        let remaining = end.saturating_sub(col) as usize;
        buffer.set_stringn(
            col,
            row,
            &line.body,
            remaining,
            with_panel_bg(styles.value),
        );
    }
}

fn write_text(
    buffer: &mut Buffer,
    row: u16,
    start_col: u16,
    end_col: u16,
    text: &str,
    style: Style,
) -> u16 {
    let remaining = end_col.saturating_sub(start_col) as usize;
    let clipped = text.chars().take(remaining).collect::<String>();
    buffer.set_stringn(start_col, row, &clipped, remaining, style);
    start_col.saturating_add(clipped.chars().count() as u16)
}

fn nick_style_for(line: &threads::ThreadRenderLine) -> Style {
    if line.outgoing {
        return self_nick_style();
    }
    let palette = [
        theme::tui_theme().accent,
        theme::tui_theme().warning,
        theme::tui_theme().error,
        theme::tui_theme().menu_hotkey,
    ];
    let hash = line
        .nick_key
        .bytes()
        .fold(0usize, |acc, byte| acc.wrapping_mul(33).wrapping_add(byte as usize));
    with_panel_bg(palette[hash % palette.len()])
}

fn self_nick_style() -> Style {
    with_panel_bg(theme::tui_theme().success)
}

#[derive(Clone, Copy)]
enum TableCellAlign {
    Left,
    Right,
}

#[derive(Clone, Copy)]
struct TableColumnSpec {
    title_top: Option<&'static str>,
    title: &'static str,
    constraint: Constraint,
    align: TableCellAlign,
}

fn render_joined_games_panel(
    buffer: &mut Buffer,
    area: Rect,
    focused: bool,
    state: &LobbyState,
) {
    const COLUMNS: [TableColumnSpec; 5] = [
        TableColumnSpec {
            title_top: None,
            title: "Status",
            constraint: Constraint::Length(8),
            align: TableCellAlign::Left,
        },
        TableColumnSpec {
            title_top: None,
            title: "Game",
            constraint: Constraint::Fill(1),
            align: TableCellAlign::Left,
        },
        TableColumnSpec {
            title_top: None,
            title: "Seat",
            constraint: Constraint::Length(4),
            align: TableCellAlign::Right,
        },
        TableColumnSpec {
            title_top: None,
            title: "Year",
            constraint: Constraint::Length(4),
            align: TableCellAlign::Right,
        },
        TableColumnSpec {
            title_top: None,
            title: "Turn",
            constraint: Constraint::Length(4),
            align: TableCellAlign::Right,
        },
    ];

    render_table_panel(
        buffer,
        area,
        " JOINED GAMES ",
        focused,
        &COLUMNS,
        1,
        state.joined_games.len(),
        focused_selection(state, LobbyFocus::JoinedGames, state.joined_selected),
        "<no joined hosted games>",
        |index| {
            let row = &state.joined_games[index];
            let (year, turn) = split_turn_summary(&row.turn_summary);
            vec![
                row.status.clone(),
                row.game.clone(),
                row.seat
                    .map(|seat| seat.to_string())
                    .unwrap_or_else(|| "-".to_string()),
                year,
                turn,
            ]
        },
    );
}

fn render_open_games_panel(
    buffer: &mut Buffer,
    area: Rect,
    focused: bool,
    state: &LobbyState,
) {
    const COLUMNS: [TableColumnSpec; 10] = [
        TableColumnSpec {
            title_top: None,
            title: "Status",
            constraint: Constraint::Length(6),
            align: TableCellAlign::Left,
        },
        TableColumnSpec {
            title_top: None,
            title: "Game",
            constraint: Constraint::Fill(2),
            align: TableCellAlign::Left,
        },
        TableColumnSpec {
            title_top: None,
            title: "Host",
            constraint: Constraint::Fill(1),
            align: TableCellAlign::Left,
        },
        TableColumnSpec {
            title_top: None,
            title: "Recruiting",
            constraint: Constraint::Length(11),
            align: TableCellAlign::Left,
        },
        TableColumnSpec {
            title_top: Some("Open"),
            title: "Seats",
            constraint: Constraint::Length(5),
            align: TableCellAlign::Right,
        },
        TableColumnSpec {
            title_top: None,
            title: "Seats",
            constraint: Constraint::Length(5),
            align: TableCellAlign::Right,
        },
        TableColumnSpec {
            title_top: Some("Map"),
            title: "Size",
            constraint: Constraint::Length(5),
            align: TableCellAlign::Right,
        },
        TableColumnSpec {
            title_top: Some("Date"),
            title: "Created",
            constraint: Constraint::Length(10),
            align: TableCellAlign::Right,
        },
        TableColumnSpec {
            title_top: None,
            title: "Year",
            constraint: Constraint::Length(4),
            align: TableCellAlign::Right,
        },
        TableColumnSpec {
            title_top: None,
            title: "Turn",
            constraint: Constraint::Length(4),
            align: TableCellAlign::Right,
        },
    ];

    render_table_panel(
        buffer,
        area,
        " GAMES ",
        focused,
        &COLUMNS,
        2,
        state.open_games.len(),
        focused_selection(state, LobbyFocus::OpenGames, state.open_selected),
        "<no hosted games>",
        |index| {
            let row = &state.open_games[index];
            let (year, turn) = split_turn_summary(&row.turn_summary);
            vec![
                row.status.clone(),
                row.game.clone(),
                row.host.clone(),
                row.recruiting.clone(),
                row.open_seats.to_string(),
                row.total_seats.to_string(),
                map_size_summary(row.total_seats),
                row.created_date.clone(),
                year,
                turn,
            ]
        },
    );
}

fn split_turn_summary(summary: &str) -> (String, String) {
    let mut parts = summary.split_whitespace();
    let year = parts
        .next()
        .map(|part| part.trim_start_matches(['Y', 'y']).to_string())
        .filter(|part| !part.is_empty())
        .unwrap_or_else(|| summary.to_string());
    let turn = parts
        .next()
        .map(|part| part.trim_start_matches(['T', 't']).to_string())
        .unwrap_or_default();
    (year, turn)
}

fn map_size_summary(total_seats: u8) -> String {
    let edge = match total_seats {
        0..=4 => 18,
        5..=9 => 27,
        10..=16 => 36,
        _ => 45,
    };
    format!("{edge}x{edge}")
}

fn render_table_panel(
    buffer: &mut Buffer,
    area: Rect,
    title: &str,
    focused: bool,
    columns: &[TableColumnSpec],
    header_rows: u16,
    row_count: usize,
    selected: Option<usize>,
    empty: &str,
    row_cells: impl Fn(usize) -> Vec<String>,
) {
    let styles = theme::tui_theme();
    let block = panel_block(title, focused);
    let inner = block.inner(area);
    block.render(area, buffer);
    if inner.width == 0 || inner.height == 0 {
        return;
    }

    let [header_area, body_area] =
        Layout::vertical([Constraint::Length(header_rows), Constraint::Min(0)]).areas(inner);
    render_table_header(buffer, header_area, columns);

    if row_count == 0 {
        if body_area.height > 0 {
            buffer.set_stringn(
                body_area.x,
                body_area.y,
                empty,
                body_area.width as usize,
                with_panel_bg(styles.dim),
            );
        }
        return;
    }

    let visible_rows = body_area.height as usize;
    if visible_rows == 0 {
        return;
    }
    let scroll = scroll_offset(row_count, visible_rows, selected.unwrap_or(0));
    for (offset, index) in (scroll..row_count).take(visible_rows).enumerate() {
        let row_area = Rect::new(body_area.x, body_area.y + offset as u16, body_area.width, 1);
        let row_style = if selected == Some(index) {
            styles.selected
        } else {
            with_panel_bg(styles.value)
        };
        render_table_row(buffer, row_area, columns, &row_cells(index), row_style);
    }
}

fn render_table_header(buffer: &mut Buffer, area: Rect, columns: &[TableColumnSpec]) {
    if area.width == 0 || area.height == 0 {
        return;
    }
    let styles = theme::tui_theme();
    for row in area.top()..area.bottom() {
        buffer.set_stringn(
            area.x,
            row,
            &" ".repeat(area.width as usize),
            area.width as usize,
            with_panel_bg(styles.label),
        );
    }
    let top_cells = columns
        .iter()
        .map(|column| column.title_top.unwrap_or(""))
        .collect::<Vec<_>>();
    let bottom_cells = columns.iter().map(|column| column.title).collect::<Vec<_>>();
    if area.height > 1 {
        let top_area = Rect::new(area.x, area.y, area.width, 1);
        render_table_cells(buffer, top_area, columns, &top_cells, with_panel_bg(styles.label));
        let bottom_area = Rect::new(area.x, area.y + area.height - 1, area.width, 1);
        render_table_cells(
            buffer,
            bottom_area,
            columns,
            &bottom_cells,
            with_panel_bg(styles.label),
        );
    } else {
        render_table_cells(
            buffer,
            area,
            columns,
            &bottom_cells,
            with_panel_bg(styles.label),
        );
    }
}

fn render_table_row(
    buffer: &mut Buffer,
    area: Rect,
    columns: &[TableColumnSpec],
    cells: &[String],
    style: Style,
) {
    if area.width == 0 || area.height == 0 {
        return;
    }
    buffer.set_stringn(
        area.x,
        area.y,
        &" ".repeat(area.width as usize),
        area.width as usize,
        style,
    );
    let borrowed = cells.iter().map(String::as_str).collect::<Vec<_>>();
    render_table_cells(buffer, area, columns, &borrowed, style);
}

fn render_table_cells(
    buffer: &mut Buffer,
    area: Rect,
    columns: &[TableColumnSpec],
    cells: &[&str],
    style: Style,
) {
    let cell_areas = Layout::horizontal(columns.iter().map(|column| column.constraint).collect::<Vec<_>>())
        .spacing(1)
        .split(area);
    for ((column, cell), cell_area) in columns.iter().zip(cells.iter()).zip(cell_areas.iter()) {
        if cell_area.width == 0 {
            continue;
        }
        let text_width = cell.chars().count().min(cell_area.width as usize) as u16;
        let start = match column.align {
            TableCellAlign::Left => cell_area.x,
            TableCellAlign::Right => cell_area.right().saturating_sub(text_width),
        };
        buffer.set_stringn(start, cell_area.y, cell, cell_area.width as usize, style);
    }
}

fn focused_selection(state: &LobbyState, target: LobbyFocus, selected: usize) -> Option<usize> {
    (state.focus == target).then_some(selected)
}

fn status_style(tone: LobbyStatusTone) -> Style {
    let styles = theme::tui_theme();
    match tone {
        LobbyStatusTone::Info => styles.border,
        LobbyStatusTone::Success => styles.success,
        LobbyStatusTone::Error => styles.error,
    }
}

fn network_style(status: LobbyNetworkStatus) -> Style {
    let styles = theme::tui_theme();
    match status {
        LobbyNetworkStatus::NoRelay => styles.warning,
        LobbyNetworkStatus::Connecting | LobbyNetworkStatus::Refreshing => styles.accent,
        LobbyNetworkStatus::Connected => styles.value,
        LobbyNetworkStatus::Synced => styles.success,
        LobbyNetworkStatus::Error => styles.error,
    }
}

fn toast_text_style(tone: LobbyStatusTone) -> Style {
    let styles = theme::tui_theme();
    match tone {
        LobbyStatusTone::Info => styles.value,
        LobbyStatusTone::Success => styles.success,
        LobbyStatusTone::Error => styles.error,
    }
}

fn right_align(buffer: &mut Buffer, area: Rect, row: u16, text: &str, style: Style) {
    let width = text.chars().count().min(area.width as usize) as u16;
    let start = area.right().saturating_sub(width);
    buffer.set_stringn(start, row, text, area.width as usize, style);
}

fn render_footer_tokens(buffer: &mut Buffer, area: Rect) {
    let styles = theme::tui_theme();
    let tokens = [
        FooterToken::leading("?", " Help"),
        FooterToken::embedded("I<", "N", ">vite"),
        FooterToken::leading("L", "<ock"),
        FooterToken::leading("M", ">essage"),
        FooterToken::leading("S", ">ettings"),
        FooterToken::leading("R", ">efresh"),
        FooterToken::leading("Q", ">uit"),
    ];
    let gap = 2usize;
    let total_width = tokens.iter().map(FooterToken::width).sum::<usize>()
        + gap * tokens.len().saturating_sub(1);
    let start = area.x + area.width.saturating_sub(total_width as u16) / 2;
    let row = area.y;
    let mut col = start;
    for (idx, token) in tokens.iter().enumerate() {
        if idx > 0 {
            buffer.set_stringn(col, row, "  ", 2, with_panel_bg(styles.menu));
            col += 2;
        }
        col = token.render(
            buffer,
            row,
            col,
            with_panel_bg(styles.menu),
            with_panel_bg(styles.menu_hotkey),
        ) as u16;
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

struct FooterToken {
    prefix: &'static str,
    hotkey: &'static str,
    suffix: &'static str,
}

impl FooterToken {
    const fn leading(hotkey: &'static str, suffix: &'static str) -> Self {
        Self {
            prefix: "",
            hotkey,
            suffix,
        }
    }

    const fn embedded(prefix: &'static str, hotkey: &'static str, suffix: &'static str) -> Self {
        Self {
            prefix,
            hotkey,
            suffix,
        }
    }

    fn width(&self) -> usize {
        self.prefix.chars().count() + self.hotkey.chars().count() + self.suffix.chars().count()
    }

    fn render(&self, buffer: &mut Buffer, row: u16, start: u16, label: Style, hotkey: Style) -> usize {
        let mut col = start;
        if !self.prefix.is_empty() {
            buffer.set_stringn(col, row, self.prefix, self.prefix.len(), label);
            col += self.prefix.chars().count() as u16;
        }
        buffer.set_stringn(col, row, self.hotkey, self.hotkey.len(), hotkey);
        col += self.hotkey.chars().count() as u16;
        if !self.suffix.is_empty() {
            buffer.set_stringn(col, row, self.suffix, self.suffix.len(), label);
            col += self.suffix.chars().count() as u16;
        }
        col as usize
    }
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

fn on_off(value: bool) -> &'static str {
    if value { "ON" } else { "OFF" }
}

trait LobbyAppExt {
    fn state_ref(&self) -> &LobbyState;
}

impl LobbyAppExt for LobbyApp {
    fn state_ref(&self) -> &LobbyState {
        &self.state
    }
}
