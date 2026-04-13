use nc_ui::PlayfieldBuffer;
use nc_ui::modal::render_modal_box;
use ratatui::buffer::Buffer;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::widgets::{Block, Borders, Clear, Paragraph, Widget, Wrap};

use crate::lobby::state::{LobbyFocus, LobbyNetworkStatus, LobbyState};
use crate::lobby::threads;
use crate::theme;

const HOME_MIN_WIDTH: u16 = 72;
const HOME_MIN_HEIGHT: u16 = 20;
const HUD_HEIGHT: u16 = 2;
const COMMAND_BAR_HEIGHT: u16 = 4;
const SETTINGS_ROWS: [&str; 5] = [
    "Mouse Follow",
    "Grid Dots",
    "Theme",
    "Save",
    "Cancel",
];

pub fn render_home(playfield: &mut PlayfieldBuffer, state: &LobbyState) {
    let width = playfield.width() as u16;
    let height = playfield.height() as u16;
    if width < HOME_MIN_WIDTH || height < HOME_MIN_HEIGHT {
        let lines = vec![
            "nc-lobby needs a larger window.".to_string(),
            "Resize and reopen the lobby.".to_string(),
        ];
        let _ = render_modal_box(playfield, "WINDOW TOO SMALL", &lines, modal_theme());
        return;
    }

    let area = Rect::new(0, 0, width, height);
    let mut buffer = Buffer::empty(area);
    let [hud_area, body_area] = Layout::vertical([Constraint::Length(HUD_HEIGHT), Constraint::Min(0)])
        .areas(area);
    let [left, center, right] = Layout::horizontal([
        Constraint::Fill(30),
        Constraint::Fill(34),
        Constraint::Fill(36),
    ])
    .spacing(1)
    .areas(body_area);
    let [joined, inbox, commands] = Layout::vertical([
        Constraint::Fill(5),
        Constraint::Fill(3),
        Constraint::Length(COMMAND_BAR_HEIGHT),
    ])
    .spacing(1)
    .areas(left);
    let [notices, thread] =
        Layout::vertical([Constraint::Fill(2), Constraint::Fill(3)]).spacing(1).areas(right);

    HeaderHudWidget { state }.render(hud_area, &mut buffer);
    JoinedGamesWidget { state }.render(joined, &mut buffer);
    InboxWidget { state }.render(inbox, &mut buffer);
    CommandBarWidget { state }.render(commands, &mut buffer);
    OpenGamesWidget { state }.render(center, &mut buffer);
    NoticesWidget { state }.render(notices, &mut buffer);
    ThreadWidget { state }.render(thread, &mut buffer);

    if state.show_help {
        let popup = centered_popup(
            body_area,
            body_area.width.saturating_sub(8).min(72),
            13,
        );
        Clear.render(popup, &mut buffer);
        HelpPopupWidget.render(popup, &mut buffer);
    }

    paint_buffer(playfield, &buffer);
}

pub fn render_settings(playfield: &mut PlayfieldBuffer, state: &LobbyState) {
    let Some(area) = content_area(playfield) else {
        return;
    };
    let mut buffer = Buffer::empty(area);
    let styles = theme::tui_theme();
    let outer = Block::default()
        .borders(Borders::ALL)
        .title(" LOBBY SETTINGS ")
        .style(styles.panel)
        .border_style(styles.border)
        .title_style(styles.title);
    let inner = outer.inner(area);
    outer.render(area, &mut buffer);

    for (idx, label) in SETTINGS_ROWS.iter().enumerate() {
        let row = inner.y + idx as u16;
        if row >= inner.bottom() {
            break;
        }
        let value = match *label {
            "Mouse Follow" => on_off(state.settings_draft.follow_mouse_on_map).to_string(),
            "Grid Dots" => on_off(state.settings_draft.dense_empty_sector_dots).to_string(),
            "Theme" => theme::display_name_for_key(&state.settings_draft.theme_key),
            _ => String::new(),
        };
        let prefix = if state.settings_selected == idx { ">" } else { " " };
        let line = if value.is_empty() {
            format!("{prefix} {label}")
        } else {
            format!("{prefix} {label:<12} : {value}")
        };
        let style = if state.settings_selected == idx {
            styles.selected
        } else if idx >= 3 {
            styles.accent
        } else {
            styles.value
        };
        buffer.set_stringn(inner.x + 1, row, line, inner.width.saturating_sub(2) as usize, style);
    }

    if inner.height > SETTINGS_ROWS.len() as u16 + 1 {
        let info_row = inner.y + SETTINGS_ROWS.len() as u16 + 1;
        buffer.set_stringn(
            inner.x + 1,
            info_row,
            "Theme selection previews immediately and applies to the hosted dashboard too.",
            inner.width.saturating_sub(2) as usize,
            styles.dim,
        );
        if let Some(status) = state.status_message.as_deref() {
            let status_style = if status.to_ascii_lowercase().contains("fail") {
                styles.error
            } else {
                styles.success
            };
            let row = info_row.saturating_add(2);
            if row < inner.bottom() {
                buffer.set_stringn(
                    inner.x + 1,
                    row,
                    status,
                    inner.width.saturating_sub(2) as usize,
                    status_style,
                );
            }
        }
    }

    paint_buffer(playfield, &buffer);
}

