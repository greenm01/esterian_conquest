use nc_ui::{CellStyle, GameColor, PlayfieldBuffer, ScreenGeometry};

use super::{
    DEFAULT_GEOMETRY, FirstRunField, LobbyTab, Model, NetworkState, Route, mask, status_color,
};

const BODY: CellStyle = CellStyle::new(
    GameColor::BrightWhite,
    GameColor::Rgb(0x12, 0x13, 0x1c),
    false,
);
const DIM: CellStyle = CellStyle::new(
    GameColor::BrightBlack,
    GameColor::Rgb(0x12, 0x13, 0x1c),
    false,
);
const ACCENT: CellStyle = CellStyle::new(
    GameColor::BrightCyan,
    GameColor::Rgb(0x12, 0x13, 0x1c),
    true,
);
const WARN: CellStyle = CellStyle::new(
    GameColor::BrightYellow,
    GameColor::Rgb(0x12, 0x13, 0x1c),
    true,
);
const ERROR: CellStyle =
    CellStyle::new(GameColor::BrightRed, GameColor::Rgb(0x12, 0x13, 0x1c), true);
const PANEL: CellStyle = CellStyle::new(
    GameColor::BrightWhite,
    GameColor::Rgb(0x19, 0x1b, 0x26),
    false,
);

pub fn render(model: &Model) -> PlayfieldBuffer {
    let geometry = if model.geometry.width() == 0 || model.geometry.height() == 0 {
        DEFAULT_GEOMETRY
    } else {
        model.geometry
    };
    let mut buffer = PlayfieldBuffer::new(geometry.width(), geometry.height(), BODY);
    fill(&mut buffer, BODY);
    draw_frame(
        &mut buffer,
        0,
        0,
        geometry.width(),
        geometry.height(),
        ACCENT,
    );

    match &model.route {
        Route::Boot(boot) => render_boot(&mut buffer, geometry, &boot.status),
        Route::FirstRun(first_run) => {
            render_first_run(&mut buffer, geometry, model.relay_url.as_str(), first_run)
        }
        Route::Locked(locked) => {
            render_locked(&mut buffer, geometry, model.relay_url.as_str(), locked)
        }
        Route::Lobby(lobby) => render_lobby(&mut buffer, geometry, model, lobby),
        Route::FatalError(message) => render_fatal(&mut buffer, geometry, message),
    }

    buffer
}

fn render_boot(buffer: &mut PlayfieldBuffer, geometry: ScreenGeometry, status: &str) {
    centered_box(buffer, geometry, 52, 9, "NC-HELM", |buffer, left, top| {
        buffer.write_text(top + 2, left + 3, "Booting local player client...", ACCENT);
        buffer.write_text(top + 4, left + 3, status, BODY);
    });
}

fn render_first_run(
    buffer: &mut PlayfieldBuffer,
    geometry: ScreenGeometry,
    relay_url: &str,
    first_run: &super::FirstRunModel,
) {
    centered_box(
        buffer,
        geometry,
        68,
        15,
        "CREATE IDENTITY",
        |buffer, left, top| {
            buffer.write_text(
                top + 2,
                left + 3,
                "NC-HELM stores encrypted player keys in SQLite.",
                ACCENT,
            );
            buffer.write_text(
                top + 3,
                left + 3,
                "Tab cycles fields. Enter submits. Esc quits.",
                DIM,
            );
            field_row(
                buffer,
                left + 3,
                top + 6,
                "Handle",
                &first_run.handle_input,
                first_run.active_field == FirstRunField::Handle,
                false,
            );
            field_row(
                buffer,
                left + 3,
                top + 8,
                "Password",
                &first_run.password_input,
                first_run.active_field == FirstRunField::Password,
                true,
            );
            field_row(
                buffer,
                left + 3,
                top + 10,
                "Confirm",
                &first_run.confirm_input,
                first_run.active_field == FirstRunField::Confirm,
                true,
            );
            field_row(
                buffer,
                left + 3,
                top + 12,
                "Relay",
                &first_run.relay_input,
                first_run.active_field == FirstRunField::Relay,
                false,
            );
            buffer.write_text(
                top + 13,
                left + 3,
                &format!("Active relay: {relay_url}"),
                DIM,
            );
            if let Some(status) = &first_run.status {
                buffer.write_text(top + 14, left + 3, status, status_style(status));
            }
            place_field_cursor(
                buffer,
                left + 15,
                top + 6,
                &first_run.handle_input,
                first_run.active_field == FirstRunField::Handle,
                false,
            );
            place_field_cursor(
                buffer,
                left + 15,
                top + 8,
                &first_run.password_input,
                first_run.active_field == FirstRunField::Password,
                true,
            );
            place_field_cursor(
                buffer,
                left + 15,
                top + 10,
                &first_run.confirm_input,
                first_run.active_field == FirstRunField::Confirm,
                true,
            );
            place_field_cursor(
                buffer,
                left + 15,
                top + 12,
                &first_run.relay_input,
                first_run.active_field == FirstRunField::Relay,
                false,
            );
        },
    );
}

