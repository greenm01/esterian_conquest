use crate::ratatui::buffer::Buffer;
use crate::ratatui::layout::{Constraint, Rect};
use crate::ratatui::style::Style;
use crate::ratatui::widgets::Widget;

use crate::lobby::state::{LobbyApp, LobbyState, LobbyTab};
use crate::lobby::threads;
use crate::theme;

use super::chrome::{network_style, shell_block, with_panel_bg};
use super::layout::{HomeLayout, home_tab_content_area};
use super::tables::{TableCellAlign, TableColumnSpec, render_table_panel};

pub(crate) fn hit_test_tabs(
    state: &LobbyState,
    area: Rect,
    col: u16,
    row: u16,
) -> Option<LobbyTab> {
    if area.height < 2 || row != area.y + 1 {
        return None;
    }

    for span in tab_spans(state, Rect::new(area.x, area.y + 1, area.width, 1)) {
        if col >= span.x && col < span.x + span.width {
            return Some(span.tab);
        }
    }
    None
}

struct TabSpec {
    tab: LobbyTab,
    label: String,
    width: u16,
}

#[derive(Clone)]
struct TabSpan {
    tab: LobbyTab,
    text: String,
    x: u16,
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
            width: "[ My Games ]".chars().count() as u16,
        },
        TabSpec {
            tab: LobbyTab::OpenGames,
            label: " Open Games ".to_string(),
            width: "[ Open Games ]".chars().count() as u16,
        },
        TabSpec {
            tab: LobbyTab::Comms,
            label: comms_label,
            width: if unread > 0 {
                format!("[ Comms ({}) ]", unread).chars().count() as u16
            } else {
                "[ Comms ]".chars().count() as u16
            },
        },
    ]
}

fn tab_spans(state: &LobbyState, area: Rect) -> Vec<TabSpan> {
    let specs = get_tab_specs(state);
    let gap = 2u16;
    let total_width =
        specs.iter().map(|spec| spec.width).sum::<u16>() + gap * (specs.len() as u16 - 1);
    let mut x = area.x + area.width.saturating_sub(total_width) / 2;

    specs
        .into_iter()
        .map(|spec| {
            let span = TabSpan {
                tab: spec.tab,
                text: format!("[{}]", spec.label),
                x,
                width: spec.width,
            };
            x += spec.width + gap;
            span
        })
        .collect()
}

pub(super) fn render_home_base(buffer: &mut Buffer, app: &LobbyApp, layout: HomeLayout) {
    let state = &app.state;
    render_shell(buffer, state, layout);
    match state.active_tab {
        LobbyTab::MyGames => {
            render_joined_games_panel(
                buffer,
                home_tab_content_area(layout.body, LobbyTab::MyGames),
                true,
                state,
            );
        }
        LobbyTab::OpenGames => {
            render_open_games_panel(
                buffer,
                home_tab_content_area(layout.body, LobbyTab::OpenGames),
                true,
                state,
            );
        }
        LobbyTab::Comms => {
            threads::render_comms_scene(
                buffer,
                home_tab_content_area(layout.body, LobbyTab::Comms),
                app,
            );
        }
    }
}

fn render_shell(buffer: &mut Buffer, state: &LobbyState, layout: HomeLayout) {
    let styles = theme::tui_theme();
    let block = shell_block(styles.border);
    block.render(layout.shell, buffer);
    render_header(buffer, state, layout.header);
    render_footer_tokens(buffer, layout.footer);
}

fn render_header(buffer: &mut Buffer, state: &LobbyState, area: Rect) {
    if area.width == 0 || area.height == 0 {
        return;
    }
    let styles = theme::tui_theme();
    let network_line = format!("NETWORK: {}", state.network_status.label());
    buffer.set_stringn(
        area.x,
        area.y,
        "NOSTRIAN CONQUEST LOBBY",
        area.width as usize,
        with_panel_bg(styles.title),
    );
    right_align(
        buffer,
        area,
        area.y,
        &network_line,
        with_panel_bg(network_style(state.network_status)),
    );

    if area.height > 1 {
        render_tabs(buffer, state, Rect::new(area.x, area.y + 1, area.width, 1));
    }
}

fn render_tabs(buffer: &mut Buffer, state: &LobbyState, area: Rect) {
    for span in tab_spans(state, area) {
        let styles = theme::tui_theme();
        let is_active = state.active_tab == span.tab;
        let has_unread = span.tab == LobbyTab::Comms && state.thread_unread_total() > 0;

        let style = if is_active {
            styles.selected
        } else if has_unread {
            styles.accent
        } else {
            styles.label
        };

        buffer.set_stringn(
            span.x,
            area.y,
            &span.text,
            area_width_limit(span.x, span.text.len(), buffer),
            with_panel_bg(style),
        );
    }
}

fn area_width_limit(x: u16, len: usize, buffer: &Buffer) -> usize {
    let remaining = buffer.area.width.saturating_sub(x) as usize;
    len.min(remaining)
}

fn render_joined_games_panel(buffer: &mut Buffer, area: Rect, focused: bool, state: &LobbyState) {
    const COLUMNS: [TableColumnSpec; 5] = [
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
            title: "Type",
            constraint: Constraint::Length(9),
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
        " MY GAMES ",
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
                row.game_tier.clone(),
                row.seat
                    .map(|seat| seat.to_string())
                    .unwrap_or_else(|| "-".to_string()),
                format!("Y{}:T{}", year, turn),
            ]
        },
    );
}

fn render_open_games_panel(buffer: &mut Buffer, area: Rect, focused: bool, state: &LobbyState) {
    const COLUMNS: [TableColumnSpec; 8] = [
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
            title: "Type",
            constraint: Constraint::Length(9),
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
        "<no open games - check back later or ask the sysop in COMMS>",
        |index| {
            let row = &state.open_games[index];
            let (year, turn) = split_turn_summary(&row.turn_summary);
            vec![
                row.status.clone(),
                row.game.clone(),
                row.host.clone(),
                row.game_tier.clone(),
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
        "expired" => "Expired",
        "final" => "Final",
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

fn right_align(
    buffer: &mut Buffer,
    area: Rect,
    row: u16,
    text: &str,
    style: Style,
) {
    let width = text.chars().count().min(area.width as usize) as u16;
    let start = area.right().saturating_sub(width);
    buffer.set_stringn(start, row, text, area.width as usize, style);
}

fn render_footer_tokens(buffer: &mut Buffer, area: Rect) {
    let styles = theme::tui_theme();
    let tokens = [
        FooterToken::leading("Tab", " Next Tab"),
        FooterToken::leading("?", " Keys"),
        FooterToken::leading("H", ">elp"),
        FooterToken::leading("J", ">oin"),
        FooterToken::embedded("Alt-", "L", "ock"),
        FooterToken::embedded("Alt-", "Q", "uit"),
        FooterToken::leading("S", ">ettings"),
        FooterToken::leading("R", ">efresh"),
    ];
    let gap = 2usize;
    let total_width =
        tokens.iter().map(FooterToken::width).sum::<usize>() + gap * tokens.len().saturating_sub(1);
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
        label: Style,
        hotkey: Style,
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
