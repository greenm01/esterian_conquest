use super::{
    chrome::{draw_panel, draw_top_tag},
    DEFAULT_GEOMETRY, FirstRunField, LobbyTab, Model, NetworkState, Route, mask, status_color,
};
use crate::{
    BackgroundMode, CellStyle, Column, GameColor, PlayfieldBuffer, Point, Row, ScreenGeometry,
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
const ROOT_BORDER: CellStyle = CellStyle::new(
    GameColor::Rgb(0x7f, 0x91, 0x7b),
    GameColor::Rgb(0x12, 0x13, 0x1c),
    false,
);
const ROOT_TITLE: CellStyle = CellStyle::new(
    GameColor::BrightCyan,
    GameColor::Rgb(0x12, 0x13, 0x1c),
    true,
);
const PANEL_BODY: CellStyle = CellStyle::new(
    GameColor::BrightWhite,
    GameColor::Rgb(0x19, 0x1b, 0x26),
    false,
);
const PANEL_DIM: CellStyle = CellStyle::new(
    GameColor::BrightBlack,
    GameColor::Rgb(0x19, 0x1b, 0x26),
    false,
);
const PANEL_ACCENT: CellStyle = CellStyle::new(
    GameColor::BrightCyan,
    GameColor::Rgb(0x19, 0x1b, 0x26),
    true,
);
const PANEL_WARN: CellStyle = CellStyle::new(
    GameColor::BrightYellow,
    GameColor::Rgb(0x19, 0x1b, 0x26),
    true,
);
const PANEL_ERROR: CellStyle = CellStyle::new(
    GameColor::BrightRed,
    GameColor::Rgb(0x19, 0x1b, 0x26),
    true,
);
const PANEL_BORDER: CellStyle = CellStyle::new(
    GameColor::Rgb(0x7f, 0x91, 0x7b),
    GameColor::Rgb(0x19, 0x1b, 0x26),
    false,
);
const FORM_FIELD_LABEL_WIDTH: usize = 9;
const FORM_FIELD_TRACK_WIDTH: usize = 30;
const SETTINGS_FIELD_LABEL_WIDTH: usize = 12;
const SETTINGS_FIELD_TRACK_WIDTH: usize = 44;

pub fn render(model: &Model) -> PlayfieldBuffer {
    let geometry = if model.geometry.width() == 0 || model.geometry.height() == 0 {
        DEFAULT_GEOMETRY
    } else {
        model.geometry
    };
    let mut buffer = PlayfieldBuffer::new(geometry.width(), geometry.height(), BODY);
    fill(&mut buffer, BODY);
    draw_panel(
        &mut buffer,
        0,
        0,
        geometry.width(),
        geometry.height(),
        ROOT_BORDER,
        ROOT_TITLE,
        None,
        Some("nc-helm"),
        None,
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
        buffer.write_text(top + 2, left + 3, "Booting local player client...", PANEL_ACCENT);
        buffer.write_text(top + 4, left + 3, status, PANEL_BODY);
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
                PANEL_ACCENT,
            );
            buffer.write_text(
                top + 3,
                left + 3,
                "Tab cycles fields. Enter submits. Esc quits.",
                PANEL_DIM,
            );
            draw_boxed_input_row(
                buffer,
                left + 3,
                top + 6,
                FORM_FIELD_LABEL_WIDTH,
                FORM_FIELD_TRACK_WIDTH,
                "Handle",
                &first_run.handle_input,
                first_run.active_field == FirstRunField::Handle,
                false,
            );
            draw_boxed_input_row(
                buffer,
                left + 3,
                top + 8,
                FORM_FIELD_LABEL_WIDTH,
                FORM_FIELD_TRACK_WIDTH,
                "Password",
                &first_run.password_input,
                first_run.active_field == FirstRunField::Password,
                true,
            );
            draw_boxed_input_row(
                buffer,
                left + 3,
                top + 10,
                FORM_FIELD_LABEL_WIDTH,
                FORM_FIELD_TRACK_WIDTH,
                "Confirm",
                &first_run.confirm_input,
                first_run.active_field == FirstRunField::Confirm,
                true,
            );
            draw_boxed_input_row(
                buffer,
                left + 3,
                top + 12,
                FORM_FIELD_LABEL_WIDTH,
                FORM_FIELD_TRACK_WIDTH,
                "Relay",
                &first_run.relay_input,
                first_run.active_field == FirstRunField::Relay,
                false,
            );
            buffer.write_text(
                top + 13,
                left + 3,
                &format!("Active relay: {relay_url}"),
                PANEL_DIM,
            );
            if let Some(status) = &first_run.status {
                buffer.write_text(top + 14, left + 3, status, panel_status_style(status));
            }
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
                PANEL_ACCENT,
            );
            buffer.write_text(
                top + 3,
                left + 3,
                &format!("Relay: {relay_url}"),
                PANEL_DIM,
            );
            draw_boxed_input_row(
                buffer,
                left + 3,
                top + 6,
                FORM_FIELD_LABEL_WIDTH,
                FORM_FIELD_TRACK_WIDTH,
                "Password",
                &locked.password_input,
                true,
                true,
            );
            if let Some(status) = &locked.status {
                buffer.write_text(top + 8, left + 3, status, panel_status_style(status));
            }
            buffer.write_text(top + 9, left + 3, "Press Esc to quit.", PANEL_DIM);
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
    buffer.write_text(1, 3, "NOSTRIAN CONQUEST", ROOT_TITLE);
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
    match lobby.active_tab {
        LobbyTab::Home => draw_home(buffer, geometry, model),
        LobbyTab::OpenGames => draw_open_games(buffer, geometry, model, lobby),
        LobbyTab::Comms => draw_comms(buffer, geometry, model),
        LobbyTab::Settings => draw_settings(buffer, geometry, model),
    }
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

fn draw_home(buffer: &mut PlayfieldBuffer, geometry: ScreenGeometry, model: &Model) {
    draw_panel(
        buffer,
        2,
        5,
        geometry.width().saturating_sub(4),
        17,
        PANEL_BORDER,
        PANEL_ACCENT,
        Some(PANEL),
        Some("HOME"),
        None,
    );
    buffer.write_text(
        7,
        5,
        "NC-HELM runs the hosted lobby on a fresh TEA runtime.",
        PANEL_BODY,
    );
    buffer.write_text(
        8,
        5,
        "Background sync is isolated from the window loop.",
        PANEL_BODY,
    );
    buffer.write_text(10, 5, "Session", PANEL_ACCENT);
    if let Some(session) = &model.session {
        buffer.write_text(
            11,
            7,
            session
                .active_handle
                .as_deref()
                .unwrap_or("anonymous identity"),
            PANEL_BODY,
        );
        buffer.write_text_clipped(12, 7, &session.active_npub, PANEL_DIM);
    } else {
        buffer.write_text(11, 7, "No active session", PANEL_WARN);
    }
    buffer.write_text(14, 5, "Lobby Snapshot", PANEL_ACCENT);
    buffer.write_text(
        15,
        7,
        &format!("Open games : {}", model.games.len()),
        PANEL_BODY,
    );
    buffer.write_text(
        16,
        7,
        &format!("Notices    : {}", model.notices.len()),
        PANEL_BODY,
    );
    buffer.write_text(18, 5, "Shortcuts", PANEL_ACCENT);
    buffer.write_text(19, 7, "2 opens the game catalog.", PANEL_BODY);
    buffer.write_text(20, 7, "3 opens lobby notices/COMMS.", PANEL_BODY);
    buffer.write_text(21, 7, "4 opens settings and lock controls.", PANEL_BODY);
    draw_status_panel(buffer, geometry, model);
}

fn draw_open_games(
    buffer: &mut PlayfieldBuffer,
    geometry: ScreenGeometry,
    model: &Model,
    lobby: &super::LobbyModel,
) {
    let table_width = geometry.width().saturating_sub(36);
    draw_panel(
        buffer,
        2,
        5,
        table_width,
        geometry.height().saturating_sub(16),
        PANEL_BORDER,
        PANEL_ACCENT,
        Some(PANEL),
        Some("OPEN GAMES"),
        None,
    );
    draw_games_table(buffer, model, lobby);
    draw_status_panel(buffer, geometry, model);
    if let Some(selected) = model.games.get(lobby.selected_game) {
        let top = geometry.height().saturating_sub(11);
        draw_panel(
            buffer,
            geometry.width().saturating_sub(34),
            5,
            30,
            10,
            PANEL_BORDER,
            PANEL_ACCENT,
            Some(PANEL),
            Some("SELECTED GAME"),
            None,
        );
        let left = geometry.width().saturating_sub(32);
        buffer.write_text_clipped(7, left, &selected.name, PANEL_BODY);
        buffer.write_text_clipped(8, left, &format!("Host  : {}", selected.host), PANEL_BODY);
        buffer.write_text_clipped(9, left, &format!("Tier  : {}", selected.tier), PANEL_BODY);
        buffer.write_text_clipped(10, left, &format!("Seats : {}", selected.seats), PANEL_BODY);
        buffer.write_text_clipped(11, left, &format!("Turn  : {}", selected.when), PANEL_BODY);
        buffer.write_text_clipped(12, left, &selected.game_id, PANEL_DIM);
        buffer.write_text_clipped(top + 2, 4, "Use Up/Down to change selection.", DIM);
    }
}

fn draw_comms(buffer: &mut PlayfieldBuffer, geometry: ScreenGeometry, model: &Model) {
    draw_panel(
        buffer,
        2,
        5,
        geometry.width().saturating_sub(4),
        17,
        PANEL_BORDER,
        PANEL_ACCENT,
        Some(PANEL),
        Some("COMMS"),
        None,
    );
    draw_panel(
        buffer,
        4,
        7,
        geometry.width().saturating_sub(8),
        11,
        PANEL_BORDER,
        PANEL_ACCENT,
        None,
        Some("LOBBY NOTICES"),
        None,
    );
    if model.notices.is_empty() {
        buffer.write_text(11, 7, "No recent notices from the relay.", PANEL_DIM);
    } else {
        for (idx, notice) in model.notices.iter().take(10).enumerate() {
            buffer.write_text_clipped(9 + idx, 7, notice, PANEL_BODY);
        }
    }
    buffer.write_text(20, 5, "Direct replies and threads are not wired yet.", PANEL_WARN);
    draw_status_panel(buffer, geometry, model);
}

fn draw_settings(buffer: &mut PlayfieldBuffer, geometry: ScreenGeometry, model: &Model) {
    draw_panel(
        buffer,
        2,
        5,
        geometry.width().saturating_sub(4),
        17,
        PANEL_BORDER,
        PANEL_ACCENT,
        Some(PANEL),
        Some("SETTINGS"),
        None,
    );
    let (relay_draft, editing_relay) = if let Route::Lobby(lobby) = &model.route {
        (lobby.relay_draft.as_str(), lobby.editing_relay)
    } else {
        (model.relay_url.as_str(), false)
    };
    draw_boxed_input_row(
        buffer,
        5,
        8,
        SETTINGS_FIELD_LABEL_WIDTH,
        SETTINGS_FIELD_TRACK_WIDTH,
        "Relay URL",
        relay_draft,
        editing_relay,
        false,
    );
    buffer.write_text(
        9,
        5,
        &format!(
            "Window Focus : {}",
            if model.window_focused { "yes" } else { "no" }
        ),
        PANEL_BODY,
    );
    buffer.write_text(
        10,
        5,
        &format!(
            "Text Input   : {}",
            if model.wants_text_input() {
                "armed"
            } else {
                "off"
            }
        ),
        PANEL_BODY,
    );
    if let Some(session) = &model.session {
        buffer.write_text(
            12,
            5,
            &format!(
                "Handle       : {}",
                session.active_handle.as_deref().unwrap_or("unset")
            ),
            PANEL_BODY,
        );
        buffer.write_text_clipped(
            13,
            5,
            &format!("Identity     : {}", session.active_npub),
            PANEL_BODY,
        );
    }
    buffer.write_text(
        15,
        5,
        "R : Edit relay URL   Enter : Save relay   Esc : Cancel edit",
        if editing_relay { PANEL_ACCENT } else { PANEL_DIM },
    );
    buffer.write_text(
        16,
        5,
        "L : Lock the local session and stop background sync",
        PANEL_ACCENT,
    );
    buffer.write_text(18, 5, "Esc/Q : Quit nc-helm", PANEL_DIM);
    draw_status_panel(buffer, geometry, model);
}

fn render_fatal(buffer: &mut PlayfieldBuffer, geometry: ScreenGeometry, message: &str) {
    centered_box(buffer, geometry, 64, 9, "FATAL", |buffer, left, top| {
        buffer.write_text(top + 2, left + 3, "The nc-helm bootstrap failed.", PANEL_ERROR);
        buffer.write_text_clipped(top + 4, left + 3, message, PANEL_BODY);
        buffer.write_text(top + 6, left + 3, "Press Q or Esc to quit.", PANEL_DIM);
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
        let title_style = if lobby.active_tab == tab {
            ROOT_TITLE
        } else {
            DIM
        };
        let width = draw_top_tag(buffer, 3, col, buffer.width().saturating_sub(col), label, ROOT_BORDER, title_style);
        col += width + 2;
    }
}

fn draw_games_table(buffer: &mut PlayfieldBuffer, model: &Model, lobby: &super::LobbyModel) {
    buffer.write_text(
        7,
        5,
        "STAT  NAME                 HOST         TYPE     SEATS  YEAR",
        PANEL_DIM,
    );
    if model.games.is_empty() {
        buffer.write_text(
            9,
            5,
            "No open games synced yet. Leave the client running.",
            PANEL_WARN,
        );
        return;
    }
    for (index, row) in model.games.iter().enumerate() {
        let style = if index == lobby.selected_game {
            CellStyle::new(GameColor::Black, GameColor::BrightCyan, true)
        } else {
            PANEL_BODY
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
        buffer.write_text_clipped(8 + index, 5, &line, style);
    }
}

fn draw_status_panel(buffer: &mut PlayfieldBuffer, geometry: ScreenGeometry, model: &Model) {
    let top = geometry.height().saturating_sub(11);
    draw_panel(
        buffer,
        2,
        top,
        geometry.width().saturating_sub(4),
        8,
        PANEL_BORDER,
        PANEL_ACCENT,
        Some(PANEL),
        Some("STATUS"),
        None,
    );
    let lines = [
        format!(
            "Network : {}",
            match model.network {
                NetworkState::Idle => "idle",
                NetworkState::Connecting => "connecting",
                NetworkState::Synced => "synced",
                NetworkState::Error => "error",
            }
        ),
        format!("Games   : {}", model.games.len()),
        format!("Notices : {}", model.notices.len()),
    ];
    for (idx, line) in lines.into_iter().enumerate() {
        buffer.write_text_clipped(top + 2 + idx, 4, &line, PANEL_BODY);
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
        "Tab next tab   ? help   Up/Down select   L lock   Esc quit"
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
                    PANEL_BODY
                } else {
                    if idx == 0 { PANEL_ACCENT } else { PANEL_BODY }
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
    draw_panel(
        buffer,
        left,
        top,
        width,
        height,
        PANEL_BORDER,
        PANEL_ACCENT,
        Some(PANEL),
        Some(title),
        None,
    );
    inner(buffer, left, top);
}

fn draw_boxed_input_row(
    buffer: &mut PlayfieldBuffer,
    left: usize,
    row: usize,
    label_width: usize,
    track_width: usize,
    label: &str,
    value: &str,
    active: bool,
    masked: bool,
) {
    let field_style = if active { PANEL_ACCENT } else { PANEL_BODY };
    let track_style = PANEL_BODY.with_background_mode(BackgroundMode::TextBand);
    let value_style = field_style.with_background_mode(BackgroundMode::TextBand);
    let field_left = left + label_width + 2;
    let field_width = track_width.min(buffer.width().saturating_sub(field_left));
    let text_col = field_left.saturating_add(1);
    buffer.write_text(row, left, &format!("{label:<label_width$}: "), PANEL_DIM);
    buffer.fill_rect(row, field_left, field_width, 1, track_style);
    let shown = if masked {
        mask(value)
    } else {
        value.to_string()
    };
    buffer.write_text_clipped(row, text_col, &shown, value_style);
    if active && field_width > 0 {
        let max_col = field_left + field_width - 1;
        let cursor_col = (text_col + shown.chars().count()).min(max_col);
        if cursor_col < buffer.width() {
            buffer.set_cursor(Point::new(Column(cursor_col), Row(row)));
        }
    }
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

fn panel_status_style(status: &str) -> CellStyle {
    if status.contains("error")
        || status.contains("invalid")
        || status.contains("failed")
        || status.contains("empty")
    {
        PANEL_ERROR
    } else if status.contains("sync") || status.contains("created") || status.contains("unlocked") {
        PANEL_ACCENT
    } else {
        PANEL_WARN
    }
}
