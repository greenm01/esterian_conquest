use super::{
    DEFAULT_GEOMETRY, FirstRunField, HELP_CLOSE_LABEL, HELP_POPUP_HEIGHT, HELP_POPUP_WIDTH,
    LobbyTab, Model, NetworkState, Route, centered_box_geometry,
    chrome::{draw_panel, draw_top_tag, draw_top_tag_right},
    mask, status_color,
};
use crate::grid::OverlayTextFamily;
use crate::{CellStyle, Column, GameColor, PlayfieldBuffer, Point, Row, ScreenGeometry};

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
const PANEL_BRAND: CellStyle = CellStyle::new(
    GameColor::BrightCyan,
    GameColor::Rgb(0x19, 0x1b, 0x26),
    false,
);
const PANEL_WARN: CellStyle = CellStyle::new(
    GameColor::BrightYellow,
    GameColor::Rgb(0x19, 0x1b, 0x26),
    true,
);
const PANEL_ERROR: CellStyle =
    CellStyle::new(GameColor::BrightRed, GameColor::Rgb(0x19, 0x1b, 0x26), true);
const PANEL_BORDER: CellStyle = CellStyle::new(
    GameColor::Rgb(0x7f, 0x91, 0x7b),
    GameColor::Rgb(0x19, 0x1b, 0x26),
    false,
);
const FORM_FIELD_LABEL_WIDTH: usize = 9;
const SETTINGS_FIELD_LABEL_WIDTH: usize = 12;
const SETTINGS_FIELD_TRACK_WIDTH: usize = 44;
const FIRST_RUN_GATE_WIDTH: usize = 68;
const FIRST_RUN_GATE_HEIGHT: usize = 22;
const LOCKED_GATE_WIDTH: usize = 60;
const LOCKED_GATE_HEIGHT: usize = 13;
const LOCKED_GATE_HEIGHT_WITH_STATUS: usize = 15;
const GATE_SIDE_PADDING: usize = 3;
const GATE_LOGO_WIDTH_INSET: usize = 3;
const GATE_STORMFAZE_MIN_WIDTH: usize = 48;
const GATE_STORMFAZE_MIN_HEIGHT: usize = 13;
const GATE_LOGO_BLOCK_ROWS: usize = 7;

struct GateLayout {
    content_left: usize,
    content_width: usize,
    next_row: usize,
}

pub fn render(model: &Model) -> PlayfieldBuffer {
    let geometry = if model.geometry.width() == 0 || model.geometry.height() == 0 {
        DEFAULT_GEOMETRY
    } else {
        model.geometry
    };
    let mut buffer = PlayfieldBuffer::new(geometry.width(), geometry.height(), BODY);
    fill(&mut buffer, BODY);

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
        buffer.write_text(
            top + 2,
            left + 3,
            "Booting local player client...",
            PANEL_ACCENT,
        );
        buffer.write_text(top + 4, left + 3, status, PANEL_BODY);
    });
}

