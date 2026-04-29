mod chrome;
mod update;
mod view;

use std::path::Path;

use nc_client::cache::ClientCache;
use nc_nostr::first_join::FIRST_JOIN_NAME_MAX_CHARS;
use nc_nostr::state_sync::GameState;
pub(crate) use view::ViewRenderTimings;

use crate::dashboard;
use crate::input::{KeyCode, KeyEvent, MouseEvent};
use crate::storage::{BootSnapshot, StoredSession};
use crate::theme;
use crate::transport::{
    HostedGameOpenResult, HostedGameOpenSuccess, LobbySnapshot, SandboxJoinResult,
    SandboxReleaseSuccess,
};
use crate::{CellStyle, GameColor, PlayfieldBuffer, Point, ScreenGeometry};

pub const DEFAULT_RELAY_URL: &str = "ws://localhost:8080";
pub const DEFAULT_GEOMETRY: ScreenGeometry = ScreenGeometry::new(100, 36);
pub(crate) const MIN_SUPPORTED_GEOMETRY: ScreenGeometry = ScreenGeometry::new(68, 24);
pub const DEFAULT_LOCK_TIMEOUT_MINUTES: u16 = 10;
pub const LOCK_TIMEOUT_OPTIONS: [u16; 5] = [0, 5, 10, 15, 30];
pub const HELP_POPUP_WIDTH: usize = 60;
pub const HELP_POPUP_HEIGHT: usize = 14;
pub const HELP_CLOSE_LABEL: &str = "[X]";
pub(crate) const LOBBY_TAB_ROW: usize = 2;
const LOBBY_TAB_GAP: usize = 1;

#[derive(Debug, Clone)]
pub struct App {
    model: Model,
    view_cache: view::ViewCache,
    last_view_cache_hit: bool,
    last_view_render_timings: view::ViewRenderTimings,
}

#[derive(Debug, Default)]
pub(crate) struct DispatchOutcome {
    pub effects: Vec<Effect>,
    pub needs_redraw: bool,
}

impl DispatchOutcome {
    pub(crate) fn new(effects: Vec<Effect>, needs_redraw: bool) -> Self {
        Self {
            effects,
            needs_redraw,
        }
    }

    pub(crate) fn redraw(effects: Vec<Effect>) -> Self {
        Self::new(effects, true)
    }

    pub(crate) fn no_redraw(effects: Vec<Effect>) -> Self {
        Self::new(effects, false)
    }
}

impl App {
    pub fn new(relay_override: Option<String>) -> (Self, Vec<Effect>) {
        let relay_overridden = relay_override.is_some();
        let model = Model {
            geometry: DEFAULT_GEOMETRY,
            relay_url: relay_override.unwrap_or_else(|| DEFAULT_RELAY_URL.to_string()),
            relay_overridden,
            route: Route::Boot(BootModel {
                status: "Loading local client state...".to_string(),
            }),
            lock_resume_route: None,
            session: None,
            network: NetworkState::Idle,
            cache: ClientCache::empty(),
            my_games: Vec::new(),
            open_games: Vec::new(),
            notices: Vec::new(),
            lock_timeout_minutes: DEFAULT_LOCK_TIMEOUT_MINUTES,
            matrix_rain: MatrixRain::new(DEFAULT_GEOMETRY.width(), DEFAULT_GEOMETRY.height()),
            window_focused: false,
            should_quit: false,
        };
        (
            Self {
                model,
                view_cache: view::ViewCache::default(),
                last_view_cache_hit: false,
                last_view_render_timings: view::ViewRenderTimings::default(),
            },
            vec![Effect::LoadBoot],
        )
    }

