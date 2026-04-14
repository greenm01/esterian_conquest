use ratatui::style::Style;
use ratatui::widgets::{Block, Borders, Padding};

use crate::lobby::state::{LobbyNetworkStatus, LobbyStatusTone};
use crate::theme;

pub fn panel_block<'a>(title: &'a str, focused: bool) -> Block<'a> {
    let styles = theme::tui_theme();
    let border = if focused { styles.accent } else { styles.border };
    let title_style = if focused { styles.selected } else { styles.title };
    Block::default()
        .borders(Borders::ALL)
        .padding(Padding::horizontal(1))
        .title(title)
        .style(styles.body)
        .border_style(with_panel_bg(border))
        .title_style(with_panel_bg(title_style))
}

pub(super) fn chrome_block(border_style: Style) -> Block<'static> {
    let styles = theme::tui_theme();
    Block::default()
        .borders(Borders::ALL)
        .padding(Padding::horizontal(1))
        .style(styles.body)
        .border_style(with_panel_bg(border_style))
}

pub(super) fn popup_block<'a>(title: &'a str, border_style: Style) -> Block<'a> {
    let styles = theme::tui_theme();
    Block::default()
        .borders(Borders::ALL)
        .padding(Padding::horizontal(1))
        .title(title)
        .style(styles.body)
        .border_style(with_panel_bg(border_style))
        .title_style(with_panel_bg(styles.title))
}

pub(super) fn network_style(status: LobbyNetworkStatus) -> Style {
    let styles = theme::tui_theme();
    match status {
        LobbyNetworkStatus::NoRelay => styles.dim,
        LobbyNetworkStatus::Connecting => styles.label,
        LobbyNetworkStatus::Connected => styles.success,
        LobbyNetworkStatus::Refreshing => styles.accent,
        LobbyNetworkStatus::Synced => styles.success,
        LobbyNetworkStatus::Error => styles.error,
    }
}

pub(super) fn status_style(tone: LobbyStatusTone) -> Style {
    let styles = theme::tui_theme();
    match tone {
        LobbyStatusTone::Info => styles.label,
        LobbyStatusTone::Success => styles.success,
        LobbyStatusTone::Error => styles.error,
    }
}

pub(super) fn toast_text_style(tone: LobbyStatusTone) -> Style {
    let styles = theme::tui_theme();
    match tone {
        LobbyStatusTone::Info => styles.value,
        LobbyStatusTone::Success => styles.success,
        LobbyStatusTone::Error => styles.error,
    }
}

pub fn truncate_title(title: &str, limit: usize) -> String {
    let trimmed = title.trim();
    if trimmed.chars().count() <= limit {
        return trimmed.to_string();
    }
    let keep = limit.saturating_sub(1);
    format!("{}…", trimmed.chars().take(keep).collect::<String>())
}

pub fn write_text(
    buffer: &mut ratatui::buffer::Buffer,
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

pub fn with_panel_bg(style: Style) -> Style {
    let styles = theme::tui_theme();
    if let Some(bg) = styles.body.bg {
        style.bg(bg)
    } else {
        style
    }
}