fn render_first_run(
    buffer: &mut PlayfieldBuffer,
    geometry: ScreenGeometry,
    relay_url: &str,
    first_run: &super::FirstRunModel,
) {
    let layout = render_gate_shell(
        buffer,
        geometry,
        FIRST_RUN_GATE_WIDTH,
        FIRST_RUN_GATE_HEIGHT,
        "CREATE IDENTITY",
        first_run.status.as_deref(),
        &[
            "NC-HELM stores encrypted player keys in SQLite.",
            "Tab cycles fields. Enter submits. Esc quits.",
        ],
    );
    let track_width = gate_track_width(layout.content_width);
    draw_boxed_input_row(
        buffer,
        layout.content_left,
        layout.next_row,
        FORM_FIELD_LABEL_WIDTH,
        track_width,
        "Handle",
        &first_run.handle_input,
        first_run.active_field == FirstRunField::Handle,
        false,
    );
    draw_boxed_input_row(
        buffer,
        layout.content_left,
        layout.next_row + 1,
        FORM_FIELD_LABEL_WIDTH,
        track_width,
        "Password",
        &first_run.password_input,
        first_run.active_field == FirstRunField::Password,
        true,
    );
    draw_boxed_input_row(
        buffer,
        layout.content_left,
        layout.next_row + 2,
        FORM_FIELD_LABEL_WIDTH,
        track_width,
        "Confirm",
        &first_run.confirm_input,
        first_run.active_field == FirstRunField::Confirm,
        true,
    );
    draw_boxed_input_row(
        buffer,
        layout.content_left,
        layout.next_row + 3,
        FORM_FIELD_LABEL_WIDTH,
        track_width,
        "Relay",
        &first_run.relay_input,
        first_run.active_field == FirstRunField::Relay,
        false,
    );
    buffer.write_text_clipped(
        layout.next_row + 5,
        layout.content_left,
        &format!("Active relay: {relay_url}"),
        PANEL_DIM,
    );
}

fn render_locked(
    buffer: &mut PlayfieldBuffer,
    geometry: ScreenGeometry,
    _relay_url: &str,
    locked: &super::LockedModel,
) {
    let height = if locked.status.is_some() {
        LOCKED_GATE_HEIGHT_WITH_STATUS
    } else {
        LOCKED_GATE_HEIGHT
    };
    let (left, top, width, height) = centered_box_geometry(geometry, LOCKED_GATE_WIDTH, height);
    draw_panel(
        buffer,
        left,
        top,
        width,
        height,
        PANEL_BORDER,
        PANEL_ACCENT,
        Some(PANEL),
        Some("UNLOCK KEYCHAIN"),
        None,
    );
    let content_left = left + GATE_SIDE_PADDING;
    let mut row = top + 2;
    if let Some(status) = &locked.status {
        buffer.write_text_clipped(row, content_left, status, panel_status_style(status));
        row += 2;
    }
    let use_stormfaze = width >= GATE_STORMFAZE_MIN_WIDTH && height >= GATE_STORMFAZE_MIN_HEIGHT;
    draw_gate_wordmark(buffer, left, row, width, use_stormfaze);
    row += if use_stormfaze {
        GATE_LOGO_BLOCK_ROWS
    } else {
        2
    };
    let password_row = row + 1;
    draw_inline_unlock_password_row(buffer, content_left, password_row, &locked.password_input);
    buffer.write_text(
        password_row + 1,
        content_left,
        "Press Esc to quit.",
        PANEL_DIM,
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
        buffer.write_text_clipped(0, 25, identity, BODY);
    }
    let network_style = CellStyle::new(status_color(model.network), BODY.bg, true);
    buffer.write_text(0, 1, "NOSTRIAN CONQUEST", ROOT_TITLE);
    buffer.write_text(0, geometry.width().saturating_sub(25), "NETWORK:", DIM);
    buffer.write_text(
        0,
        geometry.width().saturating_sub(14),
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
            geometry.height().saturating_sub(2),
            1,
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
        1,
        4,
        geometry.width().saturating_sub(2),
        17,
        PANEL_BORDER,
        PANEL_ACCENT,
        Some(PANEL),
        Some("HOME"),
        None,
    );
    buffer.write_text(
        6,
        4,
        "NC-HELM runs the hosted lobby on a fresh TEA runtime.",
        PANEL_BODY,
    );
    buffer.write_text(
        7,
        4,
        "Background sync is isolated from the window loop.",
        PANEL_BODY,
    );
    buffer.write_text(9, 4, "Session", PANEL_ACCENT);
    if let Some(session) = &model.session {
        buffer.write_text(
            10,
            6,
            session
                .active_handle
                .as_deref()
                .unwrap_or("anonymous identity"),
            PANEL_BODY,
        );
        buffer.write_text_clipped(11, 6, &session.active_npub, PANEL_DIM);
    } else {
        buffer.write_text(10, 6, "No active session", PANEL_WARN);
    }
    buffer.write_text(13, 4, "Lobby Snapshot", PANEL_ACCENT);
    buffer.write_text(
        14,
        6,
        &format!("Open games : {}", model.games.len()),
        PANEL_BODY,
    );
    buffer.write_text(
        15,
        6,
        &format!("Notices    : {}", model.notices.len()),
        PANEL_BODY,
    );
    buffer.write_text(17, 4, "Shortcuts", PANEL_ACCENT);
    buffer.write_text(18, 6, "2 opens the game catalog.", PANEL_BODY);
    buffer.write_text(19, 6, "3 opens lobby notices/COMMS.", PANEL_BODY);
    buffer.write_text(20, 6, "4 opens settings and lock controls.", PANEL_BODY);
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
        1,
        4,
        table_width,
        geometry.height().saturating_sub(15),
        PANEL_BORDER,
        PANEL_ACCENT,
        Some(PANEL),
        Some("OPEN GAMES"),
        None,
    );
    draw_games_table(buffer, model, lobby);
    draw_status_panel(buffer, geometry, model);
    if let Some(selected) = model.games.get(lobby.selected_game) {
        let top = geometry.height().saturating_sub(10);
        draw_panel(
            buffer,
            geometry.width().saturating_sub(33),
            4,
            30,
            10,
            PANEL_BORDER,
            PANEL_ACCENT,
            Some(PANEL),
            Some("SELECTED GAME"),
            None,
        );
        let left = geometry.width().saturating_sub(31);
        buffer.write_text_clipped(6, left, &selected.name, PANEL_BODY);
        buffer.write_text_clipped(7, left, &format!("Host  : {}", selected.host), PANEL_BODY);
        buffer.write_text_clipped(8, left, &format!("Tier  : {}", selected.tier), PANEL_BODY);
        buffer.write_text_clipped(9, left, &format!("Seats : {}", selected.seats), PANEL_BODY);
        buffer.write_text_clipped(10, left, &format!("Turn  : {}", selected.when), PANEL_BODY);
        buffer.write_text_clipped(11, left, &selected.game_id, PANEL_DIM);
        buffer.write_text_clipped(top + 2, 3, "Use Up/Down to change selection.", DIM);
    }
}

