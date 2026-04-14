use ratatui::buffer::Buffer;
use ratatui::layout::{Constraint, Rect};
use ratatui::widgets::Widget;

use crate::lobby::state::{LobbyFocus, LobbyState};
use crate::lobby::threads;
use crate::theme;

use super::chrome::{chrome_block, network_style, with_panel_bg};
use super::layout::HomeLayout;
use super::tables::{render_table_panel, TableCellAlign, TableColumnSpec};

pub(super) fn render_home_base(buffer: &mut Buffer, state: &LobbyState, layout: HomeLayout) {
    HeaderHudWidget { state }.render(layout.header, buffer);
    JoinedGamesWidget { state }.render(layout.joined, buffer);
    OpenGamesWidget { state }.render(layout.open, buffer);
    CommsWidget { state }.render(layout.comms, buffer);
    FooterMenuWidget.render(layout.footer, buffer);
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

struct CommsWidget<'a> {
    state: &'a LobbyState,
}

impl Widget for CommsWidget<'_> {
    fn render(self, area: Rect, buffer: &mut Buffer) {
        threads::render_comms_hotlist_panel(
            buffer,
            area,
            self.state.focus == LobbyFocus::Thread,
            self.state,
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
        " MY GAMES ",
        focused,
        &COLUMNS,
        1,
        state.joined_games.len(),
        focused_selection(state, LobbyFocus::JoinedGames, state.joined_selected),
        "<no games yet>",
        |index| {
            let row = &state.joined_games[index];
            let (year, turn) = split_turn_summary(&row.turn_summary);
            vec![
                joined_game_status_label(&row.status).to_string(),
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

fn joined_game_status_label(status: &str) -> &str {
    match status {
        "requested" => "Requested",
        "rejected" => "Rejected",
        "joined" => "Joined",
        other => other,
    }
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

fn focused_selection(state: &LobbyState, target: LobbyFocus, selected: usize) -> Option<usize> {
    (state.focus == target).then_some(selected)
}

fn right_align(buffer: &mut Buffer, area: Rect, row: u16, text: &str, style: ratatui::style::Style) {
    let width = text.chars().count().min(area.width as usize) as u16;
    let start = area.right().saturating_sub(width);
    buffer.set_stringn(start, row, text, area.width as usize, style);
}

fn render_footer_tokens(buffer: &mut Buffer, area: Rect) {
    let styles = theme::tui_theme();
    let tokens = [
        FooterToken::leading("?", " Help"),
        FooterToken::leading("J", ">oin"),
        FooterToken::embedded("Alt-", "L", "ock"),
        FooterToken::leading("T", ">Comms"),
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

    fn render(
        &self,
        buffer: &mut Buffer,
        row: u16,
        start: u16,
        label: ratatui::style::Style,
        hotkey: ratatui::style::Style,
    ) -> usize {
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
