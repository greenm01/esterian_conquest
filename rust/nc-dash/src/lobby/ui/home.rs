use ratatui::buffer::Buffer;
use ratatui::layout::{Constraint, Rect};
use ratatui::widgets::Widget;

use crate::lobby::state::{LobbyApp, LobbyState, LobbyTab};
use crate::lobby::threads;
use crate::theme;

use super::chrome::{chrome_block, network_style, with_panel_bg};
use super::layout::HomeLayout;
use super::tables::{render_table_panel, TableCellAlign, TableColumnSpec};

pub(super) fn hit_test_tabs(state: &LobbyState, area: Rect, col: u16, row: u16) -> Option<LobbyTab> {
    let block = chrome_block(ratatui::style::Style::default());
    let inner = block.inner(area);
    if inner.height <= 1 || row != inner.y + 1 {
        return None;
    }
    let specs = get_tab_specs(state);
    let gap = 2u16;
    let total_width: u16 = specs.iter().map(|s| s.width).sum::<u16>() + gap * (specs.len() as u16 - 1);
    let mut current_col = inner.x + inner.width.saturating_sub(total_width) / 2;

    for spec in specs {
        if col >= current_col && col < current_col + spec.width {
            return Some(spec.tab);
        }
        current_col += spec.width + gap;
    }
    None
}

struct TabSpec {
    tab: LobbyTab,
    label: String,
    width: u16,
}

fn get_tab_specs(state: &LobbyState) -> Vec<TabSpec> {
    let unread = state.thread_unread_total();
    let comms_label = if unread > 0 {
        format!(" Comms ({}) ", unread)
    } else {
        " Comms ".to_string()
    };
    vec![
        TabSpec {
            tab: LobbyTab::MyGames,
            label: " My Games ".to_string(),
            width: 12,
        },
        TabSpec {
            tab: LobbyTab::OpenGames,
            label: " Open Games ".to_string(),
            width: 14,
        },
        TabSpec {
            tab: LobbyTab::Comms,
            label: comms_label,
            width: 9 + if unread > 0 { unread.to_string().len() as u16 + 2 } else { 0 },
        },
    ]
}

pub(super) fn render_home_base(buffer: &mut Buffer, app: &LobbyApp, layout: HomeLayout) {
    let state = &app.state;
    HeaderHudWidget { state }.render(layout.header, buffer);
    match state.active_tab {
        LobbyTab::MyGames => {
            render_joined_games_panel(buffer, layout.body, true, state);
        }
        LobbyTab::OpenGames => {
            render_open_games_panel(buffer, layout.body, true, state);
        }
        LobbyTab::Comms => {
            threads::render_comms_scene(buffer, layout.body, app);
        }
    }
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

        if inner.height > 1 {
            self.render_tabs(Rect::new(inner.x, inner.y + 1, inner.width, 1), buffer);
        }
    }
}

impl HeaderHudWidget<'_> {
    fn render_tabs(&self, area: Rect, buffer: &mut Buffer) {
        let specs = get_tab_specs(self.state);
        let gap = 2u16;
        let total_width: u16 =
            specs.iter().map(|s| s.width).sum::<u16>() + gap * (specs.len() as u16 - 1);
        let mut col = area.x + area.width.saturating_sub(total_width) / 2;

        for spec in specs {
            self.render_tab(buffer, col, area.y, &spec.label, spec.tab);
            col += spec.width + gap;
        }
    }

    fn render_tab(&self, buffer: &mut Buffer, x: u16, y: u16, label: &str, tab: LobbyTab) -> u16 {
        let styles = theme::tui_theme();
        let is_active = self.state.active_tab == tab;
        let has_unread = tab == LobbyTab::Comms && self.state.thread_unread_total() > 0;
        
        let style = if is_active {
            styles.selected
        } else if has_unread {
            styles.accent
        } else {
            styles.label
        };

        let text = format!("[{}]", label);
        buffer.set_stringn(x, y, &text, area_width_limit(x, text.len(), buffer), with_panel_bg(style));
        x + text.chars().count() as u16
    }
}

fn area_width_limit(x: u16, len: usize, buffer: &Buffer) -> usize {
    let remaining = buffer.area.width.saturating_sub(x) as usize;
    len.min(remaining)
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
    const COLUMNS: [TableColumnSpec; 4] = [
        TableColumnSpec {
            title_top: None,
            title: "Status",
            constraint: Constraint::Length(10),
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
            constraint: Constraint::Length(6),
            align: TableCellAlign::Right,
        },
        TableColumnSpec {
            title_top: None,
            title: "Time (Y:T)",
            constraint: Constraint::Length(12),
            align: TableCellAlign::Right,
        },
    ];

    render_table_panel(
        buffer,
        area,
        " MY ACTIVE GAMES ",
        focused,
        &COLUMNS,
        1,
        state.joined_games.len(),
        Some(state.joined_selected),
        "<no games yet - press 'j' to join an open game>",
        |index| {
            let row = &state.joined_games[index];
            let (year, turn) = split_turn_summary(&row.turn_summary);
            vec![
                joined_game_status_label(&row.status).to_string(),
                row.game.clone(),
                row.seat
                    .map(|seat| seat.to_string())
                    .unwrap_or_else(|| "-".to_string()),
                format!("Y{}:T{}", year, turn),
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
    const COLUMNS: [TableColumnSpec; 7] = [
        TableColumnSpec {
            title_top: None,
            title: "Status",
            constraint: Constraint::Length(10),
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
            title: "Seats",
            constraint: Constraint::Length(8),
            align: TableCellAlign::Right,
        },
        TableColumnSpec {
            title_top: None,
            title: "Map",
            constraint: Constraint::Length(8),
            align: TableCellAlign::Right,
        },
        TableColumnSpec {
            title_top: None,
            title: "Created",
            constraint: Constraint::Length(12),
            align: TableCellAlign::Right,
        },
        TableColumnSpec {
            title_top: None,
            title: "Time",
            constraint: Constraint::Length(10),
            align: TableCellAlign::Right,
        },
    ];

    render_table_panel(
        buffer,
        area,
        " OPEN GAMES AVAILABLE TO JOIN ",
        focused,
        &COLUMNS,
        1,
        state.open_games.len(),
        Some(state.open_selected),
        "<no open games - press 'h' to host a new game>",
        |index| {
            let row = &state.open_games[index];
            let (year, turn) = split_turn_summary(&row.turn_summary);
            vec![
                row.status.clone(),
                row.game.clone(),
                row.host.clone(),
                format!("{}/{}", row.open_seats, row.total_seats),
                map_size_summary(row.total_seats),
                row.created_date.clone(),
                format!("Y{}:T{}", year, turn),
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
        .unwrap_or_else(|| "0".to_string());
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

fn right_align(buffer: &mut Buffer, area: Rect, row: u16, text: &str, style: ratatui::style::Style) {
    let width = text.chars().count().min(area.width as usize) as u16;
    let start = area.right().saturating_sub(width);
    buffer.set_stringn(start, row, text, area.width as usize, style);
}

fn render_footer_tokens(buffer: &mut Buffer, area: Rect) {
    let styles = theme::tui_theme();
    let tokens = [
        FooterToken::leading("Tab", " Next Tab"),
        FooterToken::leading("?", " Help"),
        FooterToken::leading("J", ">oin"),
        FooterToken::embedded("Alt-", "L", "ock"),
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
