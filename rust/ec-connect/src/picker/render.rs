//! Ratatui draw functions for the picker TUI.

use ratatui::Frame;
use ratatui::layout::{Alignment, Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph};

use super::{PickerState, Screen};
use crate::connect::handshake::GameEntry;

/// Draw the full picker UI into `frame`.
pub fn draw(frame: &mut Frame, state: &PickerState) {
    let area = frame.area();

    // Split into header, game list, and footer.
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(2), // header
            Constraint::Min(3),    // game list
            Constraint::Length(3), // footer / prompt
        ])
        .split(area);

    draw_header(frame, chunks[0]);
    draw_game_list(frame, chunks[1], state);
    draw_footer(frame, chunks[2], state);

    // Overlay on top of everything if needed.
    match &state.screen {
        super::Screen::IdentityOverlay => {
            draw_identity_overlay(frame, area, state);
        }
        super::Screen::GameSelect {
            games, selected, ..
        } => {
            draw_game_select_overlay(frame, area, games, *selected);
        }
        _ => {}
    }
}

// ── Header ────────────────────────────────────────────────────────────────────

fn draw_header(frame: &mut Frame, area: Rect) {
    let title = Paragraph::new("ESTERIAN CONQUEST  ──  CONNECT")
        .style(Style::default().add_modifier(Modifier::BOLD))
        .alignment(Alignment::Center);
    frame.render_widget(title, area);
}

// ── Game list ─────────────────────────────────────────────────────────────────

fn draw_game_list(frame: &mut Frame, area: Rect, state: &PickerState) {
    let block = Block::default().title(" Your Games ").borders(Borders::ALL);
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let sorted = state.cache.sorted();

    if sorted.is_empty() {
        let msg = Paragraph::new("No games yet. Press J to join a game.")
            .style(Style::default().fg(Color::DarkGray))
            .alignment(Alignment::Center);
        frame.render_widget(msg, inner);
        return;
    }

    let rows: Vec<Line> = sorted
        .iter()
        .enumerate()
        .map(|(i, g)| {
            let cursor = if i == state.selected { "> " } else { "  " };
            let time_str = relative_time(g.last_connected.as_deref());
            let line = format!(
                "{}{:<30} {:<22} Seat {:>2}   {}",
                cursor,
                truncate(&g.name, 28),
                truncate(&g.server, 20),
                g.seat,
                time_str,
            );
            let style = if i == state.selected {
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };
            Line::styled(line, style)
        })
        .collect();

    let para = Paragraph::new(rows);
    frame.render_widget(para, inner);
}

// ── Footer / join prompt ──────────────────────────────────────────────────────

fn draw_footer(frame: &mut Frame, area: Rect, state: &PickerState) {
    match state.screen {
        Screen::JoinPrompt => {
            let prompt = format!(" Enter invite code: {}█", state.join_input);
            let para = Paragraph::new(prompt)
                .block(Block::default().borders(Borders::ALL))
                .style(Style::default().fg(Color::Cyan));
            frame.render_widget(para, area);
        }
        _ => {
            // Show status message if set, otherwise show the key hints.
            let content = if let Some(msg) = &state.status_msg {
                Span::styled(msg.as_str(), Style::default().fg(Color::Red))
            } else {
                Span::raw(" [J] Join new game   [I] Identity info   [Q] Quit")
            };
            let para = Paragraph::new(Line::from(vec![content]))
                .block(Block::default().borders(Borders::TOP));
            frame.render_widget(para, area);
        }
    }
}

// ── Identity overlay ──────────────────────────────────────────────────────────

fn draw_identity_overlay(frame: &mut Frame, area: Rect, state: &PickerState) {
    // Centre a small popup.
    let popup = centered_rect(60, 5, area);
    frame.render_widget(Clear, popup);

    let npub_short = short_npub(&state.npub);
    let text = format!(
        " Identity: {}  ({})   [{} of {} {}]",
        npub_short,
        state.identity_type,
        state.identity_count.min(1),
        state.identity_count,
        if state.identity_count == 1 {
            "identity"
        } else {
            "identities"
        },
    );
    let para = Paragraph::new(text)
        .block(
            Block::default()
                .title(" Identity ")
                .borders(Borders::ALL)
                .style(Style::default().fg(Color::Cyan)),
        )
        .alignment(Alignment::Left);
    frame.render_widget(para, popup);
}

// ── Helpers ───────────────────────────────────────────────────────────────────

/// Truncate a string with an ellipsis if it exceeds `max` chars.
pub fn truncate(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        s.to_string()
    } else {
        let mut t: String = s.chars().take(max.saturating_sub(1)).collect();
        t.push('…');
        t
    }
}

