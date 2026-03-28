use ec_ui::buffer::{CellStyle, GameColor, PlayfieldBuffer};
use ec_ui::prompt::{
    draw_command_line_prompt_text_at, draw_plain_prompt, draw_table_command_bar_at,
};
use ec_ui::theme::classic;

use super::help::{GAME_SELECT_RAIL, MAIN_MENU_RAIL, WALLET_MENU_RAIL};
use super::layout::{
    BODY_ROWS, BODY_START_ROW, COMMAND_ROW, Column, PLAYFIELD_HEIGHT, PLAYFIELD_WIDTH, TABLE_RIGHT,
    draw_box, draw_scroll_gutter, draw_table_frame, draw_title, middle_ellipsis, pad_right,
    scroll_start,
};
pub use super::layout::{Rect, centered_rect, relative_time, short_npub, truncate};
use super::overlay::render_overlay;
use super::{MatrixState, PickerSession, PickerState, Screen};
use crate::connect::handshake::GameEntry;

const MAIN_COLUMNS: [Column<'_>; 5] = [
    Column {
        title: "Empire",
        width: 13,
    },
    Column {
        title: "Game",
        width: 17,
    },
    Column {
        title: "Server",
        width: 18,
    },
    Column {
        title: "Gate",
        width: 19,
    },
    Column {
        title: "Seat",
        width: 6,
    },
];

const WALLET_COLUMNS: [Column<'_>; 5] = [
    Column {
        title: "#",
        width: 2,
    },
    Column {
        title: "Alias",
        width: 14,
    },
    Column {
        title: "Npub",
        width: 29,
    },
    Column {
        title: "Type",
        width: 8,
    },
    Column {
        title: "Created",
        width: 20,
    },
];

const GAME_SELECT_COLUMNS: [Column<'_>; 2] = [
    Column {
        title: "Game",
        width: 66,
    },
    Column {
        title: "Seat",
        width: 6,
    },
];

pub fn render_buffer(
    state: &PickerState,
    session: Option<&PickerSession>,
    term_width: u16,
    term_height: u16,
) -> PlayfieldBuffer {
    if usize::from(term_width) < PLAYFIELD_WIDTH || usize::from(term_height) < PLAYFIELD_HEIGHT {
        return render_resize_blocker(term_width, term_height);
    }

    let mut buffer = PlayfieldBuffer::new(PLAYFIELD_WIDTH, PLAYFIELD_HEIGHT, classic::body_style());
    let identity_label = session.map(PickerSession::header_identity_label);
    draw_title(
        &mut buffer,
        "ESTERIAN CONQUEST CONNECT",
        identity_label.as_deref(),
    );

    match &state.screen {
        Screen::GameList | Screen::JoinPrompt | Screen::IdentityOverlay => {
            render_main_menu(&mut buffer, state, session);
        }
        Screen::WalletList | Screen::WalletAliasPrompt | Screen::WalletImportPrompt => {
            render_wallet_menu(&mut buffer, state, session);
        }
        Screen::GameSelect {
            games, selected, ..
        } => {
            render_game_select(&mut buffer, games, *selected);
        }
        Screen::Locked => render_locked_screen(&mut buffer, &state.matrix),
    }

    render_overlay(&mut buffer, state);
    buffer
}

