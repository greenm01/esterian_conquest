use ratatui::style::Style;
use ratatui::widgets::{Block, Borders, Padding};

use crate::lobby::state::{LobbyNetworkStatus, LobbyStatusTone};
use crate::theme;

pub(crate) fn panel_block<'a>(title: &'a str, focused: bool) -> Block<'a> {
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

pub(super) fn chrome_block(border_style: Style) -> Block<'static> {
    let styles = theme::tui_theme();
    Block::default()
        .borders(Borders::ALL)
        .padding(Padding::uniform(1))
        .style(styles.body)
        .border_style(with_panel_bg(border_style))
}

pub(super) fn popup_block<'a>(title: &'a str, border_style: Style) -> Block<'a> {
    let styles = theme::tui_theme();
    Block::default()
        .borders(Borders::ALL)
        .padding(Padding::uniform(1))
        .title(title)
        .style(styles.body)
        .border_style(with_panel_bg(border_style))
        .title_style(with_panel_bg(styles.title))
}

pub(crate) fn with_panel_bg(style: Style) -> Style {
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

pub(super) fn status_style(tone: LobbyStatusTone) -> Style {
    let styles = theme::tui_theme();
    match tone {
        LobbyStatusTone::Info => styles.border,
        LobbyStatusTone::Success => styles.success,
        LobbyStatusTone::Error => styles.error,
    }
}

pub(super) fn network_style(status: LobbyNetworkStatus) -> Style {
    let styles = theme::tui_theme();
    match status {
        LobbyNetworkStatus::NoRelay => styles.warning,
        LobbyNetworkStatus::Connecting | LobbyNetworkStatus::Refreshing => styles.accent,
        LobbyNetworkStatus::Connected => styles.value,
        LobbyNetworkStatus::Synced => styles.success,
        LobbyNetworkStatus::Error => styles.error,
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

pub(crate) fn truncate_title(text: &str, limit: usize) -> String {
    let trimmed = text.trim();
    if trimmed.chars().count() <= limit {
        return trimmed.to_string();
    }
    let keep = limit.saturating_sub(1);
    format!("{}…", trimmed.chars().take(keep).collect::<String>())
}

pub(crate) fn write_text(
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
