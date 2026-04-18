use super::{
    DEFAULT_GEOMETRY, FirstRunField, HELP_CLOSE_LABEL, HELP_POPUP_HEIGHT, HELP_POPUP_WIDTH,
    LOBBY_TAB_ROW, LobbyTab, Model, NetworkState, Route, centered_box_geometry,
    chrome::{draw_panel, draw_top_tag_right},
    lobby_tab_bounds, mask,
};
use crate::grid::OverlayTextFamily;
use crate::theme;
use crate::{CellStyle, Column, PlayfieldBuffer, Point, Row, ScreenGeometry, StyledSpan};

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
const COMMAND_PANEL_HEIGHT: usize = 3;

struct GateLayout {
    content_left: usize,
    content_width: usize,
    next_row: usize,
}

fn body() -> CellStyle {
    theme::body_style()
}

fn label() -> CellStyle {
    theme::label_style()
}

fn dim() -> CellStyle {
    theme::dim_style()
}

fn accent() -> CellStyle {
    theme::accent_style()
}

fn warning() -> CellStyle {
    theme::warning_style()
}

fn error() -> CellStyle {
    theme::error_style()
}

fn panel() -> CellStyle {
    theme::panel_style()
}

fn panel_dim() -> CellStyle {
    theme::panel_dim_style()
}

fn panel_accent() -> CellStyle {
    theme::panel_accent_style()
}

fn panel_brand() -> CellStyle {
    theme::panel_brand_style()
}

fn panel_warning() -> CellStyle {
    theme::panel_warning_style()
}

fn panel_error() -> CellStyle {
    theme::panel_error_style()
}

fn root_border() -> CellStyle {
    theme::root_border_style()
}

fn root_title() -> CellStyle {
    theme::root_title_style()
}

fn command_panel_top(geometry: ScreenGeometry) -> usize {
    geometry.height().saturating_sub(COMMAND_PANEL_HEIGHT)
}

fn lobby_status_row(geometry: ScreenGeometry) -> usize {
    command_panel_top(geometry).saturating_sub(1)
}

fn content_bottom_row(geometry: ScreenGeometry, reserve_status_row: bool) -> usize {
    if reserve_status_row {
        lobby_status_row(geometry).saturating_sub(1)
    } else {
        command_panel_top(geometry).saturating_sub(1)
    }
}

fn content_panel_height(geometry: ScreenGeometry, top: usize, reserve_status_row: bool) -> usize {
    content_bottom_row(geometry, reserve_status_row)
        .saturating_sub(top)
        .saturating_add(1)
}

fn network_style(network: NetworkState) -> CellStyle {
    let fg = match network {
        NetworkState::Idle => theme::idle_network_color(),
        NetworkState::Connecting => theme::connecting_network_color(),
        NetworkState::Synced => theme::synced_network_color(),
        NetworkState::Error => theme::error_network_color(),
    };
    CellStyle::new(fg, body().bg, true)
}

pub fn render(model: &Model) -> PlayfieldBuffer {
    let geometry = if model.geometry.width() == 0 || model.geometry.height() == 0 {
        DEFAULT_GEOMETRY
    } else {
        model.geometry
    };
    let mut buffer = PlayfieldBuffer::new(geometry.width(), geometry.height(), body());
    fill(&mut buffer, body());

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
            panel_accent(),
        );
        buffer.write_text(top + 4, left + 3, status, panel());
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
        panel_dim(),
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
        theme::panel_border_style(),
        panel_accent(),
        Some(panel()),
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
        panel_dim(),
    );
}