fn render_locked(
    buffer: &mut PlayfieldBuffer,
    geometry: ScreenGeometry,
    relay_url: &str,
    locked: &super::LockedModel,
) {
    centered_box(
        buffer,
        geometry,
        60,
        11,
        "UNLOCK KEYCHAIN",
        |buffer, left, top| {
            buffer.write_text(
                top + 2,
                left + 3,
                "Enter your local keychain password.",
                ACCENT,
            );
            buffer.write_text(top + 3, left + 3, &format!("Relay: {relay_url}"), DIM);
            field_row(
                buffer,
                left + 3,
                top + 6,
                "Password",
                &locked.password_input,
                true,
                true,
            );
            place_field_cursor(
                buffer,
                left + 15,
                top + 6,
                &locked.password_input,
                true,
                true,
            );
            if let Some(status) = &locked.status {
                buffer.write_text(top + 8, left + 3, status, status_style(status));
            }
            buffer.write_text(top + 9, left + 3, "Press Esc to quit.", DIM);
        },
    );
}

fn render_lobby(
    buffer: &mut PlayfieldBuffer,
    geometry: ScreenGeometry,
    model: &Model,
    lobby: &super::LobbyModel,
) {
    if let Some(session) = &model.session {
        let identity = session
            .active_handle
            .as_deref()
            .unwrap_or(session.active_npub.as_str());
        buffer.write_text_clipped(1, 28, identity, BODY);
    }
    let network_style = CellStyle::new(status_color(model.network), BODY.bg, true);
    buffer.write_text(1, 3, "NOSTRIAN CONQUEST", ACCENT);
    buffer.write_text(1, geometry.width().saturating_sub(28), "NETWORK:", DIM);
    buffer.write_text(
        1,
        geometry.width().saturating_sub(17),
        match model.network {
            NetworkState::Idle => "IDLE",
            NetworkState::Connecting => "CONNECTING",
            NetworkState::Synced => "SYNCED",
            NetworkState::Error => "ERROR",
        },
        network_style,
    );

    draw_tabs(buffer, lobby);
    draw_games_table(buffer, model, lobby);
    draw_notices(buffer, geometry, model);
    draw_footer(buffer, geometry, model, lobby);

    if let Some(status) = &lobby.status {
        buffer.write_text(
            geometry.height().saturating_sub(3),
            3,
            status,
            status_style(status),
        );
    }
    if lobby.help_open {
        draw_help_popup(buffer, geometry);
    }
}

fn render_fatal(buffer: &mut PlayfieldBuffer, geometry: ScreenGeometry, message: &str) {
    centered_box(buffer, geometry, 64, 9, "FATAL", |buffer, left, top| {
        buffer.write_text(top + 2, left + 3, "The nc-helm bootstrap failed.", ERROR);
        buffer.write_text_clipped(top + 4, left + 3, message, BODY);
        buffer.write_text(top + 6, left + 3, "Press Q or Esc to quit.", DIM);
    });
}

fn draw_tabs(buffer: &mut PlayfieldBuffer, lobby: &super::LobbyModel) {
    let tabs = [
        ("1 HOME", LobbyTab::Home),
        ("2 OPEN GAMES", LobbyTab::OpenGames),
        ("3 COMMS", LobbyTab::Comms),
        ("4 SETTINGS", LobbyTab::Settings),
    ];
    let mut col = 3usize;
    for (label, tab) in tabs {
        let style = if lobby.active_tab == tab { ACCENT } else { DIM };
        buffer.write_text(3, col, "[", DIM);
        buffer.write_text(3, col + 1, label, style);
        buffer.write_text(3, col + 1 + label.len(), "]", DIM);
        col += label.len() + 4;
    }
}

fn draw_games_table(buffer: &mut PlayfieldBuffer, model: &Model, lobby: &super::LobbyModel) {
    buffer.write_text(5, 3, "OPEN GAMES", ACCENT);
    buffer.write_text(
        6,
        3,
        "STAT  NAME                 HOST         TYPE     SEATS  YEAR",
        DIM,
    );
    if model.games.is_empty() {
        buffer.write_text(
            7,
            3,
            "No open games synced yet. Leave the client running.",
            WARN,
        );
        return;
    }
    for (index, row) in model.games.iter().enumerate() {
        let style = if index == lobby.selected_game {
            CellStyle::new(GameColor::Black, GameColor::BrightCyan, true)
        } else {
            BODY
        };
        let line = format!(
            "{:<5} {:<20} {:<12} {:<8} {:<6} {}",
            "open",
            truncate(&row.name, 20),
            truncate(&row.host, 12),
            truncate(&row.tier, 8),
            truncate(&row.seats, 6),
            truncate(&row.when, 10),
        );
        buffer.write_text_clipped(7 + index, 3, &line, style);
    }
}

fn draw_notices(buffer: &mut PlayfieldBuffer, geometry: ScreenGeometry, model: &Model) {
    let top = geometry.height().saturating_sub(11);
    draw_frame(buffer, 2, top, geometry.width().saturating_sub(4), 8, DIM);
    buffer.write_text(top, 4, " NOTICES ", ACCENT);
    if model.notices.is_empty() {
        buffer.write_text(top + 2, 4, "No recent lobby notices.", DIM);
        return;
    }
    for (idx, notice) in model.notices.iter().take(4).enumerate() {
        buffer.write_text_clipped(top + 1 + idx, 4, notice, BODY);
    }
}

