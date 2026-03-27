use ec_ui::buffer::{CellStyle, PlayfieldBuffer};
use ec_ui::prompt::{
    draw_command_line_prompt_text_at, draw_plain_prompt, draw_table_command_bar_at,
};
use ec_ui::theme::classic;

use super::{PickerState, Screen};
use crate::connect::handshake::GameEntry;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Rect {
    pub x: u16,
    pub y: u16,
    pub width: u16,
    pub height: u16,
}

impl Rect {
    pub const fn new(x: u16, y: u16, width: u16, height: u16) -> Self {
        Self {
            x,
            y,
            width,
            height,
        }
    }
}

pub fn render_buffer(state: &PickerState, width: u16, height: u16) -> PlayfieldBuffer {
    let width = width.max(1) as usize;
    let height = height.max(1) as usize;
    let mut buffer = PlayfieldBuffer::new(width, height, classic::body_style());
    if width < 48 || height < 10 {
        render_tiny(&mut buffer, state);
        return buffer;
    }

    draw_title(&mut buffer, "ESTERIAN CONQUEST CONNECT");
    let command_row = height - 1;
    let status_row = height - 2;

    let list_rect = Rect::new(
        1,
        2,
        width.saturating_sub(2) as u16,
        height.saturating_sub(6) as u16,
    );
    draw_box(
        &mut buffer,
        list_rect,
        "Your Games",
        classic::table_chrome_style(),
        classic::table_header_style(),
    );
    draw_game_list(&mut buffer, list_rect, state);

    draw_status_row(&mut buffer, status_row, state.status_msg.as_deref());
    match &state.screen {
        Screen::JoinPrompt => {
            let prompt = format!("Invite code <Q> -> {}", state.join_input);
            draw_command_line_prompt_text_at(&mut buffer, command_row, "CONNECT COMMAND", &prompt);
        }
        Screen::GameSelect {
            games, selected, ..
        } => {
            draw_table_command_bar_at(&mut buffer, command_row, "J K <Q>", None, "");
            draw_game_select_overlay(&mut buffer, games, *selected);
        }
        Screen::IdentityOverlay => {
            draw_table_command_bar_at(
                &mut buffer,
                command_row,
                "J K ^U ^D <N> <I> <M> <Q>",
                None,
                "",
            );
            draw_identity_overlay(&mut buffer, state);
        }
        Screen::GameList => {
            draw_table_command_bar_at(
                &mut buffer,
                command_row,
                "J K ^U ^D <N> <I> <M> <Q>",
                None,
                "",
            );
        }
    }

    buffer
}

fn render_tiny(buffer: &mut PlayfieldBuffer, state: &PickerState) {
    let title = "ec-connect";
    let line = match &state.status_msg {
        Some(msg) => msg.as_str(),
        None => "Terminal too small. Resize or press Q.",
    };
    buffer.write_text_clipped(0, 0, title, classic::title_style());
    if buffer.height() > 1 {
        buffer.write_text_clipped(1, 0, line, classic::notice_style());
    }
}

pub(crate) fn draw_title(buffer: &mut PlayfieldBuffer, title: &str) {
    let row = 0;
    buffer.fill_row(row, classic::title_style());
    let col = buffer.width().saturating_sub(title.chars().count()) / 2;
    buffer.write_text_clipped(row, col, title, classic::title_style());
    if buffer.height() > 1 {
        buffer.fill_row(1, classic::body_style());
    }
}

fn draw_game_list(buffer: &mut PlayfieldBuffer, rect: Rect, state: &PickerState) {
    let inner_x = rect.x as usize + 1;
    let inner_y = rect.y as usize + 1;
    let inner_width = rect.width.saturating_sub(2) as usize;
    let inner_height = rect.height.saturating_sub(2) as usize;
    if inner_width == 0 || inner_height == 0 {
        return;
    }

    let sorted = state.cache.sorted();
    if sorted.is_empty() {
        let msg = "No games yet. Press N to join a game.";
        let row = inner_y + inner_height / 2;
        let col = inner_x + inner_width.saturating_sub(msg.chars().count()) / 2;
        buffer.write_text_clipped(row, col, msg, classic::notice_style());
        return;
    }

    let visible_rows = inner_height;
    let start = scroll_start(state.selected, visible_rows, sorted.len());
    for (screen_row, game) in sorted.iter().skip(start).take(visible_rows).enumerate() {
        let row = inner_y + screen_row;
        let is_selected = start + screen_row == state.selected;
        draw_game_row(buffer, row, inner_x, inner_width, game, is_selected);
    }
}