fn draw_comms(buffer: &mut PlayfieldBuffer, geometry: ScreenGeometry, model: &Model) {
    draw_panel(
        buffer,
        1,
        4,
        geometry.width().saturating_sub(2),
        17,
        PANEL_BORDER,
        PANEL_ACCENT,
        Some(PANEL),
        Some("COMMS"),
        None,
    );
    draw_panel(
        buffer,
        3,
        6,
        geometry.width().saturating_sub(6),
        11,
        PANEL_BORDER,
        PANEL_ACCENT,
        None,
        Some("LOBBY NOTICES"),
        None,
    );
    if model.notices.is_empty() {
        buffer.write_text(10, 6, "No recent notices from the relay.", PANEL_DIM);
    } else {
        for (idx, notice) in model.notices.iter().take(10).enumerate() {
            buffer.write_text_clipped(8 + idx, 6, notice, PANEL_BODY);
        }
    }
    buffer.write_text(
        19,
        4,
        "Direct replies and threads are not wired yet.",
        PANEL_WARN,
    );
    draw_status_panel(buffer, geometry, model);
}

fn draw_settings(buffer: &mut PlayfieldBuffer, geometry: ScreenGeometry, model: &Model) {
    draw_panel(
        buffer,
        1,
        4,
        geometry.width().saturating_sub(2),
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
        4,
        7,
        SETTINGS_FIELD_LABEL_WIDTH,
        SETTINGS_FIELD_TRACK_WIDTH,
        "Relay URL",
        relay_draft,
        editing_relay,
        false,
    );
    buffer.write_text(
        8,
        4,
        &format!(
            "Window Focus : {}",
            if model.window_focused { "yes" } else { "no" }
        ),
        PANEL_BODY,
    );
    buffer.write_text(
        9,
        4,
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
            11,
            4,
            &format!(
                "Handle       : {}",
                session.active_handle.as_deref().unwrap_or("unset")
            ),
            PANEL_BODY,
        );
        buffer.write_text_clipped(
            12,
            4,
            &format!("Identity     : {}", session.active_npub),
            PANEL_BODY,
        );
    }
    buffer.write_text(
        14,
        4,
        "R : Edit relay URL   Enter : Save relay   Esc : Cancel edit",
        if editing_relay {
            PANEL_ACCENT
        } else {
            PANEL_DIM
        },
    );
    buffer.write_text(
        15,
        4,
        "L : Lock the local session and stop background sync",
        PANEL_ACCENT,
    );
    buffer.write_text(17, 4, "Esc/Q : Quit nc-helm", PANEL_DIM);
    draw_status_panel(buffer, geometry, model);
}

