use ec_ui::buffer::{CellStyle, GameColor, PlayfieldBuffer};
use ec_ui::prompt::{draw_right_aligned_footer_text, draw_table_command_bar_at_col};
use ec_ui::table_layout::{
    HorizontalAlign, LayoutRect, TableWidthMode, VerticalAlign, layout_table_block,
};
use ec_ui::theme::classic;

use crate::cache::CachedGameStatus;

use super::help::{
    GAME_SELECT_RAIL, MAIN_MENU_RAIL, RELAY_GAMES_RAIL, RELAY_MENU_RAIL, KEYCHAIN_MENU_RAIL,
};
use super::layout::{
    Column, INNER_COMMAND_ROW, MAX_BODY_ROWS, PLAYFIELD_HEIGHT, PLAYFIELD_WIDTH, TableMetrics,
    displayed_body_rows, draw_scroll_gutter, draw_table_frame, pad_right, resolve_columns,
    scroll_start, table_cell_start, table_message_col_in, table_render_width, table_text_col,
};
pub use super::layout::{Rect, centered_rect, relative_time, short_date, short_npub, truncate};
use super::overlay::{render_identity_popup, render_overlay, render_keychain_add_popup};
use super::relay::{relay_games, relay_status_label, relay_summaries};
use super::{MatrixState, PickerSession, PickerState, Screen};
use crate::connect::handshake::GameEntry;
use crate::shell::{terminal_fits_outer, wrap_inner_buffer_in_terminal};

const MAIN_COLUMNS: [Column<'_>; 6] = [
    Column::flex("Empire", 13, 1),
    Column::flex("Game", 17, 2),
    Column::flex("Server", 22, 1),
    Column::fixed("Status", 8),
    Column::fixed("Seat", 4),
    Column::fixed("Joined", 10),
];

const KEYCHAIN_COLUMNS: [Column<'_>; 3] = [
    Column::flex("Npub", 46, 2),
    Column::fixed("Type", 8),
    Column::flex("Created", 20, 1),
];