fn render_lobby(
    buffer: &mut PlayfieldBuffer,
    geometry: ScreenGeometry,
    model: &Model,
    lobby: &super::LobbyModel,
) {
    let reserve_status_row = lobby.status.is_some();
    let title = format!(
        "Nostrian Conquest <v{}>",
        short_version_label(env!("CARGO_PKG_VERSION"))
    );
    let network = format!(
        "NETWORK: {}",
        match model.network {
            NetworkState::Idle => "IDLE",
            NetworkState::Connecting => "CONNECTING",
            NetworkState::Synced => "SYNCED",
            NetworkState::Error => "ERROR",
        }
    );
    let title_col = 1usize;
    let network_col = geometry
        .width()
        .saturating_sub(network.chars().count().saturating_add(1));
    let left_gap_start = title_col + title.chars().count() + 1;
    let right_gap_end = network_col.saturating_sub(1);
    let network_status = match model.network {
        NetworkState::Idle => "IDLE",
        NetworkState::Connecting => "CONNECTING",
        NetworkState::Synced => "SYNCED",
        NetworkState::Error => "ERROR",
    };
    buffer.write_text_clipped(0, title_col, &title, root_title());
    if let Some(session) = &model.session {
        let identity = session
            .active_handle
            .as_deref()
            .unwrap_or(session.active_npub.as_str());
        let available_width = right_gap_end
            .saturating_sub(left_gap_start)
            .saturating_add(1);
        if available_width > 0 {
            let identity_text: String = identity.chars().take(available_width).collect();
            let centered_col = geometry
                .width()
                .saturating_sub(identity_text.chars().count())
                / 2;
            let identity_col = centered_col.clamp(
                left_gap_start,
                right_gap_end
                    .saturating_sub(identity_text.chars().count())
                    .saturating_add(1),
            );
            buffer.write_text_clipped(0, identity_col, &identity_text, body());
        }
    }
    let label_len = network.len().saturating_sub(network_status.chars().count());
    buffer.write_text(0, network_col, &network[..label_len], label());
    buffer.write_text(
        0,
        network_col + label_len,
        network_status,
        network_style(model.network),
    );

    draw_tabs(buffer, geometry, lobby);
    match lobby.active_tab {
        LobbyTab::MyGames => draw_my_games(buffer, geometry, model, reserve_status_row),
        LobbyTab::OpenGames => draw_open_games(buffer, geometry, model, lobby, reserve_status_row),
        LobbyTab::Comms => draw_comms(buffer, geometry, model, reserve_status_row),
        LobbyTab::Settings => draw_settings(buffer, geometry, model, reserve_status_row),
    }
    draw_command_panel(buffer, geometry);

    if let Some(status) = &lobby.status {
        buffer.write_text(lobby_status_row(geometry), 1, status, status_style(status));
    }
    if lobby.help_open {
        draw_help_popup(buffer, geometry);
    }
}

fn draw_my_games(
    buffer: &mut PlayfieldBuffer,
    geometry: ScreenGeometry,
    model: &Model,
    reserve_status_row: bool,
) {
    draw_panel(
        buffer,
        1,
        4,
        geometry.width().saturating_sub(2),
        content_panel_height(geometry, 4, reserve_status_row),
        theme::panel_border_style(),
        panel_accent(),
        Some(panel()),
        Some("MY GAMES"),
        None,
    );
    buffer.write_text(
        6,
        4,
        "NC-HELM runs the hosted lobby on a fresh TEA runtime.",
        panel(),
    );
    buffer.write_text(
        7,
        4,
        "Background sync is isolated from the window loop.",
        panel(),
    );
    buffer.write_text(9, 4, "Session", panel_accent());
    if let Some(session) = &model.session {
        buffer.write_text(
            10,
            6,
            session
                .active_handle
                .as_deref()
                .unwrap_or("anonymous identity"),
            panel(),
        );
        buffer.write_text_clipped(11, 6, &session.active_npub, panel_dim());
    } else {
        buffer.write_text(10, 6, "No active session", panel_warning());
    }
    buffer.write_text(13, 4, "Lobby Snapshot", panel_accent());
    buffer.write_text(
        14,
        6,
        &format!("Open games : {}", model.games.len()),
        panel(),
    );
    buffer.write_text(
        15,
        6,
        &format!("Notices    : {}", model.notices.len()),
        panel(),
    );
    buffer.write_text(17, 4, "Shortcuts", panel_accent());
    buffer.write_text(18, 6, "O opens the game catalog.", panel());
    buffer.write_text(19, 6, "C opens lobby notices and comms.", panel());
    buffer.write_text(20, 6, "S opens settings and lock controls.", panel());
}