fn render_fatal(buffer: &mut PlayfieldBuffer, geometry: ScreenGeometry, message: &str) {
    centered_box(buffer, geometry, 64, 9, "FATAL", |buffer, left, top| {
        buffer.write_text(
            top + 2,
            left + 3,
            "The nc-helm bootstrap failed.",
            PANEL_ERROR,
        );
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
    let mut col = 1usize;
    for (label, tab) in tabs {
        let title_style = if lobby.active_tab == tab {
            ROOT_TITLE
        } else {
            DIM
        };
        let width = draw_top_tag(
            buffer,
            2,
            col,
            buffer.width().saturating_sub(col),
            label,
            ROOT_BORDER,
            title_style,
        );
        col += width + 2;
    }
}

fn draw_games_table(buffer: &mut PlayfieldBuffer, model: &Model, lobby: &super::LobbyModel) {
    buffer.write_text(
        6,
        4,
        "STAT  NAME                 HOST         TYPE     SEATS  YEAR",
        PANEL_DIM,
    );
    if model.games.is_empty() {
        buffer.write_text(
            8,
            4,
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
        buffer.write_text_clipped(7 + index, 4, &line, style);
    }
}

fn draw_status_panel(buffer: &mut PlayfieldBuffer, geometry: ScreenGeometry, model: &Model) {
    let top = geometry.height().saturating_sub(10);
    draw_panel(
        buffer,
        1,
        top,
        geometry.width().saturating_sub(2),
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
        buffer.write_text_clipped(top + 2 + idx, 3, &line, PANEL_BODY);
    }
}

fn draw_footer(
    buffer: &mut PlayfieldBuffer,
    geometry: ScreenGeometry,
    model: &Model,
    lobby: &super::LobbyModel,
) {
    let row = geometry.height().saturating_sub(1);
    let footer = if lobby.help_open {
        "Any key closes help."
    } else {
        "Tab next tab   ? help   Up/Down select   L lock   Esc quit"
    };
    buffer.write_text_clipped(row, 1, footer, DIM);
    if let Some(selected) = model.games.get(lobby.selected_game) {
        let text = format!("Selected: {}", selected.game_id);
        let start = geometry.width().saturating_sub(text.len() + 1);
        buffer.write_text_clipped(row, start, &text, DIM);
    }
}

fn draw_help_popup(buffer: &mut PlayfieldBuffer, geometry: ScreenGeometry) {
    let (left, top, width, height) =
        centered_box_geometry(geometry, HELP_POPUP_WIDTH, HELP_POPUP_HEIGHT);
    draw_panel(
        buffer,
        left,
        top,
        width,
        height,
        PANEL_BORDER,
        PANEL_ACCENT,
        Some(PANEL),
        Some("HELP"),
        None,
    );
    draw_top_tag_right(
        buffer,
        top,
        left,
        width,
        HELP_CLOSE_LABEL,
        PANEL_BORDER,
        PANEL_ACCENT,
    );

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
            } else if idx == 0 {
                PANEL_ACCENT
            } else {
                PANEL_BODY
            },
        );
    }
}