fn draw_game_row(
    buffer: &mut PlayfieldBuffer,
    row: usize,
    col: usize,
    width: usize,
    game: &crate::cache::CachedGame,
    selected: bool,
) {
    let style = if selected {
        classic::selected_row_style()
    } else {
        classic::table_body_style()
    };
    let muted = if selected {
        classic::selected_row_style()
    } else {
        classic::status_label_style()
    };
    let marker = if selected { ">" } else { " " };
    let time_str = relative_time(game.last_connected.as_deref());
    let right = format!("Seat {:>2}  {}", game.seat, time_str);
    let right_width = right.chars().count().min(width.saturating_sub(4));
    let name_width = width.saturating_sub(4 + right_width + 1).max(8);
    let server_width = width
        .saturating_sub(4 + right_width + 1 + name_width + 3)
        .max(8);
    buffer.fill_row(row, style);
    let mut cursor = col;
    cursor += buffer.write_text_clipped(row, cursor, marker, style);
    cursor += buffer.write_text_clipped(row, cursor, " ", style);
    cursor += buffer.write_text_clipped(row, cursor, &truncate(&game.name, name_width), style);
    cursor += buffer.write_text_clipped(row, cursor, "  ", style);
    let _ = buffer.write_text_clipped(row, cursor, &truncate(&game.server, server_width), muted);
    if right_width < width {
        let right_col = col + width.saturating_sub(right_width);
        buffer.write_text_clipped(row, right_col, &right, muted);
    }
}

fn draw_status_row(buffer: &mut PlayfieldBuffer, row: usize, msg: Option<&str>) {
    buffer.fill_row(row, classic::body_style());
    if let Some(msg) = msg {
        let style = if msg.starts_with("Error:") {
            classic::error_style()
        } else if msg.starts_with("Warning:") {
            classic::alert_style()
        } else {
            classic::notice_style()
        };
        buffer.write_text_clipped(row, 1, msg, style);
    }
}

fn draw_identity_overlay(buffer: &mut PlayfieldBuffer, state: &PickerState) {
    let popup = centered_rect(
        70,
        7,
        Rect::new(0, 0, buffer.width() as u16, buffer.height() as u16),
    );
    draw_box(
        buffer,
        popup,
        "Identity",
        classic::table_chrome_style(),
        classic::table_header_style(),
    );
    let npub_short = short_npub(&state.npub);
    buffer.write_text_clipped(
        popup.y as usize + 2,
        popup.x as usize + 2,
        &format!("Identity: {}", npub_short),
        classic::table_body_style(),
    );
    buffer.write_text_clipped(
        popup.y as usize + 3,
        popup.x as usize + 2,
        &format!("Type: {}", state.identity_type),
        classic::table_body_style(),
    );
    buffer.write_text_clipped(
        popup.y as usize + 4,
        popup.x as usize + 2,
        &format!("Wallet identities: {}", state.identity_count),
        classic::table_body_style(),
    );
    draw_plain_prompt(
        buffer,
        popup.y as usize + popup.height as usize - 2,
        "(slap a key)",
    );
}

fn draw_game_select_overlay(buffer: &mut PlayfieldBuffer, games: &[GameEntry], selected: usize) {
    let height = (games.len() as u16 + 6).clamp(7, 20);
    let popup = centered_rect(
        72,
        height,
        Rect::new(0, 0, buffer.width() as u16, buffer.height() as u16),
    );
    draw_box(
        buffer,
        popup,
        "Choose Game",
        classic::table_chrome_style(),
        classic::table_header_style(),
    );
    let inner_x = popup.x as usize + 2;
    let inner_y = popup.y as usize + 2;
    let inner_width = popup.width.saturating_sub(4) as usize;
    let visible_rows = popup.height.saturating_sub(4) as usize;
    let start = scroll_start(selected, visible_rows, games.len());
    for (idx, game) in games.iter().skip(start).take(visible_rows).enumerate() {
        let row = inner_y + idx;
        let is_selected = start + idx == selected;
        let style = if is_selected {
            classic::selected_row_style()
        } else {
            classic::table_body_style()
        };
        let marker = if is_selected { ">" } else { " " };
        buffer.fill_row(row, style);
        buffer.write_text_clipped(
            row,
            inner_x,
            &format!(
                "{} {} (Seat {})",
                marker,
                truncate(&game.name, inner_width.saturating_sub(10)),
                game.seat
            ),
            style,
        );
    }
}