fn draw_open_games(
    buffer: &mut PlayfieldBuffer,
    geometry: ScreenGeometry,
    model: &Model,
    lobby: &super::LobbyModel,
    reserve_status_row: bool,
) {
    let table_width = geometry.width().saturating_sub(36);
    draw_panel(
        buffer,
        1,
        4,
        table_width,
        content_panel_height(geometry, 4, reserve_status_row),
        theme::panel_border_style(),
        panel_accent(),
        Some(panel()),
        Some("OPEN GAMES"),
        None,
    );
    draw_games_table(buffer, model, lobby);
    if let Some(selected) = model.games.get(lobby.selected_game) {
        draw_panel(
            buffer,
            geometry.width().saturating_sub(33),
            4,
            30,
            content_panel_height(geometry, 4, reserve_status_row),
            theme::panel_border_style(),
            panel_accent(),
            Some(panel()),
            Some("SELECTED GAME"),
            None,
        );
        let left = geometry.width().saturating_sub(31);
        let bottom = content_bottom_row(geometry, reserve_status_row);
        buffer.write_text_clipped(6, left, &selected.name, panel());
        buffer.write_text_clipped(7, left, &format!("Host  : {}", selected.host), panel());
        buffer.write_text_clipped(8, left, &format!("Tier  : {}", selected.tier), panel());
        buffer.write_text_clipped(9, left, &format!("Seats : {}", selected.seats), panel());
        buffer.write_text_clipped(10, left, &format!("Turn  : {}", selected.when), panel());
        buffer.write_text_clipped(11, left, &selected.game_id, panel_dim());
        buffer.write_text_clipped(
            bottom.saturating_sub(1),
            left,
            "Use Up/Down to change selection.",
            panel_dim(),
        );
    }
}

fn draw_comms(
    buffer: &mut PlayfieldBuffer,
    geometry: ScreenGeometry,
    model: &Model,
    reserve_status_row: bool,
) {
    draw_panel(
        buffer,
        1,
        4,
        geometry.width().saturating_sub(2),
        content_panel_height(geometry, 4, reserve_status_row),
        theme::panel_border_style(),
        panel_accent(),
        Some(panel()),
        Some("COMMS"),
        None,
    );
    draw_panel(
        buffer,
        3,
        6,
        geometry.width().saturating_sub(6),
        content_panel_height(geometry, 6, reserve_status_row),
        theme::panel_border_style(),
        panel_accent(),
        None,
        Some("LOBBY NOTICES"),
        None,
    );
    if model.notices.is_empty() {
        buffer.write_text(10, 6, "No recent notices from the relay.", panel_dim());
    } else {
        for (idx, notice) in model.notices.iter().take(10).enumerate() {
            buffer.write_text_clipped(8 + idx, 6, notice, panel());
        }
    }
    buffer.write_text(
        content_bottom_row(geometry, reserve_status_row).saturating_sub(1),
        4,
        "Direct replies and threads are not wired yet.",
        panel_warning(),
    );
}

fn draw_settings(
    buffer: &mut PlayfieldBuffer,
    geometry: ScreenGeometry,
    model: &Model,
    reserve_status_row: bool,
) {
    draw_panel(
        buffer,
        1,
        4,
        geometry.width().saturating_sub(2),
        content_panel_height(geometry, 4, reserve_status_row),
        theme::panel_border_style(),
        panel_accent(),
        Some(panel()),
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
        panel(),
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
        panel(),
    );
    if let Some(session) = &model.session {
        buffer.write_text(
            11,
            4,
            &format!(
                "Handle       : {}",
                session.active_handle.as_deref().unwrap_or("unset")
            ),
            panel(),
        );
        buffer.write_text_clipped(
            12,
            4,
            &format!("Identity     : {}", session.active_npub),
            panel(),
        );
    }
    buffer.write_text(
        14,
        4,
        "R : Edit relay URL   Enter : Save relay   Esc : Cancel edit",
        if editing_relay {
            panel_accent()
        } else {
            panel_dim()
        },
    );
    buffer.write_text(
        15,
        4,
        "L : Lock the local session and stop background sync",
        panel_accent(),
    );
    buffer.write_text(17, 4, "Alt+Q : Quit nc-helm", panel_dim());
}

