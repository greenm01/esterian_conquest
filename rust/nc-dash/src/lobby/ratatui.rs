use nc_ui::PlayfieldBuffer;
use nc_ui::modal::render_modal_box;
use ratatui::buffer::Buffer;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::widgets::{Block, Borders, Clear, Paragraph, Widget, Wrap};

use crate::lobby::state::{LobbyFocus, LobbyNetworkStatus, LobbyState, LobbyStatusTone};
use crate::lobby::threads;
use crate::theme;

const HOME_MIN_WIDTH: u16 = 72;
const HOME_MIN_HEIGHT: u16 = 20;
const HEADER_HEIGHT: u16 = 3;
const FOOTER_HEIGHT: u16 = 3;
const SETTINGS_ROWS: [&str; 6] = [
    "Handle",
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
    let [header_area, body_area, footer_area] = Layout::vertical([
        Constraint::Length(HEADER_HEIGHT),
        Constraint::Min(0),
        Constraint::Length(FOOTER_HEIGHT),
    ])
    .areas(area);
    let [left, center, right] = Layout::horizontal([
        Constraint::Fill(30),
        Constraint::Fill(34),
        Constraint::Fill(36),
    ])
    .spacing(1)
    .areas(body_area);
    let [joined, inbox] = Layout::vertical([
        Constraint::Fill(5),
        Constraint::Fill(3),
    ])
    .spacing(1)
    .areas(left);
    let [notices, thread] =
        Layout::vertical([Constraint::Fill(2), Constraint::Fill(3)]).spacing(1).areas(right);

    HeaderHudWidget { state }.render(header_area, &mut buffer);
    JoinedGamesWidget { state }.render(joined, &mut buffer);
    InboxWidget { state }.render(inbox, &mut buffer);
    OpenGamesWidget { state }.render(center, &mut buffer);
    NoticesWidget { state }.render(notices, &mut buffer);
    ThreadWidget { state }.render(thread, &mut buffer);
    FooterMenuWidget.render(footer_area, &mut buffer);

    if state.status_message.is_some() && !state.show_help {
        ToastOverlayWidget { state }.render(body_area, &mut buffer);
    }

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
            "Handle" => self_or_unset(state.player_handle.as_deref()),
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
        } else if idx >= 4 {
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
            let status_style = toast_text_style(state.status_tone);
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
            styles.title,
        );
        right_align(
            buffer,
            inner,
            inner.y,
            &network_line,
            network_style(self.state.network_status),
        );
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
        let lines = wrap_lines(message, area.width.saturating_sub(8) as usize);
        let height = (lines.len() as u16 + 2).clamp(3, 6);
        let width = lines
            .iter()
            .map(|line| line.chars().count() as u16)
            .max()
            .unwrap_or(0)
            .saturating_add(2)
            .clamp(18, area.width.saturating_sub(4));
        let popup = Rect::new(
            area.x + area.width.saturating_sub(width) / 2,
            area.bottom().saturating_sub(height),
            width,
            height,
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
                toast_text_style(self.state.status_tone),
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
            "Settings   : open settings, including local handle",
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

fn chrome_block(border_style: Style) -> Block<'static> {
    let styles = theme::tui_theme();
    Block::default()
        .borders(Borders::ALL)
        .style(styles.panel)
        .border_style(border_style)
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

fn render_footer_tokens(buffer: &mut Buffer, area: Rect) {
    let styles = theme::tui_theme();
    let tokens = [
        FooterToken::leading("?", " Help"),
        FooterToken::embedded("I<", "N", ">vite"),
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
            buffer.set_stringn(col, row, "  ", 2, styles.menu);
            col += 2;
        }
        col = token.render(buffer, row, col, styles.menu, styles.menu_hotkey) as u16;
    }
}

fn wrap_lines(text: &str, max_width: usize) -> Vec<String> {
    let width = max_width.max(8);
    let mut out = Vec::new();
    for raw in text.lines() {
        let mut current = String::new();
        for word in raw.split_whitespace() {
            let extra = if current.is_empty() { 0 } else { 1 };
            if current.chars().count() + extra + word.chars().count() > width && !current.is_empty() {
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

fn modal_theme() -> nc_ui::modal::ModalTheme {
    nc_ui::modal::ModalTheme {
        body_style: theme::table_body_style(),
        pad_style: theme::body_style(),
        chrome_style: theme::table_chrome_style(),
        title_style: theme::table_header_style(),
    }
}