pub(crate) fn draw_box(
    buffer: &mut PlayfieldBuffer,
    rect: Rect,
    title: &str,
    chrome_style: CellStyle,
    title_style: CellStyle,
) {
    if rect.width < 2 || rect.height < 2 {
        return;
    }
    let left = rect.x as usize;
    let top = rect.y as usize;
    let right = left + rect.width as usize - 1;
    let bottom = top + rect.height as usize - 1;
    for x in left + 1..right {
        buffer.set_cell(top, x, '─', chrome_style);
        buffer.set_cell(bottom, x, '─', chrome_style);
    }
    for y in top + 1..bottom {
        buffer.set_cell(y, left, '│', chrome_style);
        buffer.set_cell(y, right, '│', chrome_style);
    }
    buffer.set_cell(top, left, '┌', chrome_style);
    buffer.set_cell(top, right, '┐', chrome_style);
    buffer.set_cell(bottom, left, '└', chrome_style);
    buffer.set_cell(bottom, right, '┘', chrome_style);
    if !title.is_empty() && rect.width > 4 {
        let title_col = left + 2;
        buffer.write_text_clipped(top, title_col, title, title_style);
    }
}

fn scroll_start(selected: usize, visible_rows: usize, total_rows: usize) -> usize {
    if visible_rows == 0 || total_rows <= visible_rows {
        return 0;
    }
    let half = visible_rows / 2;
    selected
        .saturating_sub(half)
        .min(total_rows.saturating_sub(visible_rows))
}

pub fn truncate(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        s.to_string()
    } else {
        let mut t: String = s.chars().take(max.saturating_sub(1)).collect();
        t.push('…');
        t
    }
}

pub fn relative_time(ts: Option<&str>) -> String {
    let Some(ts) = ts else {
        return "—".to_string();
    };
    match parse_elapsed_secs(ts) {
        None => "connected".to_string(),
        Some(secs) if secs < 60 => "just now".to_string(),
        Some(secs) if secs < 3600 => format!("{} min ago", secs / 60),
        Some(secs) if secs < 86400 => format!("{} hr ago", secs / 3600),
        Some(secs) => format!("{} days ago", secs / 86400),
    }
}

fn parse_elapsed_secs(ts: &str) -> Option<u64> {
    if ts.len() < 19 {
        return None;
    }
    let year: u64 = ts[0..4].parse().ok()?;
    let month: u64 = ts[5..7].parse().ok()?;
    let day: u64 = ts[8..10].parse().ok()?;
    let hour: u64 = ts[11..13].parse().ok()?;
    let min: u64 = ts[14..16].parse().ok()?;
    let sec: u64 = ts[17..19].parse().ok()?;

    if month < 1 || month > 12 || day < 1 || day > 31 || hour > 23 || min > 59 || sec > 59 {
        return None;
    }

    let days_before_year = days_since_epoch_for_year(year);
    let days_in_year = days_before_month(year, month) + (day - 1);
    let total_days = days_before_year + days_in_year;
    let ts_epoch: u64 = total_days * 86400 + hour * 3600 + min * 60 + sec;

    let now_epoch = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .ok()?
        .as_secs();

    now_epoch.checked_sub(ts_epoch)
}

fn days_since_epoch_for_year(year: u64) -> u64 {
    if year < 1970 {
        return 0;
    }
    let y = year - 1970;
    let base = 1970u64;
    let leaps = count_leaps(base, year);
    y * 365 + leaps
}

fn count_leaps(from: u64, to: u64) -> u64 {
    if to <= from {
        return 0;
    }
    let count_leaps_before = |y: u64| -> u64 {
        if y == 0 {
            return 0;
        }
        let y1 = y - 1;
        y1 / 4 - y1 / 100 + y1 / 400
    };
    count_leaps_before(to) - count_leaps_before(from)
}

fn days_before_month(year: u64, month: u64) -> u64 {
    const DAYS: [u64; 12] = [0, 31, 59, 90, 120, 151, 181, 212, 243, 273, 304, 334];
    let m = (month as usize).saturating_sub(1).min(11);
    let base = DAYS[m];
    if month > 2 && is_leap(year) {
        base + 1
    } else {
        base
    }
}

fn is_leap(year: u64) -> bool {
    (year % 4 == 0 && year % 100 != 0) || year % 400 == 0
}

pub fn short_npub(npub: &str) -> String {
    let chars: Vec<char> = npub.chars().collect();
    if chars.len() <= 24 {
        return npub.to_string();
    }
    let head: String = chars[..16].iter().collect();
    let tail: String = chars[chars.len() - 8..].iter().collect();
    format!("{head}…{tail}")
}

pub fn centered_rect(percent_x: u16, height: u16, parent: Rect) -> Rect {
    let popup_width = parent.width * percent_x / 100;
    let x = parent.x + (parent.width.saturating_sub(popup_width)) / 2;
    let y = parent.y + (parent.height.saturating_sub(height)) / 2;
    Rect::new(
        x,
        y,
        popup_width.min(parent.width),
        height.min(parent.height),
    )
}