/// Return a relative time string for an ISO-8601 timestamp (or "—" if None).
///
/// Parses timestamps of the form `YYYY-MM-DDThh:mm:ssZ` using only string
/// slicing (no external time library).  Computes elapsed seconds against
/// `SystemTime::now()` and returns a human-readable label such as:
/// "just now", "3 min ago", "2 hours ago", "5 days ago".
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

/// Parse an ISO-8601 UTC timestamp (`YYYY-MM-DDThh:mm:ssZ`) and return the
/// number of seconds elapsed since that moment, or `None` if the timestamp
/// cannot be parsed or is in the future.
fn parse_elapsed_secs(ts: &str) -> Option<u64> {
    // Accept both `Z` and `+00:00` suffixes by requiring at least 19 chars
    // for the `YYYY-MM-DDThh:mm:ss` portion.
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

    // Convert the parsed timestamp to a Unix epoch count (seconds since
    // 1970-01-01T00:00:00Z) using the proleptic Gregorian calendar formula.
    let days_before_year = days_since_epoch_for_year(year);
    let days_in_year = days_before_month(year, month) + (day - 1);
    let total_days = days_before_year + days_in_year;
    let ts_epoch: u64 = total_days * 86400 + hour * 3600 + min * 60 + sec;

    // Obtain current time as Unix epoch seconds.
    let now_epoch = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .ok()?
        .as_secs();

    now_epoch.checked_sub(ts_epoch)
}

/// Days from 1970-01-01 to the start of the given year.
fn days_since_epoch_for_year(year: u64) -> u64 {
    if year < 1970 {
        return 0;
    }
    let y = year - 1970;
    // Every 4 years is a leap year, except centuries, except 400-year marks.
    // Approximate: 365*y + leap days.
    let base = 1970u64;
    // Count leap years in [1970, year).
    let leaps = count_leaps(base, year);
    y * 365 + leaps
}

/// Count leap years in the range [from, to) (exclusive upper bound).
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

/// Days before the start of `month` (1-based) within `year`.
fn days_before_month(year: u64, month: u64) -> u64 {
    const DAYS: [u64; 12] = [0, 31, 59, 90, 120, 151, 181, 212, 243, 273, 304, 334];
    let m = (month as usize).saturating_sub(1).min(11);
    let base = DAYS[m];
    // Add one if month > February and year is a leap year.
    if month > 2 && is_leap(year) {
        base + 1
    } else {
        base
    }
}

fn is_leap(year: u64) -> bool {
    (year % 4 == 0 && year % 100 != 0) || year % 400 == 0
}

/// Shorten an npub to the first 16 and last 8 characters.
pub fn short_npub(npub: &str) -> String {
    let chars: Vec<char> = npub.chars().collect();
    if chars.len() <= 24 {
        return npub.to_string();
    }
    let head: String = chars[..16].iter().collect();
    let tail: String = chars[chars.len() - 8..].iter().collect();
    format!("{head}…{tail}")
}

/// Return a centred rectangle of `percent_x`% width and `height` lines.
pub fn centered_rect(percent_x: u16, height: u16, r: Rect) -> Rect {
    let popup_width = r.width * percent_x / 100;
    let x = r.x + (r.width.saturating_sub(popup_width)) / 2;
    let y = r.y + (r.height.saturating_sub(height)) / 2;
    Rect::new(x, y, popup_width.min(r.width), height.min(r.height))
}

// ── Game-select overlay ───────────────────────────────────────────────────────

/// Draw the game-selection disambiguation overlay.
///
/// Shows a pop-up listing the candidate games returned by the gate when
/// the player has multiple active games on the same server.  The player
/// uses arrow keys to select one and Enter to connect.
fn draw_game_select_overlay(frame: &mut Frame, area: Rect, games: &[GameEntry], selected: usize) {
    // Height: title row + one row per game + footer hint row, capped at 20.
    let height = (games.len() as u16 + 3).min(20).max(5);
    let popup = centered_rect(70, height, area);
    frame.render_widget(Clear, popup);

    let rows: Vec<Line> = games
        .iter()
        .enumerate()
        .map(|(i, g)| {
            let cursor = if i == selected { "> " } else { "  " };
            let line = format!("{}{} (Seat {})", cursor, g.name, g.seat);
            let style = if i == selected {
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };
            Line::styled(line, style)
        })
        .collect();

    let mut all_lines = rows;
    all_lines.push(Line::from(Span::styled(
        " [↑↓] Select   [Enter] Connect   [Esc] Cancel",
        Style::default().fg(Color::DarkGray),
    )));

    let para = Paragraph::new(all_lines)
        .block(
            Block::default()
                .title(" Multiple Games — Select One ")
                .borders(Borders::ALL)
                .style(Style::default().fg(Color::Yellow)),
        )
        .alignment(Alignment::Left);
    frame.render_widget(para, popup);
}