pub fn render_theme_picker(playfield: &mut PlayfieldBuffer, state: &LobbyState) {
    let Some(area) = content_area(playfield) else {
        return;
    };
    let mut buffer = Buffer::empty(area);
    let styles = theme::tui_theme();
    let outer = Block::default()
        .borders(Borders::ALL)
        .title(" THEME PICKER ")
        .style(styles.panel)
        .border_style(styles.border)
        .title_style(styles.title);
    let inner = outer.inner(area);
    outer.render(area, &mut buffer);

    let [list_area, preview_area] = Layout::horizontal([
        Constraint::Percentage(52),
        Constraint::Percentage(48),
    ])
    .areas(inner);

    let list_block = Block::default()
        .borders(Borders::ALL)
        .title(" Themes ")
        .style(styles.panel)
        .border_style(styles.border)
        .title_style(styles.title);
    let preview_block = Block::default()
        .borders(Borders::ALL)
        .title(" Preview ")
        .style(styles.panel)
        .border_style(styles.border)
        .title_style(styles.title);
    let list_inner = list_block.inner(list_area);
    let preview_inner = preview_block.inner(preview_area);
    list_block.render(list_area, &mut buffer);
    preview_block.render(preview_area, &mut buffer);

    let themes = state.available_themes();
    let visible_rows = list_inner.height as usize;
    let scroll = state
        .theme_selected
        .saturating_sub(visible_rows.saturating_sub(1));
    for (offset, entry) in themes.iter().skip(scroll).take(visible_rows).enumerate() {
        let row = list_inner.y + offset as u16;
        let absolute_index = scroll + offset;
        let prefix = if absolute_index == state.theme_selected {
            ">"
        } else {
            " "
        };
        let line = format!("{prefix} {}", entry.display_name);
        let style = if absolute_index == state.theme_selected {
            styles.selected
        } else if entry.key == state.settings_draft.theme_key {
            styles.accent
        } else {
            styles.value
        };
        buffer.set_stringn(
            list_inner.x + 1,
            row,
            line,
            list_inner.width.saturating_sub(2) as usize,
            style,
        );
    }

    let preview_lines = [
        format!(
            "Current : {}",
            theme::display_name_for_key(&state.settings_draft.theme_key)
        ),
        format!("Key     : {}", state.settings_draft.theme_key),
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
            0 => styles.label,
            1 => styles.dim,
            3 => styles.accent,
            4 => styles.value,
            5 => styles.selected,
            _ => styles.value,
        };
        buffer.set_stringn(
            preview_inner.x + 1,
            row,
            line,
            preview_inner.width.saturating_sub(2) as usize,
            style,
        );
    }

    paint_buffer(playfield, &buffer);
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
        let handle = self
            .state
            .player_handle
            .as_deref()
            .unwrap_or("unset")
            .to_ascii_uppercase();
        let relay = self
            .state
            .relay_label()
            .unwrap_or_else(|| "relay: not set".to_string())
            .to_ascii_uppercase();
        let handle_line = format!("HANDLE: {handle}");
        let relay_line = relay.replace("relay:", "RELAY:");
        let network_line = format!("NETWORK: {}", self.state.network_status.label());

        buffer.set_stringn(area.x, area.y, "NOSTRIAN CONQUEST LOBBY", area.width as usize, styles.title);
        if area.height > 1 {
            buffer.set_stringn(area.x, area.y + 1, network_line, area.width as usize, network_style(self.state.network_status));
        }
        right_align(buffer, area, area.y, &handle_line, styles.label);
        if area.height > 1 {
            right_align(buffer, area, area.y + 1, &relay_line, styles.dim);
        }
    }
}