    pub fn new_local_dashboard(
        game_dir: impl AsRef<Path>,
    ) -> Result<(Self, Vec<Effect>), Box<dyn std::error::Error>> {
        let game_dir = game_dir.as_ref().to_path_buf();
        let dashboard = dashboard::DashLaunchState::from_local_dir(game_dir.clone())?.into_app(
            dashboard::ScreenGeometry::new(DEFAULT_GEOMETRY.width(), DEFAULT_GEOMETRY.height()),
        )?;
        let model = Model {
            geometry: DEFAULT_GEOMETRY,
            relay_url: DEFAULT_RELAY_URL.to_string(),
            relay_overridden: false,
            route: Route::HostedGame(HostedGameModel {
                row: local_dashboard_row(&game_dir),
                dashboard,
                status: None,
            }),
            lock_resume_route: None,
            session: None,
            network: NetworkState::Idle,
            cache: ClientCache::empty(),
            my_games: Vec::new(),
            open_games: Vec::new(),
            notices: Vec::new(),
            lock_timeout_minutes: DEFAULT_LOCK_TIMEOUT_MINUTES,
            matrix_rain: MatrixRain::new(DEFAULT_GEOMETRY.width(), DEFAULT_GEOMETRY.height()),
            window_focused: false,
            should_quit: false,
        };
        Ok((
            Self {
                model,
                view_cache: view::ViewCache::default(),
                last_view_cache_hit: false,
                last_view_render_timings: view::ViewRenderTimings::default(),
            },
            Vec::new(),
        ))
    }

    pub fn model(&self) -> &Model {
        &self.model
    }

    pub fn dispatch(&mut self, msg: Msg) -> Vec<Effect> {
        self.dispatch_with_outcome(msg).effects
    }

    pub(crate) fn dispatch_with_outcome(&mut self, msg: Msg) -> DispatchOutcome {
        update::update(&mut self.model, msg)
    }

    pub fn view(&mut self) -> &PlayfieldBuffer {
        self.view_with_cache_hit().1
    }

    pub fn view_with_cache_hit(&mut self) -> (bool, &PlayfieldBuffer) {
        let (hit, _timings, buffer) = self.view_with_cache_hit_and_timings();
        (hit, buffer)
    }

    pub(crate) fn view_with_cache_hit_and_timings(
        &mut self,
    ) -> (bool, ViewRenderTimings, &PlayfieldBuffer) {
        let (hit, timings, buffer) = self.view_cache.render(&self.model);
        self.last_view_cache_hit = hit;
        self.last_view_render_timings = timings;
        (hit, timings, buffer)
    }

    pub fn last_view_cache_hit(&self) -> bool {
        self.last_view_cache_hit
    }

