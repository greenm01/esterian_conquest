use std::time::Duration;

use crossterm::event::{Event, KeyCode, KeyEventKind, KeyModifiers, poll, read};
use ec_ui::buffer::PlayfieldBuffer;
use ec_ui::paint::render_to_stdout;
use ec_ui::session::TerminalSession;
use ec_ui::theme::classic;

use crate::picker::layout::{Rect, centered_rect, draw_box};
use crate::shell::{INNER_HEIGHT, INNER_WIDTH, terminal_fits_outer, wrap_inner_buffer};
use crate::wallet::io::{now_iso8601, save_wallet_to, wallet_path};
use crate::wallet::{Wallet, push_identity_from_input, set_identity_alias};

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

    match &state.mode {
        SetupMode::AddOrImport => {
            buffer.write_text_clipped(
                row,
                left,
                "Paste an nsec or leave blank to create a new keypair.",
                classic::table_body_style(),
            );
            row += 2;
            buffer.write_text_clipped(row, left, "Nsec:", classic::status_label_style());
        }
        SetupMode::Alias { npub, .. } => {
            buffer.write_text_clipped(row, left, "Identity created:", classic::table_body_style());
            row += 1;
            buffer.write_text_clipped(row, left, npub, classic::table_header_style());
            row += 2;
            buffer.write_text_clipped(
                row,
                left,
                "Alias (optional):",
                classic::status_label_style(),
            );
        }
    }

    let value_col = left + 17;
    let cursor_col = value_col
        + buffer.write_text_clipped(row, value_col, &state.input, classic::prompt_hotkey_style());
    if cursor_col < buffer.width() {
        buffer.set_cursor(cursor_col as u16, row as u16);
    }

    wrap_inner_buffer(&buffer, None)
}