fn draw_footer(
    buffer: &mut PlayfieldBuffer,
    geometry: ScreenGeometry,
    model: &Model,
    lobby: &super::LobbyModel,
) {
    let row = geometry.height().saturating_sub(2);
    let footer = if lobby.help_open {
        "Any key closes help."
    } else {
        "Tab next tab   ? help   Up/Down select   Esc quit"
    };
    buffer.write_text_clipped(row, 3, footer, DIM);
    if let Some(selected) = model.games.get(lobby.selected_game) {
        let text = format!("Selected: {}", selected.game_id);
        let start = geometry.width().saturating_sub(text.len() + 3);
        buffer.write_text_clipped(row, start, &text, DIM);
    }
}

fn draw_help_popup(buffer: &mut PlayfieldBuffer, geometry: ScreenGeometry) {
    centered_box(buffer, geometry, 60, 11, "HELP [X]", |buffer, left, top| {
        let lines = [
            "NC-HELM is the new TEA player client.",
            "",
            "Tab     : switch lobby tabs",
            "Up/Down : move the open-game cursor",
            "? or H  : reopen this help popup",
            "Esc/Q   : quit the client",
            "",
            "The lobby sync runs in the background.",
        ];
        for (idx, line) in lines.iter().enumerate() {
            buffer.write_text_clipped(
                top + 2 + idx,
                left + 3,
                line,
                if line.is_empty() {
                    BODY
                } else {
                    if idx == 0 { ACCENT } else { BODY }
                },
            );
        }
    });
}

fn fill(buffer: &mut PlayfieldBuffer, style: CellStyle) {
    for row in 0..buffer.height() {
        buffer.fill_row(row, style);
    }
}

fn draw_frame(
    buffer: &mut PlayfieldBuffer,
    left: usize,
    top: usize,
    width: usize,
    height: usize,
    style: CellStyle,
) {
    if width < 2 || height < 2 {
        return;
    }
    for col in left + 1..left + width - 1 {
        buffer.set_cell(top, col, '-', style);
        buffer.set_cell(top + height - 1, col, '-', style);
    }
    for row in top + 1..top + height - 1 {
        buffer.set_cell(row, left, '|', style);
        buffer.set_cell(row, left + width - 1, '|', style);
    }
    buffer.set_cell(top, left, '+', style);
    buffer.set_cell(top, left + width - 1, '+', style);
    buffer.set_cell(top + height - 1, left, '+', style);
    buffer.set_cell(top + height - 1, left + width - 1, '+', style);
}

fn centered_box<F: FnOnce(&mut PlayfieldBuffer, usize, usize)>(
    buffer: &mut PlayfieldBuffer,
    geometry: ScreenGeometry,
    width: usize,
    height: usize,
    title: &str,
    inner: F,
) {
    let left = geometry.width().saturating_sub(width) / 2;
    let top = geometry.height().saturating_sub(height) / 2;
    fill_rect(buffer, left, top, width, height, PANEL);
    draw_frame(buffer, left, top, width, height, ACCENT);
    buffer.write_text(top, left + 2, &format!(" {title} "), ACCENT);
    inner(buffer, left, top);
}

fn fill_rect(
    buffer: &mut PlayfieldBuffer,
    left: usize,
    top: usize,
    width: usize,
    height: usize,
    style: CellStyle,
) {
    for row in top..top.saturating_add(height).min(buffer.height()) {
        for col in left..left.saturating_add(width).min(buffer.width()) {
            buffer.set_cell(row, col, ' ', style);
        }
    }
}

fn field_row(
    buffer: &mut PlayfieldBuffer,
    left: usize,
    row: usize,
    label: &str,
    value: &str,
    active: bool,
    masked: bool,
) {
    let field_style = if active { ACCENT } else { BODY };
    buffer.write_text(row, left, &format!("{label:<9}: "), DIM);
    let shown = if masked {
        mask(value)
    } else {
        value.to_string()
    };
    buffer.write_text_clipped(row, left + 12, &shown, field_style);
}

fn place_field_cursor(
    buffer: &mut PlayfieldBuffer,
    left: usize,
    row: usize,
    value: &str,
    active: bool,
    masked: bool,
) {
    if !active {
        return;
    }
    let shown_len = if masked {
        value.chars().count()
    } else {
        value.chars().count()
    };
    let column = (left + shown_len).min(buffer.width().saturating_sub(1));
    buffer.set_cursor(column as u16, row as u16);
}

fn truncate(value: &str, width: usize) -> String {
    value.chars().take(width).collect()
}

fn status_style(status: &str) -> CellStyle {
    if status.contains("error")
        || status.contains("invalid")
        || status.contains("failed")
        || status.contains("empty")
    {
        ERROR
    } else if status.contains("sync") || status.contains("created") || status.contains("unlocked") {
        ACCENT
    } else {
        WARN
    }
}