struct JoinedGamesWidget<'a> {
    state: &'a LobbyState,
}

impl Widget for JoinedGamesWidget<'_> {
    fn render(self, area: Rect, buffer: &mut Buffer) {
        let rows = self
            .state
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
        render_rows_panel(
            buffer,
            area,
            " JOINED GAMES ",
            &rows,
            focused_selection(self.state, LobbyFocus::JoinedGames, self.state.joined_selected),
            self.state.focus == LobbyFocus::JoinedGames,
            "<no joined hosted games>",
        );
    }
}

struct InboxWidget<'a> {
    state: &'a LobbyState,
}

impl Widget for InboxWidget<'_> {
    fn render(self, area: Rect, buffer: &mut Buffer) {
        let rows = self
            .state
            .inbox
            .iter()
            .map(|item| {
                format!(
                    "{} | {} | {} | {}",
                    item.kind, item.game, item.status, item.message
                )
            })
            .collect::<Vec<_>>();
        render_rows_panel(
            buffer,
            area,
            " INBOX ",
            &rows,
            focused_selection(self.state, LobbyFocus::Inbox, self.state.inbox_selected),
            self.state.focus == LobbyFocus::Inbox,
            "<no inbox activity>",
        );
    }
}

struct OpenGamesWidget<'a> {
    state: &'a LobbyState,
}

impl Widget for OpenGamesWidget<'_> {
    fn render(self, area: Rect, buffer: &mut Buffer) {
        let rows = self
            .state
            .open_games
            .iter()
            .map(|row| {
                format!(
                    "{} | {} | {} | {} seats | {}",
                    row.game, row.host, row.recruiting, row.open_seats, row.turn_summary
                )
            })
            .collect::<Vec<_>>();
        render_rows_panel(
            buffer,
            area,
            " OPEN GAMES ",
            &rows,
            focused_selection(self.state, LobbyFocus::OpenGames, self.state.open_selected),
            self.state.focus == LobbyFocus::OpenGames,
            "<no recruiting hosted games>",
        );
    }
}

struct NoticesWidget<'a> {
    state: &'a LobbyState,
}

impl Widget for NoticesWidget<'_> {
    fn render(self, area: Rect, buffer: &mut Buffer) {
        let rows = threads::notice_rows(self.state);
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
        let rows = threads::thread_rows(self.state);
        render_rows_panel(
            buffer,
            area,
            " THREAD ",
            &rows,
            focused_selection(self.state, LobbyFocus::Thread, self.state.thread_selected),
            self.state.focus == LobbyFocus::Thread,
            "<no private thread messages>",
        );
    }
}

struct CommandBarWidget<'a> {
    state: &'a LobbyState,
}

impl Widget for CommandBarWidget<'_> {
    fn render(self, area: Rect, buffer: &mut Buffer) {
        let styles = theme::tui_theme();
        let block = panel_block(" COMMANDS ", false);
        let inner = block.inner(area);
        block.render(area, buffer);
        if inner.width == 0 || inner.height == 0 {
            return;
        }

        let commands =
            "Tab cycle  J/K move  Enter action  N invite  M thread  H handle  S settings  R refresh  ? help  Q quit";
        buffer.set_stringn(inner.x, inner.y, commands, inner.width as usize, styles.accent);

        if inner.height > 1 {
            let status = self
                .state
                .status_message
                .clone()
                .unwrap_or_else(|| default_command_status(self.state));
            buffer.set_stringn(
                inner.x,
                inner.y + 1,
                status,
                inner.width as usize,
                status_style(self.state.status_message.as_deref(), self.state.network_status),
            );
        }
    }
}

struct HelpPopupWidget;

