use super::{
    DEFAULT_GEOMETRY, FirstJoinSetupField, FirstRunField, HELP_CLOSE_LABEL, HELP_POPUP_HEIGHT,
    HELP_POPUP_WIDTH, LOBBY_TAB_ROW, LobbyTab, MIN_SUPPORTED_GEOMETRY, Model, NetworkState, Route,
    centered_box_geometry,
    chrome::{draw_modal_panel, draw_panel, draw_top_tag_right},
    lobby_tab_bounds, mask,
};
use crate::dashboard::table::{
    TableAlign, TableColumn as DashboardTableColumn, table_render_width,
};
use crate::grid::OverlayLogoKind;
use crate::theme;
use crate::{
    CellStyle, Column, GameColor, PlayfieldBuffer, Point, Row, ScreenGeometry, StyledSpan,
};
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::time::Duration;

const FORM_FIELD_LABEL_WIDTH: usize = 9;
const SETTINGS_FIELD_LABEL_WIDTH: usize = 12;
const SETTINGS_FIELD_TRACK_WIDTH: usize = 44;
const FIRST_RUN_GATE_WIDTH: usize = 68;
const FIRST_RUN_GATE_HEIGHT: usize = 22;
const LOCKED_GATE_WIDTH: usize = 60;
const LOCKED_GATE_HEIGHT: usize = 14;
const LOCKED_GATE_HEIGHT_WITH_STATUS: usize = 16;
const SANDBOX_JOIN_CONFIRM_WIDTH: usize = 64;
const SANDBOX_JOIN_CONFIRM_HEIGHT: usize = 11;
const SANDBOX_JOIN_UNAVAILABLE_WIDTH: usize = 68;
const SANDBOX_JOIN_UNAVAILABLE_HEIGHT: usize = 11;
const SANDBOX_DELETE_CONFIRM_WIDTH: usize = 68;
const SANDBOX_DELETE_CONFIRM_HEIGHT: usize = 11;
const FIRST_JOIN_GATE_WIDTH: usize = 72;
const FIRST_JOIN_GATE_HEIGHT: usize = 18;
const GATE_SIDE_PADDING: usize = 3;
const GATE_LOGO_WIDTH_INSET: usize = 3;
const GATE_STORMFAZE_MIN_WIDTH: usize = 48;
const GATE_STORMFAZE_MIN_HEIGHT: usize = 13;
const GATE_LOGO_BLOCK_ROWS: usize = 8;
const COMMAND_PANEL_HEIGHT: usize = 3;
const HEADER_WORDMARK_WIDTH: usize = 22;
const LOBBY_PANEL_TOP: usize = 4;
const LOBBY_PANEL_INSET_X: usize = 2;
const LOBBY_PANEL_TABLE_WIDTH_GUTTER: usize = 1;
const LOBBY_TABLE_GAME_WIDTH: usize = 24;
const LOBBY_OPEN_GAME_WIDTH: usize = 17;

struct GateLayout {
    content_left: usize,
    content_width: usize,
    next_row: usize,
}

struct LobbyPanelLayout {
    left: usize,
    top: usize,
    width: usize,
    height: usize,
    content_left: usize,
    content_width: usize,
    first_content_row: usize,
    first_data_row: usize,
    visible_data_rows: usize,
    scrollbar_col: usize,
}

#[derive(Clone, Copy)]
struct LobbyBodyLayout {
    top: usize,
    height: usize,
}

