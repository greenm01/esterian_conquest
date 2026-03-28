use ec_ui::buffer::{CellStyle, GameColor, PlayfieldBuffer};
use ec_ui::prompt::draw_table_command_bar_at;
use ec_ui::theme::classic;

use super::help::{GAME_SELECT_RAIL, MAIN_MENU_RAIL, WALLET_MENU_RAIL};
use super::layout::{
    BODY_START_ROW, Column, INNER_COMMAND_ROW, MAX_BODY_ROWS, PLAYFIELD_HEIGHT, PLAYFIELD_WIDTH,
    displayed_body_rows, draw_scroll_gutter, draw_table_frame, middle_ellipsis, pad_right,
    scroll_start, table_cell_start, table_message_col,
};
pub use super::layout::{Rect, centered_rect, relative_time, short_date, short_npub, truncate};
use super::overlay::{render_identity_popup, render_overlay, render_wallet_add_popup};
use super::{MatrixState, PickerSession, PickerState, Screen};
use crate::connect::handshake::GameEntry;
use crate::shell::{terminal_fits_outer, wrap_inner_buffer};

const MAIN_COLUMNS: [Column<'_>; 6] = [
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
        width: 16,
    },
    Column {
        title: "Gate",
        width: 12,
    },
    Column {
        title: "Seat",
        width: 4,
    },
    Column {
        title: "Joined",
        width: 10,
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
    if !terminal_fits_outer(usize::from(term_width), usize::from(term_height)) {
        return render_resize_blocker(term_width, term_height);
    }

    let mut buffer = PlayfieldBuffer::new(PLAYFIELD_WIDTH, PLAYFIELD_HEIGHT, classic::body_style());
    let identity_label = session.map(PickerSession::header_identity_label);

    let command_row = match &state.screen {
        Screen::GameList | Screen::IdentityOverlay => {
            render_main_menu(&mut buffer, state, session)
        }
        Screen::WalletList | Screen::WalletAddPrompt => {
            render_wallet_menu(&mut buffer, state, session)
        }
        Screen::GameSelect {
            games, selected, ..
        } => render_game_select(&mut buffer, games, *selected),
        Screen::Locked => {
            render_locked_screen(&mut buffer, &state.matrix);
            INNER_COMMAND_ROW
        }
    };

    render_overlay(&mut buffer, state, session, command_row);
    wrap_inner_buffer(&buffer, identity_label.as_deref())
}

fn render_resize_blocker(term_width: u16, term_height: u16) -> PlayfieldBuffer {
    let width = usize::from(term_width.max(1));
    let height = usize::from(term_height.max(1));
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
    buffer
}

fn render_main_menu(
    buffer: &mut PlayfieldBuffer,
    state: &PickerState,
    session: Option<&PickerSession>,
) -> usize {
    let sorted = state.cache.sorted();
    let start = scroll_start(state.selected, MAX_BODY_ROWS, sorted.len());
    let metrics = draw_table_frame(
        buffer,
        &MAIN_COLUMNS,
        displayed_body_rows(sorted.len(), start),
    );
    let sorted = state.cache.sorted();
    if sorted.is_empty() {
        let message = "No joined games yet. Press N to join one.";
        let row = BODY_START_ROW + metrics.displayed_rows / 2;
        let col = table_message_col(&MAIN_COLUMNS, message);
        buffer.write_text_clipped(row, col, message, classic::notice_style());
    } else {
        for (idx, game) in sorted
            .iter()
            .skip(start)
            .take(metrics.displayed_rows)
            .enumerate()
        {
            let row = BODY_START_ROW + idx;
            let is_selected = start + idx == state.selected;
            draw_main_row(buffer, row, game, is_selected);
        }
        draw_scroll_gutter(buffer, metrics, start, sorted.len());
    }

    if matches!(state.screen, Screen::IdentityOverlay) {
        if let Some(session) = session {
            render_identity_popup(buffer, session);
        }
    }

    match state.screen {
        Screen::GameList => {
            draw_table_command_bar_at(buffer, metrics.command_row, MAIN_MENU_RAIL, None, "");
        }
        _ => {}
    }
    metrics.command_row
}

