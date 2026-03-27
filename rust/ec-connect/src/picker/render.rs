//! Ratatui draw functions for the picker TUI.

use ratatui::layout::{Alignment, Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph};
use ratatui::Frame;

use super::{PickerState, Screen};

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
    if state.screen == Screen::IdentityOverlay {
        draw_identity_overlay(frame, area, state);
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
/// This is a minimal approximation that avoids pulling in a time library:
/// it compares the timestamp string against the current system time obtained
/// from `SystemTime`.
pub fn relative_time(ts: Option<&str>) -> &'static str {
    // Full relative-time formatting is deferred to step 12.
    // For now, show a static placeholder based on whether we have a timestamp.
    match ts {
        None => "—",
        Some(_) => "connected",
    }
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