fn render_resize_blocker(term_width: u16, term_height: u16) -> PlayfieldBuffer {
    let width = usize::from(term_width.max(1));
    let height = usize::from(term_height.max(1));
    let mut buffer = PlayfieldBuffer::new(width, height, classic::body_style());
    let lines = [
        "ec-connect requires an 80x25 terminal.",
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
    buffer
}

fn render_main_menu(
    buffer: &mut PlayfieldBuffer,
    state: &PickerState,
    session: Option<&PickerSession>,
) {
    draw_table_frame(buffer, &MAIN_COLUMNS);
    let sorted = state.cache.sorted();
    if sorted.is_empty() {
        let message = "No joined games yet. Press N to join one.";
        let row = BODY_START_ROW + BODY_ROWS / 2;
        let col = TABLE_RIGHT.saturating_sub(message.chars().count()) / 2;
        buffer.write_text_clipped(row, col, message, classic::notice_style());
    } else {
        let start = scroll_start(state.selected, BODY_ROWS, sorted.len());
        for (idx, game) in sorted.iter().skip(start).take(BODY_ROWS).enumerate() {
            let row = BODY_START_ROW + idx;
            let is_selected = start + idx == state.selected;
            draw_main_row(buffer, row, game, is_selected);
        }
        draw_scroll_gutter(buffer, start, BODY_ROWS, sorted.len());
    }

    if matches!(state.screen, Screen::IdentityOverlay) {
        if let Some(session) = session {
            draw_identity_overlay(buffer, session);
        }
    }

    match state.screen {
        Screen::JoinPrompt => {
            let prompt = format!("Invite code <Q> <?> -> {}", state.join_input);
            draw_command_line_prompt_text_at(buffer, COMMAND_ROW, "CONNECT COMMAND", &prompt);
        }
        Screen::IdentityOverlay => {
            draw_table_command_bar_at(buffer, COMMAND_ROW, "<Q> <?>", None, "");
        }
        Screen::GameList => {
            draw_table_command_bar_at(buffer, COMMAND_ROW, MAIN_MENU_RAIL, None, "");
        }
        _ => {}
    }
}

fn render_wallet_menu(
    buffer: &mut PlayfieldBuffer,
    state: &PickerState,
    session: Option<&PickerSession>,
) {
    draw_table_frame(buffer, &WALLET_COLUMNS);
    if let Some(session) = session {
        if session.wallet.identities.is_empty() {
            let message = "Wallet has no identities.";
            let row = BODY_START_ROW + BODY_ROWS / 2;
            let col = TABLE_RIGHT.saturating_sub(message.chars().count()) / 2;
            buffer.write_text_clipped(row, col, message, classic::notice_style());
        } else {
            let start = scroll_start(
                state.wallet_selected,
                BODY_ROWS,
                session.wallet.identities.len(),
            );
            for (idx, identity) in session
                .wallet
                .identities
                .iter()
                .skip(start)
                .take(BODY_ROWS)
                .enumerate()
            {
                let row = BODY_START_ROW + idx;
                let absolute = start + idx;
                let is_selected = absolute == state.wallet_selected;
                let is_active = absolute == session.wallet.active;
                draw_wallet_row(buffer, row, identity, absolute, is_selected, is_active);
            }
            draw_scroll_gutter(buffer, start, BODY_ROWS, session.wallet.identities.len());
        }
    }

    match state.screen {
        Screen::WalletAliasPrompt => {
            let default = session
                .and_then(|session| session.wallet.identities.get(state.wallet_selected))
                .and_then(|identity| identity.alias.as_deref())
                .unwrap_or("");
            draw_alias_prompt(buffer, default, &state.alias_input);
        }
        Screen::WalletImportPrompt => {
            let prompt = format!(
                "Import nsec <Q> <?> -> {}",
                "*".repeat(state.import_input.chars().count())
            );
            draw_command_line_prompt_text_at(buffer, COMMAND_ROW, "WALLET COMMAND", &prompt);
        }
        Screen::WalletList => {
            draw_table_command_bar_at(buffer, COMMAND_ROW, WALLET_MENU_RAIL, None, "");
        }
        _ => {}
    }
}

fn render_game_select(buffer: &mut PlayfieldBuffer, games: &[GameEntry], selected: usize) {
    draw_table_frame(buffer, &GAME_SELECT_COLUMNS);
    let start = scroll_start(selected, BODY_ROWS, games.len());
    for (idx, game) in games.iter().skip(start).take(BODY_ROWS).enumerate() {
        let row = BODY_START_ROW + idx;
        let is_selected = start + idx == selected;
        draw_select_row(buffer, row, game, is_selected);
    }
    draw_scroll_gutter(buffer, start, BODY_ROWS, games.len());
    draw_table_command_bar_at(buffer, COMMAND_ROW, GAME_SELECT_RAIL, None, "");
}

fn render_locked_screen(buffer: &mut PlayfieldBuffer, matrix: &MatrixState) {
    let trail_style = CellStyle::new(GameColor::Green, GameColor::Black, false);
    let head_style = CellStyle::new(GameColor::BrightGreen, GameColor::Black, true);

    for x in 0..PLAYFIELD_WIDTH {
        let speed = 1 + (x * 7 % 3);
        let length = 4 + (x * 11 % 10);
        let cycle = PLAYFIELD_HEIGHT + length + 8;
        let head = ((matrix.frame as usize / speed) + (x * 5)) % cycle;
        let head = head as isize - length as isize;
        for y in 0..PLAYFIELD_HEIGHT {
            let y_isize = y as isize;
            if y_isize > head || y_isize <= head - length as isize {
                continue;
            }
            let dist = (head - y_isize) as usize;
            let glyph = matrix_glyph(x, y, matrix.frame);
            let style = if dist == 0 { head_style } else { trail_style };
            buffer.set_cell(y, x, glyph, style);
        }
    }
}

fn matrix_glyph(x: usize, y: usize, frame: u64) -> char {
    const GLYPHS: &[u8] = b"01{}[]<>*+#$%&";
    let index = ((frame as usize) + (x * 13) + (y * 7)) % GLYPHS.len();
    GLYPHS[index] as char
}

fn draw_main_row(
    buffer: &mut PlayfieldBuffer,
    row: usize,
    game: &crate::cache::CachedGame,
    selected: bool,
) {
    let columns = [
        pad_right(
            game.player_name.as_deref().unwrap_or(""),
            MAIN_COLUMNS[0].width,
        ),
        pad_right(
            &truncate(&game.name, MAIN_COLUMNS[1].width),
            MAIN_COLUMNS[1].width,
        ),
        pad_right(
            &truncate(
                &format!("{}:{}", game.server, game.port),
                MAIN_COLUMNS[2].width,
            ),
            MAIN_COLUMNS[2].width,
        ),
        pad_right(
            &middle_ellipsis(game.gate_npub.as_str(), MAIN_COLUMNS[3].width, 8, 6),
            MAIN_COLUMNS[3].width,
        ),
        format!("{:>width$}", game.seat, width = MAIN_COLUMNS[4].width),
    ];
    draw_row_cells(buffer, row, &MAIN_COLUMNS, &columns, selected, false);
}

fn draw_wallet_row(
    buffer: &mut PlayfieldBuffer,
    row: usize,
    identity: &crate::wallet::Identity,
    index: usize,
    selected: bool,
    active: bool,
) {
    let npub = crate::wallet::identity_npub(identity).unwrap_or_else(|_| "<invalid>".to_string());
    let columns = [
        format!("{:>width$}", index + 1, width = WALLET_COLUMNS[0].width),
        pad_right(
            &truncate(
                identity.alias.as_deref().unwrap_or(""),
                WALLET_COLUMNS[1].width,
            ),
            WALLET_COLUMNS[1].width,
        ),
        pad_right(
            &truncate(&npub, WALLET_COLUMNS[2].width),
            WALLET_COLUMNS[2].width,
        ),
        pad_right(identity.identity_type.as_str(), WALLET_COLUMNS[3].width),
        pad_right(
            &truncate(&identity.created, WALLET_COLUMNS[4].width),
            WALLET_COLUMNS[4].width,
        ),
    ];
    draw_row_cells(buffer, row, &WALLET_COLUMNS, &columns, selected, active);
}

fn draw_select_row(buffer: &mut PlayfieldBuffer, row: usize, game: &GameEntry, selected: bool) {
    let columns = [
        pad_right(
            &truncate(&game.name, GAME_SELECT_COLUMNS[0].width),
            GAME_SELECT_COLUMNS[0].width,
        ),
        format!(
            "{:>width$}",
            game.seat,
            width = GAME_SELECT_COLUMNS[1].width
        ),
    ];
    draw_row_cells(buffer, row, &GAME_SELECT_COLUMNS, &columns, selected, false);
}

fn draw_row_cells(
    buffer: &mut PlayfieldBuffer,
    row: usize,
    columns: &[Column<'_>],
    values: &[String],
    selected: bool,
    active: bool,
) {
    let row_style = classic::table_body_style();
    let selected_style = classic::selected_row_style();
    let active_style = CellStyle::new(GameColor::BrightYellow, GameColor::Black, true);
    let mut col = 1usize;
    for (idx, (column, value)) in columns.iter().zip(values.iter()).enumerate() {
        let style = if idx == 0 && selected {
            selected_style
        } else if idx == 0 && active {
            active_style
        } else {
            row_style
        };
        let filler = if idx == 0 && (selected || active) {
            style
        } else {
            row_style
        };
        buffer.write_text_clipped(row, col, value, style);
        col += column.width;
        if col <= TABLE_RIGHT {
            buffer.set_cell(row, col, '│', classic::table_chrome_style());
            col += 1;
        }
        if filler != row_style {
            for x in 1..=column.width {
                if x > value.chars().count() {
                    buffer.set_cell(row, col - column.width - 1 + x, ' ', style);
                }
            }
        }
    }
}

fn draw_identity_overlay(buffer: &mut PlayfieldBuffer, session: &PickerSession) {
    let popup = centered_rect(
        60,
        10,
        Rect::new(0, 0, PLAYFIELD_WIDTH as u16, PLAYFIELD_HEIGHT as u16),
    );
    draw_box(
        buffer,
        popup,
        "Identity",
        classic::table_chrome_style(),
        classic::table_header_style(),
    );
    let left = popup.x as usize + 2;
    let top = popup.y as usize + 2;
    let lines = [
        format!("Alias: {}", session.active_alias().unwrap_or("(none)")),
        format!("Npub: {}", short_npub(&session.npub)),
        format!("Type: {}", session.active_identity_type()),
        format!("Wallet identities: {}", session.wallet.identities.len()),
        format!(
            "Wallet: {}",
            truncate(&crate::wallet::io::wallet_path().display().to_string(), 46)
        ),
    ];
    for (idx, line) in lines.iter().enumerate() {
        buffer.write_text_clipped(top + idx, left, line, classic::table_body_style());
    }
    draw_plain_prompt(
        buffer,
        popup.y as usize + popup.height as usize - 2,
        "<Q> <?> ->",
    );
}

fn draw_alias_prompt(buffer: &mut PlayfieldBuffer, default: &str, input: &str) {
    let prompt = if default.is_empty() {
        format!("Alias <Q> <?> -> {input}")
    } else {
        format!("Alias [{}] <Q> <?> -> {input}", truncate(default, 16))
    };
    draw_command_line_prompt_text_at(buffer, COMMAND_ROW, "WALLET COMMAND", &prompt);
}