impl Widget for HelpPopupWidget {
    fn render(self, area: Rect, buffer: &mut Buffer) {
        let styles = theme::tui_theme();
        let lines = [
            "Tab        : cycle focus across lobby panels",
            "J / K      : move within the focused panel",
            "Enter      : open selected joined game or request selected open game",
            "N          : compose an invite request",
            "M          : compose a private thread message",
            "H          : edit your local handle",
            "S          : open lobby settings",
            "R          : refresh the hosted lobby",
            "? / Esc    : close this help popup",
            "Q          : quit nc-dash from the lobby",
        ]
        .join("\n");
        Paragraph::new(lines)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(" LOBBY HELP ")
                    .style(styles.panel)
                    .border_style(styles.accent)
                    .title_style(styles.title),
            )
            .style(styles.value)
            .wrap(Wrap { trim: false })
            .render(area, buffer);
    }
}

fn content_area(playfield: &PlayfieldBuffer) -> Option<Rect> {
    let width = playfield.width() as u16;
    let height = playfield.height() as u16;
    if width < 20 || height < 6 {
        return None;
    }
    Some(Rect::new(0, 1, width, height.saturating_sub(2)))
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
        buffer.set_stringn(inner.x, inner.y, empty, inner.width as usize, styles.dim);
        return;
    }

    let visible = inner.height as usize;
    let scroll = selected
        .unwrap_or(0)
        .saturating_sub(visible.saturating_sub(1))
        .min(rows.len().saturating_sub(visible));
    for (offset, row) in rows.iter().skip(scroll).take(visible).enumerate() {
        let absolute = scroll + offset;
        let style = if selected == Some(absolute) {
            styles.selected
        } else {
            styles.value
        };
        buffer.set_stringn(inner.x, inner.y + offset as u16, row, inner.width as usize, style);
    }
}

fn panel_block<'a>(title: &'a str, focused: bool) -> Block<'a> {
    let styles = theme::tui_theme();
    let border = if focused { styles.accent } else { styles.border };
    let title_style = if focused { styles.selected } else { styles.title };
    Block::default()
        .borders(Borders::ALL)
        .title(title)
        .style(styles.panel)
        .border_style(border)
        .title_style(title_style)
}

fn focused_selection(state: &LobbyState, target: LobbyFocus, selected: usize) -> Option<usize> {
    (state.focus == target).then_some(selected)
}

fn default_command_status(state: &LobbyState) -> String {
    if state.show_help {
        return "Press ? or Esc to close help.".to_string();
    }
    match state.focus {
        LobbyFocus::JoinedGames => {
            "Enter opens the selected hosted game or claims an approved invite.".to_string()
        }
        LobbyFocus::Inbox => "Inbox items track request, claim, and turn receipts.".to_string(),
        LobbyFocus::OpenGames => {
            "Enter or N sends an invite request for the selected hosted game.".to_string()
        }
        LobbyFocus::Notices => "Public notices come from nc-host and the hosted lobby.".to_string(),
        LobbyFocus::Thread => "M writes a private message to the selected game's sysop thread.".to_string(),
    }
}

fn status_style(status: Option<&str>, network_status: LobbyNetworkStatus) -> Style {
    let styles = theme::tui_theme();
    match status {
        Some(message) if message.to_ascii_lowercase().contains("fail") => styles.error,
        Some(_) => styles.value,
        None if network_status == LobbyNetworkStatus::Error => styles.error,
        None => styles.dim,
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

fn centered_popup(area: Rect, width: u16, height: u16) -> Rect {
    let popup_width = width.min(area.width.saturating_sub(2)).max(10);
    let popup_height = height.min(area.height.saturating_sub(2)).max(5);
    let x = area.x + area.width.saturating_sub(popup_width) / 2;
    let y = area.y + area.height.saturating_sub(popup_height) / 2;
    Rect::new(x, y, popup_width, popup_height)
}

fn right_align(buffer: &mut Buffer, area: Rect, row: u16, text: &str, style: Style) {
    let width = text.chars().count().min(area.width as usize) as u16;
    let start = area.right().saturating_sub(width);
    buffer.set_stringn(start, row, text, area.width as usize, style);
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

fn modal_theme() -> nc_ui::modal::ModalTheme {
    nc_ui::modal::ModalTheme {
        body_style: theme::table_body_style(),
        pad_style: theme::body_style(),
        chrome_style: theme::table_chrome_style(),
        title_style: theme::table_header_style(),
    }
}