    pub fn hosted_on_idle(&mut self) -> bool {
        match &mut self.model.route {
            Route::HostedGame(hosted) => dashboard::hosted_on_idle(&mut hosted.dashboard),
            _ => false,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Model {
    pub geometry: ScreenGeometry,
    pub relay_url: String,
    pub relay_overridden: bool,
    pub route: Route,
    pub lock_resume_route: Option<Route>,
    pub session: Option<SessionState>,
    pub network: NetworkState,
    pub cache: ClientCache,
    pub my_games: Vec<MyGameRow>,
    pub open_games: Vec<OpenGameRow>,
    pub notices: Vec<String>,
    pub lock_timeout_minutes: u16,
    pub matrix_rain: MatrixRain,
    pub window_focused: bool,
    pub should_quit: bool,
}

impl Model {
    pub fn wants_text_input(&self) -> bool {
        match &self.route {
            Route::FirstRun(_) | Route::Locked(_) | Route::FirstJoinSetup(_) => true,
            Route::HostedGame(hosted) => dashboard::hosted_wants_text_input(&hosted.dashboard),
            _ => false,
        }
    }

    pub fn wants_window_focus(&self) -> bool {
        match &self.route {
            Route::HostedGame(hosted) => dashboard::hosted_wants_window_focus(&hosted.dashboard),
            _ => true,
        }
    }
}

#[derive(Debug, Clone)]
pub enum Route {
    Boot(BootModel),
    FirstRun(FirstRunModel),
    MatrixLocked,
    Locked(LockedModel),
    Lobby(LobbyModel),
    SandboxJoinConfirm(OpenGameRow),
    SandboxJoinUnavailable { row: OpenGameRow, notice: String },
    SandboxDeleteConfirm(MyGameRow),
    FirstJoinSetup(FirstJoinSetupModel),
    HostedGame(HostedGameModel),
    FatalError(String),
}

#[derive(Debug, Clone)]
pub struct BootModel {
    pub status: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FirstRunField {
    Handle,
    Password,
    Confirm,
    Relay,
}

impl FirstRunField {
    fn next(self) -> Self {
        match self {
            Self::Handle => Self::Password,
            Self::Password => Self::Confirm,
            Self::Confirm => Self::Relay,
            Self::Relay => Self::Handle,
        }
    }

    fn previous(self) -> Self {
        match self {
            Self::Handle => Self::Relay,
            Self::Password => Self::Handle,
            Self::Confirm => Self::Password,
            Self::Relay => Self::Confirm,
        }
    }
}

#[derive(Debug, Clone)]
pub struct FirstRunModel {
    pub active_field: FirstRunField,
    pub handle_input: String,
    pub password_input: String,
    pub confirm_input: String,
    pub relay_input: String,
    pub status: Option<String>,
}

#[derive(Debug, Clone)]
pub struct LockedModel {
    pub password_input: String,
    pub status: Option<String>,
    pub resume_session: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FirstJoinSetupField {
    Empire,
    Homeworld,
}

impl FirstJoinSetupField {
    fn next(self) -> Self {
        match self {
            Self::Empire => Self::Homeworld,
            Self::Homeworld => Self::Empire,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LobbyTab {
    MyGames,
    OpenGames,
    Comms,
    Settings,
}

impl LobbyTab {
    fn next(self) -> Self {
        match self {
            Self::MyGames => Self::OpenGames,
            Self::OpenGames => Self::Comms,
            Self::Comms => Self::Settings,
            Self::Settings => Self::MyGames,
        }
    }

    pub(crate) fn label(self) -> &'static str {
        match self {
            Self::MyGames => "My Games",
            Self::OpenGames => "Open Games",
            Self::Comms => "Comms",
            Self::Settings => "Settings",
        }
    }
}

#[derive(Debug, Clone)]
pub struct LobbyModel {
    pub active_tab: LobbyTab,
    pub help_open: bool,
    pub quit_confirm_open: bool,
    pub selected_my_game: usize,
    pub my_games_scroll: usize,
    pub selected_open_game: usize,
    pub open_games_scroll: usize,
    pub settings_scroll: usize,
    pub editing_relay: bool,
    pub relay_draft: String,
    pub status: Option<String>,
}

#[derive(Debug, Clone)]
pub struct FirstJoinSetupModel {
    pub row: MyGameRow,
    pub empire_input: String,
    pub homeworld_input: String,
    pub active_field: FirstJoinSetupField,
    pub status: Option<String>,
    pub homeworld_coords: [u8; 2],
    pub present_production: u16,
    pub potential_production: u16,
}

pub struct HostedGameModel {
    pub row: MyGameRow,
    pub dashboard: dashboard::DashApp,
    pub status: Option<String>,
}

impl Clone for HostedGameModel {
    fn clone(&self) -> Self {
        Self {
            row: self.row.clone(),
            dashboard: self.dashboard.clone(),
            status: self.status.clone(),
        }
    }
}

impl std::fmt::Debug for HostedGameModel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("HostedGameModel")
            .field("row", &self.row)
            .field("status", &self.status)
            .finish()
    }
}

#[derive(Debug, Clone)]
pub struct SessionState {
    pub password: String,
    pub active_npub: String,
    pub active_nsec: String,
    pub active_handle: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum NetworkState {
    Idle,
    Connecting,
    Synced,
    Error,
}

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct MyGameRow {
    pub game_id: String,
    pub status: String,
    pub game_tier: String,
    pub game: String,
    pub host: String,
    pub host_contact_npub: Option<String>,
    pub relay_url: String,
    pub daemon_pubkey: String,
    pub seat: Option<u8>,
    pub turn_summary: String,
    pub last_turn: Option<u32>,
    pub last_hash: Option<String>,
}

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct OpenGameRow {
    pub game_id: String,
    pub status: String,
    pub game_tier: String,
    pub game: String,
    pub host: String,
    pub host_contact_npub: Option<String>,
    pub relay_url: String,
    pub daemon_pubkey: String,
    pub open_seats: u8,
    pub total_seats: u8,
    pub created_date: String,
    pub turn_summary: String,
    pub summary: String,
}

#[derive(Debug, Clone)]
pub enum Msg {
    Resize(ScreenGeometry),
    FocusChanged(bool),
    MatrixFrame,
    IdleLock,
    Key(KeyEvent),
    TextInput(String),
    Mouse(MouseEvent),
    BootLoaded(Result<BootSnapshot, String>),
    IdentityCreated(Result<StoredSession, String>),
    Unlocked(Result<StoredSession, String>),
    LobbyUpdated(Result<LobbySnapshot, String>),
    LobbyRefreshed(Result<LobbySnapshot, String>),
    SandboxJoined(Result<SandboxJoinResult, String>),
    SandboxReleased(Result<SandboxReleaseSuccess, String>),
    HostedGameOpened(Result<HostedGameOpenResult, String>),
    FirstJoinSetupCompleted(Result<HostedGameOpenSuccess, String>),
    RelaySaved(Result<String, String>),
}

#[derive(Debug, Clone)]
pub enum Effect {
    LoadBoot,
    CreateIdentity {
        handle: String,
        password: String,
        relay_url: String,
    },
    Unlock {
        password: String,
    },
    ConnectTransport {
        relay_url: String,
        nsec: String,
        cache: ClientCache,
    },
    SaveRelayUrl {
        relay_url: String,
    },
    SaveClientCache {
        cache: ClientCache,
        password: String,
    },
    SaveLockTimeout {
        lock_timeout_minutes: u16,
    },
    RefreshLobby,
    JoinSandboxGame {
        row: OpenGameRow,
        password: String,
        handle: Option<String>,
    },
    ReleaseSandboxGame {
        row: MyGameRow,
    },
    OpenHostedGame {
        row: MyGameRow,
        password: String,
        handle: Option<String>,
    },
    CompleteFirstJoinSetup {
        row: MyGameRow,
        empire_name: String,
        homeworld_name: String,
        password: String,
    },
    DisconnectTransport,
    Quit,
}

fn bootstrap_route(snapshot: &BootSnapshot, relay_url: String) -> Route {
    if snapshot.has_keychain {
        Route::Locked(LockedModel {
            password_input: String::new(),
            status: None,
            resume_session: false,
        })
    } else {
        Route::FirstRun(FirstRunModel {
            active_field: FirstRunField::Handle,
            handle_input: String::new(),
            password_input: String::new(),
            confirm_input: String::new(),
            relay_input: relay_url,
            status: None,
        })
    }
}

fn lobby_route(status: Option<String>, relay_url: String) -> Route {
    Route::Lobby(LobbyModel {
        active_tab: LobbyTab::MyGames,
        help_open: false,
        quit_confirm_open: false,
        selected_my_game: 0,
        my_games_scroll: 0,
        selected_open_game: 0,
        open_games_scroll: 0,
        settings_scroll: 0,
        editing_relay: false,
        relay_draft: relay_url,
        status,
    })
}

pub(crate) fn route_supports_session_lock(route: &Route) -> bool {
    matches!(
        route,
        Route::Lobby(_)
            | Route::SandboxJoinConfirm(_)
            | Route::SandboxJoinUnavailable { .. }
            | Route::SandboxDeleteConfirm(_)
            | Route::FirstJoinSetup(_)
            | Route::HostedGame(_)
    )
}

fn active_session_from_stored(stored: StoredSession, password: String) -> SessionState {
    SessionState {
        password,
        active_npub: stored.active_npub,
        active_nsec: stored.active_nsec,
        active_handle: stored.active_handle,
    }
}

fn local_dashboard_row(game_dir: &Path) -> MyGameRow {
    let game = game_dir
        .file_name()
        .and_then(|name| name.to_str())
        .filter(|name| !name.is_empty())
        .unwrap_or("Local Dashboard");
    MyGameRow {
        game_id: game_dir.display().to_string(),
        status: "Local".to_string(),
        game_tier: "Local".to_string(),
        game: game.to_string(),
        host: "local".to_string(),
        host_contact_npub: None,
        relay_url: String::new(),
        daemon_pubkey: String::new(),
        seat: Some(1),
        turn_summary: "Local".to_string(),
        last_turn: None,
        last_hash: None,
    }
}

pub(crate) fn centered_box_geometry(
    geometry: ScreenGeometry,
    width: usize,
    height: usize,
) -> (usize, usize, usize, usize) {
    let width = width.min(geometry.width());
    let height = height.min(geometry.height());
    let left = geometry.width().saturating_sub(width) / 2;
    let top = geometry.height().saturating_sub(height) / 2;
    (left, top, width, height)
}

pub(crate) fn help_popup_geometry(geometry: ScreenGeometry) -> (usize, usize, usize, usize) {
    centered_box_geometry(geometry, HELP_POPUP_WIDTH, HELP_POPUP_HEIGHT)
}

pub(crate) fn help_close_tag_bounds(geometry: ScreenGeometry) -> Option<(usize, usize, usize)> {
    let (left, top, width, _) = help_popup_geometry(geometry);
    let col = chrome::top_tag_right_col(left, width, HELP_CLOSE_LABEL)?;
    Some((top, col, chrome::top_tag_width(HELP_CLOSE_LABEL)))
}

pub(crate) fn lobby_tab_bounds(geometry: ScreenGeometry) -> [(LobbyTab, usize, usize); 4] {
    let tabs = [
        LobbyTab::MyGames,
        LobbyTab::OpenGames,
        LobbyTab::Comms,
        LobbyTab::Settings,
    ];
    let total_width = tabs
        .iter()
        .map(|tab| tab.label().chars().count() + 2)
        .sum::<usize>()
        + tabs.len().saturating_sub(1) * LOBBY_TAB_GAP;
    let mut col = if total_width >= geometry.width() {
        0
    } else {
        (geometry.width() - total_width) / 2
    };
    let mut bounds = [(LobbyTab::MyGames, 0, 0); 4];
    for (index, tab) in tabs.iter().copied().enumerate() {
        let width = tab.label().chars().count() + 2;
        bounds[index] = (tab, col, col + width);
        col += width + LOBBY_TAB_GAP;
    }
    bounds
}

impl LobbyModel {
    pub fn selected_len(&self, model: &Model) -> usize {
        match self.active_tab {
            LobbyTab::MyGames => model.my_games.len(),
            LobbyTab::OpenGames => model.open_games.len(),
            _ => 0,
        }
    }

    pub fn selected_index(&self) -> usize {
        match self.active_tab {
            LobbyTab::MyGames => self.selected_my_game,
            LobbyTab::OpenGames => self.selected_open_game,
            _ => 0,
        }
    }

    pub fn set_selected_index(&mut self, index: usize) {
        match self.active_tab {
            LobbyTab::MyGames => self.selected_my_game = index,
            LobbyTab::OpenGames => self.selected_open_game = index,
            _ => {}
        }
    }

    pub fn selected_scroll(&self) -> usize {
        match self.active_tab {
            LobbyTab::MyGames => self.my_games_scroll,
            LobbyTab::OpenGames => self.open_games_scroll,
            LobbyTab::Settings => self.settings_scroll,
            LobbyTab::Comms => 0,
        }
    }

    pub fn set_selected_scroll(&mut self, scroll: usize) {
        match self.active_tab {
            LobbyTab::MyGames => self.my_games_scroll = scroll,
            LobbyTab::OpenGames => self.open_games_scroll = scroll,
            LobbyTab::Settings => self.settings_scroll = scroll,
            LobbyTab::Comms => {}
        }
    }
}

pub fn normalize_lock_timeout_minutes(value: u16) -> u16 {
    if LOCK_TIMEOUT_OPTIONS.contains(&value) {
        value
    } else {
        DEFAULT_LOCK_TIMEOUT_MINUTES
    }
}

pub(crate) fn first_join_setup_from_snapshot(
    row: MyGameRow,
    snapshot: &GameState,
) -> Option<FirstJoinSetupModel> {
    let empire_pending = snapshot.state.player.empire_name == "In Civil Disorder";
    let homeworld = snapshot
        .state
        .owned_planets
        .iter()
        .find(|planet| {
            planet.planet_index == usize::from(snapshot.state.player.homeworld_planet_index)
        })
        .or_else(|| snapshot.state.owned_planets.first())?;
    let homeworld_pending = homeworld.name == "Not Named Yet";
    if !empire_pending && !homeworld_pending {
        return None;
    }
    Some(FirstJoinSetupModel {
        row,
        empire_input: if empire_pending {
            String::new()
        } else {
            snapshot.state.player.empire_name.clone()
        },
        homeworld_input: if homeworld_pending {
            String::new()
        } else {
            homeworld.name.clone()
        },
        active_field: if empire_pending {
            FirstJoinSetupField::Empire
        } else {
            FirstJoinSetupField::Homeworld
        },
        status: None,
        homeworld_coords: homeworld.coords,
        present_production: u16::from(homeworld.current_production),
        potential_production: homeworld.potential_production,
    })
}

pub(crate) fn trim_first_join_setup_input(value: &mut String) {
    if value.chars().count() > FIRST_JOIN_NAME_MAX_CHARS {
        *value = value.chars().take(FIRST_JOIN_NAME_MAX_CHARS).collect();
    }
}

#[derive(Debug, Clone)]
pub struct MatrixRain {
    width: usize,
    height: usize,
    tick: u64,
    rng: u64,
    columns: Vec<MatrixColumn>,
}

#[derive(Debug, Clone)]
struct MatrixColumn {
    gap_remaining: usize,
    length: usize,
    update_every: usize,
    phase: usize,
    head_row: isize,
    tail_row: isize,
    glyphs: Vec<char>,
}

const MATRIX_MIN_STREAM_LENGTH: usize = 3;
pub(crate) const MATRIX_FRAME_STEP: std::time::Duration = std::time::Duration::from_millis(80);
pub(crate) const MATRIX_GLYPHS: &[char] = &[
    'Α', 'Β', 'Γ', 'Δ', 'Ε', 'Ζ', 'Η', 'Θ', 'Ι', 'Κ', 'Λ', 'Μ', 'Ν', 'Ξ', 'Ο', 'Π', 'Ρ', 'Σ', 'Τ',
    'Υ', 'Φ', 'Χ', 'Ψ', 'Ω', '+', '#', '%', '*',
];

impl MatrixRain {
    pub fn new(width: usize, height: usize) -> Self {
        let mut rain = Self {
            width,
            height,
            tick: 0,
            rng: seed_for_size(width, height),
            columns: Vec::new(),
        };
        rain.reset_for_size(width, height);
        rain
    }

    pub fn reset_for_size(&mut self, width: usize, height: usize) {
        self.width = width;
        self.height = height;
        self.tick = 0;
        self.rng = seed_for_size(width, height);
        self.columns = (0..width)
            .map(|column| self.make_column(column))
            .collect::<Vec<_>>();
        let warmup_steps = (height / 3).max(1);
        for _ in 0..warmup_steps {
            self.advance();
        }
    }

    pub fn reset(&mut self) {
        self.reset_for_size(self.width, self.height);
    }

    pub fn advance(&mut self) {
        self.tick = self.tick.saturating_add(1);
        for column_index in 0..self.columns.len() {
            if column_index % 2 == 1 {
                continue;
            }
            let update_every = self.columns[column_index].update_every;
            let phase = self.columns[column_index].phase;
            if ((self.tick as usize) + phase) % update_every != 0 {
                continue;
            }
            self.advance_column(column_index);
        }
    }

    pub fn render(&self, buffer: &mut PlayfieldBuffer) {
        let background = theme::body_style().bg;
        let trail_style = CellStyle::new(GameColor::Green, background, false);
        let head_style = CellStyle::new(GameColor::BrightGreen, background, true);

        for (x, column) in self.columns.iter().enumerate() {
            if x >= buffer.width() || column.head_row < 0 {
                continue;
            }
            let visible_top = column.tail_row.max(0) as usize;
            let visible_bottom = column
                .head_row
                .min((self.height.saturating_sub(1)) as isize);
            for y in visible_top..=visible_bottom as usize {
                if y >= buffer.height() {
                    break;
                }
                let style = if y as isize == visible_bottom {
                    head_style
                } else {
                    trail_style
                };
                buffer.set_cell(y, x, column.glyphs[y], style);
            }
        }
    }

    fn advance_column(&mut self, column_index: usize) {
        if self.height == 0 {
            return;
        }
        let height = self.height as isize;
        if self.columns[column_index].gap_remaining > 0 {
            self.columns[column_index].gap_remaining -= 1;
            return;
        }
        if self.columns[column_index].head_row < 0 {
            let glyph = self.random_glyph();
            let column = &mut self.columns[column_index];
            column.head_row = 0;
            column.tail_row = 0;
            column.glyphs[0] = glyph;
            return;
        }

        {
            let column = &mut self.columns[column_index];
            column.head_row += 1;
        }
        let head_row = self.columns[column_index].head_row;
        if head_row < height {
            let glyph = self.random_glyph();
            self.columns[column_index].glyphs[head_row as usize] = glyph;
        }

        {
            let column = &mut self.columns[column_index];
            if column.head_row - column.tail_row + 1 > column.length as isize {
                column.tail_row += 1;
            }
        }

        let head = self.columns[column_index].head_row.min(height - 1);
        let tail = self.columns[column_index].tail_row.max(0);
        for row in tail..head {
            if self.next_random(8) == 0 {
                let glyph = self.random_glyph();
                self.columns[column_index].glyphs[row as usize] = glyph;
            }
        }

        if self.columns[column_index].tail_row >= height {
            let next = self.make_column(column_index);
            self.columns[column_index] = next;
        }
    }

    fn make_column(&mut self, column_index: usize) -> MatrixColumn {
        let height = self.height.max(1);
        let length_max = height.saturating_sub(3).max(MATRIX_MIN_STREAM_LENGTH);
        let length =
            MATRIX_MIN_STREAM_LENGTH + self.next_random(length_max - MATRIX_MIN_STREAM_LENGTH + 1);
        MatrixColumn {
            gap_remaining: 1 + self.next_random(height),
            length,
            update_every: 1 + self.next_random(3),
            phase: (column_index * 3 + self.next_random(7)) % 3,
            head_row: -1,
            tail_row: 0,
            glyphs: vec![' '; height],
        }
    }

    fn random_glyph(&mut self) -> char {
        MATRIX_GLYPHS[self.next_random(MATRIX_GLYPHS.len())]
    }

    fn next_random(&mut self, limit: usize) -> usize {
        if limit <= 1 {
            return 0;
        }
        self.rng = self
            .rng
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        ((self.rng >> 32) as usize) % limit
    }
}

fn seed_for_size(width: usize, height: usize) -> u64 {
    ((width as u64) << 32) ^ (height as u64) ^ 0x9E37_79B9_7F4A_7C15
}

fn handle_help_click(model: &mut Model, position: Point) -> bool {
    let Route::Lobby(lobby) = &mut model.route else {
        return false;
    };
    if !lobby.help_open {
        return false;
    }
    let (left, top, width, height) = help_popup_geometry(model.geometry);
    let row = position.row.as_usize();
    let column = position.column.as_usize();
    if row >= top && row < top + height && column >= left && column < left + width {
        if let Some((close_row, close_col, close_width)) = help_close_tag_bounds(model.geometry) {
            if row == close_row && column >= close_col && column < close_col + close_width {
                lobby.help_open = false;
            }
        }
        true
    } else {
        false
    }
}

fn is_printable_key(key: KeyEvent) -> Option<char> {
    match key.code {
        KeyCode::Char(ch) => Some(ch),
        _ => None,
    }
}

fn append_text(target: &mut String, text: &str) {
    target.extend(text.chars().filter(|ch| !ch.is_control()));
}

fn field_string_mut(model: &mut FirstRunModel) -> &mut String {
    match model.active_field {
        FirstRunField::Handle => &mut model.handle_input,
        FirstRunField::Password => &mut model.password_input,
        FirstRunField::Confirm => &mut model.confirm_input,
        FirstRunField::Relay => &mut model.relay_input,
    }
}

fn mask(value: &str) -> String {
    "●".repeat(value.chars().count())
}