fn render_wallet_menu(
    buffer: &mut PlayfieldBuffer,
    state: &PickerState,
    session: Option<&PickerSession>,
) -> usize {
    let wallet_len = session
        .map(|session| session.wallet.identities.len())
        .unwrap_or(0);
    let start = scroll_start(state.wallet_selected, MAX_BODY_ROWS, wallet_len);
    let metrics = draw_table_frame(
        buffer,
        &WALLET_COLUMNS,
        displayed_body_rows(wallet_len, start),
    );
    if let Some(session) = session {
        if session.wallet.identities.is_empty() {
            let message = "Wallet has no identities.";
            let row = BODY_START_ROW + metrics.displayed_rows / 2;
            let col = table_message_col(&WALLET_COLUMNS, message);
            buffer.write_text_clipped(row, col, message, classic::notice_style());
        } else {
            for (idx, identity) in session
                .wallet
                .identities
                .iter()
                .skip(start)
                .take(metrics.displayed_rows)
                .enumerate()
            {
                let row = BODY_START_ROW + idx;
                let absolute = start + idx;
                let is_selected = absolute == state.wallet_selected;
                let is_active = absolute == session.wallet.active;
                draw_wallet_row(buffer, row, identity, absolute, is_selected, is_active);
            }
            draw_scroll_gutter(buffer, metrics, start, session.wallet.identities.len());
        }
    }

    match state.screen {
        Screen::WalletList | Screen::WalletAddPrompt => {
            draw_table_command_bar_at(buffer, metrics.command_row, WALLET_MENU_RAIL, None, "");
        }
        _ => {}
    }
    if matches!(state.screen, Screen::WalletAddPrompt) {
        render_wallet_add_popup(buffer, &state.wallet_input);
    }
    metrics.command_row
}

fn render_game_select(buffer: &mut PlayfieldBuffer, games: &[GameEntry], selected: usize) -> usize {
    let start = scroll_start(selected, MAX_BODY_ROWS, games.len());
    let metrics = draw_table_frame(
        buffer,
        &GAME_SELECT_COLUMNS,
        displayed_body_rows(games.len(), start),
    );
    for (idx, game) in games
        .iter()
        .skip(start)
        .take(metrics.displayed_rows)
        .enumerate()
    {
        let row = BODY_START_ROW + idx;
        let is_selected = start + idx == selected;
        draw_select_row(buffer, row, game, is_selected);
    }
    draw_scroll_gutter(buffer, metrics, start, games.len());
    draw_table_command_bar_at(buffer, metrics.command_row, GAME_SELECT_RAIL, None, "");
    metrics.command_row
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
    let empire_label = game
        .player_name
        .as_deref()
        .filter(|value| !value.is_empty())
        .map(str::to_string)
        .unwrap_or_else(|| format!("Seat {}", game.seat));
    let columns = [
        pad_right(
            &truncate(&empire_label, MAIN_COLUMNS[0].width),
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
            &middle_ellipsis(game.gate_npub.as_str(), MAIN_COLUMNS[3].width, 5, 4),
            MAIN_COLUMNS[3].width,
        ),
        format!("{:>width$}", game.seat, width = MAIN_COLUMNS[4].width),
        pad_right(
            &short_date(&game.joined),
            MAIN_COLUMNS[5].width,
        ),
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
    for (idx, (column, value)) in columns.iter().zip(values.iter()).enumerate() {
        let Some(col) = table_cell_start(columns, idx) else {
            continue;
        };
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
        buffer.set_cell(row, col + column.width, '│', classic::table_chrome_style());
        if filler != row_style {
            for x in value.chars().count()..column.width {
                buffer.set_cell(row, col + x, ' ', style);
            }
        }
    }
}