fn render_gate_shell(
    buffer: &mut PlayfieldBuffer,
    geometry: ScreenGeometry,
    width: usize,
    height: usize,
    title: &str,
    status: Option<&str>,
    copy_lines: &[&str],
) -> GateLayout {
    let (left, top, width, height) = centered_box_geometry(geometry, width, height);
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

    let content_left = left + GATE_SIDE_PADDING;
    let content_width = width.saturating_sub(GATE_SIDE_PADDING * 2 + 2);
    let mut row = top + 2;

    if let Some(status) = status {
        buffer.write_text_clipped(row, content_left, status, panel_status_style(status));
        row += 2;
    }

    let use_stormfaze = width >= GATE_STORMFAZE_MIN_WIDTH && height >= GATE_STORMFAZE_MIN_HEIGHT;
    draw_gate_wordmark(buffer, left, row, width, use_stormfaze);
    row += if use_stormfaze {
        GATE_LOGO_BLOCK_ROWS
    } else {
        2
    };

    if !copy_lines.is_empty() {
        for (idx, line) in copy_lines.iter().enumerate() {
            buffer.write_text_clipped(
                row + idx,
                content_left,
                line,
                if idx == 0 { PANEL_ACCENT } else { PANEL_DIM },
            );
        }
        row += copy_lines.len() + 1;
    }

    GateLayout {
        content_left,
        content_width,
        next_row: row,
    }
}

fn draw_gate_wordmark(
    buffer: &mut PlayfieldBuffer,
    left: usize,
    top: usize,
    width: usize,
    use_stormfaze: bool,
) {
    let logo_left = left + GATE_LOGO_WIDTH_INSET;
    let logo_width = width.saturating_sub(GATE_LOGO_WIDTH_INSET * 2);
    if use_stormfaze {
        buffer.push_overlay_text(
            "NOSTRIAN",
            OverlayTextFamily::Stormfaze,
            PANEL_BRAND,
            logo_left,
            top,
            logo_width,
            4,
        );
        buffer.push_overlay_text(
            "CONQUEST",
            OverlayTextFamily::Stormfaze,
            PANEL_BRAND,
            logo_left,
            top + 3,
            logo_width,
            4,
        );
        return;
    }

    let line_one = "NOSTRIAN";
    let line_two = "CONQUEST";
    let line_one_col = left + width.saturating_sub(line_one.chars().count()) / 2;
    let line_two_col = left + width.saturating_sub(line_two.chars().count()) / 2;
    buffer.write_text_clipped(top, line_one_col, line_one, PANEL_BRAND);
    buffer.write_text_clipped(top + 1, line_two_col, line_two, PANEL_BRAND);
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
    let (left, top, width, height) = centered_box_geometry(geometry, width, height);
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
    let track_style = PANEL_BODY;
    let value_style = field_style;
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

fn draw_inline_unlock_password_row(
    buffer: &mut PlayfieldBuffer,
    left: usize,
    row: usize,
    value: &str,
) {
    let prefix = "Password: ";
    let shown = "X".repeat(value.chars().count());
    buffer.write_text(row, left, prefix, PANEL_DIM);
    buffer.write_text_clipped(row, left + prefix.chars().count(), &shown, PANEL_ACCENT);
    let cursor_col = left + prefix.chars().count() + shown.chars().count();
    if cursor_col < buffer.width() {
        buffer.set_cursor(Point::new(Column(cursor_col), Row(row)));
    }
}

fn truncate(value: &str, width: usize) -> String {
    value.chars().take(width).collect()
}

fn gate_track_width(content_width: usize) -> usize {
    content_width.saturating_sub(FORM_FIELD_LABEL_WIDTH + 3)
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