const GAME_SELECT_COLUMNS: [Column<'_>; 2] =
    [Column::flex("Game", 66, 1), Column::fixed("Seat", 6)];

const RELAY_COLUMNS: [Column<'_>; 5] = [
    Column::flex("Relay", 28, 2),
    Column::fixed("Status", 10),
    Column::fixed("Games", 5),
    Column::flex("Last Error", 19, 1),
    Column::fixed("Checked", 9),
];

const RELAY_GAME_COLUMNS: [Column<'_>; 5] = [
    Column::flex("Game", 29, 2),
    Column::flex("Server", 20, 1),
    Column::fixed("Seat", 4),
    Column::fixed("Joined", 10),
    Column::fixed("Last Conn", 12),
];

const TABLE_FOOTER_VERSION_TAG: &str = concat!("NC ", env!("CARGO_PKG_VERSION"));

pub fn render_buffer(
    state: &PickerState,
    session: Option<&PickerSession>,
    term_width: u16,
    term_height: u16,
) -> PlayfieldBuffer {
    if !terminal_fits_outer(usize::from(term_width), usize::from(term_height)) {
        return render_resize_blocker(term_width, term_height);
    }

    let buffer = render_inner_buffer(state, session);
    let identity_label = session.map(PickerSession::header_identity_label);
    wrap_inner_buffer_in_terminal(
        &buffer,
        identity_label.as_deref(),
        usize::from(term_width.max(1)),
        usize::from(term_height.max(1)),
        None,
    )
}

pub fn render_inner_buffer(
    state: &PickerState,
    session: Option<&PickerSession>,
) -> PlayfieldBuffer {
    let mut buffer = PlayfieldBuffer::new(PLAYFIELD_WIDTH, PLAYFIELD_HEIGHT, classic::body_style());

    let command_row = match &state.screen {
        Screen::GameList | Screen::IdentityOverlay => render_main_menu(&mut buffer, state, session),
        Screen::RelayList => render_relay_list(&mut buffer, state),
        Screen::RelayGames { relay_url } => render_relay_games(&mut buffer, state, relay_url),
        Screen::KeychainList | Screen::KeychainAddPrompt => {
            render_keychain_menu(&mut buffer, state, session)
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
    buffer
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
    let columns = layout_columns(&MAIN_COLUMNS, sorted.len(), start);
    let metrics = draw_picker_table_frame(buffer, &columns, sorted.len(), start);
    let sorted = state.cache.sorted();
    if sorted.is_empty() {
        let message = "No joined games yet. Press N to join one.";
        let row = metrics.body_start_row + metrics.displayed_rows / 2;
        let col = table_message_col_in(metrics, message);
        buffer.write_text_clipped(row, col, message, classic::notice_style());
    } else {
        for (idx, game) in sorted
            .iter()
            .skip(start)
            .take(metrics.displayed_rows)
            .enumerate()
        {
            let row = metrics.body_start_row + idx;
            let is_selected = start + idx == state.selected;
            draw_main_row(buffer, row, metrics.table_col, &columns, game, is_selected);
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
            let end_col = draw_table_command_bar_at_col(
                buffer,
                metrics.command_row,
                metrics.command_col,
                MAIN_MENU_RAIL,
                None,
                "",
            );
            draw_table_footer_version(buffer, metrics.command_row, end_col);
        }
        _ => {}
    }
    metrics.command_row
}

fn render_keychain_menu(
    buffer: &mut PlayfieldBuffer,
    state: &PickerState,
    session: Option<&PickerSession>,
) -> usize {
    let keychain_len = session
        .map(|session| usize::from(session.keychain.active_identity().is_some()))
        .unwrap_or(0);
    let start = scroll_start(state.keychain_selected, MAX_BODY_ROWS, keychain_len);
    let columns = layout_columns(&KEYCHAIN_COLUMNS, keychain_len, start);
    let metrics = draw_picker_table_frame(buffer, &columns, keychain_len, start);
    if let Some(session) = session {
        if session.keychain.identities.is_empty() {
            let message = "Keychain has no identities.";
            let row = metrics.body_start_row + metrics.displayed_rows / 2;
            let col = table_message_col_in(metrics, message);
            buffer.write_text_clipped(row, col, message, classic::notice_style());
        } else if let Some(identity) = session.keychain.active_identity() {
            let row = metrics.body_start_row;
            draw_keychain_row(
                buffer,
                row,
                metrics.table_col,
                &columns,
                identity,
                true,
                true,
            );
        }
    }

    match state.screen {
        Screen::KeychainList | Screen::KeychainAddPrompt => {
            let end_col = draw_table_command_bar_at_col(
                buffer,
                metrics.command_row,
                metrics.command_col,
                KEYCHAIN_MENU_RAIL,
                None,
                "",
            );
            draw_table_footer_version(buffer, metrics.command_row, end_col);
        }
        _ => {}
    }
    if matches!(state.screen, Screen::KeychainAddPrompt) {
        render_keychain_add_popup(buffer, &state.keychain_input);
    }
    metrics.command_row
}

fn render_game_select(buffer: &mut PlayfieldBuffer, games: &[GameEntry], selected: usize) -> usize {
    let start = scroll_start(selected, MAX_BODY_ROWS, games.len());
    let columns = layout_columns(&GAME_SELECT_COLUMNS, games.len(), start);
    let metrics = draw_picker_table_frame(buffer, &columns, games.len(), start);
    for (idx, game) in games
        .iter()
        .skip(start)
        .take(metrics.displayed_rows)
        .enumerate()
    {
        let row = metrics.body_start_row + idx;
        let is_selected = start + idx == selected;
        draw_select_row(buffer, row, metrics.table_col, &columns, game, is_selected);
    }
    draw_scroll_gutter(buffer, metrics, start, games.len());
    let end_col = draw_table_command_bar_at_col(
        buffer,
        metrics.command_row,
        metrics.command_col,
        GAME_SELECT_RAIL,
        None,
        "",
    );
    draw_table_footer_version(buffer, metrics.command_row, end_col);
    metrics.command_row
}

fn render_relay_list(buffer: &mut PlayfieldBuffer, state: &PickerState) -> usize {
    let relays = relay_summaries(state);
    let start = scroll_start(state.relay_selected, MAX_BODY_ROWS, relays.len());
    let columns = layout_columns(&RELAY_COLUMNS, relays.len(), start);
    let metrics = draw_picker_table_frame(buffer, &columns, relays.len(), start);
    if relays.is_empty() {
        let message = "No relays known yet. Press A to add one.";
        let row = metrics.body_start_row + metrics.displayed_rows / 2;
        let col = table_message_col_in(metrics, message);
        buffer.write_text_clipped(row, col, message, classic::notice_style());
    } else {
        for (idx, relay) in relays
            .iter()
            .skip(start)
            .take(metrics.displayed_rows)
            .enumerate()
        {
            let row = metrics.body_start_row + idx;
            let is_selected = start + idx == state.relay_selected;
            draw_relay_row(buffer, row, metrics.table_col, &columns, relay, is_selected);
        }
        draw_scroll_gutter(buffer, metrics, start, relays.len());
    }
    let end_col = draw_table_command_bar_at_col(
        buffer,
        metrics.command_row,
        metrics.command_col,
        RELAY_MENU_RAIL,
        None,
        "",
    );
    draw_table_footer_version(buffer, metrics.command_row, end_col);
    metrics.command_row
}

fn render_relay_games(buffer: &mut PlayfieldBuffer, state: &PickerState, relay_url: &str) -> usize {
    let games = relay_games(state, relay_url);
    let start = scroll_start(state.relay_game_selected, MAX_BODY_ROWS, games.len());
    let columns = layout_columns(&RELAY_GAME_COLUMNS, games.len(), start);
    let metrics = draw_picker_table_frame(buffer, &columns, games.len(), start);
    let relay_label = format!(
        "Relay: {}",
        truncate(relay_url, PLAYFIELD_WIDTH.saturating_sub(7))
    );
    buffer.write_text_clipped(
        0,
        table_text_col(metrics.table_col),
        &relay_label,
        classic::status_value_style(),
    );
    if games.is_empty() {
        let message = "No joined games currently use this relay.";
        let row = metrics.body_start_row + metrics.displayed_rows / 2;
        let col = table_message_col_in(metrics, message);
        buffer.write_text_clipped(row, col, message, classic::notice_style());
    } else {
        for (idx, game) in games
            .iter()
            .skip(start)
            .take(metrics.displayed_rows)
            .enumerate()
        {
            let row = metrics.body_start_row + idx;
            let is_selected = start + idx == state.relay_game_selected;
            draw_relay_game_row(buffer, row, metrics.table_col, &columns, game, is_selected);
        }
        draw_scroll_gutter(buffer, metrics, start, games.len());
    }
    let end_col = draw_table_command_bar_at_col(
        buffer,
        metrics.command_row,
        metrics.command_col,
        RELAY_GAMES_RAIL,
        None,
        "",
    );
    draw_table_footer_version(buffer, metrics.command_row, end_col);
    metrics.command_row
}

fn draw_table_footer_version(buffer: &mut PlayfieldBuffer, row: usize, footer_end_col: usize) {
    let prompt = classic::prompt_style();
    let style = CellStyle::new(GameColor::BrightBlack, prompt.bg, false);
    let screen_row = PLAYFIELD_HEIGHT.saturating_sub(1);
    if row >= screen_row.saturating_sub(2) {
        return;
    }
    let occupied_until_col = if row == screen_row { footer_end_col } else { 0 };
    let _ = draw_right_aligned_footer_text(
        buffer,
        screen_row,
        occupied_until_col,
        TABLE_FOOTER_VERSION_TAG,
        style,
    );
}

fn render_locked_screen(buffer: &mut PlayfieldBuffer, matrix: &MatrixState) {
    let background = classic::app_background();
    let trail_style = CellStyle::new(GameColor::Green, background, false);
    let head_style = CellStyle::new(GameColor::BrightGreen, background, true);

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

#[doc(hidden)]
pub fn matrix_glyph(x: usize, y: usize, frame: u64) -> char {
    const GLYPHS: &[char] = &[
        'Α', 'Β', 'Γ', 'Δ', 'Ε', 'Ζ', 'Η', 'Θ', 'Ι', 'Κ', 'Λ', 'Μ', 'Ν', 'Ξ', 'Ο', 'Π', 'Ρ', 'Σ',
        'Τ', 'Υ', 'Φ', 'Χ', 'Ψ', 'Ω', '+', '#', '%', '*',
    ];
    let index = ((frame as usize) + (x * 13) + (y * 7)) % GLYPHS.len();
    GLYPHS[index]
}

fn draw_main_row(
    buffer: &mut PlayfieldBuffer,
    row: usize,
    table_col: usize,
    columns: &[Column<'_>],
    game: &crate::cache::CachedGame,
    selected: bool,
) {
    let empire_label = game
        .player_name
        .as_deref()
        .filter(|value| !value.is_empty())
        .map(str::to_string)
        .unwrap_or_else(|| format!("Seat {}", game.seat));
    let values = [
        pad_right(&truncate(&empire_label, columns[0].width), columns[0].width),
        pad_right(&truncate(&game.name, columns[1].width), columns[1].width),
        pad_right(
            &truncate(&format!("{}:{}", game.server, game.port), columns[2].width),
            columns[2].width,
        ),
        pad_right(cached_game_status_label(game.status), columns[3].width),
        format!("{:>width$}", game.seat, width = columns[4].width),
        pad_right(&short_date(&game.joined), columns[5].width),
    ];
    draw_row_cells(buffer, row, table_col, columns, &values, selected, false);
}

fn cached_game_status_label(status: CachedGameStatus) -> &'static str {
    match status {
        CachedGameStatus::Pending => "Pending",
        CachedGameStatus::Joined => "Joined",
    }
}

fn draw_keychain_row(
    buffer: &mut PlayfieldBuffer,
    row: usize,
    table_col: usize,
    columns: &[Column<'_>],
    identity: &crate::keychain::Identity,
    selected: bool,
    active: bool,
) {
    let npub = crate::keychain::identity_npub(identity).unwrap_or_else(|_| "<invalid>".to_string());
    let values = [
        pad_right(&truncate(&npub, columns[0].width), columns[0].width),
        pad_right(identity.identity_type.as_str(), columns[1].width),
        pad_right(
            &truncate(&identity.created, columns[2].width),
            columns[2].width,
        ),
    ];
    draw_row_cells(buffer, row, table_col, columns, &values, selected, active);
}

fn draw_select_row(
    buffer: &mut PlayfieldBuffer,
    row: usize,
    table_col: usize,
    columns: &[Column<'_>],
    game: &GameEntry,
    selected: bool,
) {
    let values = [
        pad_right(&truncate(&game.name, columns[0].width), columns[0].width),
        format!("{:>width$}", game.seat, width = columns[1].width),
    ];
    draw_row_cells(buffer, row, table_col, columns, &values, selected, false);
}

fn draw_relay_row(
    buffer: &mut PlayfieldBuffer,
    row: usize,
    table_col: usize,
    columns: &[Column<'_>],
    relay: &super::relay::RelaySummary,
    selected: bool,
) {
    let relay_label = if relay.is_default {
        format!(
            "{} *",
            truncate(&relay.url, columns[0].width.saturating_sub(2))
        )
    } else {
        truncate(&relay.url, columns[0].width)
    };
    let values = [
        pad_right(&relay_label, columns[0].width),
        pad_right(relay_status_label(relay.status), columns[1].width),
        format!("{:>width$}", relay.game_count, width = columns[2].width),
        pad_right(
            &truncate(relay.last_error.as_deref().unwrap_or(""), columns[3].width),
            columns[3].width,
        ),
        pad_right(
            &relative_time(relay.last_checked.as_deref()),
            columns[4].width,
        ),
    ];
    draw_row_cells(buffer, row, table_col, columns, &values, selected, false);
}

fn draw_relay_game_row(
    buffer: &mut PlayfieldBuffer,
    row: usize,
    table_col: usize,
    columns: &[Column<'_>],
    game: &crate::cache::CachedGame,
    selected: bool,
) {
    let values = [
        pad_right(&truncate(&game.name, columns[0].width), columns[0].width),
        pad_right(
            &truncate(&format!("{}:{}", game.server, game.port), columns[1].width),
            columns[1].width,
        ),
        format!("{:>width$}", game.seat, width = columns[2].width),
        pad_right(&short_date(&game.joined), columns[3].width),
        pad_right(
            game.last_connected
                .as_deref()
                .map(short_date)
                .as_deref()
                .unwrap_or(""),
            columns[4].width,
        ),
    ];
    draw_row_cells(buffer, row, table_col, columns, &values, selected, false);
}

fn draw_row_cells(
    buffer: &mut PlayfieldBuffer,
    row: usize,
    table_col: usize,
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
        let col = table_col + col;
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

fn layout_columns<'a>(columns: &[Column<'a>], total_rows: usize, start: usize) -> Vec<Column<'a>> {
    let displayed_rows = displayed_body_rows(total_rows, start);
    let scrollable = total_rows > displayed_rows;
    resolve_columns(columns, PLAYFIELD_WIDTH, scrollable, TableWidthMode::Expand)
}

fn draw_picker_table_frame(
    buffer: &mut PlayfieldBuffer,
    columns: &[Column<'_>],
    total_rows: usize,
    start: usize,
) -> TableMetrics {
    let displayed_rows = displayed_body_rows(total_rows, start);
    let scrollable = total_rows > displayed_rows;
    let layout = layout_table_block(
        LayoutRect::new(0, 0, PLAYFIELD_WIDTH, PLAYFIELD_HEIGHT),
        table_render_width(columns),
        displayed_rows + 4,
        table_render_width(columns) + usize::from(scrollable),
        false,
        true,
        scrollable,
        HorizontalAlign::Left,
        VerticalAlign::Top,
    );
    draw_table_frame(
        buffer,
        layout.table_col,
        layout.table_row,
        columns,
        displayed_rows,
    )
}
