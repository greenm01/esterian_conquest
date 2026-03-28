//! Deprecated: TUI identity setup onboarding screen.
//!
//! This module is retained for potential future use but is no longer called
//! from the main launcher flow. New wallet creation now silently generates a
//! Nostr keypair without prompting the user. Power-user identity management
//! (import, alias, additional keys) is handled via the `ec-connect id` CLI
//! subcommands.

#![allow(dead_code, unused_imports)]

use std::time::Duration;

use crossterm::event::{poll, read, Event, KeyCode, KeyEventKind, KeyModifiers};
use ec_ui::buffer::PlayfieldBuffer;
use ec_ui::paint::render_to_stdout;
use ec_ui::session::TerminalSession;
use ec_ui::theme::classic;

use crate::hard_quit::is_hard_quit_key;
use crate::input_field::{draw_labeled_input_row, input_width};
use crate::picker::layout::{centered_rect, draw_box, Rect};
use crate::shell::{terminal_fits_outer, wrap_inner_buffer, INNER_HEIGHT, INNER_WIDTH};
use crate::wallet::io::{now_iso8601, save_wallet_to, wallet_path};
use crate::wallet::{push_identity_from_input, set_identity_alias, Wallet};

enum SetupMode {
    AddOrImport,
    Alias { index: usize, npub: String },
}

struct SetupState {
    mode: SetupMode,
    input: String,
    error_msg: Option<String>,
}

impl SetupState {
    fn new() -> Self {
        Self {
            mode: SetupMode::AddOrImport,
            input: String::new(),
            error_msg: None,
        }
    }
}

pub fn run_first_identity_setup_in_session(
    _session: &mut TerminalSession,
    password: &str,
) -> Result<bool, Box<dyn std::error::Error>> {
    let mut wallet = Wallet::empty();
    let mut state = SetupState::new();
    let path = wallet_path();

    loop {
        let (width, height) = crossterm::terminal::size().unwrap_or((82, 27));
        let buffer = render_setup_buffer(&state, width, height);
        render_to_stdout(&buffer)?;

        if !poll(Duration::from_millis(250))? {
            continue;
        }

        let Event::Key(key) = read()? else {
            continue;
        };
        if key.kind != KeyEventKind::Press {
            continue;
        }
        if is_hard_quit_key(key) {
            return Ok(false);
        }

        match (&state.mode, key.code) {
            (SetupMode::AddOrImport, KeyCode::Esc)
            | (SetupMode::AddOrImport, KeyCode::Char('q' | 'Q'))
                if matches!(key.modifiers, KeyModifiers::NONE | KeyModifiers::SHIFT) =>
            {
                return Ok(false);
            }
            (SetupMode::Alias { .. }, KeyCode::Esc)
            | (SetupMode::Alias { .. }, KeyCode::Char('q' | 'Q'))
                if matches!(key.modifiers, KeyModifiers::NONE | KeyModifiers::SHIFT) =>
            {
                return Ok(true);
            }
            (_, KeyCode::Backspace) => {
                state.input.pop();
            }
            (SetupMode::AddOrImport, KeyCode::Enter) => {
                match push_identity_from_input(&mut wallet, &state.input, now_iso8601()) {
                    Ok(npub) => {
                        let index = wallet.identities.len().saturating_sub(1);
                        wallet.active = index;
                        save_wallet_to(&wallet, password, &path)?;
                        state.mode = SetupMode::Alias { index, npub };
                        state.input.clear();
                        state.error_msg = None;
                    }
                    Err(err) => state.error_msg = Some(err.to_string()),
                }
            }
            (SetupMode::Alias { index, .. }, KeyCode::Enter) => {
                set_identity_alias(&mut wallet, *index, Some(state.input.clone()))?;
                save_wallet_to(&wallet, password, &path)?;
                return Ok(true);
            }
            (_, KeyCode::Char(ch))
                if matches!(key.modifiers, KeyModifiers::NONE | KeyModifiers::SHIFT) =>
            {
                state.input.push(ch);
                state.error_msg = None;
            }
            _ => {}
        }
    }
}

fn render_setup_buffer(state: &SetupState, width: u16, height: u16) -> PlayfieldBuffer {
    let width = usize::from(width.max(1));
    let height = usize::from(height.max(1));

    if !terminal_fits_outer(width, height) {
        let mut buffer = PlayfieldBuffer::new(width, height, classic::body_style());
        let lines = [
            "ec-connect requires an 82x27 terminal.",
            "Resize this window, then continue.",
            "Press Q to quit.",
        ];
        let start_row = height.saturating_sub(lines.len()) / 2;
        for (idx, line) in lines.iter().enumerate() {
            let row = start_row + idx;
            let col = width.saturating_sub(line.chars().count()) / 2;
            let style = if idx == 0 {
                classic::table_header_style()
            } else {
                classic::table_body_style()
            };
            buffer.write_text_clipped(row, col, line, style);
        }
        return buffer;
    }

    let mut buffer = PlayfieldBuffer::new(INNER_WIDTH, INNER_HEIGHT, classic::body_style());
    let outer = Rect::new(0, 2, INNER_WIDTH as u16, 21);
    let popup = centered_rect(
        76,
        8,
        Rect::new(
            outer.x + 1,
            outer.y + 1,
            outer.width.saturating_sub(2),
            outer.height.saturating_sub(2),
        ),
    );
    let title = match state.mode {
        SetupMode::AddOrImport => "SET UP IDENTITY",
        SetupMode::Alias { .. } => "SET IDENTITY ALIAS",
    };
    draw_box(
        &mut buffer,
        popup,
        title,
        classic::table_chrome_style(),
        classic::table_header_style(),
    );

    let left = popup.x as usize + 2;
    let mut row = popup.y as usize + 1;
    if let Some(msg) = state.error_msg.as_deref() {
        buffer.write_text_clipped(row, left, msg, classic::error_style());
        row += 1;
    }

    let label = match &state.mode {
        SetupMode::AddOrImport => {
            buffer.write_text_clipped(
                row,
                left,
                "Paste an nsec or leave blank to create a new keypair.",
                classic::table_body_style(),
            );
            row += 2;
            "Nsec:"
        }
        SetupMode::Alias { npub, .. } => {
            buffer.write_text_clipped(row, left, "Identity created:", classic::table_body_style());
            row += 1;
            buffer.write_text_clipped(row, left, npub, classic::table_header_style());
            row += 2;
            "Alias (optional):"
        }
    };
    let input_col = left + label.chars().count() + 1;
    let inner_right = popup.x as usize + popup.width as usize - 2;
    draw_labeled_input_row(
        &mut buffer,
        row,
        left,
        label,
        &state.input,
        input_width(inner_right, input_col),
        classic::status_label_style(),
        classic::prompt_hotkey_style(),
    );

    wrap_inner_buffer(&buffer, None)
}