fn render_fatal(buffer: &mut PlayfieldBuffer, geometry: ScreenGeometry, message: &str) {
    centered_box(buffer, geometry, 64, 9, "FATAL", |buffer, left, top| {
        buffer.write_text(
            top + 2,
            left + 3,
            "The nc-helm bootstrap failed.",
            panel_error(),
        );
        buffer.write_text_clipped(top + 4, left + 3, message, panel());
        buffer.write_text(top + 6, left + 3, "Press Q or Esc to quit.", panel_dim());
    });
}

fn draw_tabs(buffer: &mut PlayfieldBuffer, geometry: ScreenGeometry, lobby: &super::LobbyModel) {
    for (tab, start, _) in lobby_tab_bounds(geometry) {
        let label = tab.label();
        let label_style = if lobby.active_tab == tab {
            root_title()
        } else {
            dim()
        };
        let spans = [
            StyledSpan::new("[", root_border()),
            StyledSpan::new(label, label_style),
            StyledSpan::new("]", root_border()),
        ];
        let _ = buffer.write_spans_clipped(LOBBY_TAB_ROW, start, &spans);
    }
}

fn draw_games_table(buffer: &mut PlayfieldBuffer, model: &Model, lobby: &super::LobbyModel) {
    buffer.write_text(
        6,
        4,
        "STAT  NAME                 HOST         TYPE     SEATS  YEAR",
        panel_dim(),
    );
    if model.games.is_empty() {
        buffer.write_text(
            8,
            4,
            "No open games synced yet. Leave the client running.",
            panel_warning(),
        );
        return;
    }
    for (index, row) in model.games.iter().enumerate() {
        let style = if index == lobby.selected_game {
            theme::selected_panel_row_style()
        } else {
            panel()
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

fn draw_command_panel(buffer: &mut PlayfieldBuffer, geometry: ScreenGeometry) {
    let top = command_panel_top(geometry);
    let left = 1usize;
    let width = geometry.width().saturating_sub(2);
    draw_panel(
        buffer,
        left,
        top,
        width,
        COMMAND_PANEL_HEIGHT,
        theme::panel_border_style(),
        panel_accent(),
        Some(panel()),
        Some("COMMANDS"),
        None,
    );
    let bg = panel().bg;
    let spans = [
        StyledSpan::new("Alt+ ", theme::prompt_style_on(bg)),
        StyledSpan::new("Q", theme::prompt_hotkey_style_on(bg)),
        StyledSpan::new(">", theme::prompt_delimiter_style_on(bg)),
        StyledSpan::new("uit ", theme::prompt_style_on(bg)),
        StyledSpan::new("<", theme::prompt_delimiter_style_on(bg)),
        StyledSpan::new("?", theme::prompt_hotkey_style_on(bg)),
        StyledSpan::new(">", theme::prompt_delimiter_style_on(bg)),
        StyledSpan::new("Keys ", theme::prompt_style_on(bg)),
        StyledSpan::new("H", theme::prompt_hotkey_style_on(bg)),
        StyledSpan::new(">", theme::prompt_delimiter_style_on(bg)),
        StyledSpan::new("ints ", theme::prompt_style_on(bg)),
        StyledSpan::new("L", theme::prompt_hotkey_style_on(bg)),
        StyledSpan::new(">", theme::prompt_delimiter_style_on(bg)),
        StyledSpan::new("ock ", theme::prompt_style_on(bg)),
        StyledSpan::new("M", theme::prompt_hotkey_style_on(bg)),
        StyledSpan::new(">", theme::prompt_delimiter_style_on(bg)),
        StyledSpan::new("y Games ", theme::prompt_style_on(bg)),
        StyledSpan::new("O", theme::prompt_hotkey_style_on(bg)),
        StyledSpan::new(">", theme::prompt_delimiter_style_on(bg)),
        StyledSpan::new("pen Games ", theme::prompt_style_on(bg)),
        StyledSpan::new("C", theme::prompt_hotkey_style_on(bg)),
        StyledSpan::new(">", theme::prompt_delimiter_style_on(bg)),
        StyledSpan::new("omms ", theme::prompt_style_on(bg)),
        StyledSpan::new("S", theme::prompt_hotkey_style_on(bg)),
        StyledSpan::new(">", theme::prompt_delimiter_style_on(bg)),
        StyledSpan::new("ettings", theme::prompt_style_on(bg)),
    ];
    let text_width = spans
        .iter()
        .map(|span| span.text.chars().count())
        .sum::<usize>();
    let inner_left = left + 2;
    let inner_width = width.saturating_sub(4);
    let row = top + 1;
    let start = if text_width >= inner_width {
        inner_left
    } else {
        inner_left + (inner_width - text_width) / 2
    };
    let _ = buffer.write_spans_clipped(row, start, &spans);
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
        theme::panel_border_style(),
        panel_accent(),
        Some(panel()),
        Some("HELP"),
        None,
    );
    draw_top_tag_right(
        buffer,
        top,
        left,
        width,
        HELP_CLOSE_LABEL,
        theme::panel_border_style(),
        panel_accent(),
    );

    let lines = [
        "NC-HELM is the new TEA player client.",
        "",
        "M/O/C/S : switch lobby tabs",
        "Up/Down : move the open-game cursor",
        "? or H  : reopen this help popup",
        "Q or Esc: close this popup",
        "Alt+Q   : quit the client",
        "",
        "The lobby sync runs in the background.",
    ];
    for (idx, line) in lines.iter().enumerate() {
        buffer.write_text_clipped(
            top + 2 + idx,
            left + 3,
            line,
            if line.is_empty() {
                panel()
            } else if idx == 0 {
                panel_accent()
            } else {
                panel()
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
        theme::panel_border_style(),
        panel_accent(),
        Some(panel()),
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
                if idx == 0 {
                    panel_accent()
                } else {
                    panel_dim()
                },
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
            panel_brand(),
            logo_left,
            top,
            logo_width,
            4,
        );
        buffer.push_overlay_text(
            "CONQUEST",
            OverlayTextFamily::Stormfaze,
            panel_brand(),
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
    buffer.write_text_clipped(top, line_one_col, line_one, panel_brand());
    buffer.write_text_clipped(top + 1, line_two_col, line_two, panel_brand());
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
        theme::panel_border_style(),
        panel_accent(),
        Some(panel()),
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
    let field_style = if active { panel_accent() } else { panel() };
    let track_style = panel();
    let value_style = field_style;
    let field_left = left + label_width + 2;
    let field_width = track_width.min(buffer.width().saturating_sub(field_left));
    let text_col = field_left.saturating_add(1);
    buffer.write_text(row, left, &format!("{label:<label_width$}: "), panel_dim());
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
    buffer.write_text(row, left, prefix, panel_dim());
    buffer.write_text_clipped(row, left + prefix.chars().count(), &shown, panel_accent());
    let cursor_col = left + prefix.chars().count() + shown.chars().count();
    if cursor_col < buffer.width() {
        buffer.set_cursor(Point::new(Column(cursor_col), Row(row)));
    }
}

fn truncate(value: &str, width: usize) -> String {
    value.chars().take(width).collect()
}

fn short_version_label(version: &str) -> String {
    let (release, prerelease) = match version.split_once('-') {
        Some(parts) => parts,
        None => return version.to_string(),
    };
    let mut release_parts = release.split('.');
    let major = release_parts.next().unwrap_or(release);
    let minor = release_parts.next().unwrap_or("0");
    let release_label = format!("{major}.{minor}");
    let prerelease_label = prerelease
        .strip_prefix("beta.")
        .map(|suffix| format!("b.{suffix}"))
        .or_else(|| {
            prerelease
                .strip_prefix("alpha.")
                .map(|suffix| format!("a.{suffix}"))
        })
        .unwrap_or_else(|| prerelease.to_string());
    format!("{release_label}-{prerelease_label}")
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
        error()
    } else if status.contains("sync")
        || status.contains("created")
        || status.contains("unlocked")
        || status.contains("saved")
    {
        accent()
    } else {
        warning()
    }
}

fn panel_status_style(status: &str) -> CellStyle {
    if status.contains("error")
        || status.contains("invalid")
        || status.contains("failed")
        || status.contains("empty")
    {
        panel_error()
    } else if status.contains("sync")
        || status.contains("created")
        || status.contains("unlocked")
        || status.contains("saved")
    {
        panel_accent()
    } else {
        panel_warning()
    }
}