#[derive(Debug, Clone, Default)]
pub(crate) struct ViewCache {
    simple: SimpleViewCache,
    lobby: LobbyViewCache,
    hosted: HostedGameViewCache,
    last_hit: bool,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct ViewRenderTimings {
    pub hosted_dashboard_render: Duration,
    pub hosted_convert: Duration,
    pub hosted_dirty_regions: usize,
    pub hosted_full_rebuild: bool,
}

#[derive(Debug, Clone, Default)]
struct SimpleViewCache {
    key: Option<u64>,
    buffer: Option<PlayfieldBuffer>,
}

#[derive(Debug, Clone, Default)]
struct LobbyViewCache {
    shell_key: Option<LobbyShellKey>,
    content_key: Option<LobbyContentKey>,
    shell_buffer: Option<PlayfieldBuffer>,
    buffer: Option<PlayfieldBuffer>,
}

#[derive(Debug, Clone)]
struct HostedGameViewCache {
    key: Option<HostedRenderKey>,
    region_hashes: Option<crate::dashboard::app::render::RegionHashes>,
    dashboard_buffer: crate::dashboard::buffer::PlayfieldBuffer,
    buffer: Option<PlayfieldBuffer>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct LobbyShellKey {
    geometry: ScreenGeometry,
    network: NetworkState,
    active_tab: LobbyTab,
    reserve_status_row: bool,
    identity_hash: u64,
    my_games_len: usize,
    open_games_len: usize,
    settings_rows: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct LobbyContentKey {
    active_tab: LobbyTab,
    selected_my_game: usize,
    my_games_scroll: usize,
    selected_open_game: usize,
    open_games_scroll: usize,
    settings_scroll: usize,
    editing_relay: bool,
    relay_draft: String,
    relay_url: String,
    status: Option<String>,
    help_open: bool,
    quit_confirm_open: bool,
    my_games_hash: u64,
    open_games_hash: u64,
    notices_hash: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct HostedRenderKey {
    width: usize,
    height: usize,
    game_data_revision: u64,
    player_record_index_1_based: usize,
    focus: crate::dashboard::app::state::PanelFocus,
    help_return_overlay: crate::dashboard::app::state::ActiveOverlay,
    overlay_position: Option<crate::dashboard::overlays::frame::RelativePopupOrigin>,
    popup_position: Option<crate::dashboard::overlays::frame::RelativePopupOrigin>,
    help_return_overlay_position: Option<crate::dashboard::overlays::frame::RelativePopupOrigin>,
    mouse_gesture: crate::dashboard::app::state::ActiveMouseGesture,
    crosshair_x: u8,
    crosshair_y: u8,
    map_view_mode: crate::dashboard::app::state::MapViewMode,
    map_zoom_level: u8,
    dense_empty_sector_dots: bool,
    diplomacy_scroll: usize,
    command_line_toast_message: Option<String>,
    report_block_rows_len: usize,
    queued_mail_len: usize,
    is_terminal_too_small: bool,
}

enum SettingsRow {
    RelayInput,
    Text { content: String, style: CellStyle },
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

fn lobby_body_layout(geometry: ScreenGeometry, reserve_status_row: bool) -> LobbyBodyLayout {
    LobbyBodyLayout {
        top: LOBBY_PANEL_TOP,
        height: content_panel_height(geometry, LOBBY_PANEL_TOP, reserve_status_row),
    }
}

fn centered_lobby_panel_top(body: LobbyBodyLayout, panel_height: usize) -> usize {
    body.top + body.height.saturating_sub(panel_height) / 2
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

impl Default for HostedGameViewCache {
    fn default() -> Self {
        Self {
            key: None,
            region_hashes: None,
            dashboard_buffer: crate::dashboard::buffer::PlayfieldBuffer::new(
                0,
                0,
                crate::dashboard::buffer::CellStyle::new(
                    crate::dashboard::buffer::GameColor::Black,
                    crate::dashboard::buffer::GameColor::Black,
                    false,
                ),
            ),
            buffer: None,
        }
    }
}

impl ViewCache {
    pub(crate) fn render<'a>(
        &'a mut self,
        model: &Model,
    ) -> (bool, ViewRenderTimings, &'a PlayfieldBuffer) {
        let geometry = normalized_geometry(model);
        if geometry.width() < MIN_SUPPORTED_GEOMETRY.width()
            || geometry.height() < MIN_SUPPORTED_GEOMETRY.height()
        {
            return self.render_simple(model);
        }
        match &model.route {
            Route::Lobby(lobby) => self.render_lobby(model, lobby),
            Route::HostedGame(hosted) => self.render_hosted(hosted),
            _ if is_simple_route_cacheable(&model.route) => self.render_simple(model),
            _ => self.render_simple_uncached(model),
        }
    }

    fn render_simple<'a>(
        &'a mut self,
        model: &Model,
    ) -> (bool, ViewRenderTimings, &'a PlayfieldBuffer) {
        let key = simple_render_key(model);
        if self.simple.key != Some(key) || self.simple.buffer.is_none() {
            self.simple.key = Some(key);
            self.simple.buffer = Some(render(model));
            self.last_hit = false;
        } else {
            self.last_hit = true;
        }
        (
            self.last_hit,
            ViewRenderTimings::default(),
            self.simple.buffer.as_ref().expect("simple buffer"),
        )
    }

    fn render_simple_uncached<'a>(
        &'a mut self,
        model: &Model,
    ) -> (bool, ViewRenderTimings, &'a PlayfieldBuffer) {
        self.simple.key = None;
        self.simple.buffer = Some(render(model));
        self.last_hit = false;
        (
            self.last_hit,
            ViewRenderTimings::default(),
            self.simple.buffer.as_ref().expect("simple buffer"),
        )
    }

    fn render_lobby<'a>(
        &'a mut self,
        model: &Model,
        lobby: &super::LobbyModel,
    ) -> (bool, ViewRenderTimings, &'a PlayfieldBuffer) {
        let geometry = normalized_geometry(model);
        let reserve_status_row = lobby.status.is_some();
        let shell_key = lobby_shell_key(model, lobby, geometry, reserve_status_row);
        let content_key = lobby_content_key(model, lobby);
        let shell_changed =
            self.lobby.shell_key.as_ref() != Some(&shell_key) || self.lobby.shell_buffer.is_none();

        if shell_changed {
            let mut shell_buffer =
                PlayfieldBuffer::new(geometry.width(), geometry.height(), body());
            fill(&mut shell_buffer, body());
            render_lobby_shell(
                &mut shell_buffer,
                geometry,
                model,
                lobby,
                reserve_status_row,
            );
            let mut buffer = shell_buffer.clone();
            render_lobby_content(&mut buffer, geometry, model, lobby, reserve_status_row);
            self.lobby.shell_key = Some(shell_key);
            self.lobby.content_key = Some(content_key);
            self.lobby.shell_buffer = Some(shell_buffer);
            self.lobby.buffer = Some(buffer);
            self.last_hit = false;
        } else if self.lobby.content_key.as_ref() != Some(&content_key)
            || self.lobby.buffer.is_none()
        {
            let mut buffer = self
                .lobby
                .shell_buffer
                .as_ref()
                .expect("lobby shell buffer")
                .clone();
            render_lobby_content(&mut buffer, geometry, model, lobby, reserve_status_row);
            self.lobby.content_key = Some(content_key);
            self.lobby.buffer = Some(buffer);
            self.last_hit = false;
        } else {
            self.last_hit = true;
        }

        (
            self.last_hit,
            ViewRenderTimings::default(),
            self.lobby.buffer.as_ref().expect("lobby buffer"),
        )
    }

    fn render_hosted<'a>(
        &'a mut self,
        hosted: &super::HostedGameModel,
    ) -> (bool, ViewRenderTimings, &'a PlayfieldBuffer) {
        let key = hosted_render_key(&hosted.dashboard);
        let can_hit = hosted_render_is_cacheable(&hosted.dashboard);
        let mut timings = ViewRenderTimings::default();
        if !can_hit || self.hosted.key.as_ref() != Some(&key) || self.hosted.buffer.is_none() {
            let buffer = self.hosted.buffer.get_or_insert_with(|| {
                PlayfieldBuffer::new(
                    0,
                    0,
                    CellStyle::new(GameColor::Black, GameColor::Black, false),
                )
            });
            let render_result = crate::dashboard::render_hosted_buffer_incremental_into(
                &hosted.dashboard,
                self.hosted.region_hashes.as_ref(),
                &mut self.hosted.dashboard_buffer,
                buffer,
            )
            .expect("hosted dashboard should render");
            self.hosted.region_hashes = Some(render_result.hashes);
            self.hosted.key = can_hit.then_some(key);
            timings = ViewRenderTimings {
                hosted_dashboard_render: render_result.stats.dashboard_render,
                hosted_convert: render_result.stats.convert,
                hosted_dirty_regions: render_result.stats.dirty_regions,
                hosted_full_rebuild: render_result.stats.full_rebuild,
            };
            self.last_hit = false;
        } else {
            self.last_hit = true;
        }
        (
            self.last_hit,
            timings,
            self.hosted.buffer.as_ref().expect("hosted buffer"),
        )
    }
}

fn normalized_geometry(model: &Model) -> ScreenGeometry {
    if model.geometry.width() == 0 || model.geometry.height() == 0 {
        DEFAULT_GEOMETRY
    } else {
        model.geometry
    }
}

fn is_simple_route_cacheable(route: &Route) -> bool {
    !matches!(route, Route::MatrixLocked)
}

fn simple_render_key(model: &Model) -> u64 {
    let mut hasher = DefaultHasher::new();
    normalized_geometry(model).width().hash(&mut hasher);
    normalized_geometry(model).height().hash(&mut hasher);
    model.relay_url.hash(&mut hasher);
    match &model.route {
        Route::Boot(boot) => {
            0u8.hash(&mut hasher);
            boot.status.hash(&mut hasher);
        }
        Route::FirstRun(first_run) => {
            1u8.hash(&mut hasher);
            first_run.active_field.hash(&mut hasher);
            first_run.handle_input.hash(&mut hasher);
            first_run.password_input.hash(&mut hasher);
            first_run.confirm_input.hash(&mut hasher);
            first_run.relay_input.hash(&mut hasher);
            first_run.status.hash(&mut hasher);
        }
        Route::MatrixLocked => {
            2u8.hash(&mut hasher);
        }
        Route::Locked(locked) => {
            3u8.hash(&mut hasher);
            locked.password_input.hash(&mut hasher);
            locked.status.hash(&mut hasher);
            locked.resume_session.hash(&mut hasher);
        }
        Route::SandboxJoinConfirm(row) => {
            4u8.hash(&mut hasher);
            row.hash(&mut hasher);
        }
        Route::SandboxJoinUnavailable { row, notice } => {
            5u8.hash(&mut hasher);
            row.hash(&mut hasher);
            notice.hash(&mut hasher);
        }
        Route::SandboxDeleteConfirm(row) => {
            6u8.hash(&mut hasher);
            row.hash(&mut hasher);
        }
        Route::FirstJoinSetup(setup) => {
            7u8.hash(&mut hasher);
            setup.row.hash(&mut hasher);
            setup.empire_input.hash(&mut hasher);
            setup.homeworld_input.hash(&mut hasher);
            setup.active_field.hash(&mut hasher);
            setup.status.hash(&mut hasher);
            setup.homeworld_coords.hash(&mut hasher);
            setup.present_production.hash(&mut hasher);
            setup.potential_production.hash(&mut hasher);
        }
        Route::FatalError(message) => {
            8u8.hash(&mut hasher);
            message.hash(&mut hasher);
        }
        Route::Lobby(_) => {
            9u8.hash(&mut hasher);
        }
        Route::HostedGame(_) => {
            10u8.hash(&mut hasher);
        }
    }
    hasher.finish()
}

fn hash_value<T: Hash>(value: &T) -> u64 {
    let mut hasher = DefaultHasher::new();
    value.hash(&mut hasher);
    hasher.finish()
}

fn lobby_identity_hash(model: &Model) -> u64 {
    match &model.session {
        Some(session) => hash_value(&(session
            .active_handle
            .as_deref()
            .unwrap_or(session.active_npub.as_str()),)),
        None => 0,
    }
}

fn lobby_shell_key(
    model: &Model,
    lobby: &super::LobbyModel,
    geometry: ScreenGeometry,
    reserve_status_row: bool,
) -> LobbyShellKey {
    let settings_rows = wrapped_settings_rows(model, settings_content_width(geometry)).len();
    LobbyShellKey {
        geometry,
        network: model.network,
        active_tab: lobby.active_tab,
        reserve_status_row,
        identity_hash: lobby_identity_hash(model),
        my_games_len: model.my_games.len(),
        open_games_len: model.open_games.len(),
        settings_rows,
    }
}

fn lobby_content_key(model: &Model, lobby: &super::LobbyModel) -> LobbyContentKey {
    LobbyContentKey {
        active_tab: lobby.active_tab,
        selected_my_game: lobby.selected_my_game,
        my_games_scroll: lobby.my_games_scroll,
        selected_open_game: lobby.selected_open_game,
        open_games_scroll: lobby.open_games_scroll,
        settings_scroll: lobby.settings_scroll,
        editing_relay: lobby.editing_relay,
        relay_draft: lobby.relay_draft.clone(),
        relay_url: model.relay_url.clone(),
        status: lobby.status.clone(),
        help_open: lobby.help_open,
        quit_confirm_open: lobby.quit_confirm_open,
        my_games_hash: hash_value(&model.my_games),
        open_games_hash: hash_value(&model.open_games),
        notices_hash: hash_value(&model.notices),
    }
}

fn hosted_render_key(dashboard: &crate::dashboard::DashApp) -> HostedRenderKey {
    HostedRenderKey {
        width: dashboard.geometry.width(),
        height: dashboard.geometry.height(),
        game_data_revision: dashboard.game_data_revision,
        player_record_index_1_based: dashboard.player_record_index_1_based,
        focus: dashboard.focus,
        help_return_overlay: dashboard.help_return_overlay,
        overlay_position: dashboard.overlay_position,
        popup_position: dashboard.popup_position,
        help_return_overlay_position: dashboard.help_return_overlay_position,
        mouse_gesture: dashboard.mouse_gesture,
        crosshair_x: dashboard.crosshair_x,
        crosshair_y: dashboard.crosshair_y,
        map_view_mode: dashboard.map_view_mode,
        map_zoom_level: dashboard.map_zoom_level,
        dense_empty_sector_dots: dashboard.client_settings.dense_empty_sector_dots,
        diplomacy_scroll: dashboard.diplomacy_scroll,
        command_line_toast_message: dashboard.command_line_toast_message.clone(),
        report_block_rows_len: dashboard.report_block_rows.len(),
        queued_mail_len: dashboard.queued_mail.len(),
        is_terminal_too_small: dashboard.is_terminal_too_small,
    }
}

fn hosted_render_is_cacheable(dashboard: &crate::dashboard::DashApp) -> bool {
    dashboard.overlay == crate::dashboard::app::state::ActiveOverlay::None
        && dashboard.popup == crate::dashboard::app::state::ActivePopup::None
        && dashboard.overlay_position.is_none()
        && dashboard.popup_position.is_none()
        && dashboard.help_return_overlay == crate::dashboard::app::state::ActiveOverlay::None
        && dashboard.help_return_overlay_position.is_none()
        && dashboard.mouse_gesture == crate::dashboard::app::state::ActiveMouseGesture::None
}

pub fn render(model: &Model) -> PlayfieldBuffer {
    let geometry = if model.geometry.width() == 0 || model.geometry.height() == 0 {
        DEFAULT_GEOMETRY
    } else {
        model.geometry
    };
    let mut buffer = PlayfieldBuffer::new(geometry.width(), geometry.height(), body());
    fill(&mut buffer, body());

    if geometry.width() < MIN_SUPPORTED_GEOMETRY.width()
        || geometry.height() < MIN_SUPPORTED_GEOMETRY.height()
    {
        render_too_small(&mut buffer, geometry);
        return buffer;
    }

    match &model.route {
        Route::Boot(boot) => render_boot(&mut buffer, geometry, &boot.status),
        Route::FirstRun(first_run) => {
            render_first_run(&mut buffer, geometry, model.relay_url.as_str(), first_run)
        }
        Route::MatrixLocked => render_matrix_locked(&mut buffer, geometry, model),
        Route::Locked(locked) => {
            render_locked(&mut buffer, geometry, model.relay_url.as_str(), locked)
        }
        Route::Lobby(lobby) => render_lobby(&mut buffer, geometry, model, lobby),
        Route::SandboxJoinConfirm(row) => render_sandbox_join_confirm(&mut buffer, geometry, row),
        Route::SandboxJoinUnavailable { row, notice } => {
            render_sandbox_join_unavailable(&mut buffer, geometry, row, notice)
        }
        Route::SandboxDeleteConfirm(row) => {
            render_sandbox_delete_confirm(&mut buffer, geometry, row)
        }
        Route::FirstJoinSetup(setup) => render_first_join_setup(&mut buffer, geometry, setup),
        Route::HostedGame(hosted) => return render_hosted_game(geometry, hosted),
        Route::FatalError(message) => render_fatal(&mut buffer, geometry, message),
    }

    buffer
}

fn render_matrix_locked(buffer: &mut PlayfieldBuffer, geometry: ScreenGeometry, model: &Model) {
    let _ = geometry;
    model.matrix_rain.render(buffer);
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

fn render_sandbox_join_confirm(
    buffer: &mut PlayfieldBuffer,
    geometry: ScreenGeometry,
    row: &super::OpenGameRow,
) {
    centered_box(
        buffer,
        geometry,
        SANDBOX_JOIN_CONFIRM_WIDTH,
        SANDBOX_JOIN_CONFIRM_HEIGHT,
        "JOIN SANDBOX",
        |buffer, left, top| {
            let content_left = left + 3;
            buffer.write_text_clipped(
                top + 2,
                content_left,
                &format!("Game: {}", row.game),
                panel(),
            );
            buffer.write_text_clipped(
                top + 4,
                content_left,
                "Join this sandbox now?",
                panel_accent(),
            );
            buffer.write_text_clipped(
                top + 6,
                content_left,
                "Y joins an open seat immediately. Any other key cancels.",
                panel_dim(),
            );
        },
    );
}

fn render_sandbox_join_unavailable(
    buffer: &mut PlayfieldBuffer,
    geometry: ScreenGeometry,
    row: &super::OpenGameRow,
    notice: &str,
) {
    centered_box(
        buffer,
        geometry,
        SANDBOX_JOIN_UNAVAILABLE_WIDTH,
        SANDBOX_JOIN_UNAVAILABLE_HEIGHT,
        "SANDBOX UNAVAILABLE",
        |buffer, left, top| {
            let content_left = left + 3;
            buffer.write_text_clipped(
                top + 2,
                content_left,
                &format!("Game: {}", row.game),
                panel(),
            );
            buffer.write_text_clipped(top + 4, content_left, notice, panel_warning());
            buffer.write_text_clipped(
                top + 6,
                content_left,
                "Press any key to return to the lobby.",
                panel_dim(),
            );
        },
    );
}

fn render_sandbox_delete_confirm(
    buffer: &mut PlayfieldBuffer,
    geometry: ScreenGeometry,
    row: &super::MyGameRow,
) {
    centered_box(
        buffer,
        geometry,
        SANDBOX_DELETE_CONFIRM_WIDTH,
        SANDBOX_DELETE_CONFIRM_HEIGHT,
        "DELETE SANDBOX",
        |buffer, left, top| {
            let content_left = left + 3;
            buffer.write_text_clipped(
                top + 2,
                content_left,
                &format!("Game: {}", row.game),
                panel(),
            );
            buffer.write_text_clipped(
                top + 4,
                content_left,
                "Release this sandbox seat and remove it from My Games?",
                panel_accent(),
            );
            buffer.write_text_clipped(
                top + 6,
                content_left,
                "Y releases the seat. Any other key cancels.",
                panel_dim(),
            );
        },
    );
}

fn render_first_join_setup(
    buffer: &mut PlayfieldBuffer,
    geometry: ScreenGeometry,
    setup: &super::FirstJoinSetupModel,
) {
    let layout = render_gate_shell(
        buffer,
        geometry,
        FIRST_JOIN_GATE_WIDTH,
        FIRST_JOIN_GATE_HEIGHT,
        "FIRST JOIN SETUP",
        setup.status.as_deref(),
        &[
            "Name your empire and homeworld before entering the hosted game.",
            "Tab switches fields. Enter advances or submits. Esc returns to lobby.",
        ],
    );
    let track_width = gate_track_width(layout.content_width);
    draw_boxed_input_row(
        buffer,
        layout.content_left,
        layout.next_row,
        FORM_FIELD_LABEL_WIDTH,
        track_width,
        "Empire",
        &setup.empire_input,
        setup.active_field == FirstJoinSetupField::Empire,
        false,
    );
    draw_boxed_input_row(
        buffer,
        layout.content_left,
        layout.next_row + 1,
        FORM_FIELD_LABEL_WIDTH,
        track_width,
        "Homeworld",
        &setup.homeworld_input,
        setup.active_field == FirstJoinSetupField::Homeworld,
        false,
    );
    buffer.write_text_clipped(
        layout.next_row + 3,
        layout.content_left,
        &format!(
            "Coordinates: {},{}   Production: {}/{}",
            setup.homeworld_coords[0],
            setup.homeworld_coords[1],
            setup.present_production,
            setup.potential_production
        ),
        panel_dim(),
    );
}

fn render_hosted_game(
    geometry: ScreenGeometry,
    hosted: &super::HostedGameModel,
) -> PlayfieldBuffer {
    match crate::dashboard::render_hosted_buffer(&hosted.dashboard) {
        Ok(buffer) => buffer,
        Err(err) => {
            let mut buffer = PlayfieldBuffer::new(geometry.width(), geometry.height(), body());
            fill(&mut buffer, body());
            centered_box(
                &mut buffer,
                geometry,
                72,
                9,
                "HOSTED DASHBOARD ERROR",
                |buffer, left, top| {
                    buffer.write_text_clipped(
                        top + 2,
                        left + 3,
                        "Unable to render the hosted dashboard.",
                        panel_error(),
                    );
                    buffer.write_text_clipped(top + 4, left + 3, &err.to_string(), panel());
                    buffer.write_text_clipped(
                        top + 6,
                        left + 3,
                        "Press Alt+Q to quit.",
                        panel_dim(),
                    );
                },
            );
            buffer
        }
    }
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
    draw_modal_panel(
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
    render_lobby_shell(buffer, geometry, model, lobby, reserve_status_row);
    render_lobby_content(buffer, geometry, model, lobby, reserve_status_row);
}

fn render_lobby_shell(
    buffer: &mut PlayfieldBuffer,
    geometry: ScreenGeometry,
    model: &Model,
    lobby: &super::LobbyModel,
    reserve_status_row: bool,
) {
    let version = format!("<v{}>", short_version_label(env!("CARGO_PKG_VERSION")));
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
    let version_col = title_col + HEADER_WORDMARK_WIDTH + 1;
    let network_col = geometry
        .width()
        .saturating_sub(network.chars().count().saturating_add(1));
    let left_gap_start = version_col + version.chars().count() + 1;
    let right_gap_end = network_col.saturating_sub(1);
    let network_status = match model.network {
        NetworkState::Idle => "IDLE",
        NetworkState::Connecting => "CONNECTING",
        NetworkState::Synced => "SYNCED",
        NetworkState::Error => "ERROR",
    };
    buffer.push_overlay_logo(
        OverlayLogoKind::HeaderWordmark,
        root_title().fg,
        title_col,
        0,
    );
    buffer.write_text_clipped(0, version_col, &version, label());
    if let Some(session) = &model.session {
        let identity = session
            .active_handle
            .as_deref()
            .unwrap_or(session.active_npub.as_str());
        if right_gap_end >= left_gap_start {
            let available_width = right_gap_end - left_gap_start + 1;
            let identity_text: String = identity.chars().take(available_width).collect();
            let centered_col = geometry
                .width()
                .saturating_sub(identity_text.chars().count())
                / 2;
            let max_col = right_gap_end
                .saturating_sub(identity_text.chars().count())
                .saturating_add(1);
            let identity_col = centered_col.clamp(left_gap_start, max_col);
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
        LobbyTab::MyGames => draw_my_games_shell(buffer, geometry, model, reserve_status_row),
        LobbyTab::OpenGames => draw_open_games_shell(buffer, geometry, model, reserve_status_row),
        LobbyTab::Comms => draw_comms_shell(buffer, geometry, reserve_status_row),
        LobbyTab::Settings => draw_settings_shell(buffer, geometry, model, reserve_status_row),
    }
    draw_command_panel(buffer, geometry);
}

fn render_lobby_content(
    buffer: &mut PlayfieldBuffer,
    geometry: ScreenGeometry,
    model: &Model,
    lobby: &super::LobbyModel,
    reserve_status_row: bool,
) {
    match lobby.active_tab {
        LobbyTab::MyGames => draw_my_games_content(buffer, geometry, model, reserve_status_row),
        LobbyTab::OpenGames => draw_open_games_content(buffer, geometry, model, reserve_status_row),
        LobbyTab::Comms => draw_comms_content(buffer, geometry, model, reserve_status_row),
        LobbyTab::Settings => draw_settings_content(buffer, geometry, model, reserve_status_row),
    }
    if let Some(status) = &lobby.status {
        buffer.write_text(lobby_status_row(geometry), 1, status, status_style(status));
    }
    if lobby.help_open {
        draw_help_popup(buffer, geometry);
    }
    if lobby.quit_confirm_open {
        draw_lobby_quit_confirm_popup(buffer, geometry);
    }
}

fn draw_my_games_shell(
    buffer: &mut PlayfieldBuffer,
    geometry: ScreenGeometry,
    model: &Model,
    reserve_status_row: bool,
) {
    let columns = my_games_columns();
    let layout = lobby_table_panel_layout(
        geometry,
        reserve_status_row,
        "MY GAMES",
        &columns,
        model.my_games.len(),
    );
    draw_panel(
        buffer,
        layout.left,
        layout.top,
        layout.width,
        layout.height,
        theme::panel_border_style(),
        panel_accent(),
        Some(panel()),
        Some("MY GAMES"),
        None,
    );
}

fn draw_my_games_content(
    buffer: &mut PlayfieldBuffer,
    geometry: ScreenGeometry,
    model: &Model,
    reserve_status_row: bool,
) {
    let columns = my_games_columns();
    let layout = lobby_table_panel_layout(
        geometry,
        reserve_status_row,
        "MY GAMES",
        &columns,
        model.my_games.len(),
    );
    render_my_games_table(buffer, &layout, model);
}

fn draw_open_games_shell(
    buffer: &mut PlayfieldBuffer,
    geometry: ScreenGeometry,
    model: &Model,
    reserve_status_row: bool,
) {
    let columns = open_games_columns();
    let layout = lobby_table_panel_layout(
        geometry,
        reserve_status_row,
        "OPEN GAMES AVAILABLE TO JOIN",
        &columns,
        model.open_games.len(),
    );
    draw_panel(
        buffer,
        layout.left,
        layout.top,
        layout.width,
        layout.height,
        theme::panel_border_style(),
        panel_accent(),
        Some(panel()),
        Some("OPEN GAMES AVAILABLE TO JOIN"),
        None,
    );
}

fn draw_open_games_content(
    buffer: &mut PlayfieldBuffer,
    geometry: ScreenGeometry,
    model: &Model,
    reserve_status_row: bool,
) {
    let columns = open_games_columns();
    let layout = lobby_table_panel_layout(
        geometry,
        reserve_status_row,
        "OPEN GAMES AVAILABLE TO JOIN",
        &columns,
        model.open_games.len(),
    );
    render_open_games_table(buffer, &layout, model);
}

fn draw_comms_shell(
    buffer: &mut PlayfieldBuffer,
    geometry: ScreenGeometry,
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
}

fn draw_comms_content(
    buffer: &mut PlayfieldBuffer,
    geometry: ScreenGeometry,
    model: &Model,
    reserve_status_row: bool,
) {
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

fn draw_settings_shell(
    buffer: &mut PlayfieldBuffer,
    geometry: ScreenGeometry,
    model: &Model,
    reserve_status_row: bool,
) {
    let layout = settings_panel_layout(geometry, reserve_status_row, model);
    draw_panel(
        buffer,
        layout.left,
        layout.top,
        layout.width,
        layout.height,
        theme::panel_border_style(),
        panel_accent(),
        Some(panel()),
        Some("SETTINGS"),
        None,
    );
}

fn draw_settings_content(
    buffer: &mut PlayfieldBuffer,
    geometry: ScreenGeometry,
    model: &Model,
    reserve_status_row: bool,
) {
    let layout = settings_panel_layout(geometry, reserve_status_row, model);
    let (relay_draft, editing_relay) = if let Route::Lobby(lobby) = &model.route {
        (lobby.relay_draft.as_str(), lobby.editing_relay)
    } else {
        (model.relay_url.as_str(), false)
    };
    render_settings_rows(buffer, &layout, model, relay_draft, editing_relay);
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

fn render_too_small(buffer: &mut PlayfieldBuffer, geometry: ScreenGeometry) {
    let minimum = format!(
        "Minimum supported size: {}x{}",
        MIN_SUPPORTED_GEOMETRY.width(),
        MIN_SUPPORTED_GEOMETRY.height()
    );
    let current = format!("Current grid: {}x{}", geometry.width(), geometry.height());

    buffer.write_text_clipped(0, 0, "Window too small for nc-helm.", warning());
    buffer.write_text_clipped(1, 0, &minimum, panel_dim());
    buffer.write_text_clipped(2, 0, &current, panel_dim());
    buffer.write_text_clipped(4, 0, "Resize larger to restore the full lobby.", panel());
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

fn render_my_games_table(buffer: &mut PlayfieldBuffer, layout: &LobbyPanelLayout, model: &Model) {
    let base_columns = my_games_columns();
    let columns = resolve_lobby_table_columns(&base_columns, layout.content_width);
    render_table_header(
        buffer,
        layout.content_left,
        layout.first_content_row,
        &columns,
    );
    if model.my_games.is_empty() {
        buffer.write_text_clipped(
            layout.first_data_row + 1,
            layout.content_left,
            "<no games yet - press 'j' to join an open game>",
            panel_warning(),
        );
        return;
    }
    let scroll = lobby_my_games_scroll(model).min(
        model
            .my_games
            .len()
            .saturating_sub(layout.visible_data_rows.max(1)),
    );
    for (index, row) in model
        .my_games
        .iter()
        .enumerate()
        .skip(scroll)
        .take(layout.visible_data_rows)
    {
        let draw_row = layout.first_data_row + index - scroll;
        let style = if index == lobby_selected_my_game(model) {
            theme::selected_panel_row_style()
        } else {
            panel()
        };
        let values = [
            joined_game_status_label(&row.status).to_string(),
            row.game.clone(),
            row.game_tier.clone(),
            row.seat
                .map(|seat| seat.to_string())
                .unwrap_or_else(|| "-".to_string()),
            formatted_turn_summary(&row.turn_summary),
        ];
        render_table_row(
            buffer,
            layout.content_left,
            draw_row,
            &columns,
            &values,
            style,
        );
    }
    draw_lobby_scrollbar(
        buffer,
        layout.first_data_row,
        layout.scrollbar_col,
        layout.visible_data_rows,
        model.my_games.len(),
        scroll,
    );
}

fn render_open_games_table(buffer: &mut PlayfieldBuffer, layout: &LobbyPanelLayout, model: &Model) {
    let base_columns = open_games_columns();
    let columns = resolve_lobby_table_columns(&base_columns, layout.content_width);
    render_table_header(
        buffer,
        layout.content_left,
        layout.first_content_row,
        &columns,
    );
    if model.open_games.is_empty() {
        buffer.write_text_clipped(
            layout.first_data_row + 1,
            layout.content_left,
            "<no open games - check back later or ask the sysop in COMMS>",
            panel_warning(),
        );
        return;
    }
    let scroll = lobby_open_games_scroll(model).min(
        model
            .open_games
            .len()
            .saturating_sub(layout.visible_data_rows.max(1)),
    );
    for (index, row) in model
        .open_games
        .iter()
        .enumerate()
        .skip(scroll)
        .take(layout.visible_data_rows)
    {
        let draw_row = layout.first_data_row + index - scroll;
        let style = if index == lobby_selected_open_game(model) {
            theme::selected_panel_row_style()
        } else {
            panel()
        };
        let values = [
            row.status.clone(),
            row.game.clone(),
            row.host.clone(),
            row.game_tier.clone(),
            format!("{}/{}", row.open_seats, row.total_seats),
            map_size_summary(row.total_seats),
            row.created_date.clone(),
            formatted_turn_summary(&row.turn_summary),
        ];
        render_table_row(
            buffer,
            layout.content_left,
            draw_row,
            &columns,
            &values,
            style,
        );
    }
    draw_lobby_scrollbar(
        buffer,
        layout.first_data_row,
        layout.scrollbar_col,
        layout.visible_data_rows,
        model.open_games.len(),
        scroll,
    );
}

fn my_games_columns() -> [DashboardTableColumn<'static>; 5] {
    [
        DashboardTableColumn::left("Status", 10),
        DashboardTableColumn::left_flex("Game", LOBBY_TABLE_GAME_WIDTH, 1),
        DashboardTableColumn::left("Type", 9),
        DashboardTableColumn::right("Seat", 6),
        DashboardTableColumn::right("Time (Y:T)", 12),
    ]
}

fn open_games_columns() -> [DashboardTableColumn<'static>; 8] {
    [
        DashboardTableColumn::left("Status", 10),
        DashboardTableColumn::left_flex("Game", LOBBY_OPEN_GAME_WIDTH, 1),
        DashboardTableColumn::left("Host", 12),
        DashboardTableColumn::left("Type", 9),
        DashboardTableColumn::right("Seats", 7),
        DashboardTableColumn::right("Map", 7),
        DashboardTableColumn::right("Created", 10),
        DashboardTableColumn::right("Time", 12),
    ]
}

fn lobby_table_panel_layout(
    geometry: ScreenGeometry,
    reserve_status_row: bool,
    title: &str,
    columns: &[DashboardTableColumn<'_>],
    total_rows: usize,
) -> LobbyPanelLayout {
    let body = lobby_body_layout(geometry, reserve_status_row);
    let desired_width = (table_render_width(columns) + 5).max(title.chars().count() + 4);
    let width = desired_width.min(geometry.width().saturating_sub(2));
    let left = geometry.width().saturating_sub(width) / 2;
    let content_width =
        width.saturating_sub(LOBBY_PANEL_INSET_X * 2 + LOBBY_PANEL_TABLE_WIDTH_GUTTER);
    let max_visible_data_rows = body.height.saturating_sub(5).max(1);
    let desired_data_rows = if total_rows == 0 { 2 } else { total_rows };
    let visible_data_rows = desired_data_rows.min(max_visible_data_rows);
    let height = visible_data_rows + 5;
    let top = centered_lobby_panel_top(body, height);
    LobbyPanelLayout {
        left,
        top,
        width,
        height,
        content_left: left + LOBBY_PANEL_INSET_X,
        content_width,
        first_content_row: top + 2,
        first_data_row: top + 3,
        visible_data_rows,
        scrollbar_col: left + width.saturating_sub(2),
    }
}

fn settings_panel_layout(
    geometry: ScreenGeometry,
    reserve_status_row: bool,
    model: &Model,
) -> LobbyPanelLayout {
    let body = lobby_body_layout(geometry, reserve_status_row);
    let desired_width = (SETTINGS_FIELD_LABEL_WIDTH + SETTINGS_FIELD_TRACK_WIDTH + 5)
        .max("SETTINGS".chars().count() + 4);
    let width = desired_width.min(geometry.width().saturating_sub(2));
    let left = geometry.width().saturating_sub(width) / 2;
    let content_width =
        width.saturating_sub(LOBBY_PANEL_INSET_X * 2 + LOBBY_PANEL_TABLE_WIDTH_GUTTER);
    let physical_rows = wrapped_settings_rows(model, content_width);
    let max_visible_data_rows = body.height.saturating_sub(4).max(1);
    let visible_data_rows = physical_rows.len().min(max_visible_data_rows);
    let height = visible_data_rows + 4;
    let top = centered_lobby_panel_top(body, height);
    LobbyPanelLayout {
        left,
        top,
        width,
        height,
        content_left: left + LOBBY_PANEL_INSET_X,
        content_width,
        first_content_row: top + 3,
        first_data_row: top + 3,
        visible_data_rows,
        scrollbar_col: left + width.saturating_sub(2),
    }
}

fn settings_content_width(geometry: ScreenGeometry) -> usize {
    let desired_width = (SETTINGS_FIELD_LABEL_WIDTH + SETTINGS_FIELD_TRACK_WIDTH + 5)
        .max("SETTINGS".chars().count() + 4);
    let width = desired_width.min(geometry.width().saturating_sub(2));
    width.saturating_sub(LOBBY_PANEL_INSET_X * 2 + LOBBY_PANEL_TABLE_WIDTH_GUTTER)
}

fn render_settings_rows(
    buffer: &mut PlayfieldBuffer,
    layout: &LobbyPanelLayout,
    model: &Model,
    relay_draft: &str,
    editing_relay: bool,
) {
    let rows = wrapped_settings_rows(model, layout.content_width);
    let scroll = lobby_settings_scroll(model)
        .min(rows.len().saturating_sub(layout.visible_data_rows.max(1)));
    let track_width = layout
        .content_width
        .saturating_sub(SETTINGS_FIELD_LABEL_WIDTH + 2)
        .min(SETTINGS_FIELD_TRACK_WIDTH);
    for (index, row) in rows
        .iter()
        .enumerate()
        .skip(scroll)
        .take(layout.visible_data_rows)
    {
        let draw_row = layout.first_content_row + index - scroll;
        match row {
            SettingsRow::RelayInput => draw_boxed_input_row(
                buffer,
                layout.content_left,
                draw_row,
                SETTINGS_FIELD_LABEL_WIDTH,
                track_width,
                "Relay URL",
                relay_draft,
                editing_relay,
                false,
            ),
            SettingsRow::Text { content, style } => {
                buffer.write_text_clipped(draw_row, layout.content_left, content, *style);
            }
        }
    }
    draw_lobby_scrollbar(
        buffer,
        layout.first_content_row,
        layout.scrollbar_col,
        layout.visible_data_rows,
        rows.len(),
        scroll,
    );
}

fn settings_rows(model: &Model) -> Vec<SettingsRow> {
    let mut rows = vec![
        SettingsRow::RelayInput,
        SettingsRow::Text {
            content: format!(
                "Window Focus : {}",
                if model.window_focused { "yes" } else { "no" }
            ),
            style: panel(),
        },
        SettingsRow::Text {
            content: format!(
                "Text Input   : {}",
                if model.wants_text_input() {
                    "armed"
                } else {
                    "off"
                }
            ),
            style: panel(),
        },
        SettingsRow::Text {
            content: format!(
                "Idle Lock    : {}",
                if model.lock_timeout_minutes == 0 {
                    String::from("Off")
                } else {
                    format!("{} min", model.lock_timeout_minutes)
                }
            ),
            style: panel(),
        },
    ];
    if let Some(session) = &model.session {
        rows.push(SettingsRow::Text {
            content: format!(
                "Handle       : {}",
                session.active_handle.as_deref().unwrap_or("unset")
            ),
            style: panel(),
        });
        rows.push(SettingsRow::Text {
            content: format!("Identity     : {}", session.active_npub),
            style: panel(),
        });
    }
    rows.push(SettingsRow::Text {
        content: String::from("R : Edit relay URL   Enter : Save relay   Esc : Cancel edit"),
        style: if lobby_is_editing_relay(model) {
            panel_accent()
        } else {
            panel_dim()
        },
    });
    rows.push(SettingsRow::Text {
        content: String::from("L : Lock the local session and stop background sync"),
        style: panel_accent(),
    });
    rows.push(SettingsRow::Text {
        content: String::from("I : Cycle idle lock timeout"),
        style: panel_accent(),
    });
    rows.push(SettingsRow::Text {
        content: String::from("Alt+Q : Quit nc-helm"),
        style: panel_dim(),
    });
    rows
}

fn wrapped_settings_rows(model: &Model, max_width: usize) -> Vec<SettingsRow> {
    let mut wrapped = Vec::new();
    for row in settings_rows(model) {
        match row {
            SettingsRow::RelayInput => wrapped.push(SettingsRow::RelayInput),
            SettingsRow::Text { content, style } => {
                for line in wrap_lobby_text(&content, max_width.max(1)) {
                    wrapped.push(SettingsRow::Text {
                        content: line,
                        style,
                    });
                }
            }
        }
    }
    wrapped
}

fn wrap_lobby_text(text: &str, width: usize) -> Vec<String> {
    if text.is_empty() {
        return vec![String::new()];
    }
    if width == 0 {
        return vec![String::new()];
    }
    if text.chars().count() <= width {
        return vec![text.to_string()];
    }

    let mut lines = Vec::new();
    let mut current = String::new();
    for word in text.split_whitespace() {
        let word_width = word.chars().count();
        if current.is_empty() {
            if word_width <= width {
                current.push_str(word);
            } else {
                let mut chunks = chunk_lobby_word(word, width);
                current = chunks.pop().unwrap_or_default();
                lines.extend(chunks);
            }
            continue;
        }
        if current.chars().count() + 1 + word_width <= width {
            current.push(' ');
            current.push_str(word);
            continue;
        }
        lines.push(current);
        current = String::new();
        if word_width <= width {
            current.push_str(word);
        } else {
            let mut chunks = chunk_lobby_word(word, width);
            current = chunks.pop().unwrap_or_default();
            lines.extend(chunks);
        }
    }
    if !current.is_empty() {
        lines.push(current);
    }
    lines
}

fn chunk_lobby_word(word: &str, width: usize) -> Vec<String> {
    let mut chunks = Vec::new();
    let mut current = String::new();
    for ch in word.chars() {
        if current.chars().count() == width {
            chunks.push(current);
            current = String::new();
        }
        current.push(ch);
    }
    if !current.is_empty() {
        chunks.push(current);
    }
    chunks
}

fn resolve_lobby_table_columns<'a>(
    columns: &'a [DashboardTableColumn<'a>],
    available_width: usize,
) -> Vec<DashboardTableColumn<'a>> {
    let mut resolved = columns.to_vec();
    let minimum_flex_width = 8usize;
    let mut current_width = table_render_width(&resolved);
    while current_width > available_width {
        let mut shrank = false;
        for column in &mut resolved {
            if column.flex == 0 || column.width <= minimum_flex_width {
                continue;
            }
            column.width -= 1;
            current_width = current_width.saturating_sub(1);
            shrank = true;
            if current_width <= available_width {
                break;
            }
        }
        if !shrank {
            break;
        }
    }
    resolved
}

fn render_table_header(
    buffer: &mut PlayfieldBuffer,
    left: usize,
    top: usize,
    columns: &[DashboardTableColumn<'_>],
) {
    let mut col = left;
    for column in columns {
        let cell = format_table_cell(column.header, column.width, column.align);
        buffer.write_text_clipped(top, col, &cell, panel_dim());
        col += column.width + 1;
    }
}

fn render_table_row(
    buffer: &mut PlayfieldBuffer,
    left: usize,
    top: usize,
    columns: &[DashboardTableColumn<'_>],
    values: &[String],
    style: CellStyle,
) {
    let mut col = left;
    for (index, column) in columns.iter().enumerate() {
        let value = values.get(index).map(String::as_str).unwrap_or("");
        let cell = format_table_cell(value, column.width, column.align);
        buffer.write_text_clipped(top, col, &cell, style);
        col += column.width + 1;
    }
}

fn format_table_cell(value: &str, width: usize, align: TableAlign) -> String {
    let clipped = truncate(value, width);
    match align {
        TableAlign::Left => format!("{clipped:<width$}"),
        TableAlign::Center => {
            let padding = width.saturating_sub(clipped.chars().count());
            let left = padding / 2;
            let right = padding.saturating_sub(left);
            format!("{}{}{}", " ".repeat(left), clipped, " ".repeat(right))
        }
        TableAlign::Right => format!("{clipped:>width$}"),
    }
}

fn draw_lobby_scrollbar(
    buffer: &mut PlayfieldBuffer,
    start_row: usize,
    col: usize,
    visible_rows: usize,
    total_rows: usize,
    scroll_offset: usize,
) {
    if total_rows <= visible_rows || visible_rows < 3 || col >= buffer.width() {
        return;
    }

    let displayed_rows = usize::min(visible_rows, total_rows.saturating_sub(scroll_offset));
    if displayed_rows < 3 {
        return;
    }

    let last_row = start_row + displayed_rows - 1;
    let track_top = start_row + 1;
    let track_bottom = last_row.saturating_sub(1);

    buffer.write_text(start_row, col, "^", panel_dim());
    buffer.write_text(last_row, col, "v", panel_dim());
    for row in track_top..=track_bottom {
        buffer.write_text(row, col, "|", panel_dim());
    }

    let max_offset = total_rows.saturating_sub(visible_rows);
    let thumb_span = track_bottom.saturating_sub(track_top);
    let thumb_row = if max_offset == 0 || thumb_span == 0 {
        track_top
    } else {
        track_top + (scroll_offset * thumb_span) / max_offset
    };
    buffer.write_text(thumb_row, col, "#", panel_accent());
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
        StyledSpan::new("R", theme::prompt_hotkey_style_on(bg)),
        StyledSpan::new(">", theme::prompt_delimiter_style_on(bg)),
        StyledSpan::new("efresh ", theme::prompt_style_on(bg)),
        StyledSpan::new("D", theme::prompt_hotkey_style_on(bg)),
        StyledSpan::new(">", theme::prompt_delimiter_style_on(bg)),
        StyledSpan::new("elete ", theme::prompt_style_on(bg)),
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
    draw_modal_panel(
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
        "Up/Down : move the active table cursor",
        "Alt+R   : refresh My Games, Open Games, and notices",
        "Alt+D   : delete the selected sandbox from My Games",
        "? or H  : reopen this help popup",
        "Q or Esc: close this popup",
        "Alt+Q   : quit the client",
        "",
        "Background sync still runs every few seconds.",
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

fn draw_lobby_quit_confirm_popup(buffer: &mut PlayfieldBuffer, geometry: ScreenGeometry) {
    let message = "Quit NC? Y/[N]";
    let width = crate::dashboard::quit_confirm_popup_width(message);
    centered_box(
        buffer,
        geometry,
        width,
        crate::dashboard::QUIT_CONFIRM_HEIGHT,
        crate::dashboard::QUIT_CONFIRM_TITLE,
        |buffer, left, top| {
            let content_left = left + 1;
            let content_width = width.saturating_sub(2);
            let start_col =
                content_left + content_width.saturating_sub(message.chars().count()) / 2;
            buffer.write_text_clipped(top + 2, start_col, message, panel_accent());
        },
    );
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
    draw_modal_panel(
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
        if let Some((nostrian, conquest)) = gate_logo_kinds(logo_width) {
            buffer.push_overlay_logo(nostrian, panel_brand().fg, logo_left, top);
            buffer.push_overlay_logo(conquest, panel_brand().fg, logo_left, top + 4);
            return;
        }
    }

    let line_one = "NOSTRIAN";
    let line_two = "CONQUEST";
    let line_one_col = left + width.saturating_sub(line_one.chars().count()) / 2;
    let line_two_col = left + width.saturating_sub(line_two.chars().count()) / 2;
    buffer.write_text_clipped(top, line_one_col, line_one, panel_brand());
    buffer.write_text_clipped(top + 1, line_two_col, line_two, panel_brand());
}

fn gate_logo_kinds(width_cols: usize) -> Option<(OverlayLogoKind, OverlayLogoKind)> {
    match width_cols {
        54 => Some((
            OverlayLogoKind::GateNostrian54x4,
            OverlayLogoKind::GateConquest54x4,
        )),
        62 => Some((
            OverlayLogoKind::GateNostrian62x4,
            OverlayLogoKind::GateConquest62x4,
        )),
        66 => Some((
            OverlayLogoKind::GateNostrian66x4,
            OverlayLogoKind::GateConquest66x4,
        )),
        _ => None,
    }
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
    draw_modal_panel(
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

fn lobby_selected_my_game(model: &Model) -> usize {
    match &model.route {
        Route::Lobby(lobby) => lobby.selected_my_game,
        _ => 0,
    }
}

fn lobby_selected_open_game(model: &Model) -> usize {
    match &model.route {
        Route::Lobby(lobby) => lobby.selected_open_game,
        _ => 0,
    }
}

fn lobby_my_games_scroll(model: &Model) -> usize {
    match &model.route {
        Route::Lobby(lobby) => lobby.my_games_scroll,
        _ => 0,
    }
}

fn lobby_open_games_scroll(model: &Model) -> usize {
    match &model.route {
        Route::Lobby(lobby) => lobby.open_games_scroll,
        _ => 0,
    }
}

fn lobby_settings_scroll(model: &Model) -> usize {
    match &model.route {
        Route::Lobby(lobby) => lobby.settings_scroll,
        _ => 0,
    }
}

fn lobby_is_editing_relay(model: &Model) -> bool {
    match &model.route {
        Route::Lobby(lobby) => lobby.editing_relay,
        _ => false,
    }
}

fn split_turn_summary(summary: &str) -> (String, String) {
    let mut parts = summary.split_whitespace();
    let year = parts
        .next()
        .map(|part| part.trim_start_matches(['Y', 'y']).to_string())
        .filter(|part| !part.is_empty())
        .unwrap_or_else(|| summary.to_string());
    let turn = parts
        .next()
        .map(|part| part.trim_start_matches(['T', 't']).to_string())
        .unwrap_or_else(|| "0".to_string());
    (year, turn)
}

fn formatted_turn_summary(summary: &str) -> String {
    let (year, turn) = split_turn_summary(summary);
    format!("Y{year}:T{turn}")
}

fn joined_game_status_label(status: &str) -> &str {
    match status {
        "requested" | "approved" => "Requested",
        "rejected" => "Rejected",
        "joined" => "Joined",
        "expired" => "Expired",
        "final" => "Final",
        other => other,
    }
}

fn map_size_summary(total_seats: u8) -> String {
    let edge = match total_seats {
        0..=4 => 18,
        5..=9 => 27,
        10..=16 => 36,
        _ => 45,
    };
    format!("{edge}x{edge}")
}

#[cfg(test)]
mod tests {
    use super::{GATE_LOGO_BLOCK_ROWS, body, draw_gate_wordmark};
    use crate::PlayfieldBuffer;
    use crate::grid::OverlayLogoKind;

    #[test]
    fn stormfaze_gate_logos_use_a_full_blank_row_between_words() {
        let mut buffer = PlayfieldBuffer::new(80, 20, body());

        draw_gate_wordmark(&mut buffer, 10, 5, 60, true);

        let overlays = buffer.overlay_logos();
        assert_eq!(overlays.len(), 2);
        assert_eq!(overlays[0].kind, OverlayLogoKind::GateNostrian54x4);
        assert_eq!(overlays[1].kind, OverlayLogoKind::GateConquest54x4);
        assert_eq!(overlays[0].top_row, 5);
        assert_eq!(overlays[1].top_row, 9);
        assert_eq!(overlays[1].top_row - overlays[0].top_row, 4);
        assert_eq!(GATE_LOGO_BLOCK_ROWS, 8);
    }
}
