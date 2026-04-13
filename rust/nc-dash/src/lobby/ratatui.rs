use nc_ui::PlayfieldBuffer;
use ratatui::buffer::Buffer;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::Modifier;
use ratatui::widgets::{Block, Borders, Widget};

use crate::lobby::state::LobbyState;
use crate::theme;

const SETTINGS_ROWS: [&str; 5] = [
    "Mouse Follow",
    "Grid Dots",
    "Theme",
    "Save",
    "Cancel",
];

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

    let [list_area, preview_area] = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(52), Constraint::Percentage(48)])
        .split(inner)[..2]
        .try_into()
        .expect("two theme picker columns");

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

fn content_area(playfield: &PlayfieldBuffer) -> Option<Rect> {
    let width = playfield.width() as u16;
    let height = playfield.height() as u16;
    if width < 20 || height < 6 {
        return None;
    }
    Some(Rect::new(0, 1, width, height.saturating_sub(2)))
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
